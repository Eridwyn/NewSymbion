/**
 * Focus Trap Utility
 *
 * Traps keyboard focus within a container element (modal, dialog, etc.)
 * Accessibility requirement for modal dialogs.
 */

const FOCUSABLE_SELECTORS = [
  'button:not([disabled])',
  'a[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]'
].join(', ')

/**
 * Creates a focus trap for a container element
 *
 * @param {HTMLElement} container - The element to trap focus within
 * @returns {Object} - Object with activate(), deactivate(), and destroy() methods
 */
export function createFocusTrap(container) {
  let previouslyFocused = null
  let active = false
  let keydownHandler = null

  function getFocusableElements() {
    return Array.from(container.querySelectorAll(FOCUSABLE_SELECTORS))
      .filter(el => el.offsetParent !== null) // Filter out hidden elements
  }

  function handleKeydown(e) {
    if (e.key !== 'Tab') return

    const focusable = getFocusableElements()
    if (focusable.length === 0) return

    const firstElement = focusable[0]
    const lastElement = focusable[focusable.length - 1]

    // Shift+Tab on first element -> go to last
    if (e.shiftKey && document.activeElement === firstElement) {
      e.preventDefault()
      lastElement.focus()
    }
    // Tab on last element -> go to first
    else if (!e.shiftKey && document.activeElement === lastElement) {
      e.preventDefault()
      firstElement.focus()
    }
  }

  function activate() {
    if (active) return

    // Store currently focused element
    previouslyFocused = document.activeElement

    // Add keydown listener
    keydownHandler = handleKeydown.bind(this)
    container.addEventListener('keydown', keydownHandler)

    // Focus first focusable element (or container if none)
    const focusable = getFocusableElements()
    if (focusable.length > 0) {
      // Small delay to ensure DOM is ready
      requestAnimationFrame(() => {
        focusable[0].focus()
      })
    } else {
      container.setAttribute('tabindex', '-1')
      container.focus()
    }

    active = true
  }

  function deactivate() {
    if (!active) return

    // Remove keydown listener
    if (keydownHandler) {
      container.removeEventListener('keydown', keydownHandler)
      keydownHandler = null
    }

    // Restore focus to previously focused element
    if (previouslyFocused && previouslyFocused.focus) {
      previouslyFocused.focus()
    }

    active = false
  }

  function destroy() {
    deactivate()
    previouslyFocused = null
  }

  return {
    activate,
    deactivate,
    destroy,
    get isActive() { return active }
  }
}

/**
 * Helper to manage focus trap lifecycle with a modal
 * Automatically activates on open and deactivates on close
 *
 * @param {HTMLElement} modalElement - The modal container
 * @param {boolean} isOpen - Whether the modal is open
 * @param {Object} existingTrap - Existing trap instance to reuse
 * @returns {Object|null} - Focus trap instance or null
 */
export function manageFocusTrap(modalElement, isOpen, existingTrap = null) {
  if (isOpen && modalElement) {
    if (!existingTrap) {
      const trap = createFocusTrap(modalElement)
      trap.activate()
      return trap
    } else if (!existingTrap.isActive) {
      existingTrap.activate()
    }
    return existingTrap
  } else if (existingTrap) {
    existingTrap.deactivate()
  }
  return existingTrap
}
