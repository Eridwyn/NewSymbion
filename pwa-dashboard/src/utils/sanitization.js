/**
 * Sanitization Utilities
 *
 * Centralized XSS protection helpers for the PWA dashboard.
 * Use these instead of raw innerHTML assignments.
 */

/**
 * Escape HTML special characters to prevent XSS
 * Use for text content that will be inserted into HTML
 *
 * @param {string} text - Raw text to escape
 * @returns {string} HTML-safe escaped string
 */
export function escapeHtml(text) {
  if (text === null || text === undefined) {
    return ''
  }
  const div = document.createElement('div')
  div.textContent = String(text)
  return div.innerHTML
}

/**
 * Escape HTML and preserve newlines as <br>
 * Use for multi-line text content
 *
 * @param {string} text - Raw text to escape
 * @returns {string} HTML-safe escaped string with <br/> for newlines
 */
export function escapeHtmlPreserveNewlines(text) {
  return escapeHtml(text).replace(/\n/g, '<br/>')
}

/**
 * Escape a string for use in HTML attributes
 *
 * @param {string} value - Raw attribute value
 * @returns {string} Safe attribute value
 */
export function escapeAttribute(value) {
  if (value === null || value === undefined) {
    return ''
  }
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}
