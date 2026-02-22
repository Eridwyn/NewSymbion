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
    background: rgba(0, 0, 0, 0.88);
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
    width: 36px;
    height: 36px;
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
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: var(--ctx-border-strong, rgba(0, 212, 170, 0.5));
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
    border-radius: 16px;
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
      rgba(255, 255, 255, 0.02) 100%);
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
