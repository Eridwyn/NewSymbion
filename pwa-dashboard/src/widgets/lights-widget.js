/**
 * Widget Lampes — ampoules Tuya pilotées en local
 *
 * Une carte par pièce : bascule de la pièce entière, puis chaque lampe avec
 * sa luminosité et sa température de couleur. MQTT temps réel
 * (symbion/lights/#, retenu) + repli HTTP. Les commandes passent par
 * /v1/plugin-api/lights/*.
 *
 * Pas d'optimisme : un curseur relâché envoie l'ordre, et l'affichage attend
 * l'état que la lampe renvoie. Une lampe injoignable se grise au lieu de
 * mentir.
 */

import { LitElement, html, css } from 'lit'
import { sharedBase } from '../styles/shared-base.js'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import { getApiBase } from '../services/config.js'
import '../components/organic-loader.js'

class LightsWidget extends LitElement {
  static properties = {
    lights: { type: Array },
    rooms: { type: Object },
    scenes: { type: Array },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean },
    busy: { type: Object },
    error: { type: String }
  }

  static styles = [sharedBase, sharedAnimations, widgetHeaderStyles, emptyStateStyles, statusBadgeStyles, statusDotStyles, css`
    :host { display: block; }

    .room {
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      margin-bottom: 0.6rem;
    }

    .room-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
    }

    .room-name {
      font-weight: 600;
      text-transform: capitalize;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.4rem;
    }

    .room-count {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #6c757d);
      font-weight: 400;
    }

    /* ── Interrupteur ── */
    .switch {
      position: relative;
      width: 46px;
      height: 26px;
      border-radius: 999px;
      border: 1px solid var(--border-subtle);
      background: var(--surface-glass-strong, rgba(255, 255, 255, 0.08));
      cursor: pointer;
      flex-shrink: 0;
      transition: background 0.25s ease;
      /* cible tactile : le bouton visible fait 46px, la zone cliquable 44px
         de haut minimum, sinon on rate la bascule au pouce */
      padding: 0;
    }

    .switch::after {
      content: '';
      position: absolute;
      top: 2px;
      left: 2px;
      width: 20px;
      height: 20px;
      border-radius: 50%;
      background: var(--color-dark-text-secondary, #cbd5e1);
      transition: transform 0.25s ease, background 0.25s ease;
    }

    .switch[aria-checked="true"] { background: var(--context-primary, #00d4aa); }
    .switch[aria-checked="true"]::after {
      transform: translateX(20px);
      background: #fff;
    }

    .switch:disabled { opacity: 0.4; cursor: not-allowed; }

    .lamp {
      display: flex;
      flex-direction: column;
      gap: 0.35rem;
      padding: 0.5rem 0 0.1rem;
      border-top: 1px solid var(--border-subtle);
      margin-top: 0.5rem;
    }

    .lamp.offline { opacity: 0.45; }

    .lamp-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
    }

    .lamp-name {
      font-size: var(--text-sm, 0.875rem);
      color: var(--color-dark-text-secondary, #cbd5e1);
      display: flex;
      align-items: center;
      gap: 0.4rem;
      min-width: 0;
    }

    .lamp-name span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

    .sliders { display: flex; flex-direction: column; gap: 0.3rem; }

    .slider-row {
      display: grid;
      grid-template-columns: 1.6rem 1fr 2.6rem;
      align-items: center;
      gap: 0.4rem;
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    input[type="range"] {
      width: 100%;
      height: 24px;             /* cible tactile confortable */
      accent-color: var(--context-primary, #00d4aa);
      background: transparent;
    }

    input[type="range"]:disabled { opacity: 0.4; }

    .scenes {
      display: flex;
      flex-wrap: wrap;
      gap: 0.4rem;
      margin-top: 0.3rem;
    }

    .chip {
      background: var(--surface-glass-strong, rgba(255, 255, 255, 0.08));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-xl, 999px);
      color: var(--color-dark-text-secondary, #cbd5e1);
      padding: 0.45rem 0.85rem;
      font-size: var(--text-xs, 0.75rem);
      cursor: pointer;
      min-height: 36px;
    }

    .chip:hover:not(:disabled) { color: var(--color-dark-text-primary, #f8f9fa); }
    .chip:disabled { opacity: 0.4; cursor: not-allowed; }
    .chip.danger { border-color: rgba(255, 107, 107, 0.4); }

    .error {
      font-size: var(--text-xs, 0.75rem);
      color: #ff6b6b;
      margin-top: 0.4rem;
    }

    .footer-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-top: 0.5rem;
    }
  `]

  constructor() {
    super()
    this.lights = []
    this.rooms = {}
    this.scenes = []
    this.loading = true
    this.mqttConnected = false
    this.busy = {}
    this.error = ''
    this._boundSummary = (e) => this._applySummary(e.detail.payload)
  }

  connectedCallback() {
    super.connectedCallback()
    this._retrySetup()
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) { this.loading = false; this.requestUpdate() }
    }, 10000)
    this._fetch()
    this._refreshInterval = setInterval(() => this._fetch(), 60000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._mqttService) this._mqttService.removeEventListener('lights-summary', this._boundSummary)
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    if (this._refreshInterval) clearInterval(this._refreshInterval)
  }

  // ── MQTT ──

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      mqttService.addEventListener('lights-summary', this._boundSummary)
      this._mqttService = mqttService
      this.mqttConnected = true
      if (typeof mqttService.getLightsCache === 'function') {
        const cache = mqttService.getLightsCache()
        if (cache) this._applySummary(cache)
      }
    } else if (attempts < 10) {
      setTimeout(() => this._retrySetup(attempts + 1), 500)
    }
  }

  _applySummary(payload) {
    if (!payload) return
    this.lights = payload.lights || []
    this.rooms = payload.rooms || {}
    this.scenes = payload.scenes || []
    this.loading = false
    this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    this.requestUpdate()
  }

  // ── HTTP ──

  get _base() { return `${getApiBase()}/v1/plugin-api/lights` }

  _headers() {
    const token = localStorage.getItem('auth_token') || ''
    return {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    }
  }

  async _fetch() {
    try {
      const resp = await fetch(`${this._base}/lights`, { headers: this._headers() })
      if (resp.ok) this._applySummary(await resp.json())
    } catch (e) {
      console.warn('[lights-widget] repli HTTP indisponible :', e.message)
    }
  }

  /** Envoie un ordre et RELIT : on n'affiche jamais un état supposé. */
  async _commander(cle, url, methode, corps) {
    this.busy = { ...this.busy, [cle]: true }
    this.error = ''
    try {
      const resp = await fetch(url, {
        method: methode,
        headers: this._headers(),
        body: corps === undefined ? undefined : JSON.stringify(corps)
      })
      if (!resp.ok) {
        const d = await resp.json().catch(() => ({}))
        this.error = d.error || `échec (${resp.status})`
      }
      // La lampe met ~1 s à renvoyer son état : on relit après ce délai, et
      // MQTT corrigera de toute façon si on a relu trop tôt.
      setTimeout(() => this._fetch(), 1200)
    } catch (e) {
      this.error = e.message
    } finally {
      this.busy = { ...this.busy, [cle]: false }
      this.requestUpdate()
    }
  }

  _basculerPiece(piece, allumee) {
    this._commander(`room:${piece}`, `${this._base}/lights/groups/${encodeURIComponent(piece)}`, 'PUT', { on: !allumee })
  }

  _basculerLampe(lampe) {
    this._commander(`lamp:${lampe.id}`, `${this._base}/lights/${lampe.id}`, 'PUT', { on: !lampe.on })
  }

  _reglerLampe(lampe, champ, valeur) {
    this._commander(`lamp:${lampe.id}`, `${this._base}/lights/${lampe.id}`, 'PUT', { [champ]: Number(valeur) })
  }

  _activerScene(id) {
    this._commander(`scene:${id}`, `${this._base}/scenes/${encodeURIComponent(id)}/activate`, 'PUT')
  }

  _toutEteindre() {
    this._commander('all_off', `${this._base}/actions/all_off`, 'POST', {})
  }

  // ── Rendu ──

  _parPiece() {
    const groupes = new Map()
    for (const l of this.lights) {
      if (!groupes.has(l.room)) groupes.set(l.room, [])
      groupes.get(l.room).push(l)
    }
    return [...groupes.entries()].sort(([a], [b]) => a.localeCompare(b, 'fr'))
  }

  _lampe(lampe) {
    const occupe = !!this.busy[`lamp:${lampe.id}`]
    return html`
      <div class="lamp ${lampe.online ? '' : 'offline'}">
        <div class="lamp-head">
          <span class="lamp-name">
            <span class="status-dot ${lampe.online ? (lampe.on ? 'ok' : 'offline') : 'critical'}"></span>
            <span>${lampe.name}</span>
          </span>
          <button class="switch"
                  role="switch"
                  aria-checked="${lampe.on ? 'true' : 'false'}"
                  aria-label="${lampe.name}"
                  ?disabled="${!lampe.online || occupe}"
                  @click="${() => this._basculerLampe(lampe)}"></button>
        </div>
        ${lampe.online && lampe.on ? html`
          <div class="sliders">
            ${lampe.caps?.dimmable ? html`
              <div class="slider-row">
                <span title="Luminosité">🔆</span>
                <input type="range" min="1" max="100" .value="${String(lampe.brightness ?? 100)}"
                       aria-label="Luminosité de ${lampe.name}"
                       ?disabled="${occupe}"
                       @change="${(e) => this._reglerLampe(lampe, 'brightness', e.target.value)}">
                <span>${lampe.brightness ?? '—'} %</span>
              </div>
            ` : ''}
            ${lampe.caps?.color_temp ? html`
              <div class="slider-row">
                <span title="Température de couleur">🌡️</span>
                <input type="range" min="2200" max="6500" step="100" .value="${String(lampe.color_temp_k ?? 4000)}"
                       aria-label="Température de couleur de ${lampe.name}"
                       ?disabled="${occupe}"
                       @change="${(e) => this._reglerLampe(lampe, 'color_temp_k', e.target.value)}">
                <span>${lampe.color_temp_k ? `${Math.round(lampe.color_temp_k / 100) / 10}k` : '—'}</span>
              </div>
            ` : ''}
          </div>
        ` : ''}
        ${!lampe.online && lampe.last_error ? html`
          <div class="error">injoignable — ${lampe.last_error}</div>
        ` : ''}
      </div>
    `
  }

  render() {
    if (this.loading) {
      return html`<div class="loader-container"><organic-loader text="Connexion aux lampes..."></organic-loader></div>`
    }

    const groupes = this._parPiece()
    const allumees = this.lights.filter(l => l.on && l.online).length
    const injoignables = this.lights.filter(l => !l.online).length

    return html`
      <div class="widget-header">
        <span class="widget-title">💡 Lampes</span>
        <span class="status-badge ${injoignables ? 'warning' : 'ok'}">
          <span class="status-dot ${injoignables ? 'warning' : 'ok'}"></span>
          ${allumees} allumée${allumees > 1 ? 's' : ''}
        </span>
      </div>

      ${!groupes.length ? html`
        <div class="empty-state">
          <div class="empty-state-icon">💡</div>
          <div class="empty-state-text">Aucune lampe déclarée</div>
          <div class="empty-state-hint">Voir /opt/symbion/config/lights.toml</div>
        </div>
      ` : html`
        ${groupes.map(([piece, lampes]) => {
          const allumee = lampes.some(l => l.on && l.online)
          const occupe = !!this.busy[`room:${piece}`]
          const joignables = lampes.filter(l => l.online).length
          return html`
            <div class="room">
              <div class="room-head">
                <span class="room-name">
                  ${piece}
                  <span class="room-count">${joignables}/${lampes.length}</span>
                </span>
                <button class="switch"
                        role="switch"
                        aria-checked="${allumee ? 'true' : 'false'}"
                        aria-label="Toute la pièce ${piece}"
                        ?disabled="${!joignables || occupe}"
                        @click="${() => this._basculerPiece(piece, allumee)}"></button>
              </div>
              ${lampes.map(l => this._lampe(l))}
            </div>
          `
        })}

        <div class="scenes">
          ${this.scenes.map(id => html`
            <button class="chip" ?disabled="${!!this.busy[`scene:${id}`]}"
                    @click="${() => this._activerScene(id)}">🎬 ${id}</button>
          `)}
          <button class="chip danger" ?disabled="${!allumees || !!this.busy['all_off']}"
                  @click="${() => this._toutEteindre()}">🌙 Tout éteindre</button>
        </div>
      `}

      ${this.error ? html`<div class="error">${this.error}</div>` : ''}

      <div class="footer-row">
        <span>
          <span class="status-dot ${this.mqttConnected ? 'ok' : 'offline'}"></span>
          Tuya en local
        </span>
        ${this.lastUpdate ? html`<span>Màj : ${this.lastUpdate}</span>` : ''}
      </div>
    `
  }
}

customElements.define('lights-widget', LightsWidget)

export { LightsWidget }
