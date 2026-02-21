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

  return {
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
    https: {
      // Utiliser les certificats mkcert de confiance
      key: fs.readFileSync(path.resolve(__dirname, '../symbion-kernel/certs/key-mkcert.pem')),
      cert: fs.readFileSync(path.resolve(__dirname, '../symbion-kernel/certs/cert-mkcert.pem')),
    },
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
        headers: {
          'x-api-key': env.VITE_SYMBION_API_KEY || 's3cr3t-42'  // Load from .env
        }
      },
      // Proxy pour endpoints v1 (Environment IoT, Metrics, etc.)
      '/v1': {
        target: 'https://localhost:8443',    // Kernel Symbion HTTPS
        changeOrigin: true,
        secure: false,  // Accept self-signed certificates
        headers: {
          'x-api-key': env.VITE_SYMBION_API_KEY || 's3cr3t-42'  // Load from .env
        }
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