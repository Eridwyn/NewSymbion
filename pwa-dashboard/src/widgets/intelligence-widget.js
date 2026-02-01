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
  `

  static properties = {
    intelligenceStatus: { type: Object },
    signals: { type: Object },
    patterns: { type: Array },
    loading: { type: Boolean },
  }

  constructor() {
    super()
    this.intelligenceStatus = null
    this.signals = null
    this.patterns = []
    this.loading = true
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadData()
    // Refresh every 30 seconds
    this._interval = setInterval(() => this.loadData(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    clearInterval(this._interval)
  }

  async loadData() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        console.warn('[intelligence-widget] api-service not found')
        return
      }

      // Load status, signals, and patterns in parallel
      const [status, signalsData, patternsData] = await Promise.all([
        apiService.request('/v1/intelligence/status').catch(() => null),
        apiService.request('/v1/intelligence/signals').catch(() => null),
        apiService.request('/v1/intelligence/patterns').catch(() => ({ patterns: [] })),
      ])

      this.intelligenceStatus = status
      this.signals = signalsData
      this.patterns = patternsData?.patterns || []
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
      'veille': '🌙', 'neutre': '🌙', 'sleep': '🌙'
    }
    return icons[mode?.toLowerCase()] || '🎯'
  }

  getModeName(mode) {
    const names = {
      'pro': 'Pro', 'cravate': 'Focus Pro', 'focus': 'Focus',
      'maison': 'Maison', 'intime': 'Maison', 'home': 'Home',
      'veille': 'Veille', 'neutre': 'Veille', 'sleep': 'Sleep'
    }
    return names[mode?.toLowerCase()] || mode || 'Inconnu'
  }

  getDayName(day) {
    const days = ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam']
    return days[day] || '?'
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
          <div class="prediction-section">
            <div class="prediction-icon">${this.getModeIcon(prediction.mode)}</div>
            <div class="prediction-info">
              <div class="prediction-mode">${this.getModeName(prediction.mode)}</div>
              <div class="confidence-row">
                <div class="confidence-bar">
                  <div class="confidence-fill ${confidenceClass}"
                       style="width: ${prediction.confidence * 100}%"></div>
                </div>
                <span class="confidence-text">${Math.round(prediction.confidence * 100)}%</span>
              </div>
              <span class="action-indicator ${actionIndicator.class}">
                ${actionIndicator.text}
              </span>
            </div>
          </div>

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
                      ${this.getDayName(p.day_of_week)} ${p.hour}h
                    </span>
                    <span class="pattern-confidence">
                      ${Math.round(p.confidence * 100)}%
                    </span>
                  </div>
                `)}
              </div>
            </div>
          ` : ''}
        </div>
      </div>
    `
  }
}

customElements.define('intelligence-widget', IntelligenceWidget)

export { IntelligenceWidget }
