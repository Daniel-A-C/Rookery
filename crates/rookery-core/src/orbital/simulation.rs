//! Real-time 3D simulation step, ATC flight trajectories, autonomous agents,
//! and multi-camera director logic for *Orbital Haven v3*.

use std::f32::consts::TAU;

use super::types::*;
use super::{
    OrbitalHavenState, PAD_A_GATE, PAD_A_POS, PAD_B_GATE, PAD_B_POS, PAD_C_GATE, PAD_C_POS,
};

const DIRECTOR_INTERVAL_SECS: f32 = 24.0;
const STATIC_GLITCH_DURATION: f32 = 0.18;

const CALLSIGN_PREFIXES: [&str; 6] = ["USV", "TRN", "SEC", "TUG", "CORP", "VBN"];
const CALLSIGN_NAMES: [&str; 10] = [
    "ICARUS",
    "DAEDALUS",
    "VONBRAUN",
    "HELIOS",
    "VERSA-4",
    "UNATCO-7",
    "CITADEL",
    "SHODAN",
    "MANDERLEY",
    "PAGE-9",
];

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn len3(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn norm3(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let l = len3(v);
    if l > 1e-4 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        fallback
    }
}

fn smoothstep(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

pub fn pad_touchdown_pos(pad: LandingPadId) -> [f32; 3] {
    match pad {
        LandingPadId::PadAHangar => PAD_A_POS,
        LandingPadId::PadBPort => PAD_B_POS,
        LandingPadId::PadCStarboard => PAD_C_POS,
    }
}

pub fn pad_gate_pos(pad: LandingPadId) -> [f32; 3] {
    match pad {
        LandingPadId::PadAHangar => PAD_A_GATE,
        LandingPadId::PadBPort => PAD_B_GATE,
        LandingPadId::PadCStarboard => PAD_C_GATE,
    }
}

impl OrbitalHavenState {
    /// Advance the 3D station simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.25);
        if dt <= 0.0 {
            return;
        }

        self.total_age_secs += dt as f64;
        self.orbit_angle = (self.orbit_angle + dt * 0.0085) % TAU;
        self.hab_ring_angle = (self.hab_ring_angle + dt * 0.14) % TAU;
        self.radar_angle = (self.radar_angle + dt * 0.72) % TAU;

        // 1. Update physical orbiting camera drones
        self.step_drones(dt);

        // 2. Update 3D spacecraft fleet & ATC docking cycles
        self.step_ships(dt);

        // 3. Update interior crew, security droids, cargo loaders & exterior welders
        self.step_agents(dt);

        // 4. Update station event cycle
        self.step_events(dt);

        // 5. Update camera director, static glitch decay, and manual pan ease-back
        self.step_camera_rig(dt);

        // 6. Refresh live telemetry
        self.recompute_telemetry();
    }

    /// Fast-forward simulation clocks after system sleep/wake.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let clamped = elapsed_secs.clamp(0.0, 3600.0 * 24.0);
        if clamped < 1.0 {
            return;
        }
        self.total_offline_caught_up_secs += clamped;
        self.total_age_secs += clamped;
        self.orbit_angle = (self.orbit_angle + (clamped as f32) * 0.0085) % TAU;
        self.hab_ring_angle = (self.hab_ring_angle + (clamped as f32) * 0.14) % TAU;
        // Run a handful of bounded steps so ships/drones settle cleanly
        let ticks = ((clamped / 2.0).min(24.0)) as usize;
        for _ in 0..ticks {
            self.step(0.2);
        }
    }

    fn step_drones(&mut self, dt: f32) {
        for drone in &mut self.drones {
            drone.orbit_angle = (drone.orbit_angle + drone.orbit_speed * dt).rem_euclid(TAU);
            let x = drone.orbit_angle.cos() * drone.orbit_radius;
            let z = drone.orbit_angle.sin() * drone.orbit_radius;
            let y = drone.height_base + (drone.orbit_angle * 1.5).sin() * drone.height_amp;
            drone.pos = [x, y, z];

            // Subtle breathing look target near the station core / hangar mouth
            let sway = (drone.orbit_angle * 2.0).sin() * 4.0;
            drone.look_target = match drone.feed {
                CameraFeedId::DroneAlphaWide => [0.0, 2.5, 6.0 + sway * 0.5],
                _ => [sway * 0.8, 0.5, 16.0],
            };
        }
    }

    fn step_ships(&mut self, dt: f32) {
        let mut serviced_delta = 0u32;
        let mut need_replacement_pads: Vec<LandingPadId> = Vec::new();
        let mut remove_ids: Vec<u32> = Vec::new();

        for ship in &mut self.ships {
            match ship.phase {
                ShipFlightPhase::PerimeterPatrol => {
                    ship.patrol_angle = (ship.patrol_angle + dt * 0.16) % TAU;
                    let r = ship.patrol_radius;
                    let x = ship.patrol_angle.cos() * r;
                    let z = ship.patrol_angle.sin() * r;
                    let y = -7.5 + (ship.patrol_angle * 2.0).sin() * 14.0;
                    let next_pos = [x, y, z];
                    let dir = norm3(sub3(next_pos, ship.pos), ship.forward);
                    ship.vel = [dir[0] * 10.0, dir[1] * 10.0, dir[2] * 10.0];
                    ship.pos = next_pos;
                    ship.forward = dir;
                    ship.up = [0.0, 1.0, 0.0];
                    ship.thruster_power = 0.65;
                    ship.gear_deploy = 0.0;
                }
                ShipFlightPhase::InboundApproach => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;
                    ship.pos = lerp3(ship.leg_start, ship.leg_end, t);
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];
                    ship.forward = norm3(sub3(ship.leg_end, ship.pos), ship.forward);
                    ship.thruster_power = 0.85 - 0.35 * t;
                    ship.gear_deploy = (t - 0.65).max(0.0) / 0.35 * 0.3;

                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::FinalDescent;
                        ship.leg_start = ship.pos;
                        ship.leg_end = pad_touchdown_pos(ship.assigned_pad);
                        ship.leg_progress = 0.0;
                        ship.leg_duration = match ship.assigned_pad {
                            LandingPadId::PadAHangar => 14.0,
                            _ => 12.0,
                        };
                    }
                }
                ShipFlightPhase::FinalDescent => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;
                    // Smooth arc onto the pad
                    let mut p = lerp3(ship.leg_start, ship.leg_end, t);
                    if ship.assigned_pad == LandingPadId::PadAHangar {
                        // Glide straight in through the forcefield, then settle down onto Pad A
                        p[1] = ship.leg_start[1] * (1.0 - t * t) + ship.leg_end[1] * (t * t);
                    }
                    ship.pos = p;
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];
                    let desired_fwd = match ship.assigned_pad {
                        LandingPadId::PadAHangar => [0.0, 0.0, -1.0],
                        _ => [0.0, 0.0, 1.0],
                    };
                    let flight_dir = norm3(sub3(ship.leg_end, ship.pos), desired_fwd);
                    ship.forward = norm3(lerp3(flight_dir, desired_fwd, t), desired_fwd);
                    ship.gear_deploy = (0.3 + t * 0.7).clamp(0.0, 1.0);
                    ship.thruster_power = (0.75 * (1.0 - t) + 0.15).clamp(0.1, 0.9);

                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::Docked;
                        ship.pos = ship.leg_end;
                        ship.vel = [0.0, 0.0, 0.0];
                        ship.forward = desired_fwd;
                        ship.gear_deploy = 1.0;
                        ship.thruster_power = 0.06;
                        ship.leg_progress = 0.0;
                        ship.leg_duration = 20.0;
                    }
                }
                ShipFlightPhase::Docked => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    ship.gear_deploy = 1.0;
                    ship.thruster_power = if ship.leg_progress > 0.85 { 0.45 } else { 0.06 };
                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::Liftoff;
                        ship.leg_start = ship.pos;
                        ship.leg_end = pad_gate_pos(ship.assigned_pad);
                        ship.leg_progress = 0.0;
                        ship.leg_duration = 11.0;
                    }
                }
                ShipFlightPhase::Liftoff => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;
                    ship.pos = lerp3(ship.leg_start, ship.leg_end, t);
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];
                    ship.forward = norm3(sub3(ship.leg_end, ship.pos), [0.0, 0.0, 1.0]);
                    ship.gear_deploy = (1.0 - t * 1.4).clamp(0.0, 1.0);
                    ship.thruster_power = 0.6 + 0.35 * t;

                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::OutboundBurn;
                        ship.leg_start = ship.pos;
                        let exit_x = match ship.assigned_pad {
                            LandingPadId::PadBPort => -135.0,
                            LandingPadId::PadCStarboard => 135.0,
                            LandingPadId::PadAHangar => 28.0,
                        };
                        ship.leg_end = [exit_x, 34.0, 145.0];
                        ship.leg_progress = 0.0;
                        ship.leg_duration = 14.0;
                    }
                }
                ShipFlightPhase::OutboundBurn => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;
                    ship.pos = lerp3(ship.leg_start, ship.leg_end, t);
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];
                    ship.forward = norm3(sub3(ship.leg_end, ship.pos), ship.forward);
                    ship.gear_deploy = 0.0;
                    ship.thruster_power = 1.0;

                    if ship.leg_progress >= 1.0 {
                        serviced_delta += 1;
                        need_replacement_pads.push(ship.assigned_pad);
                        remove_ids.push(ship.id);
                    }
                }
            }
        }

        if !remove_ids.is_empty() {
            self.ships.retain(|s| !remove_ids.contains(&s.id));
            self.stats.ships_serviced += serviced_delta;
        }

        for pad in need_replacement_pads {
            self.spawn_inbound_ship_for_pad(pad);
        }
    }

    fn spawn_inbound_ship_for_pad(&mut self, pad: LandingPadId) {
        let class = match pad {
            LandingPadId::PadAHangar => ShipClass::CorporateShuttle,
            LandingPadId::PadBPort => {
                if self.rng.range(0.0, 1.0) < 0.7 {
                    ShipClass::BulkFreighter
                } else {
                    ShipClass::MaintenanceTug
                }
            }
            LandingPadId::PadCStarboard => {
                if self.rng.range(0.0, 1.0) < 0.5 {
                    ShipClass::MaintenanceTug
                } else {
                    ShipClass::BulkFreighter
                }
            }
        };

        let prefix = CALLSIGN_PREFIXES[(self.rng.next_u64() as usize) % CALLSIGN_PREFIXES.len()];
        let name = CALLSIGN_NAMES[(self.rng.next_u64() as usize) % CALLSIGN_NAMES.len()];
        let callsign = format!("{prefix}-{name}");

        let start = match pad {
            LandingPadId::PadAHangar => [
                self.rng.range(-48.0, 48.0),
                self.rng.range(10.0, 28.0),
                135.0,
            ],
            LandingPadId::PadBPort => [
                -115.0,
                self.rng.range(24.0, 42.0),
                self.rng.range(55.0, 95.0),
            ],
            LandingPadId::PadCStarboard => [
                115.0,
                self.rng.range(24.0, 42.0),
                self.rng.range(55.0, 95.0),
            ],
        };
        let gate = pad_gate_pos(pad);
        let fwd = norm3(sub3(gate, start), [0.0, 0.0, -1.0]);

        let id = self.next_id;
        self.next_id += 1;

        self.ships.push(Spacecraft {
            id,
            class,
            callsign,
            assigned_pad: pad,
            phase: ShipFlightPhase::InboundApproach,
            pos: start,
            vel: [fwd[0] * 4.0, fwd[1] * 4.0, fwd[2] * 4.0],
            forward: fwd,
            up: [0.0, 1.0, 0.0],
            leg_start: start,
            leg_end: gate,
            leg_progress: 0.0,
            leg_duration: self.rng.range(14.0, 19.0),
            gear_deploy: 0.0,
            thruster_power: 0.85,
            patrol_angle: 0.0,
            patrol_radius: 0.0,
        });
    }

    fn step_agents(&mut self, dt: f32) {
        let pad_a_docked = self.ships.iter().any(|s| {
            s.assigned_pad == LandingPadId::PadAHangar && s.phase == ShipFlightPhase::Docked
        });

        for agent in &mut self.agents {
            agent.phase = (agent.phase + dt * 3.5) % TAU;
            agent.patrol_t += dt * agent.speed * agent.direction;
            if agent.patrol_t > 1.0 {
                agent.patrol_t = 1.0;
                agent.direction = -1.0;
            } else if agent.patrol_t < 0.0 {
                agent.patrol_t = 0.0;
                agent.direction = 1.0;
            }

            let t = agent.patrol_t;
            match agent.zone {
                AgentZone::HangarBay => {
                    let lane_x = if agent.id % 2 == 0 { -8.0 } else { 8.0 };
                    let target_x = if pad_a_docked { lane_x * 0.62 } else { lane_x };
                    let z = 17.5 + t * 17.5;
                    agent.pos = [target_x, -3.5, z];
                    agent.yaw = if agent.direction > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    };
                }
                AgentZone::PadBPort => {
                    let z = -6.5 + t * 13.0;
                    agent.pos = [-31.5, -1.45, z];
                    agent.yaw = if agent.direction > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    };
                }
                AgentZone::PadCStarboard => {
                    let z = -6.5 + t * 13.0;
                    agent.pos = [31.5, -1.45, z];
                    agent.yaw = if agent.direction > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    };
                }
                AgentZone::OpsDeck => {
                    let angle = t * TAU;
                    let r = 6.2;
                    agent.pos = [angle.cos() * r, 12.1, angle.sin() * r];
                    agent.yaw = -angle + std::f32::consts::FRAC_PI_2 * agent.direction;
                }
                AgentZone::Concourse => {
                    let z = if agent.id % 2 == 0 { 5.4 } else { -5.4 };
                    let x = -13.0 + t * 26.0;
                    agent.pos = [x, -14.8, z];
                    agent.yaw = if agent.direction > 0.0 {
                        std::f32::consts::FRAC_PI_2
                    } else {
                        -std::f32::consts::FRAC_PI_2
                    };
                }
                AgentZone::ReactorCatwalk => {
                    let angle = t * TAU;
                    let r = 6.0;
                    agent.pos = [angle.cos() * r, -14.8, angle.sin() * r];
                    agent.yaw = -angle;
                }
                AgentZone::ExteriorTruss => {
                    let side = if agent.id % 2 == 0 { -1.0 } else { 1.0 };
                    let x = side * (16.0 + t * 16.0);
                    let y = 3.5 + (agent.phase * 0.5).sin() * 2.2;
                    let z = side * 11.0;
                    agent.pos = [x, y, z];
                    agent.yaw = agent.phase * 0.3;
                }
            }
        }
    }

    fn step_events(&mut self, dt: f32) {
        self.event_timer -= dt;
        if self.event_timer <= 0.0 {
            let any_approaching = self.ships.iter().any(|s| {
                matches!(
                    s.phase,
                    ShipFlightPhase::InboundApproach | ShipFlightPhase::FinalDescent
                )
            });
            self.event = match self.event {
                StationEventKind::Nominal => {
                    if any_approaching {
                        StationEventKind::DockingOps
                    } else {
                        StationEventKind::SolarEclipse
                    }
                }
                StationEventKind::DockingOps => {
                    if self.rng.range(0.0, 1.0) < 0.35 {
                        StationEventKind::SolarEclipse
                    } else if self.rng.range(0.0, 1.0) < 0.25 {
                        StationEventKind::SecurityAlert
                    } else {
                        StationEventKind::Nominal
                    }
                }
                StationEventKind::SolarEclipse => StationEventKind::Nominal,
                StationEventKind::SecurityAlert => StationEventKind::Nominal,
            };
            self.event_timer = match self.event {
                StationEventKind::SecurityAlert => 22.0,
                StationEventKind::SolarEclipse => 32.0,
                _ => 48.0,
            };
        }
    }

    fn step_camera_rig(&mut self, dt: f32) {
        if self.camera_rig.static_glitch_timer > 0.0 {
            self.camera_rig.static_glitch_timer =
                (self.camera_rig.static_glitch_timer - dt).max(0.0);
        }

        // Gently relax manual pan offset back toward center
        let decay = (-dt * 1.4).exp();
        self.camera_rig.manual_pan[0] *= decay;
        self.camera_rig.manual_pan[1] *= decay;

        // Pick the best active spacecraft for live ship uplinks (prefer FinalDescent -> Inbound -> Liftoff -> Patrol)
        let best_ship_id = self
            .ships
            .iter()
            .max_by_key(|s| match s.phase {
                ShipFlightPhase::FinalDescent => 5,
                ShipFlightPhase::InboundApproach => 4,
                ShipFlightPhase::Liftoff => 3,
                ShipFlightPhase::OutboundBurn => 2,
                ShipFlightPhase::PerimeterPatrol => 1,
                ShipFlightPhase::Docked => 0,
            })
            .map(|s| s.id);
        self.camera_rig.uplink_ship_id = best_ship_id;

        if self.camera_rig.auto_director {
            self.camera_rig.director_timer -= dt;
            if self.camera_rig.director_timer <= 0.0 {
                self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;

                // Opportunistically cut to ShipNoseUplink if a ship is on FinalDescent!
                let descent_active = self
                    .ships
                    .iter()
                    .any(|s| s.phase == ShipFlightPhase::FinalDescent);
                let next_feed = if descent_active
                    && self.camera_rig.primary_feed != CameraFeedId::ShipNoseUplink
                    && self.rng.range(0.0, 1.0) < 0.45
                {
                    CameraFeedId::ShipNoseUplink
                } else {
                    self.camera_rig.primary_feed.next()
                };
                self.camera_rig.primary_feed = next_feed;
                self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
            }
        }
    }

    pub fn recompute_telemetry(&mut self) {
        self.stats.station_age_hours = self.total_age_secs / 3600.0;
        self.stats.active_ships = self.ships.len() as u32;
        self.stats.docked_ships = self
            .ships
            .iter()
            .filter(|s| s.phase == ShipFlightPhase::Docked)
            .count() as u32;
        self.stats.active_drones = self.drones.len() as u32;
        self.stats.active_agents = self.agents.len() as u32;

        let uplink_info = self.uplink_ship().map(|ship| {
            let phase = match ship.phase {
                ShipFlightPhase::InboundApproach => "APPROACH",
                ShipFlightPhase::FinalDescent => "FINAL DESCENT",
                ShipFlightPhase::Docked => "DOCKED",
                ShipFlightPhase::Liftoff => "LIFTOFF",
                ShipFlightPhase::OutboundBurn => "DEPARTING",
                ShipFlightPhase::PerimeterPatrol => "PATROL",
            };
            (ship.callsign.clone(), phase.to_string())
        });
        if let Some((callsign, phase)) = uplink_info {
            self.stats.uplink_callsign = callsign;
            self.stats.uplink_phase = phase;
        } else {
            self.stats.uplink_callsign = "NO SIGNAL".into();
            self.stats.uplink_phase = "STANDBY".into();
        }
    }

    /// Return the spacecraft currently targeted by `ShipNoseUplink` / `ShipDorsalUplink`.
    pub fn uplink_ship(&self) -> Option<&Spacecraft> {
        self.camera_rig
            .uplink_ship_id
            .and_then(|id| self.ships.iter().find(|s| s.id == id))
            .or_else(|| self.ships.first())
    }

    // =========================================================================
    // Public User / UI Actions
    // =========================================================================

    pub fn cycle_camera(&mut self) {
        self.camera_rig.primary_feed = self.camera_rig.primary_feed.next();
        self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;
        self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        self.camera_rig.manual_pan = [0.0, 0.0];
    }

    pub fn select_feed(&mut self, feed: CameraFeedId) {
        if self.camera_rig.primary_feed != feed {
            self.camera_rig.primary_feed = feed;
            self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
            self.camera_rig.manual_pan = [0.0, 0.0];
        }
    }

    /// Swap a PiP slot with the primary feed.
    pub fn promote_pip_slot(&mut self, slot: usize) {
        if slot < 2 {
            std::mem::swap(
                &mut self.camera_rig.pip_feeds[slot],
                &mut self.camera_rig.primary_feed,
            );
            self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        }
    }

    /// Cycle a specific quadrant in 2x2 Grid mode to the next camera feed.
    pub fn cycle_grid_slot(&mut self, slot: usize) {
        if slot < 4 {
            self.camera_rig.grid_feeds[slot] = self.camera_rig.grid_feeds[slot].next();
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        }
    }

    pub fn cycle_layout(&mut self) {
        self.camera_rig.layout = self.camera_rig.layout.next();
        self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
    }

    pub fn set_layout(&mut self, layout: CameraLayout) {
        if self.camera_rig.layout != layout {
            self.camera_rig.layout = layout;
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        }
    }

    pub fn toggle_auto_director(&mut self) {
        self.camera_rig.auto_director = !self.camera_rig.auto_director;
        if self.camera_rig.auto_director {
            self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;
        }
    }

    /// Dispatch a new inbound spacecraft toward the least busy landing pad.
    pub fn dispatch_ship(&mut self) {
        if self.ships.len() >= 7 {
            // Keep fleet tight and readable; nudge an existing docked ship to lift off
            if let Some(docked) = self
                .ships
                .iter_mut()
                .find(|s| s.phase == ShipFlightPhase::Docked)
            {
                docked.leg_progress = 0.99;
            }
            return;
        }
        let pads = [
            LandingPadId::PadAHangar,
            LandingPadId::PadBPort,
            LandingPadId::PadCStarboard,
        ];
        let best_pad = pads
            .into_iter()
            .min_by_key(|&pad| {
                self.ships
                    .iter()
                    .filter(|s| {
                        s.assigned_pad == pad && s.phase != ShipFlightPhase::PerimeterPatrol
                    })
                    .count()
            })
            .unwrap_or(LandingPadId::PadAHangar);

        self.spawn_inbound_ship_for_pad(best_pad);
        self.event = StationEventKind::DockingOps;
        self.event_timer = 35.0;
        self.recompute_telemetry();
    }

    pub fn cycle_palette(&mut self) {
        self.palette = self.palette.next();
    }

    pub fn set_palette(&mut self, palette: OrbitalPalette) {
        self.palette = palette;
    }

    pub fn trigger_station_event(&mut self) {
        self.event = self.event.next();
        self.event_timer = 35.0;
        self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION * 0.7;
    }

    pub fn toggle_hud(&mut self) {
        self.hud_enabled = !self.hud_enabled;
    }

    pub fn nudge_pan(&mut self, dyaw: f32, dpitch: f32) {
        self.camera_rig.manual_pan[0] = (self.camera_rig.manual_pan[0] + dyaw).clamp(-0.65, 0.65);
        self.camera_rig.manual_pan[1] = (self.camera_rig.manual_pan[1] + dpitch).clamp(-0.38, 0.38);
    }
}
