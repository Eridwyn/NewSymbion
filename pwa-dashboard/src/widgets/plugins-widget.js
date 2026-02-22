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
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles } from '../styles/shared-patterns.js'

class PluginsWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, statusBadgeStyles, css`
    :host {
      display: block;
    }

    /* widget-count provided by widgetHeaderStyles */
    
    .plugins-list {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }
    
    .plugin-card {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-base);
      padding: 1rem;
      transition: all var(--duration-base) var(--ease-out);
    }
    
    .plugin-card:hover {
      border-color: rgba(0, 122, 204, 0.3);
      background: var(--surface-glass);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
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
    
    /* status-badge variants (.running, .stopped, .starting, etc.) from shared statusBadgeStyles */
    
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
      background: var(--ctx-bg-subtle);
      border: 1px solid var(--ctx-bg-strong);
      border-radius: var(--radius-sm);
      padding: 0.8rem;
      margin-bottom: 1rem;
      font-size: 0.85em;
      color: var(--context-primary, #00d4aa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .plugin-actions {
      display: flex;
      gap: 0.75rem;
      margin-top: 1rem;
    }

    .action-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      padding: 0;
      border: 1px solid transparent;
      border-radius: 50%;
      font-size: 1em;
      cursor: pointer;
      transition: all 0.25s cubic-bezier(0, 0, 0.2, 1);
    }

    .action-btn.success {
      background: var(--ctx-bg-medium);
      color: var(--context-primary, #00d4aa);
      border-color: var(--ctx-bg-emphasis);
    }

    .action-btn.success:hover:not(:disabled) {
      background: var(--ctx-bg-emphasis);
      border-color: var(--ctx-border-strong);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px var(--ctx-bg-strong);
    }

    .action-btn.danger {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
      border-color: rgba(239, 68, 68, 0.25);
    }

    .action-btn.danger:hover:not(:disabled) {
      background: rgba(239, 68, 68, 0.25);
      border-color: rgba(239, 68, 68, 0.4);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(239, 68, 68, 0.2);
    }

    .action-btn.warning {
      background: rgba(245, 158, 11, 0.15);
      color: #f59e0b;
      border-color: rgba(245, 158, 11, 0.25);
    }

    .action-btn.warning:hover:not(:disabled) {
      background: rgba(245, 158, 11, 0.25);
      border-color: rgba(245, 158, 11, 0.4);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(245, 158, 11, 0.2);
    }

    .action-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
      box-shadow: none;
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
      background: var(--surface-glass-strong);
      padding: 0.2rem 0.4rem;
      border-radius: var(--radius-sm);
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
      border-radius: var(--radius-sm);
    }

    .success {
      text-align: center;
      padding: 1rem;
      color: var(--context-primary, #00d4aa);
      background: var(--ctx-bg-subtle);
      border: 1px solid var(--ctx-bg-intense);
      border-radius: var(--radius-sm);
      margin-bottom: 1rem;
    }
  `]

  static properties = {
    plugins: { type: Array },
    apiService: { type: Object },
    loading: { type: Boolean },
    error: { type: String },
    successMessage: { type: String }
  }
  
  constructor() {
    super()
    this.plugins = []
    this.apiService = null
    this.loading = false
    this.error = null
    this.successMessage = null
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
    this.successMessage = null

    try {
      let result

      // Use dedicated apiService methods instead of direct csrfService access
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
        case 'status':
          result = await this.apiService.getPlugin(pluginName)
          break
        default:
          throw new Error(`Unknown action: ${action}`)
      }

      console.log(`[plugins-widget] ${action} ${pluginName}:`, result)

      // Show success message
      const actionLabel = {
        start: 'Démarrage',
        stop: 'Arrêt',
        restart: 'Redémarrage'
      }[action] || action

      this.successMessage = `${actionLabel} de ${pluginName} en cours... (vérifiez dans 2-3s)`

      // Clear success message after 5 seconds
      setTimeout(() => {
        this.successMessage = null
      }, 5000)

      // Refresh plugin list after action
      if (action !== 'status') {
        setTimeout(() => {
          window.dispatchEvent(new CustomEvent('refresh-plugins'))
        }, 2000)
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
        <span class="widget-count">
          ${runningCount}/${this.plugins.length} actifs
        </span>
      </div>

      <div class="readonly-notice">
        ⚙️ Plugins gérés via systemd - contrôle direct disponible
      </div>

      ${this.successMessage ? html`
        <div class="success">✅ ${this.successMessage}</div>
      ` : ''}

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
              <span class="status-badge ${this.normalizeStatus(plugin.status).toLowerCase()}">
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
                title="Démarrer"
              >▶</button>
              <button
                class="action-btn danger"
                @click=${() => this.handlePluginAction(plugin.name, 'stop')}
                ?disabled=${this.loading || this.normalizeStatus(plugin.status).toLowerCase() !== 'running'}
                title="Arrêter"
              >■</button>
              <button
                class="action-btn warning"
                @click=${() => this.handlePluginAction(plugin.name, 'restart')}
                ?disabled=${this.loading}
                title="Redémarrer"
              >↻</button>
            </div>
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('plugins-widget', PluginsWidget)