/**
 * Symbion Log Viewer — Page standalone
 *
 * Affiche les logs kernel (via API journalctl) et PWA (via BroadcastChannel).
 * Filtrable par source, component, level. Recherche texte. Tri par colonne.
 * Clic sur ligne = expand JSON complet.
 */

import { LitElement, html, css } from 'lit'

class LogsViewer extends LitElement {
  static styles = css`
    :host {
      display: block;
      min-height: 100vh;
      background: var(--color-dark-bg, #0a0a0b);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    }

    .header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.5rem;
      background: linear-gradient(135deg, var(--color-dark-surface, #111) 0%, var(--color-dark-elevated, #1a1a2e) 100%);
      border-bottom: 1px solid var(--border-default);
      position: sticky;
      top: 0;
      z-index: 10;
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .header h1 {
      font-size: 1.2em;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .header-icon {
      font-size: 1.4em;
    }

    .connection-badge {
      font-size: 0.75em;
      padding: 0.25rem 0.6rem;
      border-radius: 12px;
      font-weight: 500;
    }

    .connection-badge.online {
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
      border: 1px solid rgba(34, 197, 94, 0.3);
    }

    .connection-badge.offline {
      background: rgba(255, 107, 107, 0.15);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.3);
    }

    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 0.6rem;
      padding: 0.8rem 1.5rem;
      background: var(--surface-glass-faint);
      border-bottom: 1px solid var(--border-subtle);
      align-items: center;
    }

    .source-tabs {
      display: flex;
      gap: 0;
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      overflow: hidden;
    }

    .source-tab {
      padding: 0.4rem 0.8rem;
      font-size: 0.8em;
      font-weight: 500;
      background: transparent;
      border: none;
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      transition: all 0.2s;
    }

    .source-tab:not(:last-child) {
      border-right: 1px solid var(--border-medium);
    }

    .source-tab.active {
      background: rgba(99, 102, 241, 0.2);
      color: #818cf8;
    }

    .source-tab:hover:not(.active) {
      background: var(--surface-glass);
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .search-input {
      flex: 1;
      min-width: 180px;
      padding: 0.45rem 0.8rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85em;
      font-family: inherit;
      outline: none;
      transition: border-color 0.2s;
    }

    .search-input:focus {
      border-color: rgba(99, 102, 241, 0.5);
    }

    .search-input::placeholder {
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .filter-select {
      padding: 0.45rem 0.6rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.8em;
      font-family: inherit;
      cursor: pointer;
      outline: none;
    }

    .filter-select option {
      background: var(--color-dark-surface, #1a1a2e);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .toolbar-btn {
      padding: 0.45rem 0.8rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.8em;
      cursor: pointer;
      transition: all 0.2s;
      font-family: inherit;
    }

    .toolbar-btn:hover {
      background: var(--surface-glass-strong);
      border-color: rgba(255,255,255,0.2);
    }

    .toolbar-btn.active {
      background: rgba(99, 102, 241, 0.2);
      border-color: rgba(99, 102, 241, 0.4);
      color: #818cf8;
    }

    .log-count {
      font-size: 0.75em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      white-space: nowrap;
    }

    .log-table-wrapper {
      overflow-x: auto;
    }

    /* Cards hidden on desktop, shown on mobile via media query */
    .log-cards {
      display: none;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.82em;
    }

    thead {
      position: sticky;
      top: 60px;
      z-index: 5;
    }

    th {
      padding: 0.6rem 0.8rem;
      text-align: left;
      background: var(--color-dark-surface, #111);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-weight: 600;
      font-size: 0.85em;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      border-bottom: 1px solid var(--border-default);
      cursor: pointer;
      user-select: none;
      white-space: nowrap;
    }

    th:hover {
      color: #bbb;
    }

    th .sort-arrow {
      margin-left: 0.3rem;
      opacity: 0.4;
    }

    th .sort-arrow.active {
      opacity: 1;
      color: #818cf8;
    }

    td {
      padding: 0.45rem 0.8rem;
      border-bottom: 1px solid rgba(255,255,255,0.03);
      vertical-align: top;
    }

    tr {
      transition: background 0.15s;
    }

    tr:hover {
      background: var(--surface-glass-subtle);
    }

    tr.clickable {
      cursor: pointer;
    }

    .td-timestamp {
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.85em;
      color: #777;
      white-space: nowrap;
    }

    .td-source {
      white-space: nowrap;
    }

    .source-badge {
      display: inline-block;
      padding: 0.15rem 0.45rem;
      border-radius: var(--radius-sm);
      font-size: 0.8em;
      font-weight: 500;
    }

    .source-badge.kernel {
      background: rgba(245, 158, 11, 0.15);
      color: #f59e0b;
    }

    .source-badge.pwa {
      background: rgba(99, 102, 241, 0.15);
      color: #818cf8;
    }

    .level-badge {
      display: inline-block;
      padding: 0.15rem 0.45rem;
      border-radius: var(--radius-sm);
      font-size: 0.8em;
      font-weight: 600;
      min-width: 55px;
      text-align: center;
    }

    .level-debug { background: rgba(148,163,184,0.12); color: #94a3b8; }
    .level-info { background: rgba(34,197,94,0.12); color: #22c55e; }
    .level-notice { background: rgba(59,130,246,0.12); color: #3b82f6; }
    .level-warning { background: rgba(245,158,11,0.15); color: #f59e0b; }
    .level-error { background: rgba(239,68,68,0.15); color: #ef4444; }
    .level-critical { background: rgba(239,68,68,0.25); color: #ff4444; font-weight: 700; }

    .td-component {
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.85em;
      color: #818cf8;
      white-space: nowrap;
    }

    .td-message {
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.85em;
      word-break: break-word;
      max-width: 600px;
    }

    .trace-id {
      color: #f59e0b;
      font-weight: 500;
    }

    .expanded-row td {
      padding: 0;
    }

    .json-expand {
      background: var(--surface-glass-strong, rgba(0,0,0,0.3));
      padding: 0.8rem 1rem;
      border-left: 3px solid #818cf8;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.8em;
      white-space: pre-wrap;
      word-break: break-all;
      color: #aaa;
      max-height: 300px;
      overflow-y: auto;
    }

    .empty-state {
      text-align: center;
      padding: 4rem 2rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .empty-state .icon {
      font-size: 3em;
      margin-bottom: 1rem;
    }

    .loading-bar {
      height: 2px;
      background: linear-gradient(90deg, transparent, #818cf8, transparent);
      animation: loading-slide 1.5s ease-in-out infinite;
    }

    @keyframes loading-slide {
      0% { transform: translateX(-100%); }
      100% { transform: translateX(100%); }
    }

    /* ===== Mobile: card layout instead of table ===== */
    @media (max-width: 768px) {
      .header {
        padding: 0.75rem 1rem;
      }

      .header h1 {
        font-size: 1em;
      }

      .toolbar {
        padding: 0.5rem 0.75rem;
        gap: 0.4rem;
      }

      .source-tabs {
        width: 100%;
      }

      .source-tab {
        flex: 1;
        text-align: center;
        padding: 0.5rem;
      }

      .search-input {
        min-width: 0;
        width: 100%;
        order: -1;
      }

      .filter-select {
        flex: 1;
        min-width: 0;
        font-size: 0.75em;
      }

      .toolbar-btn {
        padding: 0.4rem 0.6rem;
        font-size: 0.75em;
      }

      /* Hide table, show cards */
      table, thead {
        display: none;
      }

      .log-table-wrapper {
        overflow-x: visible;
      }

      .log-cards {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding: 0.5rem 0.75rem;
      }

      .log-card {
        background: var(--surface-glass-subtle);
        border: 1px solid var(--border-subtle);
        border-radius: 8px;
        padding: 0.6rem 0.75rem;
        cursor: pointer;
        transition: background 0.15s;
      }

      .log-card:active {
        background: rgba(255,255,255,0.06);
      }

      .log-card-header {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        margin-bottom: 0.3rem;
        flex-wrap: wrap;
      }

      .log-card-time {
        font-family: 'JetBrains Mono', monospace;
        font-size: 0.7em;
        color: var(--color-dark-text-tertiary, #94a3b8);
        margin-left: auto;
      }

      .log-card-message {
        font-family: 'JetBrains Mono', monospace;
        font-size: 0.78em;
        color: var(--color-dark-text-secondary, #cbd5e1);
        word-break: break-word;
        line-height: 1.4;
      }

      .log-card-expand {
        margin-top: 0.5rem;
        background: var(--surface-glass-strong, rgba(0,0,0,0.3));
        padding: 0.6rem;
        border-radius: var(--radius-sm);
        border-left: 3px solid #818cf8;
        font-family: 'JetBrains Mono', monospace;
        font-size: 0.7em;
        white-space: pre-wrap;
        word-break: break-all;
        color: #aaa;
        max-height: 250px;
        overflow-y: auto;
      }

      .json-expand {
        font-size: 0.7em;
        padding: 0.6rem;
      }
    }

    /* Utility classes (ex-inline) */
    .lv-hint { margin-top: 0.5rem; font-size: 0.85em; color: #666; }
    .lv-search-compact { max-width: 160px; }
  `

  static properties = {
    sourceTab: { type: String },
    searchText: { type: String },
    levelFilter: { type: String },
    componentFilter: { type: String },
    sinceFilter: { type: String },
    sortColumn: { type: String },
    sortDirection: { type: String },
    expandedRow: { type: Number },
    kernelLogs: { type: Array },
    pwaLogs: { type: Array },
    loading: { type: Boolean },
    autoRefresh: { type: Boolean },
    authenticated: { type: Boolean },
    components: { type: Array },
    traceIdFilter: { type: String }
  }

  constructor() {
    super()
    this.sourceTab = 'all'
    this.searchText = ''
    this.levelFilter = ''
    this.componentFilter = ''
    this.sinceFilter = '1h'
    this.sortColumn = 'timestamp'
    this.sortDirection = 'desc'
    this.expandedRow = -1
    this.kernelLogs = []
    this.pwaLogs = []
    this.loading = false
    this.autoRefresh = false
    this.authenticated = false
    this.components = []
    this.traceIdFilter = ''
    this._refreshInterval = null
    this._searchDebounce = null
    this._channel = null
    this.baseUrl = window.SYMBION_CONFIG?.API_BASE || window.location.origin
  }

  connectedCallback() {
    super.connectedCallback()
    this._checkAuth()
    this._setupBroadcastChannel()
    this.fetchKernelLogs()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._refreshInterval) clearInterval(this._refreshInterval)
    if (this._channel) this._channel.close()
  }

  _checkAuth() {
    const token = sessionStorage.getItem('symbion_auth_token')
    this.authenticated = !!token
  }

  _getAuthHeader() {
    const token = sessionStorage.getItem('symbion_auth_token')
    return token ? { 'Authorization': `Bearer ${token}` } : {}
  }

  _setupBroadcastChannel() {
    try {
      this._channel = new BroadcastChannel('symbion-logs')
      this._channel.onmessage = (event) => {
        const entry = event.data
        this.pwaLogs = [entry, ...this.pwaLogs.slice(0, 499)]
        this._updateComponents(entry.component)
        this.requestUpdate()
      }
    } catch (_) {
      // BroadcastChannel not supported
    }
  }

  _updateComponents(component) {
    if (component && !this.components.includes(component)) {
      this.components = [...this.components, component].sort()
    }
  }

  async fetchKernelLogs() {
    if (!this.authenticated) return
    this.loading = true

    try {
      const params = new URLSearchParams()
      if (this.levelFilter) params.set('level', this.levelFilter)
      if (this.searchText) params.set('search', this.searchText)
      if (this.traceIdFilter) params.set('trace_id', this.traceIdFilter)
      params.set('limit', '500')
      params.set('since', this.sinceFilter)

      const resp = await fetch(`${this.baseUrl}/v1/logs?${params}`, {
        headers: this._getAuthHeader()
      })

      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)

      const data = await resp.json()
      this.kernelLogs = data.entries || []

      // Collect components
      for (const entry of this.kernelLogs) {
        this._updateComponents(entry.component)
      }
    } catch (e) {
      console.error('[logs-viewer] Failed to fetch kernel logs:', e)
    } finally {
      this.loading = false
    }
  }

  get filteredLogs() {
    let logs = []

    if (this.sourceTab === 'kernel' || this.sourceTab === 'all') {
      logs = logs.concat(this.kernelLogs)
    }
    if (this.sourceTab === 'pwa' || this.sourceTab === 'all') {
      logs = logs.concat(this.pwaLogs)
    }

    // Component filter
    if (this.componentFilter) {
      logs = logs.filter(l => l.component === this.componentFilter)
    }

    // Level filter (for PWA logs when sourceTab is pwa/all)
    if (this.levelFilter && this.sourceTab !== 'kernel') {
      const levels = this.levelFilter.split(',')
      logs = logs.filter(l => l.source === 'kernel' || levels.includes(l.level))
    }

    // Search filter (client-side for PWA, server-side already done for kernel)
    if (this.searchText && this.sourceTab !== 'kernel') {
      const s = this.searchText.toLowerCase()
      logs = logs.filter(l =>
        l.source === 'kernel' ||
        l.message.toLowerCase().includes(s) ||
        l.component.toLowerCase().includes(s)
      )
    }

    // Trace ID filter (client-side for PWA logs)
    if (this.traceIdFilter) {
      logs = logs.filter(l =>
        l.source === 'kernel' || l.message.includes(this.traceIdFilter)
      )
    }

    // Sort
    logs.sort((a, b) => {
      let va = a[this.sortColumn] || ''
      let vb = b[this.sortColumn] || ''
      if (this.sortColumn === 'timestamp') {
        va = new Date(va).getTime() || 0
        vb = new Date(vb).getTime() || 0
      }
      const cmp = va < vb ? -1 : va > vb ? 1 : 0
      return this.sortDirection === 'asc' ? cmp : -cmp
    })

    return logs
  }

  _handleSearchInput(e) {
    clearTimeout(this._searchDebounce)
    const val = e.target.value
    this._searchDebounce = setTimeout(() => {
      this.searchText = val
      if (this.sourceTab !== 'pwa') this.fetchKernelLogs()
    }, 300)
  }

  _handleSourceTab(tab) {
    this.sourceTab = tab
    if (tab !== 'pwa') this.fetchKernelLogs()
  }

  _handleLevelChange(e) {
    this.levelFilter = e.target.value
    if (this.sourceTab !== 'pwa') this.fetchKernelLogs()
  }

  _handleComponentChange(e) {
    this.componentFilter = e.target.value
  }

  _handleSinceChange(e) {
    this.sinceFilter = e.target.value
    if (this.sourceTab !== 'pwa') this.fetchKernelLogs()
  }

  _handleSort(column) {
    if (this.sortColumn === column) {
      this.sortDirection = this.sortDirection === 'asc' ? 'desc' : 'asc'
    } else {
      this.sortColumn = column
      this.sortDirection = column === 'timestamp' ? 'desc' : 'asc'
    }
  }

  _toggleExpand(index) {
    this.expandedRow = this.expandedRow === index ? -1 : index
  }

  _toggleAutoRefresh() {
    this.autoRefresh = !this.autoRefresh
    if (this.autoRefresh) {
      this._refreshInterval = setInterval(() => this.fetchKernelLogs(), 5000)
    } else {
      clearInterval(this._refreshInterval)
      this._refreshInterval = null
    }
  }

  _clearPwaLogs() {
    this.pwaLogs = []
  }

  _handleTraceIdInput(e) {
    clearTimeout(this._traceIdDebounce)
    const val = e.target.value.trim()
    this._traceIdDebounce = setTimeout(() => {
      this.traceIdFilter = val
      if (this.sourceTab !== 'pwa') this.fetchKernelLogs()
    }, 300)
  }

  _formatTimestamp(ts) {
    if (!ts) return ''
    const d = new Date(ts)
    if (isNaN(d.getTime())) return ts
    return d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
      + '.' + String(d.getMilliseconds()).padStart(3, '0')
  }

  _sortArrow(column) {
    if (this.sortColumn !== column) return html`<span class="sort-arrow"></span>`
    return html`<span class="sort-arrow active">${this.sortDirection === 'asc' ? '\u25B2' : '\u25BC'}</span>`
  }

  _highlightTraceId(msg) {
    if (typeof msg !== 'string') return msg
    const traceMatch = msg.match(/trace[_-]?id[=: ]*([a-f0-9-]+)/i)
    if (!traceMatch) return msg
    const idx = msg.indexOf(traceMatch[0])
    return html`${msg.substring(0, idx)}<span class="trace-id">${traceMatch[0]}</span>${msg.substring(idx + traceMatch[0].length)}`
  }

  render() {
    if (!this.authenticated) {
      return html`
        <div class="header">
          <div class="header-left">
            <span class="header-icon">&#x1F4DC;</span>
            <h1>Symbion Logs</h1>
          </div>
          <span class="connection-badge offline">Non authentifie</span>
        </div>
        <div class="empty-state">
          <div class="icon">&#x1F512;</div>
          <p>Ouvrez d'abord le dashboard Symbion et connectez-vous.</p>
          <p class="lv-hint">
            Le token de session est partage via sessionStorage.
          </p>
        </div>
      `
    }

    const logs = this.filteredLogs

    return html`
      <div class="header">
        <div class="header-left">
          <span class="header-icon">&#x1F4DC;</span>
          <h1>Symbion Logs</h1>
          <span class="connection-badge online">Connecte</span>
        </div>
        <span class="log-count">${logs.length} entrees</span>
      </div>

      ${this.loading ? html`<div class="loading-bar"></div>` : ''}

      <div class="toolbar">
        <div class="source-tabs">
          <button class="source-tab ${this.sourceTab === 'all' ? 'active' : ''}"
                  @click="${() => this._handleSourceTab('all')}">Tous</button>
          <button class="source-tab ${this.sourceTab === 'kernel' ? 'active' : ''}"
                  @click="${() => this._handleSourceTab('kernel')}">Kernel</button>
          <button class="source-tab ${this.sourceTab === 'pwa' ? 'active' : ''}"
                  @click="${() => this._handleSourceTab('pwa')}">PWA</button>
        </div>

        <input class="search-input"
               type="text"
               placeholder="Rechercher..."
               @input="${this._handleSearchInput}">

        <select class="filter-select" @change="${this._handleLevelChange}">
          <option value="">Tous niveaux</option>
          <option value="debug">Debug</option>
          <option value="info">Info</option>
          <option value="warning">Warning</option>
          <option value="error">Error</option>
          <option value="critical">Critical</option>
        </select>

        <select class="filter-select" @change="${this._handleComponentChange}">
          <option value="">Tous composants</option>
          ${this.components.map(c => html`<option value="${c}">${c}</option>`)}
        </select>

        ${this.sourceTab !== 'pwa' ? html`
          <select class="filter-select" @change="${this._handleSinceChange}" .value="${this.sinceFilter}">
            <option value="5m">5 min</option>
            <option value="15m">15 min</option>
            <option value="1h">1 heure</option>
            <option value="6h">6 heures</option>
            <option value="24h">24 heures</option>
          </select>
        ` : ''}

        <input class="search-input lv-search-compact"
               type="text"
               placeholder="trace_id..."
               @input="${this._handleTraceIdInput}">

        <button class="toolbar-btn" @click="${() => this.fetchKernelLogs()}"
                title="Rafraichir">&#x21BB; Refresh</button>
        <button class="toolbar-btn ${this.autoRefresh ? 'active' : ''}"
                @click="${this._toggleAutoRefresh}"
                title="Auto-refresh 5s">&#x23F1; Auto</button>
        ${this.sourceTab !== 'kernel' ? html`
          <button class="toolbar-btn" @click="${this._clearPwaLogs}"
                  title="Vider logs PWA">&#x1F5D1; PWA</button>
        ` : ''}
      </div>

      <div class="log-table-wrapper">
        ${logs.length === 0 ? html`
          <div class="empty-state">
            <div class="icon">&#x1F4ED;</div>
            <p>Aucun log correspondant aux filtres.</p>
          </div>
        ` : html`
          <!-- Desktop: table -->
          <table>
            <thead>
              <tr>
                <th @click="${() => this._handleSort('timestamp')}">
                  Heure ${this._sortArrow('timestamp')}
                </th>
                <th @click="${() => this._handleSort('source')}">
                  Source ${this._sortArrow('source')}
                </th>
                <th @click="${() => this._handleSort('level')}">
                  Level ${this._sortArrow('level')}
                </th>
                <th @click="${() => this._handleSort('component')}">
                  Component ${this._sortArrow('component')}
                </th>
                <th>Message</th>
              </tr>
            </thead>
            <tbody>
              ${logs.map((entry, i) => html`
                <tr class="clickable" @click="${() => this._toggleExpand(i)}">
                  <td class="td-timestamp">${this._formatTimestamp(entry.timestamp)}</td>
                  <td class="td-source">
                    <span class="source-badge ${entry.source}">${entry.source}</span>
                  </td>
                  <td>
                    <span class="level-badge level-${entry.level}">${entry.level}</span>
                  </td>
                  <td class="td-component">${entry.component}</td>
                  <td class="td-message">${this._highlightTraceId(entry.message)}</td>
                </tr>
                ${this.expandedRow === i ? html`
                  <tr class="expanded-row">
                    <td colspan="5">
                      <div class="json-expand">${JSON.stringify(entry.raw || entry, null, 2)}</div>
                    </td>
                  </tr>
                ` : ''}
              `)}
            </tbody>
          </table>

          <!-- Mobile: cards -->
          <div class="log-cards">
            ${logs.map((entry, i) => html`
              <div class="log-card" @click="${() => this._toggleExpand(i)}">
                <div class="log-card-header">
                  <span class="source-badge ${entry.source}">${entry.source}</span>
                  <span class="level-badge level-${entry.level}">${entry.level}</span>
                  <span class="td-component">${entry.component}</span>
                  <span class="log-card-time">${this._formatTimestamp(entry.timestamp)}</span>
                </div>
                <div class="log-card-message">${this._highlightTraceId(entry.message)}</div>
                ${this.expandedRow === i ? html`
                  <div class="log-card-expand">${JSON.stringify(entry.raw || entry, null, 2)}</div>
                ` : ''}
              </div>
            `)}
          </div>
        `}
      </div>
    `
  }
}

customElements.define('logs-viewer', LogsViewer)
