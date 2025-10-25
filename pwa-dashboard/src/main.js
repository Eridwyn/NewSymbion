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

// Configuration globale
window.SYMBION_CONFIG = {
  API_BASE: 'http://192.168.1.14:8080',  // IP locale du PC (accessible depuis téléphone)
  MQTT_BROKER: '192.168.1.14',
  MQTT_PORT: 9001, // WebSocket port
  VERSION: '0.1.0',
  HOST_AGENT_IP: '192.168.1.14' // IP de l'agent qui héberge le kernel/dashboard (ne pas shutdown!)
}

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
    container.appendChild(app)
    console.log('[app] Boot terminal mounted')
  }
}

// Démarrer dès que le DOM est prêt
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', startApp)
} else {
  startApp()
}

// Service Worker pour PWA
if ('serviceWorker' in navigator) {
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