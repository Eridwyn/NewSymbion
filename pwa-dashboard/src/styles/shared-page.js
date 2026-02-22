/**
 * Shared Page Styles — Lit CSS Module
 *
 * Styles communs aux pages overlay (header, tabs pill).
 * Usage : import { pageHeaderStyles, tabPillStyles } from '../styles/shared-page.js'
 *         static styles = [pageHeaderStyles, tabPillStyles, css`...local...`]
 */
import { css } from 'lit'

/**
 * Header de page overlay avec titre et bouton fermer.
 * Classes: .page-header (ou .settings-header, .notes-header), .page-title
 */
export const pageHeaderStyles = css`
  .page-header {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-6, 1.5rem);
    padding-bottom: var(--space-4, 1rem);
    border-bottom: 1px solid var(--border-default, rgba(255, 255, 255, 0.08));
  }

  .page-title {
    font-size: var(--text-2xl, 1.5rem);
    font-weight: var(--font-bold, 700);
    color: var(--color-dark-text-primary, #f8f9fa);
    display: flex;
    align-items: center;
    gap: var(--space-3, 0.75rem);
  }
`

/**
 * Tabs pill style avec container glass et items actifs context-aware.
 * Classes: .tabs, .tab, .tab.active
 */
export const tabPillStyles = css`
  .tabs {
    display: flex;
    gap: var(--space-1, 0.25rem);
    margin-bottom: var(--space-6, 1.5rem);
    padding: var(--space-2, 0.5rem);
    background: var(--surface-glass-subtle, rgba(255, 255, 255, 0.03));
    border-radius: var(--radius-lg, 12px);
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.05));
  }

  .tab {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-dark-text-secondary, #adb5bd);
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    font-size: var(--text-sm, 0.875rem);
    font-weight: var(--font-medium, 500);
    cursor: pointer;
    border-radius: var(--radius-md, 8px);
    transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
    text-align: center;
    white-space: nowrap;
  }

  .tab:hover {
    background: var(--surface-glass, rgba(255, 255, 255, 0.06));
    color: var(--color-dark-text-primary, #f8f9fa);
  }

  .tab.active {
    background: var(--ctx-bg, rgba(0, 212, 170, 0.05));
    border-color: var(--ctx-border, rgba(0, 212, 170, 0.15));
    color: var(--context-primary, #00d4aa);
  }
`
