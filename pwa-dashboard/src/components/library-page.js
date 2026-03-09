/**
 * Library Page — Bibliothèque de Connaissances
 *
 * Page principale avec 5 vues accessibles par tabs :
 * 1. Bibliothèque (vue graphe sections)
 * 2. Bureau d'étude (noeud actif + connexions)
 * 3. Éditeur (création/édition fiche)
 * 4. Patrons (gestion templates)
 * 5. Recherche (barre universelle FTS)
 */

import { LitElement, html, css } from 'lit'
import { unsafeHTML } from 'lit/directives/unsafe-html.js'
import DOMPurify from 'dompurify'
import { sharedAnimations, pageTransitionStyles } from '../styles/shared-animations.js'
import { overlayStyles } from '../styles/shared-patterns.js'
import { pageHeaderStyles, tabPillStyles } from '../styles/shared-page.js'
import { formInputStyles, formGroupStyles, btnStyles } from '../styles/shared-forms.js'

const API_BASE = () => window.location.origin
const AUTH = () => sessionStorage.getItem('symbion_auth_token') || ''

async function api(path, options = {}) {
  const res = await fetch(`${API_BASE()}/v1/plugin-api/library${path}`, {
    ...options,
    headers: {
      'Authorization': `Bearer ${AUTH()}`,
      'Content-Type': 'application/json',
      ...(options.headers || {})
    }
  })
  if (!res.ok && res.status !== 204) throw new Error(`HTTP ${res.status}`)
  if (res.status === 204) return null
  return res.json()
}

export class LibraryPage extends LitElement {
  static properties = {
    activeTab: { type: String },
    // Library view
    sections: { type: Array },
    graphData: { type: Object },
    // Study desk
    deskNodeId: { type: String },
    desk: { type: Object },
    // Nodes
    nodes: { type: Array },
    selectedNode: { type: Object },
    // Editor
    editingNode: { type: Object },
    editorMode: { type: String }, // 'create' | 'edit'
    // Templates
    templates: { type: Array },
    editingTemplate: { type: Object },
    // Search
    searchQuery: { type: String },
    searchResults: { type: Array },
    // Tags
    tags: { type: Array },
    // Pending links
    pendingLinks: { type: Array },
    // Trash
    trash: { type: Array },
    showTrash: { type: Boolean },
    // State
    loading: { type: Boolean },
    sectionNodes: { type: Array },
    selectedSection: { type: Object },
    // Toasts
    toasts: { type: Array },
    // Template editor
    templateJsonError: { type: String },
    // Versions panel
    showVersions: { type: Boolean },
    // Unsaved changes
    _hasUnsavedChanges: { type: Boolean },
  }

  static styles = [sharedAnimations, pageTransitionStyles, overlayStyles, pageHeaderStyles, formInputStyles, btnStyles, css`
    :host {
      position: fixed;
      top: 0; left: 0; right: 0; bottom: 0;
      z-index: 1000;
      background: var(--color-dark-bg-primary, #0a0a0f);
      overflow-y: auto;
      animation: slideUp 0.3s ease-out;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .page-wrap {
      max-width: 1100px;
      margin: 0 auto;
      padding: 1rem 1.5rem 3rem;
    }

    .page-header {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 1rem;
    }

    .back-btn {
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      color: var(--context-primary, #00d4aa);
      border-radius: var(--radius-md);
      padding: 0.4rem 0.8rem;
      cursor: pointer;
      font-size: 1.1em;
      transition: all 0.2s;
    }

    .back-btn:hover {
      background: var(--surface-glass-hover);
      border-color: var(--context-primary, #00d4aa);
    }

    h2 {
      margin: 0;
      font-size: 1.3em;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Tabs */
    .tabs {
      display: flex;
      gap: 0.4rem;
      margin-bottom: 1.5rem;
      overflow-x: auto;
      padding-bottom: 0.3rem;
      border-bottom: 1px solid var(--border-default);
    }

    .tab-btn {
      padding: 0.5rem 1rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-bottom: none;
      border-radius: var(--radius-lg) var(--radius-lg) 0 0;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.82em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
      letter-spacing: 0.3px;
    }

    .tab-btn:hover {
      background: var(--surface-glass-hover);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .tab-btn.active {
      background: var(--ctx-bg-emphasis);
      border-color: var(--context-primary, #00d4aa);
      border-bottom: 2px solid var(--context-primary, #00d4aa);
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
    }

    /* Cards */
    .card {
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 1rem;
      margin-bottom: 0.8rem;
      cursor: pointer;
      transition: all 0.2s;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    }

    .card:hover {
      border-color: var(--context-primary, #00d4aa);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
      background: var(--surface-glass-hover);
    }

    .card-title {
      font-weight: 600;
      margin-bottom: 0.3rem;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .card-meta {
      font-size: 0.75em;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    /* Grid */
    .cards-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 0.8rem;
    }

    /* Section cards */
    .section-card {
      border-left: 4px solid var(--section-color, var(--context-primary, #00d4aa));
    }

    .section-count {
      font-size: 0.8em;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    /* Study desk */
    .desk-center {
      background: var(--surface-glass);
      border: 2px solid var(--context-primary, #00d4aa);
      border-radius: var(--radius-xl);
      padding: 1.5rem;
      margin-bottom: 1.5rem;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    }

    .desk-connections {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
      gap: 0.6rem;
    }

    .connection-card {
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 0.8rem;
      cursor: pointer;
      transition: all 0.2s;
      box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
    }

    .connection-card:hover {
      border-color: var(--context-primary, #00d4aa);
      transform: translateY(-1px);
      background: var(--surface-glass-hover);
    }

    .relation-badge {
      display: inline-block;
      font-size: 0.7em;
      padding: 0.15rem 0.4rem;
      background: var(--ctx-bg-emphasis);
      border: 1px solid var(--ctx-border, rgba(0, 212, 170, 0.15));
      border-radius: var(--radius-sm);
      color: var(--context-primary, #00d4aa);
      margin-top: 0.3rem;
    }

    /* Pending links */
    .pending-section {
      margin-top: 1.5rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border-default);
    }

    .pending-card {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: var(--surface-glass);
      border: 1px solid rgba(251, 191, 36, 0.35);
      border-radius: var(--radius-lg);
      padding: 0.7rem 1rem;
      margin-bottom: 0.5rem;
      box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
    }

    .pending-actions {
      display: flex;
      gap: 0.4rem;
    }

    .btn-sm {
      padding: 0.3rem 0.6rem;
      font-size: 0.75em;
      border-radius: var(--radius-md);
      border: 1px solid var(--border-default);
      cursor: pointer;
      background: var(--surface-glass);
      color: var(--color-dark-text-primary, #f8f9fa);
      transition: all 0.2s;
    }

    .btn-sm:hover {
      background: var(--surface-glass-hover);
    }

    .btn-confirm { border-color: rgba(34, 197, 94, 0.45); color: #22c55e; }
    .btn-confirm:hover { background: rgba(34, 197, 94, 0.15); }
    .btn-dismiss { border-color: rgba(239, 68, 68, 0.45); color: #ef4444; }
    .btn-dismiss:hover { background: rgba(239, 68, 68, 0.15); }

    /* Editor */
    .editor-wrap {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .editor-form {
      display: flex;
      flex-direction: column;
      gap: 0.8rem;
    }

    .form-field label {
      display: block;
      font-size: 0.8em;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 0.3rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .form-field input,
    .form-field textarea,
    .form-field select {
      width: 100%;
      padding: 0.6rem 0.8rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: inherit;
      font-size: 0.9em;
      box-sizing: border-box;
      transition: border-color 0.2s;
    }

    .form-field input:focus,
    .form-field textarea:focus,
    .form-field select:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 2px var(--ctx-border-subtle);
    }

    .form-field textarea {
      min-height: 200px;
      resize: vertical;
      font-family: 'Fira Code', monospace;
    }

    .form-row {
      display: flex;
      gap: 0.8rem;
    }

    .form-row > .form-field { flex: 1; }

    .btn-primary {
      padding: 0.6rem 1.5rem;
      background: var(--context-primary, #00d4aa);
      border: none;
      border-radius: var(--radius-lg);
      color: #0a0a0f;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    }

    .btn-primary:hover {
      opacity: 0.9;
      transform: translateY(-1px);
      box-shadow: 0 4px 10px rgba(0, 0, 0, 0.4);
    }

    .btn-secondary {
      padding: 0.6rem 1.5rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      color: var(--color-dark-text-primary, #f8f9fa);
      cursor: pointer;
      transition: all 0.2s;
    }

    .btn-secondary:hover {
      background: var(--surface-glass-hover);
      border-color: var(--context-primary, #00d4aa);
    }

    .btn-danger {
      padding: 0.6rem 1.5rem;
      background: rgba(239, 68, 68, 0.15);
      border: 1px solid rgba(239, 68, 68, 0.45);
      border-radius: var(--radius-lg);
      color: #ef4444;
      cursor: pointer;
    }

    .btn-danger:hover {
      background: rgba(239, 68, 68, 0.25);
    }

    .actions-bar {
      display: flex;
      gap: 0.6rem;
      flex-wrap: wrap;
    }

    /* Template cards */
    .template-card {
      border-left: 4px solid var(--context-primary, #00d4aa);
    }

    .template-structure {
      font-size: 0.75em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-top: 0.3rem;
    }

    /* Tags */
    .tags-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.3rem;
      margin-top: 0.4rem;
    }

    .tag {
      background: var(--ctx-bg-emphasis);
      color: var(--context-primary, #00d4aa);
      padding: 0.15rem 0.5rem;
      border-radius: var(--radius-sm);
      font-size: 0.75em;
      border: 1px solid var(--ctx-border, rgba(0, 212, 170, 0.15));
    }

    /* Empty */
    .empty {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    /* Content preview */
    .content-preview {
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 1rem;
      margin-top: 1rem;
      white-space: pre-wrap;
      font-size: 0.85em;
      max-height: 400px;
      overflow-y: auto;
      box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.3);
    }

    /* Template preview container */
    .template-preview {
      margin-top: 1.2rem;
      padding: 1.5rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      box-shadow: inset 0 2px 12px rgba(0, 0, 0, 0.3);
      display: flex;
      justify-content: center;
      overflow: hidden;
    }

    .template-preview > style {
      display: none;
    }

    /* Versions */
    .version-item {
      display: flex;
      justify-content: space-between;
      padding: 0.5rem 0;
      border-bottom: 1px solid var(--border-default);
      font-size: 0.85em;
    }

    /* Trash bar */
    .trash-toggle {
      display: flex;
      justify-content: flex-end;
      margin-bottom: 0.5rem;
    }

    .toast-container {
      position: fixed;
      bottom: 1.5rem;
      right: 1.5rem;
      z-index: 9999;
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .toast {
      padding: 0.7rem 1.2rem;
      border-radius: var(--radius-lg);
      font-size: 0.85em;
      font-weight: 500;
      animation: slideUp 0.3s ease-out;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
      max-width: 350px;
    }

    .toast-success {
      background: rgba(34, 197, 94, 0.2);
      border: 1px solid rgba(34, 197, 94, 0.5);
      color: #22c55e;
    }

    .toast-error {
      background: rgba(239, 68, 68, 0.2);
      border: 1px solid rgba(239, 68, 68, 0.5);
      color: #ef4444;
    }

    .toast-info {
      background: rgba(59, 130, 246, 0.2);
      border: 1px solid rgba(59, 130, 246, 0.5);
      color: #3b82f6;
    }

    @media (max-width: 768px) {
      .page-wrap { padding: 0.75rem; }
      .cards-grid { grid-template-columns: 1fr; }
      .form-row { flex-direction: column; }
      .desk-connections { grid-template-columns: 1fr; }
    }
  `]

  constructor() {
    super()
    this.activeTab = 'library'
    this.sections = []
    this.graphData = null
    this.deskNodeId = null
    this.desk = null
    this.nodes = []
    this.selectedNode = null
    this.editingNode = null
    this.editorMode = 'create'
    this.templates = []
    this.editingTemplate = null
    this.searchQuery = ''
    this.searchResults = []
    this.tags = []
    this.pendingLinks = []
    this.trash = []
    this.showTrash = false
    this.loading = false
    this.sectionNodes = []
    this.selectedSection = null
    this.toasts = []
    this.templateJsonError = ''
    this.showVersions = false
    this._hasUnsavedChanges = false
    this._versions = []
  }

  connectedCallback() {
    super.connectedCallback()
    this._handleEscape = (e) => { if (e.key === 'Escape') this.close() }
    document.addEventListener('keydown', this._handleEscape)
    this._handleBeforeUnload = (e) => {
      if (this._hasUnsavedChanges) {
        e.preventDefault()
        e.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', this._handleBeforeUnload)
    this.loadTab()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this._handleEscape)
    window.removeEventListener('beforeunload', this._handleBeforeUnload)
  }

  close() {
    if (this._hasUnsavedChanges) {
      if (!confirm('Vous avez des modifications non sauvegardées. Quitter ?')) return
    }
    this.dispatchEvent(new CustomEvent('close'))
  }

  setTab(tab) {
    if (tab === 'editor' && this.activeTab === 'editor') return
    if (this.activeTab === 'editor' && this._hasUnsavedChanges) {
      if (!confirm('Vous avez des modifications non sauvegardées. Quitter ?')) return
    }
    this._hasUnsavedChanges = false
    this.activeTab = tab
    this.loadTab()
  }

  showToast(message, type = 'info', duration = 3000) {
    const id = Date.now()
    this.toasts = [...this.toasts, { id, message, type }]
    setTimeout(() => {
      this.toasts = this.toasts.filter(t => t.id !== id)
    }, duration)
  }

  async loadTab() {
    this.loading = true
    try {
      switch (this.activeTab) {
        case 'library':
          const [graphData, sectionsData] = await Promise.all([
            api('/graph'),
            api('/sections')
          ])
          this.graphData = graphData
          this.sections = sectionsData.sections || []
          break
        case 'desk':
          if (this.deskNodeId) {
            this.desk = await api(`/nodes/${this.deskNodeId}/desk`)
          } else {
            // Load nodes and find active one
            const nodesData = await api('/nodes')
            this.nodes = nodesData.nodes || []
            const active = this.nodes.find(n => n.is_active)
            if (active) {
              this.deskNodeId = active.id
              this.desk = await api(`/nodes/${active.id}/desk`)
            }
          }
          this.templates = (await api('/templates')).templates || []
          this.pendingLinks = (await api('/pending-links')).pending_links || []
          break
        case 'editor':
          this.templates = (await api('/templates')).templates || []
          this.tags = (await api('/tags')).tags || []
          if (!this.editingNode && this.editorMode === 'create') {
            this.editingNode = { title: '', content: '', template_id: '', tags: [], section_ids: [] }
          }
          this.sections = (await api('/sections')).sections || []
          break
        case 'templates':
          this.templates = (await api('/templates')).templates || []
          break
        case 'search':
          this.tags = (await api('/tags')).tags || []
          break
      }
    } catch (err) {
      console.error('[library] Load error:', err)
    } finally {
      this.loading = false
    }
  }

  // ── Actions ──

  async openSection(section) {
    this.selectedSection = section
    try {
      const data = await api(`/sections/${section.id}/nodes`)
      this.sectionNodes = data.nodes || []
    } catch (err) {
      console.error('[library] Section nodes error:', err)
    }
  }

  closeSection() {
    this.selectedSection = null
    this.sectionNodes = []
  }

  openDesk(nodeId) {
    this.deskNodeId = nodeId
    this.desk = null
    this.activeTab = 'desk'
    this.loadTab()
  }

  async openEditor(node = null) {
    if (node) {
      this.editorMode = 'edit'
      // Load tags and sections for this node
      let nodeTags = []
      let nodeSections = []
      try {
        const desk = await api(`/nodes/${node.id}/desk`)
        nodeTags = (desk.tags || []).map(t => t.name)
        nodeSections = (desk.sections || []).map(s => s.id)
      } catch {}
      this.editingNode = {
        ...node,
        fields: node.fields || {},
        tags: nodeTags,
        section_ids: nodeSections
      }
    } else {
      this.editorMode = 'create'
      this.editingNode = { title: '', content: '', fields: {}, template_id: '', tags: [], section_ids: [] }
    }
    this.activeTab = 'editor'
    this.loadTab()
  }

  async saveNode() {
    if (!this.editingNode?.title?.trim()) return
    try {
      const hasFields = this.editingNode.fields && Object.keys(this.editingNode.fields).length > 0
      const body = {
        title: this.editingNode.title,
        content: hasFields ? null : (this.editingNode.content || null),
        fields: hasFields ? this.editingNode.fields : null,
        template_id: this.editingNode.template_id || null,
        tag_names: this.editingNode.tags || [],
        section_ids: this.editingNode.section_ids || []
      }

      let result
      if (this.editorMode === 'edit' && this.editingNode.id) {
        result = await api(`/nodes/${this.editingNode.id}`, { method: 'PUT', body: JSON.stringify(body) })
      } else {
        result = await api('/nodes', { method: 'POST', body: JSON.stringify(body) })
      }

      // Open the saved node in desk
      this._hasUnsavedChanges = false
      this.showToast('Fiche enregistrée', 'success')
      if (result?.id) {
        this.openDesk(result.id)
      }
    } catch (err) {
      console.error('[library] Save error:', err)
      this.showToast('Erreur lors de la sauvegarde', 'error')
    }
  }

  async deleteNode(nodeId) {
    if (!confirm('Supprimer cette fiche ?')) return
    try {
      await api(`/nodes/${nodeId}`, { method: 'DELETE' })
      this.showToast('Fiche supprimée', 'success')
      this.activeTab = 'library'
      this.loadTab()
    } catch (err) {
      console.error('[library] Delete error:', err)
    }
  }

  async confirmLink(pendingId) {
    try {
      await api(`/pending-links/${pendingId}/confirm`, { method: 'POST', body: JSON.stringify({ relation: null }) })
      this.showToast('Lien confirmé', 'success')
      this.loadTab()
    } catch (err) {
      console.error('[library] Confirm link error:', err)
    }
  }

  async dismissLink(pendingId) {
    try {
      await api(`/pending-links/${pendingId}/dismiss`, { method: 'POST' })
      this.showToast('Lien ignoré', 'info')
      this.loadTab()
    } catch (err) {
      console.error('[library] Dismiss link error:', err)
    }
  }

  async doSearch() {
    if (!this.searchQuery.trim()) return
    try {
      this.loading = true
      const data = await api(`/search?q=${encodeURIComponent(this.searchQuery)}`)
      this.searchResults = data.nodes || []
    } catch (err) {
      console.error('[library] Search error:', err)
      this.searchResults = []
    } finally {
      this.loading = false
    }
  }

  async saveTemplate() {
    if (!this.editingTemplate?.name?.trim()) return
    try {
      const body = {
        name: this.editingTemplate.name,
        structure: this.editingTemplate.structure || null,
        preview_css: this.editingTemplate.preview_css || null,
        preview_html: this.editingTemplate.preview_html || null
      }
      if (this.editingTemplate.id) {
        await api(`/templates/${this.editingTemplate.id}`, { method: 'PUT', body: JSON.stringify(body) })
      } else {
        await api('/templates', { method: 'POST', body: JSON.stringify(body) })
      }
      this.editingTemplate = null
      this.showToast('Patron enregistré', 'success')
      this.loadTab()
    } catch (err) {
      console.error('[library] Save template error:', err)
    }
  }

  async deleteTemplate(id) {
    if (!confirm('Supprimer ce patron ?')) return
    try {
      await api(`/templates/${id}`, { method: 'DELETE' })
      this.loadTab()
    } catch (err) {
      console.error('[library] Delete template error:', err)
    }
  }

  async loadTrash() {
    try {
      const data = await api('/trash')
      this.trash = data.trash || []
      this.showTrash = true
    } catch (err) {
      console.error('[library] Trash error:', err)
    }
  }

  async restoreNode(id) {
    try {
      await api(`/trash/${id}/restore`, { method: 'POST' })
      this.showToast('Fiche restaurée', 'success')
      this.loadTrash()
      this.loadTab()
    } catch (err) {
      console.error('[library] Restore error:', err)
    }
  }

  async purgeNode(id) {
    if (!confirm('Supprimer definitivement ? Cette action est irreversible.')) return
    try {
      await api(`/trash/${id}/purge`, { method: 'DELETE' })
      this.loadTrash()
    } catch (err) {
      console.error('[library] Purge error:', err)
    }
  }

  // ── Template rendering helpers ──

  formatFieldValue(value) {
    if (Array.isArray(value)) return value.join(', ')
    if (typeof value === 'number') return String(value)
    return value || ''
  }

  getTemplateForNode(node) {
    if (!node?.template_id) return null
    return this.templates.find(t => t.id === node.template_id) || null
  }

  renderTemplateHtml(node, fields, template) {
    if (!template?.preview_html || !fields) return null
    let compiled = template.preview_html

    // Replace field placeholders
    for (const [key, value] of Object.entries(fields)) {
      if (key === 'associations' && Array.isArray(value)) {
        const tags = value.map(a => `<span class="assoc-tag">${this._esc(a)}</span>`).join('')
        compiled = compiled.replaceAll('{{associations_tags}}', tags)
      }
      if (key === 'intensite') {
        const numVal = typeof value === 'number' ? value : parseInt(value, 10)
        if (!isNaN(numVal)) {
          const clamped = Math.max(0, Math.min(5, numVal))
          const dots = Array.from({ length: 5 }, (_, i) =>
            `<span class="dot ${i < clamped ? 'filled' : 'empty'}"></span>`
          ).join('')
          compiled = compiled.replaceAll('{{intensite_dots}}', dots)
          const labels = ['', 'Très doux', 'Doux', 'Moyen', 'Intense', 'Très intense']
          compiled = compiled.replaceAll('{{intensite_label}}', labels[clamped] || '')
        }
      }
      const escaped = typeof value === 'string' ? this._esc(value) : String(value)
      compiled = compiled.replaceAll(`{{${key}}}`, escaped)
    }

    // Replace title
    compiled = compiled.replaceAll('{{title}}', this._esc(node.title))
    // Clean remaining placeholders
    compiled = compiled.replace(/\{\{[^}]+\}\}/g, '')

    // Sanitize final HTML
    return DOMPurify.sanitize(compiled, {
      ALLOWED_TAGS: ['div', 'span', 'p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
                      'strong', 'em', 'b', 'i', 'u', 'br', 'hr', 'ul', 'ol', 'li',
                      'table', 'tr', 'td', 'th', 'thead', 'tbody', 'img', 'a',
                      'section', 'header', 'footer', 'main', 'article'],
      ALLOWED_ATTR: ['class', 'style', 'src', 'alt', 'href', 'target', 'rel'],
      ALLOW_DATA_ATTR: false
    })
  }

  _getSelectedTemplate() {
    if (!this.editingNode?.template_id) return null
    return this.templates.find(t => t.id === this.editingNode.template_id) || null
  }

  renderFieldInputs() {
    const tpl = this._getSelectedTemplate()
    if (!tpl?.structure || !Array.isArray(tpl.structure)) {
      return html`<div class="empty" style="padding: 1rem;">Ce patron n'a pas de structure définie.</div>`
    }

    const fields = this.editingNode.fields || {}

    return html`
      <div style="border: 1px solid var(--border-default); border-radius: var(--radius-lg); padding: 1rem;">
        <div style="font-size: 0.8em; color: var(--color-dark-text-secondary); margin-bottom: 0.8rem; text-transform: uppercase; letter-spacing: 0.5px;">
          Champs du patron : ${tpl.name}
        </div>
        ${tpl.structure.map(field => {
          const name = field.name
          const label = field.label || name
          const type = field.type || 'text'
          const currentVal = fields[name]

          if (type === 'number') {
            return html`
              <div class="form-field">
                <label>${label}</label>
                <input type="number" .value=${currentVal != null ? String(currentVal) : ''}
                  @input=${(e) => this._updateField(name, e.target.value ? Number(e.target.value) : null)}>
              </div>
            `
          }
          if (type === 'array') {
            return html`
              <div class="form-field">
                <label>${label} (valeurs séparées par des virgules)</label>
                <input type="text" .value=${Array.isArray(currentVal) ? currentVal.join(', ') : (currentVal || '')}
                  @input=${(e) => {
                    const arr = e.target.value.split(',').map(v => v.trim()).filter(Boolean)
                    this._updateField(name, arr)
                  }}>
              </div>
            `
          }
          if (type === 'textarea') {
            return html`
              <div class="form-field">
                <label>${label}</label>
                <textarea style="min-height: 80px;" .value=${currentVal || ''}
                  @input=${(e) => this._updateField(name, e.target.value)}></textarea>
              </div>
            `
          }
          // Default: text input
          return html`
            <div class="form-field">
              <label>${label}</label>
              <input type="text" .value=${currentVal || ''}
                @input=${(e) => this._updateField(name, e.target.value)}>
            </div>
          `
        })}
      </div>
    `
  }

  _updateField(name, value) {
    const fields = { ...(this.editingNode.fields || {}), [name]: value }
    this.editingNode = { ...this.editingNode, fields }
    this._hasUnsavedChanges = true
  }

  _esc(str) {
    const el = document.createElement('span')
    el.textContent = str
    return el.innerHTML
  }

  _sanitizeCSS(cssText) {
    if (!cssText) return ''
    return cssText
      .replace(/@import\b[^;]*;?/gi, '/* import blocked */')
      .replace(/url\s*\([^)]*\)/gi, '/* url blocked */')
      .replace(/expression\s*\([^)]*\)/gi, '/* expression blocked */')
      .replace(/javascript\s*:/gi, '/* js blocked */')
  }

  _safeColor(color) {
    if (!color) return 'var(--context-primary, #00d4aa)'
    // Allow hex, rgb, hsl, named colors
    if (/^#([0-9a-f]{3,8})$/i.test(color)) return color
    if (/^(rgb|hsl)a?\([^)]+\)$/i.test(color)) return color
    if (/^[a-z]{3,20}$/i.test(color)) return color
    return 'var(--context-primary, #00d4aa)'
  }

  // ── Render ──

  render() {
    return html`
      <div class="page-wrap" role="dialog" aria-modal="true">
        <div class="page-header">
          <button class="back-btn" @click=${() => this.close()} aria-label="Retour">&#8592;</button>
          <h2>Bibliothèque de Connaissances</h2>
        </div>

        <div class="tabs">
          <button class="tab-btn ${this.activeTab === 'library' ? 'active' : ''}" @click=${() => this.setTab('library')}>Bibliothèque</button>
          <button class="tab-btn ${this.activeTab === 'desk' ? 'active' : ''}" @click=${() => this.setTab('desk')}>Bureau d'étude</button>
          <button class="tab-btn ${this.activeTab === 'editor' ? 'active' : ''}" @click=${() => this.activeTab === 'editor' ? null : this.openEditor()}>Éditeur</button>
          <button class="tab-btn ${this.activeTab === 'templates' ? 'active' : ''}" @click=${() => this.setTab('templates')}>Patrons</button>
          <button class="tab-btn ${this.activeTab === 'search' ? 'active' : ''}" @click=${() => this.setTab('search')}>Recherche</button>
        </div>

        ${this.loading ? html`<div class="empty">Chargement...</div>` : this.renderTab()}
      </div>

      ${this.toasts.length > 0 ? html`
        <div class="toast-container">
          ${this.toasts.map(t => html`
            <div class="toast toast-${t.type}">${t.message}</div>
          `)}
        </div>
      ` : ''}
    `
  }

  renderTab() {
    switch (this.activeTab) {
      case 'library': return this.renderLibrary()
      case 'desk': return this.renderDesk()
      case 'editor': return this.renderEditor()
      case 'templates': return this.renderTemplates()
      case 'search': return this.renderSearch()
      default: return ''
    }
  }

  // ── Tab 1: Library (sections grid) ──

  renderLibrary() {
    if (this.selectedSection) return this.renderSectionDetail()

    return html`
      <div class="actions-bar">
        <button class="btn-primary" @click=${() => this.openEditor()}>+ Nouvelle fiche</button>
        <button class="btn-secondary" @click=${() => this.loadTrash()}>Corbeille</button>
      </div>

      ${this.sections.length === 0 ? html`
        <div class="empty">Aucune section. Créez votre première fiche pour commencer.</div>
      ` : html`
        <div class="cards-grid" style="margin-top: 1rem;">
          ${(this.graphData?.sections || []).map(s => html`
            <div class="card section-card" style="--section-color: ${this._safeColor(s.section.color)}"
                 @click=${() => this.openSection(s.section)}>
              <div class="card-title">${s.section.name}</div>
              ${s.section.description ? html`<div class="card-meta">${s.section.description}</div>` : ''}
              <div class="section-count">${s.node_count} fiche${s.node_count !== 1 ? 's' : ''}</div>
            </div>
          `)}
        </div>
      `}

      ${this.showTrash ? this.renderTrashModal() : ''}
    `
  }

  renderSectionDetail() {
    return html`
      <div class="actions-bar">
        <button class="btn-secondary" @click=${() => this.closeSection()}>&#8592; Sections</button>
        <button class="btn-primary" @click=${() => this.openEditor()}>+ Nouvelle fiche</button>
      </div>
      <h3 style="margin: 1rem 0 0.5rem; color: ${this._safeColor(this.selectedSection.color)}">${this.selectedSection.name}</h3>

      ${this.sectionNodes.length === 0 ? html`
        <div class="empty">Aucune fiche dans cette section.</div>
      ` : html`
        <div class="cards-grid">
          ${this.sectionNodes.map(node => html`
            <div class="card" @click=${() => this.openDesk(node.id)}>
              <div class="card-title">${node.title}</div>
              <div class="card-meta">${node.updated_at?.slice(0, 10) || ''}</div>
              ${node.is_pinned ? html`<span class="tag">favori</span>` : ''}
            </div>
          `)}
        </div>
      `}
    `
  }

  renderTrashModal() {
    return html`
      <div style="margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--border-default);">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <h3>Corbeille</h3>
          <button class="btn-sm" @click=${() => { this.showTrash = false }}>Fermer</button>
        </div>
        ${this.trash.length === 0 ? html`<div class="empty">Corbeille vide</div>` : html`
          ${this.trash.map(node => html`
            <div class="pending-card">
              <div>
                <div style="font-weight: 500;">${node.title}</div>
                <div class="card-meta">Supprimé le ${node.deleted_at?.slice(0, 10) || ''}</div>
              </div>
              <div class="pending-actions">
                <button class="btn-sm btn-confirm" @click=${() => this.restoreNode(node.id)}>Restaurer</button>
                <button class="btn-sm btn-dismiss" @click=${() => this.purgeNode(node.id)}>Supprimer</button>
              </div>
            </div>
          `)}
        `}
      </div>
    `
  }

  // ── Tab 2: Study Desk ──

  renderDesk() {
    if (!this.desk) {
      return html`
        <div class="empty">
          <p>Aucune fiche active au bureau d'étude.</p>
          <p>Cliquez sur une fiche depuis la bibliothèque pour l'étudier.</p>
        </div>
      `
    }

    const fields = this.desk.fields || this.desk.node.fields || null
    const template = this.getTemplateForNode(this.desk.node)
    const compiledHtml = this.renderTemplateHtml(this.desk.node, fields, template)

    return html`
      <div class="desk-center">
        <div style="display: flex; justify-content: space-between; align-items: start;">
          <div>
            <h3 style="margin: 0;">${this.desk.node.title}</h3>
            <div class="tags-row">
              ${(this.desk.tags || []).map(t => html`<span class="tag">${t.name}</span>`)}
            </div>
            <div class="card-meta" style="margin-top: 0.3rem;">
              ${(this.desk.sections || []).map(s => s.name).join(' / ')} &middot;
              ${this.desk.versions_count} version${this.desk.versions_count !== 1 ? 's' : ''}
            </div>
          </div>
          <div class="actions-bar">
            <button class="btn-sm" @click=${() => this.openEditor(this.desk.node)}>Modifier</button>
            <button class="btn-sm btn-dismiss" @click=${() => this.deleteNode(this.desk.node.id)}>Supprimer</button>
          </div>
        </div>

        ${compiledHtml ? html`
          <div class="template-preview">
            ${template?.preview_css ? html`<style>${this._sanitizeCSS(template.preview_css)}</style>` : ''}
            <div class="tpl-scope">${unsafeHTML(compiledHtml)}</div>
          </div>
        ` : fields && Object.keys(fields).length > 0 ? html`
          <div class="content-preview">
            ${Object.entries(fields).map(([key, value]) => html`
              <div style="margin-bottom: 0.5rem;">
                <span style="color: var(--color-dark-text-secondary); font-size: 0.8em; text-transform: uppercase;">${key.replace(/_/g, ' ')}</span><br>
                <span>${this.formatFieldValue(value)}</span>
              </div>
            `)}
          </div>
        ` : this.desk.node.content ? html`
          <div class="content-preview">${this.desk.node.content}</div>
        ` : ''}
      </div>

      ${this.desk.connections.length > 0 ? html`
        <h4 style="margin: 0 0 0.5rem;">Connexions (${this.desk.connections.length})</h4>
        <div class="desk-connections">
          ${this.desk.connections.map(c => html`
            <div class="connection-card" @click=${() => this.openDesk(c.node.id)}>
              <div style="font-weight: 500;">${c.node.title}</div>
              ${c.edge.relation ? html`<span class="relation-badge">${c.edge.relation}</span>` : ''}
            </div>
          `)}
        </div>
      ` : ''}

      ${this.desk.versions_count > 0 ? html`
        <div style="margin-top: 1rem;">
          <button class="btn-sm" @click=${async () => {
            if (!this.showVersions) {
              try {
                const data = await api(`/nodes/${this.desk.node.id}/versions`)
                this._versions = data.versions || []
              } catch {}
            }
            this.showVersions = !this.showVersions
          }}>
            ${this.showVersions ? 'Masquer' : 'Voir'} les versions (${this.desk.versions_count})
          </button>
          ${this.showVersions && this._versions ? html`
            <div style="margin-top: 0.5rem;">
              ${this._versions.map(v => html`
                <div class="version-item">
                  <span>v${v.version_num}</span>
                  <span class="card-meta">${v.created_at?.slice(0, 16).replace('T', ' ') || ''}</span>
                </div>
              `)}
            </div>
          ` : ''}
        </div>
      ` : ''}

      ${this.pendingLinks.length > 0 ? html`
        <div class="pending-section">
          <h4 style="margin: 0 0 0.5rem; color: #fbbf24;">Liens suggérés (${this.pendingLinks.length})</h4>
          ${this.pendingLinks.map(pl => html`
            <div class="pending-card">
              <div>
                <span style="font-weight: 500;">${pl.occurrence || 'Lien détecté'}</span>
              </div>
              <div class="pending-actions">
                <button class="btn-sm btn-confirm" @click=${() => this.confirmLink(pl.id)}>Confirmer</button>
                <button class="btn-sm btn-dismiss" @click=${() => this.dismissLink(pl.id)}>Ignorer</button>
              </div>
            </div>
          `)}
        </div>
      ` : ''}
    `
  }

  // ── Tab 3: Editor ──

  renderEditor() {
    if (!this.editingNode) return html`<div class="empty">Aucune fiche en cours d'édition.</div>`

    return html`
      <div class="editor-wrap">
        <h3>${this.editorMode === 'edit' ? 'Modifier la fiche' : 'Nouvelle fiche'}${this._hasUnsavedChanges ? html`<span style="color: #fbbf24; font-size: 0.8em; margin-left: 0.5rem;">● non sauvegardé</span>` : ''}</h3>

        <div class="editor-form">
          <div class="form-field">
            <label>Titre</label>
            <input type="text" .value=${this.editingNode.title || ''}
              @input=${(e) => { this.editingNode = { ...this.editingNode, title: e.target.value }; this._hasUnsavedChanges = true }}>
          </div>

          <div class="form-row">
            <div class="form-field">
              <label>Patron (optionnel)</label>
              <select .value=${this.editingNode.template_id || ''}
                @change=${(e) => { this.editingNode = { ...this.editingNode, template_id: e.target.value }; this._hasUnsavedChanges = true }}>
                <option value="">Texte libre</option>
                ${this.templates.map(t => html`
                  <option value="${t.id}" ?selected=${this.editingNode.template_id === t.id}>${t.name}</option>
                `)}
              </select>
            </div>
            <div class="form-field">
              <label>Sections</label>
              <div style="display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 0.3rem 0;">
                ${this.sections.map(s => html`
                  <label style="display: flex; align-items: center; gap: 0.3rem; font-size: 0.85em; cursor: pointer;">
                    <input type="checkbox"
                      ?checked=${(this.editingNode.section_ids || []).includes(s.id)}
                      @change=${(e) => {
                        const ids = [...(this.editingNode.section_ids || [])]
                        if (e.target.checked) {
                          if (!ids.includes(s.id)) ids.push(s.id)
                        } else {
                          const idx = ids.indexOf(s.id)
                          if (idx > -1) ids.splice(idx, 1)
                        }
                        this.editingNode = { ...this.editingNode, section_ids: ids }
                        this._hasUnsavedChanges = true
                      }}>
                    ${s.name}
                  </label>
                `)}
              </div>
            </div>
          </div>

          <div class="form-field">
            <label>Tags (séparés par des virgules)</label>
            <input type="text" .value=${(this.editingNode.tags || []).join(', ')}
              @input=${(e) => {
                const tags = e.target.value.split(',').map(t => t.trim()).filter(Boolean)
                this.editingNode = { ...this.editingNode, tags }
                this._hasUnsavedChanges = true
              }}>
          </div>

          ${this._getSelectedTemplate() ? this.renderFieldInputs() : html`
            <div class="form-field">
              <label>Contenu</label>
              <textarea .value=${this.editingNode.content || ''}
                @input=${(e) => { this.editingNode = { ...this.editingNode, content: e.target.value }; this._hasUnsavedChanges = true }}
                placeholder="Contenu de la fiche (texte libre)"></textarea>
            </div>
          `}

          <div class="actions-bar">
            <button class="btn-primary" @click=${() => this.saveNode()}>
              ${this.editorMode === 'edit' ? 'Enregistrer' : 'Créer la fiche'}
            </button>
            <button class="btn-secondary" @click=${() => this.setTab('library')}>Annuler</button>
          </div>
        </div>
      </div>
    `
  }

  // ── Tab 4: Templates ──

  renderTemplates() {
    return html`
      <div class="actions-bar" style="margin-bottom: 1rem;">
        <button class="btn-primary" @click=${() => { this.editingTemplate = { name: '', structure: null, preview_css: '', preview_html: '' } }}>
          + Nouveau patron
        </button>
      </div>

      ${this.editingTemplate ? html`
        <div class="card" style="margin-bottom: 1rem;">
          <div class="editor-form">
            <div class="form-field">
              <label>Nom du patron</label>
              <input type="text" .value=${this.editingTemplate.name || ''}
                @input=${(e) => this.editingTemplate = { ...this.editingTemplate, name: e.target.value }}>
            </div>
            <div class="form-field">
              <label>Structure JSON (champs)</label>
              <textarea style="min-height: 120px; ${this.templateJsonError ? 'border-color: #ef4444;' : ''}"
                .value=${this.editingTemplate.structure ? JSON.stringify(this.editingTemplate.structure, null, 2) : ''}
                @input=${(e) => {
                  try {
                    const parsed = JSON.parse(e.target.value)
                    this.editingTemplate = { ...this.editingTemplate, structure: parsed }
                    this.templateJsonError = ''
                  } catch (err) {
                    this.templateJsonError = err.message
                  }
                }}
                placeholder='[{"name": "champ", "type": "text", "label": "Mon champ"}]'></textarea>
              ${this.templateJsonError ? html`
                <div style="color: #ef4444; font-size: 0.75em; margin-top: 0.3rem;">JSON invalide : ${this.templateJsonError}</div>
              ` : ''}
            </div>
            <div class="form-field">
              <label>HTML du patron (placeholders {{champ}})</label>
              <textarea style="min-height: 120px;"
                .value=${this.editingTemplate.preview_html || ''}
                @input=${(e) => this.editingTemplate = { ...this.editingTemplate, preview_html: e.target.value }}
                placeholder='<div class="card">{{title}} — {{champ1}}</div>'></textarea>
            </div>
            <div class="form-field">
              <label>CSS du patron</label>
              <textarea style="min-height: 80px;"
                .value=${this.editingTemplate.preview_css || ''}
                @input=${(e) => this.editingTemplate = { ...this.editingTemplate, preview_css: e.target.value }}></textarea>
            </div>
            <div class="actions-bar">
              <button class="btn-primary" @click=${() => this.saveTemplate()}>Enregistrer</button>
              <button class="btn-secondary" @click=${() => { this.editingTemplate = null }}>Annuler</button>
            </div>
          </div>
        </div>
      ` : ''}

      ${this.templates.length === 0 ? html`
        <div class="empty">Aucun patron. Le patron "Fiche Épice" sera créé automatiquement au premier lancement du plugin.</div>
      ` : html`
        <div class="cards-grid">
          ${this.templates.map(t => html`
            <div class="card template-card">
              <div class="card-title">${t.name}</div>
              ${t.structure ? html`
                <div class="template-structure">
                  ${(Array.isArray(t.structure) ? t.structure : []).map(f => f.label || f.name).join(', ')}
                </div>
              ` : ''}
              <div class="actions-bar" style="margin-top: 0.5rem;">
                <button class="btn-sm" @click=${(e) => { e.stopPropagation(); this.editingTemplate = { ...t } }}>Modifier</button>
                <button class="btn-sm btn-dismiss" @click=${(e) => { e.stopPropagation(); this.deleteTemplate(t.id) }}>Supprimer</button>
              </div>
            </div>
          `)}
        </div>
      `}
    `
  }

  // ── Tab 5: Search ──

  renderSearch() {
    return html`
      <div style="display: flex; gap: 0.5rem; margin-bottom: 1rem;">
        <div class="form-field" style="flex: 1; margin-bottom: 0;">
          <input type="text" placeholder="Rechercher dans la bibliothèque..."
            style="font-size: 1.1em; padding: 0.8rem 1rem;"
            .value=${this.searchQuery}
            @input=${(e) => this.searchQuery = e.target.value}
            @keydown=${(e) => { if (e.key === 'Enter') this.doSearch() }}>
        </div>
        <button class="btn-primary" style="align-self: flex-end; white-space: nowrap;" @click=${() => this.doSearch()}>Rechercher</button>
      </div>

      ${this.searchResults.length > 0 ? html`
        <p style="color: var(--color-dark-text-secondary); font-size: 0.85em; margin-bottom: 0.5rem;">
          ${this.searchResults.length} résultat${this.searchResults.length !== 1 ? 's' : ''}
        </p>
        <div class="cards-grid">
          ${this.searchResults.map(node => html`
            <div class="card" @click=${() => this.openDesk(node.id)}>
              <div class="card-title">${node.title}</div>
              <div class="card-meta">${node.updated_at?.slice(0, 10) || ''}</div>
              ${node.fields && Object.keys(node.fields).length > 0 ? html`
                <div style="margin-top: 0.3rem; font-size: 0.75em; color: var(--color-dark-text-tertiary);">
                  ${Object.entries(node.fields).slice(0, 3).map(([k, v]) => html`
                    <span style="margin-right: 0.5rem;">${k.replace(/_/g, ' ')}: ${this.formatFieldValue(v).substring(0, 30)}</span>
                  `)}
                </div>
              ` : ''}
            </div>
          `)}
        </div>
      ` : this.searchQuery ? html`
        <div class="empty">Aucun résultat pour "${this.searchQuery}"</div>
      ` : html`
        <div class="empty">
          <p>Recherche plein texte sur les titres et contenus</p>
        </div>
      `}
    `
  }
}

customElements.define('library-page', LibraryPage)
