/**
 * Shared Animations — Lit CSS Module
 *
 * Centralise les @keyframes les plus dupliqués à travers les composants.
 * Usage : import { sharedAnimations } from '../styles/shared-animations.js'
 *         static styles = [sharedAnimations, css`...local...`]
 */
import { css } from 'lit'

export const sharedAnimations = css`
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes fadeOut {
    from { opacity: 1; }
    to { opacity: 0; }
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(40px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes scaleIn {
    from {
      opacity: 0;
      transform: scale(0.9);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes shimmer {
    0% { background-position: -200% 0; }
    100% { background-position: 200% 0; }
  }

  @keyframes float {
    0%, 100% {
      transform: translate(0, 0) scale(1);
      opacity: 0.15;
    }
    33% {
      transform: translate(30px, -30px) scale(1.1);
      opacity: 0.2;
    }
    66% {
      transform: translate(-20px, 20px) scale(0.9);
      opacity: 0.12;
    }
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-30px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes titlePulse {
    0%, 100% {
      filter: drop-shadow(0 0 20px var(--ctx-border, rgba(0, 212, 170, 0.15)));
    }
    50% {
      filter: drop-shadow(0 0 30px var(--ctx-border-medium, rgba(0, 212, 170, 0.25)));
    }
  }

  @keyframes inputGlow {
    0% {
      box-shadow: 0 0 0 0 var(--ctx-border-strong, rgba(0, 212, 170, 0.3));
    }
    50% {
      box-shadow: 0 0 0 8px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1));
    }
    100% {
      box-shadow: 0 0 0 4px var(--ctx-border, rgba(0, 212, 170, 0.15));
    }
  }

  @keyframes modalHeaderSlideIn {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes labelFadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes gradient-shift {
    0%, 100% {
      background-position: 0% 50%;
    }
    50% {
      background-position: 100% 50%;
    }
  }

  /* === Bioluminescent Animations === */

  @keyframes borderGlow {
    0%, 100% {
      box-shadow: 0 0 12px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1)),
                  inset 0 1px 0 var(--ctx-border-subtle, rgba(0, 212, 170, 0.1));
    }
    50% {
      box-shadow: 0 0 20px var(--ctx-border, rgba(0, 212, 170, 0.15)),
                  inset 0 1px 0 var(--ctx-border, rgba(0, 212, 170, 0.15));
    }
  }

  @keyframes textGlow {
    0%, 100% { text-shadow: 0 0 8px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1)); }
    50% { text-shadow: 0 0 16px var(--ctx-border, rgba(0, 212, 170, 0.15)); }
  }

  @keyframes sheenSweep {
    from { transform: translateX(-100%) skewX(-15deg); }
    to { transform: translateX(200%) skewX(-15deg); }
  }

  @keyframes widgetEntrance {
    from {
      opacity: 0;
      transform: translateY(24px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes dataFlash {
    0% { background: transparent; }
    30% { background: var(--ctx-bg-subtle, rgba(0, 212, 170, 0.05)); }
    100% { background: transparent; }
  }

  @keyframes bgBreathing {
    0%, 100% { opacity: 0.03; }
    50% { opacity: 0.07; }
  }

  @keyframes metricPulse {
    0%, 100% {
      filter: drop-shadow(0 0 4px var(--ctx-border-subtle, rgba(0, 212, 170, 0.1)));
    }
    50% {
      filter: drop-shadow(0 0 10px var(--ctx-border, rgba(0, 212, 170, 0.15)));
    }
  }

  @keyframes particleDrift {
    0% { transform: translate(0, 0) scale(1); opacity: 0.04; }
    25% { transform: translate(60px, -40px) scale(1.3); opacity: 0.07; }
    50% { transform: translate(-30px, -80px) scale(0.8); opacity: 0.05; }
    75% { transform: translate(-60px, 20px) scale(1.1); opacity: 0.06; }
    100% { transform: translate(0, 0) scale(1); opacity: 0.04; }
  }
`
