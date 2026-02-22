/**
 * Shared Widget Styles — Lit CSS Module
 *
 * Styles communs aux widgets du dashboard (header, title).
 * Usage : import { widgetHeaderStyles } from '../styles/shared-widget.js'
 *         static styles = [sharedAnimations, widgetHeaderStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Header de widget avec titre et zone actions.
 * Classes: .widget-header, .widget-title
 */
export const widgetHeaderStyles = css`
  .widget-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
  }

  .widget-title {
    font-size: 1.2em;
    font-weight: 600;
    color: #e0e0e0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    animation: textGlow var(--bio-breathe-fast, 8s) ease-in-out infinite;
  }
`

/**
 * Sous-titres de section dans les widgets.
 * Classes: .section-title
 */
export const widgetSectionStyles = css`
  .section-title {
    font-size: var(--text-sm, 0.875rem);
    font-weight: 600;
    color: var(--color-dark-text-secondary, #adb5bd);
    margin-bottom: var(--space-3, 0.75rem);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .section-title svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }
`
