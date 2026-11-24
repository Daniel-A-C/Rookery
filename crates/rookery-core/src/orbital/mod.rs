//! Orbital Haven v3 Simulation State (Station HELIOS-09).
//!
//! An always-operational, living 3D orbital station styled after late-90s / Y2K
//! immersive sims (Deus Ex / System Shock 2), observed through a multi-camera
//! CCTV and live spacecraft uplink network.

use std::f32::consts::PI;

use serde::{Deserialize, Serialize};

use crate::world::WorldRng;

pub mod nav;
pub mod persistence;
pub mod simulation;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::*;

/// Unified 3D world coordinate anchors for Station HELIOS-09 (right-handed Y-up).
pub const STATION_ORIGIN: [f32; 3] = [0.0, 0.0, 0.0];

/// Main Hangar Bay interior landing pad (Pad A), inside the forward hull (`Z = +12..+42`).
pub const PAD_A_POS: [f32; 3] = [0.0, -3.2, 26.0];
/// Approach gate just outside the Hangar Bay atmospheric containment forcefield (`Z = +42`).
pub const PAD_A_GATE: [f32; 3] = [0.0, 1.4, 58.0];

/// External cantilevered Port Freight Deck (Pad B).
pub const PAD_B_POS: [f32; 3] = [-38.0, -1.2, 0.0];
/// Hover alignment gate directly above Port Pad B (for vertical VTOL descent/ascent).
pub const PAD_B_GATE: [f32; 3] = [-38.0, 14.5, 0.0];

/// External cantilevered Starboard Utility Deck (Pad C).
pub const PAD_C_POS: [f32; 3] = [38.0, -1.2, 0.0];
/// Hover alignment gate directly above Starboard Pad C (for vertical VTOL descent/ascent).
pub const PAD_C_GATE: [f32; 3] = [38.0, 14.5, 0.0];

/// Deterministic 3D simulation state for *Orbital Haven v3*.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitalHavenState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub total_age_secs: f64,
    #[serde(default)]
    pub total_offline_caught_up_secs: f64,
    /// Continuous orbital phase driving planetary terminator & eclipse timing.
    pub orbit_angle: f32,
    /// Rotating central counterweight habitation ring angle (radians).
    pub hab_ring_angle: f32,
    /// Dorsal radar dish rotation angle (radians).
    pub radar_angle: f32,
    /// Active Y2K visual color & lighting palette.
    pub palette: OrbitalPalette,
    /// Active station lighting/operational event.
    pub event: StationEventKind,
    /// Countdown timer for the current station event before cycling naturally.
    pub event_timer: f32,
    /// Whether the diegetic CCTV HUD text overlay is enabled.
    pub hud_enabled: bool,
    /// Multi-camera surveillance rig, layout mode, and auto-director.
    pub camera_rig: CameraRigState,
    /// Physical 3D camera drones orbiting the station.
    pub drones: Vec<CameraDrone>,
    /// Active 3D spacecraft fleet in the sector.
    pub ships: Vec<Spacecraft>,
    /// Autonomous bots and crew operating inside interiors and on landing pads.
    pub agents: Vec<StationAgent>,
    /// Functioning 3-deck central glass elevator (`X = 0, Z = -6.0`).
    #[serde(default)]
    pub elevator: ElevatorState,
    /// Patrolling Interior Walker Camera Bot (`BOT-01`).
    #[serde(default)]
    pub walker: WalkerBotState,
    /// `[0.0, 1.0]` sliding blast door openings: `[0]` Hangar↔Atrium, `[1]` Atrium↔Pipe, `[2]` Pipe↔Solar.
    #[serde(default)]
    pub door_open: [f32; 3],
    /// Entity ID (`0` = BOT-01 Walker Rover, `> 0` = agent ID) holding single-lane Pipe Tunnel mutex.
    #[serde(default)]
    pub pipe_tunnel_owner: Option<u32>,
    /// Active docked-ship service choreographies for Pad A (`[0]`), Pad B (`[1]`), and Pad C (`[2]`).
    #[serde(default)]
    pub pad_services: [PadServiceState; 3],
    /// Monotonic entity ID counter.
    pub next_id: u32,
    /// Cached operational telemetry.
    pub stats: OrbitalHavenStats,
}

impl OrbitalHavenState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut next_id = 1u32;

        // 1. Two physical orbiting 3D camera drones (visible from other feeds & windows!)
        let drones = vec![
            CameraDrone {
                id: next_id,
                feed: CameraFeedId::DroneAlphaWide,
                label: "DRONE-ALPHA".into(),
                pos: [58.0, 26.0, 58.0],
                look_target: [0.0, 3.0, 8.0],
                orbit_angle: 0.72,
                orbit_radius: 82.0,
                orbit_speed: 0.055,
                height_base: 26.0,
                height_amp: 9.0,
            },
            CameraDrone {
                id: next_id + 1,
                feed: CameraFeedId::DroneBetaPatrol,
                label: "DRONE-BETA".into(),
                pos: [-34.0, 8.0, 36.0],
                look_target: [0.0, 1.0, 16.0],
                orbit_angle: 2.35,
                orbit_radius: 50.0,
                orbit_speed: -0.095,
                height_base: 6.0,
                height_amp: 11.0,
            },
        ];
        next_id += 2;

        // 2. Initial bustling spacecraft fleet across different flight phases
        let ships = vec![
            // Corporate Shuttle on final approach right into Hangar Pad A (immediate Ship Nose-Cam action!)
            Spacecraft {
                id: next_id,
                class: ShipClass::CorporateShuttle,
                callsign: "USV-ICARUS".into(),
                assigned_pad: LandingPadId::PadAHangar,
                phase: ShipFlightPhase::FinalDescent,
                pos: [0.0, 0.8, 52.0],
                vel: [0.0, -0.3, -2.5],
                forward: [0.0, -0.1, -1.0],
                up: [0.0, 1.0, 0.0],
                leg_start: PAD_A_GATE,
                leg_end: PAD_A_POS,
                leg_progress: 0.18,
                leg_duration: 15.0,
                gear_deploy: 0.3,
                thruster_power: 0.25,
                vtol_thrust: 0.55,
                retro_thrust: 0.45,
                patrol_angle: 0.0,
                patrol_radius: 0.0,
            },
            // Heavy Bulk Freighter docked on Port Pad B being serviced by cargo loaders
            Spacecraft {
                id: next_id + 1,
                class: ShipClass::BulkFreighter,
                callsign: "TRN-VONBRAUN".into(),
                assigned_pad: LandingPadId::PadBPort,
                phase: ShipFlightPhase::Docked,
                pos: PAD_B_POS,
                vel: [0.0, 0.0, 0.0],
                forward: [0.0, 0.0, 1.0],
                up: [0.0, 1.0, 0.0],
                leg_start: PAD_B_POS,
                leg_end: PAD_B_POS,
                leg_progress: 0.35,
                leg_duration: 24.0,
                gear_deploy: 1.0,
                thruster_power: 0.0,
                vtol_thrust: 0.0,
                retro_thrust: 0.0,
                patrol_angle: 0.0,
                patrol_radius: 0.0,
            },
            // Maintenance Tug inbound toward Starboard Pad C
            Spacecraft {
                id: next_id + 2,
                class: ShipClass::MaintenanceTug,
                callsign: "TUG-VERSA-4".into(),
                assigned_pad: LandingPadId::PadCStarboard,
                phase: ShipFlightPhase::InboundApproach,
                pos: [82.0, 28.0, 68.0],
                vel: [-2.0, -0.6, -2.2],
                forward: [-0.65, -0.2, -0.73],
                up: [0.0, 1.0, 0.0],
                leg_start: [96.0, 32.0, 84.0],
                leg_end: PAD_C_GATE,
                leg_progress: 0.25,
                leg_duration: 18.0,
                gear_deploy: 0.0,
                thruster_power: 0.85,
                vtol_thrust: 0.0,
                retro_thrust: 0.0,
                patrol_angle: 0.0,
                patrol_radius: 0.0,
            },
            // Swept-wing Patrol Cutter sweeping around the station perimeter
            Spacecraft {
                id: next_id + 3,
                class: ShipClass::PatrolCutter,
                callsign: "SEC-DAEDALUS".into(),
                assigned_pad: LandingPadId::PadCStarboard,
                phase: ShipFlightPhase::PerimeterPatrol,
                pos: [64.0, -6.0, 0.0],
                vel: [0.0, 0.0, 6.0],
                forward: [0.0, 0.0, 1.0],
                up: [0.0, 1.0, 0.0],
                leg_start: [0.0, 0.0, 0.0],
                leg_end: [0.0, 0.0, 0.0],
                leg_progress: 0.0,
                leg_duration: 1.0,
                gear_deploy: 0.0,
                thruster_power: 0.65,
                vtol_thrust: 0.0,
                retro_thrust: 0.0,
                patrol_angle: 0.4,
                patrol_radius: 64.0,
            },
        ];
        let initial_uplink_id = ships[0].id;
        next_id += 4;

        // 3. Unique rolling robots for every room + cross-room courier rover (100% wheels & treads!)
        let mut agents = Vec::new();
        let mut spawn_nav_bot = |kind: AgentKind, zone: AgentZone, start_node: usize| {
            let id = next_id;
            next_id += 1;
            let pos = nav::NAV_NODES[start_node].pos;
            agents.push(StationAgent {
                id,
                kind,
                zone,
                pos,
                yaw: 0.0,
                patrol_t: 0.0,
                speed: kind.cruise_speed(),
                direction: 1.0,
                phase: rng.range(0.0, PI * 2.0),
                carrying_cargo: matches!(kind, AgentKind::CargoForklift | AgentKind::CourierRunner)
                    && id.is_multiple_of(2),
                lift_height: if kind == AgentKind::CargoForklift && id.is_multiple_of(2) {
                    0.35
                } else {
                    0.06
                },
                dwell_timer: rng.range(0.2, 1.4),
                nav_node: start_node,
                nav_path: Vec::new(),
                yielding: false,
                stuck_timer: 0.0,
            });
        };

        // Main Hangar Bay: 2 tracked CargoForklifts + 1 wheeled FloorScrubber
        spawn_nav_bot(AgentKind::CargoForklift, AgentZone::HangarBay, 0);
        spawn_nav_bot(AgentKind::CargoForklift, AgentZone::HangarBay, 5);
        spawn_nav_bot(AgentKind::FloorScrubber, AgentZone::HangarBay, 7);

        // Central Core Atrium: 1 wheeled FloorScrubber + 1 tracked SecurityEnforcer
        spawn_nav_bot(AgentKind::FloorScrubber, AgentZone::CentralAtrium, 11);
        spawn_nav_bot(AgentKind::SecurityEnforcer, AgentZone::CentralAtrium, 14);

        // Cross-Room Data Courier Runner (Hangar <-> Atrium <-> Pipe Tunnel <-> Solar Bay!)
        spawn_nav_bot(AgentKind::CourierRunner, AgentZone::CentralAtrium, 12);

        // Aft Solar Engineering Bay: 2 narrow-tracked SolarCapacitorBots
        spawn_nav_bot(AgentKind::SolarCapacitorBot, AgentZone::SolarBay, 21);
        spawn_nav_bot(AgentKind::SolarCapacitorBot, AgentZone::SolarBay, 23);

        // Ops Command Spire (Deck 3): 2 wheeled OpsHoloDroids
        spawn_nav_bot(AgentKind::OpsHoloDroid, AgentZone::OpsDeck, 24);
        spawn_nav_bot(AgentKind::OpsHoloDroid, AgentZone::OpsDeck, 27);

        // Concourse & Reactor Core (Deck 1): 1 wheeled ReactorCoolantBot + 2 tracked SecurityEnforcers
        spawn_nav_bot(AgentKind::ReactorCoolantBot, AgentZone::ReactorCatwalk, 33);
        spawn_nav_bot(AgentKind::SecurityEnforcer, AgentZone::Concourse, 37);
        spawn_nav_bot(AgentKind::SecurityEnforcer, AgentZone::Concourse, 38);

        // External Landing Pads B & C: 1 tracked CargoForklift per pad
        spawn_nav_bot(AgentKind::CargoForklift, AgentZone::PadBPort, 41);
        spawn_nav_bot(AgentKind::CargoForklift, AgentZone::PadCStarboard, 44);

        // Exterior Hull Welder Drones (2)
        for (i, pos) in [[-22.0, 5.0, 12.0], [22.0, -4.0, -12.0]]
            .into_iter()
            .enumerate()
        {
            let id = next_id;
            next_id += 1;
            agents.push(StationAgent {
                id,
                kind: AgentKind::HullWelder,
                zone: AgentZone::ExteriorTruss,
                pos,
                yaw: 0.0,
                patrol_t: if i == 0 { 0.1 } else { 0.55 },
                speed: 0.12,
                direction: if i == 0 { 1.0 } else { -1.0 },
                phase: rng.range(0.0, PI * 2.0),
                carrying_cargo: false,
                lift_height: 0.0,
                dwell_timer: 0.0,
                nav_node: 0,
                nav_path: Vec::new(),
                yielding: false,
                stuck_timer: 0.0,
            });
        }

        let camera_rig = CameraRigState {
            uplink_ship_id: Some(initial_uplink_id),
            ..Default::default()
        };

        let pad_b_ship_id = ships[1].id;
        let pad_b_ship_class = ships[1].class;

        let mut state = Self {
            version: ORBITAL_HAVEN_STATE_VERSION,
            seed,
            rng,
            total_age_secs: 0.0,
            total_offline_caught_up_secs: 0.0,
            orbit_angle: 0.42,
            hab_ring_angle: 0.0,
            radar_angle: 0.0,
            palette: OrbitalPalette::UnatcoCobalt,
            event: StationEventKind::DockingOps,
            event_timer: 45.0,
            hud_enabled: true,
            camera_rig,
            drones,
            ships,
            agents,
            elevator: ElevatorState::default(),
            walker: WalkerBotState::default(),
            door_open: [0.0; 3],
            pipe_tunnel_owner: None,
            pad_services: [
                PadServiceState::default(),
                PadServiceState::default(),
                PadServiceState::default(),
            ],
            next_id,
            stats: OrbitalHavenStats::default(),
        };
        state.begin_pad_service(LandingPadId::PadBPort, pad_b_ship_id, pad_b_ship_class);
        state.recompute_telemetry();
        state
    }
}
