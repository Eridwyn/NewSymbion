/**
 * Page Gestion Complète des Notes Symbion
 *
 * Interface dédiée pour gestion avancée des notes
 * Utilise utils/notes-scoring.js et utils/notes-filters.js pour la logique
 */

import { LitElement, html, css } from 'lit'
import { unsafeHTML } from 'lit/directives/unsafe-html.js'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import { calculatePriorityScore, sortNotesByPriority, isHighPriority } from '../utils/notes-scoring.js'
import { applyAllFilters, extractAllTags } from '../utils/notes-filters.js'
import notesStreamService from '../services/notes-stream-service.js'
import '../components/organic-loader.js'
import { sharedAnimations, pageTransitionStyles, scrollRevealStyles } from '../styles/shared-animations.js'
import { overlayStyles, closeButtonStyles, scrollbarStyles } from '../styles/shared-patterns.js'
import { setupScrollReveal } from '../utils/scroll-reveal.js'

class NotesPage extends LitElement {
  static styles = [sharedAnimations, pageTransitionStyles, scrollRevealStyles, overlayStyles, closeButtonStyles, scrollbarStyles, css`
    .notes-container {
      max-width: 1200px;
      margin: var(--space-6) auto;
      padding: var(--space-6);
      overflow-x: hidden;
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-lg);
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.4);
      animation: slideUp var(--duration-slow) var(--ease-out);
    }

    .notes-header {
      position: relative;
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: var(--space-6);
      padding: var(--space-4) var(--space-5);
      background: var(--surface-glass-strong, rgba(0, 0, 0, 0.3));
      border-bottom: 1px solid var(--border-default);
      border-radius: var(--radius-lg) var(--radius-lg) 0 0;
      margin: calc(-1 * var(--space-6)) calc(-1 * var(--space-6)) var(--space-6);
    }

    .notes-title {
      font-size: var(--text-2xl);
      font-weight: var(--font-bold);
      color: var(--color-dark-text-primary, #f8f9fa);
      display: flex;
      align-items: center;
      gap: var(--space-3);
    }

    .toolbar {
      display: flex;
      gap: var(--space-3);
      flex-wrap: wrap;
      align-items: center;
      margin-bottom: var(--space-6);
      padding: var(--space-4);
      background: var(--surface-glass-subtle, rgba(255,255,255,0.03));
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      transition: all var(--duration-base) var(--ease-out);
    }

    .search-box {
      flex: 1;
      min-width: 250px;
    }

    .search-input {
      width: 100%;
      max-width: 100%;
      box-sizing: border-box;
      background: var(--surface-glass);
      border: 1px solid var(--border-hover);
      border-radius: var(--radius-base);
      padding: 0.6rem 1rem;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-size: 0.9em;
      transition: all var(--duration-base) var(--ease-out);
    }

    .search-input:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 3px var(--ctx-border-subtle),
                  0 0 20px var(--ctx-border-subtle);
      animation: inputGlow 0.6s ease-out;
    }

    .search-input:hover:not(:focus) {
      border-color: var(--border-hover);
      transform: translateY(-1px);
    }

    .search-input::placeholder {
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .filters-group {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
    }

    .filter-btn {
      background: transparent;
      border: 1px solid transparent;
      color: var(--color-dark-text-secondary, #adb5bd);
      padding: var(--space-2) var(--space-3);
      border-radius: var(--radius-md);
      font-size: var(--text-sm);
      font-weight: var(--font-medium);
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      white-space: nowrap;
    }

    .filter-btn:hover {
      background: var(--surface-glass, rgba(255,255,255,0.06));
      color: var(--color-dark-text-primary, #f8f9fa);
    }

    .filter-btn.active {
      background: var(--ctx-bg, rgba(0,212,170,0.05));
      border-color: var(--ctx-border, rgba(0,212,170,0.15));
      color: var(--context-primary, #00d4aa);
    }

    .filter-btn:active {
      transform: scale(0.98);
    }

    .context-filter-toggle {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      background: var(--surface-glass);
      border: 1px solid var(--border-hover);
      border-radius: var(--radius-base);
      padding: 0.5rem 1rem;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      font-size: 0.85em;
    }

    .context-filter-toggle:hover {
      background: var(--surface-glass-hover);
    }

    .context-filter-toggle.active {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.3) 0%, var(--ctx-bg-emphasis) 100%);
      border-color: var(--context-primary, #00d4aa);
      color: var(--context-primary, #00d4aa);
      box-shadow: 0 2px 10px var(--ctx-border-strong);
    }

    .toggle-switch {
      position: relative;
      width: 40px;
      height: 20px;
      background: var(--surface-glass-strong);
      border-radius: var(--radius-full, 9999px);
      transition: background var(--duration-base) var(--ease-in-out);
    }

    .context-filter-toggle.active .toggle-switch {
      background: var(--context-primary, #00d4aa);
    }

    .toggle-switch::after {
      content: '';
      position: absolute;
      top: 2px;
      left: 2px;
      width: 16px;
      height: 16px;
      background: white;
      border-radius: 50%;
      transition: transform var(--duration-base) var(--ease-bounce);
    }

    .context-filter-toggle.active .toggle-switch::after {
      transform: translateX(20px);
    }

    .add-note-btn {
      background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, var(--ctx-bg-strong) 100%);
      border: 1px solid var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
      padding: 0.6rem 1.2rem;
      border-radius: var(--radius-base);
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      box-shadow: 0 2px 8px var(--ctx-border-medium);
      white-space: nowrap;
      position: relative;
      overflow: hidden;
      animation: buttonPulse 3s ease-in-out infinite; /* Pulse subtil pour attirer attention */
    }

    @keyframes buttonPulse {
      0%, 100% {
        box-shadow: 0 2px 8px var(--ctx-border-medium);
      }
      50% {
        box-shadow: 0 2px 12px var(--ctx-border-strong),
                    0 0 20px var(--ctx-border);
      }
    }

    .add-note-btn::before {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: 0;
      height: 0;
      border-radius: 50%;
      background: var(--ctx-bg-intense);
      transform: translate(-50%, -50%);
      transition: width 0.6s ease, height 0.6s ease;
    }

    .add-note-btn:hover::before {
      width: 300px;
      height: 300px;
    }

    .add-note-btn:hover {
      background: linear-gradient(135deg, var(--ctx-border-strong) 0%, var(--ctx-bg-intense) 100%);
      border-color: color-mix(in srgb, var(--context-primary) 60%, transparent);
      transform: translateY(-2px) scale(1.02);
      box-shadow: 0 4px 16px var(--ctx-border-strong);
      animation: none; /* Stop pulse sur hover */
    }

    .add-note-btn:active {
      transform: translateY(0) scale(0.98);
    }

    .tags-bar {
      display: flex;
      gap: 0.4rem;
      flex-wrap: wrap;
      margin-bottom: 1.5rem;
    }

    .tag-filter-btn {
      background: var(--surface-glass-subtle);
      border: 1px solid var(--border-medium);
      color: var(--color-dark-text-tertiary, #94a3b8);
      padding: 0.3rem 0.7rem;
      border-radius: var(--radius-md);
      font-size: 0.75em;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
    }

    .tag-filter-btn:hover {
      border-color: var(--ctx-border-strong);
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .tag-filter-btn.active {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.2) 0%, var(--ctx-border) 100%);
      border-color: var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
      box-shadow: 0 2px 6px var(--ctx-bg-emphasis);
    }

    .notes-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
      gap: 1.2rem;
      margin-top: 1.5rem;
    }

    .note-card {
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
      border: 1px solid var(--border-medium);
      border-radius: var(--radius-md);
      padding: 1.2rem;
      transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      position: relative;
      overflow: hidden;
      cursor: pointer;
      animation: cardBreathing 8s ease-in-out infinite; /* Respiration subtile */
    }

    @keyframes cardBreathing {
      0%, 100% {
        border-color: var(--border-medium);
      }
      50% {
        border-color: var(--border-hover);
      }
    }

    .note-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 3px;
      height: 100%;
      background: linear-gradient(180deg, var(--context-primary, #00d4aa) 0%, var(--ctx-bg-intense) 100%);
      opacity: 0;
      transition: opacity 0.3s ease;
      box-shadow: 0 0 10px var(--context-primary, #00d4aa);
    }

    .note-card:hover {
      border-color: var(--ctx-border-strong);
      transform: translateY(-4px) scale(1.01); /* Légère élévation + zoom */
      box-shadow: 0 12px 32px var(--ctx-border-medium),
                  0 0 40px var(--ctx-bg);
      animation: none; /* Stop breathing sur hover */
    }

    .note-card:hover::before {
      opacity: 1;
    }

    .note-card:active {
      transform: translateY(-2px) scale(0.99); /* Feedback tactile */
    }

    .note-card.urgent {
      border-color: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 50%, transparent);
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 15%, transparent) 0%, color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 5%, transparent) 100%);
    }

    .note-card.urgent::before {
      background: linear-gradient(180deg, var(--color-danger-text-muted, #ff6b6b) 0%, var(--color-danger-strong, #ef4444) 100%);
      opacity: 1;
      width: 4px;
      box-shadow: 0 0 15px color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 50%, transparent);
    }

    .note-card.priority {
      border-color: color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 30%, transparent);
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 8%, transparent) 0%, color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 2%, transparent) 100%);
    }

    .note-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: 0.8rem;
    }

    .note-indicators {
      display: flex;
      gap: 0.4rem;
      flex-wrap: wrap;
      align-items: center;
    }

    .urgent-indicator {
      color: var(--color-danger-text-muted, #ff6b6b);
      font-weight: bold;
      font-size: 1.1em;
      filter: drop-shadow(0 2px 6px color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 60%, transparent));
      animation: pulse-urgent 2s ease-in-out infinite;
    }

    @keyframes pulse-urgent {
      0%, 100% {
        opacity: 1;
        transform: scale(1);
      }
      50% {
        opacity: 0.8;
        transform: scale(1.1);
      }
    }

    .priority-badge {
      background: linear-gradient(135deg, color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 25%, transparent) 0%, color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 20%, transparent) 100%);
      color: var(--color-warning-text-muted, #fbbf24);
      padding: 0.2rem 0.5rem;
      border-radius: var(--radius-base);
      font-size: 0.7em;
      font-weight: 600;
      border: 1px solid color-mix(in srgb, var(--color-warning-text-muted, #fbbf24) 30%, transparent);
    }

    .context-tag {
      background: linear-gradient(135deg, rgba(0, 122, 204, 0.25) 0%, var(--ctx-border-medium) 100%);
      color: var(--context-primary, #00d4aa);
      padding: 0.2rem 0.6rem;
      border-radius: var(--radius-md);
      font-size: 0.7em;
      font-weight: 500;
      letter-spacing: 0.5px;
      border: 1px solid var(--ctx-border-strong);
      text-transform: uppercase;
    }

    .note-actions {
      display: flex;
      gap: 0.3rem;
    }

    .note-action {
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      color: var(--color-dark-text-tertiary, #94a3b8);
      padding: 0.3rem 0.5rem;
      border-radius: var(--radius-sm);
      cursor: pointer;
      font-size: 0.9em;
      transition: all var(--duration-base) var(--ease-out);
    }

    .note-action:hover {
      background: var(--surface-glass-hover);
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .note-action.delete {
      color: var(--color-danger-text-muted, #ff6b6b);
      border-color: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent);
    }

    .note-action.delete:hover {
      background: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 20%, transparent);
      border-color: color-mix(in srgb, var(--color-danger-text-muted, #ff6b6b) 40%, transparent);
    }

    .note-preview {
      color: var(--color-dark-text-secondary, #cbd5e1);
      line-height: 1.6;
      margin-bottom: 0.8rem;
      max-height: 4.8em;
      overflow: hidden;
      display: -webkit-box;
      -webkit-line-clamp: 3;
      -webkit-box-orient: vertical;
    }

    .note-meta {
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 0.75em;
      opacity: 0.6;
      margin-top: 0.5rem;
      padding-top: 0.5rem;
      border-top: 1px solid var(--border-default);
    }

    .note-tags {
      color: var(--context-primary, #00d4aa);
    }

    .note-timestamp {
      color: var(--color-dark-text-tertiary, #94a3b8);
    }

    .placeholder {
      text-align: center;
      padding: 4rem 2rem;
      opacity: 0.6;
      font-size: 1.1em;
    }

    /* Modal styles (shared for create and detail) */
    .modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: var(--app-overlay-dim, rgba(0, 0, 0, 0.9));
      backdrop-filter: blur(var(--blur-base));
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 10000;
      animation: fadeIn 0.3s ease;
    }

    .modal-content {
      background: linear-gradient(135deg, var(--app-page-bg-a) 0%, var(--app-page-bg-b) 100%);
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-lg);
      width: 90%;
      max-width: 700px;
      max-height: 85vh;
      overflow-y: auto;
      overflow-x: hidden; /* Empêche débordement horizontal */
      padding: 2rem;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6);
      animation: modalSlideIn 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .modal-header {
      position: relative;
      margin-bottom: 1.5rem;
      padding-bottom: 1rem;
      padding-right: 50px; /* Espace pour bouton close */
      border-bottom: 1px solid var(--ctx-border);
      animation: modalHeaderSlideIn 0.5s ease-out 0.1s backwards;
    }

    /* modalHeaderSlideIn — see shared-animations.js */

    .modal-title {
      font-size: 1.4em;
      font-weight: 600;
      color: var(--context-primary, #00d4aa);
      filter: drop-shadow(0 0 15px var(--ctx-border));
      animation: titlePulse 4s ease-in-out infinite;
    }

    .modal-close-btn {
      position: absolute;
      top: 0;
      right: 0;
      background: var(--surface-glass);
      border: 1px solid var(--border-medium);
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-size: var(--text-2xl, 1.5rem);
      cursor: pointer;
      padding: 6px 10px;
      border-radius: var(--radius-base);
      transition: all var(--duration-base) var(--ease-out);
      line-height: 1;
    }

    .modal-close-btn:hover {
      background: color-mix(in srgb, var(--color-danger-strong, #ef4444) 25%, transparent);
      border-color: color-mix(in srgb, var(--color-danger-strong, #ef4444) 40%, transparent);
      color: var(--color-danger-text-muted, #ff6b6b);
      transform: rotate(90deg);
    }

    .form-field {
      margin-bottom: 1.2rem;
      max-width: 100%; /* Force containment */
      overflow: hidden; /* Empêche débordement */
    }

    .form-field label {
      display: block;
      margin-bottom: 0.5rem;
      font-size: 0.9em;
      color: var(--color-dark-text-secondary, #cbd5e1);
      font-weight: 500;
      animation: labelFadeIn 0.4s ease-out; /* Apparition douce */
    }

    /* labelFadeIn — see shared-animations.js */

    .form-field input,
    .form-field textarea {
      width: 100%;
      max-width: 100%; /* CRITIQUE: Empêche débordement horizontal */
      min-width: 0; /* Permet rétrécissement si nécessaire */
      box-sizing: border-box; /* Padding inclus dans width */
      background: var(--surface-glass);
      border: 1px solid var(--border-hover);
      border-radius: var(--radius-base);
      padding: 0.7rem 1rem;
      color: var(--color-dark-text-primary, #f8f9fa);
      font-family: inherit;
      font-size: 0.9em;
      transition: all var(--duration-base) var(--ease-out);
    }

    .form-field textarea {
      resize: vertical;
      min-height: 120px;
      font-family: 'Monaco', 'Consolas', monospace;
      line-height: 1.5;
    }

    .form-field input:focus,
    .form-field textarea:focus {
      outline: none;
      border-color: var(--context-primary, #00d4aa);
      box-shadow: 0 0 0 3px var(--ctx-border-subtle),
                  0 0 20px var(--ctx-border-subtle);
      animation: inputGlow 0.6s ease-out; /* Pulse au focus */
    }

    .form-field input:hover:not(:focus),
    .form-field textarea:hover:not(:focus) {
      border-color: var(--border-strong);
      transform: translateY(-1px); /* Légère élévation */
    }

    .form-checkboxes {
      display: flex;
      gap: 1.5rem;
      margin-bottom: 1.2rem;
    }

    .checkbox-field {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      cursor: pointer;
    }

    .checkbox-field input[type="checkbox"] {
      width: 18px;
      height: 18px;
      cursor: pointer;
    }

    .form-actions {
      display: flex;
      gap: 0.8rem;
      justify-content: flex-end;
      margin-top: 2rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border-default);
    }

    .form-btn {
      padding: 0.7rem 1.5rem;
      border-radius: var(--radius-base);
      font-size: 0.9em;
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-base) var(--ease-out);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
      position: relative;
      overflow: hidden;
    }

    .form-btn::before {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      width: 0;
      height: 0;
      border-radius: 50%;
      background: var(--ctx-border-medium);
      transform: translate(-50%, -50%);
      transition: width 0.6s ease, height 0.6s ease;
    }

    .form-btn:hover::before {
      width: 300px;
      height: 300px;
    }

    .form-btn.primary {
      background: linear-gradient(135deg, var(--ctx-bg-emphasis) 0%, var(--ctx-bg-strong) 100%);
      border: 1px solid var(--ctx-border-strong);
      color: var(--context-primary, #00d4aa);
    }

    .form-btn.primary:hover {
      background: linear-gradient(135deg, var(--ctx-border-strong) 0%, var(--ctx-bg-intense) 100%);
      border-color: color-mix(in srgb, var(--context-primary) 60%, transparent);
      transform: translateY(-2px);
      box-shadow: 0 4px 12px var(--ctx-border-strong);
    }

    .form-btn.primary:active {
      transform: translateY(0) scale(0.98); /* Feedback tactile */
    }

    .form-btn.secondary {
      background: var(--surface-glass);
      border: 1px solid var(--border-hover);
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .form-btn.secondary::before {
      background: var(--surface-glass-strong);
    }

    .form-btn.secondary:hover {
      background: var(--surface-glass-hover);
      border-color: var(--border-hover);
      transform: translateY(-2px);
    }

    .form-btn.secondary:active {
      transform: translateY(0) scale(0.98);
    }

    .note-content {
      color: var(--color-dark-text-primary, #f8f9fa);
      line-height: 1.8;
    }

    .note-content h1, .note-content h2, .note-content h3 {
      color: var(--context-primary, #00d4aa);
      margin: 1em 0 0.5em 0;
      font-weight: 600;
    }

    .note-content h1 { font-size: 1.6em; }
    .note-content h2 { font-size: 1.3em; }
    .note-content h3 { font-size: 1.1em; }

    .note-content p {
      margin: 0.8em 0;
    }

    .note-content code {
      background: var(--ctx-border);
      color: var(--context-primary, #00d4aa);
      padding: 0.2em 0.4em;
      border-radius: var(--radius-sm);
      font-family: 'Monaco', 'Consolas', monospace;
      font-size: 0.9em;
    }

    .note-content pre {
      background: var(--surface-glass-strong, rgba(0, 0, 0, 0.3));
      border: 1px solid var(--ctx-border-medium);
      border-radius: var(--radius-sm);
      padding: 1em;
      overflow-x: auto;
      margin: 0.8em 0;
    }

    .note-content pre code {
      background: none;
      padding: 0;
    }

    .note-content ul, .note-content ol {
      margin: 0.5em 0;
      padding-left: 1.8em;
    }

    .note-content li {
      margin: 0.4em 0;
    }

    .note-content a {
      color: var(--context-primary, #00d4aa);
      text-decoration: none;
      border-bottom: 1px solid var(--ctx-border-strong);
      transition: all 0.2s ease;
    }

    .note-content a:hover {
      border-bottom-color: var(--context-primary, #00d4aa);
    }

    .note-content blockquote {
      border-left: 3px solid var(--context-primary, #00d4aa);
      padding-left: 1em;
      margin: 0.8em 0;
      color: var(--color-dark-text-tertiary, #94a3b8);
      font-style: italic;
    }

    @media (max-width: 768px) {
      .notes-container {
        padding: 1rem;
      }

      .notes-title {
        font-size: 1.5em;
      }

      .toolbar {
        flex-direction: column;
        align-items: stretch;
      }

      .search-box {
        min-width: 100%;
      }

      .notes-grid {
        grid-template-columns: 1fr;
        gap: 1rem;
      }

      .filters-group {
        width: 100%;
      }

      .filter-btn {
        flex: 1;
      }

      .modal-content {
        width: 95%;
        padding: 1.5rem;
      }

      .modal-header {
        padding-right: 0;
      }

      .modal-title {
        font-size: 1.2em;
        max-width: calc(100% - 50px);
      }
    }

    /* === Utility classes (ex-inline styles) === */
    .np-modal-header-content { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; }
    .np-flex-wrap { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
    .np-tags-meta { color: var(--color-dark-text-tertiary, #6b7280); font-size: 0.8em; font-weight: 500; }
    .np-note-footer { margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--border-default); display: flex; justify-content: space-between; align-items: center; }
    .np-flex-gap { display: flex; gap: 0.5rem; }
    .np-action-compact { padding: 0.4rem 0.8rem; }
  `]

  static properties = {
    notes: { type: Array },
    apiService: { type: Object },
    contextService: { type: Object },
    currentContext: { type: String },
    searchQuery: { type: String },
    currentFilter: { type: String },
    selectedTags: { type: Array },
    availableTags: { type: Array },
    contextFilterEnabled: { type: Boolean },
    showNoteForm: { type: Boolean },
    selectedNote: { type: Object },
    editingNote: { type: Object },
    loading: { type: Boolean }
  }

  constructor() {
    super()
    this.notes = []
    this.apiService = null
    this.contextService = null
    this.currentContext = 'veille'
    this.searchQuery = ''
    this.currentFilter = 'all'
    this.selectedTags = []
    this.availableTags = []
    this.contextFilterEnabled = false
    this.showNoteForm = false
    this.selectedNote = null
    this.editingNote = null
    this.loading = false
  }

  connectedCallback() {
    super.connectedCallback()

    // Get services
    this.apiService = document.querySelector('api-service')
    this.contextService = document.querySelector('context-service')

    if (this.contextService) {
      this.currentContext = this.contextService.getCurrentMode()
    }

    this.loadNotes()

    // Escape key handler
    this.handleEscape = (e) => {
      if (e.key === 'Escape') {
        if (this.editingNote) {
          this.editingNote = null
        } else if (this.selectedNote) {
          this.selectedNote = null
        } else if (this.showNoteForm) {
          this.showNoteForm = false
        } else {
          this.close()
        }
      }
    }
    document.addEventListener('keydown', this.handleEscape)
  }

  firstUpdated() {
    this._cleanupReveal = setupScrollReveal(this.shadowRoot)
  }

  disconnectedCallback() {
    super.disconnectedCallback()
    document.removeEventListener('keydown', this.handleEscape)
    this._cleanupReveal?.()
  }

  async loadNotes() {
    this.loading = true
    this.notes = [] // Reset for progressive loading

    console.log('[notes-page] 📡 Loading notes via WebSocket...')

    const onNoteReceived = (e) => {
      console.log('[notes-page] 📝 Note received')
      this.notes = [...this.notes, e.detail.note]
      this.availableTags = extractAllTags(this.notes)
      this.requestUpdate()
    }

    const onNotesComplete = (e) => {
      this.loading = false
      console.log(`[notes-page] ✅ Loaded ${this.notes.length} notes (total: ${e.detail.total})`)
      this.availableTags = extractAllTags(this.notes)

      // Cleanup listeners
      notesStreamService.removeEventListener('note-received', onNoteReceived)
      notesStreamService.removeEventListener('notes-complete', onNotesComplete)
      notesStreamService.removeEventListener('notes-error', onNotesError)

      this.requestUpdate()
    }

    const onNotesError = (e) => {
      this.loading = false
      console.error('[notes-page] ❌ WebSocket error:', e.detail.error)

      // Cleanup listeners
      notesStreamService.removeEventListener('note-received', onNoteReceived)
      notesStreamService.removeEventListener('notes-complete', onNotesComplete)
      notesStreamService.removeEventListener('notes-error', onNotesError)

      this.requestUpdate()
    }

    // Register event listeners
    notesStreamService.addEventListener('note-received', onNoteReceived)
    notesStreamService.addEventListener('notes-complete', onNotesComplete)
    notesStreamService.addEventListener('notes-error', onNotesError)

    // Start WebSocket streaming
    try {
      await notesStreamService.loadNotes({}) // Empty filters = all notes
    } catch (error) {
      console.error('[notes-page] ❌ Failed to start WebSocket:', error)
      this.loading = false

      // Cleanup listeners
      notesStreamService.removeEventListener('note-received', onNoteReceived)
      notesStreamService.removeEventListener('notes-complete', onNotesComplete)
      notesStreamService.removeEventListener('notes-error', onNotesError)
    }
  }

  getFilteredAndSortedNotes() {
    // Apply filters using utility
    const filtered = applyAllFilters(this.notes, {
      context: this.currentContext,
      contextFilterEnabled: this.contextFilterEnabled,
      search: this.searchQuery,
      tags: this.selectedTags,
      urgentOnly: this.currentFilter === 'urgent'
    })

    // Sort by priority using utility
    const sorted = sortNotesByPriority(filtered, this.currentContext)

    // Limit for 'recent' filter
    if (this.currentFilter === 'recent') {
      return sorted.slice(0, 20)
    }

    return sorted
  }

  toggleTagFilter(tag) {
    if (this.selectedTags.includes(tag)) {
      this.selectedTags = this.selectedTags.filter(t => t !== tag)
    } else {
      this.selectedTags = [...this.selectedTags, tag]
    }
  }

  async handleCreateNote(event) {
    event.preventDefault()

    const formData = new FormData(event.target)
    const note = {
      content: formData.get('content'),
      context: formData.get('context') || null,
      urgent: formData.has('urgent'),
      tags: formData.get('tags') ? formData.get('tags').split(',').map(t => t.trim()).filter(t => t) : []
    }

    try {
      await this.apiService.createNote(note)
      this.showNoteForm = false
      await this.loadNotes()
      console.log('✅ Note created successfully')
    } catch (error) {
      console.error('❌ Failed to create note:', error)
    }
  }

  async handleDeleteNote(noteId, event) {
    event.stopPropagation()
    if (!confirm('Supprimer cette note ?')) return

    try {
      await this.apiService.deleteNote(noteId)
      await this.loadNotes()
      this.selectedNote = null
      console.log('✅ Note deleted successfully')
    } catch (error) {
      console.error('❌ Failed to delete note:', error)
    }
  }

  openEditNote(note, event) {
    if (event) event.stopPropagation()
    this.editingNote = note
    this.selectedNote = null
  }

  closeEditNote() {
    this.editingNote = null
  }

  async handleUpdateNote(event) {
    event.preventDefault()

    const formData = new FormData(event.target)
    const updatedData = {
      content: formData.get('content'),
      context: formData.get('context') || null,
      urgent: formData.has('urgent'),
      tags: formData.get('tags') ? formData.get('tags').split(',').map(t => t.trim()).filter(t => t) : []
    }

    try {
      await this.apiService.updateNote(this.editingNote.id, updatedData)
      this.editingNote = null
      await this.loadNotes()
      console.log('✅ Note updated successfully')
    } catch (error) {
      console.error('❌ Failed to update note:', error)
    }
  }

  openNoteDetail(note) {
    this.selectedNote = note
  }

  closeNoteDetail() {
    this.selectedNote = null
  }

  formatTimestamp(timestamp) {
    if (!timestamp || !Array.isArray(timestamp)) return ''

    const [year, day, hour, minute] = timestamp
    const date = new Date(year, 0, day, hour || 0, minute || 0)

    const now = new Date()
    const diff = now - date

    if (diff < 60000) return 'À l\'instant'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`

    return date.toLocaleDateString('fr-FR', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    })
  }

  getPreviewText(content) {
    if (!content) return ''
    const plainText = content.replace(/[#*`[\]()]/g, '').trim()
    return plainText.length > 150 ? plainText.substring(0, 150) + '...' : plainText
  }

  renderMarkdown(content) {
    if (!content) return ''
    try {
      const rawHtml = marked.parse(content)
      return DOMPurify.sanitize(rawHtml)
    } catch (error) {
      console.error('Failed to parse markdown:', error)
      return DOMPurify.sanitize(content)
    }
  }

  getContextIcon(context) {
    const icons = {
      'pro': '👔',
      'cravate': '👔',
      'maison': '🏡',
      'intime': '🏡',
      'focus': '🎯',
      'veille': '🌱',
      'neutre': '🌱'
    }
    return icons[context] || '📍'
  }

  close() {
    this.dispatchEvent(new CustomEvent('close', {
      bubbles: true,
      composed: true
    }))
  }

  openCreateModal() {
    console.log('[notes-page] Opening create modal')
    this.showNoteForm = true
  }

  render() {
    const filteredNotes = this.getFilteredAndSortedNotes()

    return html`
      <div class="notes-container">
        <div class="notes-header">
          <h1 class="notes-title">📝 Gestion des Notes</h1>
          <button class="close-button" @click="${this.close}" aria-label="Fermer">✕</button>
        </div>

        <div class="toolbar">
          <div class="search-box">
            <input
              type="text"
              class="search-input"
              placeholder="🔍 Rechercher dans les notes..."
              .value="${this.searchQuery}"
              @input="${(e) => this.searchQuery = e.target.value}">
          </div>

          <div class="filters-group">
            <button
              class="filter-btn ${this.currentFilter === 'all' ? 'active' : ''}"
              @click="${() => this.currentFilter = 'all'}">
              Toutes
            </button>
            <button
              class="filter-btn ${this.currentFilter === 'urgent' ? 'active' : ''}"
              @click="${() => this.currentFilter = 'urgent'}">
              🚨 Urgentes
            </button>
            <button
              class="filter-btn ${this.currentFilter === 'recent' ? 'active' : ''}"
              @click="${() => this.currentFilter = 'recent'}">
              📅 Récentes
            </button>
          </div>

          <div
            class="context-filter-toggle ${this.contextFilterEnabled ? 'active' : ''}"
            @click="${() => this.contextFilterEnabled = !this.contextFilterEnabled}">
            <span>${this.getContextIcon(this.currentContext)} Contexte actuel uniquement</span>
            <div class="toggle-switch"></div>
          </div>

          <button class="add-note-btn" @click="${() => this.showNoteForm = true}">
            ➕ Nouvelle Note
          </button>
        </div>

        ${this.availableTags.length > 0 ? html`
          <div class="tags-bar">
            ${this.availableTags.map(tag => html`
              <button
                class="tag-filter-btn ${this.selectedTags.includes(tag) ? 'active' : ''}"
                @click="${() => this.toggleTagFilter(tag)}">
                #${tag}
              </button>
            `)}
          </div>
        ` : ''}

        ${this.loading ? html`
          <organic-loader text="🧬 Organisme en synapse..."></organic-loader>
        ` : filteredNotes.length === 0 ? html`
          <div class="placeholder">
            ${this.searchQuery || this.contextFilterEnabled || this.selectedTags.length > 0
              ? '🔍 Aucune note ne correspond aux filtres'
              : '📝 Aucune note pour le moment'}
          </div>
        ` : html`
          <div class="notes-grid">
            ${filteredNotes.map((note, idx) => {
              const isPriority = isHighPriority(note, this.currentContext)

              return html`
                <div class="scroll-reveal" style="transition-delay: ${idx * 0.05}s">
                  <div
                    class="note-card ${note.data.urgent ? 'urgent' : ''} ${isPriority ? 'priority' : ''}"
                    @click="${() => this.openNoteDetail(note)}">
                  <div class="note-header">
                    <div class="note-indicators">
                      ${note.data.urgent ? html`<span class="urgent-indicator">🚨</span>` : ''}
                      ${isPriority ? html`<span class="priority-badge">⭐ Prioritaire</span>` : ''}
                      ${note.data.context ? html`
                        <span class="context-tag">
                          ${this.getContextIcon(note.data.context)} ${note.data.context}
                        </span>
                      ` : ''}
                    </div>
                    <div class="note-actions">
                      <button
                        class="note-action edit"
                        @click="${(e) => this.openEditNote(note, e)}"
                        title="Modifier">
                        ✏️
                      </button>
                      <button
                        class="note-action delete"
                        @click="${(e) => this.handleDeleteNote(note.id, e)}"
                        title="Supprimer">
                        🗑️
                      </button>
                    </div>
                  </div>

                  <div class="note-preview">
                    ${this.getPreviewText(note.data.content)}
                  </div>

                  <div class="note-meta">
                    <span class="note-tags">
                      ${note.data.tags && note.data.tags.length > 0 ? `#${note.data.tags.join(' #')}` : ''}
                    </span>
                    <span class="note-timestamp">
                      ${this.formatTimestamp(note.timestamp)}
                    </span>
                  </div>
                  </div>
                </div>
              `
            })}
          </div>
        `}
      </div>

      <!-- New Note Modal -->
      ${this.showNoteForm ? html`
        <div class="modal-overlay" @click="${() => this.showNoteForm = false}">
          <div class="modal-content" @click="${(e) => e.stopPropagation()}">
            <div class="modal-header">
              <h2 class="modal-title">✍️ Nouvelle Note</h2>
              <button class="modal-close-btn" @click="${() => this.showNoteForm = false}" aria-label="Fermer">×</button>
            </div>

            <form @submit="${this.handleCreateNote}">
              <div class="form-field">
                <label for="content">Contenu *</label>
                <textarea
                  name="content"
                  id="content"
                  required
                  placeholder="Votre note (markdown supporté)..."></textarea>
              </div>

              <div class="form-field">
                <label for="context">Contexte</label>
                <input
                  name="context"
                  id="context"
                  placeholder="pro, maison, veille, focus..."
                  .value="${this.currentContext || 'veille'}">
              </div>

              <div class="form-field">
                <label for="tags">Tags</label>
                <input name="tags" id="tags" placeholder="tag1, tag2, tag3">
              </div>

              <div class="form-checkboxes">
                <label class="checkbox-field">
                  <input type="checkbox" name="urgent" id="urgent">
                  <span>🚨 Marquer comme urgent</span>
                </label>
              </div>

              <div class="form-actions">
                <button type="button" class="form-btn secondary" @click="${() => this.showNoteForm = false}">
                  Annuler
                </button>
                <button type="submit" class="form-btn primary">
                  ✅ Créer la note
                </button>
              </div>
            </form>
          </div>
        </div>
      ` : ''}

      <!-- Note Detail Modal -->
      ${this.selectedNote ? html`
        <div class="modal-overlay" @click="${this.closeNoteDetail}">
          <div class="modal-content" @click="${(e) => e.stopPropagation()}">
            <div class="modal-header">
              <div class="np-modal-header-content">
                <div class="np-flex-wrap">
                  ${this.selectedNote.data.urgent ? html`<span class="urgent-indicator">🚨 URGENT</span>` : ''}
                  ${this.selectedNote.data.context ? html`
                    <span class="context-tag">
                      ${this.getContextIcon(this.selectedNote.data.context)} ${this.selectedNote.data.context}
                    </span>
                  ` : ''}
                </div>
                ${this.selectedNote.data.tags && this.selectedNote.data.tags.length > 0 ? html`
                  <div class="np-tags-meta">
                    ${this.selectedNote.data.tags.map(tag => `#${tag}`).join(' ')}
                  </div>
                ` : ''}
              </div>
              <button class="modal-close-btn" @click="${this.closeNoteDetail}" aria-label="Fermer">×</button>
            </div>

            <div class="note-content">
              ${unsafeHTML(this.renderMarkdown(this.selectedNote.data.content))}
            </div>

            <div class="note-meta np-note-footer">
              <span>📅 ${this.formatTimestamp(this.selectedNote.timestamp)}</span>
              <div class="np-flex-gap">
                <button
                  class="note-action edit"
                  @click="${(e) => this.openEditNote(this.selectedNote, e)}"
                  class="np-action-compact">
                  ✏️ Modifier
                </button>
                <button
                  class="note-action delete"
                  @click="${(e) => this.handleDeleteNote(this.selectedNote.id, e)}"
                  class="np-action-compact">
                  🗑️ Supprimer
                </button>
              </div>
            </div>
          </div>
        </div>
      ` : ''}

      <!-- Edit Note Modal -->
      ${this.editingNote ? html`
        <div class="modal-overlay" @click="${this.closeEditNote}">
          <div class="modal-content" @click="${(e) => e.stopPropagation()}">
            <div class="modal-header">
              <h2 class="modal-title">✏️ Modifier la Note</h2>
              <button class="modal-close-btn" @click="${this.closeEditNote}" aria-label="Fermer">×</button>
            </div>

            <form @submit="${this.handleUpdateNote}">
              <div class="form-field">
                <label for="edit-content">Contenu *</label>
                <textarea
                  name="content"
                  id="edit-content"
                  required
                  placeholder="Votre note (markdown supporté)..."
                  .value="${this.editingNote.data.content}"></textarea>
              </div>

              <div class="form-field">
                <label for="edit-context">Contexte</label>
                <input
                  name="context"
                  id="edit-context"
                  placeholder="pro, maison, veille, focus..."
                  .value="${this.editingNote.data.context || ''}">
              </div>

              <div class="form-field">
                <label for="edit-tags">Tags</label>
                <input
                  name="tags"
                  id="edit-tags"
                  placeholder="tag1, tag2, tag3..."
                  .value="${this.editingNote.data.tags ? this.editingNote.data.tags.join(', ') : ''}">
              </div>

              <div class="form-checkboxes">
                <label class="checkbox-field">
                  <input
                    type="checkbox"
                    name="urgent"
                    ?checked="${this.editingNote.data.urgent}">
                  <span>🚨 Marquer comme urgent</span>
                </label>
              </div>

              <div class="form-actions">
                <button type="button" class="form-btn secondary" @click="${this.closeEditNote}">
                  Annuler
                </button>
                <button type="submit" class="form-btn primary">
                  ✅ Enregistrer
                </button>
              </div>
            </form>
          </div>
        </div>
      ` : ''}
    `
  }
}

customElements.define('notes-page', NotesPage)
