/**
 * Widget Environment - Monitoring environnemental IoT évolutif
 *
 * Affiche TOUS les sensors/rooms du système de manière dynamique:
 * - Fetch automatique depuis GET /v1/environment/sensors
 * - Regroupement par room_id (chambre, salon, bureau...)
 * - Température + Humidité + Status temps réel
 * - Scalable pour 1...N sensors sans hard-coding
 * - MQTT updates en temps réel (optionnel future)
 */

import { LitElement, html, css } from 'lit'
import { emptyStateStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles } from '../styles/shared-patterns.js'
import DOMPurify from 'dompurify'
import '../components/organic-loader.js'
import { Chart, registerables } from 'chart.js'
import csrfService from '../services/csrf-service.js'
import authService from '../services/auth-service.js'
import pollingScheduler from '../services/polling-scheduler.js'
import { escapeHtml } from '../utils/sanitization.js'
import { createFocusTrap } from '../utils/focus-trap.js'

// Register Chart.js components
Chart.register(...registerables)

class EnvironmentWidget extends LitElement {
  // Disable shadow DOM to allow modal to escape and overlay properly
  createRenderRoot() {
    return this
  }

  static properties = {
    sensors: { type: Array },
    environments: { type: Object }, // Map: room_id -> environment data
    loading: { type: Boolean },
    error: { type: String },
    viewMode: { type: String }, // 'grid' or 'list'
    selectedRoom: { type: String }, // Room ID for modal
    modalOpen: { type: Boolean },
    chartData: { type: Array }, // Historical data for chart
    loadingChart: { type: Boolean }
  }

  static styles = [emptyStateStyles, statusBadgeStyles, css`
    :host {
      display: block;
      background: var(--widget-background, var(--color-dark-surface, #1a1a1a));
      border-radius: var(--radius-md, 0.75rem);
      padding: 20px;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
      color: var(--widget-color, #e5e5e5);
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    }

    .widget-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
      padding-bottom: 12px;
      border-bottom: 1px solid var(--border-medium);
    }

    .widget-title {
      font-size: var(--text-lg, 1.125rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 8px;
    }

    /* widget-count provided by widgetHeaderStyles */

    .loading-state {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 40px;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: var(--text-sm, 0.875rem);
    }

    .error-state {
      padding: 20px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: var(--radius-base, 0.5rem);
      color: var(--color-error-text-muted, #fca5a5);
      text-align: center;
    }

    .auth-required-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 40px 20px;
      text-align: center;
    }

    .auth-icon {
      font-size: 48px;
      margin-bottom: 16px;
      opacity: 0.8;
    }

    .auth-title {
      font-size: var(--text-lg, 1.125rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      margin-bottom: 8px;
    }

    .auth-message {
      font-size: var(--text-sm, 0.875rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-bottom: 20px;
      max-width: 280px;
    }

    .auth-login-btn {
      background: linear-gradient(135deg, #10b981 0%, #059669 100%);
      color: white;
      border: none;
      padding: 12px 24px;
      border-radius: var(--radius-base, 0.5rem);
      font-size: var(--text-sm, 0.875rem);
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .auth-login-btn:hover {
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
    }

    .auth-login-btn:focus-visible {
      outline: 2px solid var(--color-focus, #4f9eff);
      outline-offset: 2px;
    }

    .rooms-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      gap: 16px;
    }

    .room-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-md, 0.75rem);
      padding: 18px;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      cursor: pointer;
      position: relative;
      overflow: hidden;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    }

    .room-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 4px;
      height: 100%;
      transition: all var(--duration-base) var(--ease-out);
    }

    .room-card.normal::before {
      background: linear-gradient(180deg, #22c55e 0%, #16a34a 100%);
      box-shadow: 0 0 15px rgba(34, 197, 94, 0.4);
    }

    .room-card.mold_risk::before {
      background: linear-gradient(180deg, #f97316 0%, #ea580c 100%);
      box-shadow: 0 0 20px rgba(249, 115, 22, 0.5);
    }

    .room-card.temp_low::before {
      background: linear-gradient(180deg, #3b82f6 0%, #2563eb 100%);
      box-shadow: 0 0 15px rgba(59, 130, 246, 0.4);
    }

    .room-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass-hover) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-4px) scale(1.02);
      box-shadow: 0 12px 32px var(--ctx-bg-strong);
    }

    .room-card:hover::before {
      width: 6px;
    }

    .room-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
    }

    .room-name {
      font-size: var(--text-base, 1rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      text-transform: capitalize;
    }

    /* status-badge variants (.ok, .normal, .mold_risk, .temp_low) from shared statusBadgeStyles */
    .status-badge.mold_risk {
      animation: pulse-warning 2s ease-in-out infinite;
    }

    @keyframes pulse-warning {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.6; }
    }

    .readings {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 12px;
      margin-bottom: 12px;
    }

    .reading-item {
      background: var(--surface-glass-strong, rgba(0, 0, 0, 0.2));
      padding: 12px;
      border-radius: var(--radius-base, 0.5rem);
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .reading-label {
      font-size: 11px;
      color: var(--color-dark-text-tertiary, #94a3b8);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .reading-value {
      font-size: var(--text-2xl, 1.5rem);
      font-weight: 700;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: baseline;
      gap: 4px;
    }

    .reading-unit {
      font-size: var(--text-sm, 0.875rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-weight: 400;
    }

    .sensor-info {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-top: 12px;
      border-top: 1px solid var(--border-medium);
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .sensor-type {
      text-transform: uppercase;
      font-weight: 600;
    }

    .sensor-signal {
      display: flex;
      align-items: center;
      gap: 4px;
    }

    .signal-icon {
      width: 16px;
      height: 16px;
    }

    .sensor-delete-btn {
      background: rgba(156, 163, 175, 0.15);
      border: 1px solid rgba(156, 163, 175, 0.25);
      color: var(--color-dark-text-tertiary, #94a3b8);
      padding: 4px 8px;
      border-radius: var(--radius-sm, 0.375rem);
      cursor: pointer;
      font-size: var(--text-xs, 0.75rem);
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 4px;
    }

    .sensor-delete-btn:hover {
      background: rgba(239, 68, 68, 0.2);
      border-color: rgba(239, 68, 68, 0.4);
      color: var(--color-error-text-muted, #fca5a5);
    }

    /* empty-state provided by emptyStateStyles */

    /* Modal Styles */
    .modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: var(--surface-overlay, rgba(0, 0, 0, 0.85));
      backdrop-filter: blur(var(--blur-base));
      z-index: 1000;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 20px;
      animation: fadeIn 0.2s ease;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    .modal-content {
      background: linear-gradient(135deg, var(--color-dark-surface, #1a1a1a) 0%, var(--color-dark-elevated, #252525) 100%);
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-lg);
      max-width: 900px;
      width: 100%;
      max-height: 90vh;
      overflow-y: auto;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
      animation: slideUp 0.3s ease;
    }

    @keyframes slideUp {
      from {
        transform: translateY(30px);
        opacity: 0;
      }
      to {
        transform: translateY(0);
        opacity: 1;
      }
    }

    .modal-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 24px;
      border-bottom: 1px solid var(--border-medium);
    }

    .modal-title {
      font-size: var(--text-2xl, 1.5rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      text-transform: capitalize;
    }

    .modal-close {
      background: var(--surface-glass-strong);
      border: 1px solid var(--border-hover);
      color: var(--color-dark-text-primary, #f8f9fa);
      width: 36px;
      height: 36px;
      border-radius: var(--radius-base, 0.5rem);
      cursor: pointer;
      font-size: var(--text-xl, 1.25rem);
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .modal-close:hover {
      background: var(--surface-glass-bright);
      border-color: var(--border-strong);
    }

    .modal-body {
      padding: 24px;
    }

    .chart-container {
      position: relative;
      height: 400px;
      margin-top: 20px;
    }

    .chart-loading {
      display: flex;
      align-items: center;
      justify-content: center;
      height: 400px;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .room-card {
      cursor: pointer;
    }

    .room-card:hover {
      transform: translateY(-4px);
    }

    /* Mobile Responsive */
    @media (max-width: 768px) {
      .modal-overlay {
        padding: 0;
        align-items: stretch;
      }

      .modal-content {
        max-width: 100%;
        max-height: 100vh;
        height: 100vh;
        border-radius: 0;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
      }

      .modal-header {
        padding: 16px;
        position: sticky;
        top: 0;
        background: linear-gradient(135deg, var(--color-dark-surface, #1a1a1a) 0%, var(--color-dark-elevated, #252525) 100%);
        z-index: 10;
        flex-shrink: 0;
      }

      .modal-title {
        font-size: var(--text-lg, 1.125rem);
      }

      .modal-body {
        padding: 16px;
        padding-bottom: 80px; /* Space for mobile nav */
        flex: 1;
        overflow-y: auto;
      }

      .chart-container {
        height: 300px;
        margin-top: 12px;
      }

      .chart-loading {
        height: 300px;
        font-size: var(--text-sm, 0.875rem);
      }
    }

    /* Utility classes (ex-inline) */
    .ew-chart-hint { color: var(--color-dark-text-tertiary, #888); margin: 0 0 12px 0; }
    .ew-sensor-value-row { display: flex; align-items: center; gap: 8px; }
  `]

  constructor() {
    super()
    this.sensors = []
    this.environments = {}
    this.loading = true
    this.error = null
    this.viewMode = 'grid'
    this.selectedRoom = null
    this.modalOpen = false
    this.chartData = []
    this.loadingChart = false
    this.chart = null  // Chart.js instance
    this.focusTrap = null  // Focus trap for modal
  }

  connectedCallback() {
    super.connectedCallback()
    // Use centralized polling scheduler (auto-pauses when page hidden)
    this._unsubscribePolling = pollingScheduler.subscribe('30s', () => this.loadEnvironmentData())
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._unsubscribePolling) {
      this._unsubscribePolling()
      this._unsubscribePolling = null
    }
  }

  /**
   * Check if user is authenticated
   * @returns {boolean}
   */
  isAuthenticated() {
    return authService.isAuthenticated()
  }

  /**
   * Get auth headers for API requests (JWT)
   * @returns {Object} Headers object with Authorization if authenticated
   */
  getAuthHeaders() {
    if (!this.isAuthenticated()) {
      return {}
    }
    return authService.getAuthHeader()
  }

  async loadEnvironmentData() {
    try {
      this.loading = true
      this.error = null

      // Check authentication first
      if (!this.isAuthenticated()) {
        console.log('[environment-widget] Not authenticated, waiting for login')
        this.loading = false
        this.error = 'auth_required'
        return
      }

      // Fetch all sensors with JWT auth
      const apiBase = window.SYMBION_CONFIG?.API_BASE || 'https://localhost:8443'
      const headers = this.getAuthHeaders()
      const sensorsResponse = await fetch(`${apiBase}/v1/environment/sensors`, { headers })

      if (!sensorsResponse.ok) {
        if (sensorsResponse.status === 401 || sensorsResponse.status === 403) {
          console.log('[environment-widget] Auth expired or invalid')
          this.loading = false
          this.error = 'auth_required'
          return
        }
        throw new Error(`Failed to fetch sensors: ${sensorsResponse.status}`)
      }

      const sensorsData = await sensorsResponse.json()
      this.sensors = sensorsData.sensors || []

      // Fetch environment data for each unique room
      const uniqueRooms = [...new Set(this.sensors.map(s => s.room_id))]
      const environments = {}

      await Promise.all(
        uniqueRooms.map(async (roomId) => {
          try {
            const envResponse = await fetch(`${apiBase}/v1/environment/${roomId}`, { headers })

            if (envResponse.ok) {
              environments[roomId] = await envResponse.json()
            }
          } catch (err) {
            console.warn(`Failed to fetch environment for ${roomId}:`, err)
          }
        })
      )

      this.environments = environments
      this.loading = false
    } catch (err) {
      console.error('Environment widget error:', err)
      this.error = err.message
      this.loading = false
    }
  }

  getStatusIcon(status) {
    const icons = {
      // New dew point-based alert levels (physics-based)
      safe: '✅',
      weak: '💧',
      moderate: '💧💧',
      strong: '⚠️',
      critical: '🚨',
      danger: '🔴',
      // Legacy compatibility (deprecated)
      ok: '✓',
      normal: '✓',
      mold_risk: '🚨',
      temp_low: '❄',
      n_a: '⚠️',
      humid: '💧',
      risk_mold: '🚨',
      cold: '❄'
    }
    return icons[status] || '?'
  }

  getStatusLabel(status) {
    const labels = {
      // New dew point-based alert levels (Magnus formula)
      safe: 'Normal',
      weak: 'Tendance Haute',
      moderate: 'Humidité Excessive',
      strong: 'Risque Condensation',
      critical: 'Condensation Probable',
      danger: 'Condensation Certaine',
      // Legacy compatibility (deprecated)
      ok: 'Normal',
      normal: 'Normal',
      mold_risk: 'Risque Moisissure',
      temp_low: 'Froid',
      n_a: 'Capteur Déconnecté',
      humid: 'Humide',
      risk_mold: 'Risque Moisissure',
      cold: 'Froid'
    }
    return labels[status] || status
  }

  formatSignalStrength(rssi) {
    if (!rssi) return ''
    if (rssi > -50) return '📶 Excellent'
    if (rssi > -60) return '📶 Bon'
    if (rssi > -70) return '📶 Moyen'
    return '📶 Faible'
  }

  async openRoomModal(roomId) {
    console.log('[environment-widget] Opening modal for room:', roomId)
    this.selectedRoom = roomId
    this.modalOpen = true
    this.loadingChart = true
    this.chartData = []

    // Create modal container and append to document.body (portal pattern)
    this.modalContainer = document.createElement('div')
    this.modalContainer.id = 'environment-modal-portal'
    document.body.appendChild(this.modalContainer)

    // Render loading state
    this.renderModalToPortal()

    try {
      // Check authentication before fetching
      if (!this.isAuthenticated()) {
        throw new Error('auth_required')
      }

      // Fetch 7 days of history (168 hours)
      const apiBase = window.SYMBION_CONFIG?.API_BASE || 'https://localhost:8443'
      const response = await fetch(`${apiBase}/v1/environment/${roomId}/history?hours=168`, {
        headers: this.getAuthHeaders()
      })

      if (!response.ok) {
        throw new Error(`Failed to fetch history: ${response.status}`)
      }

      const historyData = await response.json()
      this.chartData = historyData
      this.loadingChart = false

      // Re-render modal with chart data
      this.renderModalToPortal()

      // Wait a tick for DOM update then create chart
      setTimeout(() => this.createChartInPortal(), 100)
    } catch (err) {
      console.error('Failed to load chart data:', err)
      this.loadingChart = false
      this.renderModalToPortal()
    }
  }

  closeModal() {
    this.modalOpen = false
    this.selectedRoom = null

    // Destroy focus trap
    if (this.focusTrap) {
      this.focusTrap.destroy()
      this.focusTrap = null
    }

    if (this.chart) {
      this.chart.destroy()
      this.chart = null
    }
    // Remove modal from document.body
    if (this.modalContainer && this.modalContainer.parentNode) {
      this.modalContainer.parentNode.removeChild(this.modalContainer)
      this.modalContainer = null
    }
  }

  renderModalToPortal() {
    if (!this.modalContainer) return

    const env = this.environments[this.selectedRoom]
    if (!env) {
      console.warn('[environment-widget] No environment data for:', this.selectedRoom)
      return
    }

    // Inject styles separately (DOMPurify strips <style> content)
    const styleEl = document.createElement('style')
    styleEl.textContent = [].concat(EnvironmentWidget.styles).map(s => s.cssText).join('\n')

    // Build modal HTML (sanitized without styles)
    const modalHTML = `
      <div class="modal-overlay" id="modal-overlay" aria-hidden="true">
        <div class="modal-content" id="modal-content" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <div class="modal-header">
            <h2 class="modal-title" id="modal-title">
              📊 Historique - ${escapeHtml(this.selectedRoom)}
            </h2>
            <button class="modal-close" id="modal-close-btn" aria-label="Fermer">
              ✕
            </button>
          </div>

          <div class="modal-body">
            ${this.loadingChart ? `
              <div class="chart-loading">
                <organic-loader></organic-loader>
              </div>
            ` : `
              <div>
                <p class="ew-chart-hint">
                  Derniers 7 jours (${this.chartData.length} lectures)
                </p>
                <div class="chart-container">
                  <canvas id="environmentChart"></canvas>
                </div>
              </div>
            `}
          </div>
        </div>
      </div>
    `

    this.modalContainer.innerHTML = ''
    this.modalContainer.appendChild(styleEl)
    this.modalContainer.insertAdjacentHTML('beforeend', DOMPurify.sanitize(modalHTML, { ADD_TAGS: ['canvas'], ADD_ATTR: ['id'] }))

    // Attach event listeners
    const overlay = this.modalContainer.querySelector('#modal-overlay')
    const closeBtn = this.modalContainer.querySelector('#modal-close-btn')
    const content = this.modalContainer.querySelector('#modal-content')

    if (overlay) {
      overlay.addEventListener('click', () => this.closeModal())
    }
    if (closeBtn) {
      closeBtn.addEventListener('click', () => this.closeModal())
    }
    if (content) {
      content.addEventListener('click', (e) => e.stopPropagation())

      // Escape key closes modal
      content.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
          this.closeModal()
        }
      })

      // Activate focus trap (only once when modal first opens)
      if (!this.focusTrap) {
        this.focusTrap = createFocusTrap(content)
        this.focusTrap.activate()
      }
    }
  }

  createChartInPortal() {
    if (!this.modalContainer) return

    const canvas = this.modalContainer.querySelector('#environmentChart')
    if (!canvas || !this.chartData || this.chartData.length === 0) {
      console.warn('Cannot create chart: missing canvas or data')
      return
    }

    // Destroy existing chart if any
    if (this.chart) {
      this.chart.destroy()
    }

    // Downsample data for better performance and readability
    // Keep 1 point every 30 minutes instead of every 5 seconds
    const downsampleInterval = 6 // 30 min = 6 * 5 sec
    const downsampledData = this.chartData.filter((_, index) => index % downsampleInterval === 0)

    console.log(`[chart] Downsampled from ${this.chartData.length} to ${downsampledData.length} points`)

    // Reverse data so oldest is on left, newest on right (chronological order)
    downsampledData.reverse()

    // Calculate stats for display
    const temps = this.chartData.map(r => r.temperature_c)
    const humids = this.chartData.map(r => r.humidity_pct)
    const tempStats = {
      min: Math.min(...temps).toFixed(1),
      max: Math.max(...temps).toFixed(1),
      avg: (temps.reduce((a, b) => a + b, 0) / temps.length).toFixed(1)
    }
    const humidStats = {
      min: Math.min(...humids).toFixed(1),
      max: Math.max(...humids).toFixed(1),
      avg: (humids.reduce((a, b) => a + b, 0) / humids.length).toFixed(1)
    }

    // Prepare data for Chart.js with adaptive date formatting
    const labels = downsampledData.map(r => {
      const date = new Date(r.timestamp)
      // If more than 2 days of data, show only day + hour
      if (this.chartData.length > 576) { // 2 days at 5sec interval
        return date.toLocaleDateString('fr-FR', {
          day: 'numeric',
          month: 'short',
          hour: '2-digit'
        })
      }
      // Otherwise show full date + time
      return date.toLocaleDateString('fr-FR', {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      })
    })

    const temperatures = downsampledData.map(r => r.temperature_c)
    const humidities = downsampledData.map(r => r.humidity_pct)

    this.chart = new Chart(canvas, {
      type: 'line',
      data: {
        labels,
        datasets: [
          {
            label: `Température (°C) - Min: ${tempStats.min}° | Moy: ${tempStats.avg}° | Max: ${tempStats.max}°`,
            data: temperatures,
            borderColor: 'rgb(34, 197, 94)', // Green for temperature
            backgroundColor: 'rgba(34, 197, 94, 0.15)',
            borderWidth: 2,
            tension: 0.4,
            fill: true,
            yAxisID: 'y',
            pointRadius: 0, // Hide points for cleaner look
            pointHoverRadius: 6 // Show on hover
          },
          {
            label: `Humidité (%) - Min: ${humidStats.min}% | Moy: ${humidStats.avg}% | Max: ${humidStats.max}%`,
            data: humidities,
            borderColor: 'rgb(59, 130, 246)', // Blue for humidity
            backgroundColor: 'rgba(59, 130, 246, 0.15)',
            borderWidth: 2,
            tension: 0.4,
            fill: true,
            yAxisID: 'y1',
            pointRadius: 0,
            pointHoverRadius: 6
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: 'index',
          intersect: false,
        },
        plugins: {
          legend: {
            position: window.innerWidth < 768 ? 'bottom' : 'top',
            labels: {
              color: '#e0e0e0',
              font: {
                size: window.innerWidth < 768 ? 11 : 14
              },
              padding: window.innerWidth < 768 ? 8 : 10,
              boxWidth: window.innerWidth < 768 ? 12 : 15
            }
          },
          tooltip: {
            backgroundColor: 'rgba(0, 0, 0, 0.9)',
            titleColor: '#ffffff',
            bodyColor: '#e0e0e0',
            borderColor: 'rgba(59, 130, 246, 0.3)',
            borderWidth: 1,
            titleFont: {
              size: window.innerWidth < 768 ? 12 : 14
            },
            bodyFont: {
              size: window.innerWidth < 768 ? 11 : 13
            },
            padding: window.innerWidth < 768 ? 8 : 12
          }
        },
        scales: {
          x: {
            ticks: {
              color: '#888',
              maxRotation: window.innerWidth < 768 ? 60 : 45,
              minRotation: window.innerWidth < 768 ? 60 : 45,
              maxTicksLimit: window.innerWidth < 768 ? 8 : 12,
              autoSkip: true,
              font: {
                size: window.innerWidth < 768 ? 9 : 11
              }
            },
            grid: {
              color: 'rgba(255, 255, 255, 0.1)'
            }
          },
          y: {
            type: 'linear',
            display: true,
            position: 'left',
            title: {
              display: true,
              text: 'Température (°C)',
              color: 'rgb(34, 197, 94)',
              font: {
                size: window.innerWidth < 768 ? 10 : 12
              }
            },
            ticks: {
              color: 'rgb(34, 197, 94)',
              font: {
                size: window.innerWidth < 768 ? 9 : 11
              }
            },
            grid: {
              color: 'rgba(255, 255, 255, 0.1)'
            }
          },
          y1: {
            type: 'linear',
            display: true,
            position: 'right',
            title: {
              display: true,
              text: 'Humidité (%)',
              color: 'rgb(59, 130, 246)',
              font: {
                size: window.innerWidth < 768 ? 10 : 12
              }
            },
            ticks: {
              color: 'rgb(59, 130, 246)',
              font: {
                size: window.innerWidth < 768 ? 9 : 11
              }
            },
            grid: {
              drawOnChartArea: false,
            }
          }
        }
      }
    })
  }

  render() {
    // Inject styles in light DOM since we disabled shadow DOM
    const styles = EnvironmentWidget.styles
    const cssText = Array.isArray(styles) ? styles.map(s => s.cssText).join('\n') : styles.cssText
    const styleTag = html`<style>${cssText}</style>`

    if (this.loading) {
      return html`
        ${styleTag}
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
        </div>
        <div class="loading-state">
          <organic-loader></organic-loader>
        </div>
      `
    }

    if (this.error) {
      // Special UI for authentication required
      if (this.error === 'auth_required') {
        return html`
          ${styleTag}
          <div class="widget-header">
            <div class="widget-title">🌡️ Environnement</div>
          </div>
          <div class="auth-required-state">
            <div class="auth-icon">🔐</div>
            <div class="auth-title">Authentification requise</div>
            <div class="auth-message">
              Connectez-vous pour accéder aux données d'environnement
            </div>
            <button class="auth-login-btn" @click=${() => window.location.hash = '#/settings'}>
              Se connecter
            </button>
          </div>
        `
      }

      return html`
        ${styleTag}
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
        </div>
        <div class="error-state">
          <span class="error-icon">⚠️</span> ${this.error}
        </div>
      `
    }

    const roomCount = Object.keys(this.environments).length

    if (roomCount === 0) {
      return html`
        ${styleTag}
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
          <span class="widget-count">0 capteur</span>
        </div>
        <div class="empty-state">
          <div class="empty-state-icon">🌡️</div>
          <div class="empty-state-text">
            Aucun capteur environnemental détecté.<br>
            Connectez un ESP32 avec BME280 pour commencer.
          </div>
        </div>
      `
    }

    return html`
      ${styleTag}
      <div class="widget-header">
        <div class="widget-title">
          🌡️ Environnement
          <span class="widget-count">${this.sensors.length} capteur${this.sensors.length > 1 ? 's' : ''}</span>
        </div>
      </div>

      <div class="rooms-grid">
        ${Object.entries(this.environments).map(([roomId, env]) => {
          const sensor = this.sensors.find(s => s.room_id === roomId)
          return this.renderRoomCard(roomId, env, sensor)
        })}
      </div>
    `
  }

  renderRoomCard(roomId, env, sensor) {
    // Handle null values when sensor is offline (>30 sec)
    const temp = env.current.temperature_c !== null
      ? env.current.temperature_c.toFixed(1)
      : 'N/A'
    const humidity = env.current.humidity_pct !== null
      ? env.current.humidity_pct.toFixed(1)
      : 'N/A'
    const status = env.status
    const statusLabel = this.getStatusLabel(status)

    return html`
      <div class="room-card ${status}" @click="${() => this.openRoomModal(roomId)}">
        <div class="room-header">
          <div class="room-name">${roomId}</div>
          <div class="status-badge ${status}">
            ${this.getStatusIcon(status)} ${statusLabel}
          </div>
        </div>

        <div class="readings">
          <div class="reading-item">
            <div class="reading-label">Température</div>
            <div class="reading-value">
              ${temp}<span class="reading-unit">°C</span>
            </div>
          </div>

          <div class="reading-item">
            <div class="reading-label">Humidité</div>
            <div class="reading-value">
              ${humidity}<span class="reading-unit">%</span>
            </div>
          </div>
        </div>

        ${sensor ? html`
          <div class="sensor-info">
            <span class="sensor-type">${sensor.sensor_type}</span>
            <div class="ew-sensor-value-row">
              ${status !== 'n_a' ? html`
                <span class="sensor-signal">${this.formatSignalStrength(sensor.signal_rssi)}</span>
              ` : ''}
              <button
                class="sensor-delete-btn"
                @click="${(e) => this.confirmDeleteSensor(e, sensor)}"
                title="Supprimer le capteur (purge après 7 jours)">
                🗑️
              </button>
            </div>
          </div>
        ` : ''}
      </div>
    `
  }

  async confirmDeleteSensor(event, sensor) {
    event.stopPropagation() // Empêche l'ouverture du modal

    const confirmMsg = `Supprimer le capteur "${sensor.sensor_id}" (${sensor.room_id}) ?\n\n` +
      `Le capteur sera marqué comme supprimé et définitivement effacé après 7 jours.\n\n` +
      `S'il se reconnecte pendant cette période, il sera réactivé automatiquement.`

    if (!confirm(confirmMsg)) {
      return
    }

    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      const url = `${API_BASE}/v1/environment/sensors/${encodeURIComponent(sensor.sensor_id)}`

      const response = await csrfService.fetchWithCsrf(url, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json'
        }
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`)
      }

      alert(`✅ Capteur "${sensor.sensor_id}" supprimé (purge dans 7 jours)`)

      // Refresh la liste
      await this.loadEnvironmentData()

    } catch (error) {
      console.error(`[environment-widget] Failed to delete sensor:`, error)
      alert(`❌ Erreur lors de la suppression: ${error.message}`)
    }
  }

}

customElements.define('environment-widget', EnvironmentWidget)
