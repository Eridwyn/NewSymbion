# Symbion PWA — Design System & Shared Styles

## Architecture

```
src/styles/
  design-system.css      # Tokens CSS (200+) appliques via :root
  shared-animations.js   # @keyframes partages (12 animations) — Lit css module
  shared-patterns.js     # Patterns CSS reutilisables (7 exports) — Lit css module
```

## Design System Tokens (`design-system.css`)

Charge globalement dans `index.html`. Les custom properties penetrent le Shadow DOM des Web Components Lit.

### Categories de tokens

| Categorie | Prefix | Exemple |
|-----------|--------|---------|
| Typography | `--font-*`, `--text-*` | `--text-sm`, `--font-semibold` |
| Spacing | `--space-*` | `--space-md` (1rem) |
| Border radius | `--radius-*` | `--radius-base` (8px), `--radius-md` (12px) |
| Durations | `--duration-*` | `--duration-base` (0.3s) |
| Easings | `--ease-*` | `--ease-out` |
| Surfaces | `--surface-*` | `--surface-glass`, `--surface-overlay` |
| Borders | `--border-*` | `--border-default`, `--border-hover` |
| Status colors | `--color-{status}-{type}` | `--color-success-bg`, `--color-danger-text` |
| Context derives | `--ctx-*` | `--ctx-border`, `--ctx-bg-subtle`, `--ctx-glow-md` |

### Status colors (5 familles)

Chaque famille a 3 variantes : `-bg`, `-border`, `-text`.

- `--color-success-*` (#22c55e) — en ligne, actif, valide
- `--color-warning-*` (#fbbf24) — attention, en attente
- `--color-error-*` (#ef4444) — erreur systeme
- `--color-danger-*` (#ff6b6b) — action destructrice
- `--color-info-*` (#3b82f6) — information neutre

### Context-primary derives

Tokens derives de `--context-primary` (couleur du mode actif) via `color-mix()` :

```css
--ctx-border-subtle   /* 15% opacity */
--ctx-border          /* 25% opacity */
--ctx-border-medium   /* 30% opacity */
--ctx-border-strong   /* 40% opacity */
--ctx-bg-subtle       /* 8% opacity */
--ctx-bg              /* 12% opacity */
--ctx-bg-medium       /* 20% opacity */
--ctx-bg-strong       /* 30% opacity */
--ctx-glow-sm         /* 0 0 10px, 15% */
--ctx-glow-md         /* 0 0 20px, 25% */
--ctx-glow-lg         /* 0 0 30px, 35% */
```

---

## Shared Animations (`shared-animations.js`)

12 `@keyframes` partages entre composants.

### Usage

```js
import { sharedAnimations } from '../styles/shared-animations.js'

class MyComponent extends LitElement {
  static styles = [sharedAnimations, css`
    .element { animation: fadeIn 0.3s ease-out; }
  `]
}
```

### Animations disponibles

| Nom | Description |
|-----|-------------|
| `fadeIn` | Opacity 0 → 1 |
| `fadeOut` | Opacity 1 → 0 |
| `slideUp` | Translate Y +40px → 0 + scale |
| `slideDown` | Translate Y -20px → 0 |
| `scaleIn` | Scale 0.9 → 1 + fade |
| `spin` | Rotation 360deg |
| `shimmer` | Background position slide (loading) |
| `float` | Mouvement organique 3 points |
| `pulse` | Opacity pulse 1 → 0.6 → 1 |
| `modalSlideIn` | Translate Y -30px + scale 0.95 → 1 |
| `titlePulse` | Drop-shadow pulse (context-aware) |
| `inputGlow` | Box-shadow glow (context-aware) |

---

## Shared Patterns (`shared-patterns.js`)

7 patterns CSS reutilisables, exportes en tant que `css` Lit.

### Usage

```js
import { overlayStyles, closeButtonStyles, badgeStyles } from '../styles/shared-patterns.js'

class MyModal extends LitElement {
  static styles = [overlayStyles, closeButtonStyles, badgeStyles, css`
    /* styles locaux */
  `]
}
```

### Exports disponibles

| Export | Classes CSS | Description |
|--------|------------|-------------|
| `overlayStyles` | `:host` | Overlay plein ecran avec backdrop blur |
| `closeButtonStyles` | `.close-btn` | Bouton fermer (danger style, top-right) |
| `badgeStyles` | `.badge`, `.badge-success/warning/error/info` | Badges de statut |
| `scrollbarStyles` | `::-webkit-scrollbar` | Scrollbar custom context-aware |
| `sectionCardStyles` | `.section-card` | Card avec bordure et hover |
| `formInputStyles` | `.form-input`, `.form-textarea`, `.form-select` | Inputs avec focus context-primary |

---

## Convention de nommage

- **Tokens** : `--category-variant` (ex: `--border-hover`, `--ctx-bg-subtle`)
- **Classes partagees** : BEM simplifie (ex: `.badge-success`, `.form-input`)
- **Animations** : camelCase (ex: `fadeIn`, `modalSlideIn`)

## Exception : `environment-widget.js`

Ce widget n'utilise **pas** le Shadow DOM (il injecte dans le DOM global). Les tokens `:root` s'appliquent directement sans import de modules Lit.
