/**
 * Widget Gestion des Notes
 * 
 * Interface pour les notes/mémos Symbion:
 * - Liste des notes avec filtres
 * - Création/édition/suppression  
 * - Marquage urgent/contexte
 */

import { LitElement, html, css } from 'lit'

class NotesWidget extends LitElement {
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
    
    .add-note-btn {
      background: rgba(0, 212, 170, 0.2);
      border: 1px solid rgba(0, 212, 170, 0.3);
      color: #00d4aa;
      padding: 0.4rem 0.8rem;
      border-radius: 6px;
      font-size: 0.8em;
      cursor: pointer;
      transition: all 0.3s ease;
    }
    
    .add-note-btn:hover {
      background: rgba(0, 212, 170, 0.3);
      border-color: rgba(0, 212, 170, 0.5);
    }
    
    .notes-filters {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
    }
    
    .filter-btn {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #e0e0e0;
      padding: 0.3rem 0.6rem;
      border-radius: 4px;
      font-size: 0.75em;
      cursor: pointer;
      transition: all 0.3s ease;
    }
    
    .filter-btn.active {
      background: rgba(0, 122, 204, 0.3);
      border-color: rgba(0, 122, 204, 0.5);
      color: #007acc;
    }
    
    .notes-list {
      display: flex;
      flex-direction: column;
      gap: 0.8rem;
      max-height: 400px;
      overflow-y: auto;
    }
    
    .note-card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 6px;
      padding: 0.8rem;
      transition: all 0.3s ease;
    }
    
    .note-card:hover {
      border-color: rgba(0, 122, 204, 0.3);
      background: rgba(255, 255, 255, 0.05);
    }
    
    .note-card.urgent {
      border-color: rgba(255, 107, 107, 0.5);
      background: rgba(255, 107, 107, 0.05);
    }
    
    .note-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.5rem;
    }
    
    .note-indicators {
      display: flex;
      gap: 0.3rem;
      align-items: center;
    }
    
    .urgent-indicator {
      color: #ff6b6b;
      font-weight: bold;
    }
    
    .context-tag {
      background: rgba(0, 122, 204, 0.2);
      color: #007acc;
      padding: 0.1rem 0.4rem;
      border-radius: 3px;
      font-size: 0.7em;
    }
    
    .note-content {
      color: #e0e0e0;
      margin-bottom: 0.5rem;
      line-height: 1.4;
    }
    
    .note-meta {
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 0.75em;
      opacity: 0.6;
    }
    
    .note-actions {
      display: flex;
      gap: 0.3rem;
    }
    
    .note-action {
      background: none;
      border: none;
      color: #007acc;
      cursor: pointer;
      padding: 0.2rem;
      border-radius: 3px;
      transition: all 0.3s ease;
    }
    
    .note-action:hover {
      background: rgba(0, 122, 204, 0.2);
    }
    
    .note-action.delete {
      color: #ff6b6b;
    }
    
    .note-action.delete:hover {
      background: rgba(255, 107, 107, 0.2);
    }
    
    .new-note-form {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 6px;
      padding: 1rem;
      margin-bottom: 1rem;
    }
    
    .form-field {
      margin-bottom: 0.8rem;
    }
    
    .form-field label {
      display: block;
      margin-bottom: 0.3rem;
      font-size: 0.9em;
      color: #e0e0e0;
    }
    
    .form-field input,
    .form-field textarea {
      width: 100%;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 4px;
      padding: 0.5rem;
      color: #e0e0e0;
      font-family: inherit;
      font-size: 0.9em;
    }
    
    .form-field textarea {
      resize: vertical;
      min-height: 80px;
    }
    
    .form-checkboxes {
      display: flex;
      gap: 1rem;
      margin-bottom: 0.8rem;
    }
    
    .checkbox-field {
      display: flex;
      align-items: center;
      gap: 0.3rem;
    }
    
    .form-actions {
      display: flex;
      gap: 0.5rem;
      justify-content: flex-end;
    }
    
    .form-btn {
      padding: 0.4rem 0.8rem;
      border-radius: 4px;
      font-size: 0.8em;
      cursor: pointer;
      transition: all 0.3s ease;
    }
    
    .form-btn.primary {
      background: rgba(0, 212, 170, 0.2);
      border: 1px solid rgba(0, 212, 170, 0.3);
      color: #00d4aa;
    }
    
    .form-btn.secondary {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #e0e0e0;
    }
    
    .placeholder {
      text-align: center;
      padding: 2rem;
      opacity: 0.6;
    }
  `
  
  static properties = {
    notes: { type: Array },
    apiService: { type: Object },
    showForm: { type: Boolean },
    currentFilter: { type: String },
    loading: { type: Boolean }
  }
  
  constructor() {
    super()
    this.notes = []
    this.apiService = null
    this.showForm = false
    this.currentFilter = 'all'
    this.loading = false
  }
  
  connectedCallback() {
    super.connectedCallback()
    this.loadNotes()
  }
  
  async loadNotes() {
    if (!this.apiService) return
    
    this.loading = true
    try {
      const filters = this.getFiltersForAPI()
      this.notes = await this.apiService.getNotes(filters)
    } catch (error) {
      console.error('❌ Failed to load notes:', error)
    } finally {
      this.loading = false
    }
  }
  
  getFiltersForAPI() {
    switch (this.currentFilter) {
      case 'urgent':
        return { urgent: 'true' }
      case 'recent':
        return { limit: '10' }
      default:
        return {}
    }
  }
  
  async handleCreateNote(event) {
    event.preventDefault()
    
    const formData = new FormData(event.target)
    const note = {
      content: formData.get('content'),
      context: formData.get('context') || null,
      urgent: formData.has('urgent'),
      tags: formData.get('tags') ? formData.get('tags').split(',').map(t => t.trim()) : []
    }
    
    try {
      await this.apiService.createNote(note)
      this.showForm = false
      await this.loadNotes()
      console.log('✅ Note created successfully')
    } catch (error) {
      console.error('❌ Failed to create note:', error)
    }
  }
  
  async handleDeleteNote(noteId) {
    if (!confirm('Supprimer cette note ?')) return
    
    try {
      await this.apiService.deleteNote(noteId)
      await this.loadNotes()
      console.log('✅ Note deleted successfully')
    } catch (error) {
      console.error('❌ Failed to delete note:', error)
    }
  }
  
  setFilter(filter) {
    this.currentFilter = filter
    this.loadNotes()
  }
  
  formatTimestamp(timestamp) {
    if (!timestamp || !Array.isArray(timestamp)) return ''
    
    // Format: [year, day_of_year, hour, minute, second, nanos, ?, ?, ?]
    const [year, day, hour, minute] = timestamp
    const date = new Date(year, 0, day, hour, minute)
    
    const now = new Date()
    const diff = now - date
    
    if (diff < 3600000) return `il y a ${Math.floor(diff/60000)}m`
    if (diff < 86400000) return `il y a ${Math.floor(diff/3600000)}h`
    return `${date.getDate()}/${date.getMonth()+1}`
  }

  formatDate(dateString) {
    if (!dateString) return ''
    
    const date = new Date(dateString)
    const now = new Date()
    const diff = now - date
    
    if (diff < 60000) return 'À l\'instant'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`
    
    return date.toLocaleDateString('fr-FR', { 
      month: 'short', 
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    })
  }
  
  render() {
    return html`
      <div class="widget-header">
        <h3 class="widget-title">📝 Notes</h3>
        <button 
          class="add-note-btn"
          @click="${() => this.showForm = !this.showForm}">
          ${this.showForm ? '❌' : '➕'} ${this.showForm ? 'Annuler' : 'Nouvelle'}
        </button>
      </div>
      
      ${this.showForm ? html`
        <form class="new-note-form" @submit="${this.handleCreateNote}">
          <div class="form-field">
            <label for="content">Contenu *</label>
            <textarea name="content" id="content" required placeholder="Votre note..."></textarea>
          </div>
          
          <div class="form-field">
            <label for="context">Contexte</label>
            <input name="context" id="context" placeholder="bureau, maison, travail...">
          </div>
          
          <div class="form-field">
            <label for="tags">Tags</label>
            <input name="tags" id="tags" placeholder="tag1, tag2, tag3">
          </div>
          
          <div class="form-checkboxes">
            <div class="checkbox-field">
              <input type="checkbox" name="urgent" id="urgent">
              <label for="urgent">🚨 Urgent</label>
            </div>
          </div>
          
          <div class="form-actions">
            <button type="button" class="form-btn secondary" @click="${() => this.showForm = false}">
              Annuler
            </button>
            <button type="submit" class="form-btn primary">
              ✅ Créer
            </button>
          </div>
        </form>
      ` : ''}
      
      <div class="notes-filters">
        <button 
          class="filter-btn ${this.currentFilter === 'all' ? 'active' : ''}"
          @click="${() => this.setFilter('all')}">
          Toutes
        </button>
        <button 
          class="filter-btn ${this.currentFilter === 'urgent' ? 'active' : ''}"
          @click="${() => this.setFilter('urgent')}">
          🚨 Urgentes
        </button>
        <button 
          class="filter-btn ${this.currentFilter === 'recent' ? 'active' : ''}"
          @click="${() => this.setFilter('recent')}">
          📅 Récentes
        </button>
      </div>
      
      ${this.notes.length === 0 ? html`
        <div class="placeholder">
          ${this.loading ? '⏳ Chargement...' : '📝 Aucune note trouvée'}
        </div>
      ` : html`
        <div class="notes-list">
          ${this.notes.map(note => html`
            <div class="note-card ${note.data.urgent ? 'urgent' : ''}">
              <div class="note-header">
                <div class="note-indicators">
                  ${note.data.urgent ? html`<span class="urgent-indicator">🚨</span>` : ''}
                  ${note.data.context ? html`<span class="context-tag">${note.data.context}</span>` : ''}
                </div>
                <div class="note-actions">
                  <button 
                    class="note-action delete"
                    @click="${() => this.handleDeleteNote(note.id)}"
                    title="Supprimer">
                    🗑️
                  </button>
                </div>
              </div>
              
              <div class="note-content">
                ${note.data.content}
              </div>
              
              <div class="note-meta">
                <span>
                  ${note.data.tags && note.data.tags.length > 0 ? `#${note.data.tags.join(' #')}` : ''}
                </span>
                <span>
                  ${this.formatTimestamp(note.timestamp)}
                </span>
              </div>
            </div>
          `)}
        </div>
      `}
    `
  }
}

customElements.define('notes-widget', NotesWidget)