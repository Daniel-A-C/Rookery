import { api, DisplayStatus, FrameBudget, HostStatus, Settings } from "./api";

const el = <T extends HTMLElement>(id: string): T => {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing #${id}`);
  return node as T;
};

const powerBadge = el<HTMLSpanElement>("power-badge");
const btnInteractive = el<HTMLButtonElement>("btn-interactive");
const lblInteractive = el<HTMLSpanElement>("lbl-interactive");
const kbdHotkey = el<HTMLElement>("kbd-hotkey");
const btnPause = el<HTMLButtonElement>("btn-pause");
const displaysList = el<HTMLUListElement>("displays-list");
const sliderPassive = el<HTMLInputElement>("slider-passive");
const valPassive = el<HTMLElement>("val-passive");
const sliderLowPower = el<HTMLInputElement>("slider-lowpower");
const valLowPower = el<HTMLElement>("val-lowpower");
const chkBattery = el<HTMLInputElement>("chk-battery");
const btnRecordHotkey = el<HTMLButtonElement>("btn-record-hotkey");
const hotkeyError = el<HTMLParagraphElement>("hotkey-error");
const btnQuit = el<HTMLButtonElement>("btn-quit");

let currentStatus: HostStatus | null = null;
let recordingHotkey = false;

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

function renderStatus(s: HostStatus): void {
  currentStatus = s;
  const powerLabels: Record<HostStatus["power"], string> = {
    ac: "AC Power",
    battery: "Battery",
    low_power: "Low Power",
  };
  powerBadge.textContent = powerLabels[s.power] ?? s.power;

  btnInteractive.classList.toggle("active", s.interactive);
  lblInteractive.textContent = s.interactive
    ? "Interactive · Active"
    : "Interact on Desktop";
  btnPause.textContent = s.user_paused ? "Resume" : "Pause";

  if (s.displays.length === 0) {
    displaysList.innerHTML = `<li class="empty">No active displays</li>`;
    return;
  }
  displaysList.innerHTML = "";
  for (const d of s.displays) {
    const li = document.createElement("li");
    li.className = "display-item";
    const left = document.createElement("span");
    left.textContent = d.name;
    const meta = document.createElement("span");
    meta.className = "display-meta";
    meta.textContent = `${d.width}×${d.height}`;
    left.appendChild(meta);

    const right = document.createElement("span");
    const fmt = formatBudget(d);
    right.className = `status-pill${fmt.paused ? " paused" : ""}`;
    right.textContent = fmt.text;

    li.append(left, right);
    displaysList.appendChild(li);
  }
}

function renderSettings(s: Settings): void {
  sliderPassive.value = String(s.frame.passive_fps);
  valPassive.textContent = `${s.frame.passive_fps} fps`;
  sliderLowPower.value = String(s.frame.low_power_fps);
  valLowPower.textContent = `${s.frame.low_power_fps} fps`;
  chkBattery.checked = s.frame.pause_on_battery;
  btnRecordHotkey.textContent = s.hotkey;
  kbdHotkey.textContent = s.hotkey
    .replace("CommandOrControl", "⌘/Ctrl")
    .replace(/\+/g, "");
}

btnInteractive.addEventListener("click", async () => {
  const next = !(currentStatus?.interactive ?? false);
  renderStatus(await api.setInteractive(next));
});

btnPause.addEventListener("click", async () => {
  const next = !(currentStatus?.user_paused ?? false);
  renderStatus(await api.setPaused(next));
});

sliderPassive.addEventListener("input", () => {
  valPassive.textContent = `${sliderPassive.value} fps`;
});
sliderPassive.addEventListener("change", async () => {
  renderSettings(
    await api.updateSettings({ passive_fps: Number(sliderPassive.value) })
  );
});

sliderLowPower.addEventListener("input", () => {
  valLowPower.textContent = `${sliderLowPower.value} fps`;
});
sliderLowPower.addEventListener("change", async () => {
  renderSettings(
    await api.updateSettings({ low_power_fps: Number(sliderLowPower.value) })
  );
});

chkBattery.addEventListener("change", async () => {
  renderSettings(
    await api.updateSettings({ pause_on_battery: chkBattery.checked })
  );
});

btnRecordHotkey.addEventListener("click", () => {
  recordingHotkey = !recordingHotkey;
  hotkeyError.hidden = true;
  btnRecordHotkey.classList.toggle("recording", recordingHotkey);
  if (recordingHotkey) {
    btnRecordHotkey.textContent = "Press keys… (Esc to cancel)";
  }
});

window.addEventListener("keydown", async (ev) => {
  if (!recordingHotkey) return;
  ev.preventDefault();
  ev.stopPropagation();

  if (ev.key === "Escape") {
    recordingHotkey = false;
    btnRecordHotkey.classList.remove("recording");
    renderSettings(await api.getSettings());
    return;
  }

  const ignored = new Set(["Meta", "Control", "Alt", "Shift"]);
  if (ignored.has(ev.key)) return;

  const parts: string[] = [];
  if (ev.metaKey || ev.ctrlKey) parts.push("CommandOrControl");
  if (ev.altKey) parts.push("Alt");
  if (ev.shiftKey) parts.push("Shift");
  if (parts.length === 0) {
    hotkeyError.textContent = "Include at least one modifier (⌘/Ctrl/Alt/Shift).";
    hotkeyError.hidden = false;
    return;
  }
  const key = ev.key.length === 1 ? ev.key.toUpperCase() : ev.key;
  parts.push(key);
  const combo = parts.join("+");

  recordingHotkey = false;
  btnRecordHotkey.classList.remove("recording");
  try {
    const updated = await api.updateSettings({ hotkey: combo });
    hotkeyError.hidden = true;
    renderSettings(updated);
  } catch (err) {
    hotkeyError.textContent = String(err);
    hotkeyError.hidden = false;
    renderSettings(await api.getSettings());
  }
});

btnQuit.addEventListener("click", () => {
  void api.quit();
});

async function init(): Promise<void> {
  const [status, settings] = await Promise.all([
    api.getStatus(),
    api.getSettings(),
  ]);
  renderStatus(status);
  renderSettings(settings);
  await api.onStatus(renderStatus);
}

void init();

