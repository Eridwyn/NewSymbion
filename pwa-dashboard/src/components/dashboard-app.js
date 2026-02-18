/**
 * Composant principal du dashboard Symbion
 * 
 * Interface adaptative qui charge dynamiquement les widgets
 * basés sur les manifestes des plugins actifs
 */

import { LitElement, html, css } from 'lit'
import authService from '../services/auth-service.js'
import csrfService from '../services/csrf-service.js'
import '../services/api-service.js'
import '../services/mqtt-service.js'
import '../services/agents-service.js'
import '../services/context-service.js'
import '../widgets/system-health-widget.js'
import '../widgets/freebox-widget.js'
import '../widgets/ssl-widget.js'
// import '../widgets/hosts-widget.js'  // DEPRECATED: remplacé par agents-network-widget
import '../widgets/plugins-widget.js'
import '../widgets/notes-widget.js'
import '../widgets/agents-network-widget.js'
import '../widgets/agent-control-widget.js'
import '../widgets/environment-widget.js'
import '../widgets/context-engine-widget.js'
import './user-settings-page.js'
import './notes-page.js'
import './context-engine-page.js'
import './ssl-config-page.js'
import './toast-notifications.js'
import './notification-center.js'
import automationsService from '../services/automations-service.js'

class DashboardApp extends LitElement {
  static styles = css`
    :host {
      display: block;
      min-height: 100vh;
      background: linear-gradient(180deg, #0e0e13 0%, #0a0a0f 100%);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: var(--font-sans);
      position: relative;
    }

    /* Bordures lumineuses thématiques sur les côtés */
    :host::before,
    :host::after {
      content: '';
      position: fixed;
      top: 0;
      bottom: 0;
      width: 3px;
      pointer-events: none;
      z-index: 0;
      background: linear-gradient(
        180deg,
        transparent 0%,
        var(--context-primary, #00d4aa) 20%,
        var(--context-primary, #00d4aa) 80%,
        transparent 100%
      );
      opacity: 0.25;
      filter: blur(2px);
      animation: glowPulse 6s ease-in-out infinite;
    }

    :host::before {
      left: 0;
      box-shadow: 0 0 20px 8px var(--context-primary, #00d4aa);
    }

    :host::after {
      right: 0;
      box-shadow: 0 0 20px 8px var(--context-primary, #00d4aa);
      animation-delay: 3s;
    }

    @keyframes glowPulse {
      0%, 100% { opacity: 0.2; }
      50% { opacity: 0.35; }
    }

    /* Header bioluminescent avec glassmorphism CONTEXTUEL */
    .header {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent) 0%,
        rgba(19, 20, 26, 0.85) 50%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 6%, transparent) 100%);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      padding: var(--space-6) var(--space-8);
      position: -webkit-sticky;
      position: sticky;
      top: 0;
      z-index: var(--z-sticky);
      box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: var(--space-4);
      transition: all var(--duration-base) var(--ease-out);
    }

    .header-left {
      flex: 1;
      min-width: 0;
    }

    /* Titre avec gradient bioluminescent CONTEXTUEL */
    .header h1 {
      font-size: var(--text-3xl);
      font-weight: var(--font-bold);
      margin: 0;
      background: linear-gradient(135deg,
        var(--context-primary, #00d4aa) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 80%, white) 50%,
        var(--context-primary, #00d4aa) 100%);
      background-size: 200% 200%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      animation: bio-gradient-shift 6s ease infinite;
      letter-spacing: var(--tracking-tight);
      transition: all var(--duration-base) var(--ease-out);
      display: flex;
      align-items: center;
      gap: var(--space-3);
      filter: drop-shadow(0 0 20px var(--context-primary, rgba(0, 212, 170, 0.3)));
    }

    @keyframes bio-gradient-shift {
      0%, 100% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
    }

    /* Logo bioluminescent avec colorisation DYNAMIQUE basée sur --context-primary */
    .header-logo {
      width: 2rem;
      height: 2rem;
      object-fit: contain;
      transition: filter var(--duration-base) var(--ease-out);
      animation: logo-bio-pulse 4s ease-in-out infinite;
      /* Colorisation dynamique depuis context-service (--context-logo-*) */
      filter: invert(1) sepia(1)
              saturate(var(--context-logo-saturation, 3))
              hue-rotate(var(--context-logo-hue, 100deg))
              brightness(var(--context-logo-brightness, 1.1))
              drop-shadow(0 0 12px color-mix(in srgb, var(--context-primary, #00d4aa) 60%, transparent))
              drop-shadow(0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent));
    }

    @keyframes logo-bio-pulse {
      0%, 100% {
        opacity: 1;
      }
      50% {
        opacity: 0.85;
      }
    }

    .header-logo:hover {
      animation: none;
      opacity: 1 !important;
      /* Hover intensifie le glow */
      filter: invert(1) sepia(1)
              saturate(calc(var(--context-logo-saturation, 3) + 1))
              hue-rotate(var(--context-logo-hue, 100deg))
              brightness(calc(var(--context-logo-brightness, 1.1) + 0.15))
              drop-shadow(0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 90%, transparent))
              drop-shadow(0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 60%, transparent)) !important;
    }

    /* Status Bar - Modern Pills */
    .status-bar {
      display: flex;
      gap: var(--space-3);
      align-items: center;
      margin-top: var(--space-3);
      font-size: var(--text-sm);
      font-weight: var(--font-medium);
    }

    .status-indicator {
      display: flex;
      align-items: center;
      gap: var(--space-2);
      padding: var(--space-2) var(--space-3);
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
      border-radius: var(--radius-md);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      transition: all var(--duration-base) var(--ease-out);
      font-size: 0.7rem;
      letter-spacing: 0.03em;
      font-weight: var(--font-medium);
      color: var(--context-primary, #00d4aa);
    }

    .status-indicator:hover {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      transform: translateY(-1px);
    }

    /* Status Dots - Bioluminescent pulse */
    .status-dot {
      width: 10px;
      height: 10px;
      border-radius: var(--radius-full);
      transition: all var(--duration-base) var(--ease-out);
    }

    .status-dot.online,
    .status-dot.connected {
      background: var(--context-primary, #00d4aa);
      box-shadow: 0 0 15px color-mix(in srgb, var(--context-primary, #00d4aa) 70%, transparent),
                  0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent),
                  inset 0 0 10px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, white);
      animation: bio-pulse-glow 2.5s ease-in-out infinite;
    }

    .status-dot.offline {
      background: #4b5563;
      box-shadow: 0 0 0 2px rgba(107, 114, 128, 0.3);
      opacity: 0.6;
    }

    .status-dot.polling {
      background: #3b82f6;
      box-shadow: 0 0 15px rgba(59, 130, 246, 0.6),
                  0 0 25px rgba(59, 130, 246, 0.3);
      animation: bio-pulse-glow 2s ease-in-out infinite;
    }

    .status-dot.loading {
      background: #ffd93d;
      box-shadow: 0 0 12px rgba(255, 217, 61, 0.6);
      animation: bio-pulse-loading 1.2s ease-in-out infinite;
    }

    @keyframes bio-pulse-glow {
      0%, 100% {
        transform: scale(1);
        opacity: 1;
      }
      50% {
        transform: scale(1.15);
        opacity: 0.8;
        box-shadow: 0 0 20px currentColor,
                    0 0 40px currentColor;
      }
    }

    @keyframes bio-pulse-loading {
      0%, 100% {
        opacity: 1;
        transform: scale(1);
      }
      50% {
        opacity: 0.5;
        transform: scale(0.9);
      }
    }

    /* Clock Display - Ultra-discrète, cachée sur mobile */
    .system-clock {
      display: none; /* Cachée par défaut (mobile) */
      align-items: center;
      gap: 0.25rem;
      font-family: var(--font-mono);
      font-size: 0.7rem;
      font-weight: var(--font-normal);
      color: var(--color-dark-text-tertiary);
      letter-spacing: 0.03em;
      opacity: 0.4;
      transition: opacity var(--duration-base) var(--ease-out);
    }

    /* Visible seulement sur desktop */
    @media (min-width: 769px) {
      .system-clock {
        display: flex;
      }
    }

    .system-clock:hover {
      opacity: 0.7;
    }

    .system-clock .icon {
      font-size: 0.9em;
      opacity: 0.6;
    }

    /* User Menu */
    .user-menu {
      position: relative;
    }

    .user-button {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
      padding: var(--space-3) var(--space-4);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-semibold);
      cursor: pointer;
      display: flex;
      align-items: center;
      gap: var(--space-2);
      transition: all var(--duration-base) var(--ease-out);
      box-shadow: 0 0 20px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
    }

    .user-button:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 60%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 6px 20px color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent),
                  0 0 30px color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
    }

    /* User Dropdown - Bio-Organic Menu */
    .user-dropdown {
      position: absolute;
      top: calc(100% + var(--space-3));
      right: 0;
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 4%, rgba(19, 20, 26, 0.98)) 0%,
        rgba(15, 15, 15, 0.96) 100%);
      backdrop-filter: blur(var(--blur-xl));
      -webkit-backdrop-filter: blur(var(--blur-xl));
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: var(--radius-lg);
      padding: var(--space-5);
      min-width: 260px;
      box-shadow: 0 16px 48px rgba(0, 0, 0, 0.6),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 40px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent);
      z-index: 1000;
      animation: dropdownSlide var(--duration-slow) var(--ease-out);
    }

    @keyframes dropdownSlide {
      from {
        opacity: 0;
        transform: translateY(-12px) scale(0.95);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
    }

    .user-info {
      padding-bottom: var(--space-4);
      border-bottom: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      margin-bottom: var(--space-4);
      position: relative;
    }

    .user-info::after {
      content: '';
      position: absolute;
      bottom: -1px;
      left: 0;
      width: 40%;
      height: 1px;
      background: linear-gradient(90deg,
        var(--context-primary, #00d4aa) 0%,
        transparent 100%);
      opacity: 0.6;
    }

    .user-name {
      color: var(--context-primary, #00d4aa);
      font-weight: var(--font-semibold);
      font-size: var(--text-base);
      margin-bottom: var(--space-2);
      text-shadow: 0 0 12px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
    }

    .user-role {
      color: var(--color-dark-text-secondary);
      font-size: var(--text-xs);
      text-transform: uppercase;
      letter-spacing: var(--tracking-wider);
      font-weight: var(--font-medium);
    }

    .user-session {
      color: var(--color-dark-text-tertiary);
      font-size: var(--text-xs);
      margin-top: var(--space-2);
      font-family: var(--font-mono);
      opacity: 0.8;
    }

    /* Bouton Paramètres - Style contextuel */
    .settings-button {
      width: 100%;
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      color: var(--context-primary, #00d4aa);
      padding: var(--space-3) var(--space-4);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-semibold);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      display: flex;
      align-items: center;
      justify-content: center;
      gap: var(--space-2);
      margin-bottom: var(--space-3);
    }

    .settings-button:hover {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                  0 0 24px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
    }

    /* Bouton Context Engine - Style distinctif */
    .context-engine-button {
      width: 100%;
      background: linear-gradient(135deg,
        rgba(147, 51, 234, 0.15) 0%,
        rgba(147, 51, 234, 0.08) 100%);
      border: 1px solid rgba(147, 51, 234, 0.35);
      color: #a855f7;
      padding: var(--space-3) var(--space-4);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-semibold);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      display: flex;
      align-items: center;
      justify-content: center;
      gap: var(--space-2);
      margin-bottom: var(--space-3);
    }

    .context-engine-button:hover {
      background: linear-gradient(135deg,
        rgba(147, 51, 234, 0.25) 0%,
        rgba(147, 51, 234, 0.15) 100%);
      border-color: rgba(147, 51, 234, 0.5);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px rgba(147, 51, 234, 0.2),
                  0 0 24px rgba(147, 51, 234, 0.15);
    }

    /* Bouton Déconnexion - Style danger mais élégant */
    .logout-button {
      width: 100%;
      background: linear-gradient(135deg,
        rgba(239, 68, 68, 0.15) 0%,
        rgba(239, 68, 68, 0.08) 100%);
      border: 1px solid rgba(239, 68, 68, 0.35);
      color: #ff6b6b;
      padding: var(--space-3) var(--space-4);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-semibold);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      display: flex;
      align-items: center;
      justify-content: center;
      gap: var(--space-2);
    }

    .logout-button:hover {
      background: linear-gradient(135deg,
        rgba(239, 68, 68, 0.25) 0%,
        rgba(239, 68, 68, 0.15) 100%);
      border-color: rgba(239, 68, 68, 0.55);
      transform: translateY(-2px);
      box-shadow: 0 6px 16px rgba(239, 68, 68, 0.25),
                  0 0 24px rgba(239, 68, 68, 0.2);
    }

    /* Main Content - Spacious Layout */
    .main-content {
      padding: var(--space-10) var(--space-8);
      max-width: 1600px;
      margin: 0 auto;
    }

    /* Widget Grid - Modern Cards */
    .widgets-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
      gap: var(--space-6);
      margin-bottom: var(--space-8);
    }

    /* Widget Container - Bio-Organic Card Design CONTEXTUEL */
    .widget-container {
      /* Gradient organique comme une membrane cellulaire */
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 3%, transparent) 0%,
        rgba(19, 20, 26, 0.95) 20%,
        rgba(28, 29, 36, 0.98) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 12%, transparent);
      border-radius: var(--radius-xl);
      padding: var(--space-8);
      backdrop-filter: blur(var(--blur-lg));
      transition: all var(--duration-slow) var(--ease-out);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent);
      position: relative;
      overflow: hidden;
    }

    /* Border bioluminescent qui pulse comme un influx nerveux */
    .widget-container::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 2px;
      background: linear-gradient(90deg,
        transparent 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 80%, transparent) 50%,
        transparent 100%);
      opacity: 0;
      animation: neural-pulse 4s ease-in-out infinite;
      transition: opacity var(--duration-base) var(--ease-out);
    }

    @keyframes neural-pulse {
      0%, 100% {
        opacity: 0;
        transform: translateX(-100%);
      }
      50% {
        opacity: 1;
        transform: translateX(0%);
      }
    }

    /* Hover - Activation organique CONTEXTUEL */
    .widget-container:hover {
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      transform: translateY(-4px);
      box-shadow: 0 16px 48px color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent),
                  0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent),
                  0 0 60px color-mix(in srgb, var(--context-primary, #00d4aa) 8%, transparent),
                  inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent) 0%,
        rgba(19, 20, 26, 0.95) 20%,
        rgba(28, 29, 36, 0.98) 100%);
    }

    .widget-container:hover::before {
      opacity: 1;
      animation: neural-pulse-active 2s ease-in-out infinite;
    }

    @keyframes neural-pulse-active {
      0%, 100% {
        opacity: 0.6;
        transform: translateX(0%);
      }
      50% {
        opacity: 1;
        transform: translateX(100%);
      }
    }

    .error-message {
      background: linear-gradient(135deg, rgba(255, 107, 107, 0.15) 0%, rgba(255, 107, 107, 0.05) 100%);
      border: 1px solid rgba(255, 107, 107, 0.4);
      border-radius: 12px;
      padding: 1.2rem;
      margin: 1rem 0;
      color: #ff6b6b;
      font-weight: 500;
      box-shadow: 0 4px 16px rgba(255, 107, 107, 0.1);
    }

    /* FAB Log Viewer */
    .logs-fab {
      position: fixed;
      bottom: 1.2rem;
      right: 1.2rem;
      width: 40px;
      height: 40px;
      border-radius: 50%;
      background: rgba(99, 102, 241, 0.15);
      border: 1px solid rgba(99, 102, 241, 0.25);
      color: #818cf8;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 80;
      transition: all 0.3s ease;
      opacity: 0.4;
    }

    .logs-fab:hover {
      opacity: 1;
      background: rgba(99, 102, 241, 0.25);
      border-color: rgba(99, 102, 241, 0.5);
      transform: scale(1.1);
    }

    @media (max-width: 768px) {
      .logs-fab {
        bottom: 5rem;
      }
    }

    /* Tabs mobile */
    .tabs-container {
      display: none;
    }

    .tabs {
      display: flex;
      gap: 0.5rem;
      border-bottom: 2px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
    }

    @media (max-width: 768px) {
      .tabs {
        position: fixed;
        bottom: 0;
        left: 0;
        right: 0;
        margin-bottom: 0;
        background: linear-gradient(to top, #0f0f0f 0%, rgba(15, 15, 15, 0.98) 80%, rgba(15, 15, 15, 0.95) 100%);
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
        z-index: 90;
        padding: 0.5rem 1rem;
        box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.5);
      }

      .tabs-container {
        padding-bottom: 70px; /* Espace pour les tabs fixes */
      }
    }

    .tab {
      padding: 0.75rem 1.25rem;
      background: transparent;
      border: none;
      color: #888;
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s ease;
      border-bottom: 2px solid transparent;
      white-space: nowrap;
      position: relative;
      bottom: -2px;
    }

    .tab:hover {
      color: var(--context-primary, #00d4aa);
    }

    .tab.active {
      color: var(--context-primary, #00d4aa);
      border-bottom-color: var(--context-primary, #00d4aa);
    }

    .tab-content {
      display: none;
    }

    .tab-content.active {
      display: grid;
      grid-template-columns: 1fr;
      gap: 1.2rem;
    }

    /* Mobile Responsive - Compact */
    @media (max-width: 768px) {
      .header {
        padding: var(--space-3) var(--space-3);
        gap: var(--space-2);
      }

      .header h1 {
        font-size: var(--text-base); /* Plus petit sur mobile */
      }

      .header-logo {
        width: 1.25rem; /* Logo plus petit */
        height: 1.25rem;
      }

      .status-bar {
        flex-wrap: nowrap;
        gap: var(--space-2);
        margin-top: var(--space-2);
        overflow-x: auto;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none; /* Firefox */
      }

      .status-bar::-webkit-scrollbar {
        display: none; /* Chrome/Safari */
      }

      .status-indicator {
        padding: 0.25rem 0.5rem;
        font-size: 0.6rem;
        white-space: nowrap;
        flex-shrink: 0;
        gap: 0.25rem;
        letter-spacing: 0;
      }

      .status-dot {
        width: 6px;
        height: 6px;
      }

      /* Hide uptime and clock on mobile */
      .uptime-indicator,
      .system-clock {
        display: none;
      }

      .user-button {
        padding: 0.35rem 0.6rem;
        font-size: 0.65rem;
        gap: 0.25rem;
      }

      .main-content {
        padding: var(--space-5) var(--space-3);
      }

      .widgets-grid {
        display: none; /* Hide grid on mobile */
        grid-template-columns: 1fr;
        gap: var(--space-4);
      }

      .tabs-container {
        display: block; /* Show tabs on mobile */
      }

      .widget-container {
        padding: var(--space-6);
        border-radius: var(--radius-lg);
      }
    }

    /* Tablet & Desktop */
    @media (min-width: 769px) {
      .widgets-grid {
        grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
      }
    }

    /* Large Desktop - 3 columns */
    @media (min-width: 1400px) {
      .widgets-grid {
        grid-template-columns: repeat(3, 1fr);
      }
    }
  `
  
  static properties = {
    connected: { type: Boolean },
    mqttStatus: { type: String },
    apiStatus: { type: String },
    systemHealth: { type: Object },
    plugins: { type: Array },
    agents: { type: Array },
    error: { type: String },
    showUserMenu: { type: Boolean },
    showSettingsPage: { type: Boolean },
    showNotesPage: { type: Boolean },
    showContextEnginePage: { type: Boolean },
    showSslConfigPage: { type: Boolean },
    currentUser: { type: Object },
    activeTab: { type: String },
    currentTime: { type: String },
    showLogsFab: { type: Boolean }
  }
  
  constructor() {
    super()
    this.connected = false
    this.mqttStatus = 'connecting'
    this.apiStatus = 'loading'
    this.systemHealth = null
    this.plugins = []
    this.agents = []
    this.error = null
    this.showUserMenu = false
    this.showSettingsPage = false
    this.showNotesPage = false
    this.showContextEnginePage = false
    this.showSslConfigPage = false
    this.showLogsFab = localStorage.getItem('symbion_show_logs') === 'true'
    this.currentUser = authService.getCurrentUser()
    // Restaurer le dernier onglet actif depuis sessionStorage (persiste aux reloads, reset à la fermeture du navigateur)
    this.activeTab = sessionStorage.getItem('dashboardTab') || 'controle'
    this.currentTime = this.formatTime(new Date())

    this.apiService = null
    this.mqttService = null
    this.agentsService = null
    this.timeInterval = null
    this._realtimeInterval = null  // [Audit] Store for cleanup

    // [P0-5] Store bound handlers for cleanup
    this._boundHandlers = {
      apiStatus: null,
      mqttStatus: null,
      systemHealth: null,
      contextChange: null
    }
  }

  formatTime(date) {
    // Détecter mobile pour afficher HH:MM ou HH:MM:SS
    const isMobile = window.innerWidth <= 768
    return date.toLocaleTimeString('fr-FR', {
      hour: '2-digit',
      minute: '2-digit',
      second: isMobile ? undefined : '2-digit',
      hour12: false
    })
  }

  updateTime() {
    this.currentTime = this.formatTime(new Date())
  }
  
  async connectedCallback() {
    super.connectedCallback()

    // Démarrer l'horloge
    this.timeInterval = setInterval(() => this.updateTime(), 1000)

    // Écouter les événements du notes-widget
    this.addEventListener('open-notes-page', this.handleOpenNotesPage.bind(this))
    this.addEventListener('create-note', this.handleCreateNote.bind(this))

    // Écouter les événements du context-engine-widget
    this.addEventListener('open-context-engine', this.handleOpenContextEngine.bind(this))

    // Écouter les événements du ssl-widget pour ouvrir la page de config
    this.addEventListener('open-ssl-config', this.handleOpenSslConfig.bind(this))

    // Écouter auth:expired pour rediriger vers login (session expirée)
    this._boundHandlers.authExpired = this.handleAuthExpired.bind(this)
    window.addEventListener('auth:expired', this._boundHandlers.authExpired)

    // Écouter le toggle logs depuis les paramètres
    this._boundHandlers.logsToggle = (e) => { this.showLogsFab = e.detail.enabled }
    window.addEventListener('symbion-logs-toggle', this._boundHandlers.logsToggle)

    // Log context changes (theme is applied via CSS variables by context-service)
    this._boundHandlers.contextChange = (e) => {
      const mode = e.detail?.context?.mode_slug || e.detail?.context?.mode || 'unknown'
      console.log(`[dashboard-app] Context changed: ${mode}`)
    }
    window.addEventListener('context-change', this._boundHandlers.contextChange)

    try {
      // Initialiser les services
      await this.initializeServices()

      // Charger les données initiales
      await this.loadInitialData()

      // Démarrer les mises à jour temps réel
      this.startRealtimeUpdates()

    } catch (error) {
      console.error('❌ Dashboard initialization failed:', error)
      this.error = `Erreur d'initialisation: ${error.message}`
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    // Nettoyer l'intervalle d'horloge
    if (this.timeInterval) {
      clearInterval(this.timeInterval)
      this.timeInterval = null
    }
    // [Audit] Nettoyer l'intervalle realtime updates
    if (this._realtimeInterval) {
      clearInterval(this._realtimeInterval)
      this._realtimeInterval = null
    }

    // [P0-5] Cleanup all stored event handlers to prevent memory leaks
    if (this._boundHandlers.authExpired) {
      window.removeEventListener('auth:expired', this._boundHandlers.authExpired)
    }
    if (this._boundHandlers.contextChange) {
      window.removeEventListener('context-change', this._boundHandlers.contextChange)
    }
    if (this.apiService && this._boundHandlers.apiStatus) {
      this.apiService.removeEventListener('status-change', this._boundHandlers.apiStatus)
    }
    if (this.mqttService) {
      if (this._boundHandlers.mqttStatus) {
        this.mqttService.removeEventListener('status-change', this._boundHandlers.mqttStatus)
      }
      if (this._boundHandlers.systemHealth) {
        this.mqttService.removeEventListener('system-health', this._boundHandlers.systemHealth)
      }
    }
  }
  
  async initializeServices() {
    console.log('🔧 Initializing services...')

    // Service API - réutiliser si existant (créé par main.js)
    this.apiService = document.querySelector('api-service')
    if (!this.apiService) {
      this.apiService = document.createElement('api-service')
      document.body.appendChild(this.apiService)
    }
    // [P0-5] Store bound handlers for cleanup
    this._boundHandlers.apiStatus = this.handleApiStatus.bind(this)
    this.apiService.addEventListener('status-change', this._boundHandlers.apiStatus)

    // Service MQTT - réutiliser si existant (créé par main.js)
    this.mqttService = document.querySelector('mqtt-service')
    if (!this.mqttService) {
      this.mqttService = document.createElement('mqtt-service')
      document.body.appendChild(this.mqttService)
    }
    // [P0-5] Store bound handlers for cleanup
    this._boundHandlers.mqttStatus = this.handleMqttStatus.bind(this)
    this._boundHandlers.systemHealth = this.handleSystemHealth.bind(this)
    this.mqttService.addEventListener('status-change', this._boundHandlers.mqttStatus)
    this.mqttService.addEventListener('system-health', this._boundHandlers.systemHealth)
    // Sync initial MQTT status (service may already be connected)
    if (this.mqttService.status) {
      this.mqttStatus = this.mqttService.status
    }

    // Service Agents - réutiliser si existant
    this.agentsService = document.querySelector('agents-service')
    if (!this.agentsService) {
      this.agentsService = document.createElement('agents-service')
      document.body.appendChild(this.agentsService)
    }

    // Service Context - réutiliser si existant
    this.contextService = document.querySelector('context-service')
    if (!this.contextService) {
      this.contextService = document.createElement('context-service')
      document.body.appendChild(this.contextService)
    }

    // Initialiser CSRF service avec authService
    csrfService.setAuthService(authService)
    console.log('🔐 CSRF service initialized with authService')

    // Attendre que apiService soit prêt (connectedCallback exécuté)
    await new Promise(resolve => setTimeout(resolve, 50))

    // Initialiser Automations service APRÈS que apiService soit dans le DOM
    automationsService.init(this.apiService, csrfService)
    console.log('🤖 Automations service initialized')
  }
  
  async loadInitialData() {
    console.log('📊 Loading initial data...')

    try {
      // Charger l'état du système
      const health = await this.apiService.getSystemHealth()
      this.systemHealth = { ...health } // Force new reference

      // Charger les plugins
      const response = await this.apiService.getPlugins()
      // API returns {"plugins": [...]} so extract the array
      this.plugins = Array.isArray(response?.plugins) ? [...response.plugins] : []

      // Charger les agents
      const agents = await this.apiService.request('/v1/agents')
      this.agents = Array.isArray(agents) ? [...agents] : [] // Force new array reference

      this.apiStatus = 'online'
      this.connected = true

      console.log('✅ Initial data loaded:', { plugins: this.plugins.length, agents: this.agents.length })

      this.requestUpdate() // Force Lit to re-render

    } catch (error) {
      console.error('❌ Failed to load initial data:', error)
      this.apiStatus = 'offline'
      this.error = `Impossible de charger les données: ${error.message}`
    }
  }
  
  startRealtimeUpdates() {
    console.log('⚡ Starting realtime updates...')

    // Fonction de mise à jour
    const updateData = async () => {
      if (this.apiStatus === 'online') {
        try {
          const health = await this.apiService.getSystemHealth()
          this.systemHealth = { ...health } // Force new reference for Lit reactivity

          // Note: MQTT status is managed by mqtt-service via 'status-change' event
          // Don't override it from API health to avoid stale/incorrect status

          const response = await this.apiService.getPlugins()
          // API returns {"plugins": [...]} so extract the array
          this.plugins = Array.isArray(response?.plugins) ? [...response.plugins] : []

          const agents = await this.apiService.request('/v1/agents')
          this.agents = Array.isArray(agents) ? [...agents] : [] // Force new array reference

          this.requestUpdate() // Force Lit to re-render
        } catch (error) {
          console.warn('⚠️ Periodic update failed:', error)
        }
      }
    }

    // Première mise à jour immédiate
    updateData()

    // Puis mise à jour périodique - [Audit] Store for cleanup
    this._realtimeInterval = setInterval(updateData, 10000)
  }
  
  handleApiStatus(event) {
    this.apiStatus = event.detail.status
    if (event.detail.status === 'offline') {
      this.connected = false
    }
    this.requestUpdate()
  }
  
  handleMqttStatus(event) {
    this.mqttStatus = event.detail.status
    this.requestUpdate()
  }
  
  handleSystemHealth(event) {
    this.systemHealth = event.detail.health
    this.requestUpdate()
  }
  
  render() {
    return html`
      <div class="header">
        <div class="header-left">
          <h1><img src="/icon-192-transparent-v2.png" alt="Symbion" class="header-logo"> Symbion Dashboard</h1>
          <div class="status-bar">
            <div class="status-indicator">
              <div class="status-dot ${this.apiStatus}"></div>
              <span>API: ${this.apiStatus}</span>
            </div>
            <div class="status-indicator">
              <div class="status-dot ${this.mqttStatus}"></div>
              <span>MQTT: ${this.mqttStatus}</span>
            </div>
            ${this.systemHealth ? html`
              <div class="status-indicator uptime-indicator">
                <span>Uptime: ${this.formatUptime(this.systemHealth.uptime_seconds)}</span>
              </div>
            ` : ''}
          </div>
        </div>

        <div class="system-clock">
          <span class="icon">🕐</span>
          <span>${this.currentTime}</span>
        </div>

        <!-- Notification Center -->
        <notification-center></notification-center>

        ${this.currentUser ? html`
          <div class="user-menu">
            <button class="user-button" @click="${this.toggleUserMenu}">
              <span>👤</span>
              <span>${this.currentUser.username}</span>
            </button>

            ${this.showUserMenu ? html`
              <div class="user-dropdown">
                <div class="user-info">
                  <div class="user-name">${this.currentUser.username}</div>
                  <div class="user-role">${this.currentUser.role}</div>
                  <div class="user-session">${this.getSessionDuration()}</div>
                </div>
                <button class="context-engine-button" @click="${this.handleOpenContextEngine}">
                  <span>🧠</span>
                  <span>Decision Engine</span>
                </button>
                <button class="settings-button" @click="${this.handleOpenSettings}">
                  <span>⚙️</span>
                  <span>Paramètres</span>
                </button>
                <button class="logout-button" @click="${this.handleLogout}">
                  <span>🚪</span>
                  <span>Déconnexion</span>
                </button>
              </div>
            ` : ''}
          </div>
        ` : ''}
      </div>
      
      <div class="main-content">
        ${this.error ? html`
          <div class="error-message">
            ❌ ${this.error}
          </div>
        ` : ''}

        <!-- Tabs mobile uniquement -->
        <div class="tabs-container">
          <div class="tabs">
            <button class="tab ${this.activeTab === 'controle' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('controle')}">
              🎛️ Contrôle
            </button>
            <button class="tab ${this.activeTab === 'systeme' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('systeme')}">
              ⚙️ Système
            </button>
            <button class="tab ${this.activeTab === 'donnees' ? 'active' : ''}"
                    @click="${() => this.setActiveTab('donnees')}">
              📝 Données
            </button>
          </div>

          <!-- Contenu tab Contrôle -->
          <div class="tab-content ${this.activeTab === 'controle' ? 'active' : ''}">
            <div class="widget-container">
              <context-engine-widget></context-engine-widget>
            </div>
            <div class="widget-container">
              <agents-network-widget
                .connected="${this.connected}"
                .agents="${this.agents}">
              </agents-network-widget>
            </div>
          </div>

          <!-- Contenu tab Système -->
          <div class="tab-content ${this.activeTab === 'systeme' ? 'active' : ''}">
            <div class="widget-container">
              <system-health-widget
                .health="${this.systemHealth}"
                .connected="${this.connected}">
              </system-health-widget>
            </div>
            <div class="widget-container">
              <plugins-widget
                .plugins="${this.plugins}"
                .apiService="${this.apiService}">
              </plugins-widget>
            </div>
          </div>

          <!-- Contenu tab Données -->
          <div class="tab-content ${this.activeTab === 'donnees' ? 'active' : ''}">
            <div class="widget-container">
              <environment-widget></environment-widget>
            </div>
            <div class="widget-container">
              <freebox-widget></freebox-widget>
            </div>
            <div class="widget-container">
              <ssl-widget></ssl-widget>
            </div>
            <div class="widget-container">
              <notes-widget
                .apiService="${this.apiService}"
                .connected="${this.connected}">
              </notes-widget>
            </div>
          </div>
        </div>

        <!-- Grille desktop complète -->
        <div class="widgets-grid">
          <!-- Widget Context Engine (Mode + Automations résumé) -->
          <div class="widget-container">
            <context-engine-widget></context-engine-widget>
          </div>

          <!-- Widget environnement (F1) -->
          <div class="widget-container">
            <environment-widget></environment-widget>
          </div>

          <!-- Widget Freebox (presence + connexion) -->
          <div class="widget-container">
            <freebox-widget></freebox-widget>
          </div>

          <!-- Widget SSL Monitor -->
          <div class="widget-container">
            <ssl-widget></ssl-widget>
          </div>

          <!-- Widget santé système -->
          <div class="widget-container">
            <system-health-widget
              .health="${this.systemHealth}"
              .connected="${this.connected}">
            </system-health-widget>
          </div>

          <!-- Widget plugins -->
          <div class="widget-container">
            <plugins-widget
              .plugins="${this.plugins}"
              .apiService="${this.apiService}">
            </plugins-widget>
          </div>

          <!-- Widget notes -->
          <div class="widget-container">
            <notes-widget
              .apiService="${this.apiService}"
              .connected="${this.connected}">
            </notes-widget>
          </div>

          <!-- Widget agents network -->
          <div class="widget-container">
            <agents-network-widget
              .connected="${this.connected}"
              .agents="${this.agents}">
            </agents-network-widget>
          </div>
        </div>
        
        <!-- Modal de contrôle agent détaillé -->
        <agent-control-widget></agent-control-widget>

        <!-- Page Paramètres Utilisateur -->
        ${this.showSettingsPage ? html`
          <user-settings-page @close="${this.handleCloseSettings}"></user-settings-page>
        ` : ''}

        <!-- Page Gestion Notes -->
        ${this.showNotesPage ? html`
          <notes-page @close="${this.handleCloseNotesPage}"></notes-page>
        ` : ''}

        <!-- Page Context Engine (Mode + Automations + Validations + Stats + Config) -->
        ${this.showContextEnginePage ? html`
          <context-engine-page @close="${this.handleCloseContextEngine}"></context-engine-page>
        ` : ''}

        ${this.showSslConfigPage ? html`
          <ssl-config-page @close="${this.handleCloseSslConfig}"></ssl-config-page>
        ` : ''}
      </div>

      <!-- FAB Logs (discret, bas-droite) -->
      ${this.showLogsFab ? html`
        <button class="logs-fab" @click="${this._openLogViewer}" title="Ouvrir Log Viewer">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 17 10 11 4 5"></polyline>
            <line x1="12" y1="19" x2="20" y2="19"></line>
          </svg>
        </button>
      ` : ''}

      <!-- Toast Notifications (position fixe) -->
      <toast-notifications></toast-notifications>
    `
  }

  _openLogViewer() {
    window.open('/logs.html', '_blank')
  }
  
  setActiveTab(tab) {
    this.activeTab = tab
    sessionStorage.setItem('dashboardTab', tab)
  }

  toggleUserMenu() {
    this.showUserMenu = !this.showUserMenu
  }

  handleOpenSettings() {
    this.showSettingsPage = true
    this.showUserMenu = false // Fermer le menu dropdown
  }

  handleCloseSettings() {
    this.showSettingsPage = false
  }

  handleOpenNotesPage(event) {
    console.log('[dashboard] Opening notes page', event)
    this.showNotesPage = true
  }

  handleCloseNotesPage() {
    this.showNotesPage = false
  }

  handleOpenContextEngine() {
    console.log('[dashboard] Opening Context Engine page')
    this.showContextEnginePage = true
    this.showUserMenu = false // Fermer le menu dropdown
  }

  handleCloseContextEngine() {
    this.showContextEnginePage = false
  }

  handleOpenSslConfig() {
    console.log('[dashboard] Opening SSL Config page')
    this.showSslConfigPage = true
  }

  handleCloseSslConfig() {
    this.showSslConfigPage = false
  }

  handleAuthExpired(event) {
    console.warn('[dashboard] Session expirée - redirection vers login', event.detail)
    // Clear auth state
    authService.clearStorage()
    // Redirect to login
    this.isAuthenticated = false
    this.currentUser = null
    this.showLoginPage = true
  }

  handleCreateNote(event) {
    console.log('[dashboard] Opening notes page in create mode', event)
    // Ouvrir la page notes (elle détectera automatiquement qu'on veut créer)
    this.showNotesPage = true

    // Déclencher l'ouverture du formulaire de création après un court délai
    setTimeout(() => {
      const notesPage = this.shadowRoot.querySelector('notes-page')
      if (notesPage && notesPage.openCreateModal) {
        notesPage.openCreateModal()
      }
    }, 100)
  }

  async handleLogout() {
    const confirmed = confirm('Êtes-vous sûr de vouloir vous déconnecter ?')

    if (confirmed) {
      console.log('[dashboard] Logging out user')
      await authService.logout()

      // Rediriger vers boot terminal
      window.location.reload()
    }
  }

  getSessionDuration() {
    if (!this.currentUser || !this.currentUser.expires_at) {
      return 'N/A'
    }

    const now = Math.floor(Date.now() / 1000)
    const remaining = this.currentUser.expires_at - now

    if (remaining <= 0) {
      return 'Expirée'
    }

    const hours = Math.floor(remaining / 3600)
    const minutes = Math.floor((remaining % 3600) / 60)

    if (hours > 0) {
      return `${hours}h ${minutes}m restantes`
    }
    return `${minutes}m restantes`
  }

  formatUptime(seconds) {
    if (!seconds) return 'N/A'

    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)

    if (days > 0) {
      return `${days}j ${hours}h ${minutes}m`
    } else if (hours > 0) {
      return `${hours}h ${minutes}m`
    } else {
      return `${minutes}m`
    }
  }
}

customElements.define('dashboard-app', DashboardApp)