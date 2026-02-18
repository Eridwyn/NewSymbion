/**
 * Service MQTT Symbion
 * 
 * Connexion temps réel aux événements MQTT du système
 * Écoute les heartbeats, health updates, etc.
 */

import { LitElement } from 'lit'
import mqtt from 'mqtt'

class MqttService extends LitElement {
  static properties = {
    status: { type: String },
    client: { type: Object }
  }
  
  constructor() {
    super()
    this.status = 'connecting'
    this.client = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 5
  }
  
  connectedCallback() {
    super.connectedCallback()
    this.connect()
  }
  
  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.client) {
      this.client.end()
    }
  }
  
  connect() {
    // MQTT_BROKER contient déjà le protocole et le port (ex: wss://symbion.local:9001)
    const brokerUrl = window.SYMBION_CONFIG?.MQTT_BROKER || 'ws://localhost:9001'

    console.log('🔌 Connecting to MQTT broker:', brokerUrl)

    this.client = mqtt.connect(brokerUrl, {
      clientId: `symbion-dashboard-${Math.random().toString(16).substr(2, 8)}`,
      reconnectPeriod: 3000,
      connectTimeout: 30000,
      keepalive: 15,  // Ping every 15 seconds to keep connection alive
      clean: true,
      resubscribe: true,
      protocolVersion: 4  // MQTT 3.1.1
    })
    
    this.client.on('connect', this.handleConnect.bind(this))
    this.client.on('message', this.handleMessage.bind(this))
    this.client.on('error', this.handleError.bind(this))
    this.client.on('close', this.handleClose.bind(this))
    this.client.on('offline', this.handleOffline.bind(this))
    this.client.on('reconnect', this.handleReconnect.bind(this))
  }
  
  handleConnect() {
    console.log('✅ MQTT Connected')
    this.status = 'online'
    this.reconnectAttempts = 0
    this.updateStatus('online')

    // S'abonner aux topics Symbion
    this.subscribeToTopics()
  }
  
  handleMessage(topic, message) {
    try {
      const payload = JSON.parse(message.toString())
      console.log(`📨 MQTT [${topic}]:`, payload)
      
      // Router les messages vers les handlers appropriés
      this.routeMessage(topic, payload)
      
    } catch (error) {
      console.warn(`⚠️ Failed to parse MQTT message from ${topic}:`, error)
    }
  }
  
  handleError(error) {
    console.error('❌ MQTT Error:', error)
    this.updateStatus('offline')
  }

  handleClose() {
    console.warn('⚠️ MQTT Connection closed')
    this.updateStatus('offline')
  }

  handleOffline() {
    console.warn('⚠️ MQTT Offline')
    this.updateStatus('offline')
  }
  
  handleReconnect() {
    this.reconnectAttempts++
    console.log(`🔄 MQTT Reconnecting... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`)
    this.updateStatus('connecting')
    
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('❌ Max reconnection attempts reached')
      this.client.end()
      this.updateStatus('offline')
    }
  }
  
  subscribeToTopics() {
    const topics = [
      'symbion/kernel/health@v1',
      'symbion/hosts/heartbeat@v2',
      'symbion/hosts/wake@v1',
      'symbion/notes/response@v1',
      'symbion/dashboard/context@v1',
      'symbion/dashboard/agents/+',  // Wildcard pour topics individuels par agent (ex: agents/abc123@v1)
      // 'symbion/dashboard/agents@v1', // ANCIEN: tous les agents dans un seul message
      'symbion/dashboard/health@v1',
      'symbion/dashboard/notes@v1',
      'symbion/dashboard/stats@v1',
      'symbion/dashboard/pattern@v1',
      'symbion/notifications/sent@v1',  // Notifications push pour toasts PWA (kernel publie ici)
      // Freebox plugin topics
      'symbion/freebox/presence/#',
      'symbion/freebox/connection/metrics',
      // SSL plugin topics
      'symbion/ssl/summary',
      'symbion/ssl/+'
    ]

    // Storage pour agréger les agents reçus individuellement
    this.agentsCache = this.agentsCache || {}
    
    topics.forEach(topic => {
      this.client.subscribe(topic, (error) => {
        if (error) {
          console.error(`❌ Failed to subscribe to ${topic}:`, error)
        } else {
          console.log(`📥 Subscribed to ${topic}`)
        }
      })
    })
  }
  
  routeMessage(topic, payload) {
    // NOUVEAU: Gestion des topics individuels par agent (wildcard)
    // Match topics like: symbion/dashboard/agents/abc123@v1 -> extract "abc123"
    const agentTopicMatch = topic.match(/^symbion\/dashboard\/agents\/([^@]+)@v\d+$/)
    if (agentTopicMatch) {
      const agentId = agentTopicMatch[1]
      this.handleIndividualAgent(agentId, payload)
      return
    }

    // Router les messages vers les composants appropriés
    switch (topic) {
      case 'symbion/kernel/health@v1':
        this.handleSystemHealth(payload)
        break

      case 'symbion/hosts/heartbeat@v2':
        this.handleHostHeartbeat(payload)
        break

      case 'symbion/hosts/wake@v1':
        this.handleWakeCommand(payload)
        break

      case 'symbion/notes/response@v1':
        this.handleNotesResponse(payload)
        break

      // Nouveaux topics dashboard
      case 'symbion/dashboard/context@v1':
        this.handleDashboardContext(payload)
        break

      // ANCIEN: tous les agents dans un seul message
      // case 'symbion/dashboard/agents@v1':
      //   this.handleDashboardAgents(payload)
      //   break

      case 'symbion/dashboard/health@v1':
        this.handleDashboardHealth(payload)
        break

      case 'symbion/dashboard/notes@v1':
        this.handleDashboardNotes(payload)
        break

      case 'symbion/dashboard/stats@v1':
        this.handleDashboardStats(payload)
        break

      case 'symbion/dashboard/pattern@v1':
        this.handleDashboardPattern(payload)
        break

      case 'symbion/notifications/sent@v1':
        this.handleNotificationReceived(payload)
        break

      case 'symbion/freebox/connection/metrics':
        this.handleFreeboxConnection(payload)
        break

      case 'symbion/ssl/summary':
        this.handleSslSummary(payload)
        break

      default:
        // Handle Freebox presence topics (wildcard)
        if (topic.startsWith('symbion/freebox/presence/')) {
          this.handleFreeboxPresence(topic, payload)
        } else if (topic.startsWith('symbion/ssl/') && topic !== 'symbion/ssl/summary') {
          this.handleSslDomain(topic, payload)
        } else {
          console.log(`🤷 Unhandled topic: ${topic}`)
        }
    }
  }

  handleFreeboxPresence(topic, payload) {
    console.log('📡 [mqtt] Freebox presence:', topic, payload)
    // Cache presence data for late subscribers
    this._freeboxPresenceCache = this._freeboxPresenceCache || {}
    this._freeboxPresenceCache[topic] = payload
    this.dispatchEvent(new CustomEvent('freebox-presence', {
      detail: { topic, payload },
      bubbles: true,
      composed: true
    }))
  }

  handleFreeboxConnection(payload) {
    console.log('📡 [mqtt] Freebox connection:', payload)
    // Cache connection data for late subscribers
    this._freeboxConnectionCache = payload
    this.dispatchEvent(new CustomEvent('freebox-connection', {
      detail: { payload },
      bubbles: true,
      composed: true
    }))
  }

  // Get cached Freebox data for widgets that subscribe late
  getFreeboxCache() {
    return {
      presence: this._freeboxPresenceCache || {},
      connection: this._freeboxConnectionCache || null
    }
  }

  handleSslSummary(payload) {
    console.log('🔒 [mqtt] SSL summary:', payload)
    // Cache summary data for late subscribers
    this._sslSummaryCache = payload
    this.dispatchEvent(new CustomEvent('ssl-summary', {
      detail: { payload },
      bubbles: true,
      composed: true
    }))
  }

  handleSslDomain(topic, payload) {
    console.log('🔒 [mqtt] SSL domain:', topic, payload)
    // Cache domain data for late subscribers
    this._sslDomainsCache = this._sslDomainsCache || {}
    const domainId = topic.replace('symbion/ssl/', '')
    this._sslDomainsCache[domainId] = payload
    this.dispatchEvent(new CustomEvent('ssl-domain', {
      detail: { topic, domainId, payload },
      bubbles: true,
      composed: true
    }))
  }

  // Get cached SSL data for widgets that subscribe late
  getSslCache() {
    return {
      summary: this._sslSummaryCache || null,
      domains: this._sslDomainsCache || {}
    }
  }

  handleSystemHealth(health) {
    this.dispatchEvent(new CustomEvent('system-health', {
      detail: { health },
      bubbles: true
    }))
  }
  
  handleHostHeartbeat(heartbeat) {
    this.dispatchEvent(new CustomEvent('host-heartbeat', {
      detail: { heartbeat },
      bubbles: true
    }))
  }
  
  handleWakeCommand(wakeCommand) {
    this.dispatchEvent(new CustomEvent('wake-command', {
      detail: { wakeCommand },
      bubbles: true
    }))
  }
  
  handleNotesResponse(response) {
    this.dispatchEvent(new CustomEvent('notes-response', {
      detail: { response },
      bubbles: true
    }))
  }

  // === Handlers Dashboard Events ===

  handleDashboardContext(context) {
    console.log('📨 Dashboard context update:', context)
    this.dispatchEvent(new CustomEvent('dashboard-context', {
      detail: { context },
      bubbles: true,
      composed: true
    }))
  }

  // NOUVEAU: Gestion des agents individuels (topics séparés)
  handleIndividualAgent(agentId, agent) {
    // Stocker/mettre à jour l'agent dans le cache
    this.agentsCache = this.agentsCache || {}
    this.agentsCache[agentId] = { ...agent, _lastSeen: Date.now() }

    // LRU eviction: max 50 agents en cache
    const keys = Object.keys(this.agentsCache)
    if (keys.length > 50) {
      const oldest = keys.reduce((a, b) =>
        (this.agentsCache[a]._lastSeen || 0) < (this.agentsCache[b]._lastSeen || 0) ? a : b
      )
      delete this.agentsCache[oldest]
    }

    // Convertir le cache en array et dispatcher (même format qu'avant)
    const agents = Object.values(this.agentsCache)
    console.log(`📨 Agent update: ${agentId} (total: ${agents.length})`)
    this.dispatchEvent(new CustomEvent('dashboard-agents', {
      detail: { agents },
      bubbles: true,
      composed: true
    }))
  }

  // ANCIEN: tous les agents dans un seul message
  handleDashboardAgents(agents) {
    console.log('📨 Dashboard agents update:', agents)
    this.dispatchEvent(new CustomEvent('dashboard-agents', {
      detail: { agents },
      bubbles: true,
      composed: true
    }))
  }

  handleDashboardHealth(health) {
    console.log('📨 Dashboard health update:', health)
    this.dispatchEvent(new CustomEvent('dashboard-health', {
      detail: { health },
      bubbles: true,
      composed: true
    }))
  }

  handleDashboardNotes(note) {
    console.log('📨 Dashboard note created:', note)
    this.dispatchEvent(new CustomEvent('dashboard-note-created', {
      detail: { note },
      bubbles: true,
      composed: true
    }))
  }

  handleDashboardStats(stats) {
    console.log('📨 Dashboard stats update:', stats)
    this.dispatchEvent(new CustomEvent('dashboard-stats', {
      detail: { stats },
      bubbles: true,
      composed: true
    }))
  }

  handleDashboardPattern(pattern) {
    console.log('📨 Dashboard pattern detected:', pattern)
    this.dispatchEvent(new CustomEvent('dashboard-pattern', {
      detail: { pattern },
      bubbles: true,
      composed: true
    }))
  }

  handleNotificationReceived(payload) {
    // Le kernel envoie { notification: {...}, timestamp } - extraire la notification
    const notification = payload.notification || payload
    console.log('🔔 Notification received from MQTT:', notification)
    const event = new CustomEvent('notification-received', {
      detail: { notification },
      bubbles: true,
      composed: true
    })
    console.log('🔔 Dispatching notification-received event to document.body')
    this.dispatchEvent(event)
  }

  updateStatus(status) {
    this.status = status
    this.dispatchEvent(new CustomEvent('status-change', {
      detail: { status },
      bubbles: true
    }))
  }
  
  // ===== API publique =====
  
  isConnected() {
    return this.status === 'online'
  }
  
  publish(topic, payload) {
    if (this.client && this.isConnected()) {
      const message = typeof payload === 'string' ? payload : JSON.stringify(payload)
      this.client.publish(topic, message)
      console.log(`📤 Published to ${topic}:`, payload)
    } else {
      console.warn('⚠️ Cannot publish: MQTT not connected')
    }
  }
  
  subscribe(topic, callback) {
    if (this.client) {
      this.client.subscribe(topic, (error) => {
        if (error) {
          console.error(`❌ Failed to subscribe to ${topic}:`, error)
        } else {
          console.log(`📥 Subscribed to ${topic}`)
          if (callback) callback()
        }
      })
    }
  }
}

customElements.define('mqtt-service', MqttService)

export { MqttService }