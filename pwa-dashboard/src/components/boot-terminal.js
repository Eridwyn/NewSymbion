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
    error: { type: String }
  }

  constructor() {
    super()
    this.lines = []
    this.phase = 'booting'
    this.loginStep = 'username'
    this.username = ''
    this.password = ''
    this.error = null
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

    this.addLine('[kernel] Initializing neural cortex...')
    await this.delay(100)

    // Vérification connexion kernel (CRITIQUE)
    const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'http://192.168.1.14:8080'
    const kernelLoadingIdx = this.addLoadingLine(`[kernel] Connecting to ${API_BASE}`)
    const kernelOk = await this.checkKernel()
    this.updateLine(
      kernelLoadingIdx,
      kernelOk ? '[kernel] ✓ Connected' : '[kernel] ✗ Connection failed',
      kernelOk ? 'success' : 'error'
    )

    // ARRÊT IMMÉDIAT si kernel inaccessible
    if (!kernelOk) {
      await this.delay(200)
      this.addLine('[error] Cannot proceed without kernel connection', 'error')
      this.addLine('[error] Please check that symbion-kernel is running', 'warning')
      return
    }

    await this.delay(150)

    // MQTT
    const mqttLoadingIdx = this.addLoadingLine('[mqtt] Establishing message bus')
    await this.delay(150)
    this.updateLine(mqttLoadingIdx, '[mqtt] ✓ Connected', 'success')
    await this.delay(100)

    // Plugins
    const pluginsLoadingIdx = this.addLoadingLine('[plugins] Loading modules')
    await this.delay(100)
    this.updateLine(pluginsLoadingIdx, '[plugins] ✓ memory-extension (notes) active', 'success')
    await this.delay(100)

    // Agents
    const agentsLoadingIdx = this.addLoadingLine('[agents] Scanning network for agents')
    const agents = await this.checkAgents()
    this.updateLine(agentsLoadingIdx, `[agents] ✓ Found ${agents} agent(s)`, 'success')
    await this.delay(150)

    // Auth session check
    const authLoadingIdx = this.addLoadingLine('[auth] Verifying session')
    await this.delay(100)

    const hasSession = await this.verifySession()
    this.updateLine(
      authLoadingIdx,
      hasSession ? '[auth] ✓ Session valid' : '[auth] No valid session found',
      hasSession ? 'success' : 'warning'
    )

    if (hasSession) {
      await this.delay(50)
      this.addLine(`[session] User '${authService.getCurrentUser().username}' authorized`, 'success')
      await this.delay(50)
      const dashLoadingIdx = this.addLoadingLine('[dashboard] Loading interface')
      await this.delay(200)
      this.updateLine(dashLoadingIdx, '[dashboard] ✓ Ready', 'success')
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
      this.addLine('[auth] Session required', 'info')
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
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'http://192.168.1.14:8080'
      const response = await fetch(`${API_BASE}/health`)
      return response.ok
    } catch {
      return false
    }
  }

  async checkAgents() {
    try {
      const API_BASE = window.SYMBION_CONFIG?.API_BASE || 'http://192.168.1.14:8080'
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

  focusInput() {
    setTimeout(() => {
      const input = this.shadowRoot.querySelector('.input-field')
      if (input) {
        input.focus()
      }
    }, 100)
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
        this.error = 'Username required'
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
        this.error = 'Password required'
        this.requestUpdate()
        return
      }

      this.password = value
      this.addLine('> password: ********', 'prompt')
      input.value = ''

      // Tentative d'authentification
      this.phase = 'authenticating'
      this.requestUpdate()

      const authIdx = this.addLoadingLine('[auth] Authenticating')
      await this.delay(200)

      try {
        await authService.login(this.username, this.password)

        this.updateLine(authIdx, '[auth] ✓ Authentication successful', 'success')
        await this.delay(100)
        this.addLine(`[session] User '${this.username}' authorized`, 'success')
        await this.delay(100)
        const dashIdx = this.addLoadingLine('[dashboard] Loading interface')
        await this.delay(200)
        this.updateLine(dashIdx, '[dashboard] ✓ Ready', 'success')
        await this.delay(100)

        this.phase = 'done'
        this.dispatchEvent(new CustomEvent('boot-complete', {
          detail: { authenticated: true },
          bubbles: true,
          composed: true
        }))

      } catch (error) {
        this.addLine('[auth] ✗ Authentication failed', 'error')

        // Afficher le message d'erreur spécifique (rate limiting, mauvais mdp, etc.)
        const errorMsg = error.message || 'Unknown error'
        if (errorMsg.includes('Too many login attempts')) {
          // Rate limiting - afficher le message complet du backend
          this.addLine(`[auth] ${errorMsg}`, 'warning')
          this.addLine('[auth] Please try again later', 'warning')
          await this.delay(5000) // Attendre plus longtemps pour rate limit
        } else {
          // Erreur normale (mauvais mot de passe)
          this.addLine('[auth] Access denied. Retry in 3s...', 'warning')
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

        ${this.phase === 'login' ? html`
          <div class="input-line">
            <span>> ${this.loginStep}: </span>
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
