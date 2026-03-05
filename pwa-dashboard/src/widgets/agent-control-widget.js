/**
 * Widget Agent Control - Contrôle détaillé d'un agent système
 *
 * Modal avec 10 tabs pour contrôle complet:
 * - System: Power management, infos système, health score
 * - Processes: Liste processus + kill
 * - Metrics: CPU, RAM, disque temps réel
 * - Commands: Exécution commandes shell + historique
 * - Services: Gestion services système
 * - Logs: Streaming logs agent (WARN/ERROR)
 * - Watchdog: État watchdog agent (v2.5+)
 * - Scheduler: Tâches planifiées (create/delete/list)
 * - Plugins: Données plugins + notifications agent
 * - Screenshot: Capture d'écran distante
 * - Files: Gestion fichiers agent (upload drag & drop, download, delete)
 */

import { LitElement, html, css } from 'lit'
import { sharedAnimations } from '../styles/shared-animations.js'
import { overlayStyles, statusBadgeStyles } from '../styles/shared-patterns.js'
import { widgetSectionStyles } from '../styles/shared-widget.js'
import '../services/agents-service.js'
import pollingScheduler from '../services/polling-scheduler.js'

class AgentControlWidget extends LitElement {
  static properties = {
    agentId: { type: String },
    agent: { type: Object },
    isOpen: { type: Boolean },
    currentTab: { type: String },
    loading: { type: Boolean },
    refreshing: { type: Boolean },
    processes: { type: Array },
    metrics: { type: Object },
    commandOutput: { type: String },
    commandInput: { type: String },
    currentCommandId: { type: String },
    latestVersion: { type: String },
    services: { type: Array },
    commandHistory: { type: Array },
    agentLogs: { type: Array },
    logLevelFilter: { type: String },
    watchdogData: { type: Object },
    pluginData: { type: Object },
    scheduledTasks: { type: Object },
    screenshotStatus: { type: String },
    screenshotImage: { type: String },
    expandedCommandId: { type: String },
    agentFiles: { type: Array },
    fileTransfers: { type: Array },
    fileDragOver: { type: Boolean }
  }

  static styles = [sharedAnimations, overlayStyles, statusBadgeStyles, widgetSectionStyles, css`
    :host {
      background: var(--app-overlay-dim, rgba(0, 0, 0, 0.88));
      display: flex;
      align-items: center;
      justify-content: center;
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    }

    :host(:not([is-open])) {
      display: none;
    }

    .modal {
      background: linear-gradient(135deg, var(--app-page-bg-a, rgba(30, 30, 30, 0.98)) 0%, var(--app-page-bg-b, rgba(20, 20, 20, 0.98)) 100%);
      border: 1px solid var(--ctx-border);
      border-radius: var(--radius-xl);
      width: 90%;
      max-width: 900px;
      height: 80%;
      max-height: 700px;
      display: flex;
      flex-direction: column;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6),
                  0 0 40px var(--ctx-bg);
      color: var(--color-dark-text-primary, #e5e5e5);
      overflow: hidden;
      animation: modalSlideIn 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .modal-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: var(--space-6, 1.5rem) var(--space-7, 1.75rem);
      border-bottom: 1px solid var(--ctx-border);
      background: linear-gradient(135deg, var(--surface-glass) 0%, var(--surface-glass-faint) 100%);
      position: relative;
      animation: modalHeaderSlideIn 0.5s ease-out 0.1s backwards;
    }

    /* modalHeaderSlideIn — see shared-animations.js */

    .modal-header::after {
      content: '';
      position: absolute;
      bottom: -1px;
      left: 0;
      width: 30%;
      height: 2px;
      background: linear-gradient(90deg,
        var(--context-primary, #00d4aa) 0%,
        transparent 100%);
      opacity: 0.8;
      box-shadow: 0 0 10px var(--context-primary, #00d4aa);
    }

    .modal-title {
      display: flex;
      align-items: center;
      gap: 12px;
      font-size: var(--text-xl, 1.25rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .os-icon {
      font-size: var(--text-3xl, 2rem);
    }

    .agent-info {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .agent-hostname {
      font-size: var(--text-xl, 1.25rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .agent-meta {
      font-size: var(--text-sm, 0.875rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    /* Local override — animation only (base from statusBadgeStyles) */
    .status-badge.online {
      animation: pulse-online 3s ease-in-out infinite;
    }

    @keyframes pulse-online {
      0%, 100% {
        box-shadow: 0 2px 12px var(--ctx-border-strong);
      }
      50% {
        box-shadow: 0 2px 16px var(--ctx-border-intense);
      }
    }

    .close-btn {
      background: linear-gradient(135deg,
        rgba(255, 107, 107, 0.15) 0%,
        rgba(255, 107, 107, 0.08) 100%);
      border: 1px solid rgba(255, 107, 107, 0.3);
      color: var(--color-danger-text-muted, #ff6b6b);
      font-size: 1.75rem;
      cursor: pointer;
      padding: 8px 12px;
      border-radius: var(--radius-md, 0.75rem);
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      line-height: 1;
    }

    .close-btn:hover {
      background: linear-gradient(135deg,
        rgba(255, 107, 107, 0.25) 0%,
        rgba(255, 107, 107, 0.15) 100%);
      border-color: rgba(255, 107, 107, 0.5);
      color: var(--color-dark-text-primary, #f8f9fa);
      transform: rotate(90deg) translateY(-2px);
      box-shadow: 0 6px 16px rgba(255, 107, 107, 0.3);
    }

    .modal-tabs {
      display: flex;
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      padding: 0 24px;
      overflow-x: auto;
      border-bottom: 1px solid var(--border-default);
    }

    .tab-btn {
      padding: 14px 24px;
      border: none;
      background: transparent;
      color: var(--color-dark-text-tertiary, #94a3b8);
      cursor: pointer;
      border-bottom: 3px solid transparent;
      font-size: var(--text-sm, 0.875rem);
      font-weight: 500;
      white-space: nowrap;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      position: relative;
    }

    .tab-btn::before {
      content: '';
      position: absolute;
      bottom: 0;
      left: 0;
      right: 0;
      height: 3px;
      background: linear-gradient(90deg, #007acc, var(--context-primary, #00d4aa));
      transform: scaleX(0);
      transition: transform 0.3s ease;
    }

    .tab-btn.active {
      color: var(--context-primary, #00d4aa);
      background: linear-gradient(135deg, var(--ctx-bg) 0%, rgba(0, 122, 204, 0.05) 100%);
    }

    .tab-btn.active::before {
      transform: scaleX(1);
    }

    .tab-btn:hover {
      color: var(--color-dark-text-secondary, #cbd5e1);
      background: var(--surface-glass);
    }

    .modal-content {
      flex: 1;
      overflow: auto;
      padding: 24px;
    }

    .tab-panel {
      display: none;
      animation: fadeIn 0.2s ease;
    }

    .tab-panel.active {
      display: block;
    }

    .section {
      margin-bottom: 24px;
    }

    /* section-title provided by widgetSectionStyles */

    .power-controls {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }

    .power-btn {
      padding: 12px 24px;
      border: none;
      border-radius: var(--radius-md, 0.75rem);
      font-size: var(--text-sm, 0.875rem);
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      display: flex;
      align-items: center;
      gap: 10px;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    .power-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
    }

    .power-btn.danger {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(220, 38, 38, 0.2) 100%);
      color: var(--color-error-text-muted, #fca5a5);
      border: 1px solid rgba(239, 68, 68, 0.4);
      box-shadow: 0 2px 8px rgba(239, 68, 68, 0.2);
    }

    .power-btn.danger:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(239, 68, 68, 0.35) 0%, rgba(220, 38, 38, 0.3) 100%);
      border-color: rgba(239, 68, 68, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(239, 68, 68, 0.35);
    }

    .power-btn.warning {
      background: linear-gradient(135deg, rgba(245, 158, 11, 0.25) 0%, rgba(251, 191, 36, 0.2) 100%);
      color: var(--color-warning-text-muted, #fbbf24);
      border: 1px solid rgba(245, 158, 11, 0.4);
      box-shadow: 0 2px 8px rgba(245, 158, 11, 0.2);
    }

    .power-btn.warning:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(245, 158, 11, 0.35) 0%, rgba(251, 191, 36, 0.3) 100%);
      border-color: rgba(245, 158, 11, 0.6);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(245, 158, 11, 0.35);
    }

    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 16px;
    }

    .info-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 18px;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    }

    .info-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px var(--ctx-border);
    }

    .info-label {
      font-size: var(--text-xs, 0.75rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 6px;
    }

    .info-value {
      font-size: var(--text-base, 1rem);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: 'Monaco', 'Consolas', monospace;
    }

    .processes-table {
      background: var(--surface-glass);
      border-radius: var(--radius-base);
      overflow: hidden;
    }

    .process-header {
      display: grid;
      grid-template-columns: 80px 1fr 100px 100px 80px;
      gap: 16px;
      padding: 12px 16px;
      background: var(--surface-glass-strong);
      font-size: var(--text-xs, 0.75rem);
      font-weight: 600;
      color: var(--color-dark-text-tertiary, #94a3b8);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .process-row {
      display: grid;
      grid-template-columns: 80px 1fr 100px 100px 80px;
      gap: 16px;
      padding: 12px 16px;
      border-bottom: 1px solid var(--surface-glass);
      font-size: var(--text-sm, 0.875rem);
      align-items: center;
      transition: background 0.2s ease;
    }

    .process-row:hover {
      background: var(--surface-glass);
    }

    .process-name {
      font-family: 'Monaco', 'Consolas', monospace;
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .kill-btn {
      padding: 4px 8px;
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: var(--radius-sm);
      font-size: var(--text-xs);
      cursor: pointer;
      transition: all 0.2s ease;
    }

    .kill-btn:hover {
      background: rgba(239, 68, 68, 0.3);
    }

    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
      gap: 16px;
    }

    .metric-card {
      background: linear-gradient(135deg, var(--surface-glass-hover) 0%, var(--surface-glass-subtle) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 24px;
      text-align: center;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
      position: relative;
      overflow: hidden;
    }

    .metric-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 3px;
      background: linear-gradient(90deg, #007acc, var(--context-primary, #00d4aa));
      opacity: 0;
      transition: opacity 0.3s ease;
    }

    .metric-card:hover {
      background: linear-gradient(135deg, var(--surface-glass-bright) 0%, var(--surface-glass) 100%);
      border-color: var(--ctx-border-strong);
      transform: translateY(-4px);
      box-shadow: 0 8px 24px var(--ctx-border);
    }

    .metric-card:hover::before {
      opacity: 1;
    }

    .metric-value {
      font-size: 2.25rem;
      font-weight: 700;
      margin: 10px 0;
      background: linear-gradient(135deg, #007acc 0%, var(--context-primary, #00d4aa) 50%, #22c55e 100%);
      background-size: 200% 200%;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
      animation: gradient-shift 3s ease infinite;
      filter: drop-shadow(0 2px 4px var(--ctx-border-strong));
    }

    /* gradient-shift — see shared-animations.js */

    .metric-label {
      font-size: var(--text-sm, 0.875rem);
      color: var(--color-dark-text-tertiary, #94a3b8);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .progress-bar {
      width: 100%;
      height: 8px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-sm);
      margin-top: 14px;
      overflow: hidden;
      box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.2);
    }

    .progress-fill {
      height: 100%;
      border-radius: var(--radius-sm);
      transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 0 12px currentColor;
      position: relative;
    }

    .progress-fill::after {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: linear-gradient(90deg, transparent, var(--surface-glass-bright), transparent);
      animation: shimmer 2s infinite;
    }

    .progress-fill.cpu { background: linear-gradient(90deg, #22c55e, var(--context-primary, #00d4aa), #007acc); }
    .progress-fill.memory { background: linear-gradient(90deg, #3b82f6, var(--context-primary, #00d4aa), #8b5cf6); }
    .progress-fill.disk { background: linear-gradient(90deg, #f59e0b, #fbbf24, #ef4444); }
    .progress-fill.gpu { background: linear-gradient(90deg, #10b981, #06b6d4, #8b5cf6); }
    .progress-fill.gpu-mem { background: linear-gradient(90deg, #6366f1, #a78bfa, #c084fc); }

    .io-stats {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
      margin-top: 8px;
    }

    .io-stat {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px;
      border-radius: var(--radius-sm);
      background: var(--surface-glass);
    }

    .io-stat-label {
      font-size: 10px;
      color: var(--text-secondary);
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .io-stat-value {
      font-size: 14px;
      font-weight: 600;
      color: var(--text-primary);
      font-family: 'JetBrains Mono', monospace;
    }

    .net-stat-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 6px 0;
      border-bottom: 1px solid var(--border-subtle);
    }

    .net-stat-row:last-child {
      border-bottom: none;
    }

    .net-stat-label {
      font-size: 12px;
      color: var(--text-secondary);
    }

    .net-stat-value {
      font-size: 13px;
      font-weight: 600;
      color: var(--text-primary);
      font-family: 'JetBrains Mono', monospace;
    }

    .command-section {
      display: flex;
      flex-direction: column;
      gap: 16px;
    }

    .command-input {
      display: flex;
      gap: 12px;
    }

    .command-field {
      flex: 1;
      padding: 12px 16px;
      background: var(--surface-glass);
      border: 1px solid var(--border-hover);
      border-radius: var(--radius-base);
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: var(--text-sm, 0.875rem);
    }

    .command-field:focus {
      outline: none;
      border-color: #3b82f6;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
    }

    .execute-btn {
      padding: 12px 24px;
      background: linear-gradient(135deg, rgba(59, 130, 246, 0.8) 0%, rgba(37, 99, 235, 0.7) 100%);
      color: white;
      border: 1px solid rgba(59, 130, 246, 0.5);
      border-radius: var(--radius-md, 0.75rem);
      font-size: var(--text-sm, 0.875rem);
      font-weight: 500;
      cursor: pointer;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 0 2px 8px rgba(59, 130, 246, 0.3);
    }

    .execute-btn:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(59, 130, 246, 1) 0%, rgba(37, 99, 235, 0.9) 100%);
      border-color: rgba(59, 130, 246, 0.7);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(59, 130, 246, 0.5);
    }

    .execute-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
      transform: none;
    }

    .command-output {
      background: var(--app-terminal-bg, #0d1117);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-base);
      padding: 16px;
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 0.8125rem;
      color: var(--app-terminal-text, #e6edf3);
      white-space: pre-wrap;
      overflow-y: auto;
      max-height: 300px;
      min-height: 150px;
    }

    .loading-state {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 40px;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: var(--text-sm, 0.875rem);
    }

    .error-state {
      padding: 16px;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.3);
      border-radius: var(--radius-base);
      color: var(--color-error-text-muted, #fca5a5);
      text-align: center;
    }

    /* === Utility classes (ex-inline styles) === */
    .ac-meta-hint { font-size: var(--text-xs); color: var(--color-dark-text-tertiary, #888); margin-top: 8px; }
    .ac-meta-inline { font-size: var(--text-xs); color: var(--color-dark-text-tertiary, #888); font-weight: normal; }
    .ac-refresh-indicator { margin-left: 8px; color: #3b82f6; }
    .ac-text-center-muted { text-align: center; color: var(--color-dark-text-tertiary, #888); }
    .ac-power-btn-blue { background: rgba(59, 130, 246, 0.2); color: #3b82f6; border: 1px solid rgba(59, 130, 246, 0.3); }
    .ac-power-btn-green { background: rgba(34, 197, 94, 0.2); color: var(--color-success-text-muted, #22c55e); border: 1px solid rgba(34, 197, 94, 0.3); }
    .ac-cmd-actions { margin-top: 12px; }
    .ac-cmd-cancel { padding: 8px 16px; font-size: var(--text-xs); }
    .ac-cmd-id { color: var(--color-dark-text-tertiary, #888); font-size: var(--text-xs); margin-left: 12px; }
    .ac-icon-lg { font-size: var(--text-2xl); }
    .ac-title-inline { margin: 0 0 0 0.5rem; }
    .ac-error-body { display: flex; align-items: center; justify-content: center; flex-direction: column; padding: 3rem; }
    .ac-error-icon { font-size: var(--text-4xl); margin-bottom: 1rem; opacity: 0.5; }
    .ac-error-text { font-size: var(--text-lg); opacity: 0.8; text-align: center; }
    .ac-error-hint { opacity: 0.6; }
    .ac-error-close-btn { margin-top: 2rem; padding: 0.8rem 1.5rem; background: var(--surface-glass-strong); border: 1px solid var(--border-hover); border-radius: var(--radius-base); color: var(--color-dark-text-primary, #f8f9fa); cursor: pointer; font-size: 0.95em; }
    .ac-version-outdated { color: var(--warning-color, #f59e0b); }
    .ac-version-icon { cursor: help; margin-left: 4px; }

    /* === P1-9: Migrated inline style classes === */
    .ac-span-2 { grid-column: span 2; }
    .ac-health-score { font-size: var(--text-2xl, 1.5rem); }
    .ac-health-good { color: #22c55e; }
    .ac-health-warn { color: #f59e0b; }
    .ac-health-bad { color: #ef4444; }
    .ac-btn-sm { padding: 2px 8px; font-size: var(--text-xs, 0.75rem); margin-left: 8px; }
    .ac-btn-compact { padding: 4px 8px; font-size: var(--text-xs, 0.75rem); }
    .ac-btn-filter { padding: 4px 12px; font-size: var(--text-xs, 0.75rem); }
    .ac-scroll-x { overflow-x: auto; }
    .ac-flex-col { display: flex; flex-direction: column; gap: 8px; }
    .ac-flex-row { display: flex; gap: 8px; }
    .ac-flex-row-12 { display: flex; gap: 12px; margin-bottom: 12px; }
    .ac-flex-1 { flex: 1; }
    .ac-select-field { flex: 0 0 auto; background: rgba(0,0,0,0.3); color: #e5e5e5; border: 1px solid var(--ctx-border); border-radius: 6px; padding: 4px 8px; }
    .ac-select-w120 { width: 120px; }
    .ac-select-w100 { width: 100px; }
    .ac-cursor-pointer { cursor: pointer; }
    .ac-text-xs { font-size: var(--text-xs, 0.75rem); }
    .ac-text-sm { font-size: var(--text-sm, 0.875rem); }
    .ac-text-xl { font-size: var(--text-xl, 1.25rem); }
    .ac-text-4xl { font-size: var(--text-4xl, 2.25rem); }
    .ac-opacity-half { opacity: 0.5; }
    .ac-opacity-40 { opacity: 0.4; }
    .ac-opacity-60 { opacity: 0.6; }
    .ac-opacity-70 { opacity: 0.7; }
    .ac-cmd-row { cursor: pointer; }
    .ac-cmd-detail { padding: 8px 12px; background: rgba(0,0,0,0.2); border-radius: 4px; }
    .ac-cmd-output-label { font-size: var(--text-xs, 0.75rem); opacity: 0.6; }
    .ac-cmd-output-pre { margin: 4px 0; padding: 6px 8px; background: rgba(0,0,0,0.3); border-radius: 4px; font-size: var(--text-xs, 0.75rem); white-space: pre-wrap; word-break: break-all; max-height: 150px; overflow-y: auto; }
    .ac-cmd-error-label { font-size: var(--text-xs, 0.75rem); color: #ef4444; }
    .ac-cmd-error-pre { margin: 4px 0; padding: 6px 8px; background: rgba(239,68,68,0.1); border-radius: 4px; font-size: var(--text-xs, 0.75rem); white-space: pre-wrap; color: #fca5a5; }
    .ac-log-output { max-height: 400px; overflow-y: auto; font-size: var(--text-xs, 0.75rem); }
    .ac-log-entry { margin-bottom: 4px; }
    .ac-log-error { color: #ef4444; }
    .ac-log-warn { color: #f59e0b; }
    .ac-log-default { color: #e5e5e5; }
    .ac-log-timestamp { opacity: 0.6; }
    .ac-log-module { opacity: 0.4; }
    .ac-ml-auto { margin-left: auto; }
    .ac-actions-row { display: flex; gap: 4px; }
    .ac-screenshot-center { display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 24px; }
    .ac-screenshot-status { text-align: center; font-size: var(--text-sm, 0.875rem); }
    .ac-screenshot-capture { color: var(--context-primary, #667eea); }
    .ac-screenshot-done { color: #22c55e; }
    .ac-screenshot-error { color: #ef4444; }
    .ac-screenshot-frame { border: 1px solid var(--border-subtle, rgba(255,255,255,0.1)); border-radius: 8px; overflow: hidden; max-width: 100%; }
    .ac-screenshot-img { width: 100%; height: auto; display: block; cursor: pointer; }
    .ac-screenshot-caption { opacity: 0.5; display: block; margin-top: 4px; }
    .ac-execute-btn-lg { padding: 12px 32px; font-size: var(--text-base, 1rem); }
    .ac-plugin-output { max-height: 200px; overflow-y: auto; font-size: var(--text-xs, 0.75rem); }
    .ac-section-muted { opacity: 0.6; }
    .ac-p0-note { font-size: var(--text-xs, 0.75rem); margin: 0; }
    .ac-mb-6 { margin-bottom: 6px; }

    /* Files tab */
    .ac-dropzone {
      border: 2px dashed var(--border-hover, rgba(255,255,255,0.2));
      border-radius: 12px;
      padding: 32px;
      text-align: center;
      transition: all 0.3s ease;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
      cursor: pointer;
    }
    .ac-dropzone:hover, .ac-dropzone-active {
      border-color: var(--context-primary, #00d4aa);
      background: var(--surface-glass, rgba(255,255,255,0.03));
    }
    .ac-dropzone-disabled {
      opacity: 0.5;
      pointer-events: none;
    }
    .ac-file-table {
      width: 100%;
      border-collapse: collapse;
    }
    .ac-file-table th {
      text-align: left;
      font-size: var(--text-xs, 0.75rem);
      opacity: 0.6;
      padding: 8px;
      border-bottom: 1px solid var(--border-default, rgba(255,255,255,0.1));
    }
    .ac-file-table td {
      padding: 8px;
      border-bottom: 1px solid var(--surface-glass, rgba(255,255,255,0.03));
      font-size: var(--text-sm, 0.875rem);
    }
    .ac-file-table tr:hover td {
      background: var(--surface-glass, rgba(255,255,255,0.03));
    }
    .ac-btn-icon {
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px 6px;
      border-radius: 4px;
      transition: background 0.2s;
    }
    .ac-btn-icon:hover {
      background: var(--surface-glass-hover, rgba(255,255,255,0.08));
    }
    .ac-btn-icon:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
    .ac-progress-bar {
      width: 100%;
      height: 4px;
      background: var(--surface-glass, rgba(255,255,255,0.05));
      border-radius: 2px;
      overflow: hidden;
    }
    .ac-progress-fill {
      height: 100%;
      background: var(--context-primary, #00d4aa);
      border-radius: 2px;
    }
    .ac-progress-indeterminate {
      width: 40%;
      animation: ac-progress-slide 1.5s ease-in-out infinite;
    }
    @keyframes ac-progress-slide {
      0% { transform: translateX(-100%); }
      100% { transform: translateX(350%); }
    }
    .ac-loading-state, .ac-empty-state {
      text-align: center;
      padding: 24px;
    }
    .ac-empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
    }
    .ac-mt-xs { margin-top: 6px; }

    /* Responsive */
    @media (max-width: 768px) {
      .modal {
        width: 95%;
        height: 90%;
      }
      
      .info-grid, .metrics-grid {
        grid-template-columns: 1fr;
      }
      
      .power-controls {
        flex-direction: column;
      }
      
      .command-input {
        flex-direction: column;
      }
      
      .process-header,
      .process-row {
        grid-template-columns: 60px 1fr 60px;
        font-size: var(--text-xs, 0.75rem);
      }
      
      .process-header .cpu-col,
      .process-header .memory-col,
      .process-row .cpu-col,
      .process-row .memory-col {
        display: none;
      }
    }
  `]

  constructor() {
    super()
    this.agentId = null
    this.agent = null
    this.isOpen = false
    this.currentTab = 'system'
    this.loading = false
    this.refreshing = false
    this.processes = []
    this.metrics = null
    this.commandOutput = '# Command output will appear here...\n'
    this.commandInput = ''
    this.currentCommandId = null
    this.agentsService = null
    this.latestVersion = null
    this.services = []
    this.commandHistory = []
    this.agentLogs = []
    this.logLevelFilter = null
    this.watchdogData = null
    this.pluginData = null
    this.scheduledTasks = null
    this.screenshotStatus = null
    this.expandedCommandId = null
    this.agentFiles = []
    this.fileTransfers = []
    this.fileDragOver = false
    this.scheduledTaskForm = { name: '', commandType: 'shell', scheduleType: 'once', schedule: '', parameters: '{}' }
    this.notifyForm = { title: '', body: '', urgency: 'normal' }
  }

  connectedCallback() {
    super.connectedCallback()
    this.initializeService()
    
    // Fermer modal avec Escape
    this.handleEscape = (e) => {
      if (e.key === 'Escape' && this.isOpen) {
        this.close()
      }
    }
    document.addEventListener('keydown', this.handleEscape)
    
    // Écouter les événements d'ouverture du modal
    this.handleOpenEvent = (e) => {
      console.debug('Agent control widget received open event:', e.detail)
      this.open(e.detail.agentId)
    }
    document.addEventListener('open-agent-control', this.handleOpenEvent)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this.handleEscape)
    document.removeEventListener('open-agent-control', this.handleOpenEvent)
    this.stopRefreshInterval()
  }

  async initializeService() {
    this.agentsService = document.querySelector('agents-service')
    if (!this.agentsService) {
      this.agentsService = document.createElement('agents-service')
      document.body.appendChild(this.agentsService)
    }
  }

  async open(agentId) {
    console.debug('Opening modal for agent:', agentId)
    this.agentId = agentId
    this.agent = this.agentsService?.getAgentById(agentId)
    console.debug('Found agent:', this.agent)
    this.isOpen = true
    this.setAttribute('is-open', '')  // Ajouter l'attribut HTML pour le CSS
    this.currentTab = 'system'
    
    if (this.agent) {
      // Fetch latest version in background (cached, non-blocking)
      this.agentsService?.getLatestAgentVersion?.().then(v => {
        if (v) this.latestVersion = v
      })
      await this.loadTabData()
      this.startRefreshInterval()
    } else {
      console.warn('Agent not found in service, modal may not display properly')
    }
  }

  close() {
    this.isOpen = false
    this.removeAttribute('is-open')  // Supprimer l'attribut HTML
    this.agentId = null
    this.agent = null
    this.stopRefreshInterval()
  }

  switchTab(tab) {
    this.currentTab = tab
    this.loadTabData()
  }

  startRefreshInterval() {
    this.stopRefreshInterval()
    // Refresh via scheduler centralisé (15s, pause si onglet caché)
    this._unsubscribeRefresh = pollingScheduler.subscribe('15s', () => {
      // Only refresh if modal is open and visible, and not currently loading
      if (this.isOpen && !this.loading) {
        if (this.currentTab === 'processes') {
          this.loadProcesses()
        } else if (this.currentTab === 'metrics') {
          this.loadMetrics()
        } else if (this.currentTab === 'watchdog') {
          this.loadWatchdog()
        } else if (this.currentTab === 'plugins') {
          this.loadPlugins()
        }
      }
    })
  }

  stopRefreshInterval() {
    if (this._unsubscribeRefresh) {
      this._unsubscribeRefresh()
      this._unsubscribeRefresh = null
    }
  }

  async loadTabData() {
    if (!this.agent) return
    
    switch (this.currentTab) {
      case 'processes':
        await this.loadProcesses()
        break
      case 'metrics':
        await this.loadMetrics()
        break
      case 'services':
        await this.loadServices()
        break
      case 'commands':
        await this.loadCommandHistory()
        break
      case 'logs':
        await this.loadAgentLogs()
        break
      case 'watchdog':
        await this.loadWatchdog()
        break
      case 'scheduler':
        await this.loadScheduledTasks()
        break
      case 'plugins':
        await this.loadPlugins()
        break
      case 'screenshot':
        // No data to preload — tab is action-driven
        break
      case 'files':
        await this.loadFiles()
        break
    }
  }

  async loadProcesses() {
    try {
      // Silent loading - keep existing data visible during refresh
      const isInitialLoad = !this.processes || this.processes.length === 0
      if (isInitialLoad) {
        this.loading = true
      } else {
        this.refreshing = true
      }
      
      console.debug('Loading processes for agent:', this.agentId)
      const newProcesses = await this.agentsService.getAgentProcesses(this.agentId)
      console.debug('Loaded processes:', newProcesses)
      
      // Only update if we got valid data
      if (newProcesses && (Array.isArray(newProcesses) || newProcesses.total_count !== undefined)) {
        this.processes = newProcesses
      }
    } catch (error) {
      console.error('Failed to load processes:', error)
      // Don't clear existing data on refresh errors - keep what we have
    } finally {
      this.loading = false
      this.refreshing = false
    }
  }

  async loadMetrics() {
    try {
      // Silent loading - keep existing data visible during refresh  
      const isInitialLoad = !this.metrics
      if (isInitialLoad) {
        this.loading = true
      } else {
        this.refreshing = true
      }
      
      console.debug('Loading metrics for agent:', this.agentId)
      const newMetrics = await this.agentsService.getAgentMetrics(this.agentId)
      console.debug('Loaded metrics:', newMetrics)
      
      // Only update if we got valid data
      if (newMetrics && (newMetrics.cpu || newMetrics.memory)) {
        this.metrics = newMetrics
      }
    } catch (error) {
      console.error('Failed to load metrics:', error)
      // Don't clear existing data on refresh errors - keep what we have
    } finally {
      this.loading = false
      this.refreshing = false
    }
  }

  async loadServices() {
    try {
      this.loading = true
      const result = await this.agentsService.getAgentServices(this.agentId)
      this.services = result?.services || []
    } catch (error) {
      console.error('Failed to load services:', error)
    } finally {
      this.loading = false
    }
  }

  async loadCommandHistory() {
    try {
      const result = await this.agentsService.getCommandHistory(this.agentId, 20, 0)
      this.commandHistory = result?.history || []
    } catch (error) {
      console.error('Failed to load command history:', error)
    }
  }

  async loadAgentLogs() {
    try {
      this.loading = true
      const result = await this.agentsService.getAgentLogs(this.agentId, this.logLevelFilter)
      this.agentLogs = result?.logs || []
    } catch (error) {
      console.error('Failed to load agent logs:', error)
    } finally {
      this.loading = false
    }
  }

  async loadWatchdog() {
    try {
      this.loading = true
      this.watchdogData = await this.agentsService.getAgentWatchdog(this.agentId)
    } catch (error) {
      console.error('Failed to load watchdog:', error)
      this.watchdogData = null
    } finally {
      this.loading = false
    }
  }

  async loadPlugins() {
    try {
      this.loading = true
      this.pluginData = await this.agentsService.getAgentPlugins(this.agentId)
    } catch (error) {
      console.error('Failed to load plugins:', error)
      this.pluginData = null
    } finally {
      this.loading = false
    }
  }

  async loadScheduledTasks() {
    try {
      this.loading = true
      this.scheduledTasks = await this.agentsService.getScheduledTasks(this.agentId)
    } catch (error) {
      console.error('Failed to load scheduled tasks:', error)
      this.scheduledTasks = null
    } finally {
      this.loading = false
    }
  }

  async sendNotification() {
    if (!this.notifyForm.title.trim()) return
    try {
      await this.agentsService.notifyAgent(
        this.agentId,
        this.notifyForm.title,
        this.notifyForm.body,
        this.notifyForm.urgency
      )
      this.notifyForm = { title: '', body: '', urgency: 'normal' }
      this.requestUpdate()
      alert('Notification sent to agent')
    } catch (error) {
      console.error('Failed to send notification:', error)
      alert(`Failed to send notification: ${error.message}`)
    }
  }

  async takeScreenshot() {
    try {
      this.screenshotStatus = 'capturing'
      this.screenshotImage = null
      this.requestUpdate()
      const result = await this.agentsService.takeScreenshot(this.agentId, true)
      if (result?.command_id) {
        this.screenshotStatus = `polling:${result.command_id}`
        this.requestUpdate()
        this._pollScreenshot(result.command_id)
      } else {
        this.screenshotStatus = 'sent'
        this.requestUpdate()
      }
    } catch (error) {
      console.error('Failed to take screenshot:', error)
      this.screenshotStatus = `error:${error.message}`
      this.requestUpdate()
    }
  }

  async _pollScreenshot(commandId) {
    let attempts = 0
    const poll = async () => {
      if (attempts++ > 30) {
        this.screenshotStatus = 'error:Timeout waiting for screenshot'
        this.requestUpdate()
        return
      }
      try {
        const status = await this.agentsService.getCommandStatus(commandId)
        if (status.status === 'Completed' && status.output?.image_base64) {
          this.screenshotImage = `data:${status.output.content_type || 'image/png'};base64,${status.output.image_base64}`
          this.screenshotStatus = `done:${status.output.filename || 'screenshot.png'}`
          this.requestUpdate()
          return
        } else if (status.status === 'Completed') {
          this.screenshotStatus = `done:${status.output?.filename || 'unknown'}`
          this.requestUpdate()
          return
        } else if (status.status === 'Failed') {
          this.screenshotStatus = `error:${status.error?.message || 'Capture failed'}`
          this.requestUpdate()
          return
        }
      } catch (e) { /* continue polling */ }
      setTimeout(poll, 1000)
    }
    poll()
  }

  async createScheduledTask() {
    const f = this.scheduledTaskForm
    if (!f.name.trim() || !f.schedule.trim()) return
    try {
      let params = {}
      try { params = JSON.parse(f.parameters) } catch { /* empty */ }
      await this.agentsService.createScheduledTask(
        this.agentId, f.name, f.commandType, f.schedule, params
      )
      this.scheduledTaskForm = { name: '', commandType: 'shell', scheduleType: 'once', schedule: '', parameters: '{}' }
      await this.loadScheduledTasks()
    } catch (error) {
      console.error('Failed to create scheduled task:', error)
      alert(`Failed to create task: ${error.message}`)
    }
  }

  async deleteScheduledTask(taskName) {
    if (!confirm(`Delete scheduled task "${taskName}"?`)) return
    try {
      await this.agentsService.deleteScheduledTask(this.agentId, taskName)
      await this.loadScheduledTasks()
    } catch (error) {
      console.error('Failed to delete scheduled task:', error)
      alert(`Failed to delete task: ${error.message}`)
    }
  }

  async controlService(serviceName, action) {
    try {
      await this.agentsService.controlService(this.agentId, serviceName, action)
      // Reload services after action
      setTimeout(() => this.loadServices(), 2000)
    } catch (error) {
      console.error(`Failed to ${action} service ${serviceName}:`, error)
      alert(`Failed to ${action} service: ${error.message}`)
    }
  }

  async executePowerAction(action) {
    if (!this.agentsService.isAgentOnline(this.agentId)) {
      alert('⚠️ Agent is offline - cannot execute command')
      return
    }

    const confirmMsg = `Are you sure you want to ${action} ${this.agent.hostname}?`
    if (!confirm(confirmMsg)) return

    try {
      switch (action) {
        case 'shutdown':
          await this.agentsService.shutdownAgent(this.agentId)
          break
        case 'reboot':
          await this.agentsService.rebootAgent(this.agentId)
          break
        case 'hibernate':
          await this.agentsService.hibernateAgent(this.agentId)
          break
      }
      
      // Fermer modal après action power
      this.close()
      
    } catch (error) {
      console.error(`Failed to ${action}:`, error)
      alert(`❌ Failed to ${action}: ${error.message}`)
    }
  }

  async killProcess(pid) {
    if (!this.agent || this.agent.status !== 'online') {
      alert('Agent is offline — cannot kill process')
      return
    }
    const confirmMsg = `Kill process ${pid}?`
    if (!confirm(confirmMsg)) return

    try {
      await this.agentsService.killAgentProcess(this.agentId, pid)
      await this.loadProcesses() // Refresh
    } catch (error) {
      console.error('Failed to kill process:', error)
      alert(`❌ Failed to kill process: ${error.message}`)
    }
  }

  async executeCommand() {
    if (!this.commandInput.trim()) return

    const command = this.commandInput.trim()
    this.commandOutput += `$ ${command}\n`
    
    try {
      const result = await this.agentsService.executeCommand(this.agentId, command)
      this.commandOutput += result.output || result.data || 'Command executed successfully\n'
    } catch (error) {
      this.commandOutput += `Error: ${error.message}\n`
    }
    
    this.commandOutput += '\n'
    this.commandInput = ''
    this.requestUpdate()
  }

  async executeCommandTracked() {
    if (!this.commandInput.trim()) return

    const command = this.commandInput.trim()
    this.commandOutput += `$ ${command}\n`
    this.commandOutput += `Starting command execution...\n`
    
    try {
      // Start command with tracking
      const result = await this.agentsService.executeCommandWithTracking(this.agentId, command, 60)
      this.currentCommandId = result.command_id
      
      this.commandOutput += `Command started (ID: ${this.currentCommandId})\n`
      this.commandOutput += `Waiting for response...\n`
      
      // Poll for status updates
      this.pollCommandStatus()
      
    } catch (error) {
      this.commandOutput += `Error starting command: ${error.message}\n`
      this.currentCommandId = null
    }
    
    this.commandInput = ''
    this.requestUpdate()
  }

  /** Format bytes to human-readable (KB, MB, GB) */
  _formatBytes(bytes) {
    if (bytes == null || bytes === 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB']
    const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
    return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`
  }

  /** Format command output/error from JSON value to displayable string */
  _formatOutput(value) {
    if (!value) return ''
    if (typeof value === 'string') return value
    // JSON objects: extract stdout/message or pretty-print
    if (typeof value === 'object') {
      if (value.stdout) return value.stdout
      if (value.message) return value.message
      if (value.output) return value.output
      return JSON.stringify(value, null, 2)
    }
    return String(value)
  }

  async pollCommandStatus() {
    if (!this.currentCommandId) return

    let attempts = 0
    const maxAttempts = 120 // 2 minutes max

    const poll = async () => {
      try {
        const status = await this.agentsService.getCommandStatus(this.currentCommandId)

        if (status.status === 'Completed') {
          this.commandOutput += `\n=== Command Completed ===\n`
          this.commandOutput += this._formatOutput(status.output) || 'No output\n'
          this.currentCommandId = null
          this.requestUpdate()
          return
        } else if (status.status === 'Failed') {
          this.commandOutput += `\n=== Command Failed ===\n`
          if (status.output) {
            this.commandOutput += this._formatOutput(status.output) + '\n'
          }
          this.commandOutput += this._formatOutput(status.error) || 'Unknown error\n'
          this.currentCommandId = null
          this.requestUpdate()
          return
        } else if (status.status === 'Cancelled') {
          this.commandOutput += `\n=== Command Cancelled ===\n`
          this.currentCommandId = null
          this.requestUpdate()
          return
        } else if (status.status === 'TimedOut') {
          this.commandOutput += `\n=== Command Timed Out ===\n`
          this.commandOutput += this._formatOutput(status.error) || 'No response from agent\n'
          this.currentCommandId = null
          this.requestUpdate()
          return
        }

        // Command still running, continue polling
        attempts++
        if (attempts < maxAttempts) {
          setTimeout(poll, 1000) // Poll every second
        } else {
          this.commandOutput += `\n=== Command Timeout (client) ===\n`
          this.currentCommandId = null
          this.requestUpdate()
        }

      } catch (error) {
        console.error('[agent-control] Poll error:', error.message || error)
        attempts++
        if (attempts < maxAttempts) {
          setTimeout(poll, 2000) // Retry after 2s on error
        } else {
          this.commandOutput += `\nFailed to get command status: ${error.message}\n`
          this.currentCommandId = null
          this.requestUpdate()
        }
      }
    }

    // Start polling after 1 second
    setTimeout(poll, 1000)
  }

  async cancelCurrentCommand() {
    if (!this.currentCommandId) return
    
    try {
      await this.agentsService.cancelCommand(this.currentCommandId)
      this.commandOutput += `\nCancellation requested for command ${this.currentCommandId}...\n`
      this.requestUpdate()
    } catch (error) {
      console.error('Failed to cancel command:', error)
      this.commandOutput += `\nFailed to cancel command: ${error.message}\n`
      this.requestUpdate()
    }
  }

  handleCommandKeyPress(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      this.executeCommandTracked()
    }
  }

  async openLocalDashboard() {
    const agentIP = this.agentsService?.getAgentIP(this.agentId)
    if (!agentIP) {
      alert('⚠️ Cannot determine agent IP address')
      return
    }

    const dashboardURL = this.agentsService.getAgentLocalDashboardURL(agentIP)
    console.debug('Opening agent local dashboard:', dashboardURL)
    
    // Open in new tab
    window.open(dashboardURL, '_blank', 'noopener,noreferrer')
  }

  async reconnectAgent() {
    if (!this.agentId) {
      alert('⚠️ No agent selected')
      return
    }

    try {
      await this.agentsService.reconnectAgent(this.agentId)
      alert('✅ Reconnection signal sent to agent via kernel')
    } catch (error) {
      console.error('Failed to reconnect agent:', error)
      alert(`❌ Failed to reconnect: ${error.message}`)
    }
  }

  renderSystemTab() {
    if (!this.agent) return html``

    const isOnline = this.agent.status === 'online'

    return html`
      <div class="section">
        <div class="section-title">⚡ Power Management</div>
        <div class="power-controls">
          <button 
            class="power-btn danger"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('shutdown')}">
            🔴 Shutdown
          </button>
          <button 
            class="power-btn warning"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('reboot')}">
            🔄 Reboot
          </button>
          <button 
            class="power-btn warning"
            ?disabled="${!isOnline}"
            @click="${() => this.executePowerAction('hibernate')}">
            💤 Hibernate
          </button>
        </div>
      </div>

      <div class="section">
        <div class="section-title">🌐 Local Dashboard</div>
        <div class="power-controls">
          <button 
            class="power-btn ac-power-btn-blue"
            ?disabled="${!this.agentsService?.hasLocalDashboard(this.agentId)}"
            @click="${this.openLocalDashboard}">
            🖥️ Open Local Dashboard
          </button>
          <button 
            class="power-btn ac-power-btn-green"
            ?disabled="${!isOnline}"
            @click="${this.reconnectAgent}">
            🔄 Reconnect Agent
          </button>
        </div>
      </div>

      ${this.agent.health_score != null ? html`
        <div class="section">
          <div class="section-title">💓 Health Score</div>
          <div class="info-grid">
            <div class="info-card ac-span-2">
              <div class="info-label">Overall Health</div>
              <div class="info-value ac-health-score ${this.agent.health_score >= 80 ? 'ac-health-good' : this.agent.health_score >= 50 ? 'ac-health-warn' : 'ac-health-bad'}">
                ${this.agent.health_score}/100
              </div>
            </div>
          </div>
        </div>
      ` : ''}

      <div class="section">
        <div class="section-title">📊 System Information</div>
        <div class="info-grid">
          <div class="info-card">
            <div class="info-label">Hostname</div>
            <div class="info-value">${this.agent.hostname}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Operating System</div>
            <div class="info-value">${this.agent.os} ${this.agent.architecture}</div>
          </div>
          <div class="info-card">
            <div class="info-label">IP Address</div>
            <div class="info-value">${this.agent.primary_ip}</div>
          </div>
          <div class="info-card">
            <div class="info-label">MAC Address</div>
            <div class="info-value">${this.agent.primary_mac}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Agent ID</div>
            <div class="info-value">${this.agent.agent_id}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Last Seen</div>
            <div class="info-value">${this.agentsService?.formatLastSeen(this.agent) || 'Unknown'}</div>
          </div>
          ${this.agent.version ? html`
            <div class="info-card">
              <div class="info-label">Version</div>
              <div class="info-value ${this.latestVersion && this.agent.version !== this.latestVersion ? 'ac-version-outdated' : ''}">
                ${this.agent.version}${this.latestVersion && this.agent.version !== this.latestVersion ? html`<span class="ac-version-icon" title="Update available (latest: ${this.latestVersion})">&#9888;</span>` : ''}
              </div>
            </div>
          ` : ''}
        </div>
      </div>

      <div class="section">
        <div class="section-title">🛠️ Capabilities</div>
        <div class="info-grid">
          ${this.agent.capabilities?.map(cap => html`
            <div class="info-card">
              <div class="info-label">Capability</div>
              <div class="info-value">${cap.replace(/_/g, ' ')}</div>
            </div>
          `) || html`<div class="info-card"><div class="info-value">No capabilities data</div></div>`}
        </div>
      </div>
    `
  }

  renderProcessesTab() {
    if (this.loading) {
      return html`<div class="loading-state">🔄 Loading processes...</div>`
    }

    if (!this.processes) {
      return html`
        <div class="error-state">
          📋 No process data available<br>
          <small>Process monitoring may not be supported on this agent</small>
        </div>
      `
    }

    // L'API retourne un objet avec top_cpu, top_memory, running_count, total_count
    const allProcesses = [
      ...(this.processes.top_cpu || []),
      ...(this.processes.top_memory || [])
    ]
    
    // Dédupliquer par PID
    const uniqueProcesses = allProcesses.reduce((acc, proc) => {
      if (!acc.find(p => p.pid === proc.pid)) {
        acc.push(proc)
      }
      return acc
    }, [])

    return html`
      <div class="section">
        <div class="section-title">
          📋 Running Processes 
          <span class="ac-meta-inline">
            (top 15 by CPU/memory • ${this.processes.running_count || 0} running, ${this.processes.total_count || 0} total)
          </span>
          ${this.refreshing && this.currentTab === 'processes' ? html`<span class="ac-refresh-indicator">🔄</span>` : ''}
        </div>
        <div class="processes-table">
          <div class="process-header">
            <span>PID</span>
            <span>Name</span>
            <span class="cpu-col">CPU %</span>
            <span class="memory-col">Memory</span>
            <span>Actions</span>
          </div>
          ${uniqueProcesses.length > 0 ? uniqueProcesses.map(proc => html`
            <div class="process-row">
              <span>${proc.pid}</span>
              <span class="process-name">${proc.name}</span>
              <span class="cpu-col">${(proc.cpu_percent || 0).toFixed(1)}%</span>
              <span class="memory-col">${(proc.memory_mb || 0).toFixed(1)}MB</span>
              <span>
                <button 
                  class="kill-btn"
                  @click="${() => this.killProcess(proc.pid)}">
                  Kill
                </button>
              </span>
            </div>
          `) : html`
            <div class="process-row">
              <span colspan="5" class="ac-text-center-muted">No top processes to display</span>
            </div>
          `}
        </div>
      </div>
    `
  }

  renderMetricsTab() {
    if (this.loading) {
      return html`<div class="loading-state">🔄 Loading metrics...</div>`
    }

    if (!this.metrics) {
      return html`
        <div class="error-state">
          📊 No metrics data available<br>
          <small>Real-time metrics may not be supported on this agent</small>
        </div>
      `
    }

    const cpuPercent = this.metrics.cpu?.percent || 0
    const memoryPercent = this.metrics.memory?.percent_used || 0
    const diskPercent = this.metrics.disk?.[0]?.percent_used || 0
    const uptimeHours = Math.round((this.metrics.uptime_seconds || 0) / 3600)
    const memoryUsedGB = ((this.metrics.memory?.used_mb || 0) / 1024).toFixed(1)
    const memoryTotalGB = ((this.metrics.memory?.total_mb || 0) / 1024).toFixed(1)

    return html`
      <div class="section">
        <div class="section-title">
          📊 System Metrics
          ${this.refreshing && this.currentTab === 'metrics' ? html`<span class="ac-refresh-indicator">🔄</span>` : ''}
        </div>
        <div class="metrics-grid">
          <div class="metric-card">
            <div class="metric-label">CPU Usage</div>
            <div class="metric-value">${cpuPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill cpu" style="width: ${cpuPercent}%"></div>
            </div>
            ${this.metrics.cpu?.core_count ? html`
              <div class="ac-meta-hint">
                ${this.metrics.cpu.core_count} cores
              </div>
            ` : ''}
          </div>
          <div class="metric-card">
            <div class="metric-label">Memory Usage</div>
            <div class="metric-value">${memoryPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill memory" style="width: ${memoryPercent}%"></div>
            </div>
            <div class="ac-meta-hint">
              ${memoryUsedGB} / ${memoryTotalGB} GB
            </div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Disk Usage</div>
            <div class="metric-value">${diskPercent.toFixed(1)}%</div>
            <div class="progress-bar">
              <div class="progress-fill disk" style="width: ${diskPercent}%"></div>
            </div>
            ${this.metrics.disk?.[0] ? html`
              <div class="ac-meta-hint">
                ${this.metrics.disk[0].path}: ${this.metrics.disk[0].used_gb?.toFixed(1)} / ${this.metrics.disk[0].total_gb?.toFixed(1)} GB
              </div>
            ` : ''}
          </div>
          <div class="metric-card">
            <div class="metric-label">Uptime</div>
            <div class="metric-value">${uptimeHours}h</div>
            ${this.metrics.cpu?.load_avg ? html`
              <div class="ac-meta-hint">
                Load: ${this.metrics.cpu.load_avg.map(l => l.toFixed(2)).join(', ')}
              </div>
            ` : ''}
          </div>
        </div>
      </div>

      ${this.metrics.gpu?.gpus?.length ? html`
        <div class="section">
          <div class="section-title">🎮 GPU</div>
          <div class="metrics-grid">
            ${this.metrics.gpu.gpus.map(gpu => html`
              <div class="metric-card">
                <div class="metric-label">${gpu.name || 'GPU'}</div>
                ${gpu.utilization_percent != null ? html`
                  <div class="metric-value">${gpu.utilization_percent.toFixed(0)}%</div>
                  <div class="progress-bar">
                    <div class="progress-fill gpu" style="width: ${gpu.utilization_percent}%"></div>
                  </div>
                ` : html`
                  <div class="metric-value">N/A</div>
                `}
                <div class="ac-meta-hint">${gpu.vendor || 'unknown'}</div>
              </div>
              ${gpu.memory_total_mb ? html`
                <div class="metric-card">
                  <div class="metric-label">VRAM</div>
                  <div class="metric-value">${((gpu.memory_used_mb || 0) / 1024).toFixed(1)} / ${(gpu.memory_total_mb / 1024).toFixed(1)} GB</div>
                  <div class="progress-bar">
                    <div class="progress-fill gpu-mem" style="width: ${((gpu.memory_used_mb || 0) / gpu.memory_total_mb * 100)}%"></div>
                  </div>
                  ${gpu.temperature_celsius != null ? html`
                    <div class="ac-meta-hint">🌡️ ${gpu.temperature_celsius.toFixed(0)}°C${gpu.power_watts != null ? ` · ⚡ ${gpu.power_watts.toFixed(0)}W` : ''}</div>
                  ` : ''}
                </div>
              ` : ''}
            `)}
          </div>
        </div>
      ` : ''}

      ${this.metrics.disk_io?.disks?.length ? html`
        <div class="section">
          <div class="section-title">💾 Disk I/O</div>
          <div class="metrics-grid">
            ${this.metrics.disk_io.disks.map(d => {
              const isIdle = !d.read_bytes_per_sec && !d.write_bytes_per_sec && !d.read_iops && !d.write_iops
              return html`
                <div class="metric-card">
                  <div class="metric-label">${d.device}</div>
                  ${isIdle ? html`
                    <div class="ac-meta-hint ac-text-center-muted ac-opacity-half">Idle</div>
                  ` : html`
                    <div class="io-stats">
                      <div class="io-stat">
                        <span class="io-stat-label">⬇ Read</span>
                        <span class="io-stat-value">${this._formatBytes(d.read_bytes_per_sec)}/s</span>
                      </div>
                      <div class="io-stat">
                        <span class="io-stat-label">⬆ Write</span>
                        <span class="io-stat-value">${this._formatBytes(d.write_bytes_per_sec)}/s</span>
                      </div>
                      <div class="io-stat">
                        <span class="io-stat-label">Read IOPS</span>
                        <span class="io-stat-value">${d.read_iops}</span>
                      </div>
                      <div class="io-stat">
                        <span class="io-stat-label">Write IOPS</span>
                        <span class="io-stat-value">${d.write_iops}</span>
                      </div>
                    </div>
                  `}
                </div>
              `
            })}
          </div>
        </div>
      ` : ''}

      ${this.metrics.network_advanced ? html`
        <div class="section">
          <div class="section-title">🌐 Network</div>
          <div class="metric-card">
            ${this.metrics.network_advanced.gateway_latency_ms != null ? html`
              <div class="net-stat-row">
                <span class="net-stat-label">Gateway Latency</span>
                <span class="net-stat-value">${this.metrics.network_advanced.gateway_latency_ms.toFixed(1)} ms</span>
              </div>
            ` : ''}
            ${this.metrics.network_advanced.dns_latency_ms != null ? html`
              <div class="net-stat-row">
                <span class="net-stat-label">DNS Latency</span>
                <span class="net-stat-value">${this.metrics.network_advanced.dns_latency_ms.toFixed(1)} ms</span>
              </div>
            ` : ''}
            ${this.metrics.network_advanced.active_connections != null ? html`
              <div class="net-stat-row">
                <span class="net-stat-label">Active Connections</span>
                <span class="net-stat-value">${this.metrics.network_advanced.active_connections}</span>
              </div>
            ` : ''}
            ${this.metrics.network_advanced.interfaces?.length ? html`
              ${this.metrics.network_advanced.interfaces.map(iface => html`
                <div class="net-stat-row">
                  <span class="net-stat-label">${iface.name}</span>
                  <span class="net-stat-value">⬇ ${this._formatBytes(iface.rx_bytes_per_sec)}/s · ⬆ ${this._formatBytes(iface.tx_bytes_per_sec)}/s</span>
                </div>
              `)}
            ` : ''}
          </div>
        </div>
      ` : ''}
    `
  }

  renderCommandsTab() {
    const isOnline = this.agent && this.agent.status === 'online'
    const canExecute = this.agentsService?.canExecuteCommands(this.agentId)

    if (!canExecute) {
      return html`
        <div class="section">
          <div class="section-title">💻 Command Execution</div>
          <div class="error-state">
            ⚠️ Command execution not supported<br>
            <small>This agent does not have command execution capabilities</small>
          </div>
        </div>
      `
    }

    return html`
      <div class="section">
        <div class="section-title">💻 Command Execution</div>
        <div class="command-section">
          <div class="command-input">
            <input 
              type="text" 
              class="command-field"
              placeholder="Enter command (e.g., ls -la, ps aux, systemctl status)"
              .value="${this.commandInput}"
              @input="${(e) => this.commandInput = e.target.value}"
              @keypress="${this.handleCommandKeyPress}"
              ?disabled="${!isOnline}"
            />
            <button 
              class="execute-btn"
              @click="${this.executeCommandTracked}"
              ?disabled="${!isOnline || !this.commandInput.trim()}">
              ▶️ Execute
            </button>
          </div>
          <div class="command-output">${this.commandOutput}</div>
          ${this.currentCommandId ? html`
            <div class="command-actions ac-cmd-actions">
              <button
                class="power-btn danger ac-cmd-cancel"
                @click="${this.cancelCurrentCommand}">
                ⏹️ Cancel Command
              </button>
              <span class="ac-cmd-id">
                Command ID: ${this.currentCommandId}
              </span>
            </div>
          ` : ''}
        </div>
      </div>

      <div class="section">
        <div class="section-title">📋 Command History
          <button class="power-btn ac-btn-sm"
            @click="${() => this.loadCommandHistory()}">🔄</button>
        </div>
        ${this.commandHistory.length === 0
          ? html`<div class="error-state">No command history yet</div>`
          : html`
            <div class="ac-scroll-x">
              <table class="ac-table">
                <thead>
                  <tr><th>Time</th><th>Type</th><th>Status</th><th></th></tr>
                </thead>
                <tbody>
                  ${this.commandHistory.slice(0, 15).map(cmd => html`
                    <tr class="ac-cmd-row" @click="${() => { this.expandedCommandId = this.expandedCommandId === cmd.command_id ? null : cmd.command_id; this.requestUpdate() }}">
                      <td class="ac-text-xs ac-opacity-70">${cmd.created_at?.substring(11, 19) || ''}</td>
                      <td>${cmd.command_type}</td>
                      <td>
                        <span class="status-badge ${cmd.status === 'Completed' ? 'online' : cmd.status === 'Failed' ? 'offline' : 'unknown'}">
                          ${cmd.status}
                        </span>
                      </td>
                      <td class="ac-text-xs ac-opacity-half">${this.expandedCommandId === cmd.command_id ? '▼' : '▶'}</td>
                    </tr>
                    ${this.expandedCommandId === cmd.command_id ? html`
                      <tr>
                        <td colspan="4" class="ac-cmd-detail">
                          ${cmd.output ? html`
                            <div class="ac-mb-6">
                              <strong class="ac-cmd-output-label">Output:</strong>
                              <pre class="ac-cmd-output-pre">${typeof cmd.output === 'string' ? cmd.output : JSON.stringify(cmd.output, null, 2)}</pre>
                            </div>
                          ` : ''}
                          ${cmd.error ? html`
                            <div>
                              <strong class="ac-cmd-error-label">Error:</strong>
                              <pre class="ac-cmd-error-pre">${typeof cmd.error === 'string' ? cmd.error : JSON.stringify(cmd.error, null, 2)}</pre>
                            </div>
                          ` : ''}
                          ${!cmd.output && !cmd.error ? html`
                            <span class="ac-text-xs ac-opacity-40">No output data</span>
                          ` : ''}
                        </td>
                      </tr>
                    ` : ''}
                  `)}
                </tbody>
              </table>
            </div>
          `
        }
      </div>
    `
  }

  renderServicesTab() {
    const isOnline = this.agent && this.agent.status === 'online'

    if (!this.services || this.services.length === 0) {
      return html`
        <div class="section">
          <div class="section-title">🔧 Services Management</div>
          <div class="error-state">
            ${this.loading ? 'Loading services...' : 'No services data available'}
            <br><small>Services are reported via agent heartbeat</small>
          </div>
        </div>
      `
    }

    return html`
      <div class="section">
        <div class="section-title">🔧 Services (${this.services.length})</div>
        <div class="ac-scroll-x">
          <table class="ac-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              ${this.services.map(svc => html`
                <tr>
                  <td>${svc.name}</td>
                  <td>
                    <span class="status-badge ${svc.status === 'active' || svc.status === 'running' ? 'online' : svc.status === 'failed' ? 'offline' : 'unknown'}">
                      ${svc.status}
                    </span>
                  </td>
                  <td>
                    <div class="ac-actions-row">
                      <button class="power-btn ac-power-btn-green ac-btn-compact"
                        ?disabled="${!isOnline}" @click="${() => this.controlService(svc.name, 'start')}">Start</button>
                      <button class="power-btn danger ac-btn-compact"
                        ?disabled="${!isOnline}" @click="${() => this.controlService(svc.name, 'stop')}">Stop</button>
                      <button class="power-btn warning ac-btn-compact"
                        ?disabled="${!isOnline}" @click="${() => this.controlService(svc.name, 'restart')}">Restart</button>
                    </div>
                  </td>
                </tr>
              `)}
            </tbody>
          </table>
        </div>
      </div>
    `
  }

  renderLogsTab() {
    return html`
      <div class="section">
        <div class="section-title">📜 Agent Logs</div>
        <div class="ac-flex-row-12">
          <button class="power-btn ${!this.logLevelFilter ? 'ac-power-btn-blue' : ''} ac-btn-filter"
            @click="${() => { this.logLevelFilter = null; this.loadAgentLogs() }}">All</button>
          <button class="power-btn ${this.logLevelFilter === 'WARN' ? 'warning' : ''} ac-btn-filter"
            @click="${() => { this.logLevelFilter = 'WARN'; this.loadAgentLogs() }}">WARN</button>
          <button class="power-btn ${this.logLevelFilter === 'ERROR' ? 'danger' : ''} ac-btn-filter"
            @click="${() => { this.logLevelFilter = 'ERROR'; this.loadAgentLogs() }}">ERROR</button>
          <button class="power-btn ac-btn-filter ac-ml-auto"
            @click="${() => this.loadAgentLogs()}">🔄 Refresh</button>
        </div>
        ${this.agentLogs.length === 0
          ? html`<div class="error-state">${this.loading ? 'Loading logs...' : 'No logs available'}<br><small>Only WARN/ERROR logs are collected by default</small></div>`
          : html`
            <div class="command-output ac-log-output">
              ${this.agentLogs.map(log => html`
                <div class="ac-log-entry ${log.level === 'ERROR' ? 'ac-log-error' : log.level === 'WARN' ? 'ac-log-warn' : 'ac-log-default'}">
                  <span class="ac-log-timestamp">${log.timestamp?.substring(11, 19) || ''}</span>
                  [${log.level}] ${log.message}
                  ${log.module ? html`<span class="ac-log-module"> (${log.module})</span>` : ''}
                </div>
              `)}
            </div>
          `
        }
      </div>
    `
  }

  renderWatchdogTab() {
    if (this.loading) {
      return html`<div class="loading-state">Loading watchdog data...</div>`
    }

    if (!this.watchdogData) {
      return html`
        <div class="section">
          <div class="section-title">🛡️ Watchdog Status</div>
          <div class="error-state">
            No watchdog data available
            <br><small>Watchdog data is reported via agent heartbeat (v2.5+)</small>
          </div>
        </div>
      `
    }

    const resp = this.watchdogData
    const wd = resp.watchdog
    const componentStatus = (s) => s === 'healthy' || s === 'ok' ? '#22c55e' : s === 'degraded' ? '#f59e0b' : '#ef4444'

    if (!wd) {
      return html`
        <div class="section">
          <div class="section-title">🛡️ Watchdog Status</div>
          <div class="error-state">
            No watchdog report from agent
            <br><small>Agent may be running a version older than v2.5</small>
          </div>
        </div>
        ${resp.health_score != null ? html`
          <div class="section">
            <div class="section-title">💓 Health Score</div>
            <div class="info-grid">
              <div class="info-card ac-span-2">
                <div class="info-label">Score</div>
                <div class="info-value ac-health-score ${resp.health_score >= 80 ? 'ac-health-good' : resp.health_score >= 50 ? 'ac-health-warn' : 'ac-health-bad'}">
                  ${resp.health_score}/100
                </div>
              </div>
            </div>
          </div>
        ` : ''}
      `
    }

    const statusColor = wd.status === 'healthy' ? '#22c55e' : wd.status === 'degraded' ? '#f59e0b' : '#ef4444'

    return html`
      <div class="section">
        <div class="section-title">🛡️ Watchdog Status
          <button class="power-btn ac-btn-sm"
            @click="${() => this.loadWatchdog()}">🔄</button>
        </div>
        <div class="info-grid">
          <div class="info-card ac-span-2">
            <div class="info-label">Overall Status</div>
            <div class="info-value ac-text-xl" style="color: ${statusColor}">
              ${wd.status?.toUpperCase() || 'UNKNOWN'}
            </div>
          </div>
          <div class="info-card">
            <div class="info-label">MQTT</div>
            <div class="info-value" style="color: ${componentStatus(wd.mqtt_status)}">${wd.mqtt_status || 'N/A'}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Metrics</div>
            <div class="info-value" style="color: ${componentStatus(wd.metrics_status)}">${wd.metrics_status || 'N/A'}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Heartbeat</div>
            <div class="info-value" style="color: ${componentStatus(wd.heartbeat_status)}">${wd.heartbeat_status || 'N/A'}</div>
          </div>
          <div class="info-card">
            <div class="info-label">Recovery Attempts</div>
            <div class="info-value">${wd.recovery_attempts ?? 0}</div>
          </div>
        </div>
      </div>

      ${resp.health_score != null ? html`
        <div class="section">
          <div class="section-title">💓 Health Score</div>
          <div class="info-grid">
            <div class="info-card ac-span-2">
              <div class="info-label">Score</div>
              <div class="info-value ac-health-score ${resp.health_score >= 80 ? 'ac-health-good' : resp.health_score >= 50 ? 'ac-health-warn' : 'ac-health-bad'}">
                ${resp.health_score}/100
              </div>
            </div>
            ${resp.health_details ? html`
              <div class="info-card">
                <div class="info-label">Heartbeat</div>
                <div class="info-value">${resp.health_details.heartbeat_score}/25</div>
              </div>
              <div class="info-card">
                <div class="info-label">Resources</div>
                <div class="info-value">${resp.health_details.resource_score}/25</div>
              </div>
              <div class="info-card">
                <div class="info-label">Uptime</div>
                <div class="info-value">${resp.health_details.uptime_score}/25</div>
              </div>
              <div class="info-card">
                <div class="info-label">Commands</div>
                <div class="info-value">${resp.health_details.command_score}/25</div>
              </div>
            ` : ''}
          </div>
        </div>
      ` : ''}
    `
  }

  renderSchedulerTab() {
    const isOnline = this.agent && this.agent.status === 'online'

    return html`
      <div class="section">
        <div class="section-title">📅 Create Scheduled Task</div>
        <div class="ac-flex-col">
          <div class="ac-flex-row">
            <input type="text" class="command-field ac-flex-1" placeholder="Task name"
              .value="${this.scheduledTaskForm.name}"
              @input="${(e) => { this.scheduledTaskForm = {...this.scheduledTaskForm, name: e.target.value}; this.requestUpdate() }}" />
            <select class="command-field ac-select-field ac-select-w120"
              @change="${(e) => { this.scheduledTaskForm = {...this.scheduledTaskForm, commandType: e.target.value}; this.requestUpdate() }}">
              <option value="shell" ?selected="${this.scheduledTaskForm.commandType === 'shell'}">Shell</option>
              <option value="reboot" ?selected="${this.scheduledTaskForm.commandType === 'reboot'}">Reboot</option>
              <option value="shutdown" ?selected="${this.scheduledTaskForm.commandType === 'shutdown'}">Shutdown</option>
            </select>
          </div>
          <div class="ac-flex-row">
            <input type="text" class="command-field ac-flex-1" placeholder="Schedule (cron: '0 2 * * *' or interval: '30m')"
              .value="${this.scheduledTaskForm.schedule}"
              @input="${(e) => { this.scheduledTaskForm = {...this.scheduledTaskForm, schedule: e.target.value}; this.requestUpdate() }}" />
          </div>
          <div class="ac-flex-row">
            <input type="text" class="command-field ac-flex-1" placeholder='Parameters JSON (e.g. {"cmd": "apt update"})'
              .value="${this.scheduledTaskForm.parameters}"
              @input="${(e) => { this.scheduledTaskForm = {...this.scheduledTaskForm, parameters: e.target.value}; this.requestUpdate() }}" />
            <button class="execute-btn" ?disabled="${!isOnline || !this.scheduledTaskForm.name.trim()}"
              @click="${() => this.createScheduledTask()}">
              ➕ Create
            </button>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="section-title">📋 Scheduled Tasks
          <button class="power-btn ac-btn-sm"
            @click="${() => this.loadScheduledTasks()}">🔄</button>
        </div>
        ${this.loading
          ? html`<div class="loading-state">Loading tasks...</div>`
          : !this.scheduledTasks || (Array.isArray(this.scheduledTasks) && this.scheduledTasks.length === 0)
            ? html`<div class="error-state">No scheduled tasks<br><small>Tasks are managed on the agent via command pipeline</small></div>`
            : html`
              <div class="ac-scroll-x">
                <table class="ac-table">
                  <thead>
                    <tr><th>Name</th><th>Schedule</th><th>Type</th><th>Actions</th></tr>
                  </thead>
                  <tbody>
                    ${(Array.isArray(this.scheduledTasks) ? this.scheduledTasks : []).map(task => html`
                      <tr>
                        <td>${task.name || task.task_name || 'unnamed'}</td>
                        <td class="ac-text-xs">${task.schedule || ''}</td>
                        <td>${task.command_type || task.type || ''}</td>
                        <td>
                          <button class="power-btn danger ac-btn-compact"
                            ?disabled="${!isOnline}"
                            @click="${() => this.deleteScheduledTask(task.name || task.task_name)}">
                            Delete
                          </button>
                        </td>
                      </tr>
                    `)}
                  </tbody>
                </table>
              </div>
            `
        }
      </div>
    `
  }

  renderPluginsTab() {
    if (this.loading) {
      return html`<div class="loading-state">Loading plugins data...</div>`
    }

    const isOnline = this.agent && this.agent.status === 'online'
    // API returns {agent_id, plugin_data: {...}} — unwrap
    const plugins = this.pluginData?.plugin_data || this.pluginData

    if (!plugins || Object.keys(plugins).length === 0) {
      return html`
        <div class="section">
          <div class="section-title">🔌 Plugins Data</div>
          <div class="error-state">
            No plugin data available
            <br><small>Plugin data is reported via agent heartbeat (v2.5+)</small>
          </div>
        </div>
      `
    }

    return html`
      ${plugins.activity_tracker ? html`
        <div class="section">
          <div class="section-title">🖱️ Activity Tracker
            <button class="power-btn ac-btn-sm"
              @click="${() => this.loadPlugins()}">🔄</button>
          </div>
          <div class="info-grid">
            <div class="info-card">
              <div class="info-label">Status</div>
              <div class="info-value">
                <span class="status-badge ${plugins.activity_tracker.is_idle ? 'offline' : 'online'}">
                  ${plugins.activity_tracker.is_idle ? 'Idle' : 'Active'}
                </span>
              </div>
            </div>
            <div class="info-card">
              <div class="info-label">Idle Time</div>
              <div class="info-value">${Math.round((plugins.activity_tracker.idle_secs || 0) / 60)}m</div>
            </div>
            ${plugins.activity_tracker.active_window ? html`
              <div class="info-card ac-span-2">
                <div class="info-label">Active Window</div>
                <div class="info-value ac-text-xs">${plugins.activity_tracker.active_window}</div>
              </div>
            ` : ''}
            ${plugins.activity_tracker.total_active_secs != null ? html`
              <div class="info-card">
                <div class="info-label">Total Active Today</div>
                <div class="info-value">${Math.round(plugins.activity_tracker.total_active_secs / 3600)}h ${Math.round((plugins.activity_tracker.total_active_secs % 3600) / 60)}m</div>
              </div>
            ` : ''}
          </div>
        </div>
      ` : ''}

      ${Object.entries(plugins).filter(([k]) => k !== 'activity_tracker').map(([pluginId, data]) => html`
        <div class="section">
          <div class="section-title">🔌 ${pluginId}</div>
          <div class="command-output ac-plugin-output">
            ${JSON.stringify(data, null, 2)}
          </div>
        </div>
      `)}

      ${isOnline ? html`
        <div class="section">
          <div class="section-title">📤 Send Notification to Agent</div>
          <div class="ac-flex-col">
            <div class="ac-flex-row">
              <input type="text" class="command-field ac-flex-1" placeholder="Title"
                .value="${this.notifyForm.title}"
                @input="${(e) => { this.notifyForm = {...this.notifyForm, title: e.target.value}; this.requestUpdate() }}" />
              <select class="command-field ac-select-field ac-select-w100"
                @change="${(e) => { this.notifyForm = {...this.notifyForm, urgency: e.target.value}; this.requestUpdate() }}">
                <option value="low">Low</option>
                <option value="normal" selected>Normal</option>
                <option value="critical">Critical</option>
              </select>
            </div>
            <div class="ac-flex-row">
              <input type="text" class="command-field ac-flex-1" placeholder="Message body"
                .value="${this.notifyForm.body}"
                @input="${(e) => { this.notifyForm = {...this.notifyForm, body: e.target.value}; this.requestUpdate() }}" />
              <button class="execute-btn" ?disabled="${!this.notifyForm.title.trim()}"
                @click="${() => this.sendNotification()}">
                📤 Send
              </button>
            </div>
          </div>
        </div>
      ` : ''}
    `
  }

  // ===== Files Tab =====

  async loadFiles() {
    try {
      this.loading = true
      const data = await this.agentsService.listAgentFiles(this.agentId)
      this.agentFiles = data?.files || []
    } catch (error) {
      console.error('[files] Failed to load:', error)
      this.agentFiles = []
    } finally {
      this.loading = false
    }
  }

  _handleFileDragOver(e) {
    e.preventDefault()
    e.stopPropagation()
    this.fileDragOver = true
  }

  _handleFileDragLeave(e) {
    e.preventDefault()
    e.stopPropagation()
    this.fileDragOver = false
  }

  _handleFileDrop(e) {
    e.preventDefault()
    e.stopPropagation()
    this.fileDragOver = false
    const files = e.dataTransfer?.files
    if (files && files.length > 0) {
      this._uploadFile(files[0])
    }
  }

  _handleFileSelect(e) {
    const files = e.target?.files
    if (files && files.length > 0) {
      this._uploadFile(files[0])
    }
    // Reset input so the same file can be selected again
    e.target.value = ''
  }

  async _uploadFile(file) {
    if (file.size > 200 * 1024 * 1024) {
      alert('File too large (max 200 MB)')
      return
    }

    const transfer = {
      id: `upload-${Date.now()}`,
      filename: file.name,
      direction: 'upload',
      status: 'uploading',
      size: file.size
    }
    this.fileTransfers = [...this.fileTransfers, transfer]

    try {
      const result = await this.agentsService.uploadFileToAgent(this.agentId, file)
      transfer.status = 'processing'
      transfer.transferId = result.transfer_id
      this.fileTransfers = [...this.fileTransfers]

      // Poll transfer status until agent pulls the file
      await this._pollTransfer(transfer)
    } catch (error) {
      console.error('[files] Upload failed:', error)
      transfer.status = 'failed'
      transfer.error = error.message
      this.fileTransfers = [...this.fileTransfers]
    }
  }

  async _downloadFile(filename) {
    const transfer = {
      id: `download-${Date.now()}`,
      filename,
      direction: 'download',
      status: 'requesting'
    }
    this.fileTransfers = [...this.fileTransfers, transfer]

    try {
      const result = await this.agentsService.requestFileDownload(this.agentId, filename)
      transfer.transferId = result.transfer_id
      transfer.status = 'processing'
      this.fileTransfers = [...this.fileTransfers]

      // Poll until agent pushes the file to kernel
      const finalStatus = await this._pollTransfer(transfer)
      if (finalStatus?.status === 'Completed' && finalStatus?.download_token) {
        const url = this.agentsService.getTransferDownloadUrl(
          transfer.transferId, finalStatus.download_token
        )
        // Trigger browser download
        const a = document.createElement('a')
        a.href = url
        a.download = filename
        a.click()
        transfer.status = 'completed'
      } else if (transfer.status !== 'failed') {
        transfer.status = 'completed'
      }
      this.fileTransfers = [...this.fileTransfers]
    } catch (error) {
      console.error('[files] Download failed:', error)
      transfer.status = 'failed'
      transfer.error = error.message
      this.fileTransfers = [...this.fileTransfers]
    }
  }

  async _deleteFile(filename) {
    if (!confirm(`Delete "${filename}" from agent?`)) return

    try {
      await this.agentsService.deleteAgentFile(this.agentId, filename)
      await this.loadFiles()
    } catch (error) {
      console.error('[files] Delete failed:', error)
      alert(`Delete failed: ${error.message}`)
    }
  }

  async _pollTransfer(transfer) {
    const maxAttempts = 300 // 5 min at 1s interval
    for (let i = 0; i < maxAttempts; i++) {
      await new Promise(r => setTimeout(r, 1000))
      try {
        const status = await this.agentsService.getTransferStatus(transfer.transferId)
        transfer.status = status.status?.toLowerCase() || 'unknown'
        this.fileTransfers = [...this.fileTransfers]

        if (status.status === 'Completed' || status.status === 'Failed' || status.status === 'Expired') {
          if (status.status === 'Failed') {
            transfer.status = 'failed'
            transfer.error = status.error || 'Transfer failed'
          } else if (status.status === 'Completed') {
            transfer.status = 'completed'
          }
          this.fileTransfers = [...this.fileTransfers]
          await this.loadFiles()
          return status
        }
      } catch {
        // Polling error — continue
      }
    }
    transfer.status = 'timeout'
    this.fileTransfers = [...this.fileTransfers]
    return null
  }

  _formatFileSize(bytes) {
    if (!bytes || bytes === 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`
  }

  _removeTransfer(transferId) {
    this.fileTransfers = this.fileTransfers.filter(t => t.id !== transferId)
  }

  renderFilesTab() {
    const isOnline = this.agent && this.agent.status === 'online'
    const activeTransfers = this.fileTransfers.filter(t =>
      t.status !== 'completed' && t.status !== 'failed' && t.status !== 'timeout'
    )
    const doneTransfers = this.fileTransfers.filter(t =>
      t.status === 'completed' || t.status === 'failed' || t.status === 'timeout'
    )

    return html`
      <!-- Upload Zone -->
      <div class="section">
        <div class="section-title">📤 Upload File</div>
        <div class="ac-dropzone ${this.fileDragOver ? 'ac-dropzone-active' : ''} ${!isOnline ? 'ac-dropzone-disabled' : ''}"
             @dragover="${e => this._handleFileDragOver(e)}"
             @dragleave="${e => this._handleFileDragLeave(e)}"
             @drop="${e => this._handleFileDrop(e)}">
          <div class="ac-text-4xl">📤</div>
          <p class="ac-opacity-70">Drag & drop a file here</p>
          <span class="ac-opacity-50">or</span>
          <input type="file" id="fileUploadInput" @change="${e => this._handleFileSelect(e)}" hidden>
          <button class="execute-btn"
            ?disabled="${!isOnline}"
            @click="${() => this.shadowRoot.getElementById('fileUploadInput').click()}">
            Choose File
          </button>
          <small class="ac-opacity-50">Max 200 MB</small>
        </div>
      </div>

      <!-- Active Transfers -->
      ${activeTransfers.length > 0 ? html`
        <div class="section">
          <div class="section-title">🔄 Active Transfers</div>
          ${activeTransfers.map(t => html`
            <div class="info-card">
              <div class="ac-flex ac-items-center ac-gap-sm">
                <span>${t.direction === 'upload' ? '⬆️' : '⬇️'}</span>
                <span class="ac-font-medium">${t.filename}</span>
                <span class="ac-ml-auto ac-opacity-60 ac-text-xs">${t.status}</span>
              </div>
              <div class="ac-progress-bar ac-mt-xs">
                <div class="ac-progress-fill ac-progress-indeterminate"></div>
              </div>
            </div>
          `)}
        </div>
      ` : ''}

      <!-- Done Transfers -->
      ${doneTransfers.length > 0 ? html`
        <div class="section">
          <div class="section-title">📋 Recent Transfers</div>
          ${doneTransfers.map(t => html`
            <div class="info-card ac-flex ac-items-center ac-gap-sm">
              <span>${t.status === 'completed' ? '✅' : '❌'}</span>
              <span>${t.direction === 'upload' ? '⬆️' : '⬇️'} ${t.filename}</span>
              <span class="ac-ml-auto ac-opacity-60 ac-text-xs">${t.status}</span>
              <button class="ac-btn-icon" @click="${() => this._removeTransfer(t.id)}" title="Dismiss">✕</button>
            </div>
          `)}
        </div>
      ` : ''}

      <!-- File List -->
      <div class="section">
        <div class="section-title ac-flex ac-items-center ac-gap-sm">
          📁 Agent Files
          <button class="ac-btn-icon ac-ml-auto" @click="${() => this.loadFiles()}" title="Refresh">🔄</button>
        </div>

        ${this.loading ? html`
          <div class="ac-loading-state">Loading files...</div>
        ` : this.agentFiles.length === 0 ? html`
          <div class="ac-empty-state">
            <div class="ac-text-4xl">📂</div>
            <p class="ac-opacity-60">No files on this agent</p>
          </div>
        ` : html`
          <table class="ac-file-table">
            <thead>
              <tr>
                <th>File</th>
                <th>Size</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              ${this.agentFiles.map(f => html`
                <tr>
                  <td class="ac-font-medium">${f.name}</td>
                  <td class="ac-opacity-60">${this._formatFileSize(f.size)}</td>
                  <td>
                    <button class="ac-btn-icon" @click="${() => this._downloadFile(f.name)}"
                            ?disabled="${!isOnline}" title="Download">⬇️</button>
                    <button class="ac-btn-icon" @click="${() => this._deleteFile(f.name)}"
                            ?disabled="${!isOnline}" title="Delete">🗑️</button>
                  </td>
                </tr>
              `)}
            </tbody>
          </table>
        `}
      </div>

      <div class="section ac-section-muted">
        <div class="section-title">ℹ️ About File Transfers</div>
        <p class="ac-p0-note">
          Files are transferred securely via HTTPS through the kernel (not MQTT).
          Uploaded files are stored in the agent's transfer directory.
          Max file size: 200 MB. Transfers expire after 30 minutes.
        </p>
      </div>
    `
  }

  renderScreenshotTab() {
    const isOnline = this.agent && this.agent.status === 'online'

    return html`
      <div class="section">
        <div class="section-title">📸 Remote Screenshot</div>
        <div class="ac-screenshot-center">
          <div class="ac-text-4xl">📸</div>
          <p class="ac-screenshot-status ac-opacity-70">
            Request a screenshot from the remote agent.<br>
            The agent will be notified before capture.
          </p>
          <button class="execute-btn ac-execute-btn-lg"
            ?disabled="${!isOnline || this.screenshotStatus === 'capturing'}"
            @click="${() => this.takeScreenshot()}">
            ${this.screenshotStatus === 'capturing' ? 'Capturing...' : 'Take Screenshot'}
          </button>
          ${this.screenshotStatus?.startsWith('polling') ? html`
            <div class="ac-screenshot-status">
              <span class="ac-screenshot-capture">⏳ Capturing screenshot...</span>
            </div>
          ` : ''}
          ${this.screenshotStatus?.startsWith('done') ? html`
            <div class="ac-screenshot-status">
              <span class="ac-screenshot-done">✅ ${this.screenshotStatus.split(':')[1]}</span>
            </div>
          ` : ''}
          ${this.screenshotStatus?.startsWith('error') ? html`
            <div class="ac-screenshot-status">
              <span class="ac-screenshot-error">❌ ${this.screenshotStatus.split(':').slice(1).join(':')}</span>
            </div>
          ` : ''}
          ${this.screenshotStatus?.startsWith('sent') ? html`
            <div class="ac-screenshot-status">
              <span class="ac-screenshot-done">Screenshot command sent</span>
            </div>
          ` : ''}
        </div>
      </div>
      ${this.screenshotImage ? html`
        <div class="section">
          <div class="section-title">🖼️ Captured Screenshot</div>
          <div class="ac-screenshot-frame">
            <img src="${this.screenshotImage}" alt="Screenshot"
                 class="ac-screenshot-img"
                 @click="${() => window.open(this.screenshotImage, '_blank')}" />
          </div>
          <small class="ac-screenshot-caption">
            ${this.screenshotStatus?.split(':')[1] || ''} — Click image to open full size
          </small>
        </div>
      ` : ''}
      <div class="section ac-section-muted">
        <div class="section-title">ℹ️ Privacy Notice</div>
        <p class="ac-p0-note">
          Screenshots are captured on the remote agent machine. The agent displays a notification
          before capture to inform the user. Screenshots are stored locally on the agent.
        </p>
      </div>
    `
  }

  render() {
    console.debug('Agent control render - isOpen:', this.isOpen, 'agent:', this.agent)

    // Si pas ouvert, ne rien afficher
    if (!this.isOpen) {
      return html``
    }

    // Si pas d'agent, afficher erreur au lieu d'une modal vide
    if (!this.agent) {
      return html`
        <div class="modal">
          <div class="modal-header">
            <div class="modal-title">
              <span class="ac-icon-lg">⚠️</span>
              <h2 class="ac-title-inline">Agent non trouvé</h2>
            </div>
            <button class="icon-btn close-btn" @click="${this.close}" title="Fermer" aria-label="Fermer">
              ✕
            </button>
          </div>
          <div class="modal-body ac-error-body">
            <div class="ac-error-icon">🤖❌</div>
            <p class="ac-error-text">
              Impossible de charger les données de l'agent.
              <br>
              <small class="ac-error-hint">L'agent est peut-être hors ligne ou l'ID est invalide.</small>
            </p>
            <button
              @click="${this.close}"
              class="ac-error-close-btn">
              Fermer
            </button>
          </div>
        </div>
      `
    }

    return html`
      <div class="modal" @click="${(e) => e.target === e.currentTarget && this.close()}">
        <div class="modal-header">
          <div class="modal-title">
            <span class="os-icon">${this.agentsService?.getOSIcon(this.agent.os) || '💻'}</span>
            <div class="agent-info">
              <div class="agent-hostname">${this.agent.hostname}</div>
              <div class="agent-meta">${this.agent.os} • ${this.agent.primary_ip}</div>
            </div>
            <div class="status-badge ${this.agent.status}">
              <span class="status-indicator ${this.agent.status}"></span>
              ${this.agent.status}
            </div>
          </div>
          <button class="close-btn" @click="${this.close}" aria-label="Fermer">×</button>
        </div>

        <div class="modal-tabs">
          <button 
            class="tab-btn ${this.currentTab === 'system' ? 'active' : ''}"
            @click="${() => this.switchTab('system')}">
            🖥️ System
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'processes' ? 'active' : ''}"
            @click="${() => this.switchTab('processes')}">
            📋 Processes
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'metrics' ? 'active' : ''}"
            @click="${() => this.switchTab('metrics')}">
            📊 Metrics
          </button>
          <button 
            class="tab-btn ${this.currentTab === 'commands' ? 'active' : ''}"
            @click="${() => this.switchTab('commands')}">
            💻 Commands
          </button>
          <button
            class="tab-btn ${this.currentTab === 'services' ? 'active' : ''}"
            @click="${() => this.switchTab('services')}">
            🔧 Services
          </button>
          <button
            class="tab-btn ${this.currentTab === 'logs' ? 'active' : ''}"
            @click="${() => this.switchTab('logs')}">
            📜 Logs
          </button>
          <button
            class="tab-btn ${this.currentTab === 'watchdog' ? 'active' : ''}"
            @click="${() => this.switchTab('watchdog')}">
            🛡️ Watchdog
          </button>
          <button
            class="tab-btn ${this.currentTab === 'scheduler' ? 'active' : ''}"
            @click="${() => this.switchTab('scheduler')}">
            📅 Scheduler
          </button>
          <button
            class="tab-btn ${this.currentTab === 'plugins' ? 'active' : ''}"
            @click="${() => this.switchTab('plugins')}">
            🔌 Plugins
          </button>
          <button
            class="tab-btn ${this.currentTab === 'screenshot' ? 'active' : ''}"
            @click="${() => this.switchTab('screenshot')}">
            📸 Screenshot
          </button>
          <button
            class="tab-btn ${this.currentTab === 'files' ? 'active' : ''}"
            @click="${() => this.switchTab('files')}">
            📁 Files
          </button>
        </div>

        <div class="modal-content">
          <div class="tab-panel ${this.currentTab === 'system' ? 'active' : ''}">
            ${this.renderSystemTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'processes' ? 'active' : ''}">
            ${this.renderProcessesTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'metrics' ? 'active' : ''}">
            ${this.renderMetricsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'commands' ? 'active' : ''}">
            ${this.renderCommandsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'services' ? 'active' : ''}">
            ${this.renderServicesTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'logs' ? 'active' : ''}">
            ${this.renderLogsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'watchdog' ? 'active' : ''}">
            ${this.renderWatchdogTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'scheduler' ? 'active' : ''}">
            ${this.renderSchedulerTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'plugins' ? 'active' : ''}">
            ${this.renderPluginsTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'screenshot' ? 'active' : ''}">
            ${this.renderScreenshotTab()}
          </div>
          <div class="tab-panel ${this.currentTab === 'files' ? 'active' : ''}">
            ${this.renderFilesTab()}
          </div>
        </div>
      </div>
    `
  }
}

customElements.define('agent-control-widget', AgentControlWidget)

export { AgentControlWidget }