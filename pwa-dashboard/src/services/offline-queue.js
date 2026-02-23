/**
 * Offline Request Queue
 * Stores failed mutation requests (POST/PUT/DELETE) when offline
 * and replays them when connectivity returns.
 */

const STORAGE_KEY = 'symbion-offline-queue'

class OfflineQueueService {
  constructor() {
    this._queue = this._loadQueue()
    this._syncing = false

    // Auto-sync when coming back online
    window.addEventListener('online', () => this.sync())
  }

  /** Queue a failed request for later replay */
  enqueue(url, options) {
    const entry = {
      id: Date.now() + '-' + Math.random().toString(36).substr(2, 6),
      url,
      method: options.method || 'POST',
      body: options.body || null,
      headers: options.headers || {},
      timestamp: Date.now()
    }

    this._queue.push(entry)
    this._saveQueue()
    console.log(`[offline-queue] Queued ${entry.method} ${entry.url} (${this._queue.length} pending)`)

    // Dispatch event for UI notification
    document.body.dispatchEvent(new CustomEvent('offline-queue-change', {
      detail: { pending: this._queue.length }
    }))

    return entry.id
  }

  /** Get pending request count */
  get pendingCount() {
    return this._queue.length
  }

  /** Replay all queued requests */
  async sync() {
    if (this._syncing || this._queue.length === 0) return
    if (!navigator.onLine) return

    this._syncing = true
    console.log(`[offline-queue] Syncing ${this._queue.length} queued requests...`)

    const results = { success: 0, failed: 0 }
    const remaining = []

    for (const entry of this._queue) {
      try {
        const res = await fetch(entry.url, {
          method: entry.method,
          headers: { 'Content-Type': 'application/json', ...entry.headers },
          body: entry.body,
          credentials: 'include'
        })

        if (res.ok || res.status === 409) {
          // 409 = conflict (already applied), treat as success
          results.success++
          console.log(`[offline-queue] Replayed ${entry.method} ${entry.url}: ${res.status}`)
        } else {
          results.failed++
          // Keep for retry if server error (5xx), discard if client error (4xx)
          if (res.status >= 500) {
            remaining.push(entry)
          }
          console.warn(`[offline-queue] Failed ${entry.method} ${entry.url}: ${res.status}`)
        }
      } catch (e) {
        results.failed++
        remaining.push(entry)
        console.warn(`[offline-queue] Network error for ${entry.method} ${entry.url}:`, e.message)
      }
    }

    this._queue = remaining
    this._saveQueue()
    this._syncing = false

    console.log(`[offline-queue] Sync complete: ${results.success} ok, ${results.failed} failed, ${remaining.length} remaining`)

    document.body.dispatchEvent(new CustomEvent('offline-queue-change', {
      detail: { pending: this._queue.length, lastSync: results }
    }))

    return results
  }

  /** Clear the queue */
  clear() {
    this._queue = []
    this._saveQueue()
  }

  _loadQueue() {
    try {
      const data = localStorage.getItem(STORAGE_KEY)
      return data ? JSON.parse(data) : []
    } catch {
      return []
    }
  }

  _saveQueue() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this._queue))
    } catch (e) {
      console.warn('[offline-queue] Failed to persist queue:', e.message)
    }
  }
}

const offlineQueue = new OfflineQueueService()
export default offlineQueue
