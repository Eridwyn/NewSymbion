/**
 * Widget Agent Control - Contrôle détaillé d'un agent système
 * 
 * Modal avec 5 tabs pour contrôle complet:
 * - System: Power management, infos système
 * - Processes: Liste processus + kill
 * - Metrics: CPU, RAM, disque temps réel 
 * - Commands: Exécution commandes shell
 * - Services: Gestion services système
 */

import { LitElement, html, css } from 'lit'
import '../services/agents-service.js'

class AgentControlWidget extends LitElement {
  static properties = {
    agentId: { type: String },
    agent: { type: Object },
    isOpen: { type: Boolean },
    currentTab: { type: String },
    loading: { type: Boolean },
    refreshing: { type: Boolean },
    processes: { type: Array },
    metrics: { type: Object },
    commandOutput: { type: String },
    commandInput: { type: String }
  }
  
  static styles = css`
    :host {
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: rgba(0, 0, 0, 0.8);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    }

    :host(:not([is-open])) {
      display: none;
    }

    .modal {
      background: var(--widget-background, #1a1a1a);
      border-radius: 16px;
      width: 90%;
      max-width: 900px;
      height: 80%;
      max-height: 700px;
      display: flex;
      flex-direction: column;
      box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.4);
      color: var(--widget-color, #e5e5e5);
      overflow: hidden;
    }

    .modal-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 20px 24px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .modal-title {
      display: flex;
      align-items: center;
      gap: 12px;
      font-size: 20px;
      font-weight: 600;
      color: #ffffff;
    }

    .os-icon {
      font-size: 32px;
    }

    .agent-info {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .agent-hostname {
      font-size: 20px;
      font-weight: 600;
      color: #ffffff;
    }

    .agent-meta {
      font-size: 14px;
      color: #888;
    }

    .status-badge {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      border-radius: 12px;
      font-size: 12px;
      font-weight: 600;
      text-transform: uppercase;
    }

    .status-badge.online {
      background: rgba(34, 197, 94, 0.2);
      color: #22c55e;
    }

    .status-badge.offline {
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
    }

    .close-btn {
      background: none;
      border: none;
      color: #888;
      font-size: 24px;
      cursor: pointer;
      padding: 8px;
      border-radius: 8px;
      transition: all 0.2s ease;
    }

    .close-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      color: #ccc;
    }

    .modal-tabs {
      display: flex;
      background: rgba(255, 255, 255, 0.05);
      padding: 0 24px;
      overflow-x: auto;
    }

    .tab-btn {
      padding: 12px 20px;
      border: none;
      background: transparent;
      color: #888;
      cursor: pointer;
      border-bottom: 3px solid transparent;
      font-size: 14px;
      font-weight: 500;
      white-space: nowrap;
      transition: all 0.2s ease;
    }

    .tab-btn.active {
      color: #3b82f6;
      border-bottom-color: #3b82f6;
    }

    .tab-btn:hover {
      color: #ccc;
    }

    .modal-content {
      flex: 1;
      overflow: auto;
      padding: 24px;
    }

    .tab-panel {
      display: none;
      animation: fadeIn 0.2s ease;
    }

    .tab-panel.active {
      display: block;
    }

    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(10px); }
      to { opacity: 1; transform: translateY(0); }
    }

    .section {
      margin-bottom: 24px;
    }

    .section-title {
      font-size: 16px;
      font-weight: 600;
      color: #ffffff;
      margin-bottom: 12px;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .power-controls {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }

    .power-btn {
      padding: 12px 20px;
      border: none;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .power-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .power-btn.danger {
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
      border: 1px solid rgba(239, 68, 68, 0.3);
    }

    .power-btn.danger:hover:not(:disabled) {
      background: rgba(239, 68, 68, 0.3);
    }

    .power-btn.warning {
      background: rgba(245, 158, 11, 0.2);
      color: #f59e0b;
      border: 1px solid rgba(245, 158, 11, 0.3);
    }

    .power-btn.warning:hover:not(:disabled) {
      background: rgba(245, 158, 11, 0.3);
    }

    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 16px;
    }

    .info-card {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 16px;
    }

    .info-label {
      font-size: 12px;
      color: #888;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 6px;
    }

    .info-value {
      font-size: 16px;
      color: #ffffff;
      font-family: 'Monaco', 'Consolas', monospace;
    }

    .processes-table {
      background: rgba(255, 255, 255, 0.05);
      border-radius: 8px;
      overflow: hidden;
    }

    .process-header {
      display: grid;
      grid-template-columns: 80px 1fr 100px 100px 80px;
      gap: 16px;
      padding: 12px 16px;
      background: rgba(255, 255, 255, 0.1);
      font-size: 12px;
      font-weight: 600;
      color: #888;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .process-row {
      display: grid;
      grid-template-columns: 80px 1fr 100px 100px 80px;
      gap: 16px;
      padding: 12px 16px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
      font-size: 14px;
      align-items: center;
      transition: background 0.2s ease;
    }

    .process-row:hover {
      background: rgba(255, 255, 255, 0.05);
    }

    .process-name {
      font-family: 'Monaco', 'Consolas', monospace;
      color: #ffffff;
    }

    .kill-btn {
      padding: 4px 8px;
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: 4px;
      font-size: 11px;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .kill-btn:hover {
      background: rgba(239, 68, 68, 0.3);
    }

    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
      gap: 16px;
    }

    .metric-card {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 20px;
      text-align: center;
    }

    .metric-value {
      font-size: 32px;
      font-weight: 700;
      color: #ffffff;
      margin: 8px 0;
    }

    .metric-label {
      font-size: 14px;
      color: #888;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .progress-bar {
      width: 100%;
      height: 6px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 3px;
      margin-top: 12px;
      overflow: hidden;
    }

    .progress-fill {
      height: 100%;
      border-radius: 3px;
      transition: all 0.3s ease;
    }

    .progress-fill.cpu { background: linear-gradient(90deg, #22c55e, #ef4444); }
    .progress-fill.memory { background: linear-gradient(90deg, #3b82f6, #8b5cf6); }
    .progress-fill.disk { background: linear-gradient(90deg, #f59e0b, #ef4444); }

    .command-section {
      display: flex;
      flex-direction: column;
      gap: 16px;
    }

    .command-input {
      display: flex;
      gap: 12px;
    }

    .command-field {
      flex: 1;
      padding: 12px 16px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 8px;
      color: #ffffff;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 14px;
    }

    .command-field:focus {
      outline: none;
      border-color: #3b82f6;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
    }

    .execute-btn {
      padding: 12px 20px;
      background: #3b82f6;
      color: white;
      border: none;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .execute-btn:hover {
      background: #2563eb;
    }

    .execute-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .command-output {
      background: #0d1117;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 16px;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 13px;
      color: #e6edf3;
      white-space: pre-wrap;
      overflow-y: auto;
      max-height: 300px;
      min-height: 150px;
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
      padding: 16px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: 8px;
      color: #fca5a5;
      text-align: center;
    }

    /* Responsive */
    @media (max-width: 768px) {
      .modal {
        width: 95%;
        height: 90%;
      }
      
      .info-grid, .metrics-grid {
        grid-template-columns: 1fr;
      }
      
      .power-controls {
        flex-direction: column;
      }
      
      .command-input {
        flex-direction: column;
      }
      
      .process-header,
      .process-row {
        grid-template-columns: 60px 1fr 60px;
        font-size: 12px;
      }
      
      .process-header .cpu-col,
      .process-header .memory-col,
      .process-row .cpu-col,
      .process-row .memory-col {
        display: none;
      }
    }
  `

  constructor() {
    super()
    this.agentId = null
    this.agent = null
    this.isOpen = false
    this.currentTab = 'system'
    this.loading = false
    this.refreshing = false
    this.processes = []
    this.metrics = null
    this.commandOutput = '# Command output will appear here...\n'
    this.commandInput = ''
    this.agentsService = null
  }

  connectedCallback() {
    super.connectedCallback()
    this.initializeService()
    
    // Fermer modal avec Escape
    this.handleEscape = (e) => {
      if (e.key === 'Escape' && this.isOpen) {
        this.close()
      }
    }
    document.addEventListener('keydown', this.handleEscape)
    
    // Écouter les événements d'ouverture du modal
    this.handleOpenEvent = (e) => {
      console.log('Agent control widget received open event:', e.detail)
      this.open(e.detail.agentId)
    }
    document.addEventListener('open-agent-control', this.handleOpenEvent)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this.handleEscape)
    document.removeEventListener('open-agent-control', this.handleOpenEvent)
    this.stopRefreshInterval()
  }

  async initializeService() {
    this.agentsService = document.querySelector('agents-service')
    if (!this.agentsService) {
      this.agentsService = document.createElement('agents-service')
      document.body.appendChild(this.agentsService)
    }
  }

  async open(agentId) {
    console.log('Opening modal for agent:', agentId)
    this.agentId = agentId
    this.agent = this.agentsService?.getAgentById(agentId)
    console.log('Found agent:', this.agent)
    this.isOpen = true
    this.setAttribute('is-open', '')  // Ajouter l'attribut HTML pour le CSS
    this.currentTab = 'system'
    
    if (this.agent) {
      await this.loadTabData()
      this.startRefreshInterval()
    } else {
      console.warn('Agent not found in service, modal may not display properly')
    }
  }

  close() {
    this.isOpen = false
    this.removeAttribute('is-open')  // Supprimer l'attribut HTML
    this.agentId = null
    this.agent = null
    this.stopRefreshInterval()
  }

  switchTab(tab) {
    this.currentTab = tab
    this.loadTabData()
  }

  startRefreshInterval() {
    this.stopRefreshInterval()
    this.refreshInterval = setInterval(() => {
      // Only refresh if modal is open and visible, and not currently loading
      if (this.isOpen && !this.loading) {
        if (this.currentTab === 'processes') {
          this.loadProcesses()
        } else if (this.currentTab === 'metrics') {
          this.loadMetrics()
        }
      }
    }, 15000) // Refresh toutes les 15s (moins fréquent)
  }

  stopRefreshInterval() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval)
      this.refreshInterval = null
    }
  }

  async loadTabData() {
    if (!this.agent) return
    
    switch (this.currentTab) {
      case 'processes':
        await this.loadProcesses()
        break
      case 'metrics':
        await this.loadMetrics()
        break
    }
  }

  async loadProcesses() {
    try {
      // Silent loading - keep existing data visible during refresh
      const isInitialLoad = !this.processes || this.processes.length === 0
      if (isInitialLoad) {
        this.loading = true
      } else {
        this.refreshing = true
      }
      
      console.log('Loading processes for agent:', this.agentId)
      const newProcesses = await this.agentsService.getAgentProcesses(this.agentId)
      console.log('Loaded processes:', newProcesses)
      
      // Only update if we got valid data
      if (newProcesses && (Array.isArray(newProcesses) || newProcesses.total_count !== undefined)) {
        this.processes = newProcesses
      }
    } catch (error) {
      console.error('Failed to load processes:', error)
      // Don't clear existing data on refresh errors - keep what we have
    } finally {
      this.loading = false
      this.refreshing = false
    }
  }

  async loadMetrics() {
    try {
      // Silent loading - keep existing data visible during refresh  
      const isInitialLoad = !this.metrics
      if (isInitialLoad) {
        this.loading = true
      } else {
        this.refreshing = true
      }
      
      console.log('Loading metrics for agent:', this.agentId)
      const newMetrics = await this.agentsService.getAgentMetrics(this.agentId)
      console.log('Loaded metrics:', newMetrics)
      
      // Only update if we got valid data
      if (newMetrics && (newMetrics.cpu || newMetrics.memory)) {
        this.metrics = newMetrics
      }
    } catch (error) {
      console.error('Failed to load metrics:', error)
      // Don't clear existing data on refresh errors - keep what we have
    } finally {
      this.loading = false
      this.refreshing = false
    }
  }

  async executePowerAction(action) {
    if (!this.agentsService.isAgentOnline(this.agentId)) {
      alert('⚠️ Agent is offline - cannot execute command')
      return
    }

    const confirmMsg = `Are you sure you want to ${action} ${this.agent.hostname}?`
    if (!confirm(confirmMsg)) return

    try {
      switch (action) {
        case 'shutdown':
          await this.agentsService.shutdownAgent(this.agentId)
          break
        case 'reboot':
          await this.agentsService.rebootAgent(this.agentId)
          break
        case 'hibernate':
          await this.agentsService.hibernateAgent(this.agentId)
          break
      }
      
      // Fermer modal après action power
      this.close()
      
    } catch (error) {
      console.error(`Failed to ${action}:`, error)
      alert(`❌ Failed to ${action}: ${error.message}`)
    }
  }

  async killProcess(pid) {
    const confirmMsg = `Kill process ${pid}?`
    if (!confirm(confirmMsg)) return

    try {
      await this.agentsService.killAgentProcess(this.agentId, pid)
      await this.loadProcesses() // Refresh
    } catch (error) {
      console.error('Failed to kill process:', error)
      alert(`❌ Failed to kill process: ${error.message}`)
    }
  }

  async executeCommand() {
    if (!this.commandInput.trim()) return

    const command = this.commandInput.trim()
    this.commandOutput += `$ ${command}\n`
    
    try {
      const result = await this.agentsService.executeCommand(this.agentId, command)
      this.commandOutput += result.output || result.data || 'Command executed successfully\n'
    } catch (error) {
      this.commandOutput += `Error: ${error.message}\n`
    }
    
    this.commandOutput += '\n'
    this.commandInput = ''
    this.requestUpdate()
  }

  handleCommandKeyPress(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      this.executeCommand()
    }
  }

  renderSystemTab() {
    if (!this.agent) return html``

    const isOnline = this.agent.status === 'online'

    return html`
      <div class="section">
        <div class="section-title">⚡ Power Management</div>
        <div class="power-controls">
          <button 
            class="power-btn danger"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('shutdown')}">
            🔴 Shutdown
          </button>
          <button 
            class="power-btn warning"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('reboot')}">
            🔄 Reboot
          </button>
          <button 
            class="power-btn warning"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('hibernate')}">
            💤 Hibernate
          </button>
        </div>
      </div>

      <div class="section">
        <div class="section-title">📊 System Information</div>
        <div class="info-grid">
          <div class="info-card">
            <div class="info-label">Hostname</div>
            <div class="info-value">${this.agent.hostname}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Operating System</div>
            <div class="info-value">${this.agent.os} ${this.agent.architecture}</div>
          </div>
          <div class="info-card">
            <div class="info-label">IP Address</div>
            <div class="info-value">${this.agent.primary_ip}</div>
          </div>
          <div class="info-card">
            <div class="info-label">MAC Address</div>
            <div class="info-value">${this.agent.primary_mac}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Agent ID</div>
            <div class="info-value">${this.agent.agent_id}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Last Seen</div>
            <div class="info-value">${this.agentsService?.formatLastSeen(this.agent) || 'Unknown'}</div>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="section-title">🛠️ Capabilities</div>
        <div class="info-grid">
          ${this.agent.capabilities?.map(cap => html`
            <div class="info-card">
              <div class="info-label">Capability</div>
              <div class="info-value">${cap.replace(/_/g, ' ')}</div>
            </div>
          `) || html`<div class="info-card"><div class="info-value">No capabilities data</div></div>`}
        </div>
      </div>
    `
  }

  renderProcessesTab() {
    if (this.loading) {
      return html`<div class="loading-state">🔄 Loading processes...</div>`
    }

    if (!this.processes) {
      return html`
        <div class="error-state">
          📋 No process data available<br>
          <small>Process monitoring may not be supported on this agent</small>
        </div>
      `
    }

    // L'API retourne un objet avec top_cpu, top_memory, running_count, total_count
    const allProcesses = [
      ...(this.processes.top_cpu || []),
      ...(this.processes.top_memory || [])
    ]
    
    // Dédupliquer par PID
    const uniqueProcesses = allProcesses.reduce((acc, proc) => {
      if (!acc.find(p => p.pid === proc.pid)) {
        acc.push(proc)
      }
      return acc
    }, [])

    return html`
      <div class="section">
        <div class="section-title">
          📋 Running Processes 
          <span style="font-size: 12px; color: #888; font-weight: normal;">
            (top 15 by CPU/memory • ${this.processes.running_count || 0} running, ${this.processes.total_count || 0} total)
          </span>
          ${this.refreshing && this.currentTab === 'processes' ? html`<span style="margin-left: 8px; color: #3b82f6;">🔄</span>` : ''}
        </div>
        <div class="processes-table">
          <div class="process-header">
            <span>PID</span>
            <span>Name</span>
            <span class="cpu-col">CPU %</span>
            <span class="memory-col">Memory</span>
            <span>Actions</span>
          </div>
          ${uniqueProcesses.length > 0 ? uniqueProcesses.map(proc => html`
            <div class="process-row">
              <span>${proc.pid}</span>
              <span class="process-name">${proc.name}</span>
              <span class="cpu-col">${(proc.cpu_percent || 0).toFixed(1)}%</span>
              <span class="memory-col">${(proc.memory_mb || 0).toFixed(1)}MB</span>
              <span>
                <button 
                  class="kill-btn"
                  @click="${() => this.killProcess(proc.pid)}">
                  Kill
                </button>
              </span>
            </div>
          `) : html`
            <div class="process-row">
              <span colspan="5" style="text-align: center; color: #888;">No top processes to display</span>
            </div>
          `}
        </div>
      </div>
    `
  }

  renderMetricsTab() {
    if (this.loading) {
      return html`<div class="loading-state">🔄 Loading metrics...</div>`
    }

    if (!this.metrics) {
      return html`
        <div class="error-state">
          📊 No metrics data available<br>
          <small>Real-time metrics may not be supported on this agent</small>
        </div>
      `
    }

    const cpuPercent = this.metrics.cpu?.percent || 0
    const memoryPercent = this.metrics.memory?.percent_used || 0
    const diskPercent = this.metrics.disk?.[0]?.percent_used || 0
    const uptimeHours = Math.round((this.metrics.uptime_seconds || 0) / 3600)
    const memoryUsedGB = ((this.metrics.memory?.used_mb || 0) / 1024).toFixed(1)
    const memoryTotalGB = ((this.metrics.memory?.total_mb || 0) / 1024).toFixed(1)

    return html`
      <div class="section">
        <div class="section-title">
          📊 System Metrics
          ${this.refreshing && this.currentTab === 'metrics' ? html`<span style="margin-left: 8px; color: #3b82f6;">🔄</span>` : ''}
        </div>
        <div class="metrics-grid">
          <div class="metric-card">
            <div class="metric-label">CPU Usage</div>
            <div class="metric-value">${cpuPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill cpu" style="width: ${cpuPercent}%"></div>
            </div>
            ${this.metrics.cpu?.core_count ? html`
              <div style="font-size: 12px; color: #888; margin-top: 8px;">
                ${this.metrics.cpu.core_count} cores
              </div>
            ` : ''}
          </div>
          <div class="metric-card">
            <div class="metric-label">Memory Usage</div>
            <div class="metric-value">${memoryPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill memory" style="width: ${memoryPercent}%"></div>
            </div>
            <div style="font-size: 12px; color: #888; margin-top: 8px;">
              ${memoryUsedGB} / ${memoryTotalGB} GB
            </div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Disk Usage</div>
            <div class="metric-value">${diskPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill disk" style="width: ${diskPercent}%"></div>
            </div>
            ${this.metrics.disk?.[0] ? html`
              <div style="font-size: 12px; color: #888; margin-top: 8px;">
                ${this.metrics.disk[0].path}: ${this.metrics.disk[0].used_gb}/${this.metrics.disk[0].total_gb} GB
              </div>
            ` : ''}
          </div>
          <div class="metric-card">
            <div class="metric-label">Uptime</div>
            <div class="metric-value">${uptimeHours}h</div>
            ${this.metrics.cpu?.load_avg ? html`
              <div style="font-size: 12px; color: #888; margin-top: 8px;">
                Load: ${this.metrics.cpu.load_avg.map(l => l.toFixed(2)).join(', ')}
              </div>
            ` : ''}
          </div>
        </div>
      </div>
    `
  }

  renderCommandsTab() {
    const isOnline = this.agent && this.agent.status === 'online'

    return html`
      <div class="section">
        <div class="section-title">💻 Command Execution</div>
        <div class="command-section">
          <div class="command-input">
            <input 
              type="text"
              class="command-field"
              placeholder="Enter command to execute..."
              .value="${this.commandInput}"
              @input="${(e) => this.commandInput = e.target.value}"
              @keypress="${this.handleCommandKeyPress}"
              ?disabled="${!isOnline}">
            <button 
              class="execute-btn"
              ?disabled="${!isOnline || !this.commandInput.trim()}"
              @click="${this.executeCommand}">
              Execute
            </button>
          </div>
          <div class="command-output">${this.commandOutput}</div>
        </div>
      </div>
    `
  }

  renderServicesTab() {
    return html`
      <div class="section">
        <div class="section-title">🔧 Services Management</div>
        <div class="error-state">
          🚧 Services management coming soon!<br>
          <small>This feature will allow you to start/stop/restart system services</small>
        </div>
      </div>
    `
  }

  render() {
    console.log('Agent control render - isOpen:', this.isOpen, 'agent:', this.agent)
    if (!this.isOpen || !this.agent) {
      return html``
    }

    return html`
      <div class="modal" @click="${(e) => e.target === e.currentTarget && this.close()}">
        <div class="modal-header">
          <div class="modal-title">
            <span class="os-icon">${this.agentsService?.getOSIcon(this.agent.os) || '💻'}</span>
            <div class="agent-info">
              <div class="agent-hostname">${this.agent.hostname}</div>
              <div class="agent-meta">${this.agent.os} • ${this.agent.primary_ip}</div>
            </div>
            <div class="status-badge ${this.agent.status}">
              <span class="status-indicator ${this.agent.status}"></span>
              ${this.agent.status}
            </div>
          </div>
          <button class="close-btn" @click="${this.close}">×</button>
        </div>

        <div class="modal-tabs">
          <button 
            class="tab-btn ${this.currentTab === 'system' ? 'active' : ''}"
            @click="${() => this.switchTab('system')}">
            🖥️ System
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'processes' ? 'active' : ''}"
            @click="${() => this.switchTab('processes')}">
            📋 Processes
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'metrics' ? 'active' : ''}"
            @click="${() => this.switchTab('metrics')}">
            📊 Metrics
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'commands' ? 'active' : ''}"
            @click="${() => this.switchTab('commands')}">
            💻 Commands
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'services' ? 'active' : ''}"
            @click="${() => this.switchTab('services')}">
            🔧 Services
          </button>
        </div>

        <div class="modal-content">
          <div class="tab-panel ${this.currentTab === 'system' ? 'active' : ''}">
            ${this.renderSystemTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'processes' ? 'active' : ''}">
            ${this.renderProcessesTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'metrics' ? 'active' : ''}">
            ${this.renderMetricsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'commands' ? 'active' : ''}">
            ${this.renderCommandsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'services' ? 'active' : ''}">
            ${this.renderServicesTab()}
          </div>
        </div>
      </div>
    `
  }
}

customElements.define('agent-control-widget', AgentControlWidget)

export { AgentControlWidget }