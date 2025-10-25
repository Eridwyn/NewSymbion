/**
 * Widget Paramètres Contexte Symbion
 *
 * Panneau de configuration pour les options contextuelles
 */

import { LitElement, html, css } from 'lit'

class ContextSettingsWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .widget-container {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 16px;
      padding: 1.5rem;
      backdrop-filter: blur(10px);
    }

    .widget-header {
      font-size: 1.25rem;
      font-weight: 700;
      color: #e0e0e0;
      margin-bottom: 1.5rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .settings-section {
      margin-bottom: 1.5rem;
    }

    .section-title {
      font-size: 0.875rem;
      font-weight: 600;
      color: #a0a0a0;
      margin-bottom: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .setting-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.75rem;
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      margin-bottom: 0.5rem;
      transition: all 0.2s ease;
    }

    .setting-item:hover {
      background: rgba(255, 255, 255, 0.05);
      border-color: rgba(255, 255, 255, 0.15);
    }

    .setting-label {
      font-size: 0.875rem;
      color: #e0e0e0;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .setting-description {
      font-size: 0.75rem;
      color: #808080;
    }

    .toggle-switch {
      position: relative;
      width: 48px;
      height: 24px;
    }

    .toggle-switch input {
      opacity: 0;
      width: 0;
      height: 0;
    }

    .toggle-slider {
      position: absolute;
      cursor: pointer;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: rgba(255, 255, 255, 0.1);
      transition: 0.3s;
      border-radius: 24px;
      border: 1px solid rgba(255, 255, 255, 0.2);
    }

    .toggle-slider:before {
      position: absolute;
      content: "";
      height: 16px;
      width: 16px;
      left: 3px;
      bottom: 3px;
      background-color: #808080;
      transition: 0.3s;
      border-radius: 50%;
    }

    input:checked + .toggle-slider {
      background-color: rgba(16, 185, 129, 0.3);
      border-color: #10b981;
    }

    input:checked + .toggle-slider:before {
      background-color: #10b981;
      transform: translateX(24px);
    }

    .select-input {
      padding: 0.5rem;
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.15);
      color: #e0e0e0;
      font-size: 0.875rem;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .select-input:hover {
      background: rgba(255, 255, 255, 0.08);
      border-color: rgba(255, 255, 255, 0.25);
    }

    .select-input:focus {
      outline: none;
      border-color: #10b981;
    }

    .save-button {
      width: 100%;
      padding: 0.75rem;
      border: 1px solid rgba(16, 185, 129, 0.4);
      border-radius: 8px;
      background: linear-gradient(135deg, rgba(16, 185, 129, 0.15) 0%, rgba(16, 185, 129, 0.1) 100%);
      color: #10b981;
      font-size: 0.875rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.3s ease;
    }

    .save-button:hover {
      background: linear-gradient(135deg, rgba(16, 185, 129, 0.25) 0%, rgba(16, 185, 129, 0.2) 100%);
      border-color: rgba(16, 185, 129, 0.6);
      transform: translateY(-2px);
    }

    .saved-message {
      text-align: center;
      color: #10b981;
      font-size: 0.75rem;
      margin-top: 0.5rem;
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .saved-message.show {
      opacity: 1;
    }
  `

  static properties = {
    patternsEnabled: { type: Boolean },
    refreshInterval: { type: Number },
    showProductivity: { type: Boolean },
    showConfirmed: { type: Boolean }
  }

  constructor() {
    super()
    this.loadSettings()
    this.showConfirmed = false
  }

  loadSettings() {
    // Charger depuis localStorage ou valeurs par défaut
    this.patternsEnabled = localStorage.getItem('symbion.patterns.enabled') !== 'false'
    this.refreshInterval = parseInt(localStorage.getItem('symbion.refresh.interval')) || 30
    this.showProductivity = localStorage.getItem('symbion.show.productivity') !== 'false'
  }

  saveSettings() {
    localStorage.setItem('symbion.patterns.enabled', this.patternsEnabled)
    localStorage.setItem('symbion.refresh.interval', this.refreshInterval)
    localStorage.setItem('symbion.show.productivity', this.showProductivity)

    // Afficher confirmation
    this.showConfirmed = true
    setTimeout(() => {
      this.showConfirmed = false
    }, 2000)

    // Dispatch event pour que les autres widgets se mettent à jour
    this.dispatchEvent(new CustomEvent('settings-changed', {
      detail: {
        patternsEnabled: this.patternsEnabled,
        refreshInterval: this.refreshInterval,
        showProductivity: this.showProductivity
      },
      bubbles: true,
      composed: true
    }))
  }

  handleTogglePatterns(e) {
    this.patternsEnabled = e.target.checked
  }

  handleToggleProductivity(e) {
    this.showProductivity = e.target.checked
  }

  handleRefreshChange(e) {
    this.refreshInterval = parseInt(e.target.value)
  }

  render() {
    return html`
      <div class="widget-container">
        <div class="widget-header">
          ⚙️ Paramètres Contextuels
        </div>

        <!-- Section Fonctionnalités -->
        <div class="settings-section">
          <div class="section-title">Fonctionnalités</div>

          <div class="setting-item">
            <div class="setting-label">
              <span>Détection de patterns</span>
              <span class="setting-description">Apprendre vos habitudes de changement de mode</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                ?checked="${this.patternsEnabled}"
                @change="${this.handleTogglePatterns}"
              >
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="setting-item">
            <div class="setting-label">
              <span>Métriques de productivité</span>
              <span class="setting-description">Afficher les notes créées par mode</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                ?checked="${this.showProductivity}"
                @change="${this.handleToggleProductivity}"
              >
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>

        <!-- Section Affichage -->
        <div class="settings-section">
          <div class="section-title">Affichage</div>

          <div class="setting-item">
            <div class="setting-label">
              <span>Intervalle de rafraîchissement</span>
              <span class="setting-description">Fréquence de mise à jour des données</span>
            </div>
            <select
              class="select-input"
              @change="${this.handleRefreshChange}"
              .value="${this.refreshInterval}"
            >
              <option value="10">10 secondes</option>
              <option value="30">30 secondes</option>
              <option value="60">1 minute</option>
              <option value="120">2 minutes</option>
              <option value="300">5 minutes</option>
            </select>
          </div>
        </div>

        <!-- Bouton Sauvegarder -->
        <button class="save-button" @click="${this.saveSettings}">
          💾 Sauvegarder les paramètres
        </button>
        <div class="saved-message ${this.showConfirmed ? 'show' : ''}">
          ✓ Paramètres sauvegardés
        </div>
      </div>
    `
  }
}

customElements.define('context-settings-widget', ContextSettingsWidget)

export { ContextSettingsWidget }
