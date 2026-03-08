import { describe, it, expect, vi, beforeEach } from 'vitest'

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// ── Mock fetch for csrf-dependent calls ──────────────────────────────
const mockFetch = vi.fn()
global.fetch = mockFetch

// ── Mock dependencies ────────────────────────────────────────────────
const mockApiService = {
  request: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn()
}

vi.mock('./api-service.js', () => ({
  ApiService: class {}
}))

vi.mock('./csrf-service.js', () => ({
  default: {
    fetchWithCsrf: vi.fn(() => Promise.resolve({
      ok: true,
      json: async () => ({ success: true }),
      text: async () => 'ok'
    }))
  }
}))

vi.mock('./auth-service.js', () => ({
  default: {
    isAuthenticated: vi.fn(() => false),
    getToken: vi.fn(() => null)
  }
}))

vi.mock('./config.js', () => ({
  getApiBase: () => ''
}))

const { AgentsService } = await import('./agents-service.js')
const { default: csrfService } = await import('./csrf-service.js')

// ── Helpers ──────────────────────────────────────────────────────────

function createService(agents = []) {
  const svc = new AgentsService()
  svc.apiService = mockApiService
  svc.agents = [...agents]
  return svc
}

const SAMPLE_AGENTS = [
  {
    agent_id: 'pc-salon',
    status: 'online',
    os: 'Linux',
    primary_ip: '192.168.1.10',
    capabilities: ['power_management', 'process_control', 'command_execution', 'service_management'],
    last_seen: new Date().toISOString()
  },
  {
    agent_id: 'pc-bureau',
    status: 'online',
    os: 'Windows',
    primary_ip: '192.168.1.20',
    capabilities: ['power_management', 'command_execution'],
    last_seen: new Date().toISOString()
  },
  {
    agent_id: 'phone-mark',
    status: 'offline',
    os: 'Android',
    primary_ip: null,
    capabilities: [],
    last_seen: new Date(Date.now() - 86400000).toISOString() // 1 day ago
  }
]

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  mockApiService.request.mockReset()
  mockFetch.mockReset()
  csrfService.fetchWithCsrf.mockReset()
  csrfService.fetchWithCsrf.mockResolvedValue({
    ok: true,
    json: async () => ({ success: true }),
    text: async () => 'ok'
  })
})

// =====================================================================
// getAgents()
// =====================================================================
describe('getAgents()', () => {
  it('calls apiService.request with /agents', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce(SAMPLE_AGENTS)

    await svc.getAgents()

    expect(mockApiService.request).toHaveBeenCalledWith('/agents')
  })

  it('stores agents in svc.agents', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce(SAMPLE_AGENTS)

    await svc.getAgents()

    expect(svc.agents).toHaveLength(3)
    expect(svc.agents[0].agent_id).toBe('pc-salon')
  })

  it('dispatches agents-updated event', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce(SAMPLE_AGENTS)
    const handler = vi.fn()
    svc.addEventListener('agents-updated', handler)

    await svc.getAgents()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.agents).toHaveLength(3)

    svc.removeEventListener('agents-updated', handler)
  })

  it('returns agents', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce(SAMPLE_AGENTS)

    const result = await svc.getAgents()

    expect(result).toBe(SAMPLE_AGENTS)
  })

  it('throws on API error', async () => {
    const svc = createService()
    mockApiService.request.mockRejectedValueOnce(new Error('Network error'))

    await expect(svc.getAgents()).rejects.toThrow('Network error')
  })
})

// =====================================================================
// getAgent()
// =====================================================================
describe('getAgent()', () => {
  it('calls apiService.request with encoded agent id', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce(SAMPLE_AGENTS[0])

    await svc.getAgent('pc-salon')

    expect(mockApiService.request).toHaveBeenCalledWith('/agents/pc-salon')
  })
})

// =====================================================================
// shutdownAgent()
// =====================================================================
describe('shutdownAgent()', () => {
  it('calls csrfService.fetchWithCsrf with POST and correct URL', async () => {
    const svc = createService()

    await svc.shutdownAgent('pc-salon')

    expect(csrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-salon/shutdown')
    expect(options.method).toBe('POST')
  })

  it('throws on HTTP error', async () => {
    const svc = createService()
    csrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: false,
      status: 503,
      text: async () => 'Agent unavailable'
    })

    await expect(svc.shutdownAgent('pc-salon'))
      .rejects.toThrow('HTTP 503: Agent unavailable')
  })
})

// =====================================================================
// rebootAgent()
// =====================================================================
describe('rebootAgent()', () => {
  it('calls POST to /v1/agents/:id/reboot', async () => {
    const svc = createService()

    await svc.rebootAgent('pc-bureau')

    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-bureau/reboot')
    expect(options.method).toBe('POST')
  })
})

// =====================================================================
// hibernateAgent()
// =====================================================================
describe('hibernateAgent()', () => {
  it('calls POST to /v1/agents/:id/hibernate', async () => {
    const svc = createService()

    await svc.hibernateAgent('pc-salon')

    const [url] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-salon/hibernate')
  })
})

// =====================================================================
// deleteAgent()
// =====================================================================
describe('deleteAgent()', () => {
  it('calls DELETE to /v1/agents/:id', async () => {
    const svc = createService()

    await svc.deleteAgent('phone-mark')

    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/phone-mark')
    expect(options.method).toBe('DELETE')
  })

  it('returns true on success', async () => {
    const svc = createService()

    const result = await svc.deleteAgent('phone-mark')

    expect(result).toBe(true)
  })

  it('throws on HTTP error', async () => {
    const svc = createService()
    csrfService.fetchWithCsrf.mockResolvedValueOnce({
      ok: false,
      status: 404,
      text: async () => 'Agent not found'
    })

    await expect(svc.deleteAgent('unknown'))
      .rejects.toThrow('HTTP 404: Agent not found')
  })
})

// =====================================================================
// getAgentProcesses()
// =====================================================================
describe('getAgentProcesses()', () => {
  it('calls apiService.request with processes endpoint', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce([])

    await svc.getAgentProcesses('pc-salon')

    expect(mockApiService.request).toHaveBeenCalledWith('/agents/pc-salon/processes')
  })
})

// =====================================================================
// killAgentProcess()
// =====================================================================
describe('killAgentProcess()', () => {
  it('calls POST to /v1/agents/:id/processes/:pid/kill', async () => {
    const svc = createService()

    await svc.killAgentProcess('pc-salon', 1234)

    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-salon/processes/1234/kill')
    expect(options.method).toBe('POST')
  })
})

// =====================================================================
// executeCommand()
// =====================================================================
describe('executeCommand()', () => {
  it('calls POST via csrfService with command', async () => {
    const svc = createService()

    await svc.executeCommand('pc-salon', 'ls -la', 60)

    expect(csrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-salon/command')
    expect(options.method).toBe('POST')
    const body = JSON.parse(options.body)
    expect(body.command).toBe('ls -la')
    expect(body.timeout_secs).toBe(60)
  })

  it('uses default timeout of 30', async () => {
    const svc = createService()

    await svc.executeCommand('pc-salon', 'whoami')

    const body = JSON.parse(csrfService.fetchWithCsrf.mock.calls[0][1].body)
    expect(body.timeout_secs).toBe(30)
  })
})

// =====================================================================
// executeCommandWithTracking()
// =====================================================================
describe('executeCommandWithTracking()', () => {
  it('calls POST to /v1/agents/:id/commands via csrfService', async () => {
    const svc = createService()

    await svc.executeCommandWithTracking('pc-salon', 'uptime', 30)

    expect(csrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/agents/pc-salon/commands')
    expect(options.method).toBe('POST')
    const body = JSON.parse(options.body)
    expect(body.command_type).toBe('shell_command')
    expect(body.parameters.command).toBe('uptime')
  })
})

// =====================================================================
// getCommandStatus() / cancelCommand()
// =====================================================================
describe('command tracking', () => {
  it('getCommandStatus calls correct endpoint via fetch', async () => {
    const svc = createService()
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ status: 'completed' })
    })

    await svc.getCommandStatus('cmd-1')

    expect(mockFetch).toHaveBeenCalledTimes(1)
    const [url] = mockFetch.mock.calls[0]
    expect(url).toContain('/v1/commands/cmd-1/status')
  })

  it('cancelCommand calls POST via csrfService', async () => {
    const svc = createService()

    await svc.cancelCommand('cmd-1')

    expect(csrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toContain('/v1/commands/cmd-1/cancel')
    expect(options.method).toBe('POST')
  })
})

// =====================================================================
// getAgentMetrics()
// =====================================================================
describe('getAgentMetrics()', () => {
  it('calls correct metrics endpoint', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce({ cpu: 25 })

    await svc.getAgentMetrics('pc-salon')

    expect(mockApiService.request).toHaveBeenCalledWith('/agents/pc-salon/metrics')
  })
})

// =====================================================================
// reconnectAgent()
// =====================================================================
describe('reconnectAgent()', () => {
  it('calls POST to /v1/agents/:id/reconnect', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce({ reconnecting: true })

    await svc.reconnectAgent('pc-salon')

    const [url, options] = mockApiService.request.mock.calls[0]
    expect(url).toBe('/v1/agents/pc-salon/reconnect')
    expect(options.method).toBe('POST')
  })
})

// =====================================================================
// wakeAgent()
// =====================================================================
describe('wakeAgent()', () => {
  it('calls POST to /v1/wake with host_id', async () => {
    const svc = createService()
    mockApiService.request.mockResolvedValueOnce({ sent: true })

    await svc.wakeAgent('pc-salon')

    const [url, options] = mockApiService.request.mock.calls[0]
    expect(url).toBe('/v1/wake?host_id=pc-salon')
    expect(options.method).toBe('POST')
  })
})

// =====================================================================
// Helper methods (pure logic, no API calls)
// =====================================================================
describe('getAgentById()', () => {
  it('finds agent by agent_id', () => {
    const svc = createService(SAMPLE_AGENTS)

    const agent = svc.getAgentById('pc-bureau')

    expect(agent.agent_id).toBe('pc-bureau')
    expect(agent.os).toBe('Windows')
  })

  it('returns undefined for unknown id', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.getAgentById('nonexistent')).toBeUndefined()
  })
})

describe('getOnlineAgents()', () => {
  it('returns only online agents', () => {
    const svc = createService(SAMPLE_AGENTS)

    const online = svc.getOnlineAgents()

    expect(online).toHaveLength(2)
    expect(online.every(a => a.status === 'online')).toBe(true)
  })
})

describe('getOfflineAgents()', () => {
  it('returns only offline agents', () => {
    const svc = createService(SAMPLE_AGENTS)

    const offline = svc.getOfflineAgents()

    expect(offline).toHaveLength(1)
    expect(offline[0].agent_id).toBe('phone-mark')
  })
})

describe('getAgentsByOS()', () => {
  it('filters by OS (case-insensitive)', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.getAgentsByOS('linux')).toHaveLength(1)
    expect(svc.getAgentsByOS('LINUX')).toHaveLength(1)
    expect(svc.getAgentsByOS('windows')).toHaveLength(1)
    expect(svc.getAgentsByOS('android')).toHaveLength(1)
    expect(svc.getAgentsByOS('macos')).toHaveLength(0)
  })
})

describe('isAgentOnline()', () => {
  it('returns true for online agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.isAgentOnline('pc-salon')).toBe(true)
  })

  it('returns false for offline agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.isAgentOnline('phone-mark')).toBe(false)
  })

  it('returns false for unknown agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.isAgentOnline('unknown')).toBeFalsy()
  })
})

describe('hasCapability()', () => {
  it('returns true when agent has capability', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasCapability('pc-salon', 'power_management')).toBe(true)
    expect(svc.hasCapability('pc-salon', 'process_control')).toBe(true)
  })

  it('returns false when agent lacks capability', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasCapability('pc-bureau', 'process_control')).toBe(false)
  })

  it('returns false for unknown agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasCapability('unknown', 'power_management')).toBeFalsy()
  })
})

describe('canExecutePowerCommands()', () => {
  it('delegates to hasCapability power_management', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.canExecutePowerCommands('pc-salon')).toBe(true)
    expect(svc.canExecutePowerCommands('phone-mark')).toBe(false)
  })
})

describe('canControlProcesses()', () => {
  it('delegates to hasCapability process_control', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.canControlProcesses('pc-salon')).toBe(true)
    expect(svc.canControlProcesses('pc-bureau')).toBe(false)
  })
})

describe('canExecuteCommands()', () => {
  it('delegates to hasCapability command_execution', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.canExecuteCommands('pc-salon')).toBe(true)
    expect(svc.canExecuteCommands('phone-mark')).toBe(false)
  })
})

describe('canManageServices()', () => {
  it('delegates to hasCapability service_management', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.canManageServices('pc-salon')).toBe(true)
    expect(svc.canManageServices('pc-bureau')).toBe(false)
  })
})

describe('hasLocalDashboard()', () => {
  it('returns true for online agent with primary_ip', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasLocalDashboard('pc-salon')).toBeTruthy()
  })

  it('returns false for offline agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasLocalDashboard('phone-mark')).toBeFalsy()
  })

  it('returns false for unknown agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.hasLocalDashboard('unknown')).toBeFalsy()
  })
})

describe('getAgentIP()', () => {
  it('returns primary_ip for known agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.getAgentIP('pc-salon')).toBe('192.168.1.10')
  })

  it('returns null for unknown agent', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.getAgentIP('unknown')).toBeNull()
  })

  it('returns null for agent without IP', () => {
    const svc = createService(SAMPLE_AGENTS)

    expect(svc.getAgentIP('phone-mark')).toBeNull()
  })
})

describe('getAgentLocalDashboardURL()', () => {
  it('builds URL with IP and default port', () => {
    const svc = createService()

    const url = svc.getAgentLocalDashboardURL('192.168.1.10')

    expect(url).toBe('http://192.168.1.10:9899/')
  })

  it('uses custom port', () => {
    const svc = createService()

    const url = svc.getAgentLocalDashboardURL('192.168.1.10', 8080)

    expect(url).toBe('http://192.168.1.10:8080/')
  })
})

// =====================================================================
// formatLastSeen()
// =====================================================================
describe('formatLastSeen()', () => {
  const svc = createService()

  it('returns "Never" when no last_seen', () => {
    expect(svc.formatLastSeen({})).toBe('Never')
    expect(svc.formatLastSeen({ last_seen: null })).toBe('Never')
  })

  it('returns "Just now" for < 1 minute ago', () => {
    const agent = { last_seen: new Date().toISOString() }
    expect(svc.formatLastSeen(agent)).toBe('Just now')
  })

  it('returns minutes for < 1 hour', () => {
    const agent = { last_seen: new Date(Date.now() - 15 * 60 * 1000).toISOString() }
    expect(svc.formatLastSeen(agent)).toBe('15m ago')
  })

  it('returns hours for < 24 hours', () => {
    const agent = { last_seen: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString() }
    expect(svc.formatLastSeen(agent)).toBe('3h ago')
  })

  it('returns days for >= 24 hours', () => {
    const agent = { last_seen: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString() }
    expect(svc.formatLastSeen(agent)).toBe('2d ago')
  })
})

// =====================================================================
// getOSIcon()
// =====================================================================
describe('getOSIcon()', () => {
  const svc = createService()

  it('returns penguin for linux', () => {
    expect(svc.getOSIcon('linux')).toBe('🐧')
    expect(svc.getOSIcon('Linux')).toBe('🐧')
  })

  it('returns window for windows', () => {
    expect(svc.getOSIcon('windows')).toBe('🪟')
    expect(svc.getOSIcon('Windows')).toBe('🪟')
  })

  it('returns robot for android', () => {
    expect(svc.getOSIcon('android')).toBe('🤖')
    expect(svc.getOSIcon('Android')).toBe('🤖')
  })

  it('returns apple for macos', () => {
    expect(svc.getOSIcon('macos')).toBe('🍎')
    expect(svc.getOSIcon('MacOS')).toBe('🍎')
  })

  it('returns computer for unknown OS', () => {
    expect(svc.getOSIcon('freebsd')).toBe('💻')
  })
})

// =====================================================================
// getStatusColor()
// =====================================================================
describe('getStatusColor()', () => {
  const svc = createService()

  it('returns green for online', () => {
    expect(svc.getStatusColor({ status: 'online' })).toBe('#22c55e')
  })

  it('returns red for offline', () => {
    expect(svc.getStatusColor({ status: 'offline' })).toBe('#ef4444')
  })

  it('returns amber for unknown', () => {
    expect(svc.getStatusColor({ status: 'unknown' })).toBe('#f59e0b')
  })

  it('returns gray for other status', () => {
    expect(svc.getStatusColor({ status: 'maintenance' })).toBe('#6b7280')
  })

  it('returns dark gray for null agent', () => {
    expect(svc.getStatusColor(null)).toBe('#666')
  })
})
