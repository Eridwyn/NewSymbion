import { html } from 'lit'
import csrfService from '../services/csrf-service.js'

export const IntelligenceMixin = (Base) => class extends Base {

  async loadIntelligence() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const [features, vector, prediction] = await Promise.all([
        apiService.request('/v1/intelligence/features').catch(() => null),
        apiService.request('/v1/intelligence/vector').catch(() => null),
        apiService.request('/v1/intelligence/prediction2').catch(() => null),
      ])
      this.intelligenceFeatures = features
      this.intelligenceVector = vector
      this.intelligencePrediction = prediction
    } catch (e) {
      console.error('[context-engine] Failed to load intelligence v2:', e)
    }
  }

  togglePredictionCorrection() {
    this.showPredictionCorrection = !this.showPredictionCorrection
    this.predictionCorrectionSent = false
  }

  async sendPredictionCorrection(modeSlug) {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/intelligence/feedback', {
        method: 'POST',
        body: JSON.stringify({ chosen_mode: modeSlug }),
      })

      if (res.ok) {
        this.predictionCorrectionSent = true
        setTimeout(() => {
          this.showPredictionCorrection = false
          this.predictionCorrectionSent = false
          this.loadIntelligence()
        }, 1500)
      }
    } catch (e) {
      console.error('[context-engine] Prediction correction failed:', e)
    }
  }

  renderIntelligenceTab() {
    const prediction = this.intelligencePrediction?.prediction
    const vector = this.intelligencePrediction?.vector || this.intelligenceVector?.vector
    const stats = this.intelligencePrediction?.stats
    const features = this.intelligenceFeatures?.features || []
    const summary = this.intelligenceFeatures?.summary

    // Group features by source
    const featuresBySource = {}
    features.forEach(f => {
      const source = f.source.split('.')[0] // agent, classifier, sensor
      if (!featuresBySource[source]) featuresBySource[source] = []
      featuresBySource[source].push(f)
    })

    return html`
      <div class="intelligence-tab ce-anim-card-enter">
        <!-- Prediction v2 Section -->
        <div class="section-card ce-intelligence-section">
          <div class="ce-flex ce-gap-md ce-mb-xl">
            <span class="ce-float-icon">🧠</span>
            <h3 class="ce-m-0 ce-text-bold ce-text-primary ce-text-lg">Intelligence v2</h3>
            ${stats ? html`
              <span class="ce-ml-auto ce-badge-purple ce-text-bold">
                ${stats.total_samples} samples
              </span>
            ` : ''}
          </div>

          ${prediction ? html`
            <div class="ce-flex-wrap ce-gap-xl">
              <!-- Confidence Gauge -->
              <div class="ce-flex-shrink-0">
                ${this.renderConfidenceGauge(prediction.confidence, 130)}
              </div>

              <!-- Mode Info -->
              <div class="ce-flex-grow-min200">
                <div class="ce-flex ce-gap-md ce-mb-lg">
                  <span class="ce-float-icon-lg ce-anim-slow">${this.getModeIcon(prediction.mode)}</span>
                  <div>
                    <div class="ce-text-2xl-bold ce-text-purple ce-mb-xs">${this.getModeName(prediction.mode)}</div>
                    <div class="ce-flex-wrap">
                      <span class="ce-confidence-badge" style="background: ${prediction.is_confident ? 'linear-gradient(135deg, rgba(34, 197, 94, 0.2), rgba(34, 197, 94, 0.1))' : 'linear-gradient(135deg, rgba(251, 146, 60, 0.2), rgba(251, 146, 60, 0.1))'}; border: 1px solid ${prediction.is_confident ? 'rgba(34, 197, 94, 0.4)' : 'rgba(251, 146, 60, 0.4)'}; color: ${prediction.is_confident ? '#22c55e' : '#fb923c'};">
                        ${prediction.is_confident ? '✓ Confiant' : '⚠ Incertain'}
                      </span>
                      <span class="ce-badge-neutral ce-text-xs ce-text-secondary">
                        ${prediction.samples_used} samples utilises
                      </span>
                    </div>
                  </div>
                </div>

                <!-- Alternatives -->
                ${prediction.alternatives?.length > 0 ? html`
                  <div class="ce-flex-wrap">
                    ${prediction.alternatives.map(alt => html`
                      <span class="ce-badge-purple ce-text-sm ce-text-secondary ce-badge-purple-bg">
                        ${this.getModeIcon(alt.mode)} ${this.getModeName(alt.mode)}: ${Math.round(alt.score * 100)}%
                      </span>
                    `)}
                  </div>
                ` : ''}
              </div>
            </div>

            <!-- Correction Button -->
            <div class="ce-flex-center ce-mt-1-25">
              <button @click=${this.togglePredictionCorrection}
                class="ce-btn-correction"
                style="background: ${this.showPredictionCorrection ? 'rgba(239, 68, 68, 0.15)' : 'rgba(139, 92, 246, 0.1)'}; border: 1px solid ${this.showPredictionCorrection ? 'rgba(239, 68, 68, 0.3)' : 'rgba(139, 92, 246, 0.25)'}; color: ${this.showPredictionCorrection ? '#f87171' : '#a78bfa'};">
                ${this.showPredictionCorrection ? '✕ Annuler' : '✏️ Corriger la prediction'}
              </button>
            </div>

            <!-- Correction Panel -->
            ${this.showPredictionCorrection ? html`
              <div class="ce-mt-lg ce-p-lg ce-correction-panel">
                ${this.predictionCorrectionSent ? html`
                  <div class="ce-text-center ce-text-green ce-text-bold ce-p-md ce-text-095">
                    ✓ Correction enregistree (v1+v2)
                  </div>
                ` : html`
                  <div class="ce-section-header">
                    Quel est le bon mode ?
                  </div>
                  <div class="ce-flex-wrap">
                    ${this.modes.map(m => html`
                      <button @click=${() => this.sendPredictionCorrection(m.slug)}
                        ?disabled=${m.slug === prediction.mode}
                        class="ce-mode-choice-btn"
                        style="background: ${m.slug === prediction.mode ? 'var(--surface-glass-faint)' : 'var(--surface-glass)'}; border: 1px solid ${m.slug === prediction.mode ? 'var(--border-subtle)' : 'var(--border-medium)'}; color: ${m.slug === prediction.mode ? 'var(--color-dark-text-tertiary)' : 'var(--color-dark-text-primary)'}; cursor: ${m.slug === prediction.mode ? 'not-allowed' : 'pointer'}; opacity: ${m.slug === prediction.mode ? '0.35' : '1'};">
                        <span class="ce-text-2xl">${m.icon}</span>
                        <span class="ce-text-xs">${m.name}</span>
                      </button>
                    `)}
                  </div>
                `}
              </div>
            ` : ''}

            <!-- Why Chain - Samples Contributing -->
            ${prediction.why?.length > 0 ? html`
              <div class="ce-mt-xl ce-border-top-purple">
                <div class="ce-section-header">Samples Contributifs</div>
                <div class="ce-flex-wrap">
                  ${prediction.why.slice(0, 5).map(w => html`
                    <div class="ce-bg-dark ce-text-xs">
                      <span class="ce-text-purple">${this.getModeIcon(w.mode)}</span>
                      <span class="ce-text-xs-secondary">${w.mode}</span>
                      <span style="color: ${w.similarity >= 0.8 ? '#22c55e' : w.similarity >= 0.5 ? '#fb923c' : '#9ca3af'}; margin-left: 0.5rem;">
                        ${Math.round(w.similarity * 100)}% sim
                      </span>
                    </div>
                  `)}
                </div>
              </div>
            ` : ''}
          ` : html`
            <div class="ce-text-center ce-text-tertiary ce-p-3xl">
              <div class="ce-float-icon-lg ce-mb-lg ce-opacity-4">🧠</div>
              <div class="ce-text-base">Pas de prediction v2 disponible</div>
              <div class="ce-text-sm ce-mt-sm ce-opacity-7">Le systeme collecte des donnees...</div>
            </div>
          `}
        </div>

        <!-- Context Vector Section -->
        ${vector ? html`
          <div class="section-card ce-bg-section">
            <div class="ce-flex ce-mb-lg">
              <span class="ce-float-icon-sm">📊</span>
              <h3 class="ce-m-0 ce-text-bold ce-text-primary ce-text-base">Context Vector</h3>
              <span class="ce-ml-auto ce-text-tiny-tertiary">
                ${vector.feature_count || 0} features
              </span>
            </div>

            <!-- Dimensions as bars -->
            <div class="ce-flex-col-md">
              ${Object.entries(vector.dimensions || {}).map(([dim, value]) => this.renderDimensionBar(dim, value, vector.why?.[dim]))}
            </div>
          </div>
        ` : ''}

        <!-- Features Section -->
        ${features.length > 0 ? html`
          <div class="section-card ce-bg-section">
            <div class="ce-flex ce-mb-lg">
              <span class="ce-float-icon-sm">📡</span>
              <h3 class="ce-m-0 ce-text-bold ce-text-primary ce-text-base">Features Registry</h3>
              <span class="ce-ml-auto ce-badge-green ce-text-bold">
                ${summary?.active_count || features.length} actives
              </span>
            </div>

            ${Object.entries(featuresBySource).map(([source, sourceFeatures]) => html`
              <div class="ce-mb-lg">
                <div class="ce-text-tiny ce-text-bold ce-text-tertiary ce-mb-sm ce-source-header">
                  ${source === 'agent' ? '🖥️ Agent' : source === 'classifier' ? '🏷️ Classifier' : source === 'sensor' ? '🌡️ Sensors' : source}
                </div>
                <div class="ce-grid-auto ce-gap-sm">
                  ${sourceFeatures.map(f => this.renderFeatureCard(f))}
                </div>
              </div>
            `)}
          </div>
        ` : ''}

        <!-- Stats Section -->
        ${stats ? html`
          <div class="section-card ce-bg-stats-section">
            <div class="ce-flex ce-mb-md">
              <span class="ce-text-base">📈</span>
              <h3 class="ce-m-0 ce-text-bold ce-text-primary ce-text-09">Statistiques Apprentissage</h3>
            </div>
            <div class="ce-grid-auto-sm">
              <div class="ce-stat-item">
                <div class="ce-text-2xl-bold ce-text-purple">${stats.total_samples}</div>
                <div class="ce-text-tiny-tertiary">Total Samples</div>
              </div>
              <div class="ce-stat-item">
                <div class="ce-text-2xl-bold ce-text-green">${stats.by_source?.UserCorrection || 0}</div>
                <div class="ce-text-tiny-tertiary">Corrections</div>
              </div>
              <div class="ce-stat-item">
                <div class="ce-text-2xl-bold ce-text-orange">${stats.by_source?.Bootstrap || 0}</div>
                <div class="ce-text-tiny-tertiary">Bootstrap</div>
              </div>
              <div class="ce-stat-item">
                <div class="ce-text-2xl-bold ce-text-primary">${(stats.average_weight || 0).toFixed(2)}</div>
                <div class="ce-text-tiny-tertiary">Poids Moyen</div>
              </div>
            </div>

            <!-- Samples by mode -->
            ${stats.by_mode ? html`
              <div class="ce-mt-lg ce-border-top-muted">
                <div class="ce-text-tiny-tertiary ce-mb-sm">Samples par mode</div>
                <div class="ce-flex-wrap">
                  ${Object.entries(stats.by_mode).map(([mode, count]) => html`
                    <span class="ce-text-tiny ce-text-secondary ce-badge-purple-pill">
                      ${this.getModeIcon(mode)} ${mode}: ${count}
                    </span>
                  `)}
                </div>
              </div>
            ` : ''}
          </div>
        ` : ''}
      </div>
    `
  }

  // Render a dimension bar with why chain
  renderDimensionBar(dimension, value, why) {
    const colors = {
      'home_prob': { start: '#22c55e', end: '#4ade80', icon: '🏠' },
      'work_prob': { start: '#3b82f6', end: '#60a5fa', icon: '💼' },
      'focus_prob': { start: '#8b5cf6', end: '#a78bfa', icon: '🎯' },
      'sleep_prob': { start: '#6366f1', end: '#818cf8', icon: '😴' },
      'pc_active': { start: '#f59e0b', end: '#fbbf24', icon: '🖥️' },
      'away_prob': { start: '#ec4899', end: '#f472b6', icon: '🚶' }
    }
    const c = colors[dimension] || { start: '#6b7280', end: '#9ca3af', icon: '📊' }
    const percentage = Math.round(value * 100)
    const label = dimension.replace('_prob', '').replace('_', ' ')

    return html`
      <div class="ce-flex ce-gap-md">
        <span class="ce-dim-icon">${c.icon}</span>
        <div class="ce-flex-grow">
          <div class="ce-flex-between-mb">
            <span class="ce-text-xs-secondary ce-capitalize">${label}</span>
            <span style="font-size: var(--text-xs); font-weight: 600; color: ${c.start};">${percentage}%</span>
          </div>
          <div class="ce-progress-track">
            <div style="height: 100%; width: ${percentage}%; background: linear-gradient(90deg, ${c.start}, ${c.end}); border-radius: 3px; transition: width 0.5s ease-out;"></div>
          </div>
          ${why?.length > 0 ? html`
            <div class="ce-flex-wrap-sm ce-mt-xs">
              ${why.slice(0, 3).map(w => html`
                <span class="ce-badge-tiny ce-text-tertiary ce-badge-glass-bg">
                  ${w.feature_id}: ${w.contribution > 0 ? '+' : ''}${Math.round(w.contribution * 100)}%
                </span>
              `)}
            </div>
          ` : ''}
        </div>
      </div>
    `
  }

  // Render a feature card
  renderFeatureCard(feature) {
    const value = feature.value?.value
    const type = feature.value?.type
    let displayValue = ''
    let status = 'neutral'

    if (type === 'Bool') {
      displayValue = value ? '✓ Oui' : '✗ Non'
      status = value ? 'good' : 'neutral'
    } else if (type === 'Float') {
      displayValue = typeof value === 'number' ? value.toFixed(1) : value
      // Add unit based on feature_id
      if (feature.feature_id.includes('temperature')) displayValue += '°C'
      else if (feature.feature_id.includes('humidity')) displayValue += '%'
      else if (feature.feature_id.includes('cpu') || feature.feature_id.includes('memory')) displayValue += '%'
    } else if (type === 'StringList') {
      displayValue = `${value?.length || 0} items`
    } else {
      displayValue = String(value)
    }

    const statusColors = {
      good: { bg: 'rgba(34, 197, 94, 0.1)', border: 'rgba(34, 197, 94, 0.3)', text: '#22c55e' },
      warning: { bg: 'rgba(251, 146, 60, 0.1)', border: 'rgba(251, 146, 60, 0.3)', text: '#fb923c' },
      neutral: { bg: 'var(--surface-glass-subtle)', border: 'var(--border-medium)', text: 'var(--color-dark-text-primary)' }
    }
    const s = statusColors[status]

    return html`
      <div class="ce-feature-card" style="background: ${s.bg}; border: 1px solid ${s.border};">
        <div class="ce-text-tertiary ce-feature-label">
          ${feature.feature_id}
        </div>
        <div class="ce-feature-value" style="color: ${s.text};">${displayValue}</div>
        <div class="ce-text-tertiary ce-opacity-7 ce-text-06">
          conf: ${Math.round(feature.confidence * 100)}%
        </div>
      </div>
    `
  }

  // ============ Intelligence UI Helpers ============

  renderConfidenceGauge(value, size = 120) {
    const percentage = Math.round(value * 100)
    const radius = 45
    const circumference = 2 * Math.PI * radius // ~283
    const offset = circumference - (value * circumference)
    const color = value >= 0.7 ? '#22c55e' : value >= 0.4 ? '#fb923c' : '#ef4444'
    const glowColor = value >= 0.7 ? 'rgba(34, 197, 94, 0.6)' : value >= 0.4 ? 'rgba(251, 146, 60, 0.6)' : 'rgba(239, 68, 68, 0.6)'
    const label = value >= 0.7 ? 'Haute' : value >= 0.4 ? 'Moyenne' : 'Faible'

    return html`
      <div class="ce-gauge-container" style="width: ${size}px; height: ${size}px;">
        <svg class="ce-gauge-svg" width="${size}" height="${size}" viewBox="0 0 100 100" style="filter: drop-shadow(0 0 12px ${glowColor});">
          <!-- Background circle -->
          <circle cx="50" cy="50" r="${radius}" fill="none" stroke="var(--border-medium)" stroke-width="8"/>
          <!-- Progress circle -->
          <circle cx="50" cy="50" r="${radius}" fill="none" stroke="url(#gauge-gradient-${percentage})" stroke-width="8"
            stroke-linecap="round"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            style="transition: stroke-dashoffset 1s ease-out;"/>
          <defs>
            <linearGradient id="gauge-gradient-${percentage}" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="${color}"/>
              <stop offset="100%" stop-color="${value >= 0.7 ? '#4ade80' : value >= 0.4 ? '#fdba74' : '#f87171'}"/>
            </linearGradient>
          </defs>
        </svg>
        <div class="ce-gauge-center-col">
          <span style="font-size: ${size / 4}px; font-weight: 700; color: ${color}; animation: count-up 0.5s ease-out;">${percentage}%</span>
          <span class="ce-text-tertiary ce-uppercase-spaced" style="font-size: ${size / 10}px;">${label}</span>
        </div>
      </div>
    `
  }
}
