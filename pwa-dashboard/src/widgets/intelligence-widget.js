/**
 * Intelligence Widget - Symbion Context Intelligence
 *
 * Affiche les predictions du moteur d'intelligence contextuelle:
 * - Prediction actuelle avec confiance
 * - Facteurs contributifs
 * - Resume des patterns appris
 * - Precision sur 7 jours
 */

import { LitElement, html, css } from 'lit'
import { getDayNameShort, utcHourToLocal } from '../utils/time-utils.js'
import pollingScheduler from '../services/polling-scheduler.js'

class IntelligenceWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .widget {
      background: linear-gradient(135deg,
        rgba(19, 20, 26, 0.95) 0%,
        rgba(10, 10, 11, 0.98) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      border-radius: 16px;
      overflow: hidden;
      transition: all 0.3s ease;
    }

    .widget:hover {
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 35%, transparent);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3),
                  0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
    }

    .header {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 1rem 1.25rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      background: rgba(0, 0, 0, 0.2);
    }

    .header-icon {
      font-size: 1.25rem;
      animation: pulse 2s ease-in-out infinite;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.7; }
    }

    .header-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
    }

    .accuracy-badge {
      padding: 0.25rem 0.5rem;
      border-radius: 12px;
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
      font-size: 0.7rem;
      font-weight: 600;
    }

    .accuracy-badge.low {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
    }

    .accuracy-badge.medium {
      background: rgba(251, 146, 60, 0.15);
      color: #fb923c;
    }

    .content {
      padding: 1.25rem;
    }

    /* Prediction Section */
    .prediction-section {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 1.25rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }

    .prediction-icon {
      font-size: 2.5rem;
      animation: float 3s ease-in-out infinite;
    }

    @keyframes float {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-4px); }
    }

    .prediction-info {
      flex: 1;
    }

    .prediction-mode {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 0.25rem;
    }

    .confidence-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.5rem;
    }

    .confidence-bar {
      flex: 1;
      height: 6px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 3px;
      overflow: hidden;
    }

    .confidence-fill {
      height: 100%;
      border-radius: 3px;
      transition: width 0.5s ease;
    }

    .confidence-fill.high {
      background: linear-gradient(90deg, #22c55e, #4ade80);
    }

    .confidence-fill.medium {
      background: linear-gradient(90deg, #fb923c, #fdba74);
    }

    .confidence-fill.low {
      background: linear-gradient(90deg, #ef4444, #f87171);
    }

    .confidence-text {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--color-dark-text-secondary, #adb5bd);
      min-width: 40px;
      text-align: right;
    }

    .action-indicator {
      font-size: 0.7rem;
      padding: 0.2rem 0.5rem;
      border-radius: 10px;
      display: inline-block;
    }

    .action-indicator.auto-apply {
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
    }

    .action-indicator.suggest {
      background: rgba(251, 146, 60, 0.15);
      color: #fb923c;
    }

    .action-indicator.observe {
      background: rgba(156, 163, 175, 0.15);
      color: #9ca3af;
    }

    /* Reasons Section */
    .reasons {
      margin-bottom: 1rem;
    }

    .reasons-label {
      font-size: 0.7rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 0.5rem;
    }

    .reason-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 0.4rem;
    }

    .reason-chip {
      padding: 0.25rem 0.5rem;
      background: rgba(139, 92, 246, 0.1);
      border: 1px solid rgba(139, 92, 246, 0.2);
      border-radius: 12px;
      font-size: 0.7rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* Factors Section */
    .factors {
      margin-bottom: 1rem;
    }

    .factor-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.4rem;
    }

    .factor-label {
      font-size: 0.7rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      min-width: 80px;
    }

    .factor-bar {
      flex: 1;
      height: 4px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 2px;
      overflow: hidden;
    }

    .factor-fill {
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary, #00d4aa), color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white));
      border-radius: 2px;
      transition: width 0.3s ease;
    }

    .factor-value {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      min-width: 30px;
      text-align: right;
    }

    /* Patterns Summary */
    .patterns-summary {
      padding: 0.75rem;
      background: rgba(255, 255, 255, 0.03);
      border-radius: 10px;
    }

    .patterns-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .patterns-label {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .patterns-count {
      font-size: 0.7rem;
      color: var(--context-primary, #00d4aa);
    }

    /* Advanced section (v1.1.9) */
    .advanced-toggle {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.4rem;
      margin-top: 0.75rem;
      padding: 0.4rem;
      background: transparent;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      color: var(--color-dark-text-tertiary, #6c757d);
      font-size: 0.7rem;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .advanced-toggle:hover {
      border-color: rgba(255, 255, 255, 0.2);
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .advanced-actions {
      display: flex;
      gap: 0.5rem;
      margin-top: 0.5rem;
    }

    .export-btn {
      flex: 1;
      padding: 0.5rem 0.75rem;
      background: rgba(139, 92, 246, 0.1);
      border: 1px solid rgba(139, 92, 246, 0.3);
      border-radius: 8px;
      color: #a78bfa;
      font-size: 0.7rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .export-btn:hover {
      background: rgba(139, 92, 246, 0.2);
      border-color: rgba(139, 92, 246, 0.5);
    }

    .patterns-list {
      display: flex;
      flex-direction: column;
      gap: 0.3rem;
    }

    .pattern-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-size: 0.7rem;
      padding: 0.25rem 0;
    }

    .pattern-mode {
      color: var(--color-dark-text-primary, #f8f9fa);
      font-weight: 500;
    }

    .pattern-when {
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .pattern-confidence {
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
    }

    /* Uncertain state */
    .prediction-section.uncertain {
      opacity: 0.7;
      border: 1px dashed rgba(251, 146, 60, 0.4);
      border-radius: 8px;
      padding: 0.5rem;
      margin: -0.5rem;
      margin-bottom: 0.75rem;
    }

    .uncertain-hint {
      font-size: 0.7rem;
      color: #fb923c;
      font-style: italic;
      margin-bottom: 0.25rem;
    }

    /* Top modes */
    .top-modes {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
      padding: 0.5rem;
      background: rgba(255, 255, 255, 0.02);
      border-radius: 8px;
    }

    .top-mode-item {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.2rem;
      padding: 0.4rem;
      background: rgba(255, 255, 255, 0.03);
      border-radius: 6px;
    }

    .top-mode-item:first-child {
      background: rgba(0, 212, 170, 0.1);
      border: 1px solid rgba(0, 212, 170, 0.2);
    }

    .top-mode-icon {
      font-size: 1rem;
    }

    .top-mode-name {
      font-size: 0.65rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .top-mode-pct {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 2rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.85rem;
    }

    /* Empty state */
    .empty {
      text-align: center;
      padding: 1.5rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      font-size: 0.8rem;
    }

    .empty-icon {
      font-size: 2rem;
      margin-bottom: 0.5rem;
      opacity: 0.5;
    }

    /* Health Section (v1.1.9 P1) */
    .health-section {
      margin-top: 1rem;
      padding: 0.75rem;
      background: rgba(139, 92, 246, 0.05);
      border: 1px solid rgba(139, 92, 246, 0.15);
      border-radius: 10px;
    }

    .health-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.75rem;
    }

    .health-title {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--color-dark-text-secondary, #adb5bd);
      display: flex;
      align-items: center;
      gap: 0.4rem;
    }

    .health-period {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      padding: 0.15rem 0.4rem;
      background: rgba(255, 255, 255, 0.05);
      border-radius: 8px;
    }

    .health-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 0.5rem;
    }

    .health-stat {
      padding: 0.5rem;
      background: rgba(0, 0, 0, 0.2);
      border-radius: 8px;
      text-align: center;
    }

    .health-stat-value {
      font-size: 1.1rem;
      font-weight: 700;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .health-stat-value.success {
      color: #22c55e;
    }

    .health-stat-value.warning {
      color: #fb923c;
    }

    .health-stat-value.na {
      color: var(--color-dark-text-tertiary, #6c757d);
      font-size: 0.9rem;
    }

    .health-stat-label {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-top: 0.2rem;
    }

    .health-divider {
      grid-column: span 2;
      height: 1px;
      background: rgba(255, 255, 255, 0.06);
      margin: 0.25rem 0;
    }

    .accuracy-detail {
      grid-column: span 2;
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.5rem;
      background: rgba(0, 0, 0, 0.2);
      border-radius: 8px;
    }

    .accuracy-info {
      display: flex;
      flex-direction: column;
      gap: 0.2rem;
    }

    .accuracy-main {
      font-size: 1rem;
      font-weight: 700;
    }

    .accuracy-main.na {
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .accuracy-sub {
      font-size: 0.6rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .accuracy-warning {
      font-size: 0.6rem;
      color: #fb923c;
      font-style: italic;
    }
  `

  static properties = {
    intelligenceStatus: { type: Object },
    signals: { type: Object },
    patterns: { type: Array },
    health: { type: Object },
    loading: { type: Boolean },
    showAdvanced: { type: Boolean },
  }

  constructor() {
    super()
    this.intelligenceStatus = null
    this.signals = null
    this.patterns = []
    this.health = null
    this.loading = true
    this.showAdvanced = false
  }

  connectedCallback() {
    super.connectedCallback()
    // Use centralized polling scheduler (auto-pauses when page hidden)
    this._unsubscribePolling = pollingScheduler.subscribe('30s', () => this.loadData())
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._unsubscribePolling) {
      this._unsubscribePolling()
      this._unsubscribePolling = null
    }
  }

  async loadData() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        console.warn('[intelligence-widget] api-service not found')
        return
      }

      // Load status, signals, patterns, and health in parallel
      const [status, signalsData, patternsData, healthData] = await Promise.all([
        apiService.request('/v1/intelligence/status').catch(() => null),
        apiService.request('/v1/intelligence/signals').catch(() => null),
        apiService.request('/v1/intelligence/patterns').catch(() => ({ patterns: [] })),
        apiService.request('/v1/intelligence/health').catch(() => null),
      ])

      this.intelligenceStatus = status
      this.signals = signalsData
      this.patterns = patternsData?.patterns || []
      this.health = healthData
      this.loading = false
    } catch (e) {
      console.error('[intelligence-widget] Failed to load data:', e)
      this.loading = false
    }
  }

  getModeIcon(mode) {
    const icons = {
      'pro': '👔', 'cravate': '👔', 'focus': '🎯',
      'maison': '🏡', 'intime': '🏡', 'home': '🏡',
      'veille': '🌙', 'neutre': '🌙', 'sleep': '🌙',
      'unknown': '❓'
    }
    return icons[mode?.toLowerCase()] || '🎯'
  }

  getModeName(mode) {
    const names = {
      'pro': 'Pro', 'cravate': 'Focus Pro', 'focus': 'Focus',
      'maison': 'Maison', 'intime': 'Maison', 'home': 'Home',
      'veille': 'Veille', 'neutre': 'Veille', 'sleep': 'Sleep',
      'unknown': 'Incertain'
    }
    // Modes custom: capitaliser
    if (!names[mode?.toLowerCase()] && mode) {
      return mode.charAt(0).toUpperCase() + mode.slice(1)
    }
    return names[mode?.toLowerCase()] || mode || 'Inconnu'
  }

  getDayName(day) {
    // Use centralized ISO convention (0=Monday from kernel)
    return getDayNameShort(day)
  }

  getConfidenceClass(confidence) {
    if (confidence >= 0.7) return 'high'
    if (confidence >= 0.4) return 'medium'
    return 'low'
  }

  getAccuracyClass(accuracy) {
    if (accuracy >= 70) return 'high'
    if (accuracy >= 50) return 'medium'
    return 'low'
  }

  getActionIndicator(confidence) {
    if (confidence >= 0.9) {
      return { class: 'auto-apply', text: 'Auto-apply' }
    } else if (confidence >= 0.7) {
      return { class: 'suggest', text: 'Suggerer' }
    }
    return { class: 'observe', text: 'Observer' }
  }

  getFactorLabel(factor) {
    const labels = {
      'temporal': 'Temporel',
      'behavioral': 'Comportement',
      'agent_activity': 'Activite',
      'environment': 'Environnement',
      'momentum': 'Momentum'
    }
    return labels[factor] || factor
  }

  toggleAdvanced() {
    this.showAdvanced = !this.showAdvanced
  }

  async exportPatterns() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return

      const data = await apiService.request('/v1/intelligence/patterns/export')

      // Create and download JSON file
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `symbion-patterns-${new Date().toISOString().split('T')[0]}.json`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
    } catch (e) {
      console.error('[intelligence-widget] Export failed:', e)
    }
  }

  renderHealthSection() {
    if (!this.health) return ''

    const counters = this.health.counters || {}
    const accuracy = this.health.accuracy || {}
    const patternsActive = this.health.patterns_active || 0
    const patternsEstablished = this.health.patterns_established || 0

    // Accuracy display logic: null = N/A (sample too small)
    const hasEnoughSamples = accuracy.accuracy_strict !== null
    const accuracyValue = hasEnoughSamples
      ? `${Math.round(accuracy.accuracy_strict)}%`
      : 'N/A'
    const accuracyClass = hasEnoughSamples
      ? (accuracy.accuracy_strict >= 70 ? 'success' : 'warning')
      : 'na'

    return html`
      <div class="health-section">
        <div class="health-header">
          <span class="health-title">📊 Health</span>
          <span class="health-period">24h</span>
        </div>

        <div class="health-grid">
          <!-- 24h Counters -->
          <div class="health-stat">
            <div class="health-stat-value">${counters.push_sent || 0}</div>
            <div class="health-stat-label">Push envoyés</div>
          </div>
          <div class="health-stat">
            <div class="health-stat-value">${counters.suggestions_generated || 0}</div>
            <div class="health-stat-label">Suggestions</div>
          </div>
          <div class="health-stat">
            <div class="health-stat-value ${counters.auto_applied > 0 ? 'success' : ''}">${counters.auto_applied || 0}</div>
            <div class="health-stat-label">Auto-apply</div>
          </div>
          <div class="health-stat">
            <div class="health-stat-value ${counters.denied > 0 ? 'warning' : ''}">${counters.denied || 0}</div>
            <div class="health-stat-label">Refusés</div>
          </div>

          <div class="health-divider"></div>

          <!-- Patterns -->
          <div class="health-stat">
            <div class="health-stat-value">${patternsActive}</div>
            <div class="health-stat-label">Patterns actifs</div>
          </div>
          <div class="health-stat">
            <div class="health-stat-value ${patternsEstablished > 0 ? 'success' : ''}">${patternsEstablished}</div>
            <div class="health-stat-label">Établis</div>
          </div>

          <div class="health-divider"></div>

          <!-- Accuracy detail -->
          <div class="accuracy-detail">
            <div class="accuracy-info">
              <span class="accuracy-main ${accuracyClass}">${accuracyValue}</span>
              <span class="accuracy-sub">${accuracy.predictions_total || 0} prédictions (7j)</span>
              ${!hasEnoughSamples ? html`
                <span class="accuracy-warning">Échantillon faible (&lt;${accuracy.min_sample_size || 20})</span>
              ` : ''}
            </div>
            <span class="health-stat-label">Précision</span>
          </div>
        </div>
      </div>
    `
  }

  render() {
    if (this.loading) {
      return html`
        <div class="widget">
          <div class="header">
            <span class="header-icon">🧠</span>
            <span class="header-title">Intelligence Contextuelle</span>
          </div>
          <div class="loading">Analyse en cours...</div>
        </div>
      `
    }

    const prediction = this.signals?.prediction
    const status = this.intelligenceStatus?.status
    const accuracy = Math.round((status?.accuracy_7_days || 0) * 100)

    if (!prediction) {
      return html`
        <div class="widget">
          <div class="header">
            <span class="header-icon">🧠</span>
            <span class="header-title">Intelligence Contextuelle</span>
          </div>
          <div class="empty">
            <div class="empty-icon">🔮</div>
            <div>Pas de prediction disponible</div>
          </div>
        </div>
      `
    }

    const confidenceClass = this.getConfidenceClass(prediction.confidence)
    const actionIndicator = this.getActionIndicator(prediction.confidence)
    const topPatterns = this.patterns.slice(0, 4)

    return html`
      <div class="widget">
        <div class="header">
          <span class="header-icon">🧠</span>
          <span class="header-title">Intelligence Contextuelle</span>
          <span class="accuracy-badge ${this.getAccuracyClass(accuracy)}">
            ${accuracy}% precision
          </span>
        </div>

        <div class="content">
          <!-- Prediction Section -->
          <div class="prediction-section ${prediction.is_uncertain ? 'uncertain' : ''}">
            <div class="prediction-icon">${this.getModeIcon(prediction.mode)}</div>
            <div class="prediction-info">
              <div class="prediction-mode">${this.getModeName(prediction.mode)}</div>
              ${prediction.is_uncertain ? html`
                <div class="uncertain-hint">Signaux contradictoires</div>
              ` : html`
                <div class="confidence-row">
                  <div class="confidence-bar">
                    <div class="confidence-fill ${confidenceClass}"
                         style="width: ${prediction.confidence * 100}%"></div>
                  </div>
                  <span class="confidence-text">${Math.round(prediction.confidence * 100)}%</span>
                </div>
              `}
              <span class="action-indicator ${actionIndicator.class}">
                ${actionIndicator.text}
              </span>
            </div>
          </div>

          <!-- Top 3 Modes (scores normalisés) -->
          ${prediction.top_modes?.length > 0 ? html`
            <div class="top-modes">
              ${prediction.top_modes.map(([mode, pct]) => html`
                <div class="top-mode-item">
                  <span class="top-mode-icon">${this.getModeIcon(mode)}</span>
                  <span class="top-mode-name">${this.getModeName(mode)}</span>
                  <span class="top-mode-pct">${Math.round(pct)}%</span>
                </div>
              `)}
            </div>
          ` : ''}

          <!-- Reasons -->
          ${prediction.reasons?.length > 0 ? html`
            <div class="reasons">
              <div class="reasons-label">Raisons</div>
              <div class="reason-chips">
                ${prediction.reasons.map(r => html`
                  <span class="reason-chip">${r}</span>
                `)}
              </div>
            </div>
          ` : ''}

          <!-- Contributing Factors -->
          ${prediction.contributing_factors?.length > 0 ? html`
            <div class="factors">
              ${prediction.contributing_factors.map(([factor, weight]) => html`
                <div class="factor-row">
                  <span class="factor-label">${this.getFactorLabel(factor)}</span>
                  <div class="factor-bar">
                    <div class="factor-fill" style="width: ${weight * 100}%"></div>
                  </div>
                  <span class="factor-value">${Math.round(weight * 100)}%</span>
                </div>
              `)}
            </div>
          ` : ''}

          <!-- Patterns Summary -->
          ${topPatterns.length > 0 ? html`
            <div class="patterns-summary">
              <div class="patterns-header">
                <span class="patterns-label">Patterns appris</span>
                <span class="patterns-count">${this.patterns.length} total</span>
              </div>
              <div class="patterns-list">
                ${topPatterns.map(p => html`
                  <div class="pattern-item">
                    <span class="pattern-mode">
                      ${this.getModeIcon(p.mode)} ${this.getModeName(p.mode)}
                    </span>
                    <span class="pattern-when">
                      ${this.getDayName(p.day_of_week)} ${utcHourToLocal(p.hour)}h
                    </span>
                    <span class="pattern-confidence">
                      ${Math.round(p.confidence * 100)}%
                    </span>
                  </div>
                `)}
              </div>

              <!-- Advanced toggle (v1.1.9) -->
              <button class="advanced-toggle" @click=${this.toggleAdvanced}>
                ${this.showAdvanced ? '▼' : '▶'} Avancé
              </button>

              ${this.showAdvanced ? html`
                <div class="advanced-actions">
                  <button class="export-btn" @click=${this.exportPatterns}>
                    📥 Exporter JSON
                  </button>
                </div>
              ` : ''}
            </div>
          ` : ''}

          <!-- Health Section (v1.1.9 P1) -->
          ${this.renderHealthSection()}
        </div>
      </div>
    `
  }
}

customElements.define('intelligence-widget', IntelligenceWidget)

export { IntelligenceWidget }
