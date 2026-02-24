/**
 * Automation Timeline Component - Symbion
 * Planning hebdomadaire continu — barres positionnees aux heures exactes
 */

import { LitElement, html, css } from 'lit'

const MIN_HOUR = 6
const MAX_HOUR = 24
const TOTAL_HOURS = MAX_HOUR - MIN_HOUR

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
      background: var(--app-section-bg);
      border-radius: var(--radius-md, 0.75rem);
      padding: 1rem;
      border: 1px solid var(--border-medium);
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
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    /* Layout: hour labels + 7 day columns */
    .planning {
      display: grid;
      grid-template-columns: 32px repeat(7, 1fr);
      gap: 3px;
    }

    .day-header {
      text-align: center;
      font-size: 0.7rem;
      font-weight: 600;
      color: var(--color-dark-text-secondary, #cbd5e1);
      padding-bottom: 0.5rem;
    }

    .day-header.corner { }

    .hour-axis {
      position: relative;
      height: 320px;
    }

    .hour-tick {
      position: absolute;
      left: 0;
      right: 0;
      font-size: 0.6rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
      transform: translateY(-50%);
      text-align: right;
      padding-right: 4px;
      line-height: 1;
    }

    /* Each day column */
    .day-column {
      position: relative;
      height: 320px;
      background: var(--surface-glass-hover);
      border-radius: var(--radius-sm);
      overflow: hidden;
    }

    /* Hour grid lines */
    .hour-line {
      position: absolute;
      left: 0;
      right: 0;
      height: 1px;
      background: var(--border-subtle);
    }

    /* Automation bar */
    .auto-bar {
      position: absolute;
      left: 2px;
      right: 2px;
      border-radius: var(--radius-sm);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 2px;
      transition: opacity 0.2s, box-shadow 0.2s;
      overflow: hidden;
      min-height: 14px;
      z-index: 1;
    }

    .auto-bar:hover {
      opacity: 0.9;
      box-shadow: 0 0 8px var(--surface-glass-bright);
      z-index: 2;
    }

    .auto-bar.highlighted {
      box-shadow: 0 0 10px var(--border-strong);
      z-index: 3;
    }

    .bar-icon {
      font-size: var(--text-xs);
      flex-shrink: 0;
    }

    .bar-label {
      font-size: 0.7rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .bar-hours {
      font-size: 0.65rem;
      color: var(--color-dark-text-secondary, #cbd5e1);
      white-space: nowrap;
    }

    /* Current time indicator */
    .now-line {
      position: absolute;
      left: 0;
      right: 0;
      height: 2px;
      background: var(--color-danger-text-muted, #ef4444);
      z-index: 5;
      pointer-events: none;
    }

    .now-line::before {
      content: '';
      position: absolute;
      left: -3px;
      top: -3px;
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--color-danger-text-muted, #ef4444);
    }

    .legend {
      display: flex;
      flex-wrap: wrap;
      gap: 1rem;
      margin-top: 1rem;
      padding-top: 0.75rem;
      border-top: 1px solid var(--border-medium);
    }

    .legend-item {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      font-size: var(--text-xs);
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .legend-color {
      width: 14px;
      height: 14px;
      border-radius: var(--radius-sm);
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .empty-state-icon {
      font-size: var(--text-3xl);
      margin-bottom: 0.5rem;
    }

    @media (max-width: 768px) {
      .planning {
        grid-template-columns: 26px repeat(7, 1fr);
        gap: 2px;
      }

      .hour-axis, .day-column {
        height: 260px;
      }

      .day-header {
        font-size: 0.65rem;
      }

      .hour-tick {
        font-size: 0.65rem;
      }

      .bar-label {
        display: none;
      }

      .bar-icon {
        font-size: 0.65rem;
      }
    }

    /* Utility classes (ex-inline) */
    .at-schedule-hint { font-size: var(--text-xs); color: var(--text-secondary); }
  `

  constructor() {
    super()
    this.automations = []
    this.modes = []
    this.highlightedId = null
  }

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

  // Heures affichees sur l'axe (ticks)
  get hourTicks() {
    return [6, 8, 10, 12, 14, 16, 18, 20, 22, 24]
  }

  // Position en % pour une heure donnee
  hourToPercent(hour) {
    return ((hour - MIN_HOUR) / TOTAL_HOURS) * 100
  }

  getScheduledAutomations() {
    if (!this.automations) return []

    return this.automations.filter(auto => {
      if (!auto.enabled) return false
      const conditions = auto.conditions?.conditions || []
      const triggers = auto.triggers?.triggers || []
      const hasTimeRange = conditions.some(c => c.type === 'time_range')
      const hasDayOfWeek = conditions.some(c => c.type === 'day_of_week')
      const hasScheduled = triggers.some(t => t.type === 'scheduled')
        || auto.trigger?.type === 'scheduled'
      return hasScheduled || (hasTimeRange && hasDayOfWeek)
    }).map(auto => {
      const conditions = auto.conditions?.conditions || []
      const timeRange = conditions.find(c => c.type === 'time_range') || {}
      const dayOfWeek = conditions.find(c => c.type === 'day_of_week') || {}
      const forceMode = auto.actions?.find(a => a.type === 'force_mode')

      return {
        id: auto.id,
        name: auto.name,
        startHour: timeRange.start_hour ?? 0,
        endHour: timeRange.end_hour ?? 24,
        days: (dayOfWeek.days || [1, 2, 3, 4, 5]).map(d => parseInt(d)),
        mode: forceMode?.mode || null,
        icon: this.getModeIcon(forceMode?.mode)
      }
    })
  }

  // Automations pour un jour donne
  getAutomationsForDay(dayValue) {
    return this.getScheduledAutomations().filter(auto => auto.days.includes(dayValue))
  }

  getModeIcon(modeSlug) {
    if (this.modes?.length && modeSlug) {
      const mode = this.modes.find(m => m.value === modeSlug || m.slug === modeSlug)
      if (mode?.label) {
        const match = mode.label.match(/^(\p{Emoji})/u)
        if (match) return match[1]
      }
    }
    const fallback = { 'pro': '👔', 'maison': '🏡', 'focus': '🎯', 'veille': '🌱' }
    return fallback[modeSlug?.toLowerCase()] || '⚡'
  }

  getModeColor(modeSlug, alpha = 0.5) {
    if (this.modes?.length && modeSlug) {
      const mode = this.modes.find(m => m.value === modeSlug || m.slug === modeSlug)
      if (mode?.color) return this._hexToRgba(mode.color, alpha)
    }
    const fallback = {
      'pro': `rgba(37, 99, 235, ${alpha})`,
      'focus': `rgba(99, 102, 241, ${alpha})`,
      'maison': `rgba(16, 185, 129, ${alpha})`,
      'veille': `rgba(107, 114, 128, ${alpha})`
    }
    return fallback[modeSlug?.toLowerCase()] || `rgba(0, 212, 170, ${alpha})`
  }

  getModeBorder(modeSlug) {
    return this.getModeColor(modeSlug, 0.8)
  }

  _hexToRgba(hex, alpha) {
    if (!hex) return `rgba(0, 212, 170, ${alpha})`
    const r = parseInt(hex.slice(1, 3), 16)
    const g = parseInt(hex.slice(3, 5), 16)
    const b = parseInt(hex.slice(5, 7), 16)
    return `rgba(${r}, ${g}, ${b}, ${alpha})`
  }

  // Position de la ligne "maintenant"
  getNowPercent() {
    const now = new Date()
    const hour = now.getHours() + now.getMinutes() / 60
    if (hour < MIN_HOUR || hour > MAX_HOUR) return null
    return this.hourToPercent(hour)
  }

  getCurrentDayValue() {
    return new Date().getDay() // 0=dim, 1=lun...
  }

  handleBarClick(auto, day) {
    this.dispatchEvent(new CustomEvent('slot-click', {
      detail: { hour: auto.startHour, day: day.value, dayName: day.full, automations: [auto] },
      bubbles: true,
      composed: true
    }))
  }

  renderDayColumn(day, scheduled) {
    const dayAutos = scheduled.filter(a => a.days.includes(day.value))
    const nowPct = this.getNowPercent()
    const isToday = day.value === this.getCurrentDayValue()

    return html`
      <div class="day-column">
        <!-- Grid lines -->
        ${this.hourTicks.map(h => html`
          <div class="hour-line" style="top: ${this.hourToPercent(h)}%"></div>
        `)}

        <!-- Automation bars -->
        ${dayAutos.map(auto => {
          const startClamped = Math.max(auto.startHour, MIN_HOUR)
          const endClamped = Math.min(auto.endHour, MAX_HOUR)
          const top = this.hourToPercent(startClamped)
          const height = this.hourToPercent(endClamped) - top
          const isHighlighted = auto.id === this.highlightedId
          const barHeight = height

          return html`
            <div
              class="auto-bar ${isHighlighted ? 'highlighted' : ''}"
              style="
                top: ${top}%;
                height: ${height}%;
                background: ${this.getModeColor(auto.mode, 0.45)};
                border: 1px solid ${this.getModeBorder(auto.mode)};
              "
              title="${auto.name} — ${auto.startHour}h-${auto.endHour}h"
              @click=${() => this.handleBarClick(auto, day)}
              @mouseenter=${() => this.dispatchEvent(new CustomEvent('automation-highlight', { detail: { id: auto.id }, bubbles: true, composed: true }))}
              @mouseleave=${() => this.dispatchEvent(new CustomEvent('automation-highlight', { detail: { id: null }, bubbles: true, composed: true }))}
            >
              <span class="bar-icon">${auto.icon}</span>
              ${barHeight > 20 ? html`<span class="bar-label">${auto.startHour}h-${auto.endHour}h</span>` : ''}
            </div>
          `
        })}

        <!-- Now indicator -->
        ${isToday && nowPct !== null ? html`
          <div class="now-line" style="top: ${nowPct}%"></div>
        ` : ''}
      </div>
    `
  }

  render() {
    const scheduled = this.getScheduledAutomations()
    const usedModes = [...new Set(scheduled.map(s => s.mode).filter(Boolean))]

    return html`
      <div class="timeline-container">
        <div class="timeline-header">
          <div class="timeline-title">
            <span>📅</span>
            <span>Planning Hebdomadaire</span>
          </div>
          <span class="at-schedule-hint">
            ${scheduled.length} automation${scheduled.length > 1 ? 's' : ''} planifiee${scheduled.length > 1 ? 's' : ''}
          </span>
        </div>

        ${scheduled.length === 0 ? html`
          <div class="empty-state">
            <div class="empty-state-icon">📆</div>
            <div>Aucune automation planifiee</div>
          </div>
        ` : ''}

        <div class="planning">
          <!-- Headers -->
          <div class="day-header corner"></div>
          ${this.weekdays.map(day => html`
            <div class="day-header" style="${day.value === this.getCurrentDayValue() ? 'color: var(--primary-color, #00d4aa); font-weight: 700;' : ''}">${day.short}</div>
          `)}

          <!-- Hour axis -->
          <div class="hour-axis">
            ${this.hourTicks.map(h => html`
              <div class="hour-tick" style="top: ${this.hourToPercent(h)}%">${h}h</div>
            `)}
          </div>

          <!-- Day columns -->
          ${this.weekdays.map(day => this.renderDayColumn(day, scheduled))}
        </div>

        ${usedModes.length > 0 ? html`
          <div class="legend">
            ${usedModes.map(mode => html`
              <div class="legend-item">
                <div class="legend-color" style="background: ${this.getModeColor(mode, 0.6)}; border: 1px solid ${this.getModeBorder(mode)};"></div>
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
