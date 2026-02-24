/**
 * Symbion Mode Transition — Epic Animation System
 *
 * Animation plein-ecran de transition de mode contextuel.
 * Singleton injecte dans document.body (hors Shadow DOM).
 *
 * API: modeTransition.play({ mode, icon, name, color, duration, origin })
 */

const STYLES = `
/* ============================================================
   Symbion Mode Transition (smt-*)
   GPU-accelerated, document.body overlay
   ============================================================ */

.smt-overlay {
  position: fixed;
  inset: 0;
  z-index: 100000;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  animation: smt-bg-flood 0.5s cubic-bezier(0.4, 0, 0.2, 1) forwards;
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  pointer-events: all;
}

/* === Phase 1: Ripple / Wave === */

.smt-ripple {
  position: absolute;
  left: var(--smt-origin-x);
  top: var(--smt-origin-y);
  width: 300vmax;
  height: 300vmax;
  border-radius: 50%;
  background: radial-gradient(
    circle,
    rgba(var(--smt-color-rgb), 0.35) 0%,
    rgba(var(--smt-color-rgb), 0.12) 40%,
    transparent 70%
  );
  transform: translate(-50%, -50%) scale(0);
  animation: smt-ripple-expand 0.6s cubic-bezier(0.22, 0.61, 0.36, 1) forwards;
  pointer-events: none;
  will-change: transform, opacity;
}

.smt-ripple-2 {
  animation-delay: 0.08s;
  animation-duration: 0.7s;
  background: radial-gradient(
    circle,
    rgba(var(--smt-color-rgb), 0.18) 0%,
    rgba(var(--smt-color-rgb), 0.06) 50%,
    transparent 70%
  );
}

@keyframes smt-ripple-expand {
  0% {
    transform: translate(-50%, -50%) scale(0);
    opacity: 0.9;
  }
  60% { opacity: 0.5; }
  100% {
    transform: translate(-50%, -50%) scale(1);
    opacity: 0;
  }
}

@keyframes smt-bg-flood {
  0%   { background-color: rgba(0, 0, 0, 0); }
  25%  { background-color: rgba(var(--smt-color-rgb), 0.10); }
  50%  { background-color: rgba(var(--smt-color-rgb), 0.06); }
  100% { background-color: rgba(0, 0, 0, 0.85); }
}

/* === Phase 2: Mode Reveal === */

.smt-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  position: relative;
  z-index: 2;
  will-change: transform, opacity;
}

/* Glow ring */
.smt-glow-ring {
  position: absolute;
  width: 180px;
  height: 180px;
  border-radius: 50%;
  border: 2px solid rgba(var(--smt-color-rgb), 0.4);
  animation: smt-glow-pulse 1.2s cubic-bezier(0.4, 0, 0.2, 1) 0.2s both;
  will-change: transform, opacity, box-shadow;
  z-index: 1;
}

@keyframes smt-glow-pulse {
  0% {
    transform: scale(0);
    opacity: 0;
    box-shadow: 0 0 0 0 rgba(var(--smt-color-rgb), 0);
  }
  40% {
    transform: scale(1);
    opacity: 0.6;
    box-shadow:
      0 0 60px 20px rgba(var(--smt-color-rgb), 0.3),
      0 0 120px 40px rgba(var(--smt-color-rgb), 0.15);
  }
  100% {
    transform: scale(1.6);
    opacity: 0;
    box-shadow:
      0 0 80px 30px rgba(var(--smt-color-rgb), 0),
      0 0 160px 60px rgba(var(--smt-color-rgb), 0);
  }
}

/* Icon */
.smt-icon {
  font-size: 5.5rem;
  line-height: 1;
  animation: smt-icon-enter 0.8s cubic-bezier(0.34, 1.56, 0.64, 1) 0.25s both;
  will-change: transform, opacity, filter;
  z-index: 2;
}

@keyframes smt-icon-enter {
  0% {
    transform: scale(0) rotate(-180deg);
    opacity: 0;
    filter: blur(10px);
  }
  50% {
    transform: scale(1.3) rotate(10deg);
    opacity: 1;
    filter: blur(0) drop-shadow(0 0 30px rgba(var(--smt-color-rgb), 0.8));
  }
  70% {
    transform: scale(0.9) rotate(-5deg);
  }
  100% {
    transform: scale(1) rotate(0deg);
    opacity: 1;
    filter: drop-shadow(0 0 20px rgba(var(--smt-color-rgb), 0.6));
  }
}

/* Mode name */
.smt-name {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 2.5rem;
  font-weight: 700;
  color: var(--smt-color);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin-top: 0.5rem;
  animation: smt-name-enter 0.6s cubic-bezier(0.22, 0.61, 0.36, 1) 0.5s both;
  text-shadow: 0 0 40px rgba(var(--smt-color-rgb), 0.5);
  will-change: transform, opacity;
  z-index: 2;
}

@keyframes smt-name-enter {
  0% {
    transform: translateY(30px) scaleX(0.3);
    opacity: 0;
    letter-spacing: 1em;
  }
  60% { letter-spacing: 0.15em; }
  100% {
    transform: translateY(0) scaleX(1);
    opacity: 1;
    letter-spacing: 0.08em;
  }
}

/* Duration subtitle */
.smt-subtitle {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 0.95rem;
  color: rgba(255, 255, 255, 0.6);
  margin-top: 0.25rem;
  animation: smt-subtitle-enter 0.4s ease-out 0.7s both;
  will-change: transform, opacity;
  z-index: 2;
}

@keyframes smt-subtitle-enter {
  from { opacity: 0; transform: translateY(15px); }
  to   { opacity: 0.7; transform: translateY(0); }
}

/* Particles */
.smt-particles {
  position: absolute;
  width: 0;
  height: 0;
  z-index: 3;
}

.smt-particle {
  position: absolute;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--smt-color);
  box-shadow: 0 0 12px 4px rgba(var(--smt-color-rgb), 0.6);
  animation: smt-particle-burst 1s cubic-bezier(0.4, 0, 0.2, 1) both;
  animation-delay: calc(0.3s + var(--i) * 0.04s);
  will-change: transform, opacity;
}

@keyframes smt-particle-burst {
  0% {
    transform: rotate(calc(var(--i) * 30deg)) translateY(0) scale(0);
    opacity: 0;
  }
  30% {
    opacity: 1;
    transform: rotate(calc(var(--i) * 30deg)) translateY(-70px) scale(1);
  }
  100% {
    transform: rotate(calc(var(--i) * 30deg + 60deg)) translateY(-150px) scale(0);
    opacity: 0;
  }
}

/* HUD Scanline */
.smt-scanline {
  position: absolute;
  width: 300px;
  height: 2px;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(var(--smt-color-rgb), 0.4),
    rgba(var(--smt-color-rgb), 0.7),
    rgba(var(--smt-color-rgb), 0.4),
    transparent
  );
  animation: smt-scanline-sweep 1s ease-in-out 0.4s both;
  will-change: transform, opacity;
  z-index: 4;
  pointer-events: none;
}

@keyframes smt-scanline-sweep {
  0%   { transform: translateY(-200px); opacity: 0; }
  20%  { opacity: 0.6; }
  80%  { opacity: 0.6; }
  100% { transform: translateY(200px); opacity: 0; }
}

/* === Phase 3: Dissolution === */

.smt-dissolve {
  position: absolute;
  inset: 0;
  opacity: 0;
  pointer-events: none;
  z-index: 5;
}

.smt-hex {
  position: absolute;
  width: calc(100vw / 6);
  height: calc(100vh / 4);
  background: rgba(var(--smt-color-rgb), 0.03);
  border: 1px solid rgba(var(--smt-color-rgb), 0.06);
  animation: smt-hex-dissolve 0.5s cubic-bezier(0.4, 0, 0.2, 1) both;
  animation-delay: var(--delay);
  will-change: transform, opacity;
}

@keyframes smt-hex-dissolve {
  0% {
    transform: scale(1) translate(0, 0) rotate(0deg);
    opacity: 0.5;
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
  }
  100% {
    transform:
      scale(0.3)
      translate(
        calc((var(--hx) - 2.5) * 60px),
        calc((var(--hy) - 1.5) * 60px)
      )
      rotate(calc(var(--hx) * 30deg));
    opacity: 0;
  }
}

/* Content exit (applied via JS at phase 3) */
@keyframes smt-content-exit {
  0% {
    transform: scale(1) translateY(0);
    opacity: 1;
    filter: blur(0);
  }
  100% {
    transform: scale(0.85) translateY(-30px);
    opacity: 0;
    filter: blur(8px);
  }
}

/* Overlay exit (applied via JS) */
@keyframes smt-overlay-exit {
  0% {
    opacity: 1;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }
  100% {
    opacity: 0;
    backdrop-filter: blur(0);
    -webkit-backdrop-filter: blur(0);
  }
}

/* === Reduced Motion === */
@media (prefers-reduced-motion: reduce) {
  .smt-overlay,
  .smt-ripple,
  .smt-ripple-2,
  .smt-icon,
  .smt-name,
  .smt-subtitle,
  .smt-glow-ring,
  .smt-particle,
  .smt-scanline,
  .smt-hex {
    animation-duration: 0.01ms;
    animation-delay: 0.01ms;
  }
  .smt-overlay {
    transition: opacity 0.3s ease;
  }
}
`

class ModeTransition {
  constructor() {
    this._overlay = null
    this._playing = false
    this._ensureStyles()
  }

  _ensureStyles() {
    if (document.getElementById('symbion-mode-transition-styles')) return
    const style = document.createElement('style')
    style.id = 'symbion-mode-transition-styles'
    style.textContent = STYLES
    document.head.appendChild(style)
  }

  _hexToRgb(hex) {
    hex = hex.replace('#', '')
    return [
      parseInt(hex.substring(0, 2), 16),
      parseInt(hex.substring(2, 4), 16),
      parseInt(hex.substring(4, 6), 16)
    ].join(',')
  }

  _generateHexGrid() {
    const cols = 6
    const rows = 4
    let html = ''
    for (let y = 0; y < rows; y++) {
      for (let x = 0; x < cols; x++) {
        const cx = (cols - 1) / 2
        const cy = (rows - 1) / 2
        const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2)
        const maxDist = Math.sqrt(cx ** 2 + cy ** 2)
        const delay = 1.3 + (dist / maxDist) * 0.3
        html += `<div class="smt-hex" style="--hx:${x};--hy:${y};--delay:${delay.toFixed(2)}s;left:${(x / cols) * 100}%;top:${(y / rows) * 100}%"></div>`
      }
    }
    return html
  }

  /**
   * Play the epic mode transition animation.
   * @param {Object} opts
   * @param {string} opts.icon - Emoji icon
   * @param {string} opts.name - Mode display name
   * @param {string} opts.color - Primary hex color (e.g. '#2563eb')
   * @param {string} opts.duration - Human-readable duration (e.g. '1h')
   * @param {{x: number, y: number}} [opts.origin] - Click position for ripple
   * @returns {Promise<void>}
   */
  async play({ icon, name, color, duration, origin }) {
    if (this._playing) return
    this._playing = true
    this._ensureStyles()

    const ox = origin?.x ?? window.innerWidth / 2
    const oy = origin?.y ?? window.innerHeight / 2
    const rgb = this._hexToRgb(color)

    const overlay = document.createElement('div')
    overlay.className = 'smt-overlay'
    overlay.style.cssText = `--smt-color:${color};--smt-color-rgb:${rgb};--smt-origin-x:${ox}px;--smt-origin-y:${oy}px;`

    const particles = Array.from({ length: 12 }, (_, i) =>
      `<span class="smt-particle" style="--i:${i}"></span>`
    ).join('')

    overlay.innerHTML = `
      <div class="smt-ripple"></div>
      <div class="smt-ripple smt-ripple-2"></div>
      <div class="smt-content">
        <div class="smt-particles">${particles}</div>
        <div class="smt-glow-ring"></div>
        <div class="smt-icon">${icon}</div>
        <div class="smt-name">${name}</div>
        <div class="smt-subtitle">Activ\u00e9 pour ${duration}</div>
        <div class="smt-scanline"></div>
      </div>
      <div class="smt-dissolve">${this._generateHexGrid()}</div>
    `

    document.body.appendChild(overlay)
    this._overlay = overlay

    // Force reflow
    overlay.offsetHeight

    return new Promise((resolve) => {
      // Phase 3: content exit + hex dissolve at 1.3s
      setTimeout(() => {
        const content = overlay.querySelector('.smt-content')
        if (content) {
          content.style.animation = 'smt-content-exit 0.5s cubic-bezier(0.4, 0, 0.2, 1) forwards'
        }
        const dissolve = overlay.querySelector('.smt-dissolve')
        if (dissolve) {
          dissolve.style.opacity = '1'
        }
      }, 1300)

      // Overlay fade-out at 1.6s
      setTimeout(() => {
        overlay.style.animation = 'smt-overlay-exit 0.4s ease-in forwards'
      }, 1600)

      // Cleanup at 2.0s
      setTimeout(() => {
        if (overlay.parentNode) {
          overlay.parentNode.removeChild(overlay)
        }
        this._overlay = null
        this._playing = false
        resolve()
      }, 2000)
    })
  }

  cancel() {
    if (this._overlay?.parentNode) {
      this._overlay.parentNode.removeChild(this._overlay)
    }
    this._overlay = null
    this._playing = false
  }
}

const modeTransition = new ModeTransition()
export default modeTransition
