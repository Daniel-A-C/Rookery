//! Orbital Haven v3 Simulation State (Station HELIOS-09).
//!
//! An always-operational, living 3D orbital station styled after late-90s / Y2K
//! immersive sims (Deus Ex / System Shock 2), observed through a multi-camera
//! CCTV and live spacecraft uplink network.

use std::f32::consts::PI;

use serde::{Deserialize, Serialize};

use crate::world::WorldRng;

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
/// Hover alignment gate above Port Pad B.
pub const PAD_B_GATE: [f32; 3] = [-38.0, 16.0, 18.0];

/// External cantilevered Starboard Utility Deck (Pad C).
pub const PAD_C_POS: [f32; 3] = [38.0, -1.2, 0.0];
/// Hover alignment gate above Starboard Pad C.
pub const PAD_C_GATE: [f32; 3] = [38.0, 16.0, 18.0];

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
                thruster_power: 0.75,
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
                thruster_power: 0.08,
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
                thruster_power: 0.9,
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
                patrol_angle: 0.4,
                patrol_radius: 64.0,
            },
        ];
        let initial_uplink_id = ships[0].id;
        next_id += 4;

        // 3. Autonomous bots & interior crew bringing docks and rooms to life
        let mut agents = Vec::new();
        let mut push_agent = |kind: AgentKind,
                              zone: AgentZone,
                              pos: [f32; 3],
                              patrol_t: f32,
                              speed: f32,
                              direction: f32| {
            let id = next_id;
            next_id += 1;
            agents.push(StationAgent {
                id,
                kind,
                zone,
                pos,
                yaw: 0.0,
                patrol_t,
                speed,
                direction,
                phase: rng.range(0.0, PI * 2.0),
            });
        };

        // Hangar Bay cargo loader bots (2)
        push_agent(
            AgentKind::CargoLoader,
            AgentZone::HangarBay,
            [-7.5, -3.5, 22.0],
            0.25,
            0.14,
            1.0,
        );
        push_agent(
            AgentKind::CargoLoader,
            AgentZone::HangarBay,
            [7.5, -3.5, 31.0],
            0.70,
            0.12,
            -1.0,
        );
        // External Pad B & Pad C cargo loaders
        push_agent(
            AgentKind::CargoLoader,
            AgentZone::PadBPort,
            [-33.0, -1.5, 5.5],
            0.4,
            0.16,
            1.0,
        );
        push_agent(
            AgentKind::CargoLoader,
            AgentZone::PadCStarboard,
            [33.0, -1.5, -5.0],
            0.6,
            0.15,
            -1.0,
        );
        // Ops Command Deck crew & floating monitor droid
        push_agent(
            AgentKind::OpsCrew,
            AgentZone::OpsDeck,
            [4.8, 12.0, 3.2],
            0.15,
            0.09,
            1.0,
        );
        push_agent(
            AgentKind::OpsCrew,
            AgentZone::OpsDeck,
            [-4.8, 12.0, -2.5],
            0.62,
            0.08,
            -1.0,
        );
        // Concourse Corridor security droids & crew
        push_agent(
            AgentKind::SecurityDroid,
            AgentZone::Concourse,
            [-8.0, -14.8, 5.4],
            0.22,
            0.11,
            1.0,
        );
        push_agent(
            AgentKind::OpsCrew,
            AgentZone::Concourse,
            [6.0, -14.8, -5.2],
            0.78,
            0.09,
            -1.0,
        );
        // Reactor Catwalk security droid
        push_agent(
            AgentKind::SecurityDroid,
            AgentZone::ReactorCatwalk,
            [5.8, -14.8, 0.0],
            0.3,
            0.14,
            1.0,
        );
        // Exterior truss welder drones (2)
        push_agent(
            AgentKind::HullWelder,
            AgentZone::ExteriorTruss,
            [-22.0, 5.0, 12.0],
            0.1,
            0.12,
            1.0,
        );
        push_agent(
            AgentKind::HullWelder,
            AgentZone::ExteriorTruss,
            [22.0, -4.0, -12.0],
            0.55,
            0.11,
            -1.0,
        );

        let camera_rig = CameraRigState {
            uplink_ship_id: Some(initial_uplink_id),
            ..Default::default()
        };

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
            next_id,
            stats: OrbitalHavenStats::default(),
        };
        state.recompute_telemetry();
        state
    }
}
