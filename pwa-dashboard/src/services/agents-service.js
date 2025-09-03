/**
 * Service Agents Symbion
 * 
 * Interface avec l'API REST agents du kernel Symbion
 * Gère le contrôle système à distance multi-OS
 */

import { LitElement } from 'lit'
import { ApiService } from './api-service.js'

class AgentsService extends LitElement {
  static properties = {
    agents: { type: Array },
    status: { type: String }
  }
  
  constructor() {
    super()
    this.agents = []
    this.status = 'loading'
    this.apiService = null
  }
  
  connectedCallback() {
    super.connectedCallback()
    this.initApiService()
  }
  
  async initApiService() {
    // Utilise le service API existant 
    this.apiService = document.querySelector('api-service') || new ApiService()
    if (!document.querySelector('api-service')) {
      document.body.appendChild(this.apiService)
    }
    
    // Écoute les changements de statut API
    this.apiService.addEventListener('status-change', (e) => {
      this.status = e.detail.status
    })
  }
  
  // ===== Agents Management =====
  
  async getAgents() {
    try {
      const agents = await this.apiService.request('/agents')
      this.agents = agents
      this.dispatchEvent(new CustomEvent('agents-updated', {
        detail: { agents },
        bubbles: true
      }))
      return agents
    } catch (error) {
      console.error('❌ Failed to fetch agents:', error)
      throw error
    }
  }
  
  async getAgent(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}`)
  }
  
  // ===== Power Management =====
  
  async shutdownAgent(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/shutdown`, {
      method: 'POST'
    })
  }
  
  async rebootAgent(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/reboot`, {
      method: 'POST'
    })
  }
  
  async hibernateAgent(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/hibernate`, {
      method: 'POST'
    })
  }
  
  // ===== Process Control =====
  
  async getAgentProcesses(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/processes`)
  }
  
  async killAgentProcess(agentId, pid) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/processes/${pid}/kill`, {
      method: 'POST'
    })
  }
  
  // ===== Command Execution =====
  
  async executeCommand(agentId, command) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/command`, {
      method: 'POST',
      body: JSON.stringify({ command })
    })
  }
  
  // ===== Metrics =====
  
  async getAgentMetrics(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/metrics`)
  }
  
  // ===== Wake-on-LAN =====
  
  async wakeAgent(agentId) {
    // Utilise l'API wake existante avec agent_id comme host_id
    return await this.apiService.request(`/wake?host_id=${encodeURIComponent(agentId)}`, {
      method: 'POST'
    })
  }
  
  // ===== Helpers =====
  
  getAgentById(agentId) {
    return this.agents.find(agent => agent.agent_id === agentId)
  }
  
  getOnlineAgents() {
    return this.agents.filter(agent => agent.status === 'online')
  }
  
  getOfflineAgents() {
    return this.agents.filter(agent => agent.status === 'offline')
  }
  
  getAgentsByOS(os) {
    return this.agents.filter(agent => agent.os.toLowerCase() === os.toLowerCase())
  }
  
  isAgentOnline(agentId) {
    const agent = this.getAgentById(agentId)
    return agent && agent.status === 'online'
  }
  
  hasCapability(agentId, capability) {
    const agent = this.getAgentById(agentId)
    return agent && agent.capabilities && agent.capabilities.includes(capability)
  }
  
  canExecutePowerCommands(agentId) {
    return this.hasCapability(agentId, 'power_management')
  }
  
  canControlProcesses(agentId) {
    return this.hasCapability(agentId, 'process_control')
  }
  
  canExecuteCommands(agentId) {
    return this.hasCapability(agentId, 'command_execution')
  }
  
  formatLastSeen(agent) {
    if (!agent.last_seen) return 'Never'
    
    const lastSeen = new Date(agent.last_seen)
    const now = new Date()
    const diffMinutes = Math.round((now - lastSeen) / (1000 * 60))
    
    if (diffMinutes < 1) return 'Just now'
    if (diffMinutes < 60) return `${diffMinutes}m ago`
    
    const diffHours = Math.round(diffMinutes / 60)
    if (diffHours < 24) return `${diffHours}h ago`
    
    const diffDays = Math.round(diffHours / 24)
    return `${diffDays}d ago`
  }
  
  getOSIcon(os) {
    switch (os.toLowerCase()) {
      case 'linux': return '🐧'
      case 'windows': return '🪟'
      case 'android': return '🤖'
      case 'macos': return '🍎'
      default: return '💻'
    }
  }
  
  getStatusColor(agent) {
    if (!agent) return '#666'
    
    switch (agent.status) {
      case 'online': return '#22c55e'  // Green
      case 'offline': return '#ef4444' // Red  
      case 'unknown': return '#f59e0b' // Amber
      default: return '#6b7280'        // Gray
    }
  }
}

customElements.define('agents-service', AgentsService)

export { AgentsService }