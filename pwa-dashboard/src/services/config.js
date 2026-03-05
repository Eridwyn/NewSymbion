/**
 * Centralized API configuration
 *
 * Single source of truth for API base URL.
 * Uses SYMBION_CONFIG if set, otherwise falls back to current hostname.
 */
export function getApiBase() {
  return window.SYMBION_CONFIG?.API_BASE || `https://${window.location.hostname}:8443`
}
