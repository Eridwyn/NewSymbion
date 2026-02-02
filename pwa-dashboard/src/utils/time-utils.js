/**
 * Time Utilities for Symbion PWA
 *
 * Centralizes time/date conversions between kernel (ISO/UTC) and display (local/JS).
 *
 * KERNEL CONVENTIONS:
 * - day_of_week: ISO 8601 (0=Monday, 6=Sunday)
 * - hour: UTC timezone
 *
 * PWA DISPLAY:
 * - Day names in French
 * - Local timezone for hours
 */

// ============================================================================
// Day of Week Conversion (ISO 0=Mon → Display)
// ============================================================================

const DAY_NAMES_SHORT = ['Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam', 'Dim']
const DAY_NAMES_FULL = ['Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi', 'Dimanche']

/**
 * Convert ISO day_of_week (0=Monday) to short French name
 * @param {number} isoDayOfWeek - 0-6 where 0=Monday (from kernel)
 * @returns {string} Short day name (Lun, Mar, etc.)
 */
export function getDayNameShort(isoDayOfWeek) {
  const index = Math.max(0, Math.min(6, isoDayOfWeek ?? 0))
  return DAY_NAMES_SHORT[index]
}

/**
 * Convert ISO day_of_week (0=Monday) to full French name
 * @param {number} isoDayOfWeek - 0-6 where 0=Monday (from kernel)
 * @returns {string} Full day name (Lundi, Mardi, etc.)
 */
export function getDayNameFull(isoDayOfWeek) {
  const index = Math.max(0, Math.min(6, isoDayOfWeek ?? 0))
  return DAY_NAMES_FULL[index]
}

/**
 * Get all day names (short) in ISO order (Monday first)
 * @returns {string[]} ['Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam', 'Dim']
 */
export function getAllDayNamesShort() {
  return [...DAY_NAMES_SHORT]
}

/**
 * Get all day names (full) in ISO order (Monday first)
 * @returns {string[]} ['Lundi', 'Mardi', ...]
 */
export function getAllDayNamesFull() {
  return [...DAY_NAMES_FULL]
}

// ============================================================================
// Hour Conversion (UTC → Local)
// ============================================================================

/**
 * Convert UTC hour to local hour
 * @param {number} utcHour - Hour in UTC (0-23)
 * @returns {number} Hour in local timezone (0-23)
 */
export function utcHourToLocal(utcHour) {
  // Get timezone offset in hours (negative for UTC+X)
  const offsetMinutes = new Date().getTimezoneOffset()
  const offsetHours = -offsetMinutes / 60  // Convert to positive for UTC+X

  let localHour = (utcHour + offsetHours) % 24
  if (localHour < 0) localHour += 24

  return Math.floor(localHour)
}

/**
 * Convert local hour to UTC hour
 * @param {number} localHour - Hour in local timezone (0-23)
 * @returns {number} Hour in UTC (0-23)
 */
export function localHourToUtc(localHour) {
  const offsetMinutes = new Date().getTimezoneOffset()
  const offsetHours = -offsetMinutes / 60

  let utcHour = (localHour - offsetHours) % 24
  if (utcHour < 0) utcHour += 24

  return Math.floor(utcHour)
}

/**
 * Format hour for display (e.g., "14h" or "14:00")
 * @param {number} hour - Hour (0-23)
 * @param {boolean} withMinutes - Include ":00" suffix
 * @returns {string} Formatted hour
 */
export function formatHour(hour, withMinutes = false) {
  const h = Math.max(0, Math.min(23, hour ?? 0))
  return withMinutes ? `${h}:00` : `${h}h`
}

// ============================================================================
// Combined Conversions for Kernel Signals
// ============================================================================

/**
 * Convert kernel signals time to local display format
 * @param {Object} signals - Kernel signals object with hour and day_of_week
 * @returns {Object} { localHour, dayNameShort, dayNameFull, displayText }
 */
export function convertSignalsToLocal(signals) {
  const localHour = utcHourToLocal(signals.hour ?? 0)
  const dayNameShort = getDayNameShort(signals.day_of_week)
  const dayNameFull = getDayNameFull(signals.day_of_week)

  return {
    localHour,
    dayNameShort,
    dayNameFull,
    displayText: `${dayNameFull} ${localHour}h`
  }
}

/**
 * Convert kernel pattern to local display format
 * @param {Object} pattern - Pattern with hour and day_of_week
 * @returns {Object} { localHour, dayNameShort, dayNameFull }
 */
export function convertPatternToLocal(pattern) {
  return {
    localHour: utcHourToLocal(pattern.hour ?? 0),
    dayNameShort: getDayNameShort(pattern.day_of_week),
    dayNameFull: getDayNameFull(pattern.day_of_week)
  }
}
