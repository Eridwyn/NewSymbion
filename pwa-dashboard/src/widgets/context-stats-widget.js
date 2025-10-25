/**
 * Widget Statistiques Contextuelles Symbion
 *
 * Affiche les statistiques d'utilisation des modes et les patterns détectés
 */

import { LitElement, html, css } from 'lit'

class ContextStatsWidget extends LitElement {
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
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }

    .widget-title {
      font-size: 1.25rem;
      font-weight: 700;
      color: #e0e0e0;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .refresh-btn {
      padding: 0.5rem 1rem;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      color: #e0e0e0;
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.875rem;
    }

    .refresh-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.3);
    }

    .section {
      margin-bottom: 2rem;
    }

    .section:last-child {
      margin-bottom: 0;
    }

    .section-title {
      font-size: 1rem;
      font-weight: 600;
      color: #a0a0a0;
      margin-bottom: 1rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .stats-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 1rem;
    }

    .stat-card {
      padding: 1rem;
      border-radius: 12px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .stat-icon {
      font-size: 2rem;
      margin-bottom: 0.5rem;
    }

    .stat-label {
      font-size: 0.75rem;
      color: #808080;
      margin-bottom: 0.25rem;
    }

    .stat-value {
      font-size: 1.5rem;
      font-weight: 700;
      color: #e0e0e0;
    }

    .stat-subvalue {
      font-size: 0.875rem;
      color: #a0a0a0;
      margin-top: 0.25rem;
    }

    .progress-bar {
      width: 100%;
      height: 8px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 4px;
      overflow: hidden;
      margin-top: 0.5rem;
    }

    .progress-fill {
      height: 100%;
      transition: width 0.5s ease;
    }

    .progress-fill.cravate {
      background: linear-gradient(90deg, #2563eb 0%, #1e40af 100%);
    }

    .progress-fill.intime {
      background: linear-gradient(90deg, #10b981 0%, #059669 100%);
    }

    .progress-fill.neutre {
      background: linear-gradient(90deg, #6b7280 0%, #4b5563 100%);
    }

    .patterns-list {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
    }

    .pattern-item {
      padding: 1rem;
      border-radius: 12px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .pattern-icon {
      font-size: 2rem;
    }

    .pattern-info {
      flex: 1;
    }

    .pattern-description {
      font-size: 0.875rem;
      color: #e0e0e0;
      font-weight: 500;
    }

    .pattern-meta {
      font-size: 0.75rem;
      color: #808080;
      margin-top: 0.25rem;
    }

    .pattern-confidence {
      padding: 0.25rem 0.75rem;
      border-radius: 20px;
      font-size: 0.75rem;
      font-weight: 600;
      background: rgba(16, 185, 129, 0.15);
      color: #10b981;
      border: 1px solid #10b981;
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: #808080;
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 0.5rem;
      opacity: 0.5;
    }

    .loading {
      text-align: center;
      color: #808080;
      padding: 2rem;
    }

    .error {
      color: #ff6b6b;
      text-align: center;
      padding: 1rem;
    }
  `

  static properties = {
    stats: { type: Array },
    patterns: { type: Array },
    productivity: { type: Array },
    status: { type: String }
  }

  constructor() {
    super()
    this.stats = []
    this.patterns = []
    this.productivity = []
    this.status = 'loading'
  }

  connectedCallback() {
    super.connectedCallback()
    this.fetchData()

    // Rafraîchir toutes les 30 secondes
    this.intervalId = setInterval(() => this.fetchData(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.intervalId) {
      clearInterval(this.intervalId)
    }
  }

  async fetchData() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        this.status = 'error'
        return
      }

      // Récupérer stats, patterns et productivity en parallèle
      const [stats, patterns, productivity] = await Promise.all([
        apiService.request('/context/stats'),
        apiService.request('/context/patterns'),
        apiService.request('/context/productivity')
      ])

      this.stats = stats || []
      this.patterns = patterns || []
      this.productivity = productivity || []
      this.status = 'ready'
    } catch (error) {
      console.error('[context-stats-widget] Failed to fetch data:', error)
      this.status = 'error'
    }
  }

  getModeIcon(mode) {
    const icons = {
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌱'
    }
    return icons[mode.toLowerCase()] || '🤔'
  }

  getModeName(mode) {
    const names = {
      'cravate': 'Focus Pro',
      'intime': 'Maison',
      'neutre': 'Veille'
    }
    return names[mode.toLowerCase()] || mode
  }

  formatDuration(minutes) {
    if (minutes < 60) {
      return `${minutes}m`
    }
    const hours = Math.floor(minutes / 60)
    const mins = minutes % 60
    if (hours < 24) {
      return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`
    }
    const days = Math.floor(hours / 24)
    const remainingHours = hours % 24
    return remainingHours > 0 ? `${days}j ${remainingHours}h` : `${days}j`
  }

  getDayName(dayNumber) {
    const days = ['', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam', 'Dim']
    return days[dayNumber] || dayNumber
  }

  render() {
    if (this.status === 'loading') {
      return html`
        <div class="widget-container">
          <div class="loading">Chargement des statistiques...</div>
        </div>
      `
    }

    if (this.status === 'error') {
      return html`
        <div class="widget-container">
          <div class="error">Impossible de charger les statistiques</div>
        </div>
      `
    }

    return html`
      <div class="widget-container">
        <div class="widget-header">
          <div class="widget-title">
            📊 Statistiques Contextuelles
          </div>
          <button class="refresh-btn" @click="${() => this.fetchData()}">
            🔄 Actualiser
          </button>
        </div>

        <!-- Statistiques par mode -->
        <div class="section">
          <div class="section-title">Temps par mode</div>
          ${this.stats.length === 0 ? html`
            <div class="empty-state">
              <div class="empty-icon">⏱️</div>
              <div>Aucune statistique disponible</div>
              <div style="font-size: 0.75rem; margin-top: 0.5rem;">
                Les données seront collectées au fil du temps
              </div>
            </div>
          ` : html`
            <div class="stats-grid">
              ${this.stats.map(stat => html`
                <div class="stat-card">
                  <div class="stat-icon">${this.getModeIcon(stat.mode)}</div>
                  <div class="stat-label">${this.getModeName(stat.mode)}</div>
                  <div class="stat-value">${this.formatDuration(stat.total_duration_minutes)}</div>
                  <div class="stat-subvalue">${stat.percentage.toFixed(1)}% du temps</div>
                  <div class="progress-bar">
                    <div class="progress-fill ${stat.mode.toLowerCase()}" style="width: ${stat.percentage}%"></div>
                  </div>
                </div>
              `)}
            </div>
          `}
        </div>

        <!-- Patterns détectés -->
        <div class="section">
          <div class="section-title">Patterns Détectés</div>
          ${this.patterns.length === 0 ? html`
            <div class="empty-state">
              <div class="empty-icon">🔍</div>
              <div>Aucun pattern détecté</div>
              <div style="font-size: 0.75rem; margin-top: 0.5rem;">
                Les patterns seront détectés après plusieurs changements manuels récurrents
              </div>
            </div>
          ` : html`
            <div class="patterns-list">
              ${this.patterns.map(pattern => html`
                <div class="pattern-item">
                  <div class="pattern-icon">${this.getModeIcon(pattern.mode)}</div>
                  <div class="pattern-info">
                    <div class="pattern-description">
                      ${this.getModeName(pattern.mode)} - ${this.getDayName(pattern.day_of_week)} à ${pattern.hour}h
                    </div>
                    <div class="pattern-meta">
                      ${pattern.occurrences} fois • Dernière: ${new Date(pattern.last_seen).toLocaleDateString('fr-FR')}
                    </div>
                  </div>
                  <div class="pattern-confidence">
                    ${Math.round(pattern.confidence * 100)}%
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

customElements.define('context-stats-widget', ContextStatsWidget)

export { ContextStatsWidget }
