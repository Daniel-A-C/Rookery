//! Core data types, enums, and telemetry structs for *Orbital Haven*.

use serde::{Deserialize, Serialize};

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
