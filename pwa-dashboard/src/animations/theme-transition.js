/**
 * Symbion Theme Transition — Wavy Circular Expansion
 *
 * Expansion circulaire ondulée depuis le bouton toggle, avec halo lumineux
 * (radial-gradient scalé via transform — zéro filter, zéro clip-path sur le glow)
 * et traînées radiales.
 *
 * API: themeTransition.play({ to, origin, onSwitch })
 */

const STYLES = `
.stt-overlay {
  position: fixed;
  inset: 0;
  z-index: 100001;
  pointer-events: all;
}

.stt-glow {
  position: fixed;
  pointer-events: none;
  z-index: 100000;
  border-radius: 50%;
  transform: translate(-50%, -50%) scale(0);
  will-change: transform, opacity;
}

.stt-overlay.stt-to-light {
  background: linear-gradient(180deg, #f5f5f7 0%, #eeeef2 100%);
  --stt-shimmer: rgba(255, 240, 180, 0.3);
}

.stt-overlay.stt-to-dark {
  background: linear-gradient(180deg, #0e0e13 0%, #0a0a0f 100%);
  --stt-shimmer: rgba(130, 140, 255, 0.2);
}

.stt-shimmer { position: absolute; width: 0; height: 0; pointer-events: none; }

.stt-ray {
  position: absolute; left: 0; top: 0;
  width: var(--ray-len, 140px); height: 2px;
  transform-origin: 0 50%;
  transform: rotate(var(--ray-angle));
  background: linear-gradient(to right, var(--stt-shimmer), transparent);
  border-radius: 1px; opacity: 0;
  animation: stt-ray-shoot 1000ms ease-out forwards;
  animation-delay: var(--ray-delay, 0.4s);
}

@keyframes stt-ray-shoot {
  0%   { opacity: 0; transform: rotate(var(--ray-angle)) scaleX(0); }
  15%  { opacity: 1; }
  50%  { opacity: 0.7; transform: rotate(var(--ray-angle)) scaleX(1); }
  100% { opacity: 0; transform: rotate(var(--ray-angle)) scaleX(1.4); }
}

@media (prefers-reduced-motion: reduce) {
  .stt-overlay { clip-path: none !important; }
  .stt-glow { display: none !important; }
  .stt-ray { animation-duration: 0.01ms !important; animation-delay: 0.01ms !important; }
}
`

function easeInOutCubic(t) {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
}

class ThemeTransition {
  constructor() {
    this._overlay = null
    this._glowEl = null
    this._playing = false
    this._rafId = null
    this._ensureStyles()
  }

  _ensureStyles() {
    if (document.getElementById('symbion-theme-transition-styles')) return
    const s = document.createElement('style')
    s.id = 'symbion-theme-transition-styles'
    s.textContent = STYLES
    document.head.appendChild(s)
  }

  _generateRays() {
    const rays = []
    for (let i = 0; i < 12; i++) {
      const angle = (i / 12) * 360 + (Math.random() - 0.5) * 12
      const len = 100 + Math.random() * 200
      const delay = 0.35 + Math.random() * 0.45
      rays.push(`<div class="stt-ray" style="--ray-angle:${angle}deg;--ray-len:${len}px;--ray-delay:${delay}s"></div>`)
    }
    return rays.join('')
  }

  _wavyPolygon(cx, cy, r, t, ampFactor) {
    const N = 90, amp = 22 * ampFactor, pts = []
    for (let i = 0; i < N; i++) {
      const a = (i / N) * Math.PI * 2
      const wave = Math.sin(a * 8 + t * 2.5) * amp
                 + Math.sin(a * 13 - t * 1.8) * amp * 0.35
                 + Math.sin(a * 5 + t * 3.2) * amp * 0.2
      const finalR = Math.max(0, r + wave)
      pts.push(`${cx + Math.cos(a) * finalR}px ${cy + Math.sin(a) * finalR}px`)
    }
    return `polygon(${pts.join(',')})`
  }

  async play({ to, origin, onSwitch }) {
    if (this._playing) return
    this._playing = true
    this._ensureStyles()

    const themeClass = to === 'light' ? 'stt-to-light' : 'stt-to-dark'
    const isLight = to === 'light'
    const ox = origin?.x ?? window.innerWidth / 2
    const oy = origin?.y ?? window.innerHeight / 2
    const initialClip = `circle(0px at ${ox}px ${oy}px)`

    // Glow — single div with radial-gradient, scaled via transform (no filter, no clip-path)
    const glowSize = 300  // base size in px — scaled up via transform
    const glowEl = document.createElement('div')
    glowEl.className = 'stt-glow'
    const gc = isLight ? '255, 210, 60' : '140, 150, 255'
    glowEl.style.cssText = `
      left: ${ox}px; top: ${oy}px;
      width: ${glowSize}px; height: ${glowSize}px;
      background: radial-gradient(circle,
        rgba(${gc}, 0.45) 0%,
        rgba(${gc}, 0.25) 30%,
        rgba(${gc}, 0.08) 60%,
        transparent 100%
      );
    `
    document.body.appendChild(glowEl)
    this._glowEl = glowEl

    // Main overlay with clip-path
    const overlay = document.createElement('div')
    overlay.className = `stt-overlay ${themeClass}`
    overlay.style.clipPath = initialClip
    overlay.innerHTML = ''
    document.body.appendChild(overlay)
    this._overlay = overlay
    overlay.offsetHeight

    const EXPAND = 1500
    const diag = Math.sqrt(
      Math.max(ox, window.innerWidth - ox) ** 2 +
      Math.max(oy, window.innerHeight - oy) ** 2
    )
    const maxR = diag * 1.15
    const startTime = performance.now()

    // Phase 1: wavy expansion + glow scale
    await new Promise(resolve => {
      const frame = (now) => {
        const elapsed = now - startTime
        if (elapsed < EXPAND) {
          const progress = elapsed / EXPAND
          const eased = easeInOutCubic(progress)
          const r = eased * maxR
          const t = elapsed / 1000
          let ampFactor
          if (progress < 0.08) ampFactor = progress / 0.08
          else if (progress > 0.75) ampFactor = (1 - progress) / 0.25
          else ampFactor = 1

          // Main overlay — wavy clip-path
          overlay.style.clipPath = this._wavyPolygon(ox, oy, r, t, ampFactor)

          // Glow — scale to match radius, with extra spread
          const glowScale = (r * 2.4) / glowSize
          const glowOpacity = progress < 0.1 ? progress / 0.1
                            : progress > 0.7 ? (1 - progress) / 0.3
                            : 1
          glowEl.style.transform = `translate(-50%, -50%) scale(${glowScale})`
          glowEl.style.opacity = glowOpacity

          this._rafId = requestAnimationFrame(frame)
        } else {
          overlay.style.clipPath = 'inset(0)'
          glowEl.style.opacity = '0'
          this._rafId = null
          resolve()
        }
      }
      this._rafId = requestAnimationFrame(frame)
    })

    // Phase 2: switch theme under full-screen mask
    if (onSwitch) onSwitch()
    await new Promise(r => setTimeout(r, 400))

    // Phase 3: smooth CSS fade-out
    overlay.style.transition = 'opacity 500ms ease-out'
    overlay.style.pointerEvents = 'none'
    overlay.style.opacity = '0'

    await new Promise(r => setTimeout(r, 550))
    if (overlay.parentNode) overlay.remove()
    if (glowEl.parentNode) glowEl.remove()
    this._overlay = null
    this._glowEl = null
    this._playing = false
  }

  cancel() {
    if (this._rafId) { cancelAnimationFrame(this._rafId); this._rafId = null }
    if (this._overlay?.parentNode) this._overlay.remove()
    if (this._glowEl?.parentNode) this._glowEl.remove()
    this._overlay = null
    this._glowEl = null
    this._playing = false
  }
}

const themeTransition = new ThemeTransition()
export default themeTransition
