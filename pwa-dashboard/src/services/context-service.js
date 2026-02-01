/**
 * Service Contexte Symbion
 *
 * Gère la détection et l'affichage du mode contextuel actuel
 * Modes: Cravate (👔 pro), Intime (🏡 home), Neutre (🌱 idle)
 */

import { LitElement } from 'lit'
import authService from './auth-service.js'

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
    this.retryCount = 0
    this.maxRetries = 10 // Max 10 retries (5 seconds total)
  }

  connectedCallback() {
    super.connectedCallback()

    // Listen for login success event
    window.addEventListener('login-success', () => {
      console.log('[context-service] User logged in, fetching context...')
      this.fetchContext()

      // Start polling only after successful login
      if (!this.pollInterval) {
        this.pollInterval = setInterval(() => {
          this.fetchContext()
        }, 30000)
      }
    })

    // Listen for external context-change events (from context-engine-page)
    this._externalContextHandler = (event) => {
      if (event.detail?.context) {
        console.log('[context-service] External context change received:', event.detail.context.mode)
        this.currentMode = event.detail.context.mode
        this.contextState = event.detail.context
        // Apply theme immediately
        if (event.detail.context.theme) {
          this.applyTheme(event.detail.context.theme)
        }
      }
    }
    document.body.addEventListener('context-change', this._externalContextHandler)

    // Check if already logged in (use authService to verify)
    if (authService.isAuthenticated()) {
      console.log('[context-service] User already authenticated, fetching context...')
      this.fetchContext()

      // Poll context every 30 seconds (same as backend detection interval)
      this.pollInterval = setInterval(() => {
        this.fetchContext()
      }, 30000)
    } else {
      console.log('[context-service] No active session, waiting for login event...')
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.pollInterval) {
      clearInterval(this.pollInterval)
      this.pollInterval = null
    }
    if (this._externalContextHandler) {
      document.body.removeEventListener('context-change', this._externalContextHandler)
    }
  }

  async fetchContext() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        if (this.retryCount < this.maxRetries) {
          this.retryCount++
          console.warn(`[context-service] API service not available, retry ${this.retryCount}/${this.maxRetries}...`)
          setTimeout(() => this.fetchContext(), 500)
        } else {
          console.error('[context-service] API service not available after max retries')
          this.status = 'error'
        }
        return
      }

      // Reset retry count on successful API service connection
      this.retryCount = 0

      const context = await apiService.request('/context/current')

      if (context && (context.mode_slug || context.mode)) {
        const previousMode = this.currentMode
        const previousState = this.contextState
        // Prefer mode_slug (dynamic) over mode (legacy enum)
        this.currentMode = context.mode_slug || context.mode?.toLowerCase()
        this.contextState = context
        this.status = 'ready'

        console.log(`[context-service] Mode: ${this.currentMode} (${context.reason})`)

        // Apply theme (always, even on first load)
        this.applyTheme(context.theme)

        // Notify mode change if it changed OR if this is the first load (previousState was null)
        if (previousMode !== this.currentMode || previousState === null) {
          if (previousState === null) {
            console.log(`[context-service] Initial context loaded: ${this.currentMode}`)
          } else {
            console.log(`[context-service] Mode changed: ${previousMode} → ${this.currentMode}`)
          }
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
      // Legacy modes
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌱',
      // New dynamic modes
      'pro': '👔',
      'focus': '🎯',
      'maison': '🏡',
      'veille': '🌱'
    }
    return icons[this.currentMode] || '🤔'
  }

  getModeName() {
    const names = {
      // Legacy modes
      'cravate': 'Focus Pro',
      'intime': 'Maison',
      'neutre': 'Veille',
      // New dynamic modes
      'pro': 'Pro',
      'focus': 'Focus',
      'maison': 'Maison',
      'veille': 'Veille'
    }
    return names[this.currentMode] || 'Inconnu'
  }

  getTheme() {
    return this.contextState?.theme || null
  }

  /**
   * Attendre que le contexte soit prêt avec timeout
   * Résout la race condition où les widgets chargent avant le contexte
   *
   * @param {number} timeout - Timeout en ms (défaut: 2000ms)
   * @returns {Promise<Object|null>} - Le contexte ou null si timeout
   */
  async waitForContextReady(timeout = 2000) {
    // Si déjà prêt, retourner immédiatement
    if (this.status === 'ready' && this.contextState) {
      console.log('[context-service] Context already ready')
      return this.contextState
    }

    // Attendre avec timeout
    return new Promise((resolve) => {
      const timeoutId = setTimeout(() => {
        console.warn(`[context-service] ⏱️ Context timeout after ${timeout}ms, proceeding with default`)
        resolve(null) // Timeout = continuer sans contexte
      }, timeout)

      // Écouter l'événement context-change
      const onContextChange = (event) => {
        clearTimeout(timeoutId)
        window.removeEventListener('context-change', onContextChange)
        console.log('[context-service] ✅ Context ready:', event.detail.context.mode)
        resolve(event.detail.context)
      }

      window.addEventListener('context-change', onContextChange)

      // Si le contexte devient prêt pendant qu'on écoute
      if (this.status === 'ready' && this.contextState) {
        clearTimeout(timeoutId)
        window.removeEventListener('context-change', onContextChange)
        resolve(this.contextState)
      }
    })
  }
}

customElements.define('context-service', ContextService)

export { ContextService }
