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
      padding: 0.5rem 1rem;
      border-radius: 20px;
      font-size: 0.75em;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.8px;
      transition: all 0.3s ease;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    .status-healthy {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.25) 0%, rgba(34, 197, 94, 0.2) 100%);
      color: #00d4aa;
      border: 1px solid rgba(0, 212, 170, 0.4);
      box-shadow: 0 2px 12px rgba(0, 212, 170, 0.3);
      animation: pulse-healthy 3s ease-in-out infinite;
    }

    .status-warning {
      background: linear-gradient(135deg, rgba(255, 217, 61, 0.25) 0%, rgba(251, 191, 36, 0.2) 100%);
      color: #ffd93d;
      border: 1px solid rgba(255, 217, 61, 0.4);
      box-shadow: 0 2px 12px rgba(255, 217, 61, 0.3);
      animation: pulse-warning 2s ease-in-out infinite;
    }

    .status-error {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.25) 0%, rgba(239, 68, 68, 0.2) 100%);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.4);
      box-shadow: 0 2px 12px rgba(255, 107, 107, 0.3);
      animation: pulse-error 1.5s ease-in-out infinite;
    }

    @keyframes pulse-healthy {
      0%, 100% {
        box-shadow: 0 2px 12px rgba(0, 212, 170, 0.3);
      }
      50% {
        box-shadow: 0 2px 16px rgba(0, 212, 170, 0.5);
      }
    }

    @keyframes pulse-warning {
      0%, 100% {
        box-shadow: 0 2px 12px rgba(255, 217, 61, 0.3);
      }
      50% {
        box-shadow: 0 2px 16px rgba(255, 217, 61, 0.5);
      }
    }

    @keyframes pulse-error {
      0%, 100% {
        box-shadow: 0 2px 12px rgba(255, 107, 107, 0.3);
      }
      50% {
        box-shadow: 0 2px 16px rgba(255, 107, 107, 0.6);
      }
    }
    
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
      gap: 1rem;
    }
    
    .metric-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 12px;
      padding: 1.2rem;
      text-align: center;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      position: relative;
      overflow: hidden;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
    }

    .metric-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 3px;
      background: linear-gradient(90deg, #007acc, #00d4aa);
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .metric-card:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      border-color: rgba(0, 212, 170, 0.3);
      transform: translateY(-4px);
      box-shadow: 0 8px 24px rgba(0, 212, 170, 0.15);
    }

    .metric-card:hover::before {
      opacity: 1;
    }
    
    .metric-value {
      font-size: 2em;
      font-weight: 700;
      margin-bottom: 0.5rem;
      background: linear-gradient(135deg, #007acc 0%, #00d4aa 50%, #22c55e 100%);
      background-size: 200% 200%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      animation: gradient-shift 3s ease infinite;
      filter: drop-shadow(0 2px 4px rgba(0, 212, 170, 0.3));
    }

    @keyframes gradient-shift {
      0%, 100% {
        background-position: 0% 50%;
      }
      50% {
        background-position: 100% 50%;
      }
    }

    .metric-label {
      font-size: 0.8em;
      color: #888;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 1px;
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

    /* Responsive */
    @media (max-width: 768px) {
      .metrics-grid {
        grid-template-columns: repeat(2, 1fr);
        gap: 0.8rem;
      }

      .metric-card {
        padding: 1rem;
      }

      .metric-value {
        font-size: 1.6em;
      }

      .metric-label {
        font-size: 0.75em;
      }

      .widget-header {
        flex-direction: column;
        gap: 0.8rem;
        align-items: flex-start;
      }

      .status-badge {
        align-self: flex-start;
      }
    }

    @media (max-width: 480px) {
      .metrics-grid {
        grid-template-columns: 1fr;
      }
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