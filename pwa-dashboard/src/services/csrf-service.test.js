import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import csrfService from './csrf-service.js'

const mockFetch = vi.fn()
global.fetch = mockFetch

const mockAuthService = {
  isAuthenticated: vi.fn(() => true),
  getToken: vi.fn(() => 'test-jwt-token')
}

beforeEach(() => {
  csrfService.currentNonce = null
  csrfService.expiresAt = null
  csrfService.authService = mockAuthService
  if (csrfService.refreshTimer) clearTimeout(csrfService.refreshTimer)
  csrfService.refreshTimer = null
  mockFetch.mockReset()
  mockAuthService.isAuthenticated.mockReturnValue(true)
  mockAuthService.getToken.mockReturnValue('test-jwt-token')
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('getNonce', () => {
  it('returns cached nonce if still valid', async () => {
    csrfService.currentNonce = 'cached-nonce-123'
    csrfService.expiresAt = Date.now() + 60000 // 60s in future

    const nonce = await csrfService.getNonce()

    expect(nonce).toBe('cached-nonce-123')
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('fetches new nonce if none exists', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'new-nonce-456', expires_in_seconds: 300 })
    })

    const nonce = await csrfService.getNonce()

    expect(nonce).toBe('new-nonce-456')
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('fetches new nonce if expired', async () => {
    csrfService.currentNonce = 'expired-nonce'
    csrfService.expiresAt = Date.now() - 1000 // 1s in past

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'fresh-nonce-789', expires_in_seconds: 300 })
    })

    const nonce = await csrfService.getNonce()

    expect(nonce).toBe('fresh-nonce-789')
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('returns null if not authenticated', async () => {
    mockAuthService.isAuthenticated.mockReturnValue(false)

    const nonce = await csrfService.getNonce()

    expect(nonce).toBeNull()
    expect(mockFetch).not.toHaveBeenCalled()
  })
})

describe('fetchNewNonce', () => {
  it('returns null if authService not set', async () => {
    csrfService.authService = null

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBeNull()
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('returns null if not authenticated', async () => {
    mockAuthService.isAuthenticated.mockReturnValue(false)

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBeNull()
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('fetches nonce even without explicit token (SW handles auth)', async () => {
    mockAuthService.getToken.mockReturnValue(null)

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'sw-auth-nonce', expires_in_seconds: 300 })
    })

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBe('sw-auth-nonce')
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('fetches from /auth/csrf/nonce with correct headers (no Authorization — SW injects it)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'test-nonce', expires_in_seconds: 300 })
    })

    await csrfService.fetchNewNonce()

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/auth/csrf/nonce')
    expect(options.method).toBe('GET')
    expect(options.headers['Authorization']).toBeUndefined() // SW handles this
    expect(options.headers['Content-Type']).toBe('application/json')
    expect(options.signal).toBeInstanceOf(AbortSignal)
  })

  it('stores nonce and expiration on success', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'stored-nonce', expires_in_seconds: 300 })
    })

    const now = Date.now()
    await csrfService.fetchNewNonce()

    expect(csrfService.currentNonce).toBe('stored-nonce')
    expect(csrfService.expiresAt).toBeGreaterThanOrEqual(now + 300000)
    expect(csrfService.expiresAt).toBeLessThanOrEqual(now + 300000 + 100)
  })

  it('dispatches csrf:fetched event on success', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'event-nonce', expires_in_seconds: 300 })
    })

    const handler = vi.fn()
    csrfService.addEventListener('csrf:fetched', handler)

    await csrfService.fetchNewNonce()

    expect(handler).toHaveBeenCalledTimes(1)
    const event = handler.mock.calls[0][0]
    expect(event.detail.nonce).toBe('event-nonce')
    expect(event.detail.expiresAt).toBeDefined()

    csrfService.removeEventListener('csrf:fetched', handler)
  })

  it('schedules refresh on success', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ nonce: 'refresh-nonce', expires_in_seconds: 300 })
    })

    await csrfService.fetchNewNonce()

    expect(csrfService.refreshTimer).not.toBeNull()
  })

  it('returns null on HTTP error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      text: async () => 'Unauthorized'
    })

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBeNull()
  })

  it('dispatches csrf:error on HTTP error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 403,
      text: async () => 'Forbidden'
    })

    const handler = vi.fn()
    csrfService.addEventListener('csrf:error', handler)

    await csrfService.fetchNewNonce()

    expect(handler).toHaveBeenCalledTimes(1)
    const event = handler.mock.calls[0][0]
    expect(event.detail.status).toBe(403)
    expect(event.detail.error).toBe('Forbidden')

    csrfService.removeEventListener('csrf:error', handler)
  })

  it('returns null on invalid response (missing nonce)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ token: 'wrong-field' })
    })

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBeNull()
    expect(csrfService.currentNonce).toBeNull()
  })

  it('returns null on network error (fetch throws)', async () => {
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    const nonce = await csrfService.fetchNewNonce()

    expect(nonce).toBeNull()
  })

  it('handles abort timeout', async () => {
    // Simulate a fetch that never resolves until aborted
    mockFetch.mockImplementationOnce((_url, options) => {
      return new Promise((_resolve, reject) => {
        options.signal.addEventListener('abort', () => {
          reject(new DOMException('The operation was aborted.', 'AbortError'))
        })
      })
    })

    const noncePromise = csrfService.fetchNewNonce()

    // Advance time past the 10s abort timeout
    vi.advanceTimersByTime(10000)

    const nonce = await noncePromise

    expect(nonce).toBeNull()
  })
})

describe('invalidateNonce', () => {
  it('clears nonce and expiration', () => {
    csrfService.currentNonce = 'to-clear'
    csrfService.expiresAt = Date.now() + 60000

    csrfService.invalidateNonce()

    expect(csrfService.currentNonce).toBeNull()
    expect(csrfService.expiresAt).toBeNull()
  })

  it('clears refresh timer', () => {
    csrfService.refreshTimer = setTimeout(() => {}, 60000)
    expect(csrfService.refreshTimer).not.toBeNull()

    csrfService.invalidateNonce()

    expect(csrfService.refreshTimer).toBeNull()
  })

  it('dispatches csrf:expired event', () => {
    const handler = vi.fn()
    csrfService.addEventListener('csrf:expired', handler)

    csrfService.invalidateNonce()

    expect(handler).toHaveBeenCalledTimes(1)

    csrfService.removeEventListener('csrf:expired', handler)
  })
})

describe('fetchWithCsrf', () => {
  it('throws if no nonce available', async () => {
    // authService not authenticated -> getNonce returns null
    mockAuthService.isAuthenticated.mockReturnValue(false)

    await expect(csrfService.fetchWithCsrf('/v1/test'))
      .rejects.toThrow('Failed to obtain CSRF nonce')
  })

  it('adds X-CSRF-Token header', async () => {
    // Pre-set a valid nonce so getNonce returns it without fetching
    csrfService.currentNonce = 'csrf-token-abc'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 })

    await csrfService.fetchWithCsrf('/v1/test', { method: 'POST' })

    const [, options] = mockFetch.mock.calls[0]
    expect(options.headers['X-CSRF-Token']).toBe('csrf-token-abc')
  })

  it('does not add Authorization header (SW injects it)', async () => {
    csrfService.currentNonce = 'csrf-token-xyz'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 })

    await csrfService.fetchWithCsrf('/v1/test', { method: 'POST' })

    const [, options] = mockFetch.mock.calls[0]
    expect(options.headers['Authorization']).toBeUndefined()
  })

  it('builds full URL from relative path', async () => {
    csrfService.currentNonce = 'csrf-for-url'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 })

    await csrfService.fetchWithCsrf('/v1/modes', { method: 'GET' })

    const [url] = mockFetch.mock.calls[0]
    expect(url).toMatch(/^https?:\/\//)
    expect(url).toContain('/v1/modes')
  })

  it('adds Content-Type for JSON body', async () => {
    csrfService.currentNonce = 'csrf-json'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 })

    await csrfService.fetchWithCsrf('/v1/test', {
      method: 'POST',
      body: JSON.stringify({ key: 'value' })
    })

    const [, options] = mockFetch.mock.calls[0]
    expect(options.headers['Content-Type']).toBe('application/json')
  })

  it('invalidates nonce after each request (single-use)', async () => {
    csrfService.currentNonce = 'single-use-nonce'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 })

    await csrfService.fetchWithCsrf('/v1/test', { method: 'POST' })

    expect(csrfService.currentNonce).toBeNull()
    expect(csrfService.expiresAt).toBeNull()
  })

  it('propagates fetch errors', async () => {
    csrfService.currentNonce = 'csrf-error'
    csrfService.expiresAt = Date.now() + 60000

    mockFetch.mockRejectedValueOnce(new Error('Connection refused'))

    await expect(csrfService.fetchWithCsrf('/v1/test'))
      .rejects.toThrow('Connection refused')
  })
})

describe('cleanup', () => {
  it('clears all state', () => {
    csrfService.currentNonce = 'cleanup-nonce'
    csrfService.expiresAt = Date.now() + 60000
    csrfService.refreshTimer = setTimeout(() => {}, 60000)

    csrfService.cleanup()

    expect(csrfService.currentNonce).toBeNull()
    expect(csrfService.expiresAt).toBeNull()
  })

  it('clears timers', () => {
    csrfService.refreshTimer = setTimeout(() => {}, 60000)
    expect(csrfService.refreshTimer).not.toBeNull()

    csrfService.cleanup()

    expect(csrfService.refreshTimer).toBeNull()
  })
})
