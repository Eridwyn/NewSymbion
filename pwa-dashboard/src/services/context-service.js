/**
 * Service Contexte Symbion
 *
 * Gère la détection et l'affichage du mode contextuel actuel
 * Modes chargés dynamiquement depuis /v1/modes
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
    this.currentMode = 'veille'
    this.contextState = null
    this.status = 'loading'
    this.pollInterval = null
    this.retryCount = 0
    this.maxRetries = 10 // Max 10 retries (5 seconds total)
    this.dynamicModes = new Map() // slug → { name, icon, theme }
  }

  connectedCallback() {
    super.connectedCallback()

    // Listen for login success event
    window.addEventListener('login-success', () => {
      console.log('[context-service] User logged in, fetching context...')
      this.fetchModes() // Load dynamic modes from API
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
        const ctx = event.detail.context
        const mode = ctx.mode_slug || ctx.mode
        console.log('[context-service] External context change received:', mode)
        this.currentMode = mode
        this.contextState = ctx
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
      this.fetchModes()
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

  async fetchModes() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return

      const modes = await apiService.request('/modes')
      if (Array.isArray(modes)) {
        this.dynamicModes.clear()
        modes.forEach(m => {
          this.dynamicModes.set(m.slug, {
            name: m.name,
            icon: m.icon,
            theme: m.theme
          })
        })
        console.log(`[context-service] Loaded ${this.dynamicModes.size} dynamic modes`)
      }
    } catch (error) {
      console.warn('[context-service] Failed to fetch modes:', error)
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

        // Sync appearance.theme from kernel (automation-driven changes)
        this._syncAppearanceTheme()
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

    // Calculate and set logo filter variables from primary color
    const { hue, saturation, isGray } = this.hexToHSL(theme.primary)

    // For CSS filter chain: invert(1) sepia(1) then hue-rotate
    // Sepia gives ~30-40° base, so we adjust the rotation
    const filterHue = isGray ? 0 : (hue - 40 + 360) % 360
    const filterSaturation = isGray ? 0 : Math.min(saturation / 25, 5)
    const filterBrightness = isGray ? 1.3 : 1.1

    document.documentElement.style.setProperty('--context-logo-hue', `${filterHue}deg`)
    document.documentElement.style.setProperty('--context-logo-saturation', filterSaturation.toString())
    document.documentElement.style.setProperty('--context-logo-brightness', filterBrightness.toString())

    console.log(`[context-service] Theme applied: ${theme.primary} (hue: ${hue}°)`)
  }

  // Convert hex color to HSL values
  hexToHSL(hex) {
    // Remove # if present
    hex = hex.replace('#', '')

    const r = parseInt(hex.substring(0, 2), 16) / 255
    const g = parseInt(hex.substring(2, 4), 16) / 255
    const b = parseInt(hex.substring(4, 6), 16) / 255

    const max = Math.max(r, g, b)
    const min = Math.min(r, g, b)
    const l = (max + min) / 2
    const d = max - min

    // Check if it's a gray (low saturation)
    const isGray = d < 0.1

    let h = 0
    let s = 0

    if (d !== 0) {
      s = d / (1 - Math.abs(2 * l - 1))

      switch (max) {
        case r:
          h = 60 * (((g - b) / d) % 6)
          break
        case g:
          h = 60 * ((b - r) / d + 2)
          break
        case b:
          h = 60 * ((r - g) / d + 4)
          break
      }
    }

    if (h < 0) h += 360

    return {
      hue: Math.round(h),
      saturation: Math.round(s * 100),
      lightness: Math.round(l * 100),
      isGray
    }
  }

  notifyModeChange(context) {
    this.dispatchEvent(new CustomEvent('context-change', {
      detail: { context },
      bubbles: true,
      composed: true
    }))
  }

  async _syncAppearanceTheme() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const data = await apiService.request('/v1/intelligence/features')
      const feat = data?.features?.find(f => f.feature_id === 'appearance.theme')
      // API returns { type: "String", value: "dark" } (serde adjacently tagged)
      const kernelTheme = feat?.value?.value || feat?.value?.String
      if (!kernelTheme) return

      const { default: themeService } = await import('./theme-service.js')
      if (themeService.current !== kernelTheme) {
        console.log(`[context-service] Syncing theme from kernel: ${themeService.current} → ${kernelTheme}`)
        themeService.current = kernelTheme
        localStorage.setItem('symbion_theme', kernelTheme)
        themeService._apply(kernelTheme)
        document.body.dispatchEvent(new CustomEvent('theme-changed', {
          detail: { theme: kernelTheme },
          bubbles: true
        }))
      }
    } catch (_) {
      // Silencieux — sync non critique
    }
  }

  // ===== API publique =====

  getCurrentMode() {
    return this.currentMode
  }

  getContextState() {
    return this.contextState
  }

  getModeIcon() {
    // Dynamic modes from API (preferred)
    const dynamic = this.dynamicModes.get(this.currentMode)
    if (dynamic?.icon) return dynamic.icon

    // Fallback for legacy/offline
    const fallback = { 'pro': '👔', 'focus': '🎯', 'maison': '🏡', 'veille': '🌱' }
    return fallback[this.currentMode] || '🤔'
  }

  getModeName() {
    // Dynamic modes from API (preferred)
    const dynamic = this.dynamicModes.get(this.currentMode)
    if (dynamic?.name) return dynamic.name

    // Fallback for legacy/offline
    const fallback = { 'pro': 'Pro', 'focus': 'Focus', 'maison': 'Maison', 'veille': 'Veille' }
    return fallback[this.currentMode] || 'Inconnu'
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
        console.log('[context-service] Context ready:', event.detail.context.mode_slug || event.detail.context.mode)
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
