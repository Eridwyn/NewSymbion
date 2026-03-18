/**
 * Service de gestion des nonces CSRF Symbion
 *
 * Gère automatiquement les nonces CSRF pour les requêtes protégées
 * - Fetch automatique de nonces depuis /auth/csrf/nonce
 * - Expiration après 5 minutes (TTL backend)
 * - Refresh automatique avant expiration
 * - Helper pour ajouter X-CSRF-Token aux requêtes
 * - Events csrf:fetched, csrf:expired, csrf:error
 */

import { getApiBase } from './config.js'
const API_BASE = getApiBase()
const NONCE_REFRESH_MARGIN = 30 // Refresh 30s avant expiration

class CsrfService extends EventTarget {
  constructor() {
    super()
    this.currentNonce = null
    this.expiresAt = null
    this.refreshTimer = null
    this.authService = null // Injecté par dashboard-app
  }

  /**
   * Initialiser le service avec référence au AuthService
   */
  setAuthService(authService) {
    this.authService = authService
  }

  /**
   * Obtenir un nonce CSRF valide (fetch si nécessaire)
   * @returns {Promise<string>} Le nonce CSRF ou null si échec
   */
  async getNonce() {
    // Si nonce valide existe, le retourner
    if (this.currentNonce && this.expiresAt && Date.now() < this.expiresAt) {
      return this.currentNonce
    }

    // Sinon, fetch un nouveau nonce
    return await this.fetchNewNonce()
  }

  /**
   * Fetch un nouveau nonce CSRF depuis le backend
   * @returns {Promise<string>} Le nonce CSRF ou null si échec
   */
  async fetchNewNonce() {
    try {
      // Vérifier que l'utilisateur est authentifié
      if (!this.authService || !this.authService.isAuthenticated()) {
        console.warn('[csrf] Cannot fetch nonce - user not authenticated')
        this.dispatchEvent(new CustomEvent('csrf:error', {
          detail: { error: 'Not authenticated' }
        }))
        return null
      }

      // Requête au backend pour obtenir un nonce
      // Le SW injecte automatiquement le header Authorization
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), 10000)
      const response = await fetch(`${API_BASE}/auth/csrf/nonce`, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' },
        signal: controller.signal
      })
      clearTimeout(timeoutId)

      if (!response.ok) {
        const errorText = await response.text()
        console.error('[csrf] Failed to fetch nonce:', response.status, errorText)
        this.dispatchEvent(new CustomEvent('csrf:error', {
          detail: { status: response.status, error: errorText }
        }))
        return null
      }

      const data = await response.json()

      if (!data.nonce || !data.expires_in_seconds) {
        console.error('[csrf] Invalid nonce response:', data)
        return null
      }

      // Stocker le nonce et son expiration
      this.currentNonce = data.nonce
      this.expiresAt = Date.now() + (data.expires_in_seconds * 1000)

      console.log(`[csrf] New nonce fetched: ${this.currentNonce.substring(0, 8)}... (expires in ${data.expires_in_seconds}s)`)

      // Dispatcher event
      this.dispatchEvent(new CustomEvent('csrf:fetched', {
        detail: { nonce: this.currentNonce, expiresAt: this.expiresAt }
      }))

      // Programmer refresh automatique avant expiration
      this.scheduleNonceRefresh(data.expires_in_seconds)

      return this.currentNonce

    } catch (error) {
      console.error('[csrf] Error fetching nonce:', error)
      this.dispatchEvent(new CustomEvent('csrf:error', {
        detail: { error: error.message }
      }))
      return null
    }
  }

  /**
   * Programmer le refresh automatique du nonce avant expiration
   * @param {number} expiresInSeconds - TTL du nonce en secondes
   */
  scheduleNonceRefresh(expiresInSeconds) {
    // Clear timer existant
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
    }

    // Programmer refresh NONCE_REFRESH_MARGIN secondes avant expiration
    const refreshIn = Math.max(0, (expiresInSeconds - NONCE_REFRESH_MARGIN)) * 1000

    this.refreshTimer = setTimeout(async () => {
      console.log('[csrf] Auto-refreshing nonce before expiration')
      await this.fetchNewNonce()
    }, refreshIn)
  }

  /**
   * Invalider le nonce actuel (par ex. après utilisation ou erreur 403)
   */
  invalidateNonce() {
    console.log('[csrf] Nonce invalidated (consumed or expired)')
    this.currentNonce = null
    this.expiresAt = null

    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = null
    }

    this.dispatchEvent(new CustomEvent('csrf:expired'))
  }

  /**
   * Helper pour effectuer une requête protégée par CSRF
   * Auto-fetch nonce si nécessaire et ajoute X-CSRF-Token header
   *
   * @param {string} url - URL de la requête
   * @param {object} options - Options fetch (method, body, etc.)
   * @returns {Promise<Response>} La réponse fetch
   */
  async fetchWithCsrf(url, options = {}) {
    // Obtenir un nonce valide (fetch automatique si nécessaire)
    const nonce = await this.getNonce()

    if (!nonce) {
      throw new Error('Failed to obtain CSRF nonce')
    }

    // Préparer headers avec CSRF token
    const headers = {
      ...options.headers,
      'X-CSRF-Token': nonce
    }

    // Le SW injecte automatiquement le header Authorization

    // Construire URL complète si relative
    const fullUrl = url.startsWith('http') ? url : `${API_BASE}${url}`

    // Ajouter Content-Type si body JSON
    if (options.body && typeof options.body === 'string') {
      headers['Content-Type'] = 'application/json'
    }

    // Effectuer la requête
    try {
      const response = await fetch(fullUrl, {
        ...options,
        headers
      })

      // Les nonces CSRF sont à usage unique - invalider après chaque utilisation
      // (même en cas de succès, le nonce est consommé côté serveur)
      this.invalidateNonce()
      console.log('[csrf] Nonce consumed after request, next request will fetch a fresh one')

      return response

    } catch (error) {
      console.error('[csrf] Request failed:', error)
      throw error
    }
  }

  /**
   * Cleanup lors de logout ou destruction
   */
  cleanup() {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = null
    }
    this.currentNonce = null
    this.expiresAt = null
  }
}

// Export singleton
const csrfService = new CsrfService()
export default csrfService
