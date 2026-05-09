import { LitElement, html, css } from 'lit'
import './organic-loader.js'
import { sharedAnimations, pageTransitionStyles } from '../styles/shared-animations.js'
import { overlayStyles, scrollbarStyles } from '../styles/shared-patterns.js'
import { pageHeaderStyles } from '../styles/shared-page.js'
import { sectionStyles } from '../styles/shared-cards.js'

/**
 * Page de configuration du plugin Telegram.
 *
 * - Lecture status (uptime, nb users autorisés, sessions actives)
 * - Toggles on/off par catégorie de notification (auto-save)
 *
 * Le token et la liste des allowed_user_ids ne sont PAS exposés (sécu).
 * Pour les modifier : éditer scripts/telegram-bridge/config.env puis
 * relancer scripts/install-plugin-telegram.sh.
 */
export class TelegramConfigPage extends LitElement {
  static properties = {
    loading: { type: Boolean },
    error: { type: String },
    status: { type: Object },
    categories: { type: Array },
    saving: { type: String }, // id de la catégorie en cours de sauvegarde
  }

  static styles = [
    sharedAnimations,
    pageTransitionStyles,
    overlayStyles,
    scrollbarStyles,
    pageHeaderStyles,
    sectionStyles,
    css`
      :host {
        z-index: 1000;
        overflow-x: hidden;
        -webkit-overflow-scrolling: touch;
        overscroll-behavior: contain;
      }

      .page-container {
        max-width: 720px;
        margin: var(--space-6) auto;
        padding: var(--space-6);
        padding-bottom: 120px;
        box-sizing: border-box;
        background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
        border: 1px solid var(--border-medium);
        border-radius: var(--radius-lg);
        box-shadow: 0 24px 64px rgba(0, 0, 0, 0.4);
      }

      .page-header {
        padding: var(--space-4) var(--space-5);
        background: var(--surface-glass-strong, rgba(0, 0, 0, 0.3));
        border-radius: var(--radius-lg) var(--radius-lg) 0 0;
        margin: calc(-1 * var(--space-6)) calc(-1 * var(--space-6)) var(--space-6);
      }

      .header-left {
        display: flex;
        align-items: center;
        gap: 1rem;
      }

      .back-btn {
        background: var(--surface-glass-hover, rgba(255, 255, 255, 0.08));
        border: none;
        color: var(--color-dark-text-secondary, #adb5bd);
        width: 36px;
        height: 36px;
        border-radius: 50%;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all var(--duration-base) var(--ease-out);
        font-size: 1.1rem;
      }

      .back-btn:hover {
        background: var(--ctx-bg, rgba(0, 212, 170, 0.05));
        color: var(--context-primary, #00d4aa);
      }

      .page-title-group { display: flex; flex-direction: column; gap: 0.25rem; }
      .page-title { font-weight: 600; }
      .page-title-icon { color: var(--context-primary, #00d4aa); }
      .page-subtitle { font-size: var(--text-sm); color: var(--color-dark-text-tertiary, #6c757d); }

      .loader-container { display: flex; justify-content: center; padding: 3rem 1rem; }

      .error-banner {
        background: rgba(220, 53, 69, 0.15);
        border: 1px solid rgba(220, 53, 69, 0.4);
        color: #ff8a95;
        padding: var(--space-3) var(--space-4);
        border-radius: var(--radius-md);
        margin-bottom: var(--space-4);
        font-size: var(--text-sm);
      }

      .status-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
        gap: var(--space-3);
      }

      .status-cell {
        background: var(--surface-glass-hover, rgba(255, 255, 255, 0.04));
        padding: var(--space-3) var(--space-4);
        border-radius: var(--radius-md);
        border: 1px solid var(--border-light, rgba(255, 255, 255, 0.06));
      }

      .status-cell-label {
        font-size: var(--text-xs);
        color: var(--color-dark-text-tertiary, #6c757d);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .status-cell-value {
        font-size: var(--text-xl);
        font-weight: 600;
        color: var(--color-dark-text-primary, #f8f9fa);
        margin-top: 0.25rem;
      }

      .category-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: var(--space-3) var(--space-4);
        border-bottom: 1px solid var(--border-light, rgba(255, 255, 255, 0.05));
      }

      .category-row:last-child { border-bottom: none; }

      .category-info {
        display: flex;
        align-items: center;
        gap: var(--space-3);
      }

      .category-icon { font-size: 1.5rem; }

      .category-label {
        font-size: var(--text-base);
        color: var(--color-dark-text-primary, #f8f9fa);
      }

      .category-saving {
        font-size: var(--text-xs);
        color: var(--color-dark-text-tertiary, #6c757d);
        margin-left: var(--space-2);
        opacity: 0.7;
      }

      /* Toggle switch */
      .toggle {
        position: relative;
        width: 48px;
        height: 26px;
        background: rgba(120, 120, 130, 0.4);
        border-radius: 13px;
        cursor: pointer;
        transition: background var(--duration-base) var(--ease-out);
        flex-shrink: 0;
        border: none;
        padding: 0;
      }

      .toggle.on {
        background: var(--context-primary, #00d4aa);
      }

      .toggle.disabled {
        opacity: 0.5;
        cursor: wait;
      }

      .toggle::after {
        content: '';
        position: absolute;
        top: 2px;
        left: 2px;
        width: 22px;
        height: 22px;
        background: white;
        border-radius: 50%;
        transition: transform var(--duration-base) var(--ease-out);
      }

      .toggle.on::after { transform: translateX(22px); }

      .info-note {
        font-size: var(--text-sm);
        color: var(--color-dark-text-tertiary, #6c757d);
        padding: var(--space-3) var(--space-4);
        background: rgba(255, 255, 255, 0.03);
        border-left: 3px solid var(--context-primary, #00d4aa);
        margin-top: var(--space-3);
        border-radius: 0 var(--radius-md) var(--radius-md) 0;
        line-height: 1.5;
      }
    `,
  ]

  constructor() {
    super()
    this.loading = true
    this.error = ''
    this.status = null
    this.categories = []
    this.saving = ''
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadConfig()
    this._handleEscape = (e) => {
      if (e.key === 'Escape') this._close()
    }
    document.addEventListener('keydown', this._handleEscape)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._handleEscape) {
      document.removeEventListener('keydown', this._handleEscape)
    }
  }

  async loadConfig() {
    this.loading = true
    this.error = ''
    try {
      const resp = await fetch('/v1/plugin-api/telegram/config', {
        headers: { Accept: 'application/json' },
      })
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`)
      }
      const data = await resp.json()
      this.status = data.status || {}
      this.categories = Array.isArray(data.categories) ? data.categories : []
    } catch (e) {
      this.error = `Impossible de charger la config Telegram : ${e.message}`
      console.error('[telegram-config-page]', e)
    } finally {
      this.loading = false
    }
  }

  async _toggle(category) {
    if (this.saving === category.id) return
    const newValue = !category.enabled
    this.saving = category.id
    // Optimistic update
    this.categories = this.categories.map((c) =>
      c.id === category.id ? { ...c, enabled: newValue } : c
    )

    try {
      const resp = await fetch('/v1/plugin-api/telegram/config', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ categories: { [category.id]: newValue } }),
      })
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`)
      }
      const data = await resp.json()
      this.categories = Array.isArray(data.categories) ? data.categories : this.categories
    } catch (e) {
      // Rollback
      this.categories = this.categories.map((c) =>
        c.id === category.id ? { ...c, enabled: !newValue } : c
      )
      this.error = `Échec sauvegarde : ${e.message}`
      console.error('[telegram-config-page]', e)
    } finally {
      this.saving = ''
    }
  }

  _close() {
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }))
  }

  _formatUptime(seconds) {
    if (!seconds && seconds !== 0) return '—'
    if (seconds < 60) return `${seconds}s`
    if (seconds < 3600) return `${Math.floor(seconds / 60)}min`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}min`
    return `${Math.floor(seconds / 86400)}j ${Math.floor((seconds % 86400) / 3600)}h`
  }

  render() {
    return html`
      <div class="page-container scrollbar">
        <div class="page-header">
          <div class="header-left">
            <button class="back-btn" @click="${this._close}" title="Fermer (Échap)">←</button>
            <div class="page-title-group">
              <h1 class="page-title">
                <span class="page-title-icon">📱</span> Config Telegram
              </h1>
              <span class="page-subtitle">Filtre les notifications par catégorie</span>
            </div>
          </div>
        </div>

        ${this.error ? html`<div class="error-banner">${this.error}</div>` : ''}

        ${this.loading
          ? html`<div class="loader-container"><organic-loader></organic-loader></div>`
          : html`
              <div class="section">
                <div class="section-header">
                  <span class="section-title">Statut</span>
                </div>
                <div class="status-grid">
                  <div class="status-cell">
                    <div class="status-cell-label">Uptime</div>
                    <div class="status-cell-value">
                      ${this._formatUptime(this.status?.uptime_seconds)}
                    </div>
                  </div>
                  <div class="status-cell">
                    <div class="status-cell-label">Utilisateurs autorisés</div>
                    <div class="status-cell-value">${this.status?.allowed_users_count ?? '—'}</div>
                  </div>
                  <div class="status-cell">
                    <div class="status-cell-label">Sessions actives</div>
                    <div class="status-cell-value">${this.status?.active_sessions ?? '—'}</div>
                  </div>
                </div>
              </div>

              <div class="section">
                <div class="section-header">
                  <span class="section-title">Catégories de notifications</span>
                </div>
                ${this.categories.map(
                  (cat) => html`
                    <div class="category-row">
                      <div class="category-info">
                        <span class="category-icon">${cat.icon}</span>
                        <span class="category-label">${cat.label}</span>
                        ${this.saving === cat.id ? html`<span class="category-saving">…</span>` : ''}
                      </div>
                      <button
                        class="toggle ${cat.enabled ? 'on' : ''} ${this.saving === cat.id ? 'disabled' : ''}"
                        @click="${() => this._toggle(cat)}"
                        title="${cat.enabled ? 'Cliquer pour désactiver' : 'Cliquer pour activer'}"
                      ></button>
                    </div>
                  `
                )}
                <div class="info-note">
                  Les notifications de priorité <strong>P0</strong> (urgences) sont toujours
                  envoyées, peu importe les toggles. Pour modifier le token bot ou la liste
                  des utilisateurs autorisés, édite
                  <code>scripts/telegram-bridge/config.env</code> puis relance
                  <code>scripts/install-plugin-telegram.sh</code>.
                </div>
              </div>
            `}
      </div>
    `
  }
}

customElements.define('telegram-config-page', TelegramConfigPage)
