/**
 * Library Widget — Validation des liens entre fiches
 *
 * Affiche les pending links à valider directement depuis le dashboard.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { statusDotStyles } from '../styles/shared-patterns.js'

const API_BASE = () => window.location.origin
// SW injects Authorization header automatically

async function api(path, options = {}) {
  const headers = { ...(options.headers || {}) }
  if (options.body) headers['Content-Type'] = 'application/json'
  const res = await fetch(`${API_BASE()}/v1/plugin-api/library${path}`, { ...options, headers })
  if (!res.ok && res.status !== 204) throw new Error(`HTTP ${res.status}`)
  if (res.status === 204) return null
  const text = await res.text()
  return text ? JSON.parse(text) : null
}

export class LibraryWidget extends LitElement {
  static properties = {
    stats: { type: Object },
    pendingLinks: { type: Array },
    nodeCache: { type: Object },
    loading: { type: Boolean },
  }

  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, emptyStateStyles, statusDotStyles, css`
    :host { display: block; }

    .stats-row {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 0.8rem;
    }

    .stat-card {
      flex: 1;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 0.5rem 0.6rem;
      text-align: center;
    }

    .stat-value {
      font-size: 1.3em;
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
    }

    .stat-label {
      font-size: 0.65em;
      color: var(--color-dark-text-secondary);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .section-title {
      font-size: 0.7em;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--color-dark-text-secondary);
      margin-bottom: 0.5rem;
    }

    .link-list {
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
      max-height: 280px;
      overflow-y: auto;
    }

    .link-card {
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-left: 3px solid #fbbf24;
      border-radius: var(--radius-md);
      padding: 0.5rem 0.6rem;
      animation: slideUp 0.2s ease-out;
    }

    .link-nodes {
      display: flex;
      align-items: center;
      gap: 0.3rem;
      font-size: 0.82em;
      margin-bottom: 0.3rem;
      flex-wrap: wrap;
    }

    .link-node-name {
      font-weight: 500;
      color: var(--color-dark-text-primary);
      max-width: 40%;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .link-arrow {
      color: var(--context-primary, #00d4aa);
      font-size: 0.9em;
      flex-shrink: 0;
    }

    .link-occurrence {
      font-size: 0.68em;
      color: var(--color-dark-text-secondary);
      font-style: italic;
      margin-bottom: 0.3rem;
    }

    .link-actions {
      display: flex;
      gap: 0.3rem;
    }

    .btn-confirm, .btn-reject {
      flex: 1;
      padding: 0.5rem 0.75rem;
      min-height: 44px;
      border-radius: var(--radius-md);
      font-size: 0.8em;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      border: 1px solid;
    }

    .btn-confirm {
      background: rgba(34, 197, 94, 0.1);
      border-color: rgba(34, 197, 94, 0.3);
      color: #22c55e;
    }
    .btn-confirm:hover { background: rgba(34, 197, 94, 0.2); }

    .btn-reject {
      background: rgba(239, 68, 68, 0.1);
      border-color: rgba(239, 68, 68, 0.3);
      color: #ef4444;
    }
    .btn-reject:hover { background: rgba(239, 68, 68, 0.2); }

    .all-good {
      text-align: center;
      padding: 1rem 0.5rem;
      color: var(--color-dark-text-secondary);
      font-size: 0.82em;
    }

    .all-good .check {
      font-size: 1.5em;
      margin-bottom: 0.3rem;
      display: block;
    }

    .open-btn {
      width: 100%;
      margin-top: 0.6rem;
      padding: 0.5rem;
      background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, rgba(34, 197, 94, 0.15) 100%);
      border: 1px solid var(--ctx-border-strong);
      border-radius: var(--radius-lg);
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
      font-size: 0.8em;
      cursor: pointer;
      transition: all 0.2s ease;
    }
    .open-btn:hover { background: var(--ctx-bg-emphasis); transform: translateY(-1px); }

    @media (max-width: 480px) {
      .stats-row {
        display: grid;
        grid-template-columns: 1fr 1fr;
      }
    }
  `]

  constructor() {
    super()
    this.stats = { nodes: 0, sections: 0, pending_links: 0 }
    this.pendingLinks = []
    this.nodeCache = {}
    this.loading = true
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchData()
    this._refreshInterval = setInterval(() => this.fetchData(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._refreshInterval) clearInterval(this._refreshInterval)
  }

  async fetchData() {
    this.loading = true
    try {
      const [healthRes, linksRes, edgesRes] = await Promise.all([
        api('/health'),
        api('/pending-links'),
        api('/edges'),
      ])
      const stats = healthRes?.stats || { nodes: 0, sections: 0, pending_links: 0 }
      stats.edges = (edgesRes?.edges || []).length
      this.stats = stats
      this.pendingLinks = (linksRes?.pending_links || []).slice(0, 8)

      // Resolve node names for pending links
      const nodeIds = new Set()
      for (const link of this.pendingLinks) {
        nodeIds.add(link.node_from)
        nodeIds.add(link.node_to)
      }
      for (const id of nodeIds) {
        if (!this.nodeCache[id]) {
          try {
            const node = await api(`/nodes/${id}`)
            if (node) this.nodeCache = { ...this.nodeCache, [id]: node.title }
          } catch { this.nodeCache = { ...this.nodeCache, [id]: id.slice(0, 8) } }
        }
      }
    } catch (err) {
      console.error('[library-widget] Error:', err)
    } finally {
      this.loading = false
    }
  }

  async confirmLink(link) {
    try {
      await api(`/pending-links/${link.id}/confirm`, {
        method: 'POST',
        body: JSON.stringify({ relation: link.occurrence || 'lié' })
      })
      this.pendingLinks = this.pendingLinks.filter(l => l.id !== link.id)
      this.stats = { ...this.stats, pending_links: Math.max(0, (this.stats.pending_links || 0) - 1) }
    } catch (err) {
      console.error('[library-widget] Confirm error:', err)
    }
  }

  async rejectLink(link) {
    try {
      await api(`/pending-links/${link.id}/dismiss`, { method: 'POST' })
      this.pendingLinks = this.pendingLinks.filter(l => l.id !== link.id)
      this.stats = { ...this.stats, pending_links: Math.max(0, (this.stats.pending_links || 0) - 1) }
    } catch (err) {
      console.error('[library-widget] Reject error:', err)
    }
  }

  openLibrary() {
    this.dispatchEvent(new CustomEvent('open-library-page', { bubbles: true, composed: true }))
  }

  render() {
    return html`
      <div class="widget-header">
        <h3>Bibliothèque</h3>
        ${this.stats.pending_links > 0 ? html`
          <span style="background:rgba(251,191,36,0.2);color:#fbbf24;border:1px solid rgba(251,191,36,0.4);padding:0.15rem 0.45rem;border-radius:12px;font-size:0.68em;font-weight:600;">${this.stats.pending_links}</span>
        ` : ''}
      </div>

      ${this.loading ? html`<div class="empty-state">Chargement...</div>` : html`
        <div class="stats-row">
          <div class="stat-card">
            <div class="stat-value">${this.stats.nodes}</div>
            <div class="stat-label">Fiches</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">${this.stats.sections}</div>
            <div class="stat-label">Sections</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">${this.stats.edges || 0}</div>
            <div class="stat-label">Liens</div>
          </div>
        </div>

        ${this.pendingLinks.length > 0 ? html`
          <div class="section-title">Liens a valider</div>
          <div class="link-list">
            ${this.pendingLinks.map(link => html`
              <div class="link-card">
                <div class="link-nodes">
                  <span class="link-node-name">${this.nodeCache[link.node_from] || '...'}</span>
                  <span class="link-arrow">→</span>
                  <span class="link-node-name">${this.nodeCache[link.node_to] || '...'}</span>
                </div>
                ${link.occurrence ? html`<div class="link-occurrence">"${link.occurrence}"</div>` : ''}
                <div class="link-actions">
                  <button class="btn-confirm" @click=${() => this.confirmLink(link)}>Confirmer</button>
                  <button class="btn-reject" @click=${() => this.rejectLink(link)}>Rejeter</button>
                </div>
              </div>
            `)}
          </div>
        ` : html`
          <div class="all-good">
            <span class="check">&#10003;</span>
            Aucun lien en attente
          </div>
        `}

        <button class="open-btn" @click=${() => this.openLibrary()}>
          Ouvrir la Bibliothèque
        </button>
      `}
    `
  }
}

customElements.define('library-widget', LibraryWidget)
