/**
 * Service API Symbion
 * 
 * Interface avec l'API REST du kernel Symbion
 * Gère l'authentification et les erreurs automatiquement
 */

import { LitElement } from 'lit'

class ApiService extends LitElement {
  static properties = {
    status: { type: String },
    baseUrl: { type: String },
    apiKey: { type: String }
  }
  
  constructor() {
    super()
    this.status = 'loading'
    this.baseUrl = window.SYMBION_CONFIG?.API_BASE || '/api'
    this.apiKey = 's3cr3t-42' // TODO: Depuis config ou env
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
    const config = {
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': this.apiKey,
        ...options.headers
      },
      ...options
    }
    
    try {
      const response = await fetch(url, config)
      
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
      // Network errors (fetch failed completely)
      if (error.name === 'TypeError' || error.message.includes('Failed to fetch')) {
        console.error(`❌ Network error [${endpoint}]:`, error)
        this.updateStatus('offline')
      }
      
      throw error
    }
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
    return await this.request(`/plugins/${name}/start`, { method: 'POST' })
  }
  
  async stopPlugin(name) {
    return await this.request(`/plugins/${name}/stop`, { method: 'POST' })
  }
  
  async restartPlugin(name) {
    return await this.request(`/plugins/${name}/restart`, { method: 'POST' })
  }
  
  async getHosts() {
    return await this.request('/hosts')
  }
  
  async getHost(id) {
    return await this.request(`/hosts/${id}`)
  }
  
  async wakeHost(hostId) {
    return await this.request(`/wake?host_id=${encodeURIComponent(hostId)}`, { method: 'POST' })
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
    return await this.request(`/ports/memo${query}`)
  }
  
  async createNote(note) {
    return await this.request('/ports/memo', {
      method: 'POST',
      body: JSON.stringify(note)
    })
  }
  
  async updateNote(id, updates) {
    return await this.request(`/ports/memo/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates)
    })
  }
  
  async deleteNote(id) {
    return await this.request(`/ports/memo/${id}`, {
      method: 'DELETE'
    })
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