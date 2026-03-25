/**
 * Service Worker Symbion — Workbox + Auth Token Vault
 *
 * Le token JWT est stocké exclusivement dans le SW (IndexedDB).
 * Le main thread n'y a jamais accès directement.
 * Le SW injecte le header Authorization sur chaque requête API.
 */

import { precacheAndRoute, cleanupOutdatedCaches, createHandlerBoundToURL } from 'workbox-precaching'
import { registerRoute, NavigationRoute } from 'workbox-routing'
import { NetworkOnly, StaleWhileRevalidate } from 'workbox-strategies'
import { ExpirationPlugin } from 'workbox-expiration'
import { clientsClaim } from 'workbox-core'

// ══════════════════════════════════════
// ── Auth Token Vault (IndexedDB) ──
// ══════════════════════════════════════

const DB_NAME = 'symbion-vault'
const DB_VER = 1
const STORE = 'auth'

let _token = null
let _userInfo = null
let _tokenExp = null // JWT expiration timestamp (seconds)

function openVault() {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(DB_NAME, DB_VER)
    r.onupgradeneeded = () => r.result.createObjectStore(STORE)
    r.onsuccess = () => resolve(r.result)
    r.onerror = () => reject(r.error)
  })
}

async function vaultPut(key, value) {
  const db = await openVault()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, 'readwrite')
    tx.objectStore(STORE).put(value, key)
    tx.oncomplete = resolve
    tx.onerror = reject
  })
}

async function vaultGet(key) {
  const db = await openVault()
  return new Promise((resolve) => {
    const tx = db.transaction(STORE, 'readonly')
    const r = tx.objectStore(STORE).get(key)
    r.onsuccess = () => resolve(r.result ?? null)
    r.onerror = () => resolve(null)
  })
}

async function vaultClear() {
  const db = await openVault()
  return new Promise((resolve) => {
    const tx = db.transaction(STORE, 'readwrite')
    tx.objectStore(STORE).clear()
    tx.oncomplete = resolve
    tx.onerror = resolve
  })
}

function parseJwtExp(token) {
  try {
    const payload = JSON.parse(atob(token.split('.')[1]))
    return payload.exp || null
  } catch { return null }
}

async function restoreToken() {
  try {
    _token = await vaultGet('token')
    const raw = await vaultGet('userInfo')
    _userInfo = raw ? JSON.parse(raw) : null
    _tokenExp = _token ? parseJwtExp(_token) : null

    // Clear if already expired
    if (_tokenExp && _tokenExp <= Math.floor(Date.now() / 1000)) {
      console.log('[sw-auth] Restored token already expired, clearing')
      _token = null
      _userInfo = null
      _tokenExp = null
      await vaultClear()
    }
  } catch (e) {
    console.error('[sw-auth] Failed to restore token:', e)
  }
}

async function clearAllAuthState() {
  _token = null
  _userInfo = null
  _tokenExp = null
  await vaultClear()
  // Purge SWR cache so stale 200 responses don't mask 401s
  try {
    const deleted = await caches.delete('api-swr-cache')
    if (deleted) console.log('[sw-auth] Purged api-swr-cache')
  } catch (e) {
    console.warn('[sw-auth] Failed to purge SWR cache:', e)
  }
}

// Restore on SW startup
restoreToken()

// ══════════════════════════════════════
// ── Fetch Interception (Auth Header) ──
// ══════════════════════════════════════
// Registered BEFORE Workbox routes so it takes priority for API requests

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url)

  // Only same-origin API/auth requests
  if (url.origin !== self.location.origin) return

  // Skip static assets — Workbox handles those
  const isStatic = /\.(js|css|html|png|jpg|svg|ico|woff2?|ttf|map|json)$/.test(url.pathname) ||
    url.pathname === '/'

  if (isStatic) return

  // Don't inject auth on login (no token yet)
  if (url.pathname === '/auth/login') return

  if (!_token) return // No token → pass through unauthenticated

  // Check JWT expiration before injecting
  if (_tokenExp && _tokenExp <= Math.floor(Date.now() / 1000)) {
    console.log('[sw-auth] Token expired (exp check), clearing')
    clearAllAuthState()
    return // Let request pass through without auth
  }

  // Already has Authorization → don't override
  if (event.request.headers.get('Authorization')) return

  // Clone request with Authorization header
  const headers = new Headers(event.request.headers)
  headers.set('Authorization', `Bearer ${_token}`)

  const authedRequest = new Request(event.request, { headers })
  event.respondWith(
    fetch(authedRequest).then(response => {
      // If server returns 401, clear auth state + SWR cache
      if (response.status === 401) {
        console.log('[sw-auth] Server returned 401, clearing all auth state')
        clearAllAuthState()
      }
      return response
    })
  )
})

// ══════════════════════════════════════
// ── Workbox Setup ──
// ══════════════════════════════════════

self.skipWaiting()
clientsClaim()

precacheAndRoute(self.__WB_MANIFEST)
cleanupOutdatedCaches()

// Navigation fallback → index.html (SPA)
registerRoute(
  new NavigationRoute(createHandlerBoundToURL('index.html'), {
    denylist: [/\/v1\//, /\/health$/, /\/auth\//]
  })
)

// Stale-while-revalidate for read-only API data
// Shows cached data immediately, updates cache in background
registerRoute(
  ({ url, request }) => {
    if (request.method !== 'GET') return false
    const swrPaths = [
      '/v1/context/current',
      '/v1/agents',
      '/v1/plugins',
      '/v1/environment/sensors',
      '/v1/modes',
      '/health',
      '/agents',
      '/plugins',
      '/v1/plugin-api/coffee/status',
      '/v1/plugin-api/library/health',
      '/v1/notifications/active'
    ]
    return swrPaths.some(p => url.pathname === p || url.pathname.startsWith(p))
  },
  new StaleWhileRevalidate({
    cacheName: 'api-swr-cache',
    plugins: [
      new ExpirationPlugin({
        maxEntries: 50,
        maxAgeSeconds: 5 * 60 // 5 minutes max cache age
      })
    ]
  })
)

// ══════════════════════════════════════
// ── Message Handler ──
// ══════════════════════════════════════

self.addEventListener('message', async (event) => {
  const { type, data } = event.data || {}

  switch (type) {
    case 'AUTH_STORE': {
      _token = data.token
      _userInfo = data.userInfo
      _tokenExp = data.token ? parseJwtExp(data.token) : null
      await vaultPut('token', data.token)
      await vaultPut('userInfo', JSON.stringify(data.userInfo))
      event.source?.postMessage({ type: 'AUTH_ACK' })
      break
    }

    case 'AUTH_CLEAR': {
      await clearAllAuthState()
      event.source?.postMessage({ type: 'AUTH_ACK' })
      break
    }

    case 'AUTH_CHECK': {
      if (!_token) await restoreToken()
      event.source?.postMessage({
        type: 'AUTH_STATUS',
        authenticated: !!_token,
        userInfo: _userInfo
      })
      break
    }

    case 'AUTH_GET_WS_TOKEN': {
      // WebSocket ne passe pas par fetch → fournir le token sur demande
      if (!_token) await restoreToken()
      event.source?.postMessage({
        type: 'AUTH_WS_TOKEN',
        token: _token
      })
      break
    }

    case 'SKIP_WAITING':
      self.skipWaiting()
      break
  }
})
