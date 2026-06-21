/**
 * Widget Synology UPS — onduleur via NUT
 *
 * Affiche l'état de l'onduleur du NAS Synology en temps réel :
 * statut secteur/batterie, niveau de charge, autonomie, charge.
 * MQTT temps réel (symbion/synology/ups) + fallback HTTP polling.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import { getApiBase } from '../services/config.js'
import '../components/organic-loader.js'

class SynologyWidget extends LitElement {
  static properties = {
    ups: { type: Object },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean },
    lastUpdate: { type: String }
  }

  static styles = [sharedAnimations, widgetHeaderStyles, emptyStateStyles, statusBadgeStyles, statusDotStyles, css`
    :host { display: block; }

    /* ── Battery gauge ── */
    .battery-block {
      display: flex;
      align-items: center;
      gap: 1rem;
      padding: 1rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      margin-bottom: 0.75rem;
    }

    .battery-icon {
      position: relative;
      width: 52px;
      height: 26px;
      border: 2px solid var(--color-dark-text-secondary, #cbd5e1);
      border-radius: 4px;
      flex-shrink: 0;
    }

    .battery-icon::after {
      content: '';
      position: absolute;
      right: -5px;
      top: 7px;
      width: 3px;
      height: 10px;
      background: var(--color-dark-text-secondary, #cbd5e1);
      border-radius: 0 2px 2px 0;
    }

    .battery-fill {
      position: absolute;
      left: 2px;
      top: 2px;
      bottom: 2px;
      border-radius: 2px;
      transition: width 0.6s ease, background 0.3s ease;
    }

    .battery-fill.ok { background: var(--context-primary, #00d4aa); }
    .battery-fill.warning { background: #fbbf24; }
    .battery-fill.critical {
      background: #ff6b6b;
      animation: pulse-batt 1.5s ease-in-out infinite;
    }

    @keyframes pulse-batt {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.6; }
    }

    .battery-meta {
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
    }

    .battery-pct {
      font-size: 1.5em;
      font-weight: 700;
      line-height: 1;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .battery-pct.warning { color: #fbbf24; }
    .battery-pct.critical { color: #ff6b6b; }

    .battery-runtime {
      font-size: 0.78em;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    /* ── Metrics grid ── */
    .metrics-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .metric-chip {
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
      padding: 0.55rem 0.7rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-base);
    }

    .metric-label {
      font-size: 0.7em;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .metric-value {
      font-size: 0.95em;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* ── Footer ── */
    .footer-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-top: 0.5rem;
      border-top: 1px solid var(--border-subtle);
      font-size: 0.72em;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .device-id {
      display: flex;
      align-items: center;
      gap: 0.35rem;
    }

    .loader-container {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 160px;
      padding: 2rem;
    }

    @container widget (max-width: 320px) {
      .metrics-grid { grid-template-columns: 1fr; }
    }
  `]

  constructor() {
    super()
    this.ups = null
    this.loading = true
    this.mqttConnected = false
    this.lastUpdate = null
    this._mqttService = null
    this._boundUpsHandler = (e) => this._handleUps(e.detail)
  }

  connectedCallback() {
    super.connectedCallback()
    this._retrySetup()
    // Filet de sécurité : sortir du loader même sans données après 10s
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) { this.loading = false; this.requestUpdate() }
    }, 10000)
    this._fetchStatus()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._mqttService) {
      this._mqttService.removeEventListener('synology-ups', this._boundUpsHandler)
    }
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    if (this._refreshInterval) clearInterval(this._refreshInterval)
  }

  // ── MQTT ──

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      mqttService.addEventListener('synology-ups', this._boundUpsHandler)
      this._mqttService = mqttService
      this.mqttConnected = true
      // Charger le cache (topic retained, widget peut être monté tard)
      if (typeof mqttService.getSynologyCache === 'function') {
        const cache = mqttService.getSynologyCache()
        if (cache.ups) this._applyUps(cache.ups)
      }
    } else if (attempts < 10) {
      setTimeout(() => this._retrySetup(attempts + 1), 500)
    } else {
      this.loading = false
      this.requestUpdate()
    }
  }

  _handleUps({ payload }) {
    this.mqttConnected = true
    this._applyUps(payload)
  }

  _applyUps(payload) {
    this.ups = payload
    this.loading = false
    this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    this.requestUpdate()
  }

  // ── HTTP fallback ──

  async _fetchStatus() {
    await this._fetchStatusSilent()
    // Poll de secours si MQTT muet (le plugin publie ~toutes les 30s)
    this._refreshInterval = setInterval(() => this._fetchStatusSilent(), 30000)
  }

  async _fetchStatusSilent() {
    try {
      const token = localStorage.getItem('auth_token') || ''
      const resp = await fetch(`${getApiBase()}/v1/plugin-api/synology/ups`, {
        headers: token ? { 'Authorization': `Bearer ${token}` } : {}
      })
      if (resp.ok) {
        this._applyUps(await resp.json())
      }
    } catch (e) {
      // MQTT reste la source primaire — on n'alerte pas sur un échec de fallback
      console.warn('[synology-widget] HTTP fallback failed:', e.message)
    }
  }

  // ── Helpers ──

  _statusLevel() {
    if (!this.ups) return 'offline'
    if (this.ups.on_battery && this.ups.battery_low) return 'critical'
    if (this.ups.on_battery) return 'warning'
    return 'ok'
  }

  _statusLabel() {
    switch (this._statusLevel()) {
      case 'critical': return 'Batterie faible'
      case 'warning': return 'Sur batterie'
      case 'ok': return 'Sur secteur'
      default: return 'Hors ligne'
    }
  }

  _batteryClass() {
    const pct = this.ups?.battery_charge ?? 0
    if (this.ups?.battery_low || pct <= 20) return 'critical'
    if (pct <= 50) return 'warning'
    return 'ok'
  }

  _formatRuntime(sec) {
    if (sec === null || sec === undefined) return '—'
    if (sec <= 0) return '0s'
    const h = Math.floor(sec / 3600)
    const m = Math.floor((sec % 3600) / 60)
    if (h > 0) return `${h}h${String(m).padStart(2, '0')}`
    if (m > 0) return `${m}min`
    return `${sec}s`
  }

  _num(val, suffix = '', digits = 0) {
    if (val === null || val === undefined || Number.isNaN(Number(val))) return '—'
    return `${Number(val).toFixed(digits)}${suffix}`
  }

  // ── Render ──

  render() {
    if (this.loading) {
      return html`
        <div class="loader-container">
          <organic-loader text="Connexion onduleur..."></organic-loader>
        </div>
      `
    }

    const level = this._statusLevel()
    const badgeClass = level === 'ok' ? 'ok' : level === 'warning' ? 'warning' : level === 'critical' ? 'critical' : 'offline'
    const pct = Math.max(0, Math.min(100, this.ups?.battery_charge ?? 0))
    const battClass = this._batteryClass()

    return html`
      <div class="widget-header">
        <span class="widget-title">Onduleur</span>
        <span class="status-badge ${badgeClass}">
          <span class="status-dot ${level === 'ok' ? 'ok' : level}"></span>
          ${this._statusLabel()}
        </span>
      </div>

      ${!this.ups ? html`
        <div class="empty-state">En attente des données onduleur...</div>
      ` : html`
        <div class="battery-block">
          <div class="battery-icon">
            <div class="battery-fill ${battClass}" style="width: calc(${pct}% - 4px)"></div>
          </div>
          <div class="battery-meta">
            <span class="battery-pct ${battClass === 'ok' ? '' : battClass}">${this._num(pct, '%')}</span>
            <span class="battery-runtime">Autonomie : ${this._formatRuntime(this.ups.battery_runtime_seconds)}</span>
          </div>
        </div>

        <div class="metrics-grid">
          <div class="metric-chip">
            <span class="metric-label">Charge</span>
            <span class="metric-value">${this._num(this.ups.load_percent, '%')}</span>
          </div>
          <div class="metric-chip">
            <span class="metric-label">Tension sortie</span>
            <span class="metric-value">${this._num(this.ups.output_voltage, ' V')}</span>
          </div>
        </div>

        <div class="footer-row">
          <span class="device-id">
            <span class="status-dot ${this.mqttConnected ? 'ok' : 'offline'}"></span>
            ${this.ups.manufacturer || this.ups.model ? `${this.ups.manufacturer || ''} ${this.ups.model || ''}`.trim() : 'UPS'}
          </span>
          ${this.lastUpdate ? html`<span>Màj : ${this.lastUpdate}</span>` : ''}
        </div>
      `}
    `
  }
}

customElements.define('synology-widget', SynologyWidget)

export { SynologyWidget }
