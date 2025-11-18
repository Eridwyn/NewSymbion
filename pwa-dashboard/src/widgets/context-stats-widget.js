/**
 * Widget Statistiques Contextuelles Symbion
 *
 * Affiche les statistiques d'utilisation des modes et les patterns détectés
 */

import { LitElement, html, css } from 'lit'
import '../components/patterns-modal.js'

class ContextStatsWidget extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .widget-container {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 12px;
      padding: 1rem;
      backdrop-filter: blur(10px);
    }

    .widget-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1rem;
    }

    .widget-title {
      font-size: 1.1rem;
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
      margin-bottom: 1rem;
    }

    .section:last-child {
      margin-bottom: 0;
    }

    .section-title {
      font-size: 0.875rem;
      font-weight: 600;
      color: #a0a0a0;
      margin-bottom: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .stats-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 0.75rem;
    }

    .stat-card {
      padding: 0.75rem;
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .stat-icon {
      font-size: 1.5rem;
      margin-bottom: 0.5rem;
    }

    .stat-label {
      font-size: 0.7rem;
      color: #808080;
      margin-bottom: 0.25rem;
    }

    .stat-value {
      font-size: 1.25rem;
      font-weight: 700;
      color: #e0e0e0;
    }

    .stat-subvalue {
      font-size: 0.75rem;
      color: #a0a0a0;
      margin-top: 0.25rem;
    }

    .progress-bar {
      width: 100%;
      height: 6px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 3px;
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
      gap: 0.5rem;
    }

    .pattern-item {
      padding: 0.75rem;
      border-radius: 12px;
      background: linear-gradient(135deg,
        rgba(255, 255, 255, 0.06) 0%,
        rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      display: flex;
      align-items: center;
      gap: 0.75rem;
      max-width: 100%; /* Empêche débordement */
      overflow: hidden;
      transition: all var(--duration-base) var(--ease-out);
      animation: patternItemSlideIn 0.5s cubic-bezier(0.4, 0, 0.2, 1) backwards;
    }

    /* Stagger animation pour liste patterns */
    .pattern-item:nth-child(1) { animation-delay: 0.1s; }
    .pattern-item:nth-child(2) { animation-delay: 0.15s; }
    .pattern-item:nth-child(3) { animation-delay: 0.2s; }
    .pattern-item:nth-child(4) { animation-delay: 0.25s; }
    .pattern-item:nth-child(5) { animation-delay: 0.3s; }
    .pattern-item:nth-child(n+6) { animation-delay: 0.35s; }

    @keyframes patternItemSlideIn {
      from {
        opacity: 0;
        transform: translateX(-20px);
      }
      to {
        opacity: 1;
        transform: translateX(0);
      }
    }

    .pattern-item:hover {
      background: linear-gradient(135deg,
        rgba(255, 255, 255, 0.1) 0%,
        rgba(255, 255, 255, 0.05) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
    }

    .pattern-icon {
      font-size: 1.5rem;
      transition: transform var(--duration-base) var(--ease-out);
    }

    .pattern-item:hover .pattern-icon {
      transform: scale(1.1) rotate(-5deg);
    }

    .pattern-info {
      flex: 1;
      min-width: 0; /* Permet truncation si nécessaire */
    }

    .pattern-description {
      font-size: 0.8rem;
      color: #e0e0e0;
      font-weight: 500;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .pattern-meta {
      font-size: 0.7rem;
      color: #808080;
      margin-top: 0.25rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .pattern-confidence {
      padding: 0.2rem 0.6rem;
      border-radius: 12px;
      font-size: 0.7rem;
      font-weight: 600;
      background: linear-gradient(135deg,
        rgba(16, 185, 129, 0.2) 0%,
        rgba(16, 185, 129, 0.1) 100%);
      color: #10b981;
      border: 1px solid rgba(16, 185, 129, 0.4);
      box-shadow: 0 0 10px rgba(16, 185, 129, 0.2);
      transition: all var(--duration-base) var(--ease-out);
      animation: confidencePulse 3s ease-in-out infinite;
      flex-shrink: 0; /* Ne rétrécit pas */
    }

    @keyframes confidencePulse {
      0%, 100% {
        box-shadow: 0 0 10px rgba(16, 185, 129, 0.2);
      }
      50% {
        box-shadow: 0 0 15px rgba(16, 185, 129, 0.35);
      }
    }

    .pattern-item:hover .pattern-confidence {
      background: linear-gradient(135deg,
        rgba(16, 185, 129, 0.3) 0%,
        rgba(16, 185, 129, 0.15) 100%);
      border-color: rgba(16, 185, 129, 0.6);
      transform: scale(1.05);
    }

    .empty-state {
      text-align: center;
      padding: 1rem;
      color: #808080;
    }

    .empty-icon {
      font-size: 2rem;
      margin-bottom: 0.5rem;
      opacity: 0.5;
    }

    .loading {
      text-align: center;
      color: #808080;
      padding: 1rem;
    }

    .error {
      color: #ff6b6b;
      text-align: center;
      padding: 1rem;
    }

    .see-all-btn {
      position: relative;
      width: 100%;
      max-width: 100%; /* Empêche débordement */
      padding: 0.75rem 1rem;
      margin-top: 0.75rem;
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: 12px;
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, rgba(255, 255, 255, 0.05)) 0%,
        rgba(255, 255, 255, 0.03) 100%);
      color: var(--context-primary, #00d4aa);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      font-size: 0.875rem;
      font-weight: 600;
      overflow: hidden;
      animation: seeAllPulse 4s ease-in-out infinite;
    }

    @keyframes seeAllPulse {
      0%, 100% {
        box-shadow: 0 2px 8px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      }
      50% {
        box-shadow: 0 4px 12px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent),
                    0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      }
    }

    .see-all-btn::before {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: 0;
      height: 0;
      border-radius: 50%;
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      transform: translate(-50%, -50%);
      transition: width 0.6s var(--ease-out), height 0.6s var(--ease-out);
    }

    .see-all-btn:hover::before {
      width: 300px;
      height: 300px;
    }

    .see-all-btn:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 12%, rgba(255, 255, 255, 0.08)) 0%,
        rgba(255, 255, 255, 0.05) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                  0 0 25px color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      animation: none; /* Stop pulse on hover */
    }

    .see-all-btn:active {
      transform: translateY(0);
      box-shadow: 0 2px 8px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
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
    this.modalElement = null
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
    this.closeModal()
  }

  openModal() {
    // Créer la modale et l'ajouter au document
    this.modalElement = document.createElement('patterns-modal')
    this.modalElement.patterns = this.patterns
    this.modalElement.addEventListener('close', () => this.closeModal())
    document.body.appendChild(this.modalElement)
  }

  closeModal() {
    // Retirer la modale du DOM
    if (this.modalElement && this.modalElement.parentNode) {
      this.modalElement.parentNode.removeChild(this.modalElement)
      this.modalElement = null
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

  formatDate(dateString) {
    try {
      // Format API: "2025-10-25 14:21:45.06683966 +00:00:00"
      // Extraire "2025-10-25 14:21:45"
      const parts = dateString.split(' ')
      const datePart = parts[0]
      const timePart = parts[1]?.split('.')[0] || '00:00:00'

      // Parser comme UTC puis convertir en heure locale
      const date = new Date(`${datePart}T${timePart}Z`)
      if (isNaN(date.getTime())) {
        return 'N/A'
      }
      return date.toLocaleDateString('fr-FR', {
        day: 'numeric',
        month: 'short',
        hour: '2-digit',
        minute: '2-digit'
      })
    } catch (e) {
      console.error('[context-stats] Invalid date format:', dateString, e)
      return 'N/A'
    }
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
              <div style="font-size: 0.85rem;">Aucune statistique disponible</div>
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
              <div style="font-size: 0.85rem;">Aucun pattern détecté</div>
            </div>
          ` : html`
            <div class="patterns-list">
              ${this.patterns.slice(0, 1).map(pattern => html`
                <div class="pattern-item">
                  <div class="pattern-icon">${this.getModeIcon(pattern.mode)}</div>
                  <div class="pattern-info">
                    <div class="pattern-description">
                      ${this.getModeName(pattern.mode)} - ${this.getDayName(pattern.day_of_week)} à ${pattern.hour}h
                    </div>
                    <div class="pattern-meta">
                      ${pattern.occurrences} fois • Dernière: ${this.formatDate(pattern.last_seen)}
                    </div>
                  </div>
                  <div class="pattern-confidence">
                    ${Math.round(pattern.confidence * 100)}%
                  </div>
                </div>
              `)}
            </div>
            ${this.patterns.length > 1 ? html`
              <button class="see-all-btn" @click="${() => this.openModal()}">
                📋 Voir tous les patterns (${this.patterns.length})
              </button>
            ` : ''}
          `}
        </div>
      </div>
    `
  }
}

customElements.define('context-stats-widget', ContextStatsWidget)

export { ContextStatsWidget }
