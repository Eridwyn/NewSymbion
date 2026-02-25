import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Silence console noise
vi.spyOn(console, 'log').mockImplementation(() => {})
vi.spyOn(console, 'warn').mockImplementation(() => {})
vi.spyOn(console, 'error').mockImplementation(() => {})

// Mock auth-service
vi.mock('./auth-service.js', () => ({
  default: {
    isAuthenticated: vi.fn(() => false),
    getToken: vi.fn(() => null)
  }
}))

// Mock theme-service (dynamic import in _syncAppearanceTheme)
vi.mock('./theme-service.js', () => ({
  default: {
    current: 'dark',
    _apply: vi.fn()
  }
}))

// Import class (not singleton — we'll instantiate fresh)
const { ContextService } = await import('./context-service.js')

// ── Helpers ──────────────────────────────────────────────────────────

function createService() {
  const svc = new ContextService()
  // Don't trigger connectedCallback (avoids DOM + polling)
  return svc
}

// ── Lifecycle ────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

// =====================================================================
// hexToHSL()
// =====================================================================
describe('hexToHSL()', () => {
  const svc = createService()

  it('converts pure red (#ff0000)', () => {
    const result = svc.hexToHSL('#ff0000')
    expect(result.hue).toBe(0)
    expect(result.saturation).toBe(100)
    expect(result.lightness).toBe(50)
    expect(result.isGray).toBe(false)
  })

  it('converts pure green (#00ff00)', () => {
    const result = svc.hexToHSL('#00ff00')
    expect(result.hue).toBe(120)
    expect(result.saturation).toBe(100)
    expect(result.lightness).toBe(50)
    expect(result.isGray).toBe(false)
  })

  it('converts pure blue (#0000ff)', () => {
    const result = svc.hexToHSL('#0000ff')
    expect(result.hue).toBe(240)
    expect(result.saturation).toBe(100)
    expect(result.lightness).toBe(50)
    expect(result.isGray).toBe(false)
  })

  it('converts white (#ffffff)', () => {
    const result = svc.hexToHSL('#ffffff')
    expect(result.hue).toBe(0)
    expect(result.saturation).toBe(0)
    expect(result.lightness).toBe(100)
    expect(result.isGray).toBe(true)
  })

  it('converts black (#000000)', () => {
    const result = svc.hexToHSL('#000000')
    expect(result.hue).toBe(0)
    expect(result.saturation).toBe(0)
    expect(result.lightness).toBe(0)
    expect(result.isGray).toBe(true)
  })

  it('handles hex without # prefix', () => {
    const result = svc.hexToHSL('2563eb')
    expect(result.hue).toBeGreaterThan(200)
    expect(result.hue).toBeLessThan(240)
    expect(result.isGray).toBe(false)
  })

  it('detects gray colors (low saturation)', () => {
    const result = svc.hexToHSL('#6b7280')
    expect(result.isGray).toBe(true)
  })

  it('converts Symbion blue (#2563eb)', () => {
    const result = svc.hexToHSL('#2563eb')
    // Should be a blue with high saturation
    expect(result.hue).toBeGreaterThan(200)
    expect(result.hue).toBeLessThan(240)
    expect(result.saturation).toBeGreaterThan(50)
    expect(result.isGray).toBe(false)
  })

  it('converts Symbion green (#10b981)', () => {
    const result = svc.hexToHSL('#10b981')
    // Should be green-ish
    expect(result.hue).toBeGreaterThan(140)
    expect(result.hue).toBeLessThan(180)
    expect(result.isGray).toBe(false)
  })

  it('converts indigo-violet (#6366f1)', () => {
    const result = svc.hexToHSL('#6366f1')
    // Should be purple-ish
    expect(result.hue).toBeGreaterThan(230)
    expect(result.hue).toBeLessThan(250)
    expect(result.isGray).toBe(false)
  })
})

// =====================================================================
// getModeIcon()
// =====================================================================
describe('getModeIcon()', () => {
  it('returns dynamic mode icon if available', () => {
    const svc = createService()
    svc.dynamicModes.set('pro', { name: 'Pro', icon: '👔', theme: {} })
    svc.currentMode = 'pro'

    expect(svc.getModeIcon()).toBe('👔')
  })

  it('falls back to legacy icons', () => {
    const svc = createService()
    svc.currentMode = 'focus'

    expect(svc.getModeIcon()).toBe('🎯')
  })

  it('returns all legacy mode icons', () => {
    const svc = createService()

    svc.currentMode = 'pro'
    expect(svc.getModeIcon()).toBe('👔')

    svc.currentMode = 'focus'
    expect(svc.getModeIcon()).toBe('🎯')

    svc.currentMode = 'maison'
    expect(svc.getModeIcon()).toBe('🏡')

    svc.currentMode = 'veille'
    expect(svc.getModeIcon()).toBe('🌱')
  })

  it('returns fallback icon for unknown mode', () => {
    const svc = createService()
    svc.currentMode = 'unknown-mode'

    expect(svc.getModeIcon()).toBe('🤔')
  })
})

// =====================================================================
// getModeName()
// =====================================================================
describe('getModeName()', () => {
  it('returns dynamic mode name if available', () => {
    const svc = createService()
    svc.dynamicModes.set('gaming', { name: 'Gaming', icon: '🎮', theme: {} })
    svc.currentMode = 'gaming'

    expect(svc.getModeName()).toBe('Gaming')
  })

  it('falls back to legacy names', () => {
    const svc = createService()

    svc.currentMode = 'pro'
    expect(svc.getModeName()).toBe('Pro')

    svc.currentMode = 'focus'
    expect(svc.getModeName()).toBe('Focus')

    svc.currentMode = 'maison'
    expect(svc.getModeName()).toBe('Maison')

    svc.currentMode = 'veille'
    expect(svc.getModeName()).toBe('Veille')
  })

  it('returns "Inconnu" for unknown mode', () => {
    const svc = createService()
    svc.currentMode = 'xyz'

    expect(svc.getModeName()).toBe('Inconnu')
  })
})

// =====================================================================
// getCurrentMode() / getContextState() / getTheme()
// =====================================================================
describe('public API', () => {
  it('getCurrentMode returns currentMode', () => {
    const svc = createService()
    svc.currentMode = 'focus'

    expect(svc.getCurrentMode()).toBe('focus')
  })

  it('getContextState returns null initially', () => {
    const svc = createService()

    expect(svc.getContextState()).toBeNull()
  })

  it('getContextState returns contextState after set', () => {
    const svc = createService()
    const state = { mode: 'pro', reason: 'schedule' }
    svc.contextState = state

    expect(svc.getContextState()).toBe(state)
  })

  it('getTheme returns theme from contextState', () => {
    const svc = createService()
    svc.contextState = {
      mode: 'pro',
      theme: { primary: '#2563eb', bg: '#1a1a1a', accent: '#3b82f6' }
    }

    const theme = svc.getTheme()
    expect(theme.primary).toBe('#2563eb')
  })

  it('getTheme returns null when no contextState', () => {
    const svc = createService()

    expect(svc.getTheme()).toBeNull()
  })

  it('getTheme returns null when contextState has no theme', () => {
    const svc = createService()
    svc.contextState = { mode: 'pro' }

    expect(svc.getTheme()).toBeNull()
  })
})

// =====================================================================
// applyTheme()
// =====================================================================
describe('applyTheme()', () => {
  it('sets CSS custom properties', () => {
    const svc = createService()
    const theme = { primary: '#2563eb', bg: '#1a1a1a', accent: '#3b82f6' }

    svc.applyTheme(theme)

    expect(document.documentElement.style.getPropertyValue('--context-primary')).toBe('#2563eb')
    expect(document.documentElement.style.getPropertyValue('--context-bg')).toBe('#1a1a1a')
    expect(document.documentElement.style.getPropertyValue('--context-accent')).toBe('#3b82f6')
  })

  it('sets logo filter variables', () => {
    const svc = createService()
    const theme = { primary: '#2563eb', bg: '#1a1a1a', accent: '#3b82f6' }

    svc.applyTheme(theme)

    const hue = document.documentElement.style.getPropertyValue('--context-logo-hue')
    const sat = document.documentElement.style.getPropertyValue('--context-logo-saturation')
    const br = document.documentElement.style.getPropertyValue('--context-logo-brightness')

    expect(hue).toContain('deg')
    expect(parseFloat(sat)).toBeGreaterThan(0)
    expect(parseFloat(br)).toBeGreaterThan(0)
  })

  it('sets gray filter for gray colors', () => {
    const svc = createService()
    const theme = { primary: '#6b7280', bg: '#1a1a1a', accent: '#9ca3af' }

    svc.applyTheme(theme)

    const hue = document.documentElement.style.getPropertyValue('--context-logo-hue')
    const sat = document.documentElement.style.getPropertyValue('--context-logo-saturation')
    expect(hue).toBe('0deg')
    expect(sat).toBe('0')
  })

  it('does nothing if theme is null', () => {
    const svc = createService()

    // Should not throw
    svc.applyTheme(null)
    svc.applyTheme(undefined)
  })
})

// =====================================================================
// notifyModeChange()
// =====================================================================
describe('notifyModeChange()', () => {
  it('dispatches context-change event', () => {
    const svc = createService()
    const handler = vi.fn()
    svc.addEventListener('context-change', handler)

    const context = { mode: 'pro', reason: 'schedule' }
    svc.notifyModeChange(context)

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler.mock.calls[0][0].detail.context).toBe(context)

    svc.removeEventListener('context-change', handler)
  })
})

// =====================================================================
// waitForContextReady()
// =====================================================================
describe('waitForContextReady()', () => {
  it('resolves immediately if already ready', async () => {
    const svc = createService()
    svc.status = 'ready'
    svc.contextState = { mode: 'pro' }

    const result = await svc.waitForContextReady()

    expect(result).toEqual({ mode: 'pro' })
  })

  it('resolves with null on timeout', async () => {
    const svc = createService()
    svc.status = 'loading'
    svc.contextState = null

    const promise = svc.waitForContextReady(500)

    vi.advanceTimersByTime(500)

    const result = await promise
    expect(result).toBeNull()
  })
})

// =====================================================================
// constructor defaults
// =====================================================================
describe('constructor defaults', () => {
  it('sets initial mode to veille', () => {
    const svc = createService()
    expect(svc.currentMode).toBe('veille')
  })

  it('sets status to loading', () => {
    const svc = createService()
    expect(svc.status).toBe('loading')
  })

  it('initializes empty dynamicModes map', () => {
    const svc = createService()
    expect(svc.dynamicModes).toBeInstanceOf(Map)
    expect(svc.dynamicModes.size).toBe(0)
  })

  it('sets retryCount to 0', () => {
    const svc = createService()
    expect(svc.retryCount).toBe(0)
  })

  it('sets maxRetries to 10', () => {
    const svc = createService()
    expect(svc.maxRetries).toBe(10)
  })
})
