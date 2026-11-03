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

export interface HostStatus {
  displays: DisplayStatus[];
  power: PowerState;
  interactive: boolean;
  user_paused: boolean;
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
  frame: FrameSettings;
}

export interface SettingsPatch {
  hotkey?: string;
  passive_fps?: number;
  interactive_fps?: number;
  low_power_fps?: number;
  pause_on_battery?: boolean;
}

export const api = {
  getStatus: (): Promise<HostStatus> => invoke("get_status"),
  setPaused: (paused: boolean): Promise<HostStatus> =>
    invoke("set_paused", { paused }),
  setInteractive: (interactive: boolean): Promise<HostStatus> =>
    invoke("set_interactive", { interactive }),
  getSettings: (): Promise<Settings> => invoke("get_settings"),
  updateSettings: (patch: SettingsPatch): Promise<Settings> =>
    invoke("update_settings", { patch }),
  quit: (): Promise<void> => invoke("quit"),
  onStatus: (cb: (status: HostStatus) => void): Promise<UnlistenFn> =>
    listen<HostStatus>("rookery://status", (ev) => cb(ev.payload)),
};
