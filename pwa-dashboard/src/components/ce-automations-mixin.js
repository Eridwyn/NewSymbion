import automationsService from '../services/automations-service.js'
import csrfService from '../services/csrf-service.js'
import { isEventType, isStateType } from './ce-constants.js'
import { html } from 'lit'

// Import du composant timeline (utilisé dans le template)
import './automation-timeline.js'

export const AutomationsMixin = (Base) => class extends Base {

  // ========== Automation CRUD Actions ==========

  async toggleAutomation(id) {
    try {
      const auto = this.automations.find(a => a.id === id)
      if (!auto) {
        this.showToast('Automation introuvable', 'error')
        return
      }
      const newEnabled = !auto.enabled
      await automationsService.toggleAutomation(id, newEnabled)
      await this.loadAutomations()
      this.showToast(newEnabled ? 'Automation activée' : 'Automation désactivée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to toggle automation:', e)
      this.showToast('Erreur lors de la modification', 'error')
    }
  }

  async deleteAutomation(id) {
    const confirmed = await this.showConfirmDialog({
      icon: '🗑️',
      title: 'Supprimer cette automation ?',
      message: 'Cette action est irréversible. L\'automation sera définitivement supprimée.',
      confirmLabel: 'Supprimer',
      cancelLabel: 'Annuler',
      confirmClass: 'btn-danger'
    })

    if (!confirmed) return

    try {
      await automationsService.deleteAutomation(id)
      await this.loadAutomations()
      this.showToast('Automation supprimée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to delete automation:', e)
      this.showToast('Erreur lors de la suppression', 'error')
    }
  }

  openCreateForm() {
    this.editingAutomation = {
      name: '',
      enabled: true,
      triggers: { operator: 'or', triggers: [] },
      actions: [],
      cooldown_seconds: 60
    }
    // Reset trigger form state
    this.showingTriggerConfig = false
    this.pendingTriggerType = 'mode_change'
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showForm = true
  }

  // Timeline event handlers
  _handleTimelineSlotClick(e) {
    const { hour, day, dayName, automations } = e.detail
    if (automations && automations.length > 0) {
      // Open existing automation
      const autoId = automations[0].id
      const auto = this.automations.find(a => a.id === autoId)
      if (auto) {
        this.openEditForm(auto)
      }
    } else {
      // Create new scheduled automation with preset values
      this.openScheduledAutomation({ startHour: hour, endHour: hour + 3, day, dayName })
    }
  }

  _handleTimelineHighlight(e) {
    this.highlightedAutomationId = e.detail?.id || null
  }

  openScheduledAutomation(preset) {
    const dayNames = ['Dimanche', 'Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi']
    this.editingAutomation = {
      name: `Planning ${preset.dayName || dayNames[preset.day]} ${preset.startHour}h`,
      enabled: true,
      category: 'modes',
      _rules: {
        operator: 'and',
        items: [
          { type: 'scheduled', interval_seconds: 60 },
          { type: 'time_range', start_hour: preset.startHour, end_hour: preset.endHour },
          { type: 'day_of_week', days: [preset.day] }
        ]
      },
      actions: [{ type: 'force_mode', mode: '', reason: 'automation' }],
      cooldown_seconds: 60
    }
    this.showForm = true
  }

  openEditForm(auto) {
    const clone = JSON.parse(JSON.stringify(auto))
    // Migrate old trigger format to new triggers format
    if (!clone.triggers && clone.trigger) {
      clone.triggers = {
        operator: 'or',
        triggers: [clone.trigger]
      }
      delete clone.trigger
    } else if (!clone.triggers) {
      clone.triggers = { operator: 'or', triggers: [] }
    }
    this.editingAutomation = clone
    // Reset trigger form state
    this.showingTriggerConfig = false
    this.pendingTriggerType = 'mode_change'
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showForm = true
  }

  cancelForm() {
    this.showForm = false
    this.editingAutomation = null
  }

  async saveAutomation() {
    const errors = this.validateAutomation(this.editingAutomation)
    if (errors.length > 0) {
      this.showToast(errors[0], 'error')
      return
    }
    try {
      // Split unified rules into triggers and conditions for backend
      if (this.editingAutomation._rules) {
        const { triggers, conditions } = this.splitRulesForBackend(this.editingAutomation._rules)
        this.editingAutomation.triggers = triggers
        this.editingAutomation.conditions = conditions.conditions?.length > 0 ? conditions : null
      }

      // Clean up internal fields before sending
      const autoToSave = { ...this.editingAutomation }
      delete autoToSave._rules

      const isEdit = !!autoToSave.id
      if (autoToSave.id) {
        await automationsService.updateAutomation(autoToSave.id, autoToSave)
      } else {
        await automationsService.createAutomation(autoToSave)
      }
      this.showForm = false
      this.editingAutomation = null
      await this.loadAutomations()
      this.showToast(isEdit ? 'Automation mise à jour' : 'Automation créée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to save automation:', e)
      this.showToast('Erreur: ' + (e.message || 'Erreur inconnue'), 'error')
    }
  }

  validateAutomation(auto) {
    const errors = []
    if (!auto?.name?.trim()) {
      errors.push('Le nom est requis')
    }
    // Check rules - must have at least one event (trigger)
    if (auto._rules) {
      if (!this.hasEventInRules(auto._rules)) {
        errors.push('Au moins un événement (déclencheur) est requis')
      }
    } else {
      // Fallback for legacy automations without _rules
      const triggersGroup = auto?.triggers
      if (!triggersGroup?.triggers?.length) {
        errors.push('Au moins un déclencheur est requis')
      }
    }
    // Check actions
    if (!auto?.actions?.length) {
      errors.push('Au moins une action est requise')
    } else {
      // Validate each action's required fields
      auto.actions.forEach((action, idx) => {
        const actionSchema = this.schema?.actions?.find(a => a.type === action.type)
        if (actionSchema?.fields) {
          actionSchema.fields.forEach(field => {
            if (field.required) {
              const value = action[field.name]
              if (value === undefined || value === null || value === '') {
                errors.push(`Action ${idx + 1} (${actionSchema.label}): ${field.label} est requis`)
              }
            }
          })
        }
      })
    }
    return errors
  }

  // ========== Render Methods ==========

  renderAutomationsTab() {
    if (this.showForm) {
      return this.renderAutomationForm()
    }

    // Get categories from schema with icons
    const categoryIcons = {
      all: '📋', comfort: '🛋️', security: '🔒', energy: '⚡',
      notifications: '🔔', custom: '⚙️'
    }
    const categories = this.schema?.dynamic_values?.categories || []

    // Filter automations by category
    const filteredAutomations = this.categoryFilter === 'all'
      ? this.automations
      : this.automations.filter(a => (a.category || 'custom') === this.categoryFilter)

    const totalEnabled = this.automations.filter(a => a.enabled).length
    const filteredEnabled = filteredAutomations.filter(a => a.enabled).length
    const recentExecutions = this.automationHistory.filter(h => {
      const execTime = new Date(h.executed_at)
      return (Date.now() - execTime) < 24 * 60 * 60 * 1000
    }).length

    return html`
      <!-- Timeline Hebdomadaire -->
      <automation-timeline
        .automations="${this.automations}"
        .modes="${this.schema?.dynamic_values?.modes || []}"
        .highlightedId="${this.highlightedAutomationId}"
        @slot-click="${this._handleTimelineSlotClick}"
        @automation-highlight="${this._handleTimelineHighlight}"
        class="ce-mb-2xl"
      ></automation-timeline>

      <!-- Header Stats -->
      <div class="automations-header">
        <div class="automations-stats">
          <div class="automation-stat">
            <span class="automation-stat-value">${totalEnabled}</span>
            <span class="automation-stat-label">Actives</span>
          </div>
          <div class="automation-stat-divider"></div>
          <div class="automation-stat">
            <span class="automation-stat-value">${this.automations.length}</span>
            <span class="automation-stat-label">Total</span>
          </div>
          <div class="automation-stat-divider"></div>
          <div class="automation-stat">
            <span class="automation-stat-value">${recentExecutions}</span>
            <span class="automation-stat-label">Exécutions 24h</span>
          </div>
        </div>
      </div>

      <!-- Enhanced Category Filter -->
      <div class="category-filter-bar">
        <button
          class="category-pill ${this.categoryFilter === 'all' ? 'active' : ''}"
          @click="${() => { this.categoryFilter = 'all'; this.requestUpdate() }}"
        >
          <span class="category-pill-icon">${categoryIcons.all}</span>
          <span>Toutes</span>
          <span class="category-pill-count">${this.automations.length}</span>
        </button>
        ${categories.map(cat => {
          const count = this.automations.filter(a => (a.category || 'custom') === cat.value).length
          const icon = categoryIcons[cat.value] || '📁'
          return html`
            <button
              class="category-pill ${this.categoryFilter === cat.value ? 'active' : ''}"
              @click="${() => { this.categoryFilter = cat.value; this.requestUpdate() }}"
            >
              <span class="category-pill-icon">${icon}</span>
              <span>${cat.label}</span>
              <span class="category-pill-count">${count}</span>
            </button>
          `
        })}
      </div>

      <!-- Automation Cards -->
      ${filteredAutomations.length === 0 ? this.renderEmptyStateEnhanced() : html`
        <div class="automations-list">
          <!-- Add New Automation Card -->
          <div class="automation-card add-new" @click="${this.openCreateForm}">
            <div class="automation-card-inner ce-flex-center ce-min-h-80 ce-cursor-pointer">
              <div class="ce-text-center">
                <div class="ce-text-2xl-icon">+</div>
                <div class="ce-text-md ce-text-secondary">Nouvelle automation</div>
              </div>
            </div>
          </div>
          ${filteredAutomations.map(auto => this.renderAutomationCard(auto))}
        </div>
      `}

      <!-- Enhanced History Section -->
      ${this.automationHistory.length > 0 ? this.renderHistorySection() : ''}
    `
  }

  renderEmptyStateEnhanced() {
    const suggestions = [
      { icon: '🌙', label: 'Mode nuit auto', action: () => this.createSuggestionAutomation('night') },
      { icon: '🚪', label: 'Notification entrée', action: () => this.createSuggestionAutomation('entry') },
      { icon: '🌡️', label: 'Alerte température', action: () => this.createSuggestionAutomation('temp') }
    ]

    return html`
      <div class="empty-state-enhanced">
        <div class="empty-state-icon-container">⚡</div>
        <div class="empty-state-title">
          ${this.categoryFilter === 'all'
            ? 'Aucune automation configurée'
            : 'Aucune automation dans cette catégorie'}
        </div>
        <div class="empty-state-description">
          Automatisez votre maison en créant des règles intelligentes qui réagissent aux événements.
        </div>
        <button class="btn btn-primary" @click="${this.openCreateForm}">
          Créer une automation
        </button>
        ${this.categoryFilter === 'all' ? html`
          <div class="empty-state-suggestions">
            ${suggestions.map(s => html`
              <button class="suggestion-chip" @click="${s.action}">
                <span>${s.icon}</span>
                <span>${s.label}</span>
              </button>
            `)}
          </div>
        ` : ''}
      </div>
    `
  }

  createSuggestionAutomation(type) {
    // Pre-fill automation based on suggestion type
    const templates = {
      night: {
        name: 'Mode nuit automatique',
        category: 'comfort',
        triggers: { operator: 'or', triggers: [{ type: 'scheduled', start_hour: 23, end_hour: 23 }] },
        actions: [{ type: 'force_mode', mode: 'veille', duration_minutes: 480 }]
      },
      entry: {
        name: 'Notification entrée',
        category: 'security',
        triggers: { operator: 'or', triggers: [{ type: 'agent_status', status: 'online' }] },
        actions: [{ type: 'send_notification', title: 'Arrivée détectée', body: 'Un appareil vient de se connecter' }]
      },
      temp: {
        name: 'Alerte température haute',
        category: 'notifications',
        triggers: { operator: 'or', triggers: [{ type: 'sensor_alert', alert_level: 'warning' }] },
        actions: [{ type: 'send_notification', title: 'Alerte température', body: 'Température anormale détectée' }]
      }
    }

    this.editingAutomation = {
      enabled: true,
      cooldown_seconds: 300,
      conditions: { operator: 'and', conditions: [] },
      ...templates[type]
    }
    this.showForm = true
    this.requestUpdate()
  }

  renderHistorySection() {
    return html`
      <div class="history-section">
        <div class="history-header">
          <div class="history-title">
            <span class="history-title-icon">📜</span>
            Historique récent
          </div>
          <span class="ce-text-xs-tertiary">
            ${this.automationHistory.length} exécution${this.automationHistory.length !== 1 ? 's' : ''}
          </span>
        </div>
        <div class="history-timeline">
          ${this.automationHistory.slice(0, 5).map(h => this.renderHistoryItem(h))}
        </div>
      </div>
    `
  }

  renderHistoryItem(h) {
    const statusClass = h.success ? 'success' : 'failed'
    const statusIcon = h.success ? '✓' : '✗'
    const actionsCount = h.actions_executed || 0

    return html`
      <div class="history-item ${statusClass}">
        <div class="history-item-header">
          <div class="history-item-name">
            <span class="history-item-status">${statusIcon}</span>
            ${h.automation_name}
          </div>
          <span class="history-item-time">${this.formatTime(h.executed_at)}</span>
        </div>
        <div class="history-item-details">
          ${actionsCount > 0 ? html`
            <span>${actionsCount} action${actionsCount !== 1 ? 's' : ''}</span>
          ` : ''}
          ${h.trust_score != null ? html`
            <span class="trust-badge ${this.getTrustClass(h.trust_score)}">
              🧠 ${Math.round(h.trust_score * 100)}%
            </span>
          ` : ''}
          ${h.trigger_type ? html`
            <span class="ce-opacity-7">via ${h.trigger_type}</span>
          ` : ''}
        </div>
      </div>
    `
  }

  // Génère un champ de formulaire basé sur le schema
  renderSchemaField(field, value, onChange) {
    const options = field.options_key ? (this.schema?.dynamic_values?.[field.options_key] || []) : []

    switch (field.field_type) {
      case 'select':
        return html`
          <select class="form-input"
            @change="${e => onChange(e.target.value || null)}">
            ${!field.required
              ? html`<option value="">${field.placeholder || 'Tous'}</option>`
              : html`<option value="" disabled ?selected="${!value}">-- Sélectionner --</option>`
            }
            ${options.map(opt => html`
              <option value="${opt.value}" ?selected="${opt.value === value}">${opt.label}</option>
            `)}
            ${field.name === 'status' && options.length === 0 ? html`
              <option value="online" ?selected="${value === 'online'}">Online</option>
              <option value="offline" ?selected="${value === 'offline'}">Offline</option>
            ` : ''}
          </select>
        `

      case 'multi_select':
        const selectedValues = Array.isArray(value) ? value : []
        return html`
          <div class="multi-select-group">
            ${options.map(opt => html`
              <label class="checkbox-label">
                <input type="checkbox"
                  ?checked="${selectedValues.includes(opt.value)}"
                  @change="${e => {
                    const newVals = e.target.checked
                      ? [...selectedValues, opt.value]
                      : selectedValues.filter(v => v !== opt.value)
                    onChange(newVals)
                  }}">
                ${opt.label}
              </label>
            `)}
          </div>
        `

      case 'number':
        // Detect hour fields for special rendering
        const isHourField = field.name.includes('hour') && field.max <= 24
        if (isHourField) {
          const hours = Array.from({ length: 25 }, (_, i) => i) // 0-24
          const currentVal = value ?? field.default_value ?? ''
          return html`
            <select class="form-input hour-select"
              @change="${e => onChange(e.target.value !== '' ? parseInt(e.target.value) : null)}">
              ${!field.required ? html`<option value="">--</option>` : ''}
              ${hours.map(h => html`
                <option value="${h}" ?selected="${h === currentVal}">${String(h).padStart(2, '0')}:00</option>
              `)}
            </select>
          `
        }
        return html`
          <input type="number" class="form-input"
            .value="${value ?? field.default_value ?? ''}"
            min="${field.min ?? ''}"
            max="${field.max ?? ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value ? parseFloat(e.target.value) : null)}">
        `

      case 'text':
        return html`
          <input type="text" class="form-input"
            .value="${value || ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value)}">
        `

      case 'text_area':
        return html`
          <textarea class="form-input" rows="2"
            .value="${value || ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value)}"></textarea>
        `

      default:
        return html`<input type="text" class="form-input" .value="${value || ''}" @input="${e => onChange(e.target.value)}">`
    }
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
        <label>Catégorie</label>
        <select class="form-input"
          @change="${e => { this.editingAutomation.category = e.target.value; this.requestUpdate() }}">
          ${(this.schema?.dynamic_values?.categories || []).map(cat => html`
            <option value="${cat.value}" ?selected="${(auto.category || 'custom') === cat.value}">${cat.label}</option>
          `)}
        </select>
      </div>

      <div class="form-group">
        <label>Mode cible (apprentissage)</label>
        <select class="form-input"
          @change="${e => { this.editingAutomation.goal_mode = e.target.value || null; this.requestUpdate() }}">
          <option value="" ?selected="${!auto.goal_mode}">Aucun (pas d'apprentissage)</option>
          ${(this.schema?.dynamic_values?.modes || []).map(mode => html`
            <option value="${mode.value}" ?selected="${auto.goal_mode === mode.value}">${mode.label}</option>
          `)}
        </select>
        <small class="ce-text-xs-tertiary ce-mt-xs ce-block">
          Le mode que cette automation vise à atteindre. Permet à l'Intelligence d'apprendre.
        </small>
      </div>

      <!-- Règles Section (Triggers + Conditions unifiés) -->
      ${this.renderRulesSection(auto)}

      <div class="form-group">
        <label>Cooldown (secondes)</label>
        <input type="number" class="form-input" min="0" .value="${auto.cooldown_seconds || 60}"
          @input="${e => this.editingAutomation.cooldown_seconds = parseInt(e.target.value) || 0}">
      </div>

      <div class="form-group">
        <label class="ce-flex">
          <input type="checkbox" ?checked="${auto.enabled !== false}"
            @change="${e => this.editingAutomation.enabled = e.target.checked}">
          Activée
        </label>
      </div>

      <!-- Trust Settings -->
      <div class="form-group ce-bg-trust">
        <label class="ce-text-bold ce-mb-sm ce-block ce-text-context-primary">🛡️ Niveau de confiance</label>

        <label class="ce-flex ce-mb-sm">
          <input type="checkbox" ?checked="${auto.trusted === true}"
            @change="${e => { this.editingAutomation.trusted = e.target.checked; this.requestUpdate() }}">
          <span>Trusted</span>
          <span class="ce-text-tiny-tertiary">— Auto-approuvée sans validation</span>
        </label>

        <label class="ce-flex">
          <input type="checkbox" ?checked="${auto.skip_if_same_mode === true}"
            @change="${e => { this.editingAutomation.skip_if_same_mode = e.target.checked; this.requestUpdate() }}">
          <span>Skip si même mode</span>
          <span class="ce-text-tiny-tertiary">— Ne pas exécuter si déjà dans ce mode</span>
        </label>

        <div class="ce-text-xs-tertiary ce-mt-sm ce-border-top-subtle">
          ${auto.trusted ? html`
            <span class="ce-text-green">✓ Cette automation sera exécutée automatiquement sans demander de validation.</span>
          ` : html`
            <span>Le trust score augmente de +1% à chaque exécution réussie (max +20%).</span>
          `}
        </div>
      </div>

      <!-- Actions Section -->
      <div class="form-group">
        <label>Actions (${auto.actions?.length || 0})</label>
        <div class="ce-flex-col ce-mt-sm">
          ${(auto.actions || []).map((action, idx) => html`
            <div class="action-item ce-flex ce-bg-glass-item">
              <span class="ce-flex-grow ce-text-md">${this.getActionLabel(action)}</span>
              <button class="btn btn-small btn-icon btn-danger" @click="${() => this.removeAction(idx)}" title="Supprimer" aria-label="Supprimer l'action">✕</button>
            </div>
          `)}
        </div>

        <!-- Add Action -->
        <div class="ce-bg-glass-dashed">
          <div class="ce-flex-end-wrap">
            <div class="ce-flex-grow-min150">
              <label class="ce-text-xs-tertiary">Type d'action</label>
              <select class="form-input ce-mt-xs" id="new-action-type">
                ${(this.schema?.actions || []).map(a => html`
                  <option value="${a.type}">${a.icon || ''} ${a.label}</option>
                `)}
              </select>
            </div>
            <button class="btn btn-small ce-whitespace-nowrap" @click="${this.showActionConfig}">+ Configurer</button>
          </div>

          ${this.showingActionConfig ? html`
            <div class="ce-mt-md ce-border-top-faint">
              ${this.renderActionConfig()}
              <div class="ce-flex ce-mt-md">
                <button class="btn btn-small" @click="${() => this.showingActionConfig = false}">Annuler</button>
                <button class="btn btn-small btn-primary" @click="${this.addConfiguredAction}">Ajouter</button>
              </div>
            </div>
          ` : ''}
        </div>
      </div>

      <div class="ce-flex ce-gap-lg ce-mt-xl">
        <button class="btn ce-flex-grow" @click="${this.cancelForm}">Annuler</button>
        <button class="btn btn-primary ce-flex-grow" @click="${this.saveAutomation}">
          ${isEdit ? 'Enregistrer' : 'Créer'}
        </button>
      </div>
    `
  }

  /**
   * Initialize an object with default values from schema fields
   * Handles: default_value, required selects (first option), hour fields (sensible defaults)
   */
  initializeWithDefaults(type, fields) {
    const defaults = { type }

    if (!fields) return defaults

    fields.forEach(field => {
      // Set default value from schema if available
      if (field.default_value !== undefined && field.default_value !== null) {
        defaults[field.name] = field.default_value
      }
      // For required select fields, pre-select the first option
      else if (field.required && field.field_type === 'select' && field.options_key) {
        const options = this.schema?.dynamic_values?.[field.options_key] || []
        if (options.length > 0) {
          defaults[field.name] = options[0].value
        }
      }
      // For required hour fields, set sensible defaults
      else if (field.required && field.field_type === 'number' && field.name.includes('hour')) {
        if (field.name.includes('start')) {
          defaults[field.name] = 8  // Default start: 8h
        } else if (field.name.includes('end')) {
          defaults[field.name] = 18 // Default end: 18h
        }
      }
    })

    return defaults
  }

  getActionLabel(action) {
    switch (action.type) {
      case 'send_notification':
        return `📢 Notif: "${action.title || 'Sans titre'}"`
      case 'force_mode':
        const modeLabel = action.mode || '(non défini)'
        const duration = action.duration_minutes || 60
        return `🎯 Mode: ${modeLabel} (${duration}min)`
      case 'agent_command':
        return `🤖 Agent ${action.agent_id || '?'}: ${action.command_type || '?'}`
      case 'delay':
        return `⏱️ Délai: ${action.seconds || 0}s`
      default:
        return `⚙️ ${action.type || 'inconnu'}`
    }
  }

  showActionConfig() {
    this.pendingActionType = this.shadowRoot.querySelector('#new-action-type')?.value || 'send_notification'
    const actionSchema = this.schema?.actions?.find(a => a.type === this.pendingActionType)
    this.pendingAction = this.initializeWithDefaults(this.pendingActionType, actionSchema?.fields)
    this.showingActionConfig = true
  }

  renderActionConfig() {
    const type = this.pendingActionType
    const actionSchema = this.schema?.actions?.find(a => a.type === type)

    if (!actionSchema || !actionSchema.fields?.length) {
      return html`<div class="ce-text-sm ce-text-tertiary">
        ${actionSchema?.description || 'Aucune configuration requise'}
      </div>`
    }

    return html`
      ${actionSchema.fields.map((field, idx) => html`
        <div class="form-group" style="margin-bottom: ${idx < actionSchema.fields.length - 1 ? '0.5rem' : '0'};">
          <label class="ce-text-xs">${field.label}${field.required ? ' *' : ''}</label>
          ${this.renderSchemaField(field, this.pendingAction?.[field.name], (val) => {
            this.pendingAction = { ...this.pendingAction, [field.name]: val }
            this.requestUpdate()
          })}
        </div>
      `)}
    `
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

  // ========== Unified Rules Section (Triggers + Conditions) ==========

  /**
   * Initialise les règles unifiées à partir des triggers et conditions existants
   */
  initRulesFromAutomation(auto) {
    if (auto._rules) return auto._rules // Déjà initialisé

    const rules = { operator: 'and', items: [] }

    // Ajouter les triggers existants
    const triggersGroup = auto.triggers || { operator: 'or', triggers: [] }
    for (const t of (triggersGroup.triggers || [])) {
      if (t.operator && t.triggers) {
        // Groupe imbriqué
        rules.items.push({
          operator: t.operator,
          items: t.triggers.map(tr => ({ ...tr, _category: 'event' }))
        })
      } else {
        rules.items.push({ ...t, _category: 'event' })
      }
    }

    // Ajouter les conditions existantes
    const conditionsGroup = auto.conditions || { operator: 'and', conditions: [] }
    for (const c of (conditionsGroup.conditions || [])) {
      if (c.operator && c.conditions) {
        // Groupe imbriqué
        rules.items.push({
          operator: c.operator,
          items: c.conditions.map(cd => ({ ...cd, _category: 'state' }))
        })
      } else {
        rules.items.push({ ...c, _category: 'state' })
      }
    }

    return rules
  }

  /**
   * Retourne les types de règles unifiés (triggers + conditions)
   */
  getUnifiedRuleTypes() {
    const triggers = (this.schema?.triggers || []).map(t => ({
      ...t,
      _category: 'event',
      _categoryLabel: '⚡ Événement'
    }))
    const conditions = (this.schema?.conditions || []).map(c => ({
      ...c,
      _category: 'state',
      _categoryLabel: '📋 État'
    }))
    return [...triggers, ...conditions]
  }

  renderRulesSection(auto) {
    // Initialiser les règles si nécessaire
    if (!auto._rules) {
      auto._rules = this.initRulesFromAutomation(auto)
    }
    const rules = auto._rules
    const ruleCount = this.countRules(rules)

    return html`
      <div class="form-group">
        <div class="ce-flex ce-mb-xs">
          <label class="ce-m-0">Règles ${ruleCount > 0 ? `(${ruleCount})` : ''}</label>
          <button
            type="button"
            @click="${() => this.showHelp('rules')}"
            class="ce-btn-help"
          >?</button>
        </div>

        <div class="rules-editor ce-bg-glass ce-p-md">
          ${this.renderRulesGroup(rules, [], 0)}
        </div>
      </div>
    `
  }

  countRules(group) {
    if (!group?.items) return 0
    let count = 0
    for (const item of group.items) {
      if (item.operator && item.items) {
        count += this.countRules(item)
      } else {
        count++
      }
    }
    return count
  }

  renderRulesGroup(group, path, depth) {
    const operatorLabel = group.operator === 'and' ? 'ET' : 'OU'
    const operatorColor = group.operator === 'and' ? '#3b82f6' : '#f59e0b'
    const operatorHint = group.operator === 'and'
      ? 'Toutes les règles doivent correspondre'
      : 'Au moins une règle doit correspondre'
    const indent = depth * 12

    return html`
      <div class="rules-group" style="margin-left: ${indent}px; ${depth > 0 ? 'margin-top: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.02); border-radius: var(--radius-sm); border: 1px dashed rgba(255,255,255,0.1);' : ''}">
        <div class="ce-flex ce-mb-sm">
          <button
            class="btn btn-small"
            class="ce-operator-btn"
            style="background: ${operatorColor};"
            @click="${() => this.toggleRulesGroupOperator(path)}"
            title="Cliquer pour basculer AND/OR"
          >${operatorLabel}</button>
          ${depth > 0 ? html`
            <button class="btn btn-small btn-icon btn-danger ce-badge-tiny" @click="${() => this.removeRulesGroup(path)}" title="Supprimer groupe" aria-label="Supprimer le groupe de règles">✕</button>
          ` : ''}
          <span class="ce-text-tiny-tertiary">
            ${operatorHint}
          </span>
        </div>

        ${group.items?.map((item, idx) => {
          const itemPath = [...path, idx]
          if (item.operator && item.items) {
            // Nested group
            return this.renderRulesGroup(item, itemPath, depth + 1)
          } else {
            // Single rule
            return this.renderRuleItem(item, itemPath)
          }
        })}

        <div class="ce-flex ce-mt-sm">
          <button class="btn btn-small ce-text-tiny" @click="${() => this.showRuleConfigFor(path)}">
            + Règle
          </button>
          ${depth < 1 ? html`
            <button class="btn btn-small ce-text-tiny" @click="${() => this.addRulesGroup(path)}">
              + Groupe
            </button>
          ` : ''}
        </div>

        ${this.showingRuleConfig && JSON.stringify(this.pendingRulePath) === JSON.stringify(path) ? html`
          <div class="ce-bg-glass-item ce-mt-md ce-p-md">
            ${this.renderRuleConfig()}
            <div class="ce-flex ce-mt-sm">
              <button class="btn btn-small" @click="${() => this.showingRuleConfig = false}">Annuler</button>
              <button class="btn btn-small btn-primary" @click="${this.addConfiguredRule}">Ajouter</button>
            </div>
          </div>
        ` : ''}
      </div>
    `
  }

  renderRuleItem(rule, path) {
    const isEvent = isEventType(rule.type)
    const categoryColor = isEvent ? '#10b981' : '#8b5cf6'
    const categoryIcon = isEvent ? '⚡' : '📋'
    const label = this.getRuleLabel(rule)

    return html`
      <div class="rule-item ce-rule-item" style="border-left: 3px solid ${categoryColor};">
        <span class="ce-rule-icon" style="color: ${categoryColor};" title="${isEvent ? 'Événement (déclencheur)' : 'État (condition)'}">${categoryIcon}</span>
        <span class="ce-flex-grow ce-text-sm">${label}</span>
        <button class="btn btn-small btn-icon btn-danger ce-badge-tiny" @click="${() => this.removeRule(path)}" title="Supprimer" aria-label="Supprimer la règle">✕</button>
      </div>
    `
  }

  getRuleLabel(rule) {
    // Chercher dans triggers ou conditions
    const triggerSchema = this.schema?.triggers?.find(t => t.type === rule.type)
    const conditionSchema = this.schema?.conditions?.find(c => c.type === rule.type)
    const schema = triggerSchema || conditionSchema
    const icon = schema?.icon || ''

    // Labels spécifiques selon le type
    switch (rule.type) {
      case 'mode_change':
        const fromMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.from_mode)
        const toMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.to_mode)
        if (rule.from_mode && rule.to_mode) {
          return `${icon} Mode: ${fromMode?.label || rule.from_mode} → ${toMode?.label || rule.to_mode}`
        } else if (rule.to_mode) {
          return `${icon} Mode → ${toMode?.label || rule.to_mode}`
        } else if (rule.from_mode) {
          return `${icon} Mode: ${fromMode?.label || rule.from_mode} → *`
        }
        return `${icon} Changement de mode`
      case 'sensor_alert':
        const room = this.schema?.dynamic_values?.rooms?.find(r => r.value === rule.room_id)
        const level = this.schema?.dynamic_values?.alert_levels?.find(l => l.value === rule.alert_level)
        return `${icon} Alerte ${level?.label || 'capteur'} ${room ? `(${room.label})` : ''}`
      case 'agent_status':
        const agent = this.schema?.dynamic_values?.agents?.find(a => a.value === rule.agent_id)
        return `${icon} Agent ${agent?.label || rule.agent_id || '*'}: ${rule.status || '*'}`
      case 'manual':
        return `${icon} Déclenchement manuel`
      case 'plugin_health':
        const plugin = this.schema?.dynamic_values?.plugins?.find(p => p.value === rule.plugin_name)
        const status = this.schema?.dynamic_values?.plugin_health_statuses?.find(s => s.value === rule.status)
        return `${icon} Plugin ${plugin?.label || rule.plugin_name || '*'}: ${status?.label || rule.status || '*'}`
      case 'scheduled':
        const intervalSecs = rule.interval_seconds || 300
        const intervalLabel = intervalSecs >= 3600
          ? `${Math.round(intervalSecs / 3600)}h`
          : intervalSecs >= 60
            ? `${Math.round(intervalSecs / 60)}min`
            : `${intervalSecs}s`
        const activeHoursLabel = rule.active_hours
          ? ` (${rule.active_hours[0]}h-${rule.active_hours[1]}h)`
          : ''
        return `${icon} Planifié toutes les ${intervalLabel}${activeHoursLabel}`
      case 'current_mode':
        const currentMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.mode)
        return `${icon} Mode actuel = ${currentMode?.label || rule.mode}`
      case 'time_range':
        return `${icon} Heure entre ${rule.start_hour || 0}h et ${rule.end_hour || 24}h`
      case 'day_of_week':
        const dayNames = ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam']
        const days = (rule.days || []).map(d => dayNames[parseInt(d)] || d).join(', ')
        return `${icon} Jour: ${days || 'Aucun'}`
      case 'day_of_month':
        const monthDays = (rule.days || []).map(d => parseInt(d) === 31 ? 'Dernier' : d).join(', ')
        return `${icon} Jour du mois: ${monthDays || 'Aucun'}`
      case 'month':
        const monthNames = ['', 'Jan', 'Fév', 'Mar', 'Avr', 'Mai', 'Juin', 'Juil', 'Août', 'Sep', 'Oct', 'Nov', 'Déc']
        const months = (rule.months || []).map(m => monthNames[parseInt(m)] || m).join(', ')
        return `${icon} Mois: ${months || 'Tous'}`
      case 'sensor_value':
        const sensorRoom = this.schema?.dynamic_values?.rooms?.find(r => r.value === rule.room_id)
        const opLabel = { greater_than: '>', less_than: '<', equals: '=' }[rule.operator] || rule.operator
        return `${icon} ${rule.metric || 'capteur'} ${sensorRoom?.label || ''} ${opLabel} ${rule.value}`
      case 'agent_online':
        const agentCond = this.schema?.dynamic_values?.agents?.find(a => a.value === rule.agent_id)
        return `${icon} Agent ${agentCond?.label || rule.agent_id} ${rule.online ? 'en ligne' : 'hors ligne'}`
      default:
        return `${icon} ${schema?.label || rule.type}`
    }
  }

  showRuleConfigFor(path) {
    this.pendingRulePath = path
    const allTypes = this.getUnifiedRuleTypes()
    this.pendingRuleType = allTypes[0]?.type || 'mode_change'
    const ruleSchema = allTypes.find(t => t.type === this.pendingRuleType)
    this.pendingRule = this.initializeWithDefaults(this.pendingRuleType, ruleSchema?.fields)
    this.showingRuleConfig = true
    this.requestUpdate()
  }

  renderRuleConfig() {
    const type = this.pendingRuleType
    const allTypes = this.getUnifiedRuleTypes()
    const ruleSchema = allTypes.find(t => t.type === type)

    return html`
      <div class="ce-flex-col">
        <div class="form-group ce-mb-0">
          <label class="ce-text-xs">Type de règle</label>
          <select class="form-input ce-text-sm"
            @change="${e => {
              this.pendingRuleType = e.target.value
              const newSchema = allTypes.find(t => t.type === e.target.value)
              this.pendingRule = this.initializeWithDefaults(e.target.value, newSchema?.fields)
              this.requestUpdate()
            }}">
            <optgroup label="⚡ Événements (déclencheurs)">
              ${allTypes.filter(t => t._category === 'event').map(t => html`
                <option value="${t.type}" ?selected="${t.type === type}">${t.icon || ''} ${t.label}</option>
              `)}
            </optgroup>
            <optgroup label="📋 États (conditions)">
              ${allTypes.filter(t => t._category === 'state').map(t => html`
                <option value="${t.type}" ?selected="${t.type === type}">${t.icon || ''} ${t.label}</option>
              `)}
            </optgroup>
          </select>
          ${ruleSchema?.description ? html`
            <div class="ce-text-tiny-tertiary ce-mt-xs">${ruleSchema.description}</div>
          ` : ''}
        </div>

        ${ruleSchema?.fields?.map(field => html`
          <div class="form-group ce-mb-0">
            <label class="ce-text-xs">${field.label}${field.required ? ' *' : ''}</label>
            ${this.renderSchemaField(field, this.pendingRule?.[field.name], (val) => {
              this.pendingRule = { ...this.pendingRule, [field.name]: val }
              this.requestUpdate()
            })}
          </div>
        `)}
      </div>
    `
  }

  addConfiguredRule() {
    if (!this.pendingRule || this.pendingRulePath === null) return

    // Navigate to the right group using path
    let group = this.editingAutomation._rules
    for (const idx of this.pendingRulePath) {
      group = group.items[idx]
    }

    // Transform rule before adding
    let rule = { ...this.pendingRule }

    // Special handling for scheduled trigger: convert active_hours_start/end to tuple
    if (rule.type === 'scheduled') {
      const start = rule.active_hours_start
      const end = rule.active_hours_end
      if (start !== undefined && start !== null && end !== undefined && end !== null) {
        rule.active_hours = [parseInt(start), parseInt(end)]
      }
      delete rule.active_hours_start
      delete rule.active_hours_end
    }

    // Add rule
    if (!group.items) group.items = []
    group.items.push(rule)

    // Reset
    this.pendingRule = null
    this.pendingRulePath = null
    this.showingRuleConfig = false
    this.requestUpdate()
  }

  toggleRulesGroupOperator(path) {
    let group = this.editingAutomation._rules
    for (const idx of path) {
      group = group.items[idx]
    }
    group.operator = group.operator === 'and' ? 'or' : 'and'
    this.requestUpdate()
  }

  addRulesGroup(path) {
    let group = this.editingAutomation._rules
    for (const idx of path) {
      group = group.items[idx]
    }
    if (!group.items) group.items = []
    group.items.push({
      operator: 'and',
      items: []
    })
    this.requestUpdate()
  }

  removeRule(path) {
    const parentPath = path.slice(0, -1)
    const idx = path[path.length - 1]

    let group = this.editingAutomation._rules
    for (const i of parentPath) {
      group = group.items[i]
    }

    group.items = group.items.filter((_, i) => i !== idx)
    this.requestUpdate()
  }

  removeRulesGroup(path) {
    this.removeRule(path) // Same logic
  }

  /**
   * Normalize item fields for backend (convert strings to numbers where needed)
   */
  normalizeForBackend(item) {
    const normalized = { ...item }
    delete normalized._category

    // Convert string arrays to number arrays for specific fields
    if (normalized.days && Array.isArray(normalized.days)) {
      normalized.days = normalized.days.map(d => parseInt(d)).filter(d => !isNaN(d))
    }
    if (normalized.months && Array.isArray(normalized.months)) {
      normalized.months = normalized.months.map(m => parseInt(m)).filter(m => !isNaN(m))
    }

    // Convert hour fields to numbers
    if (normalized.start_hour !== undefined) {
      normalized.start_hour = parseInt(normalized.start_hour)
    }
    if (normalized.end_hour !== undefined) {
      normalized.end_hour = parseInt(normalized.end_hour)
    }
    if (normalized.active_hours_start !== undefined) {
      normalized.active_hours_start = parseInt(normalized.active_hours_start)
    }
    if (normalized.active_hours_end !== undefined) {
      normalized.active_hours_end = parseInt(normalized.active_hours_end)
    }

    // Convert interval to number
    if (normalized.interval_seconds !== undefined) {
      normalized.interval_seconds = parseInt(normalized.interval_seconds)
    }

    return normalized
  }

  /**
   * Split les règles unifiées en triggers et conditions pour le backend
   */
  splitRulesForBackend(rulesGroup) {
    const triggers = { operator: rulesGroup.operator, triggers: [] }
    const conditions = { operator: rulesGroup.operator, conditions: [] }

    for (const item of (rulesGroup.items || [])) {
      if (item.operator && item.items) {
        // Nested group - recursive split
        const nested = this.splitRulesForBackend(item)
        if (nested.triggers.triggers.length > 0) {
          triggers.triggers.push({ operator: item.operator, triggers: nested.triggers.triggers })
        }
        if (nested.conditions.conditions.length > 0) {
          conditions.conditions.push({ operator: item.operator, conditions: nested.conditions.conditions })
        }
      } else if (isEventType(item.type)) {
        // Trigger (event-based)
        triggers.triggers.push(this.normalizeForBackend(item))
      } else if (isStateType(item.type)) {
        // Condition (state-based)
        conditions.conditions.push(this.normalizeForBackend(item))
      }
    }

    return { triggers, conditions }
  }

  /**
   * Vérifie si les règles contiennent au moins un événement (trigger)
   */
  hasEventInRules(rulesGroup) {
    if (!rulesGroup?.items?.length) return false
    for (const item of rulesGroup.items) {
      if (item.operator && item.items) {
        if (this.hasEventInRules(item)) return true
      } else if (isEventType(item.type)) {
        return true
      }
    }
    return false
  }

  // ========== Automation Card & Execution ==========

  renderAutomationCard(auto) {
    // Support both old trigger and new triggers format
    let triggerLabel = 'Aucun'
    let triggerCount = 0
    if (auto.triggers?.triggers?.length > 0) {
      triggerCount = auto.triggers.triggers.length
      if (triggerCount === 1) {
        const t = auto.triggers.triggers[0]
        triggerLabel = this.getShortTriggerLabel(t)
      } else {
        const op = auto.triggers.operator === 'and' ? 'ET' : 'OU'
        triggerLabel = `${triggerCount} déclencheurs (${op})`
      }
    } else if (auto.trigger) {
      triggerCount = 1
      triggerLabel = this.getShortTriggerLabel(auto.trigger)
    }

    // Category info
    const category = auto.category || 'custom'
    const categoryIcons = {
      comfort: '🛋️', security: '🔒', energy: '⚡',
      notifications: '🔔', custom: '⚙️'
    }
    const categoryIcon = categoryIcons[category] || '⚙️'
    const statusIcon = auto.enabled ? '⚡' : '💤'

    // Find last execution for this automation
    const lastExec = this.automationHistory.find(h => h.automation_id === auto.id)
    const lastExecTime = lastExec ? this.formatTime(lastExec.executed_at) : null

    // Check if highlighted from timeline
    const isHighlighted = this.highlightedAutomationId === auto.id

    const actionsCount = auto.actions?.length || 0
    const cooldownLabel = this._formatCooldown(auto.cooldown_seconds || 0)

    return html`
      <div class="automation-card ${auto.enabled ? 'enabled' : 'disabled'} ${isHighlighted ? 'highlighted' : ''}">
        <div class="automation-card-inner">
          <div class="automation-header">
            <div class="automation-status-icon"></div>
            <div class="automation-info">
              <div class="automation-title-row">
                <span class="automation-title">${auto.name}</span>
                ${auto.trusted ? html`<span class="automation-trust-badge" title="Auto-approuvée">🛡️</span>` : ''}
              </div>
              <div class="automation-subtitle">
                <span class="automation-category-badge ${category}">${categoryIcon} ${category}</span>
                <span class="sep">·</span>
                <span>${triggerLabel}</span>
                <span class="sep">·</span>
                <span>${actionsCount} action${actionsCount !== 1 ? 's' : ''}</span>
                ${cooldownLabel !== '0s' ? html`<span class="sep">·</span><span>${cooldownLabel}</span>` : ''}
                ${lastExecTime ? html`<span class="sep">·</span><span>${lastExecTime}</span>` : ''}
              </div>
            </div>
            <div class="automation-actions">
              <div class="automation-quick-actions">
                <button class="quick-action-btn play" @click="${() => this.runAutomationManually(auto.id)}" title="Executer" aria-label="Exécuter l'automatisation">▶</button>
                <button class="quick-action-btn" @click="${() => this.openEditForm(auto)}" title="Modifier" aria-label="Modifier l'automatisation">✏️</button>
                <button class="quick-action-btn" @click="${() => this.deleteAutomation(auto.id)}" title="Supprimer" aria-label="Supprimer l'automatisation">🗑️</button>
              </div>
              <div
                class="toggle ${auto.enabled ? 'active' : ''}"
                @click="${() => this.toggleAutomation(auto.id)}"
                title="${auto.enabled ? 'Desactiver' : 'Activer'}"
              ></div>
            </div>
          </div>
        </div>
      </div>
    `
  }

  async runAutomationManually(automationId) {
    try {
      await csrfService.fetchWithCsrf(`/v1/automations/${automationId}/run`, {
        method: 'POST'
      })
      this.showToast('Automation exécutée', 'success')
      // Refresh history
      await this.loadAutomations()
    } catch (e) {
      console.error('[context-engine] Failed to run automation:', e)
      this.showToast('Erreur lors de l\'exécution', 'error')
    }
  }

  _formatCooldown(seconds) {
    if (!seconds || seconds <= 0) return '0s'
    if (seconds >= 3600) return `${Math.round(seconds / 3600)}h`
    if (seconds >= 60) return `${Math.round(seconds / 60)}min`
    return `${seconds}s`
  }

  getShortTriggerLabel(trigger) {
    if (!trigger?.type) return 'Inconnu'
    switch (trigger.type) {
      case 'mode_change':
        return `Mode: ${trigger.from_mode || '*'} → ${trigger.to_mode || '*'}`
      case 'agent_status':
        return `Agent ${trigger.agent_id || '*'} → ${trigger.status || '*'}`
      case 'sensor_alert':
        return `Capteur ${trigger.room_id || '*'}: ${trigger.alert_level || '*'}`
      case 'manual':
        return 'Manuel'
      case 'plugin_health':
        return `Plugin ${trigger.plugin_name || '*'}: ${trigger.status || '*'}`
      default:
        return trigger.type
    }
  }

  // ========== Help ==========

  showHelp(topic) {
    const helpTexts = {
      rules: `RÈGLES D'AUTOMATION

Combinez des ÉVÉNEMENTS (⚡) et des ÉTATS (📋) :

⚡ ÉVÉNEMENTS (déclencheurs)
  Mode change, Alerte capteur, Agent status, Planifié...
  → Provoquent l'exécution de l'automation

📋 ÉTATS (conditions)
  Mode actuel, Plage horaire, Jour de semaine...
  → Vérifient l'état AVANT d'exécuter

OPÉRATEURS :
• ET (bleu) = Toutes les règles doivent correspondre
• OU (orange) = Au moins une règle doit correspondre

EXEMPLE :
  (⚡ Planifié 5min) ET (📋 Heure 9h-18h) ET (📋 Jour Lun-Ven)

  = Toutes les 5 minutes, SI entre 9h-18h ET jour de semaine,
    alors exécuter les actions.

Au moins un événement (⚡) est requis pour déclencher l'automation.`,

      triggers: `DÉCLENCHEURS

Événements qui lancent l'automation.

• OU (orange) = Au moins un déclencheur doit correspondre
• ET (bleu) = Tous les déclencheurs doivent correspondre

Cliquez sur OU/ET pour basculer.
Utilisez "+ Groupe" pour des combinaisons complexes.

Exemple :
  Mode → Focus OU Agent offline
  = Se déclenche si l'un des deux arrive`,

      conditions: `CONDITIONS (optionnel)

Vérifications supplémentaires AVANT l'exécution.

Exemple :
  Déclencheur : Mode → Focus
  Conditions : Heure 9h-18h ET Jour Lun-Ven

  = L'automation se déclenche quand le mode passe en Focus,
    MAIS seulement si c'est un jour de semaine entre 9h-18h.

• ET (bleu) = Toutes les conditions doivent être vraies
• OU (orange) = Au moins une condition doit être vraie`
    }

    alert(helpTexts[topic] || 'Aide non disponible')
  }
}
