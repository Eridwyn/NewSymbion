/**
 * Modale Patterns Symbion - Full Screen Bio-Organic
 *
 * Affiche tous les patterns détectés en plein écran
 */

import { LitElement, html, css } from 'lit'
import { getDayNameShort, utcHourToLocal } from '../utils/time-utils.js'

class PatternsModal extends LitElement {
  static styles = css`
    :host {
      display: block;
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: radial-gradient(ellipse at center,
        color-mix(in srgb, var(--context-primary, #00d4aa) 3%, rgba(0, 0, 0, 0.85)) 0%,
        rgba(0, 0, 0, 0.9) 100%);
      backdrop-filter: blur(var(--blur-xl));
      -webkit-backdrop-filter: blur(var(--blur-xl));
      z-index: 9999;
      overflow-y: auto;
      animation: fadeIn 0.3s ease-out;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    .modal-container {
      max-width: 900px;
      margin: 2rem auto;
      padding: 2rem;
      overflow-x: hidden;
      animation: modalSlideIn 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    }

    @keyframes modalSlideIn {
      from {
        opacity: 0;
        transform: translateY(-30px) scale(0.95);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    .modal-content {
      background: linear-gradient(135deg, rgba(30, 30, 30, 0.98) 0%, rgba(20, 20, 20, 0.98) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-radius: 16px;
      padding: 2rem;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6),
                  0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
    }

    .modal-header {
      position: relative;
      margin-bottom: 1.5rem;
      padding-bottom: 1rem;
      padding-right: 100px;
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      animation: modalHeaderSlideIn 0.5s ease-out 0.1s backwards;
    }

    @keyframes modalHeaderSlideIn {
      from {
        opacity: 0;
        transform: translateY(-10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .modal-header::after {
      content: '';
      position: absolute;
      bottom: -1px;
      left: 0;
      width: 30%;
      height: 2px;
      background: linear-gradient(90deg,
        var(--context-primary, #00d4aa) 0%,
        transparent 100%);
      opacity: 0.8;
      box-shadow: 0 0 10px var(--context-primary, #00d4aa);
    }

    .modal-title {
      font-size: 1.5rem;
      font-weight: 700;
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white) 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      filter: drop-shadow(0 0 15px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent));
      animation: titlePulse 4s ease-in-out infinite;
    }

    @keyframes titlePulse {
      0%, 100% {
        filter: drop-shadow(0 0 15px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent));
      }
      50% {
        filter: drop-shadow(0 0 25px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent));
      }
    }

    .close-btn {
      position: absolute;
      top: 0;
      right: 0;
      padding: 0.5rem 1rem;
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: 8px;
      background: linear-gradient(135deg,
        rgba(255, 107, 107, 0.15) 0%,
        rgba(255, 107, 107, 0.08) 100%);
      color: #ff6b6b;
      cursor: pointer;
      transition: all 0.3s ease;
      font-size: 1rem;
    }

    .close-btn:hover {
      background: linear-gradient(135deg,
        rgba(255, 107, 107, 0.25) 0%,
        rgba(255, 107, 107, 0.15) 100%);
      border-color: rgba(255, 107, 107, 0.5);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px rgba(255, 107, 107, 0.3);
    }

    .patterns-list {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
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
      max-width: 100%;
      overflow: hidden;
      transition: all var(--duration-base) var(--ease-out);
      animation: patternItemSlideIn 0.5s cubic-bezier(0.4, 0, 0.2, 1) backwards;
    }

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
      min-width: 0;
    }

    .pattern-description {
      font-size: 0.875rem;
      color: #e0e0e0;
      font-weight: 500;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .pattern-meta {
      font-size: 0.75rem;
      color: #808080;
      margin-top: 0.25rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .pattern-confidence {
      padding: 0.3rem 0.7rem;
      border-radius: 12px;
      font-size: 0.75rem;
      font-weight: 600;
      background: linear-gradient(135deg,
        rgba(16, 185, 129, 0.2) 0%,
        rgba(16, 185, 129, 0.1) 100%);
      color: #10b981;
      border: 1px solid rgba(16, 185, 129, 0.4);
      box-shadow: 0 0 10px rgba(16, 185, 129, 0.2);
      transition: all var(--duration-base) var(--ease-out);
      animation: confidencePulse 3s ease-in-out infinite;
      flex-shrink: 0;
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
      padding: 2rem;
      color: #808080;
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    /* RESPONSIVE MOBILE */
    @media (max-width: 768px) {
      .modal-container {
        padding: 1rem;
        margin: 1rem;
      }

      .modal-content {
        padding: 1.5rem 1rem;
        border-radius: 16px 16px 0 0;
      }

      .modal-header {
        padding-right: 60px;
        margin-bottom: 1rem;
        padding-bottom: 0.75rem;
      }

      .modal-title {
        font-size: 1.25rem;
      }

      .close-btn {
        padding: 0.4rem 0.75rem;
        font-size: 0.875rem;
        border-radius: 8px;
      }

      .pattern-item {
        padding: 0.6rem;
        gap: 0.5rem;
        flex-wrap: wrap;
      }

      .pattern-icon {
        font-size: 1.3rem;
      }

      .pattern-description {
        font-size: 0.75rem;
      }

      .pattern-meta {
        font-size: 0.65rem;
      }

      .pattern-confidence {
        padding: 0.2rem 0.5rem;
        font-size: 0.65rem;
      }

      .pattern-item {
        animation-duration: 0.3s;
      }
    }

    @media (max-width: 480px) {
      .modal-container {
        padding: 0.5rem;
        margin: 0.5rem;
      }

      .modal-content {
        padding: 1rem 0.75rem;
        border-radius: 12px 12px 0 0;
      }

      .modal-header {
        padding-right: 50px;
      }

      .modal-title {
        font-size: 1.1rem;
      }

      .close-btn {
        padding: 0.35rem 0.6rem;
        font-size: 0.8rem;
      }

      .pattern-item {
        padding: 0.5rem;
        gap: 0.4rem;
      }

      .pattern-description {
        font-size: 0.7rem;
        white-space: normal;
        line-height: 1.3;
      }

      .pattern-meta {
        font-size: 0.6rem;
      }
    }
  `

  static properties = {
    patterns: { type: Array }
  }

  constructor() {
    super()
    this.patterns = []
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

  getDayName(dayNumber) {
    // Use centralized ISO convention (0=Monday from kernel)
    return getDayNameShort(dayNumber)
  }

  formatDate(dateString) {
    try {
      const parts = dateString.split(' ')
      const datePart = parts[0]
      const timePart = parts[1]?.split('.')[0] || '00:00:00'

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
      console.error('[patterns-modal] Invalid date format:', dateString, e)
      return 'N/A'
    }
  }

  handleClose() {
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }))
  }

  handleBackdropClick(e) {
    if (e.target === e.currentTarget) {
      this.handleClose()
    }
  }

  render() {
    return html`
      <div class="modal-container" @click="${this.handleBackdropClick}">
        <div class="modal-content" @click="${(e) => e.stopPropagation()}">
          <div class="modal-header">
            <div class="modal-title">📋 Tous les Patterns (${this.patterns.length})</div>
            <button class="close-btn" @click="${this.handleClose}">✕ Fermer</button>
          </div>

          ${this.patterns.length === 0 ? html`
            <div class="empty-state">
              <div class="empty-icon">🔍</div>
              <div>Aucun pattern détecté</div>
            </div>
          ` : html`
            <div class="patterns-list">
              ${this.patterns.map(pattern => html`
                <div class="pattern-item">
                  <div class="pattern-icon">${this.getModeIcon(pattern.mode)}</div>
                  <div class="pattern-info">
                    <div class="pattern-description">
                      ${this.getModeName(pattern.mode)} - ${this.getDayName(pattern.day_of_week)} à ${utcHourToLocal(pattern.hour)}h
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
          `}
        </div>
      </div>
    `
  }
}

customElements.define('patterns-modal', PatternsModal)

export { PatternsModal }
