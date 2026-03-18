/**
 * Service API pour Decision Engine (PR3)
 *
 * Gère les appels API pour:
 * - Validations en attente (approve/reject)
 * - Overrides actifs
 * - Audit trail
 * - Métriques et statistiques
 */

import { getApiBase } from './config.js'
const API_BASE = getApiBase()

class DecisionService {
  constructor() {
    this.authService = null
    this.csrfService = null
  }

  /**
   * Initialiser avec authService et csrfService
   */
  async init() {
    if (!this.authService) {
      const authServiceModule = await import('./auth-service.js')
      this.authService = authServiceModule.default
    }

    if (!this.csrfService) {
      const csrfServiceModule = await import('./csrf-service.js')
      this.csrfService = csrfServiceModule.default
      this.csrfService.setAuthService(this.authService)
    }
  }

  /**
   * Helper pour requêtes GET (pas de CSRF)
   */
  async get(endpoint) {
    await this.init()

    // SW injects Authorization header automatically

    const response = await fetch(`${API_BASE}${endpoint}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json'
      },
      credentials: 'include'
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return await response.json()
  }

  /**
   * Helper pour requêtes POST (avec CSRF)
   */
  async post(endpoint, body = {}) {
    await this.init()

    const response = await this.csrfService.fetchWithCsrf(`${API_BASE}${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(body)
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return await response.json()
  }

  // ========================================
  // Validations
  // ========================================

  /**
   * GET /v1/decision/validations/pending
   * Liste des validations en attente
   */
  async getPendingValidations() {
    return await this.get('/v1/decision/validations/pending')
  }

  /**
   * GET /v1/decision/validations/expired
   * Liste des validations expirées
   */
  async getExpiredValidations() {
    return await this.get('/v1/decision/validations/expired')
  }

  /**
   * POST /v1/decision/validation/:id/resolve
   * Résoudre validation (approve ou reject)
   *
   * @param {string} validationId - ID de la validation
   * @param {boolean} approved - true=approve, false=reject
   */
  async resolveValidation(validationId, approved) {
    await this.init()

    const username = this.authService.getCurrentUser()?.username || 'unknown'

    return await this.post(`/v1/decision/validation/${validationId}/resolve`, {
      approved,
      username
    })
  }

  /**
   * DELETE /v1/decision/validation/:id
   * Supprimer une validation expirée
   *
   * @param {string} validationId - ID de la validation à supprimer
   */
  async deleteValidation(validationId) {
    await this.init()

    // SW injects Authorization header automatically

    // Récupérer CSRF nonce
    const csrfNonce = await this.csrfService.getNonce()

    const response = await fetch(`${API_BASE}/v1/decision/validation/${validationId}`, {
      method: 'DELETE',
      headers: {
        'X-CSRF-Token': csrfNonce
      },
      credentials: 'include'
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }
  }

  /**
   * DELETE /v1/decision/validations/expired
   * Supprimer toutes les validations expirées
   */
  async deleteAllExpiredValidations() {
    await this.init()

    // SW injects Authorization header automatically

    // Récupérer CSRF nonce
    const csrfNonce = await this.csrfService.getNonce()

    const response = await fetch(`${API_BASE}/v1/decision/validations/expired`, {
      method: 'DELETE',
      headers: {
        'X-CSRF-Token': csrfNonce
      },
      credentials: 'include'
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return await response.json()
  }

  // ========================================
  // Overrides
  // ========================================

  /**
   * GET /v1/decision/overrides/active
   * Liste des overrides actifs
   */
  async getActiveOverrides() {
    return await this.get('/v1/decision/overrides/active')
  }

  /**
   * POST /v1/decision/override
   * Créer un override (MFA requis)
   *
   * @param {string} agentId - ID de l'agent
   * @param {object} action - Action overridée
   * @param {number} durationMinutes - Durée override
   * @param {string} reason - Raison override
   * @param {string} totpCode - Code TOTP MFA
   */
  async createOverride(agentId, action, durationMinutes, reason, totpCode) {
    return await this.post('/v1/decision/override', {
      agent_id: agentId,
      action,
      duration_minutes: durationMinutes,
      reason,
      totp_code: totpCode
    })
  }

  // ========================================
  // Audit & Stats
  // ========================================

  /**
   * GET /v1/decision/audit
   * Audit trail complet
   */
  async getAuditTrail() {
    return await this.get('/v1/decision/audit')
  }

  /**
   * GET /v1/decision/stats
   * Statistiques globales
   */
  async getStats() {
    return await this.get('/v1/decision/stats')
  }

  /**
   * GET /v1/decision/metrics
   * Métriques Prometheus
   */
  async getMetrics() {
    return await this.get('/v1/decision/metrics')
  }

  /**
   * GET /v1/decision/config
   * Configuration actuelle
   */
  async getConfig() {
    return await this.get('/v1/decision/config')
  }

  /**
   * GET /v1/decision/agent-health
   * États santé agents
   */
  async getAgentHealth() {
    return await this.get('/v1/decision/agent-health')
  }

  // ========================================
  // Evaluate (pour tests)
  // ========================================

  /**
   * POST /v1/decision/evaluate
   * Évaluer une action (pour tests UI)
   *
   * @param {object} action - Action à évaluer
   * @param {object} context - Contexte de décision (mode, ssid, agents)
   */
  async evaluateAction(action, context) {
    return await this.post('/v1/decision/evaluate', {
      action,
      context
    })
  }
}

// Export singleton
const decisionService = new DecisionService()
export default decisionService
