/**
 * Widget Santé Système
 * 
 * Affiche les métriques de santé du kernel Symbion
 * Mise à jour temps réel via MQTT et API
 */

import { LitElement, html, css } from 'lit'

class SystemHealthWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }
    
    .widget-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }
    
    .widget-title {
      font-size: 1.2em;
      font-weight: 600;
      color: #e0e0e0;
    }
    
    .status-badge {
      padding: 0.3rem 0.8rem;
      border-radius: 20px;
      font-size: 0.8em;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    
    .status-healthy {
      background: rgba(0, 212, 170, 0.2);
      color: #00d4aa;
      border: 1px solid rgba(0, 212, 170, 0.3);
    }
    
    .status-warning {
      background: rgba(255, 217, 61, 0.2);
      color: #ffd93d;
      border: 1px solid rgba(255, 217, 61, 0.3);
    }
    
    .status-error {
      background: rgba(255, 107, 107, 0.2);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.3);
    }
    
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
      gap: 1rem;
    }
    
    .metric-card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      padding: 1rem;
      text-align: center;
    }
    
    .metric-value {
      font-size: 1.8em;
      font-weight: 700;
      margin-bottom: 0.3rem;
      background: linear-gradient(90deg, #007acc, #00d4aa);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    
    .metric-label {
      font-size: 0.85em;
      opacity: 0.7;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    
    .metric-unit {
      font-size: 0.7em;
      opacity: 0.6;
      margin-left: 0.2rem;
    }
    
    .loading {
      text-align: center;
      padding: 2rem;
      opacity: 0.6;
    }
    
    .error {
      text-align: center;
      padding: 1rem;
      color: #ff6b6b;
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: 6px;
    }
    
    .last-updated {
      margin-top: 1rem;
      text-align: center;
      font-size: 0.8em;
      opacity: 0.5;
    }
  `
  
  static properties = {
    health: { type: Object },
    connected: { type: Boolean },
    lastUpdate: { type: String }
  }
  
  constructor() {
    super()
    this.health = null
    this.connected = false
    this.lastUpdate = null
  }
  
  connectedCallback() {
    super.connectedCallback()
    
    // Écouter les mises à jour MQTT
    this.addEventListener('system-health', this.handleHealthUpdate.bind(this))
  }
  
  handleHealthUpdate(event) {
    this.health = event.detail.health
    this.lastUpdate = new Date().toLocaleTimeString()
    this.requestUpdate()
  }
  
  getHealthStatus() {
    if (!this.connected) return 'error'
    if (!this.health) return 'warning'
    
    const memoryUsage = this.health.memory_usage_mb
    const mqttStatus = this.health.mqtt_status
    
    // MQTT connecting is normal, only error if failed
    if (mqttStatus === 'failed') return 'error'
    if (memoryUsage > 500) return 'warning'
    
    return 'healthy'
  }
  
  formatUptime(seconds) {
    if (!seconds) return 'N/A'
    
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    
    if (days > 0) return `${days}j`
    if (hours > 0) return `${hours}h`
    return `${minutes}m`
  }
  
  formatMemory(mb) {
    if (!mb) return 'N/A'
    
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(2)} GB`
    }
    
    return `${mb.toFixed(2)} MB`
  }
  
  render() {
    if (!this.connected) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">🏥 Santé Système</h3>
          <span class="status-badge status-error">Déconnecté</span>
        </div>
        <div class="error">
          ❌ Impossible de se connecter au kernel
        </div>
      `
    }
    
    if (!this.health) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">🏥 Santé Système</h3>
          <span class="status-badge status-warning">Chargement</span>
        </div>
        <div class="loading">
          ⏳ Chargement des métriques...
        </div>
      `
    }
    
    const status = this.getHealthStatus()
    const statusLabels = {
      healthy: 'Sain',
      warning: 'Attention',
      error: 'Erreur'
    }
    
    return html`
      <div class="widget-header">
        <h3 class="widget-title">🏥 Santé Système</h3>
        <span class="status-badge status-${status}">${statusLabels[status]}</span>
      </div>
      
      <div class="metrics-grid">
        <div class="metric-card">
          <div class="metric-value">
            ${this.formatUptime(this.health.uptime_seconds)}
          </div>
          <div class="metric-label">Uptime</div>
        </div>
        
        <div class="metric-card">
          <div class="metric-value">
            ${this.formatMemory(this.health.memory_usage_mb)}
          </div>
          <div class="metric-label">Mémoire</div>
        </div>
        
        <div class="metric-card">
          <div class="metric-value">
            ${this.health.mqtt_status === 'connected' ? '✅' : (this.health.mqtt_status === 'connecting' ? '🔄' : '❌')}
          </div>
          <div class="metric-label">MQTT</div>
        </div>
        
        <div class="metric-card">
          <div class="metric-value">
            ${this.health.mqtt_messages_per_minute ? this.health.mqtt_messages_per_minute.toFixed(0) : '0'}
          </div>
          <div class="metric-label">Msg/min</div>
        </div>
        
        <div class="metric-card">
          <div class="metric-value">
            ${this.health.agents_count || this.health.hosts_tracked || 0}
          </div>
          <div class="metric-label">Agents</div>
        </div>
        
        <div class="metric-card">
          <div class="metric-value">
            ${this.health.plugins_active || 0}
          </div>
          <div class="metric-label">Plugins</div>
        </div>
      </div>
      
      ${this.lastUpdate ? html`
        <div class="last-updated">
          Dernière MAJ: ${this.lastUpdate}
        </div>
      ` : ''}
    `
  }
}

customElements.define('system-health-widget', SystemHealthWidget)