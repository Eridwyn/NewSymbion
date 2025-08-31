/**
 * Widget Gestion des Plugins
 * 
 * Interface pour gérer les plugins Symbion:
 * - État et statut des plugins
 * - Actions start/stop/restart
 * - Monitoring en temps réel
 */

import { LitElement, html, css } from 'lit'

class PluginsWidget extends LitElement {
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
    
    .plugins-count {
      font-size: 0.9em;
      opacity: 0.7;
    }
    
    .plugins-list {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }
    
    .plugin-card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      padding: 1rem;
      transition: all 0.3s ease;
    }
    
    .plugin-card:hover {
      border-color: rgba(0, 122, 204, 0.3);
      background: rgba(255, 255, 255, 0.05);
    }
    
    .plugin-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.8rem;
    }
    
    .plugin-name {
      font-weight: 600;
      color: #e0e0e0;
    }
    
    .plugin-version {
      font-size: 0.8em;
      opacity: 0.6;
      margin-left: 0.5rem;
    }
    
    .plugin-status {
      padding: 0.2rem 0.6rem;
      border-radius: 12px;
      font-size: 0.75em;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    
    .status-running {
      background: rgba(0, 212, 170, 0.2);
      color: #00d4aa;
      border: 1px solid rgba(0, 212, 170, 0.3);
    }
    
    .status-stopped {
      background: rgba(255, 107, 107, 0.2);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.3);
    }
    
    .status-starting {
      background: rgba(255, 217, 61, 0.2);
      color: #ffd93d;
      border: 1px solid rgba(255, 217, 61, 0.3);
    }
    
    .plugin-description {
      font-size: 0.9em;
      opacity: 0.7;
      margin-bottom: 1rem;
    }
    
    .plugin-actions {
      display: flex;
      gap: 0.5rem;
    }
    
    .action-btn {
      background: rgba(0, 122, 204, 0.2);
      border: 1px solid rgba(0, 122, 204, 0.3);
      color: #007acc;
      padding: 0.4rem 0.8rem;
      border-radius: 6px;
      font-size: 0.8em;
      cursor: pointer;
      transition: all 0.3s ease;
    }
    
    .action-btn:hover {
      background: rgba(0, 122, 204, 0.3);
      border-color: rgba(0, 122, 204, 0.5);
    }
    
    .action-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
    
    .action-btn.danger {
      background: rgba(255, 107, 107, 0.2);
      border-color: rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
    }
    
    .action-btn.danger:hover {
      background: rgba(255, 107, 107, 0.3);
      border-color: rgba(255, 107, 107, 0.5);
    }
    
    .plugin-contracts {
      margin-top: 0.8rem;
      font-size: 0.8em;
      opacity: 0.6;
    }
    
    .contracts-list {
      display: flex;
      flex-wrap: wrap;
      gap: 0.3rem;
      margin-top: 0.3rem;
    }
    
    .contract-tag {
      background: rgba(255, 255, 255, 0.1);
      padding: 0.2rem 0.4rem;
      border-radius: 4px;
      font-size: 0.75em;
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
  `
  
  static properties = {
    plugins: { type: Array },
    apiService: { type: Object },
    loading: { type: Boolean },
    error: { type: String }
  }
  
  constructor() {
    super()
    this.plugins = []
    this.apiService = null
    this.loading = false
    this.error = null
  }
  
  async handlePluginAction(pluginName, action) {
    if (!this.apiService) {
      console.error('❌ API service not available')
      return
    }
    
    this.loading = true
    this.error = null
    
    try {
      console.log(`🔧 ${action} plugin: ${pluginName}`)
      
      let result
      switch (action) {
        case 'start':
          result = await this.apiService.startPlugin(pluginName)
          break
        case 'stop':
          result = await this.apiService.stopPlugin(pluginName)
          break
        case 'restart':
          result = await this.apiService.restartPlugin(pluginName)
          break
        default:
          throw new Error(`Unknown action: ${action}`)
      }
      
      console.log(`✅ Plugin ${action} result:`, result)
      
      // Recharger la liste des plugins après l'action
      setTimeout(async () => {
        try {
          this.plugins = await this.apiService.getPlugins()
        } catch (error) {
          console.error('❌ Failed to reload plugins:', error)
        }
      }, 1000)
      
    } catch (error) {
      console.error(`❌ Plugin ${action} failed:`, error)
      this.error = `Erreur ${action} ${pluginName}: ${error.message}`
    } finally {
      this.loading = false
    }
  }
  
  getStatusLabel(status) {
    const labels = {
      'running': 'En cours',
      'stopped': 'Arrêté',
      'starting': 'Démarrage',
      'stopping': 'Arrêt',
      'error': 'Erreur'
    }
    return labels[status.toLowerCase()] || status
  }
  
  render() {
    if (!this.plugins || this.plugins.length === 0) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">🔌 Plugins</h3>
        </div>
        <div class="loading">
          ⏳ Aucun plugin chargé
        </div>
      `
    }
    
    const runningCount = this.plugins.filter(p => p.status.toLowerCase() === 'running').length
    
    return html`
      <div class="widget-header">
        <h3 class="widget-title">🔌 Plugins</h3>
        <span class="plugins-count">
          ${runningCount}/${this.plugins.length} actifs
        </span>
      </div>
      
      ${this.error ? html`
        <div class="error">❌ ${this.error}</div>
      ` : ''}
      
      <div class="plugins-list">
        ${this.plugins.map(plugin => html`
          <div class="plugin-card">
            <div class="plugin-header">
              <div>
                <span class="plugin-name">${plugin.name}</span>
                <span class="plugin-version">v${plugin.version || '0.1.0'}</span>
              </div>
              <span class="plugin-status status-${plugin.status.toLowerCase()}">
                ${this.getStatusLabel(plugin.status)}
              </span>
            </div>
            
            ${plugin.description ? html`
              <div class="plugin-description">
                ${plugin.description}
              </div>
            ` : ''}
            
            <div class="plugin-actions">
              ${plugin.status.toLowerCase() === 'running' ? html`
                <button 
                  class="action-btn danger"
                  @click="${() => this.handlePluginAction(plugin.name, 'stop')}"
                  ?disabled="${this.loading}">
                  ⏹️ Arrêter
                </button>
                <button 
                  class="action-btn"
                  @click="${() => this.handlePluginAction(plugin.name, 'restart')}"
                  ?disabled="${this.loading}">
                  🔄 Redémarrer
                </button>
              ` : html`
                <button 
                  class="action-btn"
                  @click="${() => this.handlePluginAction(plugin.name, 'start')}"
                  ?disabled="${this.loading}">
                  ▶️ Démarrer
                </button>
              `}
            </div>
            
            ${plugin.contracts && plugin.contracts.length > 0 ? html`
              <div class="plugin-contracts">
                <div>Contrats:</div>
                <div class="contracts-list">
                  ${plugin.contracts.map(contract => html`
                    <span class="contract-tag">${contract}</span>
                  `)}
                </div>
              </div>
            ` : ''}
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('plugins-widget', PluginsWidget)