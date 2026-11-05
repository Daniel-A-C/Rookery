import { check, Update } from "@tauri-apps/plugin-updater";
import { api } from "./api";

export interface UpdateState {
  checking: boolean;
  available: boolean;
  version?: string;
  currentVersion?: string;
  date?: string;
  body?: string;
  downloading: boolean;
  downloadProgress: number; // 0..100
  installed: boolean;
  error?: string;
  lastChecked?: Date;
}

type UpdateListener = (state: UpdateState) => void;

class UpdaterController {
  private activeUpdate: Update | null = null;
  private listeners: UpdateListener[] = [];
  private state: UpdateState = {
    checking: false,
    available: false,
    downloading: false,
    downloadProgress: 0,
    installed: false,
  };

  public getState(): UpdateState {
    return { ...this.state };
  }

  public subscribe(listener: UpdateListener): () => void {
    this.listeners.push(listener);
    listener(this.getState());
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    const s = this.getState();
    for (const l of this.listeners) {
      try {
        l(s);
      } catch (e) {
        console.error("Updater listener error:", e);
      }
    }
  }

  public async checkForUpdates(silent = false): Promise<UpdateState> {
    if (this.state.checking || this.state.downloading) {
      return this.getState();
    }

    // In dev mode, skip automatic background checks to avoid spamming 404 errors
    // against GitHub Releases before any production releases are published.
    if (import.meta.env.DEV && silent) {
      return this.getState();
    }

    this.state.checking = true;
    this.state.error = undefined;
    this.notify();

    try {
      const update = await check();
      this.state.checking = false;
      this.state.lastChecked = new Date();

      if (update) {
        this.activeUpdate = update;
        this.state.available = true;
        this.state.version = update.version;
        this.state.currentVersion = update.currentVersion;
        this.state.date = update.date;
        this.state.body = update.body || "";
      } else {
        if (this.activeUpdate) {
          try {
            await this.activeUpdate.close();
          } catch (_) {
            // Ignore close error on stale update resource
          }
          this.activeUpdate = null;
        }
        this.state.available = false;
        this.state.version = undefined;
      }
    } catch (err: any) {
      this.state.checking = false;
      const msg = err?.message || String(err);
      this.state.error = msg;
      if (!silent) {
        console.warn("Update check failed:", err);
      }
    }

    this.notify();
    return this.getState();
  }

  public async downloadAndInstall(): Promise<void> {
    if (!this.activeUpdate || this.state.downloading) return;

    this.state.downloading = true;
    this.state.downloadProgress = 0;
    this.state.error = undefined;
    this.notify();

    let totalBytes = 0;
    let downloadedBytes = 0;

    try {
      await this.activeUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength || 0;
          this.state.downloadProgress = 0;
          this.notify();
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            this.state.downloadProgress = Math.min(
              100,
              Math.round((downloadedBytes / totalBytes) * 100),
            );
          } else {
            this.state.downloadProgress = Math.min(
              95,
              this.state.downloadProgress + 2,
            );
          }
          this.notify();
        } else if (event.event === "Finished") {
          this.state.downloadProgress = 100;
          this.notify();
        }
      });

      this.state.downloading = false;
      this.state.installed = true;
      this.notify();
    } catch (err: any) {
      this.state.downloading = false;
      this.state.error = err?.message || String(err);
      this.notify();
      throw err;
    }
  }

  public async restartApp(): Promise<void> {
    await api.relaunchApp();
  }
}

export const updater = new UpdaterController();

