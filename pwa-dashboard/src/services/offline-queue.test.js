import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ── Mock fetch globally ──────────────────────────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Fresh import — the module creates a singleton immediately
const { default: offlineQueue } = await import('./offline-queue.js')

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  offlineQueue.clear()
  mockFetch.mockReset()
  localStorage.clear()
})

// =====================================================================
// enqueue()
// =====================================================================
describe('enqueue()', () => {
  it('adds an entry to the queue', () => {
    offlineQueue.enqueue('https://api/v1/notes', { method: 'POST', body: '{}' })

    expect(offlineQueue.pendingCount).toBe(1)
  })

  it('returns an id string', () => {
    const id = offlineQueue.enqueue('https://api/test', { method: 'POST' })

    expect(typeof id).toBe('string')
    expect(id.length).toBeGreaterThan(0)
  })

  it('persists queue to localStorage', () => {
    offlineQueue.enqueue('https://api/v1/notes', { method: 'POST', body: '{"title":"test"}' })

    const stored = JSON.parse(localStorage.getItem('symbion-offline-queue'))
    expect(stored).toHaveLength(1)
    expect(stored[0].url).toBe('https://api/v1/notes')
    expect(stored[0].method).toBe('POST')
    expect(stored[0].body).toBe('{"title":"test"}')
  })

  it('preserves headers from options', () => {
    offlineQueue.enqueue('https://api/test', {
      method: 'PUT',
      headers: { 'X-Custom': 'value' }
    })

    const stored = JSON.parse(localStorage.getItem('symbion-offline-queue'))
    expect(stored[0].headers['X-Custom']).toBe('value')
  })

  it('records timestamp on each entry', () => {
    const before = Date.now()
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    const after = Date.now()

    const stored = JSON.parse(localStorage.getItem('symbion-offline-queue'))
    expect(stored[0].timestamp).toBeGreaterThanOrEqual(before)
    expect(stored[0].timestamp).toBeLessThanOrEqual(after)
  })

  it('increments pendingCount on each enqueue', () => {
    offlineQueue.enqueue('https://api/1', { method: 'POST' })
    offlineQueue.enqueue('https://api/2', { method: 'PUT' })
    offlineQueue.enqueue('https://api/3', { method: 'DELETE' })

    expect(offlineQueue.pendingCount).toBe(3)
  })

  it('dispatches offline-queue-change event on document.body', () => {
    const handler = vi.fn()
    document.body.addEventListener('offline-queue-change', handler)

    offlineQueue.enqueue('https://api/test', { method: 'POST' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.pending).toBe(1)

    document.body.removeEventListener('offline-queue-change', handler)
  })

  it('defaults method to POST when not specified', () => {
    offlineQueue.enqueue('https://api/test', {})

    const stored = JSON.parse(localStorage.getItem('symbion-offline-queue'))
    expect(stored[0].method).toBe('POST')
  })
})

// =====================================================================
// sync()
// =====================================================================
describe('sync()', () => {
  it('returns immediately if queue is empty', async () => {
    const result = await offlineQueue.sync()

    expect(result).toBeUndefined()
    expect(mockFetch).not.toHaveBeenCalled()
  })

  it('returns immediately if already syncing', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    offlineQueue._syncing = true

    const result = await offlineQueue.sync()

    expect(result).toBeUndefined()
    expect(mockFetch).not.toHaveBeenCalled()

    offlineQueue._syncing = false
  })

  it('replays queued requests in order', async () => {
    offlineQueue.enqueue('https://api/1', { method: 'POST', body: '{"a":1}' })
    offlineQueue.enqueue('https://api/2', { method: 'PUT', body: '{"b":2}' })

    mockFetch.mockResolvedValue({ ok: true, status: 200 })

    // Make sure navigator.onLine is true
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    await offlineQueue.sync()

    expect(mockFetch).toHaveBeenCalledTimes(2)
    expect(mockFetch.mock.calls[0][0]).toBe('https://api/1')
    expect(mockFetch.mock.calls[1][0]).toBe('https://api/2')
  })

  it('clears queue after successful sync', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockResolvedValue({ ok: true, status: 200 })
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const result = await offlineQueue.sync()

    expect(result.success).toBe(1)
    expect(result.failed).toBe(0)
    expect(offlineQueue.pendingCount).toBe(0)
  })

  it('treats 409 Conflict as success (idempotent)', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockResolvedValue({ ok: false, status: 409 })
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const result = await offlineQueue.sync()

    expect(result.success).toBe(1)
    expect(offlineQueue.pendingCount).toBe(0)
  })

  it('keeps 5xx failures for retry', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockResolvedValue({ ok: false, status: 503 })
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const result = await offlineQueue.sync()

    expect(result.failed).toBe(1)
    expect(offlineQueue.pendingCount).toBe(1) // kept for retry
  })

  it('discards 4xx failures (client errors)', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockResolvedValue({ ok: false, status: 400 })
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const result = await offlineQueue.sync()

    expect(result.failed).toBe(1)
    expect(offlineQueue.pendingCount).toBe(0) // discarded
  })

  it('keeps entries on network error', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockRejectedValue(new Error('Network error'))
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const result = await offlineQueue.sync()

    expect(result.failed).toBe(1)
    expect(offlineQueue.pendingCount).toBe(1)
  })

  it('dispatches offline-queue-change after sync', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    mockFetch.mockResolvedValue({ ok: true, status: 200 })
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })

    const handler = vi.fn()
    document.body.addEventListener('offline-queue-change', handler)

    await offlineQueue.sync()

    // The event fires: once from enqueue, once from sync
    const lastCall = handler.mock.calls[handler.mock.calls.length - 1][0]
    expect(lastCall.detail.pending).toBe(0)
    expect(lastCall.detail.lastSync).toEqual({ success: 1, failed: 0 })

    document.body.removeEventListener('offline-queue-change', handler)
  })

  it('returns undefined if offline', async () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    Object.defineProperty(navigator, 'onLine', { value: false, writable: true })

    const result = await offlineQueue.sync()

    expect(result).toBeUndefined()
    expect(mockFetch).not.toHaveBeenCalled()

    // Restore
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true })
  })
})

// =====================================================================
// clear()
// =====================================================================
describe('clear()', () => {
  it('empties the queue', () => {
    offlineQueue.enqueue('https://api/1', { method: 'POST' })
    offlineQueue.enqueue('https://api/2', { method: 'PUT' })
    expect(offlineQueue.pendingCount).toBe(2)

    offlineQueue.clear()

    expect(offlineQueue.pendingCount).toBe(0)
  })

  it('clears localStorage', () => {
    offlineQueue.enqueue('https://api/test', { method: 'POST' })
    expect(localStorage.getItem('symbion-offline-queue')).not.toBeNull()

    offlineQueue.clear()

    const stored = JSON.parse(localStorage.getItem('symbion-offline-queue'))
    expect(stored).toEqual([])
  })
})

// =====================================================================
// pendingCount
// =====================================================================
describe('pendingCount', () => {
  it('returns 0 for empty queue', () => {
    expect(offlineQueue.pendingCount).toBe(0)
  })

  it('reflects enqueued items', () => {
    offlineQueue.enqueue('https://api/a', { method: 'POST' })
    expect(offlineQueue.pendingCount).toBe(1)

    offlineQueue.enqueue('https://api/b', { method: 'POST' })
    expect(offlineQueue.pendingCount).toBe(2)
  })
})

// =====================================================================
// _loadQueue / _saveQueue (persistence)
// =====================================================================
describe('persistence', () => {
  it('loads queue from localStorage on construction', () => {
    localStorage.setItem('symbion-offline-queue', JSON.stringify([
      { id: '1', url: 'https://api/test', method: 'POST', body: null, headers: {}, timestamp: Date.now() }
    ]))

    // Reload the queue
    const loaded = offlineQueue._loadQueue()
    expect(loaded).toHaveLength(1)
    expect(loaded[0].url).toBe('https://api/test')
  })

  it('returns empty array on corrupted localStorage', () => {
    localStorage.setItem('symbion-offline-queue', 'not-json!!!')

    const loaded = offlineQueue._loadQueue()
    expect(loaded).toEqual([])
  })

  it('returns empty array when nothing stored', () => {
    localStorage.removeItem('symbion-offline-queue')

    const loaded = offlineQueue._loadQueue()
    expect(loaded).toEqual([])
  })
})
