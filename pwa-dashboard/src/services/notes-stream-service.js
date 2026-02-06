/**
 * WebSocket Notes Streaming Service
 *
 * Service de chargement progressif des notes via WebSocket
 * Chaque note est reçue et affichée en temps réel
 */

import { LitElement } from 'lit'
import authService from './auth-service.js'

class NotesStreamService extends LitElement {
  static properties = {
    connected: { type: Boolean },
    loading: { type: Boolean }
  }

  constructor() {
    super()
    this.connected = false
    this.loading = false
    this.ws = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 5
    this.reconnectDelay = 1000
  }

  get wsUrl() {
    // Construire l'URL WebSocket basée sur la config
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.SYMBION_CONFIG?.API_BASE?.replace(/^https?:\/\//, '') || `${window.location.hostname}:8443`

    // WebSockets ne supportent pas les headers custom
    // Use JWT token if authenticated, otherwise try explicit API key (no fallback)
    const token = authService.getToken()
    const apiKey = window.SYMBION_CONFIG?.API_KEY

    const url = `${protocol}//${host}/ws/notes/stream`

    if (token) {
      // Prefer JWT token for authentication
      return `${url}?token=${encodeURIComponent(token)}`
    } else if (apiKey) {
      // Fallback to explicit API key if configured
      return `${url}?api_key=${encodeURIComponent(apiKey)}`
    } else {
      // No auth available - connection will likely fail with 401
      console.warn('[notes-stream] No authentication available for WebSocket')
      return url
    }
  }

  /**
   * Connexion au WebSocket
   */
  connect() {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      console.log('[notes-stream] WebSocket already connected/connecting')
      return
    }

    try {
      console.log(`[notes-stream] Connecting to ${this.wsUrl}`)
      this.ws = new WebSocket(this.wsUrl)

      this.ws.onopen = () => {
        console.log('[notes-stream] WebSocket connected')
        this.connected = true
        this.reconnectAttempts = 0

        this.dispatchEvent(new CustomEvent('ws-connected', {
          bubbles: true,
          composed: true
        }))
      }

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)
          this.handleMessage(data)
        } catch (e) {
          console.error('[notes-stream] Failed to parse message:', e)
        }
      }

      this.ws.onerror = (error) => {
        console.error('[notes-stream] WebSocket error:', error)

        this.dispatchEvent(new CustomEvent('ws-error', {
          detail: { error },
          bubbles: true,
          composed: true
        }))
      }

      this.ws.onclose = () => {
        console.log('[notes-stream] WebSocket closed')
        this.connected = false
        this.loading = false

        this.dispatchEvent(new CustomEvent('ws-closed', {
          bubbles: true,
          composed: true
        }))

        // Tentative de reconnexion automatique
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
          this.reconnectAttempts++
          const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)
          console.log(`[notes-stream] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`)

          setTimeout(() => this.connect(), delay)
        } else {
          console.error('[notes-stream] Max reconnection attempts reached')
        }
      }
    } catch (e) {
      console.error('[notes-stream] Failed to create WebSocket:', e)
    }
  }

  /**
   * Déconnexion du WebSocket
   */
  disconnect() {
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.connected = false
    this.loading = false
    this.reconnectAttempts = this.maxReconnectAttempts // Empêcher reconnexion auto
  }

  /**
   * Demander la liste des notes avec filtres optionnels
   */
  async loadNotes(filters = {}) {
    if (!this.connected) {
      // Attendre la connexion si pas encore connecté
      await new Promise((resolve, reject) => {
        if (this.connected) {
          resolve()
          return
        }

        const onConnected = () => {
          this.removeEventListener('ws-connected', onConnected)
          this.removeEventListener('ws-error', onError)
          resolve()
        }

        const onError = () => {
          this.removeEventListener('ws-connected', onConnected)
          this.removeEventListener('ws-error', onError)
          reject(new Error('Failed to connect to WebSocket'))
        }

        this.addEventListener('ws-connected', onConnected)
        this.addEventListener('ws-error', onError)

        this.connect()

        // Timeout après 10 secondes
        setTimeout(() => {
          if (!this.connected) {
            this.removeEventListener('ws-connected', onConnected)
            this.removeEventListener('ws-error', onError)
            reject(new Error('WebSocket connection timeout'))
          }
        }, 10000)
      })
    }

    this.loading = true

    // Envoyer la requête avec filtres
    const request = filters
    this.ws.send(JSON.stringify(request))

    console.log('[notes-stream] Requested notes with filters:', filters)
  }

  /**
   * Traiter les messages WebSocket
   */
  handleMessage(data) {
    console.log('[notes-stream] Received message:', data.type)

    switch (data.type) {
      case 'note':
      case 'note_item':  // Format MQTT
        // Note individuelle reçue
        this.dispatchEvent(new CustomEvent('note-received', {
          detail: { note: data.note },
          bubbles: true,
          composed: true
        }))
        break

      case 'end':
      case 'list_end':  // Format MQTT
        // Fin du stream
        this.loading = false
        this.dispatchEvent(new CustomEvent('notes-complete', {
          detail: {
            totalCount: data.total_count,
            receivedCount: data.received_count
          },
          bubbles: true,
          composed: true
        }))
        console.log(`[notes-stream] Stream complete: ${data.received_count || data.total_count}/${data.total_count} notes`)
        break

      case 'error':
        // Erreur serveur
        this.loading = false
        this.dispatchEvent(new CustomEvent('notes-error', {
          detail: { error: data.error },
          bubbles: true,
          composed: true
        }))
        console.error('[notes-stream] Server error:', data.error)
        break

      default:
        console.warn('[notes-stream] Unknown message type:', data.type)
    }
  }

  connectedCallback() {
    super.connectedCallback()
    // Auto-connect au montage
    this.connect()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    // Déconnexion propre au démontage
    this.disconnect()
  }
}

customElements.define('notes-stream-service', NotesStreamService)

// Export singleton instance
const notesStreamService = document.createElement('notes-stream-service')
document.body.appendChild(notesStreamService)

export default notesStreamService
