import {
  api,
  DisplayStatus,
  FrameBudget,
  HostStatus,
  ScheduleMode,
  Settings,
} from "./api";

const el = <T extends HTMLElement>(id: string): T => {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing #${id}`);
  return node as T;
};

// Header & Power
const powerBadge = el<HTMLSpanElement>("power-badge");
const appTagline = el<HTMLParagraphElement>("app-tagline");

// App Tabs
const tabs: Record<string, HTMLButtonElement> = {
  "living-rookery": el<HTMLButtonElement>("tab-living-rookery"),
  "koi-pond": el<HTMLButtonElement>("tab-koi-pond"),
  "cosmic-nebula": el<HTMLButtonElement>("tab-cosmic-nebula"),
  "orbital-haven": el<HTMLButtonElement>("tab-orbital-haven"),
};

// Cards
const cards: Record<string, HTMLElement> = {
  "living-rookery": el<HTMLElement>("card-living-rookery"),
  "koi-pond": el<HTMLElement>("card-koi-pond"),
  "cosmic-nebula": el<HTMLElement>("card-cosmic-nebula"),
  "orbital-haven": el<HTMLElement>("card-orbital-haven"),
};

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
const btnSnapshot = el<HTMLButtonElement>("btn-snapshot");
const btnRecordLoop = el<HTMLButtonElement>("btn-record-loop");
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
const displaysList = el<HTMLUListElement>("displays-list");

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

let currentStatus: HostStatus | null = null;
let recordingInteractionHotkey = false;
let recordingPopoverHotkey = false;

const appTaglines: Record<string, string> = {
  "living-rookery": "The Living ASCII Rookery",
  "koi-pond": "Tranquil Koi Pond & Articulated Boids",
  "cosmic-nebula": "Deep Space Procedural Nebula & Gravity Wells",
  "orbital-haven": "Modular Space Station & Planetary Horizon",
};

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
function showNotification(msg: string): void {
  seedNotification.textContent = msg;
  seedNotification.hidden = false;
  if (notificationTimer !== null) {
    window.clearTimeout(notificationTimer);
  }
  notificationTimer = window.setTimeout(() => {
    seedNotification.hidden = true;
    notificationTimer = null;
  }, 3000);
}

let shareNotificationTimer: number | null = null;
function showShareNotification(msg: string, isError = false): void {
  shareNotification.textContent = msg;
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

  // Power badge
  const powerLabels: Record<string, string> = {
    ac: "AC",
    battery: "BAT",
    low_power: "LOW PWR",
  };
  powerBadge.textContent = powerLabels[s.power] ?? s.power.toUpperCase();

  // Active App Tabs & Cards
  const activeApp = s.active_app || "living-rookery";
  appTagline.textContent = appTaglines[activeApp] ?? "Rookery App";

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
  const isInterval = s.schedule_mode === "Interval";
  intervalGroup.hidden = !isInterval;
  sliderInterval.hidden = !isInterval;

  // Interactive button
  btnInteractive.classList.toggle("active", s.interactive);
  lblInteractive.textContent = s.interactive
    ? "Desktop Interactive"
    : "Interact on Desktop";

  // Pause button
  btnPause.textContent = s.user_paused ? "Resume" : "Pause";

  // Displays list
  displaysList.innerHTML = "";
  if (s.displays.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "No active displays";
    displaysList.appendChild(li);
    return;
  }

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

function renderSettings(st: Settings): void {
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

  btnRecordPopoverHotkey.textContent = st.popover_hotkey || "CommandOrControl+Alt+Shift+P";

  selectScheduleMode.value = st.schedule_mode;
  sliderInterval.value = String(st.schedule_interval_mins);
  valInterval.textContent = `${st.schedule_interval_mins} mins`;
  const isInterval = st.schedule_mode === "Interval";
  intervalGroup.hidden = !isInterval;
  sliderInterval.hidden = !isInterval;
}

// App Switcher Event Handlers
for (const [id, btn] of Object.entries(tabs)) {
  btn.addEventListener("click", async () => {
    try {
      const s = await api.switchApp(id);
      renderStatus(s);
    } catch (err) {
      console.error("switchApp error:", err);
    }
  });
}

// Living Rookery Action Handlers
btnFeed.addEventListener("click", async () => {
  try {
    const s = await api.feedBirds([500.0, 400.0]);
    renderStatus(s);
    showNotification("🌾 Snacks placed on branches!");
  } catch (err) {
    console.error("feed error:", err);
  }
});

btnPrune.addEventListener("click", async () => {
  try {
    const s = await api.pruneBranch([500.0, 300.0], 80.0);
    renderStatus(s);
    showNotification("✂️ Branches pruned gently!");
  } catch (err) {
    console.error("prune error:", err);
  }
});

btnPlant.addEventListener("click", async () => {
  try {
    const s = await api.plantSeed([500.0, 750.0]);
    renderStatus(s);
    showNotification("🌱 Sapling seed planted at roots!");
  } catch (err) {
    console.error("plant error:", err);
  }
});

btnPanLeft.addEventListener("click", async () => {
  try {
    const s = await api.panForest(-80.0);
    renderStatus(s);
    showNotification("◀ Forest panned left");
  } catch (err) {
    console.error("panForest error:", err);
  }
});

btnPanRight.addEventListener("click", async () => {
  try {
    const s = await api.panForest(80.0);
    renderStatus(s);
    showNotification("Forest panned right ▶");
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
    showNotification("🪱 Nourishing food pellets dropped into the pond!");
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
    showNotification("💧 Cast smooth pebble, rippling water!");
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
    showNotification("🌀 Gravitational attractor vortex created!");
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
    showNotification("💥 Stellar supernova pulse repelled dust!");
  } catch (err) {
    console.error("nebula repulsor error:", err);
  }
});

btnNebulaPalette.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("cosmic-nebula", "cycle_palette");
    renderStatus(s);
    showNotification("🎨 Cosmic palette cycled!");
  } catch (err) {
    console.error("nebula palette error:", err);
  }
});

// Orbital Haven Action Handlers
btnOrbitalModule.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "add_module");
    renderStatus(s);
    showNotification("🛰️ New station module deployed into orbit!");
  } catch (err) {
    console.error("orbital module error:", err);
  }
});

btnOrbitalLaunch.addEventListener("click", async () => {
  try {
    const s = await api.executeAppAction("orbital-haven", "launch_shuttle");
    renderStatus(s);
    showNotification("🚀 Transport shuttle launched from docking ring!");
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
btnSnapshot.addEventListener("click", async () => {
  try {
    btnSnapshot.disabled = true;
    const path = await api.captureSnapshot();
    showShareNotification(`📸 Snapshot saved: ${path}`);
  } catch (err) {
    showShareNotification(`Snapshot failed: ${err}`, true);
  } finally {
    btnSnapshot.disabled = false;
  }
});

btnRecordLoop.addEventListener("click", async () => {
  try {
    btnRecordLoop.disabled = true;
    showShareNotification("🎞️ Recording 5s loop…");
    const path = await api.recordLoop();
    showShareNotification(`🎞️ GIF saved: ${path}`);
  } catch (err) {
    showShareNotification(`Record loop failed: ${err}`, true);
  } finally {
    btnRecordLoop.disabled = false;
  }
});

btnCopySeed.addEventListener("click", async () => {
  try {
    const uri = await api.getWorldSeed();
    await navigator.clipboard.writeText(uri);
    showShareNotification("🌱 Copied World Seed URI!");
  } catch (err) {
    showShareNotification(`Copy seed failed: ${err}`, true);
  }
});

btnCopyState.addEventListener("click", async () => {
  try {
    const uri = await api.getWorldStateString();
    await navigator.clipboard.writeText(uri);
    showShareNotification("📦 Copied full World State snapshot!");
  } catch (err) {
    showShareNotification(`Copy state failed: ${err}`, true);
  }
});

btnImportWorld.addEventListener("click", async () => {
  const text = inputImportWorld.value.trim();
  if (!text) return;
  try {
    const s = await api.importWorldString(text);
    renderStatus(s);
    inputImportWorld.value = "";
    showShareNotification("✨ World imported successfully!");
  } catch (err) {
    showShareNotification(`Import failed: ${err}`, true);
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

// Bootstrap
(async () => {
  try {
    const [status, settings] = await Promise.all([
      api.getStatus(),
      api.getSettings(),
    ]);
    renderStatus(status);
    renderSettings(settings);
  } catch (err) {
    console.error("bootstrap error:", err);
  }

  await api.onStatus((s) => renderStatus(s));
})();
