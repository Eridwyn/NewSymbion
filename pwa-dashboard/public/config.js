/**
 * Configuration Runtime Dashboard Symbion
 *
 * Ce fichier est chargé AVANT l'application et permet de configurer
 * l'URL de l'API Symbion selon l'environnement de déploiement.
 *
 * [SECURITY] P0-3: Configuration explicite requise
 * - En production: SYMBION_DEV_MODE doit être false
 * - L'API_KEY ne doit JAMAIS être hardcodée ici
 * - Utiliser l'authentification JWT en production
 *
 * IMPORTANT: Ce fichier n'est PAS bundlé par Vite, il est servi statiquement.
 * Modifiez-le après déploiement pour pointer vers votre kernel Symbion.
 */

// Fonction pour détecter l'environnement et retourner la config appropriée
(function() {
  const port = window.location.port;
  const hostname = window.location.hostname;
  const protocol = window.location.protocol;

  // [SECURITY] P0-3: Explicit dev mode detection
  // Dev mode is ONLY enabled on localhost/127.0.0.1 with port 3000
  const isLocalDev = (hostname === 'localhost' || hostname === '127.0.0.1') && port === '3000';

  // Détection: via Nginx (port 443, 80, ou vide) ou dev direct (port 3000)
  const viaProxy = (port === '' || port === '443' || port === '80');

  window.SYMBION_CONFIG = {
    /**
     * [SECURITY] P0-3: Explicit dev mode flag
     * When true: Allows self-signed certs, relaxed security
     * When false: Production mode, strict security required
     */
    DEV_MODE: isLocalDev,

    /**
     * URL de base du Kernel Symbion API
     * - Via Nginx (proxy): https://hostname (appels vers /api/*)
     * - Dev direct (3000): https://hostname:8443
     */
    API_BASE: viaProxy
      ? window.location.origin
      : 'https://' + hostname + ':8443',

    /**
     * MQTT WebSocket Broker
     * - Via Nginx (production): wss://hostname/ws/mqtt
     * - Dev direct (port 3000): ws://localhost:9001
     */
    MQTT_BROKER: viaProxy
      ? (protocol === 'https:' ? 'wss://' : 'ws://') + hostname + '/ws/mqtt'
      : 'ws://localhost:9001'
  };

  // [SECURITY] P0-3: Log security warnings
  if (window.SYMBION_CONFIG.DEV_MODE) {
    console.warn('[SECURITY] Running in DEV_MODE - NOT suitable for production!');
    console.warn('[SECURITY] Self-signed certificates accepted, relaxed security.');
  }

  console.log('[config] Detected environment:', {
    port: port || '(default)',
    viaProxy: viaProxy,
    DEV_MODE: window.SYMBION_CONFIG.DEV_MODE,
    API_BASE: window.SYMBION_CONFIG.API_BASE,
    MQTT_BROKER: window.SYMBION_CONFIG.MQTT_BROKER
  });
})();

console.log('[config] Symbion configuration loaded:', window.SYMBION_CONFIG)
