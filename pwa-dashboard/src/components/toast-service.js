import { LitElement, html, css } from 'lit'

class ToastService extends LitElement {
  static styles = css`
    :host {
      position: fixed;
      bottom: calc(80px + env(safe-area-inset-bottom, 0px));
      left: 50%;
      transform: translateX(-50%);
      z-index: 10000;
      display: flex;
      flex-direction: column-reverse;
      gap: 8px;
      pointer-events: none;
      max-width: min(90vw, 420px);
      width: 100%;
    }

    .toast {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 0.75rem 1rem;
      border-radius: var(--radius-base, 8px);
      font-size: 0.875rem;
      line-height: 1.4;
      pointer-events: auto;
      animation: slideUp 0.25s ease-out;
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      box-shadow: 0 4px 24px rgba(0,0,0,0.4);
      touch-action: pan-x;
      transition: transform 0.2s ease, opacity 0.2s ease;
      will-change: transform, opacity;
    }

    .toast.dismissing {
      animation: slideOut 0.2s ease-in forwards;
    }

    .toast.success {
      background: rgba(16, 185, 129, 0.9);
      color: #fff;
      border: 1px solid rgba(16, 185, 129, 0.6);
    }

    .toast.error {
      background: rgba(239, 68, 68, 0.9);
      color: #fff;
      border: 1px solid rgba(239, 68, 68, 0.6);
    }

    .toast.warning {
      background: rgba(245, 158, 11, 0.9);
      color: #fff;
      border: 1px solid rgba(245, 158, 11, 0.6);
    }

    .toast.info {
      background: rgba(99, 102, 241, 0.9);
      color: #fff;
      border: 1px solid rgba(99, 102, 241, 0.6);
    }

    .toast-icon { font-size: 1.1em; flex-shrink: 0; }
    .toast-msg { flex: 1; min-width: 0; }

    .toast-close {
      background: none;
      border: none;
      color: inherit;
      opacity: 0.7;
      cursor: pointer;
      padding: 4px;
      font-size: 1.1em;
      line-height: 1;
      flex-shrink: 0;
      min-width: 28px;
      min-height: 28px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 50%;
    }
    .toast-close:hover { opacity: 1; background: rgba(255,255,255,0.15); }

    @keyframes slideUp {
      from { transform: translateY(100%); opacity: 0; }
      to { transform: translateY(0); opacity: 1; }
    }

    @keyframes slideOut {
      from { transform: translateX(0); opacity: 1; }
      to { transform: translateX(100%); opacity: 0; }
    }

    @media (prefers-reduced-motion: reduce) {
      .toast { animation-duration: 0.01ms; }
    }
  `

  static properties = {
    toasts: { type: Array }
  }

  constructor() {
    super()
    this.toasts = []
    this._counter = 0
    this._swipeState = null
  }

  connectedCallback() {
    super.connectedCallback()
    this._boundShow = (e) => this.show(e.detail.message, e.detail.type, e.detail.duration)
    window.addEventListener('toast-show', this._boundShow)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    window.removeEventListener('toast-show', this._boundShow)
  }

  show(message, type = 'info', duration = null) {
    const id = ++this._counter
    const defaultDurations = { success: 3000, error: 6000, warning: 5000, info: 3000 }
    const ms = duration || defaultDurations[type] || 3000

    this.toasts = [...this.toasts, { id, message, type }]

    // Max 3 visible
    if (this.toasts.length > 3) {
      this.toasts = this.toasts.slice(-3)
    }

    setTimeout(() => this.dismiss(id), ms)
    this.requestUpdate()
  }

  dismiss(id) {
    const el = this.shadowRoot?.querySelector(`[data-id="${id}"]`)
    if (el) {
      el.classList.add('dismissing')
      setTimeout(() => {
        this.toasts = this.toasts.filter(t => t.id !== id)
        this.requestUpdate()
      }, 200)
    } else {
      this.toasts = this.toasts.filter(t => t.id !== id)
      this.requestUpdate()
    }
  }

  _onTouchStart(e, id) {
    const touch = e.touches[0]
    this._swipeState = { id, startX: touch.clientX, currentX: touch.clientX }
  }

  _onTouchMove(e, id) {
    if (!this._swipeState || this._swipeState.id !== id) return
    this._swipeState.currentX = e.touches[0].clientX
    const dx = this._swipeState.currentX - this._swipeState.startX
    if (dx > 0) {
      const el = this.shadowRoot?.querySelector(`[data-id="${id}"]`)
      if (el) el.style.transform = `translateX(${dx}px)`
    }
  }

  _onTouchEnd(e, id) {
    if (!this._swipeState || this._swipeState.id !== id) return
    const dx = this._swipeState.currentX - this._swipeState.startX
    if (dx > 80) {
      this.dismiss(id)
    } else {
      const el = this.shadowRoot?.querySelector(`[data-id="${id}"]`)
      if (el) el.style.transform = ''
    }
    this._swipeState = null
  }

  _getIcon(type) {
    const icons = { success: '\u2713', error: '\u2715', warning: '\u26A0', info: '\u2139' }
    return icons[type] || '\u2139'
  }

  render() {
    return html`
      ${this.toasts.map(t => html`
        <div class="toast ${t.type}" data-id="${t.id}" role="${t.type === 'error' ? 'alert' : 'status'}" aria-live="${t.type === 'error' ? 'assertive' : 'polite'}"
          @touchstart=${(e) => this._onTouchStart(e, t.id)}
          @touchmove=${(e) => this._onTouchMove(e, t.id)}
          @touchend=${(e) => this._onTouchEnd(e, t.id)}>
          <span class="toast-icon">${this._getIcon(t.type)}</span>
          <span class="toast-msg">${t.message}</span>
          <button class="toast-close" @click=${() => this.dismiss(t.id)} aria-label="Fermer">\u00D7</button>
        </div>
      `)}
    `
  }
}

customElements.define('toast-service', ToastService)

// Helper function for easy usage anywhere
export function showToast(message, type = 'info', duration = null) {
  window.dispatchEvent(new CustomEvent('toast-show', { detail: { message, type, duration } }))
}

export { ToastService }
