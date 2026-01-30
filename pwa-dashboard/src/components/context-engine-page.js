/**
 * Context Engine Page - Page Unifiée
 *
 * Fusionne Context Engine + Automations + Validations + Stats + Config
 * en une seule interface cohérente.
 */

import { LitElement, html, css } from 'lit'
import csrfService from '../services/csrf-service.js'
import automationsService from '../services/automations-service.js'

class ContextEnginePage extends LitElement {
  static styles = css`
    :host {
      position: fixed;
      inset: 0;
      z-index: 9999;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      animation: fadeIn 0.2s ease-out;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    .page {
      width: 95%;
      max-width: 800px;
      max-height: 90vh;
      background: linear-gradient(135deg, rgba(19, 20, 26, 0.98) 0%, rgba(10, 10, 11, 1) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: 16px;
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6),
                  0 0 80px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      overflow: hidden;
      display: flex;
      flex-direction: column;
      animation: scaleIn 0.25s ease-out;
    }

    @keyframes scaleIn {
      from { opacity: 0; transform: scale(0.95); }
      to { opacity: 1; transform: scale(1); }
    }

    /* Header */
    .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 1.25rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(0, 0, 0, 0.3);
    }

    .header-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .close-btn {
      background: rgba(255, 255, 255, 0.08);
      border: none;
      color: var(--color-dark-text-secondary, #adb5bd);
      width: 36px;
      height: 36px;
      border-radius: 50%;
      cursor: pointer;
      font-size: 1.2rem;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .close-btn:hover {
      background: rgba(255, 255, 255, 0.15);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Tabs */
    .tabs {
      display: flex;
      gap: 0.25rem;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      background: rgba(0, 0, 0, 0.2);
      overflow-x: auto;
    }

    .tab {
      padding: 0.5rem 1rem;
      border-radius: 8px;
      background: transparent;
      border: 1px solid transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }

    .tab:hover {
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .tab.active {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .tab .badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 18px;
      height: 18px;
      padding: 0 5px;
      margin-left: 6px;
      border-radius: 9px;
      background: rgba(239, 68, 68, 0.8);
      color: white;
      font-size: 0.65rem;
      font-weight: 700;
    }

    /* Content */
    .content {
      flex: 1;
      overflow-y: auto;
      padding: 1.25rem;
    }

    .content::-webkit-scrollbar {
      width: 6px;
    }

    .content::-webkit-scrollbar-track {
      background: transparent;
    }

    .content::-webkit-scrollbar-thumb {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      border-radius: 3px;
    }

    /* Mode Tab */
    .mode-display {
      text-align: center;
      padding: 2rem 1rem;
    }

    .mode-icon {
      font-size: 4rem;
      margin-bottom: 1rem;
      animation: float 3s ease-in-out infinite;
    }

    @keyframes float {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-8px); }
    }

    .mode-name {
      font-size: 1.5rem;
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 0.5rem;
    }

    .mode-reason {
      font-size: 0.85rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 1.5rem;
    }

    .confidence-bar {
      width: 200px;
      height: 8px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 4px;
      margin: 0 auto 0.5rem;
      overflow: hidden;
    }

    .confidence-fill {
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary, #00d4aa), color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white));
      border-radius: 4px;
      transition: width 0.5s ease;
    }

    .confidence-text {
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .override-info {
      margin: 1.5rem 0;
      padding: 0.75rem 1rem;
      background: rgba(251, 146, 60, 0.1);
      border: 1px solid rgba(251, 146, 60, 0.3);
      border-radius: 8px;
      color: #fb923c;
      font-size: 0.8rem;
    }

    /* Mode Controls */
    .mode-controls {
      margin-top: 2rem;
      padding-top: 1.5rem;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    .controls-title {
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 1rem;
    }

    .mode-buttons {
      display: flex;
      gap: 0.75rem;
      justify-content: center;
      margin-bottom: 1rem;
    }

    .mode-btn {
      padding: 0.75rem 1.25rem;
      border-radius: 10px;
      border: 1px solid rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85rem;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .mode-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.25);
      transform: translateY(-2px);
    }

    .mode-btn.cravate:hover { border-color: #3b82f6; }
    .mode-btn.intime:hover { border-color: #00d4aa; }
    .mode-btn.neutre:hover { border-color: #6b7280; }

    .duration-buttons {
      display: flex;
      gap: 0.5rem;
      justify-content: center;
      margin-bottom: 1rem;
    }

    .duration-btn {
      padding: 0.4rem 0.8rem;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      background: transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.75rem;
      cursor: pointer;
      transition: all 0.2s;
    }

    .duration-btn:hover, .duration-btn.active {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .clear-override-btn {
      padding: 0.5rem 1rem;
      border-radius: 8px;
      border: 1px solid rgba(239, 68, 68, 0.3);
      background: rgba(239, 68, 68, 0.1);
      color: #ef4444;
      font-size: 0.8rem;
      cursor: pointer;
      transition: all 0.2s;
    }

    .clear-override-btn:hover {
      background: rgba(239, 68, 68, 0.2);
    }

    /* Cards */
    .card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 12px;
      padding: 1rem;
      margin-bottom: 0.75rem;
      transition: all 0.2s;
    }

    .card:hover {
      border-color: rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
    }

    .card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .card-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .card-meta {
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .card-actions {
      display: flex;
      gap: 0.5rem;
    }

    /* Buttons */
    .btn {
      padding: 0.5rem 1rem;
      border-radius: 8px;
      border: 1px solid transparent;
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
    }

    .btn-primary {
      background: linear-gradient(135deg, color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .btn-primary:hover {
      background: linear-gradient(135deg, color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 100%);
      transform: translateY(-1px);
    }

    .btn-success {
      background: rgba(34, 197, 94, 0.15);
      border-color: rgba(34, 197, 94, 0.4);
      color: #22c55e;
    }

    .btn-success:hover {
      background: rgba(34, 197, 94, 0.25);
    }

    .btn-danger {
      background: rgba(239, 68, 68, 0.15);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ef4444;
    }

    .btn-danger:hover {
      background: rgba(239, 68, 68, 0.25);
    }

    .btn-small {
      padding: 0.35rem 0.7rem;
      font-size: 0.7rem;
    }

    .btn-icon {
      padding: 0.4rem;
      min-width: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    /* Toggle */
    .toggle {
      position: relative;
      width: 40px;
      height: 22px;
      background: rgba(255, 255, 255, 0.15);
      border-radius: 11px;
      cursor: pointer;
      transition: background 0.2s;
    }

    .toggle.active {
      background: var(--context-primary, #00d4aa);
    }

    .toggle::after {
      content: '';
      position: absolute;
      top: 2px;
      left: 2px;
      width: 18px;
      height: 18px;
      background: white;
      border-radius: 50%;
      transition: transform 0.2s;
    }

    .toggle.active::after {
      transform: translateX(18px);
    }

    /* Trust Badge */
    .trust-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding: 0.2rem 0.5rem;
      border-radius: 6px;
      font-size: 0.7rem;
      font-weight: 600;
    }

    .trust-badge.high {
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
    }

    .trust-badge.medium {
      background: rgba(251, 191, 36, 0.15);
      color: #fbbf24;
    }

    .trust-badge.low {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
    }

    /* Stats */
    .stat-bar {
      margin-bottom: 1rem;
    }

    .stat-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .stat-label {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .stat-value {
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .stat-track {
      height: 8px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 4px;
      overflow: hidden;
    }

    .stat-fill {
      height: 100%;
      border-radius: 4px;
      transition: width 0.5s ease;
    }

    .stat-fill.cravate { background: linear-gradient(90deg, #2563eb, #3b82f6); }
    .stat-fill.intime { background: linear-gradient(90deg, #059669, #00d4aa); }
    .stat-fill.neutre { background: linear-gradient(90deg, #4b5563, #6b7280); }

    /* Config */
    .config-section {
      margin-bottom: 1.5rem;
    }

    .config-title {
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 1rem;
    }

    .config-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.75rem 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .config-label {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .config-input {
      width: 80px;
      padding: 0.4rem 0.6rem;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.8rem;
      text-align: center;
    }

    .config-input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
    }

    /* Empty state */
    .empty-state {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 0.9rem;
      margin-bottom: 1rem;
    }

    /* Form */
    .form-group {
      margin-bottom: 1rem;
    }

    .form-group label {
      display: block;
      font-size: 0.8rem;
      font-weight: 500;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 0.4rem;
    }

    .form-input {
      width: 100%;
      padding: 0.6rem 0.8rem;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 8px;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.9rem;
      transition: all 0.2s;
    }

    .form-input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
    }

    select.form-input {
      cursor: pointer;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 3rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* Mobile */
    @media (max-width: 600px) {
      .page {
        width: 100%;
        height: 100%;
        max-height: 100vh;
        border-radius: 0;
      }

      .tabs {
        padding: 0.5rem;
      }

      .tab {
        padding: 0.4rem 0.75rem;
        font-size: 0.75rem;
      }

      .mode-buttons {
        flex-wrap: wrap;
      }
    }
  `

  static properties = {
    activeTab: { type: String },
    contextState: { type: Object },
    automations: { type: Array },
    automationHistory: { type: Array },
    validations: { type: Array },
    stats: { type: Object },
    patterns: { type: Array },
    config: { type: Object },
    schema: { type: Object },
    loading: { type: Boolean },
    selectedDuration: { type: Number },
    showForm: { type: Boolean },
    editingAutomation: { type: Object },
    // Action form state
    showingActionConfig: { type: Boolean },
    pendingActionType: { type: String },
    pendingAction: { type: Object },
    // Config help toggle
    showConfigHelp: { type: Boolean },
  }

  constructor() {
    super()
    this.activeTab = 'mode'
    this.contextState = null
    this.automations = []
    this.automationHistory = []
    this.validations = []
    this.stats = null
    this.patterns = []
    this.config = {
      impact_thresholds: { low: 0.3, medium: 0.5, high: 0.7, very_high: 0.9 },
      initial_trust_score: 0.5
    }
    this.schema = null
    this.loading = true
    this.selectedDuration = 60
    this.showForm = false
    this.editingAutomation = null
    // Action form state
    this.showingActionConfig = false
    this.pendingActionType = 'send_notification'
    this.pendingAction = null
    this.showConfigHelp = false
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadAllData()
    document.addEventListener('keydown', this._handleKeydown = (e) => {
      if (e.key === 'Escape') this.close()
    })

    // Écouter les nouvelles notifications pour rafraîchir les validations
    this._notificationHandler = (e) => {
      const notif = e.detail?.notification
      // Si c'est une notification de validation, rafraîchir
      if (notif?.title?.includes('Validation') || notif?.title?.includes('validation')) {
        console.log('[context-engine] Validation notification received, refreshing...')
        this.loadValidations()
        this.loadAutomations() // Aussi l'historique
      }
    }
    document.body.addEventListener('notification-received', this._notificationHandler)

    // Rafraîchir périodiquement les validations (toutes les 10s)
    this._refreshInterval = setInterval(() => {
      if (this.activeTab === 'validations') {
        this.loadValidations()
      }
    }, 10000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this._handleKeydown)
    if (this._notificationHandler) {
      document.body.removeEventListener('notification-received', this._notificationHandler)
    }
    if (this._refreshInterval) {
      clearInterval(this._refreshInterval)
    }
  }

  async loadAllData() {
    this.loading = true
    try {
      await Promise.all([
        this.loadContext(),
        this.loadAutomations(),
        this.loadValidations(),
        this.loadStats(),
        this.loadConfig(),
      ])
    } catch (e) {
      console.error('[context-engine] Failed to load data:', e)
    }
    this.loading = false
  }

  async loadContext() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.contextState = await apiService.request('/v1/context/current')
    } catch (e) {
      console.error('[context-engine] Failed to load context:', e)
    }
  }

  async loadAutomations() {
    try {
      this.automations = await automationsService.fetchAutomations()
      this.automationHistory = await automationsService.fetchHistory(20)
      this.schema = await automationsService.fetchSchema()
    } catch (e) {
      console.error('[context-engine] Failed to load automations:', e)
    }
  }

  async loadValidations() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.validations = await apiService.request('/v1/decision/validations/pending')
      if (!Array.isArray(this.validations)) this.validations = []
    } catch (e) {
      console.error('[context-engine] Failed to load validations:', e)
      this.validations = []
    }
  }

  async loadStats() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const [stats, patterns] = await Promise.all([
        apiService.request('/v1/context/stats'),
        apiService.request('/v1/context/patterns'),
      ])
      this.stats = stats
      this.patterns = patterns
    } catch (e) {
      console.error('[context-engine] Failed to load stats:', e)
    }
  }

  async loadConfig() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.config = await apiService.request('/v1/decision/config')
    } catch (e) {
      // Config endpoint might not exist yet - use defaults
      console.log('[context-engine] Using default config')
    }
  }

  close() {
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }))
  }

  // Mode actions
  async setModeOverride(mode) {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/context/override', {
        method: 'POST',
        body: JSON.stringify({
          mode,
          duration_minutes: this.selectedDuration,
          reason: 'Override manuel via Decision Engine'
        })
      })
      if (res.ok) {
        this.contextState = await res.json()
        // Dispatch event pour mettre à jour le dashboard
        document.body.dispatchEvent(new CustomEvent('context-change', {
          detail: { context: this.contextState }
        }))
      }
    } catch (e) {
      console.error('[context-engine] Failed to set override:', e)
    }
  }

  async clearOverride() {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/context/clear', { method: 'POST' })
      if (res.ok) {
        await this.loadContext()
        // Dispatch event pour mettre à jour le dashboard
        document.body.dispatchEvent(new CustomEvent('context-change', {
          detail: { context: this.contextState }
        }))
      }
    } catch (e) {
      console.error('[context-engine] Failed to clear override:', e)
    }
  }

  // Automation actions
  async toggleAutomation(id) {
    try {
      await automationsService.toggleAutomation(id)
      await this.loadAutomations()
    } catch (e) {
      console.error('[context-engine] Failed to toggle automation:', e)
    }
  }

  async deleteAutomation(id) {
    if (!confirm('Supprimer cette automation ?')) return
    try {
      await automationsService.deleteAutomation(id)
      await this.loadAutomations()
    } catch (e) {
      console.error('[context-engine] Failed to delete automation:', e)
    }
  }

  openCreateForm() {
    this.editingAutomation = {
      name: '',
      enabled: true,
      trigger: { type: 'mode_change', from_mode: null, to_mode: null },
      actions: [],
      cooldown_seconds: 60
    }
    this.showForm = true
  }

  openEditForm(auto) {
    this.editingAutomation = JSON.parse(JSON.stringify(auto))
    this.showForm = true
  }

  cancelForm() {
    this.showForm = false
    this.editingAutomation = null
  }

  async saveAutomation() {
    if (!this.editingAutomation?.name) {
      alert('Le nom est requis')
      return
    }
    try {
      if (this.editingAutomation.id) {
        await automationsService.updateAutomation(this.editingAutomation.id, this.editingAutomation)
      } else {
        await automationsService.createAutomation(this.editingAutomation)
      }
      this.showForm = false
      this.editingAutomation = null
      await this.loadAutomations()
    } catch (e) {
      console.error('[context-engine] Failed to save automation:', e)
      alert('Erreur lors de la sauvegarde')
    }
  }

  // Validation actions
  async approveValidation(id) {
    try {
      const res = await csrfService.fetchWithCsrf(`/v1/decision/validation/${id}/resolve`, {
        method: 'POST',
        body: JSON.stringify({ approved: true, username: 'user' })
      })
      if (res.ok) {
        await this.loadValidations()
      }
    } catch (e) {
      console.error('[context-engine] Failed to approve validation:', e)
    }
  }

  async rejectValidation(id) {
    try {
      const res = await csrfService.fetchWithCsrf(`/v1/decision/validation/${id}/resolve`, {
        method: 'POST',
        body: JSON.stringify({ approved: false, username: 'user' })
      })
      if (res.ok) {
        await this.loadValidations()
      }
    } catch (e) {
      console.error('[context-engine] Failed to reject validation:', e)
    }
  }

  // Config actions
  async saveConfig() {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/decision/config', {
        method: 'PUT',
        body: JSON.stringify(this.config)
      })
      if (res.ok) {
        alert('Configuration sauvegardée')
      }
    } catch (e) {
      console.error('[context-engine] Failed to save config:', e)
    }
  }

  // Helpers
  getModeIcon(mode) {
    const icons = { cravate: '👔', intime: '🏡', neutre: '🌱' }
    return icons[mode?.toLowerCase()] || '🌱'
  }

  getModeName(mode) {
    const names = { cravate: 'Focus Pro', intime: 'Maison', neutre: 'Veille' }
    return names[mode?.toLowerCase()] || 'Inconnu'
  }

  getTrustClass(score) {
    if (score >= 0.7) return 'high'
    if (score >= 0.4) return 'medium'
    return 'low'
  }

  formatTime(timestamp) {
    if (!timestamp) return 'Jamais'
    const date = new Date(timestamp)
    const now = new Date()
    const diff = (now - date) / 1000
    if (diff < 60) return "À l'instant"
    if (diff < 3600) return `Il y a ${Math.floor(diff / 60)} min`
    if (diff < 86400) return `Il y a ${Math.floor(diff / 3600)}h`
    return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  }

  formatDuration(minutes) {
    if (minutes < 60) return `${minutes}min`
    const h = Math.floor(minutes / 60)
    const m = minutes % 60
    return m > 0 ? `${h}h${m}` : `${h}h`
  }

  // Render methods
  render() {
    return html`
      <div class="page" @click="${e => e.stopPropagation()}">
        <div class="header">
          <span class="header-title">🧠 Decision Engine</span>
          <button class="close-btn" @click="${this.close}">✕</button>
        </div>

        <div class="tabs">
          ${this.renderTab('mode', 'Mode')}
          ${this.renderTab('automations', 'Automations')}
          ${this.renderTab('validations', 'Validations', this.validations.length)}
          ${this.renderTab('stats', 'Stats')}
          ${this.renderTab('config', 'Config')}
        </div>

        <div class="content">
          ${this.loading ? html`<div class="loading">Chargement...</div>` : this.renderActiveTab()}
        </div>
      </div>
    `
  }

  renderTab(id, label, badge = 0) {
    return html`
      <button
        class="tab ${this.activeTab === id ? 'active' : ''}"
        @click="${() => this.switchTab(id)}"
      >
        ${label}
        ${badge > 0 ? html`<span class="badge">${badge}</span>` : ''}
      </button>
    `
  }

  switchTab(id) {
    this.activeTab = id
    // Rafraîchir les données de l'onglet sélectionné
    switch (id) {
      case 'validations':
        this.loadValidations()
        break
      case 'automations':
        this.loadAutomations()
        break
      case 'mode':
        this.loadContext()
        break
      case 'stats':
        this.loadStats()
        break
    }
  }

  renderActiveTab() {
    switch (this.activeTab) {
      case 'mode': return this.renderModeTab()
      case 'automations': return this.renderAutomationsTab()
      case 'validations': return this.renderValidationsTab()
      case 'stats': return this.renderStatsTab()
      case 'config': return this.renderConfigTab()
      default: return html`<div>Onglet inconnu</div>`
    }
  }

  renderModeTab() {
    const state = this.contextState
    if (!state) return html`<div class="empty-state"><div class="empty-icon">⏳</div><div class="empty-text">Chargement du contexte...</div></div>`

    const mode = state.mode?.toLowerCase() || 'neutre'
    const hasOverride = !!state.manual_override

    return html`
      <div class="mode-display">
        <div class="mode-icon">${this.getModeIcon(mode)}</div>
        <div class="mode-name">${this.getModeName(mode)}</div>
        <div class="mode-reason">${state.reason || 'Détection automatique'}</div>

        <div class="confidence-bar">
          <div class="confidence-fill" style="width: ${(state.confidence || 0) * 100}%"></div>
        </div>
        <div class="confidence-text">Confiance: ${Math.round((state.confidence || 0) * 100)}%</div>

        ${hasOverride ? html`
          <div class="override-info">
            ⚠️ Override manuel actif jusqu'à ${new Date(state.manual_override.until).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
          </div>
        ` : ''}

        <div class="mode-controls">
          <div class="controls-title">Contrôle Manuel</div>

          <div class="mode-buttons">
            <button class="mode-btn cravate" @click="${() => this.setModeOverride('cravate')}">
              👔 Focus
            </button>
            <button class="mode-btn intime" @click="${() => this.setModeOverride('intime')}">
              🏡 Maison
            </button>
            <button class="mode-btn neutre" @click="${() => this.setModeOverride('neutre')}">
              🌱 Veille
            </button>
          </div>

          <div class="duration-buttons">
            ${[60, 120, 240, 480].map(d => html`
              <button
                class="duration-btn ${this.selectedDuration === d ? 'active' : ''}"
                @click="${() => this.selectedDuration = d}"
              >
                ${this.formatDuration(d)}
              </button>
            `)}
          </div>

          ${hasOverride ? html`
            <button class="clear-override-btn" @click="${this.clearOverride}">
              🔄 Annuler Override
            </button>
          ` : ''}
        </div>
      </div>
    `
  }

  renderAutomationsTab() {
    if (this.showForm) {
      return this.renderAutomationForm()
    }

    const enabled = this.automations.filter(a => a.enabled).length

    return html`
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
        <div style="font-size: 0.85rem; color: var(--color-dark-text-secondary);">
          ${enabled} active${enabled !== 1 ? 's' : ''} / ${this.automations.length} total
        </div>
        <button class="btn btn-primary" @click="${this.openCreateForm}">
          + Nouvelle
        </button>
      </div>

      ${this.automations.length === 0 ? html`
        <div class="empty-state">
          <div class="empty-icon">⚡</div>
          <div class="empty-text">Aucune automation configurée</div>
          <button class="btn btn-primary" @click="${this.openCreateForm}">Créer une automation</button>
        </div>
      ` : this.automations.map(auto => this.renderAutomationCard(auto))}

      ${this.automationHistory.length > 0 ? html`
        <div style="margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid rgba(255,255,255,0.08);">
          <div class="controls-title">Historique récent</div>
          ${this.automationHistory.slice(0, 5).map(h => this.renderHistoryItem(h))}
        </div>
      ` : ''}
    `
  }

  renderAutomationForm() {
    const auto = this.editingAutomation || {}
    const isEdit = !!auto.id

    return html`
      <div class="controls-title">${isEdit ? 'Modifier' : 'Nouvelle'} automation</div>

      <div class="form-group">
        <label>Nom</label>
        <input type="text" class="form-input" .value="${auto.name || ''}"
          @input="${e => this.editingAutomation.name = e.target.value}">
      </div>

      <div class="form-group">
        <label>Trigger</label>
        <select class="form-input" .value="${auto.trigger?.type || 'mode_change'}"
          @change="${e => this.editingAutomation.trigger = { type: e.target.value }}">
          <option value="mode_change">Changement de mode</option>
          <option value="agent_status">Statut agent</option>
          <option value="sensor_alert">Alerte capteur</option>
        </select>
      </div>

      ${auto.trigger?.type === 'mode_change' ? html`
        <div style="display: flex; gap: 1rem;">
          <div class="form-group" style="flex: 1;">
            <label>De mode</label>
            <select class="form-input" .value="${auto.trigger?.from_mode || ''}"
              @change="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, from_mode: e.target.value || null }}">
              <option value="">Tous</option>
              <option value="cravate">Cravate</option>
              <option value="intime">Intime</option>
              <option value="neutre">Neutre</option>
            </select>
          </div>
          <div class="form-group" style="flex: 1;">
            <label>Vers mode</label>
            <select class="form-input" .value="${auto.trigger?.to_mode || ''}"
              @change="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, to_mode: e.target.value || null }}">
              <option value="">Tous</option>
              <option value="cravate">Cravate</option>
              <option value="intime">Intime</option>
              <option value="neutre">Neutre</option>
            </select>
          </div>
        </div>
      ` : ''}

      ${auto.trigger?.type === 'agent_status' ? html`
        <div style="display: flex; gap: 1rem;">
          <div class="form-group" style="flex: 1;">
            <label>Agent (optionnel)</label>
            <input type="text" class="form-input" placeholder="Laisser vide pour tous"
              .value="${auto.trigger?.agent_id || ''}"
              @input="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, agent_id: e.target.value || null }}">
          </div>
          <div class="form-group" style="flex: 1;">
            <label>Statut</label>
            <select class="form-input" .value="${auto.trigger?.status || 'offline'}"
              @change="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, status: e.target.value }}">
              <option value="online">Online</option>
              <option value="offline">Offline</option>
            </select>
          </div>
        </div>
      ` : ''}

      ${auto.trigger?.type === 'sensor_alert' ? html`
        <div style="display: flex; gap: 1rem;">
          <div class="form-group" style="flex: 1;">
            <label>Pièce</label>
            <input type="text" class="form-input" placeholder="ex: chambre"
              .value="${auto.trigger?.room_id || ''}"
              @input="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, room_id: e.target.value || null }}">
          </div>
          <div class="form-group" style="flex: 1;">
            <label>Niveau d'alerte</label>
            <select class="form-input" .value="${auto.trigger?.alert_level || 'warning'}"
              @change="${e => this.editingAutomation.trigger = { ...this.editingAutomation.trigger, alert_level: e.target.value }}">
              <option value="normal">Normal</option>
              <option value="warning">Attention</option>
              <option value="critical">Critique</option>
            </select>
          </div>
        </div>
      ` : ''}

      <div class="form-group">
        <label>Cooldown (secondes)</label>
        <input type="number" class="form-input" min="0" .value="${auto.cooldown_seconds || 60}"
          @input="${e => this.editingAutomation.cooldown_seconds = parseInt(e.target.value) || 0}">
      </div>

      <div class="form-group">
        <label style="display: flex; align-items: center; gap: 0.5rem;">
          <input type="checkbox" ?checked="${auto.enabled !== false}"
            @change="${e => this.editingAutomation.enabled = e.target.checked}">
          Activée
        </label>
      </div>

      <!-- Actions Section -->
      <div class="form-group">
        <label>Actions (${auto.actions?.length || 0})</label>
        <div style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.5rem;">
          ${(auto.actions || []).map((action, idx) => html`
            <div class="action-item" style="display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.03); border-radius: 8px; border: 1px solid rgba(255,255,255,0.08);">
              <span style="flex: 1; font-size: 0.85rem;">${this.getActionLabel(action)}</span>
              <button class="btn btn-small btn-icon btn-danger" @click="${() => this.removeAction(idx)}" title="Supprimer">✕</button>
            </div>
          `)}
        </div>

        <!-- Add Action -->
        <div style="margin-top: 0.75rem; padding: 0.75rem; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed rgba(255,255,255,0.1);">
          <div style="display: flex; gap: 0.5rem; align-items: flex-end; flex-wrap: wrap;">
            <div style="flex: 1; min-width: 150px;">
              <label style="font-size: 0.75rem; color: var(--color-dark-text-tertiary);">Type d'action</label>
              <select class="form-input" id="new-action-type" style="margin-top: 0.25rem;">
                <option value="send_notification">📢 Notification</option>
                <option value="force_mode">🎯 Forcer mode</option>
                <option value="agent_command">🤖 Commande agent</option>
              </select>
            </div>
            <button class="btn btn-small" @click="${this.showActionConfig}" style="white-space: nowrap;">+ Configurer</button>
          </div>

          ${this.showingActionConfig ? html`
            <div style="margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.08);">
              ${this.renderActionConfig()}
              <div style="display: flex; gap: 0.5rem; margin-top: 0.75rem;">
                <button class="btn btn-small" @click="${() => this.showingActionConfig = false}">Annuler</button>
                <button class="btn btn-small btn-primary" @click="${this.addConfiguredAction}">Ajouter</button>
              </div>
            </div>
          ` : ''}
        </div>
      </div>

      <div style="display: flex; gap: 1rem; margin-top: 1.5rem;">
        <button class="btn" style="flex: 1;" @click="${this.cancelForm}">Annuler</button>
        <button class="btn btn-primary" style="flex: 1;" @click="${this.saveAutomation}">
          ${isEdit ? 'Enregistrer' : 'Créer'}
        </button>
      </div>
    `
  }

  getActionLabel(action) {
    switch (action.type) {
      case 'send_notification':
        return `📢 Notif: "${action.title || 'Sans titre'}"`
      case 'force_mode':
        return `🎯 Mode: ${action.mode} (${action.duration_minutes || 60}min)`
      case 'agent_command':
        return `🤖 Agent ${action.agent_id}: ${action.command_type}`
      default:
        return `⚙️ ${action.type}`
    }
  }

  showActionConfig() {
    this.pendingActionType = this.shadowRoot.querySelector('#new-action-type')?.value || 'send_notification'
    this.pendingAction = { type: this.pendingActionType }
    this.showingActionConfig = true
  }

  renderActionConfig() {
    const type = this.pendingActionType

    if (type === 'send_notification') {
      return html`
        <div class="form-group" style="margin-bottom: 0.5rem;">
          <label style="font-size: 0.75rem;">Titre</label>
          <input type="text" class="form-input" placeholder="Titre notification"
            @input="${e => this.pendingAction.title = e.target.value}">
        </div>
        <div class="form-group" style="margin-bottom: 0.5rem;">
          <label style="font-size: 0.75rem;">Message</label>
          <input type="text" class="form-input" placeholder="Corps du message"
            @input="${e => this.pendingAction.body = e.target.value}">
        </div>
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Priorité</label>
          <select class="form-input" @change="${e => this.pendingAction.priority = e.target.value}">
            <option value="P2">P2 - Normal</option>
            <option value="P1">P1 - Important</option>
            <option value="P0">P0 - Critique</option>
          </select>
        </div>
      `
    }

    if (type === 'force_mode') {
      return html`
        <div class="form-group" style="margin-bottom: 0.5rem;">
          <label style="font-size: 0.75rem;">Mode cible</label>
          <select class="form-input" @change="${e => this.pendingAction.mode = e.target.value}">
            <option value="cravate">👔 Cravate</option>
            <option value="intime">🏡 Intime</option>
            <option value="neutre">🌱 Neutre</option>
          </select>
        </div>
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Durée (minutes)</label>
          <input type="number" class="form-input" value="60" min="1"
            @input="${e => this.pendingAction.duration_minutes = parseInt(e.target.value)}">
        </div>
      `
    }

    if (type === 'agent_command') {
      return html`
        <div class="form-group" style="margin-bottom: 0.5rem;">
          <label style="font-size: 0.75rem;">Agent ID</label>
          <input type="text" class="form-input" placeholder="ex: 345a604068a8"
            @input="${e => this.pendingAction.agent_id = e.target.value}">
        </div>
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Commande</label>
          <select class="form-input" @change="${e => this.pendingAction.command_type = e.target.value}">
            <option value="wake">🔔 Wake</option>
            <option value="notify">📢 Notify</option>
            <option value="lock">🔒 Lock</option>
            <option value="sleep">😴 Sleep</option>
            <option value="shutdown">⛔ Shutdown</option>
          </select>
        </div>
      `
    }

    return html`<div>Configuration non disponible</div>`
  }

  addConfiguredAction() {
    if (!this.pendingAction) return

    // Ensure actions array exists
    if (!this.editingAutomation.actions) {
      this.editingAutomation.actions = []
    }

    // Add the configured action
    this.editingAutomation.actions = [...this.editingAutomation.actions, { ...this.pendingAction }]

    // Reset
    this.pendingAction = null
    this.showingActionConfig = false
    this.requestUpdate()
  }

  removeAction(idx) {
    if (this.editingAutomation?.actions) {
      this.editingAutomation.actions = this.editingAutomation.actions.filter((_, i) => i !== idx)
      this.requestUpdate()
    }
  }

  renderAutomationCard(auto) {
    const trigger = auto.trigger
    let triggerLabel = trigger?.type || 'Inconnu'
    if (trigger?.type === 'mode_change') {
      triggerLabel = `Mode: ${trigger.from_mode || '*'} → ${trigger.to_mode || '*'}`
    } else if (trigger?.type === 'agent_status') {
      triggerLabel = `Agent ${trigger.agent_id || '*'} → ${trigger.status || '*'}`
    } else if (trigger?.type === 'sensor_alert') {
      triggerLabel = `Capteur ${trigger.room_id || '*'}: ${trigger.alert_level || '*'}`
    }

    return html`
      <div class="card">
        <div class="card-header">
          <span class="card-title">${auto.name}</span>
          <div class="card-actions">
            <div
              class="toggle ${auto.enabled ? 'active' : ''}"
              @click="${() => this.toggleAutomation(auto.id)}"
            ></div>
            <button class="btn btn-small btn-icon" @click="${() => this.openEditForm(auto)}" title="Modifier">✏️</button>
            <button class="btn btn-small btn-icon btn-danger" @click="${() => this.deleteAutomation(auto.id)}" title="Supprimer">🗑️</button>
          </div>
        </div>
        <div class="card-meta">
          Trigger: ${triggerLabel}<br>
          Actions: ${auto.actions?.length || 0} • Cooldown: ${auto.cooldown_seconds || 0}s
        </div>
      </div>
    `
  }

  renderHistoryItem(h) {
    return html`
      <div class="card" style="padding: 0.75rem;">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <div>
            <span style="font-size: 0.85rem; color: var(--color-dark-text-primary);">${h.automation_name}</span>
            <span style="margin-left: 0.5rem; font-size: 0.75rem;">${h.success ? '✓' : '✗'}</span>
          </div>
          <div style="display: flex; align-items: center; gap: 0.5rem;">
            ${h.trust_score != null ? html`
              <span class="trust-badge ${this.getTrustClass(h.trust_score)}">
                🧠 ${Math.round(h.trust_score * 100)}%
              </span>
            ` : ''}
            <span class="card-meta">${this.formatTime(h.executed_at)}</span>
          </div>
        </div>
      </div>
    `
  }

  renderValidationsTab() {
    if (this.validations.length === 0) {
      return html`
        <div class="empty-state">
          <div class="empty-icon">✓</div>
          <div class="empty-text">Aucune validation en attente</div>
        </div>
      `
    }

    return html`
      <div class="controls-title">Demandes en attente (${this.validations.length})</div>
      ${this.validations.map(v => this.renderValidationCard(v))}
    `
  }

  renderValidationCard(v) {
    return html`
      <div class="card">
        <div class="card-header">
          <span class="card-title">${v.action?.action_type || 'Action'}</span>
          <span class="trust-badge ${this.getTrustClass(v.trust_score || 0)}">
            🧠 ${Math.round((v.trust_score || 0) * 100)}%
          </span>
        </div>
        <div class="card-meta" style="margin-bottom: 0.75rem;">
          ${v.human_reasons?.join(', ') || 'Validation requise'}<br>
          Seuil: ${Math.round((v.threshold || 0.7) * 100)}%
        </div>
        <div class="card-actions">
          <button class="btn btn-success" @click="${() => this.approveValidation(v.validation_id)}">
            ✓ Approuver
          </button>
          <button class="btn btn-danger" @click="${() => this.rejectValidation(v.validation_id)}">
            ✗ Rejeter
          </button>
        </div>
      </div>
    `
  }

  renderStatsTab() {
    const stats = this.stats?.mode_stats || []
    const total = stats.reduce((sum, s) => sum + (s.duration_minutes || 0), 0)

    return html`
      <div class="controls-title">Temps par mode (24h)</div>
      ${stats.map(s => {
        const pct = total > 0 ? (s.duration_minutes / total) * 100 : 0
        const mode = s.mode?.toLowerCase() || 'neutre'
        return html`
          <div class="stat-bar">
            <div class="stat-header">
              <span class="stat-label">${this.getModeIcon(mode)} ${this.getModeName(mode)}</span>
              <span class="stat-value">${this.formatDuration(s.duration_minutes || 0)} (${Math.round(pct)}%)</span>
            </div>
            <div class="stat-track">
              <div class="stat-fill ${mode}" style="width: ${pct}%"></div>
            </div>
          </div>
        `
      })}

      ${this.patterns.length > 0 ? html`
        <div style="margin-top: 2rem;">
          <div class="controls-title">Patterns détectés</div>
          ${this.patterns.slice(0, 5).map(p => html`
            <div class="card" style="padding: 0.75rem;">
              <div style="display: flex; justify-content: space-between; align-items: center;">
                <span style="font-size: 0.85rem; color: var(--color-dark-text-primary);">
                  ${this.getModeIcon(p.mode)} ${p.description || 'Pattern'}
                </span>
                <span class="trust-badge high">${Math.round((p.confidence || 0) * 100)}%</span>
              </div>
            </div>
          `)}
        </div>
      ` : ''}
    `
  }

  renderConfigTab() {
    return html`
      <!-- Help Section (collapsible) -->
      <div class="config-section" style="background: rgba(99, 102, 241, 0.1); border: 1px solid rgba(99, 102, 241, 0.3);">
        <div class="config-title" style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer; user-select: none;"
          @click="${() => this.showConfigHelp = !this.showConfigHelp}">
          <span style="background: rgba(99, 102, 241, 0.3); width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.85rem;">?</span>
          Comment ça marche ?
          <span style="margin-left: auto; font-size: 0.75rem; opacity: 0.6;">${this.showConfigHelp ? '▼' : '▶'}</span>
        </div>
        ${this.showConfigHelp ? html`
        <div style="font-size: 0.8rem; color: var(--color-dark-text-secondary); line-height: 1.6; margin-top: 0.75rem;">

          <p style="margin: 0.5rem 0 0.75rem;"><strong>1. Calcul du Trust Score</strong> (0.0 à 1.0) :</p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;">Le système évalue 5 critères et fait la moyenne pondérée :</p>
            <div style="display: grid; grid-template-columns: 1fr auto auto; gap: 0.25rem 0.5rem;">
              <span>• Mode & SSID correspondent ?</span><span style="color: #818cf8;">25%</span><span style="color: #6b7280;">→ 0 ou 1</span>
              <span>• Agent en ligne, CPU/RAM ok ?</span><span style="color: #818cf8;">25%</span><span style="color: #6b7280;">→ 0 ou 1</span>
              <span>• Action pas expirée ?</span><span style="color: #818cf8;">20%</span><span style="color: #6b7280;">→ 0 à 1</span>
              <span>• Historique de succès</span><span style="color: #818cf8;">15%</span><span style="color: #6b7280;">→ 0 à 1</span>
              <span>• Tes approbations passées</span><span style="color: #818cf8;">15%</span><span style="color: #6b7280;">→ 0 à 1</span>
            </div>
            <p style="margin: 0.5rem 0 0; font-style: italic;">Score max = 1.0 si tout est parfait.</p>
          </div>

          <p style="margin: 0.5rem 0 0.5rem;"><strong>2. Comment choisir les seuils ?</strong></p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;">Chaque type d'action utilise un seuil selon son niveau d'impact :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.5rem;">
              <span style="color: #10b981;">Low</span><span>→ Notifications (ex: "Tu as reçu un email")</span>
              <span style="color: #3b82f6;">Medium</span><span>→ Changements de mode, ajustements légers</span>
              <span style="color: #f59e0b;">High</span><span>→ Contrôle d'appareils (allumer/éteindre PC)</span>
              <span style="color: #ef4444;">Very High</span><span>→ Actions critiques ou irréversibles</span>
            </div>
          </div>

          <p style="margin: 0.5rem 0;"><strong>3. Règle simple pour configurer :</strong></p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;"><strong>Seuil bas (0.3-0.4)</strong> = Peu exigeant, s'exécute souvent seul</p>
            <p style="margin: 0 0 0.5rem;"><strong>Seuil moyen (0.5-0.6)</strong> = Équilibré, vérifie le contexte</p>
            <p style="margin: 0 0 0.5rem;"><strong>Seuil haut (0.7-0.8)</strong> = Strict, demande validation si doute</p>
            <p style="margin: 0;"><strong>Seuil très haut (0.9+)</strong> = Quasi toujours validation manuelle</p>
          </div>

          <div style="background: rgba(16, 185, 129, 0.15); padding: 0.5rem; border-radius: 6px; border: 1px solid rgba(16, 185, 129, 0.3);">
            <p style="margin: 0; font-size: 0.75rem;">
              <strong>💡 Exemple concret :</strong><br>
              Tu as High = 0.7. Une automation "Allumer PC" calcule un score de 0.66<br>
              → <span style="color: #f59e0b;">0.66 < 0.7</span> = demande ta validation<br>
              Si tu baisses à 0.6, cette même action passera automatiquement.
            </p>
          </div>
        </div>
        ` : ''}
      </div>

      <div class="config-section">
        <div class="config-title">Seuils Trust Score</div>

        <div class="config-row">
          <span class="config-label">Low (notifications)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.low || 0.3}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, low: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">Medium (mode changes)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.medium || 0.5}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, medium: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">High (agent commands)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.high || 0.7}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, high: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">Very High (shutdown/restart)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.very_high || 0.9}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, very_high: parseFloat(e.target.value)}}}"
          >
        </div>
      </div>

      <div class="config-section">
        <div class="config-title">Trust Initial</div>
        <div class="config-row">
          <span class="config-label">Score initial (nouvelles actions)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.initial_trust_score || 0.5}"
            @change="${e => this.config = {...this.config, initial_trust_score: parseFloat(e.target.value)}}"
          >
        </div>
        <div style="font-size: 0.75rem; color: var(--color-dark-text-secondary); margin-top: 0.5rem;">
          Score attribué aux nouvelles automations sans historique
        </div>
      </div>

      <button class="btn btn-primary" @click="${this.saveConfig}" style="width: 100%; margin-top: 1rem;">
        💾 Sauvegarder
      </button>
    `
  }
}

customElements.define('context-engine-page', ContextEnginePage)

export { ContextEnginePage }
