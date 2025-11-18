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
import '../components/organic-loader.js'
import { Chart, registerables } from 'chart.js'

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

  static styles = css`
    :host {
      display: block;
      background: var(--widget-background, #1a1a1a);
      border-radius: 12px;
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
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .widget-title {
      font-size: 18px;
      font-weight: 600;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .sensor-count {
      font-size: 12px;
      color: #888;
      background: rgba(255, 255, 255, 0.1);
      padding: 4px 8px;
      border-radius: 12px;
    }

    .loading-state {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 40px;
      color: #888;
      font-size: 14px;
    }

    .error-state {
      padding: 20px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: 8px;
      color: #fca5a5;
      text-align: center;
    }

    .rooms-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      gap: 16px;
    }

    .room-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(59, 130, 246, 0.2);
      border-radius: 12px;
      padding: 18px;
      transition: all 0.3s ease;
      position: relative;
      overflow: hidden;
    }

    .room-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 4px;
      height: 100%;
      transition: all 0.3s ease;
    }

    .room-card.normal::before {
      background: linear-gradient(180deg, #22c55e 0%, #16a34a 100%);
      box-shadow: 0 0 15px rgba(34, 197, 94, 0.4);
    }

    .room-card.humid::before {
      background: linear-gradient(180deg, #eab308 0%, #ca8a04 100%);
      box-shadow: 0 0 15px rgba(234, 179, 8, 0.4);
    }

    .room-card.risk_mold::before {
      background: linear-gradient(180deg, #f97316 0%, #ea580c 100%);
      box-shadow: 0 0 20px rgba(249, 115, 22, 0.5);
    }

    .room-card.cold::before {
      background: linear-gradient(180deg, #3b82f6 0%, #2563eb 100%);
      box-shadow: 0 0 15px rgba(59, 130, 246, 0.4);
    }

    .room-card:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      border-color: rgba(59, 130, 246, 0.4);
      transform: translateY(-2px);
      box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
    }

    .room-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
    }

    .room-name {
      font-size: 16px;
      font-weight: 600;
      color: #ffffff;
      text-transform: capitalize;
    }

    .status-badge {
      padding: 4px 10px;
      border-radius: 12px;
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .status-badge.ok,
    .status-badge.normal {
      background: rgba(34, 197, 94, 0.2);
      color: #86efac;
      border: 1px solid rgba(34, 197, 94, 0.3);
    }

    .status-badge.humid {
      background: rgba(234, 179, 8, 0.2);
      color: #fde047;
      border: 1px solid rgba(234, 179, 8, 0.3);
    }

    .status-badge.risk_mold {
      background: rgba(249, 115, 22, 0.2);
      color: #fdba74;
      border: 1px solid rgba(249, 115, 22, 0.3);
      animation: pulse-warning 2s ease-in-out infinite;
    }

    .status-badge.cold {
      background: rgba(59, 130, 246, 0.2);
      color: #93c5fd;
      border: 1px solid rgba(59, 130, 246, 0.3);
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
      background: rgba(0, 0, 0, 0.2);
      padding: 12px;
      border-radius: 8px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .reading-label {
      font-size: 11px;
      color: #888;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .reading-value {
      font-size: 24px;
      font-weight: 700;
      color: #ffffff;
      display: flex;
      align-items: baseline;
      gap: 4px;
    }

    .reading-unit {
      font-size: 14px;
      color: #888;
      font-weight: 400;
    }

    .sensor-info {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-top: 12px;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
      font-size: 12px;
      color: #888;
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

    .empty-state {
      text-align: center;
      padding: 40px 20px;
      color: #888;
    }

    .empty-icon {
      font-size: 48px;
      margin-bottom: 16px;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 14px;
      line-height: 1.6;
    }

    /* Modal Styles */
    .modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(8px);
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
      background: linear-gradient(135deg, #1a1a1a 0%, #252525 100%);
      border: 1px solid rgba(59, 130, 246, 0.3);
      border-radius: 16px;
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
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .modal-title {
      font-size: 24px;
      font-weight: 600;
      color: #ffffff;
      text-transform: capitalize;
    }

    .modal-close {
      background: rgba(255, 255, 255, 0.1);
      border: 1px solid rgba(255, 255, 255, 0.2);
      color: #ffffff;
      width: 36px;
      height: 36px;
      border-radius: 8px;
      cursor: pointer;
      font-size: 20px;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .modal-close:hover {
      background: rgba(255, 255, 255, 0.2);
      border-color: rgba(255, 255, 255, 0.3);
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
      color: #888;
    }

    .room-card {
      cursor: pointer;
    }

    .room-card:hover {
      transform: translateY(-4px);
    }
  `

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
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadEnvironmentData()
    // Refresh data every 30 seconds
    this.refreshInterval = setInterval(() => this.loadEnvironmentData(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval)
    }
  }

  async loadEnvironmentData() {
    try {
      this.loading = true
      this.error = null

      // Fetch all sensors
      const apiBase = window.SYMBION_CONFIG?.API_BASE || 'https://localhost:8443'
      const sensorsResponse = await fetch(`${apiBase}/v1/environment/sensors`, {
        headers: {
          'X-API-Key': localStorage.getItem('symbion_api_key') || window.SYMBION_CONFIG?.API_KEY || 's3cr3t-42'
        }
      })

      if (!sensorsResponse.ok) {
        // Silently fail if unauthorized (user not logged in yet)
        if (sensorsResponse.status === 401 || sensorsResponse.status === 403) {
          console.log('[environment-widget] Not authorized yet, waiting for login')
          this.loading = false
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
            const envResponse = await fetch(`${apiBase}/v1/environment/${roomId}`, {
              headers: {
                'X-API-Key': localStorage.getItem('symbion_api_key') || window.SYMBION_CONFIG?.API_KEY || 's3cr3t-42'
              }
            })

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
      ok: '✓',
      normal: '✓',
      humid: '⚠',
      risk_mold: '🚨',
      cold: '❄'
    }
    return icons[status] || '?'
  }

  getStatusLabel(status) {
    const labels = {
      ok: 'Normal',
      normal: 'Normal',
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
      // Fetch 7 days of history (168 hours)
      const apiBase = window.SYMBION_CONFIG?.API_BASE || 'https://localhost:8443'
      const response = await fetch(`${apiBase}/v1/environment/${roomId}/history?hours=168`, {
        headers: {
          'X-API-Key': localStorage.getItem('symbion_api_key') || window.SYMBION_CONFIG?.API_KEY || 's3cr3t-42'
        }
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

    // Build modal HTML string
    const modalHTML = `
      <style>
        ${EnvironmentWidget.styles.cssText}
      </style>
      <div class="modal-overlay" id="modal-overlay">
        <div class="modal-content" id="modal-content">
          <div class="modal-header">
            <div class="modal-title">
              📊 Historique - ${this.selectedRoom}
            </div>
            <button class="modal-close" id="modal-close-btn">
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
                <p style="color: #888; margin: 0 0 12px 0;">
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

    this.modalContainer.innerHTML = modalHTML

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
            labels: {
              color: '#e0e0e0',
              font: {
                size: 14
              }
            }
          },
          tooltip: {
            backgroundColor: 'rgba(0, 0, 0, 0.9)',
            titleColor: '#ffffff',
            bodyColor: '#e0e0e0',
            borderColor: 'rgba(59, 130, 246, 0.3)',
            borderWidth: 1
          }
        },
        scales: {
          x: {
            ticks: {
              color: '#888',
              maxRotation: 45,
              minRotation: 45,
              maxTicksLimit: 12, // Limit to ~12 labels for readability
              autoSkip: true
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
              color: 'rgb(34, 197, 94)' // Green
            },
            ticks: {
              color: 'rgb(34, 197, 94)'
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
              color: 'rgb(59, 130, 246)' // Blue
            },
            ticks: {
              color: 'rgb(59, 130, 246)'
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
    const styleTag = html`<style>${EnvironmentWidget.styles.cssText}</style>`

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
      return html`
        ${styleTag}
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
        </div>
        <div class="error-state">
          ⚠️ Erreur: ${this.error}
        </div>
      `
    }

    const roomCount = Object.keys(this.environments).length

    if (roomCount === 0) {
      return html`
        ${styleTag}
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
          <span class="sensor-count">0 capteur</span>
        </div>
        <div class="empty-state">
          <div class="empty-icon">🌡️</div>
          <div class="empty-text">
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
          <span class="sensor-count">${this.sensors.length} capteur${this.sensors.length > 1 ? 's' : ''}</span>
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
    const temp = env.current.temperature_c.toFixed(1)
    const humidity = env.current.humidity_pct.toFixed(1)
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
            <span class="sensor-signal">${this.formatSignalStrength(sensor.signal_rssi)}</span>
          </div>
        ` : ''}
      </div>
    `
  }

}

customElements.define('environment-widget', EnvironmentWidget)
