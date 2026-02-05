/**
 * Helper centralisé pour les notifications utilisateur
 *
 * Priorités:
 * - P0: Critique (rouge) - dismiss manuel
 * - P1: Erreur (orange) - dismiss manuel
 * - P2: Info/Succès (cyan) - auto-hide 10s
 *
 * Usage:
 *   import { notify, notifyError, notifySuccess } from '../utils/notification-helper.js'
 *   notifyError('Connexion perdue', 'Tentative de reconnexion...', 'mqtt')
 *   notifySuccess('Connexion rétablie', '', 'mqtt')
 */

/**
 * Envoie une notification à l'utilisateur via le système de toast
 * @param {string} title - Titre de la notification
 * @param {string} body - Corps du message
 * @param {string} priority - 'P0' | 'P1' | 'P2'
 * @param {string} source - Identifiant du service émetteur
 */
export function notify(title, body, priority = 'P2', source = 'system') {
  const notification = {
    id: `${source}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    title,
    body,
    priority,
    source,
    timestamp: Math.floor(Date.now() / 1000)
  }

  document.body.dispatchEvent(new CustomEvent('notification-received', {
    detail: { notification },
    bubbles: true,
    composed: true
  }))
}

/**
 * Notification d'erreur (P1 - orange, dismiss manuel)
 */
export function notifyError(title, body, source = 'system') {
  notify(title, body, 'P1', source)
}

/**
 * Notification de succès (P2 - cyan, auto-hide)
 */
export function notifySuccess(title, body = '', source = 'system') {
  notify(title, body, 'P2', source)
}

/**
 * Notification critique (P0 - rouge, dismiss manuel)
 */
export function notifyCritical(title, body, source = 'system') {
  notify(title, body, 'P0', source)
}
