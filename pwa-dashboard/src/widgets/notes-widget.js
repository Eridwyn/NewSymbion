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
import { getTopPriorityNotes, isHighPriority } from '../utils/notes-scoring.js'
import { applyAllFilters } from '../utils/notes-filters.js'

class NotesWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .widget-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1rem;
    }

    .widget-title {
      font-size: 1.1em;
      font-weight: 600;
      color: #e0e0e0;
    }

    .header-actions {
      display: flex;
      gap: 0.5rem;
    }

    .view-all-btn, .create-btn {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.2) 0%, rgba(34, 197, 94, 0.15) 100%);
      border: 1px solid rgba(0, 212, 170, 0.3);
      color: #00d4aa;
      padding: 0.4rem 0.8rem;
      border-radius: 6px;
      font-size: 0.8em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s ease;
    }

    .create-btn {
      padding: 0.4rem 0.6rem;
      font-size: 1em;
    }

    .view-all-btn:hover, .create-btn:hover {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.3) 0%, rgba(34, 197, 94, 0.25) 100%);
      border-color: rgba(0, 212, 170, 0.5);
      transform: translateY(-1px);
    }

    .notes-list {
      display: flex;
      flex-direction: column;
      gap: 0.6rem;
    }

    .note-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 0.8rem;
      transition: all 0.3s ease;
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
      background: linear-gradient(180deg, rgba(0, 122, 204, 0.5) 0%, rgba(0, 212, 170, 0.3) 100%);
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .note-card:hover {
      border-color: rgba(0, 212, 170, 0.4);
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      transform: translateY(-1px);
    }

    .note-card:hover::before {
      opacity: 1;
    }

    .note-card.urgent {
      border-color: rgba(255, 107, 107, 0.5);
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.12) 0%, rgba(255, 107, 107, 0.04) 100%);
    }

    .note-card.urgent::before {
      background: linear-gradient(180deg, #ff6b6b 0%, #ef4444 100%);
      opacity: 1;
      width: 3px;
    }

    .note-card.priority {
      border-color: rgba(255, 193, 7, 0.3);
    }

    .note-header {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      margin-bottom: 0.4rem;
    }

    .urgent-indicator {
      color: #ff6b6b;
      font-size: 0.9em;
    }

    .priority-badge {
      background: rgba(255, 193, 7, 0.2);
      color: #ffc107;
      padding: 0.1rem 0.4rem;
      border-radius: 4px;
      font-size: 0.65em;
      font-weight: 600;
    }

    .context-tag {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.2) 0%, rgba(0, 212, 170, 0.15) 100%);
      color: #00d4aa;
      padding: 0.1rem 0.4rem;
      border-radius: 6px;
      font-size: 0.65em;
      font-weight: 500;
      text-transform: uppercase;
    }

    .note-preview {
      color: #ccc;
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
      color: #00d4aa;
    }

    .placeholder {
      text-align: center;
      padding: 2rem 1rem;
      opacity: 0.5;
      font-size: 0.9em;
    }

    @media (max-width: 768px) {
      .note-card {
        padding: 0.6rem;
      }

      .note-preview {
        font-size: 0.8em;
      }
    }
  `

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
    this.currentContext = 'neutre'
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
    const newMode = event.detail.context?.mode || event.detail.context
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

    const checkServices = () => {
      this.apiService = document.querySelector('api-service')
      this.contextService = document.querySelector('context-service')

      if (this.apiService && this.contextService) {
        console.log('[notes-widget] ✅ Services trouvés')

        // Écouter les changements de contexte sur window (comme context-widget)
        window.addEventListener('context-change', this.handleContextChange)

        // Récupérer le contexte actuel depuis contextService
        const contextState = this.contextService.getContextState()
        if (contextState && contextState.mode) {
          this.currentContext = contextState.mode
          console.log('[notes-widget] Initial context from service:', this.currentContext)
        } else {
          console.warn('[notes-widget] ⚠️ Context state not ready yet, defaulting to neutre')
        }

        // Charger les notes
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

  async loadNotes() {
    if (!this.apiService) {
      console.warn('[notes-widget] ⚠️ apiService not available yet')
      this.error = 'Service API non disponible'
      return
    }

    this.loading = true
    this.error = null
    try {
      console.log('[notes-widget] 📡 Loading notes from API...')
      this.notes = await this.apiService.getNotes({})
      console.log('[notes-widget] ✅ Notes loaded:', this.notes.length, 'notes')
      console.log('[notes-widget] Current context:', this.currentContext)

      if (!this.notes || this.notes.length === 0) {
        this.error = `Aucune note trouvée. Créez-en une avec ➕`
      } else {
        // Vérifier si des notes correspondent au contexte
        const filtered = applyAllFilters(this.notes, {
          context: this.currentContext,
          contextFilterEnabled: true
        })

        console.log(`[notes-widget] ${this.notes.length} notes totales, ${filtered.length} pour contexte ${this.currentContext}`)
      }

      this.requestUpdate()
    } catch (error) {
      console.error('[notes-widget] ❌ Failed to load notes:', error)
      this.error = `Erreur chargement: ${error.message || 'API inaccessible'}`
    } finally {
      this.loading = false
    }
  }

  getTopNotes() {
    console.log('[notes-widget] getTopNotes() - Total notes:', this.notes?.length || 0)
    console.log('[notes-widget] Current context:', this.currentContext)

    if (!this.notes || this.notes.length === 0) {
      console.log('[notes-widget] No notes available')
      return []
    }

    // Filtre strict par contexte actuel
    const filtered = applyAllFilters(this.notes, {
      context: this.currentContext,
      contextFilterEnabled: true
    })

    console.log('[notes-widget] After context filter:', filtered.length, 'notes (context:', this.currentContext + ')')

    // Debug: afficher les contextes des notes
    if (filtered.length === 0) {
      console.log('[notes-widget] ⚠️ Available note contexts:',
        this.notes.map(n => n.data?.context || 'undefined').join(', '))
    }

    // Retourner les 3 notes les plus prioritaires du contexte actuel UNIQUEMENT
    const topNotes = getTopPriorityNotes(filtered, this.currentContext, 3)
    console.log('[notes-widget] Top 3 priority notes:', topNotes.length)

    return topNotes
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
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌱'
    }
    return icons[context] || '📍'
  }

  render() {
    const topNotes = this.getTopNotes()

    return html`
      <div class="widget-header">
        <h3 class="widget-title">
          📝 Notes
          <small style="font-size: 0.6em; opacity: 0.7; font-weight: normal;">
            (${this.currentContext})
          </small>
        </h3>
        <div class="header-actions">
          <button
            class="create-btn"
            @click="${this.openCreateNote}"
            title="Créer une note rapide">
            ➕
          </button>
          <button
            class="view-all-btn"
            @click="${this.openNotesPage}">
            Voir toutes
          </button>
        </div>
      </div>

      ${this.error ? html`
        <div class="placeholder" style="color: #ff6b6b;">
          ⚠️ ${this.error}
        </div>
      ` : this.loading ? html`
        <div class="placeholder">⏳ Chargement...</div>
      ` : topNotes.length === 0 ? html`
        <div class="placeholder">
          📝 Aucune note pour <strong>${this.currentContext}</strong>
          <br>
          <small style="opacity: 0.7; font-size: 0.8em;">
            ${this.notes.length} note(s) au total
          </small>
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
                    <span class="context-tag" style="${note.data.context === this.currentContext ? 'border: 2px solid #00d4aa;' : 'border: 1px solid rgba(255,255,255,0.3);'}">
                      ${this.getContextIcon(note.data.context)} ${note.data.context}
                    </span>
                  ` : html`
                    <span class="context-tag" style="background: rgba(255,107,107,0.2); color: #ff6b6b;">
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
