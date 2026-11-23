//! Orbital Haven v2.0 Simulation State (Slow-Burn Sci-Fi Megastructure & Ecosystem).
//!
//! A procedural deep-space orbital citadel assembling an axial spine, counter-rotating
//! centrifugal habitat rings, tracking solar wings, thermal radiators, geodesic biodomes,
//! and orbital drydocks over real time. Serviced by 4 classes of autonomous robotics
//! and a 6-class spacecraft fleet against a 3D planetary horizon.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::world::WorldRng;

pub mod persistence;
pub mod simulation;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::*;

/// Deterministic state of Orbital Haven v2.0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub total_age_secs: f64,
    #[serde(default)]
    pub total_offline_caught_up_secs: f64,
    pub orbit_angle: f32,
    pub ring_rotation: f32,
    #[serde(default)]
    pub outer_ring_rotation: f32,
    #[serde(default)]
    pub planet_biome: PlanetBiome,
    #[serde(default)]
    pub epoch: StationEpoch,
    pub focus: ConstructionFocus,
    /// Progress `[0.0, 1.0]` of the currently assembling module.
    pub construction_progress: f32,
    pub solar_wings: u32,
    pub habitat_sectors: u32,
    #[serde(default)]
    pub outer_ring_sectors: u32,
    pub comm_arrays: u32,
    #[serde(default)]
    pub biodomes: u32,
    #[serde(default)]
    pub drydocks: u32,
    #[serde(default)]
    pub radiators: u32,
    #[serde(default)]
    pub spine_segments: u32,
    #[serde(default)]
    pub modules: Vec<StationModule>,
    #[serde(default)]
    pub robots: Vec<RobotAgent>,
    pub shuttles: Vec<Shuttle>,
    #[serde(default)]
    pub asteroids: Vec<OrbitalAsteroid>,
    #[serde(default)]
    pub sparks: Vec<SparkParticle>,
    #[serde(default)]
    pub scan_pulse: ScanPulse,
    #[serde(default = "default_true")]
    pub hud_enabled: bool,
    #[serde(default)]
    pub camera_offset: [f32; 2],
    pub next_id: u32,
    pub stats: OrbitalHavenStats,
}

fn default_true() -> bool {
    true
}

/// Station center in normalized `[0..1000, 0..750]` world coordinates.
pub const STATION_CENTER: [f32; 2] = [500.0, 355.0];

impl OrbitalHavenState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut next_id = 1u32;

        // Build the full blueprint node graph (66 total modular slots across the megastructure).
        let mut modules = Vec::with_capacity(68);
        let mut push_mod = |kind: ModuleKind, slot: u32, pos: [f32; 2], built: bool| {
            let id = next_id;
            next_id += 1;
            modules.push(StationModule {
                id,
                kind,
                slot_index: slot,
                anchor_pos: pos,
                build_progress: if built { 1.0 } else { 0.0 },
                tier: 1,
            });
        };

        // 1. Axial Spine Segments (8 segments: 4 upper, 4 lower around central core)
        for i in 0..8u32 {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let step = (i / 2 + 1) as f32;
            let y = STATION_CENTER[1] + side * step * 34.0;
            // Start with 2 inner spine segments built
            push_mod(ModuleKind::SpineTruss, i, [STATION_CENTER[0], y], i < 2);
        }

        // 2. Inner Centrifugal Habitat Ring (12 sectors)
        for i in 0..12u32 {
            let angle = (i as f32 / 12.0) * TAU;
            let x = STATION_CENTER[0] + angle.cos() * 94.0;
            let y = STATION_CENTER[1] + angle.sin() * 94.0 * 0.43;
            // Start with 4 initial sectors spaced evenly around the wheel
            push_mod(ModuleKind::HabitatRingSector, i, [x, y], i % 3 == 0);
        }

        // 3. Solar Collector Array Wings (12 wings: 6 port, 6 starboard along main truss)
        for i in 0..12u32 {
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let col = (i / 2) as f32;
            let x = STATION_CENTER[0] + side * (118.0 + col * 38.0);
            let y = STATION_CENTER[1];
            // Start with 4 solar wings built
            push_mod(ModuleKind::SolarArrayWing, i, [x, y], i < 4);
        }

        // 4. Thermal Dissipation Radiators (6 arrays mounted on upper/lower spine)
        for i in 0..6u32 {
            let side_x = if i % 2 == 0 { 1.0 } else { -1.0 };
            let row = (i / 2) as f32;
            let x = STATION_CENTER[0] + side_x * 46.0;
            let y = STATION_CENTER[1]
                + (if row == 0.0 {
                    -52.0
                } else if row == 1.0 {
                    52.0
                } else {
                    -86.0
                });
            push_mod(ModuleKind::ThermalRadiator, i, [x, y], i < 1);
        }

        // 5. Subspace Comms & Gravimetric Sensor Arrays (6 spires at dorsal/ventral crown)
        for i in 0..6u32 {
            let spread = (i as f32 - 2.5) * 22.0;
            let x = STATION_CENTER[0] + spread;
            let y = STATION_CENTER[1] - 142.0 - (i % 2) as f32 * 16.0;
            push_mod(ModuleKind::CommsSensorArray, i, [x, y], i < 1);
        }

        // 6. Geodesic Hydroponic Biodomes (4 domes on diagonal trusses)
        let dome_offsets = [
            [-115.0, -56.0],
            [115.0, -56.0],
            [-115.0, 56.0],
            [115.0, 56.0],
        ];
        for (i, off) in dome_offsets.iter().enumerate() {
            push_mod(
                ModuleKind::HydroponicBiodome,
                i as u32,
                [STATION_CENTER[0] + off[0], STATION_CENTER[1] + off[1]],
                i == 0,
            );
        }

        // 7. Orbital Drydock Slipways (2 heavy gantries below/above outer truss)
        let dock_offsets = [[-175.0, 48.0], [175.0, 48.0]];
        for (i, off) in dock_offsets.iter().enumerate() {
            push_mod(
                ModuleKind::OrbitalDrydock,
                i as u32,
                [STATION_CENTER[0] + off[0], STATION_CENTER[1] + off[1]],
                false,
            );
        }

        // 8. Outer Counter-Rotating Habitat Ring (16 sectors)
        for i in 0..16u32 {
            let angle = (i as f32 / 16.0) * TAU;
            let x = STATION_CENTER[0] + angle.cos() * 148.0;
            let y = STATION_CENTER[1] + angle.sin() * 148.0 * 0.43;
            push_mod(ModuleKind::OuterRingSector, i, [x, y], false);
        }

        // Seed initial construction progress on the first unbuilt SolarArrayWing
        if let Some(m) = modules
            .iter_mut()
            .find(|m| !m.is_complete() && m.kind == ModuleKind::SolarArrayWing)
        {
            m.build_progress = 0.35;
        }

        // Spawn initial autonomous robot crew (Welders, Spine Crawlers, Cargo Tugs, Micro-Sats)
        let mut robots = Vec::new();
        for i in 0..3 {
            robots.push(RobotAgent {
                id: next_id,
                kind: RobotKind::WelderDrone,
                pos: [
                    STATION_CENTER[0] + rng.range(-80.0, 80.0),
                    STATION_CENTER[1] + rng.range(-40.0, 40.0),
                ],
                vel: [0.0, 0.0],
                task: RobotTask::Welding {
                    module_id: 5,
                    orbit_phase: i as f32 * 2.1,
                },
                phase: i as f32 * 1.7,
                boost_timer: 0.0,
            });
            next_id += 1;
        }
        for i in 0..2 {
            robots.push(RobotAgent {
                id: next_id,
                kind: RobotKind::SpiderCrawler,
                pos: [
                    STATION_CENTER[0],
                    STATION_CENTER[1] + (i as f32 - 0.5) * 60.0,
                ],
                vel: [0.0, 0.0],
                task: RobotTask::CrawlingSpine {
                    t: if i == 0 { -0.4 } else { 0.4 },
                    dir: if i == 0 { 1.0 } else { -1.0 },
                },
                phase: i as f32 * std::f32::consts::PI,
                boost_timer: 0.0,
            });
            next_id += 1;
        }
        robots.push(RobotAgent {
            id: next_id,
            kind: RobotKind::CargoTug,
            pos: [STATION_CENTER[0] - 40.0, STATION_CENTER[1] + 20.0],
            vel: [0.0, 0.0],
            task: RobotTask::HaulingCargo {
                from: STATION_CENTER,
                to: [STATION_CENTER[0] + 140.0, STATION_CENTER[1]],
                progress: 0.2,
            },
            phase: 0.0,
            boost_timer: 0.0,
        });
        next_id += 1;

        for i in 0..2 {
            robots.push(RobotAgent {
                id: next_id,
                kind: RobotKind::DefenseMicroSat,
                pos: [STATION_CENTER[0] + 220.0, STATION_CENTER[1]],
                vel: [0.0, 0.0],
                task: RobotTask::PerimeterOrbit {
                    angle: i as f32 * std::f32::consts::PI,
                    radius_x: 245.0 + i as f32 * 30.0,
                    radius_y: 92.0 + i as f32 * 14.0,
                    speed: if i % 2 == 0 { 0.22 } else { -0.18 },
                },
                phase: i as f32 * 1.5,
                boost_timer: 0.0,
            });
            next_id += 1;
        }

        // Spawn initial orbital asteroids in the upper/lateral resource belt
        let mut asteroids = Vec::new();
        let ast_coords = [
            ([165.0, 125.0], [6.5, 0.8], 18.0, 1u8),
            ([825.0, 145.0], [-5.5, -0.6], 22.0, 2u8),
            ([310.0, 88.0], [4.2, 0.3], 14.0, 0u8),
            ([710.0, 95.0], [-4.8, 0.5], 16.0, 1u8),
        ];
        for (pos, vel, radius, richness) in ast_coords {
            asteroids.push(OrbitalAsteroid {
                id: next_id,
                pos,
                vel,
                radius,
                rotation: rng.range(0.0, TAU),
                rot_speed: rng.range(-0.35, 0.35),
                ore_remaining: 100.0,
                max_ore: 100.0,
                richness,
            });
            next_id += 1;
        }
        let first_asteroid_id = asteroids[0].id;

        // Spawn initial spacecraft fleet (1 approaching BulkFreighter, 1 active MiningSkiff, 1 PatrolInterceptor)
        let shuttles = vec![
            Shuttle {
                id: next_id,
                class: ShipClass::BulkFreighter,
                callsign: "CSV-TITAN".into(),
                pos: [140.0, 510.0],
                vel: [24.0, -11.0],
                target_dock: [STATION_CENTER[0] - 24.0, STATION_CENTER[1] + 42.0],
                state: ShuttleState::Approaching,
                docking_timer: 0.0,
                cargo_tons: 45.0,
            },
            Shuttle {
                id: next_id + 1,
                class: ShipClass::MiningSkiff,
                callsign: "RIG-04".into(),
                pos: [220.0, 185.0],
                vel: [-12.0, -14.0],
                target_dock: [STATION_CENTER[0] + 24.0, STATION_CENTER[1] - 42.0],
                state: ShuttleState::MiningAsteroid {
                    asteroid_id: first_asteroid_id,
                    timer: 9.0,
                },
                docking_timer: 0.0,
                cargo_tons: 15.0,
            },
            Shuttle {
                id: next_id + 2,
                class: ShipClass::PatrolInterceptor,
                callsign: "VIPER-1".into(),
                pos: [STATION_CENTER[0] + 190.0, STATION_CENTER[1] - 40.0],
                vel: [0.0, 20.0],
                target_dock: STATION_CENTER,
                state: ShuttleState::Patrolling {
                    angle: 0.4,
                    radius: 210.0,
                    timer: 25.0,
                },
                docking_timer: 0.0,
                cargo_tons: 0.0,
            },
        ];
        next_id += 10;

        let mut state = Self {
            version: ORBITAL_HAVEN_STATE_VERSION,
            seed,
            rng,
            total_age_secs: 0.0,
            total_offline_caught_up_secs: 0.0,
            orbit_angle: 0.35,
            ring_rotation: 0.0,
            outer_ring_rotation: 0.0,
            planet_biome: PlanetBiome::TerranCradle,
            epoch: StationEpoch::TetheredCore,
            focus: ConstructionFocus::AutoBalance,
            construction_progress: 0.35,
            solar_wings: 4,
            habitat_sectors: 4,
            outer_ring_sectors: 0,
            comm_arrays: 1,
            biodomes: 1,
            drydocks: 0,
            radiators: 1,
            spine_segments: 2,
            modules,
            robots,
            shuttles,
            asteroids,
            sparks: Vec::new(),
            scan_pulse: ScanPulse::default(),
            hud_enabled: true,
            camera_offset: [0.0, 0.0],
            next_id,
            stats: OrbitalHavenStats {
                alloy_stockpile: 120.0,
                science_telemetry: 45.0,
                ..Default::default()
            },
        };
        state.recompute_telemetry();
        state
    }
}
