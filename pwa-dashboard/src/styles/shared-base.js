/**
 * Shared Base — Lit CSS Module
 *
 * Les règles globales de design-system.css (box-sizing, focus, mouvement
 * réduit) sont déclarées au niveau du document : elles NE TRAVERSENT PAS le
 * shadow root des composants Lit. Sans cette feuille, la carte de connexion
 * débordait de 9 px au téléphone (width: 100% + padding en content-box) et
 * les animations infinies ignoraient « réduire les animations ».
 *
 * À placer EN TÊTE des styles de chaque composant, pour que ses règles
 * locales puissent encore la préciser :
 *   import { sharedBase } from '../styles/shared-base.js'
 *   static styles = [sharedBase, sharedAnimations, css`...local...`]
 */
import { css } from 'lit'

export const sharedBase = css`
  :host,
  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  /* Le focus clavier reste visible même si un composant a écrit
     « outline: none » : c'est un plancher d'accessibilité, pas un style. */
  button:focus-visible,
  a:focus-visible,
  [role="button"]:focus-visible,
  [role="tab"]:focus-visible,
  [role="switch"]:focus-visible,
  [tabindex]:focus-visible {
    outline: 2px solid var(--color-focus, #4f9eff) !important;
    outline-offset: 2px;
  }

  /* Les champs, eux, peuvent signaler le focus autrement (bordure) : le
     contour n'est qu'une valeur par défaut. */
  input:focus-visible,
  select:focus-visible,
  textarea:focus-visible {
    outline: 2px solid var(--color-focus, #4f9eff);
    outline-offset: 1px;
  }

  @media (prefers-reduced-motion: reduce) {
    :host,
    *,
    *::before,
    *::after {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
      scroll-behavior: auto !important;
    }
  }
`
