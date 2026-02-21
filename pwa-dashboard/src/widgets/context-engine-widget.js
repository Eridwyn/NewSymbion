/**
 * Context Engine Widget - Widget Compact Dashboard
 *
 * Affiche le mode actuel + résumé automations + bouton "Gérer"
 * Remplace context-widget.js et automations-widget.js
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import automationsService from '../services/automations-service.js'
import pollingScheduler from '../services/polling-scheduler.js'

class ContextEngineWidget extends LitElement {
  static styles = [sharedAnimations, css`
    :host {
      display: block;
    }

    .widget {
      background: linear-gradient(135deg,
        rgba(19, 20, 26, 0.95) 0%,
        rgba(10, 10, 11, 0.98) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      border-radius: 16px;
      overflow: hidden;
      transition: all 0.3s ease;
    }

    .widget:hover {
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 35%, transparent);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3),
                  0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
    }

    .header {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 1rem 1.25rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      background: rgba(0, 0, 0, 0.2);
    }

    .header-icon {
      font-size: 1.25rem;
    }

    .header-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .content {
      padding: 1.25rem;
    }

    /* Mode Section */
    .mode-section {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 1rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }

    .mode-icon {
      font-size: 2.5rem;
      animation: icon-float 3s ease-in-out infinite;
    }

    @keyframes icon-float {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-4px); }
    }

    .mode-info {
      flex: 1;
    }

    .mode-name {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 0.25rem;
    }

    .mode-reason {
      font-size: 0.75rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 0.5rem;
    }

    /* Confidence bar removed - now managed by Intelligence Widget */

    /* Automations Section */
    .automations-section {
      margin-bottom: 1rem;
    }

    .automations-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .automations-label {
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .automations-count {
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .automations-count .active {
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
    }

    .last-execution {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0.75rem;
      background: rgba(255, 255, 255, 0.03);
      border-radius: var(--radius-base);
      font-size: 0.75rem;
    }

    .last-execution-name {
      flex: 1;
      color: var(--color-dark-text-primary, #f8f9fa);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .last-execution-status {
      flex-shrink: 0;
    }

    .last-execution-status.success {
      color: #22c55e;
    }

    .last-execution-status.failure {
      color: #ef4444;
    }

    .last-execution-time {
      flex-shrink: 0;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    /* Validations badge */
    .validations-badge {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0.75rem;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: var(--radius-base);
      margin-bottom: 1rem;
      font-size: 0.8rem;
      color: #ef4444;
    }

    /* Manage Button */
    .manage-btn {
      width: 100%;
      padding: 0.75rem;
      border-radius: 10px;
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent) 100%);
      color: var(--context-primary, #00d4aa);
      font-size: 0.85rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
    }

    .manage-btn:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 100%);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 2rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.85rem;
    }

    /* Empty state */
    .empty {
      text-align: center;
      padding: 1rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      font-size: 0.8rem;
    }
  `]

  static properties = {
    contextState: { type: Object },
    automations: { type: Array },
    lastExecution: { type: Object },
    validationsCount: { type: Number },
    loading: { type: Boolean },
  }

  constructor() {
    super()
    this.contextState = null
    this.automations = []
    this.lastExecution = null
    this.validationsCount = 0
    this.loading = true
  }

  connectedCallback() {
    super.connectedCallback()

    // Listen for context changes
    this._contextHandler = () => this.loadContext()
    document.body.addEventListener('context-change', this._contextHandler)

    // Listen for automation changes
    this._automationHandler = () => this.loadAutomations()
    document.body.addEventListener('automations:loaded', this._automationHandler)

    // Use centralized polling scheduler (auto-pauses when page hidden)
    this._unsubscribePolling = pollingScheduler.subscribe('30s', () => this.loadData())
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.body.removeEventListener('context-change', this._contextHandler)
    document.body.removeEventListener('automations:loaded', this._automationHandler)
    if (this._unsubscribePolling) {
      this._unsubscribePolling()
      this._unsubscribePolling = null
    }
  }

  async loadData() {
    this.loading = true
    await Promise.all([
      this.loadContext(),
      this.loadAutomations(),
      this.loadValidations(),
    ])
    this.loading = false
  }

  async loadContext() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const data = await apiService.request('/v1/context/current')
      this.contextState = data
    } catch (e) {
      console.error('[context-engine-widget] Failed to load context:', e)
    }
  }

  async loadAutomations() {
    try {
      this.automations = await automationsService.fetchAutomations()
      const history = await automationsService.fetchHistory(1)
      this.lastExecution = history[0] || null
    } catch (e) {
      console.error('[context-engine-widget] Failed to load automations:', e)
    }
  }

  async loadValidations() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const validations = await apiService.request('/v1/decision/validations/pending')
      this.validationsCount = Array.isArray(validations) ? validations.length : 0
    } catch (e) {
      this.validationsCount = 0
    }
  }

  openContextEngine() {
    this.dispatchEvent(new CustomEvent('open-context-engine', {
      bubbles: true,
      composed: true
    }))
  }

  getModeIcon(mode) {
    // Support both legacy and new mode slugs
    const icons = {
      cravate: '👔', intime: '🏡', neutre: '🌱',
      pro: '👔', focus: '🎯', maison: '🏡', veille: '🌱'
    }
    return icons[mode?.toLowerCase()] || '🌱'
  }

  getModeName(mode) {
    // Support both legacy and new mode slugs
    const names = {
      cravate: 'Focus Pro', intime: 'Maison', neutre: 'Veille',
      pro: 'Pro', focus: 'Focus', maison: 'Maison', veille: 'Veille'
    }
    return names[mode?.toLowerCase()] || 'Inconnu'
  }

  formatTime(timestamp) {
    if (!timestamp) return ''
    const date = new Date(timestamp)
    const now = new Date()
    const diff = (now - date) / 1000
    if (diff < 60) return "À l'instant"
    if (diff < 3600) return `${Math.floor(diff / 60)}min`
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`
    return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  }

  render() {
    if (this.loading && !this.contextState) {
      return html`
        <div class="widget">
          <div class="header">
            <span class="header-icon">🧠</span>
            <span class="header-title">Decision Engine</span>
          </div>
          <div class="loading">Chargement...</div>
        </div>
      `
    }

    // Prefer mode_slug (dynamic) over mode (legacy enum)
    const mode = this.contextState?.mode_slug || this.contextState?.mode?.toLowerCase() || 'veille'
    const enabledCount = this.automations.filter(a => a.enabled).length

    return html`
      <div class="widget">
        <div class="header">
          <span class="header-icon">🧠</span>
          <span class="header-title">Decision Engine</span>
        </div>

        <div class="content">
          <!-- Mode Section -->
          <div class="mode-section">
            <div class="mode-icon">${this.getModeIcon(mode)}</div>
            <div class="mode-info">
              <div class="mode-name">${this.getModeName(mode)}</div>
              <div class="mode-reason">${this.contextState?.reason || 'Détection auto'}</div>
            </div>
          </div>

          <!-- Validations Badge -->
          ${this.validationsCount > 0 ? html`
            <div class="validations-badge">
              ⚠️ ${this.validationsCount} validation${this.validationsCount > 1 ? 's' : ''} en attente
            </div>
          ` : ''}

          <!-- Automations Section -->
          <div class="automations-section">
            <div class="automations-header">
              <span class="automations-label">Automations</span>
              <span class="automations-count">
                <span class="active">${enabledCount}</span> / ${this.automations.length}
              </span>
            </div>

            ${this.lastExecution ? html`
              <div class="last-execution">
                <span class="last-execution-name">${this.lastExecution.automation_name}</span>
                <span class="last-execution-status ${this.lastExecution.success ? 'success' : 'failure'}">
                  ${this.lastExecution.success ? '✓' : '✗'}
                </span>
                <span class="last-execution-time">${this.formatTime(this.lastExecution.executed_at)}</span>
              </div>
            ` : html`
              <div class="empty">Aucune exécution récente</div>
            `}
          </div>

          <!-- Manage Button -->
          <button class="manage-btn" @click="${this.openContextEngine}">
            🔧 Gérer
          </button>
        </div>
      </div>
    `
  }
}

customElements.define('context-engine-widget', ContextEngineWidget)

export { ContextEngineWidget }
