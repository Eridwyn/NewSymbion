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
      /* Bio-organic gradient background avec contextual tint */
      background: radial-gradient(ellipse at top center,
        color-mix(in srgb, var(--context-primary, #00d4aa) 4%, rgba(10, 10, 11, 0.98)) 0%,
        rgba(10, 10, 11, 1) 100%),
        linear-gradient(135deg,
          rgba(0, 212, 170, 0.02) 0%,
          transparent 100%);
      z-index: 100000;
      overflow-y: auto;
      animation: backgroundBreathing 10s ease-in-out infinite;
    }

    @keyframes backgroundBreathing {
      0%, 100% {
        background: radial-gradient(ellipse at top center,
          color-mix(in srgb, var(--context-primary, #00d4aa) 4%, rgba(10, 10, 11, 0.98)) 0%,
          rgba(10, 10, 11, 1) 100%);
      }
      50% {
        background: radial-gradient(ellipse at top center,
          color-mix(in srgb, var(--context-primary, #00d4aa) 6%, rgba(10, 10, 11, 0.98)) 0%,
          rgba(10, 10, 11, 1) 100%);
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

    @keyframes float {
      0%, 100% {
        transform: translate(0, 0) scale(1);
        opacity: 0.15;
      }
      33% {
        transform: translate(30px, -30px) scale(1.1);
        opacity: 0.2;
      }
      66% {
        transform: translate(-20px, 20px) scale(0.9);
        opacity: 0.12;
      }
    }

    .login-card {
      width: 100%;
      max-width: 420px;
      background: linear-gradient(135deg,
        rgba(30, 30, 30, 0.95) 0%,
        rgba(20, 20, 20, 0.98) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      border-radius: 24px;
      padding: 3rem 2.5rem;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.7),
                  0 0 60px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
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
                    0 0 60px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                    inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      }
      50% {
        box-shadow: 0 28px 56px rgba(0, 0, 0, 0.75),
                    0 0 80px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                    inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
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
      filter: drop-shadow(0 0 20px rgba(0, 212, 170, 0.6))
              drop-shadow(0 0 40px rgba(0, 212, 170, 0.4))
              brightness(1.05);
      animation: logoPulse 4s ease-in-out infinite,
                 logoFloat 6s ease-in-out infinite;
    }

    @keyframes logoPulse {
      0%, 100% {
        filter: drop-shadow(0 0 20px rgba(0, 212, 170, 0.6))
                drop-shadow(0 0 40px rgba(0, 212, 170, 0.4))
                brightness(1.05);
      }
      50% {
        filter: drop-shadow(0 0 30px rgba(0, 212, 170, 0.8))
                drop-shadow(0 0 60px rgba(0, 212, 170, 0.6))
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
      font-size: 0.875rem;
      color: #9ca3af;
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
      font-size: 0.875rem;
      font-weight: 500;
      color: #e5e7eb;
      margin-bottom: 0.5rem;
      letter-spacing: 0.01em;
    }

    .form-input {
      width: 100%;
      max-width: 100%;
      min-width: 0;
      box-sizing: border-box;
      padding: 0.875rem 1.125rem;
      background: linear-gradient(135deg,
        rgba(0, 0, 0, 0.5) 0%,
        rgba(0, 0, 0, 0.3) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 12px;
      color: #f3f4f6;
      font-size: 0.9375rem;
      font-family: inherit;
      transition: all var(--duration-base, 0.3s) ease;
      outline: none;
    }

    .form-input::placeholder {
      color: rgba(156, 163, 175, 0.5);
    }

    .form-input:focus {
      background: linear-gradient(135deg,
        rgba(0, 0, 0, 0.6) 0%,
        rgba(0, 0, 0, 0.4) 100%);
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 4px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      animation: inputGlow 0.6s ease-out;
    }

    @keyframes inputGlow {
      0% {
        box-shadow: 0 0 0 0 color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      }
      50% {
        box-shadow: 0 0 0 8px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      }
      100% {
        box-shadow: 0 0 0 4px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      }
    }

    .submit-btn {
      width: 100%;
      padding: 1rem 1.5rem;
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 80%, #0066cc) 100%);
      border: none;
      border-radius: 12px;
      color: #0a0a0b;
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-base, 0.3s) ease;
      box-shadow: 0 4px 12px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent),
                  0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
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
      box-shadow: 0 8px 20px color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent),
                  0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      animation: btnPulse 1.5s ease-in-out infinite;
    }

    @keyframes btnPulse {
      0%, 100% {
        box-shadow: 0 8px 20px color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent),
                    0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      }
      50% {
        box-shadow: 0 8px 24px color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent),
                    0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 35%, transparent);
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
        color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent),
        transparent);
    }

    .divider-text {
      font-size: 0.8125rem;
      color: #9ca3af;
      font-weight: 500;
    }

    .biometric-btn {
      width: 100%;
      padding: 1rem 1.5rem;
      background: linear-gradient(135deg,
        rgba(0, 212, 170, 0.12) 0%,
        rgba(0, 212, 170, 0.08) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: 12px;
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
        rgba(0, 212, 170, 0.18) 0%,
        rgba(0, 212, 170, 0.12) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      transform: translateY(-2px) scale(1.01);
      box-shadow: 0 6px 16px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                  0 0 24px color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      animation: biometricGlow 2s ease-in-out infinite;
    }

    @keyframes biometricGlow {
      0%, 100% {
        box-shadow: 0 6px 16px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                    0 0 24px color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      }
      50% {
        box-shadow: 0 6px 20px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent),
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
        rgba(255, 107, 107, 0.15) 0%,
        rgba(255, 107, 107, 0.08) 100%);
      border: 1px solid rgba(255, 107, 107, 0.3);
      border-radius: 12px;
      color: #ff6b6b;
      font-size: 0.875rem;
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
      border: 4px solid rgba(255, 255, 255, 0.1);
      border-top-color: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin: 0 auto 1.5rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .loading-text {
      color: #9ca3af;
      font-size: 0.9375rem;
      animation: pulse 2s ease-in-out infinite;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
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
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .success-subtext {
      color: #9ca3af;
      font-size: 0.875rem;
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
      font-size: 0.875rem;
      color: #9ca3af;
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
        border-radius: 22px;
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

    /* Mobile Large (480px to 640px) */
    @media (max-width: 640px) {
      .login-container {
        padding: 1.5rem 1rem;
        justify-content: flex-start;
        padding-top: 3rem;
      }

      .login-card {
        max-width: 95%;
        padding: 2rem 1.5rem;
        border-radius: 20px;
        box-shadow: 0 16px 32px rgba(0, 0, 0, 0.6),
                    0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
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
        border-radius: 10px;
      }

      .submit-btn,
      .biometric-btn {
        padding: 0.875rem 1rem;
        font-size: 0.9rem;
        border-radius: 10px;
      }

      .or-divider {
        margin: 1.25rem 0;
        font-size: 0.75rem;
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
        border-radius: 18px;
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
        font-size: 0.75rem;
        margin-bottom: 1.25rem;
      }

      .form-group {
        margin-bottom: 0.875rem;
      }

      .form-label {
        font-size: 0.75rem;
        margin-bottom: 0.35rem;
      }

      .form-input {
        padding: 0.8rem 0.875rem;
        font-size: 0.875rem;
        border-radius: 10px;
      }

      .form-input::placeholder {
        font-size: 0.85rem;
      }

      .submit-btn,
      .biometric-btn {
        padding: 0.8rem 0.875rem;
        font-size: 0.875rem;
        border-radius: 10px;
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
        font-size: 1.5rem;
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
        border-radius: 16px;
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
        border-radius: 8px;
      }

      .or-divider {
        margin: 0.875rem 0;
        font-size: 0.65rem;
      }

      .checkbox-label {
        font-size: 0.75rem;
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
  `

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
                  ${this.authenticatingBiometric ? html`<div class="spinner" style="width: 16px; height: 16px; border-width: 2px; margin: 0;"></div>` : ''}
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
