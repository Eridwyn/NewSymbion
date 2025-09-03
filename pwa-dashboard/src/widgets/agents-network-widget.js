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
import '../services/agents-service.js'

class AgentsNetworkWidget extends LitElement {
  static properties = {
    agents: { type: Array },
    loading: { type: Boolean },
    error: { type: String },
    viewMode: { type: String }, // 'grid' or 'list'
    selectedAgent: { type: Object }
  }
  
  static styles = css`
    :host {
      display: block;
      background: var(--widget-background, #1a1a1a);
      border-radius: 12px;
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
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .widget-title {
      font-size: 18px;
      font-weight: 600;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .view-toggle {
      display: flex;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 6px;
      padding: 2px;
    }

    .view-btn {
      padding: 6px 12px;
      border: none;
      background: transparent;
      color: #ccc;
      cursor: pointer;
      border-radius: 4px;
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
      border-radius: 8px;
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
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 16px;
      transition: all 0.2s ease;
      cursor: pointer;
      position: relative;
    }

    .agent-card:hover {
      background: rgba(255, 255, 255, 0.08);
      border-color: rgba(255, 255, 255, 0.2);
      transform: translateY(-2px);
    }

    .agent-card.online {
      border-left: 4px solid #22c55e;
    }

    .agent-card.offline {
      border-left: 4px solid #ef4444;
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
      font-size: 24px;
      filter: grayscale(0.2);
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
      font-size: 12px;
      font-weight: 500;
      padding: 4px 8px;
      border-radius: 12px;
      text-transform: uppercase;
    }

    .agent-status.online {
      background: rgba(34, 197, 94, 0.2);
      color: #22c55e;
    }

    .agent-status.offline {
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
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
      padding: 6px 12px;
      border: none;
      border-radius: 6px;
      font-size: 12px;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 4px;
    }

    .action-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .action-btn.power {
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
      border: 1px solid rgba(239, 68, 68, 0.3);
    }

    .action-btn.power:hover:not(:disabled) {
      background: rgba(239, 68, 68, 0.3);
    }

    .action-btn.control {
      background: rgba(59, 130, 246, 0.2);
      color: #3b82f6;
      border: 1px solid rgba(59, 130, 246, 0.3);
    }

    .action-btn.control:hover:not(:disabled) {
      background: rgba(59, 130, 246, 0.3);
    }

    .action-btn.wake {
      background: rgba(34, 197, 94, 0.2);
      color: #22c55e;
      border: 1px solid rgba(34, 197, 94, 0.3);
    }

    .action-btn.wake:hover:not(:disabled) {
      background: rgba(34, 197, 94, 0.3);
      transform: scale(1.05);
    }

    .status-indicator {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      display: inline-block;
    }

    .status-indicator.online {
      background: #22c55e;
      box-shadow: 0 0 6px rgba(34, 197, 94, 0.4);
    }

    .status-indicator.offline {
      background: #ef4444;
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
      border-radius: 6px;
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
    }
  `

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
    
    // Auto-refresh toutes les 30 secondes
    this.refreshInterval = setInterval(() => {
      this.loadAgents()
    }, 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval)
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
      this.agents = agents
    } catch (error) {
      console.error('Failed to load agents:', error)
      this.error = `Failed to load agents: ${error.message}`
    } finally {
      this.loading = false
    }
  }

  toggleViewMode() {
    this.viewMode = this.viewMode === 'grid' ? 'list' : 'grid'
  }

  async executeAction(agentId, action, event) {
    event.stopPropagation()
    
    const agent = this.agentsService.getAgentById(agentId)
    
    // Wake-on-LAN permet de réveiller un agent offline
    if (!this.agentsService.isAgentOnline(agentId) && action !== 'wake') {
      alert('⚠️ Agent is offline - cannot execute command (try Wake-on-LAN first)')
      return
    }

    try {
      const confirmMsg = `Are you sure you want to ${action} ${agent.hostname} (${agent.os})?`
      
      if (!confirm(confirmMsg)) return

      switch (action) {
        case 'shutdown':
          await this.agentsService.shutdownAgent(agentId)
          break
        case 'reboot':
          await this.agentsService.rebootAgent(agentId)
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
      console.error(`Failed to ${action} agent:`, error)
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
          <div class="loading-state">
            <div>🔄 Loading agents...</div>
          </div>
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