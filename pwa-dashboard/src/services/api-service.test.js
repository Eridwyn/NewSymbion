import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ── Mock fetch globally ──────────────────────────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Mock auth-service (SW handles auth headers now)
vi.mock('./auth-service.js', () => ({
  default: {
    getAuthHeader: vi.fn(() => ({})),
    isAuthenticated: vi.fn(() => false),
    getToken: vi.fn(() => null),
    whenReady: vi.fn(() => Promise.resolve())
  }
}))

// Mock csrf-service
vi.mock('./csrf-service.js', () => ({
  default: {
    fetchWithCsrf: vi.fn(() => Promise.resolve({ ok: true, json: async () => ({}) })),
    setAuthService: vi.fn()
  }
}))

// Mock offline-queue
vi.mock('./offline-queue.js', () => ({
  default: {
    enqueue: vi.fn()
  }
}))

const { ApiService } = await import('./api-service.js')
const { default: authService } = await import('./auth-service.js')
const { default: offlineQueue } = await import('./offline-queue.js')

// ── Helpers ──────────────────────────────────────────────────────────

function createService() {
  const svc = new ApiService()
  // Force baseUrl to avoid window.SYMBION_CONFIG dependency
  svc._baseUrl = 'https://localhost:8443'
  svc._apiKey = null
  return svc
}

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  mockFetch.mockReset()
  offlineQueue.enqueue.mockReset()
})

afterEach(() => {
  vi.useRealTimers()
})

// =====================================================================
// request() — success cases
// =====================================================================
describe('request() success', () => {
  it('calls fetch with correct URL', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ ok: true })
    })

    await svc.request('/health')

    expect(mockFetch).toHaveBeenCalledTimes(1)
    expect(mockFetch.mock.calls[0][0]).toBe('https://localhost:8443/health')
  })

  it('returns parsed JSON for application/json', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ status: 'healthy' })
    })

    const result = await svc.request('/health')

    expect(result).toEqual({ status: 'healthy' })
  })

  it('returns text for non-JSON content-type', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'text/plain' }),
      text: async () => 'OK'
    })

    const result = await svc.request('/health')

    expect(result).toBe('OK')
  })

  it('does not include auth header (SW injects it)', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/v1/context')

    const config = mockFetch.mock.calls[0][1]
    expect(config.headers['Authorization']).toBeUndefined()
  })

  it('includes api-key header when apiKey is configured', async () => {
    const svc = createService()
    svc._apiKey = 'my-api-key'
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/health')

    const config = mockFetch.mock.calls[0][1]
    expect(config.headers['x-api-key']).toBe('my-api-key')
  })

  it('does NOT include x-api-key when apiKey is null', async () => {
    const svc = createService()
    svc._apiKey = null
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/health')

    const config = mockFetch.mock.calls[0][1]
    expect(config.headers['x-api-key']).toBeUndefined()
  })

  it('sets Content-Type to application/json', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/health')

    const config = mockFetch.mock.calls[0][1]
    expect(config.headers['Content-Type']).toBe('application/json')
  })

  it('sets status to online on success', async () => {
    const svc = createService()
    svc.status = 'offline'
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/health')

    expect(svc.status).toBe('online')
  })

  it('passes through custom options (method, body)', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ created: true })
    })

    await svc.request('/v1/notes', {
      method: 'POST',
      body: JSON.stringify({ title: 'test' })
    })

    const config = mockFetch.mock.calls[0][1]
    expect(config.method).toBe('POST')
    expect(config.body).toBe('{"title":"test"}')
  })
})

// =====================================================================
// request() — error handling
// =====================================================================
describe('request() error handling', () => {
  it('throws on 401 and dispatches auth:expired event', async () => {
    const svc = createService()
    const handler = vi.fn()
    window.addEventListener('auth:expired', handler)

    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 401,
      statusText: 'Unauthorized',
      headers: new Headers()
    })

    await expect(svc.request('/v1/context')).rejects.toThrow('Session expirée')
    expect(handler).toHaveBeenCalledTimes(1)

    window.removeEventListener('auth:expired', handler)
  })

  it('throws on 5xx server errors', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 503,
      statusText: 'Service Unavailable',
      headers: new Headers()
    })

    await expect(svc.request('/v1/context'))
      .rejects.toThrow('HTTP 503: Service Unavailable')
  })

  it('throws and sets offline on 4xx client errors', async () => {
    const svc = createService()
    svc.status = 'online'
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
      statusText: 'Not Found',
      headers: new Headers()
    })

    await expect(svc.request('/v1/nonexistent'))
      .rejects.toThrow('HTTP 404: Not Found')
    expect(svc.status).toBe('offline')
  })

  it('throws on network error (TypeError)', async () => {
    const svc = createService()
    mockFetch.mockRejectedValueOnce(new TypeError('Failed to fetch'))

    await expect(svc.request('/health'))
      .rejects.toThrow('Failed to fetch')
    expect(svc.status).toBe('offline')
  })

  it('throws timeout error on AbortError', async () => {
    const svc = createService()

    mockFetch.mockImplementationOnce((_url, options) => {
      return new Promise((_resolve, reject) => {
        // Simulate abort after timeout
        if (options.signal) {
          options.signal.addEventListener('abort', () => {
            const err = new Error('The operation was aborted.')
            err.name = 'AbortError'
            reject(err)
          })
        }
      })
    })

    vi.useFakeTimers()

    const promise = svc.request('/v1/notes', { timeout: 5000 })
    vi.advanceTimersByTime(5000)

    await expect(promise).rejects.toThrow('Request timeout after 5000ms')
  })
})

// =====================================================================
// request() — timeout support
// =====================================================================
describe('request() timeout', () => {
  it('does not use AbortController when no timeout', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/health')

    const config = mockFetch.mock.calls[0][1]
    expect(config.signal).toBeUndefined()
  })

  it('uses AbortController when timeout is set', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/v1/notes', { timeout: 30000 })

    const config = mockFetch.mock.calls[0][1]
    expect(config.signal).toBeInstanceOf(AbortSignal)
  })

  it('removes timeout key from config passed to fetch', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({})
    })

    await svc.request('/v1/notes', { timeout: 30000 })

    const config = mockFetch.mock.calls[0][1]
    expect(config.timeout).toBeUndefined()
  })
})

// =====================================================================
// _sanitizeResponse()
// =====================================================================
describe('_sanitizeResponse()', () => {
  const svc = createService()

  it('strips HTML tags from strings', () => {
    const result = svc._sanitizeResponse('<script>alert("xss")</script>hello')
    expect(result).not.toContain('<script>')
    expect(result).toContain('hello')
  })

  it('sanitizes nested objects', () => {
    const result = svc._sanitizeResponse({
      name: '<img onerror=alert(1) src=x>Test',
      nested: { value: '<b>bold</b>' }
    })

    expect(result.name).not.toContain('<img')
    expect(result.nested.value).not.toContain('<b>')
  })

  it('sanitizes arrays', () => {
    const result = svc._sanitizeResponse([
      '<script>bad</script>',
      'clean'
    ])

    expect(result[0]).not.toContain('<script>')
    expect(result[1]).toBe('clean')
  })

  it('preserves numbers', () => {
    expect(svc._sanitizeResponse(42)).toBe(42)
    expect(svc._sanitizeResponse(3.14)).toBe(3.14)
  })

  it('preserves booleans', () => {
    expect(svc._sanitizeResponse(true)).toBe(true)
    expect(svc._sanitizeResponse(false)).toBe(false)
  })

  it('preserves null', () => {
    expect(svc._sanitizeResponse(null)).toBeNull()
  })

  it('handles deeply nested structures', () => {
    const result = svc._sanitizeResponse({
      a: { b: { c: [{ d: '<svg onload=alert(1)>text' }] } }
    })

    expect(result.a.b.c[0].d).not.toContain('<svg')
  })
})

// =====================================================================
// validateArrayResponse()
// =====================================================================
describe('validateArrayResponse()', () => {
  const svc = createService()

  it('returns data if it is an array', () => {
    const data = [{ id: 1 }, { id: 2 }]
    expect(svc.validateArrayResponse(data, '/test')).toBe(data)
  })

  it('returns fallback for non-array data', () => {
    expect(svc.validateArrayResponse({ error: 'not array' }, '/test')).toEqual([])
  })

  it('returns custom fallback', () => {
    const fallback = [{ id: 'default' }]
    expect(svc.validateArrayResponse('string', '/test', fallback)).toBe(fallback)
  })

  it('returns data for empty array', () => {
    const data = []
    expect(svc.validateArrayResponse(data, '/test')).toBe(data)
  })
})

// =====================================================================
// isOnline() / isOffline()
// =====================================================================
describe('isOnline() / isOffline()', () => {
  it('isOnline returns true when status is online', () => {
    const svc = createService()
    svc.status = 'online'
    expect(svc.isOnline()).toBe(true)
    expect(svc.isOffline()).toBe(false)
  })

  it('isOffline returns true when status is offline', () => {
    const svc = createService()
    svc.status = 'offline'
    expect(svc.isOffline()).toBe(true)
    expect(svc.isOnline()).toBe(false)
  })

  it('both false during loading', () => {
    const svc = createService()
    svc.status = 'loading'
    expect(svc.isOnline()).toBe(false)
    expect(svc.isOffline()).toBe(false)
  })
})

// =====================================================================
// updateStatus()
// =====================================================================
describe('updateStatus()', () => {
  it('updates status property', () => {
    const svc = createService()

    svc.updateStatus('online')

    expect(svc.status).toBe('online')
  })

  it('dispatches status-change event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('status-change', handler)

    svc.updateStatus('offline')

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.status).toBe('offline')

    svc.removeEventListener('status-change', handler)
  })
})

// =====================================================================
// checkConnection()
// =====================================================================
describe('checkConnection()', () => {
  it('sets status to online when /health succeeds', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => ({ status: 'ok' })
    })

    await svc.checkConnection()

    expect(svc.status).toBe('online')
  })

  it('sets status to offline when /health fails', async () => {
    const svc = createService()
    mockFetch.mockRejectedValueOnce(new TypeError('Failed to fetch'))

    await svc.checkConnection()

    expect(svc.status).toBe('offline')
  })
})

// =====================================================================
// baseUrl lazy getter
// =====================================================================
describe('baseUrl', () => {
  it('falls back to window.location based URL', () => {
    const svc = new ApiService()
    svc._baseUrl = null
    // window.SYMBION_CONFIG not set, should compute from location
    const url = svc.baseUrl
    expect(typeof url).toBe('string')
    expect(url).toContain(':8443')
  })
})
