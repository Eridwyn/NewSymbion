/**
 * Configuration Vite pour le PWA Dashboard Symbion
 *
 * Setup complet :
 * - PWA avec service worker et auto-update
 * - Proxy API intégré pour développement
 * - Manifest PWA pour installation mobile
 * - Build optimisé avec sourcemaps
 * - HTTPS avec certificats auto-signés (sécurité complète)
 */

import { defineConfig, loadEnv } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'
import { resolve } from 'path'
import fs from 'fs'
import path from 'path'

export default defineConfig(({ mode }) => {
  // Load env vars
  const env = loadEnv(mode, process.cwd(), '')

  // TLS certificates for dev server (optional — not needed for production build)
  const certKeyPath = path.resolve(__dirname, '../symbion-kernel/certs/key-mkcert.pem')
  const certPath = path.resolve(__dirname, '../symbion-kernel/certs/cert-mkcert.pem')
  const certsExist = fs.existsSync(certKeyPath) && fs.existsSync(certPath)

  return {
  plugins: [
    VitePWA({
      // Auto-update du service worker pour déploiement seamless
      registerType: 'autoUpdate',
      workbox: {
        // Cache tous les assets statiques pour fonctionnement offline
        globPatterns: ['**/*.{js,css,html,ico,png,svg}'],
        runtimeCaching: [
          {
            urlPattern: /\/v1\/.*$/,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'api-cache',
              expiration: {
                maxEntries: 50,
                maxAgeSeconds: 60 * 60 // 1 hour
              },
              networkTimeoutSeconds: 5,
              cacheableResponse: {
                statuses: [0, 200]
              }
            }
          },
          {
            urlPattern: /\/health$/,
            handler: 'NetworkOnly',
            options: {
              cacheName: 'health-check'
            }
          }
        ]
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
    https: certsExist ? {
      key: fs.readFileSync(certKeyPath),
      cert: fs.readFileSync(certPath),
    } : undefined,
    host: '0.0.0.0',  // Permet connexions externes (mobile, LAN)
    port: 3000,
    hmr: {
      clientPort: 3000,
      host: 'symbion.local', // HMR WebSocket via symbion.local au lieu de localhost
    },
    proxy: {
      // Proxy transparent vers l'API Symbion avec auth intégrée
      '/api': {
        target: 'https://localhost:8443',    // Kernel Symbion HTTPS
        changeOrigin: true,
        secure: false,  // Accept self-signed certificates
        rewrite: (path) => path.replace(/^\/api/, ''),
        ...(env.VITE_SYMBION_API_KEY ? {
          headers: { 'x-api-key': env.VITE_SYMBION_API_KEY }
        } : {})
      },
      // Proxy pour endpoints v1 (Environment IoT, Metrics, etc.)
      '/v1': {
        target: 'https://localhost:8443',    // Kernel Symbion HTTPS
        changeOrigin: true,
        secure: false,  // Accept self-signed certificates
        ...(env.VITE_SYMBION_API_KEY ? {
          headers: { 'x-api-key': env.VITE_SYMBION_API_KEY }
        } : {})
      }
    }
  },
  test: {
    environment: 'happy-dom',
    include: ['src/**/*.test.js'],
  },
  build: {
    outDir: 'dist',
    sourcemap: true,    // Debug en production si nécessaire
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        logs: resolve(__dirname, 'logs.html')
      }
    }
  }
}})