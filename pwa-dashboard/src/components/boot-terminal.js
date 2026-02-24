/**
 * Login Page Component - Bio-Organic Modern Design
 *
 * Page de connexion moderne avec:
 * - Glassmorphisme bio-organique
 * - Authentification standard (username/password)
 * - Support MFA/TOTP
 * - Authentification biométrique (WebAuthn)
 * - Animations fluides et professionnelles
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { focusVisibleStyles } from '../styles/shared-patterns.js'
import authService from '../services/auth-service.js'

class BootTerminal extends LitElement {
  static styles = [sharedAnimations, focusVisibleStyles, css`
    :host {
      display: block;
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      /* Bio-organic gradient background avec contextual tint */
      background: radial-gradient(ellipse at top center,
        color-mix(in srgb, var(--context-primary, #00d4aa) 4%, var(--color-dark-bg, #0a0a0b)) 0%,
        var(--color-dark-bg, #0a0a0b) 100%),
        linear-gradient(135deg,
          var(--ctx-bg-subtle, rgba(0, 212, 170, 0.02)) 0%,
          transparent 100%);
      z-index: 1200;
      overflow-y: auto;
      animation: backgroundBreathing 10s ease-in-out infinite;
    }

    @keyframes backgroundBreathing {
      0%, 100% {
        background: radial-gradient(ellipse at top center,
          color-mix(in srgb, var(--context-primary, #00d4aa) 4%, var(--color-dark-bg, #0a0a0b)) 0%,
          var(--color-dark-bg, #0a0a0b) 100%);
      }
      50% {
        background: radial-gradient(ellipse at top center,
          color-mix(in srgb, var(--context-primary, #00d4aa) 6%, var(--color-dark-bg, #0a0a0b)) 0%,
          var(--color-dark-bg, #0a0a0b) 100%);
      }
    }

    .login-container {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 2rem;
      position: relative;
    }

    /* Bio-organic ambient particles */
    .login-container::before,
    .login-container::after {
      content: '';
      position: absolute;
      border-radius: 50%;
      filter: blur(80px);
      opacity: 0.15;
      animation: float 20s ease-in-out infinite;
      pointer-events: none;
    }

    .login-container::before {
      width: 400px;
      height: 400px;
      background: radial-gradient(circle, var(--context-primary, #00d4aa), transparent);
      top: 10%;
      left: 20%;
      animation-delay: -5s;
    }

    .login-container::after {
      width: 350px;
      height: 350px;
      background: radial-gradient(circle, var(--context-primary, #00d4aa), transparent);
      bottom: 15%;
      right: 15%;
    }

    .login-card {
      width: 100%;
      max-width: 420px;
      background: linear-gradient(135deg,
        var(--app-widget-bg-a, rgba(30, 30, 30, 0.95)) 0%,
        var(--app-widget-bg-b, rgba(20, 20, 20, 0.98)) 100%);
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-xl, 1.5rem);
      padding: 3rem 2.5rem;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.7),
                  0 0 60px var(--ctx-border-subtle),
                  inset 0 1px 0 var(--ctx-border-subtle);
      backdrop-filter: blur(var(--blur-xl, 20px));
      -webkit-backdrop-filter: blur(var(--blur-xl, 20px));
      position: relative;
      z-index: 1;
      animation: cardSlideIn 0.6s cubic-bezier(0.4, 0, 0.2, 1),
                 cardBreathing 8s ease-in-out infinite 1s;
    }

    @keyframes cardBreathing {
      0%, 100% {
        box-shadow: 0 24px 48px rgba(0, 0, 0, 0.7),
                    0 0 60px var(--ctx-border-subtle),
                    inset 0 1px 0 var(--ctx-border-subtle);
      }
      50% {
        box-shadow: 0 28px 56px rgba(0, 0, 0, 0.75),
                    0 0 80px var(--ctx-border),
                    inset 0 1px 0 var(--ctx-border);
      }
    }

    @keyframes cardSlideIn {
      from {
        opacity: 0;
        transform: translateY(40px) scale(0.95);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    /* Logo Section */
    .logo-section {
      text-align: center;
      margin-bottom: 2.5rem;
      animation: logoFadeIn 0.8s ease-out 0.2s backwards;
    }

    @keyframes logoFadeIn {
      from {
        opacity: 0;
        transform: translateY(-10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .logo-image {
      width: 64px;
      height: 64px;
      margin: 0 auto 1.5rem;
      /* Anti-aliasing optimisé pour netteté */
      image-rendering: auto;
      -webkit-backface-visibility: hidden;
      backface-visibility: hidden;
      transform: translateZ(0);
      will-change: filter, transform;
      /* Optimisation rendu GPU */
      -webkit-transform: translateZ(0);
      -webkit-font-smoothing: antialiased;
      /* Logo déjà en couleur turquoise native - juste ajouter glow */
      filter: drop-shadow(0 0 20px color-mix(in srgb, var(--context-primary) 60%, transparent))
              drop-shadow(0 0 40px var(--ctx-border-strong))
              brightness(1.05);
      animation: logoPulse 4s ease-in-out infinite,
                 logoFloat 6s ease-in-out infinite;
    }

    @keyframes logoPulse {
      0%, 100% {
        filter: drop-shadow(0 0 20px color-mix(in srgb, var(--context-primary) 60%, transparent))
                drop-shadow(0 0 40px var(--ctx-border-strong))
                brightness(1.05);
      }
      50% {
        filter: drop-shadow(0 0 30px color-mix(in srgb, var(--context-primary) 80%, transparent))
                drop-shadow(0 0 60px color-mix(in srgb, var(--context-primary) 60%, transparent))
                brightness(1.1);
      }
    }

    @keyframes logoFloat {
      0%, 100% {
        transform: translateY(0) translateZ(0);
      }
      50% {
        transform: translateY(-8px) translateZ(0);
      }
    }

    .logo-title {
      font-size: 1.75rem;
      font-weight: 700;
      background: linear-gradient(90deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white) 25%,
        var(--context-primary, #00d4aa) 50%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white) 75%,
        var(--context-primary, #00d4aa) 100%);
      background-size: 200% 100%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      margin-bottom: 0.5rem;
      letter-spacing: 0.02em;
      animation: titleShimmer 6s ease-in-out infinite;
    }

    @keyframes titleShimmer {
      0%, 100% {
        background-position: 0% 50%;
      }
      50% {
        background-position: 100% 50%;
      }
    }

    .logo-subtitle {
      font-size: var(--text-sm);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-weight: 400;
      animation: subtitleFade 3s ease-in-out infinite;
    }

    @keyframes subtitleFade {
      0%, 100% {
        opacity: 0.7;
      }
      50% {
        opacity: 1;
      }
    }

    /* Form Styles */
    .login-form {
      display: flex;
      flex-direction: column;
      gap: 1.25rem;
    }

    .form-group {
      animation: formGroupSlideIn 0.5s ease-out backwards;
    }

    .form-group:nth-child(1) { animation-delay: 0.3s; }
    .form-group:nth-child(2) { animation-delay: 0.4s; }
    .form-group:nth-child(3) { animation-delay: 0.5s; }

    @keyframes formGroupSlideIn {
      from {
        opacity: 0;
        transform: translateX(-20px);
      }
      to {
        opacity: 1;
        transform: translateX(0);
      }
    }

    .form-label {
      display: block;
      font-size: var(--text-sm);
      font-weight: 500;
      color: var(--color-dark-text-primary, #e5e7eb);
      margin-bottom: 0.5rem;
      letter-spacing: 0.01em;
    }

    .form-input {
      width: 100%;
      max-width: 100%;
      min-width: 0;
      box-sizing: border-box;
      padding: 0.875rem 1.125rem;
      background: var(--app-input-bg, rgba(0, 0, 0, 0.4));
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      color: var(--color-dark-text-primary, #f3f4f6);
      font-size: 0.9375rem;
      font-family: inherit;
      transition: all var(--duration-base, 0.3s) ease;
      outline: none;
    }

    .form-input::placeholder {
      color: var(--app-placeholder, rgba(156, 163, 175, 0.5));
    }

    .form-input:focus {
      background: var(--surface-glass-strong, rgba(0, 0, 0, 0.1));
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 4px var(--ctx-border),
                  0 0 20px var(--ctx-border-medium);
      animation: inputGlow 0.6s ease-out;
    }

    .submit-btn {
      width: 100%;
      padding: 1rem 1.5rem;
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 80%, #0066cc) 100%);
      border: none;
      border-radius: var(--radius-md);
      color: var(--color-dark-bg, #0a0a0b);
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-base, 0.3s) ease;
      box-shadow: 0 4px 12px var(--ctx-bg-intense),
                  0 0 20px var(--ctx-border);
      margin-top: 0.75rem;
      animation: btnFadeIn 0.6s ease-out 0.6s backwards;
    }

    @keyframes btnFadeIn {
      from {
        opacity: 0;
        transform: translateY(10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .submit-btn:hover:not(:disabled) {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 110%, white) 0%,
        var(--context-primary, #00d4aa) 100%);
      transform: translateY(-2px) scale(1.02);
      box-shadow: 0 8px 20px var(--ctx-border-strong),
                  0 0 30px var(--ctx-bg-emphasis);
      animation: btnPulse 1.5s ease-in-out infinite;
    }

    @keyframes btnPulse {
      0%, 100% {
        box-shadow: 0 8px 20px var(--ctx-border-strong),
                    0 0 30px var(--ctx-bg-emphasis);
      }
      50% {
        box-shadow: 0 8px 24px var(--ctx-border-intense),
                    0 0 40px var(--ctx-bg-intense);
      }
    }

    .submit-btn:active:not(:disabled) {
      transform: translateY(0) scale(0.98);
    }

    .submit-btn:disabled {
      opacity: 0.6;
      cursor: not-allowed;
      transform: none;
    }

    /* Biometric Auth */
    .divider {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin: 1.75rem 0;
      opacity: 0.6;
      animation: dividerFadeIn 0.5s ease-out 0.7s backwards;
    }

    @keyframes dividerFadeIn {
      from {
        opacity: 0;
      }
      to {
        opacity: 0.6;
      }
    }

    .divider::before,
    .divider::after {
      content: '';
      flex: 1;
      height: 1px;
      background: linear-gradient(90deg,
        transparent,
        var(--ctx-bg-intense),
        transparent);
    }

    .divider-text {
      font-size: 0.8125rem;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-weight: 500;
    }

    .biometric-btn {
      width: 100%;
      padding: 1rem 1.5rem;
      background: linear-gradient(135deg,
        var(--ctx-bg-medium) 0%,
        var(--ctx-bg) 100%);
      border: 1px solid var(--ctx-bg-emphasis);
      border-radius: var(--radius-md);
      color: var(--context-primary, #00d4aa);
      font-size: 0.9375rem;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-base, 0.3s) ease;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.625rem;
      animation: biometricBtnFadeIn 0.5s ease-out 0.8s backwards;
    }

    @keyframes biometricBtnFadeIn {
      from {
        opacity: 0;
        transform: translateY(10px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }

    .biometric-btn:hover:not(:disabled) {
      background: linear-gradient(135deg,
        var(--ctx-border) 0%,
        var(--ctx-bg-medium) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-2px) scale(1.01);
      box-shadow: 0 6px 16px var(--ctx-border-medium),
                  0 0 24px var(--ctx-bg-medium);
      animation: biometricGlow 2s ease-in-out infinite;
    }

    @keyframes biometricGlow {
      0%, 100% {
        box-shadow: 0 6px 16px var(--ctx-border-medium),
                    0 0 24px var(--ctx-bg-medium);
      }
      50% {
        box-shadow: 0 6px 20px var(--ctx-bg-intense),
                    0 0 32px color-mix(in srgb, var(--context-primary, #00d4aa) 18%, transparent);
      }
    }

    .biometric-btn:active:not(:disabled) {
      transform: translateY(0) scale(0.98);
    }

    .biometric-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      transform: none;
    }

    /* Error Message */
    .error-message {
      padding: 0.875rem 1.125rem;
      background: linear-gradient(135deg,
        var(--color-danger-bg, rgba(255, 107, 107, 0.15)) 0%,
        color-mix(in srgb, var(--color-danger-text, #ff6b6b) 8%, transparent) 100%);
      border: 1px solid var(--color-danger-border, rgba(255, 107, 107, 0.3));
      border-radius: var(--radius-md);
      color: var(--color-danger-text-muted, #ff6b6b);
      font-size: var(--text-sm);
      margin-bottom: 1.25rem;
      animation: errorShake 0.4s ease-out;
    }

    @keyframes errorShake {
      0%, 100% { transform: translateX(0); }
      25% { transform: translateX(-8px); }
      75% { transform: translateX(8px); }
    }

    /* Loading State */
    .loading-state {
      text-align: center;
      padding: 2rem 0;
    }

    .spinner {
      width: 48px;
      height: 48px;
      border: 4px solid var(--border-medium);
      border-top-color: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin: 0 auto 1.5rem;
    }

    .loading-text {
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: 0.9375rem;
      animation: pulse 2s ease-in-out infinite;
    }

    /* Success State */
    .success-state {
      text-align: center;
      padding: 2rem 0;
      animation: successFadeIn 0.5s ease-out;
    }

    @keyframes successFadeIn {
      from {
        opacity: 0;
        transform: scale(0.9);
      }
      to {
        opacity: 1;
        transform: scale(1);
      }
    }

    .success-icon {
      font-size: 4rem;
      margin-bottom: 1rem;
      animation: successBounce 0.6s cubic-bezier(0.68, -0.55, 0.265, 1.55);
    }

    @keyframes successBounce {
      0% {
        transform: scale(0);
        opacity: 0;
      }
      50% {
        transform: scale(1.2);
      }
      100% {
        transform: scale(1);
        opacity: 1;
      }
    }

    .success-text {
      color: var(--context-primary, #00d4aa);
      font-size: var(--text-lg);
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .success-subtext {
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: var(--text-sm);
    }

    /* Checkbox */
    .checkbox-group {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      margin-top: 0.5rem;
    }

    .checkbox-input {
      width: 18px;
      height: 18px;
      accent-color: var(--context-primary, #00d4aa);
      cursor: pointer;
    }

    .checkbox-label {
      font-size: var(--text-sm);
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      user-select: none;
    }

    /* Tablet Responsive (768px and below) */
    @media (max-width: 768px) {
      .login-container {
        padding: 2rem 1.5rem;
      }

      .login-card {
        max-width: 90%;
        padding: 2.5rem 2rem;
        border-radius: var(--radius-xl, 1.5rem);
      }

      .logo-title {
        font-size: 1.75rem;
      }

      .logo-subtitle {
        font-size: 0.85rem;
      }

      /* Réduire les particules ambiantes sur tablette */
      .login-container::before {
        width: 300px;
        height: 300px;
        top: -80px;
        left: -80px;
      }

      .login-container::after {
        width: 250px;
        height: 250px;
        bottom: -60px;
        right: -60px;
      }
    }

    /* Mobile Large (up to 768px) */
    @media (max-width: 768px) {
      .login-container {
        padding: 1.5rem 1rem;
        justify-content: flex-start;
        padding-top: 3rem;
      }

      .login-card {
        max-width: 95%;
        padding: 2rem 1.5rem;
        border-radius: var(--radius-xl);
        box-shadow: 0 16px 32px rgba(0, 0, 0, 0.6),
                    0 0 40px var(--ctx-bg);
      }

      .logo-image {
        width: 56px;
        height: 56px;
        margin-bottom: 1rem;
      }

      .logo-title {
        font-size: 1.6rem;
        margin-bottom: 0.25rem;
      }

      .logo-subtitle {
        font-size: 0.8rem;
        margin-bottom: 1.5rem;
      }

      .form-group {
        margin-bottom: 1rem;
      }

      .form-label {
        font-size: 0.8rem;
        margin-bottom: 0.4rem;
      }

      .form-input {
        padding: 0.875rem 1rem;
        font-size: 0.9rem;
        border-radius: var(--radius-md, 0.75rem);
      }

      .submit-btn,
      .biometric-btn {
        padding: 0.875rem 1rem;
        font-size: 0.9rem;
        border-radius: var(--radius-md, 0.75rem);
      }

      .or-divider {
        margin: 1.25rem 0;
        font-size: var(--text-xs);
      }

      /* Réduire particules sur mobile */
      .login-container::before {
        width: 200px;
        height: 200px;
        top: -50px;
        left: -50px;
        filter: blur(60px);
      }

      .login-container::after {
        width: 180px;
        height: 180px;
        bottom: -40px;
        right: -40px;
        filter: blur(60px);
      }
    }

    /* Mobile Small (480px and below) */
    @media (max-width: 480px) {
      .login-container {
        padding: 1rem 0.75rem;
        padding-top: 2rem;
        min-height: 100vh;
        min-height: 100dvh; /* Dynamic viewport height pour mobile */
      }

      .login-card {
        max-width: 100%;
        padding: 1.75rem 1.25rem;
        border-radius: var(--radius-lg, 1rem);
        box-shadow: 0 12px 24px rgba(0, 0, 0, 0.5),
                    0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 6%, transparent);
      }

      .logo-image {
        width: 50px;
        height: 50px;
        margin-bottom: 0.75rem;
      }

      .logo-title {
        font-size: 1.4rem;
        margin-bottom: 0.2rem;
      }

      .logo-subtitle {
        font-size: var(--text-xs);
        margin-bottom: 1.25rem;
      }

      .form-group {
        margin-bottom: 0.875rem;
      }

      .form-label {
        font-size: var(--text-xs);
        margin-bottom: 0.35rem;
      }

      .form-input {
        padding: 0.8rem 0.875rem;
        font-size: var(--text-sm);
        border-radius: var(--radius-md, 0.75rem);
      }

      .form-input::placeholder {
        font-size: 0.85rem;
      }

      .submit-btn,
      .biometric-btn {
        padding: 0.8rem 0.875rem;
        font-size: var(--text-sm);
        border-radius: var(--radius-md, 0.75rem);
      }

      .or-divider {
        margin: 1rem 0;
        font-size: 0.7rem;
      }

      .checkbox-label {
        font-size: 0.8rem;
      }

      /* Désactiver particules ambiantes sur petit mobile (performance) */
      .login-container::before,
      .login-container::after {
        display: none;
      }

      /* Réduire intensité glassmorphisme (performance) */
      .login-card {
        backdrop-filter: blur(var(--blur-lg, 16px));
      }

      /* Success animation plus légère */
      .success-container {
        gap: 1rem;
      }

      .success-icon {
        font-size: 3.5rem;
      }

      .success-title {
        font-size: var(--text-2xl);
      }

      .success-message {
        font-size: 0.85rem;
      }
    }

    /* Mobile Extra Small (360px and below) */
    @media (max-width: 360px) {
      .login-container {
        padding: 0.75rem 0.5rem;
        padding-top: 1.5rem;
      }

      .login-card {
        padding: 1.5rem 1rem;
        border-radius: var(--radius-lg);
      }

      .logo-image {
        width: 45px;
        height: 45px;
        margin-bottom: 0.5rem;
      }

      .logo-title {
        font-size: 1.3rem;
      }

      .logo-subtitle {
        font-size: 0.7rem;
        margin-bottom: 1rem;
      }

      .form-group {
        margin-bottom: 0.75rem;
      }

      .form-label {
        font-size: 0.7rem;
        margin-bottom: 0.3rem;
      }

      .form-input,
      .submit-btn,
      .biometric-btn {
        padding: 0.75rem 0.75rem;
        font-size: 0.85rem;
        border-radius: var(--radius-base);
      }

      .or-divider {
        margin: 0.875rem 0;
        font-size: 0.65rem;
      }

      .checkbox-label {
        font-size: var(--text-xs);
      }

      .success-icon {
        font-size: 3rem;
      }

      .success-title {
        font-size: 1.3rem;
      }

      .success-message {
        font-size: 0.8rem;
      }
    }

    /* Utility classes (ex-inline) */
    .bt-spinner-sm { width: 16px; height: 16px; border-width: 2px; margin: 0; }
  `]

  static properties = {
    phase: { type: String }, // 'login', 'authenticating', 'success'
    loginStep: { type: String }, // 'credentials', 'totp'
    username: { type: String },
    password: { type: String },
    totpCode: { type: String },
    rememberDevice: { type: Boolean },
    error: { type: String },
    authenticatingBiometric: { type: Boolean },
    biometricAvailable: { type: Boolean }
  }

  constructor() {
    super()
    this.phase = 'login'
    this.loginStep = 'credentials'
    this.username = ''
    this.password = ''
    this.totpCode = ''
    this.rememberDevice = false
    this.error = null
    this.authenticatingBiometric = false
    this.biometricAvailable = false
    this.baseUrl = window.SYMBION_CONFIG?.API_BASE || 'https://symbion.local:8443'
  }

  async connectedCallback() {
    super.connectedCallback()

    // Check biometric availability
    await this.checkBiometricAvailability()

    // Check if already authenticated
    if (authService.isAuthenticated()) {
      this.phase = 'success'
      setTimeout(() => {
        this.dispatchEvent(new CustomEvent('boot-complete', {
          detail: { authenticated: true },
          bubbles: true,
          composed: true
        }))
      }, 1000)
    }
  }

  async checkBiometricAvailability() {
    if (window.PublicKeyCredential) {
      try {
        const available = await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()
        this.biometricAvailable = available
      } catch (error) {
        this.biometricAvailable = false
      }
    }
  }

  async handleFormSubmit(event) {
    event.preventDefault()
    this.error = null

    const formData = new FormData(event.target)

    if (this.loginStep === 'credentials') {
      const username = formData.get('username')?.trim()
      const password = formData.get('password')?.trim()

      if (!username || !password) {
        this.error = 'Veuillez remplir tous les champs'
        return
      }

      this.username = username
      this.password = password
      this.phase = 'authenticating'

      try {
        await authService.login(this.username, this.password)
        this.phase = 'success'

        setTimeout(() => {
          this.dispatchEvent(new CustomEvent('boot-complete', {
            detail: { authenticated: true },
            bubbles: true,
            composed: true
          }))
        }, 1500)

      } catch (error) {
        const errorMsg = error.message || 'Erreur d\'authentification'

        if (errorMsg.includes('MFA') || errorMsg.includes('TOTP')) {
          this.loginStep = 'totp'
          this.phase = 'login'
          this.error = null
          return
        }

        this.error = errorMsg
        this.phase = 'login'

        setTimeout(() => {
          this.username = ''
          this.password = ''
        }, 2000)
      }

    } else if (this.loginStep === 'totp') {
      const value = formData.get('totp')?.trim()

      if (!value || !/^\d{6,8}$/.test(value)) {
        this.error = 'Code TOTP invalide (6-8 chiffres requis)'
        return
      }

      this.totpCode = value
      this.phase = 'authenticating'

      try {
        await authService.login(this.username, this.password, this.totpCode, this.rememberDevice)
        this.phase = 'success'

        setTimeout(() => {
          this.dispatchEvent(new CustomEvent('boot-complete', {
            detail: { authenticated: true },
            bubbles: true,
            composed: true
          }))
        }, 1500)

      } catch (error) {
        this.error = error.message || 'Code TOTP invalide'
        this.phase = 'login'

        setTimeout(() => {
          this.username = ''
          this.password = ''
          this.totpCode = ''
          this.loginStep = 'credentials'
        }, 2000)
      }
    }
  }

  async authenticateWithBiometric() {
    if (this.authenticatingBiometric) return

    this.authenticatingBiometric = true
    this.error = null
    this.phase = 'authenticating'

    try {
      // Start discoverable authentication
      const startResponse = await fetch(`${this.baseUrl}/auth/webauthn/authenticate-discoverable-start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
      })

      if (!startResponse.ok) {
        throw new Error('Failed to start authentication')
      }

      const requestOptions = await startResponse.json()
      const publicKeyOptions = this.prepareAuthenticationOptions(requestOptions)

      const credential = await navigator.credentials.get({
        publicKey: publicKeyOptions
      })

      if (!credential) {
        throw new Error('Authentification annulée')
      }

      // Send credential to server
      const finishResponse = await fetch(`${this.baseUrl}/auth/webauthn/authenticate-finish`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
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
        throw new Error('Authentication failed')
      }

      const authData = await finishResponse.json()

      // Save session
      authService.token = authData.token
      authService.userInfo = {
        username: authData.username,
        role: authData.role,
        expires_at: authData.expires_at
      }
      authService.loginTime = Date.now()

      if (authData.device_token) {
        localStorage.setItem('symbion_device_token', authData.device_token)
      }

      authService.saveToStorage()
      authService.scheduleTokenRefresh()

      authService.dispatchEvent(new CustomEvent('auth:login', {
        detail: { username: authData.username, role: authData.role }
      }))

      window.dispatchEvent(new CustomEvent('login-success', {
        detail: { username: authData.username, role: authData.role }
      }))

      this.phase = 'success'
      this.authenticatingBiometric = false

      setTimeout(() => {
        this.dispatchEvent(new CustomEvent('boot-complete', {
          detail: { authenticated: true },
          bubbles: true,
          composed: true
        }))
      }, 1500)

    } catch (error) {
      this.error = error.name === 'NotAllowedError'
        ? 'Authentification annulée'
        : error.message || 'Erreur d\'authentification biométrique'
      this.phase = 'login'
      this.authenticatingBiometric = false
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
    const binaryString = atob(base64.replace(/-/g, '+').replace(/_/g, '/'))
    const bytes = new Uint8Array(binaryString.length)
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }
    return bytes.buffer
  }

  prepareAuthenticationOptions(options) {
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

  render() {
    return html`
      <div class="login-container">
        <div class="login-card">
          <!-- Logo Section -->
          <div class="logo-section">
            <img
              src="/icon-512-transparent-v2.png"
              srcset="/icon-192-transparent-v2.png 1x, /icon-512-transparent-v2.png 2x, /icon-1024-transparent-v2.png 3x"
              alt="Symbion"
              class="logo-image"
              width="64"
              height="64"
              loading="eager">
            <div class="logo-title">Symbion</div>
            <div class="logo-subtitle">Neural Cortex System</div>
          </div>

          ${this.phase === 'login' ? html`
            <!-- Error Message -->
            ${this.error ? html`
              <div class="error-message">${this.error}</div>
            ` : ''}

            <!-- Login Form -->
            ${this.loginStep === 'credentials' ? html`
              <form @submit="${this.handleFormSubmit}" class="login-form">
                <div class="form-group">
                  <label class="form-label" for="username">Identifiant</label>
                  <input
                    id="username"
                    name="username"
                    type="text"
                    class="form-input"
                    autocomplete="username"
                    placeholder="Votre nom d'utilisateur"
                    .value="${this.username}"
                    autofocus
                    required>
                </div>

                <div class="form-group">
                  <label class="form-label" for="password">Mot de passe</label>
                  <input
                    id="password"
                    name="password"
                    type="password"
                    class="form-input"
                    autocomplete="current-password"
                    placeholder="••••••••"
                    .value="${this.password}"
                    required>
                </div>

                <button type="submit" class="submit-btn" ?disabled="${this.phase === 'authenticating'}">
                  ${this.phase === 'authenticating' ? 'Connexion...' : 'Se connecter'}
                </button>
              </form>

              <!-- Biometric Auth -->
              ${this.biometricAvailable ? html`
                <div class="divider">
                  <span class="divider-text">ou</span>
                </div>
                <button
                  class="biometric-btn"
                  @click="${this.authenticateWithBiometric}"
                  ?disabled="${this.authenticatingBiometric}">
                  🔐 Connexion biométrique
                  ${this.authenticatingBiometric ? html`<div class="spinner bt-spinner-sm"></div>` : ''}
                </button>
              ` : ''}
            ` : ''}

            <!-- TOTP Step -->
            ${this.loginStep === 'totp' ? html`
              <form @submit="${this.handleFormSubmit}" class="login-form">
                <div class="form-group">
                  <label class="form-label" for="totp">Code TOTP</label>
                  <input
                    id="totp"
                    name="totp"
                    type="tel"
                    class="form-input"
                    inputmode="numeric"
                    autocomplete="one-time-code"
                    pattern="[0-9]{6,8}"
                    maxlength="8"
                    placeholder="123456"
                    .value="${this.totpCode}"
                    autofocus
                    required>

                  <div class="checkbox-group">
                    <input
                      type="checkbox"
                      id="remember"
                      name="rememberDevice"
                      class="checkbox-input"
                      .checked="${this.rememberDevice}"
                      @change="${(e) => this.rememberDevice = e.target.checked}">
                    <label for="remember" class="checkbox-label">
                      Mémoriser cet appareil (30 jours)
                    </label>
                  </div>
                </div>

                <button type="submit" class="submit-btn" ?disabled="${this.phase === 'authenticating'}">
                  ${this.phase === 'authenticating' ? 'Validation...' : 'Valider'}
                </button>
              </form>
            ` : ''}
          ` : ''}

          <!-- Authenticating State -->
          ${this.phase === 'authenticating' ? html`
            <div class="loading-state">
              <div class="spinner"></div>
              <div class="loading-text">Authentification en cours...</div>
            </div>
          ` : ''}

          <!-- Success State -->
          ${this.phase === 'success' ? html`
            <div class="success-state">
              <div class="success-icon">✓</div>
              <div class="success-text">Connexion réussie !</div>
              <div class="success-subtext">Chargement du dashboard...</div>
            </div>
          ` : ''}
        </div>
      </div>
    `
  }
}

customElements.define('boot-terminal', BootTerminal)

export default BootTerminal
