/**
 * Service de gestion du theme (dark/light)
 *
 * Persiste le choix dans localStorage et dispatche
 * un evenement 'theme-changed' sur document.body
 * pour que les composants puissent reagir.
 */

const STORAGE_KEY = 'symbion_theme'

class ThemeService {
  constructor() {
    this.current = localStorage.getItem(STORAGE_KEY) || 'dark'
    this._apply(this.current)
    // Notify kernel of initial theme after auth is ready
    document.addEventListener('login-success', () => this._notifyKernel(this.current), { once: true })
  }

  toggle() {
    this.current = this.current === 'dark' ? 'light' : 'dark'
    localStorage.setItem(STORAGE_KEY, this.current)
    this._apply(this.current)
    this._notifyKernel(this.current)
    document.body.dispatchEvent(new CustomEvent('theme-changed', {
      detail: { theme: this.current },
      bubbles: true
    }))
  }

  _apply(theme) {
    document.documentElement.setAttribute('data-theme', theme)
  }

  async _notifyKernel(theme) {
    try {
      const { default: csrfService } = await import('./csrf-service.js')
      await csrfService.fetchWithCsrf('/v1/intelligence/features', {
        method: 'POST',
        body: JSON.stringify({
          feature_id: 'appearance.theme',
          value: theme,
          source: 'pwa-dashboard',
          ttl_seconds: 0
        })
      })
    } catch (_) {
      // Silencieux — le theme reste local meme si le kernel est down
    }
  }
}

const themeService = new ThemeService()
export default themeService
