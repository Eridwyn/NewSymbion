/**
 * Configuration Vite pour le PWA Dashboard Symbion
 * 
 * Setup complet :
 * - PWA avec service worker et auto-update
 * - Proxy API intégré pour développement
 * - Manifest PWA pour installation mobile
 * - Build optimisé avec sourcemaps
 */

import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    VitePWA({
      // Auto-update du service worker pour déploiement seamless
      registerType: 'autoUpdate',
      workbox: {
        // Cache tous les assets statiques pour fonctionnement offline
        globPatterns: ['**/*.{js,css,html,ico,png,svg}']
      },
      manifest: {
        name: 'Symbion Dashboard',
        short_name: 'Symbion',
        description: 'Interface de monitoring et contrôle Symbion',
        theme_color: '#1a1a1a',        // Couleur thème sombre
        background_color: '#1a1a1a',   // Couleur splash screen
        display: 'standalone',         // Mode app native
        scope: '/',
        start_url: '/',
        icons: [
          {
            src: 'icon-192.png',
            sizes: '192x192',
            type: 'image/png'
          },
          {
            src: 'icon-512.png', 
            sizes: '512x512',
            type: 'image/png'
          }
        ]
      }
    })
  ],
  server: {
    host: '0.0.0.0',  // Permet connexions externes (mobile, LAN)
    port: 3000,
    proxy: {
      // Proxy transparent vers l'API Symbion avec auth intégrée
      '/api': {
        target: 'http://localhost:8080',    // Kernel Symbion
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
        headers: {
          'x-api-key': 's3cr3t-42'         // Auth automatique en dev
        }
      }
    }
  },
  build: {
    outDir: 'dist',
    sourcemap: true    // Debug en production si nécessaire
  }
})