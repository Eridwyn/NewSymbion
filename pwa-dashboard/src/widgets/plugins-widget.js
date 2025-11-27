/**
 * Widget Affichage des Plugins
 *
 * Interface de contrôle et visualisation des plugins Symbion:
 * - État et statut des plugins
 * - Routes enregistrées
 * - Informations d'enregistrement
 * - Contrôle systemctl (start/stop/restart)
 * - Plugins autonomes gérés via systemd/processus
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
    
    .plugin-info {
      margin-top: 0.8rem;
      font-size: 0.8em;
      opacity: 0.6;
    }

    .info-row {
      display: flex;
      justify-content: space-between;
      margin-bottom: 0.4rem;
    }

    .info-label {
      font-weight: 500;
      opacity: 0.8;
    }

    .info-value {
      font-family: monospace;
      opacity: 0.9;
    }

    .readonly-notice {
      background: rgba(76, 175, 80, 0.1);
      border: 1px solid rgba(76, 175, 80, 0.2);
      border-radius: 6px;
      padding: 0.8rem;
      margin-bottom: 1rem;
      font-size: 0.85em;
      color: #4caf50;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .plugin-actions {
      display: flex;
      gap: 0.5rem;
      margin-top: 0.8rem;
    }

    .action-btn {
      padding: 0.4rem 0.8rem;
      border: none;
      border-radius: 4px;
      font-size: 0.75em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .action-btn.success {
      background: rgba(76, 175, 80, 0.2);
      color: #4caf50;
      border: 1px solid rgba(76, 175, 80, 0.3);
    }

    .action-btn.danger {
      background: rgba(244, 67, 54, 0.2);
      color: #f44336;
      border: 1px solid rgba(244, 67, 54, 0.3);
    }

    .action-btn.warning {
      background: rgba(255, 152, 0, 0.2);
      color: #ff9800;
      border: 1px solid rgba(255, 152, 0, 0.3);
    }

    .action-btn:hover {
      filter: brightness(1.2);
    }

    .action-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
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
  
  // Normalize status (can be string "Running" or object {"Failed": "..."})
  // Registered plugins (with registered_at timestamp) are assumed to be running
  normalizeStatus(status) {
    if (typeof status === 'string') {
      return status
    } else if (typeof status === 'object' && status.Failed) {
      return 'failed'
    }
    // If no status provided, assume plugin is running (it's registered)
    return 'running'
  }

  getStatusLabel(status) {
    const normalized = this.normalizeStatus(status)
    const labels = {
      'running': 'Actif',
      'stopped': 'Arrêté',
      'starting': 'Démarrage',
      'stopping': 'Arrêt',
      'failed': 'Échoué',
      'error': 'Erreur'
    }
    return labels[normalized.toLowerCase()] || normalized
  }

  formatTimestamp(timestamp) {
    if (!timestamp) return 'N/A'
    try {
      const date = new Date(timestamp)
      return date.toLocaleString('fr-FR', {
        day: '2-digit',
        month: '2-digit',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      })
    } catch {
      return timestamp
    }
  }

  async handlePluginAction(pluginName, action) {
    if (!this.apiService) return

    this.loading = true
    this.error = null

    try {
      const url = `${this.apiService.baseUrl}/v1/plugins/${encodeURIComponent(pluginName)}/${action}`
      const response = await this.apiService.csrfService.fetchWithCsrf(url, {
        method: action === 'status' ? 'GET' : 'POST',
        headers: { 'Content-Type': 'application/json' }
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }

      const result = await response.json()
      console.log(`[plugins-widget] ${action} ${pluginName}:`, result)

      // Refresh plugin list after action
      if (action !== 'status') {
        setTimeout(() => {
          window.dispatchEvent(new CustomEvent('refresh-plugins'))
        }, 1000)
      }
    } catch (err) {
      console.error(`[plugins-widget] Failed to ${action} plugin ${pluginName}:`, err)
      this.error = `Failed to ${action} plugin: ${err.message}`
    } finally {
      this.loading = false
    }
  }
  
  render() {
    if (!Array.isArray(this.plugins) || this.plugins.length === 0) {
      return html`
        <div class="widget-header">
          <h3 class="widget-title">🔌 Plugins</h3>
        </div>
        <div class="loading">
          ⏳ Aucun plugin chargé
        </div>
      `
    }

    const runningCount = this.plugins.filter(p => {
      const status = this.normalizeStatus(p.status)
      return status.toLowerCase() === 'running'
    }).length
    
    return html`
      <div class="widget-header">
        <h3 class="widget-title">🔌 Plugins</h3>
        <span class="plugins-count">
          ${runningCount}/${this.plugins.length} actifs
        </span>
      </div>

      <div class="readonly-notice">
        ⚙️ Plugins gérés via systemd - contrôle direct disponible
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
              <span class="plugin-status status-${this.normalizeStatus(plugin.status).toLowerCase()}">
                ${this.getStatusLabel(plugin.status)}
              </span>
            </div>

            ${plugin.description ? html`
              <div class="plugin-description">
                ${plugin.description}
              </div>
            ` : ''}

            <div class="plugin-info">
              ${plugin.socket_path ? html`
                <div class="info-row">
                  <span class="info-label">Socket:</span>
                  <span class="info-value">${plugin.socket_path}</span>
                </div>
              ` : ''}
              ${plugin.registered_at ? html`
                <div class="info-row">
                  <span class="info-label">Enregistré le:</span>
                  <span class="info-value">${this.formatTimestamp(plugin.registered_at)}</span>
                </div>
              ` : ''}
            </div>

            ${plugin.contracts && plugin.contracts.length > 0 ? html`
              <div class="plugin-contracts">
                <div>Routes enregistrées:</div>
                <div class="contracts-list">
                  ${plugin.contracts.map(contract => html`
                    <span class="contract-tag">${contract}</span>
                  `)}
                </div>
              </div>
            ` : ''}

            <div class="plugin-actions">
              <button
                class="action-btn success"
                @click=${() => this.handlePluginAction(plugin.name, 'start')}
                ?disabled=${this.loading || this.normalizeStatus(plugin.status).toLowerCase() === 'running'}
              >
                ▶ Start
              </button>
              <button
                class="action-btn danger"
                @click=${() => this.handlePluginAction(plugin.name, 'stop')}
                ?disabled=${this.loading || this.normalizeStatus(plugin.status).toLowerCase() !== 'running'}
              >
                ■ Stop
              </button>
              <button
                class="action-btn warning"
                @click=${() => this.handlePluginAction(plugin.name, 'restart')}
                ?disabled=${this.loading}
              >
                ↻ Restart
              </button>
            </div>
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('plugins-widget', PluginsWidget)