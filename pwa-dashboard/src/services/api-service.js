/**
 * Service API Symbion
 *
 * Interface avec l'API REST du kernel Symbion
 * Gère l'authentification et les erreurs automatiquement
 */

import { LitElement } from 'lit'
import authService from './auth-service.js'
import csrfService from './csrf-service.js'

class ApiService extends LitElement {
  static properties = {
    status: { type: String },
    baseUrl: { type: String },
    apiKey: { type: String }
  }
  
  constructor() {
    super()
    this.status = 'loading'
    // Ne pas définir baseUrl ici, sera lazy-loaded
    this._baseUrl = null
    this._apiKey = null
  }

  // Lazy getters pour résoudre la config au moment de l'utilisation
  get baseUrl() {
    if (!this._baseUrl) {
      this._baseUrl = window.SYMBION_CONFIG?.API_BASE || `https://${window.location.hostname}:8443`
    }
    return this._baseUrl
  }

  get apiKey() {
    if (this._apiKey === undefined) {
      // Only use explicitly configured API key - NO FALLBACK
      // Security: A hardcoded fallback is not security, it's public knowledge
      this._apiKey = import.meta.env.VITE_SYMBION_API_KEY || window.SYMBION_CONFIG?.API_KEY || null
    }
    return this._apiKey
  }
  
  connectedCallback() {
    super.connectedCallback()
    this.checkConnection()
    
    // Vérification périodique de la santé API pour éviter les faux offline
    this.healthCheckInterval = setInterval(() => {
      this.checkConnection()
    }, 15000) // Toutes les 15 secondes
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval)
      this.healthCheckInterval = null
    }
  }
  
  async checkConnection() {
    try {
      await this.request('/health')
      this.updateStatus('online')
    } catch (error) {
      console.warn('⚠️ API not available:', error)
      this.updateStatus('offline')
    }
  }
  
  updateStatus(status) {
    this.status = status
    this.dispatchEvent(new CustomEvent('status-change', {
      detail: { status },
      bubbles: true
    }))
  }
  
  async request(endpoint, options = {}) {
    const url = `${this.baseUrl}${endpoint}`

    // Inclure le token JWT si l'utilisateur est authentifié
    const authHeader = authService.getAuthHeader()

    // Support timeout (30s par défaut pour notes, sinon pas de timeout)
    const timeout = options.timeout || null
    const controller = timeout ? new AbortController() : null
    const timeoutId = timeout ? setTimeout(() => controller.abort(), timeout) : null

    // Build headers - only include x-api-key if explicitly configured
    const headers = {
      'Content-Type': 'application/json',
      ...authHeader,  // Ajoute Authorization: Bearer {token} si présent
      ...options.headers
    }

    // Only add API key header if explicitly configured (no fallback)
    if (this.apiKey) {
      headers['x-api-key'] = this.apiKey
    }

    const config = {
      headers,
      signal: controller?.signal,
      ...options
    }

    // Remove timeout from config to avoid passing it to fetch
    delete config.timeout

    console.log(`[api-service] request: ${options.method || 'GET'} ${url}${timeout ? ` (timeout: ${timeout}ms)` : ''}`)
    console.log(`[api-service] baseUrl: ${this.baseUrl}`)
    console.log(`[api-service] auth header:`, authHeader)
    console.log(`[api-service] config:`, config)

    try {
      const response = await fetch(url, config)

      // Clear timeout on success
      if (timeoutId) clearTimeout(timeoutId)
      console.log(`[api-service] response status: ${response.status} ${response.statusText}`)
      
      if (!response.ok) {
        // Différencier les erreurs de connection vs erreurs applicatives
        if (response.status >= 500 && response.status <= 599) {
          // 5xx = erreur serveur/plugin mais API kernel toujours UP
          console.warn(`⚠️ Server error [${endpoint}] ${response.status}: Likely plugin issue`)
          throw new Error(`HTTP ${response.status}: ${response.statusText}`)
        } else if (response.status === 0 || response.status >= 400 && response.status < 500) {
          // 0/4xx = vraie erreur de connection/auth
          console.error(`❌ API connection failed [${endpoint}] ${response.status}`)
          this.updateStatus('offline')
          throw new Error(`HTTP ${response.status}: ${response.statusText}`)
        }
        
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }
      
      // Requête réussie = API certainement online
      if (this.status !== 'online') {
        console.log('✅ API back online')
        this.updateStatus('online')
      }
      
      const contentType = response.headers.get('content-type')
      if (contentType && contentType.includes('application/json')) {
        return await response.json()
      }
      
      return await response.text()
      
    } catch (error) {
      // Clear timeout on error
      if (timeoutId) clearTimeout(timeoutId)

      // Handle abort/timeout
      if (error.name === 'AbortError') {
        console.error(`⏱️ Request timeout [${endpoint}]`)
        throw new Error(`Request timeout after ${timeout}ms`)
      }

      // Network errors (fetch failed completely)
      if (error.name === 'TypeError' || error.message.includes('Failed to fetch')) {
        console.error(`❌ Network error [${endpoint}]:`, error)
        this.updateStatus('offline')
      }

      throw error
    }
  }

  /**
   * Valide qu'une réponse API est un array, sinon retourne le fallback
   * Ajoute des logs de debug si le format est incorrect
   */
  validateArrayResponse(data, endpoint, fallback = []) {
    if (Array.isArray(data)) {
      return data
    }

    // Log détaillé si format incorrect
    console.warn(
      `[api-service] ⚠️ ${endpoint} returned non-array data:`,
      typeof data === 'object' ? JSON.stringify(data).substring(0, 100) : data,
      '- Using fallback:', fallback
    )

    return fallback
  }

  // ===== Endpoints spécifiques =====

  async getSystemHealth() {
    return await this.request('/system/health')
  }
  
  async getHealth() {
    return await this.request('/health')
  }
  
  async getPlugins() {
    return await this.request('/plugins')
  }
  
  async getPlugin(name) {
    return await this.request(`/plugins/${name}`)
  }
  
  async startPlugin(name) {
    const url = `${this.baseUrl}/v1/plugins/${encodeURIComponent(name)}/start`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }

  async stopPlugin(name) {
    const url = `${this.baseUrl}/v1/plugins/${encodeURIComponent(name)}/stop`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }

  async restartPlugin(name) {
    const url = `${this.baseUrl}/v1/plugins/${encodeURIComponent(name)}/restart`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }
  
  async getHosts() {
    return await this.request('/hosts')
  }
  
  async getHost(id) {
    return await this.request(`/hosts/${id}`)
  }
  
  async wakeHost(hostId) {
    return await this.request(`/v1/wake?host_id=${encodeURIComponent(hostId)}`, { method: 'POST' })
  }
  
  async getContracts() {
    return await this.request('/contracts')
  }
  
  async getContract(name) {
    return await this.request(`/contracts/${encodeURIComponent(name)}`)
  }
  
  async getPorts() {
    return await this.request('/ports')
  }
  
  // Notes API (port memo)
  async getNotes(filters = {}) {
    const params = new URLSearchParams()
    Object.entries(filters).forEach(([key, value]) => {
      if (value !== null && value !== undefined && value !== '') {
        params.append(key, value)
      }
    })

    const query = params.toString() ? `?${params.toString()}` : ''
    // Timeout de 30s pour les grandes collections de notes
    return await this.request(`/v1/plugin-api/notes/notes${query}`, { timeout: 30000 })
  }

  async createNote(note) {
    const url = `${this.baseUrl}/v1/plugin-api/notes/notes`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(note)
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }

  async updateNote(id, updates) {
    const url = `${this.baseUrl}/v1/plugin-api/notes/notes/${encodeURIComponent(id)}`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(updates)
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }

  async deleteNote(id) {
    const url = `${this.baseUrl}/v1/plugin-api/notes/notes/${encodeURIComponent(id)}`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }
  
  // ===== Helpers =====
  
  isOnline() {
    return this.status === 'online'
  }
  
  isOffline() {
    return this.status === 'offline'
  }
}

customElements.define('api-service', ApiService)

export { ApiService }