//! The performance governor: a pure function from runtime state to per-display frame budgets.
//!
//! Rules (highest priority first), mirroring VISION.md §5.1:
//!
//! | # | Condition                         | Budget                                   |
//! |---|-----------------------------------|------------------------------------------|
//! | 1 | Screens asleep / session locked   | `Paused(ScreenAsleep)` on all displays   |
//! | 2 | User paused                       | `Paused(UserPaused)` on all displays     |
//! | 3 | Direct Interaction Mode           | `Fps(interactive_fps)` on all displays   |
//! | 4 | Display occluded                  | `Paused(Occluded)`                       |
//! | 5 | On battery + `pause_on_battery`   | `Paused(OnBattery)`                      |
//! | 6 | Low-power mode                    | `Fps(low_power_fps)`                     |
//! | 7 | On battery                        | passive rate capped at [`BATTERY_FPS_CAP`] |
//! | 8 | Active moments / uncovered desktop| `Fps(interactive_fps)` (60 FPS boost)   |
//! | 9 | Otherwise                         | `Idle` → [`IDLE_FPS`], `Animating` → `passive_fps` |

use serde::{Deserialize, Serialize};

use crate::{Activity, DisplayId};

/// Frame rate used when a scene reports [`Activity::Idle`] during passive viewing.
pub const IDLE_FPS: u32 = 15;
/// Upper bound on passive frame rate while running on battery.
pub const BATTERY_FPS_CAP: u32 = 20;
/// Allowed range for the user-configurable passive frame rate.
pub const PASSIVE_FPS_RANGE: (u32, u32) = (15, 30);
/// Allowed range for the user-configurable low-power frame rate.
pub const LOW_POWER_FPS_RANGE: (u32, u32) = (5, 15);
/// Allowed range for the user-configurable interactive frame rate.
pub const INTERACTIVE_FPS_RANGE: (u32, u32) = (30, 120);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    #[default]
    Ac,
    Battery,
    /// OS low-power / battery-saver mode is enabled.
    LowPower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    Occluded,
    UserPaused,
    ScreenAsleep,
    OnBattery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum FrameBudget {
    Paused(PauseReason),
    Fps(u32),
}

impl FrameBudget {
    pub fn fps(&self) -> u32 {
        match self {
            FrameBudget::Paused(_) => 0,
            FrameBudget::Fps(f) => *f,
        }
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, FrameBudget::Paused(_))
    }
}

/// User-tunable frame-rate settings. Values are clamped to their allowed ranges on use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameSettings {
    pub passive_fps: u32,
    pub interactive_fps: u32,
    pub low_power_fps: u32,
    pub pause_on_battery: bool,
}

impl Default for FrameSettings {
    fn default() -> Self {
        Self {
            passive_fps: 24,
            interactive_fps: 60,
            low_power_fps: 12,
            pause_on_battery: false,
        }
    }
}

impl FrameSettings {
    /// Return a copy with every value clamped into its allowed range.
    pub fn clamped(&self) -> Self {
        let clamp = |v: u32, (lo, hi): (u32, u32)| v.clamp(lo, hi);
        Self {
            passive_fps: clamp(self.passive_fps, PASSIVE_FPS_RANGE),
            interactive_fps: clamp(self.interactive_fps, INTERACTIVE_FPS_RANGE),
            low_power_fps: clamp(self.low_power_fps, LOW_POWER_FPS_RANGE),
            pause_on_battery: self.pause_on_battery,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayInputs {
    pub id: DisplayId,
    pub occluded: bool,
    pub activity: Activity,
    pub uncovered: bool,
}

impl DisplayInputs {
    pub fn new(id: DisplayId, occluded: bool, activity: Activity) -> Self {
        Self {
            id,
            occluded,
            activity,
            uncovered: false,
        }
    }

    pub fn with_uncovered(mut self, uncovered: bool) -> Self {
        self.uncovered = uncovered;
        self
    }
}

#[derive(Clone, Debug)]
pub struct GovernorInputs<'a> {
    pub displays: &'a [DisplayInputs],
    pub power: PowerState,
    pub interactive: bool,
    pub user_paused: bool,
    pub screens_asleep: bool,
    pub settings: &'a FrameSettings,
}

/// Compute the frame budget for every display.
pub fn compute(inputs: &GovernorInputs<'_>) -> Vec<(DisplayId, FrameBudget)> {
    let s = inputs.settings.clamped();
    inputs
        .displays
        .iter()
        .map(|d| (d.id.clone(), budget_for(d, inputs, &s)))
        .collect()
}

fn budget_for(d: &DisplayInputs, i: &GovernorInputs<'_>, s: &FrameSettings) -> FrameBudget {
    if i.screens_asleep {
        return FrameBudget::Paused(PauseReason::ScreenAsleep);
    }
    if i.user_paused {
        return FrameBudget::Paused(PauseReason::UserPaused);
    }
    if i.interactive {
        return FrameBudget::Fps(s.interactive_fps);
    }
    if d.occluded {
        return FrameBudget::Paused(PauseReason::Occluded);
    }
    let on_battery = matches!(i.power, PowerState::Battery | PowerState::LowPower);
    if on_battery && s.pause_on_battery {
        return FrameBudget::Paused(PauseReason::OnBattery);
    }
    let passive = match d.activity {
        Activity::Idle => IDLE_FPS.min(s.passive_fps),
        Activity::Animating => s.passive_fps,
    };
    if i.power == PowerState::LowPower {
        return FrameBudget::Fps(s.low_power_fps.min(passive));
    }
    if i.power == PowerState::Battery {
        let battery_rate = if d.uncovered || d.activity == Activity::Animating {
            s.passive_fps
        } else {
            passive
        };
        return FrameBudget::Fps(battery_rate.min(BATTERY_FPS_CAP));
    }

    // On AC:
    // Dynamic Adaptive Frame Pacing (60 FPS Boost):
    // Dynamically ramp up to 60 FPS (interactive_fps) during active moments (bird flights,
    // feeding frenzies, user dragging) or when all foreground windows are minimized/closed (uncovered).
    // Gracefully fall back to 15-24 FPS during idle passive periods.
    if d.activity == Activity::Animating || d.uncovered {
        FrameBudget::Fps(s.interactive_fps)
    } else {
        FrameBudget::Fps(passive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(occluded: bool, activity: Activity, uncovered: bool) -> DisplayInputs {
        DisplayInputs {
            id: DisplayId::new("d0"),
            occluded,
            activity,
            uncovered,
        }
    }

    struct Case {
        name: &'static str,
        occluded: bool,
        activity: Activity,
        uncovered: bool,
        power: PowerState,
        interactive: bool,
        user_paused: bool,
        screens_asleep: bool,
        pause_on_battery: bool,
        expected: FrameBudget,
    }

    impl Default for Case {
        fn default() -> Self {
            Self {
                name: "",
                occluded: false,
                activity: Activity::Idle,
                uncovered: false,
                power: PowerState::Ac,
                interactive: false,
                user_paused: false,
                screens_asleep: false,
                pause_on_battery: false,
                expected: FrameBudget::Fps(IDLE_FPS),
            }
        }
    }

    #[test]
    fn rule_table() {
        use FrameBudget::*;
        use PauseReason::*;
        let cases = [
            Case {
                name: "idle on AC",
                expected: Fps(15),
                ..Default::default()
            },
            Case {
                name: "animating on AC ramps to 60 FPS boost",
                activity: Activity::Animating,
                expected: Fps(60),
                ..Default::default()
            },
            Case {
                name: "uncovered on AC ramps to 60 FPS boost",
                uncovered: true,
                expected: Fps(60),
                ..Default::default()
            },
            Case {
                name: "uncovered on battery respects battery cap",
                uncovered: true,
                power: PowerState::Battery,
                expected: Fps(20),
                ..Default::default()
            },
            Case {
                name: "occluded pauses",
                occluded: true,
                expected: Paused(Occluded),
                ..Default::default()
            },
            Case {
                name: "occlusion overrides uncovered desktop",
                occluded: true,
                uncovered: true,
                expected: Paused(Occluded),
                ..Default::default()
            },
            Case {
                name: "interactive overrides occlusion",
                occluded: true,
                interactive: true,
                expected: Fps(60),
                ..Default::default()
            },
            Case {
                name: "user pause overrides interactive",
                interactive: true,
                user_paused: true,
                expected: Paused(UserPaused),
                ..Default::default()
            },
            Case {
                name: "screens asleep overrides everything",
                interactive: true,
                user_paused: true,
                screens_asleep: true,
                expected: Paused(ScreenAsleep),
                ..Default::default()
            },
            Case {
                name: "battery caps animating rate",
                activity: Activity::Animating,
                power: PowerState::Battery,
                expected: Fps(20),
                ..Default::default()
            },
            Case {
                name: "battery idle stays at idle rate",
                power: PowerState::Battery,
                expected: Fps(15),
                ..Default::default()
            },
            Case {
                name: "low power uses low-power rate",
                activity: Activity::Animating,
                power: PowerState::LowPower,
                expected: Fps(12),
                ..Default::default()
            },
            Case {
                name: "pause on battery",
                power: PowerState::Battery,
                pause_on_battery: true,
                expected: Paused(OnBattery),
                ..Default::default()
            },
            Case {
                name: "pause on battery ignored on AC",
                pause_on_battery: true,
                expected: Fps(15),
                ..Default::default()
            },
            Case {
                name: "interactive ignores battery",
                power: PowerState::LowPower,
                interactive: true,
                pause_on_battery: true,
                expected: Fps(60),
                ..Default::default()
            },
        ];

        for c in cases {
            let settings = FrameSettings {
                pause_on_battery: c.pause_on_battery,
                ..Default::default()
            };
            let displays = [display(c.occluded, c.activity, c.uncovered)];
            let out = compute(&GovernorInputs {
                displays: &displays,
                power: c.power,
                interactive: c.interactive,
                user_paused: c.user_paused,
                screens_asleep: c.screens_asleep,
                settings: &settings,
            });
            assert_eq!(out[0].1, c.expected, "case: {}", c.name);
        }
    }

    #[test]
    fn per_display_occlusion_is_independent() {
        let displays = [
            DisplayInputs {
                id: DisplayId::new("a"),
                occluded: true,
                activity: Activity::Idle,
                uncovered: false,
            },
            DisplayInputs {
                id: DisplayId::new("b"),
                occluded: false,
                activity: Activity::Animating,
                uncovered: false,
            },
        ];
        let settings = FrameSettings::default();
        let out = compute(&GovernorInputs {
            displays: &displays,
            power: PowerState::Ac,
            interactive: false,
            user_paused: false,
            screens_asleep: false,
            settings: &settings,
        });
        assert_eq!(
            out[0],
            (
                DisplayId::new("a"),
                FrameBudget::Paused(PauseReason::Occluded)
            )
        );
        assert_eq!(out[1], (DisplayId::new("b"), FrameBudget::Fps(60)));
    }

    #[test]
    fn out_of_range_settings_are_clamped() {
        let settings = FrameSettings {
            passive_fps: 500,
            interactive_fps: 1,
            low_power_fps: 0,
            pause_on_battery: false,
        };
        let c = settings.clamped();
        assert_eq!(c.passive_fps, 30);
        assert_eq!(c.interactive_fps, 30);
        assert_eq!(c.low_power_fps, 5);
    }

    #[test]
    fn idle_rate_never_exceeds_passive_cap() {
        // A user could not set passive below 15 (clamped), so idle == 15 == min.
        let settings = FrameSettings {
            passive_fps: 15,
            ..Default::default()
        };
        let displays = [display(false, Activity::Idle, false)];
        let out = compute(&GovernorInputs {
            displays: &displays,
            power: PowerState::Ac,
            interactive: false,
            user_paused: false,
            screens_asleep: false,
            settings: &settings,
        });
        assert_eq!(out[0].1, FrameBudget::Fps(15));
    }

    #[test]
    fn budget_serializes_for_ui() {
        let json = serde_json::to_string(&FrameBudget::Paused(PauseReason::Occluded)).unwrap();
        assert_eq!(json, r#"{"kind":"paused","value":"occluded"}"#);
        let json = serde_json::to_string(&FrameBudget::Fps(24)).unwrap();
        assert_eq!(json, r#"{"kind":"fps","value":24}"#);
    }
}
