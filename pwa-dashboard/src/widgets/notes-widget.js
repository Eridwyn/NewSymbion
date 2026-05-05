/**
 * Widget Notes Compact - Dashboard Symbion
 *
 * Affichage simplifié :
 * - 3 notes max (les plus prioritaires)
 * - Preview 1 ligne (60 caractères max)
 * - Filtre contexte actif par défaut (fallback si aucune note)
 * - Bouton "➕" création rapide
 * - Bouton "Voir toutes" vers page complète
 * - Mise à jour dynamique selon changement contexte
 *
 * Utilise utils/notes-scoring.js et utils/notes-filters.js
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, emptyStateStyles } from '../styles/shared-widget.js'
import { focusVisibleStyles } from '../styles/shared-patterns.js'
import { btnStyles, btnSizeStyles } from '../styles/shared-forms.js'
import { getTopPriorityNotes, isHighPriority } from '../utils/notes-scoring.js'
import { applyAllFilters } from '../utils/notes-filters.js'
import notesStreamService from '../services/notes-stream-service.js'
import '../components/organic-loader.js'

class NotesWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, btnStyles, btnSizeStyles, emptyStateStyles, focusVisibleStyles, css`
    :host {
      display: block;
    }

    /* Local overrides (differs from shared: margin-bottom 1rem, font-size 1.1em) */
    .widget-header {
      margin-bottom: 1rem;
    }

    .widget-title {
      font-size: 1.1em;
    }

    .header-actions {
      display: flex;
      gap: 0.5rem;
    }

    /* btn + btn-small base from shared; context-primary override */
    .header-actions .btn {
      background: var(--ctx-bg-medium);
      border: 1px solid var(--ctx-bg-emphasis);
      color: var(--context-primary, #00d4aa);
    }

    .header-actions .btn:hover {
      background: var(--ctx-bg-strong);
      border-color: var(--ctx-border-strong);
    }

    .notes-list {
      display: flex;
      flex-direction: column;
      gap: 0.6rem;
    }

    .note-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base);
      padding: 0.8rem;
      transition: all var(--duration-base) var(--ease-out);
      position: relative;
      cursor: pointer;
    }

    .note-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 3px;
      height: 100%;
      background: linear-gradient(180deg, rgba(0, 122, 204, 0.5) 0%, var(--ctx-border-medium) 100%);
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .note-card:hover {
      border-color: var(--ctx-border-strong);
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass-hover) 100%);
      transform: translateY(-1px);
      box-shadow: 0 8px 32px var(--ctx-border-medium, rgba(0,212,170,0.2)),
                  0 0 40px var(--ctx-bg-subtle, rgba(0,212,170,0.05));
    }

    .note-card:hover::before {
      opacity: 1;
    }

    .note-card.urgent {
      border-color: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 50%, transparent);
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 12%, transparent) 0%, color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 4%, transparent) 100%);
    }

    .note-card.urgent::before {
      background: linear-gradient(180deg, #ff6b6b 0%, #ef4444 100%);
      opacity: 1;
      width: 3px;
    }

    .note-card.priority {
      border-color: color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 30%, transparent);
    }

    .note-header {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      margin-bottom: 0.4rem;
    }

    .urgent-indicator {
      color: var(--color-danger-text-muted, #ff6b6b);
      font-size: 0.9em;
    }

    .priority-badge {
      background: color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 20%, transparent);
      color: var(--color-warning-text-muted, #fbbf24);
      padding: 0.1rem 0.4rem;
      border-radius: var(--radius-sm);
      font-size: 0.65em;
      font-weight: 600;
    }

    .context-tag {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.2) 0%, var(--ctx-border) 100%);
      color: var(--context-primary, #00d4aa);
      padding: 0.1rem 0.4rem;
      border-radius: var(--radius-sm);
      font-size: 0.65em;
      font-weight: 500;
      text-transform: uppercase;
    }

    .note-preview {
      color: var(--color-dark-text-secondary, #cbd5e1);
      font-size: 0.85em;
      line-height: 1.4;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .note-meta {
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 0.7em;
      opacity: 0.5;
      margin-top: 0.3rem;
    }

    .note-tags {
      color: var(--context-primary, #00d4aa);
    }

    /* empty-state provided by emptyStateStyles */

    /* Animation d'apparition progressive des notes */
    @keyframes note-fade-in {
      from {
        opacity: 0;
        transform: translateY(10px) scale(0.95);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    .note-card {
      animation: note-fade-in 0.4s ease-out forwards;
    }

    .note-card:nth-child(1) { animation-delay: 0.1s; opacity: 0; }
    .note-card:nth-child(2) { animation-delay: 0.2s; opacity: 0; }
    .note-card:nth-child(3) { animation-delay: 0.3s; opacity: 0; }

    @media (max-width: 768px) {
      .note-card {
        padding: 0.6rem;
      }

      .note-preview {
        font-size: 0.8em;
      }

      .header-actions .btn {
        min-height: 44px;
      }
    }

    @media (max-width: 480px) {
      .note-card {
        padding: 0.5rem;
      }

      .note-preview {
        font-size: 0.75em;
      }

      .note-header {
        font-size: 0.9em;
      }

      .note-meta {
        font-size: 0.65em;
      }

      .header-actions .btn {
        min-height: 44px;
        font-size: 0.75em;
      }
    }

    /* Utility classes (ex-inline) */
    .nw-subtitle { font-size: 0.6em; opacity: 0.7; font-weight: normal; }
    .nw-error-state { color: var(--color-danger-text-muted, #ff6b6b); }
    .nw-no-context { background: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent); color: var(--color-danger-text-muted, #ff6b6b); }
  `]

  static properties = {
    notes: { type: Array },
    apiService: { type: Object },
    contextService: { type: Object },
    currentContext: { type: String },
    loading: { type: Boolean },
    error: { type: String }
  }

  constructor() {
    super()
    this.notes = []
    this.apiService = null
    this.contextService = null
    this.currentContext = 'veille'
    this.loading = false
    this.error = null
  }

  connectedCallback() {
    super.connectedCallback()

    // Bind handler for cleanup
    this.handleContextChange = this.handleContextChange.bind(this)

    // Get services avec retry si non disponibles
    this.initServices()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    // Cleanup event listener
    window.removeEventListener('context-change', this.handleContextChange)
  }

  handleContextChange(event) {
    console.log('[notes-widget] Context changed:', event.detail.context)
    const oldContext = this.currentContext
    const newMode = event.detail.context?.mode_slug || event.detail.context?.mode || event.detail.context
    this.currentContext = newMode

    // Recharger l'affichage si le contexte a changé
    if (oldContext !== this.currentContext) {
      console.log(`[notes-widget] Context switched: ${oldContext} → ${this.currentContext}`)
      this.requestUpdate()
    }
  }

  async initServices() {
    // Retry avec timeout si services pas encore montés
    let retries = 0
    const maxRetries = 10
    const retryInterval = 100

    const checkServices = async () => {
      this.apiService = document.querySelector('api-service')
      this.contextService = document.querySelector('context-service')

      if (this.apiService && this.contextService) {
        console.log('[notes-widget] ✅ Services trouvés')

        // Écouter les changements de contexte sur window (comme context-widget)
        window.addEventListener('context-change', this.handleContextChange)

        // HYBRID APPROACH: Attendre le contexte max 2s avec waitForContextReady()
        console.log('[notes-widget] Waiting for context ready (2s timeout)...')
        const contextState = await this.contextService.waitForContextReady(2000)

        if (contextState && (contextState.mode_slug || contextState.mode)) {
          this.currentContext = contextState.mode_slug || (typeof contextState.mode === 'string' ? contextState.mode : null) || 'veille'
          console.log('[notes-widget] ✅ Context ready:', this.currentContext)
        } else {
          console.warn('[notes-widget] ⏱️ Context timeout, defaulting to veille')
          this.currentContext = 'veille' // Fallback
        }

        // Charger les notes (affichera toutes les notes si contexte pas prêt)
        this.loadNotes()
        return true
      }

      retries++
      if (retries < maxRetries) {
        setTimeout(checkServices, retryInterval)
      } else {
        console.warn('[notes-widget] ⚠️ Services non trouvés après 10 tentatives')
        // Essayer quand même de charger
        this.loadNotes()
      }
      return false
    }

    checkServices()
  }

  async _loadNotesRest() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) throw new Error('Service API non disponible')
      const data = await apiService.getNotes()
      this.notes = apiService.validateArrayResponse(data, 'notes', [])
      this.error = null
      console.log(`[notes-widget] ✅ Loaded ${this.notes.length} notes via REST`)
    } catch (err) {
      console.error('[notes-widget] ❌ REST failed:', err)
      this.error = `Erreur chargement notes: ${err.message}`
    }
    this.loading = false
    this.requestUpdate()
  }

  async loadNotes() {
    this.loading = true
    this.error = null
    this.notes = []

    let wsTimeout = null
    let completed = false

    const cleanup = () => {
      if (wsTimeout) clearTimeout(wsTimeout)
      notesStreamService.removeEventListener('note-received', onNote)
      notesStreamService.removeEventListener('notes-complete', onComplete)
      notesStreamService.removeEventListener('notes-error', onError)
    }

    const fallbackToRest = (reason) => {
      if (completed) return
      completed = true
      console.warn(`[notes-widget] ⚠️ WS ${reason}, fallback REST`)
      cleanup()
      this._loadNotesRest()
    }

    const onNote = (e) => {
      this.notes = [...this.notes, e.detail.note]
      this.requestUpdate()
      // Stream alive — reset timeout
      if (wsTimeout) clearTimeout(wsTimeout)
      wsTimeout = setTimeout(() => fallbackToRest('stream stalled'), 5000)
    }

    const onComplete = () => {
      if (completed) return
      completed = true
      this.loading = false
      this.error = null
      console.log(`[notes-widget] ✅ Stream complete: ${this.notes.length} notes`)
      cleanup()
      this.requestUpdate()
    }

    const onError = (e) => fallbackToRest(`error: ${e.detail?.error}`)

    notesStreamService.addEventListener('note-received', onNote)
    notesStreamService.addEventListener('notes-complete', onComplete)
    notesStreamService.addEventListener('notes-error', onError)

    // If no WS response within 5s → REST
    wsTimeout = setTimeout(() => fallbackToRest('no response in 5s'), 5000)

    try {
      await notesStreamService.loadNotes({})
    } catch (err) {
      fallbackToRest(`connect failed: ${err.message}`)
    }
  }

  getTopNotes() {
    if (!this.notes || this.notes.length === 0) {
      return []
    }

    // Try context filter first
    const filtered = applyAllFilters(this.notes, {
      context: this.currentContext,
      contextFilterEnabled: true
    })

    if (filtered.length > 0) {
      return getTopPriorityNotes(filtered, this.currentContext, 3)
    }

    // Fallback: try veille/neutre context
    const veilleFallback = applyAllFilters(this.notes, {
      context: 'veille',
      contextFilterEnabled: true
    })

    if (veilleFallback.length > 0) {
      return getTopPriorityNotes(veilleFallback, 'veille', 3)
    }

    // Last resort: show all notes regardless of context
    return getTopPriorityNotes(this.notes, this.currentContext, 3)
  }

  openNotesPage() {
    // Dispatch custom event pour ouvrir la page notes
    // Plus fiable que querySelector direct
    this.dispatchEvent(new CustomEvent('open-notes-page', {
      bubbles: true,
      composed: true
    }))
  }

  openCreateNote() {
    // Dispatch event pour ouvrir directement en mode création
    this.dispatchEvent(new CustomEvent('create-note', {
      bubbles: true,
      composed: true
    }))
  }

  formatTimestamp(timestamp) {
    if (!timestamp || !Array.isArray(timestamp)) return ''

    const [year, day, hour, minute] = timestamp
    const date = new Date(year, 0, day, hour || 0, minute || 0)

    const now = new Date()
    const diff = now - date

    if (diff < 3600000) return `${Math.floor(diff/60000)}m`
    if (diff < 86400000) return `${Math.floor(diff/3600000)}h`
    return `${date.getDate()}/${date.getMonth()+1}`
  }

  getPreviewText(content) {
    if (!content) return ''
    // Remove markdown syntax
    const plainText = content.replace(/[#*`[\]()]/g, '').trim()
    // Limit to 60 characters
    return plainText.length > 60 ? plainText.substring(0, 60) + '...' : plainText
  }

  getContextIcon(context) {
    const icons = {
      'pro': '👔', 'cravate': '👔',
      'maison': '🏡', 'intime': '🏡',
      'focus': '🎯',
      'veille': '🌱', 'neutre': '🌱'
    }
    return icons[context] || '📍'
  }

  render() {
    const topNotes = this.getTopNotes()

    return html`
      <div class="widget-header">
        <h3 class="widget-title">
          📝 Notes
          <small class="nw-subtitle">
            (${this.currentContext})
          </small>
        </h3>
        <div class="header-actions">
          <button
            class="btn btn-small btn-icon"
            @click="${this.openCreateNote}"
            aria-label="Créer une note"
            title="Créer une note rapide">
            ➕
          </button>
          <button
            class="btn btn-small"
            @click="${this.openNotesPage}">
            Voir toutes
          </button>
        </div>
      </div>

      ${this.error ? html`
        <div class="empty-state nw-error-state">
          <div class="empty-state-icon">⚠️</div>
          <div class="empty-state-text">${this.error}</div>
        </div>
      ` : this.loading ? html`
        <organic-loader text="🧬 Organisme en synapse..."></organic-loader>
      ` : topNotes.length === 0 ? html`
        <div class="empty-state">
          <div class="empty-state-icon">📝</div>
          <div class="empty-state-text">Aucune note pour <strong>${this.currentContext}</strong></div>
          <div class="empty-state-hint">
            ${this.notes.length} note(s) au total
          </div>
        </div>
      ` : html`
        <div class="notes-list">
          ${topNotes.map(note => {
            const isPriority = isHighPriority(note, this.currentContext)

            return html`
              <div
                class="note-card ${note.data.urgent ? 'urgent' : ''} ${isPriority ? 'priority' : ''}"
                @click="${this.openNotesPage}">
                <div class="note-header">
                  ${note.data.urgent ? html`<span class="urgent-indicator">🚨</span>` : ''}
                  ${isPriority ? html`<span class="priority-badge">⭐</span>` : ''}
                  ${note.data.context ? html`
                    <span class="context-tag" style="${note.data.context === this.currentContext ? 'border: 2px solid var(--context-primary, #00d4aa);' : 'border: 1px solid var(--border-strong);'}">
                      ${this.getContextIcon(note.data.context)} ${note.data.context}
                    </span>
                  ` : html`
                    <span class="context-tag nw-no-context">
                      ⚠️ NO CONTEXT
                    </span>
                  `}
                </div>

                <div class="note-preview">
                  ${this.getPreviewText(note.data.content)}
                </div>

                <div class="note-meta">
                  <span class="note-tags">
                    ${note.data.tags && note.data.tags.length > 0 ? `#${note.data.tags[0]}` : ''}
                  </span>
                  <span>
                    ${this.formatTimestamp(note.timestamp)}
                  </span>
                </div>
              </div>
            `
          })}
        </div>
      `}
    `
  }
}

customElements.define('notes-widget', NotesWidget)
