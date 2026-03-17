/**
 * Widget Coffee Machine
 *
 * Affiche le statut de la machine a cafe Philips EP2520/10
 * et permet de lancer des boissons depuis le dashboard.
 * Mise a jour temps reel via MQTT.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import '../components/organic-loader.js'

class CoffeeWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, statusBadgeStyles, statusDotStyles, css`
    :host {
      display: block;
    }

    .content-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 1rem;
    }

    .section-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 1rem;
    }

    .section-card.full-width {
      grid-column: 1 / -1;
    }

    /* Levels bars */
    .levels-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 0.75rem;
    }

    .level-item {
      display: flex;
      flex-direction: column;
      gap: 0.35rem;
    }

    .level-label {
      font-size: 0.8em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      display: flex;
      align-items: center;
      gap: 0.4rem;
    }

    .level-bar {
      height: 6px;
      background: var(--surface-glass, rgba(255, 255, 255, 0.08));
      border-radius: 3px;
      overflow: hidden;
    }

    .level-fill {
      height: 100%;
      border-radius: 3px;
      transition: width 0.6s ease;
    }

    .level-fill.water { background: #38bdf8; }
    .level-fill.beans { background: #a78bfa; }
    .level-fill.waste { background: #fb923c; }
    .level-fill.descale { background: #34d399; }

    .level-fill.low { background: #ef4444; }

    .level-value {
      font-size: 0.75em;
      color: var(--color-dark-text-secondary, #cbd5e1);
      text-align: right;
    }

    /* Brew buttons */
    .brew-buttons {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .brew-btn {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.65rem 0.85rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-base);
      color: var(--color-dark-text-primary, #f8f9fa);
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.9em;
    }

    .brew-btn:hover:not(:disabled) {
      background: var(--surface-glass-hover, rgba(255, 255, 255, 0.08));
      border-color: var(--context-primary);
      transform: translateY(-1px);
    }

    .brew-btn:active:not(:disabled) {
      transform: translateY(0);
    }

    .brew-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }

    .brew-btn .icon {
      font-size: 1.3em;
    }

    .brew-btn .label {
      flex: 1;
    }

    .brew-btn .shortcut {
      font-size: 0.75em;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    /* Status info */
    .status-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.4rem 0;
      border-bottom: 1px solid var(--border-subtle);
    }

    .status-row:last-child {
      border-bottom: none;
    }

    .status-key {
      font-size: 0.85em;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .status-val {
      font-size: 0.85em;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .status-val.brewing {
      color: var(--context-primary);
      animation: pulse-text 1.5s ease-in-out infinite;
    }

    @keyframes pulse-text {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.6; }
    }

    /* Maintenance alert */
    .maintenance-alert {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.6rem 0.75rem;
      background: rgba(251, 146, 60, 0.15);
      border: 1px solid rgba(251, 146, 60, 0.3);
      border-radius: var(--radius-base);
      margin-top: 0.5rem;
      font-size: 0.85em;
      color: #fb923c;
    }

    /* Stop button */
    .stop-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      padding: 0.65rem;
      background: rgba(239, 68, 68, 0.15);
      border: 1px solid rgba(239, 68, 68, 0.4);
      border-radius: var(--radius-base);
      color: #ef4444;
      cursor: pointer;
      font-size: 0.9em;
      font-weight: 600;
      transition: all 0.2s ease;
      width: 100%;
    }

    .stop-btn:hover {
      background: rgba(239, 68, 68, 0.25);
    }

    /* Power button */
    .power-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-top: 0.5rem;
    }

    .power-btn {
      padding: 0.4rem 0.8rem;
      border-radius: var(--radius-base);
      border: 1px solid var(--border-subtle);
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      color: var(--color-dark-text-primary, #f8f9fa);
      cursor: pointer;
      font-size: 0.8em;
      transition: all 0.2s ease;
    }

    .power-btn:hover {
      border-color: var(--context-primary);
    }

    .power-btn.on {
      background: rgba(52, 211, 153, 0.15);
      border-color: rgba(52, 211, 153, 0.4);
      color: #34d399;
    }

    .power-btn.off {
      background: rgba(239, 68, 68, 0.15);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ef4444;
    }

    /* Brewing progress */
    .brew-progress {
      margin-top: 0.5rem;
    }

    .brew-progress-bar {
      height: 4px;
      background: var(--surface-glass, rgba(255, 255, 255, 0.08));
      border-radius: 2px;
      overflow: hidden;
    }

    .brew-progress-fill {
      height: 100%;
      background: var(--context-primary);
      border-radius: 2px;
      transition: width 1s ease;
      animation: brew-glow 2s ease-in-out infinite;
    }

    @keyframes brew-glow {
      0%, 100% { box-shadow: 0 0 4px var(--context-primary); }
      50% { box-shadow: 0 0 12px var(--context-primary); }
    }

    .brew-progress-label {
      font-size: 0.75em;
      color: var(--context-primary);
      text-align: center;
      margin-top: 0.25rem;
    }

    .loader-container {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 200px;
      padding: 2rem;
    }

    .error-msg {
      font-size: 0.8em;
      color: #ef4444;
      margin-top: 0.5rem;
      padding: 0.4rem 0.6rem;
      background: rgba(239, 68, 68, 0.1);
      border-radius: var(--radius-base);
    }

    @media (max-width: 768px) {
      .content-grid {
        grid-template-columns: 1fr;
      }
    }
  `]

  static properties = {
    status: { type: Object },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean },
    brewError: { type: String },
    brewLoading: { type: Boolean }
  }

  constructor() {
    super()
    this.status = null
    this.loading = true
    this.mqttConnected = false
    this.brewError = null
    this.brewLoading = false
    this._mqttSubscriptions = []
  }

  connectedCallback() {
    super.connectedCallback()
    this._retrySetup()
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) {
        console.warn('[coffee-widget] Timeout waiting for data, stopping loader')
        this.loading = false
        this.requestUpdate()
      }
    }, 10000)
    // Also fetch via HTTP as fallback
    this._fetchStatus()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    this._cleanupEventListeners()
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    if (this._refreshInterval) clearInterval(this._refreshInterval)
  }

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      this._setupEventListeners()
    } else if (attempts < 10) {
      setTimeout(() => this._retrySetup(attempts + 1), 500)
    } else {
      console.warn('[coffee-widget] MQTT service not found after retries')
      this.loading = false
      this.requestUpdate()
    }
  }

  _setupEventListeners() {
    const mqttService = document.querySelector('mqtt-service')
    if (!mqttService) return

    this._boundStatusHandler = (e) => {
      this._handleStatusEvent(e.detail)
    }
    this._boundBrewingHandler = (e) => {
      this._handleBrewingEvent(e.detail)
    }
    this._boundMaintenanceHandler = (e) => {
      this._handleMaintenanceEvent(e.detail)
    }

    mqttService.addEventListener('coffee-status', this._boundStatusHandler)
    mqttService.addEventListener('coffee-brewing', this._boundBrewingHandler)
    mqttService.addEventListener('coffee-maintenance', this._boundMaintenanceHandler)
    this._mqttService = mqttService

    // Load cached data
    if (typeof mqttService.getCoffeeCache === 'function') {
      const cache = mqttService.getCoffeeCache()
      if (cache.status) {
        this.status = cache.status
        this.loading = false
        this.mqttConnected = true
        if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
        this.requestUpdate()
      }
    }
  }

  _cleanupEventListeners() {
    if (this._mqttService) {
      this._mqttService.removeEventListener('coffee-status', this._boundStatusHandler)
      this._mqttService.removeEventListener('coffee-brewing', this._boundBrewingHandler)
      this._mqttService.removeEventListener('coffee-maintenance', this._boundMaintenanceHandler)
    }
  }

  _handleStatusEvent({ payload }) {
    this.status = payload
    this.loading = false
    this.mqttConnected = true
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    this.requestUpdate()
  }

  _handleBrewingEvent({ payload }) {
    // Update brewing status from event
    if (this.status) {
      if (payload.event_type === 'brewing/started') {
        this.status = { ...this.status, brewing: true, mainstate: 3, mainstate_text: 'brewing' }
      } else if (payload.event_type === 'brewing/completed') {
        this.status = { ...this.status, brewing: false, mainstate: 2, mainstate_text: 'ready', brew_progress: 0 }
      }
    }
    this.requestUpdate()
  }

  _handleMaintenanceEvent({ payload }) {
    if (this.status) {
      this.status = { ...this.status, maintenance_needed: true, maintenance_reason: payload.payload?.reason || 'maintenance' }
    }
    this.requestUpdate()
  }

  async _fetchStatus() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const resp = await fetch(`${apiService.baseUrl || ''}/v1/plugin-api/coffee/status`)
      if (resp.ok) {
        this.status = await resp.json()
        this.loading = false
        if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
        this.requestUpdate()
      }
    } catch (e) {
      console.warn('[coffee-widget] HTTP fallback failed:', e)
    }

    // Refresh every 15s via HTTP
    this._refreshInterval = setInterval(() => this._fetchStatusSilent(), 15000)
  }

  async _fetchStatusSilent() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const resp = await fetch(`${apiService.baseUrl || ''}/v1/plugin-api/coffee/status`)
      if (resp.ok) {
        this.status = await resp.json()
        this.requestUpdate()
      }
    } catch (_) { /* silent */ }
  }

  async _brew(drink) {
    this.brewError = null
    this.brewLoading = true
    this.requestUpdate()

    try {
      const apiService = document.querySelector('api-service')
      const baseUrl = apiService?.baseUrl || ''
      const resp = await fetch(`${baseUrl}/v1/plugin-api/coffee/brew`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ drink, temperature: 2, cups: 1 })
      })
      const data = await resp.json()
      if (!resp.ok) {
        this.brewError = data.error || `Erreur ${resp.status}`
      }
    } catch (e) {
      this.brewError = `Erreur connexion: ${e.message}`
    } finally {
      this.brewLoading = false
      this.requestUpdate()
      // Clear error after 5s
      if (this.brewError) {
        setTimeout(() => { this.brewError = null; this.requestUpdate() }, 5000)
      }
    }
  }

  async _stop() {
    try {
      const apiService = document.querySelector('api-service')
      const baseUrl = apiService?.baseUrl || ''
      await fetch(`${baseUrl}/v1/plugin-api/coffee/stop`, { method: 'POST' })
    } catch (e) {
      console.error('[coffee-widget] Stop failed:', e)
    }
  }

  async _power(on) {
    try {
      const apiService = document.querySelector('api-service')
      const baseUrl = apiService?.baseUrl || ''
      await fetch(`${baseUrl}/v1/plugin-api/coffee/power`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ on })
      })
    } catch (e) {
      console.error('[coffee-widget] Power failed:', e)
    }
  }

  _getOverallStatus() {
    if (!this.status) return 'unknown'
    if (!this.status.online) return 'away'
    if (this.status.brewing) return 'home'
    if (this.status.maintenance_needed) return 'unknown'
    if (this.status.mainstate === 2) return 'home'
    return 'unknown'
  }

  _getStatusLabel() {
    if (!this.status) return 'Hors ligne'
    if (!this.status.online) return 'Hors ligne'
    if (this.status.brewing) return 'En cours...'
    if (this.status.mainstate === 2) return 'Prete'
    if (this.status.mainstate === 1) return 'Veille'
    if (this.status.mainstate === 5) return 'Maintenance'
    return this.status.mainstate_text || 'Inconnu'
  }

  _levelPercent(val, max = 15) {
    if (!val && val !== 0) return 0
    return Math.min(100, Math.round((val / max) * 100))
  }

  _isReady() {
    return this.status?.online && this.status?.mainstate === 2 && !this.status?.brewing
  }

  render() {
    if (this.loading) {
      return html`
        <div class="loader-container">
          <organic-loader text="Connexion cafetiere..."></organic-loader>
        </div>
      `
    }

    const overallStatus = this._getOverallStatus()
    const isReady = this._isReady()
    const isBrewing = this.status?.brewing

    return html`
      <div class="widget-header">
        <span class="widget-title">Cafetiere</span>
        <span class="status-badge ${overallStatus}">${this._getStatusLabel()}</span>
      </div>

      <div class="content-grid">
        <!-- Brew controls -->
        <div class="section-card">
          <div class="section-title">
            <svg viewBox="0 0 24 24"><path d="M2 21V19H20V21H2ZM20 8V5H18V8H20ZM20 3C20.5523 3 21 3.44772 21 4V9C21 9.55228 20.5523 10 20 10H18V13C18 14.6569 16.6569 16 15 16H7C5.34315 16 4 14.6569 4 13V3H20ZM16 5H6V13C6 13.5523 6.44772 14 7 14H15C15.5523 14 16 13.5523 16 13V5Z"/></svg>
            Boissons
          </div>

          ${isBrewing ? html`
            <button class="stop-btn" @click="${this._stop}">Arreter</button>
            ${this.status?.brew_progress ? html`
              <div class="brew-progress">
                <div class="brew-progress-bar">
                  <div class="brew-progress-fill" style="width: ${this.status.brew_progress}%"></div>
                </div>
                <div class="brew-progress-label">Preparation ${this.status.brew_progress}%</div>
              </div>
            ` : ''}
          ` : html`
            <div class="brew-buttons">
              <button class="brew-btn" ?disabled="${!isReady || this.brewLoading}" @click="${() => this._brew('espresso')}">
                <span class="icon">&#9749;</span>
                <span class="label">Espresso</span>
              </button>
              <button class="brew-btn" ?disabled="${!isReady || this.brewLoading}" @click="${() => this._brew('coffee')}">
                <span class="icon">&#9749;</span>
                <span class="label">Cafe long</span>
              </button>
              <button class="brew-btn" ?disabled="${!isReady || this.brewLoading}" @click="${() => this._brew('hot_water')}">
                <span class="icon">&#128167;</span>
                <span class="label">Eau chaude</span>
              </button>
            </div>
          `}

          ${this.brewError ? html`<div class="error-msg">${this.brewError}</div>` : ''}

          <div class="power-row">
            <span class="status-key">Alimentation</span>
            ${this.status?.online ? html`
              <button class="power-btn off" @click="${() => this._power(false)}">Eteindre</button>
            ` : html`
              <button class="power-btn on" @click="${() => this._power(true)}">Allumer</button>
            `}
          </div>
        </div>

        <!-- Levels -->
        <div class="section-card">
          <div class="section-title">
            <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/></svg>
            Niveaux
          </div>

          <div class="levels-grid">
            <div class="level-item">
              <div class="level-label">&#128167; Eau</div>
              <div class="level-bar">
                <div class="level-fill water ${this._levelPercent(this.status?.water_level) < 20 ? 'low' : ''}"
                     style="width: ${this._levelPercent(this.status?.water_level)}%"></div>
              </div>
              <div class="level-value">${this._levelPercent(this.status?.water_level)}%</div>
            </div>

            <div class="level-item">
              <div class="level-label">&#127793; Grains</div>
              <div class="level-bar">
                <div class="level-fill beans ${this._levelPercent(this.status?.bean_level) < 20 ? 'low' : ''}"
                     style="width: ${this._levelPercent(this.status?.bean_level)}%"></div>
              </div>
              <div class="level-value">${this._levelPercent(this.status?.bean_level)}%</div>
            </div>

            <div class="level-item">
              <div class="level-label">&#128465; Marc</div>
              <div class="level-bar">
                <div class="level-fill waste" style="width: ${this._levelPercent(this.status?.waste_bean, 14)}%"></div>
              </div>
              <div class="level-value">${this.status?.waste_bean || 0}/14</div>
            </div>

            <div class="level-item">
              <div class="level-label">&#128167; Detartrage</div>
              <div class="level-bar">
                <div class="level-fill descale ${this._levelPercent(this.status?.descale_status, 8) < 25 ? 'low' : ''}"
                     style="width: ${this._levelPercent(this.status?.descale_status, 8)}%"></div>
              </div>
              <div class="level-value">${this.status?.descale_status || 0}/8</div>
            </div>
          </div>

          ${this.status?.maintenance_needed ? html`
            <div class="maintenance-alert">
              &#9888; ${this.status.maintenance_reason || 'Maintenance requise'}
            </div>
          ` : ''}
        </div>
      </div>
    `
  }
}

customElements.define('coffee-widget', CoffeeWidget)

export { CoffeeWidget }
