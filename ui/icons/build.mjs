#!/usr/bin/env node
// Rookery icon pipeline.
//
//   node ui/icons/build.mjs          -> build (registry + SVG/PNG exports + preview sheet)
//   node ui/icons/build.mjs --sync   -> re-vendor Phosphor sources from node_modules, then build
//   node ui/icons/build.mjs --no-png -> skip raster exports (fast; no native deps needed)
//
// Every icon, whatever its origin, is normalised into the same two-layer structure on a
// 256x256 grid so it can be recoloured from CSS:
//
//   <g class="rk-duo"  fill="currentColor" opacity="0.2">  ...secondary / tint layer... </g>
//   <g class="rk-line" ...currentColor...>                ...primary outline layer... </g>
//
// In the app, `.rk-icon` sets `color` (primary) and `.rk-icon .rk-duo` overrides `color` and
// `opacity` from CSS custom properties, so a single SVG adapts to every realm theme.

import { readFileSync, writeFileSync, mkdirSync, readdirSync, rmSync, copyFileSync, existsSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const REPO = join(ROOT, "..", "..");
const SRC_PHOSPHOR = join(ROOT, "src", "phosphor");
const SRC_CUSTOM = join(ROOT, "src", "custom");
const EXPORT_DIR = join(ROOT, "export");
const REGISTRY_TS = join(REPO, "ui", "popover", "src", "icons", "registry.generated.ts");
const PHOSPHOR_PKG = join(REPO, "node_modules", "@phosphor-icons", "core");

const args = new Set(process.argv.slice(2));
const config = JSON.parse(readFileSync(join(ROOT, "icons.config.json"), "utf8"));
const entries = Object.entries(config.icons);

const LINE_FILLED = `<g class="rk-line" fill="currentColor">`;
const DUO_OPEN = `<g class="rk-duo" fill="currentColor" opacity="0.2">`;

function parseSource(spec) {
  const [kind, name] = spec.split(":");
  if (!name || (kind !== "phosphor" && kind !== "custom")) {
    throw new Error(`Invalid icon source "${spec}" (expected phosphor:<name> or custom:<name>)`);
  }
  return { kind, name };
}

// ---------------------------------------------------------------------------------------------
// 1. Sync: vendor the referenced Phosphor duotone SVGs into the repo.
// ---------------------------------------------------------------------------------------------
function sync() {
  if (!existsSync(PHOSPHOR_PKG)) {
    throw new Error("@phosphor-icons/core is not installed. Run `npm install` at the repo root first.");
  }
  rmSync(SRC_PHOSPHOR, { recursive: true, force: true });
  mkdirSync(SRC_PHOSPHOR, { recursive: true });
  const wanted = new Set(entries.map(([, s]) => parseSource(s)).filter((s) => s.kind === "phosphor").map((s) => s.name));
  for (const name of wanted) {
    const from = join(PHOSPHOR_PKG, "assets", "duotone", `${name}-duotone.svg`);
    if (!existsSync(from)) throw new Error(`Phosphor has no icon named "${name}"`);
    copyFileSync(from, join(SRC_PHOSPHOR, `${name}.svg`));
  }
  copyFileSync(join(PHOSPHOR_PKG, "LICENSE"), join(SRC_PHOSPHOR, "LICENSE"));
  const version = JSON.parse(readFileSync(join(PHOSPHOR_PKG, "package.json"), "utf8")).version;
  writeFileSync(join(SRC_PHOSPHOR, "VERSION"), `@phosphor-icons/core ${version} (duotone weight)\n`);
  console.log(`synced ${wanted.size} Phosphor icons (v${version})`);
}

// ---------------------------------------------------------------------------------------------
// 2. Normalise sources into the shared two-layer markup.
// ---------------------------------------------------------------------------------------------
function innerOf(svg, file) {
  const m = svg.match(/<svg\b[^>]*>([\s\S]*)<\/svg>/);
  if (!m) throw new Error(`${file}: not an <svg> document`);
  const open = svg.match(/<svg\b[^>]*>/)[0];
  if (!/viewBox="0 0 256 256"/.test(open)) throw new Error(`${file}: viewBox must be "0 0 256 256"`);
  return m[1];
}

function normalisePhosphor(name) {
  const file = join(SRC_PHOSPHOR, `${name}.svg`);
  if (!existsSync(file)) throw new Error(`Missing vendored Phosphor icon ${name}. Run \`npm run icons:sync\`.`);
  const inner = innerOf(readFileSync(file, "utf8"), file);
  const elements = inner.match(/<(path|circle|rect|ellipse|line|polyline|polygon)\b[^>]*\/>/g) ?? [];
  const leftover = elements.reduce((s, e) => s.replace(e, ""), inner).trim();
  if (leftover) throw new Error(`${file}: unexpected markup ${leftover.slice(0, 60)}`);
  const duo = [];
  const line = [];
  for (const el of elements) {
    if (/\sopacity="0\.2"/.test(el)) duo.push(el.replace(/\sopacity="0\.2"/, ""));
    else line.push(el);
  }
  return (duo.length ? `${DUO_OPEN}${duo.join("")}</g>` : "") + `${LINE_FILLED}${line.join("")}</g>`;
}

function normaliseCustom(name) {
  const file = join(SRC_CUSTOM, `${name}.svg`);
  if (!existsSync(file)) throw new Error(`Missing custom icon ${relative(REPO, file)}`);
  const inner = innerOf(readFileSync(file, "utf8"), file);
  if (!/class="rk-line"/.test(inner)) throw new Error(`${file}: needs a <g class="rk-line"> primary layer`);
  const colours = inner.match(/(fill|stroke|color)="([^"]+)"/g) ?? [];
  for (const c of colours) {
    if (!/"(currentColor|none)"/.test(c)) throw new Error(`${file}: hard-coded colour ${c}; use currentColor`);
  }
  // Collapse authoring whitespace so the registry stays compact.
  return inner.replace(/>\s+</g, "><").replace(/\s{2,}/g, " ").trim();
}

if (args.has("--sync")) sync();

const icons = entries.map(([name, spec]) => {
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)) throw new Error(`Icon name "${name}" must be kebab-case`);
  const src = parseSource(spec);
  const markup = src.kind === "phosphor" ? normalisePhosphor(src.name) : normaliseCustom(src.name);
  return { name, source: spec, markup };
});

// ---------------------------------------------------------------------------------------------
// 3. Outputs.
// ---------------------------------------------------------------------------------------------
const standalone = (markup, attrs = "") =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256"${attrs}>${markup}</svg>\n`;

function themed(markup, { primary, secondary, secondaryOpacity }) {
  return markup
    .replace(/class="rk-duo" fill="currentColor" opacity="[\d.]+"/g, `class="rk-duo" fill="${secondary}" opacity="${secondaryOpacity}"`)
    .replace(/currentColor/g, primary);
}

function writeRegistry() {
  mkdirSync(dirname(REGISTRY_TS), { recursive: true });
  const body = icons.map((i) => `  ${JSON.stringify(i.name)}: ${JSON.stringify(i.markup)},`).join("\n");
  writeFileSync(
    REGISTRY_TS,
    `// AUTO-GENERATED by ui/icons/build.mjs from ui/icons/icons.config.json. Do not edit by hand.\n` +
      `// Run \`npm run icons\` at the repo root to regenerate.\n\n` +
      `export const ICON_VIEWBOX = "0 0 256 256";\n\n` +
      `export const ICONS = {\n${body}\n} as const;\n\n` +
      `export type IconName = keyof typeof ICONS;\n`,
  );
}

function writeSvgExports() {
  const dir = join(EXPORT_DIR, "svg");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (const i of icons) writeFileSync(join(dir, `${i.name}.svg`), standalone(i.markup));
}

function previewSheet() {
  const cols = 10;
  const cell = 76;
  const pad = 16;
  const themes = Object.entries(config.export.previewThemes);
  const rows = Math.ceil(icons.length / cols);
  const blockH = rows * cell + 40;
  const width = cols * cell + pad * 2;
  const height = themes.length * blockH + pad;
  let out = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`;
  themes.forEach(([label, t], ti) => {
    const y0 = ti * blockH;
    out += `<rect x="0" y="${y0}" width="${width}" height="${blockH}" fill="${t.background}"/>`;
    out += `<text x="${pad}" y="${y0 + 24}" fill="${t.secondary}" font-family="Menlo, monospace" font-size="13">${label}</text>`;
    icons.forEach((icon, idx) => {
      const x = pad + (idx % cols) * cell + 18;
      const y = y0 + 34 + Math.floor(idx / cols) * cell;
      const markup = themed(icon.markup, { primary: t.text, secondary: t.secondary, secondaryOpacity: 0.45 });
      out += `<svg x="${x}" y="${y}" width="40" height="40" viewBox="0 0 256 256">${markup}</svg>`;
      out += `<text x="${x + 20}" y="${y + 56}" fill="${t.text}" fill-opacity="0.6" font-family="Menlo, monospace" font-size="8" text-anchor="middle">${icon.name}</text>`;
    });
  });
  return out + `</svg>\n`;
}

async function writePngExports() {
  let Resvg;
  try {
    ({ Resvg } = await import("@resvg/resvg-js"));
  } catch {
    console.warn("@resvg/resvg-js not installed; skipping PNG exports (run `npm install` at the repo root).");
    return false;
  }
  const pngRoot = join(EXPORT_DIR, "png");
  rmSync(pngRoot, { recursive: true, force: true });
  for (const [theme, colours] of Object.entries(config.export.themes)) {
    for (const size of config.export.sizes) {
      const dir = join(pngRoot, theme, String(size));
      mkdirSync(dir, { recursive: true });
      for (const i of icons) {
        const svg = standalone(themed(i.markup, colours));
        const png = new Resvg(svg, { fitTo: { mode: "width", value: size }, font: { loadSystemFonts: false } })
          .render()
          .asPng();
        writeFileSync(join(dir, `${i.name}.png`), png);
      }
    }
  }
  const sheet = previewSheet();
  writeFileSync(join(EXPORT_DIR, "preview.svg"), sheet);
  writeFileSync(
    join(EXPORT_DIR, "preview.png"),
    new Resvg(sheet, { fitTo: { mode: "zoom", value: 2 }, font: { loadSystemFonts: true } }).render().asPng(),
  );
  return true;
}

writeRegistry();
writeSvgExports();
const pngs = args.has("--no-png") ? false : await writePngExports();
console.log(
  `built ${icons.length} icons -> ${relative(REPO, REGISTRY_TS)}, ui/icons/export/svg` +
    (pngs ? `, ui/icons/export/png (${Object.keys(config.export.themes).join("/")} @ ${config.export.sizes.join("/")}px)` : ""),
);
