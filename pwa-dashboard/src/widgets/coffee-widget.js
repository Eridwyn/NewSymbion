/**
 * Widget Coffee Machine — Philips EP2520/10
 *
 * Dashboard widget with real-time status, brew controls,
 * auto power-on sequence, level indicators and toast feedback.
 * MQTT real-time + HTTP polling fallback.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import { getApiBase } from '../services/config.js'
import csrfService from '../services/csrf-service.js'
import '../components/organic-loader.js'

class CoffeeWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, statusBadgeStyles, statusDotStyles, css`
    :host {
      display: block;
    }

    /* ── Drink List ── */
    .drink-list {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .drink-row {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 0.55rem 0.7rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      transition: all 0.2s ease;
    }

    .drink-row.disabled {
      opacity: 0.35;
    }

    .drink-icon {
      font-size: 1.4em;
      flex-shrink: 0;
    }

    .drink-name {
      font-size: 0.88em;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
    }

    /* Cups toggle */
    .cups-toggle {
      display: flex;
      background: var(--surface-glass, rgba(255, 255, 255, 0.06));
      border-radius: var(--radius-base);
      border: 1px solid var(--border-subtle);
      overflow: hidden;
      flex-shrink: 0;
    }

    .cups-toggle .cup-opt {
      padding: 0.25rem 0.55rem;
      font-size: 0.75em;
      font-weight: 600;
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      transition: all 0.2s ease;
      border: none;
      background: transparent;
      user-select: none;
    }

    .cups-toggle .cup-opt:first-child {
      border-right: 1px solid var(--border-subtle);
    }

    .cups-toggle .cup-opt.active {
      background: var(--context-primary);
      color: #fff;
    }

    .cups-toggle .cup-opt:hover:not(.active) {
      background: rgba(255, 255, 255, 0.06);
    }

    /* Go button */
    .go-btn {
      padding: 0.3rem 0.65rem;
      border-radius: var(--radius-base);
      border: 1px solid var(--context-primary);
      background: rgba(var(--ctx-rgb, 0,212,170), 0.15);
      color: var(--context-primary);
      cursor: pointer;
      font-size: 0.78em;
      font-weight: 700;
      transition: all 0.2s ease;
      flex-shrink: 0;
    }

    .go-btn:hover:not(:disabled) {
      background: rgba(var(--ctx-rgb, 0,212,170), 0.25);
      transform: scale(1.05);
    }

    .go-btn:disabled {
      opacity: 0.35;
      cursor: not-allowed;
    }

    /* ── Brewing State ── */
    .brewing-state {
      text-align: center;
      padding: 1rem;
      background: linear-gradient(135deg, rgba(var(--ctx-rgb, 0,212,170), 0.08) 0%, transparent 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      margin-bottom: 0.75rem;
    }

    .brewing-icon {
      font-size: 2.5em;
      animation: steam 2s ease-in-out infinite;
    }

    @keyframes steam {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-4px); }
    }

    .brewing-label {
      font-size: 0.95em;
      font-weight: 600;
      color: var(--context-primary);
      margin: 0.5rem 0;
    }

    .brewing-progress {
      height: 6px;
      background: var(--surface-glass, rgba(255, 255, 255, 0.08));
      border-radius: 3px;
      overflow: hidden;
      margin: 0.5rem 0;
    }

    .brewing-progress-fill {
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary), var(--context-secondary, var(--context-primary)));
      border-radius: 3px;
      transition: width 1s ease;
      position: relative;
    }

    .brewing-progress-fill::after {
      content: '';
      position: absolute;
      inset: 0;
      background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
      animation: shimmer 1.5s ease-in-out infinite;
    }

    @keyframes shimmer {
      0% { transform: translateX(-100%); }
      100% { transform: translateX(100%); }
    }

    .stop-btn {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.45rem 1rem;
      background: rgba(239, 68, 68, 0.15);
      border: 1px solid rgba(239, 68, 68, 0.4);
      border-radius: var(--radius-base);
      color: #ef4444;
      cursor: pointer;
      font-size: 0.85em;
      font-weight: 600;
      transition: all 0.2s ease;
      margin-top: 0.25rem;
    }

    .stop-btn:hover {
      background: rgba(239, 68, 68, 0.25);
    }

    /* ── Power-on Sequence ── */
    .powering-on {
      text-align: center;
      padding: 1.5rem;
    }

    .powering-on .spin-icon {
      font-size: 2em;
      animation: spin 1.5s linear infinite;
    }

    @keyframes spin {
      0% { transform: rotate(0deg); }
      100% { transform: rotate(360deg); }
    }

    .powering-on-label {
      font-size: 0.85em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-top: 0.5rem;
    }

    /* ── Levels ── */
    .levels-row {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .level-chip {
      flex: 1;
      display: flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.45rem 0.6rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-base);
      font-size: 0.78em;
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .level-chip .chip-icon {
      font-size: 1.1em;
    }

    .level-chip .chip-val {
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .level-chip .chip-val.low {
      color: #ef4444;
    }

    .level-chip .chip-val.warn {
      color: #fb923c;
    }

    .level-chip .chip-val.ok {
      color: #34d399;
    }

    /* ── Footer Row ── */
    .footer-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-top: 0.5rem;
      border-top: 1px solid var(--border-subtle);
    }

    .machine-state {
      font-size: 0.78em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      display: flex;
      align-items: center;
      gap: 0.35rem;
    }

    .power-btn {
      padding: 0.35rem 0.7rem;
      border-radius: var(--radius-base);
      border: 1px solid var(--border-subtle);
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      color: var(--color-dark-text-primary, #f8f9fa);
      cursor: pointer;
      font-size: 0.78em;
      transition: all 0.2s ease;
    }

    .power-btn:hover {
      border-color: var(--context-primary);
    }

    .power-btn.on {
      background: rgba(52, 211, 153, 0.12);
      border-color: rgba(52, 211, 153, 0.35);
      color: #34d399;
    }

    .power-btn.off {
      background: rgba(239, 68, 68, 0.12);
      border-color: rgba(239, 68, 68, 0.35);
      color: #ef4444;
    }

    /* ── Maintenance ── */
    .maintenance-alert {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0.65rem;
      background: rgba(251, 146, 60, 0.12);
      border: 1px solid rgba(251, 146, 60, 0.25);
      border-radius: var(--radius-base);
      margin-bottom: 0.75rem;
      font-size: 0.8em;
      color: #fb923c;
    }

    /* ── Error / Toast ── */
    .error-msg {
      font-size: 0.78em;
      color: #ef4444;
      margin-top: 0.35rem;
      padding: 0.35rem 0.5rem;
      background: rgba(239, 68, 68, 0.1);
      border-radius: var(--radius-base);
    }

    .success-msg {
      font-size: 0.78em;
      color: #34d399;
      margin-top: 0.35rem;
      padding: 0.35rem 0.5rem;
      background: rgba(52, 211, 153, 0.1);
      border-radius: var(--radius-base);
      animation: fadeOut 3s ease-in-out forwards;
    }

    @keyframes fadeOut {
      0%, 70% { opacity: 1; }
      100% { opacity: 0; }
    }

    .loader-container {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 180px;
      padding: 2rem;
    }

    @media (max-width: 768px) {
      .drink-grid {
        grid-template-columns: 1fr 1fr;
      }
      .levels-row {
        flex-wrap: wrap;
      }
      .level-chip {
        min-width: calc(50% - 0.25rem);
      }
    }
  `]

  static properties = {
    status: { type: Object },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean },
    brewError: { type: String },
    brewSuccess: { type: String },
    brewLoading: { type: Boolean },
    poweringOn: { type: Boolean },
    cupsSel: { type: Object }
  }

  constructor() {
    super()
    this.status = null
    this.loading = true
    this.mqttConnected = false
    this.brewError = null
    this.brewSuccess = null
    this.brewLoading = false
    this.poweringOn = false
    // Cups selection per drink (default 1)
    this.cupsSel = { espresso: 1, coffee: 1 }
  }

  connectedCallback() {
    super.connectedCallback()
    this._retrySetup()
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) {
        this.loading = false
        this.requestUpdate()
      }
    }, 10000)
    this._fetchStatus()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    this._cleanupEventListeners()
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    if (this._refreshInterval) clearInterval(this._refreshInterval)
    if (this._powerPollInterval) clearInterval(this._powerPollInterval)
  }

  // ── MQTT Setup ──

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      this._setupEventListeners()
    } else if (attempts < 10) {
      setTimeout(() => this._retrySetup(attempts + 1), 500)
    } else {
      this.loading = false
      this.requestUpdate()
    }
  }

  _setupEventListeners() {
    const mqttService = document.querySelector('mqtt-service')
    if (!mqttService) return

    this._boundStatusHandler = (e) => this._handleStatusEvent(e.detail)
    this._boundBrewingHandler = (e) => this._handleBrewingEvent(e.detail)
    this._boundMaintenanceHandler = (e) => this._handleMaintenanceEvent(e.detail)

    mqttService.addEventListener('coffee-status', this._boundStatusHandler)
    mqttService.addEventListener('coffee-brewing', this._boundBrewingHandler)
    mqttService.addEventListener('coffee-maintenance', this._boundMaintenanceHandler)
    this._mqttService = mqttService

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
    const wasBrewing = this.status?.brewing
    this.status = payload
    this.loading = false
    this.mqttConnected = true
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)

    // Detect brew completed → show toast
    if (wasBrewing && !payload.brewing) {
      this._showSuccess('Cafe pret !')
      this._sendToast('Cafe pret !', 'success')
    }

    // Auto power-on: machine reached ready → stop polling
    if (this.poweringOn && payload.mainstate === 2) {
      this.poweringOn = false
      if (this._powerPollInterval) clearInterval(this._powerPollInterval)
      // If pending brew, execute it now
      if (this._pendingBrew) {
        const { drink, cups } = this._pendingBrew
        this._pendingBrew = null
        this._executeBrew(drink, cups)
      }
    }

    this.requestUpdate()
  }

  _handleBrewingEvent({ payload }) {
    if (this.status) {
      if (payload.event_type === 'brewing/started') {
        this.status = { ...this.status, brewing: true, mainstate: 3, mainstate_text: 'brewing' }
      } else if (payload.event_type === 'brewing/completed') {
        this.status = { ...this.status, brewing: false, mainstate: 2, mainstate_text: 'ready', brew_progress: 0 }
        this._showSuccess('Cafe pret !')
        this._sendToast('Cafe pret !', 'success')
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

  // ── HTTP Fetch ──

  async _fetchStatus() {
    try {
      const resp = await fetch(`${getApiBase()}/v1/plugin-api/coffee/status`)
      if (resp.ok) {
        this.status = await resp.json()
        this.loading = false
        if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
        this.requestUpdate()
      }
    } catch (_) { /* silent */ }

    this._refreshInterval = setInterval(() => this._fetchStatusSilent(), 15000)
  }

  async _fetchStatusSilent() {
    try {
      const resp = await fetch(`${getApiBase()}/v1/plugin-api/coffee/status`)
      if (resp.ok) {
        const prev = this.status
        this.status = await resp.json()
        // Detect state changes via polling too
        if (prev?.brewing && !this.status.brewing) {
          this._showSuccess('Cafe pret !')
          this._sendToast('Cafe pret !', 'success')
        }
        if (this.poweringOn && this.status.mainstate === 2) {
          this.poweringOn = false
          if (this._powerPollInterval) clearInterval(this._powerPollInterval)
          if (this._pendingBrew) {
            const { drink, cups } = this._pendingBrew
            this._pendingBrew = null
            this._executeBrew(drink, cups)
          }
        }
        this.requestUpdate()
      }
    } catch (_) { /* silent */ }
  }

  // ── Actions ──

  async _brew(drink, cups = 1) {
    // Auto power-on if machine is in standby
    if (this.status?.online && this.status?.mainstate === 1) {
      this._pendingBrew = { drink, cups }
      await this._powerOn()
      return
    }
    await this._executeBrew(drink, cups)
  }

  async _executeBrew(drink, cups) {
    this.brewError = null
    this.brewSuccess = null
    this.brewLoading = true
    this.requestUpdate()

    try {
      const resp = await csrfService.fetchWithCsrf(`${getApiBase()}/v1/plugin-api/coffee/brew`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ drink, temperature: 2, cups })
      })
      const data = await resp.json()
      if (!resp.ok) {
        this.brewError = data.error || `Erreur ${resp.status}`
      } else {
        const label = drink === 'espresso' ? 'Espresso' : drink === 'coffee' ? 'Cafe long' : 'Eau chaude'
        this._showSuccess(`${label}${cups > 1 ? ' x2' : ''} lance !`)
      }
    } catch (e) {
      this.brewError = `Erreur: ${e.message}`
    } finally {
      this.brewLoading = false
      this.requestUpdate()
      if (this.brewError) {
        setTimeout(() => { this.brewError = null; this.requestUpdate() }, 5000)
      }
    }
  }

  async _stop() {
    try {
      await csrfService.fetchWithCsrf(`${getApiBase()}/v1/plugin-api/coffee/stop`, { method: 'POST' })
      this._showSuccess('Arrete')
    } catch (e) {
      this.brewError = 'Erreur arret'
      this.requestUpdate()
    }
  }

  async _powerOn() {
    this.poweringOn = true
    this.requestUpdate()
    try {
      await csrfService.fetchWithCsrf(`${getApiBase()}/v1/plugin-api/coffee/power`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ on: true })
      })
      // Poll status every 2s waiting for ready (max 60s)
      let attempts = 0
      this._powerPollInterval = setInterval(async () => {
        attempts++
        if (attempts > 30) {
          clearInterval(this._powerPollInterval)
          this.poweringOn = false
          this._pendingBrew = null
          this.brewError = 'Delai allumage depasse'
          this.requestUpdate()
        }
        await this._fetchStatusSilent()
      }, 2000)
    } catch (e) {
      this.poweringOn = false
      this.brewError = 'Erreur allumage'
      this.requestUpdate()
    }
  }

  async _powerOff() {
    try {
      await csrfService.fetchWithCsrf(`${getApiBase()}/v1/plugin-api/coffee/power`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ on: false })
      })
      this._showSuccess('Machine eteinte')
    } catch (_) { /* silent */ }
  }

  _showSuccess(msg) {
    this.brewSuccess = msg
    this.requestUpdate()
    setTimeout(() => { this.brewSuccess = null; this.requestUpdate() }, 3500)
  }

  _setCups(drink, n) {
    this.cupsSel = { ...this.cupsSel, [drink]: n }
    this.requestUpdate()
  }

  _sendToast(msg, type) {
    const toast = document.querySelector('toast-notifications')
    if (toast && typeof toast.addToast === 'function') {
      toast.addToast(msg, type)
    }
  }

  // ── Helpers ──

  _getOverallStatus() {
    if (!this.status?.online) return 'away'
    if (this.status.brewing) return 'home'
    if (this.status.maintenance_needed) return 'unknown'
    if (this.status.mainstate === 2) return 'home'
    return 'unknown'
  }

  _getStatusLabel() {
    if (!this.status?.online) return 'Hors ligne'
    if (this.poweringOn) return 'Allumage...'
    if (this.status.brewing) return 'Preparation...'
    if (this.status.mainstate === 2) return 'Prete'
    if (this.status.mainstate === 1) return 'Veille'
    if (this.status.mainstate === 5) return 'Maintenance'
    return this.status.mainstate_text || 'Inconnu'
  }

  _levelVal(val) {
    if (!val && val !== 0) return 0
    return Math.min(100, Math.max(0, val))
  }

  _levelClass(val, warnThreshold = 30) {
    if (!val && val !== 0) return ''
    if (val < warnThreshold / 2) return 'low'
    if (val < warnThreshold) return 'warn'
    return 'ok'
  }

  _levelText(val, type) {
    if (!val && val !== 0) return '—'
    if ((type === 'water' || type === 'beans') && val >= 50) return 'OK'
    return `${Math.min(100, val)}%`
  }

  _canBrew() {
    if (!this.status?.online || this.brewLoading || this.poweringOn) return false
    // Allow brew from standby (auto power-on) or ready
    return this.status.mainstate === 1 || this.status.mainstate === 2
  }

  // ── Render ──

  render() {
    if (this.loading) {
      return html`
        <div class="loader-container">
          <organic-loader text="Connexion cafetiere..."></organic-loader>
        </div>
      `
    }

    const canBrew = this._canBrew()
    const isBrewing = this.status?.brewing
    const isStandby = this.status?.online && this.status?.mainstate === 1
    const isReady = this.status?.online && this.status?.mainstate === 2

    return html`
      <div class="widget-header">
        <span class="widget-title">Cafetiere</span>
        <span class="status-badge ${this._getOverallStatus()}">${this._getStatusLabel()}</span>
      </div>

      ${this.status?.maintenance_needed ? html`
        <div class="maintenance-alert">
          &#9888; ${this.status.maintenance_reason || 'Maintenance requise'}
        </div>
      ` : ''}

      ${this.poweringOn ? html`
        <div class="brewing-state">
          <div class="powering-on">
            <div class="spin-icon">&#9749;</div>
            <div class="powering-on-label">Allumage en cours...${this._pendingBrew ? ` puis ${this._pendingBrew.drink}` : ''}</div>
          </div>
        </div>
      ` : isBrewing ? html`
        <div class="brewing-state">
          <div class="brewing-icon">&#9749;</div>
          <div class="brewing-label">Preparation en cours</div>
          ${this.status?.brew_progress ? html`
            <div class="brewing-progress">
              <div class="brewing-progress-fill" style="width: ${this.status.brew_progress}%"></div>
            </div>
          ` : ''}
          <button class="stop-btn" @click="${this._stop}">&#9724; Arreter</button>
        </div>
      ` : html`
        <div class="drink-list">
          <div class="drink-row ${canBrew ? '' : 'disabled'}">
            <span class="drink-icon">&#9749;</span>
            <span class="drink-name">Espresso</span>
            <div class="cups-toggle">
              <span class="cup-opt ${this.cupsSel.espresso === 1 ? 'active' : ''}" @click="${() => this._setCups('espresso', 1)}">1</span>
              <span class="cup-opt ${this.cupsSel.espresso === 2 ? 'active' : ''}" @click="${() => this._setCups('espresso', 2)}">2</span>
            </div>
            <button class="go-btn" ?disabled="${!canBrew || this.brewLoading}" @click="${() => this._brew('espresso', this.cupsSel.espresso)}">GO</button>
          </div>
          <div class="drink-row ${canBrew ? '' : 'disabled'}">
            <span class="drink-icon">&#9749;</span>
            <span class="drink-name">Cafe long</span>
            <div class="cups-toggle">
              <span class="cup-opt ${this.cupsSel.coffee === 1 ? 'active' : ''}" @click="${() => this._setCups('coffee', 1)}">1</span>
              <span class="cup-opt ${this.cupsSel.coffee === 2 ? 'active' : ''}" @click="${() => this._setCups('coffee', 2)}">2</span>
            </div>
            <button class="go-btn" ?disabled="${!canBrew || this.brewLoading}" @click="${() => this._brew('coffee', this.cupsSel.coffee)}">GO</button>
          </div>
          <div class="drink-row ${canBrew ? '' : 'disabled'}">
            <span class="drink-icon">&#128167;</span>
            <span class="drink-name">Eau chaude</span>
            <button class="go-btn" ?disabled="${!canBrew || this.brewLoading}" @click="${() => this._brew('hot_water', 1)}">GO</button>
          </div>
        </div>
        ${isStandby ? html`<div style="font-size:0.75em;color:var(--color-dark-text-tertiary);text-align:center;margin-bottom:0.5rem;">Machine en veille — allumage auto au lancement</div>` : ''}
      `}

      ${this.brewError ? html`<div class="error-msg">${this.brewError}</div>` : ''}
      ${this.brewSuccess ? html`<div class="success-msg">${this.brewSuccess}</div>` : ''}

      <!-- Levels -->
      <div class="levels-row">
        <div class="level-chip">
          <span class="chip-icon">&#128167;</span>
          <span>Eau</span>
          <span class="chip-val ${this._levelClass(this.status?.water_level, 30)}">${this._levelText(this.status?.water_level, 'water')}</span>
        </div>
        <div class="level-chip">
          <span class="chip-icon">&#127793;</span>
          <span>Grains</span>
          <span class="chip-val ${this._levelClass(this.status?.bean_level, 30)}">${this._levelText(this.status?.bean_level, 'beans')}</span>
        </div>
        <div class="level-chip">
          <span class="chip-icon">&#128465;</span>
          <span>Marc</span>
          <span class="chip-val ${(this.status?.waste_bean || 0) >= 12 ? 'low' : (this.status?.waste_bean || 0) >= 10 ? 'warn' : 'ok'}">${this.status?.waste_bean || 0}/14</span>
        </div>
        <div class="level-chip">
          <span class="chip-icon">&#128167;</span>
          <span>Detartr.</span>
          <span class="chip-val ${this._levelClass(this.status?.descale_status, 30)}">${this._levelText(this.status?.descale_status, 'descale')}</span>
        </div>
        <div class="level-chip">
          <span class="chip-icon">&#128167;</span>
          <span>Filtre</span>
          <span class="chip-val ${this.status?.aquaclean_installed ? this._levelClass(this.status?.aquaclean_remaining, 30) : ''}">${this.status?.aquaclean_installed ? this._levelText(this.status?.aquaclean_remaining, 'filter') : 'Non'}</span>
        </div>
      </div>

      <!-- Footer -->
      <div class="footer-row">
        <div class="machine-state">
          <span class="status-dot ${this.status?.online ? 'home' : 'away'}"></span>
          ${this.status?.online ? 'Connectee' : 'Hors ligne'}
        </div>
        ${isReady || isBrewing ? html`
          <button class="power-btn off" @click="${this._powerOff}">Eteindre</button>
        ` : this.status?.online ? html`
          <button class="power-btn on" @click="${this._powerOn}">Allumer</button>
        ` : ''}
      </div>
    `
  }
}

customElements.define('coffee-widget', CoffeeWidget)

export { CoffeeWidget }
