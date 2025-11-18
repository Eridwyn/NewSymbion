/**
 * Widget Environment - Monitoring environnemental IoT évolutif
 *
 * Affiche TOUS les sensors/rooms du système de manière dynamique:
 * - Fetch automatique depuis GET /v1/environment/sensors
 * - Regroupement par room_id (chambre, salon, bureau...)
 * - Température + Humidité + Status temps réel
 * - Scalable pour 1...N sensors sans hard-coding
 * - MQTT updates en temps réel (optionnel future)
 */

import { LitElement, html, css } from 'lit'
import '../components/organic-loader.js'

class EnvironmentWidget extends LitElement {
  static properties = {
    sensors: { type: Array },
    environments: { type: Object }, // Map: room_id -> environment data
    loading: { type: Boolean },
    error: { type: String },
    viewMode: { type: String }, // 'grid' or 'list'
  }

  static styles = css`
    :host {
      display: block;
      background: var(--widget-background, #1a1a1a);
      border-radius: 12px;
      padding: 20px;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
      color: var(--widget-color, #e5e5e5);
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    }

    .widget-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
      padding-bottom: 12px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .widget-title {
      font-size: 18px;
      font-weight: 600;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .sensor-count {
      font-size: 12px;
      color: #888;
      background: rgba(255, 255, 255, 0.1);
      padding: 4px 8px;
      border-radius: 12px;
    }

    .loading-state {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 40px;
      color: #888;
      font-size: 14px;
    }

    .error-state {
      padding: 20px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: 8px;
      color: #fca5a5;
      text-align: center;
    }

    .rooms-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      gap: 16px;
    }

    .room-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.03) 100%);
      border: 1px solid rgba(59, 130, 246, 0.2);
      border-radius: 12px;
      padding: 18px;
      transition: all 0.3s ease;
      position: relative;
      overflow: hidden;
    }

    .room-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 4px;
      height: 100%;
      transition: all 0.3s ease;
    }

    .room-card.normal::before {
      background: linear-gradient(180deg, #22c55e 0%, #16a34a 100%);
      box-shadow: 0 0 15px rgba(34, 197, 94, 0.4);
    }

    .room-card.humid::before {
      background: linear-gradient(180deg, #eab308 0%, #ca8a04 100%);
      box-shadow: 0 0 15px rgba(234, 179, 8, 0.4);
    }

    .room-card.risk_mold::before {
      background: linear-gradient(180deg, #f97316 0%, #ea580c 100%);
      box-shadow: 0 0 20px rgba(249, 115, 22, 0.5);
    }

    .room-card.cold::before {
      background: linear-gradient(180deg, #3b82f6 0%, #2563eb 100%);
      box-shadow: 0 0 15px rgba(59, 130, 246, 0.4);
    }

    .room-card:hover {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
      border-color: rgba(59, 130, 246, 0.4);
      transform: translateY(-2px);
      box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
    }

    .room-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 16px;
    }

    .room-name {
      font-size: 16px;
      font-weight: 600;
      color: #ffffff;
      text-transform: capitalize;
    }

    .status-badge {
      padding: 4px 10px;
      border-radius: 12px;
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .status-badge.ok,
    .status-badge.normal {
      background: rgba(34, 197, 94, 0.2);
      color: #86efac;
      border: 1px solid rgba(34, 197, 94, 0.3);
    }

    .status-badge.humid {
      background: rgba(234, 179, 8, 0.2);
      color: #fde047;
      border: 1px solid rgba(234, 179, 8, 0.3);
    }

    .status-badge.risk_mold {
      background: rgba(249, 115, 22, 0.2);
      color: #fdba74;
      border: 1px solid rgba(249, 115, 22, 0.3);
      animation: pulse-warning 2s ease-in-out infinite;
    }

    .status-badge.cold {
      background: rgba(59, 130, 246, 0.2);
      color: #93c5fd;
      border: 1px solid rgba(59, 130, 246, 0.3);
    }

    @keyframes pulse-warning {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.6; }
    }

    .readings {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 12px;
      margin-bottom: 12px;
    }

    .reading-item {
      background: rgba(0, 0, 0, 0.2);
      padding: 12px;
      border-radius: 8px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .reading-label {
      font-size: 11px;
      color: #888;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .reading-value {
      font-size: 24px;
      font-weight: 700;
      color: #ffffff;
      display: flex;
      align-items: baseline;
      gap: 4px;
    }

    .reading-unit {
      font-size: 14px;
      color: #888;
      font-weight: 400;
    }

    .sensor-info {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-top: 12px;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
      font-size: 12px;
      color: #888;
    }

    .sensor-type {
      text-transform: uppercase;
      font-weight: 600;
    }

    .sensor-signal {
      display: flex;
      align-items: center;
      gap: 4px;
    }

    .signal-icon {
      width: 16px;
      height: 16px;
    }

    .empty-state {
      text-align: center;
      padding: 40px 20px;
      color: #888;
    }

    .empty-icon {
      font-size: 48px;
      margin-bottom: 16px;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 14px;
      line-height: 1.6;
    }
  `

  constructor() {
    super()
    this.sensors = []
    this.environments = {}
    this.loading = true
    this.error = null
    this.viewMode = 'grid'
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadEnvironmentData()
    // Refresh data every 30 seconds
    this.refreshInterval = setInterval(() => this.loadEnvironmentData(), 30000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval)
    }
  }

  async loadEnvironmentData() {
    try {
      this.loading = true
      this.error = null

      // Fetch all sensors
      const apiBase = window.SYMBION_CONFIG?.API_BASE || 'https://localhost:8443'
      const sensorsResponse = await fetch(`${apiBase}/v1/environment/sensors`, {
        headers: {
          'X-API-Key': localStorage.getItem('symbion_api_key') || window.SYMBION_CONFIG?.API_KEY || 's3cr3t-42'
        }
      })

      if (!sensorsResponse.ok) {
        throw new Error(`Failed to fetch sensors: ${sensorsResponse.status}`)
      }

      const sensorsData = await sensorsResponse.json()
      this.sensors = sensorsData.sensors || []

      // Fetch environment data for each unique room
      const uniqueRooms = [...new Set(this.sensors.map(s => s.room_id))]
      const environments = {}

      await Promise.all(
        uniqueRooms.map(async (roomId) => {
          try {
            const envResponse = await fetch(`${apiBase}/v1/environment/${roomId}`, {
              headers: {
                'X-API-Key': localStorage.getItem('symbion_api_key') || window.SYMBION_CONFIG?.API_KEY || 's3cr3t-42'
              }
            })

            if (envResponse.ok) {
              environments[roomId] = await envResponse.json()
            }
          } catch (err) {
            console.warn(`Failed to fetch environment for ${roomId}:`, err)
          }
        })
      )

      this.environments = environments
      this.loading = false
    } catch (err) {
      console.error('Environment widget error:', err)
      this.error = err.message
      this.loading = false
    }
  }

  getStatusIcon(status) {
    const icons = {
      ok: '✓',
      normal: '✓',
      humid: '⚠',
      risk_mold: '🚨',
      cold: '❄'
    }
    return icons[status] || '?'
  }

  getStatusLabel(status) {
    const labels = {
      ok: 'Normal',
      normal: 'Normal',
      humid: 'Humide',
      risk_mold: 'Risque Moisissure',
      cold: 'Froid'
    }
    return labels[status] || status
  }

  formatSignalStrength(rssi) {
    if (!rssi) return ''
    if (rssi > -50) return '📶 Excellent'
    if (rssi > -60) return '📶 Bon'
    if (rssi > -70) return '📶 Moyen'
    return '📶 Faible'
  }

  render() {
    if (this.loading) {
      return html`
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
        </div>
        <div class="loading-state">
          <organic-loader></organic-loader>
        </div>
      `
    }

    if (this.error) {
      return html`
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
        </div>
        <div class="error-state">
          ⚠️ Erreur: ${this.error}
        </div>
      `
    }

    const roomCount = Object.keys(this.environments).length

    if (roomCount === 0) {
      return html`
        <div class="widget-header">
          <div class="widget-title">🌡️ Environnement</div>
          <span class="sensor-count">0 capteur</span>
        </div>
        <div class="empty-state">
          <div class="empty-icon">🌡️</div>
          <div class="empty-text">
            Aucun capteur environnemental détecté.<br>
            Connectez un ESP32 avec BME280 pour commencer.
          </div>
        </div>
      `
    }

    return html`
      <div class="widget-header">
        <div class="widget-title">
          🌡️ Environnement
          <span class="sensor-count">${this.sensors.length} capteur${this.sensors.length > 1 ? 's' : ''}</span>
        </div>
      </div>

      <div class="rooms-grid">
        ${Object.entries(this.environments).map(([roomId, env]) => {
          const sensor = this.sensors.find(s => s.room_id === roomId)
          return this.renderRoomCard(roomId, env, sensor)
        })}
      </div>
    `
  }

  renderRoomCard(roomId, env, sensor) {
    const temp = env.current.temperature_c.toFixed(1)
    const humidity = env.current.humidity_pct.toFixed(1)
    const status = env.status
    const statusLabel = this.getStatusLabel(status)

    return html`
      <div class="room-card ${status}">
        <div class="room-header">
          <div class="room-name">${roomId}</div>
          <div class="status-badge ${status}">
            ${this.getStatusIcon(status)} ${statusLabel}
          </div>
        </div>

        <div class="readings">
          <div class="reading-item">
            <div class="reading-label">Température</div>
            <div class="reading-value">
              ${temp}<span class="reading-unit">°C</span>
            </div>
          </div>

          <div class="reading-item">
            <div class="reading-label">Humidité</div>
            <div class="reading-value">
              ${humidity}<span class="reading-unit">%</span>
            </div>
          </div>
        </div>

        ${sensor ? html`
          <div class="sensor-info">
            <span class="sensor-type">${sensor.sensor_type}</span>
            <span class="sensor-signal">${this.formatSignalStrength(sensor.signal_rssi)}</span>
          </div>
        ` : ''}
      </div>
    `
  }
}

customElements.define('environment-widget', EnvironmentWidget)
