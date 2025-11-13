/**
 * Symbion Dashboard - Point d'entrée principal
 *
 * PWA moderne utilisant Lit pour les composants et MQTT.js pour temps réel
 * Architecture modulaire avec widgets dynamiques basés sur manifestes plugins
 */

import { LitElement, html, css } from 'lit'
import './components/boot-terminal.js'
import './components/dashboard-app.js'
import './services/auth-service.js'
import './services/api-service.js'
import './services/mqtt-service.js'
// Widget registry temporarily disabled due to initialization issues

console.log('🚀 Starting Symbion Dashboard v0.1.0')

// Configuration chargée depuis /public/config.js (ne pas écraser ici)
// window.SYMBION_CONFIG est défini par /config.js avant ce fichier

// App Router Component
class SymbionApp extends LitElement {
  static styles = css`
    :host {
      display: block;
      min-height: 100vh;
    }
  `

  static properties = {
    currentView: { type: String }
  }

  constructor() {
    super()
    this.currentView = 'boot' // boot, dashboard
  }

  connectedCallback() {
    super.connectedCallback()

    // Écouter les événements de boot
    this.addEventListener('boot-complete', this.handleBootComplete.bind(this))
    console.log('[app] SymbionApp connected, listening for boot-complete events')
  }

  handleBootComplete(event) {
    console.log('[app] Boot complete:', event.detail)

    if (event.detail.authenticated) {
      console.log('[app] User authenticated, loading dashboard')
      this.currentView = 'dashboard'
      this.requestUpdate()
    } else {
      console.log('[app] Not authenticated, staying on boot screen')
      // Reste sur le boot terminal pour login
    }
  }

  render() {
    if (this.currentView === 'boot') {
      return html`<boot-terminal></boot-terminal>`
    }

    return html`<dashboard-app></dashboard-app>`
  }
}

customElements.define('symbion-app', SymbionApp)

// Démarrage de l'application immédiat
const startApp = () => {
  console.log('[app] Starting Symbion app...')
  const app = document.createElement('symbion-app')
  const container = document.getElementById('app')

  if (container) {
    // Create and append services to the main document (not shadow DOM)
    // These are hidden and used by widgets via document.querySelector()
    const apiService = document.createElement('api-service')
    apiService.style.display = 'none'
    const mqttService = document.createElement('mqtt-service')
    mqttService.style.display = 'none'
    const contextService = document.createElement('context-service')
    contextService.style.display = 'none'
    const agentsService = document.createElement('agents-service')
    agentsService.style.display = 'none'

    container.appendChild(apiService)
    container.appendChild(mqttService)
    container.appendChild(contextService)
    container.appendChild(agentsService)
    container.appendChild(app)
    console.log('[app] Boot terminal mounted with services')
  }
}

// Démarrer dès que le DOM est prêt
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', startApp)
} else {
  startApp()
}

// Service Worker pour PWA (uniquement en production)
if ('serviceWorker' in navigator && import.meta.env.PROD) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js')
      .then(registration => {
        console.log('✅ SW registered:', registration)
      })
      .catch(error => {
        console.log('❌ SW registration failed:', error)
      })
  })
}