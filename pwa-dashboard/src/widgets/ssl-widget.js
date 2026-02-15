/**
 * SSL Monitor Widget
 *
 * Affiche le statut des certificats SSL des domaines surveillés
 * Mise à jour temps réel via MQTT
 */

import { LitElement, html, css } from 'lit'

export class SslWidget extends LitElement {
  static properties = {
    domains: { type: Array },
    loading: { type: Boolean },
    lastUpdate: { type: String },
    mqttConnected: { type: Boolean }
  }

  static styles = css`
    :host {
      display: block;
    }

    .widget-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }

    .widget-title {
      font-size: 1.2em;
      font-weight: 600;
      color: #e0e0e0;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .header-right {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .summary-badge {
      padding: 0.4rem 0.8rem;
      border-radius: 20px;
      font-size: 0.75em;
      font-weight: 600;
      letter-spacing: 0.5px;
    }

    .summary-ok {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.25) 0%, rgba(34, 197, 94, 0.2) 100%);
      color: #00d4aa;
      border: 1px solid rgba(0, 212, 170, 0.4);
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
      border: 1px solid rgba(255, 255, 255, 0.15);
      color: #888;
      width: 32px;
      height: 32px;
      border-radius: 8px;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .refresh-btn:hover {
      background: rgba(255, 255, 255, 0.08);
      color: #00d4ff;
      border-color: rgba(0, 212, 255, 0.3);
    }

    .refresh-btn.spinning svg {
      animation: spin 1s linear infinite;
    }

    @keyframes spin {
      100% { transform: rotate(360deg); }
    }

    .domains-list {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
    }

    .domain-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.06) 0%, rgba(255, 255, 255, 0.02) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 12px;
      padding: 1rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      transition: all 0.2s ease;
    }

    .domain-card:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border-color: rgba(255, 255, 255, 0.15);
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

    .status-dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      flex-shrink: 0;
    }

    .status-dot.ok {
      background: #00d4aa;
      box-shadow: 0 0 8px rgba(0, 212, 170, 0.5);
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
      color: #e0e0e0;
      font-size: 0.95em;
      font-weight: 500;
    }

    .domain-issuer {
      color: #666;
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

    .days-remaining.ok { color: #00d4aa; }
    .days-remaining.warning { color: #fbbf24; }
    .days-remaining.critical { color: #ff6b6b; }
    .days-remaining.error { color: #666; }

    .expiry-date {
      font-size: 0.7em;
      color: #666;
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: #666;
      font-size: 0.9em;
    }

    .last-update {
      margin-top: 1rem;
      padding-top: 0.75rem;
      border-top: 1px solid rgba(255, 255, 255, 0.06);
      font-size: 0.7em;
      color: #555;
      text-align: right;
    }
  `

  constructor() {
    super()
    this.domains = []
    this.loading = true
    this.lastUpdate = null
    this.mqttConnected = false
    this._mqttService = null
    this._boundSummaryHandler = this.handleSslSummary.bind(this)
    this._boundDomainHandler = this.handleSslDomain.bind(this)
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

  render() {
    const validCount = this.domains.filter(d => d.ssl_valid).length
    const summaryStatus = this.getSummaryStatus()

    return html`
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
          <button class="refresh-btn ${this.loading ? 'spinning' : ''}" @click=${() => this.requestUpdate()}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
            </svg>
          </button>
        </div>
      </div>

      ${this.domains.length === 0 && !this.loading ? html`
        <div class="empty-state">En attente des données SSL...</div>
      ` : ''}

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
    `
  }
}

customElements.define('ssl-widget', SslWidget)
