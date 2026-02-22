/**
 * Widget Monitoring des Hosts
 * 
 * Affiche l'état des hosts surveillés:
 * - Heartbeats en temps réel
 * - Métriques CPU/RAM/IP
 * - Actions Wake-on-LAN
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles } from '../styles/shared-patterns.js'
import pollingScheduler from '../services/polling-scheduler.js'

class HostsWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, statusBadgeStyles, css`
    :host {
      display: block;
    }

    /* widget-count provided by widgetHeaderStyles */
    
    .hosts-list {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }
    
    .host-card {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md, 8px);
      padding: 1rem;
      transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
    }
    
    .host-card:hover {
      border-color: rgba(0, 122, 204, 0.3);
      background: var(--surface-glass);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
    }
    
    .host-card.online {
      border-color: rgba(0, 212, 170, 0.3);
    }
    
    .host-card.offline {
      border-color: rgba(255, 107, 107, 0.3);
      opacity: 0.7;
    }
    
    .host-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.8rem;
    }
    
    .host-name {
      font-weight: 600;
      color: #e0e0e0;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    
    /* host-status uses shared statusBadgeStyles (.status-badge.online/.offline) */
    
    .host-metrics {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 0.8rem;
      margin-bottom: 1rem;
    }
    
    .metric {
      text-align: center;
    }
    
    .metric-value {
      font-size: 1.2em;
      font-weight: 600;
      color: var(--context-primary, #00d4aa);
    }

    .metric-label {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #6c757d);
      text-transform: uppercase;
    }
    
    .host-actions {
      display: flex;
      gap: 0.5rem;
    }
    
    .action-btn {
      background: rgba(0, 212, 170, 0.12);
      border: 1px solid rgba(0, 212, 170, 0.25);
      color: var(--context-primary, #00d4aa);
      padding: 0.4rem 0.8rem;
      border-radius: var(--radius-sm, 4px);
      font-size: var(--text-xs, 0.75rem);
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
    }

    .action-btn:hover {
      background: rgba(0, 212, 170, 0.2);
      border-color: rgba(0, 212, 170, 0.4);
    }

    .action-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
    
    .host-info {
      margin-top: 0.8rem;
      font-size: 0.8em;
      opacity: 0.6;
      display: flex;
      justify-content: space-between;
    }
    
    .last-seen {
      color: #ffd93d;
    }
    
    .placeholder {
      text-align: center;
      padding: 2rem;
      opacity: 0.6;
    }
    
    .placeholder-icon {
      font-size: 3em;
      margin-bottom: 1rem;
    }
  `]

  static properties = {
    hosts: { type: Array },
    connected: { type: Boolean },
    apiService: { type: Object }
  }
  
  constructor() {
    super()
    this.hosts = []
    this.connected = false
    this.apiService = null
  }
  
  connectedCallback() {
    super.connectedCallback()

    // Écouter les heartbeats MQTT
    this.addEventListener('host-heartbeat', this.handleHeartbeat.bind(this))

    // Vérification périodique des hosts offline (5s) via scheduler centralisé
    this._unsubscribeStaleCheck = pollingScheduler.subscribe('5s', () => this.checkOfflineHosts())

    // Rechargement périodique depuis l'API pour synchronisation (30s)
    this._unsubscribeApiSync = pollingScheduler.subscribe('30s', () => {
      if (this.apiService && this.connected) {
        this.syncWithApi()
      }
    })
  }

  disconnectedCallback() {
    super.disconnectedCallback()

    // Cleanup polling scheduler subscriptions
    if (this._unsubscribeStaleCheck) {
      this._unsubscribeStaleCheck()
      this._unsubscribeStaleCheck = null
    }

    if (this._unsubscribeApiSync) {
      this._unsubscribeApiSync()
      this._unsubscribeApiSync = null
    }
  }

  updated(changedProperties) {
    super.updated(changedProperties)
    
    // Charger les hosts quand apiService devient disponible
    if (changedProperties.has('apiService') && this.apiService && this.connected) {
      this.loadHosts()
    }
    
    // Ou quand connected devient true avec apiService déjà présent
    if (changedProperties.has('connected') && this.connected && this.apiService) {
      this.loadHosts()
    }
  }
  
  async loadHosts() {
    if (!this.apiService) return
    
    try {
      const hosts = await this.apiService.getHosts()
      this.hosts = hosts.map(host => ({
        ...host,
        status: host.stale ? 'offline' : 'online',
        lastSeen: new Date(host.last_seen),
        metrics: {
          cpu: host.cpu * 100,  // L'API retourne 0.022 pour 2.2%
          ram: host.ram * 100   // L'API retourne 0.154 pour 15.4%
        },
        net: {
          ip: host.ip
        }
      }))
      this.requestUpdate()
    } catch (error) {
      console.error('❌ Failed to load hosts:', error)
    }
  }

  async syncWithApi() {
    if (!this.apiService) return
    
    try {
      const apiHosts = await this.apiService.getHosts()
      
      // Merge API data with current hosts, keeping MQTT heartbeat data when fresher
      const mergedHosts = this.hosts.map(currentHost => {
        const apiHost = apiHosts.find(h => h.host_id === currentHost.host_id)
        
        if (!apiHost) {
          // Host exists locally but not in API - keep current data
          return currentHost
        }
        
        const apiLastSeen = new Date(apiHost.last_seen)
        const currentLastSeen = currentHost.lastSeen
        
        // Use MQTT data if fresher, otherwise use API data
        if (currentLastSeen && currentLastSeen > apiLastSeen) {
          return currentHost // MQTT data is fresher
        } else {
          return {
            ...currentHost,
            status: apiHost.stale ? 'offline' : 'online',
            lastSeen: apiLastSeen,
            metrics: {
              cpu: apiHost.cpu * 100,
              ram: apiHost.ram * 100
            },
            net: {
              ip: apiHost.ip
            }
          }
        }
      })
      
      // Add any new hosts from API that aren't in our local list
      apiHosts.forEach(apiHost => {
        const exists = this.hosts.some(h => h.host_id === apiHost.host_id)
        if (!exists) {
          mergedHosts.push({
            ...apiHost,
            status: apiHost.stale ? 'offline' : 'online',
            lastSeen: new Date(apiHost.last_seen),
            metrics: {
              cpu: apiHost.cpu * 100,
              ram: apiHost.ram * 100
            },
            net: {
              ip: apiHost.ip
            }
          })
          console.log(`🆕 Host discovered via API sync: ${apiHost.host_id}`)
        }
      })
      
      this.hosts = mergedHosts
      this.requestUpdate()
      
    } catch (error) {
      console.warn('⚠️ API sync failed:', error)
    }
  }
  
  handleHeartbeat(event) {
    const heartbeat = event.detail.heartbeat
    const hostId = heartbeat.host_id
    
    // Mettre à jour ou ajouter le host
    const existingIndex = this.hosts.findIndex(h => h.host_id === hostId)
    const wasOffline = existingIndex >= 0 && this.hosts[existingIndex].status === 'offline'
    
    const hostData = {
      host_id: hostId,
      status: 'online',
      lastSeen: new Date(heartbeat.ts),
      metrics: heartbeat.metrics,
      net: heartbeat.net
    }
    
    if (existingIndex >= 0) {
      this.hosts[existingIndex] = { ...this.hosts[existingIndex], ...hostData }
      // Log si transition offline → online
      if (wasOffline) {
        console.log(`🟢 Host ${hostId} is back online via heartbeat`)
      }
    } else {
      this.hosts = [...this.hosts, hostData]
      console.log(`🆕 New host detected: ${hostId}`)
    }
    
    // Vérifier tous les hosts pour transitions de statut
    this.checkOfflineHosts()
    
    this.requestUpdate()
  }
  
  checkOfflineHosts() {
    const now = new Date()
    const offlineThreshold = 60 * 1000 // 60 secondes
    
    let hasChanged = false
    
    const updatedHosts = this.hosts.map(host => {
      const wasOnline = host.status === 'online'
      const isOnline = host.lastSeen && (now - host.lastSeen) < offlineThreshold
      const newStatus = isOnline ? 'online' : 'offline'
      
      if (host.status !== newStatus) {
        hasChanged = true
        console.log(`🔄 Host ${host.host_id} status changed: ${host.status} → ${newStatus}`)
      }
      
      return {
        ...host,
        status: newStatus
      }
    })
    
    this.hosts = updatedHosts
    
    // Déclencher une mise à jour du rendu si des statuts ont changé
    if (hasChanged) {
      this.requestUpdate()
    }
  }
  
  async handleWakeHost(host) {
    if (!this.apiService) {
      console.error('❌ API service not available')
      return
    }
    
    try {
      console.log(`⚡ Waking host: ${host.host_id}`)
      await this.apiService.wakeHost(host.host_id)
      console.log(`✅ Wake command sent to ${host.host_id}`)
    } catch (error) {
      console.error(`❌ Failed to wake host ${host.host_id}:`, error)
    }
  }
  
  formatLastSeen(lastSeen) {
    if (!lastSeen) return 'Jamais'
    
    const now = new Date()
    const diff = now - lastSeen
    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    
    if (seconds < 60) return `${seconds}s`
    if (minutes < 60) return `${minutes}m`
    if (hours < 24) return `${hours}h`
    return lastSeen.toLocaleDateString()
  }
  
  render() {
    if (!this.connected) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">💻 Hosts</h3>
        </div>
        <div class="placeholder">
          <div class="placeholder-icon">🔌</div>
          Connexion requise pour surveiller les hosts
        </div>
      `
    }
    
    if (this.hosts.length === 0) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">💻 Hosts</h3>
        </div>
        <div class="placeholder">
          <div class="placeholder-icon">🔍</div>
          <div>Aucun host détecté</div>
          <div style="font-size: 0.8em; margin-top: 0.5rem; opacity: 0.6;">
            Les hosts apparaîtront automatiquement lorsqu'ils enverront des heartbeats
          </div>
        </div>
      `
    }
    
    const onlineCount = this.hosts.filter(h => h.status === 'online').length
    
    return html`
      <div class="widget-header">
        <h3 class="widget-title">💻 Hosts</h3>
        <span class="widget-count">
          ${onlineCount}/${this.hosts.length} en ligne
        </span>
      </div>
      
      <div class="hosts-list">
        ${this.hosts.map(host => html`
          <div class="host-card ${host.status}">
            <div class="host-header">
              <div class="host-name">
                ${host.status === 'online' ? '🟢' : '🔴'}
                ${host.host_id}
              </div>
              <span class="status-badge ${host.status}">
                ${host.status === 'online' ? 'En ligne' : 'Hors ligne'}
              </span>
            </div>
            
            ${host.metrics ? html`
              <div class="host-metrics">
                <div class="metric">
                  <div class="metric-value">${Math.round(host.metrics.cpu)}%</div>
                  <div class="metric-label">CPU</div>
                </div>
                <div class="metric">
                  <div class="metric-value">${Math.round(host.metrics.ram)}%</div>
                  <div class="metric-label">RAM</div>
                </div>
                <div class="metric">
                  <div class="metric-value">${host.net?.ip || 'N/A'}</div>
                  <div class="metric-label">IP</div>
                </div>
              </div>
            ` : ''}
            
            <div class="host-actions">
              <button 
                class="action-btn"
                @click="${() => this.handleWakeHost(host)}"
                ?disabled="${host.status === 'online'}">
                ⚡ Wake
              </button>
            </div>
            
            <div class="host-info">
              <span>ID: ${host.host_id}</span>
              <span class="last-seen">
                Vu: ${this.formatLastSeen(host.lastSeen)}
              </span>
            </div>
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('hosts-widget', HostsWidget)