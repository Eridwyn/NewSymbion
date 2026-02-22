/**
 * Toast Notifications Component
 *
 * Affiche les notifications Symbion en temps réel via MQTT
 * Position fixe en bas à droite avec animations fluides
 */

import { LitElement, html, css } from 'lit'

class ToastNotifications extends LitElement {
  static styles = css`
    :host {
      position: fixed;
      bottom: var(--space-6, 1.5rem);
      right: var(--space-6, 1.5rem);
      z-index: 100001; /* Au dessus de tous les modals (9999) et boot-terminal (100000) */
      display: flex;
      flex-direction: column-reverse;
      gap: var(--space-3, 0.75rem);
      max-width: 400px;
      pointer-events: none;
    }

    .toast {
      pointer-events: auto;
      background: linear-gradient(135deg,
        rgba(19, 20, 26, 0.95) 0%,
        rgba(10, 10, 11, 0.98) 100%);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border-radius: var(--radius-lg, 12px);
      padding: var(--space-4, 1rem);
      border: 1px solid var(--border-medium);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4),
                  0 0 0 1px var(--surface-glass);
      animation: slideIn 0.3s ease-out;
      transition: all var(--duration-base) var(--ease-out);
    }

    .toast.exiting {
      animation: slideOut 0.3s ease-in forwards;
    }

    @keyframes slideIn {
      from {
        opacity: 0;
        transform: translateX(100px) scale(0.9);
      }
      to {
        opacity: 1;
        transform: translateX(0) scale(1);
      }
    }

    @keyframes slideOut {
      from {
        opacity: 1;
        transform: translateX(0) scale(1);
      }
      to {
        opacity: 0;
        transform: translateX(100px) scale(0.9);
      }
    }

    /* Priorités */
    .toast.P0 {
      border-color: rgba(239, 68, 68, 0.5);
      box-shadow: 0 8px 32px rgba(239, 68, 68, 0.3),
                  0 0 20px rgba(239, 68, 68, 0.2);
    }

    .toast.P1 {
      border-color: rgba(251, 146, 60, 0.5);
      box-shadow: 0 8px 32px rgba(251, 146, 60, 0.2),
                  0 0 20px rgba(251, 146, 60, 0.15);
    }

    .toast.P2 {
      border-color: var(--ctx-border-medium);
    }

    .toast-header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: var(--space-3, 0.75rem);
      margin-bottom: var(--space-2, 0.5rem);
    }

    .toast-priority {
      font-size: 0.65rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      padding: 0.2rem 0.5rem;
      border-radius: var(--radius-sm, 4px);
      flex-shrink: 0;
    }

    .toast-priority.P0 {
      background: rgba(239, 68, 68, 0.2);
      color: #ff6b6b;
    }

    .toast-priority.P1 {
      background: rgba(251, 146, 60, 0.2);
      color: #fb923c;
    }

    .toast-priority.P2 {
      background: var(--ctx-bg-strong);
      color: var(--context-primary, #00d4aa);
    }

    .toast-title {
      font-size: var(--text-sm, 0.875rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
      line-height: 1.3;
    }

    .toast-body {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-secondary, #adb5bd);
      line-height: 1.5;
      margin-bottom: var(--space-3, 0.75rem);
    }

    .toast-actions {
      display: flex;
      gap: var(--space-2, 0.5rem);
      justify-content: flex-end;
    }

    .toast-btn {
      padding: 0.4rem 0.8rem;
      border-radius: var(--radius-md, 8px);
      font-size: 0.7rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s ease;
      border: 1px solid transparent;
    }

    .toast-btn-ack {
      background: linear-gradient(135deg,
        var(--ctx-bg-strong) 0%,
        var(--ctx-bg) 100%);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    .toast-btn-ack:hover {
      background: linear-gradient(135deg,
        var(--ctx-border-medium) 0%,
        var(--ctx-bg-strong) 100%);
      transform: translateY(-1px);
    }

    .toast-btn-dismiss {
      background: var(--surface-glass);
      border-color: var(--border-medium);
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .toast-btn-dismiss:hover {
      background: var(--surface-glass-strong);
    }

    .toast-source {
      font-size: 0.6rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-top: var(--space-2, 0.5rem);
      font-family: var(--font-mono, monospace);
    }

    /* Mobile adjustments */
    @media (max-width: 480px) {
      :host {
        left: var(--space-3, 0.75rem);
        right: var(--space-3, 0.75rem);
        bottom: var(--space-4, 1rem);
        max-width: none;
      }
    }
  `

  static properties = {
    toasts: { type: Array },
    mqttService: { type: Object }
  }

  constructor() {
    super()
    this.toasts = []
    this.mqttService = null
    this.maxToasts = 5
    this.autoHideDelay = 10000 // 10 secondes pour P2, pas d'auto-hide pour P0/P1
    this._timeouts = new Map() // Track timeouts for cleanup
  }

  connectedCallback() {
    super.connectedCallback()
    this.setupNotificationListener()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._notificationHandler) {
      document.body.removeEventListener('notification-received', this._notificationHandler)
    }
    // Clear all pending timeouts to prevent memory leaks
    for (const timeoutId of this._timeouts.values()) {
      clearTimeout(timeoutId)
    }
    this._timeouts.clear()
  }

  setupNotificationListener() {
    // Écouter l'événement notification-received du mqtt-service
    this._notificationHandler = (e) => {
      const notification = e.detail?.notification
      if (notification) {
        this.addToast(notification)
      }
    }

    // Le mqtt-service est ajouté au body, donc écouter sur body
    document.body.addEventListener('notification-received', this._notificationHandler)
    console.log('[toast] Listening for notification-received events')
  }

  addToast(notification) {
    const toast = {
      ...notification,
      _id: notification.id || crypto.randomUUID(),
      _timestamp: Date.now()
    }

    console.log(`[toast] New notification: ${toast.title} (${toast.priority})`)

    // Ajouter au début
    this.toasts = [toast, ...this.toasts].slice(0, this.maxToasts)

    // Auto-hide pour P2 uniquement - track timeout for cleanup
    if (notification.priority === 'P2') {
      const timeoutId = setTimeout(() => {
        this._timeouts.delete(toast._id)
        this.dismissToast(toast._id)
      }, this.autoHideDelay)
      this._timeouts.set(toast._id, timeoutId)
    }
  }

  async acknowledgeToast(toast) {
    console.log(`[toast] Acknowledging: ${toast._id}`)

    // Publier sur MQTT via le mqtt-service
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      const payload = { notification_id: toast.id || toast._id }
      mqttService.publish('symbion/notifications/acknowledge@v1', payload)
    }

    // Notifier notification-center pour mettre à jour le badge
    document.body.dispatchEvent(new CustomEvent('notification-acknowledged', {
      detail: { notificationId: toast.id || toast._id }
    }))

    // Retirer du DOM avec animation
    this.dismissToast(toast._id)
  }

  dismissToast(toastId) {
    // Clear any pending auto-hide timeout for this toast
    if (this._timeouts.has(toastId)) {
      clearTimeout(this._timeouts.get(toastId))
      this._timeouts.delete(toastId)
    }

    // Marquer pour animation de sortie
    const toastEl = this.shadowRoot.querySelector(`[data-id="${toastId}"]`)
    if (toastEl) {
      toastEl.classList.add('exiting')
      const animKey = `anim_${toastId}`
      const animTimeout = setTimeout(() => {
        this._timeouts.delete(animKey)
        this.toasts = this.toasts.filter(t => t._id !== toastId)
      }, 300)
      this._timeouts.set(animKey, animTimeout)
    } else {
      this.toasts = this.toasts.filter(t => t._id !== toastId)
    }
  }

  render() {
    // aria-live="polite" annonce les nouveaux toasts aux lecteurs d'écran
    // role="status" indique que c'est une zone de statut dynamique
    return html`
      <div role="status" aria-live="polite" aria-atomic="false">
        ${this.toasts.map(toast => html`
          <div class="toast ${toast.priority}" data-id="${toast._id}" role="alert">
            <div class="toast-header">
              <span class="toast-title">${toast.title}</span>
              <span class="toast-priority ${toast.priority}" aria-label="Priorité ${toast.priority}">${toast.priority}</span>
            </div>
            <div class="toast-body">${toast.body}</div>
            <div class="toast-actions">
              <button class="toast-btn toast-btn-dismiss"
                      aria-label="Fermer la notification"
                      @click="${() => this.dismissToast(toast._id)}">
                Fermer
              </button>
              <button class="toast-btn toast-btn-ack"
                      aria-label="Marquer comme vu"
                      @click="${() => this.acknowledgeToast(toast)}">
                ✓ Vu
              </button>
            </div>
            <div class="toast-source">Source: ${toast.source}</div>
          </div>
        `)}
      </div>
    `
  }
}

customElements.define('toast-notifications', ToastNotifications)

export { ToastNotifications }
