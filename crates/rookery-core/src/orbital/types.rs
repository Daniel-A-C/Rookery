//! Core data types, enums, and telemetry structs for *Orbital Haven v3* (Station HELIOS-09).

use serde::{Deserialize, Serialize};

pub const ORBITAL_HAVEN_STATE_VERSION: u32 = 3;
pub const ORBITAL_STATE_FILE_NAME: &str = "orbital_haven.json";

/// Multi-camera viewport layout mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraLayout {
    /// Single full-screen live camera feed.
    Single,
    /// Full-screen primary feed with 2 framed CCTV picture-in-picture mini-monitors.
    #[default]
    Pip,
    /// 2x2 quad-split CCTV surveillance monitor wall.
    Grid2x2,
}

impl CameraLayout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Single => "Single Feed",
            Self::Pip => "Main + 2 PiP",
            Self::Grid2x2 => "2x2 CCTV Wall",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Single => "1x FULL",
            Self::Pip => "MAIN+PiP",
            Self::Grid2x2 => "2x2 GRID",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Single => Self::Pip,
            Self::Pip => Self::Grid2x2,
            Self::Grid2x2 => Self::Single,
        }
    }
}

/// Named camera feeds available on Station HELIOS-09's surveillance & uplink network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraFeedId {
    /// Wide orbital camera drone framing the entire station and planet.
    #[default]
    DroneAlphaWide,
    /// Low-altitude orbital camera drone weaving past pads and trusses.
    DroneBetaPatrol,
    /// Exterior gantry cam overlooking the main hangar approach corridor.
    ExtHangarApproach,
    /// Interior Hangar Bay CCTV framing Pad A and the open containment forcefield.
    PadAHangarInterior,
    /// External Port Landing Pad B floodlight-tower CCTV.
    PadBPortDeck,
    /// External Starboard Landing Pad C floodlight-tower CCTV.
    PadCStarboardDeck,
    /// Interior Ops & Command Deck CCTV looking past CRT consoles out the main viewports.
    IntOpsDeck,
    /// Interior Concourse Corridor CCTV with grated catwalk and space observation windows.
    IntConcourseWalk,
    /// Interior Reactor Chamber CCTV overlooking the pulsing plasma column.
    IntReactorCore,
    /// Live cockpit/nose uplink feed from the active approaching or departing spacecraft.
    ShipNoseUplink,
    /// Live dorsal over-the-shoulder hull feed from the active spacecraft.
    ShipDorsalUplink,
}

pub const ALL_CAMERA_FEEDS: [CameraFeedId; 11] = [
    CameraFeedId::DroneAlphaWide,
    CameraFeedId::DroneBetaPatrol,
    CameraFeedId::ExtHangarApproach,
    CameraFeedId::PadAHangarInterior,
    CameraFeedId::PadBPortDeck,
    CameraFeedId::PadCStarboardDeck,
    CameraFeedId::IntOpsDeck,
    CameraFeedId::IntConcourseWalk,
    CameraFeedId::IntReactorCore,
    CameraFeedId::ShipNoseUplink,
    CameraFeedId::ShipDorsalUplink,
];

impl CameraFeedId {
    pub fn code(self) -> &'static str {
        match self {
            Self::DroneAlphaWide => "CAM-01",
            Self::DroneBetaPatrol => "CAM-02",
            Self::ExtHangarApproach => "CAM-03",
            Self::PadAHangarInterior => "CAM-04",
            Self::PadBPortDeck => "CAM-05",
            Self::PadCStarboardDeck => "CAM-06",
            Self::IntOpsDeck => "CAM-07",
            Self::IntConcourseWalk => "CAM-08",
            Self::IntReactorCore => "CAM-09",
            Self::ShipNoseUplink => "UPLINK-1",
            Self::ShipDorsalUplink => "UPLINK-2",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DroneAlphaWide => "DRONE-ALPHA // WIDE ORBIT",
            Self::DroneBetaPatrol => "DRONE-BETA // LOW PATROL",
            Self::ExtHangarApproach => "EXT-01 // HANGAR APPROACH",
            Self::PadAHangarInterior => "PAD-A // MAIN HANGAR BAY",
            Self::PadBPortDeck => "PAD-B // PORT FREIGHT DECK",
            Self::PadCStarboardDeck => "PAD-C // STBD UTILITY DECK",
            Self::IntOpsDeck => "INT-01 // OPS COMMAND DECK",
            Self::IntConcourseWalk => "INT-02 // CONCOURSE WALK",
            Self::IntReactorCore => "INT-03 // REACTOR CHAMBER",
            Self::ShipNoseUplink => "LIVE // SHIP NOSE-CAM",
            Self::ShipDorsalUplink => "LIVE // SHIP DORSAL-CAM",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::DroneAlphaWide => "DRONE-ALPHA",
            Self::DroneBetaPatrol => "DRONE-BETA",
            Self::ExtHangarApproach => "HANGAR EXT",
            Self::PadAHangarInterior => "PAD-A BAY",
            Self::PadBPortDeck => "PAD-B PORT",
            Self::PadCStarboardDeck => "PAD-C STBD",
            Self::IntOpsDeck => "OPS DECK",
            Self::IntConcourseWalk => "CONCOURSE",
            Self::IntReactorCore => "REACTOR",
            Self::ShipNoseUplink => "SHIP NOSE",
            Self::ShipDorsalUplink => "SHIP DORSAL",
        }
    }

    pub fn is_interior(self) -> bool {
        matches!(
            self,
            Self::PadAHangarInterior
                | Self::IntOpsDeck
                | Self::IntConcourseWalk
                | Self::IntReactorCore
        )
    }

    pub fn is_ship_uplink(self) -> bool {
        matches!(self, Self::ShipNoseUplink | Self::ShipDorsalUplink)
    }

    pub fn next(self) -> Self {
        let idx = ALL_CAMERA_FEEDS
            .iter()
            .position(|&c| c == self)
            .unwrap_or(0);
        ALL_CAMERA_FEEDS[(idx + 1) % ALL_CAMERA_FEEDS.len()]
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "drone_alpha" | "cam_01" => Some(Self::DroneAlphaWide),
            "drone_beta" | "cam_02" => Some(Self::DroneBetaPatrol),
            "hangar_approach" | "cam_03" => Some(Self::ExtHangarApproach),
            "pad_a" | "cam_04" => Some(Self::PadAHangarInterior),
            "pad_b" | "cam_05" => Some(Self::PadBPortDeck),
            "pad_c" | "cam_06" => Some(Self::PadCStarboardDeck),
            "ops_deck" | "cam_07" => Some(Self::IntOpsDeck),
            "concourse" | "cam_08" => Some(Self::IntConcourseWalk),
            "reactor" | "cam_09" => Some(Self::IntReactorCore),
            "ship_nose" | "uplink_1" => Some(Self::ShipNoseUplink),
            "ship_dorsal" | "uplink_2" => Some(Self::ShipDorsalUplink),
            _ => None,
        }
    }
}

/// Curated Y2K immersive-sim color & lighting themes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbitalPalette {
    /// Classic Deus Ex (2000) UNATCO steel-blue, cyan CRT glow, sodium-amber dock floodlights.
    #[default]
    UnatcoCobalt,
    /// System Shock 2 (1999) Von Braun oxidized olive-teal, rust steel, toxic green & crimson strobes.
    VonBraunIndustrial,
    /// Noir corporate obsidian slate, warm amber tungsten, gold telemetry.
    VersaLifeGold,
    /// High-contrast SHODAN terminal aesthetic: stark iron, emerald phosphor, magenta glitch accents.
    ShodanCyber,
}

pub const ORBITAL_PALETTE_NAMES: [&str; 4] = [
    "UNATCO Cobalt",
    "Von Braun Industrial",
    "VersaLife Gold",
    "SHODAN Cyber",
];

impl OrbitalPalette {
    pub fn name(self) -> &'static str {
        match self {
            Self::UnatcoCobalt => ORBITAL_PALETTE_NAMES[0],
            Self::VonBraunIndustrial => ORBITAL_PALETTE_NAMES[1],
            Self::VersaLifeGold => ORBITAL_PALETTE_NAMES[2],
            Self::ShodanCyber => ORBITAL_PALETTE_NAMES[3],
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::UnatcoCobalt => 0,
            Self::VonBraunIndustrial => 1,
            Self::VersaLifeGold => 2,
            Self::ShodanCyber => 3,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx % 4 {
            0 => Self::UnatcoCobalt,
            1 => Self::VonBraunIndustrial,
            2 => Self::VersaLifeGold,
            _ => Self::ShodanCyber,
        }
    }

    pub fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }
}

/// Dynamic lighting and operational state across Station HELIOS-09.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationEventKind {
    /// Standard orbital operations.
    #[default]
    Nominal,
    /// Active docking approach: amber runway chase lights & boosted pad floodlights.
    DockingOps,
    /// Station passes into planetary umbra: exterior dims dramatically while windows & floods pop.
    SolarEclipse,
    /// Periodic security sweep / red alert: pulsing crimson corridor & beacon lights.
    SecurityAlert,
}

impl StationEventKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Nominal => "NOMINAL",
            Self::DockingOps => "DOCKING OPS",
            Self::SolarEclipse => "SOLAR ECLIPSE",
            Self::SecurityAlert => "SECURITY ALERT",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Nominal => Self::DockingOps,
            Self::DockingOps => Self::SolarEclipse,
            Self::SolarEclipse => Self::SecurityAlert,
            Self::SecurityAlert => Self::Nominal,
        }
    }
}

/// Distinct low-poly 3D spacecraft hull classes visiting HELIOS-09.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipClass {
    /// Sleek angular wedge corporate transport — flies directly inside Hangar Pad A.
    #[default]
    CorporateShuttle,
    /// Heavy boxy twin-pod industrial bulk freighter — docks on external Pad B or Pad C.
    BulkFreighter,
    /// Swept-wing orbital security interceptor — patrols perimeter or lands on Pad C.
    PatrolCutter,
    /// Chunky utility maintenance tug with amber work strobes — services Pads B & C.
    MaintenanceTug,
}

impl ShipClass {
    pub fn name(self) -> &'static str {
        match self {
            Self::CorporateShuttle => "Corporate Shuttle",
            Self::BulkFreighter => "Bulk Freighter",
            Self::PatrolCutter => "Patrol Cutter",
            Self::MaintenanceTug => "Maintenance Tug",
        }
    }
}

/// Station landing pad assignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandingPadId {
    /// Main Hangar Bay interior pad (through atmospheric forcefield).
    #[default]
    PadAHangar,
    /// External cantilevered Port Freight Deck.
    PadBPort,
    /// External cantilevered Starboard Utility Deck.
    PadCStarboard,
}

impl LandingPadId {
    pub fn label(self) -> &'static str {
        match self {
            Self::PadAHangar => "PAD-A",
            Self::PadBPort => "PAD-B",
            Self::PadCStarboard => "PAD-C",
        }
    }
}

/// 3D autonomous flight phase managed by station ATC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipFlightPhase {
    /// High-speed approach from deep space toward the pad's approach gate.
    #[default]
    InboundApproach,
    /// Braking burn, alignment, landing gear deployment, and touchdown onto the pad.
    FinalDescent,
    /// Landed on pad while cargo loader bots service the vessel.
    Docked,
    /// Vertical lift-off and clearance out of the hangar / pad zone.
    Liftoff,
    /// Accelerating back out to orbital transit lanes.
    OutboundBurn,
    /// Continuous elliptical security patrol around the station superstructure.
    PerimeterPatrol,
}

/// A 3D spacecraft operating in the HELIOS-09 orbital sector.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spacecraft {
    pub id: u32,
    pub class: ShipClass,
    pub callsign: String,
    pub assigned_pad: LandingPadId,
    pub phase: ShipFlightPhase,
    /// Current 3D world position.
    pub pos: [f32; 3],
    /// Current 3D world velocity.
    pub vel: [f32; 3],
    /// Unit forward direction vector.
    pub forward: [f32; 3],
    /// Unit up vector.
    pub up: [f32; 3],
    /// Start waypoint of the current flight leg.
    pub leg_start: [f32; 3],
    /// Target waypoint of the current flight leg.
    pub leg_end: [f32; 3],
    /// `[0.0, 1.0]` normalized progress along the current leg (or docked timer progress).
    pub leg_progress: f32,
    /// Duration in seconds for the current leg or docked stay.
    pub leg_duration: f32,
    /// `[0.0, 1.0]` landing gear extension.
    pub gear_deploy: f32,
    /// `[0.0, 1.0]` main engine thruster glow intensity.
    pub thruster_power: f32,
    /// Patrol orbit angle (used only for `PerimeterPatrol`).
    pub patrol_angle: f32,
    /// Patrol orbit radius.
    pub patrol_radius: f32,
}

/// A physical 3D camera drone orbiting Station HELIOS-09.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraDrone {
    pub id: u32,
    pub feed: CameraFeedId,
    pub label: String,
    pub pos: [f32; 3],
    pub look_target: [f32; 3],
    pub orbit_angle: f32,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub height_base: f32,
    pub height_amp: f32,
}

/// Class of autonomous robot or crew member moving inside or on the station.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// Tracked yellow/gunmetal cargo loader bot operating in the Hangar Bay & external pads.
    CargoLoader,
    /// Floating/bipedal System Shock style security patrol droid in the Concourse & Reactor.
    SecurityDroid,
    /// Stylized low-poly station officer/technician patrolling the Ops Deck & Concourse.
    OpsCrew,
    /// Exterior hull welder drone drifting along station trusses with welding arcs.
    HullWelder,
}

/// Zone where a station agent operates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentZone {
    HangarBay,
    PadBPort,
    PadCStarboard,
    OpsDeck,
    Concourse,
    ReactorCatwalk,
    ExteriorTruss,
}

/// An autonomous bot or crew figure bringing interiors and docks to life.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationAgent {
    pub id: u32,
    pub kind: AgentKind,
    pub zone: AgentZone,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub patrol_t: f32,
    pub speed: f32,
    pub direction: f32,
    pub phase: f32,
}

/// Multi-camera surveillance rig & auto-director state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraRigState {
    pub layout: CameraLayout,
    pub primary_feed: CameraFeedId,
    pub pip_feeds: [CameraFeedId; 2],
    pub grid_feeds: [CameraFeedId; 4],
    pub auto_director: bool,
    pub director_timer: f32,
    /// Brief `[0.0..0.18]` seconds static/glitch burst triggered on camera switch.
    pub static_glitch_timer: f32,
    /// User manual drag pan/tilt offset in radians `[yaw, pitch]`.
    pub manual_pan: [f32; 2],
    /// ID of the spacecraft currently locked by the live ship uplink feeds.
    pub uplink_ship_id: Option<u32>,
}

impl Default for CameraRigState {
    fn default() -> Self {
        Self {
            layout: CameraLayout::Pip,
            primary_feed: CameraFeedId::DroneAlphaWide,
            pip_feeds: [
                CameraFeedId::PadAHangarInterior,
                CameraFeedId::ShipNoseUplink,
            ],
            grid_feeds: [
                CameraFeedId::DroneAlphaWide,
                CameraFeedId::ShipNoseUplink,
                CameraFeedId::PadAHangarInterior,
                CameraFeedId::IntOpsDeck,
            ],
            auto_director: true,
            director_timer: 18.0,
            static_glitch_timer: 0.0,
            manual_pan: [0.0, 0.0],
            uplink_ship_id: None,
        }
    }
}

/// Live operational telemetry for Station HELIOS-09.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenStats {
    pub station_age_hours: f64,
    pub active_ships: u32,
    pub docked_ships: u32,
    pub ships_serviced: u32,
    pub active_drones: u32,
    pub active_agents: u32,
    pub uplink_callsign: String,
    pub uplink_phase: String,
}
