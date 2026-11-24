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

    pub fn slug(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Pip => "pip",
            Self::Grid2x2 => "grid2x2",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "single" => Some(Self::Single),
            "pip" => Some(Self::Pip),
            "grid2x2" | "grid" => Some(Self::Grid2x2),
            _ => None,
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
    /// Interior 2-story Central Core Atrium CCTV overlooking Hangar door & glass Elevator.
    IntCentralAtrium,
    /// Interior Ops & Command Deck CCTV looking across holotable & 3-deck glass elevator.
    IntOpsDeck,
    /// Interior Reactor Chamber & Concourse CCTV overlooking the plasma column & Deck 1 elevator.
    #[serde(alias = "int_concourse_walk")]
    IntReactorCore,
    /// Interior Aft Solar & Power Engineering Bay CCTV looking out onto the Solar Collector Wings.
    IntSolarBay,
    /// Live first-person camera from the patrolling Interior Walker Bot (BOT-01).
    WalkerBotCam,
    /// Live cockpit/nose uplink feed from the active approaching or departing spacecraft.
    ShipNoseUplink,
    /// Live dorsal over-the-shoulder hull feed from the active spacecraft.
    ShipDorsalUplink,
    /// Auto-rotating feed cycling through all station & ship cameras.
    AutoRotateAll,
    /// Auto-rotating feed cycling through interior rooms & the BOT-01 rover.
    AutoRotateInterior,
    /// Auto-rotating feed cycling through exterior drones, pads & ship uplinks.
    AutoRotateExterior,
    /// Automatically switches to whichever room CCTV camera currently frames BOT-01.
    FollowBotRoom,
}

pub const ALL_CAMERA_FEEDS: [CameraFeedId; 13] = [
    CameraFeedId::DroneAlphaWide,
    CameraFeedId::DroneBetaPatrol,
    CameraFeedId::ExtHangarApproach,
    CameraFeedId::PadAHangarInterior,
    CameraFeedId::PadBPortDeck,
    CameraFeedId::PadCStarboardDeck,
    CameraFeedId::IntCentralAtrium,
    CameraFeedId::IntOpsDeck,
    CameraFeedId::IntReactorCore,
    CameraFeedId::IntSolarBay,
    CameraFeedId::WalkerBotCam,
    CameraFeedId::ShipNoseUplink,
    CameraFeedId::ShipDorsalUplink,
];

pub const INTERIOR_CAMERA_FEEDS: [CameraFeedId; 6] = [
    CameraFeedId::PadAHangarInterior,
    CameraFeedId::IntCentralAtrium,
    CameraFeedId::IntOpsDeck,
    CameraFeedId::IntReactorCore,
    CameraFeedId::IntSolarBay,
    CameraFeedId::WalkerBotCam,
];

pub const EXTERIOR_CAMERA_FEEDS: [CameraFeedId; 7] = [
    CameraFeedId::DroneAlphaWide,
    CameraFeedId::DroneBetaPatrol,
    CameraFeedId::ExtHangarApproach,
    CameraFeedId::PadBPortDeck,
    CameraFeedId::PadCStarboardDeck,
    CameraFeedId::ShipNoseUplink,
    CameraFeedId::ShipDorsalUplink,
];

/// Interval in seconds between automatic cuts on a rotating feed slot.
pub const ROTATING_FEED_INTERVAL_SECS: f64 = 16.0;

impl CameraFeedId {
    pub fn code(self) -> &'static str {
        match self {
            Self::DroneAlphaWide => "CAM-01",
            Self::DroneBetaPatrol => "CAM-02",
            Self::ExtHangarApproach => "CAM-03",
            Self::PadAHangarInterior => "CAM-04",
            Self::PadBPortDeck => "CAM-05",
            Self::PadCStarboardDeck => "CAM-06",
            Self::IntCentralAtrium => "CAM-07",
            Self::IntOpsDeck => "CAM-08",
            Self::IntReactorCore => "CAM-09",
            Self::IntSolarBay => "CAM-10",
            Self::WalkerBotCam => "BOT-01",
            Self::ShipNoseUplink => "UPLINK-1",
            Self::ShipDorsalUplink => "UPLINK-2",
            Self::AutoRotateAll => "AUTO",
            Self::AutoRotateInterior => "AUTO-INT",
            Self::AutoRotateExterior => "AUTO-EXT",
            Self::FollowBotRoom => "TRACK",
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
            Self::IntCentralAtrium => "INT-01 // CENTRAL CORE ATRIUM",
            Self::IntOpsDeck => "INT-02 // OPS COMMAND DECK",
            Self::IntReactorCore => "INT-03 // REACTOR & CONCOURSE",
            Self::IntSolarBay => "INT-04 // SOLAR ENGINEERING",
            Self::WalkerBotCam => "ROVER // INTERIOR WALKER-CAM",
            Self::ShipNoseUplink => "LIVE // SHIP NOSE-CAM",
            Self::ShipDorsalUplink => "LIVE // SHIP DORSAL-CAM",
            Self::AutoRotateAll => "AUTO // ROTATING ALL CAMS",
            Self::AutoRotateInterior => "AUTO // ROTATING INTERIORS",
            Self::AutoRotateExterior => "AUTO // ROTATING EXTERIORS",
            Self::FollowBotRoom => "TRACK // FOLLOW BOT-01 ROOM",
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
            Self::IntCentralAtrium => "ATRIUM",
            Self::IntOpsDeck => "OPS DECK",
            Self::IntReactorCore => "REACTOR",
            Self::IntSolarBay => "SOLAR BAY",
            Self::WalkerBotCam => "WALKER BOT",
            Self::ShipNoseUplink => "SHIP NOSE",
            Self::ShipDorsalUplink => "SHIP DORSAL",
            Self::AutoRotateAll => "ROTATE ALL",
            Self::AutoRotateInterior => "ROTATE INT",
            Self::AutoRotateExterior => "ROTATE EXT",
            Self::FollowBotRoom => "FOLLOW BOT",
        }
    }

    pub fn is_interior(self) -> bool {
        matches!(
            self,
            Self::PadAHangarInterior
                | Self::IntCentralAtrium
                | Self::IntOpsDeck
                | Self::IntReactorCore
                | Self::IntSolarBay
                | Self::WalkerBotCam
                | Self::AutoRotateInterior
                | Self::FollowBotRoom
        )
    }

    pub fn is_ship_uplink(self) -> bool {
        matches!(self, Self::ShipNoseUplink | Self::ShipDorsalUplink)
    }

    pub fn is_rotating(self) -> bool {
        matches!(
            self,
            Self::AutoRotateAll | Self::AutoRotateInterior | Self::AutoRotateExterior
        )
    }

    /// True for any dynamic meta-feed (time-rotating or BOT-01 room-tracking) that manages its own cuts.
    pub fn is_dynamic(self) -> bool {
        self.is_rotating() || self == Self::FollowBotRoom
    }

    /// Resolve a rotating meta-feed into the concrete camera currently active at `total_age_secs`.
    /// `slot_offset` staggers simultaneous rotating sub-views so they don't duplicate each other.
    pub fn resolve_active(self, total_age_secs: f64, slot_offset: usize) -> Self {
        let step = (total_age_secs.max(0.0) / ROTATING_FEED_INTERVAL_SECS) as usize;
        match self {
            Self::AutoRotateAll => {
                ALL_CAMERA_FEEDS[(step + slot_offset * 3) % ALL_CAMERA_FEEDS.len()]
            }
            Self::AutoRotateInterior => {
                INTERIOR_CAMERA_FEEDS[(step + slot_offset * 2) % INTERIOR_CAMERA_FEEDS.len()]
            }
            Self::AutoRotateExterior => {
                EXTERIOR_CAMERA_FEEDS[(step + slot_offset * 2) % EXTERIOR_CAMERA_FEEDS.len()]
            }
            other => other,
        }
    }

    /// Resolve any dynamic meta-feed (rotating OR `FollowBotRoom`) into a concrete camera feed.
    pub fn resolve_for_state(
        self,
        total_age_secs: f64,
        walker_pos: [f32; 3],
        slot_offset: usize,
    ) -> Self {
        match self {
            Self::FollowBotRoom => {
                if walker_pos[1] > 6.0 {
                    Self::IntOpsDeck
                } else if walker_pos[1] < -9.5 {
                    Self::IntReactorCore
                } else if walker_pos[2] > 11.6 {
                    Self::PadAHangarInterior
                } else if walker_pos[2] > -14.6 {
                    Self::IntCentralAtrium
                } else if walker_pos[2] > -25.1 {
                    // Inside the Axial Pipe Tunnel: cut to BOT-01's onboard porthole view!
                    Self::WalkerBotCam
                } else {
                    Self::IntSolarBay
                }
            }
            other => other.resolve_active(total_age_secs, slot_offset),
        }
    }

    pub fn next(self) -> Self {
        let idx = ALL_CAMERA_FEEDS
            .iter()
            .position(|&c| c == self)
            .unwrap_or(0);
        ALL_CAMERA_FEEDS[(idx + 1) % ALL_CAMERA_FEEDS.len()]
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::DroneAlphaWide => "drone_alpha",
            Self::DroneBetaPatrol => "drone_beta",
            Self::ExtHangarApproach => "hangar_approach",
            Self::PadAHangarInterior => "pad_a",
            Self::PadBPortDeck => "pad_b",
            Self::PadCStarboardDeck => "pad_c",
            Self::IntCentralAtrium => "atrium",
            Self::IntOpsDeck => "ops_deck",
            Self::IntReactorCore => "reactor",
            Self::IntSolarBay => "solar_bay",
            Self::WalkerBotCam => "walker_bot",
            Self::ShipNoseUplink => "ship_nose",
            Self::ShipDorsalUplink => "ship_dorsal",
            Self::AutoRotateAll => "auto_all",
            Self::AutoRotateInterior => "auto_interior",
            Self::AutoRotateExterior => "auto_exterior",
            Self::FollowBotRoom => "follow_bot",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "drone_alpha" | "cam_01" => Some(Self::DroneAlphaWide),
            "drone_beta" | "cam_02" => Some(Self::DroneBetaPatrol),
            "hangar_approach" | "cam_03" => Some(Self::ExtHangarApproach),
            "pad_a" | "cam_04" => Some(Self::PadAHangarInterior),
            "pad_b" | "cam_05" => Some(Self::PadBPortDeck),
            "pad_c" | "cam_06" => Some(Self::PadCStarboardDeck),
            "atrium" | "cam_07" => Some(Self::IntCentralAtrium),
            "ops_deck" | "cam_08" => Some(Self::IntOpsDeck),
            "reactor" | "concourse" | "cam_09" => Some(Self::IntReactorCore),
            "solar_bay" | "cam_10" | "cam_11" => Some(Self::IntSolarBay),
            "walker_bot" | "bot_01" => Some(Self::WalkerBotCam),
            "ship_nose" | "uplink_1" => Some(Self::ShipNoseUplink),
            "ship_dorsal" | "uplink_2" => Some(Self::ShipDorsalUplink),
            "auto_all" => Some(Self::AutoRotateAll),
            "auto_interior" => Some(Self::AutoRotateInterior),
            "auto_exterior" => Some(Self::AutoRotateExterior),
            "follow_bot" => Some(Self::FollowBotRoom),
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

    pub fn slug(self) -> &'static str {
        match self {
            Self::UnatcoCobalt => "unatco",
            Self::VonBraunIndustrial => "von_braun",
            Self::VersaLifeGold => "versalife",
            Self::ShodanCyber => "shodan",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "unatco" | "unatco_cobalt" => Some(Self::UnatcoCobalt),
            "von_braun" | "von_braun_industrial" => Some(Self::VonBraunIndustrial),
            "versalife" | "versalife_gold" => Some(Self::VersaLifeGold),
            "shodan" | "shodan_cyber" => Some(Self::ShodanCyber),
            _ => None,
        }
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
    /// `[0.0, 1.0]` main aft engine thruster glow intensity.
    pub thruster_power: f32,
    /// `[0.0, 1.0]` downward belly VTOL hover thruster intensity.
    #[serde(default)]
    pub vtol_thrust: f32,
    /// `[0.0, 1.0]` forward retro/RCS braking & reverse-out thruster intensity.
    #[serde(default)]
    pub retro_thrust: f32,
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

/// Class of autonomous rolling robot operating inside or on Station HELIOS-09.
/// Every interior & dock robot rolls on wheels or caterpillar treads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// Tracked yellow/hazard forklift hauling crates in the Hangar Bay & external pads.
    #[serde(alias = "cargo_loader")]
    CargoForklift,
    /// 3-wheel UNATCO-style compact floor scrubber with spinning front brush roller.
    FloorScrubber,
    /// 4-wheel Ops Deck telemetry droid with 3 glowing CRT displays & spinning micro-dish.
    #[serde(alias = "ops_crew")]
    OpsHoloDroid,
    /// Heavy twin-tread armored security bot with sweeping crimson optic turret.
    #[serde(alias = "security_droid")]
    SecurityEnforcer,
    /// 4-wheel yellow/steel cryo-tanker rover servicing the pulsing plasma reactor core.
    ReactorCoolantBot,
    /// Narrow tracked high-voltage maintenance rover calibrating Solar Bay capacitor towers.
    SolarCapacitorBot,
    /// Fast 4-wheel angular wedge courier zipping between Hangar, Atrium, Pipe Tunnel & Solar Bay.
    CourierRunner,
    /// Exterior hull welder drone cruising along station trusses with flickering welding arcs.
    HullWelder,
}

impl AgentKind {
    /// Physical collision radius in 3D world units.
    pub fn radius(self) -> f32 {
        match self {
            Self::CargoForklift => 1.05,
            Self::SecurityEnforcer => 0.95,
            Self::ReactorCoolantBot => 0.90,
            Self::OpsHoloDroid => 0.80,
            Self::SolarCapacitorBot => 0.80,
            Self::FloorScrubber => 0.72,
            Self::CourierRunner => 0.68,
            Self::HullWelder => 0.85,
        }
    }

    /// Cruise speed in world units/second.
    pub fn cruise_speed(self) -> f32 {
        match self {
            Self::CourierRunner => 4.8,
            Self::FloorScrubber => 2.6,
            Self::OpsHoloDroid => 2.4,
            Self::SecurityEnforcer => 2.5,
            Self::ReactorCoolantBot => 2.2,
            Self::SolarCapacitorBot => 2.3,
            Self::CargoForklift => 2.4,
            Self::HullWelder => 2.2,
        }
    }
}

/// Zone where a station robot operates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentZone {
    HangarBay,
    CentralAtrium,
    AxialPipeTunnel,
    SolarBay,
    PadBPort,
    PadCStarboard,
    OpsDeck,
    Concourse,
    ReactorCatwalk,
    ExteriorTruss,
}

/// An autonomous rolling robot bringing interiors and docks to life.
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
    /// Whether a CargoForklift or CourierRunner is currently carrying cargo.
    #[serde(default)]
    pub carrying_cargo: bool,
    /// `[0.0, 1.0]` normalized hydraulic fork lift or probe extension height.
    #[serde(default)]
    pub lift_height: f32,
    /// Pause timer (seconds) when performing a room activity at a waypoint.
    #[serde(default)]
    pub dwell_timer: f32,
    /// Current `NAV_NODES` index occupied or last visited.
    #[serde(default)]
    pub nav_node: usize,
    /// Active A* path queue of upcoming `NAV_NODES` indices.
    #[serde(default)]
    pub nav_path: Vec<usize>,
    /// True while smoothly yielding for another robot or waiting for the Pipe Tunnel mutex.
    #[serde(default)]
    pub yielding: bool,
    /// Seconds spent waiting/blocked; triggers dynamic A* rerouting to prevent deadlocks.
    #[serde(default)]
    pub stuck_timer: f32,
}

/// Station deck levels served by the 3-deck central glass elevator (`X = 0, Z = -6.0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElevatorDeck {
    /// Deck 1 (`Y = -14.95`): Concourse Corridor & Reactor Core.
    Deck1Reactor,
    /// Deck 2 (`Y = -3.85`): Main Hangar Bay, Central Atrium, Axial Pipe Tunnel & Solar Bay.
    #[default]
    Deck2Atrium,
    /// Deck 3 (`Y = +12.05`): Ops & Command Spire.
    Deck3Ops,
}

impl ElevatorDeck {
    pub fn floor_y(self) -> f32 {
        match self {
            Self::Deck1Reactor => -14.95,
            Self::Deck2Atrium => -3.85,
            Self::Deck3Ops => 12.05,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Deck1Reactor => "DECK-1 REACTOR",
            Self::Deck2Atrium => "DECK-2 ATRIUM",
            Self::Deck3Ops => "DECK-3 OPS",
        }
    }
}

/// Functioning 3-deck central elevator state (`X = 0, Z = -6.0`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevatorState {
    pub current_y: f32,
    pub target_deck: ElevatorDeck,
    /// `[0.0, 1.0]` animated sliding glass/steel elevator door opening.
    pub door_open: f32,
    pub moving: bool,
}

impl Default for ElevatorState {
    fn default() -> Self {
        Self {
            current_y: ElevatorDeck::Deck2Atrium.floor_y(),
            target_deck: ElevatorDeck::Deck2Atrium,
            door_open: 0.0,
            moving: false,
        }
    }
}

/// Physical 3D Interior Walker Camera Bot (`BOT-01`) patrolling all interconnected decks & tunnels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkerBotState {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub walk_phase: f32,
    pub route_index: usize,
    pub dwell_timer: f32,
    pub riding_elevator: bool,
    pub current_room: String,
    /// Elapsed seconds on the current walk leg; prevents BOT-01 from ever wedging indefinitely.
    #[serde(default)]
    pub leg_timer: f32,
}

impl Default for WalkerBotState {
    fn default() -> Self {
        Self {
            pos: [0.0, -3.85, 22.0],
            yaw: std::f32::consts::PI,
            pitch: 0.0,
            walk_phase: 0.0,
            route_index: 0,
            dwell_timer: 1.5,
            riding_elevator: false,
            current_room: "MAIN HANGAR BAY".into(),
            leg_timer: 0.0,
        }
    }
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
            pip_feeds: [CameraFeedId::WalkerBotCam, CameraFeedId::ShipNoseUplink],
            grid_feeds: [
                CameraFeedId::DroneAlphaWide,
                CameraFeedId::WalkerBotCam,
                CameraFeedId::PadAHangarInterior,
                CameraFeedId::IntCentralAtrium,
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
