/**
 * Notification Center Component
 *
 * Centre de gestion des notifications Symbion
 * - Icône cloche avec badge compteur
 * - Panel latéral avec liste des notifications
 * - Boutons Approve/Reject pour les actions
 * - Mise à jour temps réel via MQTT
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import csrfService from '../services/csrf-service.js'
import { escapeHtml } from '../utils/sanitization.js'

class NotificationCenter extends LitElement {
  static styles = [sharedAnimations, css`
    :host {
      position: relative;
      display: inline-flex;
      align-items: center;
    }

    /* Bouton cloche */
    .bell-button {
      position: relative;
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      border-radius: var(--radius-md, 8px);
      padding: 0.5rem 0.75rem;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 0.25rem;
      color: var(--context-primary, #00d4aa);
      font-size: 1rem;
    }

    .bell-button:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 100%);
      transform: translateY(-2px);
    }

    /* Badge compteur */
    .badge {
      position: absolute;
      top: -4px;
      right: -4px;
      background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
      color: white;
      font-size: 0.6rem;
      font-weight: 700;
      min-width: 16px;
      height: 16px;
      border-radius: var(--radius-base);
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0 4px;
      box-shadow: 0 2px 8px rgba(239, 68, 68, 0.4);
      animation: pulse 2s infinite;
    }

    .badge.hidden {
      display: none;
    }

    /* Modal overlay */
    .modal-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.7);
      backdrop-filter: blur(4px);
      -webkit-backdrop-filter: blur(4px);
      z-index: 9998;
      animation: fadeIn 0.2s ease-out;
    }

    .modal-overlay.hidden {
      display: none;
    }

    /* Panel notifications - Modal centrée viewport */
    .panel {
      position: fixed;
      inset: 0;
      margin: auto;
      width: 90vw;
      height: fit-content;
      max-width: 420px;
      max-height: 80vh;
      background: linear-gradient(135deg,
        rgba(19, 20, 26, 0.99) 0%,
        rgba(10, 10, 11, 1) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: var(--radius-lg, 12px);
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6),
                  0 0 60px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      z-index: 9999;
      overflow: hidden;
      animation: scaleIn 0.2s ease-out;
    }

    .panel.hidden {
      display: none;
    }

    .panel-header {
      padding: 1rem;
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 0.5rem;
    }

    .panel-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
    }

    .close-button {
      background: rgba(255, 255, 255, 0.1);
      border: none;
      color: var(--color-dark-text-secondary, #adb5bd);
      width: 32px;
      height: 32px;
      border-radius: 50%;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 1.1rem;
      transition: all 0.2s;
      flex-shrink: 0;
    }

    .close-button:hover {
      background: rgba(255, 255, 255, 0.2);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .mark-all-read {
      font-size: 0.7rem;
      color: var(--context-primary, #00d4aa);
      background: none;
      border: none;
      cursor: pointer;
      padding: 0.25rem 0.5rem;
      border-radius: var(--radius-sm, 4px);
      transition: background 0.2s;
    }

    .mark-all-read:hover {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
    }

    .panel-content {
      max-height: calc(70vh - 60px);
      overflow-y: auto;
    }

    .panel-content::-webkit-scrollbar {
      width: 6px;
    }

    .panel-content::-webkit-scrollbar-track {
      background: transparent;
    }

    .panel-content::-webkit-scrollbar-thumb {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      border-radius: 3px;
    }

    /* Notification item */
    .notification {
      padding: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
      transition: background 0.2s;
    }

    .notification:hover {
      background: rgba(255, 255, 255, 0.02);
    }

    .notification.unread {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent);
      border-left: 3px solid var(--context-primary, #00d4aa);
    }

    .notification.P0 {
      border-left-color: #ef4444;
    }

    .notification.P1 {
      border-left-color: #fb923c;
    }

    .notification-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: 0.5rem;
    }

    .notification-title {
      font-size: 0.85rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
    }

    .notification-priority {
      font-size: 0.6rem;
      font-weight: 600;
      padding: 0.15rem 0.4rem;
      border-radius: var(--radius-sm, 4px);
      text-transform: uppercase;
    }

    .notification-priority.P0 {
      background: rgba(239, 68, 68, 0.2);
      color: #ff6b6b;
    }

    .notification-priority.P1 {
      background: rgba(251, 146, 60, 0.2);
      color: #fb923c;
    }

    .notification-priority.P2 {
      background: rgba(0, 212, 170, 0.2);
      color: #00d4aa;
    }

    .notification-body {
      font-size: 0.75rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      line-height: 1.4;
      margin-bottom: 0.75rem;
    }

    .notification-meta {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 0.75rem;
    }

    .notification-actions {
      display: flex;
      gap: 0.5rem;
    }

    .action-btn {
      flex: 1;
      padding: 0.4rem 0.6rem;
      border-radius: var(--radius-md, 8px);
      font-size: 0.7rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      border: 1px solid transparent;
    }

    .action-btn.approve {
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.2) 0%, rgba(34, 197, 94, 0.1) 100%);
      border-color: rgba(34, 197, 94, 0.4);
      color: #22c55e;
    }

    .action-btn.approve:hover {
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.3) 0%, rgba(34, 197, 94, 0.2) 100%);
      transform: translateY(-1px);
    }

    .action-btn.reject {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.2) 0%, rgba(239, 68, 68, 0.1) 100%);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ef4444;
    }

    .action-btn.reject:hover {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.3) 0%, rgba(239, 68, 68, 0.2) 100%);
      transform: translateY(-1px);
    }

    .action-btn.ack {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .action-btn.ack:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 100%);
      transform: translateY(-1px);
    }

    /* Empty state */
    .empty-state {
      padding: 2rem;
      text-align: center;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .empty-icon {
      font-size: 2rem;
      margin-bottom: 0.5rem;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 0.8rem;
    }

    /* Loading state */
    .loading {
      padding: 2rem;
      text-align: center;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.8rem;
    }

    /* Mobile */
    @media (max-width: 768px) {
      .bell-button {
        padding: 0.35rem 0.5rem;
        font-size: 0.85rem;
      }

      .badge {
        top: -3px;
        right: -3px;
        min-width: 14px;
        height: 14px;
        font-size: 0.55rem;
      }

      .panel {
        width: 95%;
        max-height: 85vh;
      }

      .panel-content {
        max-height: calc(85vh - 60px);
      }
    }
  `]

  static properties = {
    notifications: { type: Array },
    isOpen: { type: Boolean },
    isLoading: { type: Boolean }
  }

  constructor() {
    super()
    this.notifications = []
    this.isOpen = false
    this.isLoading = true
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadNotifications()
    this.setupMqttListener()
    this._createModalContainer()
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._notificationHandler) {
      document.body.removeEventListener('notification-received', this._notificationHandler)
    }
    if (this._ackHandler) {
      document.body.removeEventListener('notification-acknowledged', this._ackHandler)
    }
    this._removeModalContainer()
  }

  _createModalContainer() {
    // Créer un container dans le body pour la modal (évite les problèmes de shadow DOM)
    this._modalContainer = document.createElement('div')
    this._modalContainer.id = 'notification-modal-container'
    this._modalContainer.innerHTML = `
      <style>
        #notification-modal-overlay {
          position: fixed;
          top: 0;
          left: 0;
          width: 100vw;
          height: 100vh;
          background: rgba(0, 0, 0, 0.7);
          backdrop-filter: blur(4px);
          z-index: 9998;
          display: none;
        }
        #notification-modal-overlay.open {
          display: block;
        }
        #notification-modal {
          position: fixed;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          width: 90%;
          max-width: 400px;
          max-height: 80vh;
          background: linear-gradient(135deg, rgba(19, 20, 26, 0.99) 0%, rgba(10, 10, 11, 1) 100%);
          border: 1px solid rgba(0, 212, 170, 0.25);
          border-radius: var(--radius-md);
          box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
          z-index: 9999;
          display: none;
          overflow: hidden;
        }
        #notification-modal.open {
          display: block;
        }
        .notif-header {
          padding: 1rem;
          border-bottom: 1px solid rgba(0, 212, 170, 0.15);
          display: flex;
          justify-content: space-between;
          align-items: center;
          background: rgba(0, 0, 0, 0.3);
        }
        .notif-title {
          font-size: 1rem;
          font-weight: 600;
          color: #f8f9fa;
        }
        .notif-header-actions {
          display: flex;
          gap: 0.5rem;
          align-items: center;
        }
        .notif-header-btn {
          background: rgba(255,255,255,0.08);
          border: 1px solid rgba(255,255,255,0.15);
          color: #adb5bd;
          padding: 0.35rem 0.6rem;
          border-radius: 6px;
          cursor: pointer;
          font-size: 0.7rem;
          transition: all 0.2s;
        }
        .notif-header-btn:hover {
          background: rgba(255,255,255,0.15);
          color: #fff;
        }
        .notif-header-btn.danger {
          border-color: rgba(239,68,68,0.3);
        }
        .notif-header-btn.danger:hover {
          background: rgba(239,68,68,0.2);
          color: #ef4444;
        }
        .notif-close {
          background: rgba(255,255,255,0.1);
          border: none;
          color: #adb5bd;
          width: 32px;
          height: 32px;
          border-radius: 50%;
          cursor: pointer;
          font-size: 1.2rem;
        }
        .notif-close:hover {
          background: rgba(255,255,255,0.2);
          color: #fff;
        }
        .notif-content {
          max-height: calc(80vh - 60px);
          overflow-y: auto;
          padding: 0;
        }
        .notif-item {
          padding: 1rem;
          border-bottom: 1px solid rgba(255,255,255,0.05);
        }
        .notif-item.unread {
          background: rgba(0, 212, 170, 0.05);
          border-left: 3px solid #00d4aa;
        }
        .notif-item-header {
          display: flex;
          justify-content: space-between;
          margin-bottom: 0.5rem;
        }
        .notif-item-title {
          font-size: 0.85rem;
          font-weight: 600;
          color: #f8f9fa;
        }
        .notif-priority {
          font-size: 0.6rem;
          padding: 0.15rem 0.4rem;
          border-radius: 4px;
          font-weight: 600;
        }
        .notif-priority.P0 { background: rgba(239,68,68,0.2); color: #ff6b6b; }
        .notif-priority.P1 { background: rgba(251,146,60,0.2); color: #fb923c; }
        .notif-priority.P2 { background: rgba(0,212,170,0.2); color: #00d4aa; }
        .notif-body {
          font-size: 0.75rem;
          color: #adb5bd;
          margin-bottom: 0.5rem;
        }
        .notif-meta {
          font-size: 0.65rem;
          color: #6c757d;
          margin-bottom: 0.5rem;
        }
        .notif-actions {
          display: flex;
          gap: 0.5rem;
        }
        .notif-btn {
          flex: 1;
          padding: 0.5rem;
          border-radius: 6px;
          font-size: 0.75rem;
          font-weight: 600;
          cursor: pointer;
          border: 1px solid transparent;
        }
        .notif-btn-ack {
          background: rgba(0,212,170,0.15);
          border-color: rgba(0,212,170,0.4);
          color: #00d4aa;
        }
        .notif-btn-ack:hover {
          background: rgba(0,212,170,0.25);
        }
        .notif-btn-delete {
          background: rgba(239,68,68,0.15);
          border-color: rgba(239,68,68,0.4);
          color: #ef4444;
        }
        .notif-btn-delete:hover {
          background: rgba(239,68,68,0.25);
        }
        .notif-empty {
          padding: 2rem;
          text-align: center;
          color: #6c757d;
        }
      </style>
      <div id="notification-modal-overlay"></div>
      <div id="notification-modal">
        <div class="notif-header">
          <span class="notif-title">Notifications</span>
          <div class="notif-header-actions">
            <button class="notif-header-btn" id="mark-all-read-btn">✓ Tout lu</button>
            <button class="notif-header-btn danger" id="delete-all-btn">🗑 Tout suppr.</button>
          </div>
          <button class="notif-close" aria-label="Fermer" title="Fermer">✕</button>
        </div>
        <div class="notif-content"></div>
      </div>
    `
    document.body.appendChild(this._modalContainer)

    // Event listeners
    this._modalContainer.querySelector('#notification-modal-overlay').addEventListener('click', () => this.closePanel())
    this._modalContainer.querySelector('.notif-close').addEventListener('click', () => this.closePanel())
    this._modalContainer.querySelector('#mark-all-read-btn').addEventListener('click', () => this.markAllAsRead())
    this._modalContainer.querySelector('#delete-all-btn').addEventListener('click', () => this.deleteAllNotifications())
  }

  _removeModalContainer() {
    if (this._modalContainer) {
      this._modalContainer.remove()
    }
  }

  _updateModalContent() {
    const content = this._modalContainer?.querySelector('.notif-content')
    if (!content) return

    if (this.notifications.length === 0) {
      content.innerHTML = '<div class="notif-empty">🔕 Aucune notification</div>'
      return
    }

    content.innerHTML = this.notifications.map(n => `
      <div class="notif-item ${n.acknowledged ? '' : 'unread'}">
        <div class="notif-item-header">
          <span class="notif-item-title">${escapeHtml(n.title)}</span>
          <span class="notif-priority ${escapeHtml(n.priority)}">${escapeHtml(n.priority)}</span>
        </div>
        <div class="notif-body">${escapeHtml(n.body)}</div>
        <div class="notif-meta">${escapeHtml(n.source)} • ${this.formatTime(n.timestamp)}</div>
        <div class="notif-actions">
          ${!n.acknowledged ? `
            <button class="notif-btn notif-btn-ack" data-id="${escapeHtml(n.id)}">✓ Lu</button>
          ` : ''}
          <button class="notif-btn notif-btn-delete" data-id="${escapeHtml(n.id)}">🗑 Supprimer</button>
        </div>
      </div>
    `).join('')

    // Ajouter les event listeners pour les boutons acknowledge
    content.querySelectorAll('.notif-btn-ack').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const id = e.target.dataset.id
        const notif = this.notifications.find(n => n.id === id)
        if (notif) this.acknowledgeNotification(notif)
      })
    })

    // Ajouter les event listeners pour les boutons delete
    content.querySelectorAll('.notif-btn-delete').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const id = e.target.dataset.id
        this.deleteNotification(id)
      })
    })
  }

  async loadNotifications() {
    this.isLoading = true
    try {
      // Utilise le nouvel endpoint kernel (intégré) avec authentification CSRF/JWT
      const response = await csrfService.fetchWithCsrf('/notifications', {
        method: 'GET'
      })
      if (response.ok) {
        // Le kernel retourne directement un array (pas {notifications: []})
        const data = await response.json()
        const apiNotifs = Array.isArray(data) ? data : (data.notifications || [])

        // [Fix] Merger API avec notifications locales au lieu d'écraser
        // Garder les notifications locales (MQTT) qui ne sont pas dans l'API
        const apiIds = new Set(apiNotifs.map(n => n.id))
        const localOnly = this.notifications.filter(n => !apiIds.has(n.id))

        // API d'abord (source de vérité), puis locales non persistées
        this.notifications = [...apiNotifs, ...localOnly]
        this._updateModalContent()
      }
      // Si l'API échoue, on garde les notifications locales (pas d'écrasement)
    } catch (e) {
      console.error('[notification-center] Failed to load notifications:', e)
      // Garder les notifications locales, juste mettre à jour le contenu modal
      this._updateModalContent()
    }
    this.isLoading = false
  }

  setupMqttListener() {
    this._notificationHandler = (e) => {
      console.log('[notification-center] MQTT notification received:', e.detail)
      const notification = e.detail?.notification
      if (notification) {
        console.log('[notification-center] Adding notification:', notification.title)
        // Ajouter au début de la liste
        this.notifications = [notification, ...this.notifications.filter(n => n.id !== notification.id)]
        this._updateModalContent()
        this.requestUpdate() // Met à jour le badge
      }
    }
    document.body.addEventListener('notification-received', this._notificationHandler)
    console.log('[notification-center] MQTT listener registered on document.body')

    // Écouter les acquittements depuis les toasts
    this._ackHandler = (e) => {
      const notificationId = e.detail?.notificationId
      if (notificationId) {
        console.log('[notification-center] Notification acknowledged from toast:', notificationId)
        // Marquer comme acknowledged dans la liste locale
        this.notifications = this.notifications.map(n =>
          (n.id === notificationId) ? { ...n, acknowledged: true } : n
        )
        this.requestUpdate()
      }
    }
    document.body.addEventListener('notification-acknowledged', this._ackHandler)
  }

  get unreadCount() {
    return this.notifications.filter(n => !n.acknowledged).length
  }

  togglePanel(e) {
    e.stopPropagation()
    this.isOpen = !this.isOpen

    const overlay = this._modalContainer?.querySelector('#notification-modal-overlay')
    const modal = this._modalContainer?.querySelector('#notification-modal')

    if (this.isOpen) {
      overlay?.classList.add('open')
      modal?.classList.add('open')
      this.loadNotifications()
    } else {
      overlay?.classList.remove('open')
      modal?.classList.remove('open')
    }
  }

  formatTime(timestamp) {
    const date = new Date(timestamp * 1000)
    const now = new Date()
    const diff = (now - date) / 1000

    if (diff < 60) return 'À l\'instant'
    if (diff < 3600) return `Il y a ${Math.floor(diff / 60)} min`
    if (diff < 86400) return `Il y a ${Math.floor(diff / 3600)}h`
    return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  }

  async acknowledgeNotification(notif) {
    console.log('[notification-center] Acknowledging:', notif.id)

    // Appeler le nouvel endpoint kernel (avec CSRF)
    try {
      const response = await csrfService.fetchWithCsrf(`/notifications/${notif.id}/acknowledge`, {
        method: 'POST'
      })
      if (response.ok) {
        console.log('[notification-center] Acknowledged successfully')
      }
    } catch (e) {
      console.error('[notification-center] Acknowledge failed:', e)
    }

    // Mettre à jour localement
    this.notifications = this.notifications.map(n =>
      n.id === notif.id ? { ...n, acknowledged: true } : n
    )
    this._updateModalContent()
  }

  async deleteNotification(notificationId) {
    console.log('[notification-center] Deleting:', notificationId)

    try {
      const response = await csrfService.fetchWithCsrf(`/notifications/${notificationId}`, {
        method: 'DELETE'
      })
      if (response.ok) {
        console.log('[notification-center] Deleted successfully')
      }
    } catch (e) {
      console.error('[notification-center] Delete failed:', e)
    }

    // Supprimer localement
    this.notifications = this.notifications.filter(n => n.id !== notificationId)
    this._updateModalContent()
    this.requestUpdate() // Met à jour le badge
  }

  async approveAction(notif) {
    console.log('[notification-center] Approving:', notif.id)
    // TODO: Appeler l'API de validation du Decision Engine
    // Pour l'instant, juste acknowledge
    await this.acknowledgeNotification(notif)
  }

  async rejectAction(notif) {
    console.log('[notification-center] Rejecting:', notif.id)
    // TODO: Appeler l'API de rejet du Decision Engine
    await this.acknowledgeNotification(notif)
  }

  markAllAsRead() {
    this.notifications.filter(n => !n.acknowledged).forEach(n => {
      this.acknowledgeNotification(n)
    })
  }

  async deleteAllNotifications() {
    if (this.notifications.length === 0) return

    console.log('[notification-center] Deleting all notifications')

    // Supprimer chaque notification via l'API
    const deletePromises = this.notifications.map(notif =>
      csrfService.fetchWithCsrf(`/notifications/${notif.id}`, { method: 'DELETE' })
        .catch(e => console.error('[notification-center] Delete failed for:', notif.id, e))
    )
    await Promise.all(deletePromises)

    // Vider localement
    this.notifications = []
    this._updateModalContent()
    this.requestUpdate()
    console.log('[notification-center] All notifications deleted')
  }

  hasActions(notif) {
    return notif.actions && notif.actions.length > 0
  }

  closePanel() {
    this.isOpen = false
    const overlay = this._modalContainer?.querySelector('#notification-modal-overlay')
    const modal = this._modalContainer?.querySelector('#notification-modal')
    overlay?.classList.remove('open')
    modal?.classList.remove('open')
  }

  render() {
    return html`
      <button class="bell-button" @click="${this.togglePanel}" aria-label="Notifications" title="Notifications">
        🔔
        <span class="badge ${this.unreadCount === 0 ? 'hidden' : ''}" aria-label="${this.unreadCount} notifications non lues">
          ${this.unreadCount > 9 ? '9+' : this.unreadCount}
        </span>
      </button>
    `
  }
}

customElements.define('notification-center', NotificationCenter)

export { NotificationCenter }
