/**
 * Widget Aspirateur — Deebot T50 Pro Omni en Matter local
 *
 * Montre ce que le robot dit vraiment, et pas ce qu'on aimerait :
 *
 * · le MODE DE MARCHE ment sur la disponibilité (il dit « Idle » pendant les
 *   quatre heures de séchage des serpillères) — c'est la PHASE qui est
 *   affichée en premier ;
 * · Matter n'expose aucun temps restant : l'estimation de séchage est
 *   étiquetée « estimation », et absente si le plugin a démarré alors que le
 *   séchage était déjà en cours ;
 * · l'état des consommables (bac plein, réservoir vide) ne remonte que par le
 *   code d'erreur, affiché tel quel quand il y en a un.
 *
 * Pour l'arrêt, un seul bouton : « Arrêter et renvoyer », qui clôt la tâche
 * avant de renvoyer. Un simple retour à la base le ferait repartir.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import { getApiBase } from '../services/config.js'
import '../components/organic-loader.js'

const MODES = [
  { value: 'Vacuum & Mop', label: 'Aspi + serpillère' },
  { value: 'Vacuum-Only', label: 'Aspiration seule' },
  { value: 'Mopping', label: 'Serpillère seule' },
  { value: 'Mop after Vacuum', label: 'Serpillère après aspi' }
]

class RobotWidget extends LitElement {
  static properties = {
    etat: { type: Object },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean },
    choisies: { type: Array },
    busy: { type: String },
    error: { type: String },
    note: { type: String }
  }

  static styles = [sharedAnimations, widgetHeaderStyles, emptyStateStyles, statusBadgeStyles, statusDotStyles, css`
    :host { display: block; }

    .phase-block {
      display: flex;
      align-items: center;
      gap: 0.9rem;
      padding: 0.9rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      margin-bottom: 0.6rem;
    }

    .phase-icon { font-size: 2em; line-height: 1; flex-shrink: 0; }

    .phase-meta { display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }

    .phase-label {
      font-size: 1.15em;
      font-weight: 700;
      color: var(--color-dark-text-primary, #f8f9fa);
      line-height: 1.2;
    }

    .phase-sub {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
      gap: 0.4rem;
      margin-bottom: 0.6rem;
    }

    .metric-chip {
      display: flex;
      flex-direction: column;
      gap: 0.1rem;
      padding: 0.5rem 0.6rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm, 8px);
    }

    .metric-label {
      font-size: var(--text-xs, 0.7rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .metric-value {
      font-size: var(--text-sm, 0.9rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .metric-value.warn { color: #fbbf24; }
    .metric-value.bad { color: #ff6b6b; }

    .section-label {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin: 0.5rem 0 0.35rem;
    }

    .zones { display: flex; flex-wrap: wrap; gap: 0.4rem; }

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

    .chip[aria-pressed="true"] {
      border-color: var(--context-primary, #00d4aa);
      color: var(--color-dark-text-primary, #f8f9fa);
      background: rgba(0, 212, 170, 0.12);
    }

    .chip:disabled { opacity: 0.4; cursor: not-allowed; }

    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 0.4rem;
      margin-top: 0.6rem;
    }

    .btn {
      flex: 1 1 auto;
      min-width: 8rem;
      min-height: 40px;
      border-radius: var(--radius-sm, 8px);
      border: 1px solid var(--border-subtle);
      background: var(--surface-glass-strong, rgba(255, 255, 255, 0.08));
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: var(--text-sm, 0.875rem);
      cursor: pointer;
      padding: 0.5rem 0.75rem;
    }

    .btn.primary {
      border-color: var(--context-primary, #00d4aa);
      background: rgba(0, 212, 170, 0.14);
    }

    .btn.stop { border-color: rgba(255, 107, 107, 0.45); }
    .btn:disabled { opacity: 0.4; cursor: not-allowed; }

    select {
      width: 100%;
      min-height: 40px;
      border-radius: var(--radius-sm, 8px);
      border: 1px solid var(--border-subtle);
      background: var(--surface-glass-strong, rgba(255, 255, 255, 0.08));
      color: var(--color-dark-text-primary, #f8f9fa);
      padding: 0.4rem 0.5rem;
      font-size: var(--text-sm, 0.875rem);
    }

    .error { font-size: var(--text-xs, 0.75rem); color: #ff6b6b; margin-top: 0.4rem; }
    .note { font-size: var(--text-xs, 0.75rem); color: #fbbf24; margin-top: 0.4rem; }

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
    this.etat = null
    this.loading = true
    this.mqttConnected = false
    this.choisies = []
    this.busy = ''
    this.error = ''
    this.note = ''
    this._mode = ''
    this._boundState = (e) => this._appliquer(e.detail.payload)
  }

  connectedCallback() {
    super.connectedCallback()
    this._retrySetup()
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) { this.loading = false; this.requestUpdate() }
    }, 10000)
    this._fetch()
    this._refreshInterval = setInterval(() => this._fetch(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._mqttService) this._mqttService.removeEventListener('robot-state', this._boundState)
    if (this._loadingTimeout) clearTimeout(this._loadingTimeout)
    if (this._refreshInterval) clearInterval(this._refreshInterval)
  }

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      mqttService.addEventListener('robot-state', this._boundState)
      this._mqttService = mqttService
      this.mqttConnected = true
      if (typeof mqttService.getRobotCache === 'function') {
        const cache = mqttService.getRobotCache()
        if (cache.state) this._appliquer(cache.state)
      }
    } else if (attempts < 10) {
      setTimeout(() => this._retrySetup(attempts + 1), 500)
    }
  }

  _appliquer(payload) {
    if (!payload) return
    this.etat = payload
    this.loading = false
    this.lastUpdate = new Date().toLocaleTimeString('fr-FR')
    this.requestUpdate()
  }

  get _base() { return `${getApiBase()}/v1/plugin-api/robot` }

  _headers() {
    const token = localStorage.getItem('auth_token') || ''
    return {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    }
  }

  async _fetch() {
    try {
      const resp = await fetch(`${this._base}/robot`, { headers: this._headers() })
      if (resp.ok) this._appliquer(await resp.json())
    } catch (e) {
      console.warn('[robot-widget] repli HTTP indisponible :', e.message)
    }
  }

  async _action(nom, corps) {
    this.busy = nom
    this.error = ''
    this.note = ''
    try {
      const resp = await fetch(`${this._base}/actions/${nom}`, {
        method: 'POST',
        headers: this._headers(),
        body: JSON.stringify(corps || {})
      })
      const d = await resp.json().catch(() => ({}))
      if (!resp.ok) {
        this.error = d.error || `échec (${resp.status})`
      } else {
        // Le plugin répond parfois avec un avertissement utile : la station
        // séchait et le mode a été basculé, ou le retour n'a pas clos la tâche.
        const notes = [].concat(d.notes || [], d.avertissement || [])
        if (notes.length) this.note = notes.join(' · ')
      }
      setTimeout(() => this._fetch(), 2000)
    } catch (e) {
      this.error = e.message
    } finally {
      this.busy = ''
      this.requestUpdate()
    }
  }

  _basculerZone(nom) {
    this.choisies = this.choisies.includes(nom)
      ? this.choisies.filter(z => z !== nom)
      : [...this.choisies, nom]
  }

  _lancer() {
    if (!this.choisies.length) return
    const corps = { rooms: this.choisies }
    if (this._mode) corps.mode = this._mode
    this._action('clean_rooms', corps)
  }

  // ── Lecture d'état ──

  _icone() {
    const p = this.etat?.phase?.label
    if (this.etat?.drying?.active) return '💨'
    if (this.etat?.cleaning) return '🧹'
    if (p === 'Wash Mop' || p === 'Return Wash Mop') return '🚿'
    if (p === 'Return') return '↩️'
    if (this.etat?.docked) return '🔌'
    return '🤖'
  }

  _titre() {
    if (!this.etat) return '—'
    if (this.etat.drying?.active) return 'Séchage des serpillères'
    if (this.etat.cleaning) return this.etat.zone?.name ? `Nettoie : ${this.etat.zone.name}` : 'En nettoyage'
    return this.etat.operational?.label || this.etat.phase?.label || '—'
  }

  _sousTitre() {
    if (!this.etat) return ''
    const bouts = []
    const sechage = this.etat.drying
    if (sechage?.active) {
      if (sechage.estimated_remaining_minutes !== null && sechage.estimated_remaining_minutes !== undefined) {
        bouts.push(`encore ~${sechage.estimated_remaining_minutes} min (estimation)`)
      } else {
        bouts.push('durée restante inconnue')
      }
    }
    if (this.etat.phase?.label && !sechage?.active) bouts.push(`phase : ${this.etat.phase.label}`)
    if (this.etat.clean_mode?.label) bouts.push(this.etat.clean_mode.label)
    return bouts.join(' · ')
  }

  _niveau() {
    if (!this.etat?.online) return 'offline'
    if (this.etat.error?.code) return 'critical'
    if (this.etat.drying?.active) return 'warning'
    return 'ok'
  }

  _classeBatterie() {
    const b = this.etat?.battery
    if (b === null || b === undefined) return ''
    if (b <= 20) return 'bad'
    if (b <= 40) return 'warn'
    return ''
  }

  render() {
    if (this.loading) {
      return html`<div class="loader-container"><organic-loader text="Connexion à l'aspirateur..."></organic-loader></div>`
    }

    if (!this.etat || !this.etat.online) {
      return html`
        <div class="widget-header">
          <span class="widget-title">🤖 Aspirateur</span>
          <span class="status-badge offline"><span class="status-dot offline"></span>Hors ligne</span>
        </div>
        <div class="empty-state">
          <div class="empty-state-icon">🤖</div>
          <div class="empty-state-text">Lien Matter indisponible</div>
          <div class="empty-state-hint">Le conteneur matter-server répond-il sur 5580 ?</div>
        </div>
      `
    }

    const niveau = this._niveau()
    const zones = (this.etat.zones || []).filter(z => z.name)
    const occupe = !!this.busy
    const peutLancer = this.choisies.length > 0 && !occupe

    return html`
      <div class="widget-header">
        <span class="widget-title">🤖 Aspirateur</span>
        <span class="status-badge ${niveau === 'ok' ? 'ok' : niveau}">
          <span class="status-dot ${niveau}"></span>
          ${this.etat.cleaning ? 'En nettoyage' : (this.etat.docked ? 'À la base' : 'En pause')}
        </span>
      </div>

      <div class="phase-block">
        <span class="phase-icon">${this._icone()}</span>
        <div class="phase-meta">
          <span class="phase-label">${this._titre()}</span>
          <span class="phase-sub">${this._sousTitre()}</span>
        </div>
      </div>

      <div class="metrics-grid">
        <div class="metric-chip">
          <span class="metric-label">Batterie</span>
          <span class="metric-value ${this._classeBatterie()}">
            ${this.etat.battery ?? '—'} %
          </span>
        </div>
        <div class="metric-chip">
          <span class="metric-label">Charge</span>
          <span class="metric-value">${this.etat.charge_state || '—'}</span>
        </div>
        <div class="metric-chip">
          <span class="metric-label">Alerte</span>
          <span class="metric-value ${this.etat.error?.code ? 'bad' : ''}">
            ${this.etat.error?.label || '—'}
          </span>
        </div>
      </div>

      ${zones.length ? html`
        <div class="section-label">Pièces à nettoyer</div>
        <div class="zones">
          ${zones.map(z => html`
            <button class="chip"
                    aria-pressed="${this.choisies.includes(z.name) ? 'true' : 'false'}"
                    ?disabled="${occupe}"
                    @click="${() => this._basculerZone(z.name)}">${z.name}</button>
          `)}
        </div>
      ` : html`<div class="section-label">Aucune pièce cartographiée</div>`}

      <div class="section-label">Mode de nettoyage</div>
      <select ?disabled="${occupe}" @change="${(e) => { this._mode = e.target.value }}">
        <option value="">Garder : ${this.etat.clean_mode?.label || '—'}</option>
        ${MODES.map(m => html`<option value="${m.value}">${m.label}</option>`)}
      </select>

      <div class="actions">
        <button class="btn primary" ?disabled="${!peutLancer}" @click="${() => this._lancer()}">
          🧹 Lancer${this.choisies.length ? ` (${this.choisies.length})` : ''}
        </button>
        <button class="btn" ?disabled="${occupe}" @click="${() => this._action('clean_all')}">
          🏠 Toute la maison
        </button>
        <button class="btn stop" ?disabled="${occupe}" @click="${() => this._action('stop_and_dock')}">
          🛑 Arrêter et renvoyer
        </button>
      </div>

      ${this.note ? html`<div class="note">${this.note}</div>` : ''}
      ${this.error ? html`<div class="error">${this.error}</div>` : ''}

      <div class="footer-row">
        <span>
          <span class="status-dot ${this.mqttConnected ? 'ok' : 'offline'}"></span>
          ${this.etat.selected_zones?.length
            ? `Sélection : ${this.etat.selected_zones.map(z => z.name || z.id).join(', ')}`
            : 'Matter en local'}
        </span>
        ${this.lastUpdate ? html`<span>Màj : ${this.lastUpdate}</span>` : ''}
      </div>
    `
  }
}

customElements.define('robot-widget', RobotWidget)

export { RobotWidget }
