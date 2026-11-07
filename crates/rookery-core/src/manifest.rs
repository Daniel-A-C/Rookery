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
    /// Sandboxed HTML5 Canvas / WebGL apps.
    WebCanvas,
}

impl AppArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CozyEcosystem => "Cozy Living Ecosystem",
            Self::GenerativePhysics => "Generative Physics & Shader",
            Self::SlowBurnIdle => "Slow-Burn Idle World",
            Self::AmbientUtility => "Ambient Utility",
            Self::WebCanvas => "Lightweight Web Canvas",
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
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Declarative visual theme and branding metadata for the popover HUD and desktop border.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppTheme {
    /// Short subtitle shown under the app title in the popover header.
    pub tagline: Option<String>,
    /// Optional title for the HUD card banner (defaults to `metadata.name`).
    pub banner_title: Option<String>,
    /// Primary icon name from `ui/icons/icons.config.json` (e.g. `"tree-evergreen"`, `"koi"`).
    pub icon: Option<String>,
    /// Flanking ornament icon name for the HUD card banner (e.g. `"leaf"`, `"star-four"`).
    pub ornament: Option<String>,
    /// Primary background color hex (e.g. `"#0c1912"`).
    pub bg: Option<String>,
    /// HUD card surface color (e.g. `"rgba(19, 38, 27, 0.92)"` or `"#13261b"`).
    pub card: Option<String>,
    /// Outer border hex color (e.g. `"#366847"`), also used for the desktop interactive vignette.
    pub border_outer: Option<String>,
    /// Primary accent hex color (e.g. `"#34d399"`).
    pub accent: Option<String>,
    /// Secondary accent hex color (e.g. `"#eab308"`).
    pub accent_secondary: Option<String>,
    /// Two-stop hex gradient for the header crest badge (e.g. `["#15803d", "#ca8a04"]`).
    pub crest_gradient: Option<[String; 2]>,
}

impl AppTheme {
    /// Parse a `#RRGGBB` or `RRGGBB` hex string into normalized `[f32; 3]` sRGB components.
    pub fn parse_hex_rgb(hex: &str) -> Option<[f32; 3]> {
        let s = hex.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
    }

    /// Return the normalized `[f32; 3]` desktop border color from `border_outer` or `accent`.
    pub fn border_rgb_f32(&self) -> Option<[f32; 3]> {
        self.border_outer
            .as_deref()
            .and_then(Self::parse_hex_rgb)
            .or_else(|| self.accent.as_deref().and_then(Self::parse_hex_rgb))
    }
}

/// Declarative control widgets that the host dynamically displays in the popover UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppControl {
    Button {
        id: String,
        label: String,
        /// Semantic icon name from `ui/icons/icons.config.json` (e.g. `"sparkle"`, `"palette"`),
        /// with automatic fallback for legacy emoji strings.
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
        #[serde(default)]
        icon: Option<String>,
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
    pub theme: AppTheme,
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
    fn archetype_display_names_and_serde() {
        let arch = AppArchetype::WebCanvas;
        assert_eq!(arch.display_name(), "Lightweight Web Canvas");
        let serialized = serde_json::to_string(&arch).expect("serialize");
        assert_eq!(serialized, "\"web_canvas\"");
        let deserialized: AppArchetype = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, AppArchetype::WebCanvas);
    }

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
                tags: vec!["nature".into(), "test".into()],
            },
            theme: AppTheme {
                tagline: Some("Test Tagline".into()),
                banner_title: Some("Test Banner".into()),
                icon: Some("leaf".into()),
                ornament: Some("sparkle".into()),
                bg: Some("#0c1912".into()),
                card: Some("rgba(19, 38, 27, 0.92)".into()),
                border_outer: Some("#366847".into()),
                accent: Some("#34d399".into()),
                accent_secondary: Some("#eab308".into()),
                crest_gradient: Some(["#15803d".into(), "#ca8a04".into()]),
            },
            controls: vec![
                AppControl::Button {
                    id: "btn-action".into(),
                    label: "Action".into(),
                    icon: Some("sparkle".into()),
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
                    icon: Some("sparkle".into()),
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
        let rgb = parsed.theme.border_rgb_f32().expect("valid hex rgb");
        assert!((rgb[0] - 0.2117).abs() < 0.01);
    }
}
