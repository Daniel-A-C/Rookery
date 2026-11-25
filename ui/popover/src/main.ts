import {
  api,
  AppManifest,
  DisplayStatus,
  FrameBudget,
  HostStatus,
  ScheduleMode,
  Settings,
  SettingsPatch,
} from "./api";
import { updater } from "./updater";
import { icon, iconLabel, setIcon, type IconName } from "./icons/icon";

const el = <T extends HTMLElement>(id: string): T => {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing #${id}`);
  return node as T;
};

// Root & Header Elements (Option C Realm Selector)
const popoverRoot = el<HTMLElement>("popover-root");
const btnRealmSelector = el<HTMLButtonElement>("btn-realm-selector");
const brandGlyph = el<HTMLElement>("brand-glyph");
const brandTitle = el<HTMLHeadingElement>("brand-title");
const realmSelectorCaret = el<HTMLElement>("realm-selector-caret");
const powerBadge = el<HTMLSpanElement>("power-badge");
const appTagline = el<HTMLElement>("app-tagline");
const realmCountPill = el<HTMLSpanElement>("realm-count-pill");

// View Toggle (Realm HUD vs Realm Library vs Settings Menu)
const btnToggleSettings = el<HTMLButtonElement>("btn-toggle-settings");
const settingsToggleIcon = el<HTMLElement>("settings-toggle-icon");
const settingsToggleLabel = el<HTMLSpanElement>("settings-toggle-label");
const viewRealm = el<HTMLDivElement>("view-realm");
const viewLibrary = el<HTMLDivElement>("view-library");
const viewSettings = el<HTMLDivElement>("view-settings");

// Library View Elements
const inputLibrarySearch = el<HTMLInputElement>("input-library-search");
const btnReloadCatalog = el<HTMLButtonElement>("btn-reload-catalog");
const libraryFilterBar = el<HTMLElement>("library-filter-bar");
const libraryGrid = el<HTMLUListElement>("library-grid");
const filterPills = libraryFilterBar.querySelectorAll<HTMLButtonElement>(".filter-pill");

// Settings Sub-Tabs
type SettingsTab = "general" | "controls" | "share" | "debug";
const settingsSubnav = document.querySelector<HTMLElement>(".settings-subnav");
const settingsPanels: Record<SettingsTab, HTMLElement> = {
  general: el<HTMLElement>("settings-panel-general"),
  controls: el<HTMLElement>("settings-panel-controls"),
  share: el<HTMLElement>("settings-panel-share"),
  debug: el<HTMLElement>("settings-panel-debug"),
};
const settingsNavBtns = Array.from(
  document.querySelectorAll<HTMLButtonElement>(".settings-nav-btn"),
);
const debugSettingsNavBtn = el<HTMLButtonElement>("tab-settings-debug");
let activeSettingsTab: SettingsTab = "general";

function setActiveSettingsTab(tab: SettingsTab, focusTab = false): void {
  cancelRecordingHotkey();
  clearResetConfirm();
  const resolvedTab: SettingsTab =
    tab === "debug" && debugSettingsNavBtn.hidden ? "general" : tab;
  activeSettingsTab = resolvedTab;

  for (const btn of settingsNavBtns) {
    const isSelected = btn.getAttribute("data-settings-tab") === resolvedTab;
    btn.setAttribute("aria-selected", String(isSelected));
    btn.tabIndex = isSelected ? 0 : -1;
    if (isSelected) {
      btn.scrollIntoView({ block: "nearest", inline: "nearest" });
      if (focusTab) btn.focus();
    }
  }
  for (const [key, panel] of Object.entries(settingsPanels)) {
    panel.hidden = key !== resolvedTab;
  }
  viewSettings.scrollTop = 0;
}

for (const navBtn of settingsNavBtns) {
  navBtn.addEventListener("click", () => {
    const targetTab = (navBtn.getAttribute("data-settings-tab") || "general") as SettingsTab;
    setActiveSettingsTab(targetTab);
  });
}

settingsSubnav?.addEventListener("keydown", (ev) => {
  const visibleTabs = settingsNavBtns.filter((btn) => !btn.hidden);
  if (visibleTabs.length === 0) return;
  const currentIdx = visibleTabs.indexOf(document.activeElement as HTMLButtonElement);
  if (currentIdx === -1) return;

  let nextIdx: number | null = null;
  if (ev.key === "ArrowRight") {
    nextIdx = (currentIdx + 1) % visibleTabs.length;
  } else if (ev.key === "ArrowLeft") {
    nextIdx = (currentIdx - 1 + visibleTabs.length) % visibleTabs.length;
  } else if (ev.key === "Home") {
    nextIdx = 0;
  } else if (ev.key === "End") {
    nextIdx = visibleTabs.length - 1;
  }

  if (nextIdx !== null) {
    ev.preventDefault();
    const nextTab = (visibleTabs[nextIdx].getAttribute("data-settings-tab") ||
      "general") as SettingsTab;
    setActiveSettingsTab(nextTab, true);
  }
});

function syncSettingsForActiveApp(appId: string): void {
  const isLivingRookery = appId === "living-rookery";
  debugSettingsNavBtn.hidden = !isLivingRookery;
  if (!isLivingRookery && activeSettingsTab === "debug") {
    setActiveSettingsTab("general");
  }

  const supportsSeedExport =
    appId === "living-rookery" || appId === "orbital-haven";
  btnCopySeed.disabled = !supportsSeedExport;
  btnCopyState.disabled = !supportsSeedExport;
  btnCopySeed.title = supportsSeedExport
    ? "Copy shareable World Seed URI"
    : "World seed export is available for Living Rookery and Orbital Haven";
  btnCopyState.title = supportsSeedExport
    ? "Copy full World State snapshot"
    : "World state export is available for Living Rookery and Orbital Haven";
}

const DEFAULT_HOTKEY = "CommandOrControl+Alt+Shift+P";
const DEFAULT_POPOVER_HOTKEY = "CommandOrControl+Alt+Shift+M";
const FAVORITE_HOTKEY_SLOTS = 9;

function defaultFavoriteHotkeys(): string[] {
  return Array.from(
    { length: FAVORITE_HOTKEY_SLOTS },
    (_, i) => `CommandOrControl+Alt+Shift+${i + 1}`,
  );
}

const IS_MAC = /Mac|iPhone|iPad|iPod/i.test(
  navigator.platform || navigator.userAgent,
);

function formatShortcutForDisplay(shortcut: string): string {
  const parts = shortcut
    .split("+")
    .map((p) => p.trim())
    .filter(Boolean);
  if (IS_MAC) {
    return parts
      .map((p) => {
        switch (p.toLowerCase()) {
          case "commandorcontrol":
          case "cmdorctrl":
          case "command":
          case "cmd":
          case "super":
          case "meta":
            return "⌘";
          case "alt":
          case "option":
            return "⌥";
          case "shift":
            return "⇧";
          case "control":
          case "ctrl":
            return "⌃";
          case "space":
            return "Space";
          default:
            return p.length === 1 ? p.toUpperCase() : p;
        }
      })
      .join("");
  }
  return parts
    .map((p) => {
      switch (p.toLowerCase()) {
        case "commandorcontrol":
        case "cmdorctrl":
          return "Ctrl";
        default:
          return p.length === 1 ? p.toUpperCase() : p;
      }
    })
    .join("+");
}

type ActiveView = "realm" | "library" | "settings";
let activeView: ActiveView = "realm";
let activeLibraryFilter = "all";
let librarySearchQuery = "";
let favoriteApps: string[] = [];
let favoriteHotkeys: string[] = defaultFavoriteHotkeys();

function setActiveView(view: ActiveView): void {
  cancelRecordingHotkey();
  if (view !== "settings") {
    clearResetConfirm();
  }
  activeView = view;
  const isLibrary = view === "library";
  const isSettings = view === "settings";

  viewRealm.hidden = view !== "realm";
  viewLibrary.hidden = !isLibrary;
  viewSettings.hidden = !isSettings;

  btnRealmSelector.classList.toggle("active", isLibrary);
  btnRealmSelector.setAttribute("aria-expanded", String(isLibrary));
  setIcon(realmSelectorCaret, isLibrary ? "caret-up" : "caret-down");

  btnToggleSettings.classList.toggle("active", isSettings);
  btnToggleSettings.setAttribute("aria-expanded", String(isSettings));
  if (isSettings) {
    setIcon(settingsToggleIcon, "caret-left");
    settingsToggleLabel.textContent = "Realm";
    viewSettings.scrollTop = 0;
  } else {
    setIcon(settingsToggleIcon, "settings");
    settingsToggleLabel.textContent = "Settings";
  }

  if (isLibrary) {
    renderLibraryGrid();
    inputLibrarySearch.focus();
  }
}

btnRealmSelector.addEventListener("click", () => {
  setActiveView(activeView === "library" ? "realm" : "library");
});

btnToggleSettings.addEventListener("click", () => {
  setActiveView(activeView === "settings" ? "realm" : "settings");
});

// Realm Cards Container
const realmCardsContainer = el<HTMLElement>("realm-cards-container");
const cards: Record<string, HTMLElement> = {};

interface AppThemeMeta {
  title: string;
  tagline: string;
  bannerTitle?: string;
  /** Icon name (see ui/icons/icons.config.json); legacy emoji still render as text. */
  glyph: IconName | string;
  /** Icon flanking the realm card banner (mirrored on the right). */
  ornament: IconName | string;
  bg?: string;
  card?: string;
  borderOuter?: string;
  accent?: string;
  accentSecondary?: string;
  crestGradient?: [string, string];
}

const BUILTIN_CSS_THEMES = new Set([
  "living-rookery",
  "koi-pond",
  "cosmic-nebula",
  "orbital-haven",
  "aurora-waves",
  "fireflies",
]);

const CUSTOM_THEME_VARS = [
  "--bg",
  "--bg-pattern",
  "--card",
  "--card-inner",
  "--border-outer",
  "--border-inner",
  "--border",
  "--text",
  "--muted",
  "--accent",
  "--accent-secondary",
  "--accent-glow",
  "--accent-active",
  "--btn-surface",
  "--btn-edge",
  "--crest-bg",
  "--ok",
  "--scrollbar-thumb",
  "--scrollbar-track",
];

type Rgb = readonly [number, number, number];

interface GothicMenuPalette {
  bg: Rgb;
  card: Rgb;
  cardInner: Rgb;
  borderOuter: Rgb;
  accent: Rgb;
  accentSecondary: Rgb;
  crestA: Rgb;
  crestB: Rgb;
  text: Rgb;
  muted: Rgb;
  btnTop: Rgb;
  btnBottom: Rgb;
  btnEdge: Rgb;
}

const GOTHIC_MENU_PALETTES: readonly GothicMenuPalette[] = [
  // 0: 1989 Furst Amber & Obsidian
  {
    bg: [12, 10, 14],
    card: [22, 18, 26],
    cardInner: [14, 11, 18],
    borderOuter: [217, 119, 6],
    accent: [245, 158, 11],
    accentSecondary: [251, 191, 36],
    crestA: [180, 83, 9],
    crestB: [120, 53, 15],
    text: [254, 243, 199],
    muted: [184, 168, 148],
    btnTop: [38, 30, 22],
    btnBottom: [24, 19, 15],
    btnEdge: [10, 8, 12],
  },
  // 1: Returns Ice & Gargoyle Slate
  {
    bg: [8, 14, 24],
    card: [15, 25, 42],
    cardInner: [10, 17, 30],
    borderOuter: [56, 189, 248],
    accent: [125, 211, 252],
    accentSecondary: [253, 230, 138],
    crestA: [2, 132, 199],
    crestB: [30, 58, 95],
    text: [224, 242, 254],
    muted: [148, 176, 199],
    btnTop: [22, 38, 61],
    btnBottom: [14, 25, 42],
    btnEdge: [6, 11, 19],
  },
  // 2: Crimson Noir & Brass
  {
    bg: [18, 8, 10],
    card: [31, 14, 18],
    cardInner: [21, 9, 12],
    borderOuter: [225, 29, 72],
    accent: [244, 63, 94],
    accentSecondary: [251, 191, 36],
    crestA: [190, 18, 60],
    crestB: [124, 45, 18],
    text: [255, 228, 230],
    muted: [196, 153, 160],
    btnTop: [46, 20, 26],
    btnBottom: [30, 12, 16],
    btnEdge: [14, 6, 8],
  },
  // 3: Axis Chemical Verdigris
  {
    bg: [7, 18, 14],
    card: [14, 33, 26],
    cardInner: [9, 22, 17],
    borderOuter: [16, 185, 129],
    accent: [52, 211, 153],
    accentSecondary: [163, 230, 53],
    crestA: [5, 150, 105],
    crestB: [20, 83, 45],
    text: [236, 253, 245],
    muted: [142, 190, 168],
    btnTop: [20, 48, 37],
    btnBottom: [12, 31, 24],
    btnEdge: [5, 14, 10],
  },
  // 4: Phantasm Violet & Fog
  {
    bg: [13, 8, 22],
    card: [25, 16, 41],
    cardInner: [16, 10, 28],
    borderOuter: [147, 51, 234],
    accent: [192, 132, 252],
    accentSecondary: [244, 114, 182],
    crestA: [126, 34, 206],
    crestB: [76, 29, 149],
    text: [243, 232, 255],
    muted: [176, 155, 199],
    btnTop: [39, 25, 64],
    btnBottom: [25, 15, 43],
    btnEdge: [11, 6, 19],
  },
  // 5: Clocktower Sepia & Bronze
  {
    bg: [17, 13, 9],
    card: [32, 24, 17],
    cardInner: [21, 16, 11],
    borderOuter: [180, 83, 9],
    accent: [245, 158, 11],
    accentSecondary: [45, 212, 191],
    crestA: [146, 64, 14],
    crestB: [69, 26, 3],
    text: [254, 243, 199],
    muted: [191, 168, 142],
    btnTop: [48, 36, 25],
    btnBottom: [31, 23, 16],
    btnEdge: [14, 10, 7],
  },
  // 6: Abyssal Cobalt & Neon
  {
    bg: [6, 11, 24],
    card: [13, 22, 46],
    cardInner: [8, 14, 31],
    borderOuter: [2, 132, 199],
    accent: [34, 211, 238],
    accentSecondary: [251, 146, 60],
    crestA: [3, 105, 161],
    crestB: [30, 58, 138],
    text: [224, 242, 254],
    muted: [140, 168, 204],
    btnTop: [19, 34, 69],
    btnBottom: [11, 21, 46],
    btnEdge: [5, 9, 20],
  },
  // 7: Monochrome Noir & Ivory
  {
    bg: [9, 9, 11],
    card: [22, 22, 26],
    cardInner: [14, 14, 17],
    borderOuter: [113, 113, 122],
    accent: [228, 228, 231],
    accentSecondary: [248, 113, 113],
    crestA: [82, 82, 91],
    crestB: [39, 39, 42],
    text: [250, 250, 250],
    muted: [161, 161, 170],
    btnTop: [39, 39, 44],
    btnBottom: [24, 24, 28],
    btnEdge: [9, 9, 11],
  },
];

function lerpRgb(a: Rgb, b: Rgb, t: number): Rgb {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

function rgbCss(c: Rgb): string {
  return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
}

function rgbaCss(c: Rgb, alpha: number): string {
  return `rgba(${c[0]}, ${c[1]}, ${c[2]}, ${alpha})`;
}

let lastGothicThemeKey = "";

function applyGothicPaletteTheme(palette: number, blend: number): void {
  const count = GOTHIC_MENU_PALETTES.length;
  const idxA = ((Math.floor(palette) % count) + count) % count;
  const idxB = (idxA + 1) % count;
  const t = Math.max(0, Math.min(1, Number.isFinite(blend) ? blend : 0));
  const key = `${idxA}:${Math.round(t * 255)}`;
  if (key === lastGothicThemeKey) return;
  lastGothicThemeKey = key;

  const a = GOTHIC_MENU_PALETTES[idxA];
  const b = GOTHIC_MENU_PALETTES[idxB];
  const bg = lerpRgb(a.bg, b.bg, t);
  const card = lerpRgb(a.card, b.card, t);
  const cardInner = lerpRgb(a.cardInner, b.cardInner, t);
  const borderOuter = lerpRgb(a.borderOuter, b.borderOuter, t);
  const accent = lerpRgb(a.accent, b.accent, t);
  const accentSecondary = lerpRgb(a.accentSecondary, b.accentSecondary, t);
  const crestA = lerpRgb(a.crestA, b.crestA, t);
  const crestB = lerpRgb(a.crestB, b.crestB, t);
  const text = lerpRgb(a.text, b.text, t);
  const muted = lerpRgb(a.muted, b.muted, t);
  const btnTop = lerpRgb(a.btnTop, b.btnTop, t);
  const btnBottom = lerpRgb(a.btnBottom, b.btnBottom, t);
  const btnEdge = lerpRgb(a.btnEdge, b.btnEdge, t);

  const s = popoverRoot.style;
  s.setProperty("--bg", rgbCss(bg));
  s.setProperty(
    "--bg-pattern",
    `radial-gradient(circle at 20% 15%, ${rgbaCss(accent, 0.16)}, transparent 55%), radial-gradient(circle at 80% 85%, ${rgbaCss(accentSecondary, 0.12)}, transparent 55%)`,
  );
  s.setProperty("--card", rgbaCss(card, 0.94));
  s.setProperty("--card-inner", rgbaCss(cardInner, 0.88));
  s.setProperty("--border-outer", rgbCss(borderOuter));
  s.setProperty("--border-inner", rgbaCss(accentSecondary, 0.3));
  s.setProperty("--border", rgbaCss(accent, 0.25));
  s.setProperty("--text", rgbCss(text));
  s.setProperty("--muted", rgbCss(muted));
  s.setProperty("--accent", rgbCss(accent));
  s.setProperty("--accent-secondary", rgbCss(accentSecondary));
  s.setProperty("--accent-glow", rgbaCss(accent, 0.25));
  s.setProperty("--accent-active", rgbCss(accentSecondary));
  s.setProperty("--btn-surface", `linear-gradient(180deg, ${rgbCss(btnTop)} 0%, ${rgbCss(btnBottom)} 100%)`);
  s.setProperty("--btn-edge", rgbCss(btnEdge));
  s.setProperty("--crest-bg", `linear-gradient(135deg, ${rgbCss(crestA)}, ${rgbCss(crestB)})`);
  s.setProperty("--ok", rgbCss(accent));
  s.setProperty("--scrollbar-thumb", rgbCss(borderOuter));
  s.setProperty("--scrollbar-track", rgbCss(bg));
}

const appThemes: Record<string, AppThemeMeta> = {
  "living-rookery": {
    title: "The Living Rookery",
    tagline: "Woodland Sanctuary · ASCII Realm",
    bannerTitle: "Sylvan Sanctuary Chronicle",
    glyph: "tree-evergreen",
    ornament: "leaf",
  },
  "koi-pond": {
    title: "Tranquil Koi Pond",
    tagline: "Imperial Lotus Sanctuary",
    bannerTitle: "Imperial Pond Scroll",
    glyph: "koi",
    ornament: "flower-lotus",
  },
  "cosmic-nebula": {
    title: "Cosmic Nebula",
    tagline: "Stellar Nursery · Proto-Star Forge",
    bannerTitle: "Astral Nursery Observatory",
    glyph: "nebula",
    ornament: "star-four",
  },
  "orbital-haven": {
    title: "Orbital Haven",
    tagline: "Aegis-VII Orbital Command",
    bannerTitle: "Orbital Telemetry & Ops",
    glyph: "satellite",
    ornament: "hexagon",
  },
  "shinjuku-rain": {
    title: "Shinjuku Rain",
    tagline: "Kabukicho Downpour · ASCII Realm",
    bannerTitle: "Shinjuku Street Weather & Transit",
    glyph: "drop",
    ornament: "sparkle",
  },
  "gothic-metropolis": {
    title: "Gothic Metropolis",
    tagline: "Noir Spire Citadel · Isometric 3D Realm",
    bannerTitle: "Metropolis Transit & Spire Telemetry",
    glyph: "diamond",
    ornament: "star-four",
  },
  "aurora-waves": {
    title: "Aurora Waves",
    tagline: "Borealis Sky Observatory",
    bannerTitle: "Polar Magnetosphere Telemetry",
    glyph: "shooting-star",
    ornament: "star-four",
  },
  fireflies: {
    title: "Twilight Fireflies",
    tagline: "Enchanted Gloaming Glade",
    bannerTitle: "Bioluminescent Meadow Deck",
    glyph: "sparkle",
    ornament: "sparkle",
  },
};

let lastAppliedThemeApp: string | null = null;
let lastStatusCacheWriteMs = 0;
let lastDisplaysSignature = "";
let lastTreesSignature = "";

/**
 * Coalesces high-frequency async calls (such as slider `input` events) so at most one
 * IPC request is in flight at a time, trailing with the most recent value.
 */
function createCoalescedCaller<T>(fn: (arg: T) => Promise<void>): (arg: T) => void {
  let inFlight = false;
  let hasPending = false;
  let latestArg: T;

  const pump = async () => {
    if (inFlight || !hasPending) return;
    inFlight = true;
    hasPending = false;
    const arg = latestArg;
    try {
      await fn(arg);
    } finally {
      inFlight = false;
      if (hasPending) {
        void pump();
      }
    }
  };

  return (arg: T) => {
    latestArg = arg;
    hasPending = true;
    void pump();
  };
}

/**
 * Provides immediate tactile visual feedback on a button and displays an optimistic
 * notification before awaiting the backend IPC call.
 */
function runActionWithFeedback(
  btn: HTMLButtonElement,
  iconName: IconName | string,
  optimisticMsg: string | null,
  ipcCall: () => Promise<HostStatus>,
  resolvedMsg?: (s: HostStatus) => string | null,
): void {
  btn.classList.add("is-pressed");
  window.setTimeout(() => btn.classList.remove("is-pressed"), 140);

  if (optimisticMsg) {
    showNotification(iconName, optimisticMsg);
  }

  ipcCall()
    .then((s) => {
      renderStatus(s);
      if (resolvedMsg) {
        const msg = resolvedMsg(s);
        if (msg) showNotification(iconName, msg);
      }
    })
    .catch((err) => {
      console.error("action error:", err);
    });
}

function applyThemeForApp(appId: string, force = false): void {
  if (!force && lastAppliedThemeApp === appId) {
    return;
  }
  lastAppliedThemeApp = appId;
  lastGothicThemeKey = "";
  popoverRoot.setAttribute("data-theme", appId);
  const meta = appThemes[appId];

  // Clear any previous custom inline theme variables first
  for (const prop of CUSTOM_THEME_VARS) {
    popoverRoot.style.removeProperty(prop);
  }

  if (meta) {
    setIcon(brandGlyph, meta.glyph);
    brandTitle.textContent = meta.title;
    appTagline.textContent = meta.tagline;

    // For custom apps not in BUILTIN_CSS_THEMES, apply declarative [theme] colors from rookery.toml
    if (!BUILTIN_CSS_THEMES.has(appId)) {
      if (meta.bg) {
        popoverRoot.style.setProperty("--bg", meta.bg);
        popoverRoot.style.setProperty("--scrollbar-track", meta.bg);
      }
      if (meta.card) popoverRoot.style.setProperty("--card", meta.card);
      if (meta.borderOuter) {
        popoverRoot.style.setProperty("--border-outer", meta.borderOuter);
        popoverRoot.style.setProperty("--scrollbar-thumb", meta.borderOuter);
      }
      if (meta.accent) {
        popoverRoot.style.setProperty("--accent", meta.accent);
        popoverRoot.style.setProperty("--ok", meta.accent);
      }
      if (meta.accentSecondary) {
        popoverRoot.style.setProperty("--accent-secondary", meta.accentSecondary);
      }
      if (meta.crestGradient && meta.crestGradient.length === 2) {
        popoverRoot.style.setProperty(
          "--crest-bg",
          `linear-gradient(135deg, ${meta.crestGradient[0]}, ${meta.crestGradient[1]})`,
        );
      }
    }
  } else {
    setIcon(brandGlyph, "diamond");
    brandTitle.textContent = appId.replace(/-/g, " ");
    appTagline.textContent = "Custom Rookery Realm";
  }

  for (const [id, cardEl] of Object.entries(cards)) {
    cardEl.hidden = id !== appId;
  }

  syncSettingsForActiveApp(appId);
}

async function selectRealm(appId: string): Promise<void> {
  if (currentStatus) {
    currentStatus.active_app = appId;
  }
  applyThemeForApp(appId, true);
  setActiveView("realm");
  popoverRoot.classList.add("is-switching-realm");

  try {
    const s = await api.switchApp(appId);
    renderStatus(s);
  } catch (err) {
    console.error("switchApp error:", err);
  } finally {
    popoverRoot.classList.remove("is-switching-realm");
  }
}

for (const card of realmCardsContainer.querySelectorAll<HTMLElement>('[id^="card-"]')) {
  const appId = card.id.replace("card-", "");
  cards[appId] = card;
}

// Living Rookery Stats & Controls
const statAge = el<HTMLSpanElement>("stat-age");
const statBranches = el<HTMLSpanElement>("stat-branches");
const statFoliage = el<HTMLSpanElement>("stat-foliage");
const statBirds = el<HTMLSpanElement>("stat-birds");
const btnFeed = el<HTMLButtonElement>("btn-feed");
const btnPrune = el<HTMLButtonElement>("btn-prune");
const btnPlant = el<HTMLButtonElement>("btn-plant");
const btnPanLeft = el<HTMLButtonElement>("btn-pan-left");
const btnPanRight = el<HTMLButtonElement>("btn-pan-right");
const selectRookeryStyle = el<HTMLSelectElement>("select-rookery-style");
const btnStylePrev = el<HTMLButtonElement>("btn-style-prev");
const btnStyleNext = el<HTMLButtonElement>("btn-style-next");

// Unified Top-Layer Toast Notification
const toastNotification = el<HTMLDivElement>("toast-notification");
const toastMessage = el<HTMLSpanElement>("toast-message");
const btnToastDismiss = el<HTMLButtonElement>("btn-toast-dismiss");

// Koi Pond Stats & Controls
const statKoiCount = el<HTMLSpanElement>("stat-koi-count");
const statKoiLotuses = el<HTMLSpanElement>("stat-koi-lotuses");
const statKoiTheme = el<HTMLSpanElement>("stat-koi-theme");
const statKoiWeather = el<HTMLSpanElement>("stat-koi-weather");
const btnKoiFeed = el<HTMLButtonElement>("btn-koi-feed");
const btnKoiPebble = el<HTMLButtonElement>("btn-koi-pebble");
const btnKoiSpawn = el<HTMLButtonElement>("btn-koi-spawn");
const btnKoiBloom = el<HTMLButtonElement>("btn-koi-bloom");
const btnKoiRain = el<HTMLButtonElement>("btn-koi-rain");
const btnKoiLanterns = el<HTMLButtonElement>("btn-koi-lanterns");
const btnKoiSpout = el<HTMLButtonElement>("btn-koi-spout");
const btnKoiTheme = el<HTMLButtonElement>("btn-koi-theme");
const sliderKoiColorSpeed = el<HTMLInputElement>("slider-koi-pond-color_speed");
const valKoiColorSpeed = el<HTMLElement>("val-koi-pond-color_speed");

// Cosmic Nebula Stats & Controls
const statNebulaStars = el<HTMLSpanElement>("stat-nebula-stars");
const statNebulaEvents = el<HTMLSpanElement>("stat-nebula-events");
const statNebulaPalette = el<HTMLSpanElement>("stat-nebula-palette");
const statNebulaSupernovae = el<HTMLSpanElement>("stat-nebula-supernovae");
const btnNebulaIgnite = el<HTMLButtonElement>("btn-nebula-ignite");
const btnNebulaSupernova = el<HTMLButtonElement>("btn-nebula-supernova");
const btnNebulaComet = el<HTMLButtonElement>("btn-nebula-comet");
const btnNebulaMeteors = el<HTMLButtonElement>("btn-nebula-meteors");
const btnNebulaChart = el<HTMLButtonElement>("btn-nebula-chart");
const btnNebulaPalette = el<HTMLButtonElement>("btn-nebula-palette");
const sliderNebulaColorSpeed = el<HTMLInputElement>(
  "slider-cosmic-nebula-color_speed",
);
const valNebulaColorSpeed = el<HTMLElement>("val-cosmic-nebula-color_speed");

// Orbital Haven Stats & Controls
const statOrbitalModules = el<HTMLSpanElement>("stat-orbital-modules");
const statOrbitalSolar = el<HTMLSpanElement>("stat-orbital-solar");
const statOrbitalRings = el<HTMLSpanElement>("stat-orbital-rings");
const statOrbitalShuttles = el<HTMLSpanElement>("stat-orbital-shuttles");
const btnOrbitalLayoutSingle = el<HTMLButtonElement>("btn-orbital-layout-single");
const btnOrbitalLayoutPip = el<HTMLButtonElement>("btn-orbital-layout-pip");
const btnOrbitalLayoutGrid = el<HTMLButtonElement>("btn-orbital-layout-grid");
const orbitalFeedsSinglePip = el<HTMLDivElement>("orbital-feeds-single-pip");
const orbitalPipRow = el<HTMLDivElement>("orbital-pip-row");
const orbitalFeedsGrid = el<HTMLDivElement>("orbital-feeds-grid");
const selectOrbitalPrimary = el<HTMLSelectElement>("select-orbital-primary");
const selectOrbitalPip0 = el<HTMLSelectElement>("select-orbital-pip0");
const selectOrbitalPip1 = el<HTMLSelectElement>("select-orbital-pip1");
const selectOrbitalGrid0 = el<HTMLSelectElement>("select-orbital-grid0");
const selectOrbitalGrid1 = el<HTMLSelectElement>("select-orbital-grid1");
const selectOrbitalGrid2 = el<HTMLSelectElement>("select-orbital-grid2");
const selectOrbitalGrid3 = el<HTMLSelectElement>("select-orbital-grid3");
const selectOrbitalPalette = el<HTMLSelectElement>("select-orbital-palette");
const btnOrbitalDirector = el<HTMLButtonElement>("btn-orbital-director");
const lblOrbitalDirector = el<HTMLSpanElement>("lbl-orbital-director");
const btnOrbitalLaunch = el<HTMLButtonElement>("btn-orbital-launch");
const btnOrbitalScan = el<HTMLButtonElement>("btn-orbital-scan");

const ORBITAL_CAMERA_OPTIONS: ReadonlyArray<{ slug: string; label: string }> = [
  { slug: "follow_bot", label: "TRACK · Follow BOT-01 Room" },
  { slug: "auto_all", label: "AUTO · Rotate All Feeds" },
  { slug: "auto_interior", label: "AUTO · Rotate Interiors" },
  { slug: "auto_exterior", label: "AUTO · Rotate Exteriors" },
  { slug: "drone_alpha", label: "CAM-01 · Drone Alpha (Wide)" },
  { slug: "drone_beta", label: "CAM-02 · Drone Beta (Patrol)" },
  { slug: "hangar_approach", label: "CAM-03 · Hangar Approach" },
  { slug: "pad_a", label: "CAM-04 · Pad-A Hangar Bay" },
  { slug: "pad_b", label: "CAM-05 · Pad-B Port Deck" },
  { slug: "pad_c", label: "CAM-06 · Pad-C Stbd Deck" },
  { slug: "atrium", label: "CAM-07 · Central Core Atrium" },
  { slug: "ops_deck", label: "CAM-08 · Ops Command Deck" },
  { slug: "reactor", label: "CAM-09 · Reactor & Concourse" },
  { slug: "solar_bay", label: "CAM-10 · Solar Engineering" },
  { slug: "walker_bot", label: "BOT-01 · Interior Walker-Cam" },
  { slug: "hermes", label: "HERMES · Free-Fly Camera Bot" },
  { slug: "ship_nose", label: "UPLINK-1 · Live Ship Nose" },
  { slug: "ship_dorsal", label: "UPLINK-2 · Live Ship Dorsal" },
];

for (const sel of [
  selectOrbitalPrimary,
  selectOrbitalPip0,
  selectOrbitalPip1,
  selectOrbitalGrid0,
  selectOrbitalGrid1,
  selectOrbitalGrid2,
  selectOrbitalGrid3,
]) {
  for (const opt of ORBITAL_CAMERA_OPTIONS) {
    const o = document.createElement("option");
    o.value = opt.slug;
    o.textContent = opt.label;
    sel.appendChild(o);
  }
}

// Schedule Controls
const selectScheduleMode = el<HTMLSelectElement>("select-schedule-mode");
const intervalGroup = el<HTMLDivElement>("interval-group");
const sliderInterval = el<HTMLInputElement>("slider-interval");
const valInterval = el<HTMLElement>("val-interval");

// Menu Appearance Controls
const sliderMenuOpacity = el<HTMLInputElement>("slider-menu-opacity");
const valMenuOpacity = el<HTMLElement>("val-menu-opacity");
const sliderMenuBlur = el<HTMLInputElement>("slider-menu-blur");
const valMenuBlur = el<HTMLElement>("val-menu-blur");

function applyMenuAppearance(opacity: number, blurPx: number): void {
  const clampedOpacity = Math.min(1.0, Math.max(0.15, opacity));
  const clampedBlur = Math.min(64, Math.max(0, Math.round(blurPx)));
  popoverRoot.style.setProperty("--menu-opacity", String(clampedOpacity));
  popoverRoot.style.setProperty("--menu-blur", `${clampedBlur}px`);
  const pct = Math.round(clampedOpacity * 100);
  sliderMenuOpacity.value = String(pct);
  sliderMenuOpacity.setAttribute("aria-valuetext", `${pct}%`);
  valMenuOpacity.textContent = `${pct}%`;
  sliderMenuBlur.value = String(clampedBlur);
  sliderMenuBlur.setAttribute("aria-valuetext", `${clampedBlur}px`);
  valMenuBlur.textContent = `${clampedBlur}px`;
}

// Drag the popover window from the header or any non-interactive surface
popoverRoot.addEventListener("mousedown", (ev) => {
  if (ev.button !== 0 || ev.detail > 1) return;
  const target = ev.target as HTMLElement | null;
  if (!target) return;
  if (
    target.closest(
      "button, input, select, label, a, textarea, [contenteditable], .displays-list, .update-notes",
    )
  ) {
    return;
  }
  if (target.scrollHeight > target.clientHeight && ev.offsetX >= target.clientWidth) {
    return;
  }
  ev.preventDefault();
  api.startDrag().catch(() => {});
});

// Share & Capture
const btnQuickSnapshot = el<HTMLButtonElement>("btn-quick-snapshot");
const btnSnapshot = el<HTMLButtonElement>("btn-snapshot");
const btnRecordLoop = el<HTMLButtonElement>("btn-record-loop");
const btnRecordVideo = el<HTMLButtonElement>("btn-record-video");
const btnCopySeed = el<HTMLButtonElement>("btn-copy-seed");
const btnCopyState = el<HTMLButtonElement>("btn-copy-state");
const formImportWorld = el<HTMLFormElement>("form-import-world");
const inputImportWorld = el<HTMLInputElement>("input-import-world");
const btnImportWorld = el<HTMLButtonElement>("btn-import-world");

// Actions & Displays
const btnInteractive = el<HTMLButtonElement>("btn-interactive");
const lblInteractive = el<HTMLSpanElement>("lbl-interactive");
const kbdHotkey = el<HTMLElement>("kbd-hotkey");
const btnPause = el<HTMLButtonElement>("btn-pause");
const icoPause = el<HTMLElement>("ico-pause");
const lblPause = el<HTMLSpanElement>("lbl-pause");
const displaysList = el<HTMLUListElement>("displays-list");

// Audio Subsystem Controls
const chkAudioEnabled = el<HTMLInputElement>("chk-audio-enabled");
const audioControlsGroup = el<HTMLDivElement>("audio-controls-group");
const sliderVolume = el<HTMLInputElement>("slider-volume");
const valVolume = el<HTMLElement>("val-volume");
const chkAudioDirectMode = el<HTMLInputElement>("chk-audio-direct-mode");

// Performance & Hotkeys
const sliderPassive = el<HTMLInputElement>("slider-passive");
const valPassive = el<HTMLElement>("val-passive");
const sliderLowPower = el<HTMLInputElement>("slider-lowpower");
const valLowPower = el<HTMLElement>("val-lowpower");
const chkBattery = el<HTMLInputElement>("chk-battery");
const btnRecordHotkey = el<HTMLButtonElement>("btn-record-hotkey");
const btnRecordPopoverHotkey = el<HTMLButtonElement>("btn-record-popover-hotkey");
const btnResetHotkeys = el<HTMLButtonElement>("btn-reset-hotkeys");
const favoriteHotkeysList = el<HTMLUListElement>("favorite-hotkeys-list");
const hotkeyError = el<HTMLParagraphElement>("hotkey-error");
const btnQuit = el<HTMLButtonElement>("btn-quit");

// Software Updates
const updateStatusText = el<HTMLSpanElement>("update-status-text");
const updateDetailText = el<HTMLSpanElement>("update-detail-text");
const btnCheckUpdates = el<HTMLButtonElement>("btn-check-updates");
const updateActionPanel = el<HTMLDivElement>("update-action-panel");
const updateBadge = el<HTMLDivElement>("update-badge");
const updateNotes = el<HTMLParagraphElement>("update-notes");
const updateProgressContainer = el<HTMLDivElement>("update-progress-container");
const updateProgressFill = el<HTMLDivElement>("update-progress-fill");
const updateProgressText = el<HTMLSpanElement>("update-progress-text");
const btnDownloadUpdate = el<HTMLButtonElement>("btn-download-update");
const btnRestartUpdate = el<HTMLButtonElement>("btn-restart-update");
const chkAutoCheckUpdates = el<HTMLInputElement>("chk-auto-check-updates");
const chkAutoDownloadUpdates = el<HTMLInputElement>("chk-auto-download-updates");

// Debug Mode Elements
const badgeDebugStatus = el<HTMLSpanElement>("badge-debug-status");
const chkDebugMode = el<HTMLInputElement>("chk-debug-mode");
const selectDebugSpecies = el<HTMLSelectElement>("select-debug-species");
const inputDebugSeed = el<HTMLInputElement>("input-debug-seed");
const btnDebugRerollSeed = el<HTMLButtonElement>("btn-debug-reroll-seed");
const btnDebugViewMature = el<HTMLButtonElement>("btn-debug-view-mature");
const btnDebugNextVariation = el<HTMLButtonElement>("btn-debug-next-variation");
const btnDebugShowcaseAll = el<HTMLButtonElement>("btn-debug-showcase-all");
const btnDebugVarietyGrove = el<HTMLButtonElement>("btn-debug-variety-grove");
const btnDebugPrevTree = el<HTMLButtonElement>("btn-debug-prev-tree");
const selectDebugForestTree = el<HTMLSelectElement>("select-debug-forest-tree");
const btnDebugNextTree = el<HTMLButtonElement>("btn-debug-next-tree");

const debugTelSpecies = el<HTMLElement>("debug-tel-species");
const debugTelStatus = el<HTMLElement>("debug-tel-status");
const debugTelBranches = el<HTMLElement>("debug-tel-branches");
const debugTelFoliage = el<HTMLElement>("debug-tel-foliage");
const debugTelCamX = el<HTMLElement>("debug-tel-cam-x");
const debugTelSeed = el<HTMLElement>("debug-tel-seed");

const btnDebugMatureActive = el<HTMLButtonElement>("btn-debug-mature-active");
const btnDebugStep1 = el<HTMLButtonElement>("btn-debug-step-1");
const btnDebugStep10 = el<HTMLButtonElement>("btn-debug-step-10");
const btnDebugRewind = el<HTMLButtonElement>("btn-debug-rewind");
const valDebugSpeed = el<HTMLElement>("val-debug-speed");
const speedButtons = document.querySelectorAll<HTMLButtonElement>(".btn-speed-select");

const selectDebugPalette = el<HTMLSelectElement>("select-debug-palette");
const selectDebugBirdStyle = el<HTMLSelectElement>("select-debug-bird-style");
const btnSpawnRaven = el<HTMLButtonElement>("btn-spawn-raven");
const btnSpawnTanager = el<HTMLButtonElement>("btn-spawn-tanager");
const btnSpawnOwl = el<HTMLButtonElement>("btn-spawn-owl");
const btnSpawnFox = el<HTMLButtonElement>("btn-spawn-fox");
const btnSpawnSquirrel = el<HTMLButtonElement>("btn-spawn-squirrel");
const btnSpawnBadger = el<HTMLButtonElement>("btn-spawn-badger");
const btnSpawnHedgehog = el<HTMLButtonElement>("btn-spawn-hedgehog");
const btnSpawnRabbit = el<HTMLButtonElement>("btn-spawn-rabbit");
const btnScatterSnacks = el<HTMLButtonElement>("btn-scatter-snacks");
const btnDebugResetWorld = el<HTMLButtonElement>("btn-debug-reset-world");

type HotkeyTarget =
  | { kind: "interaction" }
  | { kind: "popover" }
  | { kind: "favorite"; index: number };

let currentStatus: HostStatus | null = null;
let currentSettings: Settings | null = null;
let recordingTarget: HotkeyTarget | null = null;

let appCatalog: AppManifest[] = [];

function getAppGlyph(m: AppManifest): string {
  if (m.theme?.icon) return m.theme.icon;
  if (appThemes[m.metadata.id]) return appThemes[m.metadata.id].glyph;
  for (const c of m.controls || []) {
    if (c.icon) return c.icon;
  }
  const archetypes: Record<string, IconName> = {
    cozy_ecosystem: "leaf",
    generative_physics: "atom",
    slow_burn_idle: "hourglass",
    ambient_utility: "compass",
    web_canvas: "shooting-star",
  };
  return archetypes[m.metadata.archetype] || "diamond";
}

function getArchetypeShortLabel(archetype: string): string {
  const labels: Record<string, string> = {
    cozy_ecosystem: "Ecosystem",
    generative_physics: "Physics & Shader",
    slow_burn_idle: "Idle World",
    ambient_utility: "Utility",
    web_canvas: "Web Canvas",
  };
  return labels[archetype] || archetype.replace(/_/g, " ");
}

function renderLibraryGrid(): void {
  const activeId = currentStatus?.active_app || "living-rookery";
  const q = librarySearchQuery.trim().toLowerCase();

  const filtered = appCatalog.filter((m) => {
    if (activeLibraryFilter === "favorites") {
      if (!favoriteApps.includes(m.metadata.id)) return false;
    } else if (activeLibraryFilter !== "all") {
      if (m.metadata.archetype !== activeLibraryFilter) return false;
    }

    if (!q) return true;
    const haystack = [
      m.metadata.id,
      m.metadata.name,
      m.metadata.description,
      m.metadata.author,
      m.metadata.archetype,
      m.theme?.tagline || "",
      ...(m.metadata.tags || []),
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(q);
  });

  // Sort favorites first in their pinned slot order
  const sorted = [...filtered].sort((a, b) => {
    const aIdx = favoriteApps.indexOf(a.metadata.id);
    const bIdx = favoriteApps.indexOf(b.metadata.id);
    const aRank = aIdx === -1 ? Number.MAX_SAFE_INTEGER : aIdx;
    const bRank = bIdx === -1 ? Number.MAX_SAFE_INTEGER : bIdx;
    return aRank - bRank;
  });

  libraryGrid.replaceChildren();

  if (sorted.length === 0) {
    const emptyLi = document.createElement("li");
    emptyLi.className = "empty";
    emptyLi.style.padding = "14px 8px";
    emptyLi.style.textAlign = "center";
    emptyLi.textContent =
      activeLibraryFilter === "favorites" && favoriteApps.length === 0
        ? "No favorite realms starred yet. Click ★ on any realm to pin it here."
        : "No realms match your search filter.";
    libraryGrid.appendChild(emptyLi);
    return;
  }

  for (const m of sorted) {
    const id = m.metadata.id;
    const isActive = id === activeId;
    const isFav = favoriteApps.includes(id);
    const meta = appThemes[id];

    const li = document.createElement("li");
    li.className = `library-item${isActive ? " active" : ""}`;

    const selectBtn = document.createElement("button");
    selectBtn.type = "button";
    selectBtn.className = "library-select-btn";
    selectBtn.addEventListener("click", () => {
      selectRealm(id);
    });

    const crest = document.createElement("span");
    crest.className = "library-crest";
    if (meta?.accent) {
      crest.style.color = meta.accent;
    }
    crest.appendChild(icon(getAppGlyph(m)));

    const info = document.createElement("span");
    info.className = "library-info";

    const titleRow = document.createElement("span");
    titleRow.className = "library-title-row";
    const titleEl = document.createElement("strong");
    titleEl.className = "library-title";
    titleEl.textContent = m.metadata.name;
    titleRow.appendChild(titleEl);

    if (isActive) {
      const activePill = document.createElement("span");
      activePill.className = "library-active-pill";
      activePill.textContent = "Active";
      titleRow.appendChild(activePill);
    }

    const favSlotIdx = favoriteApps.indexOf(id);
    if (favSlotIdx >= 0 && favSlotIdx < FAVORITE_HOTKEY_SLOTS) {
      const favShortcut =
        favoriteHotkeys[favSlotIdx] ||
        `CommandOrControl+Alt+Shift+${favSlotIdx + 1}`;
      const shortcutPill = document.createElement("span");
      shortcutPill.className = "library-shortcut-pill";
      shortcutPill.textContent = `#${favSlotIdx + 1} · ${formatShortcutForDisplay(favShortcut)}`;
      shortcutPill.title = `Favorite #${favSlotIdx + 1} shortcut (${favShortcut})`;
      titleRow.appendChild(shortcutPill);
    }

    const descEl = document.createElement("span");
    descEl.className = "library-desc";
    descEl.textContent = m.theme?.tagline || m.metadata.description;

    const metaRow = document.createElement("span");
    metaRow.className = "library-meta-row";
    const archBadge = document.createElement("span");
    archBadge.className = "mini-badge";
    archBadge.textContent = getArchetypeShortLabel(m.metadata.archetype || "");
    const fpsBadge = document.createElement("span");
    fpsBadge.className = "mini-badge";
    fpsBadge.textContent = `${m.budgets?.target_fps ?? 30} FPS`;
    const verBadge = document.createElement("span");
    verBadge.className = "mini-badge";
    verBadge.textContent = `v${m.metadata.version || "1.0"}`;
    metaRow.append(archBadge, fpsBadge, verBadge);

    info.append(titleRow, descEl, metaRow);
    selectBtn.append(crest, info);

    const favBtn = document.createElement("button");
    favBtn.type = "button";
    favBtn.className = `library-fav-btn${isFav ? " favorited" : ""}`;
    favBtn.setAttribute("aria-pressed", String(isFav));
    favBtn.setAttribute(
      "aria-label",
      isFav ? `Remove ${m.metadata.name} from favorites` : `Add ${m.metadata.name} to favorites`,
    );
    favBtn.title = isFav ? "Unfavorite realm" : "Favorite realm (prioritized in rotation & library)";
    favBtn.appendChild(icon("star"));
    favBtn.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      if (favoriteApps.includes(id)) {
        favoriteApps = favoriteApps.filter((f) => f !== id);
      } else {
        favoriteApps = [...favoriteApps, id];
      }
      renderLibraryGrid();
      try {
        const st = await api.updateSettings({ favorite_apps: favoriteApps });
        renderSettings(st);
      } catch (err) {
        console.error("update favorite_apps error:", err);
      }
    });

    li.append(selectBtn, favBtn);
    libraryGrid.appendChild(li);
  }
}

// Library Search, Filter & Reload Listeners
inputLibrarySearch.addEventListener("input", () => {
  librarySearchQuery = inputLibrarySearch.value;
  renderLibraryGrid();
});

for (const pill of filterPills) {
  pill.addEventListener("click", () => {
    activeLibraryFilter = pill.getAttribute("data-filter") || "all";
    for (const p of filterPills) {
      const isSelected = p === pill;
      p.classList.toggle("active", isSelected);
      p.setAttribute("aria-pressed", String(isSelected));
    }
    renderLibraryGrid();
  });
}

btnReloadCatalog.addEventListener("click", async () => {
  try {
    btnReloadCatalog.disabled = true;
    const catalog = await api.reloadCatalog();
    appCatalog = catalog;
    try {
      localStorage.setItem("rookery_catalog", JSON.stringify(catalog));
    } catch {}
    applyCatalog();
  } catch (err) {
    console.error("reloadCatalog error:", err);
  } finally {
    btnReloadCatalog.disabled = false;
  }
});

function applyCatalog(): void {
  realmCountPill.textContent = `${appCatalog.length} ${appCatalog.length === 1 ? "Realm" : "Realms"}`;

  for (const m of appCatalog) {
    const existing = appThemes[m.metadata.id];
    appThemes[m.metadata.id] = {
      title: m.metadata.name || existing?.title || m.metadata.id,
      tagline:
        m.theme?.tagline ||
        existing?.tagline ||
        m.metadata.description ||
        "Custom Rookery Realm",
      bannerTitle: m.theme?.banner_title || existing?.bannerTitle || m.metadata.name,
      glyph: m.theme?.icon || existing?.glyph || getAppGlyph(m),
      ornament: m.theme?.ornament || existing?.ornament || "sparkle",
      bg: m.theme?.bg || existing?.bg,
      card: m.theme?.card || existing?.card,
      borderOuter: m.theme?.border_outer || existing?.borderOuter,
      accent: m.theme?.accent || existing?.accent,
      accentSecondary: m.theme?.accent_secondary || existing?.accentSecondary,
      crestGradient: m.theme?.crest_gradient || existing?.crestGradient,
    };

    // Ensure card exists inside realmCardsContainer (rebuild dynamic cards when manifest updates)
    if (cards[m.metadata.id]?.classList.contains("app-dynamic-card")) {
      cards[m.metadata.id].remove();
      delete cards[m.metadata.id];
    }
    if (!cards[m.metadata.id]) {
      const themeMeta = appThemes[m.metadata.id];
      const card = document.createElement("section");
      card.id = `card-${m.metadata.id}`;
      card.className = "card hud-card app-dynamic-card";
      card.hidden = true;

      const banner = document.createElement("div");
      banner.className = "hud-banner";
      const ornamentLeft = document.createElement("span");
      ornamentLeft.className = "hud-ornament left";
      ornamentLeft.appendChild(icon(themeMeta.ornament));
      const bannerTitle = document.createElement("span");
      bannerTitle.className = "section-title";
      bannerTitle.textContent = themeMeta.bannerTitle || m.metadata.name;
      const ornamentRight = document.createElement("span");
      ornamentRight.className = "hud-ornament right";
      ornamentRight.appendChild(icon(themeMeta.ornament));
      banner.append(ornamentLeft, bannerTitle, ornamentRight);
      card.appendChild(banner);

      const badges = document.createElement("div");
      badges.className = "app-badges";
      badges.innerHTML = `
        <span class="mini-badge">${(m.metadata.archetype || "").replace(/_/g, " ")}</span>
        <span class="mini-badge">v${m.metadata.version || "1.0"}</span>
        <span class="mini-badge">${m.budgets?.target_fps ?? 30} FPS</span>
        <span class="mini-badge">&lt;${m.budgets?.max_cpu_ms ?? 2.0}ms CPU</span>
      `;
      card.appendChild(badges);

      // Stat controls
      const statControls = (m.controls || []).filter((c) => c.kind === "stat");
      if (statControls.length > 0) {
        const statsGrid = document.createElement("div");
        statsGrid.className = "stats-grid";
        for (const sc of statControls) {
          const box = document.createElement("div");
          box.className = "stat-box";
          const labelSpan = document.createElement("span");
          labelSpan.className = "stat-label";
          if (sc.icon) {
            labelSpan.appendChild(icon(sc.icon));
          }
          labelSpan.appendChild(document.createTextNode(sc.label));

          const valSpan = document.createElement("span");
          valSpan.id = `stat-${m.metadata.id}-${sc.id}`;
          valSpan.className = "stat-value";
          valSpan.textContent = String(sc.default_val ?? "-");

          box.append(labelSpan, valSpan);
          statsGrid.appendChild(box);
        }
        card.appendChild(statsGrid);
      }

      // Slider controls
      const sliderControls = (m.controls || []).filter((c) => c.kind === "slider");
      for (const sl of sliderControls) {
        const sliderId = `slider-${m.metadata.id}-${sl.id}`;
        const row = document.createElement("label");
        row.className = "control-row";
        row.htmlFor = sliderId;
        const label = document.createElement("span");
        label.textContent = sl.label;
        const existingVal = currentStatus?.app_controls?.[m.metadata.id]?.[sl.id];
        const valBadge = document.createElement("strong");
        valBadge.id = `val-${m.metadata.id}-${sl.id}`;
        valBadge.textContent = `${existingVal ?? sl.default_val ?? 0}${sl.unit ?? ""}`;
        row.appendChild(label);
        row.appendChild(valBadge);
        card.appendChild(row);

        const input = document.createElement("input");
        input.id = sliderId;
        input.type = "range";
        input.min = String(sl.min ?? 0);
        input.max = String(sl.max ?? 100);
        input.step = String(sl.step ?? 1);
        input.value = String(existingVal ?? sl.default_val ?? 0);
        const sendSliderUpdate = createCoalescedCaller(async (val: number) => {
          try {
            const s = await api.executeAppAction(m.metadata.id, sl.id, {
              value: val,
            });
            renderStatus(s);
          } catch (e) {
            console.error("slider action error:", e);
          }
        });
        input.addEventListener("input", () => {
          valBadge.textContent = `${input.value}${sl.unit ?? ""}`;
          sendSliderUpdate(Number(input.value));
        });
        input.addEventListener("change", () => {
          sendSliderUpdate(Number(input.value));
        });
        card.appendChild(input);
      }

      // Select controls
      const selectControls = (m.controls || []).filter((c) => c.kind === "select");
      for (const sel of selectControls) {
        const row = document.createElement("div");
        row.className = "schedule-row";
        const label = document.createElement("label");
        label.htmlFor = `select-${m.metadata.id}-${sel.id}`;
        label.textContent = `${sel.label}:`;
        const selectEl = document.createElement("select");
        selectEl.id = `select-${m.metadata.id}-${sel.id}`;
        selectEl.className = "select-input";
        const existingSelVal = currentStatus?.app_controls?.[m.metadata.id]?.[sel.id];
        for (const opt of sel.options || []) {
          const optEl = document.createElement("option");
          optEl.value = opt;
          optEl.textContent = opt;
          const targetVal = existingSelVal ?? sel.default_val;
          if (opt === targetVal) optEl.selected = true;
          selectEl.appendChild(optEl);
        }
        selectEl.addEventListener("change", async () => {
          try {
            const s = await api.executeAppAction(m.metadata.id, sel.id, {
              value: selectEl.value,
            });
            renderStatus(s);
          } catch (e) {
            console.error("select action error:", e);
          }
        });
        row.appendChild(label);
        row.appendChild(selectEl);
        card.appendChild(row);
      }

      // Button controls
      const buttonControls = (m.controls || []).filter((c) => c.kind === "button");
      if (buttonControls.length > 0) {
        const actionsDiv = document.createElement("div");
        actionsDiv.className = "rookery-actions";
        buttonControls.forEach((b, idx) => {
          const btn = document.createElement("button");
          btn.id = `btn-${m.metadata.id}-${b.id}`;
          const isOddLast =
            buttonControls.length % 2 === 1 && idx === buttonControls.length - 1;
          btn.className = `btn game-cmd-btn small${isOddLast ? " span-2" : ""}`;
          btn.type = "button";
          if (b.tooltip) btn.title = b.tooltip;
          iconLabel(btn, b.icon, b.label);
          btn.addEventListener("click", () => {
            runActionWithFeedback(
              btn,
              b.icon || "check-circle",
              `${b.label} activated!`,
              () => api.executeAppAction(m.metadata.id, b.id),
            );
          });
          actionsDiv.appendChild(btn);
        });
        card.appendChild(actionsDiv);
      }

      realmCardsContainer.appendChild(card);
      cards[m.metadata.id] = card;
    }
  }

  renderLibraryGrid();
  renderFavoriteHotkeysList();

  if (currentStatus) {
    const active = currentStatus.active_app || "living-rookery";
    applyThemeForApp(active, true);
  }
}

function formatBudget(d: DisplayStatus): { text: string; paused: boolean } {
  const b: FrameBudget = d.budget;
  if (b.kind === "paused") {
    const reasons: Record<string, string> = {
      occluded: "Paused · covered",
      user_paused: "Paused",
      screen_asleep: "Paused · sleep",
      on_battery: "Paused · battery",
    };
    return { text: reasons[b.value] ?? "Paused", paused: true };
  }
  const actual = d.actual_fps > 0 ? Math.round(d.actual_fps) : b.value;
  return {
    text: `${actual}/${b.value} fps · ${d.frame_cpu_ms.toFixed(2)}ms`,
    paused: false,
  };
}

function formatAge(secs: number): string {
  if (secs < 60) return `${Math.floor(secs)}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ${Math.floor(secs % 60)}s`;
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  if (hours < 24) return `${hours}h ${remMins}m`;
  const days = Math.floor(hours / 24);
  const remHours = hours % 24;
  return `${days}d ${remHours}h`;
}

type DocumentWithAriaNotify = Document & {
  ariaNotify?: (announcement: string) => void;
};

const HAS_NATIVE_POPOVER =
  typeof HTMLElement !== "undefined" &&
  Object.hasOwn(HTMLElement.prototype, "popover");
const HAS_ARIA_NOTIFY =
  typeof (document as DocumentWithAriaNotify).ariaNotify === "function";

if (HAS_ARIA_NOTIFY) {
  toastMessage.removeAttribute("aria-live");
}

let supportsDisplayTransition: boolean | undefined;
function canTransitionDisplay(): boolean {
  if (supportsDisplayTransition !== undefined) return supportsDisplayTransition;
  if (!window.CSS?.supports?.("transition-behavior", "allow-discrete") || !document.body) {
    return false;
  }
  const probe = document.createElement("div");
  probe.style.cssText =
    "transition: display 1s allow-discrete !important; display: block;";
  document.body.appendChild(probe);
  getComputedStyle(probe).display;
  probe.style.display = "none";
  supportsDisplayTransition = getComputedStyle(probe).display === "block";
  probe.remove();
  return supportsDisplayTransition;
}

function supportsTopLayerExit(): boolean {
  return Boolean(window.CSS?.supports?.("overlay", "auto") && canTransitionDisplay());
}

function isToastOpen(): boolean {
  if (HAS_NATIVE_POPOVER) {
    return toastNotification.matches(":popover-open");
  }
  return toastNotification.classList.contains(":popover-open");
}

let notificationTimer: number | null = null;
let toastTriggerEl: HTMLElement | null = null;
let toastPendingDismissAfterBlur = false;

function restoreFocusAfterToastDismiss(): void {
  const target = toastTriggerEl;
  toastTriggerEl = null;
  if (
    target &&
    target.isConnected &&
    !target.closest("[hidden]") &&
    typeof target.focus === "function"
  ) {
    target.focus();
  }
}

async function closeTopLayerToast(restoreFocus: boolean): Promise<void> {
  if (!isToastOpen()) return;
  if (!supportsTopLayerExit()) {
    toastNotification.setAttribute("data-closing", "");
    const animations = toastNotification.getAnimations({ subtree: true });
    if (animations.length > 0) {
      await Promise.race([
        Promise.allSettled(animations.map((a) => a.finished)),
        new Promise((r) => window.setTimeout(r, 400)),
      ]);
    }
    if (!toastNotification.hasAttribute("data-closing")) return;
    toastNotification.removeAttribute("data-closing");
  }

  if (HAS_NATIVE_POPOVER) {
    try {
      toastNotification.hidePopover();
    } catch {
      // Already closed
    }
  } else {
    toastNotification.classList.remove(":popover-open");
  }

  if (restoreFocus) {
    restoreFocusAfterToastDismiss();
  }
}

function scheduleToastDismiss(delayMs: number): void {
  if (notificationTimer !== null) {
    window.clearTimeout(notificationTimer);
  }
  notificationTimer = window.setTimeout(() => {
    notificationTimer = null;
    if (toastNotification.contains(document.activeElement)) {
      toastPendingDismissAfterBlur = true;
      return;
    }
    void closeTopLayerToast(false);
  }, delayMs);
}

toastNotification.addEventListener("focusout", (ev) => {
  const next = ev.relatedTarget as Node | null;
  if (toastPendingDismissAfterBlur && (!next || !toastNotification.contains(next))) {
    toastPendingDismissAfterBlur = false;
    scheduleToastDismiss(1200);
  }
});

btnToastDismiss.addEventListener("click", (ev) => {
  if (notificationTimer !== null) {
    window.clearTimeout(notificationTimer);
    notificationTimer = null;
  }
  toastPendingDismissAfterBlur = false;
  if (!HAS_NATIVE_POPOVER || !supportsTopLayerExit()) {
    ev.preventDefault();
    void closeTopLayerToast(true);
  } else {
    // Native popovertargetaction="hide" closes it; restore focus on the next frame
    window.requestAnimationFrame(() => {
      if (!isToastOpen()) {
        restoreFocusAfterToastDismiss();
      }
    });
  }
});

toastNotification.addEventListener("toggle", (ev) => {
  const toggleEv = ev as ToggleEvent;
  if (toggleEv.newState === "closed") {
    toastNotification.removeAttribute("data-closing");
    toastPendingDismissAfterBlur = false;
    if (notificationTimer !== null) {
      window.clearTimeout(notificationTimer);
      notificationTimer = null;
    }
  }
});

function showNotification(
  iconName: IconName | string,
  msg: string,
  options?: { isError?: boolean; durationMs?: number },
): void {
  const isError = options?.isError ?? false;
  const durationMs = options?.durationMs ?? 2800;

  if (
    document.activeElement instanceof HTMLElement &&
    !toastNotification.contains(document.activeElement)
  ) {
    toastTriggerEl = document.activeElement;
  }

  toastNotification.removeAttribute("data-closing");
  toastPendingDismissAfterBlur = false;
  if (isError) {
    toastNotification.dataset.variant = "error";
  } else {
    delete toastNotification.dataset.variant;
  }

  iconLabel(toastMessage, isError ? "warning" : iconName, msg);

  if (HAS_NATIVE_POPOVER) {
    if (!toastNotification.matches(":popover-open")) {
      toastNotification.showPopover();
    }
  } else {
    toastNotification.classList.add(":popover-open");
  }

  if (HAS_ARIA_NOTIFY) {
    (document as DocumentWithAriaNotify).ariaNotify?.(msg);
  }

  scheduleToastDismiss(durationMs);
}

function formatSimSpeed(speed: number): string {
  if (speed <= 0.01) return "0.0x (Paused)";
  if (Math.abs(speed - 1.0) < 0.05) return "1.0x (Normal)";
  if (Math.abs(speed - 50.0) < 0.5) return "50.0x (Hyper)";
  return `${speed.toFixed(1)}x`;
}

function showShareNotification(iconName: IconName, msg: string, isError = false): void {
  showNotification(iconName, msg, { isError, durationMs: 3500 });
}

function renderStatus(s: HostStatus): void {
  const prevApp = currentStatus?.active_app;
  currentStatus = s;

  const now = performance.now();
  if (now - lastStatusCacheWriteMs > 5000 || prevApp !== s.active_app) {
    lastStatusCacheWriteMs = now;
    try {
      localStorage.setItem("rookery_status", JSON.stringify(s));
    } catch {}
  }

  // Power badge
  const powerLabels: Record<string, string> = {
    ac: "AC",
    battery: "BAT",
    low_power: "LOW PWR",
  };
  const nextPowerText = powerLabels[s.power] ?? s.power.toUpperCase();
  if (powerBadge.textContent !== nextPowerText) {
    powerBadge.textContent = nextPowerText;
  }

  // Active App Theme & Cards (memoized unless activeApp changed)
  const activeApp = s.active_app || "living-rookery";
  applyThemeForApp(activeApp);
  if (activeApp === "gothic-metropolis" && s.gothic) {
    applyGothicPaletteTheme(s.gothic.palette, s.gothic.palette_blend);
  }

  if (activeView === "library" && prevApp !== activeApp) {
    renderLibraryGrid();
  }

  // Living Rookery Stats
  if (s.rookery && activeApp === "living-rookery") {
    statAge.textContent = formatAge(s.rookery.total_age_secs);
    statBranches.textContent = String(s.rookery.total_branches);
    statFoliage.textContent = String(s.rookery.total_foliage);
    statBirds.textContent = `${s.rookery.birds_active} birds`;
    if (
      typeof s.rookery.aesthetic_style === "number" &&
      document.activeElement !== selectRookeryStyle
    ) {
      selectRookeryStyle.value = String(s.rookery.aesthetic_style);
    }
    if (
      typeof s.rookery.aesthetic_style === "number" &&
      document.activeElement !== selectDebugPalette
    ) {
      selectDebugPalette.value = String(s.rookery.aesthetic_style);
    }
    if (
      typeof s.rookery.bird_style === "string" &&
      document.activeElement !== selectDebugBirdStyle
    ) {
      selectDebugBirdStyle.value = s.rookery.bird_style;
    }

    // Debug Mode Telemetry & Sync
    if (typeof s.rookery.debug_mode === "boolean") {
      chkDebugMode.checked = s.rookery.debug_mode;
      badgeDebugStatus.textContent = s.rookery.debug_mode ? "Active" : "Passive";
      badgeDebugStatus.classList.toggle("active", s.rookery.debug_mode);
    }

    debugTelSpecies.textContent = s.rookery.active_species || "Ancient Oak";
    const curStep = s.rookery.active_tree_growth_steps ?? 0;
    const tgtStep = s.rookery.active_tree_target_steps ?? 42;
    const isMature = s.rookery.active_tree_is_mature ?? curStep >= tgtStep;
    debugTelStatus.textContent = `Step ${curStep} / ${tgtStep} (${isMature ? "Mature" : "Growing"})`;
    debugTelBranches.textContent = String(s.rookery.total_branches);
    debugTelFoliage.textContent = String(s.rookery.total_foliage);
    debugTelCamX.textContent = `${(s.rookery.viewport_offset_x ?? 0).toFixed(0)} px`;
    debugTelSeed.textContent = `#${s.rookery.active_tree_seed ?? s.rookery.seed}`;

    if (typeof s.rookery.sim_speed === "number") {
      valDebugSpeed.textContent = formatSimSpeed(s.rookery.sim_speed);
      for (const btn of speedButtons) {
        const spd = parseFloat(btn.getAttribute("data-speed") || "1.0");
        btn.classList.toggle("active", Math.abs(spd - s.rookery.sim_speed) < 0.05);
      }
    }

    // Populate Forest Tree Dropdown if tree list available
    if (Array.isArray(s.rookery.trees) && s.rookery.trees.length > 0) {
      const activeId = s.rookery.active_tree_id;
      const treesSig = s.rookery.trees
        .map((t) => `${t.id}:${t.species}:${t.growth_steps}:${t.is_mature}`)
        .join("|");
      if (
        treesSig !== lastTreesSignature &&
        document.activeElement !== selectDebugForestTree
      ) {
        lastTreesSignature = treesSig;
        selectDebugForestTree.innerHTML = "";
        for (const t of s.rookery.trees) {
          const opt = document.createElement("option");
          opt.value = String(t.id);
          const statusLabel = t.is_mature
            ? "Mature"
            : `Step ${t.growth_steps}/${t.target_steps}`;
          opt.textContent = `Tree #${t.id}: ${t.species} (${statusLabel}, Seed #${t.seed})`;
          if (t.id === activeId) {
            opt.selected = true;
          }
          selectDebugForestTree.appendChild(opt);
        }
      }
      if (activeId && document.activeElement !== selectDebugForestTree) {
        selectDebugForestTree.value = String(activeId);
      }
      const treeCount = selectDebugForestTree.options.length;
      const selIdx = selectDebugForestTree.selectedIndex;
      btnDebugPrevTree.disabled = treeCount <= 1 || selIdx <= 0;
      btnDebugNextTree.disabled =
        treeCount <= 1 || selIdx < 0 || selIdx >= treeCount - 1;
    }
  }

  // Koi Pond Stats
  if (s.koi) {
    statKoiCount.textContent = String(s.koi.total_koi);
    statKoiLotuses.textContent = String(s.koi.blooming_lotuses);
    statKoiTheme.textContent = s.koi.theme_name ?? "Sunlit Jade";
    const weatherParts: string[] = [];
    weatherParts.push(s.koi.rain_enabled ? "Rain" : "Clear");
    if (s.koi.lanterns_enabled) {
      weatherParts.push("Lanterns");
    }
    statKoiWeather.textContent = weatherParts.join(" · ");
    btnKoiRain.setAttribute("aria-pressed", String(Boolean(s.koi.rain_enabled)));
    btnKoiLanterns.setAttribute(
      "aria-pressed",
      String(Boolean(s.koi.lanterns_enabled)),
    );
  }

  // Cosmic Nebula Stats
  if (s.nebula) {
    const protoSuffix =
      s.nebula.protostars > 0 ? ` (${s.nebula.protostars} forming)` : "";
    statNebulaStars.textContent = `${s.nebula.stars_active}${protoSuffix}`;
    const eventParts: string[] = [];
    if (s.nebula.active_shockwaves > 0) {
      eventParts.push(`${s.nebula.active_shockwaves} Nova`);
    }
    if (s.nebula.active_comets > 0) {
      eventParts.push(`${s.nebula.active_comets} Comet`);
    }
    if (s.nebula.active_meteors > 0) {
      eventParts.push(`${s.nebula.active_meteors} Meteors`);
    }
    statNebulaEvents.textContent =
      eventParts.length > 0 ? eventParts.join(" · ") : "Calm";
    statNebulaPalette.textContent = s.nebula.palette_name;
    statNebulaSupernovae.textContent = String(s.nebula.supernovae_total);
    btnNebulaChart.classList.toggle("active", Boolean(s.nebula.star_chart_enabled));
  }

  // Orbital Haven Stats & Direct Camera/Layout/Theme Sync
  if (s.orbital) {
    const o = s.orbital;
    statOrbitalModules.textContent = `${o.active_feed_code} · ${o.active_feed_label}`;
    const dirLabel = o.auto_director ? "AUTO" : "LOCKED";
    statOrbitalSolar.textContent = `${o.layout} · ${dirLabel}`;
    statOrbitalRings.textContent = `${o.station_event} · ${o.palette}`;
    statOrbitalShuttles.textContent = `${o.active_ships} Ships · ${o.uplink_callsign}`;

    btnOrbitalLayoutSingle.classList.toggle("active", o.layout_slug === "single");
    btnOrbitalLayoutPip.classList.toggle("active", o.layout_slug === "pip");
    btnOrbitalLayoutGrid.classList.toggle("active", o.layout_slug === "grid2x2");

    const isGrid = o.layout_slug === "grid2x2";
    orbitalFeedsGrid.hidden = !isGrid;
    orbitalFeedsSinglePip.hidden = isGrid;
    orbitalPipRow.hidden = o.layout_slug !== "pip";

    const syncSelect = (sel: HTMLSelectElement, val: string) => {
      if (document.activeElement !== sel && sel.value !== val) {
        sel.value = val;
      }
    };
    syncSelect(selectOrbitalPrimary, o.primary_feed_slug);
    syncSelect(selectOrbitalPip0, o.pip_feed_slugs[0]);
    syncSelect(selectOrbitalPip1, o.pip_feed_slugs[1]);
    syncSelect(selectOrbitalGrid0, o.grid_feed_slugs[0]);
    syncSelect(selectOrbitalGrid1, o.grid_feed_slugs[1]);
    syncSelect(selectOrbitalGrid2, o.grid_feed_slugs[2]);
    syncSelect(selectOrbitalGrid3, o.grid_feed_slugs[3]);
    syncSelect(selectOrbitalPalette, o.palette_slug);

    btnOrbitalDirector.classList.toggle("active", o.auto_director);
    lblOrbitalDirector.textContent = o.auto_director
      ? "Auto-Director: ON"
      : "Auto-Director: OFF";
  }

  // Dynamic SDK App Stats & Controls
  if (s.app_stats) {
    for (const [appId, statsMap] of Object.entries(s.app_stats)) {
      for (const [statId, val] of Object.entries(statsMap)) {
        const statNode = document.getElementById(`stat-${appId}-${statId}`);
        if (statNode) {
          statNode.textContent = val;
        }
      }
    }
  }

  if (s.app_controls) {
    for (const [appId, ctrlMap] of Object.entries(s.app_controls)) {
      const manifest = appCatalog.find((m) => m.metadata.id === appId);
      for (const [ctrlId, val] of Object.entries(ctrlMap)) {
        const sliderNode = document.getElementById(
          `slider-${appId}-${ctrlId}`,
        ) as HTMLInputElement | null;
        if (sliderNode && document.activeElement !== sliderNode) {
          sliderNode.value = val;
          const badgeNode = document.getElementById(`val-${appId}-${ctrlId}`);
          const unit =
            manifest?.controls.find((c) => c.id === ctrlId && c.kind === "slider")?.unit ?? "";
          if (badgeNode) badgeNode.textContent = `${val}${unit}`;
        }
        const selectNode = document.getElementById(
          `select-${appId}-${ctrlId}`,
        ) as HTMLSelectElement | null;
        if (selectNode && document.activeElement !== selectNode) {
          selectNode.value = val;
        }
        const btnNode = document.getElementById(
          `btn-${appId}-${ctrlId}`,
        ) as HTMLButtonElement | null;
        if (btnNode && (val === "true" || val === "false")) {
          btnNode.classList.toggle("active", val === "true");
        }
      }
    }
  }

  // Schedule Mode
  if (document.activeElement !== selectScheduleMode) {
    selectScheduleMode.value = s.schedule_mode;
  }
  const isInterval = s.schedule_mode === "interval";
  intervalGroup.hidden = !isInterval;
  sliderInterval.hidden = !isInterval;

  // Interactive button
  btnInteractive.classList.toggle("active", s.interactive);
  lblInteractive.textContent = s.interactive
    ? "Desktop Interactive"
    : "Interact on Desktop";

  // Pause button
  setIcon(icoPause, s.user_paused ? "play" : "pause");
  lblPause.textContent = s.user_paused ? "Resume" : "Pause";

  // Displays list (diffed to avoid DOM teardown on every tick)
  const displaysSig = s.displays.map((d) => `${d.id}:${d.name}:${d.width}x${d.height}`).join("|");
  if (displaysSig !== lastDisplaysSignature) {
    lastDisplaysSignature = displaysSig;
    displaysList.innerHTML = "";
    if (s.displays.length === 0) {
      const li = document.createElement("li");
      li.className = "empty";
      li.textContent = "No active displays";
      displaysList.appendChild(li);
    } else {
      for (const d of s.displays) {
        const li = document.createElement("li");
        li.className = "display-item";
        li.dataset.displayId = d.id;

        const left = document.createElement("div");
        const name = document.createElement("strong");
        name.textContent = d.name;
        const meta = document.createElement("span");
        meta.className = "display-meta";
        meta.textContent = `${d.width}×${d.height}`;
        left.appendChild(name);
        left.appendChild(meta);

        const right = document.createElement("span");
        const info = formatBudget(d);
        right.className = info.paused ? "status-pill paused" : "status-pill";
        right.textContent = info.text;

        li.appendChild(left);
        li.appendChild(right);
        displaysList.appendChild(li);
      }
    }
  } else {
    for (const d of s.displays) {
      const li = displaysList.querySelector<HTMLElement>(
        `li[data-display-id="${CSS.escape(d.id)}"]`,
      );
      const pill = li?.querySelector<HTMLElement>(".status-pill");
      if (pill) {
        const info = formatBudget(d);
        const nextClass = info.paused ? "status-pill paused" : "status-pill";
        if (pill.className !== nextClass) pill.className = nextClass;
        if (pill.textContent !== info.text) pill.textContent = info.text;
      }
    }
  }

  // Audio Subsystem Status
  if (s.audio) {
    chkAudioEnabled.checked = s.audio.enabled;
    audioControlsGroup.hidden = !s.audio.enabled;
    if (document.activeElement !== sliderVolume) {
      const volPct = Math.round(s.audio.volume * 100);
      sliderVolume.value = String(volPct);
      sliderVolume.setAttribute("aria-valuetext", `${volPct}%`);
      valVolume.textContent = `${volPct}%`;
    }
    chkAudioDirectMode.checked = s.audio.direct_mode_only;
  }
}

async function patchSettings(patch: SettingsPatch, label: string): Promise<boolean> {
  try {
    const st = await api.updateSettings(patch);
    renderSettings(st);
    return true;
  } catch (err) {
    console.error(`updateSettings (${label}) error:`, err);
    if (currentSettings) {
      renderSettings(currentSettings);
    }
    return false;
  }
}

function renderSettings(st: Settings): void {
  currentSettings = st;
  try {
    localStorage.setItem("rookery_settings", JSON.stringify(st));
  } catch {}

  if (Array.isArray(st.favorite_apps)) {
    favoriteApps = st.favorite_apps;
  }
  const defaults = defaultFavoriteHotkeys();
  favoriteHotkeys = defaults.map((def, i) => {
    const val = st.favorite_hotkeys?.[i]?.trim();
    return val ? val : def;
  });

  if (activeView === "library") {
    renderLibraryGrid();
  }

  applyMenuAppearance(st.popover_opacity ?? 0.92, st.popover_blur ?? 20);

  if (st.audio) {
    chkAudioEnabled.checked = st.audio.enabled;
    audioControlsGroup.hidden = !st.audio.enabled;
    const volPct = Math.round(st.audio.volume * 100);
    sliderVolume.value = String(volPct);
    sliderVolume.setAttribute("aria-valuetext", `${volPct}%`);
    valVolume.textContent = `${volPct}%`;
    chkAudioDirectMode.checked = st.audio.direct_mode_only;
  }
  sliderPassive.value = String(st.frame.passive_fps);
  sliderPassive.setAttribute("aria-valuetext", `${st.frame.passive_fps} fps`);
  valPassive.textContent = `${st.frame.passive_fps} fps`;

  sliderLowPower.value = String(st.frame.low_power_fps);
  sliderLowPower.setAttribute("aria-valuetext", `${st.frame.low_power_fps} fps`);
  valLowPower.textContent = `${st.frame.low_power_fps} fps`;

  chkBattery.checked = st.frame.pause_on_battery;

  const interactionHk = st.hotkey?.trim() || DEFAULT_HOTKEY;
  const popoverHk = st.popover_hotkey?.trim() || DEFAULT_POPOVER_HOTKEY;
  kbdHotkey.textContent = formatShortcutForDisplay(interactionHk);

  if (recordingTarget?.kind !== "interaction") {
    btnRecordHotkey.textContent = formatShortcutForDisplay(interactionHk);
    btnRecordHotkey.classList.remove("recording");
    btnRecordHotkey.setAttribute("aria-pressed", "false");
    btnRecordHotkey.title = `Mouse Interaction shortcut (${interactionHk})`;
  }

  if (recordingTarget?.kind !== "popover") {
    btnRecordPopoverHotkey.textContent = formatShortcutForDisplay(popoverHk);
    btnRecordPopoverHotkey.classList.remove("recording");
    btnRecordPopoverHotkey.setAttribute("aria-pressed", "false");
    btnRecordPopoverHotkey.title = `Menu at Cursor shortcut (${popoverHk})`;
  }

  renderFavoriteHotkeysList();

  selectScheduleMode.value = st.schedule_mode;
  sliderInterval.value = String(st.schedule_interval_mins);
  sliderInterval.setAttribute("aria-valuetext", `${st.schedule_interval_mins} mins`);
  valInterval.textContent = `${st.schedule_interval_mins} mins`;
  const isInterval = st.schedule_mode === "interval";
  intervalGroup.hidden = !isInterval;
  sliderInterval.hidden = !isInterval;

  if (st.update) {
    chkAutoCheckUpdates.checked = st.update.auto_check;
    chkAutoDownloadUpdates.checked = st.update.auto_download;
  }

  if (typeof st.debug_mode === "boolean") {
    chkDebugMode.checked = st.debug_mode;
    badgeDebugStatus.textContent = st.debug_mode ? "Active" : "Passive";
    badgeDebugStatus.classList.toggle("active", st.debug_mode);
  }
}

// Living Rookery Action Handlers
btnFeed.addEventListener("click", () => {
  runActionWithFeedback(
    btnFeed,
    "grains",
    "Snacks scattered across the boughs!",
    () => api.feedBirds([500.0, 400.0]),
  );
});

btnPrune.addEventListener("click", () => {
  runActionWithFeedback(
    btnPrune,
    "scissors",
    "Outer canopy pruned gently!",
    () => api.pruneBranch([500.0, 300.0], 80.0),
  );
});

btnPlant.addEventListener("click", () => {
  runActionWithFeedback(
    btnPlant,
    "plant",
    "Sapling seed planted at the roots!",
    () => api.plantSeed([500.0, 750.0]),
  );
});

btnPanLeft.addEventListener("click", () => {
  runActionWithFeedback(
    btnPanLeft,
    "caret-left",
    "Forest horizon panned west",
    () => api.panForest(-80.0),
  );
});

btnPanRight.addEventListener("click", () => {
  runActionWithFeedback(
    btnPanRight,
    "caret-right",
    "Forest horizon panned east",
    () => api.panForest(80.0),
  );
});

selectRookeryStyle.addEventListener("change", async () => {
  const styleIdx = Number(selectRookeryStyle.value);
  const optText =
    selectRookeryStyle.options[selectRookeryStyle.selectedIndex]?.textContent ??
    `Palette ${styleIdx + 1}`;
  showNotification("palette", `Palette: ${optText}`);
  try {
    const s = await api.executeAppAction("living-rookery", "set_style", {
      value: styleIdx,
    });
    renderStatus(s);
  } catch (err) {
    console.error("set_style error:", err);
  }
});

btnStylePrev.addEventListener("click", () => {
  runActionWithFeedback(
    btnStylePrev,
    "palette",
    "Shifting palette…",
    () => api.executeAppAction("living-rookery", "prev_style"),
    (s) => `Palette: ${s.rookery?.aesthetic_style_name ?? "Previous Palette"}`,
  );
});

btnStyleNext.addEventListener("click", () => {
  runActionWithFeedback(
    btnStyleNext,
    "palette",
    "Shifting palette…",
    () => api.executeAppAction("living-rookery", "cycle_style"),
    (s) => `Palette: ${s.rookery?.aesthetic_style_name ?? "Next Palette"}`,
  );
});

// Koi Pond Action Handlers
btnKoiFeed.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiFeed,
    "pellets",
    "Nourishing pellets dropped into the lagoon!",
    () =>
      api.executeAppAction("koi-pond", "feed", {
        pos: [500.0, 375.0],
      }),
  );
});

btnKoiPebble.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiPebble,
    "ripple",
    "Smooth pebble skipped across the water!",
    () =>
      api.executeAppAction("koi-pond", "pebble", {
        pos: [500.0, 375.0],
        radius: 80.0,
      }),
  );
});

btnKoiSpawn.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiSpawn,
    "koi",
    "A new Nishikigoi glided into the pond!",
    () => api.executeAppAction("koi-pond", "spawn_koi", {}),
  );
});

btnKoiBloom.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiBloom,
    "flower-lotus",
    "Lotus blossoms unfurling across the water!",
    () => api.executeAppAction("koi-pond", "bloom_lotuses", {}),
  );
});

btnKoiRain.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiRain,
    "drop",
    "Toggled gentle garden rain shower!",
    () => api.executeAppAction("koi-pond", "toggle_rain", {}),
  );
});

btnKoiLanterns.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiLanterns,
    "sparkle",
    "Toggled floating tōrō paper lanterns!",
    () => api.executeAppAction("koi-pond", "toggle_lanterns", {}),
  );
});

btnKoiSpout.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiSpout,
    "leaf",
    "Shishi-odoshi bamboo spout tipped!",
    () => api.executeAppAction("koi-pond", "trigger_spout", {}),
  );
});

btnKoiTheme.addEventListener("click", () => {
  runActionWithFeedback(
    btnKoiTheme,
    "palette",
    "Shifted koi pond garden atmosphere!",
    () => api.executeAppAction("koi-pond", "cycle_theme", {}),
  );
});

const sendKoiColorSpeed = createCoalescedCaller(async (val: number) => {
  try {
    const s = await api.executeAppAction("koi-pond", "color_speed", {
      value: val,
    });
    renderStatus(s);
  } catch (e) {
    console.error("koi color_speed slider error:", e);
  }
});

sliderKoiColorSpeed.addEventListener("input", () => {
  const label = `${sliderKoiColorSpeed.value}x`;
  valKoiColorSpeed.textContent = label;
  sliderKoiColorSpeed.setAttribute("aria-valuetext", label);
  sendKoiColorSpeed(Number(sliderKoiColorSpeed.value));
});

sliderKoiColorSpeed.addEventListener("change", () => {
  sendKoiColorSpeed(Number(sliderKoiColorSpeed.value));
});

// Cosmic Nebula Action Handlers
btnNebulaIgnite.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaIgnite,
    "sparkle",
    "Collapsing interstellar gas into a new protostar!",
    () => api.executeAppAction("cosmic-nebula", "ignite_star", {}),
  );
});

btnNebulaSupernova.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaSupernova,
    "nova",
    "Supernova shockwave sweeping the nursery!",
    () => api.executeAppAction("cosmic-nebula", "trigger_supernova", {}),
  );
});

btnNebulaComet.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaComet,
    "compass",
    "Dual-tail comet summoned across the nebula!",
    () => api.executeAppAction("cosmic-nebula", "summon_comet", {}),
  );
});

btnNebulaMeteors.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaMeteors,
    "lightning",
    "Meteor shower streaking across the deep field!",
    () => api.executeAppAction("cosmic-nebula", "meteor_shower", {}),
  );
});

btnNebulaChart.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaChart,
    "star-four",
    "Toggled Astral Cartography star chart!",
    () => api.executeAppAction("cosmic-nebula", "toggle_chart", {}),
  );
});

btnNebulaPalette.addEventListener("click", () => {
  runActionWithFeedback(
    btnNebulaPalette,
    "palette",
    "Shifted cosmic nebula region!",
    () => api.executeAppAction("cosmic-nebula", "cycle_palette", {}),
  );
});

const sendNebulaColorSpeed = createCoalescedCaller(async (val: number) => {
  try {
    const s = await api.executeAppAction("cosmic-nebula", "color_speed", {
      value: val,
    });
    renderStatus(s);
  } catch (e) {
    console.error("nebula color_speed slider error:", e);
  }
});

sliderNebulaColorSpeed.addEventListener("input", () => {
  const label = `${sliderNebulaColorSpeed.value}x`;
  valNebulaColorSpeed.textContent = label;
  sliderNebulaColorSpeed.setAttribute("aria-valuetext", label);
  sendNebulaColorSpeed(Number(sliderNebulaColorSpeed.value));
});

sliderNebulaColorSpeed.addEventListener("change", () => {
  sendNebulaColorSpeed(Number(sliderNebulaColorSpeed.value));
});

// Orbital Haven Direct Controls & Action Handlers
for (const btn of [btnOrbitalLayoutSingle, btnOrbitalLayoutPip, btnOrbitalLayoutGrid]) {
  btn.addEventListener("click", () => {
    const layoutSlug = btn.dataset.layout || "pip";
    runActionWithFeedback(
      btn,
      "hexagon",
      "Switching monitor layout…",
      () => api.executeAppAction("orbital-haven", `set_layout:${layoutSlug}`),
      (s) => `Monitor layout: ${s.orbital?.layout || layoutSlug}`,
    );
  });
}

const bindOrbitalSelect = (
  sel: HTMLSelectElement,
  actionPrefix: string,
  feedbackPrefix: string,
) => {
  sel.addEventListener("change", () => {
    const val = sel.value;
    void api
      .executeAppAction("orbital-haven", `${actionPrefix}${val}`)
      .then((s) => {
        renderStatus(s);
        const label = sel.selectedOptions[0]?.textContent || val;
        showNotification("satellite", `${feedbackPrefix}: ${label}`);
      })
      .catch((err) => showNotification("warning", String(err)));
  });
};

bindOrbitalSelect(selectOrbitalPrimary, "select_camera:", "Primary feed");
bindOrbitalSelect(selectOrbitalPip0, "set_pip:0:", "PiP Top");
bindOrbitalSelect(selectOrbitalPip1, "set_pip:1:", "PiP Bottom");
bindOrbitalSelect(selectOrbitalGrid0, "set_grid:0:", "Top-Left quadrant");
bindOrbitalSelect(selectOrbitalGrid1, "set_grid:1:", "Top-Right quadrant");
bindOrbitalSelect(selectOrbitalGrid2, "set_grid:2:", "Bottom-Left quadrant");
bindOrbitalSelect(selectOrbitalGrid3, "set_grid:3:", "Bottom-Right quadrant");
bindOrbitalSelect(selectOrbitalPalette, "set_palette:", "Visual theme");

btnOrbitalDirector.addEventListener("click", () => {
  runActionWithFeedback(
    btnOrbitalDirector,
    "satellite",
    "Toggling Auto-Director…",
    () => api.executeAppAction("orbital-haven", "toggle_director"),
    (s) =>
      s.orbital?.auto_director
        ? "Auto-Director enabled"
        : "Auto-Director locked on current feed",
  );
});

btnOrbitalLaunch.addEventListener("click", () => {
  runActionWithFeedback(
    btnOrbitalLaunch,
    "rocket-launch",
    "Inbound spacecraft cleared for HELIOS-09 approach!",
    () => api.executeAppAction("orbital-haven", "dispatch_ship"),
  );
});

btnOrbitalScan.addEventListener("click", () => {
  runActionWithFeedback(
    btnOrbitalScan,
    "lightning",
    "Updating station operational state…",
    () => api.executeAppAction("orbital-haven", "trigger_event"),
    (s) => `Station status: ${s.orbital?.station_event || "Updated"}`,
  );
});

// Schedule Mode Selection
selectScheduleMode.addEventListener("change", () => {
  const mode = selectScheduleMode.value as ScheduleMode;
  void patchSettings({ schedule_mode: mode }, "schedule_mode");
});

sliderInterval.addEventListener("input", () => {
  const label = `${sliderInterval.value} mins`;
  sliderInterval.setAttribute("aria-valuetext", label);
  valInterval.textContent = label;
});

sliderInterval.addEventListener("change", () => {
  const mins = parseInt(sliderInterval.value, 10);
  void patchSettings({ schedule_interval_mins: mins }, "schedule_interval_mins");
});

// Share & Capture
btnQuickSnapshot.addEventListener("click", async () => {
  try {
    btnQuickSnapshot.disabled = true;
    const path = await api.captureSnapshot();
    showNotification("camera", `Snapshot saved: ${path}`);
  } catch (err) {
    showNotification("warning", `Snapshot failed: ${err}`, { isError: true });
  } finally {
    btnQuickSnapshot.disabled = false;
  }
});

btnSnapshot.addEventListener("click", async () => {
  try {
    btnSnapshot.disabled = true;
    const path = await api.captureSnapshot();
    showShareNotification("camera", `Snapshot saved: ${path}`);
  } catch (err) {
    showShareNotification("warning", `Snapshot failed: ${err}`, true);
  } finally {
    btnSnapshot.disabled = false;
  }
});

btnRecordLoop.addEventListener("click", async () => {
  try {
    btnRecordLoop.disabled = true;
    showShareNotification("film-strip", "Recording 5s loop…");
    const path = await api.recordLoop();
    showShareNotification("film-strip", `GIF saved: ${path}`);
  } catch (err) {
    showShareNotification("warning", `Record loop failed: ${err}`, true);
  } finally {
    btnRecordLoop.disabled = false;
  }
});

btnRecordVideo.addEventListener("click", async () => {
  try {
    btnRecordVideo.disabled = true;
    showShareNotification("video-camera", "Recording 5s 60 FPS MP4…");
    const path = await api.recordVideo();
    showShareNotification("video-camera", `Video saved: ${path}`);
  } catch (err) {
    showShareNotification("warning", `Record video failed: ${err}`, true);
  } finally {
    btnRecordVideo.disabled = false;
  }
});

btnCopySeed.addEventListener("click", async () => {
  try {
    const uri = await api.getWorldSeed();
    await navigator.clipboard.writeText(uri);
    showShareNotification("plant", "Copied World Seed URI!");
  } catch (err) {
    showShareNotification("warning", `Copy seed failed: ${err}`, true);
  }
});

btnCopyState.addEventListener("click", async () => {
  try {
    const uri = await api.getWorldStateString();
    await navigator.clipboard.writeText(uri);
    showShareNotification("package", "Copied full World State snapshot!");
  } catch (err) {
    showShareNotification("warning", `Copy state failed: ${err}`, true);
  }
});

formImportWorld.addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const text = inputImportWorld.value.trim();
  if (!text) return;
  btnImportWorld.disabled = true;
  try {
    const s = await api.importWorldString(text);
    renderStatus(s);
    inputImportWorld.value = "";
    showShareNotification("check-circle", "World imported successfully!");
  } catch (err) {
    showShareNotification("warning", `Import failed: ${err}`, true);
  } finally {
    btnImportWorld.disabled = false;
  }
});

// Interactive & Pause (Optimistic immediate toggle)
btnInteractive.addEventListener("click", async () => {
  if (!currentStatus) return;
  const nextInteractive = !currentStatus.interactive;
  currentStatus.interactive = nextInteractive;
  btnInteractive.classList.toggle("active", nextInteractive);
  lblInteractive.textContent = nextInteractive
    ? "Desktop Interactive"
    : "Interact on Desktop";
  try {
    const s = await api.setInteractive(nextInteractive);
    renderStatus(s);
  } catch (err) {
    console.error("setInteractive error:", err);
  }
});

btnPause.addEventListener("click", async () => {
  if (!currentStatus) return;
  const nextPaused = !currentStatus.user_paused;
  currentStatus.user_paused = nextPaused;
  setIcon(icoPause, nextPaused ? "play" : "pause");
  lblPause.textContent = nextPaused ? "Resume" : "Pause";
  try {
    const s = await api.setPaused(nextPaused);
    renderStatus(s);
  } catch (err) {
    console.error("setPaused error:", err);
  }
});

// Settings & Hotkeys
sliderPassive.addEventListener("input", () => {
  const label = `${sliderPassive.value} fps`;
  sliderPassive.setAttribute("aria-valuetext", label);
  valPassive.textContent = label;
});
sliderPassive.addEventListener("change", () => {
  void patchSettings({ passive_fps: Number(sliderPassive.value) }, "passive_fps");
});

sliderLowPower.addEventListener("input", () => {
  const label = `${sliderLowPower.value} fps`;
  sliderLowPower.setAttribute("aria-valuetext", label);
  valLowPower.textContent = label;
});
sliderLowPower.addEventListener("change", () => {
  void patchSettings({ low_power_fps: Number(sliderLowPower.value) }, "low_power_fps");
});

chkBattery.addEventListener("change", () => {
  void patchSettings({ pause_on_battery: chkBattery.checked }, "pause_on_battery");
});

// Menu Appearance Listeners
sliderMenuOpacity.addEventListener("input", () => {
  const pct = Number(sliderMenuOpacity.value);
  popoverRoot.style.setProperty("--menu-opacity", String(pct / 100));
  sliderMenuOpacity.setAttribute("aria-valuetext", `${pct}%`);
  valMenuOpacity.textContent = `${pct}%`;
});

sliderMenuOpacity.addEventListener("change", () => {
  void patchSettings(
    { popover_opacity: Number(sliderMenuOpacity.value) / 100 },
    "popover_opacity",
  );
});

const sendPopoverBlurCoalesced = createCoalescedCaller(async (blur: number) => {
  await api.setPopoverBlur(blur);
});

sliderMenuBlur.addEventListener("input", () => {
  const blur = Number(sliderMenuBlur.value);
  popoverRoot.style.setProperty("--menu-blur", `${blur}px`);
  sliderMenuBlur.setAttribute("aria-valuetext", `${blur}px`);
  valMenuBlur.textContent = `${blur}px`;
  sendPopoverBlurCoalesced(blur);
});

sliderMenuBlur.addEventListener("change", () => {
  void patchSettings({ popover_blur: Number(sliderMenuBlur.value) }, "popover_blur");
});

// Audio Subsystem Listeners
chkAudioEnabled.addEventListener("change", () => {
  audioControlsGroup.hidden = !chkAudioEnabled.checked;
  void patchSettings({ audio_enabled: chkAudioEnabled.checked }, "audio_enabled");
});

sliderVolume.addEventListener("input", () => {
  const label = `${sliderVolume.value}%`;
  sliderVolume.setAttribute("aria-valuetext", label);
  valVolume.textContent = label;
});

sliderVolume.addEventListener("change", () => {
  void patchSettings(
    { audio_volume: Number(sliderVolume.value) / 100 },
    "audio_volume",
  );
});

chkAudioDirectMode.addEventListener("change", () => {
  void patchSettings(
    { audio_direct_mode_only: chkAudioDirectMode.checked },
    "audio_direct_mode_only",
  );
});

function sameHotkeyTarget(a: HotkeyTarget | null, b: HotkeyTarget | null): boolean {
  if (!a || !b) return false;
  if (a.kind !== b.kind) return false;
  if (a.kind === "favorite" && b.kind === "favorite") {
    return a.index === b.index;
  }
  return true;
}

function getRealmDisplayName(appId: string): string {
  const fromCatalog = appCatalog.find((m) => m.metadata.id === appId)?.metadata.name;
  if (fromCatalog) return fromCatalog;
  if (appThemes[appId]?.title) return appThemes[appId].title;
  return appId.replace(/-/g, " ");
}

function renderFavoriteHotkeysList(): void {
  favoriteHotkeysList.replaceChildren();

  const pinned = favoriteApps.slice(0, FAVORITE_HOTKEY_SLOTS);
  if (pinned.length === 0) {
    const emptyLi = document.createElement("li");
    emptyLi.className = "favorite-hotkeys-empty";
    emptyLi.textContent = `Star realms in the Realm Library to pin quick-jump hotkeys (${formatShortcutForDisplay("CommandOrControl+Alt+Shift+1")}–9).`;
    favoriteHotkeysList.appendChild(emptyLi);
    return;
  }

  pinned.forEach((appId, idx) => {
    const slotNum = idx + 1;
    const realmName = getRealmDisplayName(appId);
    const shortcut =
      favoriteHotkeys[idx] || `CommandOrControl+Alt+Shift+${slotNum}`;
    const isRecordingThis =
      recordingTarget?.kind === "favorite" && recordingTarget.index === idx;

    const li = document.createElement("li");
    li.className = "hotkey-row";

    const labelWrap = document.createElement("span");
    labelWrap.className = "favorite-hotkey-label";
    labelWrap.id = `lbl-hotkey-fav-${slotNum}`;

    const slotBadge = document.createElement("span");
    slotBadge.className = "favorite-slot-badge";
    slotBadge.textContent = `#${slotNum}`;

    const nameSpan = document.createElement("span");
    nameSpan.className = "favorite-realm-name";
    nameSpan.textContent = realmName;
    nameSpan.title = realmName;

    labelWrap.append(slotBadge, nameSpan);

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `hotkey-btn${isRecordingThis ? " recording" : ""}`;
    btn.id = `btn-record-fav-hotkey-${slotNum}`;
    btn.setAttribute("aria-pressed", String(isRecordingThis));
    btn.setAttribute("aria-labelledby", `${labelWrap.id} ${btn.id}`);
    btn.title = `Jump to ${realmName} (${shortcut})`;
    btn.textContent = isRecordingThis
      ? "Press shortcut… (Esc)"
      : formatShortcutForDisplay(shortcut);

    btn.addEventListener("click", () => {
      toggleRecordingHotkey({ kind: "favorite", index: idx });
    });

    li.append(labelWrap, btn);
    favoriteHotkeysList.appendChild(li);
  });
}

function restoreHotkeyButtons(): void {
  const interactionHk = currentSettings?.hotkey?.trim() || DEFAULT_HOTKEY;
  const popoverHk = currentSettings?.popover_hotkey?.trim() || DEFAULT_POPOVER_HOTKEY;

  btnRecordHotkey.classList.remove("recording");
  btnRecordHotkey.setAttribute("aria-pressed", "false");
  btnRecordHotkey.textContent = formatShortcutForDisplay(interactionHk);

  btnRecordPopoverHotkey.classList.remove("recording");
  btnRecordPopoverHotkey.setAttribute("aria-pressed", "false");
  btnRecordPopoverHotkey.textContent = formatShortcutForDisplay(popoverHk);

  renderFavoriteHotkeysList();
}

function cancelRecordingHotkey(notifyBackend = true): void {
  if (!recordingTarget) return;
  recordingTarget = null;
  restoreHotkeyButtons();
  if (notifyBackend) {
    api.setHotkeyRecording(false).catch(() => {});
  }
}

function toggleRecordingHotkey(target: HotkeyTarget): void {
  if (sameHotkeyTarget(recordingTarget, target)) {
    cancelRecordingHotkey(true);
    return;
  }

  const wasRecording = recordingTarget !== null;
  recordingTarget = target;
  hotkeyError.hidden = true;
  restoreHotkeyButtons();

  if (target.kind === "interaction") {
    btnRecordHotkey.classList.add("recording");
    btnRecordHotkey.setAttribute("aria-pressed", "true");
    btnRecordHotkey.textContent = "Press shortcut… (Esc)";
  } else if (target.kind === "popover") {
    btnRecordPopoverHotkey.classList.add("recording");
    btnRecordPopoverHotkey.setAttribute("aria-pressed", "true");
    btnRecordPopoverHotkey.textContent = "Press shortcut… (Esc)";
  } else {
    renderFavoriteHotkeysList();
  }

  if (!wasRecording) {
    api.setHotkeyRecording(true).catch((err) => {
      console.warn("setHotkeyRecording(true) failed:", err);
    });
  }
}

function codeToTauriKey(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return `Num${code.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;

  const specialMap: Record<string, string> = {
    Space: "Space",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Minus: "Minus",
    Equal: "Equal",
    BracketLeft: "BracketLeft",
    BracketRight: "BracketRight",
    Backslash: "Backslash",
    Semicolon: "Semicolon",
    Quote: "Quote",
    Comma: "Comma",
    Period: "Period",
    Slash: "Slash",
    Backquote: "Backquote",
  };
  return specialMap[code] ?? null;
}

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "OSLeft",
  "OSRight",
]);

function eventToAccelerator(ev: KeyboardEvent):
  | { status: "ignore" }
  | { status: "cancel" }
  | { status: "invalid"; reason: string }
  | { status: "ok"; shortcut: string } {
  if (
    MODIFIER_CODES.has(ev.code) ||
    ["Control", "Alt", "Shift", "Meta"].includes(ev.key)
  ) {
    return { status: "ignore" };
  }

  if (ev.code === "Escape" || ev.key === "Escape") {
    return { status: "cancel" };
  }

  const keyToken = codeToTauriKey(ev.code);
  if (!keyToken) {
    return {
      status: "invalid",
      reason: `Unsupported key (${ev.code || ev.key}). Try a letter, number, or function key.`,
    };
  }

  const hasModifier = ev.metaKey || ev.ctrlKey || ev.altKey || ev.shiftKey;
  const isFunctionKey = /^F([1-9]|1[0-9]|2[0-4])$/.test(keyToken);
  if (!hasModifier && !isFunctionKey) {
    return {
      status: "invalid",
      reason: "Include at least one modifier (⌘/Ctrl, ⌥/Alt, or ⇧Shift).",
    };
  }

  const parts: string[] = [];
  if (ev.metaKey || ev.ctrlKey) parts.push("CommandOrControl");
  if (ev.altKey) parts.push("Alt");
  if (ev.shiftKey) parts.push("Shift");
  parts.push(keyToken);

  return { status: "ok", shortcut: parts.join("+") };
}

// Interaction Hotkey Recording
btnRecordHotkey.addEventListener("click", () => {
  toggleRecordingHotkey({ kind: "interaction" });
});

// Popover Toggle Hotkey Recording
btnRecordPopoverHotkey.addEventListener("click", () => {
  toggleRecordingHotkey({ kind: "popover" });
});

// Reset All Hotkeys to Defaults
btnResetHotkeys.addEventListener("click", async () => {
  cancelRecordingHotkey(false);
  hotkeyError.hidden = true;
  btnResetHotkeys.disabled = true;
  try {
    const st = await api.resetHotkeys();
    renderSettings(st);
  } catch (err) {
    hotkeyError.textContent = `Could not reset hotkeys: ${err}`;
    hotkeyError.hidden = false;
  } finally {
    btnResetHotkeys.disabled = false;
  }
});

window.addEventListener("blur", () => {
  cancelRecordingHotkey(true);
});

window.addEventListener("keydown", async (ev) => {
  if (!recordingTarget) {
    if (ev.key === "Escape" && activeView !== "realm") {
      ev.preventDefault();
      const prev = activeView;
      setActiveView("realm");
      if (prev === "library") {
        btnRealmSelector.focus();
      } else {
        btnToggleSettings.focus();
      }
    }
    return;
  }

  const result = eventToAccelerator(ev);
  if (result.status === "ignore") return;

  ev.preventDefault();
  ev.stopPropagation();

  if (result.status === "cancel") {
    cancelRecordingHotkey(true);
    return;
  }

  if (result.status === "invalid") {
    hotkeyError.textContent = result.reason;
    hotkeyError.hidden = false;
    return;
  }

  const target = recordingTarget;
  const shortcut = result.shortcut;
  recordingTarget = null;
  hotkeyError.hidden = true;

  try {
    let st: Settings;
    if (target.kind === "interaction") {
      st = await api.updateSettings({ hotkey: shortcut });
    } else if (target.kind === "popover") {
      st = await api.updateSettings({ popover_hotkey: shortcut });
    } else {
      const nextFavs = [...favoriteHotkeys];
      nextFavs[target.index] = shortcut;
      st = await api.updateSettings({ favorite_hotkeys: nextFavs });
    }
    renderSettings(st);
  } catch (err) {
    hotkeyError.textContent = `Could not bind ${formatShortcutForDisplay(shortcut)}: ${err}`;
    hotkeyError.hidden = false;
    try {
      await api.setHotkeyRecording(false);
    } catch {}
    try {
      const st = await api.getSettings();
      renderSettings(st);
    } catch {
      restoreHotkeyButtons();
    }
  }
});

btnQuit.addEventListener("click", () => {
  api.quit().catch((err) => console.error("quit error:", err));
});

// Update Controls Listeners
chkAutoCheckUpdates.addEventListener("change", () => {
  void patchSettings(
    { update_auto_check: chkAutoCheckUpdates.checked },
    "update_auto_check",
  );
});

chkAutoDownloadUpdates.addEventListener("change", () => {
  void patchSettings(
    { update_auto_download: chkAutoDownloadUpdates.checked },
    "update_auto_download",
  );
});

btnCheckUpdates.addEventListener("click", async () => {
  try {
    await updater.checkForUpdates(false);
  } catch (err) {
    console.error("Check for updates error:", err);
  }
});

btnDownloadUpdate.addEventListener("click", async () => {
  btnDownloadUpdate.disabled = true;
  try {
    await updater.downloadAndInstall();
  } catch (err) {
    console.error("Download update error:", err);
    btnDownloadUpdate.disabled = false;
  }
});

btnRestartUpdate.addEventListener("click", async () => {
  btnRestartUpdate.disabled = true;
  btnRestartUpdate.textContent = "Restarting…";
  try {
    await updater.restartApp();
  } catch (err) {
    console.error("Restart error:", err);
    btnRestartUpdate.disabled = false;
    btnRestartUpdate.textContent = "Restart Rookery";
  }
});

updater.subscribe((state) => {
  if (state.currentVersion) {
    updateStatusText.textContent = `Rookery v${state.currentVersion}`;
  }

  if (state.checking) {
    btnCheckUpdates.disabled = true;
    btnCheckUpdates.textContent = "Checking…";
    updateDetailText.textContent = "Checking for new releases…";
    return;
  }

  btnCheckUpdates.disabled = false;
  btnCheckUpdates.textContent = "Check Updates";

  if (state.installed) {
    updateActionPanel.hidden = false;
    updateBadge.hidden = false;
    updateDetailText.textContent = "Ready to install!";
    updateBadge.textContent = `v${state.version || ""} Ready`;
    updateNotes.textContent = `Update ${state.version || ""} has been downloaded and installed. Restart Rookery to complete the update.`;
    btnDownloadUpdate.hidden = true;
    btnRestartUpdate.hidden = false;
    updateProgressContainer.hidden = true;
  } else if (state.downloading) {
    updateActionPanel.hidden = false;
    updateBadge.hidden = false;
    updateDetailText.textContent = "Downloading update…";
    btnDownloadUpdate.hidden = true;
    btnRestartUpdate.hidden = true;
    updateProgressContainer.hidden = false;
    updateProgressFill.style.width = `${state.downloadProgress}%`;
    updateProgressText.textContent = `Downloading… ${state.downloadProgress}%`;
  } else if (state.available) {
    updateActionPanel.hidden = false;
    updateBadge.hidden = false;
    updateDetailText.textContent = `v${state.version} available`;
    updateBadge.textContent = `v${state.version} Available`;
    updateNotes.textContent =
      state.body || "A new release of Rookery is available.";
    btnDownloadUpdate.hidden = false;
    btnDownloadUpdate.disabled = false;
    btnRestartUpdate.hidden = true;
    updateProgressContainer.hidden = true;
  } else if (state.error) {
    updateActionPanel.hidden = true;
    updateBadge.hidden = true;
    updateDetailText.textContent =
      import.meta.env.DEV &&
      (state.error.includes("status code") || state.error.includes("404"))
        ? "No release published yet (dev mode)"
        : state.error.includes("404") ||
            state.error.includes("network") ||
            state.error.includes("connect")
          ? "Could not reach update server"
          : `Update check error: ${state.error}`;
  } else {
    updateActionPanel.hidden = true;
    updateBadge.hidden = true;
    updateDetailText.textContent = import.meta.env.DEV
      ? "Development build"
      : "Up to date";
  }
});

// =================================================================
// Debug Mode & Tree Inspector Action Listeners
// =================================================================
chkDebugMode.addEventListener("change", async () => {
  const enabled = chkDebugMode.checked;
  badgeDebugStatus.textContent = enabled ? "Active" : "Passive";
  badgeDebugStatus.classList.toggle("active", enabled);
  const ok = await patchSettings({ debug_mode: enabled }, "debug_mode");
  if (ok) {
    showNotification(
      "code",
      enabled ? "Debug Mode & in-scene telemetry active" : "Debug Mode disabled",
    );
  }
});

btnDebugRerollSeed.addEventListener("click", () => {
  const newSeed = Math.floor(Math.random() * 9000000) + 1000;
  inputDebugSeed.value = String(newSeed);
  showNotification("arrow-clockwise", `New seed: #${newSeed}`);
});

btnDebugViewMature.addEventListener("click", async () => {
  const species = selectDebugSpecies.value;
  const seed = parseInt(inputDebugSeed.value, 10) || 42;
  try {
    const s = await api.executeAppAction("living-rookery", "view_mature_tree", {
      species,
      seed,
    });
    renderStatus(s);
    showNotification("tree-evergreen", `Spawned mature ${species} (Seed #${seed})`);
  } catch (err) {
    console.error("view mature tree error:", err);
  }
});

btnDebugNextVariation.addEventListener("click", async () => {
  const species = selectDebugSpecies.value;
  const newSeed = Math.floor(Math.random() * 9000000) + 1000;
  inputDebugSeed.value = String(newSeed);
  try {
    const s = await api.executeAppAction("living-rookery", "view_mature_tree", {
      species,
      seed: newSeed,
    });
    renderStatus(s);
    showNotification("sparkle", `New mature ${species} variation (Seed #${newSeed})`);
  } catch (err) {
    console.error("next variation error:", err);
  }
});

btnDebugShowcaseAll.addEventListener("click", async () => {
  const seed = parseInt(inputDebugSeed.value, 10) || 42;
  try {
    const s = await api.executeAppAction(
      "living-rookery",
      "showcase_all_mature",
      { seed },
    );
    renderStatus(s);
    showNotification("plant", "Showcase of all 9 mature species generated!");
  } catch (err) {
    console.error("showcase all error:", err);
  }
});

btnDebugVarietyGrove.addEventListener("click", async () => {
  const species = selectDebugSpecies.value;
  const seed = parseInt(inputDebugSeed.value, 10) || 42;
  try {
    const s = await api.executeAppAction("living-rookery", "variety_grove", {
      species,
      seed,
    });
    renderStatus(s);
    showNotification(
      "branch",
      `Grove of 4 mature ${species} variations generated!`,
    );
  } catch (err) {
    console.error("variety grove error:", err);
  }
});

btnDebugPrevTree.addEventListener("click", async () => {
  const currentIndex = selectDebugForestTree.selectedIndex;
  if (currentIndex > 0) {
    selectDebugForestTree.selectedIndex = currentIndex - 1;
    const tid = parseInt(selectDebugForestTree.value, 10);
    try {
      const s = await api.executeAppAction("living-rookery", "focus_tree", {
        tree_id: tid,
      });
      renderStatus(s);
    } catch (err) {
      console.error("focus tree error:", err);
    }
  }
});

btnDebugNextTree.addEventListener("click", async () => {
  const currentIndex = selectDebugForestTree.selectedIndex;
  if (
    currentIndex >= 0 &&
    currentIndex < selectDebugForestTree.options.length - 1
  ) {
    selectDebugForestTree.selectedIndex = currentIndex + 1;
    const tid = parseInt(selectDebugForestTree.value, 10);
    try {
      const s = await api.executeAppAction("living-rookery", "focus_tree", {
        tree_id: tid,
      });
      renderStatus(s);
    } catch (err) {
      console.error("focus tree error:", err);
    }
  }
});

selectDebugForestTree.addEventListener("change", async () => {
  const tid = parseInt(selectDebugForestTree.value, 10);
  if (!isNaN(tid)) {
    try {
      const s = await api.executeAppAction("living-rookery", "focus_tree", {
        tree_id: tid,
      });
      renderStatus(s);
    } catch (err) {
      console.error("focus tree error:", err);
    }
  }
});

btnDebugMatureActive.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction(
      "living-rookery",
      "mature_active_tree",
    );
    renderStatus(s);
    showNotification("tree-evergreen", "Active tree fully matured!");
  } catch (err) {
    console.error("mature active tree error:", err);
  }
});

btnDebugStep1.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "growth_step", {
      count: 1,
    });
    renderStatus(s);
  } catch (err) {
    console.error("growth step error:", err);
  }
});

btnDebugStep10.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "growth_step", {
      count: 10,
    });
    renderStatus(s);
    showNotification("plant", "Fast-forwarded 10 growth steps!");
  } catch (err) {
    console.error("growth step error:", err);
  }
});

btnDebugRewind.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "rewind_sapling");
    renderStatus(s);
    showNotification("arrow-clockwise", "Active tree rewound to sapling!");
  } catch (err) {
    console.error("rewind sapling error:", err);
  }
});

for (const btn of speedButtons) {
  btn.addEventListener("click", async () => {
    const speed = parseFloat(btn.getAttribute("data-speed") || "1.0");
    for (const b of speedButtons) {
      b.classList.toggle("active", b === btn);
    }
    valDebugSpeed.textContent = formatSimSpeed(speed);
    try {
      const s = await api.executeAppAction("living-rookery", "set_sim_speed", {
        speed,
      });
      renderStatus(s);
    } catch (err) {
      console.error("set sim speed error:", err);
    }
  });
}

selectDebugPalette.addEventListener("change", async () => {
  const paletteIdx = parseInt(selectDebugPalette.value, 10);
  try {
    const s = await api.executeAppAction("living-rookery", "set_palette", {
      value: paletteIdx,
    });
    renderStatus(s);
  } catch (err) {
    console.error("set palette error:", err);
  }
});

selectDebugBirdStyle.addEventListener("change", async () => {
  const style = selectDebugBirdStyle.value;
  try {
    const s = await api.executeAppAction("living-rookery", "set_bird_style", {
      style,
    });
    renderStatus(s);
  } catch (err) {
    console.error("set bird style error:", err);
  }
});

btnSpawnRaven.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnRaven,
    "bird",
    "Celestial Raven summoned into canopy!",
    () =>
      api.executeAppAction("living-rookery", "spawn_bird", {
        species: "CelestialRaven",
      }),
  );
});

btnSpawnTanager.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnTanager,
    "bird",
    "Ruby Tanager arrived in forest!",
    () =>
      api.executeAppAction("living-rookery", "spawn_bird", {
        species: "RubyTanager",
      }),
  );
});

btnSpawnOwl.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnOwl,
    "bird",
    "Snowy Owl guardian appeared!",
    () =>
      api.executeAppAction("living-rookery", "spawn_bird", {
        species: "SnowyOwl",
      }),
  );
});

btnSpawnFox.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnFox,
    "sparkle",
    "Red Fox trotting across the meadow!",
    () => api.executeAppAction("living-rookery", "spawn_fauna", { species: "Fox" }),
  );
});

btnSpawnSquirrel.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnSquirrel,
    "sparkle",
    "Squirrel scampering through the clovers!",
    () => api.executeAppAction("living-rookery", "spawn_fauna", { species: "Squirrel" }),
  );
});

btnSpawnBadger.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnBadger,
    "sparkle",
    "Badger exploring near the roots!",
    () => api.executeAppAction("living-rookery", "spawn_fauna", { species: "Badger" }),
  );
});

btnSpawnHedgehog.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnHedgehog,
    "sparkle",
    "Prickly Hedgehog snuffling in the grass!",
    () => api.executeAppAction("living-rookery", "spawn_fauna", { species: "Hedgehog" }),
  );
});

btnSpawnRabbit.addEventListener("click", () => {
  runActionWithFeedback(
    btnSpawnRabbit,
    "sparkle",
    "Cottontail Rabbit hopped into the meadow!",
    () => api.executeAppAction("living-rookery", "spawn_fauna", { species: "Rabbit" }),
  );
});

btnScatterSnacks.addEventListener("click", () => {
  runActionWithFeedback(
    btnScatterSnacks,
    "grains",
    "Sweet berries & acorns scattered on the grass!",
    () => api.executeAppAction("living-rookery", "scatter_snacks"),
  );
});

let resetConfirmPending = false;
let resetConfirmTimer: number | null = null;

function clearResetConfirm(): void {
  resetConfirmPending = false;
  if (resetConfirmTimer !== null) {
    window.clearTimeout(resetConfirmTimer);
    resetConfirmTimer = null;
  }
  iconLabel(btnDebugResetWorld, "arrow-clockwise", "Reset World to Default");
}

btnDebugResetWorld.addEventListener("click", async () => {
  if (!resetConfirmPending) {
    resetConfirmPending = true;
    iconLabel(btnDebugResetWorld, "warning", "Click Again to Confirm Reset");
    resetConfirmTimer = window.setTimeout(clearResetConfirm, 3500);
    return;
  }

  clearResetConfirm();
  try {
    const s = await api.executeAppAction("living-rookery", "reset_world", {
      seed: 42,
    });
    inputDebugSeed.value = "42";
    selectDebugSpecies.value = "AncientOak";
    renderStatus(s);
    showNotification("arrow-clockwise", "World reset to default seed #42");
  } catch (err) {
    console.error("reset world error:", err);
  }
});

// Bootstrap: Instant hydration from cache, then live refresh
(() => {
  try {
    const cachedCatalog = localStorage.getItem("rookery_catalog");
    if (cachedCatalog) {
      appCatalog = JSON.parse(cachedCatalog);
      applyCatalog();
    }
    const cachedStatus = localStorage.getItem("rookery_status");
    if (cachedStatus) renderStatus(JSON.parse(cachedStatus));
    const cachedSettings = localStorage.getItem("rookery_settings");
    if (cachedSettings) renderSettings(JSON.parse(cachedSettings));
  } catch {
    // Ignore cache load errors
  }
})();

(async () => {
  try {
    const [status, settings, catalog] = await Promise.all([
      api.getStatus(),
      api.getSettings(),
      api.getCatalog(),
    ]);
    appCatalog = catalog;
    try {
      localStorage.setItem("rookery_catalog", JSON.stringify(catalog));
    } catch {}
    applyCatalog();
    renderStatus(status);
    renderSettings(settings);

    // Milestones: Auto-check and background download if configured (skip in dev mode)
    if (!import.meta.env.DEV && settings.update?.auto_check !== false) {
      updater.checkForUpdates(true).then((s) => {
        if (s.available && (settings.update?.auto_download ?? true)) {
          updater.downloadAndInstall().catch((err) => {
            console.warn("Background auto-download failed:", err);
          });
        }
      });
    }
  } catch (err) {
    console.error("bootstrap error:", err);
  }

  await api.onStatus((s) => renderStatus(s));
})();
