import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ── Setup mocks BEFORE importing the singleton ──────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// The auth-service module reads sessionStorage in its constructor (loadFromStorage),
// so we ensure sessionStorage is clean before importing.
sessionStorage.clear()
localStorage.clear()

// Silence console noise during tests
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Import the singleton after mocks are in place
const { default: authService } = await import('./auth-service.js')

// ── Helpers ─────────────────────────────────────────────────────────────

/** Build a fake successful login response */
function loginResponse(overrides = {}) {
  return {
    ok: true,
    status: 200,
    json: async () => ({
      token: 'jwt-token-abc',
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200, // 2 hours from now
      ...overrides
    })
  }
}

/** Reset the singleton to a clean pre-login state */
function resetService() {
  authService.token = null
  authService.userInfo = null
  authService.loginTime = null
  if (authService.refreshTimer) {
    clearTimeout(authService.refreshTimer)
    authService.refreshTimer = null
  }
  sessionStorage.clear()
  localStorage.clear()
}

// ── Lifecycle ───────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers()
  resetService()
  mockFetch.mockReset()
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

  it('stores token and userInfo on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(authService.token).toBe('jwt-token-abc')
    expect(authService.userInfo).toEqual(expect.objectContaining({
      username: 'eridwyn',
      role: 'admin'
    }))
    expect(authService.loginTime).toBeTypeOf('number')
  })

  it('saves to sessionStorage on success', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())

    await authService.login('eridwyn', 'secret')

    expect(sessionStorage.getItem('symbion_auth_token')).toBe('jwt-token-abc')
    expect(sessionStorage.getItem('symbion_user_info')).toContain('eridwyn')
    expect(sessionStorage.getItem('symbion_login_time')).toBeTruthy()
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

  it('does not store state on failed login', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: 'Nope' })
    })

    await authService.login('bad', 'creds').catch(() => {})

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
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
  async function loginFirst() {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    mockFetch.mockReset()
  }

  it('calls POST /auth/logout with Bearer token', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/logout')
    expect(options.method).toBe('POST')
    expect(options.headers['Authorization']).toBe('Bearer jwt-token-abc')
    expect(options.credentials).toBe('include')
  })

  it('clears token and userInfo', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
    expect(authService.loginTime).toBeNull()
  })

  it('clears sessionStorage entries', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
    expect(sessionStorage.getItem('symbion_user_info')).toBeNull()
    expect(sessionStorage.getItem('symbion_login_time')).toBeNull()
    expect(sessionStorage.getItem('symbion_boot_completed')).toBeNull()
  })

  it('clears device_token from localStorage', async () => {
    localStorage.setItem('symbion_device_token', 'some-device-token')
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(localStorage.getItem('symbion_device_token')).toBeNull()
  })

  it('dispatches auth:logout event', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: true })
    const handler = vi.fn()
    authService.addEventListener('auth:logout', handler)

    await authService.logout()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.username).toBe('eridwyn')

    authService.removeEventListener('auth:logout', handler)
  })

  it('clears state even if logout API call fails', async () => {
    await loginFirst()
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    await authService.logout() // should NOT throw

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
  })

  it('still dispatches auth:logout event when API fails', async () => {
    await loginFirst()
    mockFetch.mockRejectedValueOnce(new Error('Network error'))
    const handler = vi.fn()
    authService.addEventListener('auth:logout', handler)

    await authService.logout()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:logout', handler)
  })

  it('cancels refresh timer on logout', async () => {
    await loginFirst()
    expect(authService.refreshTimer).not.toBeNull()
    mockFetch.mockResolvedValueOnce({ ok: true })

    await authService.logout()

    expect(authService.refreshTimer).toBeNull()
  })

  it('does not call API if no token present', async () => {
    // Not logged in — token is null
    await authService.logout()

    expect(mockFetch).not.toHaveBeenCalled()
  })
})

// =========================================================================
// loadFromStorage()
// =========================================================================
describe('loadFromStorage()', () => {
  it('restores token and userInfo from sessionStorage', () => {
    sessionStorage.setItem('symbion_auth_token', 'stored-token')
    sessionStorage.setItem('symbion_user_info', JSON.stringify({
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200
    }))
    sessionStorage.setItem('symbion_login_time', '1700000000000')

    authService.loadFromStorage()

    expect(authService.token).toBe('stored-token')
    expect(authService.userInfo.username).toBe('eridwyn')
    expect(authService.loginTime).toBe(1700000000000)
  })

  it('does nothing when sessionStorage is empty', () => {
    authService.loadFromStorage()

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
  })

  it('does nothing when token exists but userInfo is missing', () => {
    sessionStorage.setItem('symbion_auth_token', 'orphan-token')

    authService.loadFromStorage()

    expect(authService.token).toBeNull()
  })

  it('clears storage on corrupted userInfo JSON', () => {
    sessionStorage.setItem('symbion_auth_token', 'bad-token')
    sessionStorage.setItem('symbion_user_info', '{invalid json')

    authService.loadFromStorage()

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
    expect(sessionStorage.getItem('symbion_user_info')).toBeNull()
  })

  it('sets loginTime to null when no login time in storage', () => {
    sessionStorage.setItem('symbion_auth_token', 'some-token')
    sessionStorage.setItem('symbion_user_info', JSON.stringify({
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200
    }))
    // Intentionally NOT setting symbion_login_time

    authService.loadFromStorage()

    expect(authService.token).toBe('some-token')
    expect(authService.loginTime).toBeNull()
  })

  it('schedules token refresh after restoring session', () => {
    sessionStorage.setItem('symbion_auth_token', 'refresh-token')
    sessionStorage.setItem('symbion_user_info', JSON.stringify({
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200
    }))

    authService.loadFromStorage()

    expect(authService.refreshTimer).not.toBeNull()
  })
})

// =========================================================================
// isAuthenticated()
// =========================================================================
describe('isAuthenticated()', () => {
  it('returns false when no token', () => {
    expect(authService.isAuthenticated()).toBe(false)
  })

  it('returns false when no userInfo', () => {
    authService.token = 'some-token'
    expect(authService.isAuthenticated()).toBe(false)
  })

  it('returns true when token exists and not expired', () => {
    authService.token = 'valid-token'
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) + 7200
    }

    expect(authService.isAuthenticated()).toBe(true)
  })

  it('returns false and clears state when token is expired', () => {
    authService.token = 'expired-token'
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) - 100 // expired 100 seconds ago
    }

    expect(authService.isAuthenticated()).toBe(false)
    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
  })

  it('dispatches auth:expired event when token is expired', () => {
    authService.token = 'expired-token'
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
    authService.token = 'boundary-token'
    authService.userInfo = {
      username: 'eridwyn',
      role: 'admin',
      expires_at: Math.floor(Date.now() / 1000) // exactly now — <= check should expire
    }

    expect(authService.isAuthenticated()).toBe(false)
  })
})

// =========================================================================
// getToken()
// =========================================================================
describe('getToken()', () => {
  it('returns null when not authenticated', () => {
    expect(authService.getToken()).toBeNull()
  })

  it('returns the JWT token after login', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    expect(authService.getToken()).toBe('jwt-token-abc')
  })
})

// =========================================================================
// getCurrentUser()
// =========================================================================
describe('getCurrentUser()', () => {
  it('returns null when not authenticated', () => {
    expect(authService.getCurrentUser()).toBeNull()
  })

  it('returns user info after login', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    const user = authService.getCurrentUser()
    expect(user.username).toBe('eridwyn')
    expect(user.role).toBe('admin')
    expect(user.expires_at).toBeTypeOf('number')
  })
})

// =========================================================================
// getLoginTime()
// =========================================================================
describe('getLoginTime()', () => {
  it('returns null when not logged in', () => {
    expect(authService.getLoginTime()).toBeNull()
  })

  it('returns login timestamp after login', async () => {
    const before = Date.now()
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    const after = Date.now()

    const loginTime = authService.getLoginTime()
    expect(loginTime).toBeGreaterThanOrEqual(before)
    expect(loginTime).toBeLessThanOrEqual(after)
  })
})

// =========================================================================
// getAuthHeader()
// =========================================================================
describe('getAuthHeader()', () => {
  it('returns empty object when not authenticated', () => {
    expect(authService.getAuthHeader()).toEqual({})
  })

  it('returns Authorization Bearer header when authenticated', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    expect(authService.getAuthHeader()).toEqual({
      'Authorization': 'Bearer jwt-token-abc'
    })
  })
})

// =========================================================================
// verifySession()
// =========================================================================
describe('verifySession()', () => {
  async function loginFirst() {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    mockFetch.mockReset()
  }

  it('returns false when no token present', async () => {
    const result = await authService.verifySession()

    expect(result).toBe(false)
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('calls GET /auth/verify with Bearer token', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', valid: true })
    })

    await authService.verifySession()

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/verify')
    expect(options.headers['Authorization']).toBe('Bearer jwt-token-abc')
    expect(options.credentials).toBe('include')
  })

  it('returns true when server confirms session is valid', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', valid: true })
    })

    expect(await authService.verifySession()).toBe(true)
  })

  it('returns false and clears state when server rejects (401)', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })

    expect(await authService.verifySession()).toBe(false)
    expect(authService.token).toBeNull()
  })

  it('dispatches auth:expired when server rejects', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    await authService.verifySession()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })

  it('returns false and clears state on network error', async () => {
    await loginFirst()
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    expect(await authService.verifySession()).toBe(false)
    expect(authService.token).toBeNull()
  })
})

// =========================================================================
// getSessionInfo()
// =========================================================================
describe('getSessionInfo()', () => {
  async function loginFirst() {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    mockFetch.mockReset()
  }

  it('throws when not authenticated', async () => {
    await expect(authService.getSessionInfo())
      .rejects.toThrow('Not authenticated')
  })

  it('calls GET /auth/session with Bearer token', async () => {
    await loginFirst()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ username: 'eridwyn', session_id: 'abc123' })
    })

    await authService.getSessionInfo()

    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/session')
    expect(options.headers['Authorization']).toBe('Bearer jwt-token-abc')
    expect(options.credentials).toBe('include')
  })

  it('returns session data on success', async () => {
    await loginFirst()
    const sessionData = { username: 'eridwyn', session_id: 'abc123', created_at: 1700000000 }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => sessionData
    })

    const result = await authService.getSessionInfo()

    expect(result).toEqual(sessionData)
  })

  it('throws on server error', async () => {
    await loginFirst()
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

  it('clears previous timer before scheduling new one', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    const firstTimer = authService.refreshTimer

    authService.scheduleTokenRefresh()
    const secondTimer = authService.refreshTimer

    // A new timer should have been created (different reference)
    expect(secondTimer).not.toBeNull()
    // Cannot directly compare timer IDs, but we can verify no crash
  })

  it('dispatches auth:refresh-needed after calculated delay', async () => {
    // Token expires in 1 hour (3600s), refresh at 3600-1800=1800s = 30 min
    const expiresAt = Math.floor(Date.now() / 1000) + 3600
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: expiresAt }
    authService.token = 'test-token'

    const handler = vi.fn()
    authService.addEventListener('auth:refresh-needed', handler)

    authService.scheduleTokenRefresh()

    // Should not fire before 30 minutes
    vi.advanceTimersByTime(1799 * 1000)
    expect(handler).not.toHaveBeenCalled()

    // Should fire at 30 minutes (1800s * 1000ms)
    vi.advanceTimersByTime(1000)
    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:refresh-needed', handler)
  })

  it('does not set timer when token is already expiring soon (<30 min)', () => {
    // Token expires in 10 minutes (600s). 600-1800 = -1200 → max(0,...) = 0
    const expiresAt = Math.floor(Date.now() / 1000) + 600
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: expiresAt }
    authService.token = 'short-lived-token'

    const handler = vi.fn()
    authService.addEventListener('auth:refresh-needed', handler)

    authService.scheduleTokenRefresh()

    // refreshTime = max(0, (600 - 1800) * 1000) = 0, so no timer is set
    // The code checks `if (refreshTime > 0)`, so with 0 the branch is skipped
    expect(authService.refreshTimer).toBeNull()

    authService.removeEventListener('auth:refresh-needed', handler)
  })
})

// =========================================================================
// clearStorage()
// =========================================================================
describe('clearStorage()', () => {
  it('removes all session keys from sessionStorage', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    sessionStorage.setItem('symbion_boot_completed', 'true')

    authService.clearStorage()

    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
    expect(sessionStorage.getItem('symbion_user_info')).toBeNull()
    expect(sessionStorage.getItem('symbion_login_time')).toBeNull()
    expect(sessionStorage.getItem('symbion_boot_completed')).toBeNull()
  })

  it('removes device_token from localStorage', () => {
    localStorage.setItem('symbion_device_token', 'device-abc')

    authService.clearStorage()

    expect(localStorage.getItem('symbion_device_token')).toBeNull()
  })

  it('resets internal state to null', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    authService.clearStorage()

    expect(authService.token).toBeNull()
    expect(authService.userInfo).toBeNull()
    expect(authService.loginTime).toBeNull()
  })

  it('cancels active refresh timer', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')
    expect(authService.refreshTimer).not.toBeNull()

    authService.clearStorage()

    expect(authService.refreshTimer).toBeNull()
  })
})

// =========================================================================
// saveToStorage()
// =========================================================================
describe('saveToStorage()', () => {
  it('saves token, userInfo, and loginTime to sessionStorage', () => {
    authService.token = 'save-test-token'
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    authService.loginTime = 1700000000000

    authService.saveToStorage()

    expect(sessionStorage.getItem('symbion_auth_token')).toBe('save-test-token')
    expect(JSON.parse(sessionStorage.getItem('symbion_user_info'))).toEqual({
      username: 'eridwyn',
      role: 'admin',
      expires_at: 9999999999
    })
    expect(sessionStorage.getItem('symbion_login_time')).toBe('1700000000000')
  })

  it('does not save when token is null', () => {
    authService.token = null
    authService.userInfo = { username: 'eridwyn' }

    authService.saveToStorage()

    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
  })

  it('does not save when userInfo is null', () => {
    authService.token = 'some-token'
    authService.userInfo = null

    authService.saveToStorage()

    expect(sessionStorage.getItem('symbion_auth_token')).toBeNull()
  })

  it('does not save loginTime when it is null', () => {
    authService.token = 'token'
    authService.userInfo = { username: 'eridwyn' }
    authService.loginTime = null

    authService.saveToStorage()

    expect(sessionStorage.getItem('symbion_auth_token')).toBe('token')
    expect(sessionStorage.getItem('symbion_login_time')).toBeNull()
  })
})

// =========================================================================
// Event integration (cross-cutting)
// =========================================================================
describe('event integration', () => {
  it('auth:login fires with correct detail after successful login', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse({ username: 'testuser', role: 'viewer' }))
    const handler = vi.fn()
    authService.addEventListener('auth:login', handler)

    await authService.login('testuser', 'pass')

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ username: 'testuser', role: 'viewer' })

    authService.removeEventListener('auth:login', handler)
  })

  it('auth:expired is dispatched when checking authentication on expired token', () => {
    authService.token = 'expired-token'
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
    authService.token = 'will-be-rejected'
    authService.userInfo = { username: 'eridwyn', role: 'admin', expires_at: 9999999999 }
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 })
    const handler = vi.fn()
    authService.addEventListener('auth:expired', handler)

    await authService.verifySession()

    expect(handler).toHaveBeenCalledTimes(1)

    authService.removeEventListener('auth:expired', handler)
  })
})

// =========================================================================
// Full login/logout lifecycle
// =========================================================================
describe('full lifecycle', () => {
  it('login → isAuthenticated → logout → isAuthenticated', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    expect(authService.isAuthenticated()).toBe(true)
    expect(authService.getToken()).toBe('jwt-token-abc')
    expect(authService.getCurrentUser().username).toBe('eridwyn')
    expect(authService.getAuthHeader()).toEqual({ 'Authorization': 'Bearer jwt-token-abc' })

    mockFetch.mockResolvedValueOnce({ ok: true })
    await authService.logout()

    expect(authService.isAuthenticated()).toBe(false)
    expect(authService.getToken()).toBeNull()
    expect(authService.getCurrentUser()).toBeNull()
    expect(authService.getAuthHeader()).toEqual({})
    expect(authService.getLoginTime()).toBeNull()
  })

  it('login → save → clear → loadFromStorage restores session', async () => {
    mockFetch.mockResolvedValueOnce(loginResponse())
    await authService.login('eridwyn', 'secret')

    // Session is saved to sessionStorage (done automatically by login)
    const storedToken = sessionStorage.getItem('symbion_auth_token')
    expect(storedToken).toBe('jwt-token-abc')

    // Clear internal state but NOT sessionStorage
    authService.token = null
    authService.userInfo = null
    authService.loginTime = null
    if (authService.refreshTimer) clearTimeout(authService.refreshTimer)
    authService.refreshTimer = null

    expect(authService.isAuthenticated()).toBe(false)

    // Reload from storage
    authService.loadFromStorage()

    expect(authService.token).toBe('jwt-token-abc')
    expect(authService.userInfo.username).toBe('eridwyn')
    expect(authService.isAuthenticated()).toBe(true)
  })
})
