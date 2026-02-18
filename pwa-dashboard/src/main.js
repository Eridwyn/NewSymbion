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

// ============================================================================
// Console Log Interception — BroadcastChannel pour Log Viewer
// ============================================================================
;(() => {
  const channel = new BroadcastChannel('symbion-logs')
  const originalLog = console.log.bind(console)
  const originalWarn = console.warn.bind(console)
  const originalError = console.error.bind(console)

  function parseComponent(msg) {
    if (typeof msg !== 'string') return 'pwa'
    const m = msg.match(/^\[([^\]]+)\]/)
    return m ? m[1] : 'pwa'
  }

  function intercept(level, originalFn, args) {
    originalFn(...args)
    try {
      const msg = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ')
      channel.postMessage({
        timestamp: new Date().toISOString(),
        level,
        component: parseComponent(args[0]),
        message: msg,
        source: 'pwa'
      })
    } catch (_) { /* ignore serialization errors */ }
  }

  console.log = (...args) => intercept('info', originalLog, args)
  console.warn = (...args) => intercept('warning', originalWarn, args)
  console.error = (...args) => intercept('error', originalError, args)
})()

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
    // [Audit] Store handler reference for cleanup
    this._bootCompleteHandler = this.handleBootComplete.bind(this)
  }

  connectedCallback() {
    super.connectedCallback()

    // Écouter les événements de boot
    this.addEventListener('boot-complete', this._bootCompleteHandler)
    console.log('[app] SymbionApp connected, listening for boot-complete events')
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    // [Audit] Cleanup event listener
    this.removeEventListener('boot-complete', this._bootCompleteHandler)
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

    // Cacher le loader de page après un court délai pour l'animation
    setTimeout(() => {
      const pageLoader = document.getElementById('page-loader')
      if (pageLoader) {
        pageLoader.classList.add('hidden')
        console.log('[app] Page loader hidden')
        // Retirer complètement du DOM après la transition
        setTimeout(() => {
          pageLoader.remove()
        }, 600) // Correspond à la durée de transition CSS (0.6s)
      }
    }, 500) // Petit délai pour voir le loader
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

// ============================================================================
// Page Lifecycle Management - Empêcher le navigateur de décharger l'onglet
// ============================================================================

// Wake Lock API - garder l'onglet actif (déclaré en premier pour hoisting)
let wakeLock = null

async function requestWakeLock() {
  if ('wakeLock' in navigator) {
    try {
      wakeLock = await navigator.wakeLock.request('screen')
      console.log('[lifecycle] 🔒 Wake Lock acquired - tab will stay active')

      wakeLock.addEventListener('release', () => {
        console.log('[lifecycle] 🔓 Wake Lock released')
      })
    } catch (err) {
      console.log('[lifecycle] ⚠️ Wake Lock not available:', err.message)
    }
  }
}

// Détection visibilité de la page - [Audit] Unified handler with wake lock management
document.addEventListener('visibilitychange', async () => {
  if (document.hidden) {
    console.log('[lifecycle] 🌙 Page hidden - maintaining background connections')
    // Page cachée mais on garde les connexions MQTT/WebSocket actives

    // [Audit] Release wake lock when page hidden to save battery
    if (wakeLock !== null) {
      try {
        await wakeLock.release()
        wakeLock = null
        console.log('[lifecycle] 🔓 Wake Lock released (page hidden)')
      } catch (e) {
        console.warn('[lifecycle] Wake Lock release failed:', e)
      }
    }
  } else {
    console.log('[lifecycle] ☀️ Page visible - resuming activity')
    // Page redevenue visible, on peut rafraîchir si besoin
    // Mais on ne recharge PAS la page

    // [Audit] Re-acquire wake lock when page becomes visible
    if (wakeLock === null) {
      await requestWakeLock()
    }
  }
})

// Page Lifecycle API - Empêcher freeze/discard
document.addEventListener('freeze', (event) => {
  console.log('[lifecycle] ❄️ Page about to freeze - preventing...')
  // Le navigateur essaie de geler la page pour économiser RAM
  // On ne peut pas vraiment empêcher ça mais on peut logger
})

document.addEventListener('resume', (event) => {
  console.log('[lifecycle] ♻️ Page resumed from freeze')
  // Page réactivée après freeze
})

// Demander le Wake Lock dès le chargement si page visible
if (document.visibilityState === 'visible') {
  requestWakeLock()
}

// ============================================================================
// Authentication Expiration Handler - Force reload to login
// ============================================================================

import authService from './services/auth-service.js'

// Écouter les événements d'expiration de session
authService.addEventListener('auth:expired', () => {
  console.log('[lifecycle] 🔐 Authentication expired - reloading to login')
  // Clear sessionStorage pour forcer retour au boot terminal
  sessionStorage.clear()
  // Reload page pour retourner au login
  window.location.reload()
})