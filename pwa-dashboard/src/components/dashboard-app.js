/**
 * Composant principal du dashboard Symbion
 * 
 * Interface adaptative qui charge dynamiquement les widgets
 * basés sur les manifestes des plugins actifs
 */

import { LitElement, html, css } from 'lit'
import authService from '../services/auth-service.js'
import csrfService from '../services/csrf-service.js'
import '../services/api-service.js'
import '../services/mqtt-service.js'
import '../services/agents-service.js'
import '../services/context-service.js'
import '../widgets/system-health-widget.js'
// import '../widgets/hosts-widget.js'  // DEPRECATED: remplacé par agents-network-widget
import '../widgets/plugins-widget.js'
import '../widgets/notes-widget.js'
import '../widgets/agents-network-widget.js'
import '../widgets/agent-control-widget.js'
import '../widgets/context-widget.js'
import '../widgets/context-stats-widget.js'
import '../widgets/context-settings-widget.js'
import './user-settings-page.js'

class DashboardApp extends LitElement {
  static styles = css`
    :host {
      display: block;
      min-height: 100vh;
      background: radial-gradient(ellipse at top, #1a1f35 0%, #0f0f0f 50%, #000000 100%);
      color: #e0e0e0;
    }

    .header {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent) 100%);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      padding: 1.5rem 2rem;
      position: -webkit-sticky;
      position: sticky;
      top: 0;
      z-index: 100;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 1rem;
      transition: all 0.5s ease;
    }

    .header-left {
      flex: 1;
    }

    .header h1 {
      font-size: 2em;
      font-weight: 600;
      margin: 0;
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 70%, #007acc) 50%,
        var(--context-primary, #00d4aa) 100%);
      background-size: 200% 200%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      animation: gradient-shift 3s ease infinite;
      letter-spacing: -0.5px;
      transition: all 0.5s ease;
    }

    @keyframes gradient-shift {
      0%, 100% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
    }

    .status-bar {
      display: flex;
      gap: 1.5rem;
      align-items: center;
      margin-top: 0.75rem;
      font-size: 0.9em;
      font-weight: 500;
    }

    .status-indicator {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 0.4rem 0.8rem;
      background: rgba(255, 255, 255, 0.03);
      border-radius: 20px;
      border: 1px solid rgba(255, 255, 255, 0.05);
      transition: all 0.3s ease;
    }

    .status-indicator:hover {
      background: rgba(255, 255, 255, 0.06);
      border-color: rgba(255, 255, 255, 0.1);
    }

    .status-dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      transition: all 0.3s ease;
      box-shadow: 0 0 10px currentColor;
    }

    .status-dot.online,
    .status-dot.connected {
      background: var(--context-primary, #00d4aa);
      box-shadow: 0 0 15px var(--context-primary, #00d4aa),
                  0 0 25px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      animation: pulse-glow 2s ease-in-out infinite;
    }
    .status-dot.offline {
      background: #ff6b6b;
      box-shadow: 0 0 10px rgba(255, 107, 107, 0.5);
    }
    .status-dot.polling {
      background: #007acc;
      box-shadow: 0 0 15px #007acc;
      animation: pulse-glow 2s ease-in-out infinite;
    }
    .status-dot.loading {
      background: #ffd93d;
      animation: pulse-loading 1s infinite;
    }

    @keyframes pulse-glow {
      0%, 100% {
        transform: scale(1);
        opacity: 1;
      }
      50% {
        transform: scale(1.1);
        opacity: 0.8;
      }
    }

    @keyframes pulse-loading {
      0%, 100% {
        opacity: 1;
        transform: scale(1);
      }
      50% {
        opacity: 0.5;
        transform: scale(0.9);
      }
    }

    /* Clock Display */
    .system-clock {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.6rem 1rem;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 10px;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 0.85em;
      font-weight: 500;
      color: #e0e0e0;
      letter-spacing: 0.5px;
      transition: all 0.3s ease;
    }

    .system-clock:hover {
      background: rgba(255, 255, 255, 0.05);
      border-color: rgba(255, 255, 255, 0.12);
    }

    .system-clock .icon {
      font-size: 1.1em;
    }

    /* User Menu */
    .user-menu {
      position: relative;
    }

    .user-button {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      color: var(--context-primary, #00d4aa);
      padding: 0.6rem 1rem;
      border-radius: 10px;
      font-size: 0.85em;
      font-weight: 500;
      cursor: pointer;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .user-button:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
    }

    .user-dropdown {
      position: absolute;
      top: calc(100% + 0.5rem);
      right: 0;
      background: linear-gradient(135deg, rgba(26, 26, 26, 0.98) 0%, rgba(15, 15, 15, 0.95) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      border-radius: 12px;
      padding: 1rem;
      min-width: 250px;
      box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
      z-index: 1000;
      animation: dropdownSlide 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }

    @keyframes dropdownSlide {
      from {
        opacity: 0;
        transform: translateY(-10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .user-info {
      padding-bottom: 0.8rem;
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      margin-bottom: 0.8rem;
    }

    .user-name {
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
      font-size: 1em;
      margin-bottom: 0.3rem;
    }

    .user-role {
      color: #888;
      font-size: 0.75em;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .user-session {
      color: #666;
      font-size: 0.7em;
      margin-top: 0.3rem;
    }

    .logout-button {
      width: 100%;
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.15) 0%, rgba(239, 68, 68, 0.1) 100%);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
      padding: 0.6rem 1rem;
      border-radius: 8px;
      font-size: 0.85em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
    }

    .logout-button:hover {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.25) 0%, rgba(239, 68, 68, 0.2) 100%);
      border-color: rgba(255, 107, 107, 0.5);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(255, 107, 107, 0.3);
    }

    .main-content {
      padding: 2.5rem;
      max-width: 1600px;
      margin: 0 auto;
    }

    .widgets-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
      gap: 1.5rem;
      margin-bottom: 2rem;
    }

    .widget-container {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-radius: 16px;
      padding: 1.8rem;
      backdrop-filter: blur(15px);
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
      position: relative;
      overflow: hidden;
    }

    .widget-container::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 2px;
      background: linear-gradient(90deg, transparent, var(--context-primary, #00d4aa), transparent);
      opacity: 0;
      transition: opacity 0.4s ease;
    }

    .widget-container:hover {
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      transform: translateY(-4px) scale(1.01);
      box-shadow: 0 16px 48px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
    }

    .widget-container:hover::before {
      opacity: 1;
    }

    .error-message {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.15) 0%, rgba(255, 107, 107, 0.05) 100%);
      border: 1px solid rgba(255, 107, 107, 0.4);
      border-radius: 12px;
      padding: 1.2rem;
      margin: 1rem 0;
      color: #ff6b6b;
      font-weight: 500;
      box-shadow: 0 4px 16px rgba(255, 107, 107, 0.1);
    }

    /* Tabs mobile */
    .tabs-container {
      display: none;
    }

    .tabs {
      display: flex;
      gap: 0.5rem;
      border-bottom: 2px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
    }

    @media (max-width: 768px) {
      .tabs {
        position: fixed;
        bottom: 0;
        left: 0;
        right: 0;
        margin-bottom: 0;
        background: linear-gradient(to top, #0f0f0f 0%, rgba(15, 15, 15, 0.98) 80%, rgba(15, 15, 15, 0.95) 100%);
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
        z-index: 90;
        padding: 0.5rem 1rem;
        box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.5);
      }

      .tabs-container {
        padding-bottom: 70px; /* Espace pour les tabs fixes */
      }
    }

    .tab {
      padding: 0.75rem 1.25rem;
      background: transparent;
      border: none;
      color: #888;
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s ease;
      border-bottom: 2px solid transparent;
      white-space: nowrap;
      position: relative;
      bottom: -2px;
    }

    .tab:hover {
      color: var(--context-primary, #00d4aa);
    }

    .tab.active {
      color: var(--context-primary, #00d4aa);
      border-bottom-color: var(--context-primary, #00d4aa);
    }

    .tab-content {
      display: none;
    }

    .tab-content.active {
      display: grid;
      grid-template-columns: 1fr;
      gap: 1.2rem;
    }

    @media (max-width: 768px) {
      .header {
        padding: 0.8rem 0.8rem;
        gap: 0.5rem;
      }
      .header h1 {
        font-size: 1.2em;
        margin-bottom: 0.3rem;
      }
      .status-bar {
        flex-wrap: nowrap;
        gap: 0.3rem;
        margin-top: 0.3rem;
      }
      .status-indicator {
        padding: 0.2rem 0.4rem;
        font-size: 0.65em;
        white-space: nowrap;
        gap: 0.3rem;
      }
      .status-dot {
        width: 6px;
        height: 6px;
      }
      /* Masquer uptime sur mobile */
      .uptime-indicator {
        display: none;
      }
      .system-clock {
        padding: 0.3rem 0.6rem;
        font-size: 0.7em;
        border-radius: 6px;
        gap: 0.25rem;
      }
      .system-clock .icon {
        font-size: 0.9em;
      }
      .user-button {
        padding: 0.3rem 0.6rem;
        font-size: 0.7em;
      }
      .main-content {
        padding: 1.2rem;
      }
      .widgets-grid {
        display: none; /* Cacher grille sur mobile */
      }
      .tabs-container {
        display: block; /* Afficher tabs sur mobile */
      }
      .widget-container {
        padding: 1.4rem;
      }
    }

    @media (min-width: 769px) {
      .widgets-grid {
        grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
      }
    }
  `
  
  static properties = {
    connected: { type: Boolean },
    mqttStatus: { type: String },
    apiStatus: { type: String },
    systemHealth: { type: Object },
    plugins: { type: Array },
    error: { type: String },
    showUserMenu: { type: Boolean },
    showSettingsPage: { type: Boolean },
    currentUser: { type: Object },
    activeTab: { type: String },
    currentTime: { type: String }
  }
  
  constructor() {
    super()
    this.connected = false
    this.mqttStatus = 'connecting'
    this.apiStatus = 'loading'
    this.systemHealth = null
    this.plugins = []
    this.error = null
    this.showUserMenu = false
    this.showSettingsPage = false
    this.currentUser = authService.getCurrentUser()
    // Restaurer le dernier onglet actif depuis sessionStorage (persiste aux reloads, reset à la fermeture du navigateur)
    this.activeTab = sessionStorage.getItem('dashboardTab') || 'controle'
    this.currentTime = this.formatTime(new Date())

    this.apiService = null
    this.mqttService = null
    this.agentsService = null
    this.timeInterval = null
  }

  formatTime(date) {
    // Détecter mobile pour afficher HH:MM ou HH:MM:SS
    const isMobile = window.innerWidth <= 768
    return date.toLocaleTimeString('fr-FR', {
      hour: '2-digit',
      minute: '2-digit',
      second: isMobile ? undefined : '2-digit',
      hour12: false
    })
  }

  updateTime() {
    this.currentTime = this.formatTime(new Date())
  }
  
  async connectedCallback() {
    super.connectedCallback()

    // Démarrer l'horloge
    this.timeInterval = setInterval(() => this.updateTime(), 1000)

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

  disconnectedCallback() {
    super.disconnectedCallback()
    // Nettoyer l'intervalle d'horloge
    if (this.timeInterval) {
      clearInterval(this.timeInterval)
      this.timeInterval = null
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

    // Service Agents
    this.agentsService = document.createElement('agents-service')

    // Service Context
    this.contextService = document.createElement('context-service')

    // Initialiser CSRF service avec authService
    csrfService.setAuthService(authService)
    console.log('🔐 CSRF service initialized with authService')

    document.body.appendChild(this.apiService)
    document.body.appendChild(this.mqttService)
    document.body.appendChild(this.agentsService)
    document.body.appendChild(this.contextService)
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

    // Fonction de mise à jour
    const updateData = async () => {
      if (this.apiStatus === 'online') {
        try {
          const health = await this.apiService.getSystemHealth()
          this.systemHealth = health

          // Mettre à jour le status MQTT du header
          if (health && health.mqtt_status) {
            this.mqttStatus = health.mqtt_status
          }

          const plugins = await this.apiService.getPlugins()
          this.plugins = plugins
        } catch (error) {
          console.warn('⚠️ Periodic update failed:', error)
        }
      }
    }

    // Première mise à jour immédiate
    updateData()

    // Puis mise à jour périodique
    setInterval(updateData, 10000) // 10 secondes
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
        <div class="header-left">
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
              <div class="status-indicator uptime-indicator">
                <span>Uptime: ${this.formatUptime(this.systemHealth.uptime_seconds)}</span>
              </div>
            ` : ''}
          </div>
        </div>

        <div class="system-clock">
          <span class="icon">🕐</span>
          <span>${this.currentTime}</span>
        </div>

        ${this.currentUser ? html`
          <div class="user-menu">
            <button class="user-button" @click="${this.toggleUserMenu}">
              <span>👤</span>
              <span>${this.currentUser.username}</span>
            </button>

            ${this.showUserMenu ? html`
              <div class="user-dropdown">
                <div class="user-info">
                  <div class="user-name">${this.currentUser.username}</div>
                  <div class="user-role">${this.currentUser.role}</div>
                  <div class="user-session">${this.getSessionDuration()}</div>
                </div>
                <button class="logout-button" @click="${this.handleOpenSettings}" style="margin-bottom: 0.5rem; background: linear-gradient(135deg, color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%); border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent); color: var(--context-primary, #00d4aa);">
                  <span>⚙️</span>
                  <span>Paramètres</span>
                </button>
                <button class="logout-button" @click="${this.handleLogout}">
                  <span>🚪</span>
                  <span>Déconnexion</span>
                </button>
              </div>
            ` : ''}
          </div>
        ` : ''}
      </div>
      
      <div class="main-content">
        ${this.error ? html`
          <div class="error-message">
            ❌ ${this.error}
          </div>
        ` : ''}

        <!-- Tabs mobile uniquement -->
        <div class="tabs-container">
          <div class="tabs">
            <button class="tab ${this.activeTab === 'controle' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('controle')}">
              🎛️ Contrôle
            </button>
            <button class="tab ${this.activeTab === 'systeme' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('systeme')}">
              ⚙️ Système
            </button>
            <button class="tab ${this.activeTab === 'donnees' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('donnees')}">
              📝 Données
            </button>
          </div>

          <!-- Contenu tab Contrôle -->
          <div class="tab-content ${this.activeTab === 'controle' ? 'active' : ''}">
            <div class="widget-container">
              <context-widget></context-widget>
            </div>
            <div class="widget-container">
              <agent-control-widget></agent-control-widget>
            </div>
          </div>

          <!-- Contenu tab Système -->
          <div class="tab-content ${this.activeTab === 'systeme' ? 'active' : ''}">
            <div class="widget-container">
              <system-health-widget
                .health="${this.systemHealth}"
                .connected="${this.connected}">
              </system-health-widget>
            </div>
            <div class="widget-container">
              <context-settings-widget></context-settings-widget>
            </div>
            <div class="widget-container">
              <plugins-widget
                .plugins="${this.plugins}"
                .apiService="${this.apiService}">
              </plugins-widget>
            </div>
          </div>

          <!-- Contenu tab Données -->
          <div class="tab-content ${this.activeTab === 'donnees' ? 'active' : ''}">
            <div class="widget-container">
              <notes-widget
                .apiService="${this.apiService}"
                .connected="${this.connected}">
              </notes-widget>
            </div>
            <div class="widget-container">
              <context-stats-widget></context-stats-widget>
            </div>
            <div class="widget-container">
              <agents-network-widget
                .connected="${this.connected}">
              </agents-network-widget>
            </div>
          </div>
        </div>

        <!-- Grille desktop complète -->
        <div class="widgets-grid">
          <!-- Widget contexte -->
          <div class="widget-container">
            <context-widget></context-widget>
          </div>

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
          
          <!-- Widget hosts DEPRECATED: remplacé par agents-network-widget -->
          <!-- <div class="widget-container">
            <hosts-widget 
              .connected="${this.connected}"
              .apiService="${this.apiService}">
            </hosts-widget>
          </div> -->
          
          <!-- Widget notes -->
          <div class="widget-container">
            <notes-widget 
              .apiService="${this.apiService}"
              .connected="${this.connected}">
            </notes-widget>
          </div>
          
          <!-- Widget agents network -->
          <div class="widget-container">
            <agents-network-widget
              .connected="${this.connected}">
            </agents-network-widget>
          </div>

          <!-- Widget statistiques contextuelles -->
          <div class="widget-container">
            <context-stats-widget></context-stats-widget>
          </div>

          <!-- Widget paramètres contexte -->
          <div class="widget-container">
            <context-settings-widget></context-settings-widget>
          </div>
        </div>
        
        <!-- Modal de contrôle agent détaillé -->
        <agent-control-widget></agent-control-widget>

        <!-- Page Paramètres Utilisateur -->
        ${this.showSettingsPage ? html`
          <user-settings-page @close="${this.handleCloseSettings}"></user-settings-page>
        ` : ''}
      </div>
    `
  }
  
  setActiveTab(tab) {
    this.activeTab = tab
    sessionStorage.setItem('dashboardTab', tab)
  }

  toggleUserMenu() {
    this.showUserMenu = !this.showUserMenu
  }

  handleOpenSettings() {
    this.showSettingsPage = true
    this.showUserMenu = false // Fermer le menu dropdown
  }

  handleCloseSettings() {
    this.showSettingsPage = false
  }

  async handleLogout() {
    const confirmed = confirm('Êtes-vous sûr de vouloir vous déconnecter ?')

    if (confirmed) {
      console.log('[dashboard] Logging out user')
      await authService.logout()

      // Rediriger vers boot terminal
      window.location.reload()
    }
  }

  getSessionDuration() {
    if (!this.currentUser || !this.currentUser.expires_at) {
      return 'N/A'
    }

    const now = Math.floor(Date.now() / 1000)
    const remaining = this.currentUser.expires_at - now

    if (remaining <= 0) {
      return 'Expirée'
    }

    const hours = Math.floor(remaining / 3600)
    const minutes = Math.floor((remaining % 3600) / 60)

    if (hours > 0) {
      return `${hours}h ${minutes}m restantes`
    }
    return `${minutes}m restantes`
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