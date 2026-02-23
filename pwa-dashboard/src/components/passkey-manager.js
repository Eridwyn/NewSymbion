/**
 * Passkey Manager Component - Gestion des Passkeys Biométriques
 *
 * Fonctionnalités :
 * - Enregistrement de nouvelles passkeys (Touch ID, Face ID, Windows Hello, etc.)
 * - Liste des passkeys enregistrées
 * - Suppression de passkeys
 *
 * Standards : WebAuthn (W3C), FIDO2
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import authService from '../services/auth-service.js'

class PasskeyManager extends LitElement {
  static styles = [sharedAnimations, css`
    :host {
      display: block;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    }

    .passkey-container {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md, 0.75rem);
      padding: 1.5rem;
    }

    .header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }

    .title {
      font-size: 1.3em;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .register-btn {
      background: linear-gradient(135deg, var(--ctx-bg-strong) 0%, var(--ctx-border) 100%);
      border: 1px solid var(--ctx-border-medium);
      color: var(--context-primary, #00d4aa);
      padding: 0.6rem 1.2rem;
      border-radius: var(--radius-base, 0.5rem);
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .register-btn:hover {
      background: linear-gradient(135deg, var(--ctx-bg-intense) 0%, var(--ctx-bg-emphasis) 100%);
      border-color: var(--ctx-border-intense);
      transform: translateY(-1px);
    }

    .register-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      transform: none;
    }

    .passkey-list {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .passkey-item {
      background: linear-gradient(135deg, var(--surface-glass) 0%, var(--surface-glass-faint) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base, 0.5rem);
      padding: 1rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      transition: all var(--duration-base) var(--ease-out);
    }

    .passkey-item:hover {
      border-color: var(--ctx-border-medium);
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, rgba(255, 255, 255, 0.04) 100%);
    }

    .passkey-info {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .passkey-icon {
      font-size: 2em;
    }

    .passkey-details {
      display: flex;
      flex-direction: column;
      gap: 0.3rem;
    }

    .passkey-name {
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .passkey-meta {
      font-size: 0.8em;
      opacity: 0.6;
    }

    .delete-btn {
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
      padding: 0.5rem 1rem;
      border-radius: var(--radius-sm);
      font-size: 0.85em;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
    }

    .delete-btn:hover {
      background: rgba(255, 107, 107, 0.2);
      border-color: rgba(255, 107, 107, 0.5);
    }

    .placeholder {
      text-align: center;
      padding: 2rem;
      opacity: 0.5;
      font-size: 0.9em;
    }

    .error {
      background: rgba(255, 107, 107, 0.1);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
      padding: 1rem;
      border-radius: var(--radius-base, 0.5rem);
      margin-bottom: 1rem;
    }

    .success {
      background: var(--ctx-bg-subtle);
      border: 1px solid var(--ctx-border-medium);
      color: var(--context-primary, #00d4aa);
      padding: 1rem;
      border-radius: var(--radius-base, 0.5rem);
      margin-bottom: 1rem;
    }

    .spinner {
      display: inline-block;
      width: 1em;
      height: 1em;
      border: 2px solid rgba(255, 255, 255, 0.3);
      border-top-color: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: spin 1s linear infinite;
    }

    /* spin — see shared-animations.js */

    /* Responsive */
    @media (max-width: 640px) {
      .passkey-container {
        padding: 1rem;
      }

      .header {
        flex-direction: column;
        align-items: stretch;
        gap: 0.75rem;
        margin-bottom: 1rem;
      }

      .title {
        font-size: 1.1em;
      }

      .register-btn {
        width: 100%;
        justify-content: center;
        padding: 0.6rem 1rem;
        font-size: 0.85em;
      }

      .passkey-item {
        flex-direction: column;
        align-items: stretch;
        gap: 0.75rem;
        padding: 0.75rem;
      }

      .passkey-info {
        gap: 0.75rem;
      }

      .passkey-icon {
        font-size: 1.5em;
      }

      .delete-btn {
        width: 100%;
        text-align: center;
        padding: 0.5rem;
      }
    }

    .pk-hint { opacity: 0.7; }
  `]

  static properties = {
    passkeys: { type: Array },
    apiService: { type: Object },
    loading: { type: Boolean },
    error: { type: String },
    success: { type: String },
    registering: { type: Boolean }
  }

  constructor() {
    super()
    this.passkeys = []
    this.apiService = null
    this.loading = false
    this.error = null
    this.success = null
    this.registering = false
    // Base URL dynamique (même logique que api-service)
    this.baseUrl = window.SYMBION_CONFIG?.API_BASE || window.location.origin
  }

  connectedCallback() {
    super.connectedCallback()
    this.apiService = document.querySelector('api-service')

    if (this.apiService) {
      this.loadPasskeys()
    } else {
      console.warn('[passkey-manager] API service not found')
    }
  }

  async loadPasskeys() {
    try {
      const response = await fetch(`${this.baseUrl}/auth/webauthn/passkeys`, {
        headers: {
          ...authService.getAuthHeader()
        }
      })

      if (!response.ok) {
        console.warn('[passkey-manager] Failed to load passkeys:', response.status)
        this.passkeys = []
        return
      }

      this.passkeys = await response.json()
      console.log('[passkey-manager] Loaded', this.passkeys.length, 'passkeys')
    } catch (error) {
      console.error('[passkey-manager] Error loading passkeys:', error)
      this.passkeys = []
    }
  }

  async registerPasskey() {
    if (!this.apiService) {
      this.error = 'Service API non disponible'
      return
    }

    // Vérifier support WebAuthn
    if (!window.PublicKeyCredential) {
      this.error = 'Votre navigateur ne supporte pas WebAuthn. Utilisez Chrome, Firefox, Safari ou Edge récent.'
      return
    }

    this.registering = true
    this.error = null
    this.success = null

    try {
      // Demander le nom de l'appareil
      const deviceName = this.detectDeviceName()
      const friendlyName = prompt(`Nom de cet appareil ?`, deviceName) || deviceName

      console.log('[passkey-manager] Starting registration for:', friendlyName)

      // Étape 1: Démarrer l'enregistrement (obtenir le challenge du serveur)
      const startResponse = await fetch(`${this.baseUrl}/auth/webauthn/register-start`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...authService.getAuthHeader()
        },
        body: JSON.stringify({ friendly_name: friendlyName })
      })

      if (!startResponse.ok) {
        const errorData = await startResponse.json()
        throw new Error(errorData.error || 'Failed to start registration')
      }

      const creationOptions = await startResponse.json()
      console.log('[passkey-manager] Creation options received:', creationOptions)

      // Convertir les champs base64 en ArrayBuffer
      const preparedOptions = this.prepareCreationOptions(creationOptions)
      console.log('[passkey-manager] Options prepared for browser:', preparedOptions)
      console.log('[passkey-manager] PublicKey options:', JSON.stringify({
        rp: creationOptions.publicKey.rp,
        user: { name: creationOptions.publicKey.user.name, displayName: creationOptions.publicKey.user.displayName },
        pubKeyCredParams: creationOptions.publicKey.pubKeyCredParams,
        authenticatorSelection: creationOptions.publicKey.authenticatorSelection,
        attestation: creationOptions.publicKey.attestation,
        timeout: creationOptions.publicKey.timeout
      }, null, 2))

      // Étape 2: Demander au navigateur de créer la passkey
      // L'utilisateur va devoir utiliser Touch ID, Face ID, Windows Hello, etc.
      console.log('[passkey-manager] Calling navigator.credentials.create()...')
      const credential = await Promise.race([
        navigator.credentials.create(preparedOptions),
        new Promise((_, reject) => setTimeout(() => reject(new Error('WebAuthn timeout (60s)')), 60000))
      ])
      console.log('[passkey-manager] Credential created successfully')

      console.log('[passkey-manager] Credential created:', credential)

      // Étape 3: Envoyer le credential au serveur pour validation
      const finishResponse = await fetch(`${this.baseUrl}/auth/webauthn/register-finish`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...authService.getAuthHeader()
        },
        body: JSON.stringify({
          friendly_name: friendlyName,
          credential: {
            id: credential.id,
            rawId: this.arrayBufferToBase64(credential.rawId),
            response: {
              attestationObject: this.arrayBufferToBase64(credential.response.attestationObject),
              clientDataJSON: this.arrayBufferToBase64(credential.response.clientDataJSON)
            },
            type: credential.type
          }
        })
      })

      if (!finishResponse.ok) {
        const errorData = await finishResponse.json()
        throw new Error(errorData.error || 'Failed to finish registration')
      }

      this.success = `✅ Passkey "${friendlyName}" enregistrée avec succès !`
      console.log('[passkey-manager] ✅ Registration successful')

      // Recharger la liste des passkeys
      await this.loadPasskeys()

    } catch (error) {
      console.error('[passkey-manager] Registration failed:', error)
      console.error('[passkey-manager] Error name:', error.name)
      console.error('[passkey-manager] Error message:', error.message)
      console.error('[passkey-manager] Error stack:', error.stack)

      if (error.name === 'NotAllowedError') {
        this.error = 'Enregistrement annulé ou échoué. Vérifiez que vous avez bien utilisé votre biométrie.'
      } else if (error.name === 'InvalidStateError') {
        this.error = 'Cette passkey existe déjà. Utilisez un nom différent.'
      } else if (error.name === 'NotSupportedError') {
        this.error = 'Authenticator non supporté. Vérifiez que Windows Hello est activé et configuré.'
      } else {
        this.error = `Erreur (${error.name}): ${error.message}`
      }
    } finally {
      this.registering = false
    }
  }

  detectDeviceName() {
    const ua = navigator.userAgent

    if (/iPhone/.test(ua)) return 'iPhone'
    if (/iPad/.test(ua)) return 'iPad'
    if (/Android/.test(ua)) return 'Android'
    if (/Windows/.test(ua)) return 'Windows PC'
    if (/Mac/.test(ua)) return 'Mac'
    if (/Linux/.test(ua)) return 'Linux PC'

    return 'Mon appareil'
  }

  arrayBufferToBase64(buffer) {
    const bytes = new Uint8Array(buffer)
    let binary = ''
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i])
    }
    return btoa(binary)
  }

  base64ToArrayBuffer(base64) {
    // Decode base64 (handle URL-safe base64)
    const binaryString = atob(base64.replace(/-/g, '+').replace(/_/g, '/'))
    const bytes = new Uint8Array(binaryString.length)
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }
    return bytes.buffer
  }

  // Convertir les champs base64 en ArrayBuffer pour WebAuthn
  prepareCreationOptions(options) {
    return {
      publicKey: {
        ...options.publicKey,
        challenge: this.base64ToArrayBuffer(options.publicKey.challenge),
        user: {
          ...options.publicKey.user,
          id: this.base64ToArrayBuffer(options.publicKey.user.id)
        },
        excludeCredentials: options.publicKey.excludeCredentials?.map(cred => ({
          ...cred,
          id: this.base64ToArrayBuffer(cred.id)
        }))
      }
    }
  }

  formatDate(timestamp) {
    if (!timestamp) return ''
    const date = new Date(timestamp * 1000)
    return date.toLocaleDateString('fr-FR', {
      day: '2-digit',
      month: 'short',
      year: 'numeric'
    })
  }

  render() {
    return html`
      <div class="passkey-container">
        <div class="header">
          <h2 class="title">
            🔐 Connexion Biométrique
          </h2>
          <button
            class="register-btn"
            @click="${this.registerPasskey}"
            ?disabled="${this.registering}">
            ${this.registering ? html`<span class="spinner"></span>` : '➕'}
            ${this.registering ? 'Enregistrement...' : 'Ajouter une Passkey'}
          </button>
        </div>

        ${this.error ? html`
          <div class="error">⚠️ ${this.error}</div>
        ` : ''}

        ${this.success ? html`
          <div class="success">${this.success}</div>
        ` : ''}

        <div class="passkey-list">
          ${this.passkeys.length === 0 ? html`
            <div class="placeholder">
              Aucune passkey enregistrée. Ajoutez-en une pour activer la connexion biométrique rapide !
              <br><br>
              <small class="pk-hint">
                Compatible : Touch ID, Face ID, Windows Hello, empreintes Android
              </small>
            </div>
          ` : this.passkeys.map(passkey => html`
            <div class="passkey-item">
              <div class="passkey-info">
                <div class="passkey-icon">
                  ${this.getDeviceIcon(passkey.friendly_name)}
                </div>
                <div class="passkey-details">
                  <div class="passkey-name">${passkey.friendly_name}</div>
                  <div class="passkey-meta">
                    Ajoutée le ${this.formatDate(passkey.created_at)}
                    ${passkey.last_used_at ? ` • Dernière utilisation : ${this.formatDate(passkey.last_used_at)}` : ''}
                  </div>
                </div>
              </div>
              <button
                class="delete-btn"
                @click="${() => this.deletePasskey(passkey.credential_id)}">
                Supprimer
              </button>
            </div>
          `)}
        </div>
      </div>
    `
  }

  getDeviceIcon(friendlyName) {
    const name = friendlyName.toLowerCase()

    if (name.includes('iphone')) return '📱'
    if (name.includes('ipad')) return '📱'
    if (name.includes('android')) return '📱'
    if (name.includes('windows')) return '💻'
    if (name.includes('mac')) return '💻'
    if (name.includes('linux')) return '💻'

    return '🔑'
  }

  async deletePasskey(credentialId) {
    if (!confirm('Êtes-vous sûr de vouloir supprimer cette passkey ?')) {
      return
    }

    this.error = null
    this.success = null

    try {
      console.log('[passkey-manager] Deleting passkey:', credentialId)

      const response = await fetch(`${this.baseUrl}/auth/webauthn/passkeys/${encodeURIComponent(credentialId)}`, {
        method: 'DELETE',
        headers: {
          ...authService.getAuthHeader()
        }
      })

      if (!response.ok) {
        const errorData = await response.json()
        throw new Error(errorData.error || 'Failed to delete passkey')
      }

      this.success = '✅ Passkey supprimée avec succès'
      console.log('[passkey-manager] ✅ Passkey deleted successfully')

      // Recharger la liste
      await this.loadPasskeys()

    } catch (error) {
      console.error('[passkey-manager] Delete failed:', error)
      this.error = `Erreur lors de la suppression: ${error.message}`
    }
  }
}

customElements.define('passkey-manager', PasskeyManager)
