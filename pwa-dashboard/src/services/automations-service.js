/**
 * Service Automations Symbion
 *
 * Client API pour la gestion des automatisations
 * - GET: Via apiService (JWT seul)
 * - POST/PUT/DELETE/PATCH: Via csrfService.fetchWithCsrf (CSRF requis)
 */

const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'

class AutomationsService extends EventTarget {
  constructor() {
    super()
    this.apiService = null
    this.csrfService = null
    this.schema = null
    this.automations = []
    this.status = 'idle' // idle, loading, ready, error
  }

  /**
   * Initialiser avec références aux services
   */
  init(apiService, csrfService) {
    this.apiService = apiService
    this.csrfService = csrfService
    console.log('[automations-service] Initialized')
  }

  // ===== GET Endpoints (JWT seul via apiService) =====

  /**
   * Charger la liste des automations
   */
  async fetchAutomations() {
    if (!this.apiService) {
      console.error('[automations-service] API service not initialized')
      return []
    }

    try {
      this.status = 'loading'
      console.log('[automations-service] Calling apiService.request(/v1/automations)')
      const response = await this.apiService.request('/v1/automations')
      console.log('[automations-service] Raw response:', JSON.stringify(response))
      this.automations = response?.automations || []
      this.status = 'ready'

      this.dispatchEvent(new CustomEvent('automations:loaded', {
        detail: { automations: this.automations, count: response?.count }
      }))

      return this.automations
    } catch (error) {
      console.error('[automations-service] Failed to fetch automations:', error)
      this.status = 'error'
      this.dispatchEvent(new CustomEvent('automations:error', {
        detail: { error: error.message }
      }))
      return []
    }
  }

  /**
   * Récupérer une automation par ID
   */
  async getAutomation(id) {
    if (!this.apiService) return null

    try {
      return await this.apiService.request(`/v1/automations/${encodeURIComponent(id)}`)
    } catch (error) {
      console.error('[automations-service] Failed to get automation:', error)
      return null
    }
  }

  /**
   * Charger le schéma pour le rule builder
   */
  async fetchSchema() {
    if (!this.apiService) return null

    try {
      this.schema = await this.apiService.request('/v1/automations/schema')
      console.log('[automations-service] Schema loaded:', {
        triggers: this.schema.triggers?.length,
        conditions: this.schema.conditions?.length,
        actions: this.schema.actions?.length
      })
      return this.schema
    } catch (error) {
      console.error('[automations-service] Failed to fetch schema:', error)
      return null
    }
  }

  /**
   * Récupérer l'historique d'exécution
   */
  async fetchHistory(limit = 50) {
    if (!this.apiService) return []

    try {
      return await this.apiService.request(`/v1/automations/history?limit=${limit}`)
    } catch (error) {
      console.error('[automations-service] Failed to fetch history:', error)
      return []
    }
  }

  // ===== Mutations (CSRF requis via csrfService) =====

  /**
   * Créer une nouvelle automation
   */
  async createAutomation(automation) {
    if (!this.csrfService) {
      throw new Error('CSRF service not initialized')
    }

    try {
      const response = await this.csrfService.fetchWithCsrf(
        `${API_BASE}/v1/automations`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(automation)
        }
      )

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }))
        throw new Error(error.error || `HTTP ${response.status}`)
      }

      const created = await response.json()
      console.log('[automations-service] Created automation:', created.id)

      // Refresh list
      await this.fetchAutomations()

      this.dispatchEvent(new CustomEvent('automation:created', {
        detail: { automation: created }
      }))

      return created
    } catch (error) {
      console.error('[automations-service] Failed to create automation:', error)
      throw error
    }
  }

  /**
   * Mettre à jour une automation
   */
  async updateAutomation(id, automation) {
    if (!this.csrfService) {
      throw new Error('CSRF service not initialized')
    }

    try {
      const response = await this.csrfService.fetchWithCsrf(
        `${API_BASE}/v1/automations/${encodeURIComponent(id)}`,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(automation)
        }
      )

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }))
        throw new Error(error.error || `HTTP ${response.status}`)
      }

      const updated = await response.json()
      console.log('[automations-service] Updated automation:', id)

      // Refresh list
      await this.fetchAutomations()

      this.dispatchEvent(new CustomEvent('automation:updated', {
        detail: { automation: updated }
      }))

      return updated
    } catch (error) {
      console.error('[automations-service] Failed to update automation:', error)
      throw error
    }
  }

  /**
   * Supprimer une automation (soft-delete)
   */
  async deleteAutomation(id) {
    if (!this.csrfService) {
      throw new Error('CSRF service not initialized')
    }

    try {
      const response = await this.csrfService.fetchWithCsrf(
        `${API_BASE}/v1/automations/${encodeURIComponent(id)}`,
        { method: 'DELETE' }
      )

      if (!response.ok && response.status !== 204) {
        const error = await response.json().catch(() => ({ error: response.statusText }))
        throw new Error(error.error || `HTTP ${response.status}`)
      }

      console.log('[automations-service] Deleted automation:', id)

      // Refresh list
      await this.fetchAutomations()

      this.dispatchEvent(new CustomEvent('automation:deleted', {
        detail: { id }
      }))

      return true
    } catch (error) {
      console.error('[automations-service] Failed to delete automation:', error)
      throw error
    }
  }

  /**
   * Activer/désactiver une automation
   */
  async toggleAutomation(id, enabled) {
    if (!this.csrfService) {
      throw new Error('CSRF service not initialized')
    }

    try {
      const response = await this.csrfService.fetchWithCsrf(
        `${API_BASE}/v1/automations/${encodeURIComponent(id)}/enable`,
        {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ enabled })
        }
      )

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }))
        throw new Error(error.error || `HTTP ${response.status}`)
      }

      const updated = await response.json()
      console.log('[automations-service] Toggled automation:', id, 'enabled:', enabled)

      // Refresh list
      await this.fetchAutomations()

      this.dispatchEvent(new CustomEvent('automation:toggled', {
        detail: { automation: updated }
      }))

      return updated
    } catch (error) {
      console.error('[automations-service] Failed to toggle automation:', error)
      throw error
    }
  }

  /**
   * Tester une automation (dry-run)
   */
  async testAutomation(id) {
    if (!this.csrfService) {
      throw new Error('CSRF service not initialized')
    }

    try {
      const response = await this.csrfService.fetchWithCsrf(
        `${API_BASE}/v1/automations/${encodeURIComponent(id)}/test`,
        { method: 'POST' }
      )

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }))
        throw new Error(error.error || `HTTP ${response.status}`)
      }

      const result = await response.json()
      console.log('[automations-service] Test result:', result)

      this.dispatchEvent(new CustomEvent('automation:tested', {
        detail: { result }
      }))

      return result
    } catch (error) {
      console.error('[automations-service] Failed to test automation:', error)
      throw error
    }
  }

  // ===== Helpers =====

  /**
   * Obtenir le label d'un trigger par son type
   */
  getTriggerLabel(type) {
    if (!this.schema?.triggers) return type
    const trigger = this.schema.triggers.find(t => t.type === type)
    return trigger?.label || type
  }

  /**
   * Obtenir le label d'une action par son type
   */
  getActionLabel(type) {
    if (!this.schema?.actions) return type
    const action = this.schema.actions.find(a => a.type === type)
    return action?.label || type
  }

  /**
   * Formater le trigger pour affichage
   */
  formatTrigger(trigger) {
    if (!trigger) return 'Aucun'

    const label = this.getTriggerLabel(trigger.type)

    switch (trigger.type) {
      case 'mode_change':
        if (trigger.to_mode) return `${label} → ${trigger.to_mode}`
        return label
      case 'sensor_alert':
        if (trigger.room_id) return `${label} (${trigger.room_id})`
        return label
      case 'agent_status':
        return `${label}: ${trigger.status || 'any'}`
      default:
        return label
    }
  }

  /**
   * Formater les actions pour affichage
   */
  formatActions(actions) {
    if (!actions || actions.length === 0) return 'Aucune'
    return actions.map(a => this.getActionLabel(a.type)).join(', ')
  }
}

// Singleton
const automationsService = new AutomationsService()
export default automationsService
