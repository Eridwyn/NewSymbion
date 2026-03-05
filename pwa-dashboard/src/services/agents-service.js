/**
 * Service Agents Symbion
 * 
 * Interface avec l'API REST agents du kernel Symbion
 * Gère le contrôle système à distance multi-OS
 */

import { LitElement } from 'lit'
import { ApiService } from './api-service.js'
import csrfService from './csrf-service.js'
import authService from './auth-service.js'
import { getApiBase } from './config.js'

const API_BASE = getApiBase()

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
    // [P0-5] Store handler reference for cleanup
    this._statusChangeHandler = null
    // Cache for latest agent version (refreshed every 5 min)
    this._latestVersionCache = null
    this._latestVersionCacheTime = 0
  }

  connectedCallback() {
    super.connectedCallback()
    this.initApiService()
  }

  // [P0-5] Cleanup event listeners to prevent memory leaks
  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.apiService && this._statusChangeHandler) {
      this.apiService.removeEventListener('status-change', this._statusChangeHandler)
    }
  }

  async initApiService() {
    // Utilise le service API existant
    this.apiService = document.querySelector('api-service') || new ApiService()
    if (!document.querySelector('api-service')) {
      document.body.appendChild(this.apiService)
    }

    // [P0-5] Store handler for cleanup, then add listener
    this._statusChangeHandler = (e) => {
      this.status = e.detail.status
    }
    this.apiService.addEventListener('status-change', this._statusChangeHandler)
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
    console.log(`[agents-service] shutdownAgent called: agentId=${agentId}`)

    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/shutdown`

    try {
      const response = await csrfService.fetchWithCsrf(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        }
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`)
      }

      const result = await response.json()
      console.log(`[agents-service] Shutdown result:`, result)
      return result
    } catch (error) {
      console.error(`[agents-service] Shutdown error:`, error)
      throw error
    }
  }
  
  async rebootAgent(agentId) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/reboot`

    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      }
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    }

    return await response.json()
  }
  
  async hibernateAgent(agentId) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/hibernate`

    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      }
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    }

    return await response.json()
  }

  // ===== Agent Deletion (soft delete, purge after 7 days) =====

  async deleteAgent(agentId) {
    console.log(`[agents-service] deleteAgent called: agentId=${agentId}`)

    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}`

    const response = await csrfService.fetchWithCsrf(url, {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json'
      }
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`HTTP ${response.status}: ${errorText}`)
    }

    console.log(`[agents-service] Agent ${agentId} deleted successfully`)
    return true
  }

  // ===== Process Control =====
  
  async getAgentProcesses(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/processes`)
  }
  
  async killAgentProcess(agentId, pid) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/processes/${pid}/kill`

    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      }
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    }

    return await response.json()
  }
  
  // ===== Command Execution Enhanced (CSRF protected) =====

  async executeCommand(agentId, command, timeout_secs = 30) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/command`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command, timeout_secs })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  async executeCommandWithTracking(agentId, command, timeout_secs = 30) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/commands`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        command_type: 'shell_command',
        parameters: { command, timeout_secs }
      })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  async getCommandStatus(commandId) {
    // Direct fetch with auth (bypass apiService to avoid null reference during polling)
    const url = `${API_BASE}/v1/commands/${encodeURIComponent(commandId)}/status`

    const headers = { 'Content-Type': 'application/json' }
    if (authService.isAuthenticated()) {
      headers['Authorization'] = `Bearer ${authService.getToken()}`
    }

    const response = await fetch(url, { headers })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return await response.json()
  }

  async cancelCommand(commandId) {
    const url = `${API_BASE}/v1/commands/${encodeURIComponent(commandId)}/cancel`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }
  
  // ===== Metrics =====

  async getAgentMetrics(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/metrics`)
  }

  // ===== Services =====

  async getAgentServices(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/services`)
  }

  async controlService(agentId, serviceName, action) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/services/${encodeURIComponent(serviceName)}/${encodeURIComponent(action)}`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  // ===== Command History =====

  async getCommandHistory(agentId, limit = 50, offset = 0) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/commands/history?limit=${limit}&offset=${offset}`)
  }

  // ===== Agent Logs =====

  async getAgentLogs(agentId, level = null) {
    const query = level ? `?level=${encodeURIComponent(level)}` : ''
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/logs${query}`)
  }

  // ===== Agent Watchdog (v2.5+) =====

  async getAgentWatchdog(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/watchdog`)
  }

  // ===== Agent Plugins (v2.5+) =====

  async getAgentPlugins(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/plugins`)
  }

  async sendPluginCommand(agentId, pluginId, action, parameters = null) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/plugins/${encodeURIComponent(pluginId)}/command`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action, parameters })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  // ===== Agent Notifications (v2.5+) =====

  async notifyAgent(agentId, title, body, urgency = 'normal', timeout_ms = 5000) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/notify`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title, body, urgency, timeout_ms })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  // ===== Agent Screenshot (v2.5+) =====

  async takeScreenshot(agentId, notifyBefore = true) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/screenshot`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ notify_before: notifyBefore })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  // ===== Agent Scheduled Tasks (v2.5+) =====

  async getScheduledTasks(agentId) {
    return await this.apiService.request(`/agents/${encodeURIComponent(agentId)}/scheduled-tasks`)
  }

  async createScheduledTask(agentId, name, commandType, schedule, parameters = null) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/scheduled-tasks`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, command_type: commandType, schedule, parameters })
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  async deleteScheduledTask(agentId, taskName) {
    const url = `${API_BASE}/v1/agents/${encodeURIComponent(agentId)}/scheduled-tasks/${encodeURIComponent(taskName)}`
    const response = await csrfService.fetchWithCsrf(url, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' }
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
    return await response.json()
  }

  // ===== Latest Agent Version (cached 5 min) =====

  async getLatestAgentVersion() {
    const now = Date.now()
    const CACHE_TTL = 5 * 60 * 1000 // 5 minutes
    if (this._latestVersionCache && (now - this._latestVersionCacheTime) < CACHE_TTL) {
      return this._latestVersionCache
    }
    try {
      const result = await this.apiService.request('/agents/latest-version')
      this._latestVersionCache = result.version || null
      this._latestVersionCacheTime = now
      return this._latestVersionCache
    } catch (error) {
      console.error('[agents-service] Failed to fetch latest agent version:', error)
      return this._latestVersionCache // Return stale cache on error
    }
  }

  // ===== Agent Reconnection =====

  async reconnectAgent(agentId) {
    // Route via kernel pour respecter la sécurité centralisée
    return await this.apiService.request(`/v1/agents/${encodeURIComponent(agentId)}/reconnect`, {
      method: 'POST'
    })
  }

  getAgentLocalDashboardURL(agentIP, port = 9899) {
    return `http://${agentIP}:${port}/`
  }
  
  // ===== Wake-on-LAN =====
  
  async wakeAgent(agentId) {
    // Utilise l'API wake existante avec agent_id comme host_id
    return await this.apiService.request(`/v1/wake?host_id=${encodeURIComponent(agentId)}`, {
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

  canManageServices(agentId) {
    return this.hasCapability(agentId, 'service_management')
  }

  hasLocalDashboard(agentId) {
    // Check if agent is online and has local API capabilities  
    const agent = this.getAgentById(agentId)
    return agent && agent.status === 'online' && agent.primary_ip
  }

  getAgentIP(agentId) {
    const agent = this.getAgentById(agentId)
    return agent ? agent.primary_ip : null
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