/**
 * Automation Timeline Component - Symbion
 * Grille visuelle 24h x 7 jours pour les automations planifiees
 */

import { LitElement, html, css } from 'lit'

class AutomationTimeline extends LitElement {
  static properties = {
    automations: { type: Array },
    modes: { type: Array },
    highlightedId: { type: String }
  }

  static styles = css`
    :host {
      display: block;
    }

    .timeline-container {
      background: var(--card-bg, rgba(30, 35, 45, 0.95));
      border-radius: 12px;
      padding: 1rem;
      border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
    }

    .timeline-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
    }

    .timeline-title {
      font-size: 1rem;
      font-weight: 600;
      color: var(--text-primary, #fff);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .timeline-grid {
      display: grid;
      grid-template-columns: 50px repeat(7, 1fr);
      gap: 2px;
      font-size: 0.75rem;
    }

    .grid-header {
      background: var(--bg-secondary, rgba(40, 45, 55, 0.8));
      padding: 0.5rem 0.25rem;
      text-align: center;
      color: var(--text-secondary, rgba(255, 255, 255, 0.7));
      font-weight: 500;
      border-radius: 4px;
    }

    .grid-header.corner {
      background: transparent;
    }

    .hour-label {
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-secondary, rgba(255, 255, 255, 0.6));
      font-size: 0.7rem;
      padding: 0 0.25rem;
    }

    .grid-cell {
      background: var(--bg-tertiary, rgba(50, 55, 65, 0.5));
      min-height: 28px;
      border-radius: 4px;
      cursor: pointer;
      transition: all 0.2s ease;
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .grid-cell:hover {
      background: var(--bg-hover, rgba(60, 65, 75, 0.8));
      transform: scale(1.02);
    }

    .grid-cell.has-automation {
      background: var(--primary-color, rgba(0, 212, 170, 0.3));
      border: 1px solid var(--primary-color, rgba(0, 212, 170, 0.5));
    }

    .grid-cell.highlighted {
      background: var(--primary-color, rgba(0, 212, 170, 0.5));
      box-shadow: 0 0 8px var(--primary-color, rgba(0, 212, 170, 0.5));
    }

    .cell-icon {
      font-size: 0.9rem;
      opacity: 0.9;
    }

    .cell-count {
      position: absolute;
      top: 2px;
      right: 2px;
      font-size: 0.6rem;
      background: var(--primary-color, #00d4aa);
      color: var(--bg-primary, #1a1f2e);
      width: 14px;
      height: 14px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 600;
    }

    .legend {
      display: flex;
      flex-wrap: wrap;
      gap: 1rem;
      margin-top: 1rem;
      padding-top: 0.75rem;
      border-top: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
    }

    .legend-item {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.75rem;
      color: var(--text-secondary, rgba(255, 255, 255, 0.7));
    }

    .legend-color {
      width: 16px;
      height: 16px;
      border-radius: 4px;
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    }

    .empty-state-icon {
      font-size: 2rem;
      margin-bottom: 0.5rem;
    }

    @media (max-width: 768px) {
      .timeline-grid {
        grid-template-columns: 40px repeat(7, 1fr);
        gap: 1px;
      }

      .grid-header {
        padding: 0.25rem;
        font-size: 0.65rem;
      }

      .grid-cell {
        min-height: 24px;
      }

      .hour-label {
        font-size: 0.6rem;
      }
    }
  `

  constructor() {
    super()
    this.automations = []
    this.modes = []
    this.highlightedId = null
  }

  // Jours de la semaine (commence par Lundi)
  get weekdays() {
    return [
      { value: 1, short: 'Lun', full: 'Lundi' },
      { value: 2, short: 'Mar', full: 'Mardi' },
      { value: 3, short: 'Mer', full: 'Mercredi' },
      { value: 4, short: 'Jeu', full: 'Jeudi' },
      { value: 5, short: 'Ven', full: 'Vendredi' },
      { value: 6, short: 'Sam', full: 'Samedi' },
      { value: 0, short: 'Dim', full: 'Dimanche' }
    ]
  }

  // Heures affichees (6h, 9h, 12h, 15h, 18h, 21h)
  get displayHours() {
    return [6, 9, 12, 15, 18, 21]
  }

  // Extraire les automations planifiees avec leurs plages horaires
  getScheduledAutomations() {
    if (!this.automations) return []

    return this.automations.filter(auto => {
      if (!auto.enabled) return false

      // Chercher un trigger scheduled ou une condition time_range + day_of_week
      const rules = auto._rules?.items || []
      const hasScheduled = rules.some(r => r.type === 'scheduled')
      const hasTimeRange = rules.some(r => r.type === 'time_range')
      const hasDayOfWeek = rules.some(r => r.type === 'day_of_week')

      return hasScheduled || (hasTimeRange && hasDayOfWeek)
    }).map(auto => {
      const rules = auto._rules?.items || []
      const timeRange = rules.find(r => r.type === 'time_range') || {}
      const dayOfWeek = rules.find(r => r.type === 'day_of_week') || {}
      const forceMode = auto.actions?.find(a => a.type === 'force_mode')

      return {
        id: auto.id,
        name: auto.name,
        startHour: timeRange.start_hour ?? 0,
        endHour: timeRange.end_hour ?? 24,
        days: dayOfWeek.days || [1, 2, 3, 4, 5],
        mode: forceMode?.mode || null,
        icon: this.getModeIcon(forceMode?.mode)
      }
    })
  }

  getModeIcon(modeSlug) {
    const icons = {
      'cravate': '👔',
      'intime': '🏡',
      'neutre': '🌙',
      'pro': '👔',
      'home': '🏡'
    }
    return icons[modeSlug?.toLowerCase()] || '⚡'
  }

  getModeColor(modeSlug) {
    const colors = {
      'cravate': 'rgba(59, 130, 246, 0.4)',  // Bleu
      'intime': 'rgba(34, 197, 94, 0.4)',    // Vert
      'neutre': 'rgba(156, 163, 175, 0.4)',  // Gris
      'pro': 'rgba(59, 130, 246, 0.4)',
      'home': 'rgba(34, 197, 94, 0.4)'
    }
    return colors[modeSlug?.toLowerCase()] || 'rgba(0, 212, 170, 0.3)'
  }

  // Trouver les automations pour une cellule specifique
  getAutomationsForCell(hour, dayValue) {
    const scheduled = this.getScheduledAutomations()
    return scheduled.filter(auto => {
      // Verifier le jour
      const dayInRange = auto.days.map(d => parseInt(d)).includes(dayValue)
      if (!dayInRange) return false

      // Verifier l'heure (plage de 3h pour chaque cellule)
      const hourEnd = hour + 3
      const autoStart = auto.startHour
      const autoEnd = auto.endHour

      // Intersection des plages
      return autoStart < hourEnd && autoEnd > hour
    })
  }

  handleCellClick(hour, day, automations) {
    this.dispatchEvent(new CustomEvent('slot-click', {
      detail: { hour, day: day.value, dayName: day.full, automations },
      bubbles: true,
      composed: true
    }))
  }

  handleCellHover(automations, entering) {
    if (automations.length > 0 && entering) {
      this.dispatchEvent(new CustomEvent('automation-highlight', {
        detail: { id: automations[0].id },
        bubbles: true,
        composed: true
      }))
    } else if (!entering) {
      this.dispatchEvent(new CustomEvent('automation-highlight', {
        detail: { id: null },
        bubbles: true,
        composed: true
      }))
    }
  }

  render() {
    const scheduled = this.getScheduledAutomations()

    // Construire la legende des modes utilises
    const usedModes = [...new Set(scheduled.map(s => s.mode).filter(Boolean))]

    return html`
      <div class="timeline-container">
        <div class="timeline-header">
          <div class="timeline-title">
            <span>📅</span>
            <span>Planning Hebdomadaire</span>
          </div>
          <span style="font-size: 0.75rem; color: var(--text-secondary);">
            ${scheduled.length} automation${scheduled.length > 1 ? 's' : ''} planifiee${scheduled.length > 1 ? 's' : ''}
          </span>
        </div>

        ${scheduled.length === 0 ? html`
          <div class="empty-state">
            <div class="empty-state-icon">📆</div>
            <div>Aucune automation planifiee</div>
            <div style="font-size: 0.7rem; margin-top: 0.5rem;">
              Cliquez sur une cellule pour creer une automation
            </div>
          </div>
        ` : ''}

        <div class="timeline-grid">
          <!-- Header row -->
          <div class="grid-header corner"></div>
          ${this.weekdays.map(day => html`
            <div class="grid-header">${day.short}</div>
          `)}

          <!-- Hour rows -->
          ${this.displayHours.map(hour => html`
            <div class="hour-label">${hour}h</div>
            ${this.weekdays.map(day => {
              const cellAutos = this.getAutomationsForCell(hour, day.value)
              const hasAutomation = cellAutos.length > 0
              const isHighlighted = cellAutos.some(a => a.id === this.highlightedId)
              const primaryAuto = cellAutos[0]

              return html`
                <div
                  class="grid-cell ${hasAutomation ? 'has-automation' : ''} ${isHighlighted ? 'highlighted' : ''}"
                  style="${hasAutomation ? `background: ${this.getModeColor(primaryAuto?.mode)}` : ''}"
                  @click=${() => this.handleCellClick(hour, day, cellAutos)}
                  @mouseenter=${() => this.handleCellHover(cellAutos, true)}
                  @mouseleave=${() => this.handleCellHover(cellAutos, false)}
                  title="${hasAutomation ? cellAutos.map(a => a.name).join(', ') : `${day.full} ${hour}h-${hour + 3}h`}"
                >
                  ${hasAutomation ? html`
                    <span class="cell-icon">${primaryAuto.icon}</span>
                    ${cellAutos.length > 1 ? html`
                      <span class="cell-count">${cellAutos.length}</span>
                    ` : ''}
                  ` : ''}
                </div>
              `
            })}
          `)}
        </div>

        ${usedModes.length > 0 ? html`
          <div class="legend">
            ${usedModes.map(mode => html`
              <div class="legend-item">
                <div class="legend-color" style="background: ${this.getModeColor(mode)}"></div>
                <span>${this.getModeIcon(mode)} ${mode}</span>
              </div>
            `)}
          </div>
        ` : ''}
      </div>
    `
  }
}

customElements.define('automation-timeline', AutomationTimeline)

export { AutomationTimeline }
