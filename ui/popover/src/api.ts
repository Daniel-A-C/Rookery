import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export type PowerState = "ac" | "battery" | "low_power";

export type PauseReason =
  | "occluded"
  | "user_paused"
  | "screen_asleep"
  | "on_battery";

export type FrameBudget =
  | { kind: "paused"; value: PauseReason }
  | { kind: "fps"; value: number };

export interface DisplayStatus {
  id: string;
  name: string;
  width: number;
  height: number;
  budget: FrameBudget;
  actual_fps: number;
  frame_cpu_ms: number;
  occluded: boolean;
}

export interface RookeryStatus {
  seed: number;
  total_age_secs: number;
  total_offline_caught_up_secs: number;
  total_branches: number;
  total_foliage: number;
  birds_active: number;
  birds_perched: number;
  pruned_branches: number;
  seeds_planted: number;
  snacks_eaten: number;
  nests_built: number;
}

export interface KoiStatus {
  seed: number;
  total_koi: number;
  total_lilies: number;
  blooming_lotuses: number;
  ripples_active: number;
  pellets_active: number;
}

export interface NebulaStatus {
  seed: number;
  particle_count: number;
  gravity_wells: number;
  palette_name: string;
  turbulence: number;
}

export interface OrbitalStatus {
  seed: number;
  total_modules: number;
  solar_wings: number;
  habitat_rings: number;
  docked_shuttles: number;
  offline_science: number;
  orbit_angle_deg: number;
}

export type ScheduleMode = "Manual" | "Interval" | "DayNight";

export interface HostStatus {
  displays: DisplayStatus[];
  power: PowerState;
  interactive: boolean;
  user_paused: boolean;
  active_app: string;
  schedule_mode: ScheduleMode;
  rookery?: RookeryStatus;
  koi?: KoiStatus;
  nebula?: NebulaStatus;
  orbital?: OrbitalStatus;
}

export interface AppMetadata {
  id: string;
  name: string;
  tagline: string;
  version: string;
  author: string;
  archetype: string;
  icon_glyph: string;
  tags: string[];
}

export interface FrameSettings {
  passive_fps: number;
  interactive_fps: number;
  low_power_fps: number;
  pause_on_battery: boolean;
}

export interface Settings {
  version: number;
  hotkey: string;
  popover_hotkey: string;
  frame: FrameSettings;
  active_app: string;
  schedule_mode: ScheduleMode;
  schedule_interval_mins: number;
}

export interface SettingsPatch {
  hotkey?: string;
  popover_hotkey?: string;
  passive_fps?: number;
  interactive_fps?: number;
  low_power_fps?: number;
  pause_on_battery?: boolean;
  active_app?: string;
  schedule_mode?: ScheduleMode;
  schedule_interval_mins?: number;
}

export const api = {
  getStatus: (): Promise<HostStatus> => invoke("get_status"),
  getCatalog: (): Promise<AppMetadata[]> => invoke("get_catalog"),
  switchApp: (appId: string): Promise<HostStatus> =>
    invoke("switch_app", { appId }),
  panForest: (deltaX: number): Promise<HostStatus> =>
    invoke("pan_forest", { deltaX }),
  executeAppAction: (
    appId: string,
    action: string,
    payload?: unknown,
  ): Promise<HostStatus> =>
    invoke("execute_app_action", { appId, action, payload }),
  setPaused: (paused: boolean): Promise<HostStatus> =>
    invoke("set_paused", { paused }),
  setInteractive: (interactive: boolean): Promise<HostStatus> =>
    invoke("set_interactive", { interactive }),
  getSettings: (): Promise<Settings> => invoke("get_settings"),
  updateSettings: (patch: SettingsPatch): Promise<Settings> =>
    invoke("update_settings", { patch }),
  plantSeed: (pos: [number, number]): Promise<HostStatus> =>
    invoke("plant_seed", { pos }),
  pruneBranch: (
    pos: [number, number],
    radius?: number,
  ): Promise<HostStatus> => invoke("prune_branch", { pos, radius }),
  feedBirds: (pos: [number, number]): Promise<HostStatus> =>
    invoke("feed_birds", { pos }),
  getWorldSeed: (): Promise<string> => invoke("get_world_seed"),
  getWorldStateString: (): Promise<string> => invoke("get_world_state_string"),
  importWorldString: (input: string): Promise<HostStatus> =>
    invoke("import_world_string", { input }),
  captureSnapshot: (): Promise<string> => invoke("capture_snapshot"),
  recordLoop: (): Promise<string> => invoke("record_loop"),
  setWorldSeed: (seed: number): Promise<HostStatus> =>
    invoke("set_world_seed", { seed }),
  quit: (): Promise<void> => invoke("quit"),
  onStatus: (cb: (status: HostStatus) => void): Promise<UnlistenFn> =>
    listen<HostStatus>("rookery://status", (ev) => cb(ev.payload)),
};
