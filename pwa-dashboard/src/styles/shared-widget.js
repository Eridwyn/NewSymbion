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
    color: var(--color-dark-text-primary, #f8f9fa);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    animation: textGlow var(--bio-breathe-fast, 8s) ease-in-out infinite;
  }

  .widget-count {
    font-size: var(--text-xs, 0.75rem);
    color: var(--color-dark-text-tertiary, #6c757d);
    background: var(--surface-glass-strong, rgba(255, 255, 255, 0.08));
    padding: 0.2rem 0.6rem;
    border-radius: var(--radius-xl, 999px);
    font-weight: 500;
  }
`

/**
 * Empty/placeholder state for widgets without data.
 * Classes: .empty-state, .empty-state-icon, .empty-state-text, .empty-state-hint
 */
export const emptyStateStyles = css`
  .empty-state {
    text-align: center;
    padding: var(--space-8, 2rem);
    color: var(--color-dark-text-tertiary, #94a3b8);
  }
  .empty-state-icon {
    font-size: 3em;
    margin-bottom: var(--space-4, 1rem);
    opacity: 0.5;
  }
  .empty-state-text {
    font-size: var(--text-sm, 0.875rem);
  }
  .empty-state-hint {
    font-size: var(--text-xs, 0.75rem);
    margin-top: var(--space-2, 0.5rem);
    opacity: 0.7;
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
