import { LitElement, html, css } from 'lit'
import './organic-loader.js'
import { sharedAnimations, pageTransitionStyles } from '../styles/shared-animations.js'
import { overlayStyles, scrollbarStyles, statusBadgeStyles, sectionBadgeStyles } from '../styles/shared-patterns.js'
import { pageHeaderStyles } from '../styles/shared-page.js'
import { formInputStyles, formGroupStyles, btnStyles } from '../styles/shared-forms.js'
import { loadingButtonStyles } from '../styles/shared-loading.js'
import { sectionStyles } from '../styles/shared-cards.js'

export class SslConfigPage extends LitElement {
  static properties = {
    domains: { type: Array },
    loading: { type: Boolean },
    editingDomain: { type: Object },
    formData: { type: Object },
    checkingAll: { type: Boolean },
    showAddForm: { type: Boolean },
    isSaving: { type: Boolean },
  }

  static styles = [sharedAnimations, pageTransitionStyles, overlayStyles, scrollbarStyles, pageHeaderStyles, formInputStyles, formGroupStyles, btnStyles, loadingButtonStyles, statusBadgeStyles, sectionBadgeStyles, sectionStyles, css`
    :host {
      z-index: 1000;
      overflow-x: hidden;
      -webkit-overflow-scrolling: touch;
      overscroll-behavior: contain;
    }

    .page-container {
      max-width: 800px;
      margin: var(--space-6) auto;
      padding: var(--space-6);
      padding-bottom: 120px;
      box-sizing: border-box;
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-lg);
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.4);
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
      background: var(--surface-glass-hover, rgba(255,255,255,0.08));
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
      background: var(--ctx-bg, rgba(0,212,170,0.05));
      color: var(--context-primary, #00d4aa);
    }

    .page-title-group {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .page-title {
      font-weight: 600;
    }

    .page-title-icon {
      color: var(--context-primary, #00d4aa);
    }

    .page-subtitle {
      font-size: var(--text-sm);
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .header-actions {
      display: flex;
      gap: 0.75rem;
    }

    /* Section override — SSL-specific simpler gradient */
    .section {
      padding: 1.5rem;
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
    }

    .section-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.25rem;
    }

    .section-title {
      font-size: var(--text-sm);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: var(--space-2);
    }

    /* Domain Grid */
    .domains-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1rem;
    }

    .domain-card {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      padding: 1.25rem;
      transition: all 0.2s ease;
    }

    .domain-card:hover {
      background: var(--surface-glass-hover, rgba(255, 255, 255, 0.06));
      border-color: var(--ctx-bg-intense);
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
      color: var(--color-dark-text-primary, #f8f9fa);
      margin-bottom: 0.25rem;
    }

    .domain-host {
      font-size: 0.8rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-family: 'JetBrains Mono', monospace;
    }

    /* domain-status uses shared statusBadgeStyles (.status-badge.ok/.warning/.critical/.error) */

    .domain-details {
      display: flex;
      gap: 1rem;
      margin-bottom: 1rem;
      padding: 0.75rem;
      background: var(--surface-glass-strong, rgba(0, 0, 0, 0.2));
      border-radius: var(--radius-base);
    }

    .detail-item {
      display: flex;
      flex-direction: column;
      gap: 0.2rem;
    }

    .detail-label {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .detail-value {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-weight: 500;
    }

    .detail-value.warning {
      color: var(--color-warning-text-muted, #fbbf24);
    }

    .detail-value.critical {
      color: var(--color-danger-text-muted, #ff6b6b);
    }

    .domain-thresholds {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
    }

    .threshold-badge {
      font-size: 0.7rem;
      padding: 0.3rem 0.6rem;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      gap: 0.3rem;
    }

    .threshold-warning {
      background: color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 10%, transparent);
      color: var(--color-warning-text-muted, #fbbf24);
      border: 1px solid color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 20%, transparent);
    }

    .threshold-critical {
      background: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 10%, transparent);
      color: var(--color-danger-text-muted, #ff6b6b);
      border: 1px solid color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent);
    }

    .domain-actions {
      display: flex;
      gap: 0.5rem;
      padding-top: 0.75rem;
      border-top: 1px solid var(--border-subtle);
    }

    .domain-actions button {
      flex: 1;
      padding: 0.5rem;
      border-radius: var(--radius-base);
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
      background: color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 10%, transparent);
      border: 1px solid color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 20%, transparent);
      color: var(--color-cyan-text-muted, #00d4ff);
    }

    .edit-btn:hover {
      background: color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 20%, transparent);
    }

    .delete-btn {
      background: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 10%, transparent);
      border: 1px solid color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent);
      color: var(--color-danger-text-muted, #ff6b6b);
    }

    .delete-btn:hover {
      background: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent);
    }

    /* Form Section */
    .form-section {
      background: linear-gradient(135deg, var(--ctx-bg) 0%, var(--ctx-bg-subtle) 100%);
      border: 1px solid var(--ctx-border-medium);
    }

    .form-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 1rem;
    }

    .form-group.full-width {
      grid-column: 1 / -1;
    }

    .form-actions {
      display: flex;
      gap: 0.75rem;
      margin-top: 1.25rem;
      padding-top: 1.25rem;
      border-top: 1px solid var(--ctx-border);
    }

    /* Local btn overrides */
    .btn-primary {
      flex: 1;
    }

    .btn-action {
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 15%, transparent) 0%, color-mix(in srgb, var(--color-cyan-text-muted, #00b4d8) 10%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 30%, transparent);
      color: var(--color-cyan-text-muted, #00d4ff);
    }

    .btn-action:hover {
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-cyan-text-muted, #00d4ff) 25%, transparent) 0%, color-mix(in srgb, var(--color-cyan-text-muted, #00b4d8) 20%, transparent) 100%);
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
      font-size: var(--text-3xl);
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-title {
      font-size: var(--text-lg);
      font-weight: 600;
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-bottom: 0.5rem;
    }

    .empty-text {
      font-size: 0.85rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 3rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .spinner {
      width: 24px;
      height: 24px;
      border: 2px solid var(--ctx-border-medium);
      border-top-color: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-right: 0.75rem;
    }

    /* Add button */
    .add-domain-btn {
      width: 100%;
      padding: 1rem;
      background: var(--ctx-bg);
      border: 2px dashed var(--ctx-bg-intense);
      border-radius: var(--radius-md);
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
      background: var(--ctx-border);
      border-color: var(--ctx-border-intense);
    }

    /* Form section collapsible */
    .form-section {
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
      border: 1px solid var(--ctx-bg-emphasis);
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
    @media (max-width: 768px) {
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
  `]

  constructor() {
    super()
    this.domains = []
    this.loading = true
    this.editingDomain = null
    this.formData = this.getEmptyFormData()
    this.checkingAll = false
    this.showAddForm = false
    this.isSaving = false
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchDomains()

    // Escape key handler to close the page
    this._handleEscape = (e) => {
      if (e.key === 'Escape') {
        this.close()
      }
    }
    document.addEventListener('keydown', this._handleEscape)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this._handleEscape)
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
    // SW injects Authorization header automatically
    return ''
  }

  async fetchDomains() {
    this.loading = true
    const baseUrl = this.getApiBaseUrl()

    console.log('[ssl-config] Fetching domains...', { baseUrl })

    try {
      // SW injects Authorization header automatically
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains`)

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
    console.log('[ssl-config] Edit domain:', domain)
    this.editingDomain = domain
    this.showAddForm = true
    this.formData = {
      hostname: domain.hostname || '',
      port: domain.port || 443,
      label: domain.label || '',
      warning_days: domain.warning_days || 30,
      critical_days: domain.critical_days || 14
    }
    // Scroll vers le haut pour voir le formulaire
    this.scrollTo({ top: 0, behavior: 'smooth' })
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
    this.isSaving = true
    const baseUrl = this.getApiBaseUrl()
    // SW injects Authorization header automatically
    const headers = {
      'Content-Type': 'application/json'
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
    } finally {
      this.isSaving = false
    }
  }

  async deleteDomain(domain) {
    if (!confirm(`Supprimer "${domain.label || domain.hostname}" ?`)) return

    const baseUrl = this.getApiBaseUrl()

    try {
      // SW injects Authorization header automatically
      const response = await fetch(`${baseUrl}/v1/plugin-api/ssl/domains/${domain.id}`, {
        method: 'DELETE'
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
    const baseUrl = this.getApiBaseUrl()

    try {
      // SW injects Authorization header automatically
      await fetch(`${baseUrl}/v1/plugin-api/ssl/check`, {
        method: 'POST'
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
      <div class="page-container" role="dialog" aria-modal="true" aria-label="Configuration SSL">
        <!-- Header -->
        <div class="page-header">
          <div class="header-left">
            <button class="back-btn" @click=${() => this.close()} aria-label="Retour">
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
              <button class="btn btn-primary ${this.isSaving ? 'is-loading' : ''}"
                @click=${() => this.saveDomain()} ?disabled=${!this.formData.hostname || this.isSaving}>
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
                    <span class="status-badge ${this.getStatusClass(domain)}">
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
