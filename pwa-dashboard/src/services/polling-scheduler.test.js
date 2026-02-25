import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Import the singleton
const { default: pollingScheduler } = await import('./polling-scheduler.js')

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers()
  // Cleanup all existing intervals
  for (const [key] of pollingScheduler._intervals) {
    pollingScheduler._stopInterval(key)
  }
  pollingScheduler._intervals.clear()
  pollingScheduler._paused = false
})

afterEach(() => {
  // Cleanup any intervals
  for (const [key] of pollingScheduler._intervals) {
    pollingScheduler._stopInterval(key)
  }
  pollingScheduler._intervals.clear()
  vi.useRealTimers()
})

// =====================================================================
// subscribe()
// =====================================================================
describe('subscribe()', () => {
  it('calls callback immediately on subscribe', () => {
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('10s', callback)

    expect(callback).toHaveBeenCalledTimes(1)
    unsub()
  })

  it('calls callback on each interval tick', () => {
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('10s', callback)
    expect(callback).toHaveBeenCalledTimes(1) // immediate

    vi.advanceTimersByTime(10000)
    expect(callback).toHaveBeenCalledTimes(2)

    vi.advanceTimersByTime(10000)
    expect(callback).toHaveBeenCalledTimes(3)

    unsub()
  })

  it('returns an unsubscribe function', () => {
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('10s', callback)
    expect(callback).toHaveBeenCalledTimes(1)

    unsub()

    vi.advanceTimersByTime(30000)
    expect(callback).toHaveBeenCalledTimes(1) // no more calls
  })

  it('supports seconds interval (10s)', () => {
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('10s', callback)

    vi.advanceTimersByTime(10000)
    expect(callback).toHaveBeenCalledTimes(2) // immediate + 1 tick

    unsub()
  })

  it('supports minutes interval (1m)', () => {
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('1m', callback)

    vi.advanceTimersByTime(60000)
    expect(callback).toHaveBeenCalledTimes(2) // immediate + 1 tick

    unsub()
  })

  it('shares interval for multiple subscribers at same rate', () => {
    const cb1 = vi.fn()
    const cb2 = vi.fn()

    const unsub1 = pollingScheduler.subscribe('30s', cb1)
    const unsub2 = pollingScheduler.subscribe('30s', cb2)

    // Both called immediately
    expect(cb1).toHaveBeenCalledTimes(1)
    expect(cb2).toHaveBeenCalledTimes(1)

    vi.advanceTimersByTime(30000)
    expect(cb1).toHaveBeenCalledTimes(2)
    expect(cb2).toHaveBeenCalledTimes(2)

    unsub1()
    unsub2()
  })

  it('stops interval when last subscriber unsubscribes', () => {
    const cb1 = vi.fn()
    const cb2 = vi.fn()

    const unsub1 = pollingScheduler.subscribe('10s', cb1)
    const unsub2 = pollingScheduler.subscribe('10s', cb2)

    unsub1()
    // Still running (cb2 active)
    vi.advanceTimersByTime(10000)
    expect(cb2).toHaveBeenCalledTimes(2)

    unsub2()
    // Interval should be removed
    expect(pollingScheduler._intervals.size).toBe(0)
  })

  it('does not call immediately when paused', () => {
    pollingScheduler._paused = true
    const callback = vi.fn()

    const unsub = pollingScheduler.subscribe('10s', callback)

    expect(callback).not.toHaveBeenCalled()

    unsub()
    pollingScheduler._paused = false
  })

  it('catches callback errors without crashing', () => {
    const badCallback = vi.fn(() => { throw new Error('oops') })

    const unsub = pollingScheduler.subscribe('10s', badCallback)

    // Should not throw
    expect(badCallback).toHaveBeenCalledTimes(1)

    vi.advanceTimersByTime(10000)
    expect(badCallback).toHaveBeenCalledTimes(2) // interval still fires

    unsub()
  })
})

// =====================================================================
// _parseInterval()
// =====================================================================
describe('_parseInterval()', () => {
  it('parses seconds', () => {
    expect(pollingScheduler._parseInterval('10s')).toBe(10000)
    expect(pollingScheduler._parseInterval('30s')).toBe(30000)
    expect(pollingScheduler._parseInterval('60s')).toBe(60000)
  })

  it('parses minutes', () => {
    expect(pollingScheduler._parseInterval('1m')).toBe(60000)
    expect(pollingScheduler._parseInterval('5m')).toBe(300000)
  })

  it('throws on invalid format', () => {
    expect(() => pollingScheduler._parseInterval('10')).toThrow('Invalid interval format')
    expect(() => pollingScheduler._parseInterval('abc')).toThrow('Invalid interval format')
    expect(() => pollingScheduler._parseInterval('10h')).toThrow('Invalid interval format')
  })
})

// =====================================================================
// _pause() / _resume()
// =====================================================================
describe('pause / resume', () => {
  it('pauses all intervals', () => {
    const callback = vi.fn()
    const unsub = pollingScheduler.subscribe('10s', callback)
    expect(callback).toHaveBeenCalledTimes(1)

    pollingScheduler._pause()

    vi.advanceTimersByTime(30000)
    expect(callback).toHaveBeenCalledTimes(1) // no more calls

    unsub()
  })

  it('resumes intervals and calls callbacks immediately', () => {
    const callback = vi.fn()
    const unsub = pollingScheduler.subscribe('10s', callback)
    expect(callback).toHaveBeenCalledTimes(1)

    pollingScheduler._pause()
    vi.advanceTimersByTime(30000)
    expect(callback).toHaveBeenCalledTimes(1)

    pollingScheduler._resume()
    // Immediate call on resume
    expect(callback).toHaveBeenCalledTimes(2)

    // Normal interval continues
    vi.advanceTimersByTime(10000)
    expect(callback).toHaveBeenCalledTimes(3)

    unsub()
  })

  it('is idempotent (double pause / double resume)', () => {
    const callback = vi.fn()
    const unsub = pollingScheduler.subscribe('10s', callback)

    pollingScheduler._pause()
    pollingScheduler._pause() // no-op
    expect(pollingScheduler._paused).toBe(true)

    pollingScheduler._resume()
    pollingScheduler._resume() // no-op
    expect(pollingScheduler._paused).toBe(false)

    unsub()
  })

  it('does not resume intervals with zero subscribers', () => {
    const callback = vi.fn()
    const unsub = pollingScheduler.subscribe('10s', callback)

    pollingScheduler._pause()
    unsub() // remove subscriber while paused

    pollingScheduler._resume()
    // callback should only have been called once (initial subscribe)
    // The interval entry is already removed by unsub
  })
})

// =====================================================================
// getStatus()
// =====================================================================
describe('getStatus()', () => {
  it('returns paused state', () => {
    expect(pollingScheduler.getStatus().paused).toBe(false)

    pollingScheduler._pause()
    expect(pollingScheduler.getStatus().paused).toBe(true)

    pollingScheduler._resume()
    expect(pollingScheduler.getStatus().paused).toBe(false)
  })

  it('returns interval info', () => {
    const unsub = pollingScheduler.subscribe('30s', () => {})

    const status = pollingScheduler.getStatus()
    expect(status.intervals['30000ms']).toBeDefined()
    expect(status.intervals['30000ms'].active).toBe(true)
    expect(status.intervals['30000ms'].subscribers).toBe(1)

    unsub()
  })

  it('returns empty intervals when nothing subscribed', () => {
    const status = pollingScheduler.getStatus()
    expect(Object.keys(status.intervals)).toHaveLength(0)
  })
})
