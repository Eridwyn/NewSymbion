/**
 * Scroll Reveal Utility
 *
 * Sets up IntersectionObserver on elements with `.scroll-reveal` class
 * inside a Shadow DOM root. Elements get `.revealed` class when visible.
 *
 * Usage in a Lit component:
 *   import { setupScrollReveal } from '../utils/scroll-reveal.js'
 *
 *   firstUpdated() {
 *     super.firstUpdated?.()
 *     this._cleanupReveal = setupScrollReveal(this.shadowRoot)
 *   }
 *
 *   disconnectedCallback() {
 *     super.disconnectedCallback()
 *     this._cleanupReveal?.()
 *   }
 */

const REDUCED_MOTION = window.matchMedia('(prefers-reduced-motion: reduce)').matches

/**
 * Observe `.scroll-reveal` elements and add `.revealed` when they enter viewport.
 * @param {ShadowRoot|Element} root - The root to query elements from
 * @param {Object} [options] - IntersectionObserver options
 * @param {number} [options.threshold=0.15] - Visibility threshold to trigger
 * @param {string} [options.rootMargin='0px 0px -40px 0px'] - Root margin
 * @returns {Function} Cleanup function to disconnect observer
 */
export function setupScrollReveal(root, options = {}) {
  if (!root || REDUCED_MOTION) {
    // Reveal all immediately if reduced motion
    root?.querySelectorAll?.('.scroll-reveal')?.forEach(el => el.classList.add('revealed'))
    return () => {}
  }

  const {
    threshold = 0.15,
    rootMargin = '0px 0px -40px 0px'
  } = options

  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        entry.target.classList.add('revealed')
        observer.unobserve(entry.target)
      }
    }
  }, { threshold, rootMargin })

  const elements = root.querySelectorAll('.scroll-reveal')
  elements.forEach(el => observer.observe(el))

  return () => observer.disconnect()
}
