//! Bespoke 5-minute (`300s`) looping room activities for Station HELIOS-09.
//!
//! Each of the five main interior rooms executes a deterministic, multi-stage 300-second
//! operational cycle with clear 3D visual progress, task-directed robot choreography,
//! and a seamless end-of-cycle reset finale:
//! 1. Main Hangar Bay (`CAM-04`): Cargo Intake, Hydraulic Compactor & Gantry Crane Cycle
//! 2. Central Core Atrium (`CAM-07`): 12-Sector Deck Decontamination & 4-Pillar Security Audit
//! 3. Ops Command Spire (`CAM-08`): 6-Layer 3D Holographic Star-Chart Reconstruction
//! 4. Concourse & Reactor Core (`CAM-09`): 4-Manifold Cryo-Rod Swap & Vat Quench Cycle
//! 5. Aft Solar Engineering Bay (`CAM-10`): 4-Tower Capacitor Charging & Grid Discharge

use std::f32::consts::{FRAC_PI_2, PI};

use serde::{Deserialize, Serialize};

use super::types::{AgentKind, AgentWorkAction, AgentZone, CargoVisualKind};

/// Standard duration of every room's visual activity loop (5 minutes = 300 seconds).
pub const ROOM_CYCLE_DURATION_SECS: f32 = 300.0;

/// Smooth cubic Hermite interpolation clamped to `[0, 1]`.
#[inline]
fn smooth01(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

/// Directed goal issued by a room's 5-minute activity coordinator to a resident robot.
#[derive(Clone, Copy, Debug)]
pub struct ActivityDirective {
    pub goal_node: usize,
    pub work_action: AgentWorkAction,
    pub face_yaw: Option<f32>,
    pub dwell_secs: f32,
    pub cargo_after_dwell: CargoVisualKind,
}

// =============================================================================
// 1. MAIN HANGAR BAY — Cargo Compactor & Gantry Crane Cycle (300s)
// =============================================================================

pub const HANGAR_CRATES_PER_CYCLE: u8 = 6;
const HANGAR_WORK_END_SECS: f32 = 255.0; // 6 crates * 42.5s = 255s; 45s gantry finale

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HangarCompactorState {
    /// Elapsed seconds in `[0.0, 300.0)`.
    pub cycle_secs: f32,
    /// Number of raw crates remaining on the staging rack (`0..=6`).
    pub raw_crates_remaining: u8,
    /// Number of compacted alloy bales stacked on the output pallet (`0..=6`).
    pub compacted_bales: u8,
    /// `[0.0, 1.0]` hydraulic ram press depth (`0` = retracted, `1` = fully crushed).
    pub ram_depth: f32,
    /// True while a crate is inside the compactor hopper being crushed.
    pub crate_in_hopper: bool,
    /// `[0.0, 1.0]` squash factor of the crate inside the hopper.
    pub hopper_squash: f32,
    /// Animated overhead gantry crane bridge Z coordinate (`18.0..=34.0`).
    pub gantry_z: f32,
    /// Animated overhead gantry crane magnetic trolley X coordinate (`-9.4..=9.4`).
    pub gantry_trolley_x: f32,
    /// `[0.0, 1.0]` magnetic grapple cable extension downward.
    pub gantry_hoist: f32,
    /// `0` = empty, `1` = hoisting 6-bale pallet, `2` = lowering fresh 6-crate supply pod.
    pub gantry_payload: u8,
    /// True during the 45-second gantry hoist & replenish finale.
    pub in_finale: bool,
}

impl Default for HangarCompactorState {
    fn default() -> Self {
        let mut s = Self {
            cycle_secs: 15.0, // Staggered 5% initial phase
            raw_crates_remaining: HANGAR_CRATES_PER_CYCLE,
            compacted_bales: 0,
            ram_depth: 0.0,
            crate_in_hopper: false,
            hopper_squash: 0.0,
            gantry_z: 25.0,
            gantry_trolley_x: 0.0,
            gantry_hoist: 0.0,
            gantry_payload: 0,
            in_finale: false,
        };
        s.sync_from_clock();
        s
    }
}

impl HangarCompactorState {
    pub fn advance(&mut self, dt: f32) {
        self.cycle_secs = (self.cycle_secs + dt).rem_euclid(ROOM_CYCLE_DURATION_SECS);
        self.sync_from_clock();
    }

    pub fn progress(&self) -> f32 {
        (self.cycle_secs / ROOM_CYCLE_DURATION_SECS).clamp(0.0, 1.0)
    }

    /// Sub-cycle normalized progress `u` in `[0.0, 1.0)` during active hauling, or `None` in finale.
    pub fn active_crate_sub_u(&self) -> Option<f32> {
        if self.cycle_secs >= HANGAR_WORK_END_SECS {
            return None;
        }
        let per_crate = HANGAR_WORK_END_SECS / HANGAR_CRATES_PER_CYCLE as f32;
        Some((self.cycle_secs % per_crate) / per_crate)
    }

    fn sync_from_clock(&mut self) {
        let t = self.cycle_secs;
        if t < HANGAR_WORK_END_SECS {
            self.in_finale = false;
            let per_crate = HANGAR_WORK_END_SECS / HANGAR_CRATES_PER_CYCLE as f32;
            let idx = ((t / per_crate).floor() as u8).min(HANGAR_CRATES_PER_CYCLE - 1);
            let u = (t - idx as f32 * per_crate) / per_crate;

            // Staging rack depletes when forklift lifts crate at u = 0.16
            let taken = idx + u8::from(u >= 0.16);
            self.raw_crates_remaining = HANGAR_CRATES_PER_CYCLE.saturating_sub(taken);

            // Output pallet increments when ram finishes crushing at u = 0.84
            self.compacted_bales = (idx + u8::from(u >= 0.84)).min(HANGAR_CRATES_PER_CYCLE);

            // Hopper & hydraulic ram animation (`u = 0.56..0.84`)
            if (0.56..0.84).contains(&u) {
                self.crate_in_hopper = true;
                if u < 0.62 {
                    self.ram_depth = 0.0;
                    self.hopper_squash = 0.0;
                } else if u < 0.74 {
                    let crush = smooth01((u - 0.62) / 0.12);
                    self.ram_depth = crush;
                    self.hopper_squash = crush;
                } else {
                    let retract = smooth01((u - 0.74) / 0.10);
                    self.ram_depth = 1.0 - retract;
                    self.hopper_squash = 1.0;
                }
            } else {
                self.crate_in_hopper = false;
                self.ram_depth = 0.0;
                self.hopper_squash = 0.0;
            }

            // Gantry crane idles cleanly near center bay during active hauling
            self.gantry_z = 25.0;
            self.gantry_trolley_x = 0.0;
            self.gantry_hoist = 0.0;
            self.gantry_payload = 0;
        } else {
            // 45-second Gantry Crane Hoist & Replenish Finale (`255..300s`)
            self.in_finale = true;
            self.crate_in_hopper = false;
            self.ram_depth = 0.0;
            self.hopper_squash = 0.0;

            let ft = t - HANGAR_WORK_END_SECS; // 0..45s
            if ft < 10.0 {
                // Roll to output pallet (Z=25.0, X=+9.4) and lower magnetic grapple
                let u = smooth01(ft / 10.0);
                self.gantry_z = 25.0;
                self.gantry_trolley_x = 9.4 * smooth01(ft / 4.5);
                self.gantry_hoist = if ft > 4.5 {
                    smooth01((ft - 4.5) / 5.5)
                } else {
                    0.0
                };
                self.gantry_payload = u8::from(u > 0.92);
                self.compacted_bales = HANGAR_CRATES_PER_CYCLE;
                self.raw_crates_remaining = 0;
            } else if ft < 22.0 {
                // Hoist 6-bale pallet up into ceiling bay and stow at Z = 18.0
                let u = smooth01((ft - 10.0) / 12.0);
                self.gantry_trolley_x = 9.4 * (1.0 - smooth01((ft - 16.0) / 6.0));
                self.gantry_z = 25.0 - 7.0 * smooth01((ft - 15.0) / 7.0);
                self.gantry_hoist = (1.0 - smooth01((ft - 10.0) / 6.0)).clamp(0.0, 1.0);
                self.gantry_payload = u8::from(u < 0.92);
                self.compacted_bales = 0;
                self.raw_crates_remaining = 0;
            } else if ft < 36.0 {
                // Roll to staging rack (Z=31.4, X=+9.4) carrying fresh 6-crate supply pod & lower
                let move_u = smooth01((ft - 22.0) / 6.0);
                self.gantry_z = 18.0 + (31.4 - 18.0) * move_u;
                self.gantry_trolley_x = 9.4 * move_u;
                self.gantry_hoist = if ft > 28.0 {
                    smooth01((ft - 28.0) / 7.0)
                } else {
                    0.0
                };
                self.gantry_payload = if ft < 35.0 { 2 } else { 0 };
                self.compacted_bales = 0;
                self.raw_crates_remaining = if ft >= 35.0 {
                    HANGAR_CRATES_PER_CYCLE
                } else {
                    0
                };
            } else {
                // Retract grapple and return crane to home position
                let ret_u = smooth01((ft - 36.0) / 9.0);
                self.gantry_hoist = (1.0 - smooth01((ft - 36.0) / 4.5)).clamp(0.0, 1.0);
                self.gantry_z = 31.4 + (25.0 - 31.4) * ret_u;
                self.gantry_trolley_x = 9.4 * (1.0 - ret_u);
                self.gantry_payload = 0;
                self.compacted_bales = 0;
                self.raw_crates_remaining = HANGAR_CRATES_PER_CYCLE;
            }
        }
    }

    pub fn hud_status(&self) -> String {
        let pct = (self.progress() * 100.0).round() as u32;
        if self.in_finale {
            format!("GANTRY RESET · {pct}%")
        } else {
            format!(
                "COMPACTOR · {}/{} BALES ({pct}%)",
                self.compacted_bales, HANGAR_CRATES_PER_CYCLE
            )
        }
    }
}

// =============================================================================
// 2. CENTRAL CORE ATRIUM — 12 Wall Control Panels, Mini-Elevator & Light Show
// =============================================================================

pub const ATRIUM_PANEL_COUNT: usize = 12;
pub const ATRIUM_CELEBRATION_SECS: f32 = 12.0;
pub const ATRIUM_PANEL_WORK_SECS: f32 = 4.4;
pub const ATRIUM_LIFT_X: f32 = 8.8;
pub const ATRIUM_LIFT_Z: f32 = 8.4;
pub const ATRIUM_LIFT_LOW_Y: f32 = super::nav::D2;
pub const ATRIUM_LIFT_HIGH_Y: f32 = super::nav::D2U;
pub const ATRIUM_LIFT_LOW_NODE: usize = 45;
pub const ATRIUM_LIFT_HIGH_NODE: usize = 59;

/// Static 3D mounting descriptor for an Atrium wall/pillar control panel.
#[derive(Clone, Copy, Debug)]
pub struct AtriumPanelSpec {
    /// World `[x, y, z]` center of the glowing panel mounted flush on a wall/pillar/mullion.
    pub wall_pos: [f32; 3],
    /// Outward surface normal `[nx, ny, nz]` pointing into the room.
    pub normal: [f32; 3],
    /// `NAV_NODES` index where a `PanelTechBot` stands to calibrate this panel.
    pub work_node: usize,
    /// Yaw angle (radians) for the robot to face directly into the panel.
    pub face_yaw: f32,
    /// True if mounted on the Upper U-Mezzanine (`Y = 3.225`), false on the forward ground floor.
    pub upper_level: bool,
}

/// 12 Atrium Control Panels: 5 on the elevator-free forward ground half (`Z >= +3.5`)
/// and 7 along the Upper U-Mezzanine (Starboard mullions, Rear skybridge wall, Port mullions).
pub const ATRIUM_PANELS: [AtriumPanelSpec; ATRIUM_PANEL_COUNT] = [
    // --- Ground Floor Forward Half (0..=4) ---
    AtriumPanelSpec {
        wall_pos: [11.12, -2.0, 3.5],
        normal: [-1.0, 0.0, 0.0],
        work_node: 54,
        face_yaw: FRAC_PI_2,
        upper_level: false,
    },
    AtriumPanelSpec {
        wall_pos: [11.12, -2.0, 6.0],
        normal: [-1.0, 0.0, 0.0],
        work_node: 55,
        face_yaw: FRAC_PI_2,
        upper_level: false,
    },
    AtriumPanelSpec {
        wall_pos: [5.75, -2.0, 5.2],
        normal: [-1.0, 0.0, 0.0],
        work_node: 56,
        face_yaw: FRAC_PI_2,
        upper_level: false,
    },
    AtriumPanelSpec {
        wall_pos: [-5.75, -2.0, 5.2],
        normal: [1.0, 0.0, 0.0],
        work_node: 57,
        face_yaw: -FRAC_PI_2,
        upper_level: false,
    },
    AtriumPanelSpec {
        wall_pos: [-11.12, -2.0, 3.5],
        normal: [1.0, 0.0, 0.0],
        work_node: 58,
        face_yaw: -FRAC_PI_2,
        upper_level: false,
    },
    // --- Upper U-Mezzanine Walkways (5..=11) ---
    AtriumPanelSpec {
        wall_pos: [11.05, 5.0, 2.55],
        normal: [-1.0, 0.0, 0.0],
        work_node: 60,
        face_yaw: FRAC_PI_2,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [11.05, 5.0, -2.55],
        normal: [-1.0, 0.0, 0.0],
        work_node: 61,
        face_yaw: FRAC_PI_2,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [11.05, 5.0, -8.2],
        normal: [-1.0, 0.0, 0.0],
        work_node: 62,
        face_yaw: FRAC_PI_2,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [3.6, 5.0, -14.1],
        normal: [0.0, 0.0, 1.0],
        work_node: 64,
        face_yaw: PI,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [-3.6, 5.0, -14.1],
        normal: [0.0, 0.0, 1.0],
        work_node: 65,
        face_yaw: PI,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [-11.05, 5.0, -8.2],
        normal: [1.0, 0.0, 0.0],
        work_node: 67,
        face_yaw: -FRAC_PI_2,
        upper_level: true,
    },
    AtriumPanelSpec {
        wall_pos: [-11.05, 5.0, -2.55],
        normal: [1.0, 0.0, 0.0],
        work_node: 68,
        face_yaw: -FRAC_PI_2,
        upper_level: true,
    },
];

/// Operational status of a single Atrium wall control panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelStatus {
    /// Waiting for calibration (glowing electric blue).
    #[default]
    Pending,
    /// Robot is actively calibrating this panel (flashing multicolor animation).
    Working,
    /// Calibration complete (steady bright emerald green).
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AtriumPanelRuntime {
    #[serde(default)]
    pub status: PanelStatus,
    /// `[0.0, 1.0]` calibration progress while `status == Working`.
    #[serde(default)]
    pub progress: f32,
    /// Agent ID currently claiming/working on this panel, if any.
    #[serde(default)]
    pub assigned_bot: Option<u32>,
}

/// Phase of the dedicated Starboard Atrium Bot Mini-Elevator (`X = +8.8, Z = +8.4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiniLiftPhase {
    #[default]
    Idle,
    MovingToPickup,
    Boarding,
    Riding,
    Exiting,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtriumMiniLiftState {
    #[serde(default = "default_lift_y")]
    pub current_y: f32,
    #[serde(default)]
    pub target_upper: bool,
    #[serde(default)]
    pub rider_id: Option<u32>,
    #[serde(default)]
    pub phase: MiniLiftPhase,
}

fn default_lift_y() -> f32 {
    ATRIUM_LIFT_LOW_Y
}

impl Default for AtriumMiniLiftState {
    fn default() -> Self {
        Self {
            current_y: ATRIUM_LIFT_LOW_Y,
            target_upper: false,
            rider_id: None,
            phase: MiniLiftPhase::Idle,
        }
    }
}

fn default_atrium_panels() -> [AtriumPanelRuntime; ATRIUM_PANEL_COUNT] {
    [AtriumPanelRuntime::default(); ATRIUM_PANEL_COUNT]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtriumMaintenanceState {
    #[serde(default = "default_atrium_panels")]
    pub panels: [AtriumPanelRuntime; ATRIUM_PANEL_COUNT],
    #[serde(default)]
    pub mini_lift: AtriumMiniLiftState,
    /// True during the ~16s whole-room light-show finale after all 12 panels turn green.
    #[serde(default)]
    pub celebrating: bool,
    /// Remaining seconds in the celebration light show (`16.0 -> 0.0`).
    #[serde(default)]
    pub celebration_timer: f32,
    /// Total full 12-panel calibration + celebration cycles completed.
    #[serde(default)]
    pub cycles_completed: u32,
}

impl Default for AtriumMaintenanceState {
    fn default() -> Self {
        Self {
            panels: default_atrium_panels(),
            mini_lift: AtriumMiniLiftState::default(),
            celebrating: false,
            celebration_timer: 0.0,
            cycles_completed: 0,
        }
    }
}

impl AtriumMaintenanceState {
    pub fn advance(&mut self, dt: f32) {
        if self.celebrating {
            self.celebration_timer = (self.celebration_timer - dt).max(0.0);
            if self.celebration_timer <= 0.0 {
                self.celebrating = false;
                self.cycles_completed = self.cycles_completed.saturating_add(1);
                self.reset_panels();
            }
        }
    }

    pub fn fixed_count(&self) -> usize {
        self.panels
            .iter()
            .filter(|p| p.status == PanelStatus::Fixed)
            .count()
    }

    pub fn all_fixed(&self) -> bool {
        self.panels.iter().all(|p| p.status == PanelStatus::Fixed)
    }

    pub fn reset_panels(&mut self) {
        for p in &mut self.panels {
            p.status = PanelStatus::Pending;
            p.progress = 0.0;
            p.assigned_bot = None;
        }
    }

    pub fn trigger_celebration(&mut self) {
        self.celebrating = true;
        self.celebration_timer = ATRIUM_CELEBRATION_SECS;
        for p in &mut self.panels {
            p.assigned_bot = None;
            p.progress = 1.0;
        }
    }

    pub fn progress(&self) -> f32 {
        if self.celebrating {
            1.0
        } else {
            (self.fixed_count() as f32 / ATRIUM_PANEL_COUNT as f32).clamp(0.0, 1.0)
        }
    }

    pub fn hud_status(&self) -> String {
        if self.celebrating {
            "ATRIUM SYNC 100% · LIGHT SHOW".to_string()
        } else {
            let fixed = self.fixed_count();
            let pct = (self.progress() * 100.0).round() as u32;
            format!("PANEL SYNC · {fixed}/{ATRIUM_PANEL_COUNT} ONLINE ({pct}%)")
        }
    }
}

// =============================================================================
// 3. OPS COMMAND SPIRE — 6-Layer 3D Holographic Star-Chart Sync (300s)
// =============================================================================

pub const OPS_HOLO_LAYERS: u8 = 6;
const OPS_BUILD_END_SECS: f32 = 255.0;
const OPS_BROADCAST_END_SECS: f32 = 285.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpsHoloSyncState {
    pub cycle_secs: f32,
    /// Number of fully materialized 3D hologram layers (`0..=6`).
    pub completed_layers: u8,
    /// `[0.0, 1.0]` materialization progress of the currently building layer.
    pub active_layer_t: f32,
    /// True during the 30s full-hologram broadcast celebration (`255..285s`).
    pub broadcasting: bool,
    /// `[0.0, 1.0]` archive-collapse progress (`285..300s`).
    pub archive_collapse: f32,
}

impl Default for OpsHoloSyncState {
    fn default() -> Self {
        let mut s = Self {
            cycle_secs: 126.0, // Staggered 42% initial phase
            completed_layers: 0,
            active_layer_t: 0.0,
            broadcasting: false,
            archive_collapse: 0.0,
        };
        s.sync_from_clock();
        s
    }
}

impl OpsHoloSyncState {
    pub fn advance(&mut self, dt: f32) {
        self.cycle_secs = (self.cycle_secs + dt).rem_euclid(ROOM_CYCLE_DURATION_SECS);
        self.sync_from_clock();
    }

    pub fn progress(&self) -> f32 {
        (self.cycle_secs / ROOM_CYCLE_DURATION_SECS).clamp(0.0, 1.0)
    }

    /// Smooth `[0.0, 1.0]` visibility scale for layer `layer_idx` (`0..6`).
    pub fn layer_scale(&self, layer_idx: u8) -> f32 {
        let base = if layer_idx < self.completed_layers {
            1.0
        } else if layer_idx == self.completed_layers && !self.broadcasting {
            self.active_layer_t
        } else {
            0.0
        };
        base * (1.0 - self.archive_collapse)
    }

    fn sync_from_clock(&mut self) {
        let t = self.cycle_secs;
        if t < OPS_BUILD_END_SECS {
            let per_layer = OPS_BUILD_END_SECS / OPS_HOLO_LAYERS as f32;
            let done = ((t / per_layer).floor() as u8).min(OPS_HOLO_LAYERS);
            self.completed_layers = done;
            self.active_layer_t = if done < OPS_HOLO_LAYERS {
                smooth01((t - done as f32 * per_layer) / per_layer)
            } else {
                1.0
            };
            self.broadcasting = false;
            self.archive_collapse = 0.0;
        } else if t < OPS_BROADCAST_END_SECS {
            self.completed_layers = OPS_HOLO_LAYERS;
            self.active_layer_t = 1.0;
            self.broadcasting = true;
            self.archive_collapse = 0.0;
        } else {
            self.completed_layers = OPS_HOLO_LAYERS;
            self.active_layer_t = 1.0;
            self.broadcasting = false;
            self.archive_collapse = smooth01((t - OPS_BROADCAST_END_SECS) / 15.0);
        }
    }

    pub fn hud_status(&self) -> String {
        let pct = (self.progress() * 100.0).round() as u32;
        if self.broadcasting {
            format!("HOLO-BROADCAST · 6/6 ({pct}%)")
        } else if self.archive_collapse > 0.01 {
            format!("HOLO-ARCHIVE RESET · {pct}%")
        } else {
            format!(
                "HOLO-SYNC · LAYER {}/{} ({pct}%)",
                self.completed_layers, OPS_HOLO_LAYERS
            )
        }
    }
}

// =============================================================================
// 4. CONCOURSE & REACTOR CORE — 4-Manifold Cryo-Rod Replacement Cycle (300s)
// =============================================================================

pub const REACTOR_ROD_COUNT: usize = 4;
const REACTOR_WORK_END_SECS: f32 = 255.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReactorCryoCycleState {
    pub cycle_secs: f32,
    /// Thermal heat `[0.0 = cool cyan, 1.0 = hot orange]` of each of the 4 core manifolds.
    pub rod_heat: [f32; REACTOR_ROD_COUNT],
    /// Number of fresh cyan cryo-cells remaining on the Starboard Rack (`0..=4`).
    pub fresh_rack_count: u8,
    /// Number of spent orange rods deposited in the Port Cooling Vat (`0..=4`).
    pub vat_rod_count: u8,
    /// Number of rods swapped in the current 5-minute cycle (`0..=4`).
    pub rods_swapped: u8,
    /// `[0.0, 1.0]` Port Cooling Vat cryo-quench shroud progress during finale (`255..300s`).
    pub vat_quench_t: f32,
    /// `[0.0, 1.0]` overhead pneumatic cell return pulse (`0` = Port Vat, `1` = Starboard Rack).
    pub pneumatic_pulse_t: f32,
    /// True during the 45s cryo-quench & pneumatic restock finale.
    pub quenching: bool,
}

impl Default for ReactorCryoCycleState {
    fn default() -> Self {
        let mut s = Self {
            cycle_secs: 186.0, // Staggered 62% initial phase
            rod_heat: [0.85, 0.65, 0.45, 0.25],
            fresh_rack_count: REACTOR_ROD_COUNT as u8,
            vat_rod_count: 0,
            rods_swapped: 0,
            vat_quench_t: 0.0,
            pneumatic_pulse_t: 0.0,
            quenching: false,
        };
        s.sync_from_clock();
        s
    }
}

impl ReactorCryoCycleState {
    pub fn advance(&mut self, dt: f32) {
        self.cycle_secs = (self.cycle_secs + dt).rem_euclid(ROOM_CYCLE_DURATION_SECS);
        self.sync_from_clock();
    }

    pub fn progress(&self) -> f32 {
        (self.cycle_secs / ROOM_CYCLE_DURATION_SECS).clamp(0.0, 1.0)
    }

    fn sync_from_clock(&mut self) {
        let t = self.cycle_secs;
        if t < REACTOR_WORK_END_SECS {
            self.quenching = false;
            self.vat_quench_t = 0.0;
            self.pneumatic_pulse_t = 0.0;

            let per_rod = REACTOR_WORK_END_SECS / REACTOR_ROD_COUNT as f32;
            let idx = ((t / per_rod).floor() as usize).min(REACTOR_ROD_COUNT - 1);
            let u = (t - idx as f32 * per_rod) / per_rod;

            let swapped = idx + usize::from(u >= 0.62);
            self.rods_swapped = swapped.min(REACTOR_ROD_COUNT) as u8;
            let taken_fresh = idx + usize::from(u >= 0.22);
            self.fresh_rack_count = (REACTOR_ROD_COUNT as u8).saturating_sub(taken_fresh as u8);
            let deposited = idx + usize::from(u >= 0.88);
            self.vat_rod_count = (deposited as u8).min(REACTOR_ROD_COUNT as u8);

            for (i, heat) in self.rod_heat.iter_mut().enumerate() {
                if i < idx || (i == idx && u >= 0.62) {
                    // Freshly swapped rod starts cool cyan and slowly warms slightly over the cycle
                    let since_swap = if i == idx {
                        (u - 0.62) * 0.25
                    } else {
                        0.08 + ((idx - i) as f32 * 0.14)
                    };
                    *heat = since_swap.clamp(0.05, 0.52);
                } else if i == idx {
                    // Currently active manifold heats up to peak orange until swapped at u=0.62
                    *heat = (0.78 + u * 0.35).clamp(0.0, 1.0);
                } else {
                    // Waiting manifolds gradually heat up as their turn approaches
                    let ahead = (i - idx) as f32;
                    *heat = (0.82 - ahead * 0.14 + u * 0.12).clamp(0.35, 0.95);
                }
            }
        } else {
            // 45-second Cryo-Quench & Pneumatic Restock Finale (`255..300s`)
            self.quenching = true;
            self.rods_swapped = REACTOR_ROD_COUNT as u8;
            let ft = t - REACTOR_WORK_END_SECS; // 0..45s
            if ft < 22.0 {
                // Shroud lowers over Port Cooling Vat & quenches all 4 rods to cyan
                self.vat_quench_t = smooth01(ft / 10.0);
                self.pneumatic_pulse_t = 0.0;
                self.vat_rod_count = REACTOR_ROD_COUNT as u8;
                self.fresh_rack_count = 0;
            } else {
                // Shroud retracts & overhead pneumatic transfer restocks Starboard Rack
                let ret_u = smooth01((ft - 22.0) / 23.0);
                self.vat_quench_t = (1.0 - smooth01((ft - 22.0) / 8.0)).clamp(0.0, 1.0);
                self.pneumatic_pulse_t = ((ft - 22.0) / 5.75).fract();
                let returned = ((ret_u * 4.0).floor() as u8).min(REACTOR_ROD_COUNT as u8);
                self.fresh_rack_count = returned;
                self.vat_rod_count = (REACTOR_ROD_COUNT as u8).saturating_sub(returned);
            }
            // Core rods pulse cool cyan during stabilized surge, then warm toward next shift
            let warm = smooth01((ft - 30.0) / 15.0) * 0.35;
            self.rod_heat = [0.20 + warm, 0.16 + warm, 0.12 + warm, 0.08 + warm];
        }
    }

    pub fn hud_status(&self) -> String {
        let pct = (self.progress() * 100.0).round() as u32;
        if self.quenching {
            format!("VAT CRYO-QUENCH · {pct}%")
        } else {
            format!(
                "CRYO-SWAP · {}/{} RODS ({pct}%)",
                self.rods_swapped, REACTOR_ROD_COUNT
            )
        }
    }
}

// =============================================================================
// 5. AFT SOLAR ENGINEERING BAY — 4-Tower Capacitor Charging & Discharge (300s)
// =============================================================================

pub const SOLAR_TOWER_COUNT: usize = 4;
const SOLAR_CHARGE_END_SECS: f32 = 260.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolarGridChargeState {
    pub cycle_secs: f32,
    /// `[0.0, 1.0]` charge level of each of the 4 Capacitor Towers:
    /// `[0]` Port-Fore, `[1]` Stbd-Fore, `[2]` Port-Aft, `[3]` Stbd-Aft.
    pub tower_charge: [f32; SOLAR_TOWER_COUNT],
    /// `[0.0, 1.0]` Rear Super-Capacitor Accumulator total charge.
    pub grid_charge: f32,
    /// True during the 40s overhead golden-cyan Grid Discharge Finale (`260..300s`).
    pub discharging: bool,
    /// `[0.0, 1.0]` intensity of the discharge energy arcs & beam-out.
    pub discharge_intensity: f32,
}

impl Default for SolarGridChargeState {
    fn default() -> Self {
        let mut s = Self {
            cycle_secs: 234.0, // Staggered 78% initial phase
            tower_charge: [0.0; SOLAR_TOWER_COUNT],
            grid_charge: 0.0,
            discharging: false,
            discharge_intensity: 0.0,
        };
        s.sync_from_clock();
        s
    }
}

impl SolarGridChargeState {
    pub fn advance(&mut self, dt: f32) {
        self.cycle_secs = (self.cycle_secs + dt).rem_euclid(ROOM_CYCLE_DURATION_SECS);
        self.sync_from_clock();
    }

    pub fn progress(&self) -> f32 {
        (self.cycle_secs / ROOM_CYCLE_DURATION_SECS).clamp(0.0, 1.0)
    }

    fn sync_from_clock(&mut self) {
        let t = self.cycle_secs;
        if t < SOLAR_CHARGE_END_SECS {
            self.discharging = false;
            self.discharge_intensity = 0.0;
            let base_u = (t / SOLAR_CHARGE_END_SECS).clamp(0.0, 1.0);
            // Slightly staggered per-tower charge progression so all 4 climb distinctly to 100%
            let offsets = [0.04, 0.0, -0.04, -0.08];
            for (i, &off) in offsets.iter().enumerate() {
                self.tower_charge[i] = ((base_u * 1.08 + off) / 1.04).clamp(0.0, 1.0);
            }
            self.grid_charge = self.tower_charge.iter().sum::<f32>() / SOLAR_TOWER_COUNT as f32;
        } else {
            self.discharging = true;
            let ft = t - SOLAR_CHARGE_END_SECS; // 0..40s
            let drain_u = smooth01(ft / 34.0);
            let remain = (1.0 - drain_u).clamp(0.0, 1.0);
            self.tower_charge = [remain; SOLAR_TOWER_COUNT];
            self.grid_charge = remain;
            // Intense arc burst during the first 30s of discharge, tapering out cleanly
            self.discharge_intensity = if ft < 30.0 {
                (0.75 + 0.25 * (ft * 2.5).sin().abs()) * (1.0 - smooth01((ft - 24.0) / 6.0))
            } else {
                0.0
            };
        }
    }

    pub fn hud_status(&self) -> String {
        let pct = (self.grid_charge * 100.0).round() as u32;
        if self.discharging {
            format!("GRID DISCHARGE · {pct}% OUTPUT")
        } else {
            let full = self.tower_charge.iter().filter(|&&c| c >= 0.95).count();
            format!("SOLAR CHARGE · {pct}% [{full}/4 FULL]")
        }
    }
}

// =============================================================================
// AGGREGATE ROOM ACTIVITIES COORDINATOR
// =============================================================================

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RoomActivitiesState {
    #[serde(default)]
    pub hangar: HangarCompactorState,
    #[serde(default)]
    pub atrium: AtriumMaintenanceState,
    #[serde(default)]
    pub ops: OpsHoloSyncState,
    #[serde(default)]
    pub reactor: ReactorCryoCycleState,
    #[serde(default)]
    pub solar: SolarGridChargeState,
}

impl RoomActivitiesState {
    pub fn step(&mut self, dt: f32) {
        self.hangar.advance(dt);
        self.atrium.advance(dt);
        self.ops.advance(dt);
        self.reactor.advance(dt);
        self.solar.advance(dt);
    }

    /// Compute the next task-directed workstation goal for a room robot based on its room's
    /// active 5-minute cycle, role slot, current node, and cargo state.
    pub fn next_directive(
        &self,
        kind: AgentKind,
        zone: AgentZone,
        role_slot: u8,
        cur_node: usize,
        cargo_kind: CargoVisualKind,
        rng_roll: usize,
    ) -> Option<ActivityDirective> {
        match (kind, zone) {
            // Hangar FloorScrubber: scrubs rear apron & hangar doorway sectors
            (AgentKind::FloorScrubber, AgentZone::HangarBay) => {
                let pool = [6usize, 7, 8, 9, 10];
                let mut goal = pool[rng_roll % pool.len()];
                if goal == cur_node {
                    goal = pool[(rng_roll + 1) % pool.len()];
                }
                Some(ActivityDirective {
                    goal_node: goal,
                    work_action: AgentWorkAction::Scrubbing,
                    face_yaw: None,
                    dwell_secs: 2.2,
                    cargo_after_dwell: CargoVisualKind::None,
                })
            }
            // OpsHoloDroids: alternate between harvesting HoloDataCubes at consoles/racks & beaming them into the Holotable
            (AgentKind::OpsHoloDroid, AgentZone::OpsDeck) => {
                let upload_node = if role_slot == 0 { 46 } else { 47 };
                let upload_yaw = if role_slot == 0 {
                    FRAC_PI_2
                } else {
                    -FRAC_PI_2
                };
                let console_pool: &[usize] = if role_slot == 0 {
                    &[24, 26, 29]
                } else {
                    &[25, 27, 30]
                };
                if cargo_kind == CargoVisualKind::HoloDataCube && cur_node != upload_node {
                    Some(ActivityDirective {
                        goal_node: upload_node,
                        work_action: AgentWorkAction::UploadingHolo,
                        face_yaw: Some(upload_yaw),
                        dwell_secs: 3.2,
                        cargo_after_dwell: CargoVisualKind::None,
                    })
                } else {
                    let mut goal = console_pool[rng_roll % console_pool.len()];
                    if goal == cur_node {
                        goal = console_pool[(rng_roll + 1) % console_pool.len()];
                    }
                    let face = match goal {
                        24 | 25 => 0.0,
                        26 => -FRAC_PI_2,
                        27 => FRAC_PI_2,
                        _ => PI,
                    };
                    Some(ActivityDirective {
                        goal_node: goal,
                        work_action: AgentWorkAction::DownloadingData,
                        face_yaw: Some(face),
                        dwell_secs: 2.8,
                        cargo_after_dwell: CargoVisualKind::HoloDataCube,
                    })
                }
            }
            // ReactorCoolantBots: Fresh Cryo Rack (49) -> Reactor Manifold (31..34) -> Spent Cooling Vat (48)
            (AgentKind::ReactorCoolantBot, _) => match cargo_kind {
                CargoVisualKind::FreshCryoCell => {
                    // Deliver to a reactor manifold node and swap for a spent orange rod!
                    let manifolds: &[usize] = if role_slot == 0 { &[33, 31] } else { &[34, 32] };
                    let mut goal = manifolds[rng_roll % manifolds.len()];
                    if goal == cur_node {
                        goal = manifolds[(rng_roll + 1) % manifolds.len()];
                    }
                    let face = match goal {
                        33 => FRAC_PI_2,
                        34 => -FRAC_PI_2,
                        31 => 2.25,
                        _ => -2.25,
                    };
                    Some(ActivityDirective {
                        goal_node: goal,
                        work_action: AgentWorkAction::SwappingCryoRod,
                        face_yaw: Some(face),
                        dwell_secs: 3.4,
                        cargo_after_dwell: CargoVisualKind::SpentFuelRod,
                    })
                }
                CargoVisualKind::SpentFuelRod => {
                    // Haul spent rod to Port Cooling Vat (node 48)
                    let goal = if cur_node == 48 { 37 } else { 48 };
                    Some(ActivityDirective {
                        goal_node: goal,
                        work_action: AgentWorkAction::LoadingCrate,
                        face_yaw: Some(-FRAC_PI_2),
                        dwell_secs: 2.4,
                        cargo_after_dwell: CargoVisualKind::None,
                    })
                }
                _ => {
                    // Pick up a fresh cyan cryo-cell at Starboard Rack (node 49)
                    let goal = if cur_node == 49 { 38 } else { 49 };
                    Some(ActivityDirective {
                        goal_node: goal,
                        work_action: AgentWorkAction::LoadingCrate,
                        face_yaw: Some(FRAC_PI_2),
                        dwell_secs: 2.4,
                        cargo_after_dwell: CargoVisualKind::FreshCryoCell,
                    })
                }
            },
            // SolarCapacitorBots: calibrate assigned Port (50, 52) or Stbd (51, 53) capacitor towers & sync at Rear Console (23)
            (AgentKind::SolarCapacitorBot, AgentZone::SolarBay) => {
                let stops: &[(usize, f32)] = if role_slot == 0 {
                    &[(50, -FRAC_PI_2), (52, -FRAC_PI_2), (23, PI)]
                } else {
                    &[(51, FRAC_PI_2), (53, FRAC_PI_2), (23, PI)]
                };
                let cur_idx = stops.iter().position(|&(n, _)| n == cur_node);
                let next_idx = match cur_idx {
                    Some(i) => (i + 1) % stops.len(),
                    None => rng_roll % stops.len(),
                };
                let (goal, face) = stops[next_idx];
                Some(ActivityDirective {
                    goal_node: goal,
                    work_action: AgentWorkAction::CalibratingCapacitor,
                    face_yaw: Some(face),
                    dwell_secs: 3.0,
                    cargo_after_dwell: CargoVisualKind::None,
                })
            }
            _ => None,
        }
    }
}
