/**
 * Utilitaire de Scoring des Notes Symbion
 *
 * Calcul du score de priorité basé sur:
 * - Urgence: +100 pts
 * - Match contexte: +50 pts
 * - Récence: 0-20 pts (max 7 jours)
 *
 * Logique pure, sans dépendance UI
 */

/**
 * Calcule le score de récence d'une note
 * @param {Array} timestamp - Format Symbion [year, day_of_year, hour, minute, second, nanos, ?, ?, ?]
 * @returns {number} Score de 0 à 20 pts
 */
function calculateRecencyScore(timestamp) {
  if (!timestamp || !Array.isArray(timestamp)) {
    return 0
  }

  try {
    const [year, dayOfYear, hour, minute] = timestamp
    const noteDate = new Date(year, 0, dayOfYear, hour || 0, minute || 0)
    const now = new Date()
    const daysDiff = (now - noteDate) / (1000 * 60 * 60 * 24)

    // Notes jusqu'à 7 jours: décroissance linéaire de 20 à 0
    if (daysDiff <= 7) {
      return Math.max(0, Math.round(20 - (daysDiff * 3)))
    }

    return 0
  } catch (error) {
    console.warn('[notes-scoring] Failed to calculate recency:', error)
    return 0
  }
}

// Alias pour compatibilite ancien/nouveau systeme de contextes
const SCORING_ALIASES = {
  pro: ['cravate', 'pro'],
  maison: ['intime', 'maison'],
  veille: ['neutre', 'veille'],
  cravate: ['cravate', 'pro'],
  intime: ['intime', 'maison'],
  neutre: ['neutre', 'veille'],
}

/**
 * Calcule le score de priorité d'une note
 * @param {Object} note - Note Symbion
 * @param {string} currentContext - Contexte actuel (pro, maison, veille, focus)
 * @returns {number} Score total (0-170 pts)
 */
export function calculatePriorityScore(note, currentContext) {
  let score = 0

  // Urgence: +100 pts
  if (note.data?.urgent === true) {
    score += 100
  }

  // Match contexte (avec alias): +50 pts
  if (currentContext && note.data?.context) {
    const ctxLower = currentContext.toLowerCase()
    const noteCtx = note.data.context.toLowerCase()
    const accepted = SCORING_ALIASES[ctxLower] || [ctxLower]
    if (accepted.includes(noteCtx)) {
      score += 50
    }
  }

  // Récence: 0-20 pts
  score += calculateRecencyScore(note.timestamp)

  return score
}

/**
 * Trie les notes par score de priorité (décroissant)
 * @param {Array} notes - Liste des notes
 * @param {string} currentContext - Contexte actuel
 * @returns {Array} Notes triées par priorité
 */
export function sortNotesByPriority(notes, currentContext) {
  if (!Array.isArray(notes)) {
    return []
  }

  return [...notes].sort((a, b) => {
    const scoreA = calculatePriorityScore(a, currentContext)
    const scoreB = calculatePriorityScore(b, currentContext)
    return scoreB - scoreA
  })
}

/**
 * Détermine si une note est hautement prioritaire
 * @param {Object} note - Note Symbion
 * @param {string} currentContext - Contexte actuel
 * @returns {boolean} true si score >= 50 et non urgente
 */
export function isHighPriority(note, currentContext) {
  const score = calculatePriorityScore(note, currentContext)
  const isUrgent = note.data?.urgent === true
  return score >= 50 && !isUrgent
}

/**
 * Obtient les N notes les plus prioritaires
 * @param {Array} notes - Liste des notes
 * @param {string} currentContext - Contexte actuel
 * @param {number} limit - Nombre de notes à retourner
 * @returns {Array} Top N notes triées
 */
export function getTopPriorityNotes(notes, currentContext, limit = 3) {
  const sorted = sortNotesByPriority(notes, currentContext)
  return sorted.slice(0, limit)
}
