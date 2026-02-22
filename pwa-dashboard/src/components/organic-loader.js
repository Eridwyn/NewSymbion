/**
 * Organic Loader Component - Symbion
 * Loader bioluminescent avec impulsions réutilisable partout
 */

import { LitElement, html, css } from 'lit'

class OrganicLoader extends LitElement {
  static properties = {
    text: { type: String }
  }

  static styles = css`
    :host {
      display: block;
    }

    .organic-loader {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      margin: 2rem auto;
      position: relative;
    }

    .organic-loader__container {
      position: relative;
      width: 140px;
      height: 140px;
      animation: slow-spin 30s linear infinite;
    }

    .organic-loader__blob {
      position: absolute;
      top: 50%;
      left: 50%;
      width: 90px;
      height: 90px;
      margin: -45px 0 0 -45px;
      background: radial-gradient(
        circle,
        color-mix(in srgb, var(--context-primary) 70%, transparent) 0%,
        var(--ctx-border-intense) 50%,
        var(--ctx-bg-intense) 100%
      );
      animation: blob-morph 6s ease-in-out infinite,
                 bio-glow 3s ease-in-out infinite;
      backdrop-filter: blur(8px);
      z-index: 2;
    }

    .organic-loader__ripple {
      position: absolute;
      top: 50%;
      left: 50%;
      width: 90px;
      height: 90px;
      margin: -45px 0 0 -45px;
      background: radial-gradient(
        circle,
        var(--ctx-border-strong) 0%,
        var(--ctx-border-medium) 30%,
        var(--ctx-bg) 60%,
        transparent 100%
      );
      border-radius: 50%;
      animation: light-propagate 4s ease-out infinite;
      z-index: 1;
    }

    .organic-loader__ripple:nth-child(2) {
      animation-delay: 1s;
    }

    .organic-loader__ripple:nth-child(3) {
      animation-delay: 2s;
    }

    .organic-loader__ripple:nth-child(4) {
      animation-delay: 3s;
    }

    .organic-loader__particle {
      position: absolute;
      background: var(--context-primary, #00d4aa);
      border-radius: 50%;
      animation: float ease-in-out infinite;
      z-index: 3;
    }

    .organic-loader__particle--1 {
      top: 15%;
      left: 20%;
      width: 8px;
      height: 8px;
      animation-duration: 3s;
    }

    .organic-loader__particle--2 {
      top: 20%;
      right: 25%;
      width: 6px;
      height: 6px;
      animation-duration: 2.7s;
      animation-delay: 0.5s;
    }

    .organic-loader__particle--3 {
      bottom: 18%;
      left: 22%;
      width: 7px;
      height: 7px;
      animation-duration: 3.2s;
      animation-delay: 1s;
    }

    .organic-loader__particle--4 {
      bottom: 20%;
      right: 20%;
      width: 5px;
      height: 5px;
      animation-duration: 2.9s;
      animation-delay: 1.5s;
    }

    .organic-loader__text {
      text-align: center;
      color: var(--context-primary, #00d4aa);
      font-size: 0.85em;
      margin-top: 1rem;
      opacity: 0.8;
      animation: text-fade 2.5s ease-in-out infinite;
    }

    /* ANIMATIONS */

    @keyframes blob-morph {
      0%, 100% {
        border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%;
        transform: scale(1) rotate(0deg);
      }
      25% {
        border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%;
        transform: scale(1.08) rotate(5deg);
      }
      50% {
        border-radius: 50% 50% 20% 80% / 25% 60% 60% 80%;
        transform: scale(0.92) rotate(-3deg);
      }
      75% {
        border-radius: 70% 30% 50% 50% / 30% 30% 70% 70%;
        transform: scale(1.05) rotate(8deg);
      }
    }

    @keyframes bio-glow {
      0%, 100% {
        box-shadow: 0 0 20px var(--ctx-border-strong),
                    0 0 40px var(--ctx-bg-intense),
                    inset 0 0 30px color-mix(in srgb, var(--context-primary) 15%, transparent);
      }
      50% {
        box-shadow: 0 0 40px color-mix(in srgb, var(--context-primary) 70%, transparent),
                    0 0 80px var(--ctx-border-intense),
                    inset 0 0 50px var(--ctx-bg-intense);
      }
    }

    @keyframes light-propagate {
      0% {
        transform: scale(0.5);
        opacity: 0;
      }
      20% {
        opacity: 0.8;
      }
      100% {
        transform: scale(3);
        opacity: 0;
      }
    }

    @keyframes float {
      0%, 100% {
        transform: translateY(0) scale(1);
        opacity: 0.4;
      }
      50% {
        transform: translateY(-15px) scale(1.2);
        opacity: 0.9;
      }
    }

    @keyframes slow-spin {
      from {
        transform: rotate(0deg);
      }
      to {
        transform: rotate(360deg);
      }
    }

    @keyframes text-fade {
      0%, 100% {
        opacity: 0.5;
      }
      50% {
        opacity: 1;
      }
    }
  `

  constructor() {
    super()
    this.text = '🧬 Organisme en synapse...'
  }

  render() {
    return html`
      <div class="organic-loader">
        <div class="organic-loader__container">
          <!-- Propagations lumineuses graduelles -->
          <div class="organic-loader__ripple"></div>
          <div class="organic-loader__ripple"></div>
          <div class="organic-loader__ripple"></div>
          <div class="organic-loader__ripple"></div>

          <!-- Blob central organique -->
          <div class="organic-loader__blob"></div>
        </div>

        ${this.text ? html`
          <div class="organic-loader__text">${this.text}</div>
        ` : ''}
      </div>
    `
  }
}

customElements.define('organic-loader', OrganicLoader)

export default OrganicLoader
