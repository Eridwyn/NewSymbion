/**
 * Time Range Picker Component - Symbion
 * Selecteur visuel pour plages horaires et jours
 */

import { LitElement, html, css } from 'lit'
import { focusVisibleStyles } from '../styles/shared-patterns.js'

class TimeRangePicker extends LitElement {
  static properties = {
    startHour: { type: Number },
    endHour: { type: Number },
    selectedDays: { type: Array },
    selectedMonthDays: { type: Array },
    showMonthDays: { type: Boolean }
  }

  static styles = [focusVisibleStyles, css`
    :host {
      display: block;
    }

    .picker-container {
      display: flex;
      flex-direction: column;
      gap: 1.25rem;
    }

    .section {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .section-label {
      font-size: 0.85rem;
      font-weight: 500;
      color: var(--color-dark-text-secondary, #cbd5e1);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    /* Time Range Slider */
    .time-range {
      display: flex;
      align-items: center;
      gap: 1rem;
      padding: 0.75rem;
      background: var(--surface-glass-hover);
      border-radius: var(--radius-base, 0.5rem);
    }

    .time-input {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.25rem;
    }

    .time-input label {
      font-size: 0.7rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .time-input input {
      width: 60px;
      padding: 0.5rem;
      background: var(--color-dark-surface);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 1rem;
      text-align: center;
      transition: border-color var(--duration-base, 0.25s) var(--ease-out, ease-out),
                  box-shadow var(--duration-base, 0.25s) var(--ease-out, ease-out);
    }

    .time-input input:hover {
      border-color: var(--border-hover);
    }

    .time-input input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 3px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1));
    }

    .time-visual {
      flex: 1;
      height: 8px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm);
      position: relative;
      margin: 0 0.5rem;
    }

    .time-visual-fill {
      position: absolute;
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary, #00d4aa), color-mix(in srgb, var(--context-primary) 60%, transparent));
      border-radius: var(--radius-sm);
      transition: all 0.2s ease;
    }

    .time-visual-labels {
      display: flex;
      justify-content: space-between;
      margin-top: 0.25rem;
      font-size: 0.6rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    /* Days of Week */
    .days-grid {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }

    .day-btn {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-width: 48px;
      padding: 0.5rem 0.75rem;
      background: var(--surface-glass-hover);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base, 0.5rem);
      color: var(--color-dark-text-secondary, #cbd5e1);
      cursor: pointer;
      transition: all 0.2s ease;
      font-size: 0.8rem;
    }

    .day-btn:hover {
      background: var(--surface-glass-bright);
    }

    .day-btn.selected {
      background: var(--ctx-bg-strong);
      border-color: var(--context-primary, #00d4aa);
      color: var(--context-primary, #00d4aa);
    }

    .day-btn.weekend {
      color: var(--color-warning-text-muted, #fbbf24);
    }

    .day-btn.weekend.selected {
      background: rgba(251, 191, 36, 0.2);
      border-color: rgba(251, 191, 36, 0.6);
    }

    /* Quick select buttons */
    .quick-select {
      display: flex;
      gap: 0.5rem;
      margin-top: 0.5rem;
    }

    .quick-btn {
      padding: 0.25rem 0.75rem;
      background: var(--surface-glass-strong);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      font-size: 0.7rem;
      transition: all 0.2s ease;
    }

    .quick-btn:hover {
      background: var(--surface-glass-bright);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Month Days Grid */
    .month-days-toggle {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      cursor: pointer;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: 0.8rem;
    }

    .month-days-toggle:hover {
      color: var(--context-primary, #00d4aa);
    }

    .month-days-toggle:focus-visible {
      outline: 2px solid var(--context-primary, #00d4aa);
      outline-offset: 2px;
      border-radius: var(--radius-sm, 4px);
    }

    .month-days-grid {
      display: grid;
      grid-template-columns: repeat(7, 1fr);
      gap: 4px;
      padding: 0.75rem;
      background: var(--surface-glass-hover);
      border-radius: var(--radius-base, 0.5rem);
    }

    .month-day-btn {
      aspect-ratio: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      background: var(--surface-glass-strong);
      border: 1px solid transparent;
      border-radius: var(--radius-sm);
      color: var(--color-dark-text-secondary, #cbd5e1);
      cursor: pointer;
      font-size: var(--text-xs);
      transition: all 0.15s ease;
    }

    .month-day-btn:hover {
      background: var(--surface-glass-bright);
    }

    .day-btn:focus-visible,
    .month-day-btn:focus-visible,
    .quick-btn:focus-visible {
      outline: 2px solid var(--context-primary, #00d4aa);
      outline-offset: 2px;
    }

    .month-day-btn.selected {
      background: var(--ctx-bg-strong);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    .month-day-btn.last-day {
      grid-column: span 2;
      font-size: 0.65rem;
    }

    /* Summary */
    .summary {
      padding: 0.75rem;
      background: var(--surface-glass-subtle);
      border-radius: var(--radius-base, 0.5rem);
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #cbd5e1);
      border-left: 3px solid var(--context-primary, #00d4aa);
    }

    .summary strong {
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    @media (max-width: 480px) {
      .days-grid {
        justify-content: center;
      }

      .day-btn {
        min-width: 40px;
        padding: 0.4rem 0.5rem;
        font-size: var(--text-xs);
      }
    }
  `]

  constructor() {
    super()
    this.startHour = 9
    this.endHour = 18
    this.selectedDays = [1, 2, 3, 4, 5]  // Lun-Ven par defaut
    this.selectedMonthDays = []
    this.showMonthDays = false
  }

  get weekdays() {
    return [
      { value: 0, short: 'Dim', full: 'Dimanche', weekend: true },
      { value: 1, short: 'Lun', full: 'Lundi', weekend: false },
      { value: 2, short: 'Mar', full: 'Mardi', weekend: false },
      { value: 3, short: 'Mer', full: 'Mercredi', weekend: false },
      { value: 4, short: 'Jeu', full: 'Jeudi', weekend: false },
      { value: 5, short: 'Ven', full: 'Vendredi', weekend: false },
      { value: 6, short: 'Sam', full: 'Samedi', weekend: true }
    ]
  }

  // Format time fill bar position
  get timeVisualStyle() {
    const left = (this.startHour / 24) * 100
    const width = ((this.endHour - this.startHour) / 24) * 100
    return `left: ${left}%; width: ${width}%`
  }

  // Generate summary text
  get summaryText() {
    const daysStr = this.selectedDays
      .sort((a, b) => a - b)
      .map(d => this.weekdays.find(w => w.value === d)?.short)
      .filter(Boolean)
      .join(', ')

    let text = `${this.startHour}h - ${this.endHour}h`

    if (daysStr) {
      text += ` | ${daysStr}`
    }

    if (this.selectedMonthDays.length > 0) {
      const hasLastDay = this.selectedMonthDays.includes(31)
      const otherDays = this.selectedMonthDays.filter(d => d !== 31).sort((a, b) => a - b)

      let monthDaysStr
      if (hasLastDay && otherDays.length === 0) {
        // Seulement le dernier jour
        monthDaysStr = 'Dernier jour du mois'
      } else if (hasLastDay) {
        // Jours + dernier jour
        monthDaysStr = `Jour${otherDays.length > 1 ? 's' : ''} ${otherDays.join(', ')} et dernier jour`
      } else {
        // Seulement des jours numeriques
        monthDaysStr = `Jour${otherDays.length > 1 ? 's' : ''} ${otherDays.join(', ')}`
      }
      text += ` | ${monthDaysStr}`
    }

    return text
  }

  _dispatchChange() {
    this.dispatchEvent(new CustomEvent('change', {
      detail: {
        startHour: this.startHour,
        endHour: this.endHour,
        days: this.selectedDays,
        monthDays: this.selectedMonthDays
      },
      bubbles: true,
      composed: true
    }))
  }

  _handleStartHourChange(e) {
    const value = parseInt(e.target.value) || 0
    this.startHour = Math.max(0, Math.min(23, value))
    if (this.startHour >= this.endHour) {
      this.endHour = Math.min(24, this.startHour + 1)
    }
    this._dispatchChange()
  }

  _handleEndHourChange(e) {
    const value = parseInt(e.target.value) || 0
    this.endHour = Math.max(1, Math.min(24, value))
    if (this.endHour <= this.startHour) {
      this.startHour = Math.max(0, this.endHour - 1)
    }
    this._dispatchChange()
  }

  _toggleDay(dayValue) {
    const index = this.selectedDays.indexOf(dayValue)
    if (index === -1) {
      this.selectedDays = [...this.selectedDays, dayValue]
    } else {
      this.selectedDays = this.selectedDays.filter(d => d !== dayValue)
    }
    this._dispatchChange()
  }

  _toggleMonthDay(dayValue) {
    const index = this.selectedMonthDays.indexOf(dayValue)
    if (index === -1) {
      this.selectedMonthDays = [...this.selectedMonthDays, dayValue]
    } else {
      this.selectedMonthDays = this.selectedMonthDays.filter(d => d !== dayValue)
    }
    this._dispatchChange()
  }

  _selectWeekdays() {
    this.selectedDays = [1, 2, 3, 4, 5]
    this._dispatchChange()
  }

  _selectWeekend() {
    this.selectedDays = [0, 6]
    this._dispatchChange()
  }

  _selectAllDays() {
    this.selectedDays = [0, 1, 2, 3, 4, 5, 6]
    this._dispatchChange()
  }

  render() {
    return html`
      <div class="picker-container">
        <!-- Time Range -->
        <div class="section">
          <div class="section-label">
            <span>⏰</span>
            <span>Plage horaire</span>
          </div>
          <div class="time-range">
            <div class="time-input">
              <label>Debut</label>
              <input
                type="number"
                min="0"
                max="23"
                .value=${this.startHour}
                @change=${this._handleStartHourChange}
              />
            </div>

            <div class="time-visual">
              <div class="time-visual-fill" style="${this.timeVisualStyle}"></div>
              <div class="time-visual-labels">
                <span>0h</span>
                <span>6h</span>
                <span>12h</span>
                <span>18h</span>
                <span>24h</span>
              </div>
            </div>

            <div class="time-input">
              <label>Fin</label>
              <input
                type="number"
                min="1"
                max="24"
                .value=${this.endHour}
                @change=${this._handleEndHourChange}
              />
            </div>
          </div>
        </div>

        <!-- Days of Week -->
        <div class="section">
          <div class="section-label">
            <span>📅</span>
            <span>Jours de la semaine</span>
          </div>
          <div class="days-grid">
            ${this.weekdays.map(day => html`
              <button
                class="day-btn ${this.selectedDays.includes(day.value) ? 'selected' : ''} ${day.weekend ? 'weekend' : ''}"
                @click=${() => this._toggleDay(day.value)}
                title=${day.full}
              >
                ${day.short}
              </button>
            `)}
          </div>
          <div class="quick-select">
            <button class="quick-btn" @click=${this._selectWeekdays}>Semaine</button>
            <button class="quick-btn" @click=${this._selectWeekend}>Week-end</button>
            <button class="quick-btn" @click=${this._selectAllDays}>Tous</button>
          </div>
        </div>

        <!-- Month Days (optional) -->
        <div class="section">
          <div
            class="month-days-toggle"
            role="button"
            tabindex="0"
            @click=${() => this.showMonthDays = !this.showMonthDays}
            @keydown=${(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); this.showMonthDays = !this.showMonthDays; } }}
            aria-expanded="${this.showMonthDays}"
          >
            <span>${this.showMonthDays ? '▼' : '▶'}</span>
            <span>📆 Jours du mois specifiques (optionnel)</span>
          </div>

          ${this.showMonthDays ? html`
            <div class="month-days-grid">
              ${Array.from({ length: 30 }, (_, i) => i + 1).map(day => html`
                <button
                  class="month-day-btn ${this.selectedMonthDays.includes(day) ? 'selected' : ''}"
                  @click=${() => this._toggleMonthDay(day)}
                >
                  ${day}
                </button>
              `)}
              <button
                class="month-day-btn last-day ${this.selectedMonthDays.includes(31) ? 'selected' : ''}"
                @click=${() => this._toggleMonthDay(31)}
                title="Dernier jour du mois quel qu'il soit"
              >
                Dernier
              </button>
            </div>
          ` : ''}
        </div>

        <!-- Summary -->
        <div class="summary">
          <strong>Resume:</strong> ${this.summaryText}
        </div>
      </div>
    `
  }
}

customElements.define('time-range-picker', TimeRangePicker)

export { TimeRangePicker }
