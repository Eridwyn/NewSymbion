/**
 * Library Page — Bibliothèque de Connaissances
 *
 * Architecture single-canvas :
 * - Header : recherche intégrée + bouton créer
 * - Gauche : arbre sections (sidebar collapsible)
 * - Centre : carte hexagonale (toujours visible)
 * - Droite : panel contextuel (fiche, éditeur, settings)
 * - Ctrl+K : command palette (recherche + actions)
 */

import { LitElement, html, svg, css } from 'lit'
import { unsafeHTML } from 'lit/directives/unsafe-html.js'
import DOMPurify from 'dompurify'
// html2pdf loaded dynamically in exportPdf() to avoid blocking initial load
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
    // Data
    sections: { type: Array },
    graphData: { type: Object },
    nodes: { type: Array },
    templates: { type: Array },
    tags: { type: Array },
    pendingLinks: { type: Array },
    trash: { type: Array },
    // Navigation
    selectedAisle: { type: Object },
    selectedSection: { type: Object },
    sectionNodes: { type: Array },
    // Panel
    panelMode: { type: String },  // null | 'desk' | 'edit' | 'create' | 'settings' | 'trash'
    desk: { type: Object },
    deskNodeId: { type: String },
    editingNode: { type: Object },
    editorMode: { type: String },
    editingTemplate: { type: Object },
    // Search
    showSearch: { type: Boolean },
    searchQuery: { type: String },
    searchResults: { type: Array },
    // UI state
    loading: { type: Boolean },
    showSidebar: { type: Boolean },
    toasts: { type: Array },
    showVersions: { type: Boolean },
    _hasUnsavedChanges: { type: Boolean },
    templateJsonError: { type: String },
    showTrash: { type: Boolean },
    settingsTab: { type: String },  // 'sections' | 'templates'
    editingSection: { type: Object },
    // Linking modal
    showLinkModal: { type: Boolean },
    linkSearchQuery: { type: String },
    linkSearchResults: { type: Array },
  }

  static styles = [sharedAnimations, pageTransitionStyles, overlayStyles, pageHeaderStyles, formInputStyles, btnStyles, css`
    /* ── Library Theme: Botanical Garden / Forest Library ── */
    :host {
      --lib-bg: #0d1712;
      --lib-surface: #131f1a;
      --lib-surface-hover: #1a2b24;
      --lib-surface-raised: #1e3028;
      --lib-border: #2d4a3a;
      --lib-border-subtle: #243830;
      --lib-accent: #6ecb8b;
      --lib-accent-hover: #85d9a0;
      --lib-accent-dim: #3d8a5c;
      --lib-text: #d8e8dc;
      --lib-text-secondary: #9ab8a4;
      --lib-text-muted: #5a7d68;
      --lib-paper: #111e18;
      --lib-paper-light: #182820;
      --lib-leather: #2a5040;
      --lib-leather-dark: #1a3828;
      --lib-green: #5cb87a;
      --lib-green-dim: rgba(92, 184, 122, 0.15);
      --lib-red: #d06858;
      --lib-red-dim: rgba(208, 104, 88, 0.15);
      --lib-amber: #d4a84a;
      --lib-amber-dim: rgba(212, 168, 74, 0.15);
      --lib-gold: #c8a46e;

      position: fixed;
      top: 0; left: 0; right: 0; bottom: 0;
      z-index: 1000;
      background: var(--lib-bg);
      background-image:
        radial-gradient(ellipse at 15% 10%, rgba(40, 100, 70, 0.12) 0%, transparent 55%),
        radial-gradient(ellipse at 85% 90%, rgba(30, 80, 55, 0.08) 0%, transparent 50%),
        radial-gradient(ellipse at 50% 50%, rgba(20, 60, 40, 0.05) 0%, transparent 70%);
      overflow: hidden;
      animation: slideUp 0.3s ease-out;
      color: var(--lib-text);
      font-family: 'Georgia', 'Times New Roman', serif;
    }

    /* ── Main Layout ── */
    .layout {
      display: flex;
      flex-direction: column;
      height: 100%;
    }

    .top-bar {
      display: flex;
      align-items: center;
      gap: 0.8rem;
      padding: 0.7rem 1rem;
      background: var(--lib-surface);
      border-bottom: 1px solid var(--lib-border-subtle);
      flex-shrink: 0;
    }

    .top-bar .back-btn {
      background: none;
      border: 1px solid var(--lib-border);
      color: var(--lib-accent);
      border-radius: 6px;
      padding: 0.35rem 0.7rem;
      cursor: pointer;
      font-size: 1em;
      transition: all 0.2s;
    }
    .top-bar .back-btn:hover { background: var(--lib-surface-hover); }

    .top-bar h2 {
      margin: 0;
      font-size: 1.1em;
      font-weight: 700;
      color: var(--lib-accent);
      white-space: nowrap;
    }

    .search-bar {
      flex: 1;
      max-width: 400px;
      position: relative;
    }

    .search-bar input {
      width: 100%;
      padding: 0.45rem 0.8rem 0.45rem 2rem;
      background: var(--lib-paper);
      border: 1px solid var(--lib-border);
      border-radius: 20px;
      color: var(--lib-text);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      font-size: 0.82em;
      box-sizing: border-box;
      transition: border-color 0.2s;
    }
    .search-bar input:focus { outline: none; border-color: var(--lib-accent); }
    .search-bar input::placeholder { color: var(--lib-text-muted); }
    .search-bar::before {
      content: '🔍';
      position: absolute;
      left: 0.65rem;
      top: 50%;
      transform: translateY(-50%);
      font-size: 0.75em;
      pointer-events: none;
    }

    .top-actions {
      display: flex;
      gap: 0.4rem;
      margin-left: auto;
    }

    .icon-btn {
      background: none;
      border: 1px solid var(--lib-border);
      color: var(--lib-text-secondary);
      border-radius: 6px;
      padding: 0.4rem 0.6rem;
      cursor: pointer;
      font-size: 0.85em;
      transition: all 0.2s;
    }
    .icon-btn:hover { background: var(--lib-surface-hover); color: var(--lib-accent); border-color: var(--lib-accent-dim); }

    .btn-create {
      background: linear-gradient(135deg, var(--lib-accent) 0%, var(--lib-accent-dim) 100%);
      border: none;
      color: #1a1208;
      border-radius: 6px;
      padding: 0.4rem 0.9rem;
      font-weight: 600;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      font-size: 0.82em;
      cursor: pointer;
      transition: all 0.2s;
    }
    .btn-create:hover { transform: translateY(-1px); }

    /* ── Content area ── */
    .content {
      display: flex;
      flex: 1;
      overflow: hidden;
    }

    /* ── Sidebar (section tree) ── */
    .sidebar {
      width: 200px;
      flex-shrink: 0;
      background: var(--lib-surface);
      border-right: 1px solid var(--lib-border-subtle);
      overflow-y: auto;
      padding: 0.6rem 0;
      transition: width 0.3s ease;
    }

    .sidebar.collapsed { width: 0; padding: 0; overflow: hidden; }

    .sidebar-item {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.45rem 0.8rem;
      cursor: pointer;
      font-size: 0.82em;
      color: var(--lib-text-secondary);
      transition: all 0.15s;
      border-left: 3px solid transparent;
    }
    .sidebar-item:hover { background: var(--lib-surface-hover); color: var(--lib-text); }
    .sidebar-item.active { background: rgba(110,203,139,0.08); color: var(--lib-accent); border-left-color: var(--lib-accent); }
    .sidebar-item.aisle { font-weight: 600; color: var(--lib-text); font-size: 0.85em; }
    .sidebar-item.shelf { padding-left: 1.6rem; font-size: 0.78em; }

    .sidebar-section {
      margin-top: 0.8rem;
      padding: 0 0.8rem;
      font-size: 0.65em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      text-transform: uppercase;
      letter-spacing: 0.15em;
      color: var(--lib-text-muted);
      margin-bottom: 0.3rem;
    }

    /* ── Main canvas (hex map) ── */
    .canvas {
      flex: 1;
      overflow: auto;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
    }

    .canvas svg { max-height: 100%; }

    .canvas svg polygon, .canvas svg circle { transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1); }

    .hex-aisle { cursor: pointer; }
    .hex-aisle:hover polygon {
      filter: drop-shadow(0 0 14px var(--hover-glow, rgba(110, 203, 139, 0.45)));
      fill-opacity: 0.2 !important;
    }
    .hex-aisle:hover .hex-label { fill: #e0f0e4 !important; }
    .hex-aisle:hover .hex-count { fill: #b0d8b8 !important; }
    .hex-aisle:hover .hex-icon { opacity: 0.9 !important; }

    .hex-shelf { cursor: pointer; }
    .hex-shelf:hover polygon {
      filter: drop-shadow(0 0 10px var(--hover-glow, rgba(110, 203, 139, 0.35)));
      fill-opacity: 0.18 !important;
    }
    .hex-shelf:hover .hex-label { fill: #e0f0e4 !important; }

    .hex-center { cursor: pointer; }
    .hex-center:hover polygon {
      filter: drop-shadow(0 0 16px rgba(110, 203, 139, 0.5));
      fill: rgba(20, 50, 35, 0.95) !important;
    }

    @keyframes shelfAppear {
      from { opacity: 0; transform: translateY(8px) scale(0.8); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    .shelf-group { animation: shelfAppear 0.45s cubic-bezier(0.34, 1.56, 0.64, 1) both; }

    @keyframes pulseGlow {
      0%, 100% { opacity: 0.3; stroke-width: 0.5; }
      50% { opacity: 0.7; stroke-width: 1.2; }
    }
    .center-pulse { animation: pulseGlow 4s ease-in-out infinite; }

    @keyframes breatheGlow {
      0%, 100% { r: 65; opacity: 0.6; }
      50% { r: 72; opacity: 1; }
    }
    .center-aura { animation: breatheGlow 5s ease-in-out infinite; }

    @keyframes spinSlow {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }

    @keyframes flowDash {
      to { stroke-dashoffset: -24; }
    }
    .edge-flow {
      animation: flowDash 1.2s linear infinite;
    }

    @keyframes flowDashSlow {
      to { stroke-dashoffset: -32; }
    }
    .edge-flow-slow {
      animation: flowDashSlow 2s linear infinite;
    }

    /* Node list (when viewing shelf) */
    .node-list {
      position: absolute;
      bottom: 0; left: 0; right: 0;
      max-height: 40%;
      background: linear-gradient(to top, var(--lib-surface) 80%, transparent);
      padding: 1.2rem 1.5rem 1rem;
      overflow-y: auto;
      animation: slideUp 0.3s ease-out;
    }

    .node-list h4 {
      margin: 0 0 0.6rem;
      font-size: 0.9em;
      color: var(--lib-text-secondary);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
    }

    .node-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 0.4rem;
    }

    .node-chip {
      background: var(--lib-paper);
      border: 1px solid var(--lib-border);
      border-radius: 6px;
      padding: 0.4rem 0.8rem;
      font-size: 0.82em;
      cursor: pointer;
      transition: all 0.2s;
      color: var(--lib-text);
    }
    .node-chip:hover { border-color: var(--lib-accent); background: var(--lib-surface-hover); transform: translateY(-1px); }
    .node-chip .chip-meta { font-size: 0.7em; color: var(--lib-text-muted); margin-top: 0.15rem; }

    /* ── Right Panel (slide-in) ── */
    .panel-overlay {
      position: absolute;
      top: 0; right: 0; bottom: 0;
      width: min(480px, 100%);
      background: var(--lib-bg);
      border-left: 1px solid var(--lib-border);
      box-shadow: -4px 0 24px rgba(0, 0, 0, 0.4);
      overflow-y: auto;
      animation: panelSlideIn 0.3s ease-out;
      z-index: 10;
      display: flex;
      flex-direction: column;
    }

    @keyframes panelSlideIn {
      from { transform: translateX(100%); opacity: 0; }
      to { transform: translateX(0); opacity: 1; }
    }

    .panel-header {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 0.8rem 1rem;
      border-bottom: 1px solid var(--lib-border-subtle);
      flex-shrink: 0;
    }

    .panel-header h3 {
      margin: 0;
      flex: 1;
      font-size: 1em;
      color: var(--lib-text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .panel-close {
      background: none;
      border: none;
      color: var(--lib-text-muted);
      cursor: pointer;
      font-size: 1.2em;
      padding: 0.2rem 0.4rem;
      border-radius: 4px;
      transition: all 0.2s;
    }
    .panel-close:hover { color: var(--lib-text); background: var(--lib-surface-hover); }

    .panel-body {
      flex: 1;
      overflow-y: auto;
      padding: 1rem;
    }

    .panel-actions {
      display: flex;
      gap: 0.4rem;
      padding: 0.7rem 1rem;
      border-top: 1px solid var(--lib-border-subtle);
      flex-shrink: 0;
    }

    /* ── Panel content styles ── */
    .desk-meta {
      font-size: 0.78em;
      color: var(--lib-text-muted);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      margin-bottom: 0.6rem;
    }

    .tags-row { display: flex; flex-wrap: wrap; gap: 0.3rem; margin-bottom: 0.6rem; }

    .tag {
      background: rgba(110, 203, 139, 0.1);
      color: var(--lib-accent);
      padding: 0.18rem 0.55rem;
      border-radius: 4px;
      font-size: 0.73em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      border: 1px solid rgba(110, 203, 139, 0.2);
    }

    .template-preview {
      margin-top: 0.8rem;
      padding: 1.2rem;
      background: linear-gradient(160deg, #0f1a14 0%, #0a1410 100%);
      border: 1px solid var(--lib-border);
      border-radius: 8px;
      box-shadow: 0 4px 24px rgba(0, 0, 0, 0.35);
      overflow: hidden;
    }
    .template-preview > style { display: none; }
    .tpl-scope { max-width: 520px; margin: 0 auto; }

    .content-preview {
      background: var(--lib-paper);
      border: 1px solid var(--lib-border-subtle);
      border-radius: 8px;
      padding: 1rem;
      margin-top: 0.8rem;
      white-space: pre-wrap;
      font-size: 0.88em;
      line-height: 1.6;
      max-height: 300px;
      overflow-y: auto;
    }

    .connection-card {
      background: var(--lib-surface);
      border: 1px solid var(--lib-border-subtle);
      border-radius: 6px;
      padding: 0.5rem 0.7rem;
      display: flex; align-items: center; gap: 0.4rem;
      transition: all 0.2s;
      font-size: 0.85em;
    }
    .connection-card:hover { border-color: var(--lib-accent-dim); background: var(--lib-surface-hover); }
    .conn-title { cursor: pointer; flex: 1; }
    .conn-remove { cursor: pointer; opacity: 0; color: var(--lib-text-muted); font-size: 1.1em; transition: opacity 0.2s; }
    .connection-card:hover .conn-remove { opacity: 0.7; }
    .conn-remove:hover { opacity: 1 !important; color: #ef4444; }

    .relation-badge {
      font-size: 0.7em;
      padding: 0.1rem 0.35rem;
      background: rgba(110, 203, 139, 0.1);
      border: 1px solid rgba(110, 203, 139, 0.2);
      border-radius: 3px;
      color: var(--lib-accent);
    }

    /* ── Rating Editor ── */
    .rating-editor { display: flex; align-items: center; gap: 0.3rem; padding: 0.3rem 0; }
    .rating-dot { font-size: 1.2em; cursor: pointer; color: var(--lib-border); transition: color 0.15s, transform 0.15s; user-select: none; }
    .rating-dot.active { color: var(--lib-accent); text-shadow: 0 0 6px rgba(110, 203, 139, 0.4); }
    .rating-dot:hover { transform: scale(1.3); color: var(--lib-accent); }
    .rating-label { font-size: 0.75em; color: var(--lib-text-muted); margin-left: 0.3rem; }

    /* ── Tags Editor ── */
    .tags-editor { display: flex; flex-wrap: wrap; gap: 0.3rem; padding: 0.4rem; background: var(--lib-surface); border: 1px solid var(--lib-border); border-radius: 6px; min-height: 36px; }
    .tag-chip { display: inline-flex; align-items: center; gap: 0.25rem; background: rgba(110, 203, 139, 0.12); border: 1px solid rgba(110, 203, 139, 0.25); border-radius: 12px; padding: 0.15rem 0.5rem; font-size: 0.8em; color: var(--lib-accent); }
    .tag-remove { cursor: pointer; font-size: 1em; opacity: 0.6; transition: opacity 0.15s; }
    .tag-remove:hover { opacity: 1; color: #ef4444; }
    .tag-input { border: none; background: transparent; color: var(--lib-text); font-size: 0.8em; outline: none; min-width: 80px; flex: 1; padding: 0.15rem; }
    .tag-input::placeholder { color: var(--lib-text-muted); }

    /* ── Object Editor ── */
    .object-editor { display: flex; flex-direction: column; gap: 0.3rem; }
    .obj-row { display: flex; gap: 0.3rem; align-items: center; }
    .obj-key { width: 35%; font-size: 0.8em; padding: 0.3rem 0.4rem; background: var(--lib-surface); border: 1px solid var(--lib-border); border-radius: 4px; color: var(--lib-accent); }
    .obj-val { flex: 1; font-size: 0.8em; padding: 0.3rem 0.4rem; background: var(--lib-surface); border: 1px solid var(--lib-border); border-radius: 4px; color: var(--lib-text); }
    .obj-remove { cursor: pointer; color: var(--lib-text-muted); font-size: 1em; }
    .obj-remove:hover { color: #ef4444; }
    .btn-xs { font-size: 0.7em; padding: 0.2rem 0.6rem; background: transparent; border: 1px dashed var(--lib-border); border-radius: 4px; color: var(--lib-text-muted); cursor: pointer; }
    .btn-xs:hover { border-color: var(--lib-accent); color: var(--lib-accent); }

    /* ── Link Modal ── */
    .btn-link-add { font-size: 0.7em; padding: 0.2rem 0.6rem; background: rgba(110, 203, 139, 0.1); border: 1px solid rgba(110, 203, 139, 0.3); border-radius: 4px; color: var(--lib-accent); cursor: pointer; transition: all 0.15s; }
    .btn-link-add:hover { background: rgba(110, 203, 139, 0.2); }
    .link-modal { margin-top: 0.5rem; background: var(--lib-surface); border: 1px solid var(--lib-accent-dim); border-radius: 8px; padding: 0.5rem; }
    .link-search-input { width: 100%; box-sizing: border-box; padding: 0.4rem 0.6rem; background: var(--lib-bg); border: 1px solid var(--lib-border); border-radius: 4px; color: var(--lib-text); font-size: 0.85em; outline: none; }
    .link-search-input:focus { border-color: var(--lib-accent); }
    .link-results { max-height: 180px; overflow-y: auto; margin-top: 0.3rem; }
    .link-result-item { padding: 0.4rem 0.5rem; border-radius: 4px; cursor: pointer; font-size: 0.82em; display: flex; justify-content: space-between; align-items: center; transition: background 0.15s; }
    .link-result-item:hover { background: var(--lib-surface-hover); }

    .version-item {
      display: flex;
      justify-content: space-between;
      padding: 0.35rem 0;
      border-bottom: 1px solid var(--lib-border-subtle);
      font-size: 0.8em;
    }

    .pending-card {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: var(--lib-surface);
      border: 1px solid rgba(212, 168, 74, 0.3);
      border-left: 3px solid var(--lib-amber);
      border-radius: 6px;
      padding: 0.5rem 0.7rem;
      margin-bottom: 0.4rem;
      font-size: 0.85em;
    }

    /* ── Editor in panel ── */
    .editor-form { display: flex; flex-direction: column; gap: 0.7rem; }

    .form-field label {
      display: block;
      font-size: 0.7em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      color: var(--lib-text-secondary);
      margin-bottom: 0.25rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .form-field input,
    .form-field textarea,
    .form-field select {
      width: 100%;
      padding: 0.5rem 0.7rem;
      background: var(--lib-paper);
      border: 1px solid var(--lib-border);
      border-radius: 6px;
      color: var(--lib-text);
      font-family: 'Georgia', serif;
      font-size: 0.85em;
      box-sizing: border-box;
    }
    .form-field input:focus, .form-field textarea:focus, .form-field select:focus {
      outline: none;
      border-color: var(--lib-accent);
    }
    .form-field textarea { min-height: 100px; resize: vertical; }
    .form-row { display: flex; gap: 0.6rem; }
    .form-row > .form-field { flex: 1; }

    .btn-sm {
      padding: 0.3rem 0.6rem;
      font-size: 0.73em;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      border-radius: 5px;
      border: 1px solid var(--lib-border);
      cursor: pointer;
      background: var(--lib-surface);
      color: var(--lib-text);
      transition: all 0.2s;
    }
    .btn-sm:hover { background: var(--lib-surface-hover); }
    .btn-confirm { border-color: rgba(107, 143, 94, 0.5); color: var(--lib-green); }
    .btn-dismiss { border-color: rgba(192, 96, 80, 0.5); color: var(--lib-red); }

    .btn-primary {
      padding: 0.5rem 1.1rem;
      background: linear-gradient(135deg, var(--lib-accent) 0%, var(--lib-accent-dim) 100%);
      border: none;
      border-radius: 6px;
      color: #1a1208;
      font-weight: 600;
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      font-size: 0.82em;
      cursor: pointer;
      transition: all 0.2s;
    }
    .btn-primary:hover { transform: translateY(-1px); }

    .btn-secondary {
      padding: 0.5rem 1.1rem;
      background: var(--lib-surface);
      border: 1px solid var(--lib-border);
      border-radius: 6px;
      color: var(--lib-text);
      font-family: -apple-system, 'Helvetica Neue', Arial, sans-serif;
      font-size: 0.82em;
      cursor: pointer;
    }
    .btn-secondary:hover { background: var(--lib-surface-hover); }

    /* ── Search overlay (Ctrl+K) ── */
    .search-overlay {
      position: absolute;
      top: 0; left: 0; right: 0; bottom: 0;
      background: rgba(8, 16, 12, 0.75);
      z-index: 50;
      display: flex;
      justify-content: center;
      padding-top: 15vh;
      animation: fadeIn 0.15s ease-out;
    }

    @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

    .search-modal {
      background: var(--lib-surface);
      border: 1px solid var(--lib-border);
      border-radius: 12px;
      width: min(500px, 90%);
      max-height: 60vh;
      box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
      overflow: hidden;
      display: flex;
      flex-direction: column;
    }

    .search-modal input {
      width: 100%;
      padding: 1rem 1.2rem;
      background: transparent;
      border: none;
      border-bottom: 1px solid var(--lib-border-subtle);
      color: var(--lib-text);
      font-family: 'Georgia', serif;
      font-size: 1.1em;
      box-sizing: border-box;
    }
    .search-modal input:focus { outline: none; }

    .search-filters {
      display: flex; gap: 0.4rem; padding: 0.4rem 0.8rem; border-bottom: 1px solid var(--lib-border-subtle); flex-wrap: wrap;
    }
    .filter-select {
      flex: 1; min-width: 100px; padding: 0.3rem 0.4rem; background: var(--lib-bg); border: 1px solid var(--lib-border); border-radius: 4px; color: var(--lib-text); font-size: 0.75em;
    }

    .view-badge {
      font-size: 0.65em; padding: 0.1rem 0.35rem; background: rgba(110, 203, 139, 0.1); border: 1px solid rgba(110, 203, 139, 0.2); border-radius: 10px; color: var(--lib-accent); margin-left: 0.3rem; vertical-align: middle;
    }

    .search-results {
      overflow-y: auto;
      padding: 0.5rem;
    }

    .search-result-item {
      padding: 0.6rem 0.8rem;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.15s;
    }
    .search-result-item:hover { background: var(--lib-surface-hover); }
    .search-result-item .result-title { font-weight: 600; font-size: 0.9em; }
    .search-result-item .result-meta { font-size: 0.72em; color: var(--lib-text-muted); font-family: sans-serif; }

    /* ── Toasts ── */
    .toast-container {
      position: fixed;
      bottom: 1rem;
      right: 1rem;
      z-index: 9999;
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
    }
    .toast {
      padding: 0.6rem 1rem;
      border-radius: 6px;
      font-size: 0.82em;
      animation: slideUp 0.3s ease-out;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    }
    .toast-success { background: rgba(107,143,94,0.2); border: 1px solid rgba(107,143,94,0.5); color: #6b8f5e; }
    .toast-error { background: rgba(192,96,80,0.2); border: 1px solid rgba(192,96,80,0.5); color: #c06050; }
    .toast-info { background: rgba(200,164,110,0.2); border: 1px solid rgba(200,164,110,0.5); color: var(--lib-accent); }

    .empty {
      text-align: center;
      padding: 2rem 1rem;
      color: var(--lib-text-muted);
      font-style: italic;
      font-size: 0.9em;
    }

    /* ── Mobile ── */
    @media (max-width: 768px) {
      .sidebar { width: 0; padding: 0; overflow: hidden; }
      .sidebar.open { width: 220px; position: absolute; z-index: 20; height: 100%; box-shadow: 4px 0 20px rgba(0,0,0,0.5); }
      .panel-overlay { width: 100%; }
      .top-bar h2 { display: none; }
      .node-list { max-height: 50%; }
    }
  `]

  constructor() {
    super()
    this.sections = []
    this.graphData = null
    this.nodes = []
    this.templates = []
    this.tags = []
    this.pendingLinks = []
    this.trash = []
    this.selectedAisle = null
    this.selectedSection = null
    this.sectionNodes = []
    this.panelMode = null
    this.desk = null
    this.deskNodeId = null
    this.editingNode = null
    this.editorMode = 'create'
    this.editingTemplate = null
    this.showSearch = false
    this.searchQuery = ''
    this.searchResults = []
    this.loading = false
    this.showSidebar = window.innerWidth > 768
    this.toasts = []
    this.showVersions = false
    this._hasUnsavedChanges = false
    this._versions = []
    this.templateJsonError = ''
    this.showTrash = false
    this.settingsTab = 'sections'
    this.editingSection = null
    this._searchTimeout = null
    this.showLinkModal = false
    this.linkSearchQuery = ''
    this.linkSearchResults = []
  }

  connectedCallback() {
    super.connectedCallback()
    this._handleKeys = (e) => {
      if (e.key === 'Escape') {
        if (this.showSearch) { this.showSearch = false; return }
        if (this.panelMode) { this.closePanel(); return }
        this.close()
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault()
        this.showSearch = !this.showSearch
        if (this.showSearch) {
          setTimeout(() => this.shadowRoot?.querySelector('.search-modal input')?.focus(), 50)
        }
      }
    }
    document.addEventListener('keydown', this._handleKeys)
    this._handleBeforeUnload = (e) => {
      if (this._hasUnsavedChanges) { e.preventDefault(); e.returnValue = '' }
    }
    window.addEventListener('beforeunload', this._handleBeforeUnload)
    this.loadData()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this._handleKeys)
    window.removeEventListener('beforeunload', this._handleBeforeUnload)
  }

  close() {
    if (this._hasUnsavedChanges) {
      if (!confirm('Modifications non sauvegardées. Quitter ?')) return
    }
    this.dispatchEvent(new CustomEvent('close'))
  }

  showToast(message, type = 'info', duration = 3000) {
    const id = Date.now()
    this.toasts = [...this.toasts, { id, message, type }]
    setTimeout(() => { this.toasts = this.toasts.filter(t => t.id !== id) }, duration)
  }

  // ── Data Loading ──

  async loadData() {
    this.loading = true
    try {
      const [graphData, sectionsData, templatesData] = await Promise.all([
        api('/graph'),
        api('/sections'),
        api('/templates'),
      ])
      this.graphData = graphData
      this.sections = sectionsData.sections || []
      this.templates = templatesData.templates || []
    } catch (err) {
      console.error('[library] Load error:', err)
    } finally {
      this.loading = false
    }
  }

  async loadDeskNode(nodeId) {
    this.deskNodeId = nodeId
    this.desk = null
    this.panelMode = 'desk'
    try {
      this.desk = await api(`/nodes/${nodeId}/desk`)
      this.pendingLinks = (await api('/pending-links')).pending_links || []
    } catch (err) {
      console.error('[library] Desk error:', err)
    }
  }

  // ── Navigation ──

  async openAisle(aisleData) {
    this.selectedAisle = aisleData
    this.selectedSection = null
    this.sectionNodes = []
    // If no children (shelves), load nodes directly
    if (!aisleData.children || aisleData.children.length === 0) {
      this.selectedSection = aisleData.section
      try {
        const data = await api(`/sections/${aisleData.section.id}/nodes`)
        this.sectionNodes = data.nodes || []
      } catch (err) {
        console.error('[library] Aisle nodes error:', err)
      }
    }
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

  // ── Panel actions ──

  closePanel() {
    if (this._hasUnsavedChanges) {
      if (!confirm('Modifications non sauvegardées. Fermer ?')) return
    }
    this.panelMode = null
    this._hasUnsavedChanges = false
    this.editingNode = null
    this.editingTemplate = null
  }

  openCreate() {
    this.editorMode = 'create'
    this.editingNode = { title: '', content: '', fields: {}, template_id: '', tags: [], section_ids: [] }
    this._hasUnsavedChanges = false
    this.panelMode = 'create'
  }

  async openEdit(node = null) {
    const n = node || this.desk?.node
    if (!n) return
    this.editorMode = 'edit'
    let nodeTags = [], nodeSections = []
    try {
      const desk = await api(`/nodes/${n.id}/desk`)
      nodeTags = (desk.tags || []).map(t => t.name)
      nodeSections = (desk.sections || []).map(s => s.id)
    } catch {}
    this.editingNode = { ...n, fields: n.fields || {}, tags: nodeTags, section_ids: nodeSections }
    this._hasUnsavedChanges = false
    this.panelMode = 'edit'
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
      this._hasUnsavedChanges = false
      this.showToast('Fiche enregistrée', 'success')
      if (result?.id) this.loadDeskNode(result.id)
      this.loadData()
    } catch (err) {
      console.error('[library] Save error:', err)
      this.showToast('Erreur lors de la sauvegarde', 'error')
    }
  }

  async exportPdf() {
    if (!this.desk) return
    const fields = this.desk.fields || this.desk.node.fields || null
    const template = this.getTemplateForNode(this.desk.node)
    const compiledHtml = this.renderTemplateHtml(this.desk.node, fields, template)
    const cssText = template?.preview_css || ''
    const title = this.desk.node.title || 'Fiche'
    let bodyContent = ''
    if (compiledHtml) {
      bodyContent = compiledHtml
    } else if (fields && Object.keys(fields).length > 0) {
      bodyContent = `<div style="font-family:Georgia,serif;color:#d8e8dc;padding:1.5rem;">
        <h2 style="color:#6ecb8b;margin-bottom:1rem;">${title}</h2>
        ${Object.entries(fields).map(([key, value]) => {
          const display = Array.isArray(value) ? value.join(', ') : String(value || '')
          return `<div style="margin-bottom:0.6rem;"><span style="color:#3d8a5c;font-size:0.8em;text-transform:uppercase;">${key.replace(/_/g,' ')}</span><br><span>${display}</span></div>`
        }).join('')}</div>`
    } else {
      bodyContent = `<div style="font-family:Georgia,serif;color:#d8e8dc;padding:1.5rem;"><h2 style="color:#6ecb8b;">${title}</h2><p>${this.desk.node.content||''}</p></div>`
    }
    const container = document.createElement('div')
    container.style.cssText = 'position:absolute;left:0;top:0;z-index:99999;pointer-events:none;width:520px;'
    container.innerHTML = `<style>${cssText}</style><div class="tpl-scope" style="background:#0a1410;padding:2rem;width:480px;">${bodyContent}</div>`
    document.body.appendChild(container)
    await new Promise(r => setTimeout(r, 300))
    try {
      this.showToast('Génération du PDF...', 'info')
      const { default: html2pdf } = await import('html2pdf.js')
      await html2pdf().set({
        margin: 8,
        filename: `${title.replace(/[^a-zA-Z0-9àâéèêëïîôùûüÿçÀÂÉÈÊËÏÎÔÙÛÜŸÇ\s-]/g, '_')}.pdf`,
        image: { type: 'jpeg', quality: 0.95 },
        html2canvas: { scale: 2, useCORS: true, backgroundColor: '#0a1410' },
        jsPDF: { unit: 'mm', format: 'a5', orientation: 'portrait' },
      }).from(container.querySelector('.tpl-scope')).save()
      this.showToast('PDF exporté', 'success')
    } catch (err) {
      this.showToast('Erreur export PDF : ' + err.message, 'error')
    } finally {
      document.body.removeChild(container)
    }
  }

  async deleteNode(nodeId) {
    if (!confirm('Supprimer cette fiche ?')) return
    try {
      await api(`/nodes/${nodeId}`, { method: 'DELETE' })
      this.showToast('Fiche supprimée', 'success')
      this.panelMode = null
      this.desk = null
      this.loadData()
    } catch (err) { console.error('[library] Delete error:', err) }
  }

  async confirmLink(pendingId) {
    try {
      await api(`/pending-links/${pendingId}/confirm`, { method: 'POST', body: JSON.stringify({ relation: null }) })
      this.showToast('Lien confirmé', 'success')
      if (this.deskNodeId) this.loadDeskNode(this.deskNodeId)
    } catch (err) { console.error('[library] Confirm link error:', err) }
  }

  async dismissLink(pendingId) {
    try {
      await api(`/pending-links/${pendingId}/dismiss`, { method: 'POST' })
      this.showToast('Lien ignoré', 'info')
      if (this.deskNodeId) this.loadDeskNode(this.deskNodeId)
    } catch (err) { console.error('[library] Dismiss link error:', err) }
  }

  // ── Manual Linking ──

  _debounceLinkSearch() {
    clearTimeout(this._linkSearchTimeout)
    this._linkSearchTimeout = setTimeout(() => this._doLinkSearch(), 250)
  }

  async _doLinkSearch() {
    if (!this.linkSearchQuery.trim()) { this.linkSearchResults = []; return }
    try {
      const data = await api(`/search?q=${encodeURIComponent(this.linkSearchQuery)}`)
      this.linkSearchResults = data.nodes || []
    } catch { this.linkSearchResults = [] }
  }

  async _createLink(targetId) {
    if (!this.desk?.node?.id) return
    try {
      await api('/edges', { method: 'POST', body: JSON.stringify({ node_from: this.desk.node.id, node_to: targetId, relation: null }) })
      this.showToast('Lien créé', 'success')
      this.showLinkModal = false
      this.loadDeskNode(this.desk.node.id)
      this.loadGraphData()
    } catch (err) {
      this.showToast('Erreur création lien', 'error')
      console.error('[library] Create link error:', err)
    }
  }

  async _removeEdge(edgeId) {
    if (!confirm('Supprimer ce lien ?')) return
    try {
      await api(`/edges/${edgeId}`, { method: 'DELETE' })
      this.showToast('Lien supprimé', 'info')
      this.loadDeskNode(this.desk.node.id)
      this.loadGraphData()
    } catch (err) { console.error('[library] Remove edge error:', err) }
  }

  _templateName(templateId) {
    if (!templateId) return ''
    const t = this.templates.find(t => t.id === templateId)
    return t ? t.name : ''
  }

  async doSearch() {
    if (!this.searchQuery.trim()) return
    try {
      let url = `/search?q=${encodeURIComponent(this.searchQuery)}`
      if (this._searchSectionFilter) url += `&section_id=${this._searchSectionFilter}`
      if (this._searchTemplateFilter) url += `&template_id=${this._searchTemplateFilter}`
      const data = await api(url)
      let results = data.nodes || []
      // Client-side sort
      const sort = this._searchSort || 'relevance'
      if (sort === 'recent') results.sort((a, b) => (b.updated_at || '').localeCompare(a.updated_at || ''))
      else if (sort === 'views') results.sort((a, b) => (b.view_count || 0) - (a.view_count || 0))
      else if (sort === 'alpha') results.sort((a, b) => a.title.localeCompare(b.title))
      this.searchResults = results
    } catch (err) {
      this.searchResults = []
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
      this.templates = (await api('/templates')).templates || []
    } catch (err) { console.error('[library] Save template error:', err) }
  }

  async deleteTemplate(id) {
    if (!confirm('Supprimer ce patron ?')) return
    try {
      await api(`/templates/${id}`, { method: 'DELETE' })
      this.templates = (await api('/templates')).templates || []
    } catch (err) { console.error('[library] Delete template error:', err) }
  }

  // ── Section CRUD ──

  openCreateSection(parentId = null) {
    this.editingSection = { name: '', description: '', color: '#4ecb71', parent_id: parentId }
  }

  async saveSection() {
    if (!this.editingSection?.name?.trim()) return
    try {
      const body = {
        name: this.editingSection.name,
        description: this.editingSection.description || null,
        color: this.editingSection.color || null,
        parent_id: this.editingSection.parent_id || null,
      }
      if (this.editingSection.id) {
        await api(`/sections/${this.editingSection.id}`, { method: 'PUT', body: JSON.stringify(body) })
      } else {
        await api('/sections', { method: 'POST', body: JSON.stringify(body) })
      }
      this.editingSection = null
      this.showToast('Section enregistrée', 'success')
      const [sectionsData, graphData] = await Promise.all([api('/sections'), api('/graph')])
      this.sections = sectionsData.sections || []
      this.graphData = graphData
    } catch (err) {
      console.error('[library] Save section error:', err)
      this.showToast('Erreur sauvegarde section', 'error')
    }
  }

  async deleteSection(id) {
    if (!confirm('Supprimer cette section ? Les fiches ne seront pas supprimées.')) return
    try {
      await api(`/sections/${id}`, { method: 'DELETE' })
      this.showToast('Section supprimée', 'success')
      const [sectionsData, graphData] = await Promise.all([api('/sections'), api('/graph')])
      this.sections = sectionsData.sections || []
      this.graphData = graphData
    } catch (err) {
      console.error('[library] Delete section error:', err)
      this.showToast('Erreur suppression', 'error')
    }
  }

  async loadTrash() {
    try {
      const data = await api('/trash')
      this.trash = data.trash || []
      this.panelMode = 'trash'
    } catch (err) { console.error('[library] Trash error:', err) }
  }

  async restoreNode(id) {
    try {
      await api(`/trash/${id}/restore`, { method: 'POST' })
      this.showToast('Fiche restaurée', 'success')
      this.loadTrash()
      this.loadData()
    } catch (err) { console.error('[library] Restore error:', err) }
  }

  async purgeNode(id) {
    if (!confirm('Supprimer définitivement ?')) return
    try {
      await api(`/trash/${id}/purge`, { method: 'DELETE' })
      this.loadTrash()
    } catch (err) { console.error('[library] Purge error:', err) }
  }

  // ── Template rendering helpers ──

  formatFieldValue(value) {
    if (Array.isArray(value)) {
      if (value.length > 0 && typeof value[0] === 'object') return value.map(v => Object.values(v).join(' ')).join(', ')
      return value.join(', ')
    }
    if (value && typeof value === 'object') return Object.entries(value).map(([k, v]) => `${k}: ${v}`).join(', ')
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
    // Build field type lookup from template structure
    const fieldTypes = {}
    if (Array.isArray(template.structure)) {
      for (const f of template.structure) { if (f.name) fieldTypes[f.name] = f.type }
    }
    for (const [key, value] of Object.entries(fields)) {
      // Ingredients array (objects with nom/quantite)
      if (key === 'ingredients' && Array.isArray(value) && value.length && value[0]?.nom) {
        const html = value.map(i => `<div class="ing-item"><span class="ing-name">${this._esc(i.nom)}</span><span class="ing-qty">${this._esc(i.quantite || '')}</span></div>`).join('')
        compiled = compiled.replaceAll('{{ingredients_display}}', html)
        compiled = compiled.replaceAll(`{{${key}}}`, value.map(i => `${i.quantite} ${i.nom}`).join(', '))
        continue
      }
      // Etapes array (objects with num/titre/description)
      if (key === 'etapes' && Array.isArray(value) && value.length && value[0]?.titre) {
        const html = value.map(e => `<div class="step"><div class="step-title"><span class="step-num">${e.num || ''}</span>${this._esc(e.titre)}</div><div class="step-body">${this._esc(e.description || '')}</div></div>`).join('')
        compiled = compiled.replaceAll('{{etapes_display}}', html)
        compiled = compiled.replaceAll(`{{${key}}}`, value.map(e => `${e.num}. ${e.titre}`).join(', '))
        continue
      }
      // Ustensiles array (simple strings)
      if (key === 'ustensiles' && Array.isArray(value)) {
        compiled = compiled.replaceAll(`{{${key}_tags}}`, value.map(u => `<span class="ust-tag">${this._esc(u)}</span>`).join(''))
        compiled = compiled.replaceAll(`{{${key}}}`, value.join(', '))
        continue
      }
      // Infos object (portions, temps_prep, temps_cuisson, difficulte)
      if (key === 'infos' && typeof value === 'object' && !Array.isArray(value)) {
        const pills = []
        if (value.portions) pills.push(`<span class="info-pill">${this._esc(String(value.portions))} pers.</span>`)
        if (value.temps_prep) pills.push(`<span class="info-pill">Prep ${this._esc(value.temps_prep)}</span>`)
        if (value.temps_cuisson) pills.push(`<span class="info-pill">Cuisson ${this._esc(value.temps_cuisson)}</span>`)
        if (value.difficulte) pills.push(`<span class="info-pill">${this._esc(value.difficulte)}</span>`)
        compiled = compiled.replaceAll('{{infos_display}}', pills.join(''))
        compiled = compiled.replaceAll(`{{${key}}}`, Object.values(value).join(', '))
        continue
      }
      // Composition array (objects with epice/role/proportion)
      if (key === 'composition' && Array.isArray(value) && value.length && value[0]?.epice) {
        const html = value.map(c => {
          const pct = parseInt(c.proportion) || 0
          const barW = Math.max(8, pct * 1.2)
          const roleClass = c.role === 'dominante' ? ' dominante' : c.role === 'surprise' ? ' surprise' : ''
          return `<div class="compo-item"><div class="compo-bar" style="width:${barW}px"></div><span class="compo-name">${this._esc(c.epice)}</span><span class="compo-role${roleClass}">${this._esc(c.role || '')}</span><span class="compo-pct">${this._esc(c.proportion || '')}</span></div>`
        }).join('')
        compiled = compiled.replaceAll('{{composition_display}}', html)
        compiled = compiled.replaceAll(`{{${key}}}`, value.map(c => `${c.epice} (${c.proportion})`).join(', '))
        continue
      }
      // Array fields → render as tags (assoc-tag spans)
      if (Array.isArray(value)) {
        compiled = compiled.replaceAll(`{{${key}_tags}}`, value.map(a => `<span class="assoc-tag">${this._esc(a)}</span>`).join(''))
        compiled = compiled.replaceAll(`{{${key}}}`, value.map(a => this._esc(a)).join(', '))
        continue
      }
      // Intensite special handling (dots)
      if (key === 'intensite') {
        const numVal = typeof value === 'number' ? value : parseInt(value, 10)
        if (!isNaN(numVal)) {
          const clamped = Math.max(0, Math.min(5, numVal))
          const filledStyle = 'display:inline-block;width:13px;height:13px;border-radius:50%;background:#6ecb8b;border:1.5px solid #6ecb8b;box-shadow:0 0 6px rgba(110,203,139,0.4);margin-right:4px;vertical-align:middle;'
          const emptyStyle = 'display:inline-block;width:13px;height:13px;border-radius:50%;background:rgba(110,203,139,0.1);border:1.5px solid #3d8a5c;margin-right:4px;vertical-align:middle;'
          const dots = Array.from({ length: 5 }, (_, i) => `<span style="${i < clamped ? filledStyle : emptyStyle}"></span>`).join('')
          compiled = compiled.replaceAll('{{intensite_dots}}', dots)
          compiled = compiled.replaceAll('{{intensite_label}}', ['', 'Très doux', 'Doux', 'Moyen', 'Intense', 'Très intense'][clamped] || '')
        }
      }
      // Textarea fields with HTML content → inject raw (DOMPurify will sanitize at the end)
      const isHtmlContent = fieldTypes[key] === 'textarea' && typeof value === 'string' && value.includes('<')
      compiled = compiled.replaceAll(`{{${key}}}`, isHtmlContent ? value : (typeof value === 'string' ? this._esc(value) : String(value)))
    }
    compiled = compiled.replaceAll('{{title}}', this._esc(node.title)).replace(/\{\{[^}]+\}\}/g, '')
    return DOMPurify.sanitize(compiled, {
      ALLOWED_TAGS: ['div','span','p','h1','h2','h3','h4','h5','h6','strong','em','b','i','u','br','hr','ul','ol','li','table','tr','td','th','thead','tbody','img','a','section','header','footer','main','article'],
      ALLOWED_ATTR: ['class','style','src','alt','href','target','rel','data-icon'],
      ALLOW_DATA_ATTR: false
    })
  }

  _getSelectedTemplate() {
    if (!this.editingNode?.template_id) return null
    return this.templates.find(t => t.id === this.editingNode.template_id) || null
  }

  _updateField(name, value) {
    const fields = { ...(this.editingNode.fields || {}), [name]: value }
    this.editingNode = { ...this.editingNode, fields }
    this._hasUnsavedChanges = true
  }

  _esc(str) { const el = document.createElement('span'); el.textContent = str; return el.innerHTML }

  _sanitizeCSS(cssText) {
    if (!cssText) return ''
    return cssText.replace(/@import\b[^;]*;?/gi, '').replace(/url\s*\([^)]*\)/gi, '').replace(/expression\s*\([^)]*\)/gi, '').replace(/javascript\s*:/gi, '')
  }

  _safeColor(color) {
    if (!color) return 'var(--lib-accent)'
    if (/^#([0-9a-f]{3,8})$/i.test(color)) return color
    if (/^(rgb|hsl)a?\([^)]+\)$/i.test(color)) return color
    if (/^[a-z]{3,20}$/i.test(color)) return color
    return 'var(--lib-accent)'
  }

  _getHierarchicalSections() {
    const sections = this.sections || []
    const result = []
    for (const aisle of sections.filter(s => !s.parent_id)) {
      result.push({ section: aisle, isAisle: true })
      for (const shelf of sections.filter(s => s.parent_id === aisle.id)) {
        result.push({ section: shelf, isAisle: false })
      }
    }
    const listed = new Set(result.map(r => r.section.id))
    for (const s of sections) { if (!listed.has(s.id)) result.push({ section: s, isAisle: false }) }
    return result
  }

  _buildSectionPath(sections) {
    if (!sections?.length) return ''
    const all = this.sections || []
    return sections.map(s => {
      const parent = all.find(p => p.id === s.parent_id)
      return parent ? `${parent.name} › ${s.name}` : s.name
    }).join(' · ')
  }

  _hexPoints(cx, cy, r) {
    const pts = []
    for (let i = 0; i < 6; i++) {
      const angle = (Math.PI / 3) * i - Math.PI / 6
      pts.push(`${cx + r * Math.cos(angle)},${cy + r * Math.sin(angle)}`)
    }
    return pts.join(' ')
  }

  // ══════════════════════════════════════
  // ── RENDER
  // ══════════════════════════════════════

  render() {
    return html`
      <div class="layout">
        <!-- Top bar -->
        <div class="top-bar">
          <button class="back-btn" @click=${() => this.close()}>←</button>
          <button class="icon-btn" @click=${() => { this.showSidebar = !this.showSidebar }} title="Sections">☰</button>
          <h2>Bibliothèque</h2>

          <div class="search-bar">
            <input type="text" placeholder="Rechercher… (Ctrl+K)"
              @focus=${() => { this.showSearch = true }}
              @click=${() => { this.showSearch = true }}>
          </div>

          <div class="top-actions">
            <button class="icon-btn" @click=${() => this.loadTrash()} title="Corbeille">🗑</button>
            <button class="icon-btn" @click=${() => { this.panelMode = 'settings' }} title="Patrons">⚙</button>
            <button class="btn-create" @click=${() => this.openCreate()}>+ Nouvelle fiche</button>
          </div>
        </div>

        <!-- Content: sidebar + canvas + panel -->
        <div class="content">
          <!-- Sidebar: section tree -->
          <div class="sidebar ${this.showSidebar ? '' : 'collapsed'} ${this.showSidebar ? 'open' : ''}">
            <div class="sidebar-section">Allées</div>
            ${(this.graphData?.sections || []).map(a => html`
              <div class="sidebar-item aisle ${this.selectedAisle?.section?.id === a.section.id ? 'active' : ''}"
                   @click=${() => this.openAisle(a)}>
                <span style="color:${this._safeColor(a.section.color)}">●</span>
                ${a.section.name}
                <span style="margin-left:auto;font-size:0.7em;color:var(--lib-text-muted);">${a.node_count}</span>
              </div>
              ${this.selectedAisle?.section?.id === a.section.id ? (a.children || []).map(shelf => html`
                <div class="sidebar-item shelf ${this.selectedSection?.id === shelf.section.id ? 'active' : ''}"
                     @click=${() => this.openShelf(shelf)}>
                  ${shelf.section.name}
                  <span style="margin-left:auto;font-size:0.7em;color:var(--lib-text-muted);">${shelf.node_count}</span>
                </div>
              `) : ''}
            `)}
          </div>

          <!-- Main canvas -->
          <div class="canvas">
            ${this.loading ? html`<div class="empty">Chargement...</div>` : this.renderHexMap()}

            <!-- Node list overlay when viewing a shelf -->
            ${this.selectedSection && this.sectionNodes.length > 0 ? html`
              <div class="node-list">
                <h4>${this.selectedSection.name} — ${this.sectionNodes.length} fiche${this.sectionNodes.length !== 1 ? 's' : ''}</h4>
                <div class="node-chips">
                  ${this.sectionNodes.map(node => html`
                    <div class="node-chip" @click=${() => this.loadDeskNode(node.id)}>
                      <div>${node.title}</div>
                      <div class="chip-meta">${node.updated_at?.slice(0, 10) || ''}</div>
                    </div>
                  `)}
                </div>
              </div>
            ` : ''}

            <!-- Right panel -->
            ${this.panelMode ? html`
              <div class="panel-overlay">
                ${this.panelMode === 'desk' ? this.renderDeskPanel() : ''}
                ${this.panelMode === 'create' || this.panelMode === 'edit' ? this.renderEditorPanel() : ''}
                ${this.panelMode === 'settings' ? this.renderSettingsPanel() : ''}
                ${this.panelMode === 'trash' ? this.renderTrashPanel() : ''}
              </div>
            ` : ''}
          </div>
        </div>

        <!-- Search overlay (Ctrl+K) -->
        ${this.showSearch ? html`
          <div class="search-overlay" @click=${(e) => { if (e.target === e.currentTarget) this.showSearch = false }}>
            <div class="search-modal">
              <input type="text" placeholder="Rechercher dans la bibliothèque..."
                .value=${this.searchQuery}
                @input=${(e) => { this.searchQuery = e.target.value; clearTimeout(this._searchTimeout); this._searchTimeout = setTimeout(() => this.doSearch(), 300) }}
                @keydown=${(e) => {
                  if (e.key === 'Escape') this.showSearch = false
                  if (e.key === 'Enter' && this.searchResults.length > 0) {
                    this.loadDeskNode(this.searchResults[0].id)
                    this.showSearch = false
                  }
                }}>
              <!-- Search filters -->
              <div class="search-filters">
                <select class="filter-select" @change=${(e) => { this._searchTemplateFilter = e.target.value; this.doSearch() }}>
                  <option value="">Tous les patrons</option>
                  ${(this.templates || []).map(t => html`<option value="${t.id}">${t.name}</option>`)}
                </select>
                <select class="filter-select" @change=${(e) => { this._searchSectionFilter = e.target.value; this.doSearch() }}>
                  <option value="">Toutes sections</option>
                  ${(this.sections || []).map(s => html`<option value="${s.id}">${s.parent_id ? '  └ ' : ''}${s.name}</option>`)}
                </select>
                <select class="filter-select" @change=${(e) => { this._searchSort = e.target.value; this.doSearch() }}>
                  <option value="relevance">Pertinence</option>
                  <option value="recent">Plus récent</option>
                  <option value="views">Plus consulté</option>
                  <option value="alpha">A → Z</option>
                </select>
              </div>
              <div class="search-results">
                ${this.searchResults.length > 0 ? this.searchResults.map(node => {
                  const tpl = node.template_id ? this.templates.find(t => t.id === node.template_id) : null
                  const fields = node.fields && typeof node.fields === 'object' ? node.fields : null
                  const snippet = fields ? Object.values(fields).filter(v => typeof v === 'string').join(' · ').slice(0, 80) : (node.content || '').slice(0, 80)
                  return html`
                  <div class="search-result-item" @click=${() => { this.loadDeskNode(node.id); this.showSearch = false }}>
                    <div class="result-title">
                      ${node.title}
                      ${node.view_count > 0 ? html`<span class="view-badge" title="${node.view_count} consultation${node.view_count > 1 ? 's' : ''}">${node.view_count}×</span>` : ''}
                    </div>
                    <div class="result-meta">
                      ${tpl ? html`<span style="color:var(--lib-accent);margin-right:0.4rem;">${tpl.name}</span>` : ''}
                      ${node.updated_at?.slice(0, 10) || ''}
                    </div>
                    ${snippet ? html`<div style="font-size:0.7em;color:var(--lib-text-muted);margin-top:0.15rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${snippet}</div>` : ''}
                  </div>`
                }) : this.searchQuery ? html`
                  <div class="empty" style="padding:1rem;">Aucun résultat</div>
                ` : html`
                  <div style="padding:0.8rem; color:var(--lib-text-muted); font-size:0.8em;">
                    Tapez pour chercher · Entrée pour ouvrir · Échap pour fermer
                  </div>
                `}
              </div>
            </div>
          </div>
        ` : ''}
      </div>

      ${this.toasts.length > 0 ? html`
        <div class="toast-container">
          ${this.toasts.map(t => html`<div class="toast toast-${t.type}">${t.message}</div>`)}
        </div>
      ` : ''}
    `
  }

  // ── Hex Map ──

  renderHexMap() {
    const aisles = (this.graphData?.sections || [])
    const interEdges = this.graphData?.inter_section_edges || []
    const CX = 300, CY = 300
    const AISLE_R = 145, HEX_R = 55
    const SHELF_R = 240, SHELF_HEX_R = 38
    const activeAisle = this.selectedAisle
    const n = Math.max(aisles.length, 1)
    const totalNodes = aisles.reduce((s, a) => s + (a.node_count || 0), 0)
    const maxNodes = Math.max(1, ...aisles.map(a => a.node_count || 0))

    // Pre-compute aisle positions for inter-section edges
    const aislePos = aisles.map((a, i) => {
      const angle = (2 * Math.PI / n) * i - Math.PI / 2
      return { id: a.section.id, x: CX + AISLE_R * Math.cos(angle), y: CY + AISLE_R * Math.sin(angle), color: this._safeColor(a.section.color), section: a }
    })

    // Build inter-section edge data: find which aisles connect
    const interLinks = []
    for (const edge of interEdges) {
      // Find which top-level section each node belongs to
      const findAisle = (nodeId) => {
        for (const ap of aislePos) {
          const a = ap.section
          // Check direct nodes in this aisle
          if (a.node_ids?.includes(nodeId)) return ap
          // Check children sections
          for (const child of (a.children || [])) {
            if (child.node_ids?.includes(nodeId)) return ap
          }
        }
        return null
      }
      const from = findAisle(edge.node_from)
      const to = findAisle(edge.node_to)
      if (from && to && from.id !== to.id) {
        const key = [from.id, to.id].sort().join('-')
        const existing = interLinks.find(l => l.key === key)
        if (existing) { existing.count++ }
        else { interLinks.push({ key, from, to, count: 1 }) }
      }
    }

    return html`
      <svg viewBox="0 0 600 600" width="100%" style="max-width:600px;">
        <defs>
          <radialGradient id="centerGlow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stop-color="rgba(110,203,139,0.25)"/>
            <stop offset="40%" stop-color="rgba(110,203,139,0.08)"/>
            <stop offset="100%" stop-color="rgba(110,203,139,0)"/>
          </radialGradient>
          <radialGradient id="bgGlow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stop-color="rgba(40,100,70,0.06)"/>
            <stop offset="100%" stop-color="transparent"/>
          </radialGradient>

          <!-- Enhanced glow filter: double-layer for neon effect -->
          <filter id="neonGlow" x="-40%" y="-40%" width="180%" height="180%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="blur1"/>
            <feGaussianBlur in="SourceGraphic" stdDeviation="2" result="blur2"/>
            <feMerge>
              <feMergeNode in="blur1"/>
              <feMergeNode in="blur2"/>
              <feMergeNode in="SourceGraphic"/>
            </feMerge>
          </filter>
          <filter id="softGlow" x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="blur"/>
            <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
          <filter id="particleGlow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="2.5" result="blur"/>
            <feMerge><feMergeNode in="blur"/><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>

          <!-- Aisle-specific gradients -->
          ${aisles.map((a, i) => {
            const c = this._safeColor(a.section.color)
            return svg`
              <radialGradient id="aisleGlow${i}" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stop-color="${c}" stop-opacity="0.18"/>
                <stop offset="70%" stop-color="${c}" stop-opacity="0.04"/>
                <stop offset="100%" stop-color="${c}" stop-opacity="0"/>
              </radialGradient>`
          })}

          <!-- Edge gradients for center→aisle connections -->
          ${aislePos.map((ap, i) => svg`
            <linearGradient id="edgeGrad${i}" gradientUnits="userSpaceOnUse"
              x1="${CX}" y1="${CY}" x2="${ap.x}" y2="${ap.y}">
              <stop offset="0%" stop-color="rgba(110,203,139,0.1)"/>
              <stop offset="40%" stop-color="${ap.color}" stop-opacity="0.5"/>
              <stop offset="100%" stop-color="${ap.color}" stop-opacity="0.2"/>
            </linearGradient>
          `)}

          <!-- Inter-section edge gradients -->
          ${interLinks.map((link, i) => svg`
            <linearGradient id="interGrad${i}" gradientUnits="userSpaceOnUse"
              x1="${link.from.x}" y1="${link.from.y}" x2="${link.to.x}" y2="${link.to.y}">
              <stop offset="0%" stop-color="${link.from.color}" stop-opacity="0.6"/>
              <stop offset="50%" stop-color="#fff" stop-opacity="0.15"/>
              <stop offset="100%" stop-color="${link.to.color}" stop-opacity="0.6"/>
            </linearGradient>
          `)}
        </defs>

        <!-- Background glow -->
        <circle cx="${CX}" cy="${CY}" r="295" fill="url(#bgGlow)"/>

        <!-- Subtle orbital rings -->
        ${[80, AISLE_R, SHELF_R].map((r, ri) => svg`
          <circle cx="${CX}" cy="${CY}" r="${r}" fill="none"
            stroke="rgba(110,203,139,${ri === 1 ? 0.08 : 0.04})"
            stroke-width="${ri === 1 ? 0.8 : 0.5}"
            stroke-dasharray="${ri === 0 ? '2,8' : ri === 1 ? '4,8' : '3,10'}"/>
        `)}

        <!-- Radial web lines (very subtle) -->
        ${aisles.map((_, i) => {
          const angle = (2 * Math.PI / n) * i - Math.PI / 2
          const ex = CX + 275 * Math.cos(angle), ey = CY + 275 * Math.sin(angle)
          return svg`<line x1="${CX}" y1="${CY}" x2="${ex}" y2="${ey}" stroke="rgba(110,203,139,0.03)" stroke-width="0.5"/>`
        })}

        <!-- ═══ Inter-section edges (between aisles) ═══ -->
        ${interLinks.map((link, i) => {
          const mx = (link.from.x + link.to.x) / 2, my = (link.from.y + link.to.y) / 2
          const dx = link.to.x - link.from.x, dy = link.to.y - link.from.y
          // Curve outward from center for cleaner arc
          const distFromCenter = Math.sqrt((mx - CX) ** 2 + (my - CY) ** 2)
          const pushOut = distFromCenter < 60 ? 50 : 25
          const normX = (mx - CX) / (distFromCenter || 1), normY = (my - CY) / (distFromCenter || 1)
          const cx1 = mx + normX * pushOut, cy1 = my + normY * pushOut
          const pathD = `M${link.from.x},${link.from.y} Q${cx1},${cy1} ${link.to.x},${link.to.y}`
          const w = Math.min(3, 1 + link.count * 0.5)
          return svg`
            <!-- Base dim path -->
            <path d="${pathD}" fill="none" stroke="url(#interGrad${i})"
              stroke-width="${w}" stroke-linecap="round" opacity="0.5"/>
            <!-- Flowing energy dashes -->
            <path id="inter${i}" d="${pathD}" fill="none" stroke="url(#interGrad${i})"
              stroke-width="${w}" stroke-linecap="round"
              stroke-dasharray="6 18" class="edge-flow-slow" opacity="0.7"/>
            <!-- Animated particles along the link -->
            <circle r="2.5" fill="${link.from.color}" opacity="0.8" filter="url(#particleGlow)">
              <animateMotion dur="3s" begin="0s" repeatCount="indefinite">
                <mpath href="#inter${i}"/>
              </animateMotion>
            </circle>
            <circle r="1.8" fill="${link.to.color}" opacity="0.5" filter="url(#particleGlow)">
              <animateMotion dur="4s" begin="1.5s" repeatCount="indefinite">
                <mpath href="#inter${i}"/>
              </animateMotion>
            </circle>
            <!-- Edge count badge -->
            ${link.count > 1 ? svg`
              <text x="${cx1}" y="${cy1 - 6}" text-anchor="middle" fill="rgba(220,240,225,0.5)"
                font-size="7" font-family="sans-serif" style="pointer-events:none;">
                ${link.count} liens
              </text>
            ` : ''}
          `
        })}

        <!-- ═══ Center → Aisle connections ═══ -->
        ${aislePos.map((ap, i) => {
          const a = ap.section
          const active = activeAisle?.section?.id === a.section.id
          const color = ap.color
          const weight = (a.node_count || 0) / maxNodes
          const lineW = active ? 2.5 : (0.8 + weight * 1.5)
          const mx = (CX + ap.x) / 2, my = (CY + ap.y) / 2
          const perpX = -(ap.y - CY) * 0.1, perpY = (ap.x - CX) * 0.1
          const pathD = `M${CX},${CY} Q${mx + perpX},${my + perpY} ${ap.x},${ap.y}`
          return svg`
            <!-- Base connection path -->
            <path id="conn${i}" d="${pathD}" fill="none"
              stroke="${active ? color : `url(#edgeGrad${i})`}"
              stroke-width="${lineW}" stroke-linecap="round"
              opacity="${active ? 0.8 : 0.35}"
              ${active ? svg`filter="url(#softGlow)"` : ''}/>
            <!-- Animated energy flow -->
            <path d="${pathD}" fill="none" stroke="${color}"
              stroke-width="${active ? 2 : lineW * 0.8}" stroke-linecap="round"
              stroke-dasharray="4 20" class="edge-flow" opacity="${active ? 0.6 : 0.2}"/>
            <!-- Flowing particle -->
            <circle r="${active ? 3 : 2}" fill="${color}" opacity="${active ? 0.9 : 0.4}" filter="url(#particleGlow)">
              <animateMotion dur="${active ? '2s' : '3.5s'}" begin="${i * 0.3}s" repeatCount="indefinite">
                <mpath href="#conn${i}"/>
              </animateMotion>
            </circle>
            ${active ? svg`
              <circle r="1.5" fill="${color}" opacity="0.5">
                <animateMotion dur="2.5s" begin="${i * 0.3 + 1}s" repeatCount="indefinite">
                  <mpath href="#conn${i}"/>
                </animateMotion>
              </circle>
            ` : ''}`
        })}

        <!-- ═══ Center hub ═══ -->
        <g class="hex-center" @click=${() => { this.showSearch = true; setTimeout(() => this.shadowRoot?.querySelector('.search-modal input')?.focus(), 50) }}>
          <circle class="center-aura" cx="${CX}" cy="${CY}" r="65" fill="url(#centerGlow)"/>
          <circle class="center-pulse" cx="${CX}" cy="${CY}" r="52" fill="none" stroke="rgba(110,203,139,0.2)" stroke-width="0.8"/>
          <polygon points="${this._hexPoints(CX, CY, 44)}" fill="rgba(12,22,16,0.95)" stroke="rgba(110,203,139,0.5)" stroke-width="1.8" filter="url(#softGlow)"/>
          <polygon points="${this._hexPoints(CX, CY, 37)}" fill="none" stroke="rgba(110,203,139,0.1)" stroke-width="0.6"/>
          <polygon points="${this._hexPoints(CX, CY, 30)}" fill="none" stroke="rgba(110,203,139,0.04)" stroke-width="0.4"/>
          <text x="${CX}" y="${CY - 5}" text-anchor="middle" fill="rgba(110,203,139,0.9)" font-size="17" style="pointer-events:none;">🔍</text>
          <text x="${CX}" y="${CY + 9}" text-anchor="middle" fill="rgba(110,203,139,0.55)" font-size="7.5" font-family="Georgia,serif" style="pointer-events:none;">Bureau d'étude</text>
          <text x="${CX}" y="${CY + 20}" text-anchor="middle" fill="rgba(154,184,164,0.4)" font-size="6.5" font-family="sans-serif" style="pointer-events:none;">${totalNodes} fiches</text>
        </g>

        <!-- ═══ Aisle hexagons ═══ -->
        ${aislePos.map((ap, i) => {
          const a = ap.section
          const color = ap.color
          const active = activeAisle?.section?.id === a.section.id
          const hasChildren = (a.children || []).length > 0
          // Count inter-edges involving this aisle
          const edgeCount = interLinks.filter(l => l.from.id === a.section.id || l.to.id === a.section.id).reduce((s, l) => s + l.count, 0)
          return svg`
            <g class="hex-aisle" style="--hover-glow:${color}" @click=${() => this.openAisle(a)}>
              <!-- Glow aura -->
              <circle cx="${ap.x}" cy="${ap.y}" r="${HEX_R + 12}" fill="url(#aisleGlow${i})" opacity="${active ? 1 : 0.6}"/>
              <!-- Main hex -->
              <polygon points="${this._hexPoints(ap.x, ap.y, HEX_R)}"
                fill="${active ? 'rgba(20,42,32,0.94)' : 'rgba(14,26,20,0.92)'}"
                stroke="${color}" stroke-width="${active ? 2.5 : 1.5}"
                ${active ? svg`filter="url(#neonGlow)"` : ''}/>
              <!-- Inner border decorations -->
              <polygon points="${this._hexPoints(ap.x, ap.y, HEX_R - 5)}" fill="none" stroke="${color}" stroke-width="0.4" stroke-opacity="${active ? 0.25 : 0.12}"/>
              <polygon points="${this._hexPoints(ap.x, ap.y, HEX_R - 10)}" fill="none" stroke="${color}" stroke-width="0.2" stroke-opacity="0.06"/>
              <!-- Section name -->
              <text class="hex-label" x="${ap.x}" y="${ap.y - 4}" text-anchor="middle" fill="${color}" font-size="12" font-weight="600" font-family="Georgia,serif" style="pointer-events:none;">${a.section.name}</text>
              <!-- Count -->
              <text class="hex-count" x="${ap.x}" y="${ap.y + 10}" text-anchor="middle" fill="rgba(154,184,164,0.6)" font-size="8.5" font-family="sans-serif" style="pointer-events:none;">${a.node_count} fiche${a.node_count !== 1 ? 's' : ''}</text>
              <!-- Edge indicator (if linked to other sections) -->
              ${edgeCount > 0 ? svg`
                <text x="${ap.x}" y="${ap.y + 20}" text-anchor="middle" fill="${color}" font-size="6" font-family="sans-serif" opacity="0.4" style="pointer-events:none;">⬡ ${edgeCount} lien${edgeCount > 1 ? 's' : ''}</text>
              ` : ''}
              <!-- Shelf indicator dots -->
              ${hasChildren ? svg`
                <g style="pointer-events:none;">
                  ${(a.children || []).map((_, ci) => {
                    const dotAngle = -Math.PI/2 + (ci - ((a.children.length-1)/2)) * 0.3
                    const dotR = HEX_R + 4
                    return svg`<circle cx="${ap.x + dotR * Math.cos(dotAngle)}" cy="${ap.y + dotR * Math.sin(dotAngle) + HEX_R * 0.55}" r="2.2" fill="${color}" opacity="0.4"/>`
                  })}
                </g>
              ` : ''}
            </g>`
        })}

        <!-- ═══ Shelf hexagons (expand from active aisle) ═══ -->
        ${activeAisle ? (activeAisle.children || []).map((shelf, j) => {
          const parentIdx = aisles.findIndex(a => a.section.id === activeAisle.section.id)
          const parentAngle = (2 * Math.PI / n) * parentIdx - Math.PI / 2
          const childCount = activeAisle.children?.length || 1
          const spread = Math.min(Math.PI / 2.5, Math.PI / (childCount + 0.5))
          const shelfAngle = parentAngle - (spread * (childCount - 1) / 2) + spread * j
          const sx = CX + SHELF_R * Math.cos(shelfAngle), sy = CY + SHELF_R * Math.sin(shelfAngle)
          const px = aislePos[parentIdx]?.x || CX, py = aislePos[parentIdx]?.y || CY
          const sc = this._safeColor(shelf.section.color || activeAisle.section.color)
          const mx = (px + sx) / 2, my = (py + sy) / 2
          const perpX = -(sy - py) * 0.12, perpY = (sx - px) * 0.12
          const pathD = `M${px},${py} Q${mx + perpX},${my + perpY} ${sx},${sy}`
          return svg`
            <g class="shelf-group" style="animation-delay:${j * 0.1}s">
              <!-- Connection to parent: base -->
              <path id="shelf${j}" d="${pathD}" fill="none" stroke="${sc}"
                stroke-width="1.2" stroke-opacity="0.3" stroke-dasharray="4,6"/>
              <!-- Flowing energy on shelf connection -->
              <path d="${pathD}" fill="none" stroke="${sc}"
                stroke-width="1" stroke-linecap="round"
                stroke-dasharray="3 21" class="edge-flow" opacity="0.4"/>
              <!-- Particle -->
              <circle r="2" fill="${sc}" opacity="0.6" filter="url(#particleGlow)">
                <animateMotion dur="2.5s" begin="${j * 0.4}s" repeatCount="indefinite">
                  <mpath href="#shelf${j}"/>
                </animateMotion>
              </circle>
              <g class="hex-shelf" style="--hover-glow:${sc}" @click=${() => this.openShelf(shelf)}>
                <polygon points="${this._hexPoints(sx, sy, SHELF_HEX_R)}" fill="rgba(14,26,20,0.92)" stroke="${sc}" stroke-width="1.5"/>
                <polygon points="${this._hexPoints(sx, sy, SHELF_HEX_R - 4)}" fill="none" stroke="${sc}" stroke-width="0.3" stroke-opacity="0.15"/>
                <text class="hex-label" x="${sx}" y="${sy - 1}" text-anchor="middle" fill="${sc}" font-size="9.5" font-weight="600" font-family="Georgia,serif" style="pointer-events:none;">${shelf.section.name}</text>
                <text class="hex-count" x="${sx}" y="${sy + 10}" text-anchor="middle" fill="rgba(154,184,164,0.5)" font-size="7" font-family="sans-serif" style="pointer-events:none;">${shelf.node_count}</text>
              </g>
            </g>`
        }) : ''}

        <!-- ═══ Decorative fireflies ═══ -->
        ${[0,1,2,3,4].map(fi => {
          const baseAngle = (fi / 5) * Math.PI * 2
          const r1 = 100 + fi * 30
          const bx = CX + r1 * Math.cos(baseAngle), by = CY + r1 * Math.sin(baseAngle)
          return svg`
            <circle r="1.5" fill="rgba(110,203,139,0.6)" filter="url(#particleGlow)">
              <animate attributeName="cx" values="${bx};${bx+15};${bx-10};${bx+8};${bx}" dur="${7+fi*2}s" repeatCount="indefinite"/>
              <animate attributeName="cy" values="${by};${by-12};${by+8};${by-5};${by}" dur="${5+fi*1.5}s" repeatCount="indefinite"/>
              <animate attributeName="opacity" values="0;0.6;0.2;0.8;0" dur="${4+fi}s" repeatCount="indefinite"/>
            </circle>`
        })}
      </svg>
    `
  }

  // ── Desk Panel ──

  renderDeskPanel() {
    if (!this.desk) return html`
      <div class="panel-header">
        <h3>Chargement...</h3>
        <button class="panel-close" @click=${() => this.closePanel()}>×</button>
      </div>`

    const fields = this.desk.fields || this.desk.node.fields || null
    const template = this.getTemplateForNode(this.desk.node)
    const compiledHtml = this.renderTemplateHtml(this.desk.node, fields, template)

    return html`
      <div class="panel-header">
        <h3>${this.desk.node.title}</h3>
        <button class="btn-sm" @click=${() => this.openEdit()}>✏️</button>
        <button class="panel-close" @click=${() => this.closePanel()}>×</button>
      </div>

      <div class="panel-body">
        <div class="desk-meta">
          ${this._buildSectionPath(this.desk.sections || [])} · ${this.desk.versions_count} version${this.desk.versions_count !== 1 ? 's' : ''}
          ${this.desk.node.view_count > 0 ? html` · <span class="view-badge" title="Dernière consultation : ${this.desk.node.last_viewed?.slice(0,16).replace('T',' ') || '?'}">${this.desk.node.view_count}× consultée</span>` : ''}
        </div>

        <div class="tags-row">
          ${(this.desk.tags || []).map(t => html`<span class="tag">${t.name}</span>`)}
        </div>

        ${compiledHtml ? html`
          <div class="template-preview">
            ${template?.preview_css ? html`<style>${this._sanitizeCSS(template.preview_css)}</style>` : ''}
            <div class="tpl-scope">${unsafeHTML(compiledHtml)}</div>
          </div>
        ` : fields && Object.keys(fields).length > 0 ? html`
          <div class="content-preview">
            ${Object.entries(fields).map(([key, value]) => html`
              <div style="margin-bottom:0.5rem;">
                <span style="color:var(--lib-text-muted);font-size:0.75em;text-transform:uppercase;">${key.replace(/_/g,' ')}</span><br>
                <span>${this.formatFieldValue(value)}</span>
              </div>
            `)}
          </div>
        ` : this.desk.node.content ? html`
          <div class="content-preview">${this.desk.node.content}</div>
        ` : ''}

        <h4 style="margin:1rem 0 0.4rem;font-size:0.85em;color:var(--lib-text-secondary);display:flex;align-items:center;justify-content:space-between;">
          Connexions (${this.desk.connections.length})
          <button class="btn-xs btn-link-add" @click=${() => { this.showLinkModal = true; this.linkSearchQuery = ''; this.linkSearchResults = [] }}>+ Lier</button>
        </h4>
        ${this.desk.connections.length > 0 ? html`
          <div style="display:flex;flex-direction:column;gap:0.3rem;">
            ${this.desk.connections.map(c => html`
              <div class="connection-card">
                <span class="conn-title" @click=${() => this.loadDeskNode(c.node.id)}>${c.node.title}</span>
                ${c.edge.relation ? html`<span class="relation-badge">${c.edge.relation}</span>` : ''}
                <span class="conn-remove" @click=${() => this._removeEdge(c.edge.id)} title="Supprimer le lien">×</span>
              </div>
            `)}
          </div>
        ` : html`<div style="font-size:0.8em;color:var(--lib-text-muted);font-style:italic;">Aucune connexion. Utilisez «+ Lier» pour relier cette fiche.</div>`}

        ${this.showLinkModal ? html`
          <div class="link-modal">
            <input type="text" class="link-search-input" placeholder="Rechercher une fiche à lier..."
              .value=${this.linkSearchQuery}
              @input=${(e) => { this.linkSearchQuery = e.target.value; this._debounceLinkSearch() }}
              @keydown=${(e) => { if (e.key === 'Escape') this.showLinkModal = false }}>
            <div class="link-results">
              ${this.linkSearchResults.filter(n => n.id !== this.desk.node.id && !this.desk.connections.some(c => c.node.id === n.id)).map(n => html`
                <div class="link-result-item" @click=${() => this._createLink(n.id)}>
                  ${n.title}
                  <span style="font-size:0.7em;color:var(--lib-text-muted);">${this._templateName(n.template_id)}</span>
                </div>
              `)}
              ${this.linkSearchQuery && this.linkSearchResults.length === 0 ? html`<div style="padding:0.4rem;color:var(--lib-text-muted);font-size:0.8em;">Aucun résultat</div>` : ''}
            </div>
          </div>
        ` : ''}

        ${this.desk.versions_count > 0 ? html`
          <div style="margin-top:0.8rem;">
            <button class="btn-sm" @click=${async () => {
              if (!this.showVersions) {
                try { this._versions = (await api(`/nodes/${this.desk.node.id}/versions`)).versions || [] } catch {}
              }
              this.showVersions = !this.showVersions
            }}>${this.showVersions ? 'Masquer' : 'Voir'} versions (${this.desk.versions_count})</button>
            ${this.showVersions ? html`<div style="margin-top:0.4rem;">${(this._versions||[]).map(v => html`<div class="version-item"><span>v${v.version_num}</span><span style="font-size:0.75em;color:var(--lib-text-muted);">${v.created_at?.slice(0,16).replace('T',' ')}</span></div>`)}</div>` : ''}
          </div>
        ` : ''}

        ${this.pendingLinks.length > 0 ? html`
          <h4 style="margin:1rem 0 0.4rem;font-size:0.85em;color:var(--lib-amber);">Liens suggérés</h4>
          ${this.pendingLinks.map(pl => html`
            <div class="pending-card">
              <span>${pl.occurrence || 'Lien détecté'}</span>
              <div style="display:flex;gap:0.3rem;">
                <button class="btn-sm btn-confirm" @click=${() => this.confirmLink(pl.id)}>✓</button>
                <button class="btn-sm btn-dismiss" @click=${() => this.dismissLink(pl.id)}>✗</button>
              </div>
            </div>
          `)}
        ` : ''}
      </div>

      <div class="panel-actions">
        <button class="btn-secondary" @click=${() => this.exportPdf()}>PDF</button>
        <button class="btn-secondary" @click=${() => this.openEdit()}>Modifier</button>
        <button class="btn-sm btn-dismiss" @click=${() => this.deleteNode(this.desk.node.id)}>Supprimer</button>
      </div>
    `
  }

  // ── Editor Panel ──

  renderEditorPanel() {
    return html`
      <div class="panel-header">
        <h3>${this.editorMode === 'edit' ? 'Modifier' : 'Nouvelle fiche'}${this._hasUnsavedChanges ? html`<span style="color:#fbbf24;font-size:0.7em;margin-left:0.5rem;">●</span>` : ''}</h3>
        <button class="panel-close" @click=${() => this.closePanel()}>×</button>
      </div>

      <div class="panel-body">
        <div class="editor-form">
          <div class="form-field">
            <label>Titre</label>
            <input type="text" .value=${this.editingNode?.title || ''}
              @input=${(e) => { this.editingNode = { ...this.editingNode, title: e.target.value }; this._hasUnsavedChanges = true }}>
          </div>

          <div class="form-row">
            <div class="form-field">
              <label>Patron</label>
              <select .value=${this.editingNode?.template_id || ''}
                @change=${(e) => { this.editingNode = { ...this.editingNode, template_id: e.target.value }; this._hasUnsavedChanges = true }}>
                <option value="">Texte libre</option>
                ${this.templates.map(t => html`<option value="${t.id}" ?selected=${this.editingNode?.template_id === t.id}>${t.name}</option>`)}
              </select>
            </div>
          </div>

          <div class="form-field">
            <label>Étagère</label>
            <div style="display:flex;flex-wrap:wrap;gap:0.4rem;">
              ${this._getHierarchicalSections().map(item => html`
                <label style="display:flex;align-items:center;gap:0.25rem;font-size:0.8em;cursor:pointer;${item.isAisle ? 'font-weight:600;margin-top:0.2rem;' : 'padding-left:0.8rem;'}">
                  ${item.isAisle ? '' : html`
                    <input type="checkbox"
                      ?checked=${(this.editingNode?.section_ids || []).includes(item.section.id)}
                      @change=${(e) => {
                        const ids = [...(this.editingNode.section_ids || [])]
                        if (e.target.checked) { if (!ids.includes(item.section.id)) ids.push(item.section.id) }
                        else { const idx = ids.indexOf(item.section.id); if (idx > -1) ids.splice(idx, 1) }
                        this.editingNode = { ...this.editingNode, section_ids: ids }
                        this._hasUnsavedChanges = true
                      }}>
                  `}
                  ${item.isAisle ? `📚 ${item.section.name}` : item.section.name}
                </label>
              `)}
            </div>
          </div>

          <div class="form-field">
            <label>Tags (virgules)</label>
            <input type="text" .value=${(this.editingNode?.tags || []).join(', ')}
              @input=${(e) => {
                this.editingNode = { ...this.editingNode, tags: e.target.value.split(',').map(t => t.trim()).filter(Boolean) }
                this._hasUnsavedChanges = true
              }}>
          </div>

          ${this._getSelectedTemplate() ? this.renderFieldInputs() : html`
            <div class="form-field">
              <label>Contenu</label>
              <textarea .value=${this.editingNode?.content || ''}
                @input=${(e) => { this.editingNode = { ...this.editingNode, content: e.target.value }; this._hasUnsavedChanges = true }}
                placeholder="Contenu de la fiche"></textarea>
            </div>
          `}
        </div>
      </div>

      <div class="panel-actions">
        <button class="btn-primary" @click=${() => this.saveNode()}>${this.editorMode === 'edit' ? 'Enregistrer' : 'Créer'}</button>
        <button class="btn-secondary" @click=${() => this.closePanel()}>Annuler</button>
      </div>
    `
  }

  renderFieldInputs() {
    const tpl = this._getSelectedTemplate()
    if (!tpl?.structure || !Array.isArray(tpl.structure)) return html`<div class="empty">Patron sans structure.</div>`
    const fields = this.editingNode?.fields || {}
    return html`
      <div style="border:1px solid var(--lib-border);border-radius:6px;padding:0.8rem;">
        <div style="font-size:0.7em;color:var(--lib-text-muted);margin-bottom:0.6rem;text-transform:uppercase;letter-spacing:0.5px;">Champs : ${tpl.name}</div>
        ${tpl.structure.map(field => {
          const name = field.name, label = field.label || name, type = field.type || 'text', val = fields[name]

          // Auto-generated field — read-only
          if (name === 'fiche_num') return html`<div class="form-field"><label>${label}</label><input type="text" .value=${val || '(auto)'} disabled style="opacity:0.6;"></div>`

          // Rating (clickable dots)
          if (type === 'rating') {
            const max = field.max || 5, current = Number(val) || 0
            return html`<div class="form-field"><label>${label}</label>
              <div class="rating-editor">
                ${Array.from({length: max}, (_, i) => html`
                  <span class="rating-dot ${i < current ? 'active' : ''}" @click=${() => this._updateField(name, i + 1 === current ? 0 : i + 1)}>●</span>
                `)}
                <span class="rating-label">${current}/${max}</span>
              </div>
            </div>`
          }

          if (type === 'number') return html`<div class="form-field"><label>${label}</label><input type="number" .value=${val != null ? String(val) : ''} @input=${(e) => this._updateField(name, e.target.value ? Number(e.target.value) : null)}></div>`

          // Tags / string arrays — visual tag editor with add/remove
          if (type === 'tags' || type === 'array') {
            // Array of objects → JSON editor (complex structures)
            if (Array.isArray(val) && val.length > 0 && typeof val[0] === 'object') {
              return html`<div class="form-field"><label>${label} (JSON)</label><textarea style="min-height:80px;font-family:monospace;font-size:0.8em;" .value=${JSON.stringify(val, null, 2)} @input=${(e) => { try { this._updateField(name, JSON.parse(e.target.value)) } catch {} }}></textarea></div>`
            }
            const items = Array.isArray(val) ? val : (val ? [val] : [])
            return html`<div class="form-field"><label>${label}</label>
              <div class="tags-editor">
                ${items.map((item, i) => html`
                  <span class="tag-chip">
                    ${item}
                    <span class="tag-remove" @click=${() => { const arr = [...items]; arr.splice(i, 1); this._updateField(name, arr) }}>×</span>
                  </span>
                `)}
                <input type="text" class="tag-input" placeholder="Ajouter..."
                  @keydown=${(e) => {
                    if (e.key === 'Enter' && e.target.value.trim()) {
                      e.preventDefault()
                      this._updateField(name, [...items, e.target.value.trim()])
                      e.target.value = ''
                    }
                  }}>
              </div>
            </div>`
          }

          if (type === 'textarea') return html`<div class="form-field"><label>${label}</label><textarea style="min-height:60px;" .value=${val || ''} @input=${(e) => this._updateField(name, e.target.value)}></textarea></div>`

          // Object value (like infos, composition) → key/value visual editor
          if (val && typeof val === 'object' && !Array.isArray(val)) {
            return html`<div class="form-field"><label>${label}</label>
              <div class="object-editor">
                ${Object.entries(val).map(([k, v]) => html`
                  <div class="obj-row">
                    <input type="text" class="obj-key" .value=${k} @input=${(e) => {
                      const obj = {...val}; delete obj[k]; obj[e.target.value] = v; this._updateField(name, obj)
                    }}>
                    <input type="text" class="obj-val" .value=${typeof v === 'object' ? JSON.stringify(v) : String(v ?? '')} @input=${(e) => {
                      let parsed = e.target.value; try { parsed = JSON.parse(parsed) } catch {}
                      this._updateField(name, {...val, [k]: parsed})
                    }}>
                    <span class="obj-remove" @click=${() => { const obj = {...val}; delete obj[k]; this._updateField(name, obj) }}>×</span>
                  </div>
                `)}
                <button class="btn-xs" @click=${() => this._updateField(name, {...val, '': ''})}>+ Ajouter</button>
              </div>
            </div>`
          }

          return html`<div class="form-field"><label>${label}</label><input type="text" .value=${val != null && typeof val === 'object' ? JSON.stringify(val) : (val || '')} @input=${(e) => this._updateField(name, e.target.value)}></div>`
        })}
      </div>
    `
  }

  // ── Settings Panel (Templates) ──

  renderSettingsPanel() {
    const aisles = (this.sections || []).filter(s => !s.parent_id)
    return html`
      <div class="panel-header">
        <h3>Gestion</h3>
        <button class="panel-close" @click=${() => this.closePanel()}>×</button>
      </div>
      <div style="display:flex;border-bottom:1px solid var(--lib-border-subtle);padding:0 0.8rem;">
        <button class="btn-sm" style="border:none;border-bottom:2px solid ${this.settingsTab === 'sections' ? 'var(--lib-accent)' : 'transparent'};border-radius:0;color:${this.settingsTab === 'sections' ? 'var(--lib-accent)' : 'var(--lib-text-muted)'};padding:0.5rem 0.8rem;" @click=${() => this.settingsTab = 'sections'}>Sections</button>
        <button class="btn-sm" style="border:none;border-bottom:2px solid ${this.settingsTab === 'templates' ? 'var(--lib-accent)' : 'transparent'};border-radius:0;color:${this.settingsTab === 'templates' ? 'var(--lib-accent)' : 'var(--lib-text-muted)'};padding:0.5rem 0.8rem;" @click=${() => this.settingsTab = 'templates'}>Patrons</button>
      </div>
      <div class="panel-body">

      ${this.settingsTab === 'sections' ? html`
        <button class="btn-create" style="width:100%;margin-bottom:0.8rem;" @click=${() => this.openCreateSection(null)}>+ Nouvelle allée</button>

        ${this.editingSection ? html`
          <div style="border:1px solid var(--lib-border);border-radius:8px;padding:0.8rem;margin-bottom:0.8rem;">
            <div class="editor-form">
              <div class="form-field"><label>Nom</label><input type="text" .value=${this.editingSection.name || ''} @input=${(e) => this.editingSection = { ...this.editingSection, name: e.target.value }}></div>
              <div class="form-field"><label>Description</label><input type="text" .value=${this.editingSection.description || ''} @input=${(e) => this.editingSection = { ...this.editingSection, description: e.target.value }}></div>
              <div class="form-row">
                <div class="form-field"><label>Couleur</label><input type="color" style="height:36px;padding:2px;" .value=${this.editingSection.color || '#4ecb71'} @input=${(e) => this.editingSection = { ...this.editingSection, color: e.target.value }}></div>
                <div class="form-field"><label>Parent</label>
                  <select .value=${this.editingSection.parent_id || ''} @change=${(e) => this.editingSection = { ...this.editingSection, parent_id: e.target.value || null }}>
                    <option value="">— Allée racine —</option>
                    ${aisles.map(a => html`<option value=${a.id} ?selected=${this.editingSection.parent_id === a.id}>${a.name}</option>`)}
                  </select>
                </div>
              </div>
              <div style="display:flex;gap:0.4rem;">
                <button class="btn-primary" @click=${() => this.saveSection()}>Enregistrer</button>
                <button class="btn-secondary" @click=${() => { this.editingSection = null }}>Annuler</button>
              </div>
            </div>
          </div>
        ` : ''}

        ${aisles.map(aisle => html`
          <div style="margin-bottom:0.6rem;">
            <div style="background:var(--lib-surface);border:1px solid var(--lib-border-subtle);border-left:4px solid ${this._safeColor(aisle.color)};border-radius:6px;padding:0.6rem 0.8rem;">
              <div style="display:flex;align-items:center;justify-content:space-between;">
                <div>
                  <div style="font-weight:600;font-size:0.9em;">${aisle.name}</div>
                  ${aisle.description ? html`<div style="font-size:0.72em;color:var(--lib-text-muted);margin-top:0.15rem;">${aisle.description}</div>` : ''}
                </div>
                <div style="display:flex;gap:0.3rem;">
                  <button class="btn-sm btn-confirm" @click=${() => this.openCreateSection(aisle.id)} title="Ajouter sous-section">+</button>
                  <button class="btn-sm" @click=${() => { this.editingSection = { ...aisle } }}>✏</button>
                  <button class="btn-sm btn-dismiss" @click=${() => this.deleteSection(aisle.id)}>✕</button>
                </div>
              </div>
            </div>
            ${(this.sections || []).filter(s => s.parent_id === aisle.id).map(shelf => html`
              <div style="background:var(--lib-surface);border:1px solid var(--lib-border-subtle);border-left:4px solid ${this._safeColor(shelf.color || aisle.color)};border-radius:6px;padding:0.45rem 0.8rem;margin-top:0.25rem;margin-left:1.2rem;">
                <div style="display:flex;align-items:center;justify-content:space-between;">
                  <div>
                    <div style="font-size:0.82em;">${shelf.name}</div>
                    ${shelf.description ? html`<div style="font-size:0.68em;color:var(--lib-text-muted);">${shelf.description}</div>` : ''}
                  </div>
                  <div style="display:flex;gap:0.3rem;">
                    <button class="btn-sm" @click=${() => { this.editingSection = { ...shelf } }}>✏</button>
                    <button class="btn-sm btn-dismiss" @click=${() => this.deleteSection(shelf.id)}>✕</button>
                  </div>
                </div>
              </div>
            `)}
          </div>
        `)}
      ` : html`
        <button class="btn-create" style="width:100%;margin-bottom:0.8rem;" @click=${() => { this.editingTemplate = { name: '', structure: null, preview_css: '', preview_html: '' } }}>+ Nouveau patron</button>

        ${this.editingTemplate ? html`
          <div style="border:1px solid var(--lib-border);border-radius:8px;padding:0.8rem;margin-bottom:0.8rem;">
            <div class="editor-form">
              <div class="form-field"><label>Nom</label><input type="text" .value=${this.editingTemplate.name || ''} @input=${(e) => this.editingTemplate = { ...this.editingTemplate, name: e.target.value }}></div>
              <div class="form-field"><label>Structure JSON</label>
                <textarea style="min-height:80px;${this.templateJsonError ? 'border-color:#ef4444;' : ''}" .value=${this.editingTemplate.structure ? JSON.stringify(this.editingTemplate.structure, null, 2) : ''}
                  @input=${(e) => { try { this.editingTemplate = { ...this.editingTemplate, structure: JSON.parse(e.target.value) }; this.templateJsonError = '' } catch (err) { this.templateJsonError = err.message } }}></textarea>
                ${this.templateJsonError ? html`<div style="color:#ef4444;font-size:0.7em;margin-top:0.2rem;">${this.templateJsonError}</div>` : ''}
              </div>
              <div class="form-field"><label>HTML</label><textarea style="min-height:80px;" .value=${this.editingTemplate.preview_html || ''} @input=${(e) => this.editingTemplate = { ...this.editingTemplate, preview_html: e.target.value }}></textarea></div>
              <div class="form-field"><label>CSS</label><textarea style="min-height:60px;" .value=${this.editingTemplate.preview_css || ''} @input=${(e) => this.editingTemplate = { ...this.editingTemplate, preview_css: e.target.value }}></textarea></div>
              <div style="display:flex;gap:0.4rem;">
                <button class="btn-primary" @click=${() => this.saveTemplate()}>Enregistrer</button>
                <button class="btn-secondary" @click=${() => { this.editingTemplate = null }}>Annuler</button>
              </div>
            </div>
          </div>
        ` : ''}

        ${this.templates.map(t => html`
          <div style="background:var(--lib-surface);border:1px solid var(--lib-border-subtle);border-left:4px solid var(--lib-leather);border-radius:6px;padding:0.6rem 0.8rem;margin-bottom:0.4rem;">
            <div style="font-weight:600;font-size:0.9em;">${t.name}</div>
            ${t.structure ? html`<div style="font-size:0.7em;color:var(--lib-text-muted);margin-top:0.2rem;">${(Array.isArray(t.structure) ? t.structure : []).map(f => f.label || f.name).join(', ')}</div>` : ''}
            <div style="margin-top:0.3rem;display:flex;gap:0.3rem;">
              <button class="btn-sm" @click=${() => { this.editingTemplate = { ...t } }}>Modifier</button>
              <button class="btn-sm btn-dismiss" @click=${() => this.deleteTemplate(t.id)}>Supprimer</button>
            </div>
          </div>
        `)}
      `}
      </div>
    `
  }

  // ── Trash Panel ──

  renderTrashPanel() {
    return html`
      <div class="panel-header">
        <h3>Corbeille</h3>
        <button class="panel-close" @click=${() => this.closePanel()}>×</button>
      </div>
      <div class="panel-body">
        ${this.trash.length === 0 ? html`<div class="empty">Corbeille vide</div>` : this.trash.map(node => html`
          <div class="pending-card">
            <div>
              <div style="font-weight:500;">${node.title}</div>
              <div style="font-size:0.7em;color:var(--lib-text-muted);">Supprimé le ${node.deleted_at?.slice(0, 10) || ''}</div>
            </div>
            <div style="display:flex;gap:0.3rem;">
              <button class="btn-sm btn-confirm" @click=${() => this.restoreNode(node.id)}>Restaurer</button>
              <button class="btn-sm btn-dismiss" @click=${() => this.purgeNode(node.id)}>Purger</button>
            </div>
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('library-page', LibraryPage)
