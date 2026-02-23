/**
 * Shared Card & Section Styles — Lit CSS Module
 *
 * Cards et sections réutilisables avec thème contextuel.
 * Usage : import { cardStyles, sectionStyles } from '../styles/shared-cards.js'
 *         static styles = [cardStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Card de base avec hover.
 * Classes: .card, .card-header, .card-title, .card-meta, .card-actions
 */
export const cardStyles = css`
  .card {
    background: var(--surface-glass-subtle, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--border-default, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 8px);
    padding: 1rem;
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
  }

  .card:hover {
    border-color: var(--border-hover, rgba(255, 255, 255, 0.15));
    background: var(--surface-glass, rgba(255, 255, 255, 0.05));
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .card-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-dark-text-primary, #f8f9fa);
  }

  .card-meta {
    font-size: var(--text-xs, 0.75rem);
    color: var(--color-dark-text-tertiary, #6c757d);
  }

  .card-actions {
    display: flex;
    gap: 0.5rem;
  }
`

/**
 * Section bio-organic avec gradient contextuel et hover glow.
 * Classes: .section, .section-header, .section-title, .section-description
 */
export const sectionStyles = css`
  .section {
    background: linear-gradient(135deg,
      color-mix(in srgb, var(--context-primary, #00d4aa) 3%, var(--app-section-bg, rgba(19, 20, 26, 0.95))) 0%,
      var(--app-widget-bg-b, rgba(15, 15, 15, 0.9)) 100%);
    border: 1px solid var(--ctx-bg-medium, rgba(0, 212, 170, 0.12));
    border-radius: var(--radius-lg, 12px);
    padding: var(--space-5, 1.25rem);
    margin-bottom: var(--space-5, 1.25rem);
    backdrop-filter: blur(var(--blur-base, 8px));
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3),
                0 0 0 1px color-mix(in srgb, var(--context-primary, #00d4aa) 6%, transparent),
                inset 0 1px 0 color-mix(in srgb, var(--context-primary, #00d4aa) 4%, transparent);
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
    overflow: hidden;
  }

  .section:hover {
    border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 18%, transparent);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4),
                0 0 0 1px var(--ctx-border-subtle, rgba(0, 212, 170, 0.15)),
                0 0 30px var(--ctx-bg-subtle, rgba(0, 212, 170, 0.05)),
                inset 0 1px 0 var(--ctx-bg, rgba(0, 212, 170, 0.08));
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.25rem;
  }

  .section-title {
    font-size: var(--text-lg, 1.125rem);
    font-weight: var(--font-semibold, 600);
    color: var(--context-primary, #00d4aa);
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.5rem);
  }

  .section-description {
    color: var(--color-dark-text-secondary, #adb5bd);
    font-size: var(--text-sm, 0.875rem);
    margin-bottom: var(--space-4, 1rem);
    line-height: var(--leading-normal, 1.5);
  }
`
