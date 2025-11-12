/**
 * Boot Terminal Component
 *
 * Terminal de démarrage Symbion avec:
 * - Animation de boot réaliste
 * - Affichage des vraies étapes de chargement
 * - Login prompt intégré
 * - Skip avec Ctrl+C ou après 3s
 */

import { LitElement, html, css } from 'lit'
import authService from '../services/auth-service.js'

class BootTerminal extends LitElement {
  static styles = css`
    :host {
      display: block;
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: #0a0a0a;
      z-index: 100000;
      overflow: hidden;
    }

    .terminal {
      font-family: 'Monaco', 'Menlo', 'Consolas', 'Courier New', monospace;
      font-size: 15px;
      line-height: 1.8;
      color: #00ff9f;
      padding: 1.5rem 2.5rem 3rem 2.5rem;
      height: 100%;
      overflow-y: auto;
      position: relative;

      /* Effet CRT scanline subtil */
      background:
        linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.25) 50%),
        linear-gradient(90deg, rgba(0, 255, 159, 0.03), rgba(0, 122, 204, 0.02));
      background-size: 100% 3px, 4px 100%;
      animation: scanline 12s linear infinite, flicker 0.15s infinite;
    }

    /* Effet scanline qui défile */
    @keyframes scanline {
      0% { background-position: 0 0; }
      100% { background-position: 0 100%; }
    }

    /* Léger flicker pour effet CRT */
    @keyframes flicker {
      0% { opacity: 0.98; }
      50% { opacity: 1; }
      100% { opacity: 0.98; }
    }

    /* Overlay CRT glow */
    .terminal::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: radial-gradient(ellipse at center, transparent 0%, rgba(0, 0, 0, 0.3) 100%);
      pointer-events: none;
    }

    .terminal::-webkit-scrollbar {
      width: 8px;
    }

    .terminal::-webkit-scrollbar-track {
      background: rgba(0, 255, 159, 0.05);
      border-radius: 4px;
    }

    .terminal::-webkit-scrollbar-thumb {
      background: rgba(0, 255, 159, 0.4);
      border-radius: 4px;
    }

    .terminal::-webkit-scrollbar-thumb:hover {
      background: rgba(0, 255, 159, 0.6);
    }

    .line {
      margin-bottom: 0.4rem;
      white-space: pre-wrap;
      opacity: 0;
      animation: fadeInLine 0.2s ease forwards;
      text-shadow: 0 0 8px rgba(0, 255, 159, 0.4);
      letter-spacing: 0.3px;
    }

    @keyframes fadeInLine {
      from {
        opacity: 0;
        transform: translateX(-5px);
        filter: blur(2px);
      }
      to {
        opacity: 1;
        transform: translateX(0);
        filter: blur(0);
      }
    }

    .line.success {
      color: #00ff9f;
      text-shadow: 0 0 10px rgba(0, 255, 159, 0.6);
    }

    .line.error {
      color: #ff4757;
      text-shadow: 0 0 10px rgba(255, 71, 87, 0.6);
      font-weight: 600;
    }

    .line.warning {
      color: #ffa502;
      text-shadow: 0 0 10px rgba(255, 165, 2, 0.6);
    }

    .line.info {
      color: #5f27cd;
      text-shadow: 0 0 10px rgba(95, 39, 205, 0.6);
    }

    .line.prompt {
      color: #ffffff;
      text-shadow: 0 0 8px rgba(255, 255, 255, 0.4);
    }

    /* Boutons d'action dans le terminal */
    .cert-setup-box {
      margin: 1.5rem 0;
      padding: 1.5rem;
      border: 2px solid #00ff9f;
      border-radius: 8px;
      background: rgba(0, 255, 159, 0.05);
      box-shadow: 0 0 20px rgba(0, 255, 159, 0.2);
    }

    .cert-download-btn {
      display: inline-block;
      margin: 1rem 0;
      padding: 1rem 2rem;
      background: linear-gradient(135deg, #00ff9f 0%, #00d4aa 100%);
      color: #0a0a0a;
      font-weight: 700;
      font-size: 1.1rem;
      text-decoration: none;
      border-radius: 6px;
      cursor: pointer;
      transition: all 0.3s ease;
      box-shadow: 0 4px 15px rgba(0, 255, 159, 0.4);
      text-shadow: none;
    }

    .cert-download-btn:hover {
      background: linear-gradient(135deg, #00d4aa 0%, #00ff9f 100%);
      transform: translateY(-2px);
      box-shadow: 0 6px 20px rgba(0, 255, 159, 0.6);
    }

    .retry-btn {
      display: inline-block;
      margin: 1rem 0.5rem;
      padding: 0.8rem 1.5rem;
      background: rgba(95, 39, 205, 0.8);
      color: #ffffff;
      font-weight: 600;
      font-size: 1rem;
      text-decoration: none;
      border-radius: 6px;
      cursor: pointer;
      transition: all 0.3s ease;
      box-shadow: 0 4px 15px rgba(95, 39, 205, 0.4);
      border: none;
    }

    .retry-btn:hover:not(:disabled) {
      background: rgba(95, 39, 205, 1);
      transform: translateY(-2px);
      box-shadow: 0 6px 20px rgba(95, 39, 205, 0.6);
    }

    .retry-btn:disabled {
      background: rgba(95, 39, 205, 0.3);
      color: rgba(255, 255, 255, 0.5);
      cursor: not-allowed;
      box-shadow: 0 2px 8px rgba(95, 39, 205, 0.2);
    }

    .step {
      color: #ffa502;
      font-weight: 600;
      margin-top: 0.5rem;
    }

    .platform-badge {
      display: inline-block;
      padding: 0.3rem 0.8rem;
      background: rgba(255, 165, 2, 0.2);
      border: 1px solid #ffa502;
      border-radius: 4px;
      color: #ffa502;
      font-weight: 600;
      margin-right: 0.5rem;
    }

    .cursor {
      display: inline-block;
      width: 10px;
      height: 18px;
      background: #00ff9f;
      margin-left: 3px;
      animation: blink 1s step-end infinite;
      box-shadow: 0 0 10px rgba(0, 255, 159, 0.8);
    }

    @keyframes blink {
      50% { opacity: 0; }
    }

    .input-line {
      display: flex;
      align-items: center;
      margin-top: 1.5rem;
      color: #ffffff;
      padding: 0.5rem 0;
    }

    .input-field {
      background: transparent;
      border: none;
      outline: none;
      color: #00ff9f;
      font-family: inherit;
      font-size: inherit;
      flex: 1;
      padding: 0;
      caret-color: #00ff9f;
      text-shadow: 0 0 8px rgba(0, 255, 159, 0.4);
    }

    .input-field::placeholder {
      color: rgba(0, 255, 159, 0.3);
    }

    .login-form {
      margin: 0;
      padding: 0;
    }

    .input-line label {
      color: #ffffff;
      text-shadow: 0 0 8px rgba(255, 255, 255, 0.4);
      font-family: inherit;
      font-size: inherit;
      font-weight: normal;
      cursor: text;
      user-select: none;
    }

    .skip-hint {
      position: fixed;
      bottom: 2.5rem;
      right: 2.5rem;
      color: rgba(0, 255, 159, 0.6);
      font-size: 0.9em;
      opacity: 0;
      animation: fadeIn 0.5s ease 2s forwards, pulse 2s ease-in-out infinite;
      text-shadow: 0 0 8px rgba(0, 255, 159, 0.4);
      border: 1px solid rgba(0, 255, 159, 0.3);
      padding: 0.5rem 1rem;
      border-radius: 4px;
      background: rgba(0, 0, 0, 0.5);
    }

    @keyframes fadeIn {
      to { opacity: 1; }
    }

    @keyframes pulse {
      0%, 100% { opacity: 0.6; }
      50% { opacity: 1; }
    }

    .logo {
      color: #00d4ff;
      font-weight: bold;
      margin-bottom: 1.5rem;
      text-shadow:
        0 0 20px rgba(0, 212, 255, 0.8),
        0 0 40px rgba(0, 212, 255, 0.4);
      letter-spacing: 1px;
      animation: logoGlow 3s ease-in-out infinite;
    }

    @keyframes logoGlow {
      0%, 100% {
        text-shadow:
          0 0 20px rgba(0, 212, 255, 0.8),
          0 0 40px rgba(0, 212, 255, 0.4);
      }
      50% {
        text-shadow:
          0 0 30px rgba(0, 212, 255, 1),
          0 0 60px rgba(0, 212, 255, 0.6);
      }
    }

    /* Animation typing dots */
    .loading-dots::after {
      content: '';
      animation: dots 1.5s steps(4, end) infinite;
    }

    @keyframes dots {
      0%, 20% { content: ''; }
      40% { content: '.'; }
      60% { content: '..'; }
      80%, 100% { content: '...'; }
    }

    /* Adaptations mobile - formulaires visibles sans scroll excessif */
    @media (max-width: 768px) {
      .terminal {
        padding: 0.75rem 1rem 1.5rem 1rem;
        font-size: 13px;
        line-height: 1.6;
      }

      .line {
        margin-bottom: 0.2rem;
      }

      .logo {
        margin-bottom: 0.75rem;
        font-size: 0.95em;
      }

      .input-line {
        margin-top: 1rem;
        padding: 0.25rem 0;
      }

      .login-form {
        margin-top: 1rem;
      }

      .cert-setup-box {
        margin: 1rem 0;
        padding: 1rem;
      }

      .cert-download-btn {
        font-size: 1rem;
        padding: 0.8rem 1.5rem;
      }

      .skip-hint {
        bottom: 1.5rem;
        right: 1.5rem;
        font-size: 0.8em;
        padding: 0.4rem 0.8rem;
      }
    }

    /* Très petits écrans - ultra compact */
    @media (max-width: 480px) {
      .terminal {
        padding: 0.5rem 0.75rem 1rem 0.75rem;
        font-size: 12px;
        line-height: 1.5;
      }

      .line {
        margin-bottom: 0.15rem;
      }

      .logo {
        font-size: 0.9em;
        margin-bottom: 0.5rem;
      }

      .input-line {
        margin-top: 0.75rem;
        flex-wrap: wrap;
      }

      .input-line label {
        font-size: 0.9em;
      }

      .cert-setup-box {
        padding: 0.75rem;
      }

      .cert-download-btn {
        font-size: 0.9rem;
        padding: 0.7rem 1.2rem;
      }

      .retry-btn {
        font-size: 0.85rem;
        padding: 0.6rem 1rem;
      }
    }

    .biometric-btn {
      margin-top: 1.5rem;
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.15) 0%, rgba(0, 122, 204, 0.1) 100%);
      border: 1px solid rgba(0, 212, 170, 0.3);
      color: #00d4aa;
      padding: 0.75rem 1.5rem;
      border-radius: 8px;
      font-family: inherit;
      font-size: 0.95em;
      cursor: pointer;
      transition: all 0.3s ease;
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      text-shadow: 0 0 8px rgba(0, 255, 159, 0.4);
    }

    .biometric-btn:hover {
      background: linear-gradient(135deg, rgba(0, 212, 170, 0.25) 0%, rgba(0, 122, 204, 0.15) 100%);
      border-color: rgba(0, 212, 170, 0.5);
      transform: translateY(-1px);
      box-shadow: 0 0 15px rgba(0, 212, 170, 0.3);
    }

    .biometric-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      transform: none;
    }

    .biometric-separator {
      margin: 1.5rem 0 1rem 0;
      display: flex;
      align-items: center;
      gap: 1rem;
      opacity: 0.5;
    }

    .biometric-separator::before,
    .biometric-separator::after {
      content: '';
      flex: 1;
      height: 1px;
      background: linear-gradient(90deg, transparent, rgba(0, 255, 159, 0.3), transparent);
    }

    .biometric-separator span {
      font-size: 0.85em;
      color: #00ff9f;
    }

    .spinner {
      display: inline-block;
      width: 1em;
      height: 1em;
      border: 2px solid rgba(255, 255, 255, 0.3);
      border-top-color: #00d4aa;
      border-radius: 50%;
      animation: spin 1s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }
  `

  static properties = {
    lines: { type: Array },
    phase: { type: String }, // 'booting', 'login', 'authenticating', 'done'
    loginStep: { type: String }, // 'credentials', 'totp'
    username: { type: String },
    password: { type: String },
    totpCode: { type: String },
    rememberDevice: { type: Boolean },
    error: { type: String },
    showCertificateUI: { type: Boolean },
    certUrl: { type: String },
    platform: { type: String },
    certVerifying: { type: Boolean },
    certInstalled: { type: Boolean },
    authenticatingBiometric: { type: Boolean },
    biometricAvailable: { type: Boolean }
  }

  constructor() {
    super()
    this.lines = []
    this.phase = 'booting'
    this.loginStep = 'credentials' // username + password ensemble
    this.username = ''
    this.password = ''
    this.totpCode = ''
    this.rememberDevice = false
    this.error = null
    this.showCertificateUI = false
    this.certUrl = ''
    this.platform = ''
    this.certVerifying = false
    this.certInstalled = false
    this.authenticatingBiometric = false
    this.biometricAvailable = false
    // Base URL dynamique (même logique que passkey-manager et api-service)
    this.baseUrl = window.SYMBION_CONFIG?.API_BASE || 'https://symbion.local:8443'
  }

  connectedCallback() {
    super.connectedCallback()
    // Vérifier disponibilité WebAuthn/biométrie
    this.checkBiometricAvailability()
    // Démarrer la séquence de boot
    this.startBootSequence()
  }

  async checkBiometricAvailability() {
    // Vérifier si le navigateur supporte WebAuthn
    if (window.PublicKeyCredential) {
      try {
        // Vérifier si un authenticator est disponible (platform = biométrie intégrée)
        const available = await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()
        this.biometricAvailable = available
        console.log('[boot-terminal] Biometric authentication available:', available)
      } catch (error) {
        console.warn('[boot-terminal] Failed to check biometric availability:', error)
        this.biometricAvailable = false
      }
    } else {
      this.biometricAvailable = false
    }
  }

  updated(changedProperties) {
    super.updated(changedProperties)
    // Attacher les écouteurs d'autofill après chaque render
    this.attachAutofillListeners()
  }

  attachAutofillListeners() {
    // Détecter l'autofill de Bitwarden/gestionnaires de mots de passe
    const inputs = this.shadowRoot.querySelectorAll('input[type="text"], input[type="password"], input[type="tel"]')

    inputs.forEach(input => {
      // Marquer si déjà surveillé
      if (input.dataset.autofillWatched) return
      input.dataset.autofillWatched = 'true'

      console.log('[autofill] Attaching listeners to', input.name, 'type:', input.type)

      // Détecter PASTE explicite (Ctrl+V ou clic droit → coller)
      input.addEventListener('paste', (e) => {
        console.log('[autofill] Paste detected in', input.name)
        // Permettre le paste, puis trigger auto-submit après un petit délai
        setTimeout(() => {
          this.triggerAutoSubmit(input)
        }, 100)
      })

      // Événement change : Bitwarden/gestionnaires déclenchent toujours ça lors de l'autofill
      // NOTE: Ne se déclenche PAS lors de la saisie manuelle, seulement lors de l'autofill
      input.addEventListener('change', () => {
        if (input.value && input.value.length > 0) {
          console.log('[autofill-change] Autofill detected in', input.name)
          this.triggerAutoSubmit(input)
        }
      })
    })

    // POLLING pour formulaire login/password (détection Bitwarden qui remplit les deux champs en même temps)
    const form = this.shadowRoot.querySelector('form[name="login-form"]')
    if (form && !form.dataset.autofillPolling) {
      form.dataset.autofillPolling = 'true'
      console.log('[autofill] Starting polling for login-form')

      let lastCheckAllFilled = false
      const checkInterval = setInterval(() => {
        // Arrêter le polling si le formulaire n'existe plus ou si on n'est plus en phase login
        if (!this.shadowRoot.querySelector('form[name="login-form"]') || this.phase !== 'login') {
          clearInterval(checkInterval)
          console.log('[autofill] Stopping polling - form removed or phase changed')
          return
        }

        const usernameInput = form.querySelector('input[name="username"]')
        const passwordInput = form.querySelector('input[name="password"]')

        if (usernameInput && passwordInput) {
          const bothFilled = usernameInput.value.length > 0 && passwordInput.value.length > 0

          // Vérifier si l'utilisateur est en train de taper (input a le focus)
          const userIsTyping = document.activeElement === usernameInput ||
                               document.activeElement === passwordInput ||
                               this.shadowRoot.activeElement === usernameInput ||
                               this.shadowRoot.activeElement === passwordInput

          // Si les deux champs viennent d'être remplis (transition de vide à rempli)
          // ET que l'utilisateur n'est PAS en train de taper
          if (bothFilled && !lastCheckAllFilled && !userIsTyping) {
            console.log('[autofill-polling] ✓ Both fields filled (not typing) - auto-submitting')
            clearInterval(checkInterval)
            form.requestSubmit()
          } else if (bothFilled && !lastCheckAllFilled && userIsTyping) {
            console.log('[autofill-polling] User is typing, skipping auto-submit')
          }

          lastCheckAllFilled = bothFilled
        }
      }, 300) // Vérifier toutes les 300ms
    }
  }

  triggerAutoSubmit(input) {
    // Éviter les soumissions multiples
    if (input.dataset.submitted === 'true') return

    input.dataset.submitted = 'true'

    setTimeout(() => {
      const form = input.closest('form')
      if (!form) {
        input.dataset.submitted = 'false'
        return
      }

      // Vérifier que TOUS les champs requis sont remplis avant de soumettre
      const requiredInputs = form.querySelectorAll('input[type="text"], input[type="password"], input[type="tel"]')
      const allFilled = Array.from(requiredInputs).every(inp => inp.value && inp.value.length > 0)

      if (allFilled) {
        console.log('[autofill] ✓ Tous les champs remplis - Auto-submitting form:', form.name)
        form.requestSubmit()
      } else {
        console.log('[autofill] ⏳ Attente que tous les champs soient remplis...')
      }

      // Reset après 2 secondes
      setTimeout(() => {
        input.dataset.submitted = 'false'
      }, 2000)
    }, 800) // Augmenté à 800ms pour laisser le temps au gestionnaire de remplir tous les champs
  }

  async startBootSequence() {
    // Vérifier si une session existe déjà (éviter le reload complet)
    if (authService.isAuthenticated()) {
      console.log('[boot] Session existante détectée - skip boot sequence')
      this.phase = 'done'
      this.dispatchEvent(new CustomEvent('boot-complete', {
        detail: { authenticated: true },
        bubbles: true,
        composed: true
      }))
      return
    }

    // Vérifier si le boot a déjà été fait dans cette session (éviter replay)
    const bootCompleted = sessionStorage.getItem('symbion_boot_completed')
    if (bootCompleted === 'true') {
      console.log('[boot] Boot déjà effectué dans cette session - skip directement au login')
      this.addLine('  ▸ SYMBION v0.1.0', 'logo')
      this.addLine('  ━━━━━━━━━━━━━━━━━━━━', 'info')
      this.addLine('')
      this.phase = 'login'
      this.loginStep = 'credentials'
      this.requestUpdate()
      return
    }

    await this.delay(50)

    // Logo ultra-compact mobile-friendly
    this.addLine('', 'logo')
    this.addLine('  ▸ SYMBION v0.1.0', 'logo')
    this.addLine('  ▸ Neural Cortex', 'logo')
    this.addLine('  ━━━━━━━━━━━━━━━━━━━━', 'info')
    this.addLine('', 'logo')
    await this.delay(200)

    this.addLine('[kernel] Initialisation du cortex neural...')
    await this.delay(100)

    // Vérification connexion kernel (CRITIQUE)
    const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
    const kernelLoadingIdx = this.addLoadingLine(`[kernel] Connexion à ${API_BASE}`)
    const kernelOk = await this.checkKernel()
    this.updateLine(
      kernelLoadingIdx,
      kernelOk ? '[kernel] ✓ Connecté' : '[kernel] ✗ Échec de connexion',
      kernelOk ? 'success' : 'error'
    )

    // ARRÊT IMMÉDIAT si kernel inaccessible
    if (!kernelOk) {
      await this.delay(200)
      this.addLine('[error] Impossible de continuer sans connexion au kernel', 'error')
      this.addLine('[error] Vérifiez que symbion-kernel est démarré', 'warning')
      await this.delay(300)

      // Afficher l'interface d'installation du certificat
      this.showCertificateSetup()
      return
    }

    await this.delay(150)

    // MQTT
    const mqttLoadingIdx = this.addLoadingLine('[mqtt] Établissement du bus de messages')
    await this.delay(150)
    this.updateLine(mqttLoadingIdx, '[mqtt] ✓ Connecté', 'success')
    await this.delay(100)

    // Plugins
    const pluginsLoadingIdx = this.addLoadingLine('[plugins] Chargement des modules')
    await this.delay(100)
    this.updateLine(pluginsLoadingIdx, '[plugins] ✓ mémoire-externe (notes) actif', 'success')
    await this.delay(100)

    // Agents
    const agentsLoadingIdx = this.addLoadingLine('[agents] Scan réseau à la recherche d\'agents')
    const agents = await this.checkAgents()
    this.updateLine(agentsLoadingIdx, `[agents] ✓ Trouvé ${agents} agent(s)`, 'success')
    await this.delay(150)

    // Services registration
    const servicesLoadingIdx = this.addLoadingLine('[services] Enregistrement de la couche de services')
    await this.delay(100)
    const services = this.checkServices()
    this.updateLine(servicesLoadingIdx, `[services] ✓ ${services.length} services actifs (${services.join(', ')})`, 'success')
    await this.delay(100)

    // Context detection
    const contextLoadingIdx = this.addLoadingLine('[context] Détection du mode actuel')
    const context = await this.checkContext()
    if (context) {
      this.updateLine(contextLoadingIdx, `[context] ✓ Mode: ${context.mode} (${context.reason})`, 'success')
      await this.delay(100)

      // Theme application
      const themeLoadingIdx = this.addLoadingLine('[theme] Application du thème contextuel')
      await this.delay(80)
      this.updateLine(themeLoadingIdx, `[theme] ✓ Thème ${context.theme?.name || 'par défaut'} activé`, 'success')
      await this.delay(100)
    } else {
      this.updateLine(contextLoadingIdx, '[context] Mode: neutre (par défaut)', 'warning')
      await this.delay(100)
    }

    // Statistics tracking
    const statsLoadingIdx = this.addLoadingLine('[analytics] Initialisation du suivi statistique')
    await this.delay(80)
    const statsEnabled = await this.checkStats()
    this.updateLine(statsLoadingIdx, statsEnabled ? '[analytics] ✓ Collecte des stats active' : '[analytics] Collecte des stats désactivée', statsEnabled ? 'success' : 'warning')
    await this.delay(100)

    // Pattern learning
    const patternsLoadingIdx = this.addLoadingLine('[learning] Activation de la reconnaissance de patterns')
    await this.delay(80)
    const patternsEnabled = await this.checkPatterns()
    this.updateLine(patternsLoadingIdx, patternsEnabled ? '[learning] ✓ Apprentissage de patterns actif' : '[learning] Apprentissage de patterns désactivé', patternsEnabled ? 'success' : 'warning')
    await this.delay(150)

    // Auth session check
    const authLoadingIdx = this.addLoadingLine('[auth] Vérification de la session')
    await this.delay(100)

    const hasSession = await this.verifySession()
    this.updateLine(
      authLoadingIdx,
      hasSession ? '[auth] ✓ Session valide' : '[auth] Aucune session valide trouvée',
      hasSession ? 'success' : 'warning'
    )

    if (hasSession) {
      await this.delay(50)
      this.addLine(`[session] Utilisateur '${authService.getCurrentUser().username}' autorisé`, 'success')
      await this.delay(50)
      const dashLoadingIdx = this.addLoadingLine('[dashboard] Chargement de l\'interface')
      await this.delay(200)
      this.updateLine(dashLoadingIdx, '[dashboard] ✓ Prêt', 'success')
      await this.delay(100)
      this.phase = 'done'
      console.log('[boot] Dispatching boot-complete event (session valid)')
      this.dispatchEvent(new CustomEvent('boot-complete', {
        detail: { authenticated: true },
        bubbles: true,
        composed: true
      }))
    } else {
      await this.delay(100)
      this.addLine('[auth] Session requise', 'info')
      await this.delay(200)

      // Activer le skip après 3 secondes
      setTimeout(() => {
        this.canSkip = true
      }, 3000)

      // Marquer le boot comme complété pour cette session
      sessionStorage.setItem('symbion_boot_completed', 'true')

      this.phase = 'login'
      this.requestUpdate()
      await this.delay(50)
      this.focusInput()
    }
  }

  async checkKernel() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      console.log('[boot-terminal] checkKernel API_BASE:', API_BASE)
      const response = await fetch(`${API_BASE}/health`, { credentials: 'include' })
      return response.ok
    } catch {
      return false
    }
  }

  async checkAgents() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      const response = await fetch(`${API_BASE}/agents`, {
        headers: {
          'x-api-key': import.meta.env.VITE_SYMBION_API_KEY
        },
        credentials: 'include'
      })
      if (response.ok) {
        const data = await response.json()
        return data.length
      }
    } catch {}
    return 0
  }

  async verifySession() {
    return await authService.verifySession()
  }

  checkServices() {
    const serviceNames = ['api-service', 'mqtt-service', 'context-service', 'agents-service']
    const activeServices = []

    for (const name of serviceNames) {
      if (document.querySelector(name)) {
        // Simplify service names for display
        const displayName = name.replace('-service', '')
        activeServices.push(displayName)
      }
    }

    return activeServices
  }

  async checkContext() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      const response = await fetch(`${API_BASE}/context/current`, {
        headers: {
          'x-api-key': import.meta.env.VITE_SYMBION_API_KEY || 's3cr3t-42'
        },
        credentials: 'include'
      })
      if (response.ok) {
        return await response.json()
      }
    } catch {}
    return null
  }

  async checkStats() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      const response = await fetch(`${API_BASE}/context/stats`, {
        headers: {
          'x-api-key': import.meta.env.VITE_SYMBION_API_KEY || 's3cr3t-42'
        },
        credentials: 'include'
      })
      return response.ok
    } catch {}
    return false
  }

  async checkPatterns() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'https://192.168.1.14:8443'
      const response = await fetch(`${API_BASE}/context/patterns`, {
        headers: {
          'x-api-key': import.meta.env.VITE_SYMBION_API_KEY || 's3cr3t-42'
        },
        credentials: 'include'
      })
      return response.ok
    } catch {}
    return false
  }

  addLine(text, className = '') {
    this.lines = [...this.lines, { text, className }]
    this.requestUpdate()
    this.autoScroll()
  }

  addLoadingLine(text) {
    const loadingIndex = this.lines.length
    this.lines = [...this.lines, { text, className: 'loading-dots' }]
    this.requestUpdate()
    this.autoScroll()
    return loadingIndex
  }

  updateLine(index, text, className = '') {
    this.lines = this.lines.map((line, i) =>
      i === index ? { text, className } : line
    )
    this.requestUpdate()
    this.autoScroll()
  }

  autoScroll() {
    this.updateComplete.then(() => {
      const terminal = this.shadowRoot.querySelector('.terminal')
      if (terminal) {
        terminal.scrollTo({
          top: terminal.scrollHeight,
          behavior: 'smooth'
        })
      }
    })
  }

  async delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  // Détecter la plateforme de l'utilisateur
  detectPlatform() {
    const userAgent = navigator.userAgent.toLowerCase()
    if (userAgent.includes('windows')) return 'windows'
    if (userAgent.includes('android')) return 'android'
    if (userAgent.includes('iphone') || userAgent.includes('ipad')) return 'ios'
    if (userAgent.includes('mac')) return 'macos'
    if (userAgent.includes('linux')) return 'linux'
    return 'unknown'
  }

  // Afficher l'interface d'installation du certificat
  showCertificateSetup() {
    const platform = this.detectPlatform()
    const certUrl = `${window.SYMBION_CONFIG.API_BASE}/ca-certificate`

    this.addLine('', '')
    this.addLine('━'.repeat(60), 'info')
    this.addLine('[setup] Première connexion détectée - Installation du certificat requise', 'info')
    this.addLine('━'.repeat(60), 'info')
    this.addLine('', '')

    // Ajouter le conteneur HTML avec le bouton
    this.showCertificateUI = true
    this.certUrl = certUrl
    this.platform = platform
    this.certVerifying = false
    this.certInstalled = false
    this.requestUpdate()

    // Ajouter les instructions spécifiques à la plateforme
    this.addPlatformInstructions(platform)
  }

  // Instructions détaillées par plateforme
  addPlatformInstructions(platform) {
    this.addLine('', '')

    switch (platform) {
      case 'windows':
        this.addLine('📝 WINDOWS - Instructions d\'installation :', 'step')
        this.addLine('', '')
        this.addLine('  1️⃣  Cliquez sur le bouton vert ci-dessus', 'info')
        this.addLine('  2️⃣  Double-cliquez sur le fichier "symbion-ca.crt" téléchargé', 'info')
        this.addLine('  3️⃣  Cliquez sur "Installer le certificat..."', 'info')
        this.addLine('  4️⃣  Sélectionnez "Ordinateur local" → Suivant', 'info')
        this.addLine('  5️⃣  Sélectionnez "Placer tous les certificats dans le magasin suivant"', 'info')
        this.addLine('  6️⃣  Cliquez sur "Parcourir" → Sélectionnez "Autorités de certification racines de confiance"', 'info')
        this.addLine('  7️⃣  Cliquez sur "Suivant" → "Terminer" → "Oui" pour confirmer', 'info')
        this.addLine('  8️⃣  Cliquez sur le bouton réessayer ci-dessous', 'info')
        break

      case 'android':
        this.addLine('📝 ANDROID - Instructions d\'installation :', 'step')
        this.addLine('', '')
        this.addLine('  1️⃣  Cliquez sur le bouton ci-dessus', 'info')
        this.addLine('  2️⃣  Ouvrez Paramètres → Sécurité → Chiffrement et identifiants', 'info')
        this.addLine('  3️⃣  Appuyez sur "Installer un certificat" → "Certificat CA"', 'info')
        this.addLine('  4️⃣  Appuyez sur "Installer quand même" si averti', 'info')
        this.addLine('  5️⃣  Sélectionnez le fichier "symbion-ca.crt" téléchargé', 'info')
        this.addLine('  6️⃣  Nommez-le "Symbion CA" → OK', 'info')
        this.addLine('  7️⃣  Cliquez sur le bouton réessayer ci-dessous', 'info')
        break

      case 'linux':
        this.addLine('📝 LINUX - Instructions d\'installation :', 'step')
        this.addLine('', '')
        this.addLine('  1️⃣  Cliquez sur le bouton ci-dessus', 'info')
        this.addLine('  2️⃣  Ouvrez un terminal et exécutez :', 'info')
        this.addLine('     sudo cp ~/Downloads/symbion-ca.crt /usr/local/share/ca-certificates/', 'warning')
        this.addLine('     sudo update-ca-certificates', 'warning')
        this.addLine('  3️⃣  Redémarrez votre navigateur', 'info')
        this.addLine('  4️⃣  Cliquez sur le bouton réessayer ci-dessous', 'info')
        break

      case 'macos':
        this.addLine('📝 MACOS - Instructions d\'installation :', 'step')
        this.addLine('', '')
        this.addLine('  1️⃣  Cliquez sur le bouton ci-dessus', 'info')
        this.addLine('  2️⃣  Double-cliquez sur le fichier téléchargé', 'info')
        this.addLine('  3️⃣  Entrez votre mot de passe si demandé', 'info')
        this.addLine('  4️⃣  Ouvrez l\'app "Trousseaux d\'accès"', 'info')
        this.addLine('  5️⃣  Trouvez "Symbion Root CA" dans le trousseau Système', 'info')
        this.addLine('  6️⃣  Double-cliquez → Confiance → "Toujours faire confiance"', 'info')
        this.addLine('  7️⃣  Fermez et cliquez sur le bouton réessayer ci-dessous', 'info')
        break

      default:
        this.addLine('📝 Instructions d\'installation :', 'step')
        this.addLine('', '')
        this.addLine('  1️⃣  Téléchargez le certificat avec le bouton ci-dessus', 'info')
        this.addLine('  2️⃣  Installez-le dans le magasin CA racine de confiance de votre système', 'info')
        this.addLine('  3️⃣  Redémarrez votre navigateur', 'info')
        this.addLine('  4️⃣  Cliquez sur le bouton réessayer ci-dessous', 'info')
    }

    this.addLine('', '')
    this.addLine('ℹ️  Pourquoi est-ce nécessaire ? Symbion utilise HTTPS avec un certificat', 'warning')
    this.addLine('   auto-signé pour la sécurité. Cette configuration unique garantit', 'warning')
    this.addLine('   que votre connexion est chiffrée et de confiance.', 'warning')
  }

  // Handler pour retry connection
  async verifyCertificateInstallation() {
    if (this.certVerifying) return

    this.certVerifying = true
    this.certInstalled = false
    this.requestUpdate()

    this.addLine('[setup] Vérification de l\'installation du certificat...', 'info')

    // Ouvrir une popup pour tester la connexion
    const testWindow = window.open(
      window.SYMBION_CONFIG.API_BASE + '/health',
      'cert_test',
      'width=400,height=300,left=1000,top=100'
    )

    if (!testWindow) {
      this.addLine('[setup] ✗ Impossible d\'ouvrir la fenêtre de test (popup bloquée)', 'error')
      this.certVerifying = false
      this.requestUpdate()
      return
    }

    // Attendre que la fenêtre charge ou échoue
    let checkCount = 0
    const checkInterval = setInterval(() => {
      checkCount++

      try {
        // Si on peut accéder à la location de la fenêtre, c'est que le cert est installé
        if (testWindow.location.href && testWindow.location.href.includes('health')) {
          clearInterval(checkInterval)
          testWindow.close()

          this.certInstalled = true
          this.certVerifying = false
          this.requestUpdate()

          this.addLine('[setup] ✓ Certificat installé avec succès !', 'success')
          this.addLine('[setup] Rechargement de l\'interface...', 'info')

          setTimeout(() => {
            location.reload()
          }, 1500)
          return
        }
      } catch (e) {
        // Exception de sécurité = pas installé correctement
      }

      // Timeout après 10 secondes
      if (checkCount > 40) {
        clearInterval(checkInterval)
        testWindow.close()

        this.certVerifying = false
        this.requestUpdate()

        this.addLine('[setup] ✗ Certificat non détecté', 'error')
        this.addLine('[setup] Assurez-vous d\'avoir installé le certificat dans le magasin système', 'warning')
        this.addLine('[setup] puis réessayez la vérification', 'warning')
      }
    }, 250)
  }

  focusInput() {
    setTimeout(() => {
      const input = this.shadowRoot.querySelector('.input-field')
      if (input) {
        input.focus()
      }
    }, 100)
  }

  get loginLabel() {
    if (this.loginStep === 'credentials') return 'identifiant & mot de passe'
    if (this.loginStep === 'totp') return 'code TOTP'
    return 'input'
  }

  async handleFormSubmit(event) {
    event.preventDefault()

    const formData = new FormData(event.target)

    if (this.loginStep === 'credentials') {
      const username = formData.get('username')?.trim()
      const password = formData.get('password')?.trim()

      if (!username) {
        this.error = 'Nom d\'utilisateur requis'
        this.requestUpdate()
        return
      }

      if (!password) {
        this.error = 'Mot de passe requis'
        this.requestUpdate()
        return
      }

      this.username = username
      this.password = password
      this.addLine(`> login: ${username}`, 'prompt')
      this.addLine('> password: ********', 'prompt')

      // Tentative d'authentification
      this.phase = 'authenticating'
      this.requestUpdate()

      const authIdx = this.addLoadingLine('[auth] Authentification')
      await this.delay(200)

      try {
        await authService.login(this.username, this.password)

        this.updateLine(authIdx, '[auth] ✓ Authentification réussie', 'success')
        await this.delay(100)
        this.addLine(`[session] Utilisateur '${this.username}' autorisé`, 'success')
        await this.delay(100)
        const dashIdx = this.addLoadingLine('[dashboard] Chargement de l\'interface')
        await this.delay(200)
        this.updateLine(dashIdx, '[dashboard] ✓ Prêt', 'success')
        await this.delay(100)

        this.phase = 'done'
        this.dispatchEvent(new CustomEvent('boot-complete', {
          detail: { authenticated: true },
          bubbles: true,
          composed: true
        }))

      } catch (error) {
        const errorMsg = error.message || 'Erreur inconnue'

        // Vérifier si MFA est requis
        if (errorMsg.includes('MFA is enabled') || errorMsg.includes('Please provide a TOTP code')) {
          this.updateLine(authIdx, '[auth] ⚠ MFA requis', 'warning')
          this.addLine('[mfa] Authentification à deux facteurs activée', 'info')
          this.addLine('[mfa] Entrez le code TOTP de votre application (6 chiffres)', 'info')
          this.loginStep = 'totp'
          this.phase = 'login'
          this.error = null
          this.requestUpdate()
          this.focusInput()
          return
        }

        // Autres erreurs (rate limiting, mauvais mot de passe, etc.)
        this.addLine('[auth] ✗ Échec d\'authentification', 'error')

        if (errorMsg.includes('Too many login attempts')) {
          // Rate limiting - afficher le message complet du backend
          this.addLine(`[auth] ${errorMsg}`, 'warning')
          this.addLine('[auth] Veuillez réessayer plus tard', 'warning')
          await this.delay(5000) // Attendre plus longtemps pour rate limit
        } else {
          // Erreur normale (mauvais mot de passe)
          this.addLine('[auth] Accès refusé. Nouvel essai dans 3s...', 'warning')
          await this.delay(3000)
        }

        // Reset login
        this.username = ''
        this.password = ''
        this.totpCode = ''
        this.loginStep = 'credentials'
        this.phase = 'login'
        this.error = null
        this.requestUpdate()
        this.focusInput()
      }

    } else if (this.loginStep === 'totp') {
      const value = formData.get('totp')?.trim()
      if (!value) {
        this.error = 'Code TOTP requis (6 chiffres)'
        this.requestUpdate()
        return
      }

      // Vérifier que le code est bien numérique et de 6 chiffres
      if (!/^\d{6,8}$/.test(value)) {
        this.error = 'Code TOTP invalide (6 à 8 chiffres requis)'
        this.requestUpdate()
        return
      }

      this.totpCode = value
      this.addLine(`> totp: ${value}`, 'prompt')

      if (this.rememberDevice) {
        this.addLine('[mfa] Appareil sera mémorisé pour 30 jours', 'info')
      }

      // Tentative d'authentification avec TOTP
      this.phase = 'authenticating'
      this.requestUpdate()

      const authIdx = this.addLoadingLine('[auth] Vérification MFA')
      await this.delay(200)

      try {
        await authService.login(this.username, this.password, this.totpCode, this.rememberDevice)

        this.updateLine(authIdx, '[auth] ✓ Authentification réussie', 'success')
        await this.delay(100)
        this.addLine(`[session] Utilisateur '${this.username}' autorisé`, 'success')
        await this.delay(100)
        const dashIdx = this.addLoadingLine('[dashboard] Chargement de l\'interface')
        await this.delay(200)
        this.updateLine(dashIdx, '[dashboard] ✓ Prêt', 'success')
        await this.delay(100)

        this.phase = 'done'
        this.dispatchEvent(new CustomEvent('boot-complete', {
          detail: { authenticated: true },
          bubbles: true,
          composed: true
        }))

      } catch (error) {
        this.addLine('[auth] ✗ Code TOTP invalide', 'error')

        const errorMsg = error.message || 'Erreur inconnue'
        this.addLine(`[auth] ${errorMsg}`, 'warning')
        this.addLine('[auth] Nouvel essai dans 3s...', 'warning')
        await this.delay(3000)

        // Reset complet du login
        this.username = ''
        this.password = ''
        this.totpCode = ''
        this.loginStep = 'credentials'
        this.phase = 'login'
        this.error = null
        this.requestUpdate()
        this.focusInput()
      }
    }
  }

  async authenticateWithBiometric() {
    if (this.authenticatingBiometric) return

    this.authenticatingBiometric = true
    this.error = null
    this.requestUpdate()

    try {
      // Mode "discoverable" : pas de username demandé !
      // L'authenticator présente toutes les passkeys disponibles
      this.addLine(`> biometric_auth`, 'prompt')
      this.phase = 'authenticating'
      this.requestUpdate()

      const authIdx = this.addLoadingLine('[auth] 🔐 Authentification biométrique')
      await this.delay(200)

      // Étape 1: Démarrer l'authentification en mode découvrable (sans username)
      const startResponse = await fetch(`${this.baseUrl}/auth/webauthn/authenticate-discoverable-start`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        }
      })

      if (!startResponse.ok) {
        const errorData = await startResponse.json()
        throw new Error(errorData.error || 'Failed to start authentication')
      }

      const requestOptions = await startResponse.json()
      console.log('[boot-terminal] Received authentication options:', requestOptions)

      // Préparer les options pour le navigateur (conversion base64 → ArrayBuffer)
      const publicKeyOptions = this.prepareAuthenticationOptions(requestOptions)
      console.log('[boot-terminal] Prepared options for browser:', publicKeyOptions)

      // Étape 2: Demander au navigateur d'authentifier avec biométrie
      // L'utilisateur va voir Touch ID, Face ID, Windows Hello, etc.
      const credential = await navigator.credentials.get({
        publicKey: publicKeyOptions
      })

      if (!credential) {
        throw new Error('Aucune passkey trouvée ou authentification annulée')
      }

      // Étape 3: Envoyer le credential au serveur pour validation
      const finishResponse = await fetch(`${this.baseUrl}/auth/webauthn/authenticate-finish`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          credential: {
            id: credential.id,
            rawId: this.arrayBufferToBase64(credential.rawId),
            response: {
              authenticatorData: this.arrayBufferToBase64(credential.response.authenticatorData),
              clientDataJSON: this.arrayBufferToBase64(credential.response.clientDataJSON),
              signature: this.arrayBufferToBase64(credential.response.signature),
              userHandle: credential.response.userHandle ? this.arrayBufferToBase64(credential.response.userHandle) : null
            },
            type: credential.type
          }
        })
      })

      if (!finishResponse.ok) {
        const errorData = await finishResponse.json()
        throw new Error(errorData.error || 'Authentication failed')
      }

      const authData = await finishResponse.json()
      console.log('[boot-terminal] Authentication successful:', authData)

      // Succès !
      this.updateLine(authIdx, '[auth] ✓ Authentification réussie', 'success')
      await this.delay(100)
      this.addLine(`[session] Utilisateur '${authData.username}' autorisé`, 'success')
      await this.delay(100)
      const dashIdx = this.addLoadingLine('[dashboard] Chargement de l\'interface')
      await this.delay(200)

      // Sauvegarder la session (même pattern que login normal dans auth-service.js:143-157)
      authService.token = authData.token
      authService.userInfo = {
        username: authData.username,
        role: authData.role,
        expires_at: authData.expires_at
      }
      authService.loginTime = Date.now()

      // Sauvegarder device token si fourni
      if (authData.device_token) {
        localStorage.setItem('symbion_device_token', authData.device_token)
        console.log('[boot-terminal] Device token saved')
      }

      authService.saveToStorage()
      authService.scheduleTokenRefresh()

      // Émettre événement de login
      authService.dispatchEvent(new CustomEvent('auth:login', {
        detail: { username: authData.username, role: authData.role }
      }))

      window.dispatchEvent(new CustomEvent('login-success', {
        detail: { username: authData.username, role: authData.role }
      }))

      this.updateLine(dashIdx, '[dashboard] ✓ Interface prête', 'success')
      await this.delay(200)

      // Animation finale
      this.addLine('[kernel] ✓ Tous les services opérationnels', 'success')
      await this.delay(200)
      this.addLine('[kernel] Bienvenue, ' + authData.username, 'prompt')
      await this.delay(500)

      this.phase = 'done'
      this.authenticatingBiometric = false
      this.requestUpdate()

      // Notifier le composant parent
      await this.delay(500)
      this.dispatchEvent(new CustomEvent('boot-complete', {
        detail: { authenticated: true },
        bubbles: true,
        composed: true
      }))

    } catch (error) {
      console.error('[boot-terminal] Biometric authentication failed:', error)

      let errorMsg = 'Authentification biométrique échouée'

      if (error.name === 'NotAllowedError') {
        errorMsg = 'Authentification annulée ou échouée. Utilisez votre biométrie.'
      } else if (error.name === 'InvalidStateError') {
        errorMsg = 'Aucune passkey trouvée pour cet utilisateur'
      } else if (error.message) {
        errorMsg = `Erreur : ${error.message}`
      }

      this.error = errorMsg
      this.addLine(`[auth] ${errorMsg}`, 'warning')
      this.addLine('[auth] Utilisez le formulaire classique pour vous connecter', 'warning')
      await this.delay(2000)

      this.phase = 'login'
      this.authenticatingBiometric = false
      this.requestUpdate()
      this.focusInput()
    }
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

  // Convertir les champs base64 en ArrayBuffer pour WebAuthn authentication
  prepareAuthenticationOptions(options) {
    // La structure peut être soit options.publicKey soit options directement
    const publicKey = options.publicKey || options.public_key || options

    return {
      challenge: this.base64ToArrayBuffer(publicKey.challenge),
      timeout: publicKey.timeout,
      rpId: publicKey.rpId || publicKey.rp_id,
      allowCredentials: publicKey.allowCredentials?.map(cred => ({
        type: cred.type,
        id: this.base64ToArrayBuffer(cred.id),
        transports: cred.transports
      })) || [],
      userVerification: publicKey.userVerification || publicKey.user_verification || 'preferred'
    }
  }

  handleTotpPaste(e) {
    // Laisser le paste natif se faire d'abord
    setTimeout(() => {
      const input = e.target
      const pastedValue = input.value
      // Nettoyer : garder seulement les chiffres
      const cleaned = pastedValue.replace(/\D/g, '').slice(0, 8)

      if (cleaned !== pastedValue) {
        input.value = cleaned
      }

      // Auto-submit si 6-8 chiffres
      if (cleaned.length >= 6 && cleaned.length <= 8) {
        console.log('[totp] Auto-submitting after paste:', cleaned.length, 'digits')
        this.totpCode = cleaned

        // Attendre un peu pour que l'utilisateur voie le code
        setTimeout(() => {
          const form = input.closest('form')
          if (form) {
            form.requestSubmit()
          }
        }, 500)
      }
    }, 0)
  }

  handleTotpInput(e) {
    const input = e.target
    let value = input.value

    // Filtrer les non-chiffres en temps réel
    const cleaned = value.replace(/\D/g, '').slice(0, 8)
    if (cleaned !== value) {
      input.value = cleaned
    }

    // Auto-submit si exactement 6 chiffres (code standard TOTP)
    if (cleaned.length === 6) {
      console.log('[totp] Auto-submitting after input: 6 digits')
      this.totpCode = cleaned

      setTimeout(() => {
        const form = input.closest('form')
        if (form) {
          form.requestSubmit()
        }
      }, 400)
    }
  }

  render() {
    return html`
      <div class="terminal">
        ${this.lines.map(line => html`
          <div class="line ${line.type}">${line.text}</div>
        `)}

        ${this.showCertificateUI ? html`
          <div class="cert-setup-box">
            <div style="margin-bottom: 1rem;">
              <span class="platform-badge">${this.platform.toUpperCase()}</span>
              <span style="color: #00ff9f; font-weight: 600;">Installation du certificat requise</span>
            </div>

            <a href="${this.certUrl}"
               class="cert-download-btn"
               download="symbion-ca.crt">
              📥 Télécharger le certificat CA Symbion
            </a>

            <button
              class="retry-btn"
              ?disabled="${this.certVerifying}"
              @click="${this.verifyCertificateInstallation}">
              ${this.certVerifying
                ? '⏳ Vérification en cours...'
                : this.certInstalled
                  ? '✓ Certificat installé'
                  : '🔍 Vérifier l\'installation du certificat'
              }
            </button>
          </div>
        ` : ''}

        ${this.phase === 'login' && this.loginStep === 'credentials' ? html`
          <form @submit="${this.handleFormSubmit}" class="login-form" name="login-form">
            <div class="input-line">
              <label for="username">> identifiant: </label>
              <input
                id="username"
                name="username"
                type="text"
                class="input-field"
                autocomplete="username"
                placeholder="_"
                autofocus>
              <span class="cursor"></span>
            </div>
            <div class="input-line" style="margin-top: 0.75rem;">
              <label for="password">> mot de passe: </label>
              <input
                id="password"
                name="password"
                type="password"
                class="input-field"
                autocomplete="current-password"
                placeholder="_">
              <span class="cursor"></span>
            </div>
            <button type="submit" style="display: none;">Submit</button>
          </form>

          ${this.biometricAvailable ? html`
            <div class="biometric-separator">
              <span>ou</span>
            </div>
            <button
              class="biometric-btn"
              @click="${this.authenticateWithBiometric}"
              ?disabled="${this.authenticatingBiometric}">
              🔐 Connexion biométrique
              ${this.authenticatingBiometric ? html` <span class="spinner"></span>` : ''}
            </button>
          ` : ''}
        ` : ''}

        ${this.phase === 'login' && this.loginStep === 'totp' ? html`
          <form @submit="${this.handleFormSubmit}" class="login-form" name="login-totp">
            <div class="input-line">
              <label for="totp">> code TOTP: </label>
              <input
                id="totp"
                name="totp"
                type="tel"
                class="input-field"
                inputmode="numeric"
                autocomplete="one-time-code"
                pattern="[0-9]{6,8}"
                maxlength="8"
                placeholder="123456"
                aria-label="Code TOTP à 6 chiffres"
                autofocus
                @paste="${this.handleTotpPaste}"
                @input="${this.handleTotpInput}">
              <span class="cursor"></span>
            </div>
            <div class="input-line" style="margin-top: 0.5rem; padding-left: 1rem;">
              <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer; font-size: 0.9em; color: #aaa;">
                <input
                  type="checkbox"
                  name="rememberDevice"
                  .checked="${this.rememberDevice}"
                  @change="${(e) => this.rememberDevice = e.target.checked}"
                  style="cursor: pointer;">
                <span>Ne plus demander sur cet appareil (30 jours)</span>
              </label>
            </div>
            <button type="submit" style="display: none;">Submit</button>
          </form>
        ` : ''}

        ${this.phase === 'login' && this.error ? html`
          <div class="line error">[error] ${this.error}</div>
        ` : ''}
      </div>
    `
  }
}

customElements.define('boot-terminal', BootTerminal)

export default BootTerminal
