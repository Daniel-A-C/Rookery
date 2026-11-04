//! App Manifest, Controls Schema, and Scheduling Models (Milestone 4).
//!
//! Provides declarative app metadata (`rookery.toml` contract), dynamic control
//! schemas for popover UI generation, and playlist/day-night scheduler logic.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Core archetype classifications defined in VISION.md §3.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppArchetype {
    /// Living ecosystems & terrariums (Living Rookery, Koi Pond).
    CozyEcosystem,
    /// Algorithmic visuals & particle/shader physics (Cosmic Nebula).
    GenerativePhysics,
    /// Slow-burn incremental & strategy worlds (Orbital Haven).
    SlowBurnIdle,
    /// Time/weather reactive companions.
    AmbientUtility,
}

impl AppArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CozyEcosystem => "Cozy Living Ecosystem",
            Self::GenerativePhysics => "Generative Physics & Shader",
            Self::SlowBurnIdle => "Slow-Burn Idle World",
            Self::AmbientUtility => "Ambient Utility",
        }
    }
}

/// Metadata identifying a Rookery App.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub archetype: AppArchetype,
}

/// Declarative control widgets that the host dynamically displays in the popover UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppControl {
    Button {
        id: String,
        label: String,
        icon: Option<String>,
        tooltip: Option<String>,
    },
    Slider {
        id: String,
        label: String,
        min: f32,
        max: f32,
        step: f32,
        default_val: f32,
        unit: Option<String>,
    },
    Stat {
        id: String,
        label: String,
        default_val: String,
    },
    Select {
        id: String,
        label: String,
        options: Vec<String>,
        default_val: String,
    },
}

/// Target performance budgets declared in `rookery.toml`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppBudgets {
    pub target_fps: u32,
    pub max_cpu_ms: f32,
    pub max_rss_mb: f32,
}

impl Default for AppBudgets {
    fn default() -> Self {
        Self {
            target_fps: 30,
            max_cpu_ms: 2.0,
            max_rss_mb: 120.0,
        }
    }
}

/// Declarative schema of a Rookery App (the `rookery.toml` contract).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppManifest {
    pub metadata: AppMetadata,
    #[serde(default)]
    pub controls: Vec<AppControl>,
    #[serde(default)]
    pub budgets: AppBudgets,
}

impl AppManifest {
    /// Parse a `rookery.toml` file from disk.
    pub fn load_from_toml_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    /// Serialize to TOML format.
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }
}

/// Playlist and day-night scheduling modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    /// App is selected manually by user.
    #[default]
    Manual,
    /// Apps rotate cyclically every N minutes.
    Interval,
    /// App transitions automatically based on local time-of-day.
    DayNight,
}

/// Time-of-day app mapping for `ScheduleMode::DayNight`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayNightMapping {
    /// 06:00 - 11:59 (e.g. "koi-pond" / gentle sunrise)
    pub morning: String,
    /// 12:00 - 17:59 (e.g. "living-rookery" / daytime tree & birds)
    pub afternoon: String,
    /// 18:00 - 21:59 (e.g. "cosmic-nebula" / twilight & stars)
    pub evening: String,
    /// 22:00 - 05:59 (e.g. "orbital-haven" / peaceful night space)
    pub night: String,
}

impl Default for DayNightMapping {
    fn default() -> Self {
        Self {
            morning: "koi-pond".into(),
            afternoon: "living-rookery".into(),
            evening: "cosmic-nebula".into(),
            night: "orbital-haven".into(),
        }
    }
}

impl DayNightMapping {
    /// Returns the mapped app ID for the given local 24-hour hour (0..=23).
    pub fn app_for_hour(&self, hour: u32) -> &str {
        match hour {
            6..=11 => &self.morning,
            12..=17 => &self.afternoon,
            18..=21 => &self.evening,
            _ => &self.night,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_night_mapping_hours() {
        let mapping = DayNightMapping::default();
        assert_eq!(mapping.app_for_hour(8), "koi-pond");
        assert_eq!(mapping.app_for_hour(14), "living-rookery");
        assert_eq!(mapping.app_for_hour(19), "cosmic-nebula");
        assert_eq!(mapping.app_for_hour(23), "orbital-haven");
        assert_eq!(mapping.app_for_hour(2), "orbital-haven");
    }

    #[test]
    fn manifest_serialization_round_trip() {
        let manifest = AppManifest {
            metadata: AppMetadata {
                id: "test-app".into(),
                name: "Test App".into(),
                description: "A test app".into(),
                author: "Rookery Team".into(),
                version: "1.0.0".into(),
                archetype: AppArchetype::CozyEcosystem,
            },
            controls: vec![
                AppControl::Button {
                    id: "btn-action".into(),
                    label: "Action".into(),
                    icon: Some("✨".into()),
                    tooltip: Some("Does something".into()),
                },
                AppControl::Slider {
                    id: "slider-speed".into(),
                    label: "Speed".into(),
                    min: 0.1,
                    max: 5.0,
                    step: 0.1,
                    default_val: 1.0,
                    unit: Some("x".into()),
                },
                AppControl::Stat {
                    id: "stat-count".into(),
                    label: "Count".into(),
                    default_val: "0".into(),
                },
                AppControl::Select {
                    id: "select-mode".into(),
                    label: "Mode".into(),
                    options: vec!["fast".into(), "slow".into()],
                    default_val: "fast".into(),
                },
            ],
            budgets: AppBudgets::default(),
        };

        let toml_str = manifest.to_toml_string().expect("serialize to toml");
        let parsed: AppManifest = toml::from_str(&toml_str).expect("deserialize from toml");
        assert_eq!(manifest, parsed);
    }
}
