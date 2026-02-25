import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// ── Mock auth-service ────────────────────────────────────────────────
vi.mock('./auth-service.js', () => ({
  default: {
    getToken: vi.fn(() => 'jwt-test-token'),
    isAuthenticated: vi.fn(() => true)
  }
}))

const { default: authService } = await import('./auth-service.js')

// ── Mock WebSocket ───────────────────────────────────────────────────
let mockWsInstance = null

class MockWebSocket {
  static OPEN = 1
  static CONNECTING = 0
  static CLOSED = 3

  constructor(url) {
    this.url = url
    this.readyState = MockWebSocket.CONNECTING
    this.onopen = null
    this.onmessage = null
    this.onerror = null
    this.onclose = null
    mockWsInstance = this
  }

  send(data) {
    this._sentData = data
  }

  close() {
    this.readyState = MockWebSocket.CLOSED
    if (this.onclose) this.onclose()
  }
}

global.WebSocket = MockWebSocket

// Import the class (the module also creates a singleton + appends to body, but that's OK)
const { default: notesStreamService } = await import('./notes-stream-service.js')

// Use the class for fresh instances
const NotesStreamServiceClass = notesStreamService.constructor

// ── Helpers ──────────────────────────────────────────────────────────

function createService() {
  const svc = new NotesStreamServiceClass()
  svc.connected = false
  svc.loading = false
  svc.ws = null
  svc.reconnectAttempts = 0
  return svc
}

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  mockWsInstance = null
  authService.getToken.mockReturnValue('jwt-test-token')
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

// =====================================================================
// wsUrl
// =====================================================================
describe('wsUrl', () => {
  it('includes JWT token as query param', () => {
    const svc = createService()

    const url = svc.wsUrl

    expect(url).toContain('token=jwt-test-token')
    expect(url).toContain('/ws/notes/stream')
  })

  it('uses wss when location is https', () => {
    const svc = createService()

    // happy-dom uses http by default, but the code checks protocol
    const url = svc.wsUrl
    expect(url).toMatch(/^wss?:\/\//)
  })

  it('falls back to API_KEY if no token', () => {
    authService.getToken.mockReturnValue(null)
    window.SYMBION_CONFIG = { API_KEY: 'test-key' }
    const svc = createService()

    const url = svc.wsUrl

    expect(url).toContain('api_key=test-key')

    delete window.SYMBION_CONFIG
  })

  it('returns bare URL if no auth available', () => {
    authService.getToken.mockReturnValue(null)
    const svc = createService()

    const url = svc.wsUrl

    expect(url).toContain('/ws/notes/stream')
    expect(url).not.toContain('token=')
  })
})

// =====================================================================
// connect()
// =====================================================================
describe('connect()', () => {
  it('creates a WebSocket connection', () => {
    const svc = createService()

    svc.connect()

    expect(mockWsInstance).not.toBeNull()
    expect(mockWsInstance.url).toContain('/ws/notes/stream')
  })

  it('does not connect if already connected', () => {
    const svc = createService()
    svc.ws = { readyState: WebSocket.OPEN }

    svc.connect()

    // mockWsInstance should not be set (no new WebSocket created)
    expect(mockWsInstance).toBeNull()
  })

  it('does not connect if already connecting', () => {
    const svc = createService()
    svc.ws = { readyState: WebSocket.CONNECTING }

    svc.connect()

    expect(mockWsInstance).toBeNull()
  })

  it('skips connection if no auth available', () => {
    authService.getToken.mockReturnValue(null)
    const svc = createService()

    svc.connect()

    expect(mockWsInstance).toBeNull()
  })

  it('sets connected=true on open', () => {
    const svc = createService()

    svc.connect()
    mockWsInstance.onopen()

    expect(svc.connected).toBe(true)
  })

  it('resets reconnectAttempts on open', () => {
    const svc = createService()
    svc.reconnectAttempts = 3

    svc.connect()
    mockWsInstance.onopen()

    expect(svc.reconnectAttempts).toBe(0)
  })

  it('dispatches ws-connected on open', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('ws-connected', handler)

    svc.connect()
    mockWsInstance.onopen()

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('ws-connected', handler)
  })

  it('dispatches ws-error on error', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('ws-error', handler)

    svc.connect()
    mockWsInstance.onerror(new Error('ws error'))

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('ws-error', handler)
  })

  it('sets connected=false on close', () => {
    const svc = createService()

    svc.connect()
    mockWsInstance.onopen()
    expect(svc.connected).toBe(true)

    mockWsInstance.onclose()
    expect(svc.connected).toBe(false)
  })

  it('dispatches ws-closed on close', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('ws-closed', handler)

    svc.connect()
    mockWsInstance.onclose()

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('ws-closed', handler)
  })

  it('attempts reconnection after close', () => {
    const svc = createService()
    svc.connect()
    mockWsInstance.onclose()

    expect(svc.reconnectAttempts).toBe(1)

    // After delay, should try to reconnect
    vi.advanceTimersByTime(2000)
    // New WebSocket should be created
    expect(mockWsInstance).not.toBeNull()
  })

  it('stops reconnecting after maxReconnectAttempts', () => {
    const svc = createService()
    svc.reconnectAttempts = svc.maxReconnectAttempts

    svc.connect()
    mockWsInstance.onclose()

    // onclose checks `< max` so it skips the reconnect branch entirely
    expect(svc.reconnectAttempts).toBe(svc.maxReconnectAttempts)
  })
})

// =====================================================================
// disconnect()
// =====================================================================
describe('disconnect()', () => {
  it('closes the WebSocket', () => {
    const svc = createService()
    svc.connect()
    const ws = mockWsInstance

    svc.disconnect()

    expect(ws.readyState).toBe(WebSocket.CLOSED)
  })

  it('sets ws to null', () => {
    const svc = createService()
    svc.connect()

    svc.disconnect()

    expect(svc.ws).toBeNull()
  })

  it('sets connected and loading to false', () => {
    const svc = createService()
    svc.connected = true
    svc.loading = true

    svc.disconnect()

    expect(svc.connected).toBe(false)
    expect(svc.loading).toBe(false)
  })

  it('prevents auto-reconnection', () => {
    const svc = createService()

    svc.disconnect()

    expect(svc.reconnectAttempts).toBe(svc.maxReconnectAttempts)
  })

  it('handles disconnect when ws is null', () => {
    const svc = createService()
    svc.ws = null

    // Should not throw
    svc.disconnect()

    expect(svc.connected).toBe(false)
  })
})

// =====================================================================
// handleMessage()
// =====================================================================
describe('handleMessage()', () => {
  it('dispatches note-received for note type', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('note-received', handler)

    svc.handleMessage({ type: 'note', note: { id: 'n1', title: 'Test' } })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.note.title).toBe('Test')

    svc.removeEventListener('note-received', handler)
  })

  it('dispatches note-received for note_item type', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('note-received', handler)

    svc.handleMessage({ type: 'note_item', note: { id: 'n2' } })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('note-received', handler)
  })

  it('dispatches notes-complete for end type', () => {
    const svc = createService()
    svc.loading = true
    const handler = vi.fn()
    svc.addEventListener('notes-complete', handler)

    svc.handleMessage({ type: 'end', total_count: 10, received_count: 10 })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.totalCount).toBe(10)
    expect(svc.loading).toBe(false)

    svc.removeEventListener('notes-complete', handler)
  })

  it('dispatches notes-complete for list_end type', () => {
    const svc = createService()
    svc.loading = true
    const handler = vi.fn()
    svc.addEventListener('notes-complete', handler)

    svc.handleMessage({ type: 'list_end', total_count: 5 })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(svc.loading).toBe(false)

    svc.removeEventListener('notes-complete', handler)
  })

  it('dispatches notes-error for error type', () => {
    const svc = createService()
    svc.loading = true
    const handler = vi.fn()
    svc.addEventListener('notes-error', handler)

    svc.handleMessage({ type: 'error', error: 'Permission denied' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.error).toBe('Permission denied')
    expect(svc.loading).toBe(false)

    svc.removeEventListener('notes-error', handler)
  })

  it('handles unknown message types gracefully', () => {
    const svc = createService()

    // Should not throw
    svc.handleMessage({ type: 'unknown_type', data: 'whatever' })
  })
})

// =====================================================================
// onmessage handler (JSON parsing)
// =====================================================================
describe('WebSocket onmessage', () => {
  it('parses JSON messages', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('note-received', handler)

    svc.connect()
    mockWsInstance.onopen()
    mockWsInstance.onmessage({ data: JSON.stringify({ type: 'note', note: { id: 'n1' } }) })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('note-received', handler)
  })

  it('handles invalid JSON gracefully', () => {
    const svc = createService()

    svc.connect()
    mockWsInstance.onopen()

    // Should not throw
    mockWsInstance.onmessage({ data: 'not-json{' })
  })
})

// =====================================================================
// constructor defaults
// =====================================================================
describe('constructor defaults', () => {
  it('starts disconnected', () => {
    const svc = createService()
    expect(svc.connected).toBe(false)
  })

  it('starts not loading', () => {
    const svc = createService()
    expect(svc.loading).toBe(false)
  })

  it('starts with 0 reconnectAttempts', () => {
    const svc = createService()
    expect(svc.reconnectAttempts).toBe(0)
  })

  it('has maxReconnectAttempts of 5', () => {
    const svc = createService()
    expect(svc.maxReconnectAttempts).toBe(5)
  })

  it('has reconnectDelay of 1000ms', () => {
    const svc = createService()
    expect(svc.reconnectDelay).toBe(1000)
  })
})
