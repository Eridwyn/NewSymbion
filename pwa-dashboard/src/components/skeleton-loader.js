import { LitElement, html, css } from 'lit'

class SkeletonLoader extends LitElement {
  static properties = {
    lines: { type: Number },
    showAvatar: { type: Boolean, attribute: 'show-avatar' },
    showHeader: { type: Boolean, attribute: 'show-header' }
  }

  static styles = css`
    :host {
      display: block;
      padding: var(--space-4, 1rem);
      animation: skeleton-fade-in var(--duration-base, 0.25s) var(--ease-out, ease-out);
    }

    :host(.removing) {
      animation: skeleton-fade-out var(--duration-base, 0.25s) var(--ease-out, ease-out) forwards;
    }

    .skeleton-line {
      height: 12px;
      background: linear-gradient(90deg,
        var(--color-dark-bg-tertiary, #2a2d35) 25%,
        var(--color-dark-bg-secondary, #1e2028) 50%,
        var(--color-dark-bg-tertiary, #2a2d35) 75%
      );
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
      border-radius: var(--radius-sm, 4px);
      margin-bottom: var(--space-3, 0.75rem);
    }

    .skeleton-line:last-child {
      margin-bottom: 0;
    }

    .skeleton-line.short { width: 60%; }
    .skeleton-line.medium { width: 80%; }
    .skeleton-line.long { width: 100%; }

    .skeleton-header {
      height: 20px;
      width: 40%;
      margin-bottom: var(--space-4, 1rem);
    }

    .skeleton-avatar {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      margin-bottom: var(--space-4, 1rem);
    }

    @keyframes shimmer {
      0% { background-position: 200% 0; }
      100% { background-position: -200% 0; }
    }

    @keyframes skeleton-fade-in {
      from { opacity: 0; transform: translateY(4px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @keyframes skeleton-fade-out {
      from { opacity: 1; transform: translateY(0); }
      to { opacity: 0; transform: translateY(-4px); }
    }
  `

  constructor() {
    super()
    this.lines = 3
    this.showAvatar = false
    this.showHeader = false
  }

  render() {
    const widths = ['long', 'medium', 'short']
    return html`
      ${this.showAvatar ? html`<div class="skeleton-line skeleton-avatar"></div>` : ''}
      ${this.showHeader ? html`<div class="skeleton-line skeleton-header"></div>` : ''}
      ${Array.from({ length: this.lines }, (_, i) => html`
        <div class="skeleton-line ${widths[i % 3]}"></div>
      `)}
    `
  }
}

customElements.define('skeleton-loader', SkeletonLoader)
