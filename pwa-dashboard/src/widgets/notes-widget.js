/**
 * Widget Gestion des Notes
 *
 * Interface pour les notes/mémos Symbion:
 * - Liste des notes avec filtres
 * - Création/édition/suppression
 * - Marquage urgent/contexte
 * - Rendering markdown avec expand/collapse
 */

import { LitElement, html, css } from 'lit'
import { unsafeHTML } from 'lit/directives/unsafe-html.js'
import { marked } from 'marked'

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
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.25) 0%, rgba(34, 197, 94, 0.2) 100%);
      border: 1px solid rgba(0, 212, 170, 0.4);
      color: #00d4aa;
      padding: 0.5rem 1rem;
      border-radius: 8px;
      font-size: 0.85em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 8px rgba(0, 212, 170, 0.2);
    }

    .add-note-btn:hover {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.35) 0%, rgba(34, 197, 94, 0.3) 100%);
      border-color: rgba(0, 212, 170, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(0, 212, 170, 0.3);
    }
    
    .notes-filters {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
    }
    
    .filter-btn {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.15);
      color: #ccc;
      padding: 0.4rem 0.8rem;
      border-radius: 8px;
      font-size: 0.75em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
    }

    .filter-btn:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      border-color: rgba(0, 212, 170, 0.3);
      transform: translateY(-1px);
    }

    .filter-btn.active {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.3) 0%, rgba(0, 212, 170, 0.25) 100%);
      border-color: rgba(0, 212, 170, 0.5);
      color: #00d4aa;
      box-shadow: 0 2px 10px rgba(0, 212, 170, 0.3);
      transform: translateY(-1px);
    }
    
    .notes-list {
      display: flex;
      flex-direction: column;
      gap: 0.8rem;
      max-height: 400px;
      overflow-y: auto;
    }
    
    .note-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 10px;
      padding: 1rem;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      position: relative;
      overflow: hidden;
      box-shadow: 0 2px 12px rgba(0, 0, 0, 0.15);
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
      transform: translateY(-2px);
      box-shadow: 0 8px 24px rgba(0, 212, 170, 0.15);
    }

    .note-card:hover::before {
      opacity: 1;
    }

    .note-card.urgent {
      border-color: rgba(255, 107, 107, 0.5);
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.15) 0%, rgba(255, 107, 107, 0.05) 100%);
      box-shadow: 0 2px 12px rgba(255, 107, 107, 0.2);
    }

    .note-card.urgent::before {
      background: linear-gradient(180deg, #ff6b6b 0%, #ef4444 100%);
      opacity: 1;
      width: 4px;
      box-shadow: 0 0 15px rgba(255, 107, 107, 0.5);
    }

    .note-card.urgent:hover {
      border-color: rgba(255, 107, 107, 0.7);
      box-shadow: 0 8px 24px rgba(255, 107, 107, 0.25);
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
      font-size: 1.1em;
      filter: drop-shadow(0 2px 6px rgba(255, 107, 107, 0.6));
      animation: pulse-urgent 2s ease-in-out infinite;
    }

    @keyframes pulse-urgent {
      0%, 100% {
        opacity: 1;
        transform: scale(1);
      }
      50% {
        opacity: 0.8;
        transform: scale(1.1);
      }
    }

    .context-tag {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.25) 0%, rgba(0, 212, 170, 0.2) 100%);
      color: #00d4aa;
      padding: 0.2rem 0.6rem;
      border-radius: 12px;
      font-size: 0.7em;
      font-weight: 500;
      letter-spacing: 0.5px;
      border: 1px solid rgba(0, 212, 170, 0.3);
      box-shadow: 0 2px 6px rgba(0, 212, 170, 0.2);
      text-transform: uppercase;
    }
    
    .note-preview {
      color: #e0e0e0;
      margin-bottom: 0.5rem;
      line-height: 1.6;
      max-height: 3.2em;
      overflow: hidden;
      text-overflow: ellipsis;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      cursor: pointer;
    }

    /* Markdown styling */
    .note-content h1, .note-content h2, .note-content h3 {
      color: #00d4aa;
      margin: 0.8em 0 0.4em 0;
      font-weight: 600;
    }

    .note-content h1 { font-size: 1.4em; }
    .note-content h2 { font-size: 1.2em; }
    .note-content h3 { font-size: 1.1em; }

    .note-content p {
      margin: 0.5em 0;
    }

    .note-content code {
      background: rgba(0, 212, 170, 0.15);
      color: #00d4aa;
      padding: 0.2em 0.4em;
      border-radius: 4px;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 0.9em;
    }

    .note-content pre {
      background: rgba(0, 0, 0, 0.3);
      border: 1px solid rgba(0, 212, 170, 0.2);
      border-radius: 6px;
      padding: 0.8em;
      overflow-x: auto;
      margin: 0.5em 0;
    }

    .note-content pre code {
      background: none;
      padding: 0;
    }

    .note-content ul, .note-content ol {
      margin: 0.5em 0;
      padding-left: 1.5em;
    }

    .note-content li {
      margin: 0.3em 0;
    }

    .note-content a {
      color: #00d4aa;
      text-decoration: none;
      border-bottom: 1px solid rgba(0, 212, 170, 0.3);
      transition: all 0.2s ease;
    }

    .note-content a:hover {
      border-bottom-color: #00d4aa;
    }

    .note-content blockquote {
      border-left: 3px solid #00d4aa;
      padding-left: 1em;
      margin: 0.5em 0;
      color: #888;
      font-style: italic;
    }

    /* Modal */
    .note-modal {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(8px);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 10000;
      animation: fadeIn 0.3s ease;
    }

    .note-modal-content {
      background: linear-gradient(135deg, rgba(26, 26, 26, 0.98) 0%, rgba(15, 15, 15, 0.95) 100%);
      border: 1px solid rgba(0, 212, 170, 0.2);
      border-radius: 16px;
      width: 90%;
      max-width: 700px;
      max-height: 85vh;
      overflow-y: auto;
      padding: 1.5rem;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(0, 212, 170, 0.1);
      animation: modalSlideIn 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .note-modal-content::-webkit-scrollbar {
      width: 8px;
    }

    .note-modal-content::-webkit-scrollbar-track {
      background: rgba(255, 255, 255, 0.05);
      border-radius: 4px;
    }

    .note-modal-content::-webkit-scrollbar-thumb {
      background: rgba(0, 212, 170, 0.3);
      border-radius: 4px;
    }

    .note-modal-content::-webkit-scrollbar-thumb:hover {
      background: rgba(0, 212, 170, 0.5);
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    @keyframes modalSlideIn {
      from {
        opacity: 0;
        transform: translateY(-30px) scale(0.95);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    .modal-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: 1.2rem;
      padding-bottom: 0.8rem;
      border-bottom: 1px solid rgba(0, 212, 170, 0.15);
      gap: 1rem;
    }

    .modal-close-btn {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #888;
      font-size: 24px;
      cursor: pointer;
      padding: 6px 10px;
      border-radius: 8px;
      transition: all 0.3s ease;
      line-height: 1;
      flex-shrink: 0;
    }

    .modal-close-btn:hover {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(220, 38, 38, 0.15) 100%);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ff6b6b;
      transform: rotate(90deg);
    }

    .note-content {
      color: #e0e0e0;
      line-height: 1.8;
    }

    /* Search and filters - Compact layout */
    .search-bar {
      margin-bottom: 0.6rem;
    }

    .search-input {
      width: 100%;
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 8px;
      padding: 0.4rem 0.7rem;
      color: #e0e0e0;
      font-size: 0.75em;
      transition: all 0.3s ease;
    }

    .search-input:focus {
      outline: none;
      border-color: rgba(0, 212, 170, 0.5);
      box-shadow: 0 0 0 2px rgba(0, 212, 170, 0.1);
    }

    .search-input::placeholder {
      color: #666;
    }

    .tags-section {
      margin-bottom: 0.8rem;
    }

    .tags-label {
      color: #888;
      font-size: 0.65em;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 0.3rem;
      display: block;
      font-weight: 600;
    }

    .tags-filter {
      display: flex;
      gap: 0.25rem;
      flex-wrap: wrap;
    }

    .tag-filter-btn {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #888;
      padding: 0.2rem 0.45rem;
      border-radius: 8px;
      font-size: 0.65em;
      cursor: pointer;
      transition: all 0.3s ease;
      font-weight: 500;
    }

    .tag-filter-btn:hover {
      border-color: rgba(0, 212, 170, 0.3);
      color: #ccc;
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.04) 100%);
    }

    .tag-filter-btn.active {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.2) 0%, rgba(0, 212, 170, 0.15) 100%);
      border-color: rgba(0, 212, 170, 0.4);
      color: #00d4aa;
      box-shadow: 0 2px 6px rgba(0, 212, 170, 0.25);
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
      padding: 0.5rem 1rem;
      border-radius: 8px;
      font-size: 0.85em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
    }

    .form-btn.primary {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.25) 0%, rgba(34, 197, 94, 0.2) 100%);
      border: 1px solid rgba(0, 212, 170, 0.4);
      color: #00d4aa;
    }

    .form-btn.primary:hover {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.35) 0%, rgba(34, 197, 94, 0.3) 100%);
      border-color: rgba(0, 212, 170, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(0, 212, 170, 0.3);
    }

    .form-btn.secondary {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.15);
      color: #ccc;
    }

    .form-btn.secondary:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      border-color: rgba(255, 255, 255, 0.25);
      transform: translateY(-2px);
    }
    
    .placeholder {
      text-align: center;
      padding: 2rem;
      opacity: 0.6;
    }

    /* Responsive */
    @media (max-width: 768px) {
      .notes-filters {
        flex-wrap: wrap;
      }

      .filter-btn {
        flex: 1;
        min-width: 90px;
      }

      .note-card {
        padding: 0.8rem;
      }

      .widget-header {
        flex-direction: row;
        gap: 0.8rem;
      }

      .add-note-btn {
        padding: 0.4rem 0.8rem;
        font-size: 0.8em;
      }

      .notes-list {
        max-height: 350px;
      }
    }

    @media (max-width: 480px) {
      .widget-header {
        flex-direction: column;
        align-items: stretch;
      }

      .add-note-btn {
        width: 100%;
      }

      .notes-filters {
        gap: 0.4rem;
      }

      .filter-btn {
        padding: 0.35rem 0.6rem;
      }
    }
  `
  
  static properties = {
    notes: { type: Array },
    apiService: { type: Object },
    showForm: { type: Boolean },
    currentFilter: { type: String },
    loading: { type: Boolean },
    selectedNote: { type: Object },
    searchQuery: { type: String },
    availableTags: { type: Array },
    selectedTags: { type: Array }
  }

  constructor() {
    super()
    this.notes = []
    this.apiService = null
    this.showForm = false
    this.currentFilter = 'all'
    this.loading = false
    this.selectedNote = null
    this.searchQuery = ''
    this.availableTags = []
    this.selectedTags = []
  }
  
  connectedCallback() {
    super.connectedCallback()
    this.loadNotes()

    // Close modal with Escape key
    this.handleEscape = (e) => {
      if (e.key === 'Escape' && this.selectedNote) {
        this.closeNoteModal()
      }
    }
    document.addEventListener('keydown', this.handleEscape)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this.handleEscape)
  }
  
  async loadNotes() {
    if (!this.apiService) return

    this.loading = true
    try {
      const filters = this.getFiltersForAPI()
      this.notes = await this.apiService.getNotes(filters)
      this.extractAvailableTags()
    } catch (error) {
      console.error('❌ Failed to load notes:', error)
    } finally {
      this.loading = false
    }
  }

  extractAvailableTags() {
    const tagsSet = new Set()
    this.notes.forEach(note => {
      if (note.data.tags && Array.isArray(note.data.tags)) {
        note.data.tags.forEach(tag => tagsSet.add(tag))
      }
    })
    this.availableTags = Array.from(tagsSet).sort()
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

  openNoteModal(note) {
    this.selectedNote = note
  }

  closeNoteModal() {
    this.selectedNote = null
  }

  toggleTagFilter(tag) {
    if (this.selectedTags.includes(tag)) {
      this.selectedTags = this.selectedTags.filter(t => t !== tag)
    } else {
      this.selectedTags = [...this.selectedTags, tag]
    }
  }

  getFilteredNotes() {
    let filtered = [...this.notes]

    // Filter by search query
    if (this.searchQuery) {
      const query = this.searchQuery.toLowerCase()
      filtered = filtered.filter(note =>
        note.data.content.toLowerCase().includes(query) ||
        (note.data.context && note.data.context.toLowerCase().includes(query)) ||
        (note.data.tags && note.data.tags.some(tag => tag.toLowerCase().includes(query)))
      )
    }

    // Filter by selected tags
    if (this.selectedTags.length > 0) {
      filtered = filtered.filter(note =>
        note.data.tags && note.data.tags.some(tag => this.selectedTags.includes(tag))
      )
    }

    // Filter by filter type (all/urgent/recent)
    switch (this.currentFilter) {
      case 'urgent':
        filtered = filtered.filter(note => note.data.urgent)
        break
      case 'recent':
        filtered = filtered.slice(0, 10)
        break
    }

    return filtered
  }

  renderMarkdown(content) {
    if (!content) return ''
    try {
      return marked.parse(content)
    } catch (error) {
      console.error('Failed to parse markdown:', error)
      return content
    }
  }

  getPreviewText(content) {
    if (!content) return ''
    // Remove markdown syntax for preview
    const plainText = content.replace(/[#*`[\]()]/g, '').trim()
    return plainText.length > 100 ? plainText.substring(0, 100) + '...' : plainText
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
      
      <!-- Search -->
      <div class="search-bar">
        <input
          type="text"
          class="search-input"
          placeholder="🔍 Rechercher..."
          .value="${this.searchQuery}"
          @input="${(e) => this.searchQuery = e.target.value}">
      </div>

      <!-- Tags filter -->
      ${this.availableTags.length > 0 ? html`
        <div class="tags-section">
          <span class="tags-label">Filtrer par tags</span>
          <div class="tags-filter">
            ${this.availableTags.map(tag => html`
              <button
                class="tag-filter-btn ${this.selectedTags.includes(tag) ? 'active' : ''}"
                @click="${() => this.toggleTagFilter(tag)}">
                #${tag}
              </button>
            `)}
          </div>
        </div>
      ` : ''}

      ${this.getFilteredNotes().length === 0 ? html`
        <div class="placeholder">
          ${this.loading ? '⏳ Chargement...' : '📝 Aucune note trouvée'}
        </div>
      ` : html`
        <div class="notes-list">
          ${this.getFilteredNotes().map(note => html`
            <div class="note-card ${note.data.urgent ? 'urgent' : ''}" @click="${() => this.openNoteModal(note)}">
              <div class="note-header">
                <div class="note-indicators">
                  ${note.data.urgent ? html`<span class="urgent-indicator">🚨</span>` : ''}
                  ${note.data.context ? html`<span class="context-tag">${note.data.context}</span>` : ''}
                </div>
                <div class="note-actions">
                  <button
                    class="note-action delete"
                    @click="${(e) => { e.stopPropagation(); this.handleDeleteNote(note.id); }}"
                    title="Supprimer">
                    🗑️
                  </button>
                </div>
              </div>

              <div class="note-preview">
                ${this.getPreviewText(note.data.content)}
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

      <!-- Note Modal -->
      ${this.selectedNote ? html`
        <div class="note-modal" @click="${this.closeNoteModal}">
          <div class="note-modal-content" @click="${(e) => e.stopPropagation()}">
            <div class="modal-header">
              <div style="flex: 1; display: flex; flex-direction: column; gap: 0.4rem;">
                <div style="display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap;">
                  ${this.selectedNote.data.urgent ? html`<span class="urgent-indicator">🚨 URGENT</span>` : ''}
                  ${this.selectedNote.data.context ? html`<span class="context-tag">${this.selectedNote.data.context}</span>` : ''}
                </div>
                ${this.selectedNote.data.tags && this.selectedNote.data.tags.length > 0 ? html`
                  <div style="color: #888; font-size: 0.75em; font-weight: 500;">
                    ${this.selectedNote.data.tags.map(tag => `#${tag}`).join(' ')}
                  </div>
                ` : ''}
              </div>
              <button class="modal-close-btn" @click="${this.closeNoteModal}">×</button>
            </div>
            <div class="note-content">
              ${unsafeHTML(this.renderMarkdown(this.selectedNote.data.content))}
            </div>
            <div class="note-meta" style="margin-top: 1.2rem; padding-top: 0.8rem; border-top: 1px solid rgba(255, 255, 255, 0.08);">
              <span style="font-size: 0.8em;">📅 ${this.formatTimestamp(this.selectedNote.timestamp)}</span>
            </div>
          </div>
        </div>
      ` : ''}
    `
  }
}

customElements.define('notes-widget', NotesWidget)