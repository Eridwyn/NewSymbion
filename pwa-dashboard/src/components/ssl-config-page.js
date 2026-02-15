import { LitElement, html, css } from 'lit'
import './organic-loader.js'

export class SslConfigPage extends LitElement {
  static properties = {
    domains: { type: Array },
    loading: { type: Boolean },
    editingDomain: { type: Object },
    formData: { type: Object },
    checkingAll: { type: Boolean },
    showAddForm: { type: Boolean }
  }

  static styles = css`
    :host {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.92);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      z-index: 1000;
      overflow-y: auto;
      overflow-x: hidden;
      -webkit-overflow-scrolling: touch;
      overscroll-behavior: contain;
    }

    .page-container {
      max-width: 800px;
      margin: 0 auto;
      padding: 1.5rem;
      padding-top: 1.5rem;
      padding-bottom: 120px;
      min-height: 100%;
      box-sizing: border-box;
    }

    /* Loader container */
    .loader-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 3rem 1rem;
    }

    /* Header */
    .page-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .back-btn {
      background: transparent;
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #888;
      width: 40px;
      height: 40px;
      border-radius: 10px;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }

    .back-btn:hover {
      background: rgba(255, 255, 255, 0.05);
      color: #e0e0e0;
    }

    .page-title-group {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .page-title {
      font-size: 1.5rem;
      font-weight: 600;
      color: #e0e0e0;
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .page-title-icon {
      color: var(--context-primary, #00d4aa);
    }

    .page-subtitle {
      font-size: 0.85rem;
      color: #666;
    }

    .header-actions {
      display: flex;
      gap: 0.75rem;
    }

    /* Sections */
    .section {
      background: linear-gradient(135deg, rgba(26, 26, 26, 0.9) 0%, rgba(15, 15, 15, 0.85) 100%);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 16px;
      padding: 1.5rem;
      margin-bottom: 1.5rem;
    }

    .section-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.25rem;
    }

    .section-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: #e0e0e0;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .section-badge {
      font-size: 0.75rem;
      padding: 0.25rem 0.6rem;
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      color: var(--context-primary, #00d4aa);
      border-radius: 6px;
      font-weight: 500;
    }

    /* Domain Grid */
    .domains-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1rem;
    }

    .domain-card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 12px;
      padding: 1.25rem;
      transition: all 0.2s ease;
    }

    .domain-card:hover {
      background: rgba(255, 255, 255, 0.06);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
    }

    .domain-card-header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      margin-bottom: 0.75rem;
    }

    .domain-info {
      flex: 1;
    }

    .domain-name {
      font-size: 1rem;
      font-weight: 600;
      color: #e0e0e0;
      margin-bottom: 0.25rem;
    }

    .domain-host {
      font-size: 0.8rem;
      color: #666;
      font-family: 'JetBrains Mono', monospace;
    }

    .domain-status {
      padding: 0.35rem 0.7rem;
      border-radius: 6px;
      font-size: 0.75rem;
      font-weight: 600;
    }

    .status-ok {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .status-warning {
      background: rgba(251, 191, 36, 0.15);
      color: #fbbf24;
    }

    .status-critical {
      background: rgba(255, 107, 107, 0.15);
      color: #ff6b6b;
    }

    .status-error {
      background: rgba(255, 107, 107, 0.2);
      color: #ff6b6b;
    }

    .domain-details {
      display: flex;
      gap: 1rem;
      margin-bottom: 1rem;
      padding: 0.75rem;
      background: rgba(0, 0, 0, 0.2);
      border-radius: 8px;
    }

    .detail-item {
      display: flex;
      flex-direction: column;
      gap: 0.2rem;
    }

    .detail-label {
      font-size: 0.65rem;
      color: #555;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .detail-value {
      font-size: 0.85rem;
      color: #e0e0e0;
      font-weight: 500;
    }

    .detail-value.warning {
      color: #fbbf24;
    }

    .detail-value.critical {
      color: #ff6b6b;
    }

    .domain-thresholds {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
    }

    .threshold-badge {
      font-size: 0.7rem;
      padding: 0.3rem 0.6rem;
      border-radius: 6px;
      display: flex;
      align-items: center;
      gap: 0.3rem;
    }

    .threshold-warning {
      background: rgba(251, 191, 36, 0.1);
      color: #fbbf24;
      border: 1px solid rgba(251, 191, 36, 0.2);
    }

    .threshold-critical {
      background: rgba(255, 107, 107, 0.1);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.2);
    }

    .domain-actions {
      display: flex;
      gap: 0.5rem;
      padding-top: 0.75rem;
      border-top: 1px solid rgba(255, 255, 255, 0.06);
    }

    .domain-actions button {
      flex: 1;
      padding: 0.5rem;
      border-radius: 8px;
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.4rem;
      transition: all 0.2s ease;
    }

    .edit-btn {
      background: rgba(0, 212, 255, 0.1);
      border: 1px solid rgba(0, 212, 255, 0.2);
      color: #00d4ff;
    }

    .edit-btn:hover {
      background: rgba(0, 212, 255, 0.2);
    }

    .delete-btn {
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.2);
      color: #ff6b6b;
    }

    .delete-btn:hover {
      background: rgba(255, 107, 107, 0.2);
    }

    /* Form Section */
    .form-section {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.08) 0%, rgba(0, 180, 140, 0.04) 100%);
      border: 1px solid rgba(0, 212, 170, 0.2);
    }

    .form-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 1rem;
    }

    .form-group {
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
    }

    .form-group.full-width {
      grid-column: 1 / -1;
    }

    .form-label {
      font-size: 0.8rem;
      font-weight: 500;
      color: #888;
    }

    .form-input {
      padding: 0.75rem 1rem;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 10px;
      color: #e0e0e0;
      font-size: 0.9rem;
      transition: all 0.2s ease;
    }

    .form-input:focus {
      outline: none;
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
    }

    .form-input::placeholder {
      color: #555;
    }

    .form-help {
      font-size: 0.7rem;
      color: #555;
    }

    .form-actions {
      display: flex;
      gap: 0.75rem;
      margin-top: 1.25rem;
      padding-top: 1.25rem;
      border-top: 1px solid rgba(0, 212, 170, 0.15);
    }

    /* Buttons */
    .btn {
      padding: 0.75rem 1.25rem;
      border-radius: 10px;
      font-size: 0.85rem;
      font-weight: 600;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      transition: all 0.2s ease;
    }

    .btn-primary {
      background: linear-gradient(135deg, var(--context-primary, #00d4aa) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 80%, #000) 100%);
      border: none;
      color: #0a0a0f;
      flex: 1;
    }

    .btn-primary:hover {
      transform: translateY(-2px);
      box-shadow: 0 6px 20px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
    }

    .btn-primary:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
      box-shadow: none;
    }

    .btn-secondary {
      background: transparent;
      border: 1px solid rgba(255, 255, 255, 0.15);
      color: #888;
    }

    .btn-secondary:hover {
      background: rgba(255, 255, 255, 0.05);
      color: #e0e0e0;
    }

    .btn-action {
      background: linear-gradient(135deg, rgba(0, 212, 255, 0.15) 0%, rgba(0, 180, 216, 0.1) 100%);
      border: 1px solid rgba(0, 212, 255, 0.3);
      color: #00d4ff;
    }

    .btn-action:hover {
      background: linear-gradient(135deg, rgba(0, 212, 255, 0.25) 0%, rgba(0, 180, 216, 0.2) 100%);
    }

    .btn-action:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    /* Empty State */
    .empty-state {
      text-align: center;
      padding: 3rem 1.5rem;
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: #888;
      margin-bottom: 0.5rem;
    }

    .empty-text {
      font-size: 0.85rem;
      color: #555;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 3rem;
      color: #666;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .spinner {
      width: 24px;
      height: 24px;
      border: 2px solid rgba(0, 212, 170, 0.2);
      border-top-color: #00d4aa;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-right: 0.75rem;
    }

    /* Add button */
    .add-domain-btn {
      width: 100%;
      padding: 1rem;
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
      border: 2px dashed color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      border-radius: 12px;
      color: var(--context-primary, #00d4aa);
      font-size: 0.9rem;
      font-weight: 600;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      transition: all 0.2s ease;
      margin-bottom: 1.5rem;
    }

    .add-domain-btn:hover {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
    }

    /* Form section collapsible */
    .form-section {
      background: linear-gradient(135deg, rgba(26, 26, 26, 0.95) 0%, rgba(15, 15, 15, 0.9) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      animation: slideDown 0.2s ease-out;
    }

    @keyframes slideDown {
      from {
        opacity: 0;
        transform: translateY(-10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    /* Mobile responsive */
    @media (max-width: 640px) {
      .page-container {
        padding: 1rem;
      }

      .domains-grid {
        grid-template-columns: 1fr;
      }

      .form-grid {
        grid-template-columns: 1fr;
      }

      .form-group.full-width {
        grid-column: auto;
      }

      .header-actions {
        flex-direction: column;
      }
    }
  `

  constructor() {
    super()
    this.domains = []
    this.loading = true
    this.editingDomain = null
    this.formData = this.getEmptyFormData()
    this.checkingAll = false
    this.showAddForm = false
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchDomains()
  }

  getEmptyFormData() {
    return {
      hostname: '',
      port: 443,
      label: '',
      warning_days: 30,
      critical_days: 14
    }
  }

  getApiBaseUrl() {
    // Utiliser la même origine que la page (nginx proxy vers kernel)
    return window.location.origin
  }

  getAuthToken() {
    // Le token est stocké dans sessionStorage par auth-service
    return sessionStorage.getItem('symbion_auth_token') || ''
  }

  async fetchDomains() {
    this.loading = true
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    console.log('[ssl-config] Fetching domains...', { baseUrl, hasToken: !!token })

    try {
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains`, {
        headers: { 'Authorization': `Bearer ${token}` }
      })

      console.log('[ssl-config] Response status:', response.status)

      if (response.ok) {
        const data = await response.json()
        console.log('[ssl-config] Got domains:', data)
        this.domains = data.domains || []
      } else {
        console.error('[ssl-config] Response not ok:', response.status, await response.text())
      }
    } catch (err) {
      console.error('[ssl-config] Fetch error:', err)
    } finally {
      this.loading = false
    }
  }

  toggleAddForm() {
    this.showAddForm = !this.showAddForm
    if (!this.showAddForm) {
      this.editingDomain = null
      this.formData = this.getEmptyFormData()
    }
  }

  editDomain(domain) {
    this.editingDomain = domain
    this.showAddForm = true
    this.formData = {
      hostname: domain.hostname || '',
      port: domain.port || 443,
      label: domain.label || '',
      warning_days: domain.warning_days || 30,
      critical_days: domain.critical_days || 14
    }
  }

  cancelEdit() {
    this.editingDomain = null
    this.showAddForm = false
    this.formData = this.getEmptyFormData()
  }

  updateFormField(field, value) {
    this.formData = { ...this.formData, [field]: value }
  }

  async saveDomain() {
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()
    const headers = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    }

    try {
      let response
      if (this.editingDomain) {
        response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains/${this.editingDomain.id}`, {
          method: 'PUT',
          headers,
          body: JSON.stringify(this.formData)
        })
      } else {
        response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains`, {
          method: 'POST',
          headers,
          body: JSON.stringify(this.formData)
        })
      }

      if (response.ok) {
        this.editingDomain = null
        this.showAddForm = false
        this.formData = this.getEmptyFormData()
        await this.fetchDomains()
        this.triggerCheck()
      } else {
        const error = await response.text()
        console.error('[ssl-config] Save failed:', error)
      }
    } catch (err) {
      console.error('[ssl-config] Save error:', err)
    }
  }

  async deleteDomain(domain) {
    if (!confirm(`Supprimer "${domain.label || domain.hostname}" ?`)) return

    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    try {
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains/${domain.id}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` }
      })

      if (response.ok) {
        await this.fetchDomains()
      }
    } catch (err) {
      console.error('[ssl-config] Delete error:', err)
    }
  }

  async triggerCheck() {
    this.checkingAll = true
    const token = this.getAuthToken()
    const baseUrl = this.getApiBaseUrl()

    try {
      await fetch(`${baseUrl}/v1/plugin-api/ssl/check`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      })
      // Wait and refresh
      setTimeout(() => {
        this.fetchDomains()
        this.checkingAll = false
      }, 3000)
    } catch (err) {
      console.error('[ssl-config] Check error:', err)
      this.checkingAll = false
    }
  }

  close() {
    this.dispatchEvent(new CustomEvent('close'))
  }

  getStatusClass(domain) {
    if (!domain.ssl_valid) return 'error'
    const days = domain.days_remaining
    if (days === null || days === undefined) return 'error'
    if (days <= (domain.critical_days || 14)) return 'critical'
    if (days <= (domain.warning_days || 30)) return 'warning'
    return 'ok'
  }

  getStatusLabel(domain) {
    const status = this.getStatusClass(domain)
    switch (status) {
      case 'ok': return 'OK'
      case 'warning': return 'Warning'
      case 'critical': return 'Critical'
      case 'error': return 'Erreur'
      default: return '?'
    }
  }

  render() {
    return html`
      <div class="page-container">
        <!-- Header -->
        <div class="page-header">
          <div class="header-left">
            <button class="back-btn" @click=${() => this.close()}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M19 12H5M12 19l-7-7 7-7"/>
              </svg>
            </button>
            <div class="page-title-group">
              <div class="page-title">
                <span class="page-title-icon">
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                    <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                  </svg>
                </span>
                Configuration SSL
              </div>
              <div class="page-subtitle">Surveillance des certificats SSL/TLS</div>
            </div>
          </div>
          <div class="header-actions">
            <button class="btn btn-action" @click=${() => this.triggerCheck()} ?disabled=${this.checkingAll}>
              ${this.checkingAll ? html`<span class="spinner"></span>` : html`
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/>
                  <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
                </svg>
              `}
              ${this.checkingAll ? 'Vérification...' : 'Vérifier tout'}
            </button>
          </div>
        </div>

        <!-- Add Domain Button (when form is hidden) -->
        ${!this.showAddForm ? html`
          <button class="add-domain-btn" @click=${() => this.toggleAddForm()}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14M5 12h14"/>
            </svg>
            Ajouter un domaine
          </button>
        ` : html`
          <!-- Add/Edit Form Section -->
          <div class="section form-section">
            <div class="section-header">
              <div class="section-title">
                ${this.editingDomain ? html`
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
                  </svg>
                  Modifier le domaine
                ` : html`
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 5v14M5 12h14"/>
                  </svg>
                  Ajouter un domaine
                `}
              </div>
            </div>

            <div class="form-grid">
            <div class="form-group full-width">
              <label class="form-label">Nom de domaine *</label>
              <input type="text" class="form-input"
                .value=${this.formData.hostname}
                @input=${(e) => this.updateFormField('hostname', e.target.value)}
                placeholder="exemple.com">
            </div>

            <div class="form-group">
              <label class="form-label">Port</label>
              <input type="number" class="form-input"
                .value=${this.formData.port}
                @input=${(e) => this.updateFormField('port', parseInt(e.target.value) || 443)}>
              <span class="form-help">443 par défaut (HTTPS)</span>
            </div>

            <div class="form-group">
              <label class="form-label">Label (optionnel)</label>
              <input type="text" class="form-input"
                .value=${this.formData.label}
                @input=${(e) => this.updateFormField('label', e.target.value)}
                placeholder="Mon Site Web">
            </div>

            <div class="form-group">
              <label class="form-label">Alerte Warning (jours)</label>
              <input type="number" class="form-input"
                .value=${this.formData.warning_days}
                @input=${(e) => this.updateFormField('warning_days', parseInt(e.target.value) || 30)}>
              <span class="form-help">Notification jaune avant expiration</span>
            </div>

            <div class="form-group">
              <label class="form-label">Alerte Critical (jours)</label>
              <input type="number" class="form-input"
                .value=${this.formData.critical_days}
                @input=${(e) => this.updateFormField('critical_days', parseInt(e.target.value) || 14)}>
              <span class="form-help">Notification rouge avant expiration</span>
            </div>
          </div>

            <div class="form-actions">
              <button class="btn btn-secondary" @click=${() => this.cancelEdit()}>
                Annuler
              </button>
              <button class="btn btn-primary" @click=${() => this.saveDomain()} ?disabled=${!this.formData.hostname}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  ${this.editingDomain ? html`
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
                    <polyline points="17 21 17 13 7 13 7 21"/>
                    <polyline points="7 3 7 8 15 8"/>
                  ` : html`
                    <path d="M12 5v14M5 12h14"/>
                  `}
                </svg>
                ${this.editingDomain ? 'Enregistrer' : 'Ajouter'}
              </button>
            </div>
          </div>
        `}

        <!-- Domains List Section -->
        <div class="section">
          <div class="section-header">
            <div class="section-title">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
              </svg>
              Domaines surveillés
            </div>
            <span class="section-badge">${this.domains.length} domaine${this.domains.length > 1 ? 's' : ''}</span>
          </div>

          ${this.loading ? html`
            <div class="loader-container">
              <organic-loader text="Chargement des domaines..."></organic-loader>
            </div>
          ` : this.domains.length === 0 ? html`
            <div class="empty-state">
              <div class="empty-icon">🔒</div>
              <div class="empty-title">Aucun domaine configuré</div>
              <div class="empty-text">Cliquez sur "Ajouter un domaine" pour commencer</div>
            </div>
          ` : html`
            <div class="domains-grid">
              ${this.domains.map(domain => html`
                <div class="domain-card">
                  <div class="domain-card-header">
                    <div class="domain-info">
                      <div class="domain-name">${domain.label || domain.hostname}</div>
                      <div class="domain-host">${domain.hostname}:${domain.port || 443}</div>
                    </div>
                    <span class="domain-status status-${this.getStatusClass(domain)}">
                      ${this.getStatusLabel(domain)}
                    </span>
                  </div>

                  <div class="domain-details">
                    <div class="detail-item">
                      <span class="detail-label">Jours restants</span>
                      <span class="detail-value ${this.getStatusClass(domain)}">
                        ${domain.days_remaining !== null ? `${domain.days_remaining}j` : '-'}
                      </span>
                    </div>
                    <div class="detail-item">
                      <span class="detail-label">Expiration</span>
                      <span class="detail-value">
                        ${domain.expiry_date || '-'}
                      </span>
                    </div>
                    <div class="detail-item">
                      <span class="detail-label">Émetteur</span>
                      <span class="detail-value">
                        ${domain.issuer ? domain.issuer.substring(0, 20) : '-'}
                      </span>
                    </div>
                  </div>

                  <div class="domain-thresholds">
                    <span class="threshold-badge threshold-warning">
                      ⚠️ Warning: ${domain.warning_days || 30}j
                    </span>
                    <span class="threshold-badge threshold-critical">
                      🔴 Critical: ${domain.critical_days || 14}j
                    </span>
                  </div>

                  <div class="domain-actions">
                    <button class="edit-btn" @click=${() => this.editDomain(domain)}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
                      </svg>
                      Modifier
                    </button>
                    <button class="delete-btn" @click=${() => this.deleteDomain(domain)}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                      </svg>
                      Supprimer
                    </button>
                  </div>
                </div>
              `)}
            </div>
          `}
        </div>
      </div>
    `
  }
}

customElements.define('ssl-config-page', SslConfigPage)
