import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ── Mocks ────────────────────────────────────────────────────────────

// Mock csrf-service (dynamic import in _notifyKernel)
vi.mock('./csrf-service.js', () => ({
  default: {
    fetchWithCsrf: vi.fn(() => Promise.resolve({ ok: true }))
  }
}))

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Clean state before importing singleton
localStorage.clear()

const { default: themeService } = await import('./theme-service.js')

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  // Reset to default state
  themeService.current = 'dark'
  localStorage.setItem('symbion_theme', 'dark')
  document.documentElement.setAttribute('data-theme', 'dark')
})

afterEach(() => {
  localStorage.clear()
})

// =====================================================================
// constructor
// =====================================================================
describe('constructor', () => {
  it('defaults to dark theme', () => {
    expect(themeService.current).toBe('dark')
  })

  it('applies data-theme attribute on document', () => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})

// =====================================================================
// toggle()
// =====================================================================
describe('toggle()', () => {
  it('switches from dark to light', () => {
    themeService.current = 'dark'

    themeService.toggle()

    expect(themeService.current).toBe('light')
  })

  it('switches from light to dark', () => {
    themeService.current = 'light'

    themeService.toggle()

    expect(themeService.current).toBe('dark')
  })

  it('persists to localStorage', () => {
    themeService.current = 'dark'

    themeService.toggle()

    expect(localStorage.getItem('symbion_theme')).toBe('light')
  })

  it('applies data-theme attribute', () => {
    themeService.current = 'dark'

    themeService.toggle()

    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('dispatches theme-changed event on document.body', () => {
    const handler = vi.fn()
    document.body.addEventListener('theme-changed', handler)

    themeService.toggle()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.theme).toBe('light')

    document.body.removeEventListener('theme-changed', handler)
  })

  it('toggles back and forth', () => {
    themeService.current = 'dark'

    themeService.toggle()
    expect(themeService.current).toBe('light')

    themeService.toggle()
    expect(themeService.current).toBe('dark')

    themeService.toggle()
    expect(themeService.current).toBe('light')
  })
})

// =====================================================================
// applySilent()
// =====================================================================
describe('applySilent()', () => {
  it('updates current theme', () => {
    themeService.applySilent('light')

    expect(themeService.current).toBe('light')
  })

  it('persists to localStorage', () => {
    themeService.applySilent('light')

    expect(localStorage.getItem('symbion_theme')).toBe('light')
  })

  it('applies data-theme attribute', () => {
    themeService.applySilent('light')

    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('does NOT dispatch theme-changed event', () => {
    const handler = vi.fn()
    document.body.addEventListener('theme-changed', handler)

    themeService.applySilent('light')

    expect(handler).not.toHaveBeenCalled()

    document.body.removeEventListener('theme-changed', handler)
  })
})

// =====================================================================
// notifyComponents()
// =====================================================================
describe('notifyComponents()', () => {
  it('dispatches theme-changed event with current theme', () => {
    themeService.current = 'light'
    const handler = vi.fn()
    document.body.addEventListener('theme-changed', handler)

    themeService.notifyComponents()

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.theme).toBe('light')

    document.body.removeEventListener('theme-changed', handler)
  })
})

// =====================================================================
// _apply()
// =====================================================================
describe('_apply()', () => {
  it('sets data-theme attribute on documentElement', () => {
    themeService._apply('light')

    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('overwrites existing data-theme', () => {
    document.documentElement.setAttribute('data-theme', 'dark')

    themeService._apply('light')

    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })
})

// =====================================================================
// _notifyKernel()
// =====================================================================
describe('_notifyKernel()', () => {
  it('calls csrfService.fetchWithCsrf with correct payload', async () => {
    const { default: csrfService } = await import('./csrf-service.js')

    await themeService._notifyKernel('dark')

    expect(csrfService.fetchWithCsrf).toHaveBeenCalledTimes(1)
    const [url, options] = csrfService.fetchWithCsrf.mock.calls[0]
    expect(url).toBe('/v1/intelligence/features')
    expect(options.method).toBe('POST')

    const body = JSON.parse(options.body)
    expect(body.feature_id).toBe('appearance.theme')
    expect(body.value).toBe('dark')
    expect(body.source).toBe('pwa-dashboard')
    expect(body.ttl_seconds).toBe(0)
  })

  it('does not throw if kernel request fails', async () => {
    const { default: csrfService } = await import('./csrf-service.js')
    csrfService.fetchWithCsrf.mockRejectedValueOnce(new Error('Network error'))

    // Should not throw
    await themeService._notifyKernel('light')
  })
})

// =====================================================================
// full lifecycle: applySilent → notifyComponents
// =====================================================================
describe('animation lifecycle', () => {
  it('applySilent then notifyComponents = full theme change without flash', () => {
    const handler = vi.fn()
    document.body.addEventListener('theme-changed', handler)

    // Step 1: Apply silently during animation
    themeService.applySilent('light')
    expect(themeService.current).toBe('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(handler).not.toHaveBeenCalled()

    // Step 2: Notify after animation is done
    themeService.notifyComponents()
    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.theme).toBe('light')

    document.body.removeEventListener('theme-changed', handler)
  })
})
