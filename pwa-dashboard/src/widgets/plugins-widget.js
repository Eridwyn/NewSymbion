/**
 * Widget Affichage des Plugins
 *
 * Interface de contrôle et visualisation des plugins Symbion:
 * - Mode compact par défaut (nom + status)
 * - Détails dépliables par plugin (socket, routes, actions)
 * - Contrôle systemctl (start/stop/restart)
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles } from '../styles/shared-patterns.js'

class PluginsWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, statusBadgeStyles, emptyStateStyles, css`
    :host {
      display: block;
    }

    .plugins-list {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    /* ── Compact row ── */
    .plugin-row {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 0.55rem 0.75rem;
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-base);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      user-select: none;
    }

    .plugin-row:hover {
      border-color: rgba(0, 122, 204, 0.3);
      background: var(--surface-glass);
    }

    .plugin-row.expanded {
      border-bottom-left-radius: 0;
      border-bottom-right-radius: 0;
      border-bottom-color: transparent;
    }

    .expand-chevron {
      font-size: 0.7em;
      opacity: 0.5;
      transition: transform 0.2s ease;
      flex-shrink: 0;
    }

    .plugin-row.expanded .expand-chevron {
      transform: rotate(90deg);
    }

    .plugin-name {
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
      min-width: 0;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .plugin-version {
      font-size: 0.75em;
      opacity: 0.45;
    }

    /* ── Expanded details ── */
    .plugin-details {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-top: none;
      border-radius: 0 0 var(--radius-base) var(--radius-base);
      padding: 0.75rem;
      margin-bottom: 4px;
      animation: slideDown 0.15s ease-out;
    }

    @keyframes slideDown {
      from { opacity: 0; max-height: 0; padding-top: 0; padding-bottom: 0; }
      to { opacity: 1; max-height: 300px; }
    }

    .plugin-description {
      font-size: 0.85em;
      opacity: 0.7;
      margin-bottom: 0.6rem;
    }

    .plugin-info {
      font-size: 0.8em;
      opacity: 0.6;
    }

    .info-row {
      display: flex;
      justify-content: space-between;
      margin-bottom: 0.3rem;
    }

    .info-label {
      font-weight: 500;
      opacity: 0.8;
    }

    .info-value {
      font-family: monospace;
      opacity: 0.9;
    }

    .plugin-contracts {
      margin-top: 0.5rem;
      font-size: 0.8em;
      opacity: 0.6;
    }

    .contracts-list {
      display: flex;
      flex-wrap: wrap;
      gap: 0.25rem;
      margin-top: 0.25rem;
    }

    .contract-tag {
      background: var(--surface-glass-strong);
      padding: 0.15rem 0.35rem;
      border-radius: var(--radius-sm);
      font-size: 0.75em;
    }

    .plugin-actions {
      display: flex;
      gap: 0.5rem;
      margin-top: 0.6rem;
      padding-top: 0.6rem;
      border-top: 1px solid var(--border-default);
    }

    .action-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 30px;
      height: 30px;
      padding: 0;
      border: 1px solid transparent;
      border-radius: 50%;
      font-size: 0.9em;
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
    }

    .action-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
    }

    .error {
      text-align: center;
      padding: 0.6rem;
      color: var(--color-danger-text-muted, #ff6b6b);
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: var(--radius-sm);
      font-size: 0.85em;
      margin-bottom: 0.5rem;
    }

    .feedback {
      text-align: center;
      padding: 0.6rem;
      color: var(--context-primary, #00d4aa);
      background: var(--ctx-bg-subtle);
      border: 1px solid var(--ctx-bg-intense);
      border-radius: var(--radius-sm);
      font-size: 0.85em;
      margin-bottom: 0.5rem;
    }

    /* Responsive */
    @media (max-width: 640px) {
      .plugin-row {
        padding: 0.5rem 0.6rem;
      }

      .plugin-details {
        padding: 0.6rem;
      }

      .info-row {
        flex-direction: column;
        gap: 0.1rem;
      }

      .info-value {
        font-size: 0.75em;
        word-break: break-all;
      }
    }
  `]

  static properties = {
    plugins: { type: Array },
    apiService: { type: Object },
    loading: { type: Boolean },
    error: { type: String },
    successMessage: { type: String },
    expandedPlugin: { type: String }
  }

  constructor() {
    super()
    this.plugins = []
    this.apiService = null
    this.loading = false
    this.error = null
    this.successMessage = null
    this.expandedPlugin = null
  }

  normalizeStatus(status) {
    if (typeof status === 'string') return status
    if (typeof status === 'object' && status.Failed) return 'failed'
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
      return new Date(timestamp).toLocaleString('fr-FR', {
        day: '2-digit', month: '2-digit', year: 'numeric',
        hour: '2-digit', minute: '2-digit'
      })
    } catch {
      return timestamp
    }
  }

  togglePlugin(name) {
    this.expandedPlugin = this.expandedPlugin === name ? null : name
  }

  async handlePluginAction(pluginName, action, e) {
    e.stopPropagation()
    if (!this.apiService) return

    this.loading = true
    this.error = null
    this.successMessage = null

    try {
      switch (action) {
        case 'start': await this.apiService.startPlugin(pluginName); break
        case 'stop': await this.apiService.stopPlugin(pluginName); break
        case 'restart': await this.apiService.restartPlugin(pluginName); break
        case 'status': await this.apiService.getPlugin(pluginName); break
        default: throw new Error(`Unknown action: ${action}`)
      }

      const label = { start: 'Démarrage', stop: 'Arrêt', restart: 'Redémarrage' }[action] || action
      this.successMessage = `${label} de ${pluginName} en cours...`
      setTimeout(() => { this.successMessage = null }, 5000)

      if (action !== 'status') {
        setTimeout(() => window.dispatchEvent(new CustomEvent('refresh-plugins')), 2000)
      }
    } catch (err) {
      this.error = `Échec ${action} ${pluginName}: ${err.message}`
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
        <div class="empty-state">
          <div class="empty-state-icon">⏳</div>
          <div class="empty-state-text">Aucun plugin chargé</div>
        </div>
      `
    }

    const runningCount = this.plugins.filter(p =>
      this.normalizeStatus(p.status).toLowerCase() === 'running'
    ).length

    return html`
      <div class="widget-header">
        <h3 class="widget-title">🔌 Plugins</h3>
        <span class="widget-count">${runningCount}/${this.plugins.length} actifs</span>
      </div>

      ${this.successMessage ? html`<div class="feedback">${this.successMessage}</div>` : ''}
      ${this.error ? html`<div class="error">${this.error}</div>` : ''}

      <div class="plugins-list">
        ${this.plugins.map(plugin => {
          const expanded = this.expandedPlugin === plugin.name
          const status = this.normalizeStatus(plugin.status).toLowerCase()
          const routes = plugin.routes?.filter(r => r && r !== '') || []

          return html`
            <div class="plugin-row ${expanded ? 'expanded' : ''}"
                 @click=${() => this.togglePlugin(plugin.name)}>
              <span class="expand-chevron">▶</span>
              <span class="plugin-name">
                ${plugin.name}
                <span class="plugin-version">v${plugin.version || '?'}</span>
              </span>
              <span class="status-badge ${status}">
                ${this.getStatusLabel(plugin.status)}
              </span>
            </div>

            ${expanded ? html`
              <div class="plugin-details">
                ${plugin.description ? html`
                  <div class="plugin-description">${plugin.description}</div>
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
                      <span class="info-label">Enregistré:</span>
                      <span class="info-value">${this.formatTimestamp(plugin.registered_at)}</span>
                    </div>
                  ` : ''}
                </div>

                ${routes.length > 0 ? html`
                  <div class="plugin-contracts">
                    <div>Routes:</div>
                    <div class="contracts-list">
                      ${routes.map(r => html`<span class="contract-tag">${r}</span>`)}
                    </div>
                  </div>
                ` : ''}

                <div class="plugin-actions">
                  <button class="action-btn success"
                    @click=${(e) => this.handlePluginAction(plugin.name, 'start', e)}
                    ?disabled=${this.loading || status === 'running'}
                    title="Démarrer">▶</button>
                  <button class="action-btn danger"
                    @click=${(e) => this.handlePluginAction(plugin.name, 'stop', e)}
                    ?disabled=${this.loading || status !== 'running'}
                    title="Arrêter">■</button>
                  <button class="action-btn warning"
                    @click=${(e) => this.handlePluginAction(plugin.name, 'restart', e)}
                    ?disabled=${this.loading}
                    title="Redémarrer">↻</button>
                </div>
              </div>
            ` : ''}
          `
        })}
      </div>
    `
  }
}

customElements.define('plugins-widget', PluginsWidget)
