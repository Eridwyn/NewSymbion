/**
 * Service Contexte Symbion
 *
 * Gère la détection et l'affichage du mode contextuel actuel
 * Modes: Cravate (👔 pro), Intime (🏡 home), Neutre (🌱 idle)
 */

import { LitElement } from 'lit'

class ContextService extends LitElement {
  static properties = {
    currentMode: { type: String },
    contextState: { type: Object },
    status: { type: String }
  }

  constructor() {
    super()
    this.currentMode = 'neutre'
    this.contextState = null
    this.status = 'loading'
    this.pollInterval = null
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchContext()

    // Poll context every 30 seconds (same as backend detection interval)
    this.pollInterval = setInterval(() => {
      this.fetchContext()
    }, 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.pollInterval) {
      clearInterval(this.pollInterval)
      this.pollInterval = null
    }
  }

  async fetchContext() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        console.warn('[context-service] API service not available')
        return
      }

      const context = await apiService.request('/context/current')

      if (context && context.mode) {
        const previousMode = this.currentMode
        this.currentMode = context.mode
        this.contextState = context
        this.status = 'ready'

        console.log(`[context-service] Mode: ${context.mode} (${context.reason})`)

        // Notify theme change if mode changed
        if (previousMode !== this.currentMode) {
          console.log(`[context-service] Mode changed: ${previousMode} → ${this.currentMode}`)
          this.applyTheme(context.theme)
          this.notifyModeChange(context)
        }
      }

    } catch (error) {
      console.error('[context-service] Failed to fetch context:', error)
      this.status = 'error'
    }
  }

  applyTheme(theme) {
    if (!theme) return

    // Apply CSS custom properties for theme
    document.documentElement.style.setProperty('--context-primary', theme.primary)
    document.documentElement.style.setProperty('--context-bg', theme.bg)
    document.documentElement.style.setProperty('--context-accent', theme.accent)

    console.log(`[context-service] Theme applied: ${theme.primary}`)
  }

  notifyModeChange(context) {
    this.dispatchEvent(new CustomEvent('context-change', {
      detail: { context },
      bubbles: true,
      composed: true
    }))
  }

  // ===== API publique =====

  getCurrentMode() {
    return this.currentMode
  }

  getContextState() {
    return this.contextState
  }

  getModeIcon() {
    const icons = {
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌱'
    }
    return icons[this.currentMode] || '🤔'
  }

  getModeName() {
    const names = {
      'cravate': 'Focus Pro',
      'intime': 'Maison',
      'neutre': 'Veille'
    }
    return names[this.currentMode] || 'Inconnu'
  }

  getTheme() {
    return this.contextState?.theme || null
  }
}

customElements.define('context-service', ContextService)

export { ContextService }
