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

// Root & Header Elements
const popoverRoot = el<HTMLElement>("popover-root");
const brandGlyph = el<HTMLElement>("brand-glyph");
const brandTitle = el<HTMLHeadingElement>("brand-title");
const powerBadge = el<HTMLSpanElement>("power-badge");
const appTagline = el<HTMLParagraphElement>("app-tagline");

// View Toggle (Realm HUD vs Settings Menu)
const btnToggleSettings = el<HTMLButtonElement>("btn-toggle-settings");
const settingsToggleIcon = el<HTMLElement>("settings-toggle-icon");
const settingsToggleLabel = el<HTMLSpanElement>("settings-toggle-label");
const viewRealm = el<HTMLDivElement>("view-realm");
const viewSettings = el<HTMLDivElement>("view-settings");

// Settings Sub-Tabs
const settingsPanels: Record<string, HTMLElement> = {
  general: el<HTMLElement>("settings-panel-general"),
  controls: el<HTMLElement>("settings-panel-controls"),
  share: el<HTMLElement>("settings-panel-share"),
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

let isSettingsOpen = false;
function setSettingsOpen(open: boolean): void {
  isSettingsOpen = open;
  viewRealm.hidden = open;
  viewSettings.hidden = !open;
  btnToggleSettings.classList.toggle("active", open);
  btnToggleSettings.setAttribute("aria-expanded", String(open));
  if (open) {
    setIcon(settingsToggleIcon, "caret-left");
    settingsToggleLabel.textContent = "Realm";
  } else {
    setIcon(settingsToggleIcon, "settings");
    settingsToggleLabel.textContent = "Settings";
  }
}

btnToggleSettings.addEventListener("click", () => {
  setSettingsOpen(!isSettingsOpen);
});

// App Switcher Container & Realm Cards Container
const appSwitcher = el<HTMLElement>("app-switcher");
const realmCardsContainer = el<HTMLElement>("realm-cards-container");

// App Tabs & Cards Maps
const tabs: Record<string, HTMLButtonElement> = {};
const cards: Record<string, HTMLElement> = {};

interface AppThemeMeta {
  title: string;
  tagline: string;
  /** Icon name (see ui/icons/icons.config.json); legacy emoji still render as text. */
  glyph: IconName | string;
  /** Icon flanking the realm card banner (mirrored on the right). */
  ornament: IconName | string;
}

const appThemes: Record<string, AppThemeMeta> = {
  "living-rookery": {
    title: "The Living Rookery",
    tagline: "Woodland Sanctuary · ASCII Realm",
    glyph: "tree-evergreen",
    ornament: "leaf",
  },
  "koi-pond": {
    title: "Tranquil Koi Pond",
    tagline: "Imperial Lotus Sanctuary",
    glyph: "koi",
    ornament: "flower-lotus",
  },
  "cosmic-nebula": {
    title: "Cosmic Nebula",
    tagline: "Astral Singularity Forge",
    glyph: "nebula",
    ornament: "star-four",
  },
  "orbital-haven": {
    title: "Orbital Haven",
    tagline: "Aegis-VII Orbital Command",
    glyph: "satellite",
    ornament: "hexagon",
  },
  "aurora-waves": {
    title: "Aurora Waves",
    tagline: "Borealis Sky Observatory",
    glyph: "shooting-star",
    ornament: "star-four",
  },
  fireflies: {
    title: "Twilight Fireflies",
    tagline: "Enchanted Gloaming Glade",
    glyph: "sparkle",
    ornament: "sparkle",
  },
};

function applyThemeForApp(appId: string): void {
  popoverRoot.setAttribute("data-theme", appId);
  const meta = appThemes[appId];
  if (meta) {
    setIcon(brandGlyph, meta.glyph);
    brandTitle.textContent = meta.title;
    appTagline.textContent = meta.tagline;
  } else {
    setIcon(brandGlyph, "diamond");
    brandTitle.textContent = appId.replace(/-/g, " ");
    appTagline.textContent = "Custom Rookery Realm";
  }
}

function bindTabEvents(appId: string, btn: HTMLButtonElement): void {
  btn.onclick = async () => {
    for (const [id, tabBtn] of Object.entries(tabs)) {
      tabBtn.classList.toggle("active", id === appId);
    }
    for (const [id, cardEl] of Object.entries(cards)) {
      cardEl.hidden = id !== appId;
    }
    applyThemeForApp(appId);

    try {
      const s = await api.switchApp(appId);
      renderStatus(s);
    } catch (err) {
      console.error("switchApp error:", err);
    }
  };
}

for (const btn of appSwitcher.querySelectorAll<HTMLButtonElement>(".app-tab-btn")) {
  const appId = btn.getAttribute("data-app");
  if (appId) {
    tabs[appId] = btn;
    bindTabEvents(appId, btn);
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

// Schedule Controls
const selectScheduleMode = el<HTMLSelectElement>("select-schedule-mode");
const intervalGroup = el<HTMLDivElement>("interval-group");
const sliderInterval = el<HTMLInputElement>("slider-interval");
const valInterval = el<HTMLElement>("val-interval");

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

let currentStatus: HostStatus | null = null;
let recordingInteractionHotkey = false;
let recordingPopoverHotkey = false;

let appCatalog: AppManifest[] = [];

function getAppGlyph(m: AppManifest): string {
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

function getAppShortName(m: AppManifest): string {
  const shortNames: Record<string, string> = {
    "living-rookery": "Rookery",
    "koi-pond": "Koi Pond",
    "cosmic-nebula": "Nebula",
    "orbital-haven": "Orbital",
    "aurora-waves": "Aurora",
    fireflies: "Fireflies",
  };
  if (shortNames[m.metadata.id]) return shortNames[m.metadata.id];
  const name = m.metadata.name;
  return name.length > 9 ? name.split(" ")[0] : name;
}

function applyCatalog(): void {
  for (const m of appCatalog) {
    if (!appThemes[m.metadata.id]) {
      appThemes[m.metadata.id] = {
        title: m.metadata.name,
        tagline: m.metadata.description || "Custom Rookery Realm",
        glyph: getAppGlyph(m),
        ornament: "sparkle",
      };
    }

    // 1. Ensure tab exists in switcher
    if (!tabs[m.metadata.id]) {
      const tabBtn = document.createElement("button");
      tabBtn.id = `tab-${m.metadata.id}`;
      tabBtn.className = "app-tab-btn";
      tabBtn.type = "button";
      tabBtn.setAttribute("data-app", m.metadata.id);
      tabBtn.title = m.metadata.description;

      const glyphSpan = icon(getAppGlyph(m), "tab-glyph");

      const labelSpan = document.createElement("span");
      labelSpan.textContent = getAppShortName(m);

      tabBtn.appendChild(glyphSpan);
      tabBtn.appendChild(labelSpan);

      bindTabEvents(m.metadata.id, tabBtn);
      appSwitcher.appendChild(tabBtn);
      tabs[m.metadata.id] = tabBtn;
    }

    // 2. Ensure card exists inside realmCardsContainer
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
      bannerTitle.textContent = m.metadata.name;
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
          box.innerHTML = `
            <span class="stat-label">${sc.label}</span>
            <span id="stat-${m.metadata.id}-${sc.id}" class="stat-value">${sc.default_val ?? "-"}</span>
          `;
          statsGrid.appendChild(box);
        }
        card.appendChild(statsGrid);
      }

      // Slider controls
      const sliderControls = (m.controls || []).filter((c) => c.kind === "slider");
      for (const sl of sliderControls) {
        const row = document.createElement("div");
        row.className = "control-row";
        const label = document.createElement("span");
        label.textContent = sl.label;
        const valBadge = document.createElement("strong");
        valBadge.id = `val-${m.metadata.id}-${sl.id}`;
        valBadge.textContent = `${sl.default_val ?? 0}${sl.unit ?? ""}`;
        row.appendChild(label);
        row.appendChild(valBadge);
        card.appendChild(row);

        const input = document.createElement("input");
        input.id = `slider-${m.metadata.id}-${sl.id}`;
        input.type = "range";
        input.min = String(sl.min ?? 0);
        input.max = String(sl.max ?? 100);
        input.step = String(sl.step ?? 1);
        input.value = String(sl.default_val ?? 0);
        input.addEventListener("input", () => {
          valBadge.textContent = `${input.value}${sl.unit ?? ""}`;
        });
        input.addEventListener("change", async () => {
          try {
            await api.executeAppAction(m.metadata.id, sl.id, { value: Number(input.value) });
          } catch (e) {
            console.error("slider action error:", e);
          }
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
        for (const opt of sel.options || []) {
          const optEl = document.createElement("option");
          optEl.value = opt;
          optEl.textContent = opt;
          if (opt === sel.default_val) optEl.selected = true;
          selectEl.appendChild(optEl);
        }
        selectEl.addEventListener("change", async () => {
          try {
            await api.executeAppAction(m.metadata.id, sel.id, { value: selectEl.value });
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

  if (currentStatus) {
    const active = currentStatus.active_app || "living-rookery";
    applyThemeForApp(active);
    for (const [id, tabBtn] of Object.entries(tabs)) {
      tabBtn.classList.toggle("active", id === active);
    }
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
  if (notificationTimer !== null) {
    window.clearTimeout(notificationTimer);
  }
  notificationTimer = window.setTimeout(() => {
    seedNotification.hidden = true;
    notificationTimer = null;
  }, 2800);
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

  // Active App Theme, Tabs & Cards
  const activeApp = s.active_app || "living-rookery";
  applyThemeForApp(activeApp);

  for (const [id, tabBtn] of Object.entries(tabs)) {
    tabBtn.classList.toggle("active", id === activeApp);
  }

  for (const [id, cardEl] of Object.entries(cards)) {
    cardEl.hidden = id !== activeApp;
  }

  // Living Rookery Stats
  if (s.rookery) {
    statAge.textContent = formatAge(s.rookery.total_age_secs);
    statBranches.textContent = String(s.rookery.total_branches);
    statFoliage.textContent = String(s.rookery.total_foliage);
    statBirds.textContent = `${s.rookery.birds_active} birds`;
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
    statOrbitalModules.textContent = String(s.orbital.total_modules);
    statOrbitalSolar.textContent = String(s.orbital.solar_wings);
    statOrbitalRings.textContent = String(s.orbital.habitat_rings);
    statOrbitalShuttles.textContent = String(s.orbital.docked_shuttles);
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
    const s = await api.executeAppAction("orbital-haven", "add_module");
    renderStatus(s);
    showNotification("satellite", "New station module deployed into orbit!");
  } catch (err) {
    console.error("orbital module error:", err);
  }
});

btnOrbitalLaunch.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "launch_shuttle");
    renderStatus(s);
    showNotification("rocket-launch", "Transport shuttle launched from docking ring!");
  } catch (err) {
    console.error("orbital launch error:", err);
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
  if (!recordingInteractionHotkey && !recordingPopoverHotkey) return;
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
