/**
 * Page Paramètres Utilisateur Symbion
 *
 * Interface de configuration utilisateur:
 * - Profil utilisateur
 * - Sécurité (changement mot de passe)
 * - MFA/TOTP (activation, désactivation, codes backup)
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import authService from '../services/auth-service.js'
import decisionService from '../services/decision-service.js'
import './passkey-manager.js'

class UserSettingsPage extends LitElement {
  static styles = [sharedAnimations, css`
    /* Overlay bio-organique avec glassmorphism */
    :host {
      display: block;
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: radial-gradient(ellipse at center,
        color-mix(in srgb, var(--context-primary, #00d4aa) 3%, rgba(0, 0, 0, 0.92)) 0%,
        rgba(0, 0, 0, 0.95) 100%);
      backdrop-filter: blur(var(--blur-xl));
      -webkit-backdrop-filter: blur(var(--blur-xl));
      z-index: 9999;
      overflow-y: auto;
      animation: fadeIn var(--duration-slow) var(--ease-out);
    }

    .settings-container {
      max-width: 800px;
      margin: var(--space-6) auto;
      padding: var(--space-6);
      overflow-x: hidden; /* Empêche scrollbar horizontal */
      animation: slideUp var(--duration-slow) var(--ease-out);
    }

    /* Header bio-organique */
    .settings-header {
      position: relative;
      margin-bottom: var(--space-6);
      padding-bottom: var(--space-4);
      padding-right: 120px; /* Espace pour le bouton fermer */
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
    }

    .settings-header::after {
      content: '';
      position: absolute;
      bottom: -1px;
      left: 0;
      width: 30%;
      height: 2px;
      background: linear-gradient(90deg,
        var(--context-primary, #00d4aa) 0%,
        transparent 100%);
      opacity: 0.8;
    }

    .settings-title {
      font-size: var(--text-3xl);
      font-weight: var(--font-bold);
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white) 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      filter: drop-shadow(0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent));
      animation: titlePulse 4s ease-in-out infinite, titleSlideIn 0.6s var(--ease-out);
    }

    @keyframes titleSlideIn {
      from {
        opacity: 0;
        transform: translateX(-20px);
      }
      to {
        opacity: 1;
        transform: translateX(0);
      }
    }

    .close-button {
      position: absolute;
      top: 0;
      right: 0;
      background: linear-gradient(135deg,
        rgba(239, 68, 68, 0.15) 0%,
        rgba(239, 68, 68, 0.08) 100%);
      border: 1px solid rgba(239, 68, 68, 0.35);
      color: #ff6b6b;
      padding: var(--space-3) var(--space-5);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-semibold);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
    }

    .close-button:hover {
      background: linear-gradient(135deg,
        rgba(239, 68, 68, 0.25) 0%,
        rgba(239, 68, 68, 0.15) 100%);
      border-color: rgba(239, 68, 68, 0.55);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px rgba(239, 68, 68, 0.3);
    }

    .tabs {
      display: flex;
      gap: var(--space-2);
      margin-bottom: var(--space-6);
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .tab {
      background: transparent;
      border: none;
      color: var(--color-dark-text-tertiary);
      padding: var(--space-3) var(--space-4);
      font-size: var(--text-sm);
      font-weight: var(--font-medium);
      cursor: pointer;
      border-bottom: 2px solid transparent;
      transition: all var(--duration-base) var(--ease-out);
      position: relative;
    }

    .tab::before {
      content: '';
      position: absolute;
      bottom: -2px;
      left: 50%;
      width: 0;
      height: 2px;
      background: var(--context-primary, #00d4aa);
      transform: translateX(-50%);
      transition: width var(--duration-base) var(--ease-out);
      box-shadow: 0 0 8px var(--context-primary, #00d4aa);
    }

    .tab:hover {
      color: var(--color-dark-text-secondary);
      transform: translateY(-1px);
    }

    .tab:hover::before {
      width: 60%;
    }

    .tab.active {
      color: var(--context-primary, #00d4aa);
      border-bottom-color: transparent;
    }

    .tab.active::before {
      width: 100%;
      box-shadow: 0 0 12px var(--context-primary, #00d4aa);
    }

    .tab-content {
      display: none;
    }

    .tab-content.active {
      display: block;
      animation: tabContentSlideIn 0.5s var(--ease-out);
    }

    @keyframes tabContentSlideIn {
      from {
        opacity: 0;
        transform: translateY(20px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    /* Staggered animation pour les enfants de tab-content */
    .tab-content.active > .section {
      animation: sectionStagger 0.6s var(--ease-out) backwards;
    }

    .tab-content.active > .section:nth-child(1) {
      animation-delay: 0.05s;
    }

    .tab-content.active > .section:nth-child(2) {
      animation-delay: 0.1s;
    }

    .tab-content.active > .section:nth-child(3) {
      animation-delay: 0.15s;
    }

    .tab-content.active > .section:nth-child(4) {
      animation-delay: 0.2s;
    }

    @keyframes sectionStagger {
      from {
        opacity: 0;
        transform: translateY(15px) scale(0.98);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    /* Section bio-organique comme les widgets */
    .section {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 3%, rgba(19, 20, 26, 0.95)) 0%,
        rgba(15, 15, 15, 0.9) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      border-radius: var(--radius-lg);
      padding: var(--space-5);
      margin-bottom: var(--space-5);
      backdrop-filter: blur(var(--blur-base));
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 6%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 4%, transparent);
      transition: all var(--duration-base) var(--ease-out);
      overflow: hidden; /* Empêche le débordement */
      animation: sectionPulse 8s ease-in-out infinite; /* Respiration subtile */
    }

    .section:hover {
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 18%, transparent);
      box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                  0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
    }

    @keyframes sectionPulse {
      0%, 100% {
        border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      }
      50% {
        border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 16%, transparent);
      }
    }

    .section-title {
      font-size: var(--text-lg);
      font-weight: var(--font-semibold);
      color: var(--context-primary, #00d4aa);
      margin-bottom: var(--space-3);
      display: flex;
      align-items: center;
      gap: var(--space-2);
    }

    .section-description {
      color: var(--color-dark-text-secondary);
      font-size: var(--text-sm);
      margin-bottom: var(--space-4);
      line-height: var(--leading-normal);
    }

    .info-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.8rem 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .info-row:last-child {
      border-bottom: none;
    }

    .toggle-switch {
      position: relative;
      width: 44px;
      height: 24px;
      background: rgba(255,255,255,0.1);
      border-radius: var(--radius-md);
      cursor: pointer;
      transition: background 0.3s;
      border: none;
      padding: 0;
    }

    .toggle-switch.active {
      background: rgba(99, 102, 241, 0.5);
    }

    .toggle-switch::after {
      content: '';
      position: absolute;
      top: 3px;
      left: 3px;
      width: 18px;
      height: 18px;
      background: #e0e0e0;
      border-radius: 50%;
      transition: transform 0.3s;
    }

    .toggle-switch.active::after {
      transform: translateX(20px);
      background: #818cf8;
    }

    .info-label {
      color: #888;
      font-size: 0.9em;
    }

    .info-value {
      color: #e0e0e0;
      font-weight: 500;
    }

    .status-badge {
      padding: 0.3rem 0.8rem;
      border-radius: 20px;
      font-size: 0.8em;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .status-badge.enabled {
      background: rgba(76, 175, 80, 0.2);
      color: #4caf50;
      border: 1px solid rgba(76, 175, 80, 0.4);
    }

    .status-badge.disabled {
      background: rgba(255, 107, 107, 0.15);
      color: #ff6b6b;
      border: 1px solid rgba(255, 107, 107, 0.3);
    }

    .button {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      color: var(--context-primary, #00d4aa);
      padding: 0.8rem 1.5rem;
      border-radius: var(--radius-base);
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s ease;
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      position: relative;
      overflow: hidden;
    }

    .button::before {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: 0;
      height: 0;
      border-radius: 50%;
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      transform: translate(-50%, -50%);
      transition: width 0.6s ease, height 0.6s ease;
    }

    .button:hover::before {
      width: 300px;
      height: 300px;
    }

    .button:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
    }

    .button:active {
      transform: translateY(0) scale(0.98); /* Feedback tactile */
    }

    .button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      transform: none;
    }

    .button:disabled::before {
      display: none;
    }

    .button.danger {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.15) 0%, rgba(239, 68, 68, 0.1) 100%);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
    }

    .button.danger:hover {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.25) 0%, rgba(239, 68, 68, 0.2) 100%);
      border-color: rgba(255, 107, 107, 0.5);
      box-shadow: 0 4px 12px rgba(255, 107, 107, 0.3);
    }

    /* Container MFA setup - Style contextuel */
    .mfa-setup-container {
      margin-top: var(--space-4);
      padding: var(--space-4);
      max-width: 100%; /* Empêche débordement */
      overflow: hidden; /* Force wrapping du contenu */
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 2%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      border-radius: var(--radius-md);
      box-shadow: 0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
      animation: containerGlow 6s ease-in-out infinite; /* Glow pulsant */
    }

    @keyframes containerGlow {
      0%, 100% {
        box-shadow: 0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent),
                    inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
      }
      50% {
        box-shadow: 0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                    inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      }
    }

    .qr-code-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 1rem;
      margin: 1.5rem 0;
    }

    .qr-code {
      padding: 1rem;
      background: white;
      border-radius: var(--radius-base);
    }

    .secret-display {
      background: rgba(0, 0, 0, 0.3);
      border: 1px solid rgba(255, 255, 255, 0.2);
      padding: 1rem;
      border-radius: var(--radius-base);
      font-family: 'Courier New', monospace;
      font-size: 1.1em;
      letter-spacing: 2px;
      text-align: center;
      color: var(--context-primary, #00d4aa);
      word-break: break-all;
    }

    .input-group {
      margin: var(--space-4) 0;
      max-width: 100%; /* Force containment */
      overflow: hidden; /* Empêche débordement */
    }

    .input-label {
      color: var(--color-dark-text-secondary);
      font-size: var(--text-sm);
      margin-bottom: var(--space-2);
      display: block;
      font-weight: var(--font-medium);
      animation: labelFadeIn 0.4s ease-out; /* Apparition douce */
    }

    @keyframes labelFadeIn {
      from {
        opacity: 0;
        transform: translateY(-4px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    /* Inputs avec focus bioluminescent */
    .input {
      width: 100%;
      max-width: 100%; /* CRITIQUE: Empêche débordement horizontal */
      min-width: 0; /* Permet rétrécissement si nécessaire */
      box-sizing: border-box; /* Padding inclus dans width */
      background: linear-gradient(135deg,
        rgba(0, 0, 0, 0.4) 0%,
        rgba(0, 0, 0, 0.3) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      color: var(--color-dark-text-primary);
      padding: var(--space-3) var(--space-4);
      border-radius: var(--radius-md);
      font-size: var(--text-base);
      font-family: var(--font-sans);
      transition: all var(--duration-base) var(--ease-out);
    }

    .input:focus {
      outline: none;
      background: rgba(0, 0, 0, 0.5);
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      animation: inputGlow 0.6s ease-out; /* Pulse au focus */
    }

    .input:hover:not(:focus) {
      border-color: rgba(255, 255, 255, 0.2);
      transform: translateY(-1px); /* Légère élévation */
    }

    .backup-codes {
      margin-top: 1.5rem;
      padding: 1rem;
      background: rgba(255, 193, 7, 0.05);
      border: 1px solid rgba(255, 193, 7, 0.3);
      border-radius: var(--radius-base);
    }

    .backup-codes-title {
      color: #ffc107;
      font-weight: 600;
      margin-bottom: 1rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .backup-codes-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 0.5rem;
    }

    .backup-code {
      background: rgba(0, 0, 0, 0.3);
      padding: 0.6rem;
      border-radius: 4px;
      font-family: 'Courier New', monospace;
      font-size: 0.9em;
      text-align: center;
      color: #ffc107;
    }

    .alert {
      padding: 1rem;
      border-radius: var(--radius-base);
      margin-bottom: 1rem;
      display: flex;
      align-items: flex-start;
      gap: 0.8rem;
    }

    .alert.success {
      background: rgba(76, 175, 80, 0.15);
      border: 1px solid rgba(76, 175, 80, 0.3);
      color: #4caf50;
    }

    .alert.error {
      background: rgba(255, 107, 107, 0.15);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: #ff6b6b;
    }

    .alert.warning {
      background: rgba(255, 193, 7, 0.15);
      border: 1px solid rgba(255, 193, 7, 0.3);
      color: #ffc107;
    }

    .loading {
      text-align: center;
      padding: 2rem;
      color: #888;
    }

    .spinner {
      display: inline-block;
      width: 40px;
      height: 40px;
      border: 3px solid rgba(255, 255, 255, 0.1);
      border-top-color: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }

    /* Mobile responsive - Compact */
    @media (max-width: 768px) {
      :host {
        padding: var(--space-2);
      }

      .settings-container {
        padding: var(--space-3);
        margin: var(--space-2);
        max-width: 100%;
      }

      .settings-header {
        padding-right: 0; /* Reset padding-right sur mobile */
        padding-bottom: var(--space-6); /* Plus d'espace pour le bouton */
      }

      .settings-title {
        font-size: var(--text-xl);
        max-width: calc(100% - 90px); /* Espace pour le bouton */
      }

      .close-button {
        padding: var(--space-2) var(--space-3);
        font-size: var(--text-xs);
        /* Reste en absolute top-right */
      }

      .tabs {
        overflow-x: auto;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none;
        margin-bottom: var(--space-4);
      }

      .tabs::-webkit-scrollbar {
        display: none;
      }

      .tab {
        white-space: nowrap;
        flex-shrink: 0;
        padding: var(--space-2) var(--space-3);
        font-size: var(--text-xs);
      }

      .section {
        padding: var(--space-4);
        margin-bottom: var(--space-4);
      }

      .mfa-setup-container {
        padding: var(--space-3);
      }

      .input-group {
        margin: var(--space-3) 0;
      }
    }
  `]

  static properties = {
    activeTab: { type: String },
    mfaStatus: { type: Object },
    mfaSetupData: { type: Object },
    loading: { type: Boolean },
    message: { type: Object }, // { type: 'success'|'error'|'warning', text: string }
    verifyCode: { type: String },
    users: { type: Array },
    newUser: { type: Object },
    // PR3 Décisions
    validations: { type: Array },
    expiredValidations: { type: Array },
    overrides: { type: Array },
    stats: { type: Object }
  }

  constructor() {
    super()
    // Restaurer le dernier onglet actif depuis sessionStorage (persiste aux reloads, reset à la fermeture du navigateur)
    this.activeTab = sessionStorage.getItem('userSettingsTab') || 'profil'
    this.mfaStatus = null
    this.mfaSetupData = null
    this.loading = false
    this.message = null
    this.verifyCode = ''
    this.users = []
    this.newUser = { username: '', password: '', role: 'admin' }
    // PR3 Décisions
    this.validations = []
    this.expiredValidations = []
    this.overrides = []
    this.stats = null
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadMfaStatus()
    this.loadUsers()

    // Charger données PR3 si onglet Décisions est actif au chargement
    if (this.activeTab === 'decisions') {
      this.loadDecisionsData()
    }
  }

  switchTab(tab) {
    this.activeTab = tab
    sessionStorage.setItem('userSettingsTab', tab)

    // Charger données PR3 si onglet Décisions activé
    // Rafraîchit toujours les données pour avoir les dernières validations/overrides
    if (tab === 'decisions') {
      this.loadDecisionsData()
    }
  }

  async loadMfaStatus() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        console.error('[settings] API service not available')
        return
      }

      this.loading = true
      this.mfaStatus = await apiService.request('/auth/mfa/status')
      console.log('[settings] MFA status loaded:', this.mfaStatus)
    } catch (error) {
      console.error('[settings] Failed to load MFA status:', error)
      this.showMessage('error', 'Impossible de charger l\'état MFA')
    } finally {
      this.loading = false
    }
  }

  async handleMfaSetup() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) throw new Error('API service not available')

      this.loading = true
      this.message = null

      const response = await apiService.request('/auth/mfa/setup', {
        method: 'POST',
        body: JSON.stringify({})  // Body vide requis par backend
      })

      this.mfaSetupData = response
      console.log('[settings] MFA setup initiated:', response)
      this.showMessage('success', 'Scannez le QR code avec votre application d\'authentification')
    } catch (error) {
      console.error('[settings] MFA setup failed:', error)
      this.showMessage('error', 'Échec de l\'initialisation MFA: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handleMfaVerify() {
    if (!this.verifyCode || this.verifyCode.length !== 6) {
      this.showMessage('error', 'Le code doit contenir 6 chiffres')
      return
    }

    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) throw new Error('API service not available')

      this.loading = true
      this.message = null

      const response = await apiService.request('/auth/mfa/verify', {
        method: 'POST',
        body: JSON.stringify({ code: this.verifyCode })
      })

      console.log('[settings] MFA verification response:', response)

      if (response.success) {
        this.showMessage('success', '✅ MFA activé avec succès !')
        this.verifyCode = ''

        // Recharger le status MFA
        await this.loadMfaStatus()

        // Effacer les données de setup
        this.mfaSetupData = null
      } else {
        this.showMessage('error', 'Code invalide, veuillez réessayer')
      }
    } catch (error) {
      console.error('[settings] MFA verification failed:', error)
      this.showMessage('error', 'Vérification échouée: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handleMfaDisable() {
    if (!confirm('Êtes-vous sûr de vouloir désactiver l\'authentification à deux facteurs ?')) {
      return
    }

    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) throw new Error('API service not available')

      this.loading = true
      this.message = null

      await apiService.request('/auth/mfa/disable', {
        method: 'POST'
      })

      this.showMessage('success', 'MFA désactivé avec succès')

      // Recharger le status MFA
      await this.loadMfaStatus()
    } catch (error) {
      console.error('[settings] MFA disable failed:', error)
      this.showMessage('error', 'Échec de la désactivation: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  showMessage(type, text) {
    this.message = { type, text }
    // Auto-clear success messages after 5 seconds
    if (type === 'success') {
      setTimeout(() => {
        if (this.message?.text === text) {
          this.message = null
        }
      }, 5000)
    }
  }

  async loadUsers() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) {
        console.warn('[settings] API service not available')
        return
      }

      const users = await apiService.request('/v1/users')
      this.users = users || []
      console.log('[settings] Loaded users:', this.users)
    } catch (error) {
      console.error('[settings] Failed to load users:', error)
      this.users = []
    }
  }

  async loadDecisionsData() {
    try {
      this.loading = true

      // Charger validations en attente
      const validationsData = await decisionService.getPendingValidations()
      this.validations = validationsData || []

      // Charger validations expirées
      const expiredData = await decisionService.getExpiredValidations()
      this.expiredValidations = expiredData || []

      // Charger overrides actifs
      const overridesData = await decisionService.getActiveOverrides()
      this.overrides = overridesData || []

      // Charger statistiques et transformer au format attendu par le frontend
      const rawStats = await decisionService.getStats()
      this.stats = {
        total_evaluations: rawStats.audit?.total_records || 0,
        approved: rawStats.validation?.approved || 0,
        rejected: rawStats.validation?.denied || 0,
        pending: rawStats.validation?.pending || 0
      }

      console.log('[settings] Loaded PR3 data:', {
        validations: this.validations.length,
        expiredValidations: this.expiredValidations.length,
        overrides: this.overrides.length,
        stats: this.stats
      })
    } catch (error) {
      console.error('[settings] Failed to load decisions data:', error)
      this.showMessage('error', 'Impossible de charger les données de décisions')
    } finally {
      this.loading = false
    }
  }

  async handleCreateUser() {
    if (!this.newUser.username || !this.newUser.password) {
      this.showMessage('error', 'Nom d\'utilisateur et mot de passe requis')
      return
    }

    try {
      const apiService = document.querySelector('api-service')
      const csrfService = (await import('../services/csrf-service.js')).default
      if (!apiService) throw new Error('API service not available')

      // Initialiser csrfService avec authService si nécessaire
      if (!csrfService.authService) {
        const authServiceModule = await import('../services/auth-service.js')
        csrfService.setAuthService(authServiceModule.default)
      }

      this.loading = true
      this.message = null

      const url = `${apiService.baseUrl}/v1/users`
      const response = await csrfService.fetchWithCsrf(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(this.newUser)
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }

      this.showMessage('success', `Utilisateur "${this.newUser.username}" créé avec succès`)
      this.newUser = { username: '', password: '', role: 'admin' }
      await this.loadUsers()
    } catch (error) {
      console.error('[settings] Failed to create user:', error)
      this.showMessage('error', 'Échec de la création: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handleDeleteUser(username) {
    if (!confirm(`Supprimer l'utilisateur "${username}" ?`)) {
      return
    }

    try {
      const apiService = document.querySelector('api-service')
      const csrfService = (await import('../services/csrf-service.js')).default
      if (!apiService) throw new Error('API service not available')

      // Initialiser csrfService avec authService si nécessaire
      if (!csrfService.authService) {
        const authServiceModule = await import('../services/auth-service.js')
        csrfService.setAuthService(authServiceModule.default)
      }

      this.loading = true
      this.message = null

      const url = `${apiService.baseUrl}/v1/users/${encodeURIComponent(username)}`
      const response = await csrfService.fetchWithCsrf(url, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' }
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }

      this.showMessage('success', `Utilisateur "${username}" supprimé`)
      await this.loadUsers()
    } catch (error) {
      console.error('[settings] Failed to delete user:', error)
      this.showMessage('error', 'Échec de la suppression: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handlePasswordChange() {
    const currentPassword = this.shadowRoot.getElementById('current-password').value
    const newPassword = this.shadowRoot.getElementById('new-password').value
    const confirmPassword = this.shadowRoot.getElementById('confirm-password').value

    // Validations
    if (!currentPassword || !newPassword || !confirmPassword) {
      this.showMessage('error', 'Tous les champs sont requis')
      return
    }

    if (newPassword !== confirmPassword) {
      this.showMessage('error', 'Les nouveaux mots de passe ne correspondent pas')
      return
    }

    if (newPassword.length < 8) {
      this.showMessage('error', 'Le nouveau mot de passe doit contenir au moins 8 caractères')
      return
    }

    if (newPassword === currentPassword) {
      this.showMessage('error', 'Le nouveau mot de passe doit être différent de l\'ancien')
      return
    }

    try {
      const apiService = document.querySelector('api-service')
      const csrfService = (await import('../services/csrf-service.js')).default
      if (!apiService) throw new Error('API service not available')

      // Initialiser csrfService avec authService si nécessaire
      if (!csrfService.authService) {
        const authServiceModule = await import('../services/auth-service.js')
        csrfService.setAuthService(authServiceModule.default)
      }

      this.loading = true
      this.message = null

      const currentUser = authService.getCurrentUser()
      const url = `${apiService.baseUrl}/v1/users/${encodeURIComponent(currentUser.username)}/password`
      const response = await csrfService.fetchWithCsrf(url, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword
        })
      })

      if (!response.ok) {
        if (response.status === 401) {
          throw new Error('Mot de passe actuel incorrect')
        }
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }

      this.showMessage('success', '✅ Mot de passe changé avec succès !')

      // Vider les champs
      this.shadowRoot.getElementById('current-password').value = ''
      this.shadowRoot.getElementById('new-password').value = ''
      this.shadowRoot.getElementById('confirm-password').value = ''
    } catch (error) {
      console.error('[settings] Failed to change password:', error)
      this.showMessage('error', 'Échec du changement: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  handleClose() {
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }))
  }

  render() {
    const currentUser = authService.getCurrentUser()

    return html`
      <div class="settings-container">
        <div class="settings-header">
          <h1 class="settings-title">⚙️ Paramètres</h1>
          <button class="close-button" @click="${this.handleClose}">
            ✕ Fermer
          </button>
        </div>

        <div class="tabs">
          <button class="tab ${this.activeTab === 'profil' ? 'active' : ''}"
                  @click="${() => this.switchTab('profil')}">
            👤 Profil
          </button>
          <button class="tab ${this.activeTab === 'securite' ? 'active' : ''}"
                  @click="${() => this.switchTab('securite')}">
            🔒 Sécurité
          </button>
          <button class="tab ${this.activeTab === 'passkeys' ? 'active' : ''}"
                  @click="${() => this.switchTab('passkeys')}">
            🔐 Passkeys
          </button>
          <button class="tab ${this.activeTab === 'mfa' ? 'active' : ''}"
                  @click="${() => this.switchTab('mfa')}">
            🛡️ Authentification 2FA
          </button>
          <button class="tab ${this.activeTab === 'users' ? 'active' : ''}"
                  @click="${() => this.switchTab('users')}">
            👥 Utilisateurs
          </button>
          <button class="tab ${this.activeTab === 'decisions' ? 'active' : ''}"
                  @click="${() => this.switchTab('decisions')}">
            ⚖️ Décisions
          </button>
        </div>

        ${this.message ? html`
          <div class="alert ${this.message.type}">
            <span>${this.message.type === 'success' ? '✅' : this.message.type === 'error' ? '❌' : '⚠️'}</span>
            <span>${this.message.text}</span>
          </div>
        ` : ''}

        <!-- Tab Profil -->
        <div class="tab-content ${this.activeTab === 'profil' ? 'active' : ''}">
          <div class="section">
            <h2 class="section-title">👤 Informations Utilisateur</h2>
            <div class="info-row">
              <span class="info-label">Nom d'utilisateur</span>
              <span class="info-value">${currentUser?.username || 'Non connecté'}</span>
            </div>
            <div class="info-row">
              <span class="info-label">Rôle</span>
              <span class="info-value">${currentUser?.role || 'N/A'}</span>
            </div>
            <div class="info-row">
              <span class="info-label">Durée de session</span>
              <span class="info-value">${this.getSessionDuration()}</span>
            </div>
          </div>

          <div class="section" style="margin-top: 1.5rem;">
            <h2 class="section-title">Avance</h2>
            <div class="info-row">
              <span class="info-label">Afficher icone Logs</span>
              <button class="toggle-switch ${this._isLogsEnabled() ? 'active' : ''}"
                      @click="${this._toggleLogs}"
                      title="Affiche une icone en bas du dashboard pour ouvrir le log viewer"></button>
            </div>
          </div>
        </div>

        <!-- Tab Sécurité -->
        <div class="tab-content ${this.activeTab === 'securite' ? 'active' : ''}">
          <div class="section">
            <h2 class="section-title">🔒 Changement de Mot de Passe</h2>
            <p class="section-description">
              Modifiez votre mot de passe pour renforcer la sécurité de votre compte.
            </p>

            <div class="mfa-setup-container">
              <div class="input-group">
                <label class="input-label">Mot de passe actuel</label>
                <input
                  type="password"
                  class="input"
                  id="current-password"
                  placeholder="••••••••"
                  autocomplete="current-password"
                />
              </div>

              <div class="input-group">
                <label class="input-label">Nouveau mot de passe</label>
                <input
                  type="password"
                  class="input"
                  id="new-password"
                  placeholder="••••••••"
                  autocomplete="new-password"
                />
              </div>

              <div class="input-group">
                <label class="input-label">Confirmer le nouveau mot de passe</label>
                <input
                  type="password"
                  class="input"
                  id="confirm-password"
                  placeholder="••••••••"
                  autocomplete="new-password"
                />
              </div>

              <button
                class="button"
                @click="${this.handlePasswordChange}"
                ?disabled="${this.loading}"
              >
                ${this.loading ? '⏳ Modification...' : '✓ Changer le mot de passe'}
              </button>
            </div>
          </div>
        </div>

        <!-- Tab Passkeys -->
        <div class="tab-content ${this.activeTab === 'passkeys' ? 'active' : ''}">
          <passkey-manager></passkey-manager>
        </div>

        <!-- Tab MFA -->
        <div class="tab-content ${this.activeTab === 'mfa' ? 'active' : ''}">
          ${this.renderMfaTab()}
        </div>

        <!-- Tab Utilisateurs -->
        <div class="tab-content ${this.activeTab === 'users' ? 'active' : ''}">
          ${this.renderUsersTab()}
        </div>

        <!-- Tab Décisions -->
        <div class="tab-content ${this.activeTab === 'decisions' ? 'active' : ''}">
          ${this.renderDecisionsTab()}
        </div>
      </div>
    `
  }

  renderMfaTab() {
    if (this.loading && !this.mfaStatus) {
      return html`
        <div class="loading">
          <div class="spinner"></div>
          <p>Chargement...</p>
        </div>
      `
    }

    const isMfaEnabled = this.mfaStatus?.enabled || false

    return html`
      <div class="section">
        <h2 class="section-title">🛡️ Authentification à Deux Facteurs (TOTP)</h2>
        <p class="section-description">
          L'authentification à deux facteurs ajoute une couche de sécurité supplémentaire à votre compte.
          Vous devrez fournir un code généré par une application comme Google Authenticator lors de la connexion.
        </p>

        <div class="info-row">
          <span class="info-label">État MFA</span>
          <span class="status-badge ${isMfaEnabled ? 'enabled' : 'disabled'}">
            ${isMfaEnabled ? '✓ Activé' : '✗ Désactivé'}
          </span>
        </div>

        ${!isMfaEnabled ? html`
          ${!this.mfaSetupData ? html`
            <!-- MFA non activé, proposer l'activation -->
            <div class="mfa-setup-container">
              <h3 style="color: var(--context-primary, #00d4aa); margin-bottom: 1rem;">
                📱 Activer l'authentification à deux facteurs
              </h3>
              <p style="color: #aaa; margin-bottom: 1rem;">
                Vous aurez besoin d'une application d'authentification compatible TOTP comme:
              </p>
              <ul style="color: #aaa; margin-left: 1.5rem; margin-bottom: 1.5rem;">
                <li>Google Authenticator</li>
                <li>Microsoft Authenticator</li>
                <li>Authy</li>
              </ul>
              <button class="button" @click="${this.handleMfaSetup}" ?disabled="${this.loading}">
                ${this.loading ? '⏳ Chargement...' : '🚀 Commencer l\'activation'}
              </button>
            </div>
          ` : html`
            <!-- Étape d'activation MFA: scan QR + vérification -->
            <div class="mfa-setup-container">
              <h3 style="color: var(--context-primary, #00d4aa); margin-bottom: 1rem;">
                📱 Scannez ce QR code
              </h3>

              <div class="qr-code-container">
                <div class="qr-code">
                  <img src="${this.mfaSetupData.qr_code}" alt="QR Code TOTP" style="display: block; width: 200px; height: 200px;" />
                </div>

                <div style="text-align: center; width: 100%;">
                  <p style="color: #aaa; margin-bottom: 0.5rem;">Ou entrez manuellement ce secret:</p>
                  <div class="secret-display">
                    ${this.mfaSetupData.secret}
                  </div>
                </div>
              </div>

              <div class="input-group">
                <label class="input-label">Entrez le code à 6 chiffres généré par l'application</label>
                <input
                  type="text"
                  class="input"
                  placeholder="000000"
                  maxlength="6"
                  pattern="[0-9]{6}"
                  .value="${this.verifyCode}"
                  @input="${(e) => this.verifyCode = e.target.value.replace(/[^0-9]/g, '')}"
                  @keypress="${(e) => e.key === 'Enter' && this.handleMfaVerify()}"
                />
              </div>

              <div style="display: flex; gap: 1rem;">
                <button class="button" @click="${this.handleMfaVerify}" ?disabled="${this.loading || this.verifyCode.length !== 6}">
                  ${this.loading ? '⏳ Vérification...' : '✓ Vérifier et Activer'}
                </button>
                <button class="button danger" @click="${() => { this.mfaSetupData = null; this.verifyCode = '' }}">
                  ✕ Annuler
                </button>
              </div>

              ${this.mfaSetupData.backup_codes ? html`
                <div class="backup-codes">
                  <div class="backup-codes-title">
                    <span>⚠️</span>
                    <span>Codes de Récupération (à conserver précieusement)</span>
                  </div>
                  <p style="color: #aaa; font-size: 0.85em; margin-bottom: 1rem;">
                    Conservez ces codes dans un endroit sûr. Ils vous permettent de vous connecter si vous perdez l'accès à votre application d'authentification.
                  </p>
                  <div class="backup-codes-grid">
                    ${this.mfaSetupData.backup_codes.map(code => html`
                      <div class="backup-code">${code}</div>
                    `)}
                  </div>
                </div>
              ` : ''}
            </div>
          `}
        ` : html`
          <!-- MFA déjà activé -->
          <div class="mfa-setup-container">
            <div class="alert success">
              <span>✅</span>
              <div>
                <strong>L'authentification à deux facteurs est active</strong>
                <p style="margin: 0.5rem 0 0 0; font-size: 0.9em; opacity: 0.9;">
                  Votre compte est protégé par une couche de sécurité supplémentaire.
                </p>
              </div>
            </div>

            <button class="button danger" @click="${this.handleMfaDisable}" ?disabled="${this.loading}">
              ${this.loading ? '⏳ Chargement...' : '🗑️ Désactiver l\'authentification 2FA'}
            </button>
          </div>
        `}
      </div>
    `
  }

  renderUsersTab() {
    const currentUser = authService.getCurrentUser()

    return html`
      <div class="section">
        <h2 class="section-title">👥 Gestion des Utilisateurs</h2>
        <p class="section-description">
          Créez et gérez les comptes utilisateurs ayant accès à Symbion.
        </p>

        <!-- Formulaire création utilisateur -->
        <div class="mfa-setup-container">
          <h3 style="color: var(--context-primary, #00d4aa); margin-bottom: 1rem;">
            ➕ Créer un nouvel utilisateur
          </h3>

          <div class="input-group">
            <label class="input-label">Nom d'utilisateur</label>
            <input
              type="text"
              class="input"
              placeholder="john_doe"
              .value="${this.newUser.username}"
              @input="${(e) => this.newUser = { ...this.newUser, username: e.target.value }}"
            />
          </div>

          <div class="input-group">
            <label class="input-label">Mot de passe</label>
            <input
              type="password"
              class="input"
              placeholder="••••••••"
              .value="${this.newUser.password}"
              @input="${(e) => this.newUser = { ...this.newUser, password: e.target.value }}"
            />
          </div>

          <div class="input-group">
            <label class="input-label">Rôle</label>
            <select
              class="input"
              .value="${this.newUser.role}"
              @change="${(e) => this.newUser = { ...this.newUser, role: e.target.value }}"
            >
              <option value="admin">Administrateur</option>
              <option value="user">Utilisateur</option>
            </select>
          </div>

          <button
            class="button"
            @click="${this.handleCreateUser}"
            ?disabled="${this.loading || !this.newUser.username || !this.newUser.password}"
          >
            ${this.loading ? '⏳ Création...' : '✓ Créer l\'utilisateur'}
          </button>
        </div>

        <!-- Liste des utilisateurs existants -->
        <div style="margin-top: 2rem;">
          <h3 style="color: var(--context-primary, #00d4aa); margin-bottom: 1rem;">
            📋 Utilisateurs existants
          </h3>

          ${this.users.length === 0 ? html`
            <div class="alert warning">
              <span>⚠️</span>
              <span>Aucun utilisateur trouvé. Le fichier users.json n'est peut-être pas accessible.</span>
            </div>
          ` : html`
            <div style="display: flex; flex-direction: column; gap: 0.8rem;">
              ${this.users.map(user => html`
                <div class="section" style="display: flex; justify-content: space-between; align-items: center; padding: 1rem;">
                  <div>
                    <div style="font-weight: 600; font-size: 1.1em; color: #e0e0e0; margin-bottom: 0.3rem;">
                      ${user.username}
                      ${user.username === currentUser?.username ? html`
                        <span style="color: var(--context-primary, #00d4aa); font-size: 0.8em; margin-left: 0.5rem;">(Vous)</span>
                      ` : ''}
                    </div>
                    <div style="color: #888; font-size: 0.9em;">
                      Rôle: ${user.role}
                      ${user.mfa_config?.enabled ? html`
                        <span style="color: #4caf50; margin-left: 1rem;">🛡️ MFA activé</span>
                      ` : ''}
                    </div>
                  </div>

                  ${user.username !== currentUser?.username ? html`
                    <button
                      class="button danger"
                      @click="${() => this.handleDeleteUser(user.username)}"
                      ?disabled="${this.loading}"
                    >
                      🗑️ Supprimer
                    </button>
                  ` : html`
                    <span style="color: #888; font-size: 0.9em; font-style: italic;">
                      (Impossible de supprimer votre propre compte)
                    </span>
                  `}
                </div>
              `)}
            </div>
          `}
        </div>
      </div>
    `
  }

  getSessionDuration() {
    const loginTime = authService.getLoginTime()
    if (!loginTime) return 'N/A'

    const now = Date.now()
    const duration = now - loginTime
    const hours = Math.floor(duration / (1000 * 60 * 60))
    const minutes = Math.floor((duration % (1000 * 60 * 60)) / (1000 * 60))

    if (hours > 0) {
      return `${hours}h ${minutes}min`
    }
    return `${minutes}min`
  }

  async handleValidationResolve(validationId, approved) {
    if (!confirm(`Confirmer ${approved ? 'l\'approbation' : 'le rejet'} de cette action ?`)) {
      return
    }

    try {
      this.loading = true
      await decisionService.resolveValidation(validationId, approved)
      this.showMessage('success', `Validation ${approved ? 'approuvée' : 'rejetée'} avec succès`)
      await this.loadDecisionsData()
    } catch (error) {
      console.error('[settings] Failed to resolve validation:', error)
      this.showMessage('error', 'Échec de la résolution: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handleDeleteExpired(validationId) {
    try {
      this.loading = true
      await decisionService.deleteValidation(validationId)
      this.showMessage('success', 'Validation expirée supprimée')
      await this.loadDecisionsData()
    } catch (error) {
      console.error('[settings] Failed to delete expired validation:', error)
      this.showMessage('error', 'Échec de la suppression: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async handleDeleteAllExpired() {
    if (!confirm(`Supprimer toutes les validations expirées (${this.expiredValidations.length}) ?`)) {
      return
    }

    try {
      this.loading = true
      const result = await decisionService.deleteAllExpiredValidations()
      this.showMessage('success', result.message || 'Toutes les validations expirées ont été supprimées')
      await this.loadDecisionsData()
    } catch (error) {
      console.error('[settings] Failed to delete all expired validations:', error)
      this.showMessage('error', 'Échec de la suppression: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  async generateTestData() {
    if (!confirm('Générer des données de test pour PR3 Decision Engine ?')) return

    try {
      this.loading = true

      // Générer une évaluation de test (action avec mode mismatch → RequireValidation)
      const action = {
        action_type: 'test_shutdown',
        agent_id: 'test-agent-123',
        impact_level: 'Medium',
        trace_id: `test-trace-${Date.now()}`,
        expires_at: null,
        dry_run: false,
        expected_mode: 'cravate',
        expected_ssid: null
      }

      const context = {
        mode: 'intime',
        ssid: 'home-wifi',
        agents: {}
      }

      await decisionService.evaluateAction(action, context)

      this.showMessage('success', 'Données de test générées avec succès')
      await this.loadDecisionsData()
    } catch (error) {
      console.error('[settings] Failed to generate test data:', error)
      this.showMessage('error', 'Échec génération test: ' + error.message)
    } finally {
      this.loading = false
    }
  }

  renderDecisionsTab() {
    if (this.loading && !this.stats) {
      return html`
        <div class="loading">
          <div class="spinner"></div>
          <p>Chargement...</p>
        </div>
      `
    }

    return html`
      <!-- Métriques temps réel -->
      <div class="section">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
          <div>
            <h2 class="section-title" style="margin-bottom: 0.5rem;">📊 Métriques Décisions</h2>
            <p class="section-description" style="margin: 0;">
              Statistiques temps réel du Decision Engine PR3
            </p>
          </div>
          <button
            class="btn-secondary"
            @click="${() => this.generateTestData()}"
            ?disabled="${this.loading}">
            🧪 Générer Test
          </button>
        </div>

        ${this.stats ? html`
          <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem;">
            <div style="background: rgba(0, 212, 170, 0.05); border: 1px solid rgba(0, 212, 170, 0.2); border-radius: var(--radius-base); padding: 1rem; text-align: center;">
              <div style="font-size: 2em; color: var(--context-primary, #00d4aa);">
                ${this.stats.total_evaluations || 0}
              </div>
              <div style="color: #888; font-size: 0.9em; margin-top: 0.5rem;">
                Évaluations totales
              </div>
            </div>
            <div style="background: rgba(76, 175, 80, 0.05); border: 1px solid rgba(76, 175, 80, 0.2); border-radius: var(--radius-base); padding: 1rem; text-align: center;">
              <div style="font-size: 2em; color: #4caf50;">
                ${this.stats.approved || 0}
              </div>
              <div style="color: #888; font-size: 0.9em; margin-top: 0.5rem;">
                Approuvées
              </div>
            </div>
            <div style="background: rgba(255, 107, 107, 0.05); border: 1px solid rgba(255, 107, 107, 0.2); border-radius: var(--radius-base); padding: 1rem; text-align: center;">
              <div style="font-size: 2em; color: #ff6b6b;">
                ${this.stats.rejected || 0}
              </div>
              <div style="color: #888; font-size: 0.9em; margin-top: 0.5rem;">
                Rejetées
              </div>
            </div>
            <div style="background: rgba(255, 193, 7, 0.05); border: 1px solid rgba(255, 193, 7, 0.2); border-radius: var(--radius-base); padding: 1rem; text-align: center;">
              <div style="font-size: 2em; color: #ffc107;">
                ${this.stats.pending || 0}
              </div>
              <div style="color: #888; font-size: 0.9em; margin-top: 0.5rem;">
                En attente
              </div>
            </div>
          </div>
        ` : html`
          <div class="alert warning">
            <span>⚠️</span>
            <span>Aucune statistique disponible</span>
          </div>
        `}
      </div>

      <!-- Validations en attente -->
      <div class="section">
        <h2 class="section-title">⏳ Validations en Attente</h2>
        <p class="section-description">
          Actions nécessitant une validation manuelle avant exécution
        </p>

        ${this.validations.length === 0 ? html`
          <div class="alert success">
            <span>✅</span>
            <span>Aucune validation en attente</span>
          </div>
        ` : html`
          <div style="display: flex; flex-direction: column; gap: 1rem;">
            ${this.validations.map(validation => html`
              <div class="section" style="padding: 1rem;">
                <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem;">
                  <div style="flex: 1;">
                    <div style="font-weight: 600; font-size: 1.1em; color: #e0e0e0; margin-bottom: 0.5rem;">
                      ${validation.action?.action_type || 'Action'}
                    </div>
                    <div style="color: #888; font-size: 0.9em; margin-bottom: 0.3rem;">
                      Agent: ${validation.action?.agent_id || 'N/A'}
                    </div>
                    <div style="color: #888; font-size: 0.9em;">
                      Impact: <span style="color: ${this.getImpactColor(validation.action?.impact_level)}; font-weight: 600;">
                        ${validation.action?.impact_level || 'MEDIUM'}
                      </span>
                    </div>
                  </div>
                  <div style="display: flex; gap: 0.5rem;">
                    <button
                      class="button"
                      style="padding: 0.5rem 1rem;"
                      @click="${() => this.handleValidationResolve(validation.validation_id, true)}"
                      ?disabled="${this.loading}"
                    >
                      ✓ Approuver
                    </button>
                    <button
                      class="button danger"
                      style="padding: 0.5rem 1rem;"
                      @click="${() => this.handleValidationResolve(validation.validation_id, false)}"
                      ?disabled="${this.loading}"
                    >
                      ✗ Rejeter
                    </button>
                  </div>
                </div>
                ${validation.reason ? html`
                  <div style="background: rgba(0, 0, 0, 0.3); border-left: 3px solid var(--context-primary, #00d4aa); padding: 0.8rem; border-radius: 4px;">
                    <div style="color: #aaa; font-size: 0.85em; margin-bottom: 0.3rem;">Raison:</div>
                    <div style="color: #e0e0e0; font-size: 0.95em;">${validation.reason}</div>
                  </div>
                ` : ''}
              </div>
            `)}
          </div>
        `}
      </div>

      <!-- Validations expirées -->
      ${this.expiredValidations.length > 0 ? html`
        <div class="section">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
            <div>
              <h2 class="section-title">📋 Validations Expirées (${this.expiredValidations.length})</h2>
              <p class="section-description" style="margin: 0;">
                Validations ayant dépassé leur délai de traitement
              </p>
            </div>
            <button
              class="button danger"
              style="padding: 0.6rem 1rem; font-size: 0.9em;"
              @click="${() => this.handleDeleteAllExpired()}"
              ?disabled="${this.loading}"
            >
              🗑️ Tout supprimer
            </button>
          </div>

          <div style="display: flex; flex-direction: column; gap: 1rem;">
            ${this.expiredValidations.map(validation => html`
              <div class="section" style="padding: 1rem; opacity: 0.7; border: 1px solid rgba(255, 193, 7, 0.3);">
                <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem;">
                  <div style="flex: 1;">
                    <div style="font-weight: 600; font-size: 1.1em; color: #e0e0e0; margin-bottom: 0.5rem;">
                      ${validation.action?.action_type || 'Action'}
                      <span style="color: #ffc107; font-size: 0.8em; margin-left: 0.5rem;">⏱️ EXPIRÉ</span>
                    </div>
                    <div style="color: #888; font-size: 0.9em; margin-bottom: 0.3rem;">
                      Agent: ${validation.action?.agent_id || 'N/A'}
                    </div>
                    <div style="color: #888; font-size: 0.9em; margin-bottom: 0.3rem;">
                      Impact: <span style="color: ${this.getImpactColor(validation.action?.impact_level)}; font-weight: 600;">
                        ${validation.action?.impact_level || 'MEDIUM'}
                      </span>
                    </div>
                    <div style="color: #888; font-size: 0.85em;">
                      Expiré le: ${validation.expires_at ? new Date(validation.expires_at).toLocaleString('fr-FR') : 'N/A'}
                    </div>
                  </div>
                  <button
                    class="button danger"
                    style="padding: 0.5rem 1rem; font-size: 0.85em;"
                    @click="${() => this.handleDeleteExpired(validation.validation_id)}"
                    ?disabled="${this.loading}"
                  >
                    🗑️ Supprimer
                  </button>
                </div>
                ${validation.reason ? html`
                  <div style="background: rgba(0, 0, 0, 0.3); border-left: 3px solid #ffc107; padding: 0.8rem; border-radius: 4px;">
                    <div style="color: #aaa; font-size: 0.85em; margin-bottom: 0.3rem;">Raison:</div>
                    <div style="color: #e0e0e0; font-size: 0.95em;">${validation.reason}</div>
                  </div>
                ` : ''}
              </div>
            `)}
          </div>
        </div>
      ` : ''}

      <!-- Overrides actifs -->
      <div class="section">
        <h2 class="section-title">🔓 Overrides Actifs</h2>
        <p class="section-description">
          Contournements de sécurité temporaires activés
        </p>

        ${this.overrides.length === 0 ? html`
          <div class="alert success">
            <span>✅</span>
            <span>Aucun override actif</span>
          </div>
        ` : html`
          <div style="display: flex; flex-direction: column; gap: 1rem;">
            ${this.overrides.map(override => html`
              <div class="section" style="padding: 1rem; border: 1px solid rgba(255, 193, 7, 0.3);">
                <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                  <div style="flex: 1;">
                    <div style="font-weight: 600; font-size: 1.1em; color: #ffc107; margin-bottom: 0.5rem;">
                      Override: ${override.override_type || 'N/A'}
                    </div>
                    <div style="color: #888; font-size: 0.9em; margin-bottom: 0.3rem;">
                      Décision: ${override.decision_id || 'N/A'}
                    </div>
                    <div style="color: #888; font-size: 0.9em; margin-bottom: 0.3rem;">
                      Par: ${override.created_by || 'N/A'}
                    </div>
                    <div style="color: #888; font-size: 0.9em;">
                      Expire: ${override.expires_at ? new Date(override.expires_at).toLocaleString('fr-FR') : 'N/A'}
                    </div>
                  </div>
                  <div style="background: rgba(255, 193, 7, 0.15); border: 1px solid rgba(255, 193, 7, 0.3); padding: 0.5rem 1rem; border-radius: 6px;">
                    <span style="color: #ffc107; font-weight: 600; font-size: 0.85em;">ACTIF</span>
                  </div>
                </div>
                ${override.reason ? html`
                  <div style="background: rgba(0, 0, 0, 0.3); border-left: 3px solid #ffc107; padding: 0.8rem; border-radius: 4px; margin-top: 1rem;">
                    <div style="color: #aaa; font-size: 0.85em; margin-bottom: 0.3rem;">Raison:</div>
                    <div style="color: #e0e0e0; font-size: 0.95em;">${override.reason}</div>
                  </div>
                ` : ''}
              </div>
            `)}
          </div>
        `}
      </div>
    `
  }

  getImpactColor(level) {
    switch (level?.toUpperCase()) {
      case 'HIGH': return '#ff6b6b'
      case 'MEDIUM': return '#ffc107'
      case 'LOW': return '#4caf50'
      default: return '#888'
    }
  }

  _isLogsEnabled() {
    return localStorage.getItem('symbion_show_logs') === 'true'
  }

  _toggleLogs() {
    const enabled = !this._isLogsEnabled()
    localStorage.setItem('symbion_show_logs', enabled ? 'true' : 'false')
    this.requestUpdate()
    // Notify dashboard to show/hide the FAB
    window.dispatchEvent(new CustomEvent('symbion-logs-toggle', { detail: { enabled } }))
  }
}

customElements.define('user-settings-page', UserSettingsPage)

export { UserSettingsPage }
