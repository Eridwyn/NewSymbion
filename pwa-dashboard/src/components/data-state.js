/**
 * <data-state> — l'état d'un bloc de données qui n'est pas (encore) là.
 *
 * Un seul composant pour les quatre cas, au lieu de « Failed to fetch » brut,
 * de chargements sans fin et de faux états vides (« Aucune validation en
 * attente » alors que le kernel ne répond pas) :
 *
 *   <data-state etat="chargement"></data-state>
 *   <data-state etat="vide" titre="Aucune validation en attente"></data-state>
 *   <data-state etat="erreur" message="Le kernel a répondu 500."
 *               @reessayer="${this.load}"></data-state>
 *   <data-state etat="injoignable" @reessayer="${this.load}"></data-state>
 *
 * Depuis une exception : <data-state ...=${etatDepuisErreur(e)}> — voir
 * `etatDepuisErreur`, qui distingue un kernel injoignable d'une erreur.
 */
import { LitElement, html, css } from 'lit'
import { sharedBase } from '../styles/shared-base.js'

const TEXTES = {
  chargement: { icone: '', titre: 'Chargement…' },
  vide: { icone: '🌱', titre: 'Rien pour l’instant' },
  erreur: { icone: '⚠️', titre: 'Le chargement a échoué' },
  injoignable: {
    icone: '📡',
    titre: 'Le kernel ne répond pas',
    message: 'Vérifiez qu’il tourne et que le réseau passe, puis réessayez.',
  },
}

/**
 * Traduit une exception de chargement en { etat, message }.
 * `fetch` lève un TypeError quand rien ne répond (« Failed to fetch »,
 * « NetworkError », « Load failed » selon le navigateur).
 */
export function etatDepuisErreur(e) {
  const brut = String(e?.message || e || '')
  if (e instanceof TypeError || /failed to fetch|networkerror|load failed/i.test(brut)) {
    return { etat: 'injoignable', message: TEXTES.injoignable.message }
  }
  const http = brut.match(/HTTP (\d{3})/)
  if (http) {
    return { etat: 'erreur', message: `Le kernel a répondu ${http[1]}.` }
  }
  return { etat: 'erreur', message: brut || 'Erreur inconnue.' }
}

export class DataState extends LitElement {
  static properties = {
    etat: { type: String },
    titre: { type: String },
    message: { type: String },
  }

  static styles = [sharedBase, css`
    :host {
      display: block;
    }

    .etat {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--space-2, 0.5rem);
      padding: var(--space-8, 2rem) var(--space-4, 1rem);
      text-align: center;
      color: var(--color-dark-text-secondary, #cbd5e1);
    }

    .icone {
      font-size: 2rem;
      line-height: 1;
    }

    .titre {
      margin: 0;
      font-size: var(--text-base, 1rem);
      font-weight: 600;
      color: var(--color-dark-text-primary, #f1f5f9);
    }

    .message {
      margin: 0;
      max-width: 40ch;
      font-size: var(--text-sm, 0.875rem);
    }

    .erreur .titre { color: var(--color-warning, #fbbf24); }
    .injoignable .titre { color: var(--color-error, #f87171); }

    .spinner {
      width: 28px;
      height: 28px;
      border: 3px solid var(--border-subtle, rgba(255, 255, 255, 0.12));
      border-top-color: var(--context-primary, #10b981);
      border-radius: 50%;
      animation: tourne 0.8s linear infinite;
    }

    @keyframes tourne {
      to { transform: rotate(360deg); }
    }

    button {
      margin-top: var(--space-2, 0.5rem);
      min-height: 44px;
      padding: 0 var(--space-5, 1.25rem);
      border: 1px solid var(--border-default, rgba(255, 255, 255, 0.2));
      border-radius: var(--radius-md, 8px);
      background: var(--surface-glass, rgba(255, 255, 255, 0.06));
      color: var(--color-dark-text-primary, #f1f5f9);
      font: inherit;
      font-weight: 600;
      cursor: pointer;
    }

    button:hover {
      background: var(--surface-glass-hover, rgba(255, 255, 255, 0.1));
    }
  `]

  constructor() {
    super()
    this.etat = 'chargement'
    this.titre = ''
    this.message = ''
  }

  render() {
    const texte = TEXTES[this.etat] || TEXTES.erreur
    const enEchec = this.etat === 'erreur' || this.etat === 'injoignable'
    const message = this.message || texte.message || ''
    return html`
      <div class="etat ${this.etat}" role="${enEchec ? 'alert' : 'status'}">
        ${this.etat === 'chargement'
          ? html`<div class="spinner" aria-hidden="true"></div>`
          : html`<div class="icone" aria-hidden="true">${texte.icone}</div>`}
        <p class="titre">${this.titre || texte.titre}</p>
        ${message ? html`<p class="message">${message}</p>` : ''}
        ${enEchec ? html`<button @click="${this._reessayer}">Réessayer</button>` : ''}
      </div>
    `
  }

  _reessayer() {
    this.dispatchEvent(new CustomEvent('reessayer'))
  }
}

customElements.define('data-state', DataState)
