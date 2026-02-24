/**
 * Symbion Theme Transition — Wave Sweep
 *
 * Vague plein-ecran avec bords ondules, crête SVG lumineuse et ombres.
 * Singleton injecte dans document.body (hors Shadow DOM).
 *
 * API: themeTransition.play({ to, onSwitch })
 */

const STYLES = `
/* ============================================================
   Symbion Theme Transition (stt-*)
   GPU-accelerated wavy sweep, document.body
   ============================================================ */

.stt-wave {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 100001;
  pointer-events: all;
  transform: translateX(-104%);
  will-change: transform;
  overflow: visible;
  filter: drop-shadow(8px 0 20px var(--stt-shadow));
}

.stt-wave.stt-to-light {
  background: #eef0f4;
  --stt-solid: #eef0f4;
  --stt-shadow: rgba(0, 0, 0, 0.3);
  --stt-crest-core: rgba(255, 220, 80, 0.95);
  --stt-crest-mid: rgba(255, 200, 50, 0.45);
  --stt-crest-outer: rgba(255, 200, 50, 0.15);
  --stt-shimmer: rgba(255, 240, 180, 0.18);
}

.stt-wave.stt-to-dark {
  background: #111827;
  --stt-solid: #111827;
  --stt-shadow: rgba(0, 0, 0, 0.5);
  --stt-crest-core: rgba(170, 180, 255, 0.9);
  --stt-crest-mid: rgba(140, 150, 255, 0.4);
  --stt-crest-outer: rgba(140, 150, 255, 0.12);
  --stt-shimmer: rgba(130, 140, 255, 0.1);
}

/* === Wave sweep — 1.8s === */

.stt-wave.stt-sweeping {
  animation: stt-sweep 1.8s cubic-bezier(0.45, 0.05, 0.55, 0.95) forwards;
}

@keyframes stt-sweep {
  0%   { transform: translateX(-104%); }
  100% { transform: translateX(104%); }
}

/* === SVG wavy edges === */

.stt-edge-r {
  position: absolute;
  top: 0;
  left: calc(100% - 1px);
  width: 50px;
  height: 100%;
  transform-origin: left center;
  animation: stt-undulate-r 1.8s ease-in-out forwards;
  will-change: transform;
}

.stt-edge-l {
  position: absolute;
  top: 0;
  right: calc(100% - 1px);
  width: 40px;
  height: 100%;
  transform-origin: right center;
  animation: stt-undulate-l 1.8s ease-in-out forwards;
  will-change: transform;
}

/* SVG fills & strokes */
.stt-edge-fill { fill: var(--stt-solid); }
.stt-edge-l-fill { fill: var(--stt-solid); }

/* Crest glow — 3 layers following the wave shape */
.stt-crest-outer {
  fill: none;
  stroke: var(--stt-crest-outer);
  stroke-width: 18px;
  stroke-linecap: round;
}
.stt-crest-mid {
  fill: none;
  stroke: var(--stt-crest-mid);
  stroke-width: 8px;
  stroke-linecap: round;
}
.stt-crest-core {
  fill: none;
  stroke: var(--stt-crest-core);
  stroke-width: 2.5px;
  stroke-linecap: round;
}

/* Undulation — edges breathe as the wave sweeps */

@keyframes stt-undulate-r {
  0%   { transform: scaleX(0.6); }
  15%  { transform: scaleX(1.15); }
  35%  { transform: scaleX(0.8); }
  55%  { transform: scaleX(1.1); }
  75%  { transform: scaleX(0.85); }
  100% { transform: scaleX(1); }
}

@keyframes stt-undulate-l {
  0%   { transform: scaleX(0.4); }
  20%  { transform: scaleX(0.9); }
  50%  { transform: scaleX(1.1); }
  80%  { transform: scaleX(0.8); }
  100% { transform: scaleX(1); }
}

/* === Shimmer streaks === */

.stt-shimmer {
  position: absolute;
  width: 100%;
  height: 100%;
  top: 0;
  left: 0;
  overflow: hidden;
  pointer-events: none;
}

.stt-streak {
  position: absolute;
  right: var(--sx);
  top: var(--sy);
  width: var(--sw);
  height: 2px;
  background: linear-gradient(to left, var(--stt-shimmer), transparent);
  border-radius: 1px;
  opacity: 0;
  animation: stt-streak-slide 1.8s ease-in-out forwards;
  animation-delay: var(--sd);
  will-change: transform, opacity;
}

@keyframes stt-streak-slide {
  0%   { opacity: 0; transform: translateX(40px); }
  15%  { opacity: 1; }
  60%  { opacity: 0.7; }
  85%  { opacity: 0; transform: translateX(-80px); }
  100% { opacity: 0; }
}

/* === Reduced Motion === */

@media (prefers-reduced-motion: reduce) {
  .stt-wave.stt-sweeping,
  .stt-streak,
  .stt-edge-r,
  .stt-edge-l {
    animation-duration: 0.01ms !important;
    animation-delay: 0.01ms !important;
  }
  .stt-wave {
    filter: none;
  }
}
`

class ThemeTransition {
  constructor() {
    this._wave = null
    this._playing = false
    this._ensureStyles()
  }

  _ensureStyles() {
    if (document.getElementById('symbion-theme-transition-styles')) return
    const style = document.createElement('style')
    style.id = 'symbion-theme-transition-styles'
    style.textContent = STYLES
    document.head.appendChild(style)
  }

  /**
   * Generate sine-wave points with jitter.
   * Returns array of {x, y} in SVG viewBox coordinates.
   */
  _wavePoints(h, steps, amp, waves, phase = -Math.PI / 2) {
    const pts = []
    for (let i = 0; i <= steps; i++) {
      const t = i / steps
      const base = amp * (0.5 + 0.5 * Math.sin(t * waves * Math.PI * 2 + phase))
      const jitter = (i === 0 || i === steps) ? 0 : (Math.random() - 0.5) * 4
      pts.push({ x: Math.max(0, base + jitter), y: t * h })
    }
    return pts
  }

  /** Build right (leading) edge SVG with fill + 3-layer crest glow */
  _rightEdge() {
    const h = 200, steps = 50, amp = 45, waves = 3.5
    const pts = this._wavePoints(h, steps, amp, waves)

    // Closed fill shape
    let fillD = 'M 0 0'
    pts.forEach(p => { fillD += ` L ${p.x.toFixed(1)} ${p.y.toFixed(1)}` })
    fillD += ` L 0 ${h} Z`

    // Open crest line (just the wavy edge)
    let crestD = `M ${pts[0].x.toFixed(1)} ${pts[0].y.toFixed(1)}`
    pts.slice(1).forEach(p => { crestD += ` L ${p.x.toFixed(1)} ${p.y.toFixed(1)}` })

    return `<svg class="stt-edge-r" viewBox="0 0 ${amp} ${h}" preserveAspectRatio="none">
      <path d="${fillD}" class="stt-edge-fill"/>
      <path d="${crestD}" class="stt-crest-outer" vector-effect="non-scaling-stroke"/>
      <path d="${crestD}" class="stt-crest-mid" vector-effect="non-scaling-stroke"/>
      <path d="${crestD}" class="stt-crest-core" vector-effect="non-scaling-stroke"/>
    </svg>`
  }

  /** Build left (trailing) edge SVG — softer, no crest */
  _leftEdge() {
    const h = 200, steps = 40, amp = 35, waves = 3, w = amp
    const pts = this._wavePoints(h, steps, amp, waves, 0)

    let d = `M ${w} 0`
    pts.forEach(p => {
      const x = w - p.x
      d += ` L ${Math.min(w, Math.max(0, x)).toFixed(1)} ${p.y.toFixed(1)}`
    })
    d += ` L ${w} ${h} Z`

    return `<svg class="stt-edge-l" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none">
      <path d="${d}" class="stt-edge-l-fill"/>
    </svg>`
  }

  _generateShimmer() {
    const streaks = []
    for (let i = 0; i < 6; i++) {
      const sx = 60 + Math.random() * 500
      const sy = 10 + (i * 16) + Math.random() * 4
      const sw = 80 + Math.random() * 200
      const sd = 0.1 + Math.random() * 0.6
      streaks.push(
        `<div class="stt-streak" style="--sx:${sx}px;--sy:${sy}%;--sw:${sw}px;--sd:${sd}s"></div>`
      )
    }
    return streaks.join('')
  }

  /**
   * Play the wave sweep transition.
   * @param {Object} opts
   * @param {'light'|'dark'} opts.to - Target theme
   * @param {Function} [opts.onSwitch] - Callback fired when theme should switch
   * @returns {Promise<void>}
   */
  async play({ to, onSwitch }) {
    if (this._playing) return
    this._playing = true
    this._ensureStyles()

    const toLight = to === 'light'

    const wave = document.createElement('div')
    wave.className = `stt-wave ${toLight ? 'stt-to-light' : 'stt-to-dark'}`

    wave.innerHTML = `
      ${this._rightEdge()}
      ${this._leftEdge()}
      <div class="stt-shimmer">${this._generateShimmer()}</div>
    `

    document.body.appendChild(wave)
    this._wave = wave

    // Force reflow
    wave.offsetHeight

    return new Promise(resolve => {
      requestAnimationFrame(() => {
        wave.classList.add('stt-sweeping')
      })

      // Switch theme at midpoint
      setTimeout(() => {
        if (onSwitch) onSwitch()
      }, 820)

      // Cleanup after wave exits
      setTimeout(() => {
        if (wave.parentNode) wave.remove()
        this._wave = null
        this._playing = false
        resolve()
      }, 1900)
    })
  }

  cancel() {
    if (this._wave?.parentNode) {
      this._wave.remove()
    }
    this._wave = null
    this._playing = false
  }
}

const themeTransition = new ThemeTransition()
export default themeTransition
