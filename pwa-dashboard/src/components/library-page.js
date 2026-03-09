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
    selectedAisle: { type: Object },  // allée (section racine)
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
    /* ── Library Theme: Warm wood / paper / book cover ── */
    :host {
      --lib-bg: #1c1710;
      --lib-surface: #261f16;
      --lib-surface-hover: #302820;
      --lib-surface-raised: #352d22;
      --lib-border: #4a3d2e;
      --lib-border-subtle: #3a3025;
      --lib-accent: #c8a46e;
      --lib-accent-hover: #d4b07a;
      --lib-accent-dim: #9a7d52;
      --lib-text: #e8dcc8;
      --lib-text-secondary: #b8a88e;
      --lib-text-muted: #7a6b55;
      --lib-paper: #2a2218;
      --lib-paper-light: #332a1e;
      --lib-leather: #5c3d20;
      --lib-leather-dark: #3a2510;
      --lib-green: #6b8f5e;
      --lib-green-dim: rgba(107, 143, 94, 0.15);
      --lib-red: #c06050;
      --lib-red-dim: rgba(192, 96, 80, 0.15);
      --lib-amber: #c89a40;
      --lib-amber-dim: rgba(200, 154, 64, 0.15);

      position: fixed;
      top: 0; left: 0; right: 0; bottom: 0;
      z-index: 1000;
      background: var(--lib-bg);
      background-image:
        radial-gradient(ellipse at 20% 0%, rgba(90, 60, 30, 0.12) 0%, transparent 60%),
        radial-gradient(ellipse at 80% 100%, rgba(60, 40, 20, 0.08) 0%, transparent 50%);
      overflow-y: auto;
      animation: slideUp 0.3s ease-out;
      color: var(--lib-text);
      font-family: 'Georgia', 'Times New Roman', serif;
    }

    .page-wrap {
      max-width: 1100px;
      margin: 0 auto;
      padding: 1.2rem 1.5rem 3rem;
    }

    .page-header {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 1.2rem;
    }

    .back-btn {
      background: var(--lib-surface);
      border: 1px solid var(--lib-border);
      color: var(--lib-accent);
      border-radius: 6px;
      padding: 0.4rem 0.8rem;
      cursor: pointer;
      font-size: 1.1em;
      transition: all 0.2s;
    }

    .back-btn:hover {
      background: var(--lib-surface-hover);
      border-color: var(--lib-accent);
    }

    h2 {
      margin: 0;
      font-size: 1.4em;
      font-weight: 700;
      color: var(--lib-accent);
      letter-spacing: 0.02em;
    }

    h3 { color: var(--lib-text); font-family: 'Georgia', serif; }
    h4 { color: var(--lib-text-secondary); font-family: 'Georgia', serif; }

    /* ── Tabs (book spine style) ── */
    .tabs {
      display: flex;
      gap: 0.3rem;
      margin-bottom: 1.5rem;
      overflow-x: auto;
      padding-bottom: 0.3rem;
      border-bottom: 2px solid var(--lib-border-subtle);
    }

    .tab-btn {
      padding: 0.55rem 1.1rem;
      background: var(--lib-surface);
      border: 1px solid var(--lib-border-subtle);
      border-bottom: none;
      border-radius: 6px 6px 0 0;
      color: var(--lib-text-muted);
      font-size: 0.82em;
      font-weight: 500;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
      letter-spacing: 0.4px;
      text-transform: uppercase;
    }

    .tab-btn:hover {
      background: var(--lib-surface-hover);
      color: var(--lib-text-secondary);
    }

    .tab-btn.active {
      background: var(--lib-surface-raised);
      border-color: var(--lib-border);
      border-bottom: 2px solid var(--lib-accent);
      color: var(--lib-accent);
      font-weight: 600;
    }

    /* ── Cards (book covers / index cards) ── */
    .card {
      background: var(--lib-surface);
      border: 1px solid var(--lib-border-subtle);
      border-radius: 8px;
      padding: 1.1rem 1.2rem;
      margin-bottom: 0.8rem;
      cursor: pointer;
      transition: all 0.25s ease;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(200, 164, 110, 0.04);
    }

    .card:hover {
      border-color: var(--lib-accent-dim);
      transform: translateY(-2px);
      box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(200, 164, 110, 0.06);
      background: var(--lib-surface-hover);
    }

    .card-title {
      font-weight: 600;
      margin-bottom: 0.3rem;
      color: var(--lib-text);
      font-family: 'Georgia', serif;
      font-size: 1.02em;
    }

    .card-meta {
      font-size: 0.75em;
      color: var(--lib-text-muted);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
    }

    /* Grid */
    .cards-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 0.8rem;
    }

    /* Section cards (leather bookmark accent) */
    .section-card {
      border-left: 4px solid var(--section-color, var(--lib-leather));
    }

    .section-count {
      font-size: 0.78em;
      color: var(--lib-text-muted);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
    }

    /* ── Study Desk (wooden desk surface) ── */
    .desk-center {
      background: linear-gradient(170deg, var(--lib-paper-light) 0%, var(--lib-paper) 100%);
      border: 1px solid var(--lib-border);
      border-top: 3px solid var(--lib-accent-dim);
      border-radius: 10px;
      padding: 1.6rem;
      margin-bottom: 1.5rem;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(200, 164, 110, 0.06);
    }

    .desk-connections {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
      gap: 0.6rem;
    }

    .connection-card {
      background: var(--lib-surface);
      border: 1px solid var(--lib-border-subtle);
      border-radius: 8px;
      padding: 0.8rem 1rem;
      cursor: pointer;
      transition: all 0.2s;
      box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
    }

    .connection-card:hover {
      border-color: var(--lib-accent-dim);
      transform: translateY(-1px);
      background: var(--lib-surface-hover);
    }

    .relation-badge {
      display: inline-block;
      font-size: 0.7em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      padding: 0.15rem 0.45rem;
      background: rgba(200, 164, 110, 0.1);
      border: 1px solid rgba(200, 164, 110, 0.2);
      border-radius: 4px;
      color: var(--lib-accent);
      margin-top: 0.35rem;
    }

    /* ── Pending links ── */
    .pending-section {
      margin-top: 1.5rem;
      padding-top: 1rem;
      border-top: 1px solid var(--lib-border-subtle);
    }

    .pending-card {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: var(--lib-surface);
      border: 1px solid rgba(200, 154, 64, 0.3);
      border-left: 3px solid var(--lib-amber);
      border-radius: 8px;
      padding: 0.7rem 1rem;
      margin-bottom: 0.5rem;
      box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
    }

    .pending-actions {
      display: flex;
      gap: 0.4rem;
    }

    .btn-sm {
      padding: 0.35rem 0.7rem;
      font-size: 0.75em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      border-radius: 5px;
      border: 1px solid var(--lib-border);
      cursor: pointer;
      background: var(--lib-surface);
      color: var(--lib-text);
      transition: all 0.2s;
    }

    .btn-sm:hover {
      background: var(--lib-surface-hover);
    }

    .btn-confirm { border-color: rgba(107, 143, 94, 0.5); color: var(--lib-green); }
    .btn-confirm:hover { background: var(--lib-green-dim); }
    .btn-dismiss { border-color: rgba(192, 96, 80, 0.5); color: var(--lib-red); }
    .btn-dismiss:hover { background: var(--lib-red-dim); }

    /* ── Editor ── */
    .editor-wrap {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .editor-form {
      display: flex;
      flex-direction: column;
      gap: 0.9rem;
    }

    .form-field label {
      display: block;
      font-size: 0.72em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      color: var(--lib-text-secondary);
      margin-bottom: 0.35rem;
      text-transform: uppercase;
      letter-spacing: 0.6px;
    }

    .form-field input,
    .form-field textarea,
    .form-field select {
      width: 100%;
      padding: 0.65rem 0.9rem;
      background: var(--lib-paper);
      border: 1px solid var(--lib-border);
      border-radius: 6px;
      color: var(--lib-text);
      font-family: 'Georgia', serif;
      font-size: 0.9em;
      box-sizing: border-box;
      transition: border-color 0.2s;
    }

    .form-field input:focus,
    .form-field textarea:focus,
    .form-field select:focus {
      outline: none;
      border-color: var(--lib-accent);
      box-shadow: 0 0 0 2px rgba(200, 164, 110, 0.15);
    }

    .form-field textarea {
      min-height: 200px;
      resize: vertical;
      font-family: 'Georgia', serif;
      line-height: 1.6;
    }

    .form-row {
      display: flex;
      gap: 0.8rem;
    }

    .form-row > .form-field { flex: 1; }

    .btn-primary {
      padding: 0.65rem 1.5rem;
      background: linear-gradient(135deg, var(--lib-accent) 0%, var(--lib-accent-dim) 100%);
      border: none;
      border-radius: 6px;
      color: #1a1208;
      font-weight: 600;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      cursor: pointer;
      transition: all 0.2s;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
    }

    .btn-primary:hover {
      background: linear-gradient(135deg, var(--lib-accent-hover) 0%, var(--lib-accent) 100%);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }

    .btn-secondary {
      padding: 0.65rem 1.5rem;
      background: var(--lib-surface);
      border: 1px solid var(--lib-border);
      border-radius: 6px;
      color: var(--lib-text);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      cursor: pointer;
      transition: all 0.2s;
    }

    .btn-secondary:hover {
      background: var(--lib-surface-hover);
      border-color: var(--lib-accent-dim);
    }

    .btn-danger {
      padding: 0.65rem 1.5rem;
      background: var(--lib-red-dim);
      border: 1px solid rgba(192, 96, 80, 0.5);
      border-radius: 6px;
      color: var(--lib-red);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      cursor: pointer;
    }

    .btn-danger:hover {
      background: rgba(192, 96, 80, 0.25);
    }

    .actions-bar {
      display: flex;
      gap: 0.6rem;
      flex-wrap: wrap;
    }

    /* Template cards (book spine) */
    .template-card {
      border-left: 4px solid var(--lib-leather);
    }

    .template-structure {
      font-size: 0.75em;
      color: var(--lib-text-muted);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      margin-top: 0.3rem;
    }

    /* ── Tags (wax seal style) ── */
    .tags-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.3rem;
      margin-top: 0.4rem;
    }

    .tag {
      background: rgba(200, 164, 110, 0.1);
      color: var(--lib-accent);
      padding: 0.18rem 0.55rem;
      border-radius: 4px;
      font-size: 0.73em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      border: 1px solid rgba(200, 164, 110, 0.2);
    }

    /* Empty */
    .empty {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--lib-text-muted);
      font-style: italic;
    }

    /* Content preview (aged paper) */
    .content-preview {
      background: var(--lib-paper);
      border: 1px solid var(--lib-border-subtle);
      border-radius: 8px;
      padding: 1.2rem;
      margin-top: 1rem;
      white-space: pre-wrap;
      font-size: 0.88em;
      line-height: 1.6;
      max-height: 400px;
      overflow-y: auto;
      box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    /* ── Template preview container (display case) ── */
    .template-preview {
      margin-top: 1.2rem;
      padding: 2rem;
      background: linear-gradient(160deg, #1a1510 0%, #14100c 100%);
      border: 1px solid var(--lib-border);
      border-radius: 10px;
      box-shadow: 0 4px 24px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(200, 164, 110, 0.04);
      display: flex;
      justify-content: center;
      overflow: hidden;
    }

    .template-preview > style {
      display: none;
    }

    .tpl-scope {
      width: 100%;
      max-width: 560px;
    }

    /* Versions */
    .version-item {
      display: flex;
      justify-content: space-between;
      padding: 0.5rem 0;
      border-bottom: 1px solid var(--lib-border-subtle);
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
      border-radius: 6px;
      font-size: 0.85em;
      font-weight: 500;
      animation: slideUp 0.3s ease-out;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
      max-width: 350px;
    }

    .toast-success {
      background: rgba(107, 143, 94, 0.2);
      border: 1px solid rgba(107, 143, 94, 0.5);
      color: #6b8f5e;
    }

    .toast-error {
      background: rgba(192, 96, 80, 0.2);
      border: 1px solid rgba(192, 96, 80, 0.5);
      color: #c06050;
    }

    .toast-info {
      background: rgba(200, 164, 110, 0.2);
      border: 1px solid rgba(200, 164, 110, 0.5);
      color: var(--lib-accent, #c8a46e);
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
    this.selectedAisle = null
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

  openAisle(aisleData) {
    this.selectedAisle = aisleData
    this.selectedSection = null
    this.sectionNodes = []
  }

  closeAisle() {
    this.selectedAisle = null
    this.selectedSection = null
    this.sectionNodes = []
  }

  async openShelf(shelfData) {
    this.selectedSection = shelfData.section
    try {
      const data = await api(`/sections/${shelfData.section.id}/nodes`)
      this.sectionNodes = data.nodes || []
    } catch (err) {
      console.error('[library] Shelf nodes error:', err)
    }
  }

  closeShelf() {
    this.selectedSection = null
    this.sectionNodes = []
  }

  // Legacy compat
  async openSection(section) {
    // If it's a root section with children, treat as aisle
    const aisleData = (this.graphData?.sections || []).find(a => a.section.id === section.id)
    if (aisleData && aisleData.children?.length > 0) {
      this.openAisle(aisleData)
    } else {
      this.selectedSection = section
      try {
        const data = await api(`/sections/${section.id}/nodes`)
        this.sectionNodes = data.nodes || []
      } catch (err) {
        console.error('[library] Section nodes error:', err)
      }
    }
  }

  closeSection() {
    this.closeAisle()
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
          const filledStyle = 'display:inline-block;width:13px;height:13px;border-radius:50%;background:#c8a46e;border:1.5px solid #c8a46e;box-shadow:0 0 6px rgba(200,164,110,0.4);margin-right:4px;vertical-align:middle;'
          const emptyStyle = 'display:inline-block;width:13px;height:13px;border-radius:50%;background:rgba(200,164,110,0.1);border:1.5px solid #9a7d52;margin-right:4px;vertical-align:middle;'
          const dots = Array.from({ length: 5 }, (_, i) =>
            `<span style="${i < clamped ? filledStyle : emptyStyle}"></span>`
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
      ALLOWED_ATTR: ['class', 'style', 'src', 'alt', 'href', 'target', 'rel', 'data-icon'],
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

  _getHierarchicalSections() {
    const sections = this.sections || []
    const result = []
    const aisles = sections.filter(s => !s.parent_id)
    for (const aisle of aisles) {
      result.push({ section: aisle, isAisle: true })
      const shelves = sections.filter(s => s.parent_id === aisle.id)
      for (const shelf of shelves) {
        result.push({ section: shelf, isAisle: false })
      }
    }
    // Orphan sections (no parent but not an aisle in graph)
    const listed = new Set(result.map(r => r.section.id))
    for (const s of sections) {
      if (!listed.has(s.id)) result.push({ section: s, isAisle: false })
    }
    return result
  }

  _buildSectionPath(sections) {
    if (!sections || sections.length === 0) return ''
    // Build path: find parent sections to display "Allée › Étagère"
    const allSections = this.sections || []
    return sections.map(s => {
      const parent = allSections.find(p => p.id === s.parent_id)
      return parent ? `${parent.name} › ${s.name}` : s.name
    }).join(' · ')
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
    // 3 levels: Allées (root) → Étagères (children) → Fiches (nodes)
    if (this.selectedSection) return this.renderShelfDetail()
    if (this.selectedAisle) return this.renderAisleDetail()

    // Level 1: Allées (root sections — no parent)
    const aisles = (this.graphData?.sections || [])

    return html`
      <div class="actions-bar">
        <button class="btn-primary" @click=${() => this.openEditor()}>+ Nouvelle fiche</button>
        <button class="btn-secondary" @click=${() => this.loadTrash()}>Corbeille</button>
      </div>

      <div class="breadcrumb" style="margin: 0.8rem 0 0.5rem; font-size: 0.85em; color: var(--lib-text-secondary, #a09080);">
        Bibliothèque
      </div>

      ${aisles.length === 0 ? html`
        <div class="empty">Aucune allée. Créez votre première fiche pour commencer.</div>
      ` : html`
        <div class="cards-grid" style="margin-top: 0.5rem;">
          ${aisles.map(a => html`
            <div class="card section-card" style="--section-color: ${this._safeColor(a.section.color)}"
                 @click=${() => this.openAisle(a)}>
              <div class="card-title">${a.section.name}</div>
              ${a.section.description ? html`<div class="card-meta">${a.section.description}</div>` : ''}
              <div class="section-count">
                ${a.children?.length || 0} étagère${(a.children?.length || 0) !== 1 ? 's' : ''}
                · ${a.node_count} fiche${a.node_count !== 1 ? 's' : ''}
              </div>
            </div>
          `)}
        </div>
      `}

      ${this.showTrash ? this.renderTrashModal() : ''}
    `
  }

  // Level 2: Étagères inside an Allée
  renderAisleDetail() {
    const aisle = this.selectedAisle
    const children = aisle.children || []

    return html`
      <div class="actions-bar">
        <button class="btn-secondary" @click=${() => this.closeAisle()}>&#8592; Bibliothèque</button>
        <button class="btn-primary" @click=${() => this.openEditor()}>+ Nouvelle fiche</button>
      </div>

      <div class="breadcrumb" style="margin: 0.8rem 0 0.5rem; font-size: 0.85em; color: var(--lib-text-secondary, #a09080);">
        <span style="cursor:pointer; text-decoration: underline;" @click=${() => this.closeAisle()}>Bibliothèque</span>
        <span style="margin: 0 0.4rem;">›</span>
        <span style="color: ${this._safeColor(aisle.section.color)}">${aisle.section.name}</span>
      </div>

      ${children.length === 0 ? html`
        <div class="empty">Aucune étagère dans cette allée.</div>
      ` : html`
        <div class="cards-grid">
          ${children.map(shelf => html`
            <div class="card section-card" style="--section-color: ${this._safeColor(shelf.section.color || aisle.section.color)}"
                 @click=${() => this.openShelf(shelf)}>
              <div class="card-title">${shelf.section.name}</div>
              ${shelf.section.description ? html`<div class="card-meta">${shelf.section.description}</div>` : ''}
              <div class="section-count">${shelf.node_count} fiche${shelf.node_count !== 1 ? 's' : ''}</div>
            </div>
          `)}
        </div>
      `}
    `
  }

  // Level 3: Fiches inside a Shelf
  renderShelfDetail() {
    const aisle = this.selectedAisle
    const shelf = this.selectedSection

    return html`
      <div class="actions-bar">
        <button class="btn-secondary" @click=${() => this.closeShelf()}>&#8592; ${aisle?.section?.name || 'Allée'}</button>
        <button class="btn-primary" @click=${() => this.openEditor()}>+ Nouvelle fiche</button>
      </div>

      <div class="breadcrumb" style="margin: 0.8rem 0 0.5rem; font-size: 0.85em; color: var(--lib-text-secondary, #a09080);">
        <span style="cursor:pointer; text-decoration: underline;" @click=${() => this.closeAisle()}>Bibliothèque</span>
        <span style="margin: 0 0.4rem;">›</span>
        <span style="cursor:pointer; text-decoration: underline;" @click=${() => this.closeShelf()}>${aisle?.section?.name || ''}</span>
        <span style="margin: 0 0.4rem;">›</span>
        <span style="color: ${this._safeColor(shelf.color)}">${shelf.name}</span>
      </div>

      ${this.sectionNodes.length === 0 ? html`
        <div class="empty">Aucune fiche sur cette étagère.</div>
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
              ${this._buildSectionPath(this.desk.sections || [])} &middot;
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
              <label>Étagère</label>
              <div style="display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 0.3rem 0;">
                ${this._getHierarchicalSections().map(item => html`
                  <label style="display: flex; align-items: center; gap: 0.3rem; font-size: 0.85em; cursor: pointer; ${item.isAisle ? 'font-weight: 600; margin-top: 0.3rem;' : 'padding-left: 1rem;'}">
                    ${item.isAisle ? '' : html`
                      <input type="checkbox"
                        ?checked=${(this.editingNode.section_ids || []).includes(item.section.id)}
                        @change=${(e) => {
                          const ids = [...(this.editingNode.section_ids || [])]
                          if (e.target.checked) {
                            if (!ids.includes(item.section.id)) ids.push(item.section.id)
                          } else {
                            const idx = ids.indexOf(item.section.id)
                            if (idx > -1) ids.splice(idx, 1)
                          }
                          this.editingNode = { ...this.editingNode, section_ids: ids }
                          this._hasUnsavedChanges = true
                        }}>
                    `}
                    ${item.isAisle ? `📚 ${item.section.name}` : item.section.name}
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
