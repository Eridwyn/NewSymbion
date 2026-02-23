/**
 * SSL Monitor Widget
 *
 * Affiche le statut des certificats SSL des domaines surveillés
 * Mise à jour temps réel via MQTT
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusDotStyles, sectionBadgeStyles } from '../styles/shared-patterns.js'
import '../components/organic-loader.js'

export class SslWidget extends LitElement {
  static properties = {
    domains: { type: Array },
    loading: { type: Boolean },
    lastUpdate: { type: String },
    mqttConnected: { type: Boolean },
    showConfig: { type: Boolean },
    editingDomain: { type: Object },
    formData: { type: Object }
  }

  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, emptyStateStyles, statusDotStyles, sectionBadgeStyles, css`
    :host {
      display: block;
    }

    .header-right {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .summary-badge {
      padding: 0.4rem 0.8rem;
      border-radius: var(--radius-xl);
      font-size: 0.75em;
      font-weight: 600;
      letter-spacing: 0.5px;
    }

    .summary-ok {
      background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, rgba(34, 197, 94, 0.2) 100%);
      color: var(--context-primary, #00d4aa);
      border: 1px solid var(--ctx-border-strong);
    }

    .summary-warning {
      background: linear-gradient(135deg, rgba(251, 191, 36, 0.25) 0%, rgba(245, 158, 11, 0.2) 100%);
      color: #fbbf24;
      border: 1px solid rgba(251, 191, 36, 0.4);
    }

    .summary-critical {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.25) 0%, rgba(239, 68, 68, 0.2) 100%);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.4);
    }

    .refresh-btn {
      background: transparent;
      border: 1px solid var(--border-hover);
      color: var(--color-dark-text-tertiary, #94a3b8);
      min-width: 44px;
      min-height: 44px;
      border-radius: var(--radius-base);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .refresh-btn:hover {
      background: var(--surface-glass-hover);
      color: #00d4ff;
      border-color: rgba(0, 212, 255, 0.3);
    }

    .refresh-btn.spinning svg {
      animation: spin 1s linear infinite;
    }

    .domains-list {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
    }

    .domain-card {
      background: linear-gradient(135deg, var(--border-subtle) 0%, var(--surface-glass-faint) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 1rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      transition: all 0.2s ease;
    }

    .domain-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border-color: var(--border-hover);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
    }

    .domain-card.critical {
      border-color: rgba(255, 107, 107, 0.3);
    }

    .domain-card.warning {
      border-color: rgba(251, 191, 36, 0.3);
    }

    .domain-info {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .status-dot.warning {
      background: #fbbf24;
      box-shadow: 0 0 8px rgba(251, 191, 36, 0.5);
      animation: pulse-warning 2s ease-in-out infinite;
    }

    .status-dot.critical {
      background: #ff6b6b;
      box-shadow: 0 0 8px rgba(255, 107, 107, 0.5);
      animation: pulse-critical 1.5s ease-in-out infinite;
    }

    .status-dot.error {
      background: #666;
    }

    @keyframes pulse-warning {
      0%, 100% { box-shadow: 0 0 8px rgba(251, 191, 36, 0.5); }
      50% { box-shadow: 0 0 14px rgba(251, 191, 36, 0.8); }
    }

    @keyframes pulse-critical {
      0%, 100% { box-shadow: 0 0 8px rgba(255, 107, 107, 0.5); }
      50% { box-shadow: 0 0 14px rgba(255, 107, 107, 0.9); }
    }

    .domain-details {
      display: flex;
      flex-direction: column;
      gap: 0.2rem;
    }

    .domain-name {
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.95em;
      font-weight: 500;
    }

    .domain-issuer {
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: 0.75em;
    }

    .expiry-info {
      text-align: right;
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      gap: 0.2rem;
    }

    .days-remaining {
      font-size: 1.1em;
      font-weight: 600;
    }

    .days-remaining.ok { color: var(--context-primary, #00d4aa); }
    .days-remaining.warning { color: #fbbf24; }
    .days-remaining.critical { color: #ff6b6b; }
    .days-remaining.error { color: var(--color-dark-text-tertiary, #94a3b8); }

    .expiry-date {
      font-size: 0.7em;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .last-update {
      margin-top: 1rem;
      padding-top: 0.75rem;
      border-top: 1px solid var(--border-subtle);
      font-size: 0.7em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      text-align: right;
    }

    .loader-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 150px;
      padding: 2rem;
    }

    /* Config Panel - Modal Overlay */
    .config-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.7);
      backdrop-filter: blur(4px);
      z-index: 1000;
      opacity: 0;
      visibility: hidden;
      transition: all var(--duration-base) var(--ease-out);
    }

    .config-overlay.open {
      opacity: 1;
      visibility: visible;
    }

    /* Config Panel - Slide-in Panel */
    .config-panel {
      position: fixed;
      top: 0;
      right: 0;
      width: 380px;
      max-width: 95vw;
      height: 100vh;
      background: linear-gradient(180deg, #12121a 0%, #0d0d14 100%);
      border-left: 1px solid var(--ctx-border-medium);
      z-index: 1001;
      transform: translateX(100%);
      transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      display: flex;
      flex-direction: column;
      box-shadow: -8px 0 32px rgba(0, 0, 0, 0.5);
    }

    .config-panel.open {
      transform: translateX(0);
    }

    /* Header */
    .config-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.25rem;
      background: var(--ctx-bg);
      border-bottom: 1px solid var(--ctx-border);
    }

    .config-header-left {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .config-header-icon {
      width: 36px;
      height: 36px;
      background: linear-gradient(135deg, var(--ctx-border-medium) 0%, rgba(0, 180, 140, 0.1) 100%);
      border-radius: var(--radius-md, 0.75rem);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--context-primary, #00d4aa);
    }

    .config-title {
      font-size: 1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .close-btn {
      background: transparent;
      border: none;
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      padding: 0.5rem;
      border-radius: var(--radius-base);
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .close-btn:hover {
      background: rgba(255, 107, 107, 0.15);
      color: #ff6b6b;
    }

    /* Content */
    .config-content {
      flex: 1;
      overflow-y: auto;
      padding: 1.25rem;
    }

    /* Section */
    .config-section {
      margin-bottom: 1.5rem;
    }

    .section-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.75rem;
    }

    /* section-title provided by widgetSectionStyles */

    /* Domain List */
    .domain-list {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .domain-card {
      display: flex;
      align-items: stretch;
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md, 0.75rem);
      overflow: hidden;
      transition: all 0.2s ease;
    }

    .domain-card:hover {
      background: var(--surface-glass);
      border-color: var(--border-medium);
    }

    .domain-card-main {
      flex: 1;
      padding: 0.75rem 1rem;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .domain-card-name {
      font-size: 0.9rem;
      font-weight: 500;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .domain-card-host {
      font-size: var(--text-xs);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-family: monospace;
    }

    .domain-card-thresholds {
      display: flex;
      gap: 0.75rem;
      margin-top: 0.25rem;
    }

    .threshold-tag {
      font-size: 0.65rem;
      padding: 0.15rem 0.4rem;
      border-radius: var(--radius-sm);
      font-weight: 500;
    }

    .threshold-warning {
      background: rgba(251, 191, 36, 0.15);
      color: #fbbf24;
    }

    .threshold-critical {
      background: rgba(255, 107, 107, 0.15);
      color: #ff6b6b;
    }

    .domain-card-actions {
      display: flex;
      flex-direction: column;
      border-left: 1px solid var(--border-subtle);
    }

    .domain-card-actions button {
      flex: 1;
      background: transparent;
      border: none;
      padding: 0 0.75rem;
      cursor: pointer;
      color: var(--color-dark-text-tertiary, #94a3b8);
      transition: all 0.2s ease;
    }

    .domain-card-actions button:first-child {
      border-bottom: 1px solid var(--border-subtle);
    }

    .domain-card-actions button:hover {
      background: var(--surface-glass);
    }

    .domain-card-actions .edit-btn:hover {
      color: #00d4ff;
    }

    .domain-card-actions .delete-btn:hover {
      color: #ff6b6b;
    }

    /* Form */
    .form-card {
      background: var(--ctx-bg-subtle);
      border: 1px solid var(--ctx-border);
      border-radius: var(--radius-md);
      padding: 1rem;
    }

    .form-card-title {
      font-size: 0.85rem;
      font-weight: 600;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 1rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .form-group {
      margin-bottom: 0.875rem;
    }

    .form-label {
      display: block;
      font-size: var(--text-xs);
      font-weight: 500;
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-bottom: 0.375rem;
    }

    .form-input {
      width: 100%;
      padding: 0.625rem 0.875rem;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: var(--text-sm);
      box-sizing: border-box;
      transition: all 0.2s ease;
    }

    .form-input:focus {
      outline: none;
      border-color: var(--ctx-border-intense);
      box-shadow: 0 0 0 3px var(--ctx-border-subtle);
    }

    .form-input::placeholder {
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .form-row {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 0.75rem;
    }

    .form-actions {
      display: flex;
      gap: 0.75rem;
      margin-top: 1rem;
    }

    .btn {
      padding: 0.625rem 1rem;
      border-radius: var(--radius-base);
      font-size: 0.8rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
    }

    .btn-primary {
      flex: 1;
      background: linear-gradient(135deg, var(--context-primary, #00d4aa) 0%, #00b89c 100%);
      border: none;
      color: #0a0a0f;
    }

    .btn-primary:hover {
      transform: translateY(-1px);
      box-shadow: 0 4px 16px var(--ctx-border-strong);
    }

    .btn-primary:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
      background: var(--ctx-bg-intense);
    }

    .btn-primary:disabled:hover {
      transform: none;
      box-shadow: none;
    }

    .btn-secondary {
      background: transparent;
      border: 1px solid var(--border-hover);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .btn-secondary:hover {
      background: var(--surface-glass);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Check Now Button */
    .check-now-section {
      margin-top: 1.5rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border-subtle);
    }

    .check-now-btn {
      width: 100%;
      background: linear-gradient(135deg, rgba(0, 212, 255, 0.15) 0%, rgba(0, 180, 216, 0.1) 100%);
      border: 1px solid rgba(0, 212, 255, 0.25);
      color: #00d4ff;
      padding: 0.75rem 1rem;
      border-radius: var(--radius-base);
      font-size: 0.85rem;
      font-weight: 500;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      transition: all 0.2s ease;
    }

    .check-now-btn:hover {
      background: linear-gradient(135deg, rgba(0, 212, 255, 0.25) 0%, rgba(0, 180, 216, 0.2) 100%);
      transform: translateY(-1px);
    }

    /* Config button in header */
    .config-btn {
      background: transparent;
      border: 1px solid var(--border-medium);
      color: var(--color-dark-text-tertiary, #94a3b8);
      min-width: 44px;
      min-height: 44px;
      border-radius: var(--radius-base);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .config-btn:hover {
      background: var(--ctx-border-subtle);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    /* Mobile responsive */
    @media (max-width: 768px) {
      .domains-list {
        gap: 0.5rem;
      }
      .widget-header {
        flex-direction: column;
        gap: 0.5rem;
        text-align: center;
      }
      .header-right {
        justify-content: center;
      }
      .domain-card {
        padding: 0.75rem;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
      }
      .domain-info {
        width: 100%;
      }
      .expiry-info {
        align-items: flex-start;
        text-align: left;
        width: 100%;
      }
      .config-panel {
        width: 100%;
      }
    }
  `]

  constructor() {
    super()
    this.domains = []
    this.loading = true
    this.lastUpdate = null
    this.mqttConnected = false
    this.showConfig = false
    this.editingDomain = null
    this.formData = this.getEmptyFormData()
    this._mqttService = null
    this._boundSummaryHandler = this.handleSslSummary.bind(this)
    this._boundDomainHandler = this.handleSslDomain.bind(this)
  }

  getEmptyFormData() {
    return {
      hostname: '',
      port: 443,
      label: '',
      warning_days: 30,
      critical_days: 14,
      check_http: true
    }
  }

  getApiBaseUrl() {
    const protocol = window.location.protocol === 'https:' ? 'https' : 'http'
    const host = window.location.hostname
    const port = protocol === 'https' ? '8443' : '8080'
    return `${protocol}://${host}:${port}`
  }

  getAuthToken() {
    return localStorage.getItem('auth_token') || ''
  }

  connectedCallback() {
    super.connectedCallback()
    this.setupMqttWithRetry()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._mqttService) {
      this._mqttService.removeEventListener('ssl-summary', this._boundSummaryHandler)
      this._mqttService.removeEventListener('ssl-domain', this._boundDomainHandler)
    }
  }

  setupMqttWithRetry() {
    let attempts = 0
    const maxAttempts = 10
    const interval = setInterval(() => {
      attempts++
      const mqttService = document.querySelector('mqtt-service')
      if (mqttService) {
        clearInterval(interval)
        this.setupMqttListeners(mqttService)
        this.mqttConnected = true
        this.loading = false
      } else if (attempts >= maxAttempts) {
        clearInterval(interval)
        console.warn('[ssl-widget] mqtt-service not found')
        this.loading = false
      }
    }, 500)
  }

  setupMqttListeners(mqttService) {
    mqttService.addEventListener('ssl-summary', this._boundSummaryHandler)
    mqttService.addEventListener('ssl-domain', this._boundDomainHandler)
    this._mqttService = mqttService

    // Load cached data
    if (typeof mqttService.getSslCache === 'function') {
      const cache = mqttService.getSslCache()
      if (cache.summary && cache.summary.domains) {
        this.domains = cache.summary.domains
        this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
      } else if (Object.keys(cache.domains).length > 0) {
        this.domains = Object.values(cache.domains)
        this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
      }
    }
  }

  handleSslSummary(event) {
    const { payload } = event.detail
    if (payload && payload.domains) {
      this.domains = payload.domains
      this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    }
  }

  handleSslDomain(event) {
    const { payload } = event.detail
    if (payload && payload.domain_id) {
      const idx = this.domains.findIndex(d => d.domain_id === payload.domain_id)
      if (idx >= 0) {
        this.domains = [
          ...this.domains.slice(0, idx),
          payload,
          ...this.domains.slice(idx + 1)
        ]
      } else {
        this.domains = [...this.domains, payload]
      }
      this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    }
  }

  formatDays(days) {
    if (days === null || days === undefined) return '?'
    if (days < 0) return 'Expiré'
    if (days === 0) return "Auj."
    if (days === 1) return '1j'
    return `${days}j`
  }

  getStatusLevel(domain) {
    if (!domain.ssl_valid) return 'error'
    if (domain.days_remaining === null) return 'error'
    if (domain.days_remaining <= 14) return 'critical'
    if (domain.days_remaining <= 30) return 'warning'
    return 'ok'
  }

  getSummaryStatus() {
    if (this.domains.length === 0) return 'ok'
    const hasCritical = this.domains.some(d => this.getStatusLevel(d) === 'critical')
    const hasWarning = this.domains.some(d => this.getStatusLevel(d) === 'warning')
    if (hasCritical) return 'critical'
    if (hasWarning) return 'warning'
    return 'ok'
  }

  async refreshData() {
    this.loading = true

    // Re-read from MQTT cache
    if (this._mqttService && typeof this._mqttService.getSslCache === 'function') {
      const cache = this._mqttService.getSslCache()
      if (cache.summary && cache.summary.domains) {
        this.domains = cache.summary.domains
      } else if (Object.keys(cache.domains).length > 0) {
        this.domains = Object.values(cache.domains)
      }
    }

    // Small delay to show loader
    await new Promise(resolve => setTimeout(resolve, 500))
    this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    this.loading = false
  }

  openConfig() {
    // Émettre un événement pour ouvrir la page de configuration SSL complète
    this.dispatchEvent(new CustomEvent('open-ssl-config', {
      bubbles: true,
      composed: true
    }))
  }

  closeConfig() {
    this.showConfig = false
    this.editingDomain = null
    this.formData = this.getEmptyFormData()
  }

  editDomain(domain) {
    this.editingDomain = domain
    this.formData = {
      hostname: domain.hostname || '',
      port: domain.port || 443,
      label: domain.label || '',
      warning_days: domain.warning_days || 30,
      critical_days: domain.critical_days || 14,
      check_http: domain.check_http !== false
    }
  }

  cancelEdit() {
    this.editingDomain = null
    this.formData = this.getEmptyFormData()
  }

  handleInputChange(e) {
    const { name, value, type, checked } = e.target
    this.formData = {
      ...this.formData,
      [name]: type === 'checkbox' ? checked : (type === 'number' ? parseInt(value, 10) : value)
    }
  }

  updateFormField(field, value) {
    this.formData = { ...this.formData, [field]: value }
    this.requestUpdate()
  }

  async saveDomain() {
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()
    const headers = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    }

    try {
      let response
      if (this.editingDomain) {
        // Update existing domain
        response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains/${this.editingDomain.id}`, {
          method: 'PUT',
          headers,
          body: JSON.stringify(this.formData)
        })
      } else {
        // Create new domain
        response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains`, {
          method: 'POST',
          headers,
          body: JSON.stringify(this.formData)
        })
      }

      if (response.ok) {
        await this.fetchDomainsFromApi()
        this.editingDomain = null
        this.formData = this.getEmptyFormData()
        await this.triggerCheck()
      } else {
        const error = await response.text()
        console.error('[ssl-widget] Save failed:', error)
      }
    } catch (err) {
      console.error('[ssl-widget] Save error:', err)
    }
  }

  async deleteDomain(domainId) {
    if (!confirm(`Supprimer ce domaine ?`)) return

    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    try {
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains/${domainId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` }
      })

      if (response.ok) {
        await this.fetchDomainsFromApi()
      }
    } catch (err) {
      console.error('[ssl-widget] Delete error:', err)
    }
  }

  async fetchDomainsFromApi() {
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    try {
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains`, {
        headers: { 'Authorization': `Bearer ${token}` }
      })

      if (response.ok) {
        const data = await response.json()
        if (data.domains) {
          this.domains = data.domains
          this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
        }
      }
    } catch (err) {
      console.error('[ssl-widget] Fetch error:', err)
    }
  }

  async triggerCheck() {
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    try {
      await fetch(`${baseUrl}/v1/plugin-api/ssl/check`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      })
      // Wait a bit then refresh
      setTimeout(() => this.fetchDomainsFromApi(), 2000)
    } catch (err) {
      console.error('[ssl-widget] Check trigger error:', err)
    }
  }

  renderConfigPanel() {
    return html`
      <div class="config-overlay ${this.showConfig ? 'open' : ''}" @click=${() => this.closeConfig()}></div>
      <div class="config-panel ${this.showConfig ? 'open' : ''}">
        <div class="config-header">
          <div class="config-header-left">
            <div class="config-header-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
              </svg>
            </div>
            <span class="config-title">Configuration SSL</span>
          </div>
          <button class="close-btn" @click=${() => this.closeConfig()}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>

        <div class="config-content">
          <!-- Domain List Section -->
          <div class="config-section">
            <div class="section-header">
              <span class="section-title">Domaines surveillés</span>
              <span class="section-badge">${this.domains.length}</span>
            </div>

            ${this.domains.length === 0 ? html`
              <div class="empty-state">
                <div class="empty-state-icon">🔒</div>
                <div class="empty-state-text">Aucun domaine configuré</div>
              </div>
            ` : html`
              <div class="domain-list">
                ${this.domains.map(domain => html`
                  <div class="domain-card">
                    <div class="domain-card-main">
                      <div class="domain-card-name">${domain.label || domain.hostname}</div>
                      <div class="domain-card-host">${domain.hostname}:${domain.port || 443}</div>
                      <div class="domain-card-thresholds">
                        <span class="threshold-tag threshold-warning">⚠️ ${domain.warning_days || 30}j</span>
                        <span class="threshold-tag threshold-critical">🔴 ${domain.critical_days || 14}j</span>
                      </div>
                    </div>
                    <div class="domain-card-actions">
                      <button class="edit-btn" @click=${() => this.editDomain(domain)} title="Modifier">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
                        </svg>
                      </button>
                      <button class="delete-btn" @click=${() => this.deleteDomain(domain.id)} title="Supprimer">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                        </svg>
                      </button>
                    </div>
                  </div>
                `)}
              </div>
            `}
          </div>

          <!-- Add/Edit Form Section -->
          <div class="config-section">
            <div class="form-card">
              <div class="form-card-title">
                ${this.editingDomain ? html`
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
                  </svg>
                  Modifier le domaine
                ` : html`
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 5v14M5 12h14"/>
                  </svg>
                  Ajouter un domaine
                `}
              </div>

              <div class="form-group">
                <label class="form-label">Nom de domaine</label>
                <input type="text" class="form-input"
                  id="ssl-hostname"
                  .value=${this.formData.hostname}
                  @input=${(e) => this.updateFormField('hostname', e.target.value)}
                  placeholder="exemple.com">
              </div>

              <div class="form-row">
                <div class="form-group">
                  <label class="form-label">Port</label>
                  <input type="number" class="form-input"
                    id="ssl-port"
                    .value=${this.formData.port}
                    @input=${(e) => this.updateFormField('port', parseInt(e.target.value) || 443)}>
                </div>
                <div class="form-group">
                  <label class="form-label">Label</label>
                  <input type="text" class="form-input"
                    id="ssl-label"
                    .value=${this.formData.label}
                    @input=${(e) => this.updateFormField('label', e.target.value)}
                    placeholder="Mon Site">
                </div>
              </div>

              <div class="form-row">
                <div class="form-group">
                  <label class="form-label">Warning (jours)</label>
                  <input type="number" class="form-input"
                    id="ssl-warning"
                    .value=${this.formData.warning_days}
                    @input=${(e) => this.updateFormField('warning_days', parseInt(e.target.value) || 30)}>
                </div>
                <div class="form-group">
                  <label class="form-label">Critical (jours)</label>
                  <input type="number" class="form-input"
                    id="ssl-critical"
                    .value=${this.formData.critical_days}
                    @input=${(e) => this.updateFormField('critical_days', parseInt(e.target.value) || 14)}>
                </div>
              </div>

              <div class="form-actions">
                ${this.editingDomain ? html`
                  <button class="btn btn-secondary" @click=${() => this.cancelEdit()}>
                    Annuler
                  </button>
                  <button class="btn btn-primary" @click=${() => this.saveDomain()}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
                      <polyline points="17 21 17 13 7 13 7 21"/>
                      <polyline points="7 3 7 8 15 8"/>
                    </svg>
                    Enregistrer
                  </button>
                ` : html`
                  <button class="btn btn-primary" style="width: 100%;" @click=${() => this.saveDomain()} ?disabled=${!this.formData.hostname}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M12 5v14M5 12h14"/>
                    </svg>
                    Ajouter un domaine
                  </button>
                `}
              </div>
            </div>
          </div>

          <!-- Check Now Section -->
          <div class="check-now-section">
            <button class="check-now-btn" @click=${() => this.triggerCheck()}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
              </svg>
              Vérifier tous les certificats
            </button>
          </div>
        </div>
      </div>
    `
  }

  render() {
    const validCount = this.domains.filter(d => d.ssl_valid).length
    const summaryStatus = this.getSummaryStatus()

    return html`
      ${this.renderConfigPanel()}

      <div class="widget-header">
        <div class="widget-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          SSL Monitor
        </div>
        <div class="header-right">
          ${this.domains.length > 0 ? html`
            <span class="summary-badge summary-${summaryStatus}">
              ${validCount}/${this.domains.length} OK
            </span>
          ` : ''}
          <button class="config-btn" @click=${() => this.openConfig()}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
          </button>
          <button class="refresh-btn ${this.loading ? 'spinning' : ''}" @click=${() => this.refreshData()}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
            </svg>
          </button>
        </div>
      </div>

      ${this.loading ? html`
        <div class="loader-container">
          <organic-loader text="Vérification SSL..."></organic-loader>
        </div>
      ` : this.domains.length === 0 ? html`
        <div class="empty-state">En attente des données SSL...</div>
      ` : html`
        <div class="domains-list">
          ${this.domains.map(domain => {
            const status = this.getStatusLevel(domain)
            return html`
              <div class="domain-card ${status}">
                <div class="domain-info">
                  <span class="status-dot ${status}"></span>
                  <div class="domain-details">
                    <span class="domain-name">${domain.hostname}</span>
                    <span class="domain-issuer">${domain.issuer || 'SSL'}</span>
                  </div>
                </div>
                <div class="expiry-info">
                  <span class="days-remaining ${status}">${this.formatDays(domain.days_remaining)}</span>
                  ${domain.expiry_date ? html`
                    <span class="expiry-date">${domain.expiry_date}</span>
                  ` : ''}
                </div>
              </div>
            `
          })}
        </div>

        ${this.lastUpdate ? html`
          <div class="last-update">Màj: ${this.lastUpdate}</div>
        ` : ''}
      `}
    `
  }
}

customElements.define('ssl-widget', SslWidget)
