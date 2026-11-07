//! Orbital Haven v2.0 Simulation State (Slow-Burn Sci-Fi Megastructure & Ecosystem).
//!
//! A procedural deep-space orbital citadel assembling an axial spine, counter-rotating
//! centrifugal habitat rings, tracking solar wings, thermal radiators, geodesic biodomes,
//! and orbital drydocks over real time. Serviced by 4 classes of autonomous robotics
//! and a 6-class spacecraft fleet against a 3D planetary horizon.

use std::f32::consts::TAU;
use std::fs;
use std::path::Path;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::Activity;
use crate::world::WorldRng;

pub const ORBITAL_HAVEN_STATE_VERSION: u32 = 2;
pub const ORBITAL_STATE_FILE_NAME: &str = "orbital_haven.json";

/// Planetary biome rendered beneath the orbital station.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanetBiome {
    /// Lush blue-green Earth-like cradle with golden coastal megacities and cyan auroras.
    #[default]
    TerranCradle,
    /// Fractured glacial ice moon with bioluminescent subsurface ocean rifts and sapphire limb.
    OceanicEuropa,
    /// Rust-red terraforming forge world with canyon dust storms and amber dome cities.
    AresForge,
    /// Dark metallic ecumenopolis wrapped in geometric neon hex-grid arcologies and violet haze.
    CyberEcumenopolis,
}

impl PlanetBiome {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TerranCradle => "Terran Cradle",
            Self::OceanicEuropa => "Oceanic Europa",
            Self::AresForge => "Ares Forge",
            Self::CyberEcumenopolis => "Cyber-Ecumenopolis",
        }
    }

    pub fn shader_index(&self) -> f32 {
        match self {
            Self::TerranCradle => 0.0,
            Self::OceanicEuropa => 1.0,
            Self::AresForge => 2.0,
            Self::CyberEcumenopolis => 3.0,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::TerranCradle => Self::OceanicEuropa,
            Self::OceanicEuropa => Self::AresForge,
            Self::AresForge => Self::CyberEcumenopolis,
            Self::CyberEcumenopolis => Self::TerranCradle,
        }
    }
}

/// Evolutionary tier of the orbital megastructure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationEpoch {
    #[default]
    PioneerOutpost,
    TetheredCore,
    CentrifugalHaven,
    DeepSpaceShipyard,
    ONeillCitadel,
}

impl StationEpoch {
    pub fn from_modules(completed_modules: u32) -> Self {
        match completed_modules {
            0..=11 => Self::PioneerOutpost,
            12..=22 => Self::TetheredCore,
            23..=36 => Self::CentrifugalHaven,
            37..=50 => Self::DeepSpaceShipyard,
            _ => Self::ONeillCitadel,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::PioneerOutpost => "Tier I · Pioneer Outpost",
            Self::TetheredCore => "Tier II · Tethered Core",
            Self::CentrifugalHaven => "Tier III · Centrifugal Haven",
            Self::DeepSpaceShipyard => "Tier IV · Deep-Space Shipyard",
            Self::ONeillCitadel => "Tier V · O'Neill Dyson Citadel",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::PioneerOutpost => "Outpost",
            Self::TetheredCore => "Core Hub",
            Self::CentrifugalHaven => "Haven Ring",
            Self::DeepSpaceShipyard => "Shipyard",
            Self::ONeillCitadel => "Citadel",
        }
    }

    pub fn tier_number(&self) -> u32 {
        match self {
            Self::PioneerOutpost => 1,
            Self::TetheredCore => 2,
            Self::CentrifugalHaven => 3,
            Self::DeepSpaceShipyard => 4,
            Self::ONeillCitadel => 5,
        }
    }
}

/// Modular construction directive for station expansion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionFocus {
    /// Automatically prioritize whichever subsystem is currently bottlenecked.
    #[default]
    AutoBalance,
    /// Expand solar collector wings and thermal dissipation radiators.
    SolarPower,
    /// Assemble inner and outer counter-rotating centrifugal habitat rings.
    HabitatRing,
    /// Construct geodesic hydroponic biodomes and orbital drydock slipways.
    ShipyardBiodome,
    /// Build subspace sensor arrays, deep-space dishes, and defense relays.
    CommsGrid,
}

impl ConstructionFocus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::AutoBalance => "Auto-Balance AI",
            Self::SolarPower => "Solar & Radiators",
            Self::HabitatRing => "Centrifugal Rings",
            Self::ShipyardBiodome => "Biodomes & Drydocks",
            Self::CommsGrid => "Subspace Comms",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::AutoBalance => Self::SolarPower,
            Self::SolarPower => Self::HabitatRing,
            Self::HabitatRing => Self::ShipyardBiodome,
            Self::ShipyardBiodome => Self::CommsGrid,
            Self::CommsGrid => Self::AutoBalance,
        }
    }
}

/// Architectural category of a station module in the megastructure graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    /// Heavy axial truss backbone segment along the vertical station spine.
    SpineTruss,
    /// Primary rotating centrifugal habitat ring sector (12 segments).
    HabitatRingSector,
    /// Counter-rotating outer gravity ring sector (16 segments).
    OuterRingSector,
    /// Articulated photovoltaic solar collector array wing (up to 12 wings).
    SolarArrayWing,
    /// Crimson/amber thermal radiator fin array extending from the upper/lower spine.
    ThermalRadiator,
    /// Geodesic glass agricultural biodome growing oxygenating flora.
    HydroponicBiodome,
    /// Orbital drydock slipway with gantry cranes for vessel servicing.
    OrbitalDrydock,
    /// Deep-space dish and gravimetric sensor spire.
    CommsSensorArray,
}

/// A structural node in the Orbital Haven megastructure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationModule {
    pub id: u32,
    pub kind: ModuleKind,
    pub slot_index: u32,
    /// World-space anchor coordinate (for non-rotating modules) or relative index.
    pub anchor_pos: [f32; 2],
    /// `[0.0, 1.0]` completion (`1.0` = operational, `< 1.0` = holographic wireframe under construction).
    pub build_progress: f32,
    /// Structural upgrade level (`1..=3`).
    pub tier: u8,
}

impl StationModule {
    pub fn is_complete(&self) -> bool {
        self.build_progress >= 0.999
    }
}

/// Autonomous robot class operating on and around the station.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RobotKind {
    /// Free-flying plasma welder drone that orbits unfinished modules and emits sparks.
    WelderDrone,
    /// Articulated hexapod spider bot crawling along the axial spine and solar trusses.
    SpiderCrawler,
    /// Heavy utility tug ferrying alloy canisters between docking bays and build sites.
    CargoTug,
    /// Autonomous perimeter defense & telemetry micro-satellite.
    DefenseMicroSat,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RobotTask {
    /// Orbiting and welding a specific station module.
    Welding { module_id: u32, orbit_phase: f32 },
    /// Crawling along the vertical spine (`t` in `[-1.0, 1.0]`).
    CrawlingSpine { t: f32, dir: f32 },
    /// Hauling cargo between two world points.
    HaulingCargo {
        from: [f32; 2],
        to: [f32; 2],
        progress: f32,
    },
    /// Patrolling an elliptical perimeter orbit around the station.
    PerimeterOrbit {
        angle: f32,
        radius_x: f32,
        radius_y: f32,
        speed: f32,
    },
}

/// An autonomous robotic agent in the station ecosystem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RobotAgent {
    pub id: u32,
    pub kind: RobotKind,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub task: RobotTask,
    pub phase: f32,
    pub boost_timer: f32,
}

/// Distinct spacecraft hull classes visiting and operating from Orbital Haven.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipClass {
    /// Long-spine heavy cargo freighter delivering refined titanium & silicon ore.
    #[default]
    BulkFreighter,
    /// Sleek centrifugal passenger liner ferrying crew & colonists.
    HabLiner,
    /// Industrial asteroid mining skiff equipped with an extraction laser.
    MiningSkiff,
    /// High-tech deep-space science & survey corvette.
    ScienceCorvette,
    /// Graceful photonic solar-sail clipper yacht.
    SolarClipper,
    /// Fast delta-wing orbital defense interceptor.
    PatrolInterceptor,
}

impl ShipClass {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BulkFreighter => "Heavy Bulk Freighter",
            Self::HabLiner => "Centrifugal Hab-Liner",
            Self::MiningSkiff => "Asteroid Mining Skiff",
            Self::ScienceCorvette => "Science Survey Corvette",
            Self::SolarClipper => "Solar-Sail Clipper",
            Self::PatrolInterceptor => "Aegis Patrol Interceptor",
        }
    }
}

/// Operational state of a spacecraft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShuttleState {
    Approaching,
    Docked,
    MiningAsteroid { asteroid_id: u32, timer: f32 },
    Patrolling { angle: f32, radius: f32, timer: f32 },
    Departing,
}

/// A spacecraft operating in the Orbital Haven sector.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shuttle {
    pub id: u32,
    #[serde(default)]
    pub class: ShipClass,
    #[serde(default)]
    pub callsign: String,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub target_dock: [f32; 2],
    pub state: ShuttleState,
    pub docking_timer: f32,
    #[serde(default)]
    pub cargo_tons: f32,
}

/// A drifting resource-rich asteroid in the upper orbital belt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitalAsteroid {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub radius: f32,
    pub rotation: f32,
    pub rot_speed: f32,
    pub ore_remaining: f32,
    pub max_ore: f32,
    /// `0` = silicate/iron, `1` = titanium-rich, `2` = crystalline helium-3 / gold.
    pub richness: u8,
}

/// Transient plasma welding spark, mining laser flare, or thruster ember.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparkParticle {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub life: f32,
    pub max_life: f32,
    /// `0` = golden welding spark, `1` = cyan plasma/laser, `2` = amber ore fragment.
    pub color_kind: u8,
}

/// Expanding radial subspace sensor scan wave.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScanPulse {
    pub center: [f32; 2],
    pub progress: f32,
    pub active: bool,
}

/// Telemetry and economy metrics for Orbital Haven.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenStats {
    pub station_age_hours: f64,
    pub total_modules: u32,
    pub solar_wings: u32,
    pub habitat_sectors: u32,
    pub outer_ring_sectors: u32,
    pub comm_arrays: u32,
    pub biodomes: u32,
    pub drydocks: u32,
    pub radiators: u32,
    pub spine_segments: u32,
    pub shuttles_serviced: u32,
    pub asteroids_mined: u32,
    pub power_mw: u32,
    pub power_demand_mw: u32,
    pub crew_population: u32,
    pub crew_capacity: u32,
    pub alloy_stockpile: f32,
    pub science_telemetry: f32,
}

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

    /// Ensure deserialized v1 or partial states have a populated megastructure graph and robotics crew.
    pub fn ensure_migrated(&mut self) {
        if self.modules.is_empty() {
            let fresh = Self::new(self.seed);
            self.modules = fresh.modules;
            self.robots = fresh.robots;
            self.asteroids = fresh.asteroids;
            self.version = ORBITAL_HAVEN_STATE_VERSION;
        }
        self.recompute_telemetry();
    }

    /// Load state from disk or initialize a fresh station with `default_seed`.
    pub fn load_or_init(path: &Path, default_seed: u64) -> Self {
        if let Ok(bytes) = fs::read(path)
            && let Ok(mut state) = serde_json::from_slice::<Self>(&bytes)
        {
            state.ensure_migrated();
            return state;
        }
        Self::new(default_seed)
    }

    /// Atomically persist Orbital Haven state to disk.
    pub fn save_atomic(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Compact shareable seed URI (`rookery://orbital/seed-<hex>`).
    pub fn seed_uri(&self) -> String {
        format!("rookery://orbital/seed-{:016x}", self.seed)
    }

    /// Full base64-encoded state URI (`rookery://orbital/state-<base64>`).
    pub fn state_uri(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        Ok(format!("rookery://orbital/state-{encoded}"))
    }

    /// Parse a `rookery://orbital/...` seed or state string.
    pub fn from_share_string(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if let Some(rest) = trimmed.strip_prefix("rookery://orbital/state-") {
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(rest.trim())
                .map_err(|e| format!("Invalid base64 orbital state: {e}"))?;
            let mut state: Self =
                serde_json::from_slice(&bytes).map_err(|e| format!("Invalid orbital JSON: {e}"))?;
            state.ensure_migrated();
            return Ok(state);
        }

        let seed_hex = if let Some(rest) = trimmed.strip_prefix("rookery://orbital/seed-") {
            Some(rest.trim())
        } else {
            trimmed.strip_prefix("orbital-seed-").map(|s| s.trim())
        };

        if let Some(hex_str) = seed_hex {
            let seed = u64::from_str_radix(hex_str, 16)
                .map_err(|e| format!("Invalid hex seed '{hex_str}': {e}"))?;
            return Ok(Self::new(seed));
        }

        Err("Unrecognized orbital share string. Expected 'rookery://orbital/seed-...' or 'rookery://orbital/state-...'".into())
    }

    /// Recompute aggregate counts, power balance, crew population, and station epoch.
    pub fn recompute_telemetry(&mut self) {
        let mut spine = 0u32;
        let mut inner_hab = 0u32;
        let mut outer_hab = 0u32;
        let mut solar = 0u32;
        let mut rads = 0u32;
        let mut comms = 0u32;
        let mut domes = 0u32;
        let mut docks = 0u32;

        for m in &self.modules {
            if m.is_complete() {
                match m.kind {
                    ModuleKind::SpineTruss => spine += 1,
                    ModuleKind::HabitatRingSector => inner_hab += 1,
                    ModuleKind::OuterRingSector => outer_hab += 1,
                    ModuleKind::SolarArrayWing => solar += 1,
                    ModuleKind::ThermalRadiator => rads += 1,
                    ModuleKind::CommsSensorArray => comms += 1,
                    ModuleKind::HydroponicBiodome => domes += 1,
                    ModuleKind::OrbitalDrydock => docks += 1,
                }
            }
        }

        self.spine_segments = spine;
        self.habitat_sectors = inner_hab;
        self.outer_ring_sectors = outer_hab;
        self.solar_wings = solar;
        self.radiators = rads;
        self.comm_arrays = comms;
        self.biodomes = domes;
        self.drydocks = docks;

        let total_completed = spine + inner_hab + outer_hab + solar + rads + comms + domes + docks;
        self.epoch = StationEpoch::from_modules(total_completed);

        // Power generation & demand
        let power_gen = 25 + solar * 38 + rads * 18;
        let power_demand =
            15 + inner_hab * 8 + outer_hab * 10 + domes * 14 + docks * 22 + comms * 9;
        let crew_cap = 12 + inner_hab * 18 + outer_hab * 24 + domes * 30;
        let crew_pop =
            ((crew_cap as f32) * (0.78 + 0.15 * (domes as f32 / 4.0).min(1.0))).round() as u32;

        self.stats.station_age_hours = self.total_age_secs / 3600.0;
        self.stats.total_modules = total_completed;
        self.stats.solar_wings = solar;
        self.stats.habitat_sectors = inner_hab + outer_hab;
        self.stats.outer_ring_sectors = outer_hab;
        self.stats.comm_arrays = comms;
        self.stats.biodomes = domes;
        self.stats.drydocks = docks;
        self.stats.radiators = rads;
        self.stats.spine_segments = spine;
        self.stats.power_mw = power_gen;
        self.stats.power_demand_mw = power_demand;
        self.stats.crew_capacity = crew_cap;
        self.stats.crew_population = crew_pop;
    }

    /// Find the index of the currently assembling module (or pick the next blueprint based on `self.focus`).
    fn active_construction_module_index(&mut self) -> Option<usize> {
        // If there is already a partially constructed module matching or in progress, continue it
        if let Some(idx) = self
            .modules
            .iter()
            .position(|m| m.build_progress > 0.001 && !m.is_complete())
        {
            return Some(idx);
        }

        let desired_kinds: &[ModuleKind] = match self.focus {
            ConstructionFocus::SolarPower => &[
                ModuleKind::SolarArrayWing,
                ModuleKind::ThermalRadiator,
                ModuleKind::SpineTruss,
            ],
            ConstructionFocus::HabitatRing => &[
                ModuleKind::HabitatRingSector,
                ModuleKind::HydroponicBiodome,
                ModuleKind::OuterRingSector,
                ModuleKind::SpineTruss,
            ],
            ConstructionFocus::ShipyardBiodome => &[
                ModuleKind::HydroponicBiodome,
                ModuleKind::OrbitalDrydock,
                ModuleKind::SpineTruss,
                ModuleKind::OuterRingSector,
            ],
            ConstructionFocus::CommsGrid => &[
                ModuleKind::CommsSensorArray,
                ModuleKind::SpineTruss,
                ModuleKind::SolarArrayWing,
            ],
            ConstructionFocus::AutoBalance => {
                if self.stats.power_mw < self.stats.power_demand_mw + 30 {
                    &[ModuleKind::SolarArrayWing, ModuleKind::ThermalRadiator]
                } else {
                    &[
                        ModuleKind::HabitatRingSector,
                        ModuleKind::SpineTruss,
                        ModuleKind::HydroponicBiodome,
                        ModuleKind::CommsSensorArray,
                        ModuleKind::OrbitalDrydock,
                        ModuleKind::OuterRingSector,
                    ]
                }
            }
        };

        for &k in desired_kinds {
            if let Some(idx) = self
                .modules
                .iter()
                .position(|m| !m.is_complete() && m.kind == k)
            {
                return Some(idx);
            }
        }

        self.modules.iter().position(|m| !m.is_complete())
    }

    /// Advance construction by `delta_progress` (`1.0` = one full module).
    pub fn advance_construction(&mut self, mut delta_progress: f32) {
        while delta_progress > 0.0 {
            let Some(idx) = self.active_construction_module_index() else {
                // All 66 modules complete! Cycle upgrade tiers and construction progress bar
                self.construction_progress = (self.construction_progress + delta_progress) % 1.0;
                if let Some(m) = self.modules.iter_mut().find(|m| m.tier < 3) {
                    m.tier += 1;
                }
                break;
            };

            let needed = (1.0 - self.modules[idx].build_progress).max(0.001);
            if delta_progress >= needed {
                self.modules[idx].build_progress = 1.0;
                delta_progress -= needed;
                self.construction_progress = 0.0;
                let anchor = self.modules[idx].anchor_pos;
                self.spawn_sparks(anchor, 14, 0);
                self.recompute_telemetry();
                // Unlock additional robots as station grows
                self.maybe_unlock_robots();
            } else {
                self.modules[idx].build_progress += delta_progress;
                self.construction_progress = self.modules[idx].build_progress;
                delta_progress = 0.0;
            }
        }
        self.recompute_telemetry();
    }

    fn maybe_unlock_robots(&mut self) {
        let target_robots = (8 + (self.stats.total_modules / 6) as usize).min(18);
        while self.robots.len() < target_robots {
            self.next_id += 1;
            let r_idx = self.robots.len();
            let kind = match r_idx % 4 {
                0 => RobotKind::WelderDrone,
                1 => RobotKind::CargoTug,
                2 => RobotKind::SpiderCrawler,
                _ => RobotKind::DefenseMicroSat,
            };
            let task = match kind {
                RobotKind::WelderDrone => RobotTask::Welding {
                    module_id: 1,
                    orbit_phase: r_idx as f32 * 1.3,
                },
                RobotKind::SpiderCrawler => RobotTask::CrawlingSpine {
                    t: 0.0,
                    dir: if r_idx.is_multiple_of(2) { 1.0 } else { -1.0 },
                },
                RobotKind::CargoTug => RobotTask::HaulingCargo {
                    from: STATION_CENTER,
                    to: [STATION_CENTER[0] - 120.0, STATION_CENTER[1]],
                    progress: 0.0,
                },
                RobotKind::DefenseMicroSat => RobotTask::PerimeterOrbit {
                    angle: r_idx as f32 * 0.9,
                    radius_x: 235.0 + (r_idx as f32 * 11.0) % 60.0,
                    radius_y: 88.0 + (r_idx as f32 * 7.0) % 30.0,
                    speed: if r_idx.is_multiple_of(2) { 0.20 } else { -0.22 },
                },
            };
            self.robots.push(RobotAgent {
                id: self.next_id,
                kind,
                pos: STATION_CENTER,
                vel: [0.0, 0.0],
                task,
                phase: r_idx as f32 * 1.1,
                boost_timer: 0.0,
            });
        }
    }

    pub fn spawn_sparks(&mut self, center: [f32; 2], count: usize, color_kind: u8) {
        for _ in 0..count {
            if self.sparks.len() >= 160 {
                break;
            }
            let angle = self.rng.range(0.0, TAU);
            let speed = self.rng.range(12.0, 58.0);
            let max_life = self.rng.range(0.45, 1.25);
            self.sparks.push(SparkParticle {
                pos: center,
                vel: [angle.cos() * speed, angle.sin() * speed],
                life: 0.0,
                max_life,
                color_kind,
            });
        }
    }

    /// User interaction: set construction focus directive.
    pub fn set_focus(&mut self, focus: ConstructionFocus) {
        self.focus = focus;
    }

    /// User interaction: cycle to the next construction focus directive.
    pub fn cycle_focus(&mut self) -> ConstructionFocus {
        self.focus = self.focus.next();
        self.focus
    }

    /// User interaction: cycle planetary biome beneath the station.
    pub fn cycle_planet_biome(&mut self) -> PlanetBiome {
        self.planet_biome = self.planet_biome.next();
        self.planet_biome
    }

    /// User interaction: toggle diegetic on-screen telemetry overlays.
    pub fn toggle_hud(&mut self) -> bool {
        self.hud_enabled = !self.hud_enabled;
        self.hud_enabled
    }

    /// User interaction: pan orbital camera offset.
    pub fn pan_camera(&mut self, dx: f32, dy: f32) {
        self.camera_offset[0] = (self.camera_offset[0] + dx).clamp(-260.0, 260.0);
        self.camera_offset[1] = (self.camera_offset[1] + dy).clamp(-140.0, 140.0);
    }

    /// User interaction: overdrive welder drones & boost active construction by 22%.
    pub fn overdrive_drones(&mut self) {
        for r in &mut self.robots {
            r.boost_timer = 10.0;
        }
        if let Some(idx) = self.active_construction_module_index() {
            let pos = self.modules[idx].anchor_pos;
            self.spawn_sparks(pos, 18, 0);
        } else {
            self.spawn_sparks(STATION_CENTER, 18, 0);
        }
        self.advance_construction(0.22);
    }

    /// User interaction: trigger a radial subspace sensor scan wave & harvest nearby ore.
    pub fn trigger_scan_pulse(&mut self, center: [f32; 2]) {
        self.scan_pulse = ScanPulse {
            center,
            progress: 0.01,
            active: true,
        };
        self.stats.science_telemetry += 25.0;
        self.spawn_sparks(center, 12, 1);
    }

    /// User interaction: manually launch a spacecraft (cycles across all 6 ship classes).
    pub fn launch_shuttle(&mut self) {
        let classes = [
            ShipClass::BulkFreighter,
            ShipClass::MiningSkiff,
            ShipClass::HabLiner,
            ShipClass::ScienceCorvette,
            ShipClass::SolarClipper,
            ShipClass::PatrolInterceptor,
        ];
        let class = classes[(self.next_id as usize) % classes.len()];
        self.launch_ship_of_class(class, None);
    }

    pub fn launch_ship_of_class(&mut self, class: ShipClass, target_pos: Option<[f32; 2]>) {
        if self.shuttles.len() >= 14 {
            self.shuttles.remove(0);
        }
        self.next_id += 1;
        let callsign = match class {
            ShipClass::BulkFreighter => format!("CSV-{:02}", self.next_id % 99),
            ShipClass::HabLiner => format!("LUX-{:02}", self.next_id % 99),
            ShipClass::MiningSkiff => format!("RIG-{:02}", self.next_id % 99),
            ShipClass::ScienceCorvette => format!("SCI-{:02}", self.next_id % 99),
            ShipClass::SolarClipper => format!("SOL-{:02}", self.next_id % 99),
            ShipClass::PatrolInterceptor => format!("VPR-{:02}", self.next_id % 99),
        };

        let (pos, vel, state, dock) = match class {
            ShipClass::MiningSkiff if !self.asteroids.is_empty() => {
                let ast_idx = (self.next_id as usize) % self.asteroids.len();
                let ast_id = self.asteroids[ast_idx].id;
                (
                    STATION_CENTER,
                    [0.0, -30.0],
                    ShuttleState::MiningAsteroid {
                        asteroid_id: ast_id,
                        timer: 10.0,
                    },
                    STATION_CENTER,
                )
            }
            ShipClass::PatrolInterceptor => (
                STATION_CENTER,
                [35.0, -15.0],
                ShuttleState::Patrolling {
                    angle: self.rng.range(0.0, TAU),
                    radius: self.rng.range(185.0, 240.0),
                    timer: 22.0,
                },
                STATION_CENTER,
            ),
            _ => {
                if let Some(dest) = target_pos {
                    let dx = dest[0] - STATION_CENTER[0];
                    let dy = dest[1] - STATION_CENTER[1];
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    (
                        STATION_CENTER,
                        [(dx / dist) * 38.0, (dy / dist) * 38.0],
                        ShuttleState::Departing,
                        STATION_CENTER,
                    )
                } else {
                    let from_left = self.rng.bool_with_prob(0.5);
                    let start_x = if from_left { 60.0 } else { 940.0 };
                    let start_y = self.rng.range(180.0, 560.0);
                    let dock_offset_x = self.rng.range(-48.0, 48.0);
                    let dock_offset_y = self.rng.range(-38.0, 38.0);
                    let dock_pt = [
                        STATION_CENTER[0] + dock_offset_x,
                        STATION_CENTER[1] + dock_offset_y,
                    ];
                    (
                        [start_x, start_y],
                        [if from_left { 28.0 } else { -28.0 }, -10.0],
                        ShuttleState::Approaching,
                        dock_pt,
                    )
                }
            }
        };

        self.spawn_sparks(pos, 8, 1);
        self.shuttles.push(Shuttle {
            id: self.next_id,
            class,
            callsign,
            pos,
            vel,
            target_dock: dock,
            state,
            docking_timer: 0.0,
            cargo_tons: 30.0,
        });
    }

    /// Direct desktop click interaction handler:
    /// - Clicking an asteroid laser-harvests ore and spawns a mining skiff.
    /// - Clicking near the station overdrives welder drones & constructs modules.
    /// - Clicking in open space dispatches an interceptor/corvette toward the cursor.
    pub fn interact_at(&mut self, world_pt: [f32; 2]) {
        // 1. Check if clicking near an asteroid
        for ast in &mut self.asteroids {
            let dx = world_pt[0] - ast.pos[0];
            let dy = world_pt[1] - ast.pos[1];
            if (dx * dx + dy * dy).sqrt() <= ast.radius + 32.0 {
                ast.ore_remaining = (ast.ore_remaining - 35.0).max(0.0);
                let pos = ast.pos;
                self.stats.alloy_stockpile += 35.0;
                self.stats.asteroids_mined += 1;
                self.spawn_sparks(pos, 16, 2);
                self.advance_construction(0.12);
                return;
            }
        }

        // 2. Check if clicking near the station megastructure
        let dx_st = world_pt[0] - STATION_CENTER[0];
        let dy_st = world_pt[1] - STATION_CENTER[1];
        if (dx_st * dx_st + dy_st * dy_st).sqrt() <= 210.0 {
            self.overdrive_drones();
            self.spawn_sparks(world_pt, 12, 0);
            return;
        }

        // 3. Otherwise launch a vessel toward the clicked coordinate
        self.launch_ship_of_class(ShipClass::ScienceCorvette, Some(world_pt));
        self.spawn_sparks(world_pt, 10, 1);
    }

    /// Offline time-delta engine for orbital station progression.
    /// Simulates module construction, asteroid mining, and fleet servicing during sleep.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 {
            return;
        }
        self.ensure_migrated();
        let capped = elapsed_secs.min(86400.0 * 7.0); // up to 7 days
        self.total_age_secs += capped;
        self.total_offline_caught_up_secs += capped;
        self.stats.station_age_hours = self.total_age_secs / 3600.0;

        // Advance orbital angles deterministically
        self.orbit_angle = (self.orbit_angle + (capped as f32) * 0.015) % TAU;
        self.ring_rotation = (self.ring_rotation + (capped as f32) * 0.08) % TAU;
        self.outer_ring_rotation = (self.outer_ring_rotation - (capped as f32) * 0.055) % TAU;

        // Construction rate: ~1 module every 45 minutes of offline time (2700s)
        let delta_modules = (capped / 2700.0) as f32;
        let shuttles_visited = (capped / 420.0) as u32;
        self.stats.shuttles_serviced = self
            .stats
            .shuttles_serviced
            .saturating_add(shuttles_visited);
        self.stats.asteroids_mined = self
            .stats
            .asteroids_mined
            .saturating_add(shuttles_visited / 3);
        self.stats.alloy_stockpile += shuttles_visited as f32 * 8.0;
        self.stats.science_telemetry += shuttles_visited as f32 * 4.5;

        self.advance_construction(delta_modules);
    }

    /// Compute the current world position of a module (accounting for rotating rings).
    pub fn module_world_pos(&self, m: &StationModule) -> [f32; 2] {
        match m.kind {
            ModuleKind::HabitatRingSector => {
                let angle = self.ring_rotation + (m.slot_index as f32 / 12.0) * TAU;
                [
                    STATION_CENTER[0] + angle.cos() * 78.0,
                    STATION_CENTER[1] + angle.sin() * 78.0 * 0.42,
                ]
            }
            ModuleKind::OuterRingSector => {
                let angle = self.outer_ring_rotation + (m.slot_index as f32 / 16.0) * TAU;
                [
                    STATION_CENTER[0] + angle.cos() * 134.0,
                    STATION_CENTER[1] + angle.sin() * 134.0 * 0.42,
                ]
            }
            _ => m.anchor_pos,
        }
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
        self.ensure_migrated();

        self.total_age_secs += dt_secs as f64;
        self.stats.station_age_hours = self.total_age_secs / 3600.0;

        // 1. Orbital & centrifugal ring mechanics
        self.orbit_angle = (self.orbit_angle + dt_secs * 0.018) % TAU;
        self.ring_rotation = (self.ring_rotation + dt_secs * 0.085) % TAU;
        self.outer_ring_rotation = (self.outer_ring_rotation - dt_secs * 0.052).rem_euclid(TAU);

        // 2. Subspace scan pulse animation
        if self.scan_pulse.active {
            self.scan_pulse.progress += dt_secs * 0.55;
            if self.scan_pulse.progress >= 1.0 {
                self.scan_pulse.active = false;
                self.scan_pulse.progress = 0.0;
            }
        }

        // 3. Slow-burn construction tick (accelerated by active WelderDrones and Overdrive)
        let active_welders = self
            .robots
            .iter()
            .filter(|r| matches!(r.kind, RobotKind::WelderDrone))
            .count() as f32;
        let boost_mult = if self.robots.iter().any(|r| r.boost_timer > 0.0) {
            4.5
        } else {
            1.0
        };
        let base_rate = (1.0 / 900.0) * (1.0 + active_welders * 0.25) * boost_mult;
        self.advance_construction(dt_secs * base_rate);

        // 4. Update Orbital Asteroids
        for ast in &mut self.asteroids {
            ast.pos[0] += ast.vel[0] * dt_secs;
            ast.pos[1] += ast.vel[1] * dt_secs;
            ast.rotation = (ast.rotation + ast.rot_speed * dt_secs) % TAU;
            if ast.pos[0] < -60.0 {
                ast.pos[0] = 1060.0;
            } else if ast.pos[0] > 1060.0 {
                ast.pos[0] = -60.0;
            }
            if ast.pos[1] < 45.0 || ast.pos[1] > 230.0 {
                ast.vel[1] = -ast.vel[1];
            }
        }
        self.asteroids.retain(|a| a.ore_remaining > 1.0);
        while self.asteroids.len() < 4 {
            self.next_id += 1;
            let from_left = self.rng.bool_with_prob(0.5);
            let x = if from_left { -40.0 } else { 1040.0 };
            let y = self.rng.range(68.0, 195.0);
            let vx = if from_left {
                self.rng.range(4.0, 9.0)
            } else {
                self.rng.range(-9.0, -4.0)
            };
            self.asteroids.push(OrbitalAsteroid {
                id: self.next_id,
                pos: [x, y],
                vel: [vx, self.rng.range(-0.8, 0.8)],
                radius: self.rng.range(13.0, 23.0),
                rotation: 0.0,
                rot_speed: self.rng.range(-0.4, 0.4),
                ore_remaining: 100.0,
                max_ore: 100.0,
                richness: (self.next_id % 3) as u8,
            });
        }

        // 5. Update Autonomous Robotics (Welders, Spine Crawlers, Cargo Tugs, Micro-Sats)
        let active_mod_pos = self
            .active_construction_module_index()
            .map(|idx| self.module_world_pos(&self.modules[idx]))
            .unwrap_or(STATION_CENTER);

        let mut new_sparks: Vec<([f32; 2], u8)> = Vec::new();
        for robot in &mut self.robots {
            robot.phase = (robot.phase + dt_secs * 3.2) % TAU;
            if robot.boost_timer > 0.0 {
                robot.boost_timer = (robot.boost_timer - dt_secs).max(0.0);
            }
            let speed_scale = if robot.boost_timer > 0.0 { 2.2 } else { 1.0 };

            match &mut robot.task {
                RobotTask::Welding { orbit_phase, .. } => {
                    *orbit_phase = (*orbit_phase + dt_secs * 1.8 * speed_scale) % TAU;
                    let target = [
                        active_mod_pos[0] + orbit_phase.cos() * 18.0,
                        active_mod_pos[1] + orbit_phase.sin() * 12.0,
                    ];
                    let dx = target[0] - robot.pos[0];
                    let dy = target[1] - robot.pos[1];
                    robot.vel = [dx * 3.5, dy * 3.5];
                    robot.pos[0] += robot.vel[0] * dt_secs;
                    robot.pos[1] += robot.vel[1] * dt_secs;

                    if (dx * dx + dy * dy) < 400.0 && self.rng.bool_with_prob(dt_secs * 4.5) {
                        new_sparks.push((active_mod_pos, 0));
                    }
                }
                RobotTask::CrawlingSpine { t, dir } => {
                    *t += *dir * dt_secs * 0.18 * speed_scale;
                    if *t > 1.0 {
                        *t = 1.0;
                        *dir = -1.0;
                    } else if *t < -1.0 {
                        *t = -1.0;
                        *dir = 1.0;
                    }
                    let spine_half_h = (self.spine_segments.max(2) as f32) * 18.0;
                    robot.pos = [
                        STATION_CENTER[0] + (robot.phase.sin() * 3.0),
                        STATION_CENTER[1] + *t * spine_half_h,
                    ];
                }
                RobotTask::HaulingCargo { from, to, progress } => {
                    *progress += dt_secs * 0.28 * speed_scale;
                    if *progress >= 1.0 {
                        *progress = 0.0;
                        std::mem::swap(from, to);
                        *to = active_mod_pos;
                    }
                    let s = *progress;
                    robot.pos = [
                        from[0] + (to[0] - from[0]) * s,
                        from[1] + (to[1] - from[1]) * s,
                    ];
                }
                RobotTask::PerimeterOrbit {
                    angle,
                    radius_x,
                    radius_y,
                    speed,
                } => {
                    *angle = (*angle + *speed * dt_secs * speed_scale).rem_euclid(TAU);
                    robot.pos = [
                        STATION_CENTER[0] + angle.cos() * *radius_x,
                        STATION_CENTER[1] + angle.sin() * *radius_y,
                    ];
                }
            }
        }

        for (pt, kind) in new_sparks {
            self.spawn_sparks(pt, 2, kind);
        }

        // 6. Update Spacecraft Fleet
        let first_ast = self.asteroids.first().map(|a| (a.id, a.pos));
        let mut mined_ast_id: Option<u32> = None;
        let mut mining_spark_pts: Vec<[f32; 2]> = Vec::new();
        let mut completed_trips = 0u32;

        for shuttle in &mut self.shuttles {
            match &mut shuttle.state {
                ShuttleState::Approaching => {
                    let dx = shuttle.target_dock[0] - shuttle.pos[0];
                    let dy = shuttle.target_dock[1] - shuttle.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 9.0 {
                        shuttle.pos = shuttle.target_dock;
                        shuttle.vel = [0.0, 0.0];
                        shuttle.state = ShuttleState::Docked;
                        shuttle.docking_timer = 7.5;
                        completed_trips += 1;
                    } else {
                        let speed = match shuttle.class {
                            ShipClass::BulkFreighter => 22.0,
                            ShipClass::HabLiner => 26.0,
                            ShipClass::SolarClipper => 30.0,
                            _ => 34.0,
                        };
                        // Gently decelerate on final approach
                        let throttle = (dist / 60.0).clamp(0.35, 1.0);
                        let vx = (dx / dist) * speed * throttle;
                        let vy = (dy / dist) * speed * throttle;
                        shuttle.pos[0] += vx * dt_secs;
                        shuttle.pos[1] += vy * dt_secs;
                        shuttle.vel = [vx, vy];
                    }
                }
                ShuttleState::Docked => {
                    shuttle.docking_timer -= dt_secs;
                    shuttle.vel = [0.0, 0.0];
                    if shuttle.docking_timer <= 0.0 {
                        if shuttle.class == ShipClass::MiningSkiff
                            && let Some((aid, _)) = first_ast
                        {
                            shuttle.state = ShuttleState::MiningAsteroid {
                                asteroid_id: aid,
                                timer: 9.0,
                            };
                        } else {
                            shuttle.state = ShuttleState::Departing;
                            let out_x = if shuttle.pos[0] >= STATION_CENTER[0] {
                                32.0
                            } else {
                                -32.0
                            };
                            shuttle.vel = [out_x, -14.0];
                        }
                    }
                }
                ShuttleState::MiningAsteroid { asteroid_id, timer } => {
                    let target_ast = self
                        .asteroids
                        .iter()
                        .find(|a| a.id == *asteroid_id)
                        .or_else(|| self.asteroids.first());

                    if let Some(ast) = target_ast {
                        *asteroid_id = ast.id;
                        let stand_off = [ast.pos[0], ast.pos[1] + ast.radius + 26.0];
                        let dx = stand_off[0] - shuttle.pos[0];
                        let dy = stand_off[1] - shuttle.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 12.0 {
                            let speed = 34.0;
                            shuttle.vel = [(dx / dist) * speed, (dy / dist) * speed];
                            shuttle.pos[0] += shuttle.vel[0] * dt_secs;
                            shuttle.pos[1] += shuttle.vel[1] * dt_secs;
                        } else {
                            // Actively laser-mining the asteroid!
                            shuttle.vel = [ast.vel[0], ast.vel[1]];
                            shuttle.pos[0] += shuttle.vel[0] * dt_secs;
                            shuttle.pos[1] += shuttle.vel[1] * dt_secs;
                            *timer -= dt_secs;
                            mined_ast_id = Some(ast.id);
                            if self.rng.bool_with_prob(dt_secs * 5.0) {
                                mining_spark_pts.push(ast.pos);
                            }
                            if *timer <= 0.0 {
                                shuttle.state = ShuttleState::Approaching;
                                shuttle.target_dock =
                                    [STATION_CENTER[0] + 28.0, STATION_CENTER[1] - 32.0];
                            }
                        }
                    } else {
                        shuttle.state = ShuttleState::Approaching;
                    }
                }
                ShuttleState::Patrolling {
                    angle,
                    radius,
                    timer,
                } => {
                    *angle = (*angle + dt_secs * 0.32) % TAU;
                    *timer -= dt_secs;
                    let target = [
                        STATION_CENTER[0] + angle.cos() * *radius,
                        STATION_CENTER[1] + angle.sin() * (*radius * 0.42),
                    ];
                    shuttle.vel = [
                        -angle.sin() * *radius * 0.32,
                        angle.cos() * (*radius * 0.42) * 0.32,
                    ];
                    shuttle.pos = target;
                    if *timer <= 0.0 {
                        shuttle.state = ShuttleState::Approaching;
                        shuttle.target_dock = STATION_CENTER;
                    }
                }
                ShuttleState::Departing => {
                    // Gently accelerate into orbital lane
                    shuttle.vel[0] *= 1.0 + dt_secs * 0.25;
                    shuttle.pos[0] += shuttle.vel[0] * dt_secs;
                    shuttle.pos[1] += shuttle.vel[1] * dt_secs;
                }
            }
        }

        if completed_trips > 0 {
            self.stats.shuttles_serviced += completed_trips;
            self.stats.alloy_stockpile += completed_trips as f32 * 18.0;
            self.advance_construction(completed_trips as f32 * 0.06);
        }

        if let Some(aid) = mined_ast_id
            && let Some(ast) = self.asteroids.iter_mut().find(|a| a.id == aid)
        {
            ast.ore_remaining = (ast.ore_remaining - dt_secs * 4.5).max(0.0);
            self.stats.alloy_stockpile += dt_secs * 4.5;
            if ast.ore_remaining <= 1.0 {
                self.stats.asteroids_mined += 1;
            }
        }

        for pt in mining_spark_pts {
            self.spawn_sparks(pt, 2, 2);
        }

        // Retain in-bounds spacecraft & keep a lively 3..=5 ship traffic cadence
        self.shuttles.retain(|s| {
            s.pos[0] > -140.0 && s.pos[0] < 1140.0 && s.pos[1] > -140.0 && s.pos[1] < 890.0
        });
        let target_fleet = (3 + (self.stats.total_modules / 16) as usize).min(6);
        if self.shuttles.len() < target_fleet {
            self.launch_shuttle();
        }

        // 7. Update Spark Particles
        for sp in &mut self.sparks {
            sp.life += dt_secs;
            sp.pos[0] += sp.vel[0] * dt_secs;
            sp.pos[1] += sp.vel[1] * dt_secs;
            sp.vel[0] *= 1.0 - dt_secs * 1.5;
            sp.vel[1] *= 1.0 - dt_secs * 1.5;
        }
        self.sparks.retain(|s| s.life < s.max_life);

        Activity::Animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbital_haven_v2_initialization() {
        let station = OrbitalHavenState::new(42);
        assert_eq!(station.solar_wings, 4);
        assert_eq!(station.habitat_sectors, 4);
        assert_eq!(station.modules.len(), 66);
        assert!(station.robots.len() >= 6);
        assert_eq!(station.shuttles.len(), 3);
        assert_eq!(station.asteroids.len(), 4);
    }

    #[test]
    fn orbital_haven_shuttle_and_fleet_diversity() {
        let mut station = OrbitalHavenState::new(42);
        let initial_len = station.shuttles.len();
        station.launch_shuttle();
        assert_eq!(station.shuttles.len(), initial_len + 1);
    }

    #[test]
    fn orbital_haven_offline_progression_and_epochs() {
        let mut station = OrbitalHavenState::new(42);
        let initial_modules = station.stats.total_modules;
        station.on_wake(3600.0 * 12.0); // 12 hours
        assert!(station.stats.total_modules > initial_modules);
        assert!(station.stats.shuttles_serviced > 0);
        assert!(station.epoch.tier_number() >= 2);
    }

    #[test]
    fn orbital_haven_persistence_and_share_strings() {
        let mut station = OrbitalHavenState::new(777);
        station.cycle_planet_biome();
        station.overdrive_drones();

        let seed_uri = station.seed_uri();
        assert!(seed_uri.starts_with("rookery://orbital/seed-"));
        let from_seed = OrbitalHavenState::from_share_string(&seed_uri).unwrap();
        assert_eq!(from_seed.seed, 777);

        let state_uri = station.state_uri().unwrap();
        assert!(state_uri.starts_with("rookery://orbital/state-"));
        let from_state = OrbitalHavenState::from_share_string(&state_uri).unwrap();
        assert_eq!(from_state.planet_biome, station.planet_biome);
        assert_eq!(from_state.stats.total_modules, station.stats.total_modules);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        station.save_atomic(tmp.path()).unwrap();
        let loaded = OrbitalHavenState::load_or_init(tmp.path(), 1);
        assert_eq!(loaded.seed, 777);
        assert_eq!(loaded.planet_biome, station.planet_biome);
    }
}
