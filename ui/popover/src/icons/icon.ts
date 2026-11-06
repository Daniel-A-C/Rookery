// Rookery icon runtime.
//
// Usage in HTML:   <rk-icon name="koi"></rk-icon>
// Usage in TS:     icon("koi")  /  setIcon(el, "koi")  /  iconLabel(button, "play", "Resume")
//
// Icons render as inline SVG (light DOM), so they inherit `color` and the theme's CSS custom
// properties:
//   --icon-primary            outline colour        (default: currentColor)
//   --icon-secondary          duotone tint colour   (default: the realm's --accent-secondary)
//   --icon-secondary-opacity  duotone tint opacity
//
// Unknown names (e.g. emoji from older third-party rookery.toml manifests) are first mapped
// through EMOJI_ALIASES and otherwise rendered as plain text so nothing ever disappears.

import { ICONS, ICON_VIEWBOX, type IconName } from "./registry.generated";

export type { IconName };

/** Legacy emoji -> icon mapping so old manifests still match the visual language. */
const EMOJI_ALIASES: Record<string, IconName> = {
  "✨": "sparkle",
  "✦": "star-four",
  "🎨": "palette",
  "🌠": "shooting-star",
  "🌾": "grains",
  "✂": "scissors",
  "🌱": "plant",
  "🌿": "leaf",
  "🍃": "leaf",
  "🌲": "tree-evergreen",
  "🐦": "bird",
  "🦅": "bird",
  "🎏": "koi",
  "🐟": "koi",
  "🪷": "flower-lotus",
  "🌸": "flower-lotus",
  "💧": "drop",
  "🦐": "pellets",
  "🪱": "pellets",
  "🌀": "spiral",
  "💥": "nova",
  "💫": "nova",
  "⚡": "lightning",
  "⚛": "atom",
  "🌌": "nebula",
  "🛰": "satellite",
  "☀": "sun",
  "🛸": "hab-ring",
  "🚀": "rocket-launch",
  "⏳": "hourglass",
  "🧭": "compass",
  "⚙": "settings",
  "◀": "caret-left",
  "▶": "caret-right",
  "⏸": "pause",
  "📸": "camera",
  "🎞": "film-strip",
  "🎥": "video-camera",
  "📦": "package",
  "⏻": "power",
};

export function isIconName(value: string): value is IconName {
  return Object.prototype.hasOwnProperty.call(ICONS, value);
}

/** Resolve an icon name or legacy emoji to a registry name, or null if it has no icon. */
export function resolveIcon(value: string | null | undefined): IconName | null {
  if (!value) return null;
  const v = value.trim();
  if (isIconName(v)) return v;
  const stripped = v.replace(/\uFE0F/g, "");
  return EMOJI_ALIASES[stripped] ?? null;
}

function svgMarkup(name: IconName): string {
  return `<svg class="rk-icon" viewBox="${ICON_VIEWBOX}" aria-hidden="true" focusable="false">${ICONS[name]}</svg>`;
}

export class RkIcon extends HTMLElement {
  static observedAttributes = ["name", "label"];
  private rendered: string | null = null;

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.isConnected) this.render();
  }

  private render(): void {
    const raw = this.getAttribute("name") ?? "";
    const label = this.getAttribute("label");
    if (label) {
      this.setAttribute("role", "img");
      this.setAttribute("aria-label", label);
      this.removeAttribute("aria-hidden");
    } else {
      this.removeAttribute("role");
      this.removeAttribute("aria-label");
      this.setAttribute("aria-hidden", "true");
    }
    if (raw === this.rendered) return;
    this.rendered = raw;
    const name = resolveIcon(raw);
    this.classList.toggle("rk-icon-text", !name);
    if (name) this.innerHTML = svgMarkup(name);
    else this.textContent = raw;
  }
}

if (!customElements.get("rk-icon")) customElements.define("rk-icon", RkIcon);

/** Create an `<rk-icon>` element. */
export function icon(name: IconName | string, className?: string): HTMLElement {
  const el = document.createElement("rk-icon");
  el.setAttribute("name", name);
  if (className) el.className = className;
  return el;
}

/** Point an existing `<rk-icon>` at a different icon. */
export function setIcon(el: Element, name: IconName | string): void {
  el.setAttribute("name", name);
}

/** Replace an element's content with `[icon] label`. */
export function iconLabel(el: HTMLElement, name: IconName | string | null | undefined, label: string): void {
  const children: Node[] = [];
  if (name) children.push(icon(name));
  const span = document.createElement("span");
  span.textContent = label;
  children.push(span);
  el.replaceChildren(...children);
}

