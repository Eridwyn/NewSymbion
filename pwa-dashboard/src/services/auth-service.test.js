import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ── Setup mocks BEFORE importing the singleton ──────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// Silence console noise during tests
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// ── Mock IndexedDB ──────────────────────────────────────────────────────
// In-memory store simulating IndexedDB for tests
let idbStore = {}

const mockObjectStore = {
  put: vi.fn((value, key) => {
    idbStore[key] = value
    return { onsuccess: null, onerror: null }
  }),
  get: vi.fn((key) => {
    const req = { result: idbStore[key] ?? null, onsuccess: null, onerror: null }
    queueMicrotask(() => req.onsuccess?.())
    return req
  }),
  clear: vi.fn(() => {
    idbStore = {}
    return { onsuccess: null, onerror: null }
  })
}

const mockTransaction = {
  objectStore: vi.fn(() => mockObjectStore),
  oncomplete: null,
  onerror: null
}

// Auto-fire oncomplete after objectStore operations
const originalPut = mockObjectStore.put
mockObjectStore.put = vi.fn((value, key) => {
  idbStore[key] = value
  queueMicrotask(() => mockTransaction.oncomplete?.())
  return { onsuccess: null, onerror: null }
})

mockObjectStore.clear = vi.fn(() => {
  idbStore = {}
  queueMicrotask(() => mockTransaction.oncomplete?.())
  return { onsuccess: null, onerror: null }
})

const mockDB = {
  transaction: vi.fn((store, mode) => {
    // Create a fresh transaction object each time
    const tx = {
      objectStore: vi.fn(() => {
        const os = {
          put: vi.fn((value, key) => {
            idbStore[key] = value
            queueMicrotask(() => tx.oncomplete?.())
            return {}
          }),
          get: vi.fn((key) => {
            const req = { result: idbStore[key] ?? null, onsuccess: null, onerror: null }
            queueMicrotask(() => req.onsuccess?.())
            return req
          }),
          clear: vi.fn(() => {
            idbStore = {}
            queueMicrotask(() => tx.oncomplete?.())
            return {}
          })
        }
        return os
      }),
      oncomplete: null,
      onerror: null
    }
    return tx
  }),
  createObjectStore: vi.fn()
}

const mockIndexedDB = {
  open: vi.fn(() => {
    const req = {
      result: mockDB,
      onupgradeneeded: null,
      onsuccess: null,
      onerror: null
    }
    queueMicrotask(() => req.onsuccess?.())
    return req
  })
}

global.indexedDB = mockIndexedDB

// ── Mock navigator.serviceWorker (optional, for _persist SW sync) ─────
const mockSWController = {
  postMessage: vi.fn()
}

Object.defineProperty(navigator, 'serviceWorker', {
  value: { controller: mockSWController },
  writable: true,
  configurable: true
})

// Clean sessionStorage/localStorage (legacy migration check)
sessionStorage.clear()
localStorage.clear()

// Import the singleton after mocks are in place
const { default: authService } = await import('./auth-service.js')

// ── Helpers ─────────────────────────────────────────────────────────────

function loginResponse(overrides = {}) {
  return {
    ok: true,
    status: 200,
    json: async () => ({
      token: 'jwt-token-abc',
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200,
      ...overrides
    })
  }
}

function resetService() {
  authService.userInfo = null
  authService.loginTime = null
  authService._token = null
  if (authService.refreshTimer) {
    clearTimeout(authService.refreshTimer)
    authService.refreshTimer = null
  }
  authService._ready = true
  sessionStorage.clear()
  localStorage.clear()
  idbStore = {}
}

// ── Lifecycle ───────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers()
  resetService()
  mockFetch.mockReset()
  mockSWController.postMessage.mockReset()
})

afterEach(() => {
  vi.useRealTimers()
})

// =========================================================================
// login()
// =========================================================================
describe('login()', () => {
  it('sends POST to /auth/login with username and password', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/login')
    expect(options.method).toBe('POST')
    const body = JSON.parse(options.body)
    expect(body.username).toBe('eridwyn')
    expect(body.password).toBe('secret')
  })

  it('includes totp_code in body when provided', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret', '123456')

    const body = JSON.parse(mockFetch.mock.calls[0][1].body)
    expect(body.totp_code).toBe('123456')
  })

  it('includes remember_device flag when true', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret', null, true)

    const body = JSON.parse(mockFetch.mock.calls[0][1].body)
    expect(body.remember_device).toBe(true)
  })

  it('does not include totp_code when null', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    const body = JSON.parse(mockFetch.mock.calls[0][1].body)
    expect(body).not.toHaveProperty('totp_code')
  })

  it('does not include remember_device when false', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret', null, false)

    const body = JSON.parse(mockFetch.mock.calls[0][1].body)
    expect(body).not.toHaveProperty('remember_device')
  })

  it('stores userInfo and token in memory on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(authService.userInfo).toEqual(expect.objectContaining({
      username: 'eridwyn',
      role: 'admin'
    }))
    expect(authService.loginTime).toBeTypeOf('number')
    expect(authService._token).toBe('jwt-token-abc')
  })

  it('persists token to IndexedDB', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(idbStore['token']).toBe('jwt-token-abc')
    expect(JSON.parse(idbStore['userInfo'])).toEqual(expect.objectContaining({
      username: 'eridwyn',
      role: 'admin'
    }))
  })

  it('sends token to SW as secondary backup', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    const storeCall = mockSWController.postMessage.mock.calls.find(
      c => c[0].type === 'AUTH_STORE'
    )
    expect(storeCall).toBeDefined()
    expect(storeCall[0].data.token).toBe('jwt-token-abc')
  })

  it('does NOT save token to sessionStorage', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
  })

  it('saves device_token to localStorage when returned', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse({ device_token: 'device-xyz-123' }))

    await authService.login('eridwyn', 'secret', null, true)

    expect(localStorage.getItem('symbion_device_token')).toBe('device-xyz-123')
  })

  it('sends existing device_token as X-Device-Token header', async () => {
    localStorage.setItem('symbion_device_token', 'existing-device-token')
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    const headers = mockFetch.mock.calls[0][1].headers
    expect(headers['X-Device-Token']).toBe('existing-device-token')
  })

  it('returns the full response data', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    const data = await authService.login('eridwyn', 'secret')

    expect(data.token).toBe('jwt-token-abc')
    expect(data.username).toBe('eridwyn')
    expect(data.role).toBe('admin')
    expect(data.expires_at).toBeTypeOf('number')
  })

  it('dispatches auth:login event on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    const handler = vi.fn()
    authService.addEventListener('auth:login', handler)

    await authService.login('eridwyn', 'secret')

    expect(handler).toHaveBeenCalledTimes(1)
    const detail = handler.mock.calls[0][0].detail
    expect(detail.username).toBe('eridwyn')
    expect(detail.role).toBe('admin')

    authService.removeEventListener('auth:login', handler)
  })

  it('dispatches window login-success event on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    const handler = vi.fn()
    window.addEventListener('login-success', handler)

    await authService.login('eridwyn', 'secret')

    expect(handler).toHaveBeenCalledTimes(1)
    const detail = handler.mock.calls[0][0].detail
    expect(detail.username).toBe('eridwyn')

    window.removeEventListener('login-success', handler)
  })

  it('schedules token refresh on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(authService.refreshTimer).not.toBeNull()
  })

  it('throws on invalid credentials (401)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: 'Invalid username or password' })
    })

    await expect(authService.login('bad', 'creds'))
      .rejects.toThrow('Invalid username or password')
  })

  it('throws on server error (500)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      json: async () => ({ error: 'Internal server error' })
    })

    await expect(authService.login('eridwyn', 'secret'))
      .rejects.toThrow('Internal server error')
  })

  it('throws default message when error response has no JSON body', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 403,
      json: async () => { throw new Error('no json') }
    })

    await expect(authService.login('eridwyn', 'secret'))
      .rejects.toThrow('Authentication failed')
  })

  it('throws on network error (fetch rejects)', async () => {
    mockFetch.mockRejectedValueOnce(new TypeError('Failed to fetch'))

    await expect(authService.login('eridwyn', 'secret'))
      .rejects.toThrow('Failed to fetch')
  })

  it('does not store userInfo on failed login', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: 'Nope' })
    })

    await authService.login('bad', 'creds').catch(() => {})

    expect(authService.userInfo).toBeNull()
  })

  it('throws on MFA required (mfa_required response)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: 'MFA required', mfa_required: true })
    })

    await expect(authService.login('eridwyn', 'secret'))
      .rejects.toThrow('MFA required')
  })
})

// =========================================================================
// logout()
// =========================================================================
describe('logout()', () => {
  function setupLoggedIn() {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: Math.floor(Date.now() / 1000) + 7200 }
    authService._token = 'jwt-token-abc'
    authService.loginTime = Date.now()
    authService.refreshTimer = setTimeout(() => {}, 60000)
  }

  it('calls POST /auth/logout', async () => {
    setupLoggedIn()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/logout')
    expect(options.method).toBe('POST')
  })

  it('clears userInfo, loginTime and _token', async () => {
    setupLoggedIn()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(authService.userInfo).toBeNull()
    expect(authService.loginTime).toBeNull()
    expect(authService._token).toBeNull()
  })

  it('sends AUTH_CLEAR to SW', async () => {
    setupLoggedIn()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    const clearCall = mockSWController.postMessage.mock.calls.find(
      c => c[0].type === 'AUTH_CLEAR'
    )
    expect(clearCall).toBeDefined()
  })

  it('clears device_token from localStorage', async () => {
    localStorage.setItem('symbion_device_token', 'some-device-token')
    setupLoggedIn()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(localStorage.getItem('symbion_device_token')).toBeNull()
  })

  it('dispatches auth:logout event', async () => {
    setupLoggedIn()
    mockFetch.mockResolvedValueOnce({ ok: true })
    const handler = vi.fn()
    authService.addEventListener('auth:logout', handler)

    await authService.logout()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.username).toBe('eridwyn')

    authService.removeEventListener('auth:logout', handler)
  })

  it('clears state even if logout API call fails', async () => {
    setupLoggedIn()
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    await authService.logout()

    expect(authService.userInfo).toBeNull()
    expect(authService.loginTime).toBeNull()
    expect(authService._token).toBeNull()
  })

  it('cancels refresh timer on logout', async () => {
    setupLoggedIn()
    expect(authService.refreshTimer).not.toBeNull()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(authService.refreshTimer).toBeNull()
  })

  it('does not call API if no userInfo present', async () => {
    await authService.logout()

    expect(mockFetch).not.toHaveBeenCalled()
  })
})

// =========================================================================
// isAuthenticated()
// =========================================================================
describe('isAuthenticated()', () => {
  it('returns false when no userInfo', () => {
    expect(authService.isAuthenticated()).toBe(false)
  })

  it('returns true when userInfo exists and not expired', () => {
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200
    }

    expect(authService.isAuthenticated()).toBe(true)
  })

  it('returns false and clears state when expired', () => {
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) - 100
    }
    authService._token = 'expired-token'

    expect(authService.isAuthenticated()).toBe(false)
    expect(authService.userInfo).toBeNull()
    expect(authService._token).toBeNull()
  })

  it('dispatches auth:expired event when expired', () => {
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) - 100
    }
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    authService.isAuthenticated()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })

  it('returns false when expires_at is exactly now (boundary)', () => {
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000)
    }

    expect(authService.isAuthenticated()).toBe(false)
  })
})

// =========================================================================
// getToken()
// =========================================================================
describe('getToken()', () => {
  it('returns null when no token stored', () => {
    expect(authService.getToken()).toBeNull()
  })

  it('returns token when authenticated', () => {
    authService._token = 'jwt-token-abc'
    expect(authService.getToken()).toBe('jwt-token-abc')
  })
})

// =========================================================================
// getTokenForWebSocket()
// =========================================================================
describe('getTokenForWebSocket()', () => {
  it('returns _token directly (no SW roundtrip)', async () => {
    authService._token = 'ws-token-123'
    const token = await authService.getTokenForWebSocket()
    expect(token).toBe('ws-token-123')
  })

  it('returns null when not authenticated', async () => {
    const token = await authService.getTokenForWebSocket()
    expect(token).toBeNull()
  })
})

// =========================================================================
// getCurrentUser()
// =========================================================================
describe('getCurrentUser()', () => {
  it('returns null when not authenticated', () => {
    expect(authService.getCurrentUser()).toBeNull()
  })

  it('returns user info when set', () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    const user = authService.getCurrentUser()
    expect(user.username).toBe('eridwyn')
    expect(user.role).toBe('admin')
  })
})

// =========================================================================
// getLoginTime()
// =========================================================================
describe('getLoginTime()', () => {
  it('returns null when not logged in', () => {
    expect(authService.getLoginTime()).toBeNull()
  })

  it('returns login timestamp when set', () => {
    authService.loginTime = 1700000000000
    expect(authService.getLoginTime()).toBe(1700000000000)
  })
})

// =========================================================================
// getAuthHeader() — deprecated
// =========================================================================
describe('getAuthHeader()', () => {
  it('always returns empty object (fetch interceptor handles auth)', () => {
    expect(authService.getAuthHeader()).toEqual({})
  })

  it('returns empty object even when authenticated', () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    expect(authService.getAuthHeader()).toEqual({})
  })
})

// =========================================================================
// verifySession()
// =========================================================================
describe('verifySession()', () => {
  it('returns false when no userInfo present', async () => {
    const result = await authService.verifySession()
    expect(result).toBe(false)
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('calls GET /auth/verify', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', valid: true })
    })

    await authService.verifySession()

    expect(mockFetch).toHaveBeenCalledTimes(1)
    expect(mockFetch.mock.calls[0][0]).toContain('/auth/verify')
  })

  it('returns true when server confirms session is valid', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', valid: true })
    })

    expect(await authService.verifySession()).toBe(true)
  })

  it('returns false and clears state when server rejects (401)', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })

    expect(await authService.verifySession()).toBe(false)
    expect(authService.userInfo).toBeNull()
  })

  it('dispatches auth:expired when server rejects', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    await authService.verifySession()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })

  it('returns false on network error', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    expect(await authService.verifySession()).toBe(false)
  })
})

// =========================================================================
// getSessionInfo()
// =========================================================================
describe('getSessionInfo()', () => {
  it('throws when not authenticated', async () => {
    await expect(authService.getSessionInfo())
      .rejects.toThrow('Not authenticated')
  })

  it('calls GET /auth/session', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', session_id: 'abc123' })
    })

    await authService.getSessionInfo()

    expect(mockFetch.mock.calls[0][0]).toContain('/auth/session')
  })

  it('returns session data on success', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    const sessionData = { username: 'eridwyn', session_id: 'abc123', created_at: 1700000000 }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => sessionData
    })

    const result = await authService.getSessionInfo()
    expect(result).toEqual(sessionData)
  })

  it('throws on server error', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })

    await expect(authService.getSessionInfo())
      .rejects.toThrow('Failed to get session info')
  })
})

// =========================================================================
// scheduleTokenRefresh()
// =========================================================================
describe('scheduleTokenRefresh()', () => {
  it('does nothing when userInfo is null', () => {
    authService.userInfo = null
    authService.scheduleTokenRefresh()
    expect(authService.refreshTimer).toBeNull()
  })

  it('clears previous timer before scheduling new one', () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: Math.floor(Date.now() / 1000) + 3600 }
    authService.scheduleTokenRefresh()
    expect(authService.refreshTimer).not.toBeNull()

    authService.scheduleTokenRefresh()
    expect(authService.refreshTimer).not.toBeNull()
  })

  it('dispatches auth:refresh-needed after calculated delay', () => {
    const expiresAt = Math.floor(Date.now() / 1000) + 3600
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: expiresAt }

    const handler = vi.fn()
    authService.addEventListener('auth:refresh-needed', handler)

    authService.scheduleTokenRefresh()

    vi.advanceTimersByTime(1799 * 1000)
    expect(handler).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1000)
    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:refresh-needed', handler)
  })

  it('does not set timer when token is already expiring soon (<30 min)', () => {
    const expiresAt = Math.floor(Date.now() / 1000) + 600
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: expiresAt }

    authService.scheduleTokenRefresh()

    expect(authService.refreshTimer).toBeNull()
  })
})

// =========================================================================
// Event integration
// =========================================================================
describe('event integration', () => {
  it('auth:expired is dispatched when checking authentication on expired token', () => {
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) - 60
    }
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    authService.isAuthenticated()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })

  it('auth:expired is dispatched when verifySession returns 401', async () => {
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    await authService.verifySession()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })
})
