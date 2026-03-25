/**
 * Widget Freebox
 *
 * Affiche la presence et le statut connexion depuis le plugin Freebox
 * Mise a jour temps reel via MQTT
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { widgetHeaderStyles, widgetSectionStyles } from '../styles/shared-widget.js'
import { statusBadgeStyles, statusDotStyles } from '../styles/shared-patterns.js'
import '../components/organic-loader.js'

class FreeboxWidget extends LitElement {
  static styles = [sharedAnimations, widgetHeaderStyles, widgetSectionStyles, statusBadgeStyles, statusDotStyles, css`
    :host {
      display: block;
    }

    /* status-badge variants (.home, .away, .unknown) from shared statusBadgeStyles */
    .status-badge.home {
      animation: pulse-home 3s ease-in-out infinite;
    }

    @keyframes pulse-home {
      0%, 100% { box-shadow: 0 2px 8px var(--ctx-bg-emphasis); }
      50% { box-shadow: 0 2px 16px var(--ctx-border-intense); }
    }

    .content-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 1rem;
    }

    .section-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 1rem;
    }

    /* section-title + svg provided by widgetSectionStyles */

    .presence-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.5rem 0;
      border-bottom: 1px solid var(--border-subtle);
    }

    .presence-item:last-child {
      border-bottom: none;
    }

    .device-info {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .device-icon {
      font-size: 1.2em;
    }

    .device-name {
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.9em;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      max-width: 150px;
    }

    .device-ip {
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: 0.75em;
    }

    .presence-status {
      display: flex;
      align-items: center;
      gap: 0.4rem;
    }

    .status-dot.home {
      background: var(--context-primary);
      box-shadow: var(--ctx-glow-sm);
    }

    .status-dot.away {
      background: #ff6b6b;
    }

    .presence-label {
      font-size: 0.85em;
      font-weight: 500;
    }

    .presence-label.home {
      color: var(--context-primary);
    }

    .presence-label.away {
      color: var(--color-danger-text-muted, #ff6b6b);
    }

    .connection-stats {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 0.75rem;
    }

    .stat-item {
      text-align: center;
      padding: 0.5rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border-radius: var(--radius-base);
    }

    .stat-value {
      font-size: 1.4em;
      font-weight: 700;
      color: var(--context-primary);
    }

    .stat-unit {
      font-size: 0.7em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-left: 2px;
    }

    .stat-label {
      font-size: 0.75em;
      color: var(--color-dark-text-tertiary, #94a3b8);
      margin-top: 0.25rem;
    }

    .connection-type {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      margin-top: 0.75rem;
      padding: 0.5rem;
      background: var(--surface-glass, rgba(255, 255, 255, 0.04));
      border-radius: var(--radius-base);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: 0.8em;
    }

    .connection-type svg {
      width: 14px;
      height: 14px;
      fill: var(--context-primary);
    }

    .loader-container {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 200px;
      padding: 2rem;
    }

    @media (max-width: 768px) {
      .content-grid {
        grid-template-columns: 1fr;
      }
    }

    /* Container Queries — adapt to widget's own width */
    @container widget (max-width: 400px) {
      .content-grid {
        grid-template-columns: 1fr;
      }
    }

    /* Utility classes (ex-inline) */
    .fb-hint-muted { color: #666; font-size: 0.85em; }
  `]

  static properties = {
    presenceDevices: { type: Array },
    connectionStatus: { type: Object },
    anyoneHome: { type: Boolean },
    loading: { type: Boolean },
    mqttConnected: { type: Boolean }
  }

  constructor() {
    super()
    this.presenceDevices = []
    this.connectionStatus = null
    this.anyoneHome = null
    this.loading = true
    this.mqttConnected = false
    this._mqttSubscriptions = []
  }

  connectedCallback() {
    super.connectedCallback()
    // Retry finding mqtt-service (may not exist immediately)
    this._retrySetup()
    // Fallback: stop loading after 10s even if no data
    this._loadingTimeout = setTimeout(() => {
      if (this.loading) {
        console.warn('[freebox-widget] Timeout waiting for data, stopping loader')
        this.loading = false
        this.requestUpdate()
      }
    }, 10000)
  }

  _retrySetup(attempts = 0) {
    const mqttService = document.querySelector('mqtt-service')
    if (mqttService) {
      this._setupEventListeners()
    } else if (attempts < 10) {
      // Retry every 500ms for up to 5 seconds
      this._retryTimeout = setTimeout(() => this._retrySetup(attempts + 1), 500)
    } else {
      console.warn('[freebox-widget] MQTT service not found after retries')
      this.loading = false
      this.requestUpdate()
    }
  }

  _checkDataLoaded() {
    // Loading complete when we have presence OR connection data
    if (this.presenceDevices.length > 0 || this.connectionStatus !== null || this.anyoneHome !== null) {
      this.loading = false
      this.mqttConnected = true
      if (this._loadingTimeout) {
        clearTimeout(this._loadingTimeout)
      }
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    if (this._retryTimeout) {
      clearTimeout(this._retryTimeout)
      this._retryTimeout = null
    }
    if (this._loadingTimeout) {
      clearTimeout(this._loadingTimeout)
      this._loadingTimeout = null
    }
    this._cleanupEventListeners()
  }

  _setupEventListeners() {
    // Find the MQTT service element
    const mqttService = document.querySelector('mqtt-service')
    if (!mqttService) {
      console.warn('[freebox-widget] MQTT service not found')
      return
    }

    console.log('[freebox-widget] Setting up event listeners on mqtt-service')

    // Listen for Freebox events
    this._boundPresenceHandler = (e) => {
      console.log('[freebox-widget] Received freebox-presence event', e.detail)
      this._handlePresenceEvent(e.detail)
    }
    this._boundConnectionHandler = (e) => {
      console.log('[freebox-widget] Received freebox-connection event', e.detail)
      this._handleConnectionEvent(e.detail)
    }

    mqttService.addEventListener('freebox-presence', this._boundPresenceHandler)
    mqttService.addEventListener('freebox-connection', this._boundConnectionHandler)

    this._mqttService = mqttService

    // Load cached data (retained messages that arrived before we subscribed)
    if (typeof mqttService.getFreeboxCache === 'function') {
      const cache = mqttService.getFreeboxCache()
      console.log('[freebox-widget] Loading cached data:', cache)

      // Load cached presence
      for (const [topic, payload] of Object.entries(cache.presence)) {
        this._handleMqttMessage(topic, payload)
      }

      // Load cached connection
      if (cache.connection) {
        this.connectionStatus = cache.connection
        this._checkDataLoaded()
        this.requestUpdate()
      }
    }
  }

  _cleanupEventListeners() {
    if (this._mqttService) {
      this._mqttService.removeEventListener('freebox-presence', this._boundPresenceHandler)
      this._mqttService.removeEventListener('freebox-connection', this._boundConnectionHandler)
    }
  }

  _handlePresenceEvent({ topic, payload }) {
    this._handleMqttMessage(topic, payload)
  }

  _handleConnectionEvent({ payload }) {
    this.connectionStatus = payload
    this._checkDataLoaded()
    this.requestUpdate()
  }

  _handleMqttMessage(topic, payload) {
    if (topic === 'symbion/freebox/presence/summary') {
      if (typeof payload === 'object') {
        this.anyoneHome = payload.anyone_home
      }
    } else if (topic === 'symbion/freebox/connection/metrics') {
      this.connectionStatus = payload
    } else if (topic.startsWith('symbion/freebox/presence/') && !topic.endsWith('/state')) {
      // Individual device presence
      if (typeof payload === 'object' && payload.device_id) {
        const deviceIndex = this.presenceDevices.findIndex(d => d.device_id === payload.device_id)
        if (deviceIndex >= 0) {
          this.presenceDevices = [
            ...this.presenceDevices.slice(0, deviceIndex),
            payload,
            ...this.presenceDevices.slice(deviceIndex + 1)
          ]
        } else {
          this.presenceDevices = [...this.presenceDevices, payload]
        }
      }
    }
    this._checkDataLoaded()
    this.requestUpdate()
  }

  _getOverallStatus() {
    if (this.anyoneHome === null) return 'unknown'
    return this.anyoneHome ? 'home' : 'away'
  }

  _getStatusLabel() {
    const status = this._getOverallStatus()
    if (status === 'home') return 'A la maison'
    if (status === 'away') return 'Absent'
    return 'Inconnu'
  }

  _formatSpeed(kbps) {
    if (!kbps) return '0'
    if (kbps >= 1000) {
      return (kbps / 1000).toFixed(1)
    }
    return kbps.toFixed(0)
  }

  _getSpeedUnit(kbps) {
    return kbps >= 1000 ? 'Mb/s' : 'Kb/s'
  }

  render() {
    if (this.loading) {
      return html`
        <div class="loader-container">
          <organic-loader text="📡 Connexion Freebox..."></organic-loader>
        </div>
      `
    }

    const status = this._getOverallStatus()

    return html`
      <div class="widget-header">
        <span class="widget-title">Freebox</span>
        <span class="status-badge ${status}">${this._getStatusLabel()}</span>
      </div>

      <div class="content-grid">
        <!-- Presence Section -->
        <div class="section-card">
          <div class="section-title">
            <svg viewBox="0 0 24 24"><path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/></svg>
            Presence
          </div>
          ${this.presenceDevices.length === 0 ? html`
            <div class="fb-hint-muted">Aucun appareil configure</div>
          ` : this.presenceDevices.map(device => html`
            <div class="presence-item">
              <div class="device-info">
                <span class="device-icon">${device.device_type === 'phone' ? '📱' : '💻'}</span>
                <div>
                  <div class="device-name">${device.friendly_name || device.device_id}</div>
                  ${device.ip_address ? html`<div class="device-ip">${device.ip_address}</div>` : ''}
                </div>
              </div>
              <div class="presence-status">
                <span class="status-dot ${device.present ? 'home' : 'away'}"></span>
                <span class="presence-label ${device.present ? 'home' : 'away'}">
                  ${device.present ? 'Home' : 'Away'}
                </span>
              </div>
            </div>
          `)}
        </div>

        <!-- Connection Section -->
        <div class="section-card">
          <div class="section-title">
            <svg viewBox="0 0 24 24"><path d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3a4.237 4.237 0 0 0-6 0zm-4-4l2 2a7.074 7.074 0 0 1 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"/></svg>
            Connexion
          </div>
          ${!this.connectionStatus ? html`
            <div class="fb-hint-muted">En attente des donnees...</div>
          ` : html`
            <div class="connection-stats">
              <div class="stat-item">
                <div class="stat-value">
                  ${this._formatSpeed(this.connectionStatus.current_download_kbps)}
                  <span class="stat-unit">${this._getSpeedUnit(this.connectionStatus.current_download_kbps)}</span>
                </div>
                <div class="stat-label">Download</div>
              </div>
              <div class="stat-item">
                <div class="stat-value">
                  ${this._formatSpeed(this.connectionStatus.current_upload_kbps)}
                  <span class="stat-unit">${this._getSpeedUnit(this.connectionStatus.current_upload_kbps)}</span>
                </div>
                <div class="stat-label">Upload</div>
              </div>
            </div>
            <div class="connection-type">
              <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zM4 12c0-.61.08-1.21.21-1.78L8.99 15v1c0 1.1.9 2 2 2v1.93C7.06 19.43 4 16.07 4 12zm13.89 5.4c-.26-.81-1-1.4-1.9-1.4h-1v-3c0-.55-.45-1-1-1h-6v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41C17.92 5.77 20 8.65 20 12c0 2.08-.81 3.98-2.11 5.4z"/></svg>
              ${this.connectionStatus.type || 'FTTH'} - ${this.connectionStatus.download_mbps || 0} Mb/s
            </div>
          `}
        </div>
      </div>
    `
  }
}

customElements.define('freebox-widget', FreeboxWidget)

export { FreeboxWidget }
