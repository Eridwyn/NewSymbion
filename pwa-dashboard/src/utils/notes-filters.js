/**
 * Utilitaire de Filtrage des Notes Symbion
 *
 * Fonctions de filtrage pour recherche, contexte, tags, urgence
 * Logique pure, sans dépendance UI
 */

/**
 * Filtre les notes par contexte
 * @param {Array} notes - Liste des notes
 * @param {string} context - Contexte à filtrer (cravate, intime, neutre)
 * @returns {Array} Notes filtrées
 */
export function filterByContext(notes, context) {
  if (!Array.isArray(notes) || !context) {
    return notes || []
  }

  return notes.filter(note => note.data?.context === context)
}

/**
 * Filtre les notes par recherche textuelle
 * @param {Array} notes - Liste des notes
 * @param {string} query - Recherche (contenu, contexte, tags)
 * @returns {Array} Notes filtrées
 */
export function filterBySearch(notes, query) {
  if (!Array.isArray(notes) || !query || query.trim() === '') {
    return notes || []
  }

  const queryLower = query.toLowerCase().trim()

  return notes.filter(note => {
    // Recherche dans le contenu
    if (note.data?.content && note.data.content.toLowerCase().includes(queryLower)) {
      return true
    }

    // Recherche dans le contexte
    if (note.data?.context && note.data.context.toLowerCase().includes(queryLower)) {
      return true
    }

    // Recherche dans les tags
    if (note.data?.tags && Array.isArray(note.data.tags)) {
      return note.data.tags.some(tag => tag.toLowerCase().includes(queryLower))
    }

    return false
  })
}

/**
 * Filtre les notes par tags sélectionnés
 * @param {Array} notes - Liste des notes
 * @param {Array} selectedTags - Tags à filtrer
 * @returns {Array} Notes filtrées (ET logique: doit avoir au moins un des tags)
 */
export function filterByTags(notes, selectedTags) {
  if (!Array.isArray(notes) || !Array.isArray(selectedTags) || selectedTags.length === 0) {
    return notes || []
  }

  return notes.filter(note => {
    if (!note.data?.tags || !Array.isArray(note.data.tags)) {
      return false
    }

    // Au moins un tag doit matcher
    return note.data.tags.some(tag => selectedTags.includes(tag))
  })
}

/**
 * Filtre les notes urgentes uniquement
 * @param {Array} notes - Liste des notes
 * @returns {Array} Notes urgentes
 */
export function filterUrgent(notes) {
  if (!Array.isArray(notes)) {
    return []
  }

  return notes.filter(note => note.data?.urgent === true)
}

/**
 * Extrait tous les tags uniques des notes
 * @param {Array} notes - Liste des notes
 * @returns {Array} Liste des tags triés
 */
export function extractAllTags(notes) {
  if (!Array.isArray(notes)) {
    return []
  }

  const tagsSet = new Set()

  notes.forEach(note => {
    if (note.data?.tags && Array.isArray(note.data.tags)) {
      note.data.tags.forEach(tag => tagsSet.add(tag))
    }
  })

  return Array.from(tagsSet).sort()
}

/**
 * Applique tous les filtres sur les notes
 * @param {Array} notes - Liste des notes
 * @param {Object} filters - Filtres à appliquer
 * @param {string} filters.context - Contexte (optionnel)
 * @param {string} filters.search - Recherche textuelle (optionnel)
 * @param {Array} filters.tags - Tags sélectionnés (optionnel)
 * @param {boolean} filters.urgentOnly - Urgentes uniquement (optionnel)
 * @param {boolean} filters.contextFilterEnabled - Activer filtre contexte (optionnel)
 * @returns {Array} Notes filtrées
 */
export function applyAllFilters(notes, filters = {}) {
  if (!Array.isArray(notes)) {
    return []
  }

  let filtered = [...notes]

  // Filtre contexte (si activé)
  if (filters.contextFilterEnabled && filters.context) {
    filtered = filterByContext(filtered, filters.context)
  }

  // Filtre recherche
  if (filters.search) {
    filtered = filterBySearch(filtered, filters.search)
  }

  // Filtre tags
  if (filters.tags && Array.isArray(filters.tags) && filters.tags.length > 0) {
    filtered = filterByTags(filtered, filters.tags)
  }

  // Filtre urgentes uniquement
  if (filters.urgentOnly === true) {
    filtered = filterUrgent(filtered)
  }

  return filtered
}
