/**
 * Service d'authentification Symbion
 *
 * Token JWT stocké dans IndexedDB (persistant, survit fermeture onglet/app).
 * Fetch interceptor injecte automatiquement le header Authorization.
 * Le SW (quand actif) ajoute une couche de protection supplémentaire.
 *
 * - Session persistante (IndexedDB)
 * - Fetch interceptor (injection auth automatique)
 * - Events auth:login, auth:logout, auth:expired
 */

import { getApiBase } from './config.js'
const API_BASE = getApiBase()

// ── IndexedDB helpers ──────────────────────────────────────────────────
const DB_NAME = 'symbion-auth'
const DB_VER = 1
const STORE = 'session'

function _openDB() {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(DB_NAME, DB_VER)
    r.onupgradeneeded = () => r.result.createObjectStore(STORE)
    r.onsuccess = () => resolve(r.result)
    r.onerror = () => reject(r.error)
  })
}

async function _dbPut(key, value) {
  try {
    const db = await _openDB()
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite')
      tx.objectStore(STORE).put(value, key)
      tx.oncomplete = resolve
      tx.onerror = reject
    })
  } catch (e) {
    console.warn('[auth] IndexedDB put failed:', e)
  }
}

async function _dbGet(key) {
  try {
    const db = await _openDB()
    return new Promise((resolve) => {
      const tx = db.transaction(STORE, 'readonly')
      const r = tx.objectStore(STORE).get(key)
      r.onsuccess = () => resolve(r.result ?? null)
      r.onerror = () => resolve(null)
    })
  } catch (e) {
    console.warn('[auth] IndexedDB get failed:', e)
    return null
  }
}

async function _dbClear() {
  try {
    const db = await _openDB()
    return new Promise((resolve) => {
      const tx = db.transaction(STORE, 'readwrite')
      tx.objectStore(STORE).clear()
      tx.oncomplete = resolve
      tx.onerror = resolve
    })
  } catch (e) {
    console.warn('[auth] IndexedDB clear failed:', e)
  }
}

// ── AuthService ────────────────────────────────────────────────────────

class AuthService extends EventTarget {
  constructor() {
    super()
    this.userInfo = null
    this.loginTime = null
    this.refreshTimer = null
    this._token = null
    this._ready = false
    this._readyPromise = this._restore()
    this._installFetchInterceptor()
  }

  /**
   * Attendre que la session ait été restaurée
   */
  async whenReady() {
    return this._readyPromise
  }

  /**
   * Intercepteur fetch — injecte Authorization sur requêtes API same-origin
   */
  _installFetchInterceptor() {
    const originalFetch = window.fetch
    const self = this

    window.fetch = function(input, init = {}) {
      if (!self._token) {
        return originalFetch.call(window, input, init)
      }

      const url = typeof input === 'string' ? new URL(input, window.location.origin) : new URL(input.url)

      // Seulement same-origin
      if (url.origin !== window.location.origin) {
        return originalFetch.call(window, input, init)
      }

      // Exclure les assets statiques
      if (/\.(js|css|html|png|jpg|svg|ico|woff2?|ttf|map)$/.test(url.pathname) ||
          url.pathname === '/' ||
          url.pathname.startsWith('/@') ||
          url.pathname.startsWith('/node_modules/')) {
        return originalFetch.call(window, input, init)
      }

      // Pas sur login
      if (url.pathname === '/auth/login') {
        return originalFetch.call(window, input, init)
      }

      // Déjà un header Authorization → ne pas écraser
      const existingHeaders = init.headers || {}
      if (existingHeaders['Authorization'] || existingHeaders['authorization']) {
        return originalFetch.call(window, input, init)
      }

      // Injecter le token
      const headers = { ...existingHeaders, 'Authorization': `Bearer ${self._token}` }
      return originalFetch.call(window, input, { ...init, headers })
    }
  }

  /**
   * Restaurer la session depuis IndexedDB (persistant)
   */
  async _restore() {
    // Migration legacy sessionStorage
    const legacyToken = sessionStorage.getItem('symbion_auth_token')
    const legacyUserInfo = sessionStorage.getItem('symbion_user_info')
    if (legacyToken && legacyUserInfo) {
      console.log('[auth] Migrating legacy sessionStorage to IndexedDB')
      try {
        const userInfo = JSON.parse(legacyUserInfo)
        this._token = legacyToken
        this.userInfo = userInfo
        this.loginTime = Date.now()
        await _dbPut('token', legacyToken)
        await _dbPut('userInfo', JSON.stringify(userInfo))
        sessionStorage.removeItem('symbion_auth_token')
        sessionStorage.removeItem('symbion_user_info')
        sessionStorage.removeItem('symbion_login_time')
        this._ready = true
        this.scheduleTokenRefresh()
        this.dispatchEvent(new CustomEvent('auth:login', {
          detail: { username: userInfo.username, role: userInfo.role }
        }))
        return
      } catch (e) {
        console.warn('[auth] Migration failed:', e)
        sessionStorage.removeItem('symbion_auth_token')
        sessionStorage.removeItem('symbion_user_info')
        sessionStorage.removeItem('symbion_login_time')
      }
    }

    // Restaurer depuis IndexedDB
    try {
      const token = await _dbGet('token')
      const rawUserInfo = await _dbGet('userInfo')

      if (token && rawUserInfo) {
        const userInfo = JSON.parse(rawUserInfo)

        // Vérifier que le token n'est pas expiré
        const now = Math.floor(Date.now() / 1000)
        if (userInfo.expires_at && userInfo.expires_at > now) {
          this._token = token
          this.userInfo = userInfo
          this.loginTime = Date.now()
          console.log('[auth] Session restored from IndexedDB:', userInfo.username)
          this.scheduleTokenRefresh()
          this.dispatchEvent(new CustomEvent('auth:login', {
            detail: { username: userInfo.username, role: userInfo.role }
          }))
        } else {
          console.log('[auth] Stored session expired, clearing')
          await _dbClear()
        }
      }
    } catch (e) {
      console.warn('[auth] Failed to restore session:', e)
    }

    this._ready = true
  }

  /**
   * Sauvegarder token + userInfo dans IndexedDB
   */
  async _persist(token, userInfo) {
    this._token = token
    await _dbPut('token', token)
    await _dbPut('userInfo', JSON.stringify(userInfo))

    // Aussi envoyer au SW si disponible (double protection)
    if (navigator.serviceWorker?.controller) {
      navigator.serviceWorker.controller.postMessage({
        type: 'AUTH_STORE',
        data: { token, userInfo }
      })
    }
  }

  /**
   * Effacer token de partout
   */
  async _clearToken() {
    this._token = null
    await _dbClear()

    if (navigator.serviceWorker?.controller) {
      navigator.serviceWorker.controller.postMessage({ type: 'AUTH_CLEAR' })
    }
  }

  /**
   * Login utilisateur
   */
  async login(username, password, totpCode = null, rememberDevice = false) {
    try {
      console.log('[auth] Sending login request for:', username)

      const body = { username, password }
      if (totpCode) body.totp_code = totpCode
      if (rememberDevice) body.remember_device = rememberDevice

      const headers = { 'Content-Type': 'application/json' }

      const deviceToken = localStorage.getItem('symbion_device_token')
      if (deviceToken) {
        headers['X-Device-Token'] = deviceToken
      }

      const response = await fetch(`${API_BASE}/auth/login`, {
        method: 'POST',
        headers,
        body: JSON.stringify(body)
      })

      if (!response.ok) {
        let errorMessage = 'Authentication failed'
        try {
          const errorData = await response.json()
          if (errorData.error) errorMessage = errorData.error
        } catch (_) { /* ignore */ }
        throw new Error(errorMessage)
      }

      const data = await response.json()

      const userInfo = {
        username: data.username,
        role: data.role,
        expires_at: data.expires_at
      }

      // Persister token dans IndexedDB + SW
      await this._persist(data.token, userInfo)

      this.userInfo = userInfo
      this.loginTime = Date.now()

      if (data.device_token) {
        localStorage.setItem('symbion_device_token', data.device_token)
      }

      this.scheduleTokenRefresh()

      this.dispatchEvent(new CustomEvent('auth:login', {
        detail: { username: data.username, role: data.role }
      }))
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
      if (this.userInfo) {
        await fetch(`${API_BASE}/auth/logout`, { method: 'POST' })
      }
    } catch (error) {
      console.warn('[auth] Logout API call failed:', error)
    } finally {
      const username = this.userInfo?.username || 'unknown'

      await this._clearToken()

      sessionStorage.removeItem('symbion_boot_completed')
      localStorage.removeItem('symbion_device_token')

      this.userInfo = null
      this.loginTime = null
      if (this.refreshTimer) {
        clearTimeout(this.refreshTimer)
        this.refreshTimer = null
      }

      this.dispatchEvent(new CustomEvent('auth:logout', {
        detail: { username }
      }))
      console.log('[auth] Logged out:', username)
    }
  }

  /**
   * Vérifier si l'utilisateur est authentifié
   */
  isAuthenticated() {
    if (!this.userInfo) return false

    const now = Math.floor(Date.now() / 1000)
    if (this.userInfo.expires_at <= now) {
      console.warn('[auth] Token expired')
      this.userInfo = null
      this._token = null
      this._clearToken()
      this.dispatchEvent(new Event('auth:expired'))
      return false
    }

    return true
  }

  /**
   * Vérifier la validité du token auprès du serveur
   */
  async verifySession() {
    if (!this.userInfo) return false

    try {
      const response = await fetch(`${API_BASE}/auth/verify`)

      if (!response.ok) {
        this.userInfo = null
        await this._clearToken()
        this.dispatchEvent(new Event('auth:expired'))
        return false
      }

      const data = await response.json()
      console.log('[auth] Session verified:', data.username)
      return true

    } catch (error) {
      console.error('[auth] Session verification error:', error)
      return false
    }
  }

  /**
   * Obtenir le token JWT pour WebSocket
   * (WS ne passe pas par le fetch interceptor)
   */
  async getTokenForWebSocket() {
    return this._token
  }

  /**
   * @deprecated — le fetch interceptor injecte le header automatiquement
   */
  getToken() {
    return this._token
  }

  getCurrentUser() { return this.userInfo }

  async getSessionInfo() {
    if (!this.userInfo) throw new Error('Not authenticated')
    const response = await fetch(`${API_BASE}/auth/session`)
    if (!response.ok) throw new Error('Failed to get session info')
    return await response.json()
  }

  scheduleTokenRefresh() {
    if (!this.userInfo) return
    if (this.refreshTimer) clearTimeout(this.refreshTimer)

    const now = Math.floor(Date.now() / 1000)
    const timeUntilExpiry = this.userInfo.expires_at - now
    const refreshTime = Math.max(0, (timeUntilExpiry - 1800) * 1000)

    if (refreshTime > 0) {
      console.log(`[auth] Token expires in ${Math.floor(timeUntilExpiry / 60)}min, refresh in ${Math.floor(refreshTime / 60000)}min`)
      this.refreshTimer = setTimeout(() => {
        this.dispatchEvent(new Event('auth:refresh-needed'))
      }, refreshTime)
    }
  }

  /**
   * @deprecated — le fetch interceptor gère ça
   */
  getAuthHeader() { return {} }

  getLoginTime() { return this.loginTime }
}

const authService = new AuthService()

export default authService
