import {
  api,
  AppManifest,
  DisplayStatus,
  FrameBudget,
  HostStatus,
  ScheduleMode,
  Settings,
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
const settingsPanels: Record<string, HTMLElement> = {
  general: el<HTMLElement>("settings-panel-general"),
  controls: el<HTMLElement>("settings-panel-controls"),
  share: el<HTMLElement>("settings-panel-share"),
  debug: el<HTMLElement>("settings-panel-debug"),
};
const settingsNavBtns = document.querySelectorAll<HTMLButtonElement>(".settings-nav-btn");

for (const navBtn of settingsNavBtns) {
  navBtn.addEventListener("click", () => {
    const targetTab = navBtn.getAttribute("data-settings-tab") || "general";
    for (const b of settingsNavBtns) {
      b.classList.toggle("active", b === navBtn);
    }
    for (const [key, panel] of Object.entries(settingsPanels)) {
      panel.hidden = key !== targetTab;
    }
  });
}

type ActiveView = "realm" | "library" | "settings";
let activeView: ActiveView = "realm";
let activeLibraryFilter = "all";
let librarySearchQuery = "";
let favoriteApps: string[] = [];

function setActiveView(view: ActiveView): void {
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
  "--card",
  "--border-outer",
  "--accent",
  "--accent-secondary",
  "--crest-bg",
  "--ok",
  "--scrollbar-thumb",
  "--scrollbar-track",
];

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
    tagline: "Astral Singularity Forge",
    bannerTitle: "Astral Star-Chart Deck",
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

function applyThemeForApp(appId: string): void {
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
}

async function selectRealm(appId: string): Promise<void> {
  for (const [id, cardEl] of Object.entries(cards)) {
    cardEl.hidden = id !== appId;
  }
  applyThemeForApp(appId);
  setActiveView("realm");

  try {
    const s = await api.switchApp(appId);
    renderStatus(s);
  } catch (err) {
    console.error("switchApp error:", err);
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
const seedNotification = el<HTMLDivElement>("seed-notification");

// Koi Pond Stats & Controls
const statKoiCount = el<HTMLSpanElement>("stat-koi-count");
const statKoiLilies = el<HTMLSpanElement>("stat-koi-lilies");
const statKoiLotuses = el<HTMLSpanElement>("stat-koi-lotuses");
const statKoiRipples = el<HTMLSpanElement>("stat-koi-ripples");
const btnKoiFeed = el<HTMLButtonElement>("btn-koi-feed");
const btnKoiPebble = el<HTMLButtonElement>("btn-koi-pebble");

// Cosmic Nebula Stats & Controls
const statNebulaParticles = el<HTMLSpanElement>("stat-nebula-particles");
const statNebulaWells = el<HTMLSpanElement>("stat-nebula-wells");
const statNebulaPalette = el<HTMLSpanElement>("stat-nebula-palette");
const statNebulaTurb = el<HTMLSpanElement>("stat-nebula-turb");
const btnNebulaAttractor = el<HTMLButtonElement>("btn-nebula-attractor");
const btnNebulaRepulsor = el<HTMLButtonElement>("btn-nebula-repulsor");
const btnNebulaPalette = el<HTMLButtonElement>("btn-nebula-palette");

// Orbital Haven Stats & Controls
const statOrbitalModules = el<HTMLSpanElement>("stat-orbital-modules");
const statOrbitalSolar = el<HTMLSpanElement>("stat-orbital-solar");
const statOrbitalRings = el<HTMLSpanElement>("stat-orbital-rings");
const statOrbitalShuttles = el<HTMLSpanElement>("stat-orbital-shuttles");
const btnOrbitalModule = el<HTMLButtonElement>("btn-orbital-module");
const btnOrbitalLaunch = el<HTMLButtonElement>("btn-orbital-launch");
const btnOrbitalFocus = el<HTMLButtonElement>("btn-orbital-focus");
const btnOrbitalPlanet = el<HTMLButtonElement>("btn-orbital-planet");
const btnOrbitalScan = el<HTMLButtonElement>("btn-orbital-scan");

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
const inputImportWorld = el<HTMLInputElement>("input-import-world");
const btnImportWorld = el<HTMLButtonElement>("btn-import-world");
const shareNotification = el<HTMLDivElement>("share-notification");

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
const debugMsgStatus = el<HTMLSpanElement>("debug-msg-status");

let currentStatus: HostStatus | null = null;
let recordingInteractionHotkey = false;
let recordingPopoverHotkey = false;

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

  // Sort favorites first when viewing "all"
  const sorted = [...filtered].sort((a, b) => {
    const aFav = favoriteApps.includes(a.metadata.id) ? 0 : 1;
    const bFav = favoriteApps.includes(b.metadata.id) ? 0 : 1;
    return aFav - bFav;
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
        const sendSliderUpdate = async () => {
          try {
            const s = await api.executeAppAction(m.metadata.id, sl.id, {
              value: Number(input.value),
            });
            renderStatus(s);
          } catch (e) {
            console.error("slider action error:", e);
          }
        };
        input.addEventListener("input", () => {
          valBadge.textContent = `${input.value}${sl.unit ?? ""}`;
          void sendSliderUpdate();
        });
        input.addEventListener("change", () => {
          void sendSliderUpdate();
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
          btn.addEventListener("click", async () => {
            try {
              const res = await api.executeAppAction(m.metadata.id, b.id);
              renderStatus(res);
              showNotification(b.icon || "check-circle", `${b.label} activated!`);
            } catch (err) {
              console.error("action error:", err);
            }
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

  if (currentStatus) {
    const active = currentStatus.active_app || "living-rookery";
    applyThemeForApp(active);
    for (const [id, cardEl] of Object.entries(cards)) {
      cardEl.hidden = id !== active;
    }
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

let notificationTimer: number | null = null;
function showNotification(iconName: IconName | string, msg: string): void {
  iconLabel(seedNotification, iconName, msg);
  seedNotification.hidden = false;
  iconLabel(debugMsgStatus, iconName, msg);
  debugMsgStatus.hidden = false;
  if (notificationTimer !== null) {
    window.clearTimeout(notificationTimer);
  }
  notificationTimer = window.setTimeout(() => {
    seedNotification.hidden = true;
    debugMsgStatus.hidden = true;
    notificationTimer = null;
  }, 2800);
}

function formatSimSpeed(speed: number): string {
  if (speed <= 0.01) return "0.0x (Paused)";
  if (Math.abs(speed - 1.0) < 0.05) return "1.0x (Normal)";
  if (Math.abs(speed - 50.0) < 0.5) return "50.0x (Hyper)";
  return `${speed.toFixed(1)}x`;
}

let shareNotificationTimer: number | null = null;
function showShareNotification(iconName: IconName, msg: string, isError = false): void {
  iconLabel(shareNotification, isError ? "warning" : iconName, msg);
  shareNotification.style.color = isError ? "var(--danger)" : "var(--ok)";
  shareNotification.hidden = false;
  if (shareNotificationTimer !== null) {
    window.clearTimeout(shareNotificationTimer);
  }
  shareNotificationTimer = window.setTimeout(() => {
    shareNotification.hidden = true;
    shareNotificationTimer = null;
  }, 3500);
}

function renderStatus(s: HostStatus): void {
  currentStatus = s;
  try {
    localStorage.setItem("rookery_status", JSON.stringify(s));
  } catch {}

  // Power badge
  const powerLabels: Record<string, string> = {
    ac: "AC",
    battery: "BAT",
    low_power: "LOW PWR",
  };
  powerBadge.textContent = powerLabels[s.power] ?? s.power.toUpperCase();

  // Active App Theme & Cards
  const activeApp = s.active_app || "living-rookery";
  applyThemeForApp(activeApp);

  for (const [id, cardEl] of Object.entries(cards)) {
    cardEl.hidden = id !== activeApp;
  }

  if (activeView === "library") {
    renderLibraryGrid();
  }

  // Living Rookery Stats
  if (s.rookery) {
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
      const optCount = selectDebugForestTree.options.length;
      if (
        optCount !== s.rookery.trees.length ||
        document.activeElement !== selectDebugForestTree
      ) {
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
    statKoiLilies.textContent = String(s.koi.total_lilies);
    statKoiLotuses.textContent = String(s.koi.blooming_lotuses);
    statKoiRipples.textContent = String(s.koi.ripples_active);
  }

  // Cosmic Nebula Stats
  if (s.nebula) {
    statNebulaParticles.textContent = String(s.nebula.particle_count);
    statNebulaWells.textContent = String(s.nebula.gravity_wells);
    statNebulaPalette.textContent = s.nebula.palette_name;
    statNebulaTurb.textContent = `${s.nebula.turbulence.toFixed(1)}x`;
  }

  // Orbital Haven Stats
  if (s.orbital) {
    const epochShort = s.orbital.epoch_short || "Citadel";
    statOrbitalModules.textContent = `${s.orbital.total_modules}/66 · ${epochShort}`;
    const pwr = s.orbital.power_mw ?? s.orbital.solar_wings * 38;
    const crew = s.orbital.crew_population ?? s.orbital.habitat_sectors * 18;
    statOrbitalSolar.textContent = `+${pwr} MW · ${crew} Crew`;
    const pct = s.orbital.construction_pct ?? 0;
    statOrbitalRings.textContent = `${s.orbital.focus} (${pct}%)`;
    const bots = s.orbital.active_robots ?? 8;
    statOrbitalShuttles.textContent = `${s.orbital.docked_shuttles} Ships · ${bots} Bots`;
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
      }
    }
  }

  // Schedule Mode
  selectScheduleMode.value = s.schedule_mode;
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

  // Displays list
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

  // Audio Subsystem Status
  if (s.audio) {
    chkAudioEnabled.checked = s.audio.enabled;
    audioControlsGroup.hidden = !s.audio.enabled;
    const volPct = Math.round(s.audio.volume * 100);
    sliderVolume.value = String(volPct);
    valVolume.textContent = `${volPct}%`;
    chkAudioDirectMode.checked = s.audio.direct_mode_only;
  }
}

function renderSettings(st: Settings): void {
  try {
    localStorage.setItem("rookery_settings", JSON.stringify(st));
  } catch {}

  if (Array.isArray(st.favorite_apps)) {
    favoriteApps = st.favorite_apps;
    if (activeView === "library") {
      renderLibraryGrid();
    }
  }

  applyMenuAppearance(st.popover_opacity ?? 0.92, st.popover_blur ?? 20);

  if (st.audio) {
    chkAudioEnabled.checked = st.audio.enabled;
    audioControlsGroup.hidden = !st.audio.enabled;
    const volPct = Math.round(st.audio.volume * 100);
    sliderVolume.value = String(volPct);
    valVolume.textContent = `${volPct}%`;
    chkAudioDirectMode.checked = st.audio.direct_mode_only;
  }
  sliderPassive.value = String(st.frame.passive_fps);
  valPassive.textContent = `${st.frame.passive_fps} fps`;

  sliderLowPower.value = String(st.frame.low_power_fps);
  valLowPower.textContent = `${st.frame.low_power_fps} fps`;

  chkBattery.checked = st.frame.pause_on_battery;

  kbdHotkey.textContent = st.hotkey
    .replace("CommandOrControl", "⌘")
    .replace("Alt", "⌥")
    .replace("Shift", "⇧");
  btnRecordHotkey.textContent = st.hotkey;

  btnRecordPopoverHotkey.textContent =
    st.popover_hotkey || "CommandOrControl+Alt+Shift+P";

  selectScheduleMode.value = st.schedule_mode;
  sliderInterval.value = String(st.schedule_interval_mins);
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
btnFeed.addEventListener("click", async () => {
  try {
    const s = await api.feedBirds([500.0, 400.0]);
    renderStatus(s);
    showNotification("grains", "Snacks scattered across the boughs!");
  } catch (err) {
    console.error("feed error:", err);
  }
});

btnPrune.addEventListener("click", async () => {
  try {
    const s = await api.pruneBranch([500.0, 300.0], 80.0);
    renderStatus(s);
    showNotification("scissors", "Outer canopy pruned gently!");
  } catch (err) {
    console.error("prune error:", err);
  }
});

btnPlant.addEventListener("click", async () => {
  try {
    const s = await api.plantSeed([500.0, 750.0]);
    renderStatus(s);
    showNotification("plant", "Sapling seed planted at the roots!");
  } catch (err) {
    console.error("plant error:", err);
  }
});

btnPanLeft.addEventListener("click", async () => {
  try {
    const s = await api.panForest(-80.0);
    renderStatus(s);
    showNotification("caret-left", "Forest horizon panned west");
  } catch (err) {
    console.error("panForest error:", err);
  }
});

btnPanRight.addEventListener("click", async () => {
  try {
    const s = await api.panForest(80.0);
    renderStatus(s);
    showNotification("caret-right", "Forest horizon panned east");
  } catch (err) {
    console.error("panForest error:", err);
  }
});

selectRookeryStyle.addEventListener("change", async () => {
  try {
    const styleIdx = Number(selectRookeryStyle.value);
    const s = await api.executeAppAction("living-rookery", "set_style", {
      value: styleIdx,
    });
    renderStatus(s);
    const styleName = s.rookery?.aesthetic_style_name ?? `Palette ${styleIdx + 1}`;
    showNotification("palette", `Palette: ${styleName}`);
  } catch (err) {
    console.error("set_style error:", err);
  }
});

btnStylePrev.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "prev_style");
    renderStatus(s);
    const styleName = s.rookery?.aesthetic_style_name ?? "Previous Palette";
    showNotification("palette", `Palette: ${styleName}`);
  } catch (err) {
    console.error("prev_style error:", err);
  }
});

btnStyleNext.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "cycle_style");
    renderStatus(s);
    const styleName = s.rookery?.aesthetic_style_name ?? "Next Palette";
    showNotification("palette", `Palette: ${styleName}`);
  } catch (err) {
    console.error("cycle_style error:", err);
  }
});

// Koi Pond Action Handlers
btnKoiFeed.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("koi-pond", "feed", {
      pos: [500.0, 375.0],
    });
    renderStatus(s);
    showNotification("pellets", "Nourishing pellets dropped into the lagoon!");
  } catch (err) {
    console.error("koi feed error:", err);
  }
});

btnKoiPebble.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("koi-pond", "pebble", {
      pos: [500.0, 375.0],
      radius: 80.0,
    });
    renderStatus(s);
    showNotification("ripple", "Smooth pebble skipped across the water!");
  } catch (err) {
    console.error("koi pebble error:", err);
  }
});

// Cosmic Nebula Action Handlers
btnNebulaAttractor.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("cosmic-nebula", "add_attractor", {
      pos: [500.0, 375.0],
    });
    renderStatus(s);
    showNotification("spiral", "Gravitational singularity vortex spawned!");
  } catch (err) {
    console.error("nebula attractor error:", err);
  }
});

btnNebulaRepulsor.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("cosmic-nebula", "repulsor_pulse", {
      strength: 400.0,
    });
    renderStatus(s);
    showNotification("nova", "Supernova shockwave repelled stellar dust!");
  } catch (err) {
    console.error("nebula repulsor error:", err);
  }
});

btnNebulaPalette.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("cosmic-nebula", "cycle_palette");
    renderStatus(s);
    showNotification("palette", "Nebular spectrum shifted!");
  } catch (err) {
    console.error("nebula palette error:", err);
  }
});

// Orbital Haven Action Handlers
btnOrbitalModule.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "overdrive_drones");
    renderStatus(s);
    showNotification(
      "lightning",
      "Welder drones overdriven — module assembly accelerated!",
    );
  } catch (err) {
    console.error("orbital module error:", err);
  }
});

btnOrbitalLaunch.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "launch_shuttle");
    renderStatus(s);
    showNotification(
      "rocket-launch",
      "Orbital vessel dispatched from Aegis docking bay!",
    );
  } catch (err) {
    console.error("orbital launch error:", err);
  }
});

btnOrbitalFocus.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "cycle_focus");
    renderStatus(s);
    const focusName = s.orbital?.focus || "Updated";
    showNotification("hab-ring", `Construction directive: ${focusName}`);
  } catch (err) {
    console.error("orbital focus error:", err);
  }
});

btnOrbitalPlanet.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "cycle_planet");
    renderStatus(s);
    const biome = s.orbital?.planet_biome || "Shifted";
    showNotification("compass", `Orbital horizon shifted to ${biome}!`);
  } catch (err) {
    console.error("orbital planet error:", err);
  }
});

btnOrbitalScan.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "scan_pulse");
    renderStatus(s);
    showNotification("satellite", "Subspace sensor scan wave emitted!");
  } catch (err) {
    console.error("orbital scan error:", err);
  }
});

// Schedule Mode Selection
selectScheduleMode.addEventListener("change", async () => {
  const mode = selectScheduleMode.value as ScheduleMode;
  try {
    const st = await api.updateSettings({ schedule_mode: mode });
    renderSettings(st);
  } catch (err) {
    console.error("update schedule mode error:", err);
  }
});

sliderInterval.addEventListener("input", () => {
  valInterval.textContent = `${sliderInterval.value} mins`;
});

sliderInterval.addEventListener("change", async () => {
  const mins = parseInt(sliderInterval.value, 10);
  try {
    const st = await api.updateSettings({ schedule_interval_mins: mins });
    renderSettings(st);
  } catch (err) {
    console.error("update schedule interval error:", err);
  }
});

// Share & Capture
btnQuickSnapshot.addEventListener("click", async () => {
  try {
    btnQuickSnapshot.disabled = true;
    const path = await api.captureSnapshot();
    showNotification("camera", `Snapshot saved: ${path}`);
  } catch (err) {
    showNotification("warning", `Snapshot failed: ${err}`);
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

btnImportWorld.addEventListener("click", async () => {
  const text = inputImportWorld.value.trim();
  if (!text) return;
  try {
    const s = await api.importWorldString(text);
    renderStatus(s);
    inputImportWorld.value = "";
    showShareNotification("check-circle", "World imported successfully!");
  } catch (err) {
    showShareNotification("warning", `Import failed: ${err}`, true);
  }
});

// Interactive & Pause
btnInteractive.addEventListener("click", async () => {
  if (!currentStatus) return;
  try {
    const s = await api.setInteractive(!currentStatus.interactive);
    renderStatus(s);
  } catch (err) {
    console.error("setInteractive error:", err);
  }
});

btnPause.addEventListener("click", async () => {
  if (!currentStatus) return;
  try {
    const s = await api.setPaused(!currentStatus.user_paused);
    renderStatus(s);
  } catch (err) {
    console.error("setPaused error:", err);
  }
});

// Settings & Hotkeys
sliderPassive.addEventListener("input", () => {
  valPassive.textContent = `${sliderPassive.value} fps`;
});
sliderPassive.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      passive_fps: Number(sliderPassive.value),
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

sliderLowPower.addEventListener("input", () => {
  valLowPower.textContent = `${sliderLowPower.value} fps`;
});
sliderLowPower.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      low_power_fps: Number(sliderLowPower.value),
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

chkBattery.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      pause_on_battery: chkBattery.checked,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

// Menu Appearance Listeners
sliderMenuOpacity.addEventListener("input", () => {
  const pct = Number(sliderMenuOpacity.value);
  popoverRoot.style.setProperty("--menu-opacity", String(pct / 100));
  sliderMenuOpacity.setAttribute("aria-valuetext", `${pct}%`);
  valMenuOpacity.textContent = `${pct}%`;
});

sliderMenuOpacity.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      popover_opacity: Number(sliderMenuOpacity.value) / 100,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings opacity error:", err);
  }
});

sliderMenuBlur.addEventListener("input", () => {
  const blur = Number(sliderMenuBlur.value);
  popoverRoot.style.setProperty("--menu-blur", `${blur}px`);
  sliderMenuBlur.setAttribute("aria-valuetext", `${blur}px`);
  valMenuBlur.textContent = `${blur}px`;
  api.setPopoverBlur(blur).catch(() => {});
});

sliderMenuBlur.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      popover_blur: Number(sliderMenuBlur.value),
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings blur error:", err);
  }
});

// Audio Subsystem Listeners
chkAudioEnabled.addEventListener("change", async () => {
  audioControlsGroup.hidden = !chkAudioEnabled.checked;
  try {
    const st = await api.updateSettings({
      audio_enabled: chkAudioEnabled.checked,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

sliderVolume.addEventListener("input", () => {
  valVolume.textContent = `${sliderVolume.value}%`;
});

sliderVolume.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      audio_volume: Number(sliderVolume.value) / 100,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

chkAudioDirectMode.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      audio_direct_mode_only: chkAudioDirectMode.checked,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

// Interaction Hotkey Recording
btnRecordHotkey.addEventListener("click", () => {
  if (recordingPopoverHotkey) recordingPopoverHotkey = false;
  recordingInteractionHotkey = true;
  btnRecordHotkey.classList.add("recording");
  btnRecordHotkey.textContent = "Press shortcut keys…";
  hotkeyError.hidden = true;
});

// Popover Toggle Hotkey Recording
btnRecordPopoverHotkey.addEventListener("click", () => {
  if (recordingInteractionHotkey) recordingInteractionHotkey = false;
  recordingPopoverHotkey = true;
  btnRecordPopoverHotkey.classList.add("recording");
  btnRecordPopoverHotkey.textContent = "Press shortcut keys…";
  hotkeyError.hidden = true;
});

window.addEventListener("keydown", async (ev) => {
  if (!recordingInteractionHotkey && !recordingPopoverHotkey) {
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
  if (["Control", "Alt", "Shift", "Meta"].includes(ev.key)) return;

  ev.preventDefault();
  const parts: string[] = [];
  if (ev.metaKey || ev.ctrlKey) parts.push("CommandOrControl");
  if (ev.altKey) parts.push("Alt");
  if (ev.shiftKey) parts.push("Shift");

  let key = ev.key.toUpperCase();
  if (key === " ") key = "SPACE";
  parts.push(key);

  const shortcut = parts.join("+");

  if (recordingInteractionHotkey) {
    recordingInteractionHotkey = false;
    btnRecordHotkey.classList.remove("recording");
    try {
      const st = await api.updateSettings({ hotkey: shortcut });
      renderSettings(st);
    } catch (err) {
      hotkeyError.textContent = `Could not bind ${shortcut}: ${err}`;
      hotkeyError.hidden = false;
      const st = await api.getSettings();
      renderSettings(st);
    }
  } else if (recordingPopoverHotkey) {
    recordingPopoverHotkey = false;
    btnRecordPopoverHotkey.classList.remove("recording");
    try {
      const st = await api.updateSettings({ popover_hotkey: shortcut });
      renderSettings(st);
    } catch (err) {
      hotkeyError.textContent = `Could not bind ${shortcut}: ${err}`;
      hotkeyError.hidden = false;
      const st = await api.getSettings();
      renderSettings(st);
    }
  }
});

btnQuit.addEventListener("click", () => {
  api.quit().catch((err) => console.error("quit error:", err));
});

// Update Controls Listeners
chkAutoCheckUpdates.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      update_auto_check: chkAutoCheckUpdates.checked,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
});

chkAutoDownloadUpdates.addEventListener("change", async () => {
  try {
    const st = await api.updateSettings({
      update_auto_download: chkAutoDownloadUpdates.checked,
    });
    renderSettings(st);
  } catch (err) {
    console.error("updateSettings error:", err);
  }
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
  try {
    await api.updateSettings({ debug_mode: enabled });
    const s = await api.executeAppAction("living-rookery", "set_debug_mode", {
      enabled,
    });
    renderStatus(s);
    showNotification(
      "code",
      enabled ? "Debug Mode & in-scene telemetry active" : "Debug Mode disabled",
    );
  } catch (err) {
    console.error("set debug mode error:", err);
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

btnSpawnRaven.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_bird", {
      species: "CelestialRaven",
    });
    renderStatus(s);
    showNotification("bird", "Celestial Raven summoned into canopy!");
  } catch (err) {
    console.error("spawn raven error:", err);
  }
});

btnSpawnTanager.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_bird", {
      species: "RubyTanager",
    });
    renderStatus(s);
    showNotification("bird", "Ruby Tanager arrived in forest!");
  } catch (err) {
    console.error("spawn tanager error:", err);
  }
});

btnSpawnOwl.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_bird", {
      species: "SnowyOwl",
    });
    renderStatus(s);
    showNotification("bird", "Snowy Owl guardian appeared!");
  } catch (err) {
    console.error("spawn owl error:", err);
  }
});

btnSpawnFox.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_fauna", { species: "Fox" });
    renderStatus(s);
    showNotification("sparkle", "Red Fox trotting across the meadow!");
  } catch (err) {
    console.error("spawn fox error:", err);
  }
});

btnSpawnSquirrel.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_fauna", { species: "Squirrel" });
    renderStatus(s);
    showNotification("sparkle", "Squirrel scampering through the clovers!");
  } catch (err) {
    console.error("spawn squirrel error:", err);
  }
});

btnSpawnBadger.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_fauna", { species: "Badger" });
    renderStatus(s);
    showNotification("sparkle", "Badger exploring near the roots!");
  } catch (err) {
    console.error("spawn badger error:", err);
  }
});

btnSpawnHedgehog.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_fauna", { species: "Hedgehog" });
    renderStatus(s);
    showNotification("sparkle", "Prickly Hedgehog snuffling in the grass!");
  } catch (err) {
    console.error("spawn hedgehog error:", err);
  }
});

btnSpawnRabbit.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "spawn_fauna", { species: "Rabbit" });
    renderStatus(s);
    showNotification("sparkle", "Cottontail Rabbit hopped into the meadow!");
  } catch (err) {
    console.error("spawn rabbit error:", err);
  }
});

btnScatterSnacks.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("living-rookery", "scatter_snacks");
    renderStatus(s);
    showNotification("grains", "Sweet berries & acorns scattered on the grass!");
  } catch (err) {
    console.error("scatter snacks error:", err);
  }
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
