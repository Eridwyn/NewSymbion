import { describe, it, expect, vi, beforeEach } from 'vitest'

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// ── Mock mqtt library ────────────────────────────────────────────────

const mockMqttClient = {
  on: vi.fn(),
  subscribe: vi.fn((topic, cb) => cb && cb(null)),
  unsubscribe: vi.fn((topic, cb) => cb && cb(null)),
  publish: vi.fn(),
  end: vi.fn(),
  connected: true
}

vi.mock('mqtt', () => ({
  default: {
    connect: vi.fn(() => mockMqttClient)
  }
}))

const { MqttService } = await import('./mqtt-service.js')

// ── Helpers ──────────────────────────────────────────────────────────

function createService() {
  const svc = new MqttService()
  svc.client = mockMqttClient
  svc.status = 'online'
  svc.agentsCache = {}
  return svc
}

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  mockMqttClient.on.mockReset()
  mockMqttClient.subscribe.mockReset()
  mockMqttClient.subscribe.mockImplementation((topic, cb) => cb && cb(null))
  mockMqttClient.unsubscribe.mockReset()
  mockMqttClient.publish.mockReset()
  mockMqttClient.end.mockReset()
  mockMqttClient.connected = true
})

// =====================================================================
// handleConnect()
// =====================================================================
describe('handleConnect()', () => {
  it('sets status to online', () => {
    const svc = createService()
    svc.status = 'connecting'

    svc.handleConnect()

    expect(svc.status).toBe('online')
  })

  it('resets reconnectAttempts to 0', () => {
    const svc = createService()
    svc.reconnectAttempts = 3

    svc.handleConnect()

    expect(svc.reconnectAttempts).toBe(0)
  })

  it('dispatches status-change event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('status-change', handler)

    svc.handleConnect()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.status).toBe('online')

    svc.removeEventListener('status-change', handler)
  })

  it('subscribes to MQTT topics', () => {
    const svc = createService()

    svc.handleConnect()

    expect(mockMqttClient.subscribe).toHaveBeenCalled()
    // Should subscribe to multiple topics
    expect(mockMqttClient.subscribe.mock.calls.length).toBeGreaterThan(5)
  })
})

// =====================================================================
// handleMessage()
// =====================================================================
describe('handleMessage()', () => {
  it('parses JSON and routes the message', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('system-health', handler)

    svc.handleMessage(
      'symbion/kernel/health@v1',
      Buffer.from(JSON.stringify({ status: 'ok', uptime: 1000 }))
    )

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.health.status).toBe('ok')

    svc.removeEventListener('system-health', handler)
  })

  it('handles invalid JSON gracefully', () => {
    const svc = createService()

    // Should not throw
    svc.handleMessage('symbion/kernel/health@v1', Buffer.from('not-json{'))
  })
})

// =====================================================================
// handleError() / handleClose() / handleOffline()
// =====================================================================
describe('error handlers', () => {
  it('handleError sets status to offline', () => {
    const svc = createService()

    svc.handleError(new Error('connection lost'))

    expect(svc.status).toBe('offline')
  })

  it('handleClose sets status to offline', () => {
    const svc = createService()

    svc.handleClose()

    expect(svc.status).toBe('offline')
  })

  it('handleOffline sets status to offline', () => {
    const svc = createService()

    svc.handleOffline()

    expect(svc.status).toBe('offline')
  })
})

// =====================================================================
// handleReconnect()
// =====================================================================
describe('handleReconnect()', () => {
  it('increments reconnectAttempts', () => {
    const svc = createService()
    svc.reconnectAttempts = 0

    svc.handleReconnect()

    expect(svc.reconnectAttempts).toBe(1)
  })

  it('sets status to connecting', () => {
    const svc = createService()

    svc.handleReconnect()

    expect(svc.status).toBe('connecting')
  })

  it('ends client and sets offline at max attempts', () => {
    const svc = createService()
    svc.reconnectAttempts = svc.maxReconnectAttempts - 1

    svc.handleReconnect()

    expect(mockMqttClient.end).toHaveBeenCalled()
    expect(svc.status).toBe('offline')
  })
})

// =====================================================================
// routeMessage()
// =====================================================================
describe('routeMessage()', () => {
  it('routes kernel health to system-health event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('system-health', handler)

    svc.routeMessage('symbion/kernel/health@v1', { status: 'ok' })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('system-health', handler)
  })

  it('routes host heartbeat to host-heartbeat event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('host-heartbeat', handler)

    svc.routeMessage('symbion/hosts/heartbeat@v2', { agent_id: 'pc-salon' })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('host-heartbeat', handler)
  })

  it('routes dashboard context to dashboard-context event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-context', handler)

    svc.routeMessage('symbion/dashboard/context@v1', { mode: 'pro' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.context.mode).toBe('pro')

    svc.removeEventListener('dashboard-context', handler)
  })

  it('routes dashboard health to dashboard-health event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-health', handler)

    svc.routeMessage('symbion/dashboard/health@v1', { agents: 2 })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('dashboard-health', handler)
  })

  it('routes dashboard notes to dashboard-note-created event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-note-created', handler)

    svc.routeMessage('symbion/dashboard/notes@v1', { id: 'note-1' })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('dashboard-note-created', handler)
  })

  it('routes dashboard stats to dashboard-stats event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-stats', handler)

    svc.routeMessage('symbion/dashboard/stats@v1', { total: 42 })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('dashboard-stats', handler)
  })

  it('routes dashboard pattern to dashboard-pattern event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-pattern', handler)

    svc.routeMessage('symbion/dashboard/pattern@v1', { pattern: 'work' })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('dashboard-pattern', handler)
  })

  it('routes notification to notification-received event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('notification-received', handler)

    svc.routeMessage('symbion/notifications/sent@v1', {
      notification: { id: 'n1', title: 'test' }
    })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.notification.title).toBe('test')

    svc.removeEventListener('notification-received', handler)
  })

  it('routes wake command to wake-command event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('wake-command', handler)

    svc.routeMessage('symbion/hosts/wake@v1', { host_id: 'pc-salon' })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('wake-command', handler)
  })

  it('routes notes response to notes-response event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('notes-response', handler)

    svc.routeMessage('symbion/notes/response@v1', { notes: [] })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('notes-response', handler)
  })

  it('routes freebox presence topics', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('freebox-presence', handler)

    svc.routeMessage('symbion/freebox/presence/phone-mark', { online: true })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.payload.online).toBe(true)

    svc.removeEventListener('freebox-presence', handler)
  })

  it('routes freebox connection topic', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('freebox-connection', handler)

    svc.routeMessage('symbion/freebox/connection/metrics', { rate_down: 100 })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('freebox-connection', handler)
  })

  it('routes ssl summary topic', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('ssl-summary', handler)

    svc.routeMessage('symbion/ssl/summary', { total: 3, valid: 2 })

    expect(handler).toHaveBeenCalledTimes(1)

    svc.removeEventListener('ssl-summary', handler)
  })

  it('routes ssl domain topics', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('ssl-domain', handler)

    svc.routeMessage('symbion/ssl/example.com', { valid: true, days_left: 30 })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.domainId).toBe('example.com')

    svc.removeEventListener('ssl-domain', handler)
  })
})

// =====================================================================
// handleIndividualAgent()
// =====================================================================
describe('handleIndividualAgent()', () => {
  it('stores agent in cache', () => {
    const svc = createService()

    svc.handleIndividualAgent('pc-salon', { agent_id: 'pc-salon', os: 'linux', status: 'online' })

    expect(svc.agentsCache['pc-salon']).toBeDefined()
    expect(svc.agentsCache['pc-salon'].os).toBe('linux')
  })

  it('adds _lastSeen timestamp', () => {
    const svc = createService()
    const before = Date.now()

    svc.handleIndividualAgent('pc-salon', { agent_id: 'pc-salon' })

    expect(svc.agentsCache['pc-salon']._lastSeen).toBeGreaterThanOrEqual(before)
  })

  it('dispatches dashboard-agents event with all cached agents', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-agents', handler)

    svc.handleIndividualAgent('pc-salon', { agent_id: 'pc-salon', os: 'linux' })
    svc.handleIndividualAgent('pc-bureau', { agent_id: 'pc-bureau', os: 'windows' })

    expect(handler).toHaveBeenCalledTimes(2)
    const lastAgents = handler.mock.calls[1][0].detail.agents
    expect(lastAgents).toHaveLength(2)

    svc.removeEventListener('dashboard-agents', handler)
  })

  it('updates existing agent in cache', () => {
    const svc = createService()

    svc.handleIndividualAgent('pc-salon', { agent_id: 'pc-salon', status: 'online' })
    svc.handleIndividualAgent('pc-salon', { agent_id: 'pc-salon', status: 'offline' })

    expect(svc.agentsCache['pc-salon'].status).toBe('offline')
    expect(Object.keys(svc.agentsCache)).toHaveLength(1)
  })

  it('evicts oldest entry when cache exceeds 50', () => {
    const svc = createService()

    // Fill 50 agents
    for (let i = 0; i < 50; i++) {
      svc.agentsCache[`agent-${i}`] = { _lastSeen: 1000 + i }
    }

    // 51st agent should evict the oldest (agent-0 with _lastSeen=1000)
    svc.handleIndividualAgent('new-agent', { agent_id: 'new-agent' })

    expect(Object.keys(svc.agentsCache)).toHaveLength(50)
    expect(svc.agentsCache['agent-0']).toBeUndefined()
    expect(svc.agentsCache['new-agent']).toBeDefined()
  })

  it('routes individual agent topic via routeMessage', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('dashboard-agents', handler)

    svc.routeMessage('symbion/dashboard/agents/pc-salon@v1', {
      agent_id: 'pc-salon',
      os: 'linux'
    })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(svc.agentsCache['pc-salon'].os).toBe('linux')

    svc.removeEventListener('dashboard-agents', handler)
  })
})

// =====================================================================
// Caching (Freebox, SSL)
// =====================================================================
describe('caching', () => {
  it('caches freebox presence data', () => {
    const svc = createService()

    svc.handleFreeboxPresence('symbion/freebox/presence/phone', { online: true })

    expect(svc._freeboxPresenceCache['symbion/freebox/presence/phone']).toEqual({ online: true })
  })

  it('caches freebox connection data', () => {
    const svc = createService()

    svc.handleFreeboxConnection({ rate_down: 500 })

    expect(svc._freeboxConnectionCache).toEqual({ rate_down: 500 })
  })

  it('getFreeboxCache returns cached data', () => {
    const svc = createService()
    svc.handleFreeboxPresence('symbion/freebox/presence/phone', { online: true })
    svc.handleFreeboxConnection({ rate_down: 500 })

    const cache = svc.getFreeboxCache()

    expect(cache.presence['symbion/freebox/presence/phone']).toEqual({ online: true })
    expect(cache.connection).toEqual({ rate_down: 500 })
  })

  it('getFreeboxCache returns empty when nothing cached', () => {
    const svc = createService()

    const cache = svc.getFreeboxCache()

    expect(cache.presence).toEqual({})
    expect(cache.connection).toBeNull()
  })

  it('caches SSL summary data', () => {
    const svc = createService()

    svc.handleSslSummary({ total: 3, valid: 2 })

    expect(svc._sslSummaryCache).toEqual({ total: 3, valid: 2 })
  })

  it('caches SSL domain data', () => {
    const svc = createService()

    svc.handleSslDomain('symbion/ssl/example.com', { valid: true })

    expect(svc._sslDomainsCache['example.com']).toEqual({ valid: true })
  })

  it('getSslCache returns cached data', () => {
    const svc = createService()
    svc.handleSslSummary({ total: 1 })
    svc.handleSslDomain('symbion/ssl/test.com', { days_left: 10 })

    const cache = svc.getSslCache()

    expect(cache.summary).toEqual({ total: 1 })
    expect(cache.domains['test.com']).toEqual({ days_left: 10 })
  })

  it('getSslCache returns empty when nothing cached', () => {
    const svc = createService()

    const cache = svc.getSslCache()

    expect(cache.summary).toBeNull()
    expect(cache.domains).toEqual({})
  })
})

// =====================================================================
// publish()
// =====================================================================
describe('publish()', () => {
  it('publishes JSON payload when connected', () => {
    const svc = createService()

    svc.publish('symbion/test', { key: 'value' })

    expect(mockMqttClient.publish).toHaveBeenCalledTimes(1)
    expect(mockMqttClient.publish.mock.calls[0][0]).toBe('symbion/test')
    expect(mockMqttClient.publish.mock.calls[0][1]).toBe('{"key":"value"}')
  })

  it('publishes string payload as-is', () => {
    const svc = createService()

    svc.publish('symbion/test', 'raw-message')

    expect(mockMqttClient.publish.mock.calls[0][1]).toBe('raw-message')
  })

  it('does not publish when not connected', () => {
    const svc = createService()
    svc.status = 'offline'

    svc.publish('symbion/test', { key: 'value' })

    expect(mockMqttClient.publish).not.toHaveBeenCalled()
  })

  it('does not publish when client is null', () => {
    const svc = createService()
    svc.client = null

    svc.publish('symbion/test', { key: 'value' })

    // Should not throw
    expect(mockMqttClient.publish).not.toHaveBeenCalled()
  })
})

// =====================================================================
// subscribe()
// =====================================================================
describe('subscribe()', () => {
  it('subscribes to topic via mqtt client', () => {
    const svc = createService()

    svc.subscribe('symbion/custom/topic')

    expect(mockMqttClient.subscribe).toHaveBeenCalledTimes(1)
    expect(mockMqttClient.subscribe.mock.calls[0][0]).toBe('symbion/custom/topic')
  })

  it('calls callback on successful subscribe', () => {
    const svc = createService()
    const callback = vi.fn()

    svc.subscribe('symbion/custom/topic', callback)

    expect(callback).toHaveBeenCalledTimes(1)
  })

  it('does not crash when client is null', () => {
    const svc = createService()
    svc.client = null

    // Should not throw
    svc.subscribe('symbion/test')
  })
})

// =====================================================================
// isConnected()
// =====================================================================
describe('isConnected()', () => {
  it('returns true when status is online', () => {
    const svc = createService()
    svc.status = 'online'

    expect(svc.isConnected()).toBe(true)
  })

  it('returns false when status is offline', () => {
    const svc = createService()
    svc.status = 'offline'

    expect(svc.isConnected()).toBe(false)
  })

  it('returns false when status is connecting', () => {
    const svc = createService()
    svc.status = 'connecting'

    expect(svc.isConnected()).toBe(false)
  })
})

// =====================================================================
// handleNotificationReceived()
// =====================================================================
describe('handleNotificationReceived()', () => {
  it('extracts notification from payload.notification', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('notification-received', handler)

    svc.handleNotificationReceived({
      notification: { id: 'n1', title: 'Alert' },
      timestamp: Date.now()
    })

    expect(handler.mock.calls[0][0].detail.notification.title).toBe('Alert')

    svc.removeEventListener('notification-received', handler)
  })

  it('uses payload directly if no notification wrapper', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('notification-received', handler)

    svc.handleNotificationReceived({ id: 'n2', title: 'Direct' })

    expect(handler.mock.calls[0][0].detail.notification.title).toBe('Direct')

    svc.removeEventListener('notification-received', handler)
  })
})

// =====================================================================
// constructor defaults
// =====================================================================
describe('constructor defaults', () => {
  it('starts with connecting status', () => {
    const svc = new MqttService()
    expect(svc.status).toBe('connecting')
  })

  it('starts with null client', () => {
    const svc = new MqttService()
    expect(svc.client).toBeNull()
  })

  it('starts with 0 reconnectAttempts', () => {
    const svc = new MqttService()
    expect(svc.reconnectAttempts).toBe(0)
  })

  it('has maxReconnectAttempts of 5', () => {
    const svc = new MqttService()
    expect(svc.maxReconnectAttempts).toBe(5)
  })
})
