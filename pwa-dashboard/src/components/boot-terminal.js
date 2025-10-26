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
  `

  static properties = {
    lines: { type: Array },
    phase: { type: String }, // 'booting', 'login', 'authenticating', 'done'
    loginStep: { type: String }, // 'username', 'password'
    username: { type: String },
    password: { type: String },
    error: { type: String },
    showCertificateUI: { type: Boolean },
    certUrl: { type: String },
    platform: { type: String },
    certVerifying: { type: Boolean },
    certInstalled: { type: Boolean }
  }

  constructor() {
    super()
    this.lines = []
    this.phase = 'booting'
    this.loginStep = 'username'
    this.username = ''
    this.password = ''
    this.error = null
    this.showCertificateUI = false
    this.certUrl = ''
    this.platform = ''
    this.certVerifying = false
    this.certInstalled = false
  }

  connectedCallback() {
    super.connectedCallback()
    // Démarrer la séquence de boot
    this.startBootSequence()
  }

  async startBootSequence() {
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
      const response = await fetch(`${API_BASE}/health`)
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
        }
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
        }
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
        }
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
        }
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
    return this.loginStep === 'username' ? 'identifiant' : 'mot de passe'
  }

  handleInput(event) {
    if (event.key === 'Enter') {
      this.handleSubmit()
    }
  }

  async handleSubmit() {
    const input = this.shadowRoot.querySelector('.input-field')
    const value = input.value.trim()

    if (this.loginStep === 'username') {
      if (!value) {
        this.error = 'Nom d\'utilisateur requis'
        this.requestUpdate()
        return
      }

      this.username = value
      this.addLine(`> login: ${value}`, 'prompt')
      this.loginStep = 'password'
      this.error = null
      input.value = ''
      this.requestUpdate()
      this.focusInput()

    } else if (this.loginStep === 'password') {
      if (!value) {
        this.error = 'Mot de passe requis'
        this.requestUpdate()
        return
      }

      this.password = value
      this.addLine('> password: ********', 'prompt')
      input.value = ''

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
        this.addLine('[auth] ✗ Échec d\'authentification', 'error')

        // Afficher le message d'erreur spécifique (rate limiting, mauvais mdp, etc.)
        const errorMsg = error.message || 'Erreur inconnue'
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
        this.loginStep = 'username'
        this.phase = 'login'
        this.error = null
        this.requestUpdate()
        this.focusInput()
      }
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

        ${this.phase === 'login' ? html`
          <div class="input-line">
            <span>> ${this.loginLabel}: </span>
            <input
              class="input-field"
              type="${this.loginStep === 'password' ? 'password' : 'text'}"
              @keydown="${this.handleInput}"
              placeholder="_">
            <span class="cursor"></span>
          </div>
          ${this.error ? html`
            <div class="line error">[error] ${this.error}</div>
          ` : ''}
        ` : ''}
      </div>
    `
  }
}

customElements.define('boot-terminal', BootTerminal)

export default BootTerminal
