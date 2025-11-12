/**
 * Widget Contexte Symbion
 *
 * Affiche le mode contextuel actuel avec thème visuel adaptatif
 * Modes: 👔 Cravate (Pro), 🏡 Intime (Home), 🌱 Neutre (Idle)
 */

import { LitElement, html, css } from 'lit'
import csrfService from '../services/csrf-service.js'

class ContextWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .widget-container {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 12px;
      padding: 0.75rem 1rem;
      backdrop-filter: blur(10px);
    }

    .mode-display {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-bottom: 0.5rem;
    }

    .mode-icon {
      font-size: 2.5rem;
      line-height: 1;
      animation: float 3s ease-in-out infinite;
    }

    @keyframes float {
      0%, 100% { transform: translateY(0px); }
      50% { transform: translateY(-8px); }
    }

    .mode-info {
      flex: 1;
    }

    .mode-title {
      font-size: 1.25rem;
      font-weight: 700;
      color: var(--context-primary, #10b981);
      margin-bottom: 0.125rem;
      transition: color 0.5s ease;
    }

    .mode-reason {
      font-size: 0.75rem;
      color: #a0a0a0;
      line-height: 1.3;
    }

    .mode-badge {
      padding: 0.375rem 0.75rem;
      border-radius: 16px;
      font-size: 0.625rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      background: var(--context-bg, rgba(16, 185, 129, 0.15));
      color: var(--context-primary, #10b981);
      border: 1px solid var(--context-primary, #10b981);
      transition: all 0.5s ease;
    }

    .mode-details {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-top: 0.5rem;
      padding-top: 0.5rem;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
      font-size: 0.625rem;
    }

    .detail-item {
      display: flex;
      align-items: center;
      gap: 0.375rem;
    }

    .detail-label {
      color: #808080;
    }

    .detail-value {
      color: #e0e0e0;
      font-weight: 600;
    }

    .confidence-bar {
      width: 60px;
      height: 3px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 2px;
      overflow: hidden;
    }

    .confidence-fill {
      height: 100%;
      background: var(--context-primary, #10b981);
      transition: width 0.5s ease, background-color 0.5s ease;
    }

    .loading {
      text-align: center;
      color: #808080;
      padding: 2rem;
    }

    .error {
      color: #ff6b6b;
      text-align: center;
      padding: 1rem;
    }

    /* Mode-specific animations */
    .mode-cravate .mode-icon {
      filter: drop-shadow(0 0 20px rgba(37, 99, 235, 0.5));
    }

    .mode-intime .mode-icon {
      filter: drop-shadow(0 0 20px rgba(16, 185, 129, 0.5));
    }

    .mode-neutre .mode-icon {
      filter: drop-shadow(0 0 20px rgba(107, 114, 128, 0.5));
    }

    /* Manual controls */
    .manual-controls {
      margin-top: 0.75rem;
      padding-top: 0.75rem;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .controls-toggle {
      width: 100%;
      padding: 0.5rem;
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      color: #a0a0a0;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .controls-toggle:hover {
      background: rgba(255, 255, 255, 0.08);
      border-color: rgba(255, 255, 255, 0.25);
    }

    .controls-content {
      margin-top: 0.75rem;
    }

    .controls-content.hidden {
      display: none;
    }

    .mode-buttons {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 0.5rem;
      margin-bottom: 1rem;
    }

    .mode-button {
      padding: 0.75rem;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      color: #e0e0e0;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.875rem;
      text-align: center;
    }

    .mode-button:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.3);
      transform: translateY(-2px);
    }

    .mode-button.active {
      background: var(--context-bg, rgba(16, 185, 129, 0.15));
      border-color: var(--context-primary, #10b981);
      color: var(--context-primary, #10b981);
    }

    .mode-button-icon {
      font-size: 1.5rem;
      margin-bottom: 0.25rem;
    }

    .mode-button-label {
      font-size: 0.75rem;
      font-weight: 600;
    }

    .duration-selector {
      display: flex;
      gap: 0.5rem;
      align-items: center;
      margin-bottom: 1rem;
    }

    .duration-label {
      font-size: 0.75rem;
      color: #a0a0a0;
    }

    .duration-options {
      display: flex;
      gap: 0.25rem;
      flex: 1;
    }

    .duration-option {
      padding: 0.5rem 0.75rem;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.05);
      color: #e0e0e0;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.75rem;
    }

    .duration-option:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    .duration-option.selected {
      background: var(--context-primary, #10b981);
      border-color: var(--context-primary, #10b981);
      color: #000;
    }

    .clear-button {
      width: 100%;
      padding: 0.75rem;
      border: 1px solid rgba(255, 152, 0, 0.5);
      border-radius: 8px;
      background: rgba(255, 152, 0, 0.1);
      color: #ffa726;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.875rem;
      font-weight: 600;
    }

    .clear-button:hover {
      background: rgba(255, 152, 0, 0.2);
      border-color: rgba(255, 152, 0, 0.7);
    }
  `

  static properties = {
    contextState: { type: Object },
    status: { type: String },
    selectedDuration: { type: Number },
    showControls: { type: Boolean }
  }

  constructor() {
    super()
    this.contextState = null
    this.status = 'loading'
    this.selectedDuration = 120 // Default: 2 hours
    this.showControls = false // Collapsed by default
    this.fetchRetries = 0
    this.maxFetchRetries = 10 // Max 10 retries (5 seconds total)
  }

  connectedCallback() {
    super.connectedCallback()

    // Listen to context changes
    window.addEventListener('context-change', this.handleContextChange.bind(this))

    // Initial fetch
    this.fetchContext()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    window.removeEventListener('context-change', this.handleContextChange.bind(this))
  }

  handleContextChange(event) {
    console.log('[context-widget] Context changed:', event.detail.context)
    this.contextState = event.detail.context
    this.status = 'ready'
  }

  async fetchContext() {
    const contextService = document.querySelector('context-service')
    if (contextService) {
      this.contextState = contextService.getContextState()
      this.status = contextService.status
      this.fetchRetries = 0 // Reset retry counter on success
    } else if (this.fetchRetries < this.maxFetchRetries) {
      // Retry after 500ms if service not ready yet
      this.fetchRetries++
      console.warn(`[context-widget] context-service not found, retry ${this.fetchRetries}/${this.maxFetchRetries} in 500ms...`)
      setTimeout(() => this.fetchContext(), 500)
    } else {
      // Max retries reached, show error
      console.error('[context-widget] context-service not found after max retries')
      this.status = 'error'
    }
  }

  getModeIcon(mode) {
    const icons = {
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌱'
    }
    return icons[mode] || '🤔'
  }

  getModeName(mode) {
    const names = {
      'cravate': 'Focus Pro',
      'intime': 'Maison',
      'neutre': 'Veille'
    }
    return names[mode] || 'Inconnu'
  }

  formatTimestamp(timestamp) {
    if (!timestamp) return 'Inconnu'
    const date = new Date(timestamp)
    return date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  }

  async setModeOverride(mode) {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'

      // Utiliser csrfService pour les routes protégées
      const response = await csrfService.fetchWithCsrf(`${API_BASE}/context/override`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          mode: mode,
          duration_minutes: this.selectedDuration,
          reason: `Mode manuel ${this.getModeName(mode)}`
        })
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`)
      }

      const data = await response.json()
      console.log(`[context-widget] Mode override set: ${mode} for ${this.selectedDuration} minutes`)

      // Update local state immediately
      this.contextState = data
      this.status = 'ready'

      // Notify context service to refresh
      const contextService = document.querySelector('context-service')
      if (contextService) {
        contextService.fetchContext()
      }
    } catch (error) {
      console.error('[context-widget] Failed to set mode override:', error)
    }
  }

  async clearOverride() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'

      // Utiliser csrfService pour les routes protégées
      const response = await csrfService.fetchWithCsrf(`${API_BASE}/context/clear`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        }
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`)
      }

      const data = await response.json()
      console.log('[context-widget] Override cleared')

      // Update local state
      this.contextState = data
      this.status = 'ready'

      // Notify context service to refresh
      const contextService = document.querySelector('context-service')
      if (contextService) {
        contextService.fetchContext()
      }
    } catch (error) {
      console.error('[context-widget] Failed to clear override:', error)
    }
  }

  selectDuration(minutes) {
    this.selectedDuration = minutes
  }

  render() {
    if (this.status === 'loading') {
      return html`
        <div class="widget-container">
          <div class="loading">Chargement du contexte...</div>
        </div>
      `
    }

    if (this.status === 'error' || !this.contextState) {
      return html`
        <div class="widget-container">
          <div class="error">Impossible de charger le contexte</div>
        </div>
      `
    }

    const { mode, reason, confidence, changed_at, manual_override } = this.contextState

    return html`
      <div class="widget-container mode-${mode}">
        <div class="mode-display">
          <div class="mode-icon">${this.getModeIcon(mode)}</div>
          <div class="mode-info">
            <div class="mode-title">${this.getModeName(mode)}</div>
            <div class="mode-reason">${reason}</div>
          </div>
          <div class="mode-badge">${mode}</div>
        </div>

        <div class="mode-details">
          <div class="detail-item">
            <div class="detail-label">Changé:</div>
            <div class="detail-value">${this.formatTimestamp(changed_at)}</div>
          </div>
          <div class="detail-item">
            <div class="detail-label">Confiance:</div>
            <div class="detail-value">${Math.round(confidence * 100)}%</div>
          </div>
          <div class="confidence-bar">
            <div class="confidence-fill" style="width: ${confidence * 100}%"></div>
          </div>
        </div>

        ${manual_override ? html`
          <div style="margin-top: 0.5rem; padding: 0.5rem; background: rgba(255, 217, 61, 0.1); border: 1px solid rgba(255, 217, 61, 0.3); border-radius: 6px;">
            <div style="font-size: 0.625rem; color: #ffd93d;">
              ⚠️ Override manuel: ${manual_override.reason}
            </div>
          </div>
        ` : ''}

        <div class="manual-controls">
          <button class="controls-toggle" @click="${() => this.showControls = !this.showControls}">
            <span>Contrôle Manuel</span>
            <span>${this.showControls ? '▲' : '▼'}</span>
          </button>

          <div class="controls-content ${this.showControls ? '' : 'hidden'}">
            <div class="mode-buttons">
            <div
              class="mode-button ${mode === 'cravate' ? 'active' : ''}"
              @click="${() => this.setModeOverride('cravate')}"
            >
              <div class="mode-button-icon">👔</div>
              <div class="mode-button-label">Focus Pro</div>
            </div>

            <div
              class="mode-button ${mode === 'intime' ? 'active' : ''}"
              @click="${() => this.setModeOverride('intime')}"
            >
              <div class="mode-button-icon">🏡</div>
              <div class="mode-button-label">Maison</div>
            </div>

            <div
              class="mode-button ${mode === 'neutre' ? 'active' : ''}"
              @click="${() => this.setModeOverride('neutre')}"
            >
              <div class="mode-button-icon">🌱</div>
              <div class="mode-button-label">Veille</div>
            </div>
          </div>

          <div class="duration-selector">
            <div class="duration-label">Durée:</div>
            <div class="duration-options">
              <div
                class="duration-option ${this.selectedDuration === 60 ? 'selected' : ''}"
                @click="${() => this.selectDuration(60)}"
              >1h</div>
              <div
                class="duration-option ${this.selectedDuration === 120 ? 'selected' : ''}"
                @click="${() => this.selectDuration(120)}"
              >2h</div>
              <div
                class="duration-option ${this.selectedDuration === 240 ? 'selected' : ''}"
                @click="${() => this.selectDuration(240)}"
              >4h</div>
              <div
                class="duration-option ${this.selectedDuration === 480 ? 'selected' : ''}"
                @click="${() => this.selectDuration(480)}"
              >8h</div>
            </div>
          </div>

            ${manual_override ? html`
              <button class="clear-button" @click="${() => this.clearOverride()}">
                🔄 Annuler Override Manuel
              </button>
            ` : ''}
          </div>
        </div>
      </div>
    `
  }
}

customElements.define('context-widget', ContextWidget)

export { ContextWidget }
