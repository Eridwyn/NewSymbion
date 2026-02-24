/**
 * Shared Loading Styles — Lit CSS Module
 *
 * Spinner et états de chargement pour boutons async.
 * Usage : import { loadingButtonStyles } from '../styles/shared-loading.js'
 *         static styles = [loadingButtonStyles, css`...local...`]
 *
 * HTML : <button class="btn btn-primary ${this.isSaving ? 'is-loading' : ''}"
 *          ?disabled="${this.isSaving}" @click="${this.save}">
 *          Enregistrer
 *        </button>
 */
import { css } from 'lit'

/**
 * Loading spinner pour boutons async.
 * Classe: .is-loading (ajouter au .btn)
 */
export const loadingButtonStyles = css`
  .btn.is-loading {
    position: relative;
    pointer-events: none;
    opacity: 0.75;
  }

  .btn.is-loading::after {
    content: '';
    display: inline-block;
    width: 14px;
    height: 14px;
    margin-left: 8px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: btn-spin 0.6s linear infinite;
    vertical-align: middle;
  }

  @keyframes btn-spin {
    to { transform: rotate(360deg); }
  }

  /* Active press feedback */
  .btn:not(:disabled):active {
    transform: scale(0.97);
  }
`
