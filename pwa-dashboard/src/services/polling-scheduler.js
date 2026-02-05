/**
 * Polling Scheduler - Centralized interval management
 *
 * Coordinates polling across widgets to:
 * - Reduce battery/CPU usage by aligning intervals
 * - Provide single cleanup point
 * - Pause when page is hidden
 *
 * Usage:
 *   import pollingScheduler from './polling-scheduler.js'
 *   const unsubscribe = pollingScheduler.subscribe('30s', () => myCallback())
 *   // In disconnectedCallback: unsubscribe()
 */

class PollingScheduler extends EventTarget {
  constructor() {
    super()
    this._intervals = new Map() // interval key -> { id, callbacks }
    this._paused = false

    // Pause polling when page hidden
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) {
        this._pause()
      } else {
        this._resume()
      }
    })

    console.log('[polling-scheduler] Initialized')
  }

  /**
   * Subscribe to a polling interval
   * @param {string} interval - '10s', '30s', '60s'
   * @param {Function} callback - Function to call on each tick
   * @returns {Function} Unsubscribe function
   */
  subscribe(interval, callback) {
    const ms = this._parseInterval(interval)
    const key = `${ms}ms`

    if (!this._intervals.has(key)) {
      this._intervals.set(key, {
        id: null,
        ms: ms,
        callbacks: new Set()
      })
      this._startInterval(key)
    }

    const entry = this._intervals.get(key)
    entry.callbacks.add(callback)

    // Immediate first call
    if (!this._paused) {
      try { callback() } catch (e) { console.error('[polling-scheduler] Callback error:', e) }
    }

    // Return unsubscribe function
    return () => {
      entry.callbacks.delete(callback)
      // Stop interval if no more subscribers
      if (entry.callbacks.size === 0) {
        this._stopInterval(key)
        this._intervals.delete(key)
      }
    }
  }

  _parseInterval(interval) {
    const match = interval.match(/^(\d+)(s|m)$/)
    if (!match) throw new Error(`Invalid interval format: ${interval}`)
    const value = parseInt(match[1])
    const unit = match[2]
    return unit === 's' ? value * 1000 : value * 60 * 1000
  }

  _startInterval(key) {
    const entry = this._intervals.get(key)
    if (entry.id !== null || this._paused) return

    entry.id = setInterval(() => {
      for (const callback of entry.callbacks) {
        try { callback() } catch (e) { console.error('[polling-scheduler] Callback error:', e) }
      }
    }, entry.ms)

    console.log(`[polling-scheduler] Started ${key} interval with ${entry.callbacks.size} subscribers`)
  }

  _stopInterval(key) {
    const entry = this._intervals.get(key)
    if (!entry || entry.id === null) return

    clearInterval(entry.id)
    entry.id = null
    console.log(`[polling-scheduler] Stopped ${key} interval`)
  }

  _pause() {
    if (this._paused) return
    this._paused = true

    for (const key of this._intervals.keys()) {
      this._stopInterval(key)
    }
    console.log('[polling-scheduler] Paused (page hidden)')
  }

  _resume() {
    if (!this._paused) return
    this._paused = false

    for (const [key, entry] of this._intervals.entries()) {
      if (entry.callbacks.size > 0) {
        // Call immediately on resume, then start interval
        for (const callback of entry.callbacks) {
          try { callback() } catch (e) { console.error('[polling-scheduler] Callback error:', e) }
        }
        this._startInterval(key)
      }
    }
    console.log('[polling-scheduler] Resumed (page visible)')
  }

  /**
   * Get current status for debugging
   */
  getStatus() {
    const status = {
      paused: this._paused,
      intervals: {}
    }
    for (const [key, entry] of this._intervals.entries()) {
      status.intervals[key] = {
        active: entry.id !== null,
        subscribers: entry.callbacks.size
      }
    }
    return status
  }
}

// Singleton instance
const pollingScheduler = new PollingScheduler()

export default pollingScheduler
