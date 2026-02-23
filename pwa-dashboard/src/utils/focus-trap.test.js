import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createFocusTrap, manageFocusTrap } from './focus-trap.js'

// Mock requestAnimationFrame for deterministic tests
global.requestAnimationFrame = (cb) => setTimeout(cb, 0)

/**
 * Helper: creates a container with focusable elements and appends it to the body.
 * Also patches offsetParent on each focusable child so that the visibility filter
 * inside createFocusTrap (el.offsetParent !== null) does not exclude them in happy-dom.
 */
function createContainer() {
  const container = document.createElement('div')
  container.innerHTML = `
    <button id="btn1">Button 1</button>
    <input id="input1" type="text" />
    <button id="btn2">Button 2</button>
  `
  document.body.appendChild(container)

  // happy-dom returns null for offsetParent; patch every focusable element
  container.querySelectorAll('button, input').forEach(el => {
    Object.defineProperty(el, 'offsetParent', { value: document.body, configurable: true })
  })

  return container
}

/**
 * Helper: creates a container with no focusable elements.
 */
function createEmptyContainer() {
  const container = document.createElement('div')
  container.innerHTML = '<p>No focusable elements here</p>'
  document.body.appendChild(container)
  return container
}

beforeEach(() => {
  document.body.innerHTML = ''
})

// ---------------------------------------------------------------------------
// createFocusTrap - basic API
// ---------------------------------------------------------------------------
describe('createFocusTrap', () => {
  it('returns object with activate, deactivate, destroy, isActive', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    expect(typeof trap.activate).toBe('function')
    expect(typeof trap.deactivate).toBe('function')
    expect(typeof trap.destroy).toBe('function')
    expect('isActive' in trap).toBe(true)
  })

  it('isActive is false initially', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    expect(trap.isActive).toBe(false)
  })

  it('activate sets isActive to true', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    trap.activate()
    expect(trap.isActive).toBe(true)
  })

  it('activate twice does not throw (idempotent)', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    trap.activate()
    expect(() => trap.activate()).not.toThrow()
    expect(trap.isActive).toBe(true)
  })

  it('deactivate sets isActive to false', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    trap.activate()
    trap.deactivate()
    expect(trap.isActive).toBe(false)
  })

  it('deactivate when not active does nothing', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    expect(() => trap.deactivate()).not.toThrow()
    expect(trap.isActive).toBe(false)
  })

  it('destroy deactivates and cleans up', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)

    trap.activate()
    expect(trap.isActive).toBe(true)

    trap.destroy()
    expect(trap.isActive).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// Focus behavior
// ---------------------------------------------------------------------------
describe('focus behavior', () => {
  it('activate focuses first focusable element', async () => {
    const container = createContainer()
    const trap = createFocusTrap(container)
    const btn1 = container.querySelector('#btn1')

    trap.activate()

    // requestAnimationFrame is mocked to setTimeout(cb, 0)
    await vi.waitFor(() => {
      expect(document.activeElement).toBe(btn1)
    })
  })

  it('activate sets tabindex=-1 on container when no focusable elements', () => {
    const container = createEmptyContainer()
    const trap = createFocusTrap(container)

    trap.activate()

    expect(container.getAttribute('tabindex')).toBe('-1')
  })

  it('deactivate restores focus to previously focused element', async () => {
    // Create an external button that will hold initial focus
    const outsideBtn = document.createElement('button')
    outsideBtn.id = 'outside'
    Object.defineProperty(outsideBtn, 'offsetParent', { value: document.body, configurable: true })
    document.body.appendChild(outsideBtn)
    outsideBtn.focus()

    const container = createContainer()
    const trap = createFocusTrap(container)

    trap.activate()

    // Wait for rAF to complete so first element is focused
    await vi.waitFor(() => {
      expect(document.activeElement).toBe(container.querySelector('#btn1'))
    })

    trap.deactivate()
    expect(document.activeElement).toBe(outsideBtn)
  })
})

// ---------------------------------------------------------------------------
// Tab key trapping
// ---------------------------------------------------------------------------
describe('Tab key trapping', () => {
  function dispatchTab(target, { shiftKey = false } = {}) {
    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey,
      bubbles: true,
      cancelable: true,
    })
    const spy = vi.spyOn(event, 'preventDefault')
    target.dispatchEvent(event)
    return spy
  }

  it('Tab on last element wraps to first', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)
    trap.activate()

    const btn1 = container.querySelector('#btn1')
    const btn2 = container.querySelector('#btn2')

    // Simulate focus on the last element
    btn2.focus()
    expect(document.activeElement).toBe(btn2)

    const preventSpy = dispatchTab(btn2, { shiftKey: false })

    expect(preventSpy).toHaveBeenCalled()
    expect(document.activeElement).toBe(btn1)
  })

  it('Shift+Tab on first element wraps to last', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)
    trap.activate()

    const btn1 = container.querySelector('#btn1')
    const btn2 = container.querySelector('#btn2')

    btn1.focus()
    expect(document.activeElement).toBe(btn1)

    const preventSpy = dispatchTab(btn1, { shiftKey: true })

    expect(preventSpy).toHaveBeenCalled()
    expect(document.activeElement).toBe(btn2)
  })

  it('Tab in the middle works normally (no prevent)', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)
    trap.activate()

    const input1 = container.querySelector('#input1')

    // input1 is in the middle, so Tab should not be prevented
    input1.focus()
    expect(document.activeElement).toBe(input1)

    const preventSpy = dispatchTab(input1, { shiftKey: false })

    expect(preventSpy).not.toHaveBeenCalled()
  })

  it('Non-Tab keys are ignored', () => {
    const container = createContainer()
    const trap = createFocusTrap(container)
    trap.activate()

    const btn2 = container.querySelector('#btn2')
    btn2.focus()

    const event = new KeyboardEvent('keydown', {
      key: 'Enter',
      bubbles: true,
      cancelable: true,
    })
    const preventSpy = vi.spyOn(event, 'preventDefault')
    btn2.dispatchEvent(event)

    // Enter should not trigger any focus trapping logic
    expect(preventSpy).not.toHaveBeenCalled()
    expect(document.activeElement).toBe(btn2)
  })
})

// ---------------------------------------------------------------------------
// manageFocusTrap
// ---------------------------------------------------------------------------
describe('manageFocusTrap', () => {
  it('creates new trap when isOpen=true and no existing trap', () => {
    const container = createContainer()
    const trap = manageFocusTrap(container, true)

    expect(trap).not.toBeNull()
    expect(trap.isActive).toBe(true)
  })

  it('activates existing trap when isOpen=true', () => {
    const container = createContainer()
    const existingTrap = createFocusTrap(container)

    expect(existingTrap.isActive).toBe(false)

    const result = manageFocusTrap(container, true, existingTrap)

    expect(result).toBe(existingTrap)
    expect(existingTrap.isActive).toBe(true)
  })

  it('deactivates trap when isOpen=false', () => {
    const container = createContainer()
    const existingTrap = createFocusTrap(container)
    existingTrap.activate()
    expect(existingTrap.isActive).toBe(true)

    const result = manageFocusTrap(container, false, existingTrap)

    expect(existingTrap.isActive).toBe(false)
    expect(result).toBe(existingTrap)
  })

  it('returns null when isOpen=false and no existing trap', () => {
    const container = createContainer()
    const result = manageFocusTrap(container, false)

    // No existingTrap passed, so default null is returned
    expect(result).toBeNull()
  })

  it('returns existingTrap reference even when deactivating', () => {
    const container = createContainer()
    const existingTrap = createFocusTrap(container)
    existingTrap.activate()

    const result = manageFocusTrap(container, false, existingTrap)

    // The function returns existingTrap (not null) so the caller can reuse it
    expect(result).toBe(existingTrap)
    expect(result.isActive).toBe(false)
  })
})
