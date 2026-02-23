import { describe, it, expect, vi, beforeEach } from 'vitest'
import automationsService from './automations-service.js'

const mockApiService = {
  request: vi.fn()
}

const mockCsrfService = {
  fetchWithCsrf: vi.fn()
}

beforeEach(() => {
  automationsService.apiService = mockApiService
  automationsService.csrfService = mockCsrfService
  automationsService.schema = null
  automationsService.automations = []
  automationsService.status = 'idle'
  mockApiService.request.mockReset()
  mockCsrfService.fetchWithCsrf.mockReset()
})

describe('init', () => {
  it('stores apiService and csrfService references', () => {
    const api = { request: vi.fn() }
    const csrf = { fetchWithCsrf: vi.fn() }
    automationsService.apiService = null
    automationsService.csrfService = null

    automationsService.init(api, csrf)

    expect(automationsService.apiService).toBe(api)
    expect(automationsService.csrfService).toBe(csrf)
  })
})

describe('fetchAutomations', () => {
  it('returns automations array from API response', async () => {
    const automations = [{ id: 'a1', name: 'Test' }]
    mockApiService.request.mockResolvedValue({ automations, count: 1 })

    const result = await automationsService.fetchAutomations()

    expect(result).toEqual(automations)
  })

  it('extracts automations from response.automations', async () => {
    const automations = [{ id: 'a1' }, { id: 'a2' }]
    mockApiService.request.mockResolvedValue({ automations, count: 2 })

    await automationsService.fetchAutomations()

    expect(automationsService.automations).toEqual(automations)
    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations')
  })

  it('sets status to ready on success', async () => {
    mockApiService.request.mockResolvedValue({ automations: [] })

    await automationsService.fetchAutomations()

    expect(automationsService.status).toBe('ready')
  })

  it('dispatches automations:loaded event', async () => {
    const automations = [{ id: 'a1' }]
    mockApiService.request.mockResolvedValue({ automations, count: 1 })
    const handler = vi.fn()
    automationsService.addEventListener('automations:loaded', handler)

    await automationsService.fetchAutomations()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ automations, count: 1 })

    automationsService.removeEventListener('automations:loaded', handler)
  })

  it('returns empty array on error', async () => {
    mockApiService.request.mockRejectedValue(new Error('Network error'))

    const result = await automationsService.fetchAutomations()

    expect(result).toEqual([])
  })

  it('sets status to error on failure', async () => {
    mockApiService.request.mockRejectedValue(new Error('fail'))

    await automationsService.fetchAutomations()

    expect(automationsService.status).toBe('error')
  })

  it('dispatches automations:error event on failure', async () => {
    mockApiService.request.mockRejectedValue(new Error('Network error'))
    const handler = vi.fn()
    automationsService.addEventListener('automations:error', handler)

    await automationsService.fetchAutomations()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ error: 'Network error' })

    automationsService.removeEventListener('automations:error', handler)
  })

  it('waits for apiService initialization (up to 2s)', async () => {
    automationsService.apiService = null

    // Set apiService after 300ms
    setTimeout(() => {
      automationsService.apiService = mockApiService
    }, 300)
    mockApiService.request.mockResolvedValue({ automations: [{ id: 'delayed' }] })

    const result = await automationsService.fetchAutomations()

    expect(result).toEqual([{ id: 'delayed' }])
  })

  it('returns empty array if apiService never initialized', async () => {
    automationsService.apiService = null

    // Use vi.useFakeTimers to speed up the 2s timeout
    vi.useFakeTimers()
    const promise = automationsService.fetchAutomations()
    // Advance past all 20 iterations of 100ms each
    for (let i = 0; i < 20; i++) {
      await vi.advanceTimersByTimeAsync(100)
    }
    const result = await promise
    vi.useRealTimers()

    expect(result).toEqual([])
  })
})

describe('getAutomation', () => {
  it('returns automation by ID via apiService', async () => {
    const automation = { id: 'auto-1', name: 'Test Automation' }
    mockApiService.request.mockResolvedValue(automation)

    const result = await automationsService.getAutomation('auto-1')

    expect(result).toEqual(automation)
    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations/auto-1')
  })

  it('returns null if apiService not set', async () => {
    automationsService.apiService = null

    const result = await automationsService.getAutomation('auto-1')

    expect(result).toBeNull()
  })

  it('returns null on error', async () => {
    mockApiService.request.mockRejectedValue(new Error('Not found'))

    const result = await automationsService.getAutomation('auto-1')

    expect(result).toBeNull()
  })
})

describe('fetchSchema', () => {
  it('loads schema via apiService', async () => {
    const schema = {
      triggers: [{ type: 'mode_change', label: 'Mode Change' }],
      conditions: [{ type: 'time_range', label: 'Time Range' }],
      actions: [{ type: 'notify', label: 'Send Notification' }]
    }
    mockApiService.request.mockResolvedValue(schema)

    const result = await automationsService.fetchSchema()

    expect(result).toEqual(schema)
    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations/schema')
  })

  it('stores schema locally', async () => {
    const schema = { triggers: [], conditions: [], actions: [] }
    mockApiService.request.mockResolvedValue(schema)

    await automationsService.fetchSchema()

    expect(automationsService.schema).toEqual(schema)
  })

  it('returns null if apiService not set', async () => {
    automationsService.apiService = null

    const result = await automationsService.fetchSchema()

    expect(result).toBeNull()
  })
})

describe('fetchHistory', () => {
  it('loads history with default limit 50', async () => {
    const history = [{ id: 'h1', timestamp: '2026-01-01T00:00:00Z' }]
    mockApiService.request.mockResolvedValue(history)

    const result = await automationsService.fetchHistory()

    expect(result).toEqual(history)
    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations/history?limit=50')
  })

  it('passes custom limit parameter', async () => {
    mockApiService.request.mockResolvedValue([])

    await automationsService.fetchHistory(10)

    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations/history?limit=10')
  })

  it('returns empty array on error', async () => {
    mockApiService.request.mockRejectedValue(new Error('fail'))

    const result = await automationsService.fetchHistory()

    expect(result).toEqual([])
  })
})

describe('createAutomation', () => {
  it('calls csrfService.fetchWithCsrf with POST', async () => {
    const automation = { name: 'New Auto', trigger: { type: 'mode_change' } }
    const created = { id: 'auto-123', ...automation }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(created)
    })
    // Mock the fetchAutomations refresh call
    mockApiService.request.mockResolvedValue({ automations: [created] })

    await automationsService.createAutomation(automation)

    expect(mockCsrfService.fetchWithCsrf).toHaveBeenCalledWith(
      expect.stringContaining('/v1/automations'),
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(automation)
      })
    )
  })

  it('throws if csrfService not initialized', async () => {
    automationsService.csrfService = null

    await expect(automationsService.createAutomation({ name: 'test' }))
      .rejects.toThrow('CSRF service not initialized')
  })

  it('dispatches automation:created event on success', async () => {
    const created = { id: 'auto-123', name: 'Test' }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(created)
    })
    mockApiService.request.mockResolvedValue({ automations: [created] })

    const handler = vi.fn()
    automationsService.addEventListener('automation:created', handler)

    await automationsService.createAutomation({ name: 'Test' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ automation: created })

    automationsService.removeEventListener('automation:created', handler)
  })

  it('refreshes automation list after creation', async () => {
    const created = { id: 'auto-123', name: 'Test' }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(created)
    })
    mockApiService.request.mockResolvedValue({ automations: [created] })

    await automationsService.createAutomation({ name: 'Test' })

    expect(mockApiService.request).toHaveBeenCalledWith('/v1/automations')
  })

  it('throws on HTTP error with error message', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: false,
      status: 400,
      statusText: 'Bad Request',
      json: () => Promise.resolve({ error: 'Invalid trigger type' })
    })

    await expect(automationsService.createAutomation({ name: 'bad' }))
      .rejects.toThrow('Invalid trigger type')
  })
})

describe('updateAutomation', () => {
  it('calls PUT with automation ID', async () => {
    const updated = { id: 'auto-1', name: 'Updated' }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(updated)
    })
    mockApiService.request.mockResolvedValue({ automations: [updated] })

    await automationsService.updateAutomation('auto-1', { name: 'Updated' })

    expect(mockCsrfService.fetchWithCsrf).toHaveBeenCalledWith(
      expect.stringContaining('/v1/automations/auto-1'),
      expect.objectContaining({
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: 'Updated' })
      })
    )
  })

  it('dispatches automation:updated event', async () => {
    const updated = { id: 'auto-1', name: 'Updated' }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(updated)
    })
    mockApiService.request.mockResolvedValue({ automations: [updated] })

    const handler = vi.fn()
    automationsService.addEventListener('automation:updated', handler)

    await automationsService.updateAutomation('auto-1', { name: 'Updated' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ automation: updated })

    automationsService.removeEventListener('automation:updated', handler)
  })
})

describe('deleteAutomation', () => {
  it('calls DELETE with automation ID', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({})
    })
    mockApiService.request.mockResolvedValue({ automations: [] })

    await automationsService.deleteAutomation('auto-1')

    expect(mockCsrfService.fetchWithCsrf).toHaveBeenCalledWith(
      expect.stringContaining('/v1/automations/auto-1'),
      expect.objectContaining({ method: 'DELETE' })
    )
  })

  it('accepts 204 No Content response', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: false,
      status: 204,
      json: () => Promise.reject(new Error('No body'))
    })
    mockApiService.request.mockResolvedValue({ automations: [] })

    const result = await automationsService.deleteAutomation('auto-1')

    expect(result).toBe(true)
  })

  it('dispatches automation:deleted event', async () => {
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({})
    })
    mockApiService.request.mockResolvedValue({ automations: [] })

    const handler = vi.fn()
    automationsService.addEventListener('automation:deleted', handler)

    await automationsService.deleteAutomation('auto-1')

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ id: 'auto-1' })

    automationsService.removeEventListener('automation:deleted', handler)
  })
})

describe('toggleAutomation', () => {
  it('calls PATCH with enabled boolean', async () => {
    const toggled = { id: 'auto-1', enabled: false }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(toggled)
    })
    mockApiService.request.mockResolvedValue({ automations: [toggled] })

    await automationsService.toggleAutomation('auto-1', false)

    expect(mockCsrfService.fetchWithCsrf).toHaveBeenCalledWith(
      expect.stringContaining('/v1/automations/auto-1/enable'),
      expect.objectContaining({
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false })
      })
    )
  })

  it('dispatches automation:toggled event', async () => {
    const toggled = { id: 'auto-1', enabled: true }
    mockCsrfService.fetchWithCsrf.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(toggled)
    })
    mockApiService.request.mockResolvedValue({ automations: [toggled] })

    const handler = vi.fn()
    automationsService.addEventListener('automation:toggled', handler)

    await automationsService.toggleAutomation('auto-1', true)

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail).toEqual({ automation: toggled })

    automationsService.removeEventListener('automation:toggled', handler)
  })
})

describe('helpers', () => {
  it('getTriggerLabel returns label from schema', () => {
    automationsService.schema = {
      triggers: [
        { type: 'mode_change', label: 'Changement de mode' },
        { type: 'sensor_alert', label: 'Alerte capteur' }
      ]
    }

    expect(automationsService.getTriggerLabel('mode_change')).toBe('Changement de mode')
  })

  it('getTriggerLabel falls back to type if no schema', () => {
    automationsService.schema = null

    expect(automationsService.getTriggerLabel('mode_change')).toBe('mode_change')
  })

  it('getActionLabel returns label from schema', () => {
    automationsService.schema = {
      actions: [
        { type: 'notify', label: 'Envoyer notification' },
        { type: 'set_mode', label: 'Changer mode' }
      ]
    }

    expect(automationsService.getActionLabel('notify')).toBe('Envoyer notification')
  })

  it('formatTrigger handles mode_change with to_mode', () => {
    automationsService.schema = {
      triggers: [{ type: 'mode_change', label: 'Changement de mode' }]
    }

    const result = automationsService.formatTrigger({ type: 'mode_change', to_mode: 'Focus' })

    expect(result).toBe('Changement de mode \u2192 Focus')
  })

  it('formatTrigger handles sensor_alert with room_id', () => {
    automationsService.schema = {
      triggers: [{ type: 'sensor_alert', label: 'Alerte capteur' }]
    }

    const result = automationsService.formatTrigger({ type: 'sensor_alert', room_id: 'salon' })

    expect(result).toBe('Alerte capteur (salon)')
  })

  it('formatTrigger handles null trigger', () => {
    expect(automationsService.formatTrigger(null)).toBe('Aucun')
  })

  it('formatActions joins action labels', () => {
    automationsService.schema = {
      actions: [
        { type: 'notify', label: 'Notification' },
        { type: 'set_mode', label: 'Mode' }
      ]
    }

    const result = automationsService.formatActions([
      { type: 'notify' },
      { type: 'set_mode' }
    ])

    expect(result).toBe('Notification, Mode')
  })

  it('formatActions returns Aucune for empty array', () => {
    expect(automationsService.formatActions([])).toBe('Aucune')
  })
})
