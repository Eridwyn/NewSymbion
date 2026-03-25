/**
 * Widget Santé Système
 * 
 * Affiche les métriques de santé du kernel Symbion
 * Mise à jour temps réel via MQTT et API
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles } from '../styles/shared-patterns.js'

class SystemHealthWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, statusBadgeStyles, css`
    :host {
      display: block;
    }

    /* status-badge variants (.healthy, .warning, .error) from shared statusBadgeStyles */
    /* Local: only animations (unique pulse per state) */
    .status-badge.healthy { animation: pulse-healthy 3s ease-in-out infinite; }
    .status-badge.warning { animation: pulse-warning 2s ease-in-out infinite; }
    .status-badge.error { animation: pulse-error 1.5s ease-in-out infinite; }

    @keyframes pulse-healthy {
      0%, 100% { box-shadow: 0 2px 8px var(--ctx-bg-emphasis); }
      50% { box-shadow: 0 2px 16px var(--ctx-border-intense); }
    }

    @keyframes pulse-warning {
      0%, 100% { box-shadow: 0 2px 8px rgba(251, 191, 36, 0.25); }
      50% { box-shadow: 0 2px 16px rgba(251, 191, 36, 0.5); }
    }

    @keyframes pulse-error {
      0%, 100% { box-shadow: 0 2px 8px rgba(255, 107, 107, 0.25); }
      50% { box-shadow: 0 2px 16px rgba(255, 107, 107, 0.6); }
    }
    
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
      gap: 1rem;
    }
    
    .metric-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
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
      background: linear-gradient(90deg, #007acc, var(--context-primary, #00d4aa));
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .metric-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-4px);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
    }

    .metric-card:hover::before {
      opacity: 1;
    }
    
    .metric-value {
      font-size: 2em;
      font-weight: 700;
      margin-bottom: 0.5rem;
      background: linear-gradient(135deg, #007acc 0%, var(--context-primary, #00d4aa) 50%, #22c55e 100%);
      background-size: 200% 200%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      animation: gradient-shift 3s ease infinite, metricPulse 3s ease-in-out infinite;
      filter: drop-shadow(0 2px 4px var(--ctx-border-strong));
    }

    /* Status Indicator Styles — compound selector beats .metric-value */
    .metric-value.status-indicator {
      font-size: 2.5em;
      line-height: 1;
      background: none;
      -webkit-background-clip: unset;
      -webkit-text-fill-color: unset;
      animation: none;
      filter: none;
    }

    .metric-value.status-indicator.connected {
      color: var(--context-primary, #00d4aa);
      text-shadow: 0 0 20px color-mix(in srgb, var(--context-primary) 80%, transparent),
                   0 0 40px var(--ctx-border-strong);
      animation: statusPulse 2s ease-in-out infinite;
    }

    .metric-value.status-indicator.connecting {
      color: var(--color-warning-text-muted, #fbbf24);
      text-shadow: 0 0 20px rgba(251, 191, 36, 0.6),
                   0 0 40px rgba(251, 191, 36, 0.3);
      animation: statusSpin 2s linear infinite;
    }

    .metric-value.status-indicator.disconnected {
      color: var(--color-danger-text-muted, #ff6b6b);
      text-shadow: 0 0 20px rgba(255, 107, 107, 0.6),
                   0 0 40px rgba(255, 107, 107, 0.3);
      animation: statusBlink 1.5s ease-in-out infinite;
    }

    @keyframes statusPulse {
      0%, 100% {
        opacity: 1;
        transform: scale(1);
      }
      50% {
        opacity: 0.7;
        transform: scale(1.1);
      }
    }

    @keyframes statusSpin {
      from {
        transform: rotate(0deg);
      }
      to {
        transform: rotate(360deg);
      }
    }

    @keyframes statusBlink {
      0%, 100% {
        opacity: 1;
      }
      50% {
        opacity: 0.3;
      }
    }

    /* gradient-shift — see shared-animations.js */

    .metric-label {
      font-size: 0.8em;
      color: var(--color-dark-text-tertiary, #94a3b8);
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
      color: var(--color-danger-text-muted, #ff6b6b);
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: var(--radius-sm);
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

      .metric-value.status-indicator {
        font-size: 2em;
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

    @media (max-width: 375px) {
      .metrics-grid {
        grid-template-columns: repeat(2, 1fr);
      }

      .metric-value {
        font-size: 1.4em;
      }
    }

    /* Container Queries — adapt to widget's own width */
    @container widget (max-width: 400px) {
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
    }

    @container widget (max-width: 280px) {
      .metrics-grid {
        grid-template-columns: 1fr;
      }

      .metric-value {
        font-size: 1.4em;
      }
    }
  `]

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
    this._boundHealthUpdate = this.handleHealthUpdate.bind(this)
    this.addEventListener('system-health', this._boundHealthUpdate)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._boundHealthUpdate) {
      this.removeEventListener('system-health', this._boundHealthUpdate)
      this._boundHealthUpdate = null
    }
  }
  
  handleHealthUpdate(event) {
    this.health = { ...event.detail.health }
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
          <span class="status-badge error">Déconnecté</span>
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
          <span class="status-badge warning">Chargement</span>
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
        <span class="status-badge ${status}">${statusLabels[status]}</span>
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
          <div class="metric-value status-indicator ${this.health.mqtt_status}">
            ${this.health.mqtt_status === 'connected' ? '●' : (this.health.mqtt_status === 'connecting' ? '◐' : '○')}
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