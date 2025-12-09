/**
 * Configuration Runtime Dashboard Symbion
 *
 * Ce fichier est chargé AVANT l'application et permet de configurer
 * l'URL de l'API Symbion selon l'environnement de déploiement.
 *
 * IMPORTANT: Ce fichier n'est PAS bundlé par Vite, il est servi statiquement.
 * Modifiez-le après déploiement pour pointer vers votre kernel Symbion.
 */

// Fonction pour détecter l'environnement et retourner la config appropriée
(function() {
  const port = window.location.port;
  const hostname = window.location.hostname;
  const protocol = window.location.protocol;

  // Détection: via Nginx (port 443, 80, ou vide) ou dev direct (port 3000)
  const viaProxy = (port === '' || port === '443' || port === '80');

  window.SYMBION_CONFIG = {
    /**
     * Clé API pour authentification
     */
    API_KEY: 's3cr3t-42',

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

  console.log('[config] Detected environment:', {
    port: port || '(default)',
    viaProxy: viaProxy,
    API_BASE: window.SYMBION_CONFIG.API_BASE,
    MQTT_BROKER: window.SYMBION_CONFIG.MQTT_BROKER
  });
})();

console.log('[config] Symbion configuration loaded:', window.SYMBION_CONFIG)
