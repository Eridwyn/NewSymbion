/**
 * Composant principal du dashboard Symbion
 * 
 * Interface adaptative qui charge dynamiquement les widgets
 * basés sur les manifestes des plugins actifs
 */

import { LitElement, html, css } from 'lit'
import '../services/api-service.js'
import '../services/mqtt-service.js'
import '../widgets/system-health-widget.js'
import '../widgets/hosts-widget.js'
import '../widgets/plugins-widget.js'
import '../widgets/notes-widget.js'

class DashboardApp extends LitElement {
  static styles = css`
    :host {
      display: block;
      min-height: 100vh;
      background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%);
      color: #e0e0e0;
    }
    
    .header {
      background: rgba(0, 0, 0, 0.5);
      backdrop-filter: blur(10px);
      border-bottom: 1px solid #333;
      padding: 1rem 2rem;
      position: sticky;
      top: 0;
      z-index: 100;
    }
    
    .header h1 {
      font-size: 1.8em;
      font-weight: 300;
      margin: 0;
      background: linear-gradient(90deg, #007acc, #00d4aa);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    
    .status-bar {
      display: flex;
      gap: 1rem;
      align-items: center;
      margin-top: 0.5rem;
      font-size: 0.9em;
      opacity: 0.8;
    }
    
    .status-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    
    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      transition: all 0.3s ease;
    }
    
    .status-dot.online { background: #00d4aa; }
    .status-dot.offline { background: #ff6b6b; }
    .status-dot.polling { background: #007acc; }
    .status-dot.loading { 
      background: #ffd93d; 
      animation: pulse 1.5s infinite;
    }
    
    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }
    
    .main-content {
      padding: 2rem;
      max-width: 1400px;
      margin: 0 auto;
    }
    
    .widgets-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
      gap: 2rem;
      margin-bottom: 2rem;
    }
    
    .widget-container {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 12px;
      padding: 1.5rem;
      backdrop-filter: blur(10px);
      transition: all 0.3s ease;
    }
    
    .widget-container:hover {
      border-color: rgba(0, 122, 204, 0.3);
      transform: translateY(-2px);
      box-shadow: 0 10px 30px rgba(0, 122, 204, 0.1);
    }
    
    .error-message {
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: 8px;
      padding: 1rem;
      margin: 1rem 0;
      color: #ff6b6b;
    }
    
    @media (max-width: 768px) {
      .header { padding: 1rem; }
      .main-content { padding: 1rem; }
      .widgets-grid { grid-template-columns: 1fr; gap: 1rem; }
    }
  `
  
  static properties = {
    connected: { type: Boolean },
    mqttStatus: { type: String },
    apiStatus: { type: String },
    systemHealth: { type: Object },
    plugins: { type: Array },
    error: { type: String }
  }
  
  constructor() {
    super()
    this.connected = false
    this.mqttStatus = 'connecting'
    this.apiStatus = 'loading'
    this.systemHealth = null
    this.plugins = []
    this.error = null
    
    this.apiService = null
    this.mqttService = null
  }
  
  async connectedCallback() {
    super.connectedCallback()
    
    try {
      // Initialiser les services
      await this.initializeServices()
      
      // Charger les données initiales
      await this.loadInitialData()
      
      // Démarrer les mises à jour temps réel
      this.startRealtimeUpdates()
      
    } catch (error) {
      console.error('❌ Dashboard initialization failed:', error)
      this.error = `Erreur d'initialisation: ${error.message}`
    }
  }
  
  async initializeServices() {
    console.log('🔧 Initializing services...')
    
    // Service API
    this.apiService = document.createElement('api-service')
    this.apiService.addEventListener('status-change', this.handleApiStatus.bind(this))
    
    // Service MQTT  
    this.mqttService = document.createElement('mqtt-service')
    this.mqttService.addEventListener('status-change', this.handleMqttStatus.bind(this))
    this.mqttService.addEventListener('system-health', this.handleSystemHealth.bind(this))
    
    document.body.appendChild(this.apiService)
    document.body.appendChild(this.mqttService)
  }
  
  async loadInitialData() {
    console.log('📊 Loading initial data...')
    
    try {
      // Charger l'état du système
      const health = await this.apiService.getSystemHealth()
      this.systemHealth = health
      
      // Charger les plugins
      const plugins = await this.apiService.getPlugins()
      this.plugins = plugins
      
      this.apiStatus = 'online'
      this.connected = true
      
      console.log('✅ Initial data loaded')
      
    } catch (error) {
      console.error('❌ Failed to load initial data:', error)
      this.apiStatus = 'offline'
      this.error = `Impossible de charger les données: ${error.message}`
    }
  }
  
  startRealtimeUpdates() {
    console.log('⚡ Starting realtime updates...')
    
    // Mise à jour périodique des données
    setInterval(async () => {
      if (this.apiStatus === 'online') {
        try {
          const health = await this.apiService.getSystemHealth()
          this.systemHealth = health
          
          const plugins = await this.apiService.getPlugins()
          this.plugins = plugins
        } catch (error) {
          console.warn('⚠️ Periodic update failed:', error)
        }
      }
    }, 10000) // 10 secondes
  }
  
  handleApiStatus(event) {
    this.apiStatus = event.detail.status
    if (event.detail.status === 'offline') {
      this.connected = false
    }
    this.requestUpdate()
  }
  
  handleMqttStatus(event) {
    this.mqttStatus = event.detail.status
    this.requestUpdate()
  }
  
  handleSystemHealth(event) {
    this.systemHealth = event.detail.health
    this.requestUpdate()
  }
  
  render() {
    return html`
      <div class="header">
        <h1>🧬 Symbion Dashboard</h1>
        <div class="status-bar">
          <div class="status-indicator">
            <div class="status-dot ${this.apiStatus}"></div>
            <span>API: ${this.apiStatus}</span>
          </div>
          <div class="status-indicator">
            <div class="status-dot ${this.mqttStatus}"></div>
            <span>MQTT: ${this.mqttStatus}</span>
          </div>
          ${this.systemHealth ? html`
            <div class="status-indicator">
              <span>Uptime: ${this.formatUptime(this.systemHealth.uptime_seconds)}</span>
            </div>
          ` : ''}
        </div>
      </div>
      
      <div class="main-content">
        ${this.error ? html`
          <div class="error-message">
            ❌ ${this.error}
          </div>
        ` : ''}
        
        <div class="widgets-grid">
          <!-- Widget santé système -->
          <div class="widget-container">
            <system-health-widget 
              .health="${this.systemHealth}"
              .connected="${this.connected}">
            </system-health-widget>
          </div>
          
          <!-- Widget plugins -->
          <div class="widget-container">
            <plugins-widget 
              .plugins="${this.plugins}"
              .apiService="${this.apiService}">
            </plugins-widget>
          </div>
          
          <!-- Widget hosts (sera ajouté quand le plugin hosts sera actif) -->
          <div class="widget-container">
            <hosts-widget 
              .connected="${this.connected}"
              .apiService="${this.apiService}">
            </hosts-widget>
          </div>
          
          <!-- Widget notes -->
          <div class="widget-container">
            <notes-widget 
              .apiService="${this.apiService}"
              .connected="${this.connected}">
            </notes-widget>
          </div>
        </div>
      </div>
    `
  }
  
  formatUptime(seconds) {
    if (!seconds) return 'N/A'
    
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    
    if (days > 0) {
      return `${days}j ${hours}h ${minutes}m`
    } else if (hours > 0) {
      return `${hours}h ${minutes}m`
    } else {
      return `${minutes}m`
    }
  }
}

customElements.define('dashboard-app', DashboardApp)