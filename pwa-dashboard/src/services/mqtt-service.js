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
    // TODO: Configure MQTT broker with WebSocket support
    // For now, disable MQTT and use API polling only
    console.log('⚠️ MQTT WebSocket not configured, using API polling only')
    this.status = 'polling'
    this.updateStatus('polling')
    return
    
    const brokerUrl = `ws://${window.SYMBION_CONFIG?.MQTT_BROKER || 'localhost'}:${window.SYMBION_CONFIG?.MQTT_PORT || 9001}`
    
    console.log('🔌 Connecting to MQTT broker:', brokerUrl)
    
    this.client = mqtt.connect(brokerUrl, {
      clientId: `symbion-dashboard-${Math.random().toString(16).substr(2, 8)}`,
      reconnectPeriod: 3000,
      connectTimeout: 10000
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
      'symbion/notes/response@v1'
    ]
    
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
        
      default:
        console.log(`🤷 Unhandled topic: ${topic}`)
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