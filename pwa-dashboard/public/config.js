/**
 * Configuration Runtime Dashboard Symbion
 *
 * Ce fichier est chargé AVANT l'application et permet de configurer
 * l'URL de l'API Symbion selon l'environnement de déploiement.
 *
 * IMPORTANT: Ce fichier n'est PAS bundlé par Vite, il est servi statiquement.
 * Modifiez-le après déploiement pour pointer vers votre kernel Symbion.
 */

window.SYMBION_CONFIG = {
  /**
   * URL de base du Kernel Symbion API
   *
   * Exemples:
   * - Développement local: 'https://localhost:8443'
   * - Production même serveur: window.location.protocol + '//' + window.location.hostname + ':8443'
   * - Production serveur distant: 'https://symbion.votredomaine.com:8443'
   * - Production IP statique: 'https://192.168.1.100:8443'
   */
  // API_BASE: 'https://localhost:8443',  // Désactivé pour utiliser détection auto

  /**
   * Clé API pour authentification
   *
   * IMPORTANT: En production, cette valeur devrait être gérée par un système
   * d'authentification utilisateur (login/password → JWT token).
   *
   * Pour dev/test uniquement:
   */
  API_KEY: 's3cr3t-42',

  /**
   * Détection automatique de l'URL (ACTIVÉ)
   *
   * Ceci configure automatiquement l'API pour pointer vers le même hostname
   * que le dashboard, sur le port 8443 en HTTPS.
   */
  API_BASE: 'https://' + window.location.hostname + ':8443',
  // API_BASE: 'https://192.168.1.14:8443',  // IP statique si détection auto échoue

  /**
   * Configuration MQTT (optionnel, si WebSocket MQTT ajouté)
   */
  MQTT_BROKER: 'ws://localhost:9001',  // WebSocket MQTT broker
}

console.log('[config] Symbion configuration loaded:', window.SYMBION_CONFIG)
