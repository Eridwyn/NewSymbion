import { LitElement, html, css } from 'lit'

class SkeletonLoader extends LitElement {
  static styles = css`
    :host {
      display: block;
    }

    .skeleton-container {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
      padding: 0.5rem 0;
    }

    .skeleton-line {
      height: var(--skeleton-height, 14px);
      background: linear-gradient(90deg,
        rgba(255,255,255,0.04) 25%,
        rgba(255,255,255,0.08) 50%,
        rgba(255,255,255,0.04) 75%
      );
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
      border-radius: var(--radius-sm, 4px);
    }

    .skeleton-line.title {
      height: 20px;
      width: 60%;
    }

    .skeleton-line.text {
      height: 14px;
      width: 100%;
    }

    .skeleton-line.text-short {
      height: 14px;
      width: 75%;
    }

    .skeleton-line.badge {
      height: 24px;
      width: 80px;
      border-radius: 12px;
    }

    .skeleton-line.circle {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      flex-shrink: 0;
    }

    .skeleton-row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .skeleton-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
      gap: 0.75rem;
    }

    .skeleton-card {
      height: 80px;
      background: linear-gradient(90deg,
        rgba(255,255,255,0.04) 25%,
        rgba(255,255,255,0.08) 50%,
        rgba(255,255,255,0.04) 75%
      );
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
      border-radius: var(--radius-base, 6px);
    }

    @keyframes shimmer {
      0% { background-position: 200% 0; }
      100% { background-position: -200% 0; }
    }

    /* Variants */
    :host([variant="widget"]) .skeleton-container {
      gap: 1rem;
    }

    :host([variant="list"]) .skeleton-container {
      gap: 0.5rem;
    }

    @media (prefers-reduced-motion: reduce) {
      .skeleton-line, .skeleton-card {
        animation: none;
        opacity: 0.5;
      }
    }
  `

  static properties = {
    variant: { type: String, reflect: true },
    lines: { type: Number }
  }

  constructor() {
    super()
    this.variant = 'widget'
    this.lines = 3
  }

  render() {
    if (this.variant === 'list') {
      return html`
        <div class="skeleton-container">
          ${Array.from({ length: this.lines }, () => html`
            <div class="skeleton-row">
              <div class="skeleton-line circle"></div>
              <div style="flex:1; display:flex; flex-direction:column; gap:0.4rem;">
                <div class="skeleton-line text" style="width:${60 + Math.random() * 30}%"></div>
                <div class="skeleton-line text-short" style="width:${40 + Math.random() * 20}%"></div>
              </div>
              <div class="skeleton-line badge"></div>
            </div>
          `)}
        </div>
      `
    }

    if (this.variant === 'grid') {
      return html`
        <div class="skeleton-container">
          <div class="skeleton-line title"></div>
          <div class="skeleton-grid">
            ${Array.from({ length: this.lines }, () => html`
              <div class="skeleton-card"></div>
            `)}
          </div>
        </div>
      `
    }

    // Default: widget
    return html`
      <div class="skeleton-container">
        <div class="skeleton-row">
          <div class="skeleton-line title"></div>
          <div class="skeleton-line badge"></div>
        </div>
        ${Array.from({ length: this.lines }, (_, i) => html`
          <div class="skeleton-line ${i === this.lines - 1 ? 'text-short' : 'text'}"></div>
        `)}
      </div>
    `
  }
}

customElements.define('skeleton-loader', SkeletonLoader)
export { SkeletonLoader }
