/**
 * Library Widget — Bibliothèque de Connaissances
 *
 * Widget compact pour le dashboard, affiche les stats et un accès rapide.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusDotStyles } from '../styles/shared-patterns.js'

export class LibraryWidget extends LitElement {
  static properties = {
    stats: { type: Object },
    recentNodes: { type: Array },
    loading: { type: Boolean },
    pendingLinks: { type: Number }
  }

  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, emptyStateStyles, statusDotStyles, css`
    :host { display: block; }

    .stats-row {
      display: flex;
      gap: 0.75rem;
      margin-bottom: 1rem;
    }

    .stat-card {
      flex: 1;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 0.6rem 0.8rem;
      text-align: center;
    }

    .stat-value {
      font-size: 1.4em;
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
    }

    .stat-label {
      font-size: 0.7em;
      color: var(--color-dark-text-secondary);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .recent-list {
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
    }

    .recent-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.5rem 0.7rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md);
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .recent-item:hover {
      background: var(--surface-glass-hover);
      border-color: var(--context-primary, #00d4aa);
      transform: translateX(4px);
    }

    .recent-title {
      font-size: 0.85em;
      font-weight: 500;
    }

    .recent-section {
      font-size: 0.7em;
      color: var(--color-dark-text-secondary);
    }

    .pending-badge {
      background: rgba(251, 191, 36, 0.2);
      color: #fbbf24;
      border: 1px solid rgba(251, 191, 36, 0.4);
      padding: 0.2rem 0.5rem;
      border-radius: var(--radius-xl);
      font-size: 0.7em;
      font-weight: 600;
    }

    .open-btn {
      width: 100%;
      margin-top: 0.75rem;
      padding: 0.6rem;
      background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, rgba(34, 197, 94, 0.15) 100%);
      border: 1px solid var(--ctx-border-strong);
      border-radius: var(--radius-lg);
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
      font-size: 0.85em;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .open-btn:hover {
      background: var(--ctx-bg-emphasis);
      transform: translateY(-1px);
    }
  `]

  constructor() {
    super()
    this.stats = { nodes: 0, sections: 0, pending_links: 0 }
    this.recentNodes = []
    this.loading = true
    this.pendingLinks = 0
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchData()
  }

  getAuthToken() {
    return sessionStorage.getItem('symbion_auth_token') || ''
  }

  async fetchData() {
    this.loading = true
    const token = this.getAuthToken()
    const base = window.location.origin

    try {
      const [healthRes, nodesRes] = await Promise.all([
        fetch(`${base}/v1/plugin-api/library/health`, { headers: { 'Authorization': `Bearer ${token}` } }),
        fetch(`${base}/v1/plugin-api/library/nodes`, { headers: { 'Authorization': `Bearer ${token}` } })
      ])

      if (healthRes.ok) {
        const health = await healthRes.json()
        this.stats = health.stats || { nodes: 0, sections: 0, pending_links: 0 }
        this.pendingLinks = this.stats.pending_links || 0
      }

      if (nodesRes.ok) {
        const data = await nodesRes.json()
        this.recentNodes = (data.nodes || []).slice(0, 4)
      }
    } catch (err) {
      console.error('[library-widget] Error:', err)
    } finally {
      this.loading = false
    }
  }

  openLibrary() {
    this.dispatchEvent(new CustomEvent('open-library-page', {
      bubbles: true,
      composed: true
    }))
  }

  render() {
    return html`
      <div class="widget-header">
        <h3>Bibliothèque</h3>
        ${this.pendingLinks > 0 ? html`
          <span class="pending-badge">${this.pendingLinks} lien${this.pendingLinks > 1 ? 's' : ''} en attente</span>
        ` : ''}
      </div>

      ${this.loading ? html`
        <div class="empty-state">Chargement...</div>
      ` : html`
        <div class="stats-row">
          <div class="stat-card">
            <div class="stat-value">${this.stats.nodes}</div>
            <div class="stat-label">Fiches</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">${this.stats.sections}</div>
            <div class="stat-label">Sections</div>
          </div>
        </div>

        ${this.recentNodes.length > 0 ? html`
          <div class="recent-list">
            ${this.recentNodes.map(node => html`
              <div class="recent-item" @click=${() => this.openLibrary()}>
                <span class="recent-title">${node.title}</span>
              </div>
            `)}
          </div>
        ` : html`
          <div class="empty-state">Aucune fiche</div>
        `}

        <button class="open-btn" @click=${() => this.openLibrary()}>
          Ouvrir la Bibliothèque
        </button>
      `}
    `
  }
}

customElements.define('library-widget', LibraryWidget)
