//! Versioned, atomically persisted host settings.
//!
//! Stored as pretty JSON at `<app config dir>/settings.json`. Loading never fails hard:
//! a missing file yields defaults, and a corrupt file is preserved as
//! `settings.json.bak-<unix-ts>` before falling back to defaults.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::governor::FrameSettings;
use crate::manifest::{DayNightMapping, ScheduleMode};

/// Current on-disk schema version. Bump and extend [`Settings::migrate`] on breaking changes.
pub const SETTINGS_VERSION: u32 = 1;

/// Default global hotkey for Direct Interaction Mode (Tauri accelerator syntax).
/// `CommandOrControl` maps to Cmd on macOS and Ctrl on Windows. Deliberately avoids
/// `Cmd/Ctrl+Shift+R`, which is "hard reload" in every major browser.
pub const DEFAULT_HOTKEY: &str = "CommandOrControl+Alt+Shift+R";

/// Default global hotkey to summon / toggle the Rookery popover menu from anywhere.
pub const DEFAULT_POPOVER_HOTKEY: &str = "CommandOrControl+Alt+Shift+P";

pub const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Global audio toggle. Strictly false (silent) by default.
    pub enabled: bool,
    /// Master volume level between 0.0 and 1.0 (clamped).
    pub volume: f32,
    /// If true, audio only plays while Direct Interaction Mode is active.
    pub direct_mode_only: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.5,
            direct_mode_only: false,
        }
    }
}

impl AudioSettings {
    pub fn clamped(mut self) -> Self {
        self.volume = self.volume.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub hotkey: String,
    pub popover_hotkey: String,
    pub active_app: String,
    pub schedule_mode: ScheduleMode,
    pub schedule_interval_mins: u32,
    pub day_night_mapping: DayNightMapping,
    pub frame: FrameSettings,
    pub audio: AudioSettings,
    /// Last time the host was known to be running (startup, heartbeat, clean shutdown).
    /// Groundwork for Milestone 2's offline time-delta engine.
    pub last_run_utc: Option<DateTime<Utc>>,
    /// False while running; set true on clean shutdown. Lets M2 detect crashes.
    pub last_shutdown_clean: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            hotkey: DEFAULT_HOTKEY.to_owned(),
            popover_hotkey: DEFAULT_POPOVER_HOTKEY.to_owned(),
            active_app: "living-rookery".to_owned(),
            schedule_mode: ScheduleMode::Manual,
            schedule_interval_mins: 30,
            day_night_mapping: DayNightMapping::default(),
            frame: FrameSettings::default(),
            audio: AudioSettings::default(),
            last_run_utc: None,
            last_shutdown_clean: true,
        }
    }
}

/// Outcome of [`Settings::load`], useful for logging.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    Missing,
    /// File was unreadable/corrupt; it was moved to the contained backup path.
    RecoveredFromCorrupt(PathBuf),
}

impl Settings {
    /// Load settings from `path`, falling back to defaults on any problem.
    pub fn load(path: &Path) -> (Self, LoadOutcome) {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Self::default(), LoadOutcome::Missing);
            }
            Err(e) => {
                log::warn!("failed to read settings {}: {e}", path.display());
                return (Self::default(), Self::backup_corrupt(path));
            }
        };
        match serde_json::from_str::<Settings>(&raw) {
            Ok(mut s) => {
                s.migrate();
                (s, LoadOutcome::Loaded)
            }
            Err(e) => {
                log::warn!("corrupt settings {}: {e}", path.display());
                (Self::default(), Self::backup_corrupt(path))
            }
        }
    }

    fn backup_corrupt(path: &Path) -> LoadOutcome {
        let ts = Utc::now().timestamp();
        let backup = path.with_extension(format!("json.bak-{ts}"));
        if let Err(e) = fs::rename(path, &backup) {
            log::warn!("could not back up corrupt settings: {e}");
        }
        LoadOutcome::RecoveredFromCorrupt(backup)
    }

    /// Upgrade older schema versions in place.
    fn migrate(&mut self) {
        if self.version < SETTINGS_VERSION {
            // v0 -> v1: no structural changes; `#[serde(default)]` fills new fields.
            self.version = SETTINGS_VERSION;
        }
        self.frame = self.frame.clamped();
        self.audio = self.audio.clamped();
        if self.hotkey.trim().is_empty() {
            self.hotkey = DEFAULT_HOTKEY.to_owned();
        }
        if self.popover_hotkey.trim().is_empty() {
            self.popover_hotkey = DEFAULT_POPOVER_HOTKEY.to_owned();
        }
        if self.active_app.trim().is_empty() {
            self.active_app = "living-rookery".to_owned();
        }
    }

    /// Atomically write settings to `path` (write temp file, fsync, rename).
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Mark the host as running now (startup / heartbeat).
    pub fn touch_running(&mut self, now: DateTime<Utc>) {
        self.last_run_utc = Some(now);
        self.last_shutdown_clean = false;
    }

    /// Mark a clean shutdown.
    pub fn mark_clean_shutdown(&mut self, now: DateTime<Utc>) {
        self.last_run_utc = Some(now);
        self.last_shutdown_clean = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_in(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("nested").join(SETTINGS_FILE_NAME)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let (s, outcome) = Settings::load(&path_in(&dir));
        assert_eq!(outcome, LoadOutcome::Missing);
        assert_eq!(s, Settings::default());
        assert_eq!(s.hotkey, DEFAULT_HOTKEY);
    }

    #[test]
    fn audio_settings_defaults_and_clamping() {
        let default_audio = AudioSettings::default();
        assert!(!default_audio.enabled);
        assert!((default_audio.volume - 0.5).abs() < f32::EPSILON);
        assert!(!default_audio.direct_mode_only);

        let clamped = AudioSettings {
            enabled: true,
            volume: 1.5,
            direct_mode_only: true,
        }
        .clamped();
        assert_eq!(clamped.volume, 1.0);

        let clamped_low = AudioSettings {
            enabled: true,
            volume: -0.5,
            direct_mode_only: true,
        }
        .clamped();
        assert_eq!(clamped_low.volume, 0.0);
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(&dir);
        let mut s = Settings::default();
        s.frame.passive_fps = 30;
        s.hotkey = "Alt+Shift+W".into();
        s.touch_running(Utc::now());
        s.save(&path).unwrap();
        let (loaded, outcome) = Settings::load(&path);
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(loaded, s);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        let (s, outcome) = Settings::load(&path);
        assert_eq!(s, Settings::default());
        match outcome {
            LoadOutcome::RecoveredFromCorrupt(backup) => {
                assert!(backup.exists());
                assert!(!path.exists());
            }
            other => panic!("unexpected outcome {other:?}"),
        }
    }

    #[test]
    fn partial_file_fills_defaults_and_migrates() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"version":0,"hotkey":"","frame":{"passive_fps":999}}"#,
        )
        .unwrap();
        let (s, outcome) = Settings::load(&path);
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(s.version, SETTINGS_VERSION);
        assert_eq!(s.hotkey, DEFAULT_HOTKEY);
        assert_eq!(s.frame.passive_fps, 30);
        assert_eq!(s.frame.interactive_fps, 60);
    }

    #[test]
    fn shutdown_flags() {
        let mut s = Settings::default();
        let now = Utc::now();
        s.touch_running(now);
        assert!(!s.last_shutdown_clean);
        s.mark_clean_shutdown(now);
        assert!(s.last_shutdown_clean);
        assert_eq!(s.last_run_utc, Some(now));
    }
}
