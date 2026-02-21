/**
 * Widget Agents Network - Vue réseau des agents Symbion
 * 
 * Affiche tous les agents du réseau local avec:
 * - Statut en temps réel (online/offline)
 * - Informations système (OS, hostname, IP)
 * - Actions rapides (power management)
 * - Vue carte réseau interactive
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import '../services/agents-service.js'
import '../components/organic-loader.js'
import pollingScheduler from '../services/polling-scheduler.js'

class AgentsNetworkWidget extends LitElement {
  static properties = {
    agents: { type: Array },
    loading: { type: Boolean },
    error: { type: String },
    viewMode: { type: String }, // 'grid' or 'list'
    selectedAgent: { type: Object }
  }
  
  static styles = [sharedAnimations, css`
    :host {
      display: block;
      background: var(--widget-background, #1a1a1a);
      border-radius: var(--radius-md);
      padding: 20px;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
      color: var(--widget-color, #e5e5e5);
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    }

    .widget-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
      padding-bottom: 12px;
      border-bottom: 1px solid var(--border-medium);
    }

    .widget-title {
      font-size: 18px;
      font-weight: 600;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 8px;
      animation: textGlow var(--bio-breathe-fast, 8s) ease-in-out infinite;
    }

    .view-toggle {
      display: flex;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm);
      padding: 2px;
    }

    .view-btn {
      padding: 6px 12px;
      border: none;
      background: transparent;
      color: #ccc;
      cursor: pointer;
      border-radius: var(--radius-sm);
      transition: all 0.2s ease;
      font-size: 14px;
    }

    .view-btn.active {
      background: #3b82f6;
      color: white;
    }

    .agents-container {
      min-height: 200px;
    }

    .loading-state {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 40px;
      color: #888;
      font-size: 14px;
    }

    .error-state {
      padding: 20px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: var(--radius-base);
      color: #fca5a5;
      text-align: center;
    }

    .agents-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
      gap: 16px;
    }

    .agents-list {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }

    .agent-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-md);
      padding: 18px;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      cursor: pointer;
      position: relative;
      overflow: hidden;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    }

    .agent-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 4px;
      height: 100%;
      transition: all var(--duration-base) var(--ease-out);
    }

    .agent-card.online::before {
      background: linear-gradient(180deg, var(--context-primary, #00d4aa) 0%, #22c55e 100%);
      box-shadow: 0 0 20px var(--ctx-border-intense);
    }

    .agent-card.offline::before {
      background: linear-gradient(180deg, #ef4444 0%, #dc2626 100%);
      box-shadow: 0 0 15px rgba(239, 68, 68, 0.3);
    }

    .agent-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-4px) scale(1.02);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
    }

    .agent-card:hover::before {
      width: 6px;
    }

    .agent-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }

    .agent-info {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .os-icon {
      font-size: 28px;
      filter: grayscale(0) drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
      transition: all var(--duration-base) var(--ease-out);
    }

    .agent-card:hover .os-icon {
      transform: scale(1.1) rotate(-5deg);
    }

    .agent-details {
      flex: 1;
    }

    .agent-hostname {
      font-size: 16px;
      font-weight: 600;
      color: #ffffff;
      margin-bottom: 4px;
    }

    .agent-os {
      font-size: 12px;
      color: #888;
      text-transform: capitalize;
    }

    .agent-status {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      font-weight: 600;
      padding: 6px 12px;
      border-radius: var(--radius-lg);
      text-transform: uppercase;
      letter-spacing: 0.8px;
      transition: all var(--duration-base) var(--ease-out);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    .agent-status.online {
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.25) 0%, var(--ctx-border-medium) 100%);
      color: var(--context-primary, #00d4aa);
      border: 1px solid var(--ctx-border-strong);
      box-shadow: 0 2px 12px var(--ctx-border-strong);
    }

    .agent-status.offline {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(220, 38, 38, 0.2) 100%);
      color: #fca5a5;
      border: 1px solid rgba(239, 68, 68, 0.3);
      box-shadow: 0 2px 12px rgba(239, 68, 68, 0.25);
    }

    .agent-meta {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
      margin-bottom: 12px;
      font-size: 13px;
    }

    .meta-item {
      display: flex;
      flex-direction: column;
      gap: 2px;
    }

    .meta-label {
      color: #888;
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .meta-value {
      color: #ccc;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 12px;
    }

    .agent-actions {
      display: flex;
      gap: 8px;
      justify-content: flex-end;
    }

    .action-btn {
      padding: 8px 14px;
      border: none;
      border-radius: var(--radius-base);
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      display: flex;
      align-items: center;
      gap: 5px;
      position: relative;
      overflow: hidden;
    }

    .action-btn::before {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: 0;
      height: 0;
      border-radius: 50%;
      background: var(--surface-glass-strong);
      transform: translate(-50%, -50%);
      transition: width 0.4s, height 0.4s;
    }

    .action-btn:hover::before {
      width: 200px;
      height: 200px;
    }

    .action-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none !important;
    }

    .action-btn.power {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(220, 38, 38, 0.2) 100%);
      color: #fca5a5;
      border: 1px solid rgba(239, 68, 68, 0.4);
      box-shadow: 0 2px 8px rgba(239, 68, 68, 0.2);
    }

    .action-btn.power:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.35) 0%, rgba(220, 38, 38, 0.3) 100%);
      border-color: rgba(239, 68, 68, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(239, 68, 68, 0.35);
    }

    .action-btn.control {
      background: linear-gradient(135deg, rgba(59, 130, 246, 0.25) 0%, rgba(37, 99, 235, 0.2) 100%);
      color: #60a5fa;
      border: 1px solid rgba(59, 130, 246, 0.4);
      box-shadow: 0 2px 8px rgba(59, 130, 246, 0.2);
    }

    .action-btn.control:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(59, 130, 246, 0.35) 0%, rgba(37, 99, 235, 0.3) 100%);
      border-color: rgba(59, 130, 246, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(59, 130, 246, 0.35);
    }

    .action-btn.wake {
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.25) 0%, var(--ctx-border-medium) 100%);
      color: #4ade80;
      border: 1px solid rgba(34, 197, 94, 0.4);
      box-shadow: 0 2px 8px rgba(34, 197, 94, 0.2);
    }

    .action-btn.wake:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.35) 0%, var(--ctx-border-strong) 100%);
      border-color: rgba(0, 212, 170, 0.6);
      transform: translateY(-2px) scale(1.05);
      box-shadow: 0 4px 16px rgba(0, 212, 170, 0.35);
    }

    .action-btn.delete {
      background: linear-gradient(135deg, rgba(156, 163, 175, 0.2) 0%, rgba(107, 114, 128, 0.15) 100%);
      color: #9ca3af;
      border: 1px solid rgba(156, 163, 175, 0.3);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    }

    .action-btn.delete:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(220, 38, 38, 0.2) 100%);
      border-color: rgba(239, 68, 68, 0.5);
      color: #fca5a5;
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(239, 68, 68, 0.25);
    }

    .status-indicator {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      display: inline-block;
      transition: all var(--duration-base) var(--ease-out);
    }

    .status-indicator.online {
      background: var(--context-primary, #00d4aa);
      box-shadow: 0 0 12px rgba(0, 212, 170, 0.7), 0 0 24px var(--ctx-border-strong);
      animation: pulse-glow 2s ease-in-out infinite;
    }

    .status-indicator.offline {
      background: #ef4444;
      box-shadow: 0 0 8px rgba(239, 68, 68, 0.4);
    }

    @keyframes pulse-glow {
      0%, 100% {
        transform: scale(1);
        opacity: 1;
      }
      50% {
        transform: scale(1.15);
        opacity: 0.85;
      }
    }

    .empty-state {
      text-align: center;
      padding: 40px 20px;
      color: #888;
    }

    .empty-state h3 {
      margin: 0 0 8px 0;
      color: #ccc;
      font-size: 18px;
    }

    .empty-state p {
      margin: 0;
      font-size: 14px;
      line-height: 1.5;
    }

    .refresh-btn {
      background: rgba(59, 130, 246, 0.2);
      color: #3b82f6;
      border: 1px solid rgba(59, 130, 246, 0.3);
      border-radius: var(--radius-sm);
      padding: 8px 16px;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 14px;
    }

    .refresh-btn:hover {
      background: rgba(59, 130, 246, 0.3);
    }

    /* Responsive */
    @media (max-width: 768px) {
      .agents-grid {
        grid-template-columns: 1fr;
      }

      .agent-meta {
        grid-template-columns: 1fr;
      }

      .widget-header {
        flex-direction: column;
        gap: 12px;
        align-items: stretch;
      }

      .agent-card {
        padding: 14px;
      }

      .agent-actions {
        flex-wrap: wrap;
      }

      .action-btn {
        flex: 1;
        min-width: 100px;
        justify-content: center;
      }

      .os-icon {
        font-size: 24px;
      }

      .agent-hostname {
        font-size: 14px;
      }
    }
  `]

  constructor() {
    super()
    this.agents = []
    this.loading = true
    this.error = null
    this.viewMode = 'grid'
    this.selectedAgent = null
    this.agentsService = null
  }

  connectedCallback() {
    super.connectedCallback()
    this.initializeService()
    this.loadAgents()

    // Auto-refresh via scheduler centralisé (30s, pause si onglet caché)
    this._unsubscribeRefresh = pollingScheduler.subscribe('30s', () => this.loadAgents())
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._unsubscribeRefresh) {
      this._unsubscribeRefresh()
      this._unsubscribeRefresh = null
    }
  }

  async initializeService() {
    this.agentsService = document.querySelector('agents-service')
    if (!this.agentsService) {
      this.agentsService = document.createElement('agents-service')
      document.body.appendChild(this.agentsService)
    }
  }

  async loadAgents() {
    try {
      this.loading = true
      this.error = null
      const agents = await this.agentsService.getAgents()
      this.agents = Array.isArray(agents) ? agents : []
    } catch (error) {
      console.error('Failed to load agents:', error)
      this.error = `Failed to load agents: ${error.message}`
      this.agents = []
    } finally {
      this.loading = false
    }
  }

  toggleViewMode() {
    this.viewMode = this.viewMode === 'grid' ? 'list' : 'grid'
  }

  async executeAction(agentId, action, event) {
    event.stopPropagation()

    console.log(`[agents-network-widget] executeAction called: agentId=${agentId}, action=${action}`)
    console.log(`[agents-network-widget] agentsService:`, this.agentsService)

    const agent = this.agentsService.getAgentById(agentId)
    console.log(`[agents-network-widget] agent:`, agent)

    // Protection: empêcher le shutdown de l'agent qui héberge le kernel/dashboard
    const hostAgentIP = window.SYMBION_CONFIG?.HOST_AGENT_IP
    if (hostAgentIP && agent.primary_ip === hostAgentIP && (action === 'shutdown' || action === 'reboot')) {
      alert(`⚠️ Cannot ${action} host agent (${agent.hostname}) - this would disconnect the dashboard and kernel!\n\nPlease ${action} from the machine directly if needed.`)
      return
    }

    // Wake-on-LAN permet de réveiller un agent offline
    if (!this.agentsService.isAgentOnline(agentId) && action !== 'wake') {
      alert('⚠️ Agent is offline - cannot execute command (try Wake-on-LAN first)')
      return
    }

    try {
      const confirmMsg = `Are you sure you want to ${action} ${agent.hostname} (${agent.os})?`

      if (!confirm(confirmMsg)) {
        console.log(`[agents-network-widget] User cancelled action`)
        return
      }

      console.log(`[agents-network-widget] Executing ${action}...`)

      switch (action) {
        case 'shutdown':
          console.log(`[agents-network-widget] Calling shutdownAgent(${agentId})`)
          const shutdownResult = await this.agentsService.shutdownAgent(agentId)
          console.log(`[agents-network-widget] Shutdown result:`, shutdownResult)
          alert(`✅ Shutdown command sent: ${shutdownResult.command_id}`)
          break
        case 'reboot':
          console.log(`[agents-network-widget] Calling rebootAgent(${agentId})`)
          const rebootResult = await this.agentsService.rebootAgent(agentId)
          console.log(`[agents-network-widget] Reboot result:`, rebootResult)
          alert(`✅ Reboot command sent: ${rebootResult.command_id}`)
          break
        case 'wake':
          await this.agentsService.wakeAgent(agentId)
          alert(`🌟 Wake-on-LAN packet sent to ${agent.hostname}`)
          break
        case 'control':
          this.openControlModal(agentId)
          return
      }

      // Refresh après action
      setTimeout(() => this.loadAgents(), 1000)

    } catch (error) {
      console.error(`[agents-network-widget] Failed to ${action} agent:`, error)
      console.error(`[agents-network-widget] Error stack:`, error.stack)
      alert(`❌ Failed to ${action} agent: ${error.message}`)
    }
  }

  openControlModal(agentId) {
    // Émission d'un événement pour ouvrir le modal de contrôle
    const event = new CustomEvent('open-agent-control', {
      detail: { agentId },
      bubbles: true
    })
    console.log('Emitting open-agent-control event for:', agentId)
    document.dispatchEvent(event)
  }

  async confirmDeleteAgent(agent, event) {
    event.stopPropagation()

    // Protection: empêcher la suppression de l'agent hôte
    const hostAgentIP = window.SYMBION_CONFIG?.HOST_AGENT_IP
    if (hostAgentIP && agent.primary_ip === hostAgentIP) {
      alert(`⚠️ Impossible de supprimer l'agent hôte (${agent.hostname}) - cela désactiverait le dashboard !`)
      return
    }

    const confirmMsg = `Supprimer l'agent "${agent.hostname}" ?\n\n` +
      `L'agent sera marqué comme supprimé et définitivement effacé après 7 jours.\n\n` +
      `S'il se reconnecte pendant cette période, il sera réactivé automatiquement.`

    if (!confirm(confirmMsg)) {
      return
    }

    try {
      await this.agentsService.deleteAgent(agent.agent_id)
      alert(`✅ Agent "${agent.hostname}" supprimé (purge dans 7 jours)`)

      // Refresh la liste
      await this.loadAgents()

    } catch (error) {
      console.error(`[agents-network-widget] Failed to delete agent:`, error)
      alert(`❌ Erreur lors de la suppression: ${error.message}`)
    }
  }

  renderAgent(agent) {
    const isOnline = agent.status === 'online'
    const lastSeen = this.agentsService?.formatLastSeen(agent) || 'Unknown'
    
    return html`
      <div class="agent-card ${agent.status}" @click="${() => this.openControlModal(agent.agent_id)}">
        <div class="agent-header">
          <div class="agent-info">
            <span class="os-icon">${this.agentsService?.getOSIcon(agent.os) || '💻'}</span>
            <div class="agent-details">
              <div class="agent-hostname">${agent.hostname}</div>
              <div class="agent-os">${agent.architecture} • ${agent.os}</div>
            </div>
          </div>
          <div class="agent-status ${agent.status}">
            <span class="status-indicator ${agent.status}"></span>
            ${agent.status}
          </div>
        </div>

        <div class="agent-meta">
          <div class="meta-item">
            <span class="meta-label">IP Address</span>
            <span class="meta-value">${agent.primary_ip}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">Last Seen</span>
            <span class="meta-value">${lastSeen}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">MAC Address</span>
            <span class="meta-value">${agent.primary_mac}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">Agent ID</span>
            <span class="meta-value">${agent.agent_id}</span>
          </div>
        </div>

        <div class="agent-actions">
          ${isOnline ? html`
            <!-- Actions pour agents online -->
            <button
              class="action-btn power"
              @click="${(e) => this.executeAction(agent.agent_id, 'shutdown', e)}"
              title="Shutdown system">
              🔴 Shutdown
            </button>
            <button
              class="action-btn power"
              @click="${(e) => this.executeAction(agent.agent_id, 'reboot', e)}"
              title="Reboot system">
              🔄 Reboot
            </button>
            <button
              class="action-btn control"
              @click="${(e) => this.executeAction(agent.agent_id, 'control', e)}"
              title="Detailed control">
              🛠️ Control
            </button>
            <button
              class="action-btn delete"
              @click="${(e) => this.confirmDeleteAgent(agent, e)}"
              title="Supprimer l'agent (purge après 7 jours)">
              🗑️
            </button>
          ` : html`
            <!-- Actions pour agents offline -->
            <button
              class="action-btn wake"
              @click="${(e) => this.executeAction(agent.agent_id, 'wake', e)}"
              title="Wake-on-LAN - Power on remotely">
              🌟 Wake Up
            </button>
            <button
              class="action-btn control"
              @click="${(e) => this.executeAction(agent.agent_id, 'control', e)}"
              title="View system information">
              📊 Info
            </button>
            <button
              class="action-btn delete"
              @click="${(e) => this.confirmDeleteAgent(agent, e)}"
              title="Supprimer l'agent (purge après 7 jours)">
              🗑️
            </button>
          `}
        </div>
      </div>
    `
  }

  render() {
    return html`
      <div class="widget-header">
        <div class="widget-title">
          🌐 Network Agents
          ${this.agents.length > 0 ? html`<span>(${this.agents.length})</span>` : ''}
        </div>
        <div class="view-toggle">
          <button 
            class="view-btn ${this.viewMode === 'grid' ? 'active' : ''}"
            @click="${() => this.viewMode = 'grid'}">
            ▦ Grid
          </button>
          <button 
            class="view-btn ${this.viewMode === 'list' ? 'active' : ''}"
            @click="${() => this.viewMode = 'list'}">
            ☰ List
          </button>
        </div>
      </div>

      <div class="agents-container">
        ${this.loading ? html`
          <organic-loader text="🌐 Chargement agents réseau..."></organic-loader>
        ` : this.error ? html`
          <div class="error-state">
            <div>❌ ${this.error}</div>
            <button class="refresh-btn" @click="${this.loadAgents}">
              🔄 Retry
            </button>
          </div>
        ` : this.agents.length === 0 ? html`
          <div class="empty-state">
            <h3>🤖 No Agents Found</h3>
            <p>No agents are registered in the network.<br>
               Deploy symbion-agent-host on your systems to start monitoring.</p>
            <br>
            <button class="refresh-btn" @click="${this.loadAgents}">
              🔄 Refresh
            </button>
          </div>
        ` : html`
          <div class="agents-${this.viewMode}">
            ${this.agents.map(agent => this.renderAgent(agent))}
          </div>
        `}
      </div>
    `
  }
}

customElements.define('agents-network-widget', AgentsNetworkWidget)

export { AgentsNetworkWidget }