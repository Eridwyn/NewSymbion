/**
 * SSL Monitor Widget
 *
 * Displays SSL certificate status for monitored domains:
 * - Certificate validity
 * - Days until expiry
 * - Status indicators (ok/warning/critical)
 * - Online/offline status
 */

import { LitElement, html, css } from 'lit'
import { apiService } from '../services/api-service.js'

export class SslWidget extends LitElement {
  static properties = {
    domains: { type: Array },
    loading: { type: Boolean },
    error: { type: String },
    lastUpdate: { type: String },
    mqttConnected: { type: Boolean }
  }

  static styles = css`
    :host {
      display: block;
    }

    .widget-container {
      background: var(--surface-color, #1a1a2e);
      border-radius: 16px;
      padding: 20px;
      border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .widget-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
    }

    .widget-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--text-color, #fff);
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .refresh-btn {
      background: none;
      border: none;
      color: var(--text-muted, #888);
      cursor: pointer;
      padding: 4px;
      border-radius: 4px;
      transition: all 0.2s;
    }

    .refresh-btn:hover {
      color: var(--accent-color, #00d4ff);
      background: rgba(255, 255, 255, 0.1);
    }

    .refresh-btn.spinning {
      animation: spin 1s linear infinite;
    }

    @keyframes spin {
      100% { transform: rotate(360deg); }
    }

    .domains-list {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }

    .domain-card {
      background: rgba(255, 255, 255, 0.05);
      border-radius: 12px;
      padding: 14px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      transition: all 0.2s;
    }

    .domain-card:hover {
      background: rgba(255, 255, 255, 0.08);
    }

    .domain-info {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .domain-name {
      font-weight: 500;
      color: var(--text-color, #fff);
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .domain-details {
      font-size: 0.85rem;
      color: var(--text-muted, #888);
    }

    .status-badge {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 12px;
      border-radius: 20px;
      font-size: 0.85rem;
      font-weight: 500;
    }

    .status-ok {
      background: rgba(0, 200, 83, 0.2);
      color: #00c853;
    }

    .status-warning {
      background: rgba(255, 193, 7, 0.2);
      color: #ffc107;
    }

    .status-critical {
      background: rgba(244, 67, 54, 0.2);
      color: #f44336;
    }

    .status-error {
      background: rgba(158, 158, 158, 0.2);
      color: #9e9e9e;
    }

    .status-expired {
      background: rgba(156, 39, 176, 0.2);
      color: #9c27b0;
    }

    .online-indicator {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: #00c853;
    }

    .online-indicator.offline {
      background: #f44336;
    }

    .expiry-info {
      text-align: right;
    }

    .days-remaining {
      font-size: 1.2rem;
      font-weight: 600;
    }

    .days-remaining.ok { color: #00c853; }
    .days-remaining.warning { color: #ffc107; }
    .days-remaining.critical { color: #f44336; }
    .days-remaining.expired { color: #9c27b0; }

    .expiry-date {
      font-size: 0.75rem;
      color: var(--text-muted, #888);
    }

    .summary-bar {
      display: flex;
      gap: 16px;
      margin-top: 16px;
      padding-top: 16px;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .summary-item {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 0.85rem;
      color: var(--text-muted, #888);
    }

    .summary-count {
      font-weight: 600;
      color: var(--text-color, #fff);
    }

    .last-update {
      font-size: 0.75rem;
      color: var(--text-muted, #888);
      text-align: right;
      margin-top: 8px;
    }

    .error-message {
      color: #f44336;
      padding: 12px;
      background: rgba(244, 67, 54, 0.1);
      border-radius: 8px;
      font-size: 0.9rem;
    }

    .empty-state {
      text-align: center;
      padding: 32px;
      color: var(--text-muted, #888);
    }
  `

  constructor() {
    super()
    this.domains = []
    this.loading = true
    this.error = null
    this.lastUpdate = null
    this.mqttConnected = false
    this._mqttService = null
    this._boundSummaryHandler = this.handleSslSummary.bind(this)
    this._boundDomainHandler = this.handleSslDomain.bind(this)
  }

  connectedCallback() {
    super.connectedCallback()
    // Retry finding mqtt-service (may not exist immediately)
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
        console.warn('[ssl-widget] mqtt-service not found after retries')
        this.loading = false
      }
    }, 500)
  }

  setupMqttListeners(mqttService) {
    console.log('[ssl-widget] Setting up event listeners on mqtt-service')

    mqttService.addEventListener('ssl-summary', this._boundSummaryHandler)
    mqttService.addEventListener('ssl-domain', this._boundDomainHandler)

    this._mqttService = mqttService

    // Load cached data (retained messages that arrived before we subscribed)
    if (typeof mqttService.getSslCache === 'function') {
      const cache = mqttService.getSslCache()
      if (cache.summary && cache.summary.domains) {
        this.domains = cache.summary.domains
        this.lastUpdate = new Date().toLocaleTimeString()
      } else if (Object.keys(cache.domains).length > 0) {
        this.domains = Object.values(cache.domains)
        this.lastUpdate = new Date().toLocaleTimeString()
      }
    }
  }

  handleSslSummary(event) {
    const { payload } = event.detail
    if (payload && payload.domains) {
      this.domains = payload.domains
      this.lastUpdate = new Date().toLocaleTimeString()
    }
  }

  handleSslDomain(event) {
    const { domainId, payload } = event.detail
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
      this.lastUpdate = new Date().toLocaleTimeString()
    }
  }

  async refresh() {
    this.loading = true
    // No API endpoint yet, just rely on MQTT
    setTimeout(() => {
      this.loading = false
    }, 500)
  }

  getStatusIcon(level) {
    switch (level) {
      case 'ok': return '✓'
      case 'warning': return '⚠'
      case 'critical': return '⚠'
      case 'expired': return '✗'
      case 'error': return '?'
      default: return '•'
    }
  }

  formatDaysRemaining(days) {
    if (days === null || days === undefined) return '?'
    if (days < 0) return 'Expiré'
    if (days === 0) return "Aujourd'hui"
    if (days === 1) return '1 jour'
    return `${days} jours`
  }

  render() {
    const validCount = this.domains.filter(d => d.ssl_valid).length
    const expiringCount = this.domains.filter(d =>
      d.days_remaining !== null && d.days_remaining <= 30 && d.days_remaining > 0
    ).length
    const criticalCount = this.domains.filter(d =>
      d.days_remaining !== null && d.days_remaining <= 14
    ).length

    return html`
      <div class="widget-container">
        <div class="widget-header">
          <div class="widget-title">
            🔒 SSL Monitor
          </div>
          <button
            class="refresh-btn ${this.loading ? 'spinning' : ''}"
            @click=${this.refresh}
            ?disabled=${this.loading}
          >
            ↻
          </button>
        </div>

        ${this.error ? html`
          <div class="error-message">${this.error}</div>
        ` : ''}

        ${this.domains.length === 0 && !this.loading ? html`
          <div class="empty-state">
            En attente des données SSL...
          </div>
        ` : ''}

        <div class="domains-list">
          ${this.domains.map(domain => html`
            <div class="domain-card">
              <div class="domain-info">
                <div class="domain-name">
                  <span class="online-indicator ${domain.online ? '' : 'offline'}"></span>
                  ${domain.hostname}
                </div>
                <div class="domain-details">
                  ${domain.issuer ? `Émetteur: ${domain.issuer}` : 'SSL'}
                </div>
              </div>

              <div class="expiry-info">
                <div class="days-remaining ${domain.status_level}">
                  ${this.formatDaysRemaining(domain.days_remaining)}
                </div>
                ${domain.expiry_date ? html`
                  <div class="expiry-date">
                    Expire: ${domain.expiry_date}
                  </div>
                ` : ''}
              </div>

              <div class="status-badge status-${domain.status_level}">
                ${this.getStatusIcon(domain.status_level)}
              </div>
            </div>
          `)}
        </div>

        ${this.domains.length > 0 ? html`
          <div class="summary-bar">
            <div class="summary-item">
              <span class="summary-count">${validCount}/${this.domains.length}</span>
              valides
            </div>
            ${expiringCount > 0 ? html`
              <div class="summary-item">
                <span class="summary-count" style="color: #ffc107">${expiringCount}</span>
                expiration proche
              </div>
            ` : ''}
            ${criticalCount > 0 ? html`
              <div class="summary-item">
                <span class="summary-count" style="color: #f44336">${criticalCount}</span>
                critique
              </div>
            ` : ''}
          </div>
        ` : ''}

        ${this.lastUpdate ? html`
          <div class="last-update">
            Dernière mise à jour: ${this.lastUpdate}
          </div>
        ` : ''}
      </div>
    `
  }
}

customElements.define('ssl-widget', SslWidget)
