/**
 * Service d'authentification Symbion
 *
 * Gestion des sessions utilisateur avec JWT
 * - Login/Logout
 * - Vérification token
 * - Storage sécurisé (sessionStorage)
 * - Auto-refresh token
 * - Events auth:login, auth:logout, auth:expired
 */

import { notifyError } from '../utils/notification-helper.js'

const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
const TOKEN_KEY = 'symbion_auth_token'
const USER_KEY = 'symbion_user_info'
const LOGIN_TIME_KEY = 'symbion_login_time'

class AuthService extends EventTarget {
  constructor() {
    super()
    this.token = null
    this.userInfo = null
    this.loginTime = null
    this.refreshTimer = null

    // Charger token depuis sessionStorage au démarrage
    this.loadFromStorage()
  }

  /**
   * Charger token et user info depuis sessionStorage
   */
  loadFromStorage() {
    const token = sessionStorage.getItem(TOKEN_KEY)
    const userInfo = sessionStorage.getItem(USER_KEY)
    const loginTime = sessionStorage.getItem(LOGIN_TIME_KEY)

    if (token && userInfo) {
      try {
        this.token = token
        this.userInfo = JSON.parse(userInfo)
        this.loginTime = loginTime ? parseInt(loginTime) : null
        console.log('[auth] Session restored from storage:', this.userInfo.username)
        this.scheduleTokenRefresh()
      } catch (error) {
        console.error('[auth] Failed to parse stored user info:', error)
        this.clearStorage()
      }
    }
  }

  /**
   * Sauvegarder token et user info dans sessionStorage
   */
  saveToStorage() {
    if (this.token && this.userInfo) {
      sessionStorage.setItem(TOKEN_KEY, this.token)
      sessionStorage.setItem(USER_KEY, JSON.stringify(this.userInfo))
      if (this.loginTime) {
        sessionStorage.setItem(LOGIN_TIME_KEY, this.loginTime.toString())
      }
    }
  }

  /**
   * Effacer token et user info de sessionStorage
   * Note: device_token dans localStorage n'est PAS supprimé (survit au logout pour remember device 30j)
   */
  clearStorage() {
    sessionStorage.removeItem(TOKEN_KEY)
    sessionStorage.removeItem(USER_KEY)
    sessionStorage.removeItem(LOGIN_TIME_KEY)
    sessionStorage.removeItem('symbion_boot_completed') // Reset boot pour prochaine session
    // Note: Ne pas supprimer symbion_device_token (localStorage) - il persiste 30 jours
    this.token = null
    this.userInfo = null
    this.loginTime = null

    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = null
    }
  }

  /**
   * Login utilisateur
   * @param {string} username
   * @param {string} password
   * @param {string} [totpCode] - Code TOTP optionnel (MFA)
   * @param {boolean} [rememberDevice] - Se souvenir de l'appareil pendant 30 jours
   * @returns {Promise<{token, username, role, expires_at}>}
   */
  async login(username, password, totpCode = null, rememberDevice = false) {
    try {
      console.log('[auth] Sending login request for:', username, 'with MFA:', !!totpCode, 'remember:', rememberDevice)

      const body = { username, password }
      if (totpCode) {
        body.totp_code = totpCode
      }
      if (rememberDevice) {
        body.remember_device = rememberDevice
      }

      // Préparer headers avec device token si existant (localStorage)
      const headers = {
        'Content-Type': 'application/json'
      }

      const deviceToken = localStorage.getItem('symbion_device_token')
      if (deviceToken) {
        headers['X-Device-Token'] = deviceToken
        console.log('[auth] Sending device token:', deviceToken.substring(0, 8) + '...')
      }

      const response = await fetch(`${API_BASE}/auth/login`, {
        method: 'POST',
        headers,
        body: JSON.stringify(body)
      })

      console.log('[auth] Response status:', response.status, 'OK:', response.ok)

      if (!response.ok) {
        console.error('[auth] Login failed - response not OK, status:', response.status)

        // Extraire le message d'erreur du backend
        let errorMessage = 'Authentication failed'
        try {
          const errorData = await response.json()
          if (errorData.error) {
            errorMessage = errorData.error
          }
        } catch (e) {
          // [Audit] Si pas de JSON, utiliser le message par défaut
          console.warn('[auth] Failed to parse error response:', e)
        }

        throw new Error(errorMessage)
      }

      const data = await response.json()
      console.log('[auth] Login response received for:', data.username, 'role:', data.role)

      this.token = data.token
      this.userInfo = {
        username: data.username,
        role: data.role,
        expires_at: data.expires_at
      }
      this.loginTime = Date.now() // Enregistrer le timestamp du login

      // Stocker le device token si renvoyé par le backend (remember_device=true)
      if (data.device_token) {
        localStorage.setItem('symbion_device_token', data.device_token)
        console.log('[auth] Device token saved to localStorage:', data.device_token.substring(0, 8) + '... (30 days)')
      }

      this.saveToStorage()
      this.scheduleTokenRefresh()

      // Émettre événement de login
      this.dispatchEvent(new CustomEvent('auth:login', {
        detail: { username: data.username, role: data.role }
      }))

      // Émettre également un événement global pour les autres services
      window.dispatchEvent(new CustomEvent('login-success', {
        detail: { username: data.username, role: data.role }
      }))

      console.log('[auth] Login successful:', data.username)
      return data

    } catch (error) {
      console.error('[auth] Login error:', error)
      throw error
    }
  }

  /**
   * Logout utilisateur
   */
  async logout() {
    try {
      // Appeler l'API de logout (même si JWT stateless)
      if (this.token) {
        await fetch(`${API_BASE}/auth/logout`, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${this.token}`
          },
          credentials: 'include' // Envoyer les cookies pour potentiellement les invalider
        })
      }
    } catch (error) {
      console.warn('[auth] Logout API call failed:', error)
      // [Audit] Notify user that server-side logout failed
      notifyError('Erreur déconnexion', 'Session locale fermée, serveur injoignable', 'auth')
    } finally {
      const username = this.userInfo?.username || 'unknown'

      this.clearStorage()

      // Émettre événement de logout
      this.dispatchEvent(new CustomEvent('auth:logout', {
        detail: { username }
      }))

      console.log('[auth] Logged out:', username)
    }
  }

  /**
   * Vérifier si l'utilisateur est authentifié
   * @returns {boolean}
   */
  isAuthenticated() {
    if (!this.token || !this.userInfo) {
      return false
    }

    // Vérifier si le token n'est pas expiré
    const now = Math.floor(Date.now() / 1000)
    if (this.userInfo.expires_at <= now) {
      console.warn('[auth] Token expired')
      this.clearStorage()
      this.dispatchEvent(new Event('auth:expired'))
      return false
    }

    return true
  }

  /**
   * Vérifier la validité du token auprès du serveur
   * @returns {Promise<boolean>}
   */
  async verifySession() {
    if (!this.token) {
      return false
    }

    try {
      const response = await fetch(`${API_BASE}/auth/verify`, {
        headers: {
          'Authorization': `Bearer ${this.token}`
        },
        credentials: 'include' // Envoyer les cookies pour vérification device trust
      })

      if (!response.ok) {
        this.clearStorage()
        this.dispatchEvent(new Event('auth:expired'))
        return false
      }

      const data = await response.json()
      console.log('[auth] Session verified:', data.username)
      return true

    } catch (error) {
      console.error('[auth] Session verification error:', error)
      this.clearStorage()
      return false
    }
  }

  /**
   * Obtenir le token JWT actuel
   * @returns {string|null}
   */
  getToken() {
    return this.token
  }

  /**
   * Obtenir les infos utilisateur courant
   * @returns {Object|null}
   */
  getCurrentUser() {
    return this.userInfo
  }

  /**
   * Obtenir les infos de session détaillées depuis le serveur
   * @returns {Promise<Object>}
   */
  async getSessionInfo() {
    if (!this.token) {
      throw new Error('Not authenticated')
    }

    const response = await fetch(`${API_BASE}/auth/session`, {
      headers: {
        'Authorization': `Bearer ${this.token}`
      },
      credentials: 'include' // Envoyer les cookies
    })

    if (!response.ok) {
      throw new Error('Failed to get session info')
    }

    return await response.json()
  }

  /**
   * Planifier le refresh automatique du token
   * Refresh 30 minutes avant expiration
   */
  scheduleTokenRefresh() {
    if (!this.userInfo) {
      return
    }

    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
    }

    const now = Math.floor(Date.now() / 1000)
    const expiresAt = this.userInfo.expires_at
    const timeUntilExpiry = expiresAt - now
    const refreshTime = Math.max(0, (timeUntilExpiry - 1800) * 1000) // 30 min avant

    if (refreshTime > 0) {
      console.log(`[auth] Token will expire in ${Math.floor(timeUntilExpiry / 60)} minutes`)
      console.log(`[auth] Scheduled refresh in ${Math.floor(refreshTime / 60000)} minutes`)

      this.refreshTimer = setTimeout(() => {
        console.log('[auth] Token refresh needed')
        this.dispatchEvent(new Event('auth:refresh-needed'))
        // TODO: Implémenter refresh token si nécessaire
      }, refreshTime)
    } else {
      console.warn('[auth] Token already expired or expiring soon')
    }
  }

  /**
   * Obtenir le header Authorization pour les requêtes API
   * @returns {Object}
   */
  getAuthHeader() {
    if (!this.token) {
      return {}
    }

    return {
      'Authorization': `Bearer ${this.token}`
    }
  }

  /**
   * Obtenir le timestamp du login (en millisecondes)
   * @returns {number|null}
   */
  getLoginTime() {
    return this.loginTime
  }
}

// Instance singleton
const authService = new AuthService()

export default authService
