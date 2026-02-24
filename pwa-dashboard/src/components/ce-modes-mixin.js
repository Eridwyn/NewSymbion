import { html } from 'lit'
import csrfService from '../services/csrf-service.js'
import modeTransition from '../animations/mode-transition.js'

export const ModesMixin = (Base) => class extends Base {

  // ============ Mode Change Overlay ============
  showModeChangeOverlay(mode, duration, clickEvent) {
    const theme = this.getModeTheme(mode)
    const origin = clickEvent
      ? { x: clickEvent.clientX, y: clickEvent.clientY }
      : undefined

    modeTransition.play({
      icon: this.getModeIcon(mode),
      name: this.getModeName(mode),
      color: theme.primary,
      duration: this.formatDuration(duration),
      origin
    })
  }

  // Mode actions
  async setModeOverride(mode, clickEvent) {
    try {
      // Show mode change overlay immediately
      this.showModeChangeOverlay(mode, this.selectedDuration, clickEvent)

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
        this.showToast(`Mode ${this.getModeName(mode)} activé pour ${this.formatDuration(this.selectedDuration)}`, 'success')

        // Record feedback for intelligence learning
        // This allows the system to learn from manual mode changes
        csrfService.fetchWithCsrf('/v1/intelligence/feedback', {
          method: 'POST',
          body: JSON.stringify({ chosen_mode: mode })
        }).catch(e => console.log('[context-engine] Feedback recording failed (non-critical):', e))
      } else {
        this.showToast('Erreur lors du changement de mode', 'error')
      }
    } catch (e) {
      console.error('[context-engine] Failed to set override:', e)
      this.showToast('Erreur lors du changement de mode', 'error')
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
        this.showToast('Override annulé - Mode automatique restauré', 'success')
      } else {
        this.showToast('Erreur lors de l\'annulation', 'error')
      }
    } catch (e) {
      console.error('[context-engine] Failed to clear override:', e)
      this.showToast('Erreur lors de l\'annulation', 'error')
    }
  }

  // ============ MODES TAB (Unified: Current Mode + Mode Management) ============

  renderModesTab() {
    const state = this.contextState
    // Prefer mode_slug (dynamic) over mode (legacy enum)
    const mode = state?.mode_slug || state?.mode?.toLowerCase() || 'veille'
    const hasOverride = !!state?.manual_override

    return html`
      <div class="modes-container">
        <!-- Section 1: Mode Actuel -->
        <div class="current-mode-section">
          <div class="section-header">
            <h3>Mode Actuel</h3>
          </div>

          ${!state ? html`
            <div class="loading-state">⏳ Chargement...</div>
          ` : html`
            <div class="current-mode-display">
              <div class="mode-status">
                <span class="current-mode-icon">${this.getModeIcon(mode)}</span>
                <div class="mode-info">
                  <span class="current-mode-name">${this.getModeName(mode)}</span>
                  <span class="mode-reason">${state.reason || 'Détection automatique'}</span>
                </div>
                <!-- Confidence indicator removed - now displayed in Intelligence Widget -->
              </div>

              ${hasOverride ? html`
                <div class="override-banner">
                  ⚠️ Override manuel jusqu'à ${new Date(state.manual_override.until).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
                  <button class="btn btn-sm" @click="${this.clearOverride}">Annuler</button>
                </div>
              ` : ''}

              <div class="quick-mode-controls">
                <div class="mode-buttons-row">
                  ${this.modes.map(m => html`
                    <button
                      class="mode-quick-btn ${mode === m.slug ? 'active' : ''}"
                      style="--btn-color: ${m.theme?.primary || '#6b7280'}"
                      @click="${(e) => this.setModeOverride(m.slug, e)}"
                      title="${m.name}"
                    >
                      ${m.icon} ${m.name}
                    </button>
                  `)}
                </div>
                <div class="duration-selector">
                  ${[60, 120, 240, 480].map(d => html`
                    <button
                      class="duration-btn ${this.selectedDuration === d ? 'active' : ''}"
                      @click="${() => this.selectedDuration = d}"
                    >
                      ${this.formatDuration(d)}
                    </button>
                  `)}
                </div>
              </div>
            </div>
          `}
        </div>

        <!-- Section 2: Gestion des Modes -->
        <div class="modes-management-section">
          <div class="section-header">
            <h3>Gestion des Modes</h3>
            <button class="btn btn-primary btn-sm" @click="${() => this.openModeForm()}">
              + Nouveau Mode
            </button>
          </div>

          <div class="modes-grid">
            ${this.modes.map(mode => this.renderModeCard(mode))}
          </div>
        </div>

        ${this.showModeForm ? this.renderModeForm() : ''}
      </div>
    `
  }

  renderModeCard(mode) {
    return html`
      <div class="mode-card" style="--mode-primary: ${mode.theme?.primary || '#6b7280'}; --mode-bg: ${mode.theme?.background || '#f9fafb'}; --mode-accent: ${mode.theme?.accent || '#4b5563'}">
        <div class="mode-card-header">
          <span class="mode-card-icon">${mode.icon}</span>
          <span class="mode-card-name">${mode.name}</span>
          ${mode.is_system ? html`<span class="system-badge">Système</span>` : ''}
        </div>
        <div class="mode-card-preview">
          <div class="color-preview" style="background: ${mode.theme?.primary}"></div>
          <div class="color-preview" style="background: ${mode.theme?.background}; border: 1px solid rgba(0,0,0,0.1);"></div>
          <div class="color-preview" style="background: ${mode.theme?.accent}"></div>
        </div>
        <div class="mode-card-slug">/${mode.slug}</div>
        <div class="mode-card-actions">
          <button class="btn btn-sm" @click="${() => this.openModeForm(mode)}" title="Modifier" aria-label="Modifier le mode">
            ✏️
          </button>
          ${!mode.is_system ? html`
            <button class="btn btn-sm btn-danger" @click="${() => this.deleteMode(mode.id)}" title="Supprimer" aria-label="Supprimer le mode">
              🗑️
            </button>
          ` : ''}
        </div>
      </div>
    `
  }

  renderModeForm() {
    const isEditing = !!this.editingMode
    return html`
      <div class="modal-overlay" @click="${() => this.closeModeForm()}">
        <div class="mode-form" @click="${e => e.stopPropagation()}">
          <div class="form-header">
            <h3 class="ce-m-0">${isEditing ? 'Modifier le Mode' : 'Nouveau Mode'}</h3>
            <button class="close-button" @click="${() => this.closeModeForm()}">✕</button>
          </div>

          <div class="form-body">
            <div class="form-group">
              <label>Nom du mode</label>
              <input type="text" class="form-input"
                .value="${this.modeFormData.name}"
                @input="${e => this.modeFormData = {...this.modeFormData, name: e.target.value}}"
                placeholder="Ex: Travail, Sport, Lecture..."
              >
            </div>

            <div class="form-group">
              <label>Icône (emoji)</label>
              <div class="emoji-picker">
                ${['🎯', '👔', '🏡', '🌱', '📚', '💪', '🎮', '🎵', '☕', '🌙', '🔥', '💼'].map(emoji => html`
                  <button
                    class="emoji-btn ${this.modeFormData.icon === emoji ? 'selected' : ''}"
                    @click="${() => this.modeFormData = {...this.modeFormData, icon: emoji}}"
                    aria-label="${emoji}"
                  >${emoji}</button>
                `)}
              </div>
            </div>

            <div class="form-group">
              <label>Couleurs du thème</label>
              <div class="color-pickers">
                <div class="color-picker-group">
                  <label>Principale</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.primary}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, primary: e.target.value}}}"
                  >
                </div>
                <div class="color-picker-group">
                  <label>Fond</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.background}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, background: e.target.value}}}"
                  >
                </div>
                <div class="color-picker-group">
                  <label>Accent</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.accent}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, accent: e.target.value}}}"
                  >
                </div>
              </div>
            </div>

            <div class="form-group">
              <label>Aperçu</label>
              <div class="mode-preview" style="--preview-primary: ${this.modeFormData.theme.primary}; --preview-bg: ${this.modeFormData.theme.background}; --preview-accent: ${this.modeFormData.theme.accent}">
                <span class="preview-icon">${this.modeFormData.icon}</span>
                <span class="preview-name">${this.modeFormData.name || 'Nom du mode'}</span>
              </div>
            </div>
          </div>

          <div class="form-actions">
            <button class="btn" @click="${() => this.closeModeForm()}" ?disabled="${this.isSavingMode}">Annuler</button>
            <button class="btn btn-primary ${this.isSavingMode ? 'is-loading' : ''}"
              @click="${() => this.saveMode()}" ?disabled="${this.isSavingMode}">
              ${isEditing ? 'Mettre à jour' : 'Créer'}
            </button>
          </div>
        </div>
      </div>
    `
  }

  openModeForm(mode = null) {
    if (mode) {
      this.editingMode = mode
      this.modeFormData = {
        name: mode.name,
        icon: mode.icon,
        theme: { ...mode.theme }
      }
    } else {
      this.editingMode = null
      this.modeFormData = {
        name: '',
        icon: '🎯',
        theme: { primary: '#2563eb', background: '#f8fafc', accent: '#1e40af' }
      }
    }
    this.showModeForm = true
  }

  closeModeForm() {
    this.showModeForm = false
    this.editingMode = null
  }

  async saveMode() {
    this.isSavingMode = true
    try {
      const isEditing = !!this.editingMode
      const url = isEditing ? `/v1/modes/${this.editingMode.id}` : '/v1/modes'
      const method = isEditing ? 'PUT' : 'POST'

      const res = await csrfService.fetchWithCsrf(url, {
        method,
        body: JSON.stringify(this.modeFormData)
      })

      if (res.ok) {
        console.log(`[context-engine] Mode ${isEditing ? 'updated' : 'created'} successfully`)
        this.closeModeForm()
        await this.loadModes()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to save mode:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to save mode:', e)
      alert(`Erreur: ${e.message}`)
    } finally {
      this.isSavingMode = false
    }
  }

  async deleteMode(id) {
    if (!confirm('Supprimer ce mode ?')) return

    try {
      const res = await csrfService.fetchWithCsrf(`/v1/modes/${id}`, {
        method: 'DELETE'
      })

      if (res.ok) {
        console.log('[context-engine] Mode deleted successfully')
        await this.loadModes()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to delete mode:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to delete mode:', e)
      alert(`Erreur: ${e.message}`)
    }
  }
}
