/**
 * Symbion Dashboard - Point d'entrée principal
 * 
 * PWA moderne utilisant Lit pour les composants et MQTT.js pour temps réel
 * Architecture modulaire avec widgets dynamiques basés sur manifestes plugins
 */

import { LitElement, html, css } from 'lit'
import './components/dashboard-app.js'
import './services/api-service.js'
import './services/mqtt-service.js'
// Widget registry temporarily disabled due to initialization issues

console.log('🚀 Starting Symbion Dashboard v0.1.0')

// Configuration globale
window.SYMBION_CONFIG = {
  API_BASE: 'http://192.168.1.14:8080',
  MQTT_BROKER: '192.168.1.14',
  MQTT_PORT: 9001, // WebSocket port
  VERSION: '0.1.0'
}

// Démarrage de l'application
document.addEventListener('DOMContentLoaded', async () => {
  const app = document.createElement('dashboard-app')
  const container = document.getElementById('app')
  
  if (container) {
    container.innerHTML = ''
    container.appendChild(app)
  }
})

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