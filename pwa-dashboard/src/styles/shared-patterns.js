/**
 * Shared CSS Patterns — Lit CSS Module
 *
 * Centralise les patterns CSS les plus dupliqués.
 * Usage : import { overlayStyles, closeButtonStyles } from '../styles/shared-patterns.js'
 *         static styles = [overlayStyles, closeButtonStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Overlay plein écran avec backdrop glass morphism.
 * Appliqué sur :host pour les composants fullscreen (modals, pages overlay).
 */
export const overlayStyles = css`
  :host {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: radial-gradient(ellipse at center,
      var(--ctx-bg-subtle, rgba(0, 0, 0, 0.85)) 0%,
      rgba(0, 0, 0, 0.9) 100%);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    z-index: 9999;
    overflow-y: auto;
    animation: fadeIn 0.3s ease-out;
  }
`

/**
 * Bouton fermer (danger style, position absolute top-right).
 * Requiert un élément avec class="close-btn".
 */
export const closeButtonStyles = css`
  .close-btn {
    position: absolute;
    top: 0;
    right: 0;
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-danger-border, rgba(255, 107, 107, 0.3));
    border-radius: 8px;
    background: linear-gradient(135deg,
      rgba(255, 107, 107, 0.15) 0%,
      rgba(255, 107, 107, 0.08) 100%);
    color: var(--color-danger-text, #ff6b6b);
    cursor: pointer;
    transition: all 0.3s ease;
    font-size: 1rem;
  }

  .close-btn:hover {
    background: linear-gradient(135deg,
      rgba(255, 107, 107, 0.25) 0%,
      rgba(255, 107, 107, 0.15) 100%);
    border-color: rgba(255, 107, 107, 0.5);
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(255, 107, 107, 0.3);
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
 * Scrollbar custom context-aware (webkit).
 * Appliquer sur le conteneur scrollable via class ou directement.
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
 * Card/section avec bordure context-aware.
 * Classes: .section-card
 */
export const sectionCardStyles = css`
  .section-card {
    background: linear-gradient(135deg,
      var(--surface-glass, rgba(255, 255, 255, 0.05)) 0%,
      rgba(255, 255, 255, 0.02) 100%);
    border: 1px solid var(--border-default, rgba(255, 255, 255, 0.08));
    border-radius: 12px;
    padding: 1.25rem;
    transition: all 0.3s ease;
  }

  .section-card:hover {
    border-color: var(--border-hover, rgba(255, 255, 255, 0.15));
    transform: translateY(-2px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }
`

/**
 * Inputs de formulaire avec focus context-primary.
 * Classes: .form-input, .form-textarea, .form-select
 */
export const formInputStyles = css`
  .form-input,
  .form-textarea,
  .form-select {
    width: 100%;
    padding: 0.75rem 1rem;
    background: var(--surface-glass, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--border-default, rgba(255, 255, 255, 0.08));
    border-radius: 8px;
    color: #f0f0f0;
    font-size: 0.9rem;
    font-family: inherit;
    transition: all 0.3s ease;
    box-sizing: border-box;
  }

  .form-input:focus,
  .form-textarea:focus,
  .form-select:focus {
    outline: none;
    border-color: var(--context-primary, #00d4aa);
    box-shadow: 0 0 0 3px var(--ctx-border, rgba(0, 212, 170, 0.15));
  }

  .form-input::placeholder,
  .form-textarea::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }
`
