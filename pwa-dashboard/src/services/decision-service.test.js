import { describe, it, expect, vi, beforeEach } from 'vitest'

// ── Mock fetch globally ──────────────────────────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// ── Mock auth-service and csrf-service ───────────────────────────────
const mockAuthService = {
  getToken: vi.fn(() => 'jwt-test-token'),
  getCurrentUser: vi.fn(() => ({ username: 'eridwyn', role: 'admin' })),
  isAuthenticated: vi.fn(() => true)
}

const mockCsrfService = {
  fetchWithCsrf: vi.fn(),
  setAuthService: vi.fn(),
  getNonce: vi.fn(() => Promise.resolve('csrf-nonce-123'))
}

vi.mock('./auth-service.js', () => ({ default: mockAuthService }))
vi.mock('./csrf-service.js', () => ({ default: mockCsrfService }))

const { default: decisionService } = await import('./decision-service.js')

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  mockFetch.mockReset()
  mockAuthService.getToken.mockReturnValue('jwt-test-token')
  mockAuthService.getCurrentUser.mockReturnValue({ username: 'eridwyn', role: 'admin' })
  mockCsrfService.fetchWithCsrf.mockReset()
  mockCsrfService.setAuthService.mockReset()
  mockCsrfService.getNonce.mockResolvedValue('csrf-nonce-123')

  // Force re-init on each test
  decisionService.authService = null
  decisionService.csrfService = null
})

// =====================================================================
// init()
// =====================================================================
describe('init()', () => {
  it('lazy-loads authService and csrfService', async () => {
    await decisionService.init()

    expect(decisionService.authService).toBe(mockAuthService)
    expect(decisionService.csrfService).toBe(mockCsrfService)
  })

  it('calls setAuthService on csrfService', async () => {
    await decisionService.init()

    expect(mockCsrfService.setAuthService).toHaveBeenCalledWith(mockAuthService)
  })

  it('does not re-init if already initialized', async () => {
    decisionService.authService = mockAuthService
    decisionService.csrfService = mockCsrfService

    await decisionService.init()

    // setAuthService should NOT be called again since csrfService was already set
    expect(mockCsrfService.setAuthService).not.toHaveBeenCalled()
  })
})

// =====================================================================
// get()
// =====================================================================
describe('get()', () => {
  it('calls fetch with GET method and correct headers', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: 'test' })
    })

    await decisionService.get('/v1/decision/stats')

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/v1/decision/stats')
    expect(options.method).toBe('GET')
    expect(options.headers['Authorization']).toBe('Bearer jwt-test-token')
    expect(options.headers['Content-Type']).toBe('application/json')
    expect(options.credentials).toBe('include')
  })

  it('returns parsed JSON response', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ validations: [] })
    })

    const result = await decisionService.get('/v1/decision/validations/pending')

    expect(result).toEqual({ validations: [] })
  })

  it('throws if not authenticated', async () => {
    mockAuthService.getToken.mockReturnValue(null)

    await expect(decisionService.get('/v1/decision/stats'))
      .rejects.toThrow('Not authenticated')
  })

  it('throws on HTTP error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error'
    })

    await expect(decisionService.get('/v1/decision/stats'))
      .rejects.toThrow('HTTP 500: Internal Server Error')
  })
})

// =====================================================================
// post()
// =====================================================================
describe('post()', () => {
  it('calls csrfService.fetchWithCsrf with POST and JSON body', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ success: true })
    })

    await decisionService.post('/v1/decision/evaluate', { action: 'test' })

    expect(mockCsrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = mockCsrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/decision/evaluate')
    expect(options.method).toBe('POST')
    expect(options.headers['Content-Type']).toBe('application/json')
    expect(JSON.parse(options.body)).toEqual({ action: 'test' })
  })

  it('returns parsed JSON response', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ decision: 'approved' })
    })

    const result = await decisionService.post('/v1/decision/evaluate', {})

    expect(result).toEqual({ decision: 'approved' })
  })

  it('throws on HTTP error from CSRF fetch', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: false,
      status: 403,
      statusText: 'Forbidden'
    })

    await expect(decisionService.post('/v1/decision/override', {}))
      .rejects.toThrow('HTTP 403: Forbidden')
  })
})

// =====================================================================
// getPendingValidations()
// =====================================================================
describe('getPendingValidations()', () => {
  it('calls GET /v1/decision/validations/pending', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ pending: [] })
    })

    const result = await decisionService.getPendingValidations()

    const url = mockFetch.mock.calls[0][0]
    expect(url).toContain('/v1/decision/validations/pending')
    expect(result).toEqual({ pending: [] })
  })
})

// =====================================================================
// getExpiredValidations()
// =====================================================================
describe('getExpiredValidations()', () => {
  it('calls GET /v1/decision/validations/expired', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ expired: [] })
    })

    await decisionService.getExpiredValidations()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/validations/expired')
  })
})

// =====================================================================
// resolveValidation()
// =====================================================================
describe('resolveValidation()', () => {
  it('posts to /v1/decision/validation/:id/resolve with approved and username', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ resolved: true })
    })

    await decisionService.resolveValidation('val-123', true)

    const [url, options] = mockCsrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/decision/validation/val-123/resolve')
    const body = JSON.parse(options.body)
    expect(body.approved).toBe(true)
    expect(body.username).toBe('eridwyn')
  })

  it('sends false for rejected validations', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ resolved: true })
    })

    await decisionService.resolveValidation('val-456', false)

    const body = JSON.parse(mockCsrfService.fetchWithCsrf.mock.calls[0][1].body)
    expect(body.approved).toBe(false)
  })

  it('uses "unknown" username if getCurrentUser returns null', async () => {
    mockAuthService.getCurrentUser.mockReturnValue(null)
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ resolved: true })
    })

    await decisionService.resolveValidation('val-789', true)

    const body = JSON.parse(mockCsrfService.fetchWithCsrf.mock.calls[0][1].body)
    expect(body.username).toBe('unknown')
  })
})

// =====================================================================
// deleteValidation()
// =====================================================================
describe('deleteValidation()', () => {
  it('sends DELETE with auth and CSRF headers', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true })

    await decisionService.deleteValidation('val-to-delete')

    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/v1/decision/validation/val-to-delete')
    expect(options.method).toBe('DELETE')
    expect(options.headers['Authorization']).toBe('Bearer jwt-test-token')
    expect(options.headers['X-CSRF-Token']).toBe('csrf-nonce-123')
    expect(options.credentials).toBe('include')
  })

  it('throws if not authenticated', async () => {
    mockAuthService.getToken.mockReturnValue(null)

    await expect(decisionService.deleteValidation('val-1'))
      .rejects.toThrow('Not authenticated')
  })

  it('throws on HTTP error', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404, statusText: 'Not Found' })

    await expect(decisionService.deleteValidation('val-unknown'))
      .rejects.toThrow('HTTP 404: Not Found')
  })
})

// =====================================================================
// deleteAllExpiredValidations()
// =====================================================================
describe('deleteAllExpiredValidations()', () => {
  it('sends DELETE to /v1/decision/validations/expired', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ deleted: 5 })
    })

    const result = await decisionService.deleteAllExpiredValidations()

    const [url, options] = mockFetch.mock.calls[0]
    expect(url).toContain('/v1/decision/validations/expired')
    expect(options.method).toBe('DELETE')
    expect(result).toEqual({ deleted: 5 })
  })

  it('throws if not authenticated', async () => {
    mockAuthService.getToken.mockReturnValue(null)

    await expect(decisionService.deleteAllExpiredValidations())
      .rejects.toThrow('Not authenticated')
  })
})

// =====================================================================
// getActiveOverrides()
// =====================================================================
describe('getActiveOverrides()', () => {
  it('calls GET /v1/decision/overrides/active', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ overrides: [] })
    })

    const result = await decisionService.getActiveOverrides()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/overrides/active')
    expect(result).toEqual({ overrides: [] })
  })
})

// =====================================================================
// createOverride()
// =====================================================================
describe('createOverride()', () => {
  it('posts override with all parameters', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ created: true })
    })

    await decisionService.createOverride('agent-1', { type: 'shutdown' }, 30, 'maintenance', '123456')

    const body = JSON.parse(mockCsrfService.fetchWithCsrf.mock.calls[0][1].body)
    expect(body.agent_id).toBe('agent-1')
    expect(body.action).toEqual({ type: 'shutdown' })
    expect(body.duration_minutes).toBe(30)
    expect(body.reason).toBe('maintenance')
    expect(body.totp_code).toBe('123456')
  })
})

// =====================================================================
// getAuditTrail() / getStats() / getMetrics() / getConfig() / getAgentHealth()
// =====================================================================
describe('read-only endpoints', () => {
  it('getAuditTrail calls /v1/decision/audit', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({ entries: [] }) })

    await decisionService.getAuditTrail()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/audit')
  })

  it('getStats calls /v1/decision/stats', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) })

    await decisionService.getStats()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/stats')
  })

  it('getMetrics calls /v1/decision/metrics', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) })

    await decisionService.getMetrics()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/metrics')
  })

  it('getConfig calls /v1/decision/config', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) })

    await decisionService.getConfig()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/config')
  })

  it('getAgentHealth calls /v1/decision/agent-health', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) })

    await decisionService.getAgentHealth()

    expect(mockFetch.mock.calls[0][0]).toContain('/v1/decision/agent-health')
  })
})

// =====================================================================
// evaluateAction()
// =====================================================================
describe('evaluateAction()', () => {
  it('posts action and context to /v1/decision/evaluate', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ decision: 'approved', trust_score: 0.85 })
    })

    const action = { type: 'shutdown', target: 'agent-1' }
    const context = { mode: 'pro', ssid: 'home' }
    const result = await decisionService.evaluateAction(action, context)

    const [url, options] = mockCsrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/decision/evaluate')
    const body = JSON.parse(options.body)
    expect(body.action).toEqual(action)
    expect(body.context).toEqual(context)
    expect(result.decision).toBe('approved')
  })
})
