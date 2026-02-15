/**
 * Context Engine Page - Page Unifiée
 *
 * Fusionne Context Engine + Automations + Validations + Stats + Config
 * en une seule interface cohérente.
 */

import { LitElement, html, css } from 'lit'
import csrfService from '../services/csrf-service.js'
import automationsService from '../services/automations-service.js'
import { getDayNameShort, getDayNameFull, getAllDayNamesShort, utcHourToLocal } from '../utils/time-utils.js'

// Import du composant timeline (utilisé dans le template)
import './automation-timeline.js'

// Classification des types de règles
// Événements = déclencheurs (triggers) - provoquent l'exécution
const EVENT_TYPES = ['mode_change', 'sensor_alert', 'agent_status', 'manual', 'plugin_health', 'scheduled']
// États = conditions - vérifient l'état actuel
const STATE_TYPES = ['current_mode', 'time_range', 'day_of_week', 'day_of_month', 'month', 'sensor_value', 'agent_online']

function isEventType(type) {
  return EVENT_TYPES.includes(type)
}

function isStateType(type) {
  return STATE_TYPES.includes(type)
}

class ContextEnginePage extends LitElement {
  static styles = css`
    :host {
      position: fixed;
      inset: 0;
      z-index: 9999;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      animation: fadeIn 0.2s ease-out;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    .page {
      width: 95%;
      max-width: 800px;
      max-height: 90vh;
      background: linear-gradient(135deg, rgba(19, 20, 26, 0.98) 0%, rgba(10, 10, 11, 1) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent);
      border-radius: 16px;
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6),
                  0 0 80px color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent);
      overflow: hidden;
      display: flex;
      flex-direction: column;
      animation: scaleIn 0.25s ease-out;
    }

    @keyframes scaleIn {
      from { opacity: 0; transform: scale(0.95); }
      to { opacity: 1; transform: scale(1); }
    }

    @keyframes slideUp {
      from { opacity: 0; transform: translateY(20px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @keyframes slideDown {
      from { opacity: 1; transform: translateY(0); }
      to { opacity: 0; transform: translateY(20px); }
    }

    @keyframes pulse-ring {
      0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--pulse-color), 0.7); }
      70% { transform: scale(1); box-shadow: 0 0 0 10px rgba(var(--pulse-color), 0); }
      100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--pulse-color), 0); }
    }

    @keyframes shimmer {
      0% { background-position: -200% 0; }
      100% { background-position: 200% 0; }
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
      border-radius: 12px;
      background: rgba(25, 26, 32, 0.95);
      border: 1px solid rgba(255, 255, 255, 0.1);
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
      backdrop-filter: blur(8px);
      animation: fadeIn 0.2s ease-out;
    }

    .confirm-dialog {
      background: linear-gradient(135deg, rgba(25, 26, 32, 0.98) 0%, rgba(15, 15, 17, 1) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 16px;
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
      font-size: 2rem;
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
        rgba(255, 255, 255, 0.03) 25%,
        rgba(255, 255, 255, 0.08) 50%,
        rgba(255, 255, 255, 0.03) 75%);
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
      border-radius: 8px;
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
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 16px;
      padding: 1.25rem;
      margin-bottom: 1rem;
      transition: all 0.3s ease;
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
      background: rgba(255, 255, 255, 0.05);
      border-color: rgba(255, 255, 255, 0.15);
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
      font-size: 2rem;
      padding: 0.5rem;
      background: rgba(245, 158, 11, 0.15);
      border-radius: 12px;
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
      background: rgba(255, 255, 255, 0.03);
      border-radius: 8px;
    }

    .validation-trust-bar {
      width: 60px;
      height: 6px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 3px;
      overflow: hidden;
    }

    .validation-trust-fill {
      height: 100%;
      border-radius: 3px;
      transition: width 0.5s ease;
    }

    .validation-trust-fill.high { background: linear-gradient(90deg, #22c55e, #4ade80); }
    .validation-trust-fill.medium { background: linear-gradient(90deg, #f59e0b, #fbbf24); }
    .validation-trust-fill.low { background: linear-gradient(90deg, #ef4444, #f87171); }

    .validation-reasons {
      background: rgba(255, 255, 255, 0.02);
      border-radius: 8px;
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
      border-radius: 10px;
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
      padding: 1rem 1.25rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(0, 0, 0, 0.3);
    }

    .header-title {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .close-btn {
      background: rgba(255, 255, 255, 0.08);
      border: none;
      color: var(--color-dark-text-secondary, #adb5bd);
      width: 36px;
      height: 36px;
      border-radius: 50%;
      cursor: pointer;
      font-size: 1.2rem;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .close-btn:hover {
      background: rgba(255, 255, 255, 0.15);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Tabs */
    .tabs {
      display: flex;
      gap: 0.25rem;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      background: rgba(0, 0, 0, 0.2);
      overflow-x: auto;
    }

    .tab {
      padding: 0.5rem 1rem;
      border-radius: 8px;
      background: transparent;
      border: 1px solid transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }

    .tab:hover {
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .tab.active {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .tab .badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 18px;
      height: 18px;
      padding: 0 5px;
      margin-left: 6px;
      border-radius: 9px;
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

    .content::-webkit-scrollbar {
      width: 6px;
    }

    .content::-webkit-scrollbar-track {
      background: transparent;
    }

    .content::-webkit-scrollbar-thumb {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
      border-radius: 3px;
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

    @keyframes float {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-8px); }
    }

    .mode-name {
      font-size: 1.5rem;
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
      background: rgba(255, 255, 255, 0.1);
      border-radius: 4px;
      margin: 0 auto 0.5rem;
      overflow: hidden;
    }

    .confidence-fill {
      height: 100%;
      background: linear-gradient(90deg, var(--context-primary, #00d4aa), color-mix(in srgb, var(--context-primary, #00d4aa) 70%, white));
      border-radius: 4px;
      transition: width 0.5s ease;
    }

    .confidence-text {
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .override-info {
      margin: 1.5rem 0;
      padding: 0.75rem 1rem;
      background: rgba(251, 146, 60, 0.1);
      border: 1px solid rgba(251, 146, 60, 0.3);
      border-radius: 8px;
      color: #fb923c;
      font-size: 0.8rem;
    }

    /* Mode Controls */
    .mode-controls {
      margin-top: 2rem;
      padding-top: 1.5rem;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    .controls-title {
      font-size: 0.75rem;
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
      border-radius: 10px;
      border: 1px solid rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.85rem;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .mode-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.25);
      transform: translateY(-2px);
    }

    .mode-btn.cravate:hover { border-color: #3b82f6; }
    .mode-btn.intime:hover { border-color: #00d4aa; }
    .mode-btn.neutre:hover { border-color: #6b7280; }

    .duration-buttons {
      display: flex;
      gap: 0.5rem;
      justify-content: center;
      margin-bottom: 1rem;
    }

    .duration-btn {
      padding: 0.4rem 0.8rem;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      background: transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.75rem;
      cursor: pointer;
      transition: all 0.2s;
    }

    .duration-btn:hover, .duration-btn.active {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .clear-override-btn {
      padding: 0.5rem 1rem;
      border-radius: 8px;
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

    /* Cards */
    .card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 12px;
      padding: 1rem;
      margin-bottom: 0.75rem;
      transition: all 0.2s;
    }

    .card:hover {
      border-color: rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
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
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary, #6c757d);
    }

    .card-actions {
      display: flex;
      gap: 0.5rem;
    }

    /* Buttons */
    .btn {
      padding: 0.5rem 1rem;
      border-radius: 8px;
      border: 1px solid transparent;
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
    }

    .btn-primary {
      background: linear-gradient(135deg, color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 10%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 40%, transparent);
      color: var(--context-primary, #00d4aa);
    }

    .btn-primary:hover {
      background: linear-gradient(135deg, color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent) 0%, color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent) 100%);
      transform: translateY(-1px);
    }

    .btn-success {
      background: rgba(34, 197, 94, 0.15);
      border-color: rgba(34, 197, 94, 0.4);
      color: #22c55e;
    }

    .btn-success:hover {
      background: rgba(34, 197, 94, 0.25);
    }

    .btn-danger {
      background: rgba(239, 68, 68, 0.15);
      border-color: rgba(239, 68, 68, 0.4);
      color: #ef4444;
    }

    .btn-danger:hover {
      background: rgba(239, 68, 68, 0.25);
    }

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

    /* Toggle */
    .toggle {
      position: relative;
      width: 40px;
      height: 22px;
      background: rgba(255, 255, 255, 0.15);
      border-radius: 11px;
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
      border-radius: 6px;
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
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.03) 0%, rgba(255, 255, 255, 0.01) 100%);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 16px;
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
      transition: all 0.3s ease;
    }

    .automation-card.enabled::before {
      background: var(--context-primary, #00d4aa);
    }

    .automation-card.disabled::before {
      background: rgba(239, 68, 68, 0.5);
    }

    .automation-card:hover {
      border-color: rgba(255, 255, 255, 0.15);
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
      transform: translateY(-2px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    }

    .automation-card.highlighted {
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 1px var(--context-primary, #00d4aa),
                  0 8px 32px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
    }

    .automation-card.add-new {
      border-style: dashed;
      border-color: rgba(255, 255, 255, 0.15);
      background: transparent;
    }

    .automation-card.add-new::before {
      display: none;
    }

    .automation-card.add-new:hover {
      border-color: var(--context-primary, #00d4aa);
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent);
    }

    .automation-card.add-new:hover div {
      color: var(--context-primary, #00d4aa) !important;
      opacity: 1 !important;
    }

    .automation-card-inner {
      padding: 1rem 1rem 1rem 1.25rem;
    }

    .automation-header {
      display: flex;
      align-items: flex-start;
      gap: 0.75rem;
      margin-bottom: 0.75rem;
    }

    .automation-status-icon {
      width: 40px;
      height: 40px;
      border-radius: 12px;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 1.25rem;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.08);
      flex-shrink: 0;
    }

    .automation-card.enabled .automation-status-icon {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
    }

    .automation-info {
      flex: 1;
      min-width: 0;
    }

    .automation-title-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.25rem;
    }

    .automation-title {
      font-size: 0.95rem;
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .automation-category-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding: 0.15rem 0.5rem;
      border-radius: 6px;
      font-size: 0.65rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      white-space: nowrap;
      background: rgba(255, 255, 255, 0.08);
      color: var(--color-dark-text-secondary, #adb5bd);
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
      border-radius: 4px;
      font-size: 0.7rem;
      background: rgba(0, 212, 170, 0.15);
      color: #00d4aa;
      animation: glow-pulse 2s ease-in-out infinite;
    }

    .automation-subtitle {
      font-size: 0.8rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-wrap: wrap;
    }

    .automation-actions {
      display: flex;
      gap: 0.5rem;
      flex-shrink: 0;
    }

    .automation-details {
      display: flex;
      align-items: center;
      gap: 1rem;
      flex-wrap: wrap;
      padding-top: 0.75rem;
      border-top: 1px solid rgba(255, 255, 255, 0.05);
    }

    .automation-detail {
      display: flex;
      align-items: center;
      gap: 0.35rem;
      font-size: 0.75rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .automation-detail-icon {
      font-size: 0.85rem;
      opacity: 0.7;
    }

    .automation-detail-value {
      font-weight: 500;
    }

    .automation-quick-actions {
      margin-left: auto;
      display: flex;
      gap: 0.5rem;
      opacity: 0;
      transition: opacity 0.2s ease;
    }

    .automation-card:hover .automation-quick-actions {
      opacity: 1;
    }

    .quick-action-btn {
      width: 28px;
      height: 28px;
      border-radius: 8px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      background: rgba(255, 255, 255, 0.05);
      color: var(--color-dark-text-secondary, #adb5bd);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.75rem;
      transition: all 0.2s ease;
    }

    .quick-action-btn:hover {
      background: rgba(255, 255, 255, 0.1);
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
      background: rgba(255, 255, 255, 0.02);
      border-radius: 12px;
      border: 1px solid rgba(255, 255, 255, 0.05);
    }

    .category-pill {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.5rem 0.85rem;
      border-radius: 20px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(255, 255, 255, 0.03);
      color: var(--color-dark-text-secondary, #adb5bd);
      font-size: 0.8rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
      white-space: nowrap;
    }

    .category-pill:hover {
      background: rgba(255, 255, 255, 0.08);
      border-color: rgba(255, 255, 255, 0.15);
    }

    .category-pill.active {
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 25%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 100%);
      border-color: color-mix(in srgb, var(--context-primary, #00d4aa) 50%, transparent);
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
      border-radius: 10px;
      background: rgba(255, 255, 255, 0.1);
      font-size: 0.7rem;
      font-weight: 600;
    }

    .category-pill.active .category-pill-count {
      background: color-mix(in srgb, var(--context-primary, #00d4aa) 30%, transparent);
    }

    /* ===== Automations Header Stats ===== */
    .automations-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
      padding: 0.75rem 1rem;
      background: rgba(255, 255, 255, 0.02);
      border-radius: 12px;
      border: 1px solid rgba(255, 255, 255, 0.05);
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
      font-size: 1.25rem;
      font-weight: 700;
      color: var(--context-primary, #00d4aa);
    }

    .automation-stat-label {
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary, #6c757d);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .automation-stat-divider {
      width: 1px;
      height: 24px;
      background: rgba(255, 255, 255, 0.1);
    }

    /* ===== Enhanced History Section ===== */
    .history-section {
      margin-top: 1.5rem;
      padding-top: 1rem;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
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
        rgba(255, 255, 255, 0.1) 0%,
        rgba(255, 255, 255, 0.02) 100%);
      border-radius: 1px;
    }

    .history-item {
      position: relative;
      padding: 0.75rem 1rem;
      margin-bottom: 0.5rem;
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.05);
      border-radius: 10px;
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
      border-color: rgba(255, 255, 255, 0.1);
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
      font-size: 0.75rem;
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
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.02) 0%, transparent 100%);
      border: 2px dashed rgba(255, 255, 255, 0.1);
      border-radius: 16px;
    }

    .empty-state-icon-container {
      width: 80px;
      height: 80px;
      border-radius: 20px;
      background: linear-gradient(135deg,
        color-mix(in srgb, var(--context-primary, #00d4aa) 15%, transparent) 0%,
        color-mix(in srgb, var(--context-primary, #00d4aa) 5%, transparent) 100%);
      border: 1px solid color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
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
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.08);
      font-size: 0.75rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .suggestion-chip:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.15);
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    /* Stats */
    .stat-bar {
      margin-bottom: 1rem;
    }

    .stat-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.5rem;
    }

    .stat-label {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .stat-value {
      font-size: 0.8rem;
      color: var(--color-dark-text-secondary, #adb5bd);
    }

    .stat-track {
      height: 8px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 4px;
      overflow: hidden;
    }

    .stat-fill {
      height: 100%;
      border-radius: 4px;
      transition: width 0.5s ease;
    }

    .stat-fill.cravate { background: linear-gradient(90deg, #2563eb, #3b82f6); }
    .stat-fill.intime { background: linear-gradient(90deg, #059669, #00d4aa); }
    .stat-fill.neutre { background: linear-gradient(90deg, #4b5563, #6b7280); }

    /* Config */
    .config-section {
      margin-bottom: 1.5rem;
    }

    .config-title {
      font-size: 0.75rem;
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
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .config-label {
      font-size: 0.85rem;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .config-input {
      width: 80px;
      padding: 0.4rem 0.6rem;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.15);
      background: rgba(255, 255, 255, 0.05);
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

    .form-input {
      width: 100%;
      padding: 0.6rem 0.8rem;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 8px;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.9rem;
      transition: all 0.2s;
    }

    .form-input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--context-primary, #00d4aa) 20%, transparent);
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
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 12px;
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
      font-size: 1.25rem;
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
      background: rgba(255, 255, 255, 0.1);
      border-radius: 3px;
      overflow: hidden;
    }

    .confidence-bar-mini .confidence-fill {
      height: 100%;
      background: linear-gradient(90deg, #22c55e, #16a34a);
      border-radius: 3px;
      transition: width 0.3s ease;
    }

    .confidence-value {
      font-size: 0.75rem;
      color: var(--color-dark-text-secondary, #adb5bd);
      font-weight: 600;
    }

    .override-banner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: rgba(245, 158, 11, 0.15);
      border: 1px solid rgba(245, 158, 11, 0.3);
      border-radius: 8px;
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
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
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
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 12px;
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
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 12px;
      padding: 1rem;
      transition: all 0.2s;
    }

    .mode-card:hover {
      background: rgba(255, 255, 255, 0.06);
      border-color: var(--mode-primary, #6b7280);
    }

    .mode-card-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .mode-card-icon {
      font-size: 1.5rem;
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
      border-radius: 4px;
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
      border-radius: 6px;
    }

    .mode-card-slug {
      font-size: 0.75rem;
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
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 16px;
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
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
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
      border-top: 1px solid rgba(255, 255, 255, 0.08);
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
      background: rgba(255, 255, 255, 0.05);
      border: 2px solid transparent;
      border-radius: 8px;
      cursor: pointer;
      transition: all 0.2s;
    }

    .emoji-btn:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    .emoji-btn.selected {
      border-color: var(--context-primary, #00d4aa);
      background: rgba(0, 212, 170, 0.1);
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
      border-radius: 8px;
      cursor: pointer;
      background: transparent;
    }

    .color-picker-group input[type="color"]::-webkit-color-swatch-wrapper {
      padding: 2px;
    }

    .color-picker-group input[type="color"]::-webkit-color-swatch {
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.2);
    }

    .mode-preview {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 1rem;
      background: var(--preview-bg, #f8fafc);
      border-radius: 10px;
      border: 2px solid var(--preview-primary, #2563eb);
    }

    .preview-icon {
      font-size: 2rem;
    }

    .preview-name {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--preview-accent, #1e40af);
    }

    /* ============ PLANNING TAB STYLES ============ */
    .planning-container {
      padding: 0;
    }

    .planning-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
    }

    .planning-default {
      display: flex;
      align-items: center;
      padding: 0.75rem;
      background: rgba(255, 255, 255, 0.03);
      border-radius: 8px;
      margin-bottom: 1rem;
      font-size: 0.85rem;
      color: var(--color-dark-text-secondary);
    }

    .planning-grid {
      background: rgba(0, 0, 0, 0.2);
      border-radius: 12px;
      padding: 0.5rem;
      margin-bottom: 1rem;
      overflow-x: auto;
    }

    .grid-header {
      display: grid;
      grid-template-columns: 40px repeat(7, 1fr);
      gap: 2px;
      margin-bottom: 2px;
    }

    .grid-day-header {
      text-align: center;
      font-size: 0.7rem;
      font-weight: 600;
      color: var(--color-dark-text-tertiary);
      padding: 0.25rem;
    }

    .grid-row {
      display: grid;
      grid-template-columns: 40px repeat(7, 1fr);
      gap: 2px;
    }

    .grid-hour-label {
      font-size: 0.65rem;
      color: var(--color-dark-text-tertiary);
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .grid-cell {
      height: 32px;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid transparent;
      border-radius: 4px;
      cursor: pointer;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .grid-cell:hover {
      background: rgba(255, 255, 255, 0.08);
    }

    .grid-cell.has-rule {
      border-width: 2px;
    }

    .cell-icon {
      font-size: 0.8rem;
    }

    .planning-rules-list {
      max-height: 200px;
      overflow-y: auto;
    }

    .rule-card {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      padding: 0.75rem;
      margin-bottom: 0.5rem;
      transition: all 0.2s;
    }

    .rule-card.disabled {
      opacity: 0.5;
    }

    .rule-card:hover {
      background: rgba(255, 255, 255, 0.06);
    }

    .rule-card-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .rule-icon {
      font-size: 1.2rem;
    }

    .rule-name {
      flex: 1;
      font-weight: 500;
      color: var(--color-dark-text-primary);
    }

    .rule-actions {
      display: flex;
      gap: 0.25rem;
    }

    .rule-details {
      display: flex;
      gap: 1rem;
      margin-top: 0.5rem;
      font-size: 0.75rem;
      color: var(--color-dark-text-tertiary);
    }

    .rule-form {
      background: linear-gradient(135deg, rgba(25, 26, 32, 0.98) 0%, rgba(15, 15, 17, 1) 100%);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 16px;
      width: 90%;
      max-width: 420px;
      max-height: 90vh;
      overflow-y: auto;
    }

    .days-picker {
      display: flex;
      gap: 0.25rem;
    }

    .day-btn {
      flex: 1;
      padding: 0.5rem;
      font-size: 0.75rem;
      background: rgba(255, 255, 255, 0.05);
      border: 2px solid transparent;
      border-radius: 6px;
      color: var(--color-dark-text-secondary);
      cursor: pointer;
      transition: all 0.2s;
    }

    .day-btn:hover {
      background: rgba(255, 255, 255, 0.1);
    }

    .day-btn.selected {
      border-color: var(--context-primary, #00d4aa);
      background: rgba(0, 212, 170, 0.1);
      color: var(--context-primary, #00d4aa);
    }

    .form-row {
      display: flex;
      gap: 1rem;
    }

    /* Mobile */
    @media (max-width: 600px) {
      .page {
        width: 100%;
        height: 100%;
        max-height: 100vh;
        border-radius: 0;
      }

      .tabs {
        padding: 0.5rem;
      }

      .tab {
        padding: 0.4rem 0.75rem;
        font-size: 0.75rem;
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
  `

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
    // Planning horaire
    schedule: { type: Object },
    showRuleForm: { type: Boolean },
    editingRule: { type: Object },
    ruleFormData: { type: Object },
    // Notification configs
    notificationConfigs: { type: Array },
    editingNotifConfig: { type: Object },
    showNotifHelp: { type: Boolean },
    // Timeline highlighting
    highlightedAutomationId: { type: String },
    categoryFilter: { type: String },
    // Intelligence v2 data
    intelligenceFeatures: { type: Object },
    intelligenceVector: { type: Object },
    intelligencePrediction: { type: Object },
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
    // Planning horaire
    this.schedule = { rules: [], default_mode_id: 'mode-veille' }
    this.showRuleForm = false
    this.editingRule = null
    this.ruleFormData = {
      mode_id: 'mode-pro',
      days: [0, 1, 2, 3, 4],
      start_time: '09:00',
      end_time: '18:00',
      priority: 0,
      name: ''
    }
    // Notification configs
    this.notificationConfigs = []
    this.editingNotifConfig = null
    this.showNotifHelp = false
    // Category filter for automations
    this.categoryFilter = 'all'
    // Timeline highlighting
    this.highlightedAutomationId = null
    // Intelligence v2 data
    this.intelligenceFeatures = null
    this.intelligenceVector = null
    this.intelligencePrediction = null
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
        this.loadSchedule(),
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

  async loadSchedule() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.schedule = await apiService.request('/v1/schedule')
    } catch (e) {
      console.error('[context-engine] Failed to load schedule:', e)
      this.schedule = { rules: [], default_mode_id: 'mode-veille' }
    }
  }

  async loadNotificationConfigs() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      this.notificationConfigs = await apiService.request('/v1/notification-types')
    } catch (e) {
      console.error('[context-engine] Failed to load notification configs:', e)
      this.notificationConfigs = []
    }
  }

  async loadIntelligence() {
    try {
      const apiService = document.querySelector('api-service')
      if (!apiService) return
      const [features, vector, prediction] = await Promise.all([
        apiService.request('/v1/intelligence/features').catch(() => null),
        apiService.request('/v1/intelligence/vector').catch(() => null),
        apiService.request('/v1/intelligence/prediction2').catch(() => null),
      ])
      this.intelligenceFeatures = features
      this.intelligenceVector = vector
      this.intelligencePrediction = prediction
    } catch (e) {
      console.error('[context-engine] Failed to load intelligence v2:', e)
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

  // ============ Mode Change Overlay ============
  showModeChangeOverlay(mode, duration) {
    this.modeChangeOverlay = {
      mode,
      duration,
      icon: this.getModeIcon(mode),
      name: this.getModeName(mode)
    }

    // Auto-hide after 1.5 seconds
    setTimeout(() => {
      this.modeChangeOverlay = null
    }, 1500)
  }

  // Mode actions
  async setModeOverride(mode) {
    try {
      // Show mode change overlay immediately
      this.showModeChangeOverlay(mode, this.selectedDuration)

      const res = await csrfService.fetchWithCsrf('/v1/context/override', {
        method: 'POST',
        body: JSON.stringify({
          mode,
          duration_minutes: this.selectedDuration,
          reason: 'Override manuel via Decision Engine'
        })
      })
      if (res.ok) {
        this.contextState = await res.json()
        // Dispatch event pour mettre à jour le dashboard
        document.body.dispatchEvent(new CustomEvent('context-change', {
          detail: { context: this.contextState }
        }))
        this.showToast(`Mode ${this.getModeName(mode)} activé pour ${this.formatDuration(this.selectedDuration)}`, 'success')

        // Record feedback for intelligence learning
        // This allows the system to learn from manual mode changes
        csrfService.fetchWithCsrf('/v1/intelligence/feedback', {
          method: 'POST',
          body: JSON.stringify({ chosen_mode: mode })
        }).catch(e => console.log('[context-engine] Feedback recording failed (non-critical):', e))
      } else {
        this.showToast('Erreur lors du changement de mode', 'error')
      }
    } catch (e) {
      console.error('[context-engine] Failed to set override:', e)
      this.showToast('Erreur lors du changement de mode', 'error')
    }
  }

  async clearOverride() {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/context/clear', { method: 'POST' })
      if (res.ok) {
        await this.loadContext()
        // Dispatch event pour mettre à jour le dashboard
        document.body.dispatchEvent(new CustomEvent('context-change', {
          detail: { context: this.contextState }
        }))
        this.showToast('Override annulé - Mode automatique restauré', 'success')
      } else {
        this.showToast('Erreur lors de l\'annulation', 'error')
      }
    } catch (e) {
      console.error('[context-engine] Failed to clear override:', e)
      this.showToast('Erreur lors de l\'annulation', 'error')
    }
  }

  // Automation actions
  async toggleAutomation(id) {
    try {
      const auto = this.automations.find(a => a.id === id)
      if (!auto) {
        this.showToast('Automation introuvable', 'error')
        return
      }
      const newEnabled = !auto.enabled
      await automationsService.toggleAutomation(id, newEnabled)
      await this.loadAutomations()
      this.showToast(newEnabled ? 'Automation activée' : 'Automation désactivée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to toggle automation:', e)
      this.showToast('Erreur lors de la modification', 'error')
    }
  }

  async deleteAutomation(id) {
    const confirmed = await this.showConfirmDialog({
      icon: '🗑️',
      title: 'Supprimer cette automation ?',
      message: 'Cette action est irréversible. L\'automation sera définitivement supprimée.',
      confirmLabel: 'Supprimer',
      cancelLabel: 'Annuler',
      confirmClass: 'btn-danger'
    })

    if (!confirmed) return

    try {
      await automationsService.deleteAutomation(id)
      await this.loadAutomations()
      this.showToast('Automation supprimée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to delete automation:', e)
      this.showToast('Erreur lors de la suppression', 'error')
    }
  }

  openCreateForm() {
    this.editingAutomation = {
      name: '',
      enabled: true,
      triggers: { operator: 'or', triggers: [] },
      actions: [],
      cooldown_seconds: 60
    }
    // Reset trigger form state
    this.showingTriggerConfig = false
    this.pendingTriggerType = 'mode_change'
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showForm = true
  }

  // Timeline event handlers
  _handleTimelineSlotClick(e) {
    const { hour, day, dayName, automations } = e.detail
    if (automations && automations.length > 0) {
      // Open existing automation
      const autoId = automations[0].id
      const auto = this.automations.find(a => a.id === autoId)
      if (auto) {
        this.openEditForm(auto)
      }
    } else {
      // Create new scheduled automation with preset values
      this.openScheduledAutomation({ startHour: hour, endHour: hour + 3, day, dayName })
    }
  }

  _handleTimelineHighlight(e) {
    this.highlightedAutomationId = e.detail?.id || null
  }

  openScheduledAutomation(preset) {
    const dayNames = ['Dimanche', 'Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi']
    this.editingAutomation = {
      name: `Planning ${preset.dayName || dayNames[preset.day]} ${preset.startHour}h`,
      enabled: true,
      category: 'modes',
      _rules: {
        operator: 'and',
        items: [
          { type: 'scheduled', interval_seconds: 60 },
          { type: 'time_range', start_hour: preset.startHour, end_hour: preset.endHour },
          { type: 'day_of_week', days: [preset.day] }
        ]
      },
      actions: [{ type: 'force_mode', mode: '', reason: 'automation' }],
      cooldown_seconds: 60
    }
    this.showForm = true
  }

  openEditForm(auto) {
    const clone = JSON.parse(JSON.stringify(auto))
    // Migrate old trigger format to new triggers format
    if (!clone.triggers && clone.trigger) {
      clone.triggers = {
        operator: 'or',
        triggers: [clone.trigger]
      }
      delete clone.trigger
    } else if (!clone.triggers) {
      clone.triggers = { operator: 'or', triggers: [] }
    }
    this.editingAutomation = clone
    // Reset trigger form state
    this.showingTriggerConfig = false
    this.pendingTriggerType = 'mode_change'
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showForm = true
  }

  cancelForm() {
    this.showForm = false
    this.editingAutomation = null
  }

  async saveAutomation() {
    const errors = this.validateAutomation(this.editingAutomation)
    if (errors.length > 0) {
      this.showToast(errors[0], 'error')
      return
    }
    try {
      // Split unified rules into triggers and conditions for backend
      if (this.editingAutomation._rules) {
        const { triggers, conditions } = this.splitRulesForBackend(this.editingAutomation._rules)
        this.editingAutomation.triggers = triggers
        this.editingAutomation.conditions = conditions.conditions?.length > 0 ? conditions : null
      }

      // Clean up internal fields before sending
      const autoToSave = { ...this.editingAutomation }
      delete autoToSave._rules

      const isEdit = !!autoToSave.id
      if (autoToSave.id) {
        await automationsService.updateAutomation(autoToSave.id, autoToSave)
      } else {
        await automationsService.createAutomation(autoToSave)
      }
      this.showForm = false
      this.editingAutomation = null
      await this.loadAutomations()
      this.showToast(isEdit ? 'Automation mise à jour' : 'Automation créée', 'success')
    } catch (e) {
      console.error('[context-engine] Failed to save automation:', e)
      this.showToast('Erreur: ' + (e.message || 'Erreur inconnue'), 'error')
    }
  }

  validateAutomation(auto) {
    const errors = []
    if (!auto?.name?.trim()) {
      errors.push('Le nom est requis')
    }
    // Check rules - must have at least one event (trigger)
    if (auto._rules) {
      if (!this.hasEventInRules(auto._rules)) {
        errors.push('Au moins un événement (déclencheur) est requis')
      }
    } else {
      // Fallback for legacy automations without _rules
      const triggersGroup = auto?.triggers
      if (!triggersGroup?.triggers?.length) {
        errors.push('Au moins un déclencheur est requis')
      }
    }
    // Check actions
    if (!auto?.actions?.length) {
      errors.push('Au moins une action est requise')
    } else {
      // Validate each action's required fields
      auto.actions.forEach((action, idx) => {
        const actionSchema = this.schema?.actions?.find(a => a.type === action.type)
        if (actionSchema?.fields) {
          actionSchema.fields.forEach(field => {
            if (field.required) {
              const value = action[field.name]
              if (value === undefined || value === null || value === '') {
                errors.push(`Action ${idx + 1} (${actionSchema.label}): ${field.label} est requis`)
              }
            }
          })
        }
      })
    }
    return errors
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
          <button class="close-btn" @click="${this.close}">✕</button>
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

  renderModeTab() {
    const state = this.contextState
    if (!state) return html`<div class="empty-state"><div class="empty-icon">⏳</div><div class="empty-text">Chargement du contexte...</div></div>`

    // Prefer mode_slug (dynamic) over mode (legacy enum)
    const mode = state.mode_slug || state.mode?.toLowerCase() || 'veille'
    const hasOverride = !!state.manual_override

    return html`
      <div class="mode-display">
        <div class="mode-icon">${this.getModeIcon(mode)}</div>
        <div class="mode-name">${this.getModeName(mode)}</div>
        <div class="mode-reason">${state.reason || 'Détection automatique'}</div>

        <!-- Confidence bar removed - now displayed in Intelligence Widget -->

        ${hasOverride ? html`
          <div class="override-info">
            ⚠️ Override manuel actif jusqu'à ${new Date(state.manual_override.until).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
          </div>
        ` : ''}

        <div class="mode-controls">
          <div class="controls-title">Contrôle Manuel</div>

          <div class="mode-buttons">
            <button class="mode-btn cravate" @click="${() => this.setModeOverride('cravate')}">
              👔 Focus
            </button>
            <button class="mode-btn intime" @click="${() => this.setModeOverride('intime')}">
              🏡 Maison
            </button>
            <button class="mode-btn neutre" @click="${() => this.setModeOverride('neutre')}">
              🌱 Veille
            </button>
          </div>

          <div class="duration-buttons">
            ${[60, 120, 240, 480].map(d => html`
              <button
                class="duration-btn ${this.selectedDuration === d ? 'active' : ''}"
                @click="${() => this.selectedDuration = d}"
              >
                ${this.formatDuration(d)}
              </button>
            `)}
          </div>

          ${hasOverride ? html`
            <button class="clear-override-btn" @click="${this.clearOverride}">
              🔄 Annuler Override
            </button>
          ` : ''}
        </div>
      </div>
    `
  }

  renderAutomationsTab() {
    if (this.showForm) {
      return this.renderAutomationForm()
    }

    // Get categories from schema with icons
    const categoryIcons = {
      all: '📋', comfort: '🛋️', security: '🔒', energy: '⚡',
      notifications: '🔔', custom: '⚙️'
    }
    const categories = this.schema?.dynamic_values?.categories || []

    // Filter automations by category
    const filteredAutomations = this.categoryFilter === 'all'
      ? this.automations
      : this.automations.filter(a => (a.category || 'custom') === this.categoryFilter)

    const totalEnabled = this.automations.filter(a => a.enabled).length
    const filteredEnabled = filteredAutomations.filter(a => a.enabled).length
    const recentExecutions = this.automationHistory.filter(h => {
      const execTime = new Date(h.executed_at)
      return (Date.now() - execTime) < 24 * 60 * 60 * 1000
    }).length

    return html`
      <!-- Timeline Hebdomadaire -->
      <automation-timeline
        .automations="${this.automations}"
        .modes="${this.schema?.dynamic_values?.modes || []}"
        .highlightedId="${this.highlightedAutomationId}"
        @slot-click="${this._handleTimelineSlotClick}"
        @automation-highlight="${this._handleTimelineHighlight}"
        style="margin-bottom: 1.5rem;"
      ></automation-timeline>

      <!-- Header Stats -->
      <div class="automations-header">
        <div class="automations-stats">
          <div class="automation-stat">
            <span class="automation-stat-value">${totalEnabled}</span>
            <span class="automation-stat-label">Actives</span>
          </div>
          <div class="automation-stat-divider"></div>
          <div class="automation-stat">
            <span class="automation-stat-value">${this.automations.length}</span>
            <span class="automation-stat-label">Total</span>
          </div>
          <div class="automation-stat-divider"></div>
          <div class="automation-stat">
            <span class="automation-stat-value">${recentExecutions}</span>
            <span class="automation-stat-label">Exécutions 24h</span>
          </div>
        </div>
      </div>

      <!-- Enhanced Category Filter -->
      <div class="category-filter-bar">
        <button
          class="category-pill ${this.categoryFilter === 'all' ? 'active' : ''}"
          @click="${() => { this.categoryFilter = 'all'; this.requestUpdate() }}"
        >
          <span class="category-pill-icon">${categoryIcons.all}</span>
          <span>Toutes</span>
          <span class="category-pill-count">${this.automations.length}</span>
        </button>
        ${categories.map(cat => {
          const count = this.automations.filter(a => (a.category || 'custom') === cat.value).length
          const icon = categoryIcons[cat.value] || '📁'
          return html`
            <button
              class="category-pill ${this.categoryFilter === cat.value ? 'active' : ''}"
              @click="${() => { this.categoryFilter = cat.value; this.requestUpdate() }}"
            >
              <span class="category-pill-icon">${icon}</span>
              <span>${cat.label}</span>
              <span class="category-pill-count">${count}</span>
            </button>
          `
        })}
      </div>

      <!-- Automation Cards -->
      ${filteredAutomations.length === 0 ? this.renderEmptyStateEnhanced() : html`
        <div class="automations-list">
          <!-- Add New Automation Card -->
          <div class="automation-card add-new" @click="${this.openCreateForm}">
            <div class="automation-card-inner" style="display: flex; align-items: center; justify-content: center; min-height: 80px; cursor: pointer;">
              <div style="text-align: center;">
                <div style="font-size: 1.5rem; margin-bottom: 0.25rem; opacity: 0.6;">+</div>
                <div style="font-size: 0.85rem; color: var(--color-dark-text-secondary);">Nouvelle automation</div>
              </div>
            </div>
          </div>
          ${filteredAutomations.map(auto => this.renderAutomationCard(auto))}
        </div>
      `}

      <!-- Enhanced History Section -->
      ${this.automationHistory.length > 0 ? this.renderHistorySection() : ''}
    `
  }

  renderEmptyStateEnhanced() {
    const suggestions = [
      { icon: '🌙', label: 'Mode nuit auto', action: () => this.createSuggestionAutomation('night') },
      { icon: '🚪', label: 'Notification entrée', action: () => this.createSuggestionAutomation('entry') },
      { icon: '🌡️', label: 'Alerte température', action: () => this.createSuggestionAutomation('temp') }
    ]

    return html`
      <div class="empty-state-enhanced">
        <div class="empty-state-icon-container">⚡</div>
        <div class="empty-state-title">
          ${this.categoryFilter === 'all'
            ? 'Aucune automation configurée'
            : 'Aucune automation dans cette catégorie'}
        </div>
        <div class="empty-state-description">
          Automatisez votre maison en créant des règles intelligentes qui réagissent aux événements.
        </div>
        <button class="btn btn-primary" @click="${this.openCreateForm}">
          Créer une automation
        </button>
        ${this.categoryFilter === 'all' ? html`
          <div class="empty-state-suggestions">
            ${suggestions.map(s => html`
              <button class="suggestion-chip" @click="${s.action}">
                <span>${s.icon}</span>
                <span>${s.label}</span>
              </button>
            `)}
          </div>
        ` : ''}
      </div>
    `
  }

  createSuggestionAutomation(type) {
    // Pre-fill automation based on suggestion type
    const templates = {
      night: {
        name: 'Mode nuit automatique',
        category: 'comfort',
        triggers: { operator: 'or', triggers: [{ type: 'scheduled', start_hour: 23, end_hour: 23 }] },
        actions: [{ type: 'force_mode', mode: 'veille', duration_minutes: 480 }]
      },
      entry: {
        name: 'Notification entrée',
        category: 'security',
        triggers: { operator: 'or', triggers: [{ type: 'agent_status', status: 'online' }] },
        actions: [{ type: 'send_notification', title: 'Arrivée détectée', body: 'Un appareil vient de se connecter' }]
      },
      temp: {
        name: 'Alerte température haute',
        category: 'notifications',
        triggers: { operator: 'or', triggers: [{ type: 'sensor_alert', alert_level: 'warning' }] },
        actions: [{ type: 'send_notification', title: 'Alerte température', body: 'Température anormale détectée' }]
      }
    }

    this.editingAutomation = {
      enabled: true,
      cooldown_seconds: 300,
      conditions: { operator: 'and', conditions: [] },
      ...templates[type]
    }
    this.showForm = true
    this.requestUpdate()
  }

  renderHistorySection() {
    return html`
      <div class="history-section">
        <div class="history-header">
          <div class="history-title">
            <span class="history-title-icon">📜</span>
            Historique récent
          </div>
          <span style="font-size: 0.75rem; color: var(--color-dark-text-tertiary);">
            ${this.automationHistory.length} exécution${this.automationHistory.length !== 1 ? 's' : ''}
          </span>
        </div>
        <div class="history-timeline">
          ${this.automationHistory.slice(0, 5).map(h => this.renderHistoryItem(h))}
        </div>
      </div>
    `
  }

  // Génère un champ de formulaire basé sur le schema
  renderSchemaField(field, value, onChange) {
    const options = field.options_key ? (this.schema?.dynamic_values?.[field.options_key] || []) : []

    switch (field.field_type) {
      case 'select':
        return html`
          <select class="form-input"
            @change="${e => onChange(e.target.value || null)}">
            ${!field.required
              ? html`<option value="">${field.placeholder || 'Tous'}</option>`
              : html`<option value="" disabled ?selected="${!value}">-- Sélectionner --</option>`
            }
            ${options.map(opt => html`
              <option value="${opt.value}" ?selected="${opt.value === value}">${opt.label}</option>
            `)}
            ${field.name === 'status' && options.length === 0 ? html`
              <option value="online" ?selected="${value === 'online'}">Online</option>
              <option value="offline" ?selected="${value === 'offline'}">Offline</option>
            ` : ''}
          </select>
        `

      case 'multi_select':
        const selectedValues = Array.isArray(value) ? value : []
        return html`
          <div class="multi-select-group">
            ${options.map(opt => html`
              <label class="checkbox-label">
                <input type="checkbox"
                  ?checked="${selectedValues.includes(opt.value)}"
                  @change="${e => {
                    const newVals = e.target.checked
                      ? [...selectedValues, opt.value]
                      : selectedValues.filter(v => v !== opt.value)
                    onChange(newVals)
                  }}">
                ${opt.label}
              </label>
            `)}
          </div>
        `

      case 'number':
        // Detect hour fields for special rendering
        const isHourField = field.name.includes('hour') && field.max <= 24
        if (isHourField) {
          const hours = Array.from({ length: 25 }, (_, i) => i) // 0-24
          const currentVal = value ?? field.default_value ?? ''
          return html`
            <select class="form-input hour-select"
              @change="${e => onChange(e.target.value !== '' ? parseInt(e.target.value) : null)}">
              ${!field.required ? html`<option value="">--</option>` : ''}
              ${hours.map(h => html`
                <option value="${h}" ?selected="${h === currentVal}">${String(h).padStart(2, '0')}:00</option>
              `)}
            </select>
          `
        }
        return html`
          <input type="number" class="form-input"
            .value="${value ?? field.default_value ?? ''}"
            min="${field.min ?? ''}"
            max="${field.max ?? ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value ? parseFloat(e.target.value) : null)}">
        `

      case 'text':
        return html`
          <input type="text" class="form-input"
            .value="${value || ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value)}">
        `

      case 'text_area':
        return html`
          <textarea class="form-input" rows="2"
            .value="${value || ''}"
            placeholder="${field.placeholder || ''}"
            @input="${e => onChange(e.target.value)}"></textarea>
        `

      default:
        return html`<input type="text" class="form-input" .value="${value || ''}" @input="${e => onChange(e.target.value)}">`
    }
  }

  renderAutomationForm() {
    const auto = this.editingAutomation || {}
    const isEdit = !!auto.id

    return html`
      <div class="controls-title">${isEdit ? 'Modifier' : 'Nouvelle'} automation</div>

      <div class="form-group">
        <label>Nom</label>
        <input type="text" class="form-input" .value="${auto.name || ''}"
          @input="${e => this.editingAutomation.name = e.target.value}">
      </div>

      <div class="form-group">
        <label>Catégorie</label>
        <select class="form-input"
          @change="${e => { this.editingAutomation.category = e.target.value; this.requestUpdate() }}">
          ${(this.schema?.dynamic_values?.categories || []).map(cat => html`
            <option value="${cat.value}" ?selected="${(auto.category || 'custom') === cat.value}">${cat.label}</option>
          `)}
        </select>
      </div>

      <div class="form-group">
        <label>Mode cible (apprentissage)</label>
        <select class="form-input"
          @change="${e => { this.editingAutomation.goal_mode = e.target.value || null; this.requestUpdate() }}">
          <option value="" ?selected="${!auto.goal_mode}">Aucun (pas d'apprentissage)</option>
          ${(this.schema?.dynamic_values?.modes || []).map(mode => html`
            <option value="${mode.value}" ?selected="${auto.goal_mode === mode.value}">${mode.label}</option>
          `)}
        </select>
        <small style="color: var(--color-dark-text-tertiary); font-size: 0.75rem; margin-top: 0.25rem; display: block;">
          Le mode que cette automation vise à atteindre. Permet à l'Intelligence d'apprendre.
        </small>
      </div>

      <!-- Règles Section (Triggers + Conditions unifiés) -->
      ${this.renderRulesSection(auto)}

      <div class="form-group">
        <label>Cooldown (secondes)</label>
        <input type="number" class="form-input" min="0" .value="${auto.cooldown_seconds || 60}"
          @input="${e => this.editingAutomation.cooldown_seconds = parseInt(e.target.value) || 0}">
      </div>

      <div class="form-group">
        <label style="display: flex; align-items: center; gap: 0.5rem;">
          <input type="checkbox" ?checked="${auto.enabled !== false}"
            @change="${e => this.editingAutomation.enabled = e.target.checked}">
          Activée
        </label>
      </div>

      <!-- Trust Settings -->
      <div class="form-group" style="background: rgba(0, 212, 170, 0.05); padding: 0.75rem; border-radius: 8px; border: 1px solid rgba(0, 212, 170, 0.2);">
        <label style="font-weight: 600; margin-bottom: 0.5rem; display: block; color: #00d4aa;">🛡️ Niveau de confiance</label>

        <label style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
          <input type="checkbox" ?checked="${auto.trusted === true}"
            @change="${e => { this.editingAutomation.trusted = e.target.checked; this.requestUpdate() }}">
          <span>Trusted</span>
          <span style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">— Auto-approuvée sans validation</span>
        </label>

        <label style="display: flex; align-items: center; gap: 0.5rem;">
          <input type="checkbox" ?checked="${auto.skip_if_same_mode === true}"
            @change="${e => { this.editingAutomation.skip_if_same_mode = e.target.checked; this.requestUpdate() }}">
          <span>Skip si même mode</span>
          <span style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">— Ne pas exécuter si déjà dans ce mode</span>
        </label>

        <div style="margin-top: 0.5rem; padding-top: 0.5rem; border-top: 1px solid rgba(255,255,255,0.1); font-size: 0.75rem; color: var(--color-dark-text-tertiary);">
          ${auto.trusted ? html`
            <span style="color: #22c55e;">✓ Cette automation sera exécutée automatiquement sans demander de validation.</span>
          ` : html`
            <span>Le trust score augmente de +1% à chaque exécution réussie (max +20%).</span>
          `}
        </div>
      </div>

      <!-- Actions Section -->
      <div class="form-group">
        <label>Actions (${auto.actions?.length || 0})</label>
        <div style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.5rem;">
          ${(auto.actions || []).map((action, idx) => html`
            <div class="action-item" style="display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.03); border-radius: 8px; border: 1px solid rgba(255,255,255,0.08);">
              <span style="flex: 1; font-size: 0.85rem;">${this.getActionLabel(action)}</span>
              <button class="btn btn-small btn-icon btn-danger" @click="${() => this.removeAction(idx)}" title="Supprimer">✕</button>
            </div>
          `)}
        </div>

        <!-- Add Action -->
        <div style="margin-top: 0.75rem; padding: 0.75rem; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed rgba(255,255,255,0.1);">
          <div style="display: flex; gap: 0.5rem; align-items: flex-end; flex-wrap: wrap;">
            <div style="flex: 1; min-width: 150px;">
              <label style="font-size: 0.75rem; color: var(--color-dark-text-tertiary);">Type d'action</label>
              <select class="form-input" id="new-action-type" style="margin-top: 0.25rem;">
                ${(this.schema?.actions || []).map(a => html`
                  <option value="${a.type}">${a.icon || ''} ${a.label}</option>
                `)}
              </select>
            </div>
            <button class="btn btn-small" @click="${this.showActionConfig}" style="white-space: nowrap;">+ Configurer</button>
          </div>

          ${this.showingActionConfig ? html`
            <div style="margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.08);">
              ${this.renderActionConfig()}
              <div style="display: flex; gap: 0.5rem; margin-top: 0.75rem;">
                <button class="btn btn-small" @click="${() => this.showingActionConfig = false}">Annuler</button>
                <button class="btn btn-small btn-primary" @click="${this.addConfiguredAction}">Ajouter</button>
              </div>
            </div>
          ` : ''}
        </div>
      </div>

      <div style="display: flex; gap: 1rem; margin-top: 1.5rem;">
        <button class="btn" style="flex: 1;" @click="${this.cancelForm}">Annuler</button>
        <button class="btn btn-primary" style="flex: 1;" @click="${this.saveAutomation}">
          ${isEdit ? 'Enregistrer' : 'Créer'}
        </button>
      </div>
    `
  }

  /**
   * Initialize an object with default values from schema fields
   * Handles: default_value, required selects (first option), hour fields (sensible defaults)
   */
  initializeWithDefaults(type, fields) {
    const defaults = { type }

    if (!fields) return defaults

    fields.forEach(field => {
      // Set default value from schema if available
      if (field.default_value !== undefined && field.default_value !== null) {
        defaults[field.name] = field.default_value
      }
      // For required select fields, pre-select the first option
      else if (field.required && field.field_type === 'select' && field.options_key) {
        const options = this.schema?.dynamic_values?.[field.options_key] || []
        if (options.length > 0) {
          defaults[field.name] = options[0].value
        }
      }
      // For required hour fields, set sensible defaults
      else if (field.required && field.field_type === 'number' && field.name.includes('hour')) {
        if (field.name.includes('start')) {
          defaults[field.name] = 8  // Default start: 8h
        } else if (field.name.includes('end')) {
          defaults[field.name] = 18 // Default end: 18h
        }
      }
    })

    return defaults
  }

  getActionLabel(action) {
    switch (action.type) {
      case 'send_notification':
        return `📢 Notif: "${action.title || 'Sans titre'}"`
      case 'force_mode':
        const modeLabel = action.mode || '(non défini)'
        const duration = action.duration_minutes || 60
        return `🎯 Mode: ${modeLabel} (${duration}min)`
      case 'agent_command':
        return `🤖 Agent ${action.agent_id || '?'}: ${action.command_type || '?'}`
      case 'delay':
        return `⏱️ Délai: ${action.seconds || 0}s`
      default:
        return `⚙️ ${action.type || 'inconnu'}`
    }
  }

  showActionConfig() {
    this.pendingActionType = this.shadowRoot.querySelector('#new-action-type')?.value || 'send_notification'
    const actionSchema = this.schema?.actions?.find(a => a.type === this.pendingActionType)
    this.pendingAction = this.initializeWithDefaults(this.pendingActionType, actionSchema?.fields)
    this.showingActionConfig = true
  }

  renderActionConfig() {
    const type = this.pendingActionType
    const actionSchema = this.schema?.actions?.find(a => a.type === type)

    if (!actionSchema || !actionSchema.fields?.length) {
      return html`<div style="font-size: 0.8rem; color: var(--color-dark-text-tertiary);">
        ${actionSchema?.description || 'Aucune configuration requise'}
      </div>`
    }

    return html`
      ${actionSchema.fields.map((field, idx) => html`
        <div class="form-group" style="margin-bottom: ${idx < actionSchema.fields.length - 1 ? '0.5rem' : '0'};">
          <label style="font-size: 0.75rem;">${field.label}${field.required ? ' *' : ''}</label>
          ${this.renderSchemaField(field, this.pendingAction?.[field.name], (val) => {
            this.pendingAction = { ...this.pendingAction, [field.name]: val }
            this.requestUpdate()
          })}
        </div>
      `)}
    `
  }

  addConfiguredAction() {
    if (!this.pendingAction) return

    // Ensure actions array exists
    if (!this.editingAutomation.actions) {
      this.editingAutomation.actions = []
    }

    // Add the configured action
    this.editingAutomation.actions = [...this.editingAutomation.actions, { ...this.pendingAction }]

    // Reset
    this.pendingAction = null
    this.showingActionConfig = false
    this.requestUpdate()
  }

  removeAction(idx) {
    if (this.editingAutomation?.actions) {
      this.editingAutomation.actions = this.editingAutomation.actions.filter((_, i) => i !== idx)
      this.requestUpdate()
    }
  }

  // ========== Unified Rules Section (Triggers + Conditions) ==========

  /**
   * Initialise les règles unifiées à partir des triggers et conditions existants
   */
  initRulesFromAutomation(auto) {
    if (auto._rules) return auto._rules // Déjà initialisé

    const rules = { operator: 'and', items: [] }

    // Ajouter les triggers existants
    const triggersGroup = auto.triggers || { operator: 'or', triggers: [] }
    for (const t of (triggersGroup.triggers || [])) {
      if (t.operator && t.triggers) {
        // Groupe imbriqué
        rules.items.push({
          operator: t.operator,
          items: t.triggers.map(tr => ({ ...tr, _category: 'event' }))
        })
      } else {
        rules.items.push({ ...t, _category: 'event' })
      }
    }

    // Ajouter les conditions existantes
    const conditionsGroup = auto.conditions || { operator: 'and', conditions: [] }
    for (const c of (conditionsGroup.conditions || [])) {
      if (c.operator && c.conditions) {
        // Groupe imbriqué
        rules.items.push({
          operator: c.operator,
          items: c.conditions.map(cd => ({ ...cd, _category: 'state' }))
        })
      } else {
        rules.items.push({ ...c, _category: 'state' })
      }
    }

    return rules
  }

  /**
   * Retourne les types de règles unifiés (triggers + conditions)
   */
  getUnifiedRuleTypes() {
    const triggers = (this.schema?.triggers || []).map(t => ({
      ...t,
      _category: 'event',
      _categoryLabel: '⚡ Événement'
    }))
    const conditions = (this.schema?.conditions || []).map(c => ({
      ...c,
      _category: 'state',
      _categoryLabel: '📋 État'
    }))
    return [...triggers, ...conditions]
  }

  renderRulesSection(auto) {
    // Initialiser les règles si nécessaire
    if (!auto._rules) {
      auto._rules = this.initRulesFromAutomation(auto)
    }
    const rules = auto._rules
    const ruleCount = this.countRules(rules)

    return html`
      <div class="form-group">
        <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.25rem;">
          <label style="margin: 0;">Règles ${ruleCount > 0 ? `(${ruleCount})` : ''}</label>
          <button
            type="button"
            @click="${() => this.showHelp('rules')}"
            style="cursor: pointer; font-size: 0.75rem; font-weight: bold; min-width: 22px; height: 22px; padding: 0 6px; border-radius: 11px; background: #3b82f6; border: none; display: inline-flex; align-items: center; justify-content: center; color: #fff; box-shadow: 0 2px 4px rgba(59,130,246,0.4);"
          >?</button>
        </div>

        <div class="rules-editor" style="padding: 0.75rem; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px solid rgba(255,255,255,0.08);">
          ${this.renderRulesGroup(rules, [], 0)}
        </div>
      </div>
    `
  }

  countRules(group) {
    if (!group?.items) return 0
    let count = 0
    for (const item of group.items) {
      if (item.operator && item.items) {
        count += this.countRules(item)
      } else {
        count++
      }
    }
    return count
  }

  renderRulesGroup(group, path, depth) {
    const operatorLabel = group.operator === 'and' ? 'ET' : 'OU'
    const operatorColor = group.operator === 'and' ? '#3b82f6' : '#f59e0b'
    const operatorHint = group.operator === 'and'
      ? 'Toutes les règles doivent correspondre'
      : 'Au moins une règle doit correspondre'
    const indent = depth * 12

    return html`
      <div class="rules-group" style="margin-left: ${indent}px; ${depth > 0 ? 'margin-top: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.02); border-radius: 6px; border: 1px dashed rgba(255,255,255,0.1);' : ''}">
        <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
          <button
            class="btn btn-small"
            style="font-size: 0.7rem; padding: 0.2rem 0.5rem; background: ${operatorColor}; color: white; border: none;"
            @click="${() => this.toggleRulesGroupOperator(path)}"
            title="Cliquer pour basculer AND/OR"
          >${operatorLabel}</button>
          ${depth > 0 ? html`
            <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeRulesGroup(path)}" title="Supprimer groupe">✕</button>
          ` : ''}
          <span style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">
            ${operatorHint}
          </span>
        </div>

        ${group.items?.map((item, idx) => {
          const itemPath = [...path, idx]
          if (item.operator && item.items) {
            // Nested group
            return this.renderRulesGroup(item, itemPath, depth + 1)
          } else {
            // Single rule
            return this.renderRuleItem(item, itemPath)
          }
        })}

        <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
          <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.showRuleConfigFor(path)}">
            + Règle
          </button>
          ${depth < 1 ? html`
            <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.addRulesGroup(path)}">
              + Groupe
            </button>
          ` : ''}
        </div>

        ${this.showingRuleConfig && JSON.stringify(this.pendingRulePath) === JSON.stringify(path) ? html`
          <div style="margin-top: 0.75rem; padding: 0.75rem; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.1);">
            ${this.renderRuleConfig()}
            <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
              <button class="btn btn-small" @click="${() => this.showingRuleConfig = false}">Annuler</button>
              <button class="btn btn-small btn-primary" @click="${this.addConfiguredRule}">Ajouter</button>
            </div>
          </div>
        ` : ''}
      </div>
    `
  }

  renderRuleItem(rule, path) {
    const isEvent = isEventType(rule.type)
    const categoryColor = isEvent ? '#10b981' : '#8b5cf6'
    const categoryIcon = isEvent ? '⚡' : '📋'
    const label = this.getRuleLabel(rule)

    return html`
      <div class="rule-item" style="display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.6rem; margin: 0.25rem 0; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.08); border-left: 3px solid ${categoryColor};">
        <span style="font-size: 0.65rem; color: ${categoryColor};" title="${isEvent ? 'Événement (déclencheur)' : 'État (condition)'}">${categoryIcon}</span>
        <span style="flex: 1; font-size: 0.8rem;">${label}</span>
        <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeRule(path)}" title="Supprimer">✕</button>
      </div>
    `
  }

  getRuleLabel(rule) {
    // Chercher dans triggers ou conditions
    const triggerSchema = this.schema?.triggers?.find(t => t.type === rule.type)
    const conditionSchema = this.schema?.conditions?.find(c => c.type === rule.type)
    const schema = triggerSchema || conditionSchema
    const icon = schema?.icon || ''

    // Labels spécifiques selon le type
    switch (rule.type) {
      case 'mode_change':
        const fromMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.from_mode)
        const toMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.to_mode)
        if (rule.from_mode && rule.to_mode) {
          return `${icon} Mode: ${fromMode?.label || rule.from_mode} → ${toMode?.label || rule.to_mode}`
        } else if (rule.to_mode) {
          return `${icon} Mode → ${toMode?.label || rule.to_mode}`
        } else if (rule.from_mode) {
          return `${icon} Mode: ${fromMode?.label || rule.from_mode} → *`
        }
        return `${icon} Changement de mode`
      case 'sensor_alert':
        const room = this.schema?.dynamic_values?.rooms?.find(r => r.value === rule.room_id)
        const level = this.schema?.dynamic_values?.alert_levels?.find(l => l.value === rule.alert_level)
        return `${icon} Alerte ${level?.label || 'capteur'} ${room ? `(${room.label})` : ''}`
      case 'agent_status':
        const agent = this.schema?.dynamic_values?.agents?.find(a => a.value === rule.agent_id)
        return `${icon} Agent ${agent?.label || rule.agent_id || '*'}: ${rule.status || '*'}`
      case 'manual':
        return `${icon} Déclenchement manuel`
      case 'plugin_health':
        const plugin = this.schema?.dynamic_values?.plugins?.find(p => p.value === rule.plugin_name)
        const status = this.schema?.dynamic_values?.plugin_health_statuses?.find(s => s.value === rule.status)
        return `${icon} Plugin ${plugin?.label || rule.plugin_name || '*'}: ${status?.label || rule.status || '*'}`
      case 'scheduled':
        const intervalSecs = rule.interval_seconds || 300
        const intervalLabel = intervalSecs >= 3600
          ? `${Math.round(intervalSecs / 3600)}h`
          : intervalSecs >= 60
            ? `${Math.round(intervalSecs / 60)}min`
            : `${intervalSecs}s`
        const activeHoursLabel = rule.active_hours
          ? ` (${rule.active_hours[0]}h-${rule.active_hours[1]}h)`
          : ''
        return `${icon} Planifié toutes les ${intervalLabel}${activeHoursLabel}`
      case 'current_mode':
        const currentMode = this.schema?.dynamic_values?.modes?.find(m => m.value === rule.mode)
        return `${icon} Mode actuel = ${currentMode?.label || rule.mode}`
      case 'time_range':
        return `${icon} Heure entre ${rule.start_hour || 0}h et ${rule.end_hour || 24}h`
      case 'day_of_week':
        const dayNames = ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam']
        const days = (rule.days || []).map(d => dayNames[parseInt(d)] || d).join(', ')
        return `${icon} Jour: ${days || 'Aucun'}`
      case 'day_of_month':
        const monthDays = (rule.days || []).map(d => parseInt(d) === 31 ? 'Dernier' : d).join(', ')
        return `${icon} Jour du mois: ${monthDays || 'Aucun'}`
      case 'month':
        const monthNames = ['', 'Jan', 'Fév', 'Mar', 'Avr', 'Mai', 'Juin', 'Juil', 'Août', 'Sep', 'Oct', 'Nov', 'Déc']
        const months = (rule.months || []).map(m => monthNames[parseInt(m)] || m).join(', ')
        return `${icon} Mois: ${months || 'Tous'}`
      case 'sensor_value':
        const sensorRoom = this.schema?.dynamic_values?.rooms?.find(r => r.value === rule.room_id)
        const opLabel = { greater_than: '>', less_than: '<', equals: '=' }[rule.operator] || rule.operator
        return `${icon} ${rule.metric || 'capteur'} ${sensorRoom?.label || ''} ${opLabel} ${rule.value}`
      case 'agent_online':
        const agentCond = this.schema?.dynamic_values?.agents?.find(a => a.value === rule.agent_id)
        return `${icon} Agent ${agentCond?.label || rule.agent_id} ${rule.online ? 'en ligne' : 'hors ligne'}`
      default:
        return `${icon} ${schema?.label || rule.type}`
    }
  }

  showRuleConfigFor(path) {
    this.pendingRulePath = path
    const allTypes = this.getUnifiedRuleTypes()
    this.pendingRuleType = allTypes[0]?.type || 'mode_change'
    const ruleSchema = allTypes.find(t => t.type === this.pendingRuleType)
    this.pendingRule = this.initializeWithDefaults(this.pendingRuleType, ruleSchema?.fields)
    this.showingRuleConfig = true
    this.requestUpdate()
  }

  renderRuleConfig() {
    const type = this.pendingRuleType
    const allTypes = this.getUnifiedRuleTypes()
    const ruleSchema = allTypes.find(t => t.type === type)

    return html`
      <div style="display: flex; flex-direction: column; gap: 0.5rem;">
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Type de règle</label>
          <select class="form-input" style="font-size: 0.8rem;"
            @change="${e => {
              this.pendingRuleType = e.target.value
              const newSchema = allTypes.find(t => t.type === e.target.value)
              this.pendingRule = this.initializeWithDefaults(e.target.value, newSchema?.fields)
              this.requestUpdate()
            }}">
            <optgroup label="⚡ Événements (déclencheurs)">
              ${allTypes.filter(t => t._category === 'event').map(t => html`
                <option value="${t.type}" ?selected="${t.type === type}">${t.icon || ''} ${t.label}</option>
              `)}
            </optgroup>
            <optgroup label="📋 États (conditions)">
              ${allTypes.filter(t => t._category === 'state').map(t => html`
                <option value="${t.type}" ?selected="${t.type === type}">${t.icon || ''} ${t.label}</option>
              `)}
            </optgroup>
          </select>
          ${ruleSchema?.description ? html`
            <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary); margin-top: 0.25rem;">${ruleSchema.description}</div>
          ` : ''}
        </div>

        ${ruleSchema?.fields?.map(field => html`
          <div class="form-group" style="margin-bottom: 0;">
            <label style="font-size: 0.75rem;">${field.label}${field.required ? ' *' : ''}</label>
            ${this.renderSchemaField(field, this.pendingRule?.[field.name], (val) => {
              this.pendingRule = { ...this.pendingRule, [field.name]: val }
              this.requestUpdate()
            })}
          </div>
        `)}
      </div>
    `
  }

  addConfiguredRule() {
    if (!this.pendingRule || this.pendingRulePath === null) return

    // Navigate to the right group using path
    let group = this.editingAutomation._rules
    for (const idx of this.pendingRulePath) {
      group = group.items[idx]
    }

    // Transform rule before adding
    let rule = { ...this.pendingRule }

    // Special handling for scheduled trigger: convert active_hours_start/end to tuple
    if (rule.type === 'scheduled') {
      const start = rule.active_hours_start
      const end = rule.active_hours_end
      if (start !== undefined && start !== null && end !== undefined && end !== null) {
        rule.active_hours = [parseInt(start), parseInt(end)]
      }
      delete rule.active_hours_start
      delete rule.active_hours_end
    }

    // Add rule
    if (!group.items) group.items = []
    group.items.push(rule)

    // Reset
    this.pendingRule = null
    this.pendingRulePath = null
    this.showingRuleConfig = false
    this.requestUpdate()
  }

  toggleRulesGroupOperator(path) {
    let group = this.editingAutomation._rules
    for (const idx of path) {
      group = group.items[idx]
    }
    group.operator = group.operator === 'and' ? 'or' : 'and'
    this.requestUpdate()
  }

  addRulesGroup(path) {
    let group = this.editingAutomation._rules
    for (const idx of path) {
      group = group.items[idx]
    }
    if (!group.items) group.items = []
    group.items.push({
      operator: 'and',
      items: []
    })
    this.requestUpdate()
  }

  removeRule(path) {
    const parentPath = path.slice(0, -1)
    const idx = path[path.length - 1]

    let group = this.editingAutomation._rules
    for (const i of parentPath) {
      group = group.items[i]
    }

    group.items = group.items.filter((_, i) => i !== idx)
    this.requestUpdate()
  }

  removeRulesGroup(path) {
    this.removeRule(path) // Same logic
  }

  /**
   * Split les règles unifiées en triggers et conditions pour le backend
   */
  /**
   * Normalize item fields for backend (convert strings to numbers where needed)
   */
  normalizeForBackend(item) {
    const normalized = { ...item }
    delete normalized._category

    // Convert string arrays to number arrays for specific fields
    if (normalized.days && Array.isArray(normalized.days)) {
      normalized.days = normalized.days.map(d => parseInt(d)).filter(d => !isNaN(d))
    }
    if (normalized.months && Array.isArray(normalized.months)) {
      normalized.months = normalized.months.map(m => parseInt(m)).filter(m => !isNaN(m))
    }

    // Convert hour fields to numbers
    if (normalized.start_hour !== undefined) {
      normalized.start_hour = parseInt(normalized.start_hour)
    }
    if (normalized.end_hour !== undefined) {
      normalized.end_hour = parseInt(normalized.end_hour)
    }
    if (normalized.active_hours_start !== undefined) {
      normalized.active_hours_start = parseInt(normalized.active_hours_start)
    }
    if (normalized.active_hours_end !== undefined) {
      normalized.active_hours_end = parseInt(normalized.active_hours_end)
    }

    // Convert interval to number
    if (normalized.interval_seconds !== undefined) {
      normalized.interval_seconds = parseInt(normalized.interval_seconds)
    }

    return normalized
  }

  splitRulesForBackend(rulesGroup) {
    const triggers = { operator: rulesGroup.operator, triggers: [] }
    const conditions = { operator: rulesGroup.operator, conditions: [] }

    for (const item of (rulesGroup.items || [])) {
      if (item.operator && item.items) {
        // Nested group - recursive split
        const nested = this.splitRulesForBackend(item)
        if (nested.triggers.triggers.length > 0) {
          triggers.triggers.push({ operator: item.operator, triggers: nested.triggers.triggers })
        }
        if (nested.conditions.conditions.length > 0) {
          conditions.conditions.push({ operator: item.operator, conditions: nested.conditions.conditions })
        }
      } else if (isEventType(item.type)) {
        // Trigger (event-based)
        triggers.triggers.push(this.normalizeForBackend(item))
      } else if (isStateType(item.type)) {
        // Condition (state-based)
        conditions.conditions.push(this.normalizeForBackend(item))
      }
    }

    return { triggers, conditions }
  }

  /**
   * Vérifie si les règles contiennent au moins un événement (trigger)
   */
  hasEventInRules(rulesGroup) {
    if (!rulesGroup?.items?.length) return false
    for (const item of rulesGroup.items) {
      if (item.operator && item.items) {
        if (this.hasEventInRules(item)) return true
      } else if (isEventType(item.type)) {
        return true
      }
    }
    return false
  }

  // ========== Trigger Editor Methods (AND/OR Logic) - LEGACY ==========

  renderTriggersSection(auto) {
    const triggersGroup = auto.triggers || { operator: 'or', triggers: [] }
    const hasTriggers = triggersGroup.triggers?.length > 0

    return html`
      <div class="form-group">
        <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.25rem;">
          <label style="margin: 0;">Déclencheurs ${hasTriggers ? `(${triggersGroup.triggers.length})` : ''}</label>
          <button
            type="button"
            @click="${() => this.showHelp('triggers')}"
            style="cursor: pointer; font-size: 0.75rem; font-weight: bold; min-width: 22px; height: 22px; padding: 0 6px; border-radius: 11px; background: #3b82f6; border: none; display: inline-flex; align-items: center; justify-content: center; color: #fff; box-shadow: 0 2px 4px rgba(59,130,246,0.4);"
          >?</button>
        </div>

        <div class="triggers-editor" style="padding: 0.75rem; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px solid rgba(255,255,255,0.08);">
          ${this.renderTriggerGroup(triggersGroup, [], 0)}
        </div>
      </div>
    `
  }

  renderTriggerGroup(group, path, depth) {
    const operatorLabel = group.operator === 'and' ? 'ET' : 'OU'
    const operatorColor = group.operator === 'and' ? '#3b82f6' : '#f59e0b'
    const operatorHint = group.operator === 'and'
      ? 'Tous les déclencheurs doivent correspondre'
      : 'Au moins un déclencheur doit correspondre'
    const indent = depth * 12

    return html`
      <div class="trigger-group" style="margin-left: ${indent}px; ${depth > 0 ? 'margin-top: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.02); border-radius: 6px; border: 1px dashed rgba(255,255,255,0.1);' : ''}">
        <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
          <button
            class="btn btn-small"
            style="font-size: 0.7rem; padding: 0.2rem 0.5rem; background: ${operatorColor}; color: white; border: none;"
            @click="${() => this.toggleTriggerGroupOperator(path)}"
            title="Cliquer pour basculer AND/OR"
          >${operatorLabel}</button>
          ${depth > 0 ? html`
            <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeTriggerGroup(path)}" title="Supprimer groupe">✕</button>
          ` : ''}
          <span style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">
            ${operatorHint}
          </span>
        </div>

        ${group.triggers?.map((trigger, idx) => {
          const triggerPath = [...path, idx]
          if (trigger.operator && trigger.triggers) {
            // Nested group
            return this.renderTriggerGroup(trigger, triggerPath, depth + 1)
          } else {
            // Single trigger
            return this.renderTriggerItem(trigger, triggerPath)
          }
        })}

        <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
          <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.showTriggerConfigFor(path)}">
            + Déclencheur
          </button>
          ${depth < 1 ? html`
            <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.addTriggerGroup(path)}">
              + Groupe
            </button>
          ` : ''}
        </div>

        ${this.showingTriggerConfig && JSON.stringify(this.pendingTriggerPath) === JSON.stringify(path) ? html`
          <div style="margin-top: 0.75rem; padding: 0.75rem; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.1);">
            ${this.renderTriggerConfig()}
            <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
              <button class="btn btn-small" @click="${() => this.showingTriggerConfig = false}">Annuler</button>
              <button class="btn btn-small btn-primary" @click="${this.addConfiguredTrigger}">Ajouter</button>
            </div>
          </div>
        ` : ''}
      </div>
    `
  }

  renderTriggerItem(trigger, path) {
    const triggerSchema = this.schema?.triggers?.find(t => t.type === trigger.type)
    const label = this.getTriggerLabel(trigger, triggerSchema)

    return html`
      <div class="trigger-item" style="display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.6rem; margin: 0.25rem 0; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.08);">
        <span style="flex: 1; font-size: 0.8rem;">${label}</span>
        <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeTrigger(path)}" title="Supprimer">✕</button>
      </div>
    `
  }

  getTriggerLabel(trigger, schema) {
    const icon = schema?.icon || '⚡'
    switch (trigger.type) {
      case 'mode_change':
        const fromMode = this.schema?.dynamic_values?.modes?.find(m => m.value === trigger.from_mode)
        const toMode = this.schema?.dynamic_values?.modes?.find(m => m.value === trigger.to_mode)
        if (trigger.from_mode && trigger.to_mode) {
          return `${icon} Mode: ${fromMode?.label || trigger.from_mode} → ${toMode?.label || trigger.to_mode}`
        } else if (trigger.to_mode) {
          return `${icon} Mode → ${toMode?.label || trigger.to_mode}`
        } else if (trigger.from_mode) {
          return `${icon} Mode: ${fromMode?.label || trigger.from_mode} → *`
        }
        return `${icon} Changement de mode`
      case 'sensor_alert':
        const room = this.schema?.dynamic_values?.rooms?.find(r => r.value === trigger.room_id)
        const level = this.schema?.dynamic_values?.alert_levels?.find(l => l.value === trigger.alert_level)
        return `${icon} Alerte ${level?.label || 'capteur'} ${room ? `(${room.label})` : ''}`
      case 'agent_status':
        const agent = this.schema?.dynamic_values?.agents?.find(a => a.value === trigger.agent_id)
        return `${icon} Agent ${agent?.label || trigger.agent_id || '*'}: ${trigger.status || '*'}`
      case 'manual':
        return `${icon} Déclenchement manuel`
      case 'plugin_health':
        const plugin = this.schema?.dynamic_values?.plugins?.find(p => p.value === trigger.plugin_name)
        const status = this.schema?.dynamic_values?.plugin_health_statuses?.find(s => s.value === trigger.status)
        return `${icon} Plugin ${plugin?.label || trigger.plugin_name || '*'}: ${status?.label || trigger.status || '*'}`
      case 'scheduled':
        const intervalSecs = trigger.interval_seconds || 300
        const intervalLabel = intervalSecs >= 3600
          ? `${Math.round(intervalSecs / 3600)}h`
          : intervalSecs >= 60
            ? `${Math.round(intervalSecs / 60)}min`
            : `${intervalSecs}s`
        const activeHoursLabel = trigger.active_hours
          ? ` (${trigger.active_hours[0]}h-${trigger.active_hours[1]}h)`
          : ''
        return `${icon} Planifié toutes les ${intervalLabel}${activeHoursLabel}`
      default:
        return `${icon} ${schema?.label || trigger.type}`
    }
  }

  showTriggerConfigFor(path) {
    this.pendingTriggerPath = path
    this.pendingTriggerType = this.schema?.triggers?.[0]?.type || 'mode_change'
    const triggerSchema = this.schema?.triggers?.find(t => t.type === this.pendingTriggerType)
    this.pendingTrigger = this.initializeWithDefaults(this.pendingTriggerType, triggerSchema?.fields)
    this.showingTriggerConfig = true
    this.requestUpdate()
  }

  renderTriggerConfig() {
    const type = this.pendingTriggerType
    const triggerSchema = this.schema?.triggers?.find(t => t.type === type)

    return html`
      <div style="display: flex; flex-direction: column; gap: 0.5rem;">
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Type de déclencheur</label>
          <select class="form-input" style="font-size: 0.8rem;"
            @change="${e => {
              this.pendingTriggerType = e.target.value
              const newSchema = this.schema?.triggers?.find(t => t.type === e.target.value)
              this.pendingTrigger = this.initializeWithDefaults(e.target.value, newSchema?.fields)
              this.requestUpdate()
            }}">
            ${(this.schema?.triggers || []).map(t => html`
              <option value="${t.type}" ?selected="${t.type === type}">${t.icon || ''} ${t.label}</option>
            `)}
          </select>
          ${triggerSchema?.description ? html`
            <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary); margin-top: 0.25rem;">${triggerSchema.description}</div>
          ` : ''}
        </div>

        ${triggerSchema?.fields?.map(field => html`
          <div class="form-group" style="margin-bottom: 0;">
            <label style="font-size: 0.75rem;">${field.label}${field.required ? ' *' : ''}</label>
            ${this.renderSchemaField(field, this.pendingTrigger?.[field.name], (val) => {
              this.pendingTrigger = { ...this.pendingTrigger, [field.name]: val }
              this.requestUpdate()
            })}
          </div>
        `)}
      </div>
    `
  }

  addConfiguredTrigger() {
    if (!this.pendingTrigger || this.pendingTriggerPath === null) return

    // Navigate to the right group using path
    let group = this.editingAutomation.triggers
    for (const idx of this.pendingTriggerPath) {
      group = group.triggers[idx]
    }

    // Transform trigger before adding
    let trigger = { ...this.pendingTrigger }

    // Special handling for scheduled trigger: convert active_hours_start/end to tuple
    if (trigger.type === 'scheduled') {
      const start = trigger.active_hours_start
      const end = trigger.active_hours_end
      if (start !== undefined && start !== null && end !== undefined && end !== null) {
        trigger.active_hours = [parseInt(start), parseInt(end)]
      }
      delete trigger.active_hours_start
      delete trigger.active_hours_end
    }

    // Add trigger
    if (!group.triggers) group.triggers = []
    group.triggers.push(trigger)

    // Reset
    this.pendingTrigger = null
    this.pendingTriggerPath = null
    this.showingTriggerConfig = false
    this.requestUpdate()
  }

  toggleTriggerGroupOperator(path) {
    let group = this.editingAutomation.triggers
    for (const idx of path) {
      group = group.triggers[idx]
    }
    group.operator = group.operator === 'and' ? 'or' : 'and'
    this.requestUpdate()
  }

  addTriggerGroup(path) {
    let group = this.editingAutomation.triggers
    for (const idx of path) {
      group = group.triggers[idx]
    }
    if (!group.triggers) group.triggers = []
    group.triggers.push({
      operator: 'and',
      triggers: []
    })
    this.requestUpdate()
  }

  removeTrigger(path) {
    const parentPath = path.slice(0, -1)
    const idx = path[path.length - 1]

    let group = this.editingAutomation.triggers
    for (const i of parentPath) {
      group = group.triggers[i]
    }

    group.triggers = group.triggers.filter((_, i) => i !== idx)
    this.requestUpdate()
  }

  removeTriggerGroup(path) {
    this.removeTrigger(path) // Same logic
  }

  // ========== Condition Editor Methods ==========

  renderConditionsSection(auto) {
    const conditions = auto.conditions || null
    const hasConditions = conditions !== null // Show editor once initialized (even if empty)
    const conditionCount = conditions?.conditions?.length || 0

    return html`
      <div class="form-group">
        <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.25rem;">
          <div style="display: flex; align-items: center; gap: 0.5rem;">
            <label style="margin: 0;">Conditions ${conditionCount > 0 ? `(${conditionCount})` : '(optionnel)'}</label>
            <button
              type="button"
              @click="${() => this.showHelp('conditions')}"
              style="cursor: pointer; font-size: 0.75rem; font-weight: bold; min-width: 22px; height: 22px; padding: 0 6px; border-radius: 11px; background: #3b82f6; border: none; display: inline-flex; align-items: center; justify-content: center; color: #fff; box-shadow: 0 2px 4px rgba(59,130,246,0.4);"
            >?</button>
          </div>
          ${hasConditions ? html`
            <button type="button" class="btn btn-small btn-danger" @click="${() => this.clearConditions()}" style="font-size: 0.7rem; padding: 0.2rem 0.5rem;">
              Supprimer
            </button>
          ` : html`
            <button type="button" class="btn btn-small" @click="${() => this.initConditions()}" style="font-size: 0.75rem;">
              + Ajouter
            </button>
          `}
        </div>

        ${hasConditions ? html`
          <div class="conditions-editor" style="padding: 0.75rem; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px solid rgba(255,255,255,0.08);">
            ${this.renderConditionGroup(conditions, [], 0)}
          </div>
        ` : ''}
      </div>
    `
  }

  initConditions() {
    this.editingAutomation.conditions = {
      operator: 'and',
      conditions: []
    }
    this.requestUpdate()
  }

  clearConditions() {
    this.editingAutomation.conditions = null
    this.showingConditionConfig = false
    this.requestUpdate()
  }

  showHelp(topic) {
    const helpTexts = {
      rules: `RÈGLES D'AUTOMATION

Combinez des ÉVÉNEMENTS (⚡) et des ÉTATS (📋) :

⚡ ÉVÉNEMENTS (déclencheurs)
  Mode change, Alerte capteur, Agent status, Planifié...
  → Provoquent l'exécution de l'automation

📋 ÉTATS (conditions)
  Mode actuel, Plage horaire, Jour de semaine...
  → Vérifient l'état AVANT d'exécuter

OPÉRATEURS :
• ET (bleu) = Toutes les règles doivent correspondre
• OU (orange) = Au moins une règle doit correspondre

EXEMPLE :
  (⚡ Planifié 5min) ET (📋 Heure 9h-18h) ET (📋 Jour Lun-Ven)

  = Toutes les 5 minutes, SI entre 9h-18h ET jour de semaine,
    alors exécuter les actions.

Au moins un événement (⚡) est requis pour déclencher l'automation.`,

      triggers: `DÉCLENCHEURS

Événements qui lancent l'automation.

• OU (orange) = Au moins un déclencheur doit correspondre
• ET (bleu) = Tous les déclencheurs doivent correspondre

Cliquez sur OU/ET pour basculer.
Utilisez "+ Groupe" pour des combinaisons complexes.

Exemple :
  Mode → Focus OU Agent offline
  = Se déclenche si l'un des deux arrive`,

      conditions: `CONDITIONS (optionnel)

Vérifications supplémentaires AVANT l'exécution.

Exemple :
  Déclencheur : Mode → Focus
  Conditions : Heure 9h-18h ET Jour Lun-Ven

  = L'automation se déclenche quand le mode passe en Focus,
    MAIS seulement si c'est un jour de semaine entre 9h-18h.

• ET (bleu) = Toutes les conditions doivent être vraies
• OU (orange) = Au moins une condition doit être vraie`
    }

    alert(helpTexts[topic] || 'Aide non disponible')
  }

  renderConditionGroup(group, path, depth) {
    const operatorLabel = group.operator === 'and' ? 'ET' : 'OU'
    const operatorColor = group.operator === 'and' ? '#3b82f6' : '#f59e0b'
    const indent = depth * 12

    return html`
      <div class="condition-group" style="margin-left: ${indent}px; ${depth > 0 ? 'margin-top: 0.5rem; padding: 0.5rem; background: rgba(255,255,255,0.02); border-radius: 6px; border: 1px dashed rgba(255,255,255,0.1);' : ''}">
        <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
          <button
            class="btn btn-small"
            style="font-size: 0.7rem; padding: 0.2rem 0.5rem; background: ${operatorColor}; color: white; border: none;"
            @click="${() => this.toggleGroupOperator(path)}"
            title="Cliquer pour basculer AND/OR"
          >${operatorLabel}</button>
          ${depth > 0 ? html`
            <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeConditionGroup(path)}" title="Supprimer groupe">✕</button>
          ` : ''}
          <span style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">
            ${group.operator === 'and' ? 'Toutes les conditions doivent être vraies' : 'Au moins une condition doit être vraie'}
          </span>
        </div>

        ${group.conditions?.map((cond, idx) => {
          const condPath = [...path, idx]
          if (cond.type === 'group' || (cond.operator && cond.conditions)) {
            // Nested group
            return this.renderConditionGroup(cond, condPath, depth + 1)
          } else {
            // Single condition
            return this.renderConditionItem(cond, condPath)
          }
        })}

        <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
          <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.showConditionConfigFor(path)}">
            + Condition
          </button>
          <button class="btn btn-small" style="font-size: 0.7rem;" @click="${() => this.addConditionGroup(path)}">
            + Groupe
          </button>
        </div>

        ${this.showingConditionConfig && JSON.stringify(this.pendingConditionPath) === JSON.stringify(path) ? html`
          <div style="margin-top: 0.75rem; padding: 0.75rem; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.1);">
            ${this.renderConditionConfig()}
            <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
              <button class="btn btn-small" @click="${() => this.showingConditionConfig = false}">Annuler</button>
              <button class="btn btn-small btn-primary" @click="${this.addConfiguredCondition}">Ajouter</button>
            </div>
          </div>
        ` : ''}
      </div>
    `
  }

  renderConditionItem(cond, path) {
    const condSchema = this.schema?.conditions?.find(c => c.type === cond.type)
    const label = this.getConditionLabel(cond, condSchema)

    return html`
      <div class="condition-item" style="display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.6rem; margin: 0.25rem 0; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(255,255,255,0.08);">
        <span style="flex: 1; font-size: 0.8rem;">${label}</span>
        <button class="btn btn-small btn-icon btn-danger" style="font-size: 0.6rem; padding: 0.15rem 0.4rem;" @click="${() => this.removeCondition(path)}" title="Supprimer">✕</button>
      </div>
    `
  }

  getConditionLabel(cond, schema) {
    const icon = '🔍'
    switch (cond.type) {
      case 'current_mode':
        const modeOpt = this.schema?.dynamic_values?.modes?.find(m => m.value === cond.mode)
        return `${icon} Mode = ${modeOpt?.label || cond.mode || '?'}`
      case 'time_range':
        return `${icon} Heure ${cond.start_time || '?'} - ${cond.end_time || '?'}`
      case 'day_of_week':
        const days = cond.days?.map(d => ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam'][d]).join(', ')
        return `${icon} Jours: ${days || '?'}`
      case 'day_of_month':
        const monthDays2 = cond.days?.map(d => d === 31 ? 'Dernier' : d).join(', ')
        return `${icon} Jour du mois: ${monthDays2 || 'Aucun'}`
      case 'month':
        const monthNames2 = ['', 'Jan', 'Fév', 'Mar', 'Avr', 'Mai', 'Juin', 'Juil', 'Août', 'Sep', 'Oct', 'Nov', 'Déc']
        const monthsVal = cond.months?.map(m => monthNames2[m]).join(', ')
        return `${icon} Mois: ${monthsVal || 'Tous'}`
      case 'sensor_value':
        return `${icon} Capteur ${cond.sensor_id || '?'} ${cond.metric || '?'} ${cond.operator || '?'} ${cond.value ?? '?'}`
      case 'agent_online':
        return `${icon} Agent ${cond.agent_id || '?'} en ligne`
      case 'feature':
        const op = cond.operator === 'equals' ? '=' : cond.operator === 'not_equals' ? '≠' : cond.operator
        return `${icon} ${cond.feature_id || '?'} ${op} ${cond.value ?? '?'}`
      default:
        return `${icon} ${schema?.label || cond.type}`
    }
  }

  showConditionConfigFor(path) {
    this.pendingConditionPath = path
    this.pendingConditionType = this.schema?.conditions?.[0]?.type || 'current_mode'
    const condSchema = this.schema?.conditions?.find(c => c.type === this.pendingConditionType)
    this.pendingCondition = this.initializeWithDefaults(this.pendingConditionType, condSchema?.fields)
    this.showingConditionConfig = true
    this.requestUpdate()
  }

  renderConditionConfig() {
    const type = this.pendingConditionType
    const condSchema = this.schema?.conditions?.find(c => c.type === type)

    return html`
      <div style="display: flex; flex-direction: column; gap: 0.5rem;">
        <div class="form-group" style="margin-bottom: 0;">
          <label style="font-size: 0.75rem;">Type de condition</label>
          <select class="form-input" style="font-size: 0.8rem;"
            @change="${e => {
              this.pendingConditionType = e.target.value
              const newSchema = this.schema?.conditions?.find(c => c.type === e.target.value)
              this.pendingCondition = this.initializeWithDefaults(e.target.value, newSchema?.fields)
              this.requestUpdate()
            }}">
            ${(this.schema?.conditions || []).map(c => html`
              <option value="${c.type}" ?selected="${c.type === type}">${c.icon || ''} ${c.label}</option>
            `)}
          </select>
        </div>

        ${condSchema?.fields?.map(field => html`
          <div class="form-group" style="margin-bottom: 0;">
            <label style="font-size: 0.75rem;">${field.label}${field.required ? ' *' : ''}</label>
            ${this.renderSchemaField(field, this.pendingCondition?.[field.name], (val) => {
              this.pendingCondition = { ...this.pendingCondition, [field.name]: val }
              this.requestUpdate()
            })}
          </div>
        `)}
      </div>
    `
  }

  addConfiguredCondition() {
    if (!this.pendingCondition || !this.pendingConditionPath) return

    // Navigate to the right group using path
    let group = this.editingAutomation.conditions
    for (const idx of this.pendingConditionPath) {
      group = group.conditions[idx]
    }

    // Add condition
    if (!group.conditions) group.conditions = []
    group.conditions.push({ ...this.pendingCondition })

    // Reset
    this.pendingCondition = null
    this.pendingConditionPath = null
    this.showingConditionConfig = false
    this.requestUpdate()
  }

  toggleGroupOperator(path) {
    let group = this.editingAutomation.conditions
    for (const idx of path) {
      group = group.conditions[idx]
    }
    group.operator = group.operator === 'and' ? 'or' : 'and'
    this.requestUpdate()
  }

  addConditionGroup(path) {
    let group = this.editingAutomation.conditions
    for (const idx of path) {
      group = group.conditions[idx]
    }
    if (!group.conditions) group.conditions = []
    group.conditions.push({
      operator: 'and',
      conditions: []
    })
    this.requestUpdate()
  }

  removeCondition(path) {
    const parentPath = path.slice(0, -1)
    const idx = path[path.length - 1]

    let group = this.editingAutomation.conditions
    for (const i of parentPath) {
      group = group.conditions[i]
    }
    group.conditions.splice(idx, 1)
    this.requestUpdate()
  }

  removeConditionGroup(path) {
    this.removeCondition(path) // Same logic
  }

  // ========== End Condition Editor ==========

  renderAutomationCard(auto) {
    // Support both old trigger and new triggers format
    let triggerLabel = 'Aucun'
    let triggerCount = 0
    if (auto.triggers?.triggers?.length > 0) {
      triggerCount = auto.triggers.triggers.length
      if (triggerCount === 1) {
        const t = auto.triggers.triggers[0]
        triggerLabel = this.getShortTriggerLabel(t)
      } else {
        const op = auto.triggers.operator === 'and' ? 'ET' : 'OU'
        triggerLabel = `${triggerCount} déclencheurs (${op})`
      }
    } else if (auto.trigger) {
      triggerCount = 1
      triggerLabel = this.getShortTriggerLabel(auto.trigger)
    }

    // Category info
    const category = auto.category || 'custom'
    const categoryIcons = {
      comfort: '🛋️', security: '🔒', energy: '⚡',
      notifications: '🔔', custom: '⚙️'
    }
    const categoryIcon = categoryIcons[category] || '⚙️'
    const statusIcon = auto.enabled ? '⚡' : '💤'

    // Find last execution for this automation
    const lastExec = this.automationHistory.find(h => h.automation_id === auto.id)
    const lastExecTime = lastExec ? this.formatTime(lastExec.executed_at) : null

    // Check if highlighted from timeline
    const isHighlighted = this.highlightedAutomationId === auto.id

    return html`
      <div class="automation-card ${auto.enabled ? 'enabled' : 'disabled'} ${isHighlighted ? 'highlighted' : ''}">
        <div class="automation-card-inner">
          <div class="automation-header">
            <div class="automation-status-icon">${statusIcon}</div>
            <div class="automation-info">
              <div class="automation-title-row">
                <span class="automation-title">${auto.name}</span>
                ${auto.trusted ? html`<span class="automation-trust-badge" title="Auto-approuvée sans validation">🛡️</span>` : ''}
                <span class="automation-category-badge ${category}">${categoryIcon} ${category}</span>
              </div>
              <div class="automation-subtitle">
                <span>${triggerLabel}</span>
              </div>
            </div>
            <div class="automation-actions">
              <div
                class="toggle ${auto.enabled ? 'active' : ''}"
                @click="${() => this.toggleAutomation(auto.id)}"
                title="${auto.enabled ? 'Désactiver' : 'Activer'}"
              ></div>
            </div>
          </div>

          <div class="automation-details">
            <div class="automation-detail">
              <span class="automation-detail-icon">🎯</span>
              <span class="automation-detail-value">${auto.actions?.length || 0}</span>
              <span>action${(auto.actions?.length || 0) !== 1 ? 's' : ''}</span>
            </div>
            <div class="automation-detail">
              <span class="automation-detail-icon">⏱️</span>
              <span class="automation-detail-value">${auto.cooldown_seconds || 0}s</span>
              <span>cooldown</span>
            </div>
            ${lastExecTime ? html`
              <div class="automation-detail">
                <span class="automation-detail-icon">🕐</span>
                <span>${lastExecTime}</span>
              </div>
            ` : ''}

            <div class="automation-quick-actions">
              <button class="quick-action-btn play" @click="${() => this.runAutomationManually(auto.id)}" title="Exécuter maintenant">
                ▶
              </button>
              <button class="quick-action-btn" @click="${() => this.openEditForm(auto)}" title="Modifier">
                ✏️
              </button>
              <button class="quick-action-btn" @click="${() => this.deleteAutomation(auto.id)}" title="Supprimer">
                🗑️
              </button>
            </div>
          </div>
        </div>
      </div>
    `
  }

  async runAutomationManually(automationId) {
    try {
      await csrfService.fetchWithCsrf(`/v1/automations/${automationId}/run`, {
        method: 'POST'
      })
      this.showToast('Automation exécutée', 'success')
      // Refresh history
      await this.loadAutomations()
    } catch (e) {
      console.error('[context-engine] Failed to run automation:', e)
      this.showToast('Erreur lors de l\'exécution', 'error')
    }
  }

  getShortTriggerLabel(trigger) {
    if (!trigger?.type) return 'Inconnu'
    switch (trigger.type) {
      case 'mode_change':
        return `Mode: ${trigger.from_mode || '*'} → ${trigger.to_mode || '*'}`
      case 'agent_status':
        return `Agent ${trigger.agent_id || '*'} → ${trigger.status || '*'}`
      case 'sensor_alert':
        return `Capteur ${trigger.room_id || '*'}: ${trigger.alert_level || '*'}`
      case 'manual':
        return 'Manuel'
      case 'plugin_health':
        return `Plugin ${trigger.plugin_name || '*'}: ${trigger.status || '*'}`
      default:
        return trigger.type
    }
  }

  renderHistoryItem(h) {
    const statusClass = h.success ? 'success' : 'failed'
    const statusIcon = h.success ? '✓' : '✗'
    const actionsCount = h.actions_executed || 0

    return html`
      <div class="history-item ${statusClass}">
        <div class="history-item-header">
          <div class="history-item-name">
            <span class="history-item-status">${statusIcon}</span>
            ${h.automation_name}
          </div>
          <span class="history-item-time">${this.formatTime(h.executed_at)}</span>
        </div>
        <div class="history-item-details">
          ${actionsCount > 0 ? html`
            <span>${actionsCount} action${actionsCount !== 1 ? 's' : ''}</span>
          ` : ''}
          ${h.trust_score != null ? html`
            <span class="trust-badge ${this.getTrustClass(h.trust_score)}">
              🧠 ${Math.round(h.trust_score * 100)}%
            </span>
          ` : ''}
          ${h.trigger_type ? html`
            <span style="opacity: 0.7;">via ${h.trigger_type}</span>
          ` : ''}
        </div>
      </div>
    `
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
      <div class="controls-title" style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 1rem;">
        <span style="font-size: 1.25rem; animation: float-icon 3s ease-in-out infinite;">⚖️</span>
        <span>Demandes en attente</span>
        <span style="padding: 0.25rem 0.75rem; background: rgba(245, 158, 11, 0.15); border: 1px solid rgba(245, 158, 11, 0.3); border-radius: 20px; font-size: 0.8rem; color: #f59e0b; font-weight: 600;">
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
        background: linear-gradient(135deg, rgba(255, 255, 255, 0.04) 0%, rgba(255, 255, 255, 0.02) 100%);
        border: 1px solid ${trustClass === 'high' ? 'rgba(34, 197, 94, 0.2)' : trustClass === 'medium' ? 'rgba(245, 158, 11, 0.2)' : 'rgba(239, 68, 68, 0.2)'};
      ">
        <div class="validation-header" style="display: flex; align-items: flex-start; gap: 1rem;">
          <!-- Mini Gauge -->
          <div style="flex-shrink: 0;">
            ${this.renderMiniGauge(trustScore, 70)}
          </div>

          <div class="validation-info" style="flex: 1; min-width: 0;">
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
              <span style="font-size: 1.25rem;">⚡</span>
              <div class="validation-title" style="font-size: 1.1rem; font-weight: 600; color: var(--color-dark-text-primary);">${actionType}</div>
            </div>
            <div class="validation-subtitle" style="font-size: 0.85rem; color: var(--color-dark-text-secondary); display: flex; align-items: center; gap: 0.5rem;">
              <span style="padding: 0.2rem 0.5rem; background: rgba(255,255,255,0.05); border-radius: 6px; font-size: 0.75rem;">
                🤖 ${v.action?.agent_id || 'Système'}
              </span>
              <span style="padding: 0.2rem 0.5rem; background: ${glowColor}; border-radius: 6px; font-size: 0.75rem; color: ${color}; font-weight: 600;">
                Seuil: ${Math.round((v.threshold || 0.7) * 100)}%
              </span>
            </div>
          </div>
        </div>

        <div class="validation-reasons" style="
          background: rgba(0, 0, 0, 0.2);
          border: 1px solid rgba(255,255,255,0.05);
          border-radius: 12px;
          padding: 1rem;
          margin: 1rem 0;
        ">
          <div class="validation-reasons-title" style="font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-dark-text-tertiary); margin-bottom: 0.75rem; display: flex; align-items: center; gap: 0.5rem;">
            📋 Raisons de la validation
          </div>
          ${reasons.map((r, i) => html`
            <div class="validation-reason-item" style="
              display: flex;
              align-items: flex-start;
              gap: 0.75rem;
              padding: 0.5rem 0;
              animation: card-enter 0.3s ease-out ${(index * 0.1) + (i * 0.05)}s backwards;
            ">
              <div style="
                width: 8px;
                height: 8px;
                border-radius: 50%;
                background: ${color};
                box-shadow: 0 0 8px ${glowColor};
                margin-top: 0.35rem;
                flex-shrink: 0;
              "></div>
              <span style="font-size: 0.875rem; color: var(--color-dark-text-secondary); line-height: 1.4;">${r}</span>
            </div>
          `)}
        </div>

        <div class="validation-actions" style="display: flex; gap: 1rem;">
          <button @click="${() => this.handleRejectValidation(v.validation_id)}" style="
            flex: 1;
            padding: 0.875rem 1.25rem;
            background: rgba(239, 68, 68, 0.1);
            border: 1px solid rgba(239, 68, 68, 0.3);
            border-radius: 12px;
            color: #ef4444;
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
          ">
            <span style="font-size: 1.1rem;">✗</span> Rejeter
          </button>
          <button @click="${() => this.handleApproveValidation(v.validation_id)}" style="
            flex: 1;
            padding: 0.875rem 1.25rem;
            background: linear-gradient(135deg, rgba(34, 197, 94, 0.2) 0%, rgba(34, 197, 94, 0.1) 100%);
            border: 1px solid rgba(34, 197, 94, 0.4);
            border-radius: 12px;
            color: #22c55e;
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            box-shadow: 0 0 20px rgba(34, 197, 94, 0.15);
          ">
            <span style="font-size: 1.1rem;">✓</span> Approuver
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
      <div style="position: relative; width: ${size}px; height: ${size}px;">
        <svg width="${size}" height="${size}" viewBox="0 0 50 50" style="transform: rotate(-90deg);">
          <circle cx="25" cy="25" r="${radius}" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="5"/>
          <circle cx="25" cy="25" r="${radius}" fill="none" stroke="${color}" stroke-width="5"
            stroke-linecap="round"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            style="transition: stroke-dashoffset 0.8s ease-out; filter: drop-shadow(0 0 6px ${color}80);"/>
        </svg>
        <div style="position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;">
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

  renderStatsTab() {
    const stats = this.stats?.mode_stats || []
    const total = stats.reduce((sum, s) => sum + (s.duration_minutes || 0), 0)

    return html`
      <div class="controls-title">Temps par mode (24h)</div>
      ${stats.map(s => {
        const pct = total > 0 ? (s.duration_minutes / total) * 100 : 0
        // Prefer mode_slug if available
        const mode = s.mode_slug || s.mode?.toLowerCase() || 'veille'
        return html`
          <div class="stat-bar">
            <div class="stat-header">
              <span class="stat-label">${this.getModeIcon(mode)} ${this.getModeName(mode)}</span>
              <span class="stat-value">${this.formatDuration(s.duration_minutes || 0)} (${Math.round(pct)}%)</span>
            </div>
            <div class="stat-track">
              <div class="stat-fill ${mode}" style="width: ${pct}%"></div>
            </div>
          </div>
        `
      })}

      <!-- Patterns section removed - now displayed in Intelligence tab -->
    `
  }

  renderNotificationsTab() {
    // Group by category
    const categories = {
      PluginHealth: { name: 'Santé Plugins', icon: '🔌', configs: [] },
      Environment: { name: 'Environnement', icon: '🌡️', configs: [] },
      Automation: { name: 'Automations', icon: '⚙️', configs: [] },
      Security: { name: 'Sécurité', icon: '🔒', configs: [] },
      System: { name: 'Système', icon: '🖥️', configs: [] },
    }

    for (const config of this.notificationConfigs) {
      const cat = categories[config.category] || categories.System
      cat.configs.push(config)
    }

    const priorityColors = {
      P0: '#ef4444',
      P1: '#f59e0b',
      P2: '#3b82f6'
    }

    return html`
      <!-- Help Section (collapsible) -->
      <div class="config-section" style="background: rgba(147, 51, 234, 0.1); border: 1px solid rgba(147, 51, 234, 0.3);">
        <div class="config-title" style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer; user-select: none;"
          @click="${() => this.showNotifHelp = !this.showNotifHelp}">
          <span style="background: rgba(147, 51, 234, 0.3); width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.85rem;">?</span>
          Variables disponibles
          <span style="margin-left: auto; font-size: 0.75rem; opacity: 0.6;">${this.showNotifHelp ? '▼' : '▶'}</span>
        </div>
        ${this.showNotifHelp ? html`
        <div style="font-size: 0.8rem; color: var(--color-dark-text-secondary); line-height: 1.6; margin-top: 0.75rem;">
          <p style="margin: 0 0 0.75rem;">Utilisez <code style="background: rgba(0,0,0,0.3); padding: 0.1rem 0.3rem; border-radius: 4px;">{variable}</code> dans les templates pour insérer des valeurs dynamiques.</p>

          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem;">
            <p style="margin: 0 0 0.5rem; font-weight: 600;">Variables communes :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; font-size: 0.75rem;">
              <code style="color: #a78bfa;">{timestamp}</code><span>Date/heure de l'événement</span>
            </div>
          </div>

          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem;">
            <p style="margin: 0 0 0.5rem; font-weight: 600;">🔌 Plugins :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; font-size: 0.75rem;">
              <code style="color: #a78bfa;">{plugin_name}</code><span>Nom du plugin</span>
              <code style="color: #a78bfa;">{status}</code><span>État (online/offline/error)</span>
            </div>
          </div>

          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem;">
            <p style="margin: 0 0 0.5rem; font-weight: 600;">🌡️ Environnement :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; font-size: 0.75rem;">
              <code style="color: #a78bfa;">{room}</code><span>Nom de la pièce</span>
              <code style="color: #a78bfa;">{temperature}</code><span>Température actuelle</span>
              <code style="color: #a78bfa;">{humidity}</code><span>Humidité actuelle</span>
              <code style="color: #a78bfa;">{threshold}</code><span>Seuil déclenché</span>
            </div>
          </div>

          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem;">
            <p style="margin: 0 0 0.5rem; font-weight: 600;">⚙️ Automations :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; font-size: 0.75rem;">
              <code style="color: #a78bfa;">{automation_name}</code><span>Nom de l'automation</span>
              <code style="color: #a78bfa;">{trigger}</code><span>Type de déclencheur</span>
              <code style="color: #a78bfa;">{action}</code><span>Action exécutée</span>
              <code style="color: #a78bfa;">{mode}</code><span>Mode actuel</span>
            </div>
          </div>

          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem;">
            <p style="margin: 0 0 0.5rem; font-weight: 600;">🔒 Sécurité :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.75rem; font-size: 0.75rem;">
              <code style="color: #a78bfa;">{ip}</code><span>Adresse IP source</span>
              <code style="color: #a78bfa;">{attempts}</code><span>Nombre de tentatives</span>
              <code style="color: #a78bfa;">{username}</code><span>Nom d'utilisateur</span>
            </div>
          </div>

          <div style="background: rgba(16, 185, 129, 0.15); padding: 0.5rem; border-radius: 6px; border: 1px solid rgba(16, 185, 129, 0.3);">
            <p style="margin: 0; font-size: 0.75rem;">
              <strong>💡 Exemple :</strong><br>
              Titre: <code>⚠️ {room} - Humidité élevée</code><br>
              Corps: <code>Humidité à {humidity}% (seuil: {threshold}%)</code>
            </p>
          </div>
        </div>
        ` : ''}
      </div>

      <!-- Notification configs by category -->
      ${Object.entries(categories).filter(([_, cat]) => cat.configs.length > 0).map(([catKey, cat]) => html`
        <div class="config-section" style="margin-top: 1rem;">
          <div class="config-title" style="display: flex; align-items: center; gap: 0.5rem;">
            <span>${cat.icon}</span>
            ${cat.name}
            <span style="font-size: 0.7rem; opacity: 0.5; margin-left: auto;">${cat.configs.length} notifications</span>
          </div>

          <div style="display: flex; flex-direction: column; gap: 0.75rem; margin-top: 0.75rem;">
            ${cat.configs.map(config => html`
              <div class="notif-config-card" style="
                background: rgba(255,255,255,0.03);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 8px;
                padding: 0.75rem;
                ${!config.enabled ? 'opacity: 0.5;' : ''}
              ">
                <div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.5rem;">
                  <!-- Toggle -->
                  <label style="display: flex; align-items: center; cursor: pointer;">
                    <input type="checkbox"
                      ?checked="${config.enabled}"
                      @change="${e => this.toggleNotificationConfig(config.type_id, e.target.checked)}"
                      style="width: 16px; height: 16px; accent-color: var(--context-primary, #00d4aa);"
                    >
                  </label>

                  <!-- Name & Description -->
                  <div style="flex: 1; min-width: 0;">
                    <div style="font-weight: 500; font-size: 0.85rem; color: var(--color-dark-text-primary);">
                      ${config.display_name}
                    </div>
                    <div style="font-size: 0.7rem; color: var(--color-dark-text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                      ${config.description}
                    </div>
                  </div>

                  <!-- Priority badge -->
                  <span style="
                    padding: 0.15rem 0.4rem;
                    border-radius: 4px;
                    font-size: 0.65rem;
                    font-weight: 600;
                    background: ${priorityColors[config.priority]}20;
                    color: ${priorityColors[config.priority]};
                    border: 1px solid ${priorityColors[config.priority]}40;
                  ">${config.priority}</span>

                  <!-- Edit button -->
                  <button
                    @click="${() => this.editNotificationConfig(config)}"
                    style="
                      background: rgba(255,255,255,0.08);
                      border: none;
                      color: var(--color-dark-text-secondary);
                      padding: 0.35rem 0.5rem;
                      border-radius: 4px;
                      cursor: pointer;
                      font-size: 0.75rem;
                    "
                  >✏️</button>
                </div>

                <!-- Templates preview -->
                <div style="font-size: 0.7rem; color: var(--color-dark-text-secondary); padding-left: 1.75rem;">
                  <div style="margin-bottom: 0.2rem;">
                    <span style="opacity: 0.6;">Titre:</span>
                    <code style="background: rgba(0,0,0,0.2); padding: 0.1rem 0.3rem; border-radius: 3px;">${config.title_template}</code>
                  </div>
                  <div>
                    <span style="opacity: 0.6;">Corps:</span>
                    <span style="opacity: 0.8;">${config.body_template.length > 50 ? config.body_template.slice(0, 50) + '...' : config.body_template}</span>
                  </div>
                </div>
              </div>
            `)}
          </div>
        </div>
      `)}

      <!-- Edit modal -->
      ${this.editingNotifConfig ? html`
        <div class="modal-overlay" style="
          position: fixed;
          inset: 0;
          background: rgba(0,0,0,0.7);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 10000;
        " @click="${e => { if (e.target === e.currentTarget) this.editingNotifConfig = null }}">
          <div style="
            background: linear-gradient(135deg, rgba(30, 32, 40, 0.98) 0%, rgba(20, 22, 28, 1) 100%);
            border: 1px solid rgba(255,255,255,0.15);
            border-radius: 12px;
            padding: 1.25rem;
            width: 90%;
            max-width: 450px;
            max-height: 80vh;
            overflow-y: auto;
          ">
            <h3 style="margin: 0 0 1rem; font-size: 1rem; color: var(--color-dark-text-primary);">
              ✏️ Modifier "${this.editingNotifConfig.display_name}"
            </h3>

            <div style="display: flex; flex-direction: column; gap: 0.75rem;">
              <!-- Enabled -->
              <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                <input type="checkbox"
                  ?checked="${this.editingNotifConfig.enabled}"
                  @change="${e => this.editingNotifConfig = {...this.editingNotifConfig, enabled: e.target.checked}}"
                  style="width: 16px; height: 16px; accent-color: var(--context-primary, #00d4aa);"
                >
                <span style="font-size: 0.85rem; color: var(--color-dark-text-primary);">Activée</span>
              </label>

              <!-- Priority -->
              <div>
                <label style="font-size: 0.75rem; color: var(--color-dark-text-secondary); display: block; margin-bottom: 0.3rem;">Priorité</label>
                <select
                  .value="${this.editingNotifConfig.priority}"
                  @change="${e => this.editingNotifConfig = {...this.editingNotifConfig, priority: e.target.value}}"
                  style="
                    width: 100%;
                    padding: 0.5rem;
                    background: rgba(0,0,0,0.3);
                    border: 1px solid rgba(255,255,255,0.15);
                    border-radius: 6px;
                    color: var(--color-dark-text-primary);
                    font-size: 0.85rem;
                  "
                >
                  <option value="P0" style="background: #1a1a1a;">P0 - Critique (email + push)</option>
                  <option value="P1" style="background: #1a1a1a;">P1 - Important (push)</option>
                  <option value="P2" style="background: #1a1a1a;">P2 - Normal (push silencieux)</option>
                </select>
              </div>

              <!-- Title template -->
              <div>
                <label style="font-size: 0.75rem; color: var(--color-dark-text-secondary); display: block; margin-bottom: 0.3rem;">Template titre</label>
                <input type="text"
                  .value="${this.editingNotifConfig.title_template}"
                  @input="${e => this.editingNotifConfig = {...this.editingNotifConfig, title_template: e.target.value}}"
                  style="
                    width: 100%;
                    padding: 0.5rem;
                    background: rgba(0,0,0,0.3);
                    border: 1px solid rgba(255,255,255,0.15);
                    border-radius: 6px;
                    color: var(--color-dark-text-primary);
                    font-size: 0.85rem;
                    box-sizing: border-box;
                  "
                  placeholder="Ex: ⚠️ {room} - Alerte"
                >
              </div>

              <!-- Body template -->
              <div>
                <label style="font-size: 0.75rem; color: var(--color-dark-text-secondary); display: block; margin-bottom: 0.3rem;">Template corps</label>
                <textarea
                  .value="${this.editingNotifConfig.body_template}"
                  @input="${e => this.editingNotifConfig = {...this.editingNotifConfig, body_template: e.target.value}}"
                  rows="3"
                  style="
                    width: 100%;
                    padding: 0.5rem;
                    background: rgba(0,0,0,0.3);
                    border: 1px solid rgba(255,255,255,0.15);
                    border-radius: 6px;
                    color: var(--color-dark-text-primary);
                    font-size: 0.85rem;
                    resize: vertical;
                    box-sizing: border-box;
                  "
                  placeholder="Ex: Valeur actuelle: {value}"
                ></textarea>
              </div>

              <!-- Available variables -->
              ${this.editingNotifConfig.available_variables?.length > 0 ? html`
                <div style="background: rgba(147, 51, 234, 0.1); padding: 0.5rem; border-radius: 6px; border: 1px solid rgba(147, 51, 234, 0.2);">
                  <div style="font-size: 0.7rem; color: var(--color-dark-text-secondary); margin-bottom: 0.3rem;">Variables disponibles:</div>
                  <div style="display: flex; flex-wrap: wrap; gap: 0.25rem;">
                    ${this.editingNotifConfig.available_variables.map(v => html`
                      <code style="
                        background: rgba(0,0,0,0.3);
                        padding: 0.1rem 0.35rem;
                        border-radius: 3px;
                        font-size: 0.7rem;
                        color: #a78bfa;
                        cursor: help;
                      " title="${v.description}">{${v.name}}</code>
                    `)}
                  </div>
                </div>
              ` : ''}
            </div>

            <!-- Actions -->
            <div style="display: flex; gap: 0.5rem; margin-top: 1rem; justify-content: flex-end;">
              <button
                @click="${() => this.editingNotifConfig = null}"
                style="
                  padding: 0.5rem 1rem;
                  background: rgba(255,255,255,0.08);
                  border: 1px solid rgba(255,255,255,0.15);
                  border-radius: 6px;
                  color: var(--color-dark-text-secondary);
                  cursor: pointer;
                  font-size: 0.85rem;
                "
              >Annuler</button>
              <button
                @click="${() => this.saveNotificationConfig()}"
                style="
                  padding: 0.5rem 1rem;
                  background: var(--context-primary, #00d4aa);
                  border: none;
                  border-radius: 6px;
                  color: #000;
                  cursor: pointer;
                  font-size: 0.85rem;
                  font-weight: 500;
                "
              >Sauvegarder</button>
            </div>
          </div>
        </div>
      ` : ''}
    `
  }

  async toggleNotificationConfig(typeId, enabled) {
    try {
      const config = this.notificationConfigs.find(c => c.type_id === typeId)
      if (!config) return

      const res = await csrfService.fetchWithCsrf(`/v1/notification-types/${typeId}`, {
        method: 'PUT',
        body: JSON.stringify({ ...config, enabled })
      })

      if (res.ok) {
        await this.loadNotificationConfigs()
      }
    } catch (e) {
      console.error('[context-engine] Failed to toggle notification config:', e)
    }
  }

  editNotificationConfig(config) {
    this.editingNotifConfig = { ...config }
  }

  async saveNotificationConfig() {
    if (!this.editingNotifConfig) return

    try {
      const res = await csrfService.fetchWithCsrf(`/v1/notification-types/${this.editingNotifConfig.type_id}`, {
        method: 'PUT',
        body: JSON.stringify(this.editingNotifConfig)
      })

      if (res.ok) {
        this.editingNotifConfig = null
        await this.loadNotificationConfigs()
      }
    } catch (e) {
      console.error('[context-engine] Failed to save notification config:', e)
    }
  }

  renderIntelligenceTab() {
    const prediction = this.intelligencePrediction?.prediction
    const vector = this.intelligencePrediction?.vector || this.intelligenceVector?.vector
    const stats = this.intelligencePrediction?.stats
    const features = this.intelligenceFeatures?.features || []
    const summary = this.intelligenceFeatures?.summary

    // Group features by source
    const featuresBySource = {}
    features.forEach(f => {
      const source = f.source.split('.')[0] // agent, classifier, sensor
      if (!featuresBySource[source]) featuresBySource[source] = []
      featuresBySource[source].push(f)
    })

    return html`
      <div class="intelligence-tab" style="animation: card-enter 0.4s ease-out;">
        <!-- Prediction v2 Section -->
        <div class="section-card" style="
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.12) 0%, rgba(109, 40, 217, 0.06) 100%);
          border: 1px solid rgba(139, 92, 246, 0.25);
          border-radius: 16px;
          padding: 1.5rem;
          margin-bottom: 1.25rem;
          box-shadow: 0 4px 24px rgba(139, 92, 246, 0.1);
        ">
          <div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 1.25rem;">
            <span style="font-size: 1.5rem; animation: float-icon 3s ease-in-out infinite;">🧠</span>
            <h3 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: var(--color-dark-text-primary);">Intelligence v2</h3>
            ${stats ? html`
              <span style="margin-left: auto; padding: 0.35rem 0.875rem; border-radius: 20px; background: rgba(139, 92, 246, 0.15); border: 1px solid rgba(139, 92, 246, 0.3); color: #a78bfa; font-size: 0.75rem; font-weight: 600;">
                ${stats.total_samples} samples
              </span>
            ` : ''}
          </div>

          ${prediction ? html`
            <div style="display: flex; align-items: center; gap: 1.5rem; flex-wrap: wrap;">
              <!-- Confidence Gauge -->
              <div style="flex-shrink: 0;">
                ${this.renderConfidenceGauge(prediction.confidence, 130)}
              </div>

              <!-- Mode Info -->
              <div style="flex: 1; min-width: 200px;">
                <div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 1rem;">
                  <span style="font-size: 3rem; animation: float-icon 4s ease-in-out infinite;">${this.getModeIcon(prediction.mode)}</span>
                  <div>
                    <div style="font-size: 1.5rem; font-weight: 700; color: #a78bfa; margin-bottom: 0.25rem;">${this.getModeName(prediction.mode)}</div>
                    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap;">
                      <span style="display: inline-block; padding: 0.3rem 0.75rem; border-radius: 20px; font-size: 0.75rem; font-weight: 600;
                        background: ${prediction.is_confident ? 'linear-gradient(135deg, rgba(34, 197, 94, 0.2), rgba(34, 197, 94, 0.1))' : 'linear-gradient(135deg, rgba(251, 146, 60, 0.2), rgba(251, 146, 60, 0.1))'};
                        border: 1px solid ${prediction.is_confident ? 'rgba(34, 197, 94, 0.4)' : 'rgba(251, 146, 60, 0.4)'};
                        color: ${prediction.is_confident ? '#22c55e' : '#fb923c'};">
                        ${prediction.is_confident ? '✓ Confiant' : '⚠ Incertain'}
                      </span>
                      <span style="padding: 0.3rem 0.75rem; border-radius: 20px; font-size: 0.75rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: var(--color-dark-text-secondary);">
                        ${prediction.samples_used} samples utilises
                      </span>
                    </div>
                  </div>
                </div>

                <!-- Alternatives -->
                ${prediction.alternatives?.length > 0 ? html`
                  <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                    ${prediction.alternatives.map(alt => html`
                      <span style="padding: 0.35rem 0.75rem; background: rgba(139, 92, 246, 0.1); border: 1px solid rgba(139, 92, 246, 0.25); border-radius: 20px; font-size: 0.8rem; color: var(--color-dark-text-secondary);">
                        ${this.getModeIcon(alt.mode)} ${this.getModeName(alt.mode)}: ${Math.round(alt.score * 100)}%
                      </span>
                    `)}
                  </div>
                ` : ''}
              </div>
            </div>

            <!-- Why Chain - Samples Contributing -->
            ${prediction.why?.length > 0 ? html`
              <div style="margin-top: 1.5rem; padding-top: 1.25rem; border-top: 1px solid rgba(139, 92, 246, 0.15);">
                <div style="font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-dark-text-tertiary); margin-bottom: 0.75rem;">Samples Contributifs</div>
                <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                  ${prediction.why.slice(0, 5).map(w => html`
                    <div style="padding: 0.5rem 0.75rem; background: rgba(0,0,0,0.2); border-radius: 8px; font-size: 0.75rem;">
                      <span style="color: #a78bfa;">${this.getModeIcon(w.mode)}</span>
                      <span style="color: var(--color-dark-text-secondary);">${w.mode}</span>
                      <span style="color: ${w.similarity >= 0.8 ? '#22c55e' : w.similarity >= 0.5 ? '#fb923c' : '#9ca3af'}; margin-left: 0.5rem;">
                        ${Math.round(w.similarity * 100)}% sim
                      </span>
                    </div>
                  `)}
                </div>
              </div>
            ` : ''}
          ` : html`
            <div style="text-align: center; padding: 3rem; color: var(--color-dark-text-tertiary);">
              <div style="font-size: 3rem; margin-bottom: 1rem; opacity: 0.4; animation: float-icon 3s ease-in-out infinite;">🧠</div>
              <div style="font-size: 1rem;">Pas de prediction v2 disponible</div>
              <div style="font-size: 0.8rem; margin-top: 0.5rem; opacity: 0.7;">Le systeme collecte des donnees...</div>
            </div>
          `}
        </div>

        <!-- Context Vector Section -->
        ${vector ? html`
          <div class="section-card" style="background: rgba(30, 35, 45, 0.7); border: 1px solid rgba(255,255,255,0.1); border-radius: 16px; padding: 1.25rem; margin-bottom: 1.25rem;">
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem;">
              <span style="font-size: 1.25rem; animation: float-icon 3s ease-in-out infinite;">📊</span>
              <h3 style="margin: 0; font-size: 1rem; font-weight: 600; color: var(--color-dark-text-primary);">Context Vector</h3>
              <span style="margin-left: auto; font-size: 0.7rem; color: var(--color-dark-text-tertiary);">
                ${vector.feature_count || 0} features
              </span>
            </div>

            <!-- Dimensions as bars -->
            <div style="display: flex; flex-direction: column; gap: 0.75rem;">
              ${Object.entries(vector.dimensions || {}).map(([dim, value]) => this.renderDimensionBar(dim, value, vector.why?.[dim]))}
            </div>
          </div>
        ` : ''}

        <!-- Features Section -->
        ${features.length > 0 ? html`
          <div class="section-card" style="background: rgba(30, 35, 45, 0.7); border: 1px solid rgba(255,255,255,0.1); border-radius: 16px; padding: 1.25rem; margin-bottom: 1.25rem;">
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem;">
              <span style="font-size: 1.25rem; animation: float-icon 3s ease-in-out infinite;">📡</span>
              <h3 style="margin: 0; font-size: 1rem; font-weight: 600; color: var(--color-dark-text-primary);">Features Registry</h3>
              <span style="margin-left: auto; padding: 0.25rem 0.75rem; background: rgba(34, 197, 94, 0.15); border: 1px solid rgba(34, 197, 94, 0.25); border-radius: 20px; font-size: 0.75rem; color: #22c55e; font-weight: 600;">
                ${summary?.active_count || features.length} actives
              </span>
            </div>

            ${Object.entries(featuresBySource).map(([source, sourceFeatures]) => html`
              <div style="margin-bottom: 1rem;">
                <div style="font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-dark-text-tertiary); margin-bottom: 0.5rem; padding-bottom: 0.25rem; border-bottom: 1px solid rgba(255,255,255,0.05);">
                  ${source === 'agent' ? '🖥️ Agent' : source === 'classifier' ? '🏷️ Classifier' : source === 'sensor' ? '🌡️ Sensors' : source}
                </div>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.5rem;">
                  ${sourceFeatures.map(f => this.renderFeatureCard(f))}
                </div>
              </div>
            `)}
          </div>
        ` : ''}

        <!-- Stats Section -->
        ${stats ? html`
          <div class="section-card" style="background: rgba(30, 35, 45, 0.6); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 1rem;">
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem;">
              <span style="font-size: 1rem;">📈</span>
              <h3 style="margin: 0; font-size: 0.9rem; font-weight: 600; color: var(--color-dark-text-primary);">Statistiques Apprentissage</h3>
            </div>
            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 0.75rem;">
              <div style="padding: 0.75rem; background: rgba(0,0,0,0.2); border-radius: 8px; text-align: center;">
                <div style="font-size: 1.5rem; font-weight: 700; color: #a78bfa;">${stats.total_samples}</div>
                <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">Total Samples</div>
              </div>
              <div style="padding: 0.75rem; background: rgba(0,0,0,0.2); border-radius: 8px; text-align: center;">
                <div style="font-size: 1.5rem; font-weight: 700; color: #22c55e;">${stats.by_source?.UserCorrection || 0}</div>
                <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">Corrections</div>
              </div>
              <div style="padding: 0.75rem; background: rgba(0,0,0,0.2); border-radius: 8px; text-align: center;">
                <div style="font-size: 1.5rem; font-weight: 700; color: #fb923c;">${stats.by_source?.Bootstrap || 0}</div>
                <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">Bootstrap</div>
              </div>
              <div style="padding: 0.75rem; background: rgba(0,0,0,0.2); border-radius: 8px; text-align: center;">
                <div style="font-size: 1.5rem; font-weight: 700; color: var(--color-dark-text-primary);">${(stats.average_weight || 0).toFixed(2)}</div>
                <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary);">Poids Moyen</div>
              </div>
            </div>

            <!-- Samples by mode -->
            ${stats.by_mode ? html`
              <div style="margin-top: 1rem; padding-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.05);">
                <div style="font-size: 0.7rem; color: var(--color-dark-text-tertiary); margin-bottom: 0.5rem;">Samples par mode</div>
                <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                  ${Object.entries(stats.by_mode).map(([mode, count]) => html`
                    <span style="padding: 0.25rem 0.5rem; background: rgba(139, 92, 246, 0.1); border-radius: 10px; font-size: 0.7rem; color: var(--color-dark-text-secondary);">
                      ${this.getModeIcon(mode)} ${mode}: ${count}
                    </span>
                  `)}
                </div>
              </div>
            ` : ''}
          </div>
        ` : ''}
      </div>
    `
  }

  // Render a dimension bar with why chain
  renderDimensionBar(dimension, value, why) {
    const colors = {
      'home_prob': { start: '#22c55e', end: '#4ade80', icon: '🏠' },
      'work_prob': { start: '#3b82f6', end: '#60a5fa', icon: '💼' },
      'focus_prob': { start: '#8b5cf6', end: '#a78bfa', icon: '🎯' },
      'sleep_prob': { start: '#6366f1', end: '#818cf8', icon: '😴' },
      'pc_active': { start: '#f59e0b', end: '#fbbf24', icon: '🖥️' },
      'away_prob': { start: '#ec4899', end: '#f472b6', icon: '🚶' }
    }
    const c = colors[dimension] || { start: '#6b7280', end: '#9ca3af', icon: '📊' }
    const percentage = Math.round(value * 100)
    const label = dimension.replace('_prob', '').replace('_', ' ')

    return html`
      <div style="display: flex; align-items: center; gap: 0.75rem;">
        <span style="font-size: 1rem; width: 1.5rem; text-align: center;">${c.icon}</span>
        <div style="flex: 1;">
          <div style="display: flex; justify-content: space-between; margin-bottom: 0.25rem;">
            <span style="font-size: 0.75rem; color: var(--color-dark-text-secondary); text-transform: capitalize;">${label}</span>
            <span style="font-size: 0.75rem; font-weight: 600; color: ${c.start};">${percentage}%</span>
          </div>
          <div style="height: 6px; background: rgba(255,255,255,0.1); border-radius: 3px; overflow: hidden;">
            <div style="height: 100%; width: ${percentage}%; background: linear-gradient(90deg, ${c.start}, ${c.end}); border-radius: 3px; transition: width 0.5s ease-out;"></div>
          </div>
          ${why?.length > 0 ? html`
            <div style="margin-top: 0.25rem; display: flex; flex-wrap: wrap; gap: 0.25rem;">
              ${why.slice(0, 3).map(w => html`
                <span style="font-size: 0.65rem; padding: 0.1rem 0.4rem; background: rgba(255,255,255,0.05); border-radius: 4px; color: var(--color-dark-text-tertiary);">
                  ${w.feature_id}: ${w.contribution > 0 ? '+' : ''}${Math.round(w.contribution * 100)}%
                </span>
              `)}
            </div>
          ` : ''}
        </div>
      </div>
    `
  }

  // Render a feature card
  renderFeatureCard(feature) {
    const value = feature.value?.value
    const type = feature.value?.type
    let displayValue = ''
    let status = 'neutral'

    if (type === 'Bool') {
      displayValue = value ? '✓ Oui' : '✗ Non'
      status = value ? 'good' : 'neutral'
    } else if (type === 'Float') {
      displayValue = typeof value === 'number' ? value.toFixed(1) : value
      // Add unit based on feature_id
      if (feature.feature_id.includes('temperature')) displayValue += '°C'
      else if (feature.feature_id.includes('humidity')) displayValue += '%'
      else if (feature.feature_id.includes('cpu') || feature.feature_id.includes('memory')) displayValue += '%'
    } else if (type === 'StringList') {
      displayValue = `${value?.length || 0} items`
    } else {
      displayValue = String(value)
    }

    const statusColors = {
      good: { bg: 'rgba(34, 197, 94, 0.1)', border: 'rgba(34, 197, 94, 0.3)', text: '#22c55e' },
      warning: { bg: 'rgba(251, 146, 60, 0.1)', border: 'rgba(251, 146, 60, 0.3)', text: '#fb923c' },
      neutral: { bg: 'rgba(255,255,255,0.03)', border: 'rgba(255,255,255,0.1)', text: 'var(--color-dark-text-primary)' }
    }
    const s = statusColors[status]

    return html`
      <div style="padding: 0.5rem 0.75rem; background: ${s.bg}; border: 1px solid ${s.border}; border-radius: 8px;">
        <div style="font-size: 0.65rem; color: var(--color-dark-text-tertiary); margin-bottom: 0.15rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
          ${feature.feature_id}
        </div>
        <div style="font-size: 0.9rem; font-weight: 600; color: ${s.text};">${displayValue}</div>
        <div style="font-size: 0.6rem; color: var(--color-dark-text-tertiary); opacity: 0.7;">
          conf: ${Math.round(feature.confidence * 100)}%
        </div>
      </div>
    `
  }

  // ============ Intelligence UI Helpers ============

  renderConfidenceGauge(value, size = 120) {
    const percentage = Math.round(value * 100)
    const radius = 45
    const circumference = 2 * Math.PI * radius // ~283
    const offset = circumference - (value * circumference)
    const color = value >= 0.7 ? '#22c55e' : value >= 0.4 ? '#fb923c' : '#ef4444'
    const glowColor = value >= 0.7 ? 'rgba(34, 197, 94, 0.6)' : value >= 0.4 ? 'rgba(251, 146, 60, 0.6)' : 'rgba(239, 68, 68, 0.6)'
    const label = value >= 0.7 ? 'Haute' : value >= 0.4 ? 'Moyenne' : 'Faible'

    return html`
      <div style="position: relative; width: ${size}px; height: ${size}px;">
        <svg width="${size}" height="${size}" viewBox="0 0 100 100" style="transform: rotate(-90deg); filter: drop-shadow(0 0 12px ${glowColor});">
          <!-- Background circle -->
          <circle cx="50" cy="50" r="${radius}" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="8"/>
          <!-- Progress circle -->
          <circle cx="50" cy="50" r="${radius}" fill="none" stroke="url(#gauge-gradient-${percentage})" stroke-width="8"
            stroke-linecap="round"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            style="transition: stroke-dashoffset 1s ease-out;"/>
          <defs>
            <linearGradient id="gauge-gradient-${percentage}" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="${color}"/>
              <stop offset="100%" stop-color="${value >= 0.7 ? '#4ade80' : value >= 0.4 ? '#fdba74' : '#f87171'}"/>
            </linearGradient>
          </defs>
        </svg>
        <div style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center;">
          <span style="font-size: ${size / 4}px; font-weight: 700; color: ${color}; animation: count-up 0.5s ease-out;">${percentage}%</span>
          <span style="font-size: ${size / 10}px; color: var(--color-dark-text-tertiary); text-transform: uppercase; letter-spacing: 0.05em;">${label}</span>
        </div>
      </div>
    `
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
      <div class="config-section" style="background: rgba(99, 102, 241, 0.1); border: 1px solid rgba(99, 102, 241, 0.3);">
        <div class="config-title" style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer; user-select: none;"
          @click="${() => this.showConfigHelp = !this.showConfigHelp}">
          <span style="background: rgba(99, 102, 241, 0.3); width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.85rem;">?</span>
          Comment ça marche ?
          <span style="margin-left: auto; font-size: 0.75rem; opacity: 0.6;">${this.showConfigHelp ? '▼' : '▶'}</span>
        </div>
        ${this.showConfigHelp ? html`
        <div style="font-size: 0.8rem; color: var(--color-dark-text-secondary); line-height: 1.6; margin-top: 0.75rem;">

          <p style="margin: 0.5rem 0 0.75rem;"><strong>1. Calcul du Trust Score</strong> (0.0 à 1.0) :</p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;">Le système évalue 5 critères et fait la moyenne pondérée :</p>
            <div style="display: grid; grid-template-columns: 1fr auto auto; gap: 0.25rem 0.5rem;">
              <span>• Mode & SSID correspondent ?</span><span style="color: #818cf8;">25%</span><span style="color: #6b7280;">→ 0 ou 1</span>
              <span>• Agent en ligne, CPU/RAM ok ?</span><span style="color: #818cf8;">25%</span><span style="color: #6b7280;">→ 0 ou 1</span>
              <span>• Action pas expirée ?</span><span style="color: #818cf8;">20%</span><span style="color: #6b7280;">→ 0 à 1</span>
              <span>• Historique de succès</span><span style="color: #818cf8;">15%</span><span style="color: #6b7280;">→ 0 à 1</span>
              <span>• Tes approbations passées</span><span style="color: #818cf8;">15%</span><span style="color: #6b7280;">→ 0 à 1</span>
            </div>
            <p style="margin: 0.5rem 0 0; font-style: italic;">Score max = 1.0 si tout est parfait.</p>
          </div>

          <p style="margin: 0.5rem 0 0.5rem;"><strong>2. Comment choisir les seuils ?</strong></p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;">Chaque type d'action utilise un seuil selon son niveau d'impact :</p>
            <div style="display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.5rem;">
              <span style="color: #10b981;">Low</span><span>→ Notifications (ex: "Tu as reçu un email")</span>
              <span style="color: #3b82f6;">Medium</span><span>→ Changements de mode, ajustements légers</span>
              <span style="color: #f59e0b;">High</span><span>→ Contrôle d'appareils (allumer/éteindre PC)</span>
              <span style="color: #ef4444;">Very High</span><span>→ Actions critiques ou irréversibles</span>
            </div>
          </div>

          <p style="margin: 0.5rem 0;"><strong>3. Règle simple pour configurer :</strong></p>
          <div style="background: rgba(0,0,0,0.2); padding: 0.5rem; border-radius: 6px; margin: 0 0 0.75rem; font-size: 0.75rem;">
            <p style="margin: 0 0 0.5rem;"><strong>Seuil bas (0.3-0.4)</strong> = Peu exigeant, s'exécute souvent seul</p>
            <p style="margin: 0 0 0.5rem;"><strong>Seuil moyen (0.5-0.6)</strong> = Équilibré, vérifie le contexte</p>
            <p style="margin: 0 0 0.5rem;"><strong>Seuil haut (0.7-0.8)</strong> = Strict, demande validation si doute</p>
            <p style="margin: 0;"><strong>Seuil très haut (0.9+)</strong> = Quasi toujours validation manuelle</p>
          </div>

          <div style="background: rgba(16, 185, 129, 0.15); padding: 0.5rem; border-radius: 6px; border: 1px solid rgba(16, 185, 129, 0.3);">
            <p style="margin: 0; font-size: 0.75rem;">
              <strong>💡 Exemple concret :</strong><br>
              Tu as High = 0.7. Une automation "Allumer PC" calcule un score de 0.66<br>
              → <span style="color: #f59e0b;">0.66 < 0.7</span> = demande ta validation<br>
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
        <div style="font-size: 0.75rem; color: var(--color-dark-text-secondary); margin-top: 0.5rem;">
          Score attribué aux nouvelles automations sans historique
        </div>
      </div>

      <button class="btn btn-primary" @click="${this.saveConfig}" style="width: 100%; margin-top: 1rem;">
        💾 Sauvegarder
      </button>
    `
  }

  // ============ MODES TAB (Unified: Current Mode + Mode Management) ============

  renderModesTab() {
    const state = this.contextState
    // Prefer mode_slug (dynamic) over mode (legacy enum)
    const mode = state?.mode_slug || state?.mode?.toLowerCase() || 'veille'
    const hasOverride = !!state?.manual_override

    return html`
      <div class="modes-container">
        <!-- Section 1: Mode Actuel -->
        <div class="current-mode-section">
          <div class="section-header">
            <h3>Mode Actuel</h3>
          </div>

          ${!state ? html`
            <div class="loading-state">⏳ Chargement...</div>
          ` : html`
            <div class="current-mode-display">
              <div class="mode-status">
                <span class="current-mode-icon">${this.getModeIcon(mode)}</span>
                <div class="mode-info">
                  <span class="current-mode-name">${this.getModeName(mode)}</span>
                  <span class="mode-reason">${state.reason || 'Détection automatique'}</span>
                </div>
                <!-- Confidence indicator removed - now displayed in Intelligence Widget -->
              </div>

              ${hasOverride ? html`
                <div class="override-banner">
                  ⚠️ Override manuel jusqu'à ${new Date(state.manual_override.until).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}
                  <button class="btn btn-sm" @click="${this.clearOverride}">Annuler</button>
                </div>
              ` : ''}

              <div class="quick-mode-controls">
                <div class="mode-buttons-row">
                  ${this.modes.map(m => html`
                    <button
                      class="mode-quick-btn ${mode === m.slug ? 'active' : ''}"
                      style="--btn-color: ${m.theme?.primary || '#6b7280'}"
                      @click="${() => this.setModeOverride(m.slug)}"
                      title="${m.name}"
                    >
                      ${m.icon} ${m.name}
                    </button>
                  `)}
                </div>
                <div class="duration-selector">
                  ${[60, 120, 240, 480].map(d => html`
                    <button
                      class="duration-btn ${this.selectedDuration === d ? 'active' : ''}"
                      @click="${() => this.selectedDuration = d}"
                    >
                      ${this.formatDuration(d)}
                    </button>
                  `)}
                </div>
              </div>
            </div>
          `}
        </div>

        <!-- Section 2: Gestion des Modes -->
        <div class="modes-management-section">
          <div class="section-header">
            <h3>Gestion des Modes</h3>
            <button class="btn btn-primary btn-sm" @click="${() => this.openModeForm()}">
              + Nouveau Mode
            </button>
          </div>

          <div class="modes-grid">
            ${this.modes.map(mode => this.renderModeCard(mode))}
          </div>
        </div>

        ${this.showModeForm ? this.renderModeForm() : ''}
      </div>
    `
  }

  renderModeCard(mode) {
    return html`
      <div class="mode-card" style="--mode-primary: ${mode.theme?.primary || '#6b7280'}; --mode-bg: ${mode.theme?.background || '#f9fafb'}; --mode-accent: ${mode.theme?.accent || '#4b5563'}">
        <div class="mode-card-header">
          <span class="mode-card-icon">${mode.icon}</span>
          <span class="mode-card-name">${mode.name}</span>
          ${mode.is_system ? html`<span class="system-badge">Système</span>` : ''}
        </div>
        <div class="mode-card-preview">
          <div class="color-preview" style="background: ${mode.theme?.primary}"></div>
          <div class="color-preview" style="background: ${mode.theme?.background}; border: 1px solid rgba(0,0,0,0.1);"></div>
          <div class="color-preview" style="background: ${mode.theme?.accent}"></div>
        </div>
        <div class="mode-card-slug">/${mode.slug}</div>
        <div class="mode-card-actions">
          <button class="btn btn-sm" @click="${() => this.openModeForm(mode)}" title="Modifier">
            ✏️
          </button>
          ${!mode.is_system ? html`
            <button class="btn btn-sm btn-danger" @click="${() => this.deleteMode(mode.id)}" title="Supprimer">
              🗑️
            </button>
          ` : ''}
        </div>
      </div>
    `
  }

  renderModeForm() {
    const isEditing = !!this.editingMode
    return html`
      <div class="modal-overlay" @click="${() => this.closeModeForm()}">
        <div class="mode-form" @click="${e => e.stopPropagation()}">
          <div class="form-header">
            <h3 style="margin: 0;">${isEditing ? 'Modifier le Mode' : 'Nouveau Mode'}</h3>
            <button class="close-btn" @click="${() => this.closeModeForm()}">✕</button>
          </div>

          <div class="form-body">
            <div class="form-group">
              <label>Nom du mode</label>
              <input type="text" class="form-input"
                .value="${this.modeFormData.name}"
                @input="${e => this.modeFormData = {...this.modeFormData, name: e.target.value}}"
                placeholder="Ex: Travail, Sport, Lecture..."
              >
            </div>

            <div class="form-group">
              <label>Icône (emoji)</label>
              <div class="emoji-picker">
                ${['🎯', '👔', '🏡', '🌱', '📚', '💪', '🎮', '🎵', '☕', '🌙', '🔥', '💼'].map(emoji => html`
                  <button
                    class="emoji-btn ${this.modeFormData.icon === emoji ? 'selected' : ''}"
                    @click="${() => this.modeFormData = {...this.modeFormData, icon: emoji}}"
                  >${emoji}</button>
                `)}
              </div>
            </div>

            <div class="form-group">
              <label>Couleurs du thème</label>
              <div class="color-pickers">
                <div class="color-picker-group">
                  <label>Principale</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.primary}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, primary: e.target.value}}}"
                  >
                </div>
                <div class="color-picker-group">
                  <label>Fond</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.background}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, background: e.target.value}}}"
                  >
                </div>
                <div class="color-picker-group">
                  <label>Accent</label>
                  <input type="color"
                    .value="${this.modeFormData.theme.accent}"
                    @input="${e => this.modeFormData = {...this.modeFormData, theme: {...this.modeFormData.theme, accent: e.target.value}}}"
                  >
                </div>
              </div>
            </div>

            <div class="form-group">
              <label>Aperçu</label>
              <div class="mode-preview" style="--preview-primary: ${this.modeFormData.theme.primary}; --preview-bg: ${this.modeFormData.theme.background}; --preview-accent: ${this.modeFormData.theme.accent}">
                <span class="preview-icon">${this.modeFormData.icon}</span>
                <span class="preview-name">${this.modeFormData.name || 'Nom du mode'}</span>
              </div>
            </div>
          </div>

          <div class="form-actions">
            <button class="btn" @click="${() => this.closeModeForm()}">Annuler</button>
            <button class="btn btn-primary" @click="${() => this.saveMode()}">
              ${isEditing ? 'Mettre à jour' : 'Créer'}
            </button>
          </div>
        </div>
      </div>
    `
  }

  openModeForm(mode = null) {
    if (mode) {
      this.editingMode = mode
      this.modeFormData = {
        name: mode.name,
        icon: mode.icon,
        theme: { ...mode.theme }
      }
    } else {
      this.editingMode = null
      this.modeFormData = {
        name: '',
        icon: '🎯',
        theme: { primary: '#2563eb', background: '#f8fafc', accent: '#1e40af' }
      }
    }
    this.showModeForm = true
  }

  closeModeForm() {
    this.showModeForm = false
    this.editingMode = null
  }

  async saveMode() {
    try {
      const isEditing = !!this.editingMode
      const url = isEditing ? `/v1/modes/${this.editingMode.id}` : '/v1/modes'
      const method = isEditing ? 'PUT' : 'POST'

      const res = await csrfService.fetchWithCsrf(url, {
        method,
        body: JSON.stringify(this.modeFormData)
      })

      if (res.ok) {
        console.log(`[context-engine] Mode ${isEditing ? 'updated' : 'created'} successfully`)
        this.closeModeForm()
        await this.loadModes()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to save mode:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to save mode:', e)
      alert(`Erreur: ${e.message}`)
    }
  }

  async deleteMode(id) {
    if (!confirm('Supprimer ce mode ?')) return

    try {
      const res = await csrfService.fetchWithCsrf(`/v1/modes/${id}`, {
        method: 'DELETE'
      })

      if (res.ok) {
        console.log('[context-engine] Mode deleted successfully')
        await this.loadModes()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to delete mode:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to delete mode:', e)
      alert(`Erreur: ${e.message}`)
    }
  }

  // ============ PLANNING TAB ============

  renderPlanningTab() {
    const dayNames = getAllDayNamesShort() // ISO order (Mon-Sun)
    const hours = [6, 8, 10, 12, 14, 16, 18, 20, 22]

    return html`
      <div class="planning-container">
        <div class="planning-header">
          <h3 style="margin: 0; color: var(--color-dark-text-primary);">Planning Horaire</h3>
          <button class="btn btn-primary btn-sm" @click="${() => this.openRuleForm()}">
            + Nouvelle Règle
          </button>
        </div>

        <div class="planning-default">
          <span>Mode par défaut :</span>
          <select class="form-input" style="width: auto; margin-left: 0.5rem;"
            .value="${this.schedule?.default_mode_id || 'mode-veille'}"
            @change="${e => this.setDefaultMode(e.target.value)}">
            ${this.modes.map(m => html`
              <option value="${m.id}" ?selected="${m.id === this.schedule?.default_mode_id}">${m.icon} ${m.name}</option>
            `)}
          </select>
        </div>

        <div class="planning-grid">
          <div class="grid-header">
            <div class="grid-hour-label"></div>
            ${dayNames.map(day => html`<div class="grid-day-header">${day}</div>`)}
          </div>
          ${hours.map(hour => html`
            <div class="grid-row">
              <div class="grid-hour-label">${hour}h</div>
              ${[0, 1, 2, 3, 4, 5, 6].map(day => {
                const rule = this.findRuleAt(hour, day)
                const mode = rule ? this.modes.find(m => m.id === rule.mode_id) : null
                return html`
                  <div class="grid-cell ${rule ? 'has-rule' : ''}"
                    style="${mode ? `background: ${mode.theme?.primary}20; border-color: ${mode.theme?.primary}` : ''}"
                    @click="${() => rule ? this.openRuleForm(rule) : this.openRuleFormForSlot(hour, day)}">
                    ${mode ? html`<span class="cell-icon">${mode.icon}</span>` : ''}
                  </div>
                `
              })}
            </div>
          `)}
        </div>

        <div class="planning-rules-list">
          <h4 style="margin: 1rem 0 0.5rem; color: var(--color-dark-text-secondary);">Règles actives (${this.schedule?.rules?.length || 0})</h4>
          ${this.schedule?.rules?.length === 0 ? html`
            <div class="empty-state" style="padding: 1rem;">
              <div class="empty-text">Aucune règle configurée</div>
            </div>
          ` : ''}
          ${this.schedule?.rules?.map(rule => this.renderRuleCard(rule))}
        </div>

        ${this.showRuleForm ? this.renderRuleForm() : ''}
      </div>
    `
  }

  findRuleAt(hour, day) {
    if (!this.schedule?.rules) return null
    return this.schedule.rules.find(rule => {
      if (!rule.enabled || !rule.days.includes(day)) return false
      const [startH] = rule.start_time.split(':').map(Number)
      const [endH] = rule.end_time.split(':').map(Number)
      return hour >= startH && hour < endH
    })
  }

  renderRuleCard(rule) {
    const mode = this.modes.find(m => m.id === rule.mode_id)
    const dayNames = getAllDayNamesShort() // ISO order (Mon-Sun)

    return html`
      <div class="rule-card ${!rule.enabled ? 'disabled' : ''}"
        style="${mode ? `border-left: 3px solid ${mode.theme?.primary}` : ''}">
        <div class="rule-card-header">
          <span class="rule-icon">${mode?.icon || '?'}</span>
          <span class="rule-name">${rule.name || mode?.name || 'Sans nom'}</span>
          <div class="rule-actions">
            <button class="btn btn-sm" @click="${() => this.toggleRule(rule)}" title="${rule.enabled ? 'Désactiver' : 'Activer'}">
              ${rule.enabled ? '✓' : '○'}
            </button>
            <button class="btn btn-sm" @click="${() => this.openRuleForm(rule)}" title="Modifier">
              ✏️
            </button>
            <button class="btn btn-sm btn-danger" @click="${() => this.deleteRule(rule.id)}" title="Supprimer">
              🗑️
            </button>
          </div>
        </div>
        <div class="rule-details">
          <span class="rule-time">${rule.start_time} - ${rule.end_time}</span>
          <span class="rule-days">${rule.days.map(d => dayNames[parseInt(d)] || d).join(', ')}</span>
        </div>
      </div>
    `
  }

  renderRuleForm() {
    const isEditing = !!this.editingRule
    const dayNames = getAllDayNamesShort() // ISO order (Mon-Sun)

    return html`
      <div class="modal-overlay" @click="${() => this.closeRuleForm()}">
        <div class="rule-form" @click="${e => e.stopPropagation()}">
          <div class="form-header">
            <h3 style="margin: 0;">${isEditing ? 'Modifier la Règle' : 'Nouvelle Règle'}</h3>
            <button class="close-btn" @click="${() => this.closeRuleForm()}">✕</button>
          </div>

          <div class="form-body">
            <div class="form-group">
              <label>Nom (optionnel)</label>
              <input type="text" class="form-input"
                .value="${this.ruleFormData.name || ''}"
                @input="${e => this.ruleFormData = {...this.ruleFormData, name: e.target.value}}"
                placeholder="Ex: Travail matin, Weekend détente...">
            </div>

            <div class="form-group">
              <label>Mode à activer</label>
              <select class="form-input"
                .value="${this.ruleFormData.mode_id}"
                @change="${e => this.ruleFormData = {...this.ruleFormData, mode_id: e.target.value}}">
                ${this.modes.map(m => html`
                  <option value="${m.id}" ?selected="${m.id === this.ruleFormData.mode_id}">${m.icon} ${m.name}</option>
                `)}
              </select>
            </div>

            <div class="form-group">
              <label>Jours</label>
              <div class="days-picker">
                ${dayNames.map((day, i) => html`
                  <button
                    class="day-btn ${this.ruleFormData.days.includes(i) ? 'selected' : ''}"
                    @click="${() => this.toggleDay(i)}">
                    ${day}
                  </button>
                `)}
              </div>
            </div>

            <div class="form-row">
              <div class="form-group" style="flex: 1;">
                <label>Début</label>
                <input type="time" class="form-input"
                  .value="${this.ruleFormData.start_time}"
                  @input="${e => this.ruleFormData = {...this.ruleFormData, start_time: e.target.value}}">
              </div>
              <div class="form-group" style="flex: 1;">
                <label>Fin</label>
                <input type="time" class="form-input"
                  .value="${this.ruleFormData.end_time}"
                  @input="${e => this.ruleFormData = {...this.ruleFormData, end_time: e.target.value}}">
              </div>
            </div>

            <div class="form-group">
              <label>Priorité (0 = basse, 10 = haute)</label>
              <input type="number" class="form-input" min="0" max="10"
                .value="${this.ruleFormData.priority}"
                @input="${e => this.ruleFormData = {...this.ruleFormData, priority: parseInt(e.target.value) || 0}}">
            </div>
          </div>

          <div class="form-actions">
            <button class="btn" @click="${() => this.closeRuleForm()}">Annuler</button>
            <button class="btn btn-primary" @click="${() => this.saveRule()}">
              ${isEditing ? 'Mettre à jour' : 'Créer'}
            </button>
          </div>
        </div>
      </div>
    `
  }

  toggleDay(day) {
    const days = [...this.ruleFormData.days]
    const idx = days.indexOf(day)
    if (idx >= 0) {
      days.splice(idx, 1)
    } else {
      days.push(day)
      days.sort((a, b) => a - b)
    }
    this.ruleFormData = {...this.ruleFormData, days}
  }

  openRuleForm(rule = null) {
    if (rule) {
      this.editingRule = rule
      this.ruleFormData = {
        mode_id: rule.mode_id,
        days: [...rule.days],
        start_time: rule.start_time,
        end_time: rule.end_time,
        priority: rule.priority,
        name: rule.name || ''
      }
    } else {
      this.editingRule = null
      this.ruleFormData = {
        mode_id: 'mode-pro',
        days: [0, 1, 2, 3, 4],
        start_time: '09:00',
        end_time: '18:00',
        priority: 0,
        name: ''
      }
    }
    this.showRuleForm = true
  }

  openRuleFormForSlot(hour, day) {
    this.editingRule = null
    this.ruleFormData = {
      mode_id: 'mode-pro',
      days: [day],
      start_time: `${hour.toString().padStart(2, '0')}:00`,
      end_time: `${(hour + 2).toString().padStart(2, '0')}:00`,
      priority: 0,
      name: ''
    }
    this.showRuleForm = true
  }

  closeRuleForm() {
    this.showRuleForm = false
    this.editingRule = null
  }

  async saveRule() {
    try {
      const isEditing = !!this.editingRule
      const url = isEditing ? `/v1/schedule/rules/${this.editingRule.id}` : '/v1/schedule/rules'
      const method = isEditing ? 'PUT' : 'POST'

      const res = await csrfService.fetchWithCsrf(url, {
        method,
        body: JSON.stringify(this.ruleFormData)
      })

      if (res.ok) {
        console.log(`[context-engine] Rule ${isEditing ? 'updated' : 'created'} successfully`)
        this.closeRuleForm()
        await this.loadSchedule()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to save rule:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to save rule:', e)
      alert(`Erreur: ${e.message}`)
    }
  }

  async deleteRule(id) {
    if (!confirm('Supprimer cette règle ?')) return

    try {
      const res = await csrfService.fetchWithCsrf(`/v1/schedule/rules/${id}`, {
        method: 'DELETE'
      })

      if (res.ok) {
        console.log('[context-engine] Rule deleted successfully')
        await this.loadSchedule()
      } else {
        const error = await res.text()
        console.error('[context-engine] Failed to delete rule:', error)
        alert(`Erreur: ${error}`)
      }
    } catch (e) {
      console.error('[context-engine] Failed to delete rule:', e)
      alert(`Erreur: ${e.message}`)
    }
  }

  async toggleRule(rule) {
    try {
      const res = await csrfService.fetchWithCsrf(`/v1/schedule/rules/${rule.id}`, {
        method: 'PUT',
        body: JSON.stringify({ enabled: !rule.enabled })
      })

      if (res.ok) {
        await this.loadSchedule()
      }
    } catch (e) {
      console.error('[context-engine] Failed to toggle rule:', e)
    }
  }

  async setDefaultMode(modeId) {
    try {
      const res = await csrfService.fetchWithCsrf('/v1/schedule/default', {
        method: 'PUT',
        body: JSON.stringify({ default_mode_id: modeId })
      })

      if (res.ok) {
        await this.loadSchedule()
      }
    } catch (e) {
      console.error('[context-engine] Failed to set default mode:', e)
    }
  }

}

customElements.define('context-engine-page', ContextEnginePage)

export { ContextEnginePage }
