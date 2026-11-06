# Rookery Iconography Pipeline

Rookery uses a unified **two-tone (duotone) SVG icon system** that combines vendored [Phosphor Icons](https://phosphoricons.com/) (MIT) with hand-authored custom Rookery icons on the same `256×256` grid and stroke language.

## Quick Commands

Run from the repository root:

```bash
# Rebuild the TypeScript registry, standalone SVGs, PNGs, and preview sheet
npm run icons

# Re-vendor Phosphor icons from @phosphor-icons/core and rebuild all outputs
npm run icons:sync
```

## Directory Layout

- [`icons.config.json`](./icons.config.json) — Single source of truth mapping semantic icon names (used in `<rk-icon name="...">` and `rookery.toml` `icon` fields) to either `phosphor:<name>` or `custom:<name>`, plus export sizes and theme palettes.
- [`src/custom/`](./src/custom/) — Hand-authored Rookery SVGs (`koi`, `nebula`, `satellite`, `branch`, `lily-pad`, `pellets`, `ripple`, `nova`, `hab-ring`).
- [`src/phosphor/`](./src/phosphor/) — Vendored duotone SVGs from `@phosphor-icons/core` (populated by `npm run icons:sync`).
- [`build.mjs`](./build.mjs) — Normalizer and asset generator.
- [`export/svg/`](./export/svg/) — Standalone `256×256` SVG files for every icon.
- [`export/png/{light,dark}/{16,32,64,128}/`](./export/png/) — Rasterized PNG exports for tray icons, documentation, and store assets.
- [`export/preview.png`](./export/preview.png) — Contact sheet previewing every icon across all realm palettes.
- [`../popover/src/icons/registry.generated.ts`](../popover/src/icons/registry.generated.ts) — Auto-generated TypeScript icon registry consumed by `<rk-icon>`.
- [`../popover/src/icons/icon.ts`](../popover/src/icons/icon.ts) — `<rk-icon>` custom element and helper functions (`icon()`, `setIcon()`, `iconLabel()`).

## Dynamic UI Recoloring

Every normalized icon consists of two layers:

```xml
<g class="rk-duo" fill="currentColor" opacity="0.2">...</g>
<g class="rk-line" fill="currentColor">...</g>
```

Because `<rk-icon>` renders inline `<svg>` in the light DOM, any icon can be recolored from CSS at the theme, card, button, or individual element level:

```css
.my-element {
  --icon-primary: var(--text);               /* Primary stroke / outline color */
  --icon-secondary: var(--accent-secondary); /* Duotone fill tint color */
  --icon-secondary-opacity: 0.42;            /* Duotone fill opacity */
}
```

## Authoring Custom Icons

When adding a new custom icon to `src/custom/<name>.svg`:

1. Use `viewBox="0 0 256 256"` with ~24 units of outer padding.
2. Put secondary tint shapes inside `<g class="rk-duo" fill="currentColor" opacity="0.2">`.
3. Put primary strokes inside `<g class="rk-line" fill="none" stroke="currentColor" stroke-width="16" stroke-linecap="round" stroke-linejoin="round">` (or `fill="currentColor"` for filled shapes).
4. Never hard-code hex/RGB colors in source SVGs; always use `currentColor` or `none`.
5. Register `"<name>": "custom:<name>"` in `icons.config.json` and run `npm run icons`.

