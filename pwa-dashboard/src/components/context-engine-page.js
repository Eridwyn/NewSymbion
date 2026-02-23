/**
 * Context Engine Page - Page Unifiée
 *
 * Fusionne Context Engine + Automations + Validations + Stats + Config
 * en une seule interface cohérente.
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations, pageTransitionStyles } from '../styles/shared-animations.js'
import { overlayStyles, closeButtonStyles, scrollbarStyles } from '../styles/shared-patterns.js'
import { cardStyles } from '../styles/shared-cards.js'
import { formInputStyles, btnSuccessStyles, btnSizeStyles } from '../styles/shared-forms.js'
import csrfService from '../services/csrf-service.js'
import automationsService from '../services/automations-service.js'
import { getDayNameShort, getDayNameFull } from '../utils/time-utils.js'

// Mixins (code splitting)
import { AutomationsMixin } from './ce-automations-mixin.js'
import { IntelligenceMixin } from './ce-intelligence-mixin.js'
import { ModesMixin } from './ce-modes-mixin.js'

class ContextEnginePage extends AutomationsMixin(IntelligenceMixin(ModesMixin(LitElement))) {
  static styles = [sharedAnimations, pageTransitionStyles, overlayStyles, closeButtonStyles, scrollbarStyles, formInputStyles, cardStyles, btnSuccessStyles, btnSizeStyles, css`
    :host {
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .page {
      width: 95%;
      max-width: 800px;
      max-height: 90vh;
      background: linear-gradient(135deg, rgba(19, 20, 26, 0.98) 0%, rgba(10, 10, 11, 1) 100%);
      border: 1px solid var(--ctx-bg-emphasis);
      border-radius: var(--radius-lg);
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6),
                  0 0 80px var(--ctx-border-subtle);
      overflow: hidden;
      display: flex;
      flex-direction: column;
      animation: scaleIn 0.25s ease-out;
    }

    @keyframes pulse-ring {
      0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--pulse-color), 0.7); }
      70% { transform: scale(1); box-shadow: 0 0 0 10px rgba(var(--pulse-color), 0); }
      100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--pulse-color), 0); }
    }

    /* Intelligence Tab Animations */
    @keyframes gauge-fill {
      from { stroke-dashoffset: 283; }
      to { stroke-dashoffset: var(--target-offset, 0); }
    }

    @keyframes glow-pulse {
      0%, 100% { filter: drop-shadow(0 0 8px var(--glow-color, rgba(139, 92, 246, 0.5))); }
      50% { filter: drop-shadow(0 0 20px var(--glow-color, rgba(139, 92, 246, 0.8))); }
    }

    @keyframes count-up {
      from { opacity: 0; transform: scale(0.5); }
      to { opacity: 1; transform: scale(1); }
    }

    @keyframes card-enter {
      from { opacity: 0; transform: translateY(16px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @keyframes bar-fill {
      from { width: 0; }
      to { width: var(--bar-width, 0%); }
    }

    @keyframes float-icon {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-6px); }
    }

    /* Toast Notifications */
    .toast-container {
      position: fixed;
      bottom: 24px;
      left: 50%;
      transform: translateX(-50%);
      z-index: 10001;
      display: flex;
      flex-direction: column;
      gap: 8px;
      pointer-events: none;
    }

    .toast {
      padding: 0.75rem 1.25rem;
      border-radius: var(--radius-md);
      background: rgba(25, 26, 32, 0.95);
      border: 1px solid var(--border-medium);
      backdrop-filter: blur(16px);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85rem;
      display: flex;
      align-items: center;
      gap: 0.75rem;
      animation: slideUp 0.3s ease-out;
      pointer-events: auto;
    }

    .toast.leaving {
      animation: slideDown 0.25s ease-in forwards;
    }

    .toast.success {
      border-color: rgba(34, 197, 94, 0.4);
      background: linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(25, 26, 32, 0.95) 100%);
    }

    .toast.error {
      border-color: rgba(239, 68, 68, 0.4);
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.15) 0%, rgba(25, 26, 32, 0.95) 100%);
    }

    .toast.info {
      border-color: rgba(59, 130, 246, 0.4);
      background: linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(25, 26, 32, 0.95) 100%);
    }

    .toast-icon {
      font-size: 1.1rem;
    }

    .toast-message {
      flex: 1;
    }

    /* Confirmation Overlay */
    .confirm-overlay {
      position: fixed;
      inset: 0;
      z-index: 10002;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(0, 0, 0, 0.8);
      backdrop-filter: blur(var(--blur-base));
      animation: fadeIn 0.2s ease-out;
    }

    .confirm-dialog {
      background: linear-gradient(135deg, rgba(25, 26, 32, 0.98) 0%, rgba(15, 15, 17, 1) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-lg);
      padding: 1.5rem;
      max-width: 380px;
      width: 90%;
      animation: scaleIn 0.25s ease-out;
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
    }

    .confirm-icon {
      font-size: 2.5rem;
      text-align: center;
      margin-bottom: 1rem;
    }

    .confirm-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      text-align: center;
      margin-bottom: 0.5rem;
    }

    .confirm-message {
      font-size: 0.85rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      text-align: center;
      margin-bottom: 1.5rem;
      line-height: 1.5;
    }

    .confirm-actions {
      display: flex;
      gap: 0.75rem;
    }

    .confirm-actions .btn {
      flex: 1;
      justify-content: center;
    }

    /* Mode Change Overlay */
    .mode-change-overlay {
      position: fixed;
      inset: 0;
      z-index: 10000;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(0, 0, 0, 0.9);
      backdrop-filter: blur(16px);
      animation: fadeIn 0.3s ease-out;
    }

    .mode-change-content {
      text-align: center;
      animation: scaleIn 0.4s ease-out;
    }

    .mode-change-icon {
      font-size: 5rem;
      margin-bottom: 1rem;
      animation: float 2s ease-in-out infinite;
    }

    .mode-change-name {
      font-size: var(--text-3xl);
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 0.5rem;
    }

    .mode-change-message {
      font-size: 0.9rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* Skeleton Loading */
    .skeleton {
      background: linear-gradient(90deg,
        var(--surface-glass-subtle) 25%,
        var(--surface-glass-hover) 50%,
        var(--surface-glass-subtle) 75%);
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
      border-radius: var(--radius-base);
    }

    .skeleton-text {
      height: 1rem;
      margin-bottom: 0.5rem;
    }

    .skeleton-text.short {
      width: 60%;
    }

    .skeleton-card {
      height: 100px;
      margin-bottom: 0.75rem;
    }

    /* Enhanced Validation Cards */
    .validation-card {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 1.25rem;
      margin-bottom: 1rem;
      transition: all var(--duration-base) var(--ease-out);
      position: relative;
      overflow: hidden;
    }

    .validation-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 3px;
      background: linear-gradient(90deg,
        var(--validation-color, #f59e0b),
        color-mix(in srgb, var(--validation-color, #f59e0b) 70%, white));
    }

    .validation-card:hover {
      background: var(--surface-glass);
      border-color: var(--border-hover);
      transform: translateY(-2px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
    }

    .validation-header {
      display: flex;
      align-items: flex-start;
      gap: 1rem;
      margin-bottom: 1rem;
    }

    .validation-icon {
      font-size: var(--text-3xl);
      padding: 0.5rem;
      background: rgba(245, 158, 11, 0.15);
      border-radius: var(--radius-md);
    }

    .validation-info {
      flex: 1;
    }

    .validation-title {
      font-size: 1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      margin-bottom: 0.25rem;
    }

    .validation-subtitle {
      font-size: 0.8rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .validation-trust-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0.75rem;
      background: var(--surface-glass-subtle);
      border-radius: var(--radius-base);
    }

    .validation-trust-bar {
      width: 60px;
      height: 6px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm, 0.375rem);
      overflow: hidden;
    }

    .validation-trust-fill {
      height: 100%;
      border-radius: var(--radius-sm, 0.375rem);
      transition: width 0.5s ease;
    }

    .validation-trust-fill.high { background: linear-gradient(90deg, #22c55e, #4ade80); }
    .validation-trust-fill.medium { background: linear-gradient(90deg, #f59e0b, #fbbf24); }
    .validation-trust-fill.low { background: linear-gradient(90deg, #ef4444, #f87171); }

    .validation-reasons {
      background: var(--surface-glass-faint);
      border-radius: var(--radius-base);
      padding: 0.75rem;
      margin-bottom: 1rem;
    }

    .validation-reasons-title {
      font-size: 0.7rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 0.5rem;
    }

    .validation-reason-item {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      padding: 0.25rem 0;
    }

    .validation-reason-bullet {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--context-primary, #00d4aa);
    }

    .validation-actions {
      display: flex;
      gap: 0.75rem;
    }

    .validation-btn {
      flex: 1;
      padding: 0.75rem 1rem;
      border-radius: var(--radius-md, 0.75rem);
      border: 1px solid transparent;
      font-size: 0.85rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
    }

    .validation-btn.approve {
      background: rgba(34, 197, 94, 0.15);
      border-color: rgba(34, 197, 94, 0.4);
      color: #22c55e;
    }

    .validation-btn.approve:hover {
      background: rgba(34, 197, 94, 0.25);
      transform: translateY(-1px);
    }

    .validation-btn.reject {
      background: rgba(239, 68, 68, 0.15);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ef4444;
    }

    .validation-btn.reject:hover {
      background: rgba(239, 68, 68, 0.25);
      transform: translateY(-1px);
    }

    /* Header */
    .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: var(--space-4) var(--space-5);
      border-bottom: 1px solid var(--border-default);
      background: rgba(0, 0, 0, 0.3);
      flex-shrink: 0;
    }

    .header-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    /* Tabs — inline compact style for modal */
    .tabs {
      display: flex;
      gap: 0;
      padding: 0;
      margin: 0;
      background: rgba(0, 0, 0, 0.15);
      border-bottom: 1px solid var(--border-subtle);
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
      scrollbar-width: none;
      flex-shrink: 0;
    }

    .tabs::-webkit-scrollbar {
      display: none;
    }

    .tab {
      flex: 1;
      background: transparent;
      border: none;
      border-bottom: 2px solid transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      padding: var(--space-3) var(--space-4);
      font-size: 0.8rem;
      font-weight: var(--font-medium, 500);
      cursor: pointer;
      transition: all var(--duration-base, 0.2s) var(--ease-out, ease-out);
      text-align: center;
      white-space: nowrap;
    }

    .tab:hover {
      background: rgba(255, 255, 255, 0.04);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .tab.active {
      color: var(--context-primary, #00d4aa);
      border-bottom-color: var(--context-primary, #00d4aa);
      background: rgba(255, 255, 255, 0.03);
    }

    .tab .badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 18px;
      height: 18px;
      padding: 0 5px;
      margin-left: 6px;
      border-radius: var(--radius-full, 9999px);
      background: rgba(239, 68, 68, 0.8);
      color: white;
      font-size: 0.65rem;
      font-weight: 700;
    }

    /* Content */
    .content {
      flex: 1;
      overflow-y: auto;
      padding: 1.25rem;
    }

    /* Mode Tab */
    .mode-display {
      text-align: center;
      padding: 2rem 1rem;
    }

    .mode-icon {
      font-size: 4rem;
      margin-bottom: 1rem;
      animation: float 3s ease-in-out infinite;
    }

    .mode-name {
      font-size: var(--text-2xl);
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
      margin-bottom: 0.5rem;
    }

    .mode-reason {
      font-size: 0.85rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 1.5rem;
    }

    .confidence-bar {
      width: 200px;
      height: 8px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm);
      margin: 0 auto 0.5rem;
      overflow: hidden;
    }

    .confidence-fill {
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary, #00d4aa), color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white));
      border-radius: var(--radius-sm);
      transition: width 0.5s ease;
    }

    .confidence-text {
      font-size: var(--text-xs);
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .override-info {
      margin: 1.5rem 0;
      padding: 0.75rem 1rem;
      background: rgba(251, 146, 60, 0.1);
      border: 1px solid rgba(251, 146, 60, 0.3);
      border-radius: var(--radius-base);
      color: #fb923c;
      font-size: 0.8rem;
    }

    /* Mode Controls */
    .mode-controls {
      margin-top: 2rem;
      padding-top: 1.5rem;
      border-top: 1px solid var(--border-default);
    }

    .controls-title {
      font-size: var(--text-xs);
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 1rem;
    }

    .mode-buttons {
      display: flex;
      gap: 0.75rem;
      justify-content: center;
      margin-bottom: 1rem;
    }

    .mode-btn {
      padding: 0.75rem 1.25rem;
      border-radius: var(--radius-md, 0.75rem);
      border: 1px solid var(--border-hover);
      background: var(--surface-glass);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85rem;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .mode-btn:hover {
      background: var(--surface-glass-strong);
      border-color: rgba(255, 255, 255, 0.25);
      transform: translateY(-2px);
    }

    .mode-btn.cravate:hover { border-color: #3b82f6; }
    .mode-btn.intime:hover { border-color: var(--context-primary, #00d4aa); }
    .mode-btn.neutre:hover { border-color: #6b7280; }

    .duration-buttons {
      display: flex;
      gap: 0.5rem;
      justify-content: center;
      margin-bottom: 1rem;
    }

    .duration-btn {
      padding: 0.4rem 0.8rem;
      border-radius: var(--radius-sm);
      border: 1px solid var(--border-medium);
      background: transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: var(--text-xs);
      cursor: pointer;
      transition: all 0.2s;
    }

    .duration-btn:hover, .duration-btn.active {
      background: var(--ctx-border);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    .clear-override-btn {
      padding: 0.5rem 1rem;
      border-radius: var(--radius-base);
      border: 1px solid rgba(239, 68, 68, 0.3);
      background: rgba(239, 68, 68, 0.1);
      color: #ef4444;
      font-size: 0.8rem;
      cursor: pointer;
      transition: all 0.2s;
    }

    .clear-override-btn:hover {
      background: rgba(239, 68, 68, 0.2);
    }

    /* Cards — local override (margin-bottom for stacking) */
    .card {
      margin-bottom: 0.75rem;
    }

    /* Buttons — local overrides (compact modal sizing) */
    .btn {
      padding: 0.5rem 1rem;
      font-size: 0.8rem;
      font-weight: 500;
    }

    .btn-primary {
      background: linear-gradient(135deg, var(--ctx-border-medium) 0%, var(--ctx-border-subtle) 100%);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    .btn-primary:hover {
      background: linear-gradient(135deg, var(--ctx-bg-intense) 0%, var(--ctx-border-medium) 100%);
      transform: translateY(-1px);
    }

    /* Toggle */
    .toggle {
      position: relative;
      width: 40px;
      height: 22px;
      background: var(--surface-glass-bright);
      border-radius: var(--radius-full, 9999px);
      cursor: pointer;
      transition: background 0.2s;
    }

    .toggle.active {
      background: var(--context-primary, #00d4aa);
    }

    .toggle::after {
      content: '';
      position: absolute;
      top: 2px;
      left: 2px;
      width: 18px;
      height: 18px;
      background: white;
      border-radius: 50%;
      transition: transform 0.2s;
    }

    .toggle.active::after {
      transform: translateX(18px);
    }

    /* Trust Badge */
    .trust-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding: 0.2rem 0.5rem;
      border-radius: var(--radius-sm);
      font-size: 0.7rem;
      font-weight: 600;
    }

    .trust-badge.high {
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
    }

    .trust-badge.medium {
      background: rgba(251, 191, 36, 0.15);
      color: #fbbf24;
    }

    .trust-badge.low {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
    }

    /* ===== Enhanced Automation Cards ===== */
    .automation-card {
      background: linear-gradient(135deg, var(--surface-glass-subtle) 0%, rgba(255, 255, 255, 0.01) 100%);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      padding: 0;
      margin-bottom: 0.75rem;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      overflow: hidden;
      position: relative;
    }

    .automation-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 4px;
      height: 100%;
      background: var(--card-status-color, rgba(255, 255, 255, 0.2));
      transition: all var(--duration-base) var(--ease-out);
    }

    .automation-card.enabled::before {
      background: var(--context-primary, #00d4aa);
    }

    .automation-card.disabled::before {
      background: rgba(239, 68, 68, 0.5);
    }

    .automation-card:hover {
      border-color: var(--border-hover);
      background: linear-gradient(135deg, var(--surface-glass) 0%, var(--surface-glass-faint) 100%);
      transform: translateY(-2px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    }

    .automation-card.highlighted {
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 1px var(--context-primary, #00d4aa),
                  0 8px 32px var(--ctx-border-medium);
    }

    .automation-card.add-new {
      border-style: dashed;
      border-color: var(--border-hover);
      background: transparent;
    }

    .automation-card.add-new::before {
      display: none;
    }

    .automation-card.add-new:hover {
      border-color: var(--context-primary, #00d4aa);
      background: var(--ctx-bg-subtle);
    }

    .automation-card.add-new:hover div {
      color: var(--context-primary, #00d4aa) !important;
      opacity: 1 !important;
    }

    .automation-card-inner {
      padding: 0.75rem 0.75rem 0.75rem 1rem;
    }

    .automation-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .automation-status-icon {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--surface-glass-bright);
      flex-shrink: 0;
    }

    .automation-card.enabled .automation-status-icon {
      background: var(--context-primary, #00d4aa);
    }

    .automation-card.disabled .automation-status-icon {
      background: rgba(239, 68, 68, 0.6);
    }

    .automation-info {
      flex: 1;
      min-width: 0;
    }

    .automation-title-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .automation-title {
      font-size: 0.9rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .automation-category-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.2rem;
      padding: 0.1rem 0.4rem;
      border-radius: var(--radius-sm);
      font-size: 0.6rem;
      font-weight: 500;
      white-space: nowrap;
      background: var(--border-subtle);
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .automation-category-badge.comfort { background: rgba(34, 197, 94, 0.15); color: #22c55e; }
    .automation-category-badge.security { background: rgba(239, 68, 68, 0.15); color: #ef4444; }
    .automation-category-badge.energy { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
    .automation-category-badge.notifications { background: rgba(168, 85, 247, 0.15); color: #a855f7; }
    .automation-category-badge.custom { background: rgba(251, 191, 36, 0.15); color: #fbbf24; }

    .automation-trust-badge {
      display: inline-flex;
      align-items: center;
      padding: 0.1rem 0.3rem;
      border-radius: var(--radius-sm);
      font-size: 0.7rem;
      background: var(--ctx-border);
      color: var(--context-primary, #00d4aa);
      animation: glow-pulse 2s ease-in-out infinite;
    }

    .automation-subtitle {
      font-size: var(--text-xs);
      color: var(--color-dark-text-tertiary, #6c757d);
      display: flex;
      align-items: center;
      gap: 0.4rem;
      flex-wrap: wrap;
      margin-top: 0.15rem;
    }

    .automation-subtitle .sep {
      opacity: 0.4;
    }

    .automation-actions {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-shrink: 0;
      margin-left: auto;
    }

    .automation-quick-actions {
      display: flex;
      gap: 0.25rem;
      opacity: 0;
      transition: opacity 0.2s ease;
    }

    .automation-card:hover .automation-quick-actions {
      opacity: 1;
    }

    @media (max-width: 768px) {
      .automation-card-inner {
        padding: 0.625rem 0.625rem 0.625rem 0.875rem;
      }
      .automation-title { font-size: 0.85rem; }
      .automation-quick-actions { opacity: 1; }
    }

    .quick-action-btn {
      width: 28px;
      height: 28px;
      border-radius: var(--radius-base);
      border: 1px solid var(--border-medium);
      background: var(--surface-glass);
      color: var(--color-dark-text-secondary, #adb5bd);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: var(--text-xs);
      transition: all 0.2s ease;
    }

    .quick-action-btn:hover {
      background: var(--surface-glass-strong);
      border-color: rgba(255, 255, 255, 0.2);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .quick-action-btn.play:hover {
      background: rgba(34, 197, 94, 0.15);
      border-color: rgba(34, 197, 94, 0.3);
      color: #22c55e;
    }

    /* ===== Enhanced Category Filter ===== */
    .category-filter-bar {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
      flex-wrap: wrap;
      padding: 0.5rem;
      background: var(--surface-glass-faint);
      border-radius: var(--radius-md);
      border: 1px solid var(--border-subtle);
    }

    .category-pill {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.5rem 0.85rem;
      border-radius: var(--radius-xl);
      border: 1px solid var(--border-default);
      background: var(--surface-glass-subtle);
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
      white-space: nowrap;
    }

    .category-pill:hover {
      background: var(--surface-glass-hover);
      border-color: var(--border-hover);
    }

    .category-pill.active {
      background: linear-gradient(135deg,
        var(--ctx-bg-emphasis) 0%,
        var(--ctx-border) 100%);
      border-color: var(--ctx-border-intense);
      color: var(--context-primary, #00d4aa);
      font-weight: 600;
    }

    .category-pill-icon {
      font-size: 0.9rem;
    }

    .category-pill-count {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 20px;
      height: 20px;
      padding: 0 0.35rem;
      border-radius: var(--radius-full, 9999px);
      background: var(--surface-glass-strong);
      font-size: 0.7rem;
      font-weight: 600;
    }

    .category-pill.active .category-pill-count {
      background: var(--ctx-bg-intense);
    }

    /* ===== Automations Header Stats ===== */
    .automations-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
      padding: 0.75rem 1rem;
      background: var(--surface-glass-faint);
      border-radius: var(--radius-md);
      border: 1px solid var(--border-subtle);
    }

    .automations-stats {
      display: flex;
      gap: 1.5rem;
    }

    .automation-stat {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .automation-stat-value {
      font-size: var(--text-xl);
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
    }

    .automation-stat-label {
      font-size: var(--text-xs);
      color: var(--color-dark-text-tertiary, #6c757d);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .automation-stat-divider {
      width: 1px;
      height: 24px;
      background: var(--surface-glass-strong);
    }

    /* ===== Enhanced History Section ===== */
    .history-section {
      margin-top: 1.5rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border-default);
    }

    .history-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.75rem;
    }

    .history-title {
      font-size: 0.85rem;
      font-weight: 600;
      color: var(--color-dark-text-secondary, #adb5bd);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .history-title-icon {
      font-size: 1rem;
    }

    .history-timeline {
      position: relative;
      padding-left: 1.5rem;
    }

    .history-timeline::before {
      content: '';
      position: absolute;
      left: 6px;
      top: 0;
      bottom: 0;
      width: 2px;
      background: linear-gradient(180deg,
        var(--surface-glass-strong) 0%,
        var(--surface-glass-faint) 100%);
      border-radius: 1px;
    }

    .history-item {
      position: relative;
      padding: 0.75rem 1rem;
      margin-bottom: 0.5rem;
      background: var(--surface-glass-faint);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md, 0.75rem);
      transition: all 0.2s ease;
    }

    .history-item::before {
      content: '';
      position: absolute;
      left: -1.5rem;
      top: 50%;
      transform: translate(-50%, -50%);
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: var(--history-status-color, rgba(255, 255, 255, 0.2));
      border: 2px solid rgba(15, 15, 17, 1);
      z-index: 1;
    }

    .history-item.success::before {
      background: #22c55e;
      box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
    }

    .history-item.failed::before {
      background: #ef4444;
      box-shadow: 0 0 8px rgba(239, 68, 68, 0.5);
    }

    .history-item:hover {
      background: rgba(255, 255, 255, 0.04);
      border-color: var(--border-medium);
    }

    .history-item-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.25rem;
    }

    .history-item-name {
      font-size: 0.85rem;
      font-weight: 500;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .history-item-status {
      font-size: 0.9rem;
    }

    .history-item-time {
      font-size: 0.7rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .history-item-details {
      display: flex;
      gap: 1rem;
      font-size: var(--text-xs);
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* ===== Enhanced Empty State ===== */
    .empty-state-enhanced {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 3rem 2rem;
      text-align: center;
      background: linear-gradient(135deg, var(--surface-glass-faint) 0%, transparent 100%);
      border: 2px dashed var(--border-medium);
      border-radius: var(--radius-lg);
    }

    .empty-state-icon-container {
      width: 80px;
      height: 80px;
      border-radius: var(--radius-xl);
      background: linear-gradient(135deg,
        var(--ctx-border) 0%,
        var(--ctx-bg-subtle) 100%);
      border: 1px solid var(--ctx-border-medium);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 2.5rem;
      margin-bottom: 1.25rem;
    }

    .empty-state-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      margin-bottom: 0.5rem;
    }

    .empty-state-description {
      font-size: 0.85rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 1.5rem;
      max-width: 280px;
      line-height: 1.5;
    }

    .empty-state-suggestions {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      justify-content: center;
      margin-top: 1rem;
    }

    .suggestion-chip {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.4rem 0.75rem;
      border-radius: var(--radius-base);
      background: var(--surface-glass);
      border: 1px solid var(--border-default);
      font-size: var(--text-xs);
      color: var(--color-dark-text-secondary, #adb5bd);
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .suggestion-chip:hover {
      background: var(--surface-glass-strong);
      border-color: var(--border-hover);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Config */
    .config-section {
      margin-bottom: 1.5rem;
    }

    .config-title {
      font-size: var(--text-xs);
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-dark-text-tertiary, #6c757d);
      margin-bottom: 1rem;
    }

    .config-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.75rem 0;
      border-bottom: 1px solid var(--border-subtle);
    }

    .config-label {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .config-input {
      width: 80px;
      padding: 0.4rem 0.6rem;
      border-radius: var(--radius-sm);
      border: 1px solid var(--border-hover);
      background: var(--surface-glass);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.8rem;
      text-align: center;
    }

    .config-input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
    }

    /* Empty state */
    .empty-state {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 0.9rem;
      margin-bottom: 1rem;
    }

    /* Form */
    .form-group {
      margin-bottom: 1rem;
    }

    .form-group label {
      display: block;
      font-size: 0.8rem;
      font-weight: 500;
      color: var(--color-dark-text-secondary, #adb5bd);
      margin-bottom: 0.4rem;
    }

    select.form-input {
      cursor: pointer;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 3rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* ============ MODES TAB STYLES (Unified) ============ */
    .modes-container {
      padding: 0;
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
    }

    .section-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
    }

    .section-header h3 {
      margin: 0;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 1rem;
      font-weight: 600;
    }

    /* Current Mode Section */
    .current-mode-section {
      background: var(--surface-glass-faint);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      padding: 1rem;
    }

    .current-mode-display {
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .mode-status {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .current-mode-icon {
      font-size: 2.5rem;
    }

    .mode-info {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .current-mode-name {
      font-size: var(--text-xl);
      font-weight: 700;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .mode-reason {
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .confidence-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .confidence-bar-mini {
      width: 60px;
      height: 6px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm, 0.375rem);
      overflow: hidden;
    }

    .confidence-bar-mini .confidence-fill {
      height: 100%;
      background: linear-gradient(90deg, #22c55e, #16a34a);
      border-radius: var(--radius-sm, 0.375rem);
      transition: width 0.3s ease;
    }

    .confidence-value {
      font-size: var(--text-xs);
      color: var(--color-dark-text-secondary, #adb5bd);
      font-weight: 600;
    }

    .override-banner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: rgba(245, 158, 11, 0.15);
      border: 1px solid rgba(245, 158, 11, 0.3);
      border-radius: var(--radius-base);
      padding: 0.5rem 0.75rem;
      font-size: 0.85rem;
      color: #fbbf24;
    }

    .quick-mode-controls {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
    }

    .mode-buttons-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }

    .mode-quick-btn {
      display: flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.5rem 0.75rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85rem;
      cursor: pointer;
      transition: all 0.2s;
    }

    .mode-quick-btn:hover {
      background: var(--btn-color, #6b7280);
      border-color: var(--btn-color, #6b7280);
      color: white;
    }

    .mode-quick-btn.active {
      background: var(--btn-color, #6b7280);
      border-color: var(--btn-color, #6b7280);
      color: white;
      box-shadow: 0 0 12px rgba(var(--btn-color), 0.3);
    }

    .duration-selector {
      display: flex;
      gap: 0.5rem;
    }

    .loading-state {
      text-align: center;
      padding: 2rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    /* Modes Management Section */
    .modes-management-section {
      background: var(--surface-glass-faint);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      padding: 1rem;
    }

    .modes-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1.5rem;
    }

    .modes-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
      gap: 1rem;
    }

    .mode-card {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md);
      padding: 1rem;
      transition: all 0.2s;
    }

    .mode-card:hover {
      background: var(--border-subtle);
      border-color: var(--mode-primary, #6b7280);
    }

    .mode-card-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .mode-card-icon {
      font-size: var(--text-2xl);
    }

    .mode-card-name {
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      flex: 1;
    }

    .system-badge {
      font-size: 0.6rem;
      padding: 0.15rem 0.4rem;
      background: rgba(99, 102, 241, 0.2);
      color: #818cf8;
      border-radius: var(--radius-sm);
      text-transform: uppercase;
      font-weight: 600;
    }

    .mode-card-preview {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .color-preview {
      width: 24px;
      height: 24px;
      border-radius: var(--radius-sm);
    }

    .mode-card-slug {
      font-size: var(--text-xs);
      color: var(--color-dark-text-tertiary, #6c757d);
      font-family: monospace;
      margin-bottom: 0.75rem;
    }

    .mode-card-actions {
      display: flex;
      gap: 0.5rem;
      justify-content: flex-end;
    }

    .btn-sm {
      padding: 0.35rem 0.6rem;
      font-size: 0.8rem;
    }

    .btn-danger {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
    }

    .btn-danger:hover {
      background: rgba(239, 68, 68, 0.25);
    }

    /* Mode Form Modal */
    .modal-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.7);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 100;
    }

    .mode-form {
      background: linear-gradient(135deg, rgba(25, 26, 32, 0.98) 0%, rgba(15, 15, 17, 1) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-lg);
      width: 90%;
      max-width: 420px;
      max-height: 90vh;
      overflow-y: auto;
    }

    .form-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 1.25rem;
      border-bottom: 1px solid var(--border-default);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .form-body {
      padding: 1.25rem;
    }

    .form-actions {
      display: flex;
      justify-content: flex-end;
      gap: 0.75rem;
      padding: 1rem 1.25rem;
      border-top: 1px solid var(--border-default);
    }

    .emoji-picker {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }

    .emoji-btn {
      width: 40px;
      height: 40px;
      font-size: 1.3rem;
      background: var(--surface-glass);
      border: 2px solid transparent;
      border-radius: var(--radius-base);
      cursor: pointer;
      transition: all 0.2s;
    }

    .emoji-btn:hover {
      background: var(--surface-glass-strong);
    }

    .emoji-btn.selected {
      border-color: var(--context-primary, #00d4aa);
      background: var(--ctx-border-subtle);
    }

    .color-pickers {
      display: flex;
      gap: 1rem;
    }

    .color-picker-group {
      flex: 1;
      text-align: center;
    }

    .color-picker-group label {
      font-size: 0.7rem;
      display: block;
      margin-bottom: 0.25rem;
    }

    .color-picker-group input[type="color"] {
      width: 100%;
      height: 40px;
      border: none;
      border-radius: var(--radius-base);
      cursor: pointer;
      background: transparent;
    }

    .color-picker-group input[type="color"]::-webkit-color-swatch-wrapper {
      padding: 2px;
    }

    .color-picker-group input[type="color"]::-webkit-color-swatch {
      border-radius: var(--radius-sm);
      border: 1px solid var(--border-hover);
    }

    .mode-preview {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 1rem;
      background: var(--preview-bg, #f8fafc);
      border-radius: var(--radius-md, 0.75rem);
      border: 2px solid var(--preview-primary, #2563eb);
    }

    .preview-icon {
      font-size: var(--text-3xl);
    }

    .preview-name {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--preview-accent, #1e40af);
    }

    /* Mobile */
    @media (max-width: 480px) {
      .page {
        width: 100%;
        height: 100%;
        max-height: 100vh;
        border-radius: 0;
      }

      .tab {
        flex: none;
        padding: 0.5rem 0.75rem;
        font-size: var(--text-xs);
      }

      .mode-buttons {
        flex-wrap: wrap;
      }

      .modes-grid {
        grid-template-columns: 1fr;
      }

      .color-pickers {
        flex-direction: column;
        gap: 0.75rem;
      }
    }

    /* === Utility classes (ex-inline styles) === */

    /* Layout */
    .ce-flex { display: flex; align-items: center; gap: 0.5rem; }
    .ce-flex-col { display: flex; flex-direction: column; gap: 0.5rem; }
    .ce-flex-col-md { display: flex; flex-direction: column; gap: 0.75rem; }
    .ce-flex-wrap { display: flex; flex-wrap: wrap; gap: 0.5rem; }
    .ce-flex-wrap-sm { display: flex; flex-wrap: wrap; gap: 0.25rem; }
    .ce-flex-between { display: flex; justify-content: space-between; align-items: center; }
    .ce-flex-between-mb { display: flex; justify-content: space-between; margin-bottom: 0.25rem; }
    .ce-flex-grow { flex: 1; }
    .ce-flex-shrink-0 { flex-shrink: 0; }
    .ce-flex-min-0 { flex: 1; min-width: 0; }
    .ce-ml-auto { margin-left: auto; }
    .ce-cursor-pointer { cursor: pointer; user-select: none; }

    /* Spacing */
    .ce-gap-md { gap: 0.75rem; }
    .ce-gap-lg { gap: 1rem; }
    .ce-mt-xs { margin-top: 0.25rem; }
    .ce-mt-sm { margin-top: 0.5rem; }
    .ce-mt-md { margin-top: 0.75rem; }
    .ce-mt-lg { margin-top: 1rem; }
    .ce-mt-xl { margin-top: 1.5rem; }
    .ce-mb-0 { margin-bottom: 0; }
    .ce-m-0 { margin: 0; }
    .ce-heading { margin: 0 0 0.5rem; font-weight: 600; }

    /* Typography */
    .ce-text-xs { font-size: var(--text-xs); }
    .ce-text-xs-secondary { font-size: var(--text-xs); color: var(--color-dark-text-secondary); }
    .ce-text-xs-tertiary { font-size: var(--text-xs); color: var(--color-dark-text-tertiary); }
    .ce-text-sm { font-size: 0.8rem; }
    .ce-text-sm-secondary { font-size: 0.8rem; color: var(--color-dark-text-secondary); line-height: 1.6; }
    .ce-text-md { font-size: 0.85rem; }
    .ce-text-tiny { font-size: 0.7rem; }
    .ce-text-tiny-tertiary { font-size: 0.7rem; color: var(--color-dark-text-tertiary); }
    .ce-text-bold { font-weight: 600; }
    .ce-text-center { text-align: center; }
    .ce-text-2xl-bold { font-size: var(--text-2xl); font-weight: 700; }

    /* Couleurs texte */
    .ce-text-primary { color: var(--color-dark-text-primary); }
    .ce-text-secondary { color: var(--color-dark-text-secondary); }
    .ce-text-tertiary { color: var(--color-dark-text-tertiary); }
    .ce-text-purple { color: #a78bfa; }
    .ce-text-indigo { color: #818cf8; }
    .ce-text-blue { color: #3b82f6; }
    .ce-text-green { color: #22c55e; }
    .ce-text-emerald { color: #10b981; }
    .ce-text-amber { color: #f59e0b; }
    .ce-text-orange { color: #fb923c; }
    .ce-text-red { color: #ef4444; }

    /* Backgrounds */
    .ce-bg-dark { background: rgba(0, 0, 0, 0.2); padding: 0.5rem; border-radius: var(--radius-sm); }
    .ce-bg-dark-xs { background: rgba(0, 0, 0, 0.2); padding: 0.1rem 0.3rem; border-radius: var(--radius-sm); }
    .ce-bg-glass { background: rgba(255, 255, 255, 0.02); border-radius: var(--radius-base); border: 1px solid rgba(255, 255, 255, 0.08); }
    .ce-bg-glass-item { background: rgba(255, 255, 255, 0.03); border-radius: var(--radius-sm); border: 1px solid rgba(255, 255, 255, 0.08); padding: 0.4rem 0.6rem; }
    .ce-bg-glass-dashed { background: rgba(255, 255, 255, 0.02); border-radius: var(--radius-base); border: 1px dashed rgba(255, 255, 255, 0.1); margin-top: 0.75rem; }
    .ce-stat-item { padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: var(--radius-base); text-align: center; }

    /* Badges */
    .ce-badge-purple { padding: 0.35rem 0.875rem; background: rgba(139, 92, 246, 0.15); border: 1px solid rgba(139, 92, 246, 0.3); color: #a78bfa; font-size: var(--text-xs); border-radius: var(--radius-xl); }
    .ce-badge-green { padding: 0.25rem 0.75rem; background: rgba(34, 197, 94, 0.15); border: 1px solid rgba(34, 197, 94, 0.25); border-radius: var(--radius-xl); font-size: var(--text-xs); color: #22c55e; }
    .ce-badge-orange { padding: 0.25rem 0.75rem; background: rgba(245, 158, 11, 0.15); border: 1px solid rgba(245, 158, 11, 0.3); border-radius: var(--radius-xl); font-size: 0.8rem; color: #f59e0b; }
    .ce-badge-neutral { padding: 0.3rem 0.75rem; border-radius: var(--radius-xl); background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); }
    .ce-badge-tiny { font-size: 0.6rem; padding: 0.15rem 0.4rem; }

    /* Bouton aide */
    .ce-btn-help { cursor: pointer; font-size: var(--text-xs); font-weight: bold; min-width: 22px; height: 22px; padding: 0 6px; border-radius: 11px; background: #3b82f6; border: none; display: inline-flex; align-items: center; justify-content: center; color: #fff; box-shadow: 0 2px 4px rgba(59, 130, 246, 0.4); }

    /* Animations */
    .ce-float-icon { font-size: var(--text-2xl); animation: float-icon 3s ease-in-out infinite; }
    .ce-float-icon-sm { font-size: var(--text-xl); animation: float-icon 3s ease-in-out infinite; }
    .ce-float-icon-lg { font-size: 3rem; animation: float-icon 3s ease-in-out infinite; }

    /* Grids */
    .ce-grid-auto { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.75rem; }
    .ce-grid-2col { display: grid; grid-template-columns: auto 1fr; gap: 0.3rem 0.75rem; align-items: center; }

    /* Progress bar */
    .ce-progress-track { height: 6px; background: rgba(255, 255, 255, 0.1); border-radius: 3px; overflow: hidden; }

    /* Additional spacing */
    .ce-p-md { padding: 0.75rem; }
    .ce-p-lg { padding: 1rem; }
    .ce-mb-xs { margin-bottom: 0.25rem; }
    .ce-mb-sm { margin-bottom: 0.5rem; }
    .ce-mb-md { margin-bottom: 0.75rem; }
    .ce-mb-lg { margin-bottom: 1rem; }
    .ce-mb-xl { margin-bottom: 1.25rem; }
    .ce-mb-2xl { margin-bottom: 1.5rem; }
    .ce-my-xs { margin: 0.25rem 0; }
    .ce-heading-md { margin: 0 0 0.75rem; }

    /* Font sizes */
    .ce-text-base { font-size: 1rem; }
    .ce-text-lg { font-size: 1.1rem; }
    .ce-text-xl { font-size: var(--text-xl); }
    .ce-text-2xl { font-size: var(--text-2xl); }
    .ce-text-normal { font-weight: normal; }

    /* Display */
    .ce-block { display: block; }
    .ce-block-mb { display: block; margin-bottom: 0.3rem; }
    .ce-opacity-6 { opacity: 0.6; }

    /* Backgrounds */
    .ce-bg-1a { background: #1a1a1a; }
    .ce-bg-emerald { background: rgba(16, 185, 129, 0.15); padding: 0.5rem; border-radius: var(--radius-sm); border: 1px solid rgba(16, 185, 129, 0.3); }
    .ce-bg-section { background: rgba(30, 35, 45, 0.7); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-lg); padding: 1.25rem; margin-bottom: 1.25rem; }

    /* Checkbox */
    .ce-checkbox { width: 16px; height: 16px; accent-color: var(--context-primary, #00d4aa); }

    /* Flex variations */
    .ce-flex-center { display: flex; align-items: center; justify-content: center; }

    /* Section headers */
    .ce-section-header { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-dark-text-tertiary); margin-bottom: 0.75rem; }

    /* Grid variations */
    .ce-grid-auto-sm { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 0.75rem; }

    /* Opacity */
    .ce-opacity-5 { opacity: 0.5; }
    .ce-opacity-7 { opacity: 0.7; }
    .ce-opacity-8 { opacity: 0.8; }

    /* Cursor */
    .ce-cursor { cursor: pointer; }

    /* Additional layout */
    .ce-flex-end-wrap { display: flex; gap: 0.5rem; align-items: flex-end; flex-wrap: wrap; }
    .ce-flex-grow-min150 { flex: 1; min-width: 150px; }
    .ce-flex-grow-min200 { flex: 1; min-width: 200px; }
    .ce-whitespace-nowrap { white-space: nowrap; }
    .ce-text-truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .ce-justify-end { justify-content: flex-end; }
    .ce-w-full { width: 100%; }
    .ce-w-auto { width: auto; }
    .ce-min-h-80 { min-height: 80px; }

    /* Padding */
    .ce-p-sm { padding: 0.5rem; }
    .ce-p-3xl { padding: 3rem; }
    .ce-px-sm { padding: 0.2rem 0.5rem; }
    .ce-pl-xl { padding-left: 1.75rem; }

    /* Font sizes extra */
    .ce-text-2xl-icon { font-size: var(--text-2xl); margin-bottom: 0.25rem; opacity: 0.6; }
    .ce-text-065 { font-size: 0.65rem; }
    .ce-text-06 { font-size: 0.6rem; }
    .ce-text-095 { font-size: 0.95rem; }
    .ce-text-09 { font-size: 0.9rem; }
    .ce-text-500 { font-weight: 500; }
    .ce-lh-14 { line-height: 1.4; }
    .ce-uppercase-spaced { text-transform: uppercase; letter-spacing: 0.08em; }
    .ce-capitalize { text-transform: capitalize; }
    .ce-dim-icon { font-size: 1rem; width: 1.5rem; text-align: center; }
    .ce-italic { font-style: italic; }
    .ce-text-context-primary { color: var(--context-primary, #00d4aa); }

    /* Opacity extra */
    .ce-opacity-4 { opacity: 0.4; }

    /* Borders & separators */
    .ce-border-top-subtle { padding-top: 0.5rem; border-top: 1px solid rgba(255,255,255,0.1); }
    .ce-border-top-faint { padding-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.08); }
    .ce-border-top-purple { padding-top: 1.25rem; border-top: 1px solid rgba(139, 92, 246, 0.15); }
    .ce-border-top-muted { padding-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.05); }
    .ce-source-header { text-transform: uppercase; letter-spacing: 0.08em; padding-bottom: 0.25rem; border-bottom: 1px solid rgba(255,255,255,0.05); }

    /* Backgrounds - sections */
    .ce-bg-trust { background: var(--ctx-bg-subtle); padding: 0.75rem; border-radius: var(--radius-base); border: 1px solid var(--ctx-border-medium); }
    .ce-bg-purple-section { background: rgba(147, 51, 234, 0.1); border: 1px solid rgba(147, 51, 234, 0.3); }
    .ce-bg-purple-subtle { background: rgba(147, 51, 234, 0.1); padding: 0.5rem; border-radius: var(--radius-sm); border: 1px solid rgba(147, 51, 234, 0.2); }
    .ce-bg-indigo-section { background: rgba(99, 102, 241, 0.1); border: 1px solid rgba(99, 102, 241, 0.3); }
    .ce-bg-stats-section { background: rgba(30, 35, 45, 0.6); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-md); padding: 1rem; }

    /* Help icon (circle with ?) */
    .ce-help-icon-purple { background: rgba(147, 51, 234, 0.3); width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.85rem; }
    .ce-help-icon-indigo { background: rgba(99, 102, 241, 0.3); width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.85rem; }

    /* Badge variants */
    .ce-badge-glass { padding: 0.2rem 0.5rem; background: rgba(255,255,255,0.05); border-radius: var(--radius-sm); }
    .ce-badge-glass-bg { background: rgba(255,255,255,0.05); border-radius: var(--radius-sm); }
    .ce-badge-purple-pill { padding: 0.25rem 0.5rem; background: rgba(139, 92, 246, 0.1); border-radius: 10px; }
    .ce-badge-purple-bg { background: rgba(139, 92, 246, 0.1); }

    /* Code tag */
    .ce-code-var { background: rgba(0,0,0,0.3); padding: 0.1rem 0.35rem; border-radius: 3px; font-size: 0.7rem; color: #a78bfa; cursor: help; }

    /* Notification modal */
    .ce-notif-modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 10000; }
    .ce-notif-modal-content { background: linear-gradient(135deg, rgba(30, 32, 40, 0.98) 0%, rgba(20, 22, 28, 1) 100%); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-md); padding: 1.25rem; width: 90%; max-width: 450px; max-height: 80vh; overflow-y: auto; }
    .ce-notif-form-input { width: 100%; padding: 0.5rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-sm); color: var(--color-dark-text-primary); font-size: 0.85rem; box-sizing: border-box; }
    .ce-notif-form-select { width: 100%; padding: 0.5rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-sm); color: var(--color-dark-text-primary); font-size: 0.85rem; }
    .ce-notif-form-textarea { width: 100%; padding: 0.5rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-sm); color: var(--color-dark-text-primary); font-size: 0.85rem; resize: vertical; box-sizing: border-box; }

    /* Buttons - notification modal */
    .ce-btn-cancel { padding: 0.5rem 1rem; background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-sm); color: var(--color-dark-text-secondary); cursor: pointer; font-size: 0.85rem; }
    .ce-btn-save { padding: 0.5rem 1rem; background: var(--context-primary, #00d4aa); border: none; border-radius: var(--radius-sm); color: #000; cursor: pointer; font-size: 0.85rem; font-weight: 500; }
    .ce-btn-edit-icon { background: rgba(255,255,255,0.08); border: none; color: var(--color-dark-text-secondary); padding: 0.35rem 0.5rem; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-xs); }
    .ce-notif-card { background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: var(--radius-base); padding: 0.75rem; }
    .ce-rule-item { display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.6rem; margin: 0.25rem 0; background: rgba(255,255,255,0.03); border-radius: var(--radius-sm); border: 1px solid rgba(255,255,255,0.08); }
    .ce-rule-icon { font-size: 0.65rem; }
    .ce-operator-btn { font-size: 0.7rem; padding: 0.2rem 0.5rem; color: white; border: none; }

    /* SVG gauge */
    .ce-gauge-container { position: relative; }
    .ce-gauge-svg { transform: rotate(-90deg); }
    .ce-gauge-center { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
    .ce-gauge-center-col { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }

    /* Validation reason items */
    .ce-reason-item { display: flex; align-items: flex-start; gap: 0.75rem; padding: 0.5rem 0; }
    .ce-reason-bullet { width: 8px; height: 8px; border-radius: 50%; margin-top: 0.35rem; flex-shrink: 0; }
    .ce-priority-badge { padding: 0.15rem 0.4rem; border-radius: var(--radius-sm); font-size: 0.65rem; font-weight: 600; }
    .ce-threshold-badge { padding: 0.2rem 0.5rem; border-radius: var(--radius-sm); font-size: var(--text-xs); font-weight: 600; }
    .ce-confidence-badge { display: inline-block; padding: 0.3rem 0.75rem; border-radius: var(--radius-xl); font-size: var(--text-xs); font-weight: 600; }
    .ce-btn-correction { padding: 0.5rem 1.25rem; border-radius: 10px; font-size: 0.8rem; font-weight: 500; cursor: pointer; transition: all 0.2s ease; }
    .ce-mode-choice-btn { flex: 1; min-width: 80px; display: flex; flex-direction: column; align-items: center; gap: 0.3rem; padding: 0.75rem 0.5rem; border-radius: 10px; transition: all 0.2s ease; }

    /* Validation buttons */
    .ce-btn-reject { flex: 1; padding: 0.875rem 1.25rem; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-md); color: #ef4444; font-size: 0.9rem; font-weight: 600; cursor: pointer; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; gap: 0.5rem; }
    .ce-btn-approve { flex: 1; padding: 0.875rem 1.25rem; background: linear-gradient(135deg, rgba(34, 197, 94, 0.2) 0%, rgba(34, 197, 94, 0.1) 100%); border: 1px solid rgba(34, 197, 94, 0.4); border-radius: var(--radius-md); color: #22c55e; font-size: 0.9rem; font-weight: 600; cursor: pointer; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; gap: 0.5rem; box-shadow: 0 0 20px rgba(34, 197, 94, 0.15); }

    /* Validation details */
    .ce-validation-reasons-box { background: rgba(0, 0, 0, 0.2); border: 1px solid rgba(255,255,255,0.05); border-radius: var(--radius-md); padding: 1rem; margin: 1rem 0; }

    /* Feature card */
    .ce-feature-label { font-size: 0.65rem; margin-bottom: 0.15rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .ce-feature-conf { font-size: 0.6rem; }
    .ce-feature-card { padding: 0.5rem 0.75rem; border-radius: var(--radius-base); }
    .ce-feature-value { font-size: 0.9rem; font-weight: 600; }

    /* Animation */
    .ce-anim-card-enter { animation: card-enter 0.4s ease-out; }
    .ce-anim-slow { animation-duration: 4s; }

    /* Grid 3col */
    .ce-grid-3col { display: grid; grid-template-columns: 1fr auto auto; gap: 0.25rem 0.5rem; }

    /* Gap variants */
    .ce-gap-sm { gap: 0.5rem; }
    .ce-gap-xl { gap: 1.5rem; }

    /* Margins */
    .ce-mt-1-25 { margin-top: 1.25rem; }
    .ce-m-heading { margin: 0 0 1rem; }
    .ce-m-para { margin: 0.5rem 0 0.75rem; }
    .ce-m-para-sm { margin: 0.5rem 0 0.5rem; }
    .ce-m-para-top { margin: 0.5rem 0 0; }
    .ce-m-para-mid { margin: 0.5rem 0; }
    .ce-m-rule-heading { margin: 1rem 0 0.5rem; }

    /* Inline select */
    .ce-select-inline { width: auto; margin-left: 0.5rem; }

    /* Intelligence correction panel */
    .ce-correction-panel { background: rgba(139, 92, 246, 0.08); border: 1px solid rgba(139, 92, 246, 0.2); border-radius: var(--radius-md); animation: card-enter 0.2s ease-out; }

    /* Intelligence prediction section */
    .ce-intelligence-section { background: linear-gradient(135deg, rgba(139, 92, 246, 0.12) 0%, rgba(109, 40, 217, 0.06) 100%); border: 1px solid rgba(139, 92, 246, 0.25); border-radius: var(--radius-lg); padding: 1.5rem; margin-bottom: 1.25rem; box-shadow: 0 4px 24px rgba(139, 92, 246, 0.1); }
  `]

  static properties = {
    activeTab: { type: String },
    contextState: { type: Object },
    automations: { type: Array },
    automationHistory: { type: Array },
    validations: { type: Array },
    stats: { type: Object },
    // patterns removed - now managed by Intelligence tab
    config: { type: Object },
    schema: { type: Object },
    loading: { type: Boolean },
    selectedDuration: { type: Number },
    showForm: { type: Boolean },
    editingAutomation: { type: Object },
    // Action form state
    showingActionConfig: { type: Boolean },
    pendingActionType: { type: String },
    pendingAction: { type: Object },
    // Condition form state
    showingConditionConfig: { type: Boolean },
    pendingConditionType: { type: String },
    pendingCondition: { type: Object },
    pendingConditionPath: { type: Array },
    // Trigger form state (for multiple triggers with AND/OR)
    showingTriggerConfig: { type: Boolean },
    pendingTriggerType: { type: String },
    pendingTrigger: { type: Object },
    pendingTriggerPath: { type: Array },
    // Config help toggle
    showConfigHelp: { type: Boolean },
    // Modes dynamiques
    modes: { type: Array },
    showModeForm: { type: Boolean },
    editingMode: { type: Object },
    modeFormData: { type: Object },
    // Timeline highlighting
    highlightedAutomationId: { type: String },
    categoryFilter: { type: String },
    // Intelligence v2 data
    intelligenceFeatures: { type: Object },
    intelligenceVector: { type: Object },
    intelligencePrediction: { type: Object },
    // Prediction correction
    showPredictionCorrection: { type: Boolean },
    predictionCorrectionSent: { type: Boolean },
    // UX Overlay states
    toasts: { type: Array },
    confirmDialog: { type: Object },
    modeChangeOverlay: { type: Object },
  }

  constructor() {
    super()
    this.activeTab = 'modes'
    this.contextState = null
    this.automations = []
    this.automationHistory = []
    this.validations = []
    this.stats = null
    // patterns removed - now managed by Intelligence tab
    this.config = {
      impact_thresholds: { low: 0.3, medium: 0.5, high: 0.7, very_high: 0.9 },
      initial_trust_score: 0.5
    }
    this.schema = null
    this.loading = true
    this.selectedDuration = 60
    this.showForm = false
    this.editingAutomation = null
    // Action form state
    this.showingActionConfig = false
    this.pendingActionType = 'send_notification'
    this.pendingAction = null
    // Condition form state
    this.showingConditionConfig = false
    this.pendingConditionType = 'current_mode'
    this.pendingCondition = null
    this.pendingConditionPath = null
    // Trigger form state
    this.showingTriggerConfig = false
    this.pendingTriggerType = 'mode_change'
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showConfigHelp = false
    // Modes dynamiques
    this.modes = []
    this.showModeForm = false
    this.editingMode = null
    this.modeFormData = {
      name: '',
      icon: '🎯',
      theme: { primary: '#2563eb', background: '#f8fafc', accent: '#1e40af' }
    }
    // Category filter for automations
    this.categoryFilter = 'all'
    // Timeline highlighting
    this.highlightedAutomationId = null
    // Intelligence v2 data
    this.intelligenceFeatures = null
    this.intelligenceVector = null
    this.intelligencePrediction = null
    // Prediction correction
    this.showPredictionCorrection = false
    this.predictionCorrectionSent = false
    // UX Overlay states
    this.toasts = []
    this.confirmDialog = null
    this.modeChangeOverlay = null
  }

  connectedCallback() {
    super.connectedCallback()
    this.loadAllData()
    document.addEventListener('keydown', this._handleKeydown = (e) => {
      if (e.key === 'Escape') this.close()
    })

    // Écouter les nouvelles notifications pour rafraîchir les validations
    this._notificationHandler = (e) => {
      const notif = e.detail?.notification
      // Si c'est une notification de validation, rafraîchir
      if (notif?.title?.includes('Validation') || notif?.title?.includes('validation')) {
        console.log('[context-engine] Validation notification received, refreshing...')
        // [Audit] Add .catch() for fire-and-forget promises
        this.loadValidations().catch(e => console.warn('[context-engine] Refresh validations failed:', e))
        this.loadAutomations().catch(e => console.warn('[context-engine] Refresh automations failed:', e))
      }
    }
    document.body.addEventListener('notification-received', this._notificationHandler)

    // Rafraîchir périodiquement les validations (toutes les 10s)
    this._refreshInterval = setInterval(() => {
      if (this.activeTab === 'validations') {
        // [Audit] Add .catch() for fire-and-forget promise
        this.loadValidations().catch(e => console.warn('[context-engine] Periodic refresh failed:', e))
      }
    }, 10000)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this._handleKeydown)
    if (this._notificationHandler) {
      document.body.removeEventListener('notification-received', this._notificationHandler)
    }
    if (this._refreshInterval) {
      clearInterval(this._refreshInterval)
    }
  }

  async loadAllData() {
    this.loading = true
    try {
      await Promise.all([
        this.loadContext(),
        this.loadAutomations(),
        this.loadValidations(),
        this.loadStats(),
        this.loadConfig(),
        this.loadModes(),
      ])
    } catch (e) {
      console.error('[context-engine] Failed to load data:', e)
    }
    this.loading = false
  }

  async loadContext() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.contextState = await apiService.request('/v1/context/current')
    } catch (e) {
      console.error('[context-engine] Failed to load context:', e)
    }
  }

  async loadAutomations() {
    try {
      this.automations = await automationsService.fetchAutomations()
      this.automationHistory = await automationsService.fetchHistory(20)
      this.schema = await automationsService.fetchSchema()
    } catch (e) {
      console.error('[context-engine] Failed to load automations:', e)
    }
  }

  async loadValidations() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.validations = await apiService.request('/v1/decision/validations/pending')
      if (!Array.isArray(this.validations)) this.validations = []
    } catch (e) {
      console.error('[context-engine] Failed to load validations:', e)
      this.validations = []
    }
  }

  async loadStats() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      // Note: patterns endpoint removed - now managed by Intelligence tab
      this.stats = await apiService.request('/v1/context/stats')
    } catch (e) {
      console.error('[context-engine] Failed to load stats:', e)
    }
  }

  async loadConfig() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.config = await apiService.request('/v1/decision/config')
    } catch (e) {
      // Config endpoint might not exist yet - use defaults
      console.log('[context-engine] Using default config')
    }
  }

  async loadModes() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.modes = await apiService.request('/v1/modes')
      if (!Array.isArray(this.modes)) this.modes = []
    } catch (e) {
      console.error('[context-engine] Failed to load modes:', e)
      this.modes = []
    }
  }



  close() {
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }))
  }

  // ============ Toast Notifications ============
  showToast(message, type = 'info', duration = 3000) {
    const toast = {
      id: Date.now(),
      message,
      type,
      icon: type === 'success' ? '✓' : type === 'error' ? '✗' : 'ℹ'
    }
    this.toasts = [...this.toasts, toast]

    // Auto-remove after duration
    setTimeout(() => {
      this.removeToast(toast.id)
    }, duration)
  }

  removeToast(id) {
    // First mark as leaving for animation
    const toastEl = this.shadowRoot?.querySelector(`[data-toast-id="${id}"]`)
    if (toastEl) {
      toastEl.classList.add('leaving')
      setTimeout(() => {
        this.toasts = this.toasts.filter(t => t.id !== id)
      }, 250)
    } else {
      this.toasts = this.toasts.filter(t => t.id !== id)
    }
  }

  // ============ Confirmation Dialog ============
  showConfirmDialog(options) {
    return new Promise((resolve) => {
      this.confirmDialog = {
        icon: options.icon || '⚠️',
        title: options.title || 'Confirmer',
        message: options.message || 'Êtes-vous sûr ?',
        confirmLabel: options.confirmLabel || 'Confirmer',
        cancelLabel: options.cancelLabel || 'Annuler',
        confirmClass: options.confirmClass || 'btn-danger',
        resolve
      }
    })
  }

  handleConfirm(confirmed) {
    if (this.confirmDialog?.resolve) {
      this.confirmDialog.resolve(confirmed)
    }
    this.confirmDialog = null
  }



  // Validation actions
  async approveValidation(id) {
    try {
      const res = await csrfService.fetchWithCsrf(`/v1/decision/validation/${id}/resolve`, {
        method: 'POST',
        body: JSON.stringify({ approved: true, username: 'user' })
      })
      if (res.ok) {
        await this.loadValidations()
      }
    } catch (e) {
      console.error('[context-engine] Failed to approve validation:', e)
    }
  }

  async rejectValidation(id) {
    try {
      const res = await csrfService.fetchWithCsrf(`/v1/decision/validation/${id}/resolve`, {
        method: 'POST',
        body: JSON.stringify({ approved: false, username: 'user' })
      })
      if (res.ok) {
        await this.loadValidations()
      }
    } catch (e) {
      console.error('[context-engine] Failed to reject validation:', e)
    }
  }

  // Config actions
  async saveConfig() {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/decision/config', {
        method: 'PUT',
        body: JSON.stringify(this.config)
      })
      if (res.ok) {
        this.showToast('Configuration sauvegardée', 'success')
      } else {
        this.showToast('Erreur lors de la sauvegarde', 'error')
      }
    } catch (e) {
      console.error('[context-engine] Failed to save config:', e)
      this.showToast('Erreur lors de la sauvegarde', 'error')
    }
  }

  // Helpers - use dynamic modes from mode_registry
  getModeIcon(modeSlug) {
    // First try to find in dynamic modes
    const slug = modeSlug?.toLowerCase()
    const dynamicMode = this.modes.find(m => m.slug === slug)
    if (dynamicMode) return dynamicMode.icon

    // Fallback for legacy mode names
    const legacyIcons = { cravate: '👔', intime: '🏡', neutre: '🌱', pro: '👔', focus: '🎯', maison: '🏡', veille: '🌱' }
    return legacyIcons[slug] || '🌱'
  }

  getModeName(modeSlug) {
    // First try to find in dynamic modes
    const slug = modeSlug?.toLowerCase()
    const dynamicMode = this.modes.find(m => m.slug === slug)
    if (dynamicMode) return dynamicMode.name

    // Fallback for legacy mode names
    const legacyNames = { cravate: 'Focus Pro', intime: 'Maison', neutre: 'Veille', pro: 'Pro', focus: 'Focus', maison: 'Maison', veille: 'Veille' }
    return legacyNames[slug] || 'Inconnu'
  }

  // Get mode theme (for styling)
  getModeTheme(modeSlug) {
    const slug = modeSlug?.toLowerCase()
    const dynamicMode = this.modes.find(m => m.slug === slug)
    return dynamicMode?.theme || { primary: '#6b7280', background: '#f9fafb', accent: '#4b5563' }
  }

  getTrustClass(score) {
    if (score >= 0.7) return 'high'
    if (score >= 0.4) return 'medium'
    return 'low'
  }

  formatTime(timestamp) {
    if (!timestamp) return 'Jamais'
    const date = new Date(timestamp)
    const now = new Date()
    const diff = (now - date) / 1000
    if (diff < 60) return "À l'instant"
    if (diff < 3600) return `Il y a ${Math.floor(diff / 60)} min`
    if (diff < 86400) return `Il y a ${Math.floor(diff / 3600)}h`
    return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  }

  formatDuration(minutes) {
    if (minutes < 60) return `${minutes}min`
    const h = Math.floor(minutes / 60)
    const m = minutes % 60
    return m > 0 ? `${h}h${m}` : `${h}h`
  }

  // Render methods
  render() {
    return html`
      <div class="page" @click="${e => e.stopPropagation()}">
        <div class="header">
          <span class="header-title">🧠 Decision Engine</span>
          <button class="close-button" @click="${this.close}" aria-label="Fermer">✕</button>
        </div>

        <div class="tabs">
          ${this.renderTab('modes', 'Modes')}
          ${this.renderTab('intelligence', 'Intelligence')}
          ${this.renderTab('automations', 'Automations')}
          ${this.renderTab('validations', 'Validations', this.validations.length)}
          ${this.renderTab('config', 'Config')}
        </div>

        <div class="content">
          ${this.loading ? this.renderSkeletonLoading() : this.renderActiveTab()}
        </div>
      </div>

      <!-- Toast Notifications -->
      ${this.toasts.length > 0 ? html`
        <div class="toast-container">
          ${this.toasts.map(t => html`
            <div class="toast ${t.type}" data-toast-id="${t.id}">
              <span class="toast-icon">${t.icon}</span>
              <span class="toast-message">${t.message}</span>
            </div>
          `)}
        </div>
      ` : ''}

      <!-- Confirmation Dialog -->
      ${this.confirmDialog ? html`
        <div class="confirm-overlay" @click="${() => this.handleConfirm(false)}" @keydown="${e => e.key === 'Escape' && this.handleConfirm(false)}">
          <div class="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" aria-describedby="confirm-message" @click="${e => e.stopPropagation()}">
            <div class="confirm-icon" aria-hidden="true">${this.confirmDialog.icon}</div>
            <h2 class="confirm-title" id="confirm-title">${this.confirmDialog.title}</h2>
            <p class="confirm-message" id="confirm-message">${this.confirmDialog.message}</p>
            <div class="confirm-actions">
              <button class="btn" @click="${() => this.handleConfirm(false)}">
                ${this.confirmDialog.cancelLabel}
              </button>
              <button class="btn ${this.confirmDialog.confirmClass}" @click="${() => this.handleConfirm(true)}">
                ${this.confirmDialog.confirmLabel}
              </button>
            </div>
          </div>
        </div>
      ` : ''}

      <!-- Mode Change Overlay -->
      ${this.modeChangeOverlay ? html`
        <div class="mode-change-overlay">
          <div class="mode-change-content">
            <div class="mode-change-icon">${this.modeChangeOverlay.icon}</div>
            <div class="mode-change-name">${this.modeChangeOverlay.name}</div>
            <div class="mode-change-message">Activé pour ${this.formatDuration(this.modeChangeOverlay.duration)}</div>
          </div>
        </div>
      ` : ''}
    `
  }

  renderSkeletonLoading() {
    return html`
      <div class="skeleton-loading">
        <div class="skeleton skeleton-text"></div>
        <div class="skeleton skeleton-text short"></div>
        <div class="skeleton skeleton-card"></div>
        <div class="skeleton skeleton-card"></div>
      </div>
    `
  }

  renderTab(id, label, badge = 0) {
    return html`
      <button
        class="tab ${this.activeTab === id ? 'active' : ''}"
        @click="${() => this.switchTab(id)}"
      >
        ${label}
        ${badge > 0 ? html`<span class="badge">${badge}</span>` : ''}
      </button>
    `
  }

  switchTab(id) {
    this.activeTab = id
    // Rafraîchir les données de l'onglet sélectionné
    // [Audit] Add .catch() for fire-and-forget promises
    switch (id) {
      case 'validations':
        this.loadValidations().catch(e => console.warn('[context-engine] Load validations failed:', e))
        break
      case 'automations':
        this.loadAutomations().catch(e => console.warn('[context-engine] Load automations failed:', e))
        break
      case 'modes':
        this.loadModes().catch(e => console.warn('[context-engine] Load modes failed:', e))
        this.loadContext().catch(e => console.warn('[context-engine] Load context failed:', e))
        break
      case 'intelligence':
        this.loadIntelligence().catch(e => console.warn('[context-engine] Load intelligence failed:', e))
        break
    }
  }

  renderActiveTab() {
    switch (this.activeTab) {
      case 'modes': return this.renderModesTab()
      case 'intelligence': return this.renderIntelligenceTab()
      case 'automations': return this.renderAutomationsTab()
      case 'validations': return this.renderValidationsTab()
      case 'config': return this.renderConfigTab()
      default: return html`<div>Onglet inconnu</div>`
    }
  }



  renderValidationsTab() {
    if (this.validations.length === 0) {
      return html`
        <div class="empty-state">
          <div class="empty-icon">✓</div>
          <div class="empty-text">Aucune validation en attente</div>
        </div>
      `
    }

    return html`
      <div class="controls-title ce-flex ce-gap-md ce-mb-lg">
        <span class="ce-float-icon-sm">⚖️</span>
        <span>Demandes en attente</span>
        <span class="ce-badge-orange ce-text-bold">
          ${this.validations.length}
        </span>
      </div>
      ${this.validations.map((v, i) => this.renderValidationCard(v, i))}
    `
  }

  renderValidationCard(v, index = 0) {
    const trustScore = v.trust_score || 0
    const trustClass = this.getTrustClass(trustScore)
    const actionType = v.action?.action_type || 'Action'
    const reasons = v.human_reasons || ['Validation requise']
    const color = trustClass === 'high' ? '#22c55e' : trustClass === 'medium' ? '#f59e0b' : '#ef4444'
    const glowColor = trustClass === 'high' ? 'rgba(34, 197, 94, 0.4)' : trustClass === 'medium' ? 'rgba(245, 158, 11, 0.4)' : 'rgba(239, 68, 68, 0.4)'

    return html`
      <div class="validation-card" style="
        --validation-color: ${color};
        animation: card-enter 0.4s ease-out ${index * 0.1}s backwards;
        background: linear-gradient(135deg, rgba(255, 255, 255, 0.04) 0%, var(--surface-glass-faint) 100%);
        border: 1px solid ${trustClass === 'high' ? 'rgba(34, 197, 94, 0.2)' : trustClass === 'medium' ? 'rgba(245, 158, 11, 0.2)' : 'rgba(239, 68, 68, 0.2)'};
      ">
        <div class="validation-header">
          <!-- Mini Gauge -->
          <div class="ce-flex-shrink-0">
            ${this.renderMiniGauge(trustScore, 70)}
          </div>

          <div class="validation-info ce-flex-min-0">
            <div class="ce-flex ce-mb-sm">
              <span class="ce-text-xl">⚡</span>
              <div class="validation-title ce-text-bold ce-text-primary ce-text-lg">${actionType}</div>
            </div>
            <div class="validation-subtitle ce-flex ce-text-md ce-text-secondary">
              <span class="ce-text-xs ce-badge-glass">
                🤖 ${v.action?.agent_id || 'Système'}
              </span>
              <span class="ce-threshold-badge" style="background: ${glowColor}; color: ${color};">
                Seuil: ${Math.round((v.threshold || 0.7) * 100)}%
              </span>
            </div>
          </div>
        </div>

        <div class="validation-reasons ce-validation-reasons-box">
          <div class="validation-reasons-title ce-flex ce-text-tiny ce-text-bold ce-text-tertiary ce-mb-md ce-uppercase-spaced">
            📋 Raisons de la validation
          </div>
          ${reasons.map((r, i) => html`
            <div class="validation-reason-item ce-reason-item" style="animation: card-enter 0.3s ease-out ${(index * 0.1) + (i * 0.05)}s backwards;">
              <div class="ce-reason-bullet" style="background: ${color}; box-shadow: 0 0 8px ${glowColor};"></div>
              <span class="ce-text-sm-secondary ce-lh-14">${r}</span>
            </div>
          `)}
        </div>

        <div class="validation-actions ce-flex ce-gap-lg">
          <button class="ce-btn-reject" @click="${() => this.handleRejectValidation(v.validation_id)}">
            <span class="ce-text-lg">✗</span> Rejeter
          </button>
          <button class="ce-btn-approve" @click="${() => this.handleApproveValidation(v.validation_id)}">
            <span class="ce-text-lg">✓</span> Approuver
          </button>
        </div>
      </div>
    `
  }

  renderMiniGauge(value, size = 60) {
    const percentage = Math.round(value * 100)
    const radius = 22
    const circumference = 2 * Math.PI * radius
    const offset = circumference - (value * circumference)
    const color = value >= 0.7 ? '#22c55e' : value >= 0.4 ? '#f59e0b' : '#ef4444'

    return html`
      <div class="ce-gauge-container" style="width: ${size}px; height: ${size}px;">
        <svg class="ce-gauge-svg" width="${size}" height="${size}" viewBox="0 0 50 50">
          <circle cx="25" cy="25" r="${radius}" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="5"/>
          <circle cx="25" cy="25" r="${radius}" fill="none" stroke="${color}" stroke-width="5"
            stroke-linecap="round"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            style="transition: stroke-dashoffset 0.8s ease-out; filter: drop-shadow(0 0 6px ${color}80);"/>
        </svg>
        <div class="ce-gauge-center">
          <span style="font-size: ${size / 4}px; font-weight: 700; color: ${color};">${percentage}%</span>
        </div>
      </div>
    `
  }

  async handleApproveValidation(id) {
    await this.approveValidation(id)
    this.showToast('Action approuvée', 'success')
  }

  async handleRejectValidation(id) {
    const confirmed = await this.showConfirmDialog({
      icon: '🚫',
      title: 'Rejeter cette action ?',
      message: 'L\'action sera annulée et l\'agent sera notifié du rejet.',
      confirmLabel: 'Rejeter',
      cancelLabel: 'Annuler',
      confirmClass: 'btn-danger'
    })

    if (!confirmed) return

    await this.rejectValidation(id)
    this.showToast('Action rejetée', 'info')
  }



  getDayNameShort(day) {
    // Use centralized ISO convention (0=Monday from kernel)
    return getDayNameShort(day)
  }

  getDayNameFull(day) {
    // Use centralized ISO convention (0=Monday from kernel)
    return getDayNameFull(day)
  }

  renderConfigTab() {
    return html`
      <!-- Help Section (collapsible) -->
      <div class="config-section ce-bg-indigo-section">
        <div class="config-title ce-flex ce-cursor-pointer"
          @click="${() => this.showConfigHelp = !this.showConfigHelp}">
          <span class="ce-help-icon-indigo">?</span>
          Comment ça marche ?
          <span class="ce-ml-auto ce-text-xs ce-opacity-6">${this.showConfigHelp ? '▼' : '▶'}</span>
        </div>
        ${this.showConfigHelp ? html`
        <div class="ce-text-sm-secondary ce-mt-md">

          <p class="ce-m-para"><strong>1. Calcul du Trust Score</strong> (0.0 à 1.0) :</p>
          <div class="ce-bg-dark ce-text-xs ce-heading-md">
            <p class="ce-heading ce-text-normal">Le système évalue 5 critères et fait la moyenne pondérée :</p>
            <div class="ce-grid-3col">
              <span>• Mode & SSID correspondent ?</span><span class="ce-text-indigo">25%</span><span class="ce-text-tertiary">→ 0 ou 1</span>
              <span>• Agent en ligne, CPU/RAM ok ?</span><span class="ce-text-indigo">25%</span><span class="ce-text-tertiary">→ 0 ou 1</span>
              <span>• Action pas expirée ?</span><span class="ce-text-indigo">20%</span><span class="ce-text-tertiary">→ 0 à 1</span>
              <span>• Historique de succès</span><span class="ce-text-indigo">15%</span><span class="ce-text-tertiary">→ 0 à 1</span>
              <span>• Tes approbations passées</span><span class="ce-text-indigo">15%</span><span class="ce-text-tertiary">→ 0 à 1</span>
            </div>
            <p class="ce-m-para-top ce-italic">Score max = 1.0 si tout est parfait.</p>
          </div>

          <p class="ce-m-para-sm"><strong>2. Comment choisir les seuils ?</strong></p>
          <div class="ce-bg-dark ce-text-xs ce-heading-md">
            <p class="ce-heading ce-text-normal">Chaque type d'action utilise un seuil selon son niveau d'impact :</p>
            <div class="ce-grid-2col">
              <span class="ce-text-emerald">Low</span><span>→ Notifications (ex: "Tu as reçu un email")</span>
              <span class="ce-text-blue">Medium</span><span>→ Changements de mode, ajustements légers</span>
              <span class="ce-text-amber">High</span><span>→ Contrôle d'appareils (allumer/éteindre PC)</span>
              <span class="ce-text-red">Very High</span><span>→ Actions critiques ou irréversibles</span>
            </div>
          </div>

          <p class="ce-m-para-mid"><strong>3. Règle simple pour configurer :</strong></p>
          <div class="ce-bg-dark ce-text-xs ce-heading-md">
            <p class="ce-heading ce-text-normal"><strong>Seuil bas (0.3-0.4)</strong> = Peu exigeant, s'exécute souvent seul</p>
            <p class="ce-heading ce-text-normal"><strong>Seuil moyen (0.5-0.6)</strong> = Équilibré, vérifie le contexte</p>
            <p class="ce-heading ce-text-normal"><strong>Seuil haut (0.7-0.8)</strong> = Strict, demande validation si doute</p>
            <p class="ce-m-0"><strong>Seuil très haut (0.9+)</strong> = Quasi toujours validation manuelle</p>
          </div>

          <div class="ce-bg-emerald">
            <p class="ce-m-0 ce-text-xs">
              <strong>💡 Exemple concret :</strong><br>
              Tu as High = 0.7. Une automation "Allumer PC" calcule un score de 0.66<br>
              → <span class="ce-text-amber">0.66 < 0.7</span> = demande ta validation<br>
              Si tu baisses à 0.6, cette même action passera automatiquement.
            </p>
          </div>
        </div>
        ` : ''}
      </div>

      <div class="config-section">
        <div class="config-title">Seuils Trust Score</div>

        <div class="config-row">
          <span class="config-label">Low (notifications)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.low || 0.3}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, low: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">Medium (mode changes)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.medium || 0.5}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, medium: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">High (agent commands)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.high || 0.7}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, high: parseFloat(e.target.value)}}}"
          >
        </div>

        <div class="config-row">
          <span class="config-label">Very High (shutdown/restart)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.impact_thresholds?.very_high || 0.9}"
            @change="${e => this.config = {...this.config, impact_thresholds: {...this.config.impact_thresholds, very_high: parseFloat(e.target.value)}}}"
          >
        </div>
      </div>

      <div class="config-section">
        <div class="config-title">Trust Initial</div>
        <div class="config-row">
          <span class="config-label">Score initial (nouvelles actions)</span>
          <input type="number" class="config-input" min="0" max="1" step="0.1"
            .value="${this.config.initial_trust_score || 0.5}"
            @change="${e => this.config = {...this.config, initial_trust_score: parseFloat(e.target.value)}}"
          >
        </div>
        <div class="ce-text-xs-secondary ce-mt-sm">
          Score attribué aux nouvelles automations sans historique
        </div>
      </div>

      <button class="btn btn-primary ce-mt-lg ce-w-full" @click="${this.saveConfig}">
        💾 Sauvegarder
      </button>
    `
  }


}

customElements.define('context-engine-page', ContextEnginePage)

export { ContextEnginePage }
