/**
 * Shared CSS Patterns — Lit CSS Module
 *
 * Centralise les patterns CSS structurels les plus dupliques.
 * Usage : import { overlayStyles, closeButtonStyles } from '../styles/shared-patterns.js'
 *         static styles = [overlayStyles, closeButtonStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Overlay plein ecran avec backdrop glassmorphism.
 * Applique sur :host pour les composants fullscreen (modals, pages overlay).
 */
export const overlayStyles = css`
  :host {
    display: block;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--app-overlay-dim, rgba(0, 0, 0, 0.88));
    backdrop-filter: blur(var(--blur-xl));
    -webkit-backdrop-filter: blur(var(--blur-xl));
    z-index: 9999;
    overflow-y: auto;
    animation: fadeIn var(--duration-slow, 0.3s) var(--ease-out, ease-out);
  }
`

/**
 * Bouton fermer circulaire 36px avec hover rouge.
 * Classe : .close-button
 */
export const closeButtonStyles = css`
  .close-button {
    background: var(--surface-glass-hover, rgba(255, 255, 255, 0.08));
    border: none;
    color: var(--color-dark-text-secondary, #adb5bd);
    width: 44px;
    height: 44px;
    border-radius: 50%;
    cursor: pointer;
    font-size: 1.2rem;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .close-button:hover {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }
`

/**
 * Scrollbar custom context-aware (webkit).
 */
export const scrollbarStyles = css`
  ::-webkit-scrollbar {
    width: 6px;
  }

  ::-webkit-scrollbar-track {
    background: transparent;
  }

  ::-webkit-scrollbar-thumb {
    background: var(--ctx-border-strong, rgba(0, 212, 170, 0.3));
    border-radius: var(--radius-sm, 0.375rem);
  }

  ::-webkit-scrollbar-thumb:hover {
    background: var(--ctx-border-strong, rgba(0, 212, 170, 0.5));
  }
`

/**
 * Focus visible styles pour accessibilite.
 * Applique un outline context-primary sur les elements focusables.
 */
export const focusVisibleStyles = css`
  :focus-visible {
    outline: 2px solid var(--context-primary, #00d4aa);
    outline-offset: 2px;
  }

  button:focus-visible {
    outline: 2px solid var(--context-primary, #00d4aa);
    outline-offset: 2px;
  }
`

/**
 * Badges de statut (success, warning, error, info).
 * Classes: .badge, .badge-success, .badge-warning, .badge-error, .badge-info
 */
export const badgeStyles = css`
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-radius: var(--radius-lg, 1rem);
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .badge-success {
    background: var(--color-success-bg, rgba(34, 197, 94, 0.15));
    color: var(--color-success-text, #22c55e);
    border: 1px solid var(--color-success-border, rgba(34, 197, 94, 0.3));
  }

  .badge-warning {
    background: var(--color-warning-bg, rgba(251, 191, 36, 0.15));
    color: var(--color-warning-text, #fbbf24);
    border: 1px solid var(--color-warning-border, rgba(251, 191, 36, 0.3));
  }

  .badge-error {
    background: var(--color-error-bg, rgba(239, 68, 68, 0.15));
    color: var(--color-error-text, #ef4444);
    border: 1px solid var(--color-error-border, rgba(239, 68, 68, 0.3));
  }

  .badge-info {
    background: var(--color-info-bg, rgba(59, 130, 246, 0.15));
    color: var(--color-info-text, #3b82f6);
    border: 1px solid var(--color-info-border, rgba(59, 130, 246, 0.3));
  }
`

/**
 * Card/section avec bordure et hover.
 * Classe: .section-card
 */
export const sectionCardStyles = css`
  .section-card {
    background: linear-gradient(135deg,
      var(--surface-glass, rgba(255, 255, 255, 0.05)) 0%,
      var(--app-card-glass, rgba(255, 255, 255, 0.02)) 100%);
    border: 1px solid var(--border-default, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-lg, 12px);
    padding: 1.25rem;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
  }

  .section-card:hover {
    border-color: var(--border-hover, rgba(255, 255, 255, 0.15));
    transform: translateY(-2px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }
`

/**
 * Status dot (cercle 10px) avec etats semantiques.
 * Classes: .status-dot, .status-dot.online, .offline, .warning, .error, .loading
 */
export const statusDotStyles = css`
  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full, 50%);
    flex-shrink: 0;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
  }

  .status-dot.online,
  .status-dot.connected,
  .status-dot.ok {
    background: var(--context-primary, #00d4aa);
    box-shadow: 0 0 8px color-mix(in srgb, var(--context-primary, #00d4aa) 70%, transparent);
  }

  .status-dot.offline {
    background: #4b5563;
    box-shadow: 0 0 0 2px rgba(107, 114, 128, 0.3);
    opacity: 0.6;
  }

  .status-dot.warning {
    background: #fbbf24;
    box-shadow: 0 0 8px rgba(251, 191, 36, 0.5);
  }

  .status-dot.critical,
  .status-dot.error {
    background: #ff6b6b;
    box-shadow: 0 0 8px rgba(255, 107, 107, 0.5);
  }

  .status-dot.loading,
  .status-dot.polling,
  .status-dot.connecting {
    background: #3b82f6;
    box-shadow: 0 0 8px rgba(59, 130, 246, 0.5);
  }
`

/**
 * Status badge pill avec etats semantiques.
 * Variantes gradient + glow coherentes pour tout le dashboard.
 *
 * Vert (actif)  : .healthy .online .ok .enabled .running .home .normal
 * Jaune (alerte) : .warning .starting .stopping .mold_risk
 * Rouge (erreur) : .error .offline .critical .disabled .stopped .failed .away
 * Bleu (info)    : .info .temp_low
 * Gris (neutre)  : .unknown .neutral
 */
export const statusBadgeStyles = css`
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0.3rem 0.8rem;
    border-radius: var(--radius-xl, 999px);
    font-size: 0.75em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
  }

  .status-badge.healthy,
  .status-badge.online,
  .status-badge.ok,
  .status-badge.enabled,
  .status-badge.running,
  .status-badge.home,
  .status-badge.normal {
    background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, var(--ctx-bg-strong) 100%);
    color: var(--context-primary, #00d4aa);
    border: 1px solid var(--ctx-border-strong);
    box-shadow: 0 2px 8px var(--ctx-border-medium);
  }

  .status-badge.warning,
  .status-badge.starting,
  .status-badge.stopping,
  .status-badge.mold_risk {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.25) 0%, rgba(251, 191, 36, 0.2) 100%);
    color: var(--color-warning-text-muted, #fbbf24);
    border: 1px solid rgba(251, 191, 36, 0.35);
    box-shadow: 0 2px 8px rgba(251, 191, 36, 0.25);
  }

  .status-badge.error,
  .status-badge.offline,
  .status-badge.critical,
  .status-badge.disabled,
  .status-badge.stopped,
  .status-badge.failed,
  .status-badge.away {
    background: linear-gradient(135deg, rgba(255, 107, 107, 0.25) 0%, rgba(239, 68, 68, 0.2) 100%);
    color: var(--color-danger-text-muted, #ff6b6b);
    border: 1px solid rgba(255, 107, 107, 0.35);
    box-shadow: 0 2px 8px rgba(255, 107, 107, 0.25);
  }

  .status-badge.info,
  .status-badge.temp_low {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.25) 0%, rgba(37, 99, 235, 0.2) 100%);
    color: var(--color-info-text-muted, #93c5fd);
    border: 1px solid rgba(59, 130, 246, 0.35);
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.25);
  }

  .status-badge.unknown,
  .status-badge.neutral {
    background: rgba(128, 128, 128, 0.15);
    color: var(--color-dark-text-tertiary, #6c757d);
    border: 1px solid rgba(128, 128, 128, 0.3);
  }

  /* Tap feedback for interactive elements */
  .status-badge:active,
  [role="button"]:active {
    transform: scale(0.96);
    transition: transform 50ms ease;
  }
`

/**
 * Badge compact pour titres de section (compteur, label).
 * Classe: .section-badge
 */
export const sectionBadgeStyles = css`
  .section-badge {
    display: inline-flex;
    align-items: center;
    font-size: var(--text-xs, 0.75rem);
    padding: 0.2rem 0.6rem;
    background: var(--ctx-border, rgba(0, 212, 170, 0.25));
    color: var(--context-primary, #00d4aa);
    border-radius: var(--radius-sm, 4px);
    font-weight: 500;
  }
`
