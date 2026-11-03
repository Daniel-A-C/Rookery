//! Orbital Haven Simulation State (Milestone 4 - Slow-Burn Idle World).
//!
//! A procedural orbital station assembling modules and habitat rings over real time,
//! serviced by automated cargo transport shuttles against a planetary horizon.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::Activity;
use crate::world::WorldRng;

pub const ORBITAL_HAVEN_STATE_VERSION: u32 = 1;

/// Modular construction priority for station expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructionFocus {
    /// Expand solar energy collector wings.
    SolarPower,
    /// Assemble rotating centrifugal habitat ring sectors.
    HabitatRing,
    /// Build science sensor arrays and communications arrays.
    CommsGrid,
}

impl ConstructionFocus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SolarPower => "Solar Power Wings",
            Self::HabitatRing => "Habitat Ring Modules",
            Self::CommsGrid => "Sensor & Comms Array",
        }
    }
}

/// An autonomous cargo or crew shuttle visiting the orbital station.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shuttle {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub target_dock: [f32; 2],
    pub state: ShuttleState,
    pub docking_timer: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShuttleState {
    Approaching,
    Docked,
    Departing,
}

/// Telemetry and stats for the orbital haven station.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenStats {
    pub station_age_hours: f64,
    pub total_modules: u32,
    pub solar_wings: u32,
    pub habitat_sectors: u32,
    pub comm_arrays: u32,
    pub shuttles_serviced: u32,
}

/// Deterministic state of Orbital Haven.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub total_age_secs: f64,
    pub orbit_angle: f32,
    pub ring_rotation: f32,
    pub focus: ConstructionFocus,
    pub construction_progress: f32, // [0.0, 1.0] towards next module
    pub solar_wings: u32,           // up to 8
    pub habitat_sectors: u32,       // up to 12
    pub comm_arrays: u32,           // up to 6
    pub shuttles: Vec<Shuttle>,
    pub next_id: u32,
    pub stats: OrbitalHavenStats,
}

impl OrbitalHavenState {
    pub fn new(seed: u64) -> Self {
        let rng = WorldRng::new(seed);
        // One initial incoming transport shuttle
        let shuttles = vec![Shuttle {
            id: 1,
            pos: [180.0, 520.0],
            vel: [22.0, -14.0],
            target_dock: [500.0, 360.0],
            state: ShuttleState::Approaching,
            docking_timer: 0.0,
        }];

        Self {
            version: ORBITAL_HAVEN_STATE_VERSION,
            seed,
            rng,
            total_age_secs: 0.0,
            orbit_angle: 0.0,
            ring_rotation: 0.0,
            focus: ConstructionFocus::SolarPower,
            construction_progress: 0.25,
            solar_wings: 2,     // starts with 2 basic wings
            habitat_sectors: 3, // starts with 3 initial habitat sections
            comm_arrays: 1,
            shuttles,
            next_id: 10,
            stats: OrbitalHavenStats {
                station_age_hours: 0.0,
                total_modules: 6,
                solar_wings: 2,
                habitat_sectors: 3,
                comm_arrays: 1,
                shuttles_serviced: 0,
            },
        }
    }

    /// User interaction: set construction focus.
    pub fn set_focus(&mut self, focus: ConstructionFocus) {
        self.focus = focus;
    }

    /// User interaction: manually launch an auxiliary shuttle.
    pub fn launch_shuttle(&mut self) {
        self.next_id += 1;
        self.shuttles.push(Shuttle {
            id: self.next_id,
            pos: [500.0, 360.0],
            vel: [self.rng.range(-35.0, 35.0), self.rng.range(-45.0, -15.0)],
            target_dock: [500.0, 360.0],
            state: ShuttleState::Departing,
            docking_timer: 0.0,
        });
    }

    /// Offline time-delta engine for orbital station progression.
    /// Simulates construction progress and station aging during sleep.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let capped = elapsed_secs.min(86400.0 * 7.0); // up to 7 days
        self.total_age_secs += capped;
        self.stats.station_age_hours = self.total_age_secs / 3600.0;

        // Construction rate: ~1 module every 2 hours of real time
        let modules_earned = (capped / 7200.0) as u32;
        for _ in 0..modules_earned {
            self.complete_module();
        }
        self.construction_progress =
            (self.construction_progress + (capped % 7200.0) as f32 / 7200.0) % 1.0;
    }

    fn complete_module(&mut self) {
        match self.focus {
            ConstructionFocus::SolarPower => {
                if self.solar_wings < 8 {
                    self.solar_wings += 1;
                } else if self.habitat_sectors < 12 {
                    self.habitat_sectors += 1;
                }
            }
            ConstructionFocus::HabitatRing => {
                if self.habitat_sectors < 12 {
                    self.habitat_sectors += 1;
                } else if self.solar_wings < 8 {
                    self.solar_wings += 1;
                }
            }
            ConstructionFocus::CommsGrid => {
                if self.comm_arrays < 6 {
                    self.comm_arrays += 1;
                } else if self.habitat_sectors < 12 {
                    self.habitat_sectors += 1;
                }
            }
        }
        self.stats.solar_wings = self.solar_wings;
        self.stats.habitat_sectors = self.habitat_sectors;
        self.stats.comm_arrays = self.comm_arrays;
        self.stats.total_modules = self.solar_wings + self.habitat_sectors + self.comm_arrays;
    }

    pub fn update(
        &mut self,
        dt_secs: f32,
        _interactive: bool,
        _pointer: Option<[f32; 2]>,
    ) -> Activity {
        if dt_secs <= 0.0 {
            return Activity::Idle;
        }

        self.total_age_secs += dt_secs as f64;
        self.stats.station_age_hours = self.total_age_secs / 3600.0;

        // Orbital mechanics
        self.orbit_angle = (self.orbit_angle + dt_secs * 0.015) % TAU;
        self.ring_rotation = (self.ring_rotation + dt_secs * 0.08) % TAU;

        // Slow-burn construction tick (advances in background)
        self.construction_progress += dt_secs * (1.0 / 1800.0); // ~30 mins per module
        if self.construction_progress >= 1.0 {
            self.construction_progress = 0.0;
            self.complete_module();
        }

        // Update shuttles
        let dock_pos = [500.0f32, 360.0f32];
        for shuttle in &mut self.shuttles {
            match shuttle.state {
                ShuttleState::Approaching => {
                    let dx = dock_pos[0] - shuttle.pos[0];
                    let dy = dock_pos[1] - shuttle.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 8.0 {
                        shuttle.pos = dock_pos;
                        shuttle.state = ShuttleState::Docked;
                        shuttle.docking_timer = 8.0;
                    } else {
                        let speed = 28.0f32;
                        shuttle.pos[0] += (dx / dist) * speed * dt_secs;
                        shuttle.pos[1] += (dy / dist) * speed * dt_secs;
                        shuttle.vel = [(dx / dist) * speed, (dy / dist) * speed];
                    }
                }
                ShuttleState::Docked => {
                    shuttle.docking_timer -= dt_secs;
                    shuttle.vel = [0.0, 0.0];
                    if shuttle.docking_timer <= 0.0 {
                        shuttle.state = ShuttleState::Departing;
                        shuttle.vel = [24.0, 16.0];
                    }
                }
                ShuttleState::Departing => {
                    shuttle.pos[0] += shuttle.vel[0] * dt_secs;
                    shuttle.pos[1] += shuttle.vel[1] * dt_secs;
                }
            }
        }

        // Respawn new incoming shuttle if none are active
        self.shuttles.retain(|s| {
            s.pos[0] > -100.0 && s.pos[0] < 1100.0 && s.pos[1] > -100.0 && s.pos[1] < 850.0
        });
        if self.shuttles.is_empty() {
            self.next_id += 1;
            let start_x = if self.rng.f32() > 0.5 { 120.0 } else { 880.0 };
            let start_y = self.rng.range(500.0, 680.0);
            self.shuttles.push(Shuttle {
                id: self.next_id,
                pos: [start_x, start_y],
                vel: [15.0, -10.0],
                target_dock: dock_pos,
                state: ShuttleState::Approaching,
                docking_timer: 0.0,
            });
            self.stats.shuttles_serviced += 1;
        }

        Activity::Animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbital_haven_initialization() {
        let station = OrbitalHavenState::new(42);
        assert_eq!(station.solar_wings, 2);
        assert_eq!(station.habitat_sectors, 3);
        assert_eq!(station.shuttles.len(), 1);
    }

    #[test]
    fn orbital_haven_shuttle_launch() {
        let mut station = OrbitalHavenState::new(42);
        station.launch_shuttle();
        assert_eq!(station.shuttles.len(), 2);
    }

    #[test]
    fn orbital_haven_offline_progression() {
        let mut station = OrbitalHavenState::new(42);
        station.on_wake(14400.0); // 4 hours
        assert!(station.stats.total_modules > 6);
    }
}
