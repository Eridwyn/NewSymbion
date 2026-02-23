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
  }

  toggle() {
    this.current = this.current === 'dark' ? 'light' : 'dark'
    localStorage.setItem(STORAGE_KEY, this.current)
    this._apply(this.current)
    document.body.dispatchEvent(new CustomEvent('theme-changed', {
      detail: { theme: this.current },
      bubbles: true
    }))
  }

  _apply(theme) {
    document.documentElement.setAttribute('data-theme', theme)
  }
}

const themeService = new ThemeService()
export default themeService
