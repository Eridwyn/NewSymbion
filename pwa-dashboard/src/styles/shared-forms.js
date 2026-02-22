/**
 * Shared Form Styles — Lit CSS Module
 *
 * Styles communs pour formulaires (inputs, labels, boutons).
 * Usage : import { formInputStyles, formGroupStyles, btnStyles } from '../styles/shared-forms.js'
 *         static styles = [formInputStyles, btnStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Inputs de formulaire avec focus context-primary.
 * Classes: .form-input, .form-textarea, .form-select
 */
export const formInputStyles = css`
  .form-input,
  .form-textarea,
  .form-select {
    width: 100%;
    box-sizing: border-box;
    padding: 0.75rem 1rem;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--border-medium, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-md, 8px);
    color: #e0e0e0;
    font-size: 0.9rem;
    font-family: inherit;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
  }

  .form-input:focus,
  .form-textarea:focus,
  .form-select:focus {
    outline: none;
    border-color: var(--context-primary, #00d4aa);
    box-shadow: 0 0 0 3px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1));
  }

  .form-input::placeholder,
  .form-textarea::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .form-textarea {
    resize: vertical;
    min-height: 80px;
  }
`

/**
 * Groupe de formulaire avec label et aide.
 * Classes: .form-group, .form-label, .form-help
 */
export const formGroupStyles = css`
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-label {
    font-size: var(--text-sm, 0.875rem);
    font-weight: 500;
    color: var(--color-dark-text-secondary, #adb5bd);
  }

  .form-help {
    font-size: var(--text-xs, 0.75rem);
    color: var(--color-dark-text-tertiary, #6c757d);
  }
`

/**
 * Boutons avec variantes primary, secondary, danger.
 * Classes: .btn, .btn-primary, .btn-secondary, .btn-danger
 */
export const btnStyles = css`
  .btn {
    padding: 0.75rem 1.25rem;
    border-radius: var(--radius-md, 8px);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    transition: all 0.2s ease;
  }

  .btn-primary {
    background: linear-gradient(135deg,
      var(--context-primary, #00d4aa) 0%,
      color-mix(in srgb, var(--context-primary, #00d4aa) 80%, #000) 100%);
    border: none;
    color: #0a0a0f;
  }

  .btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px var(--ctx-bg-intense, rgba(0, 212, 170, 0.3));
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid var(--border-hover, rgba(255, 255, 255, 0.15));
    color: var(--color-dark-text-secondary, #888);
  }

  .btn-secondary:hover {
    background: var(--surface-glass, rgba(255, 255, 255, 0.05));
    color: #e0e0e0;
  }

  .btn-danger {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  .btn-danger:hover {
    background: rgba(239, 68, 68, 0.25);
    border-color: rgba(239, 68, 68, 0.5);
  }
`

/**
 * Bouton succes (vert).
 * Classe: .btn-success
 */
export const btnSuccessStyles = css`
  .btn-success {
    background: rgba(34, 197, 94, 0.15);
    border: 1px solid rgba(34, 197, 94, 0.4);
    color: #22c55e;
  }

  .btn-success:hover {
    background: rgba(34, 197, 94, 0.25);
    transform: translateY(-1px);
  }
`

/**
 * Modificateurs de taille pour boutons.
 * Classes: .btn-small, .btn-icon
 */
export const btnSizeStyles = css`
  .btn-small {
    padding: 0.35rem 0.7rem;
    font-size: 0.7rem;
  }

  .btn-icon {
    padding: 0.4rem;
    min-width: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
`
