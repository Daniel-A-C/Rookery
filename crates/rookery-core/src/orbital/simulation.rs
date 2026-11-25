//! Real-time 3D simulation step, ATC flight trajectories, autonomous agents,
//! and multi-camera director logic for *Orbital Haven v3*.

use std::f32::consts::TAU;

use super::types::*;
use super::{
    OrbitalHavenState, PAD_A_GATE, PAD_A_POS, PAD_B_GATE, PAD_B_POS, PAD_C_GATE, PAD_C_POS,
};

const DIRECTOR_INTERVAL_SECS: f32 = 24.0;
const STATIC_GLITCH_DURATION: f32 = 0.18;

/// Z-coordinate of the Deck 1 Reactor Core center (shifted forward from `0.0` to `+1.5`
/// so a clean lobby separates the 3-deck elevator shaft from the reactor containment collar).
pub const REACTOR_CORE_Z: f32 = 1.5;

/// Route indices exported for deterministic unit tests.
#[cfg(test)]
pub(crate) const ROUTE_IDX_OPS_VIEWPORT: usize = 12;
#[cfg(test)]
pub(crate) const ROUTE_IDX_SOLAR_PIPE_RETURN: usize = 59;

/// Enforce an AABB exclusion zone around the 3-deck glass elevator shaft (`X: -2.65..2.65`,
/// `Z: -8.65..-3.35`) expanded by `margin` so rolling robots never clip through the shaft walls.
fn clamp_outside_elevator_shaft(x: f32, z: f32, margin: f32) -> (f32, f32) {
    let min_x = -2.65 - margin;
    let max_x = 2.65 + margin;
    let min_z = -8.65 - margin;
    let max_z = -3.35 + margin;
    if x > min_x && x < max_x && z > min_z && z < max_z {
        let dx_left = (x - min_x).abs();
        let dx_right = (max_x - x).abs();
        let dz_back = (z - min_z).abs();
        let dz_front = (max_z - z).abs();
        let m = dx_left.min(dx_right).min(dz_back).min(dz_front);
        if m == dx_left {
            (min_x, z)
        } else if m == dx_right {
            (max_x, z)
        } else if m == dz_back {
            (x, min_z)
        } else {
            (x, max_z)
        }
    } else {
        (x, z)
    }
}

/// Smooth floor elevation along the Deck-2 Atrium (`|X| <= 12.2`, `Y = -3.85`), sloped
/// Pad Access Hallways (`|X| = 12.2..23.2`), and level Airlocks / External Pads (`|X| >= 23.2`, `Y = -1.42`).
fn hallway_floor_y(x: f32) -> f32 {
    let ax = x.abs();
    if ax <= 12.2 {
        -3.85
    } else if ax >= 23.2 {
        -1.42
    } else {
        let t = (ax - 12.2) / 11.0;
        -3.85 + 2.43 * t
    }
}

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

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Smoothly rotate unit vector `current` toward `target` without ever snapping, even across 180°.
fn turn_toward(current: [f32; 3], target: [f32; 3], rate: f32, dt: f32) -> [f32; 3] {
    let c = norm3(current, [0.0, 0.0, 1.0]);
    let t = norm3(target, c);
    let d = dot3(c, t);
    // If nearly anti-parallel (180° turn), nudge sideways in XZ so the turn arcs smoothly instead of collapsing
    let target_safe = if d < -0.985 {
        norm3([c[2] + 0.15, c[1], -c[0]], [1.0, 0.0, 0.0])
    } else {
        t
    };
    let alpha = 1.0 - (-rate * dt).exp();
    norm3(lerp3(c, target_safe, alpha), c)
}

/// Smoothly ease a scalar toward a target value.
fn ease_scalar(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let alpha = 1.0 - (-rate * dt).exp();
    current + (target - current) * alpha
}

/// Quadratic Bézier point evaluation for graceful curved approach/departure arcs.
fn quad_bezier3(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], t: f32) -> [f32; 3] {
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    let two_ut = 2.0 * u * t;
    [
        uu * p0[0] + two_ut * p1[0] + tt * p2[0],
        uu * p0[1] + two_ut * p1[1] + tt * p2[1],
        uu * p0[2] + two_ut * p1[2] + tt * p2[2],
    ]
}

fn pad_docked_forward(pad: LandingPadId) -> [f32; 3] {
    match pad {
        LandingPadId::PadAHangar => [0.0, 0.0, -1.0],
        LandingPadId::PadBPort | LandingPadId::PadCStarboard => [0.0, 0.0, 1.0],
    }
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

        // 2b. Advance docked-ship service choreographies (welding arm, fuel boom, bio-scan, drones, ramps)
        self.step_pad_services(dt);

        // 2c. Advance bespoke 5-minute looping room activities across all 5 main interior rooms
        self.room_activities.step(dt);

        // 3. Update interior crew, security droids, cargo loaders & exterior welders
        self.step_agents(dt);

        // 3b. Update Interior Walker Camera Bot, 3-deck Glass Elevator & sliding blast doors
        self.step_walker_and_elevator(dt);

        // 3c. Update Hermes free-flying camera drone (hover, return-to-home autopilot)
        self.step_hermes(dt);

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
        self.room_activities.step(clamped as f32);
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

    /// Returns `true` when `pad` has no ship descending, docked, lifting off, or on late approach,
    /// and all pad service hardware is retracted.
    pub fn is_pad_clear(&self, pad: LandingPadId) -> bool {
        if !self.pad_services[pad.index()].hardware_retracted() {
            return false;
        }
        !self.ships.iter().any(|s| {
            s.assigned_pad == pad
                && match s.phase {
                    ShipFlightPhase::FinalDescent
                    | ShipFlightPhase::Docked
                    | ShipFlightPhase::Liftoff => true,
                    ShipFlightPhase::InboundApproach => s.leg_progress >= 0.65,
                    ShipFlightPhase::OutboundBurn | ShipFlightPhase::PerimeterPatrol => false,
                }
        })
    }

    /// Returns `true` while BOT-01 is physically out on `pad`'s landing zone (or exiting onto it).
    pub fn is_walker_on_pad(&self, pad: LandingPadId) -> bool {
        let wp = self.walker.pos;
        if wp[1] < -9.0 || wp[1] > 5.0 {
            return false;
        }
        match pad {
            LandingPadId::PadAHangar => wp[0].abs() <= 7.5 && wp[2] >= 17.8,
            LandingPadId::PadBPort => wp[0] <= -27.5,
            LandingPadId::PadCStarboard => wp[0] >= 27.5,
        }
    }

    fn step_ships(&mut self, dt: f32) {
        let mut serviced_delta = 0u32;
        let mut need_replacement_pads: Vec<LandingPadId> = Vec::new();
        let mut remove_ids: Vec<u32> = Vec::new();
        let mut newly_docked: Vec<(LandingPadId, u32, ShipClass)> = Vec::new();
        let mut departed_pads: Vec<(LandingPadId, u32)> = Vec::new();
        let walker_on_pad = [
            self.is_walker_on_pad(LandingPadId::PadAHangar),
            self.is_walker_on_pad(LandingPadId::PadBPort),
            self.is_walker_on_pad(LandingPadId::PadCStarboard),
        ];

        for ship in &mut self.ships {
            let turn_rate = match ship.class {
                ShipClass::PatrolCutter => 2.6,
                ShipClass::MedicalCourier => 2.25,
                ShipClass::CorporateShuttle => 2.0,
                ShipClass::SurveyProbeCarrier => 1.85,
                ShipClass::MaintenanceTug => 1.7,
                ShipClass::BulkFreighter | ShipClass::FuelTanker => 1.25,
            };

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
                    ship.forward = turn_toward(ship.forward, dir, turn_rate * 1.4, dt);
                    // Subtle inward bank toward the station during orbital patrol
                    let inward = norm3([-x, 0.0, -z], [0.0, 0.0, 0.0]);
                    let banked_up =
                        norm3([inward[0] * 0.34, 1.0, inward[2] * 0.34], [0.0, 1.0, 0.0]);
                    ship.up = turn_toward(ship.up, banked_up, 2.2, dt);
                    ship.thruster_power = ease_scalar(ship.thruster_power, 0.65, 3.0, dt);
                    ship.vtol_thrust = ease_scalar(ship.vtol_thrust, 0.0, 4.0, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, 0.0, 4.0, dt);
                    ship.gear_deploy = ease_scalar(ship.gear_deploy, 0.0, 3.0, dt);
                }
                ShipFlightPhase::InboundApproach => {
                    let pad_held_by_bot = walker_on_pad[ship.assigned_pad.index()];
                    if pad_held_by_bot && ship.leg_progress >= 0.96 {
                        // ATC hold: hover at the approach gate until BOT-01 clears the landing pad
                        ship.leg_progress = 0.96;
                    } else {
                        ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    }
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;

                    // Curved Bézier approach control point that swings smoothly into alignment with the gate
                    let ctrl = match ship.assigned_pad {
                        LandingPadId::PadAHangar => [
                            ship.leg_start[0] * 0.35,
                            (ship.leg_start[1] + ship.leg_end[1]) * 0.55,
                            ship.leg_end[2] + 34.0,
                        ],
                        LandingPadId::PadBPort => [
                            ship.leg_end[0] - 24.0,
                            ship.leg_end[1] + 10.0,
                            ship.leg_start[2] * 0.45,
                        ],
                        LandingPadId::PadCStarboard => [
                            ship.leg_end[0] + 24.0,
                            ship.leg_end[1] + 10.0,
                            ship.leg_start[2] * 0.45,
                        ],
                    };
                    ship.pos = quad_bezier3(ship.leg_start, ctrl, ship.leg_end, t);
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];

                    // Blend desired heading from travel direction into level pad alignment as we reach the gate
                    let travel_dir = norm3(d, ship.forward);
                    let pad_fwd = pad_docked_forward(ship.assigned_pad);
                    let align_blend = smoothstep((t - 0.55) / 0.45);
                    let desired_fwd = norm3(lerp3(travel_dir, pad_fwd, align_blend), pad_fwd);
                    ship.forward = turn_toward(ship.forward, desired_fwd, turn_rate, dt);
                    ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 2.5, dt);

                    // Taper main thrusters down and engage retro/VTOL braking near the gate
                    let target_main = (0.85 * (1.0 - smoothstep((t - 0.5) / 0.5))).max(0.0);
                    let target_retro = smoothstep((t - 0.65) / 0.35) * 0.55;
                    let target_vtol = smoothstep((t - 0.72) / 0.28) * 0.45;
                    ship.thruster_power = ease_scalar(ship.thruster_power, target_main, 3.5, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, target_retro, 4.0, dt);
                    ship.vtol_thrust = ease_scalar(ship.vtol_thrust, target_vtol, 4.0, dt);
                    ship.gear_deploy = ease_scalar(
                        ship.gear_deploy,
                        ((t - 0.72).max(0.0) / 0.28) * 0.35,
                        3.0,
                        dt,
                    );

                    if !pad_held_by_bot && ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::FinalDescent;
                        ship.leg_start = ship.pos;
                        ship.leg_end = pad_touchdown_pos(ship.assigned_pad);
                        ship.leg_progress = 0.0;
                        ship.leg_duration = match ship.assigned_pad {
                            LandingPadId::PadAHangar => 15.0,
                            _ => 12.5,
                        };
                    }
                }
                ShipFlightPhase::FinalDescent => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let t = smoothstep(ship.leg_progress);
                    let prev = ship.pos;

                    let p = if ship.assigned_pad == LandingPadId::PadAHangar {
                        // Glide straight in through the forcefield at hover height, then settle gently onto Pad A
                        let horiz_t = smoothstep((ship.leg_progress / 0.78).min(1.0));
                        let drop_t =
                            smoothstep(((ship.leg_progress - 0.62) / 0.38).clamp(0.0, 1.0));
                        let hover_y = ship.leg_end[1] + 3.4;
                        let z = ship.leg_start[2] + (ship.leg_end[2] - ship.leg_start[2]) * horiz_t;
                        let y = if ship.leg_progress < 0.62 {
                            ship.leg_start[1]
                                + (hover_y - ship.leg_start[1])
                                    * smoothstep(ship.leg_progress / 0.62)
                        } else {
                            hover_y + (ship.leg_end[1] - hover_y) * drop_t
                        };
                        [
                            ship.leg_start[0] + (ship.leg_end[0] - ship.leg_start[0]) * horiz_t,
                            y,
                            z,
                        ]
                    } else {
                        // External Pads B & C: pure smooth vertical VTOL descent straight onto the deck
                        lerp3(ship.leg_start, ship.leg_end, t)
                    };

                    ship.pos = p;
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];

                    // Stay smoothly level on pad heading — zero sudden pitching or diving!
                    let pad_fwd = pad_docked_forward(ship.assigned_pad);
                    ship.forward = turn_toward(ship.forward, pad_fwd, turn_rate * 1.4, dt);
                    ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 3.0, dt);

                    // Main aft engines stay off during VTOL touchdown so plumes never clip the floor!
                    let target_vtol = (0.78 * (1.0 - smoothstep((t - 0.86) / 0.14))).max(0.12);
                    let target_retro = if ship.assigned_pad == LandingPadId::PadAHangar {
                        0.55 * (1.0 - t)
                    } else {
                        0.0
                    };
                    ship.thruster_power = ease_scalar(ship.thruster_power, 0.0, 4.5, dt);
                    ship.vtol_thrust = ease_scalar(ship.vtol_thrust, target_vtol, 4.0, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, target_retro, 4.0, dt);
                    ship.gear_deploy =
                        ease_scalar(ship.gear_deploy, (0.35 + t * 0.85).clamp(0.0, 1.0), 4.0, dt);

                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::Docked;
                        ship.pos = ship.leg_end;
                        ship.vel = [0.0, 0.0, 0.0];
                        ship.gear_deploy = 1.0;
                        ship.leg_progress = 0.0;
                        ship.leg_duration = 2.0; // VTOL pre-ignition countdown once pad service finishes
                        newly_docked.push((ship.assigned_pad, ship.id, ship.class));
                    }
                }
                ShipFlightPhase::Docked => {
                    let pad_idx = ship.assigned_pad.index();
                    let svc = &self.pad_services[pad_idx];
                    let service_ready = svc.ship_id != Some(ship.id)
                        || (svc.is_complete() && svc.hardware_retracted());

                    let pad_fwd = pad_docked_forward(ship.assigned_pad);
                    ship.forward = turn_toward(ship.forward, pad_fwd, 2.0, dt);
                    ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 3.0, dt);
                    ship.gear_deploy = 1.0;
                    ship.thruster_power = ease_scalar(ship.thruster_power, 0.0, 5.0, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, 0.0, 5.0, dt);

                    if service_ready {
                        // Gentle VTOL pre-ignition warm-up before liftoff
                        ship.leg_progress += dt / ship.leg_duration.max(1.5);
                        let pre_ignite = smoothstep(ship.leg_progress) * 0.62;
                        ship.vtol_thrust = ease_scalar(ship.vtol_thrust, pre_ignite, 4.0, dt);
                    } else {
                        ship.leg_progress = 0.0;
                        ship.vtol_thrust = ease_scalar(ship.vtol_thrust, 0.0, 4.0, dt);
                    }

                    if service_ready && ship.leg_progress >= 1.0 {
                        departed_pads.push((ship.assigned_pad, ship.id));
                        ship.phase = ShipFlightPhase::Liftoff;
                        ship.leg_start = ship.pos;
                        ship.leg_end = pad_gate_pos(ship.assigned_pad);
                        ship.leg_progress = 0.0;
                        ship.leg_duration = match ship.assigned_pad {
                            LandingPadId::PadAHangar => 14.5,
                            _ => 12.5,
                        };
                    }
                }
                ShipFlightPhase::Liftoff => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let prev = ship.pos;
                    let pad_fwd = pad_docked_forward(ship.assigned_pad);

                    if ship.assigned_pad == LandingPadId::PadAHangar {
                        // HANGAR REVERSE-OUT:
                        // Phase A (0..0.30): Smooth vertical hover lift straight off Pad A (staying level!)
                        // Phase B (0.30..1.0): Back straight out through the containment forcefield (staying facing -Z!)
                        let hover_y = ship.leg_start[1] + 3.8;
                        let p = if ship.leg_progress < 0.30 {
                            let u = smoothstep(ship.leg_progress / 0.30);
                            [
                                ship.leg_start[0],
                                ship.leg_start[1] + (hover_y - ship.leg_start[1]) * u,
                                ship.leg_start[2],
                            ]
                        } else {
                            let u = smoothstep((ship.leg_progress - 0.30) / 0.70);
                            [
                                ship.leg_start[0] + (ship.leg_end[0] - ship.leg_start[0]) * u,
                                hover_y + (ship.leg_end[1] - hover_y) * u,
                                ship.leg_start[2] + (ship.leg_end[2] - ship.leg_start[2]) * u,
                            ]
                        };
                        ship.pos = p;
                        // Keep nose facing smoothly into the hangar (-Z) while backing out — NO magical 180° spin!
                        ship.forward = turn_toward(ship.forward, pad_fwd, 2.5, dt);
                        ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 3.0, dt);
                        ship.thruster_power = ease_scalar(ship.thruster_power, 0.0, 5.0, dt);
                        let target_vtol = if ship.leg_progress < 0.38 { 0.85 } else { 0.45 };
                        let target_retro = if ship.leg_progress >= 0.25 { 0.82 } else { 0.0 };
                        ship.vtol_thrust = ease_scalar(ship.vtol_thrust, target_vtol, 4.0, dt);
                        ship.retro_thrust = ease_scalar(ship.retro_thrust, target_retro, 4.0, dt);
                        let target_gear = if ship.leg_progress < 0.20 { 1.0 } else { 0.0 };
                        ship.gear_deploy = ease_scalar(ship.gear_deploy, target_gear, 3.0, dt);
                    } else {
                        // EXTERNAL PADS B & C:
                        // Phase A (0..0.55): Pure vertical VTOL lift straight up off the deck (completely level, aft plumes OFF!)
                        // Phase B (0.55..1.0): Smoothly drift outward away from the station & gently nose toward exit lane
                        let side = if ship.assigned_pad == LandingPadId::PadBPort {
                            -1.0
                        } else {
                            1.0
                        };
                        let hover_pt = ship.leg_end; // directly above pad at Y = +14.5
                        let clear_pt = [
                            hover_pt[0] + side * 16.0,
                            hover_pt[1] + 6.5,
                            hover_pt[2] + 18.0,
                        ];

                        let (p, desired_fwd, target_main, target_vtol) = if ship.leg_progress < 0.55
                        {
                            let u = smoothstep(ship.leg_progress / 0.55);
                            (lerp3(ship.leg_start, hover_pt, u), pad_fwd, 0.0, 0.90)
                        } else {
                            let u = smoothstep((ship.leg_progress - 0.55) / 0.45);
                            let pos = lerp3(hover_pt, clear_pt, u);
                            let drift_dir = norm3([side * 0.55, 0.20, 0.80], pad_fwd);
                            let fwd = norm3(lerp3(pad_fwd, drift_dir, u), pad_fwd);
                            (pos, fwd, u * 0.55, (1.0 - u) * 0.75)
                        };
                        ship.pos = p;
                        ship.forward = turn_toward(ship.forward, desired_fwd, turn_rate * 0.9, dt);
                        ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 2.5, dt);
                        ship.thruster_power =
                            ease_scalar(ship.thruster_power, target_main, 3.0, dt);
                        ship.vtol_thrust = ease_scalar(ship.vtol_thrust, target_vtol, 3.5, dt);
                        ship.retro_thrust = ease_scalar(ship.retro_thrust, 0.0, 4.0, dt);
                        let target_gear = if ship.leg_progress < 0.28 { 1.0 } else { 0.0 };
                        ship.gear_deploy = ease_scalar(ship.gear_deploy, target_gear, 3.0, dt);
                    }

                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];

                    if ship.leg_progress >= 1.0 {
                        ship.phase = ShipFlightPhase::OutboundBurn;
                        ship.leg_start = ship.pos;
                        let exit_x = match ship.assigned_pad {
                            LandingPadId::PadBPort => -140.0,
                            LandingPadId::PadCStarboard => 140.0,
                            LandingPadId::PadAHangar => 48.0,
                        };
                        ship.leg_end = [exit_x, 32.0, 155.0];
                        ship.leg_progress = 0.0;
                        ship.leg_duration = 16.0;
                    }
                }
                ShipFlightPhase::OutboundBurn => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let prev = ship.pos;

                    // Hangar departures start facing -Z outside the forcefield, so give them a graceful
                    // wide curving turn in open space before lighting full main engines!
                    let t = smoothstep(ship.leg_progress);
                    let ctrl = match ship.assigned_pad {
                        LandingPadId::PadAHangar => [26.0, 10.0, ship.leg_start[2] + 24.0],
                        LandingPadId::PadBPort => [
                            ship.leg_start[0] - 28.0,
                            ship.leg_start[1] + 6.0,
                            ship.leg_start[2] + 45.0,
                        ],
                        LandingPadId::PadCStarboard => [
                            ship.leg_start[0] + 28.0,
                            ship.leg_start[1] + 6.0,
                            ship.leg_start[2] + 45.0,
                        ],
                    };
                    ship.pos = quad_bezier3(ship.leg_start, ctrl, ship.leg_end, t);
                    let d = sub3(ship.pos, prev);
                    ship.vel = [
                        d[0] / dt.max(0.001),
                        d[1] / dt.max(0.001),
                        d[2] / dt.max(0.001),
                    ];
                    let exit_dir = norm3(d, ship.forward);
                    ship.forward = turn_toward(ship.forward, exit_dir, turn_rate * 0.85, dt);
                    ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 2.0, dt);

                    // Only ramp up main aft engines once the ship has swung reasonably toward its exit vector
                    let align = dot3(ship.forward, exit_dir).clamp(0.0, 1.0);
                    let target_main = 0.20 + 0.80 * align * align;
                    ship.thruster_power = ease_scalar(ship.thruster_power, target_main, 2.8, dt);
                    ship.vtol_thrust = ease_scalar(ship.vtol_thrust, 0.0, 4.0, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, 0.0, 4.0, dt);
                    ship.gear_deploy = ease_scalar(ship.gear_deploy, 0.0, 4.0, dt);

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

        for (pad, ship_id) in departed_pads {
            let svc = &mut self.pad_services[pad.index()];
            if svc.ship_id == Some(ship_id) {
                *svc = PadServiceState::default();
            }
        }
        for (pad, ship_id, class) in newly_docked {
            self.begin_pad_service(pad, ship_id, class);
        }

        for pad in need_replacement_pads {
            self.spawn_inbound_ship_for_pad(pad);
        }
    }

    /// Roll a randomized 2–3 step physical service choreography for a newly docked ship.
    pub fn begin_pad_service(&mut self, pad: LandingPadId, ship_id: u32, class: ShipClass) {
        let pool: &[ServiceKind] = match class {
            ShipClass::CorporateShuttle => &[
                ServiceKind::BoardingRampExchange,
                ServiceKind::FuelUmbilical,
                ServiceKind::BioScanSweep,
                ServiceKind::CargoTransfer,
            ],
            ShipClass::BulkFreighter => &[
                ServiceKind::CargoTransfer,
                ServiceKind::FuelUmbilical,
                ServiceKind::ArticulatedRepairArm,
                ServiceKind::BioScanSweep,
            ],
            ShipClass::PatrolCutter => &[
                ServiceKind::DroneResupply,
                ServiceKind::ArticulatedRepairArm,
                ServiceKind::FuelUmbilical,
                ServiceKind::BioScanSweep,
            ],
            ShipClass::MaintenanceTug => &[
                ServiceKind::ArticulatedRepairArm,
                ServiceKind::CargoTransfer,
                ServiceKind::DroneResupply,
                ServiceKind::FuelUmbilical,
            ],
            ShipClass::MedicalCourier => &[
                ServiceKind::BioScanSweep,
                ServiceKind::BoardingRampExchange,
                ServiceKind::CargoTransfer,
                ServiceKind::FuelUmbilical,
            ],
            ShipClass::SurveyProbeCarrier => &[
                ServiceKind::DroneResupply,
                ServiceKind::BioScanSweep,
                ServiceKind::ArticulatedRepairArm,
                ServiceKind::FuelUmbilical,
            ],
            ShipClass::FuelTanker => &[
                ServiceKind::FuelUmbilical,
                ServiceKind::BioScanSweep,
                ServiceKind::ArticulatedRepairArm,
                ServiceKind::DroneResupply,
            ],
        };

        let mut remaining: Vec<ServiceKind> = pool.to_vec();
        let step_count = if (self.rng.next_u64() & 1) == 0 { 2 } else { 3 };
        let mut steps = Vec::with_capacity(step_count);

        if matches!(
            class,
            ShipClass::BulkFreighter | ShipClass::FuelTanker | ShipClass::MedicalCourier
        ) && !remaining.is_empty()
        {
            steps.push(remaining.remove(0));
        }
        while steps.len() < step_count && !remaining.is_empty() {
            let pick = (self.rng.next_u64() as usize) % remaining.len();
            steps.push(remaining.remove(pick));
        }

        let first_dur = steps.first().map(|s| s.duration_secs()).unwrap_or(6.0);

        self.pad_services[pad.index()] = PadServiceState {
            ship_id: Some(ship_id),
            steps,
            current_step: 0,
            step_progress: 0.0,
            step_duration: first_dur,
            repair_arm_ext: 0.0,
            fuel_boom_ext: 0.0,
            scan_sweep_t: 0.0,
            drone_deploy: 0.0,
            ramp_deploy: 0.0,
            crates_transferred: 0,
        };
    }

    fn step_pad_services(&mut self, dt: f32) {
        for svc in &mut self.pad_services {
            if svc.ship_id.is_none() {
                svc.repair_arm_ext = ease_scalar(svc.repair_arm_ext, 0.0, 3.2, dt);
                svc.fuel_boom_ext = ease_scalar(svc.fuel_boom_ext, 0.0, 3.2, dt);
                svc.drone_deploy = ease_scalar(svc.drone_deploy, 0.0, 3.2, dt);
                svc.ramp_deploy = ease_scalar(svc.ramp_deploy, 0.0, 3.2, dt);
                svc.scan_sweep_t = 0.0;
                continue;
            }

            let active = svc.active_kind();
            if let Some(kind) = active {
                svc.step_progress += dt / svc.step_duration.max(1.0);
                let p = svc.step_progress.clamp(0.0, 1.0);

                let arm_tgt = if kind == ServiceKind::ArticulatedRepairArm && p < 0.82 {
                    1.0
                } else {
                    0.0
                };
                let fuel_tgt = if kind == ServiceKind::FuelUmbilical && p < 0.82 {
                    1.0
                } else {
                    0.0
                };
                let drone_tgt = if kind == ServiceKind::DroneResupply && p < 0.82 {
                    1.0
                } else {
                    0.0
                };
                let ramp_tgt = if kind == ServiceKind::BoardingRampExchange && p < 0.82 {
                    1.0
                } else {
                    0.0
                };

                svc.repair_arm_ext = ease_scalar(svc.repair_arm_ext, arm_tgt, 3.6, dt);
                svc.fuel_boom_ext = ease_scalar(svc.fuel_boom_ext, fuel_tgt, 3.6, dt);
                svc.drone_deploy = ease_scalar(svc.drone_deploy, drone_tgt, 3.4, dt);
                svc.ramp_deploy = ease_scalar(svc.ramp_deploy, ramp_tgt, 3.6, dt);
                svc.scan_sweep_t = if kind == ServiceKind::BioScanSweep {
                    p
                } else {
                    0.0
                };

                if svc.step_progress >= 1.0 {
                    if kind == ServiceKind::CargoTransfer {
                        svc.crates_transferred = svc.crates_transferred.saturating_add(1);
                    }
                    svc.current_step += 1;
                    svc.step_progress = 0.0;
                    svc.step_duration = svc
                        .active_kind()
                        .map(|next| next.duration_secs())
                        .unwrap_or(1.0);
                }
            } else {
                svc.repair_arm_ext = ease_scalar(svc.repair_arm_ext, 0.0, 4.2, dt);
                svc.fuel_boom_ext = ease_scalar(svc.fuel_boom_ext, 0.0, 4.2, dt);
                svc.drone_deploy = ease_scalar(svc.drone_deploy, 0.0, 4.2, dt);
                svc.ramp_deploy = ease_scalar(svc.ramp_deploy, 0.0, 4.2, dt);
                svc.scan_sweep_t = 0.0;
            }
        }
    }

    fn spawn_inbound_ship_for_pad(&mut self, pad: LandingPadId) {
        let roll = self.rng.range(0.0, 1.0);
        let class = match pad {
            LandingPadId::PadAHangar => {
                if roll < 0.34 {
                    ShipClass::CorporateShuttle
                } else if roll < 0.62 {
                    ShipClass::MedicalCourier
                } else if roll < 0.84 {
                    ShipClass::SurveyProbeCarrier
                } else {
                    ShipClass::PatrolCutter
                }
            }
            LandingPadId::PadBPort => {
                if roll < 0.38 {
                    ShipClass::BulkFreighter
                } else if roll < 0.68 {
                    ShipClass::FuelTanker
                } else if roll < 0.86 {
                    ShipClass::MaintenanceTug
                } else {
                    ShipClass::SurveyProbeCarrier
                }
            }
            LandingPadId::PadCStarboard => {
                if roll < 0.28 {
                    ShipClass::MaintenanceTug
                } else if roll < 0.54 {
                    ShipClass::FuelTanker
                } else if roll < 0.76 {
                    ShipClass::MedicalCourier
                } else if roll < 0.90 {
                    ShipClass::PatrolCutter
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
            vtol_thrust: 0.0,
            retro_thrust: 0.0,
            patrol_angle: 0.0,
            patrol_radius: 0.0,
        });
    }

    fn step_agents(&mut self, dt: f32) {
        use super::nav::{NAV_NODES, find_path_avoiding, goal_pool_for_agent};

        let turn_yaw = |curr: f32, target: f32, rate: f32, dt: f32| -> f32 {
            let mut diff = (target - curr).rem_euclid(TAU);
            if diff > std::f32::consts::PI {
                diff -= TAU;
            }
            curr + diff * (1.0 - (-rate * dt).exp())
        };

        // Track BOT-01's occupancy of the single-lane Axial Pipe Tunnel (owner ID = 0)
        let walker_in_pipe = (self.walker.pos[1] - (-3.85)).abs() < 1.5
            && (-25.6..=-14.2).contains(&self.walker.pos[2]);
        if walker_in_pipe {
            self.pipe_tunnel_owner = Some(0);
        } else if self.pipe_tunnel_owner == Some(0) {
            self.pipe_tunnel_owner = None;
        }

        // Snapshot all mobile robot positions + radii (including BOT-01 as ID 0) for collision avoidance
        let walker_holding = self.walker.dwell_timer > 0.0
            || self.walker.current_room.contains("HOLD")
            || self.walker.current_room.contains("BOARDING");
        let mut all_bodies: Vec<(u32, [f32; 3], f32)> = Vec::with_capacity(self.agents.len() + 1);
        all_bodies.push((0, self.walker.pos, 0.85));
        for a in &self.agents {
            all_bodies.push((a.id, a.pos, a.kind.radius()));
        }

        let agent_count = self.agents.len();
        for idx in 0..agent_count {
            let kind = self.agents[idx].kind;
            let id = self.agents[idx].id;
            let zone = self.agents[idx].zone;
            self.agents[idx].phase = (self.agents[idx].phase + dt * 3.8) % TAU;

            // Exterior hull welder drones drift freely along the outer trusses
            if kind == AgentKind::HullWelder {
                let a = &mut self.agents[idx];
                a.patrol_t += dt * a.speed * a.direction;
                if a.patrol_t > 1.0 {
                    a.patrol_t = 1.0;
                    a.direction = -1.0;
                } else if a.patrol_t < 0.0 {
                    a.patrol_t = 0.0;
                    a.direction = 1.0;
                }
                let side = if id.is_multiple_of(2) { -1.0 } else { 1.0 };
                let x = side * (16.0 + a.patrol_t * 16.0);
                let y = 3.5 + (a.phase * 0.5).sin() * 2.2;
                let z = side * 11.0;
                a.pos = [x, y, z];
                a.yaw = a.phase * 0.3;
                continue;
            }

            // Interactive Pad CargoForklifts: respond to docked-ship `CargoTransfer` first,
            // and in the Main Hangar Bay execute the 5-minute Compactor & Gantry Crane cycle between ships!
            if kind == AgentKind::CargoForklift {
                let (pad_idx, stage_pos, load_pos, face_ship_yaw, face_stage_yaw) = match zone {
                    AgentZone::HangarBay => {
                        if self.agents[idx].pos[0] < 0.0 {
                            (
                                0usize,
                                [-7.35, -3.88, 22.0],
                                [-7.35, -3.88, 26.8],
                                std::f32::consts::FRAC_PI_2,
                                0.0,
                            )
                        } else {
                            (
                                0usize,
                                [7.35, -3.88, 31.4],
                                [7.35, -3.88, 26.8],
                                -std::f32::consts::FRAC_PI_2,
                                std::f32::consts::PI,
                            )
                        }
                    }
                    AgentZone::PadBPort => (
                        1usize,
                        [-31.3, -1.42, -5.2],
                        [-31.3, -1.42, 0.2],
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                    ),
                    _ => (
                        2usize,
                        [31.3, -1.42, 3.5],
                        [31.3, -1.42, -0.4],
                        std::f32::consts::FRAC_PI_2,
                        std::f32::consts::PI,
                    ),
                };

                let cargo_active =
                    self.pad_services[pad_idx].active_kind() == Some(ServiceKind::CargoTransfer);
                let p = self.pad_services[pad_idx].step_progress.clamp(0.0, 1.0);
                let hangar_sub_u = self.room_activities.hangar.active_crate_sub_u();
                let a = &mut self.agents[idx];
                a.yielding = false;

                if cargo_active {
                    let (tgt_pos, tgt_yaw, tgt_lift) = if p < 0.34 {
                        let u = smoothstep(p / 0.34);
                        let pos = lerp3(stage_pos, load_pos, u);
                        let move_yaw =
                            (load_pos[0] - stage_pos[0]).atan2(load_pos[2] - stage_pos[2]);
                        (pos, move_yaw, if a.carrying_cargo { 0.34 } else { 0.08 })
                    } else if p < 0.66 {
                        let u = (p - 0.34) / 0.32;
                        let lift = if u < 0.65 { 0.88 } else { 0.36 };
                        if (0.45..0.55).contains(&u) && a.dwell_timer <= 0.0 {
                            a.carrying_cargo = !a.carrying_cargo;
                            a.cargo_kind = if a.carrying_cargo {
                                CargoVisualKind::RawCrate
                            } else {
                                CargoVisualKind::None
                            };
                            a.dwell_timer = 1.5;
                        }
                        (load_pos, face_ship_yaw, lift)
                    } else {
                        let u = smoothstep((p - 0.66) / 0.34);
                        let pos = lerp3(load_pos, stage_pos, u);
                        let move_yaw =
                            (stage_pos[0] - load_pos[0]).atan2(stage_pos[2] - load_pos[2]);
                        (pos, move_yaw, if a.carrying_cargo { 0.34 } else { 0.08 })
                    };
                    a.work_action = AgentWorkAction::LoadingCrate;
                    a.dwell_timer = (a.dwell_timer - dt).max(0.0);
                    a.pos = [
                        ease_scalar(a.pos[0], tgt_pos[0], 5.5, dt),
                        tgt_pos[1],
                        ease_scalar(a.pos[2], tgt_pos[2], 5.5, dt),
                    ];
                    a.yaw = turn_yaw(a.yaw, tgt_yaw, 5.2, dt);
                    a.lift_height = ease_scalar(a.lift_height, tgt_lift, 4.5, dt);
                } else if zone == AgentZone::HangarBay {
                    // Bespoke 5-minute Hangar Bay Compactor Cycle (stays strictly within Z = 22.0..=31.4!)
                    let side_x = if a.role_slot == 0 { -7.35f32 } else { 7.35f32 };
                    let face_wall_yaw = if a.role_slot == 0 {
                        -std::f32::consts::FRAC_PI_2
                    } else {
                        std::f32::consts::FRAC_PI_2
                    };
                    let rack_pos = [side_x, -3.88, 31.4];
                    let press_pos = [side_x, -3.88, 22.0];
                    let watch_pos = [side_x, -3.88, 26.8];

                    let (tgt_pos, tgt_yaw, tgt_lift, carrying, work) = if let Some(u) = hangar_sub_u
                    {
                        if u < 0.18 {
                            // At Staging Rack (Z=31.4): face wall, lower forks, lift raw crate
                            let lift = if u < 0.10 { 0.08 } else { 0.44 };
                            (
                                rack_pos,
                                face_wall_yaw,
                                lift,
                                u >= 0.16,
                                AgentWorkAction::LoadingCrate,
                            )
                        } else if u < 0.46 {
                            // Drive crate from Staging Rack (31.4) to Hydraulic Compactor (22.0)
                            let t = smoothstep((u - 0.18) / 0.28);
                            let pos = lerp3(rack_pos, press_pos, t);
                            (pos, std::f32::consts::PI, 0.38, true, AgentWorkAction::Idle)
                        } else if u < 0.60 {
                            // Face Compactor hopper, raise forks & load crate in
                            let lift = if u < 0.56 { 0.86 } else { 0.24 };
                            (
                                press_pos,
                                face_wall_yaw,
                                lift,
                                u < 0.56,
                                AgentWorkAction::LoadingCrate,
                            )
                        } else if u < 0.84 {
                            // Hold clear while hydraulic ram crushes the crate into an alloy bale
                            (press_pos, face_wall_yaw, 0.10, false, AgentWorkAction::Idle)
                        } else {
                            // Return to Staging Rack (Z=31.4) for the next crate
                            let t = smoothstep((u - 0.84) / 0.16);
                            let pos = lerp3(press_pos, rack_pos, t);
                            (pos, 0.0, 0.08, false, AgentWorkAction::Idle)
                        }
                    } else {
                        // Gantry Crane Finale (`255..300s`): park at mid-lane watching overhead hoist
                        (watch_pos, face_wall_yaw, 0.08, false, AgentWorkAction::Idle)
                    };
                    a.dwell_timer = 0.0;
                    a.carrying_cargo = carrying;
                    a.cargo_kind = if carrying {
                        CargoVisualKind::RawCrate
                    } else {
                        CargoVisualKind::None
                    };
                    a.work_action = work;
                    a.pos = [
                        ease_scalar(a.pos[0], tgt_pos[0], 4.8, dt),
                        tgt_pos[1],
                        ease_scalar(a.pos[2], tgt_pos[2], 4.8, dt),
                    ];
                    a.yaw = turn_yaw(a.yaw, tgt_yaw, 4.6, dt);
                    a.lift_height = ease_scalar(a.lift_height, tgt_lift, 4.2, dt);
                } else {
                    a.dwell_timer = 0.0;
                    a.work_action = AgentWorkAction::Idle;
                    a.pos = [
                        ease_scalar(a.pos[0], stage_pos[0], 3.5, dt),
                        stage_pos[1],
                        ease_scalar(a.pos[2], stage_pos[2], 3.5, dt),
                    ];
                    a.yaw = turn_yaw(a.yaw, face_stage_yaw, 3.2, dt);
                    a.lift_height = ease_scalar(a.lift_height, 0.06, 3.8, dt);
                }
                all_bodies[idx + 1].1 = self.agents[idx].pos;
                continue;
            }

            // Atrium PanelTechBots: calibrate 12 glowing wall panels across the forward ground half
            // (Z >= +3.5, strictly clear of the main elevator) and Upper U-Mezzanine via the Mini-Elevator!
            if kind == AgentKind::PanelTechBot {
                self.step_panel_tech_bot(idx, dt, &all_bodies);
                all_bodies[idx + 1].1 = self.agents[idx].pos;
                continue;
            }

            // Release Pipe Tunnel mutex if this bot has exited the tunnel
            let cur_pos = self.agents[idx].pos;
            let in_pipe_now =
                (cur_pos[1] - (-3.88)).abs() < 1.5 && (-25.6..=-14.2).contains(&cur_pos[2]);
            if self.pipe_tunnel_owner == Some(id) && !in_pipe_now {
                self.pipe_tunnel_owner = None;
            }

            // 1. Dwelling at a waypoint: perform room-specific work animation (forks, scanner, coolant, probe)
            if self.agents[idx].dwell_timer > 0.0 {
                // If BOT-01 rolls close to a dwelling rover, politely cut the dwell short so it moves aside!
                let walker_close = kind != AgentKind::CargoForklift
                    && (self.walker.pos[1] - cur_pos[1]).abs() < 1.5
                    && ((self.walker.pos[0] - cur_pos[0]).powi(2)
                        + (self.walker.pos[2] - cur_pos[2]).powi(2))
                        < 2.6 * 2.6;
                let a = &mut self.agents[idx];
                if walker_close {
                    a.dwell_timer = 0.0;
                } else {
                    a.dwell_timer = (a.dwell_timer - dt).max(0.0);
                }
                a.yielding = false;
                if let Some(target_yaw) = a.work_target_yaw {
                    a.yaw = turn_yaw(a.yaw, target_yaw, 4.6, dt);
                }
                match kind {
                    AgentKind::CargoForklift => {
                        let target_lift = if a.dwell_timer > 1.0 {
                            0.84
                        } else if a.carrying_cargo {
                            0.32
                        } else {
                            0.06
                        };
                        a.lift_height = ease_scalar(a.lift_height, target_lift, 4.2, dt);
                        if a.dwell_timer <= 0.0 {
                            a.carrying_cargo = !a.carrying_cargo;
                        }
                    }
                    AgentKind::SolarCapacitorBot | AgentKind::ReactorCoolantBot => {
                        // Extend calibration probe / coolant nozzle during inspection dwell
                        let target_ext = if a.dwell_timer > 0.4 { 0.92 } else { 0.1 };
                        a.lift_height = ease_scalar(a.lift_height, target_ext, 4.5, dt);
                    }
                    AgentKind::CourierRunner => {
                        if a.dwell_timer <= 0.0 {
                            a.carrying_cargo = !a.carrying_cargo;
                            a.cargo_kind = if a.carrying_cargo {
                                CargoVisualKind::SolarMicroCell
                            } else {
                                CargoVisualKind::None
                            };
                        }
                    }
                    _ => {
                        a.lift_height = ease_scalar(a.lift_height, 0.1, 4.0, dt);
                    }
                }
                if a.dwell_timer <= 0.0 {
                    if !matches!(kind, AgentKind::CourierRunner | AgentKind::CargoForklift) {
                        a.cargo_kind = a.pending_cargo_kind;
                        a.carrying_cargo = a.cargo_kind != CargoVisualKind::None;
                    }
                    a.work_action = AgentWorkAction::Idle;
                    a.work_target_yaw = None;
                }
                continue;
            }

            // Helper to compute which NavNodes are currently occupied by OTHER robots on the same deck
            let mut occupied = [false; 128];
            for (ni, node) in NAV_NODES.iter().enumerate() {
                if (node.pos[1] - cur_pos[1]).abs() > 1.5 {
                    continue;
                }
                for &(other_id, other_pos, _) in &all_bodies {
                    if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 1.5 {
                        continue;
                    }
                    let ddx = node.pos[0] - other_pos[0];
                    let ddz = node.pos[2] - other_pos[2];
                    if ddx * ddx + ddz * ddz < 2.2 * 2.2 {
                        occupied[ni] = true;
                        break;
                    }
                }
            }

            // 2. Plan a new occupancy-aware A* path on `NAV_NODES` if the current path is empty
            if self.agents[idx].nav_path.is_empty() {
                let cur_node = self.agents[idx].nav_node.min(NAV_NODES.len() - 1);
                let carrying = self.agents[idx].carrying_cargo;
                let role_slot = self.agents[idx].role_slot;
                let cargo_kind = self.agents[idx].cargo_kind;
                let roll = self.rng.next_u64() as usize;

                // Prefer bespoke 5-minute room activity directives when the target workstation is clear
                let directed = self
                    .room_activities
                    .next_directive(kind, zone, role_slot, cur_node, cargo_kind, roll);
                let mut planned = false;
                if let Some(dir) = directed
                    && dir.goal_node != cur_node
                    && !occupied[dir.goal_node]
                {
                    let path =
                        find_path_avoiding(cur_node, dir.goal_node, &occupied[..NAV_NODES.len()]);
                    if !path.is_empty() {
                        let a = &mut self.agents[idx];
                        a.nav_path = path;
                        a.work_action = dir.work_action;
                        a.work_target_yaw = dir.face_yaw;
                        a.pending_cargo_kind = dir.cargo_after_dwell;
                        planned = true;
                    }
                }

                if !planned {
                    let pool = goal_pool_for_agent(kind, zone, cur_node, carrying);
                    if pool.len() > 1 {
                        // Prefer an unoccupied goal node in the pool so bots never queue up on top of each other
                        let mut goal = cur_node;
                        for _ in 0..8 {
                            let pick = pool[(self.rng.next_u64() as usize) % pool.len()];
                            if pick != cur_node && !occupied[pick] {
                                goal = pick;
                                break;
                            }
                        }
                        if goal == cur_node {
                            for _ in 0..6 {
                                let pick = pool[(self.rng.next_u64() as usize) % pool.len()];
                                if pick != cur_node {
                                    goal = pick;
                                    break;
                                }
                            }
                        }
                        if goal != cur_node {
                            let a = &mut self.agents[idx];
                            a.nav_path =
                                find_path_avoiding(cur_node, goal, &occupied[..NAV_NODES.len()]);
                            a.work_action = AgentWorkAction::Idle;
                            a.work_target_yaw = None;
                            a.pending_cargo_kind = a.cargo_kind;
                        }
                    }
                }
            }

            let Some(&next_node_id) = self.agents[idx].nav_path.first() else {
                self.agents[idx].dwell_timer = 0.6;
                continue;
            };
            let next_node = NAV_NODES[next_node_id];
            let is_final_waypoint = self.agents[idx].nav_path.len() == 1;

            // If our final goal node is occupied by another bot right now, immediately pick a different goal!
            if is_final_waypoint
                && kind != AgentKind::CargoForklift
                && occupied[next_node_id]
                && !next_node.is_pipe_tunnel
            {
                self.agents[idx].nav_path.clear();
                continue;
            }

            // 3. Single-lane Axial Pipe Tunnel mutual exclusion (read-only check first; only claim after passing yield!)
            if next_node.is_pipe_tunnel
                && let Some(owner) = self.pipe_tunnel_owner
                && owner != id
            {
                self.agents[idx].yielding = true;
                continue;
            }

            // 4. Forward-cone multi-agent collision avoidance & starboard passing steering
            let target_pos = next_node.pos;
            let dx = target_pos[0] - cur_pos[0];
            let dz = target_pos[2] - cur_pos[2];
            let dist_to_node = (dx * dx + dz * dz).sqrt();
            let dir_x = if dist_to_node > 1e-4 {
                dx / dist_to_node
            } else {
                0.0
            };
            let dir_z = if dist_to_node > 1e-4 {
                dz / dist_to_node
            } else {
                1.0
            };

            let my_r = kind.radius();
            let mut must_yield = false;
            let mut avoid_x = 0.0f32;
            let mut avoid_z = 0.0f32;
            for &(other_id, other_pos, other_r) in &all_bodies {
                if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 2.0 {
                    continue;
                }
                let ox = other_pos[0] - cur_pos[0];
                let oz = other_pos[2] - cur_pos[2];
                let d = (ox * ox + oz * oz).sqrt();
                let safe_r = my_r + other_r + 0.45;
                let min_sep = my_r + other_r + 1.35;
                if d < min_sep && d > 1e-4 {
                    let forward_dot = (ox * dir_x + oz * dir_z) / d;
                    if forward_dot > 0.35 {
                        // A bot already inside the single-lane Pipe Tunnel must finish exiting rather than freezing in the doorway!
                        // Also steer smoothly around BOT-01 when it is parked/holding instead of freezing behind it.
                        if other_id < id && !in_pipe_now && !(other_id == 0 && walker_holding) {
                            must_yield = true;
                            break;
                        } else if !next_node.is_pipe_tunnel && kind != AgentKind::CargoForklift {
                            // Lower-ID bot (or exiting tunnel bot) arcs smoothly around the starboard side
                            let strength =
                                ((min_sep - d) / (min_sep - safe_r).max(0.25)).clamp(0.35, 1.5);
                            let right_x = dir_z;
                            let right_z = -dir_x;
                            avoid_x += right_x * strength;
                            avoid_z += right_z * strength;
                        }
                    }
                    // Soft radial repulsion if another bot is still inside personal buffer
                    if d < safe_r && !next_node.is_pipe_tunnel && kind != AgentKind::CargoForklift {
                        let push = (safe_r - d) / safe_r;
                        avoid_x -= (ox / d) * push * 1.6;
                        avoid_z -= (oz / d) * push * 1.6;
                    }
                }
            }

            if must_yield {
                // Never hold the Pipe Tunnel mutex while yielding outside the tunnel!
                if !in_pipe_now && self.pipe_tunnel_owner == Some(id) {
                    self.pipe_tunnel_owner = None;
                }
                self.agents[idx].yielding = true;
                self.agents[idx].stuck_timer += dt;
                // Anti-deadlock patience: if blocked > 1.3s, dynamically reroute to an unoccupied goal!
                if self.agents[idx].stuck_timer > 1.3 && kind != AgentKind::CargoForklift {
                    self.agents[idx].stuck_timer = 0.0;
                    self.agents[idx].nav_path.clear();
                    // Re-anchor nav_node to whichever NavNode is closest to current position
                    let mut best_n = self.agents[idx].nav_node;
                    let mut best_d2 = f32::INFINITY;
                    for (ni, node) in NAV_NODES.iter().enumerate() {
                        if (node.pos[1] - cur_pos[1]).abs() > 1.5 {
                            continue;
                        }
                        let nd2 =
                            (node.pos[0] - cur_pos[0]).powi(2) + (node.pos[2] - cur_pos[2]).powi(2);
                        if nd2 < best_d2 {
                            best_d2 = nd2;
                            best_n = ni;
                        }
                    }
                    self.agents[idx].nav_node = best_n;
                }
                continue;
            }
            // Safe to claim the Pipe Tunnel mutex now that we are actively advancing into it
            if next_node.is_pipe_tunnel && self.pipe_tunnel_owner.is_none() {
                self.pipe_tunnel_owner = Some(id);
            }
            self.agents[idx].yielding = false;

            // 5. Advance smoothly along the NavGraph edge (with starboard obstacle avoidance)
            let speed = kind.cruise_speed();
            // Intermediate transit waypoints use a generous pass-through radius so bots curving
            // around teammates never get trapped trying to hit the exact center of a busy node!
            let arrive_r = if is_final_waypoint
                || next_node.is_pipe_tunnel
                || kind == AgentKind::CargoForklift
            {
                0.22
            } else {
                1.15
            };
            if dist_to_node > arrive_r {
                let mut move_x = dir_x + avoid_x;
                let mut move_z = dir_z + avoid_z;
                let m_len = (move_x * move_x + move_z * move_z).sqrt();
                if m_len > 1e-4 {
                    move_x /= m_len;
                    move_z /= m_len;
                }
                let step = (speed * dt).min(dist_to_node);
                let mut new_x = cur_pos[0] + move_x * step;
                let mut new_z = cur_pos[2] + move_z * step;
                // Hard post-step separation clamp so no two bots ever overlap even at coarse dt
                // (only exempt inside the narrow Pipe Tunnel walls Z in -24.8..-15.2, where the mutex prevents overlap)
                let in_narrow_pipe_walls =
                    (cur_pos[1] - (-3.88)).abs() < 1.0 && (-24.8..=-15.2).contains(&cur_pos[2]);
                if !in_narrow_pipe_walls && kind != AgentKind::CargoForklift {
                    for &(other_id, other_pos, other_r) in &all_bodies {
                        if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 1.5 {
                            continue;
                        }
                        let rx = new_x - other_pos[0];
                        let rz = new_z - other_pos[2];
                        let rd = (rx * rx + rz * rz).sqrt();
                        let hard_r = my_r + other_r + 0.12;
                        if rd < hard_r && rd > 1e-4 {
                            new_x = other_pos[0] + (rx / rd) * hard_r;
                            new_z = other_pos[2] + (rz / rd) * hard_r;
                        }
                    }
                    // Keep clear of central deck obstacles (Ops Holotable, Reactor Core & Elevator Shaft)
                    if (target_pos[1] - 12.05).abs() < 0.5 {
                        let hx = new_x;
                        let hz = new_z - 1.0;
                        let hd = (hx * hx + hz * hz).sqrt();
                        if hd < 2.95 && hd > 1e-4 {
                            new_x = (hx / hd) * 2.95;
                            new_z = 1.0 + (hz / hd) * 2.95;
                        }
                        let (ex, ez) = clamp_outside_elevator_shaft(new_x, new_z, my_r + 0.08);
                        new_x = ex;
                        new_z = ez;
                    } else if (target_pos[1] - (-14.95)).abs() < 0.5 {
                        let rx = new_x;
                        let rz = new_z - REACTOR_CORE_Z;
                        let hd = (rx * rx + rz * rz).sqrt();
                        if hd < 3.65 && hd > 1e-4 {
                            new_x = (rx / hd) * 3.65;
                            new_z = REACTOR_CORE_Z + (rz / hd) * 3.65;
                        }
                        let (ex, ez) = clamp_outside_elevator_shaft(new_x, new_z, my_r + 0.08);
                        new_x = ex;
                        new_z = ez;
                    } else if (target_pos[1] - (-3.88)).abs() < 0.5 {
                        let (ex, ez) = clamp_outside_elevator_shaft(new_x, new_z, my_r + 0.08);
                        new_x = ex;
                        new_z = ez;
                    }
                }
                let actual_move =
                    ((new_x - cur_pos[0]).powi(2) + (new_z - cur_pos[2]).powi(2)).sqrt();
                let a = &mut self.agents[idx];
                a.pos = [new_x, target_pos[1], new_z];
                if actual_move < speed * dt * 0.15 {
                    a.stuck_timer += dt;
                    if a.stuck_timer > 1.0 && kind != AgentKind::CargoForklift {
                        a.stuck_timer = 0.0;
                        a.nav_path.clear();
                        let mut best_n = a.nav_node;
                        let mut best_d2 = f32::INFINITY;
                        for (ni, node) in NAV_NODES.iter().enumerate() {
                            if (node.pos[1] - cur_pos[1]).abs() > 1.5 {
                                continue;
                            }
                            let nd2 = (node.pos[0] - new_x).powi(2) + (node.pos[2] - new_z).powi(2);
                            if nd2 < best_d2 {
                                best_d2 = nd2;
                                best_n = ni;
                            }
                        }
                        a.nav_node = best_n;
                    }
                } else {
                    a.stuck_timer = 0.0;
                }
                let travel_yaw = move_x.atan2(move_z);
                a.yaw = turn_yaw(a.yaw, travel_yaw, 5.2, dt);
                let cruise_lift = if kind == AgentKind::CargoForklift && a.carrying_cargo {
                    0.32
                } else {
                    0.06
                };
                a.lift_height = ease_scalar(a.lift_height, cruise_lift, 4.0, dt);
            } else {
                let a = &mut self.agents[idx];
                if is_final_waypoint {
                    a.pos = target_pos;
                }
                a.nav_node = next_node_id;
                a.nav_path.remove(0);
                if a.nav_path.is_empty() {
                    a.dwell_timer = match a.work_action {
                        AgentWorkAction::SwappingCryoRod => 3.2,
                        AgentWorkAction::UploadingHolo => 3.0,
                        AgentWorkAction::CalibratingCapacitor
                        | AgentWorkAction::CalibratingPanel => 2.8,
                        AgentWorkAction::Scanning => 2.8,
                        AgentWorkAction::DownloadingData => 2.6,
                        AgentWorkAction::Scrubbing | AgentWorkAction::PurgingWaste => 2.4,
                        AgentWorkAction::LoadingCrate => 2.2,
                        AgentWorkAction::Celebrating => 0.4,
                        AgentWorkAction::Idle => match kind {
                            AgentKind::CargoForklift => 2.2,
                            AgentKind::SolarCapacitorBot | AgentKind::ReactorCoolantBot => 2.4,
                            AgentKind::OpsHoloDroid => 2.0,
                            AgentKind::SecurityEnforcer => 1.8,
                            AgentKind::FloorScrubber => 1.2,
                            AgentKind::PanelTechBot => 1.0,
                            AgentKind::CourierRunner => 1.1,
                            AgentKind::HullWelder => 0.0,
                        },
                    };
                }
            }

            // Refresh body snapshot so subsequent agents in this tick see the updated position
            all_bodies[idx + 1].1 = self.agents[idx].pos;
        }
    }

    fn step_panel_tech_bot(&mut self, idx: usize, dt: f32, all_bodies: &[(u32, [f32; 3], f32)]) {
        use std::f32::consts::{FRAC_PI_2, PI};

        use super::activities::{
            ATRIUM_LIFT_HIGH_NODE, ATRIUM_LIFT_HIGH_Y, ATRIUM_LIFT_LOW_NODE, ATRIUM_LIFT_LOW_Y,
            ATRIUM_LIFT_X, ATRIUM_LIFT_Z, ATRIUM_PANEL_WORK_SECS, ATRIUM_PANELS, MiniLiftPhase,
            PanelStatus,
        };
        use super::nav::{NAV_NODES, find_path_avoiding};

        let turn_yaw = |curr: f32, target: f32, rate: f32, dt: f32| -> f32 {
            let mut diff = (target - curr).rem_euclid(TAU);
            if diff > PI {
                diff -= TAU;
            }
            curr + diff * (1.0 - (-rate * dt).exp())
        };

        let id = self.agents[idx].id;
        let speed = AgentKind::PanelTechBot.cruise_speed();
        let my_r = AgentKind::PanelTechBot.radius();

        // 1. Active Mini-Elevator ride (always finishes cleanly even if celebration just started)
        if self.room_activities.atrium.mini_lift.rider_id == Some(id) {
            let lift = &mut self.room_activities.atrium.mini_lift;
            let a = &mut self.agents[idx];
            a.yielding = false;
            a.work_action = AgentWorkAction::Idle;
            a.dwell_timer = 0.0;
            a.lift_height = ease_scalar(a.lift_height, 0.1, 4.5, dt);

            let pickup_y = if lift.target_upper {
                ATRIUM_LIFT_LOW_Y
            } else {
                ATRIUM_LIFT_HIGH_Y
            };
            let dest_y = if lift.target_upper {
                ATRIUM_LIFT_HIGH_Y
            } else {
                ATRIUM_LIFT_LOW_Y
            };

            match lift.phase {
                MiniLiftPhase::Idle => {
                    lift.phase = MiniLiftPhase::MovingToPickup;
                }
                MiniLiftPhase::MovingToPickup => {
                    let dy = pickup_y - lift.current_y;
                    let step = 3.8 * dt;
                    if dy.abs() <= step {
                        lift.current_y = pickup_y;
                        lift.phase = MiniLiftPhase::Boarding;
                    } else {
                        lift.current_y += dy.signum() * step;
                    }
                    let face_lift = if lift.target_upper { FRAC_PI_2 } else { 0.0 };
                    a.yaw = turn_yaw(a.yaw, face_lift, 5.2, dt);
                }
                MiniLiftPhase::Boarding => {
                    let dx = ATRIUM_LIFT_X - a.pos[0];
                    let dz = ATRIUM_LIFT_Z - a.pos[2];
                    let d = (dx * dx + dz * dz).sqrt();
                    if d <= 0.14 {
                        a.pos = [ATRIUM_LIFT_X, pickup_y, ATRIUM_LIFT_Z];
                        lift.phase = MiniLiftPhase::Riding;
                    } else {
                        let s = (speed * dt).min(d);
                        a.pos[0] += (dx / d) * s;
                        a.pos[1] = pickup_y;
                        a.pos[2] += (dz / d) * s;
                        a.yaw = turn_yaw(a.yaw, dx.atan2(dz), 6.0, dt);
                    }
                }
                MiniLiftPhase::Riding => {
                    let dy = dest_y - lift.current_y;
                    let step = 3.6 * dt;
                    if dy.abs() <= step {
                        lift.current_y = dest_y;
                        lift.phase = MiniLiftPhase::Exiting;
                    } else {
                        lift.current_y += dy.signum() * step;
                    }
                    a.pos = [ATRIUM_LIFT_X, lift.current_y, ATRIUM_LIFT_Z];
                    let exit_yaw = if lift.target_upper { PI } else { -FRAC_PI_2 };
                    a.yaw = turn_yaw(a.yaw, exit_yaw, 4.5, dt);
                }
                MiniLiftPhase::Exiting => {
                    let exit_node = if lift.target_upper {
                        ATRIUM_LIFT_HIGH_NODE
                    } else {
                        ATRIUM_LIFT_LOW_NODE
                    };
                    let tgt = NAV_NODES[exit_node].pos;
                    let dx = tgt[0] - a.pos[0];
                    let dz = tgt[2] - a.pos[2];
                    let d = (dx * dx + dz * dz).sqrt();
                    if d <= 0.16 {
                        a.pos = tgt;
                        a.nav_node = exit_node;
                        a.nav_path.clear();
                        lift.rider_id = None;
                        lift.phase = MiniLiftPhase::Idle;
                    } else {
                        let s = (speed * dt).min(d);
                        a.pos[0] += (dx / d) * s;
                        a.pos[1] = dest_y;
                        a.pos[2] += (dz / d) * s;
                        a.yaw = turn_yaw(a.yaw, dx.atan2(dz), 6.0, dt);
                    }
                }
            }
            return;
        }

        // 2. Whole-room Celebration Finale: spin & bounce joyfully in place!
        if self.room_activities.atrium.celebrating {
            let a = &mut self.agents[idx];
            a.nav_path.clear();
            a.yielding = false;
            a.work_action = AgentWorkAction::Celebrating;
            a.dwell_timer = 0.25;
            a.yaw = (a.yaw + dt * 3.0).rem_euclid(TAU);
            let target_lift = 0.55 + 0.35 * (a.phase * 1.8).sin().abs();
            a.lift_height = ease_scalar(a.lift_height, target_lift, 6.0, dt);
            return;
        }

        // 3. Ensure this robot has an active unclaimed Pending panel assigned
        let cur_pos = self.agents[idx].pos;
        let on_upper = cur_pos[1] > 0.0;

        let mut assigned_panel = self
            .room_activities
            .atrium
            .panels
            .iter()
            .position(|p| p.assigned_bot == Some(id) && p.status != PanelStatus::Fixed);

        if assigned_panel.is_none() {
            let pick_closest = |want_upper: bool| -> Option<usize> {
                let mut best: Option<(usize, f32)> = None;
                for (pi, (spec, rt)) in ATRIUM_PANELS
                    .iter()
                    .zip(self.room_activities.atrium.panels.iter())
                    .enumerate()
                {
                    if rt.status != PanelStatus::Pending
                        || rt.assigned_bot.is_some()
                        || spec.upper_level != want_upper
                    {
                        continue;
                    }
                    let wp = NAV_NODES[spec.work_node].pos;
                    let d2 = (wp[0] - cur_pos[0]).powi(2) + (wp[2] - cur_pos[2]).powi(2);
                    if best.is_none_or(|(_, bd2)| d2 < bd2) {
                        best = Some((pi, d2));
                    }
                }
                best.map(|(pi, _)| pi)
            };

            let chosen = pick_closest(on_upper).or_else(|| pick_closest(!on_upper));
            if let Some(pi) = chosen {
                self.room_activities.atrium.panels[pi].assigned_bot = Some(id);
                assigned_panel = Some(pi);
                self.agents[idx].nav_path.clear();
            }
        }

        let Some(panel_idx) = assigned_panel else {
            // All panels are currently claimed by teammates or fixed; check celebration trigger
            if self.room_activities.atrium.all_fixed() {
                self.room_activities.atrium.trigger_celebration();
            } else {
                self.agents[idx].work_action = AgentWorkAction::Idle;
                self.agents[idx].dwell_timer = 0.0;
            }
            return;
        };

        let spec = ATRIUM_PANELS[panel_idx];
        let need_lift = spec.upper_level != on_upper;
        let goal_node = if need_lift {
            if on_upper {
                ATRIUM_LIFT_HIGH_NODE
            } else {
                ATRIUM_LIFT_LOW_NODE
            }
        } else {
            spec.work_node
        };

        let goal_pos = NAV_NODES[goal_node].pos;
        let dist_to_goal =
            ((goal_pos[0] - cur_pos[0]).powi(2) + (goal_pos[2] - cur_pos[2]).powi(2)).sqrt();

        // 4. Arrived at goal: either board the Mini-Elevator or calibrate the control panel!
        if dist_to_goal <= 0.34 {
            if need_lift {
                let lift = &mut self.room_activities.atrium.mini_lift;
                let a = &mut self.agents[idx];
                a.nav_node = goal_node;
                a.nav_path.clear();
                a.work_action = AgentWorkAction::Idle;
                a.dwell_timer = 0.0;
                if lift.rider_id.is_none() {
                    lift.rider_id = Some(id);
                    lift.target_upper = spec.upper_level;
                    lift.phase = MiniLiftPhase::MovingToPickup;
                } else {
                    a.yielding = true;
                    let wait_yaw = if on_upper { 0.0 } else { FRAC_PI_2 };
                    a.yaw = turn_yaw(a.yaw, wait_yaw, 4.5, dt);
                }
                return;
            }

            // Actively calibrating this control panel!
            {
                let a = &mut self.agents[idx];
                a.pos = goal_pos;
                a.nav_node = goal_node;
                a.nav_path.clear();
                a.yielding = false;
                a.work_action = AgentWorkAction::CalibratingPanel;
                a.work_target_yaw = Some(spec.face_yaw);
                a.dwell_timer = 0.25;
                a.yaw = turn_yaw(a.yaw, spec.face_yaw, 6.2, dt);
                a.lift_height = ease_scalar(a.lift_height, 0.88, 5.2, dt);
            }

            let panel = &mut self.room_activities.atrium.panels[panel_idx];
            panel.status = PanelStatus::Working;
            panel.progress = (panel.progress + dt / ATRIUM_PANEL_WORK_SECS).min(1.0);
            if panel.progress >= 1.0 {
                panel.status = PanelStatus::Fixed;
                panel.assigned_bot = None;
                let a = &mut self.agents[idx];
                a.work_action = AgentWorkAction::Idle;
                a.work_target_yaw = None;
                a.dwell_timer = 0.0;
                a.lift_height = 0.1;
                if self.room_activities.atrium.all_fixed() {
                    self.room_activities.atrium.trigger_celebration();
                }
            }
            return;
        }

        // 5. Navigate smoothly along NAV_NODES on our current floor (never touching aft elevator nodes!)
        let mut occupied = [false; 128];
        // Strictly forbid PanelTechBots from routing through aft Atrium elevator nodes (13, 14, 15)
        occupied[13] = true;
        occupied[14] = true;
        occupied[15] = true;
        for (ni, node) in NAV_NODES.iter().enumerate() {
            if (node.pos[1] - cur_pos[1]).abs() > 1.5 {
                continue;
            }
            for &(other_id, other_pos, _) in all_bodies {
                if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 1.5 {
                    continue;
                }
                let ddx = node.pos[0] - other_pos[0];
                let ddz = node.pos[2] - other_pos[2];
                if ddx * ddx + ddz * ddz < 2.0 * 2.0 {
                    occupied[ni] = true;
                    break;
                }
            }
        }

        if self.agents[idx].nav_path.is_empty()
            || self.agents[idx].nav_path.last().copied() != Some(goal_node)
        {
            // Re-anchor nav_node to closest node on our current floor
            let mut best_n = self.agents[idx].nav_node.min(NAV_NODES.len() - 1);
            let mut best_d2 = f32::INFINITY;
            for (ni, node) in NAV_NODES.iter().enumerate() {
                if (node.pos[1] - cur_pos[1]).abs() > 1.5 {
                    continue;
                }
                if !on_upper && matches!(ni, 13..=15) {
                    continue;
                }
                let nd2 = (node.pos[0] - cur_pos[0]).powi(2) + (node.pos[2] - cur_pos[2]).powi(2);
                if nd2 < best_d2 {
                    best_d2 = nd2;
                    best_n = ni;
                }
            }
            self.agents[idx].nav_node = best_n;
            if best_n == goal_node {
                self.agents[idx].nav_path = vec![goal_node];
            } else {
                let mut path = find_path_avoiding(best_n, goal_node, &occupied[..NAV_NODES.len()]);
                if path.is_empty() {
                    path = vec![goal_node];
                }
                self.agents[idx].nav_path = path;
            }
        }

        let Some(&next_node_id) = self.agents[idx].nav_path.first() else {
            return;
        };
        let target_pos = NAV_NODES[next_node_id].pos;
        let is_final = self.agents[idx].nav_path.len() == 1;
        let dx = target_pos[0] - cur_pos[0];
        let dz = target_pos[2] - cur_pos[2];
        let dist_to_node = (dx * dx + dz * dz).sqrt();
        // Intermediate radius > pairwise separation (1.22) so a bot cleanly curves past a teammate on a catwalk
        let arrive_r = if is_final { 0.24 } else { 1.65 };

        if dist_to_node <= arrive_r {
            let a = &mut self.agents[idx];
            if is_final {
                a.pos = target_pos;
            }
            a.nav_node = next_node_id;
            a.nav_path.remove(0);
            return;
        }

        let dir_x = dx / dist_to_node.max(1e-4);
        let dir_z = dz / dist_to_node.max(1e-4);

        let mut avoid_x = 0.0f32;
        let mut avoid_z = 0.0f32;
        for &(other_id, other_pos, other_r) in all_bodies {
            if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 1.5 {
                continue;
            }
            let ox = other_pos[0] - cur_pos[0];
            let oz = other_pos[2] - cur_pos[2];
            let d = (ox * ox + oz * oz).sqrt();
            let safe_r = my_r + other_r + 0.35;
            let min_sep = my_r + other_r + 1.15;
            if d < min_sep && d > 1e-4 {
                let forward_dot = (ox * dir_x + oz * dir_z) / d;
                if forward_dot > 0.3 {
                    let strength = ((min_sep - d) / (min_sep - safe_r).max(0.25)).clamp(0.35, 1.4);
                    avoid_x += dir_z * strength;
                    avoid_z -= dir_x * strength;
                }
                if d < safe_r {
                    let push = (safe_r - d) / safe_r;
                    avoid_x -= (ox / d) * push * 1.5;
                    avoid_z -= (oz / d) * push * 1.5;
                }
            }
        }

        let mut move_x = dir_x + avoid_x;
        let mut move_z = dir_z + avoid_z;
        let m_len = (move_x * move_x + move_z * move_z).sqrt();
        if m_len > 1e-4 {
            move_x /= m_len;
            move_z /= m_len;
        }
        let step = (speed * dt).min(dist_to_node);
        let mut new_x = cur_pos[0] + move_x * step;
        let mut new_z = cur_pos[2] + move_z * step;

        // Hard post-step separation from other same-level robots
        for &(other_id, other_pos, other_r) in all_bodies {
            if other_id == id || (other_pos[1] - cur_pos[1]).abs() > 1.5 {
                continue;
            }
            let rx = new_x - other_pos[0];
            let rz = new_z - other_pos[2];
            let rd = (rx * rx + rz * rz).sqrt();
            let hard_r = my_r + other_r + 0.12;
            if rd < hard_r && rd > 1e-4 {
                new_x = other_pos[0] + (rx / rd) * hard_r;
                new_z = other_pos[2] + (rz / rd) * hard_r;
            }
        }

        if on_upper {
            // Keep strictly inside the U-Mezzanine walkway bounds (Starboard, Rear Skybridge, Port)
            if cur_pos[2] > -10.85 {
                // Currently on the Starboard or Port side catwalk: keep X on this catwalk while allowing Z into the skybridge
                if cur_pos[0] >= 0.0 {
                    new_x = new_x.clamp(7.25, 10.35);
                } else {
                    new_x = new_x.clamp(-10.35, -7.25);
                }
                new_z = new_z.clamp(-13.15, 6.7);
            } else {
                // Currently in the Rear Upper Skybridge strip (`Z <= -10.85`): allow full X, and exit onto side catwalks once |X| >= 7.25
                new_x = new_x.clamp(-10.35, 10.35);
                if new_x.abs() < 7.25 {
                    new_z = new_z.clamp(-13.15, -10.85);
                } else {
                    new_z = new_z.clamp(-13.15, 6.7);
                }
            }
        } else {
            // Keep strictly in the elevator-free forward ground half!
            new_x = new_x.clamp(-10.1, 10.1);
            new_z = new_z.clamp(2.2, 9.6);
        }

        let a = &mut self.agents[idx];
        a.pos = [new_x, target_pos[1], new_z];
        a.yaw = turn_yaw(a.yaw, move_x.atan2(move_z), 5.6, dt);
        a.work_action = AgentWorkAction::Idle;
        a.dwell_timer = 0.0;
        a.yielding = false;
        a.lift_height = ease_scalar(a.lift_height, 0.1, 4.2, dt);
    }

    fn step_walker_and_elevator(&mut self, dt: f32) {
        use std::f32::consts::{FRAC_PI_2, PI};

        #[derive(Clone, Copy)]
        enum WalkerLeg {
            Walk {
                target: [f32; 3],
                dwell: f32,
                look_yaw: Option<f32>,
                room: &'static str,
            },
            WaitPadClear {
                pad: LandingPadId,
                look_yaw: f32,
                room: &'static str,
            },
            CycleAirlock {
                airlock_idx: usize,
                depressurize: bool,
                pad_check: Option<LandingPadId>,
                room: &'static str,
            },
            WaitElevator {
                deck: ElevatorDeck,
                room: &'static str,
            },
            RideElevator {
                target_deck: ElevatorDeck,
                room: &'static str,
            },
        }

        const D1: f32 = -14.95; // Deck 1: Concourse & Reactor
        const D2: f32 = -3.85; // Deck 2: Hangar, Central Atrium, Pipe Tunnel, Solar Bay
        const D3: f32 = 12.05; // Deck 3: Ops & Command Spire
        const PAD: f32 = -1.42; // External Landing Pads B & C & Airlock Chambers

        const ROUTE: &[WalkerLeg] = &[
            // 0..=4: Main Hangar Bay & Pad A inspection (waits behind safety line until Pad A is clear!)
            WalkerLeg::Walk {
                target: [0.0, D2, 17.5],
                dwell: 1.6,
                look_yaw: Some(0.0), // face +Z toward Pad A & cyan forcefield
                room: "MAIN HANGAR BAY",
            },
            WalkerLeg::WaitPadClear {
                pad: LandingPadId::PadAHangar,
                look_yaw: 0.0,
                room: "PAD-A HOLD // TRAFFIC",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, 25.5],
                dwell: 2.2,
                look_yaw: Some(0.0), // inspect Pad A deck & open containment forcefield
                room: "PAD-A DECK INSPECTION",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, 17.5],
                dwell: 0.0,
                look_yaw: None,
                room: "MAIN HANGAR BAY",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, 13.6],
                dwell: 0.3,
                look_yaw: None,
                room: "HANGAR AIRLOCK",
            },
            // 5..=9: Central Core Atrium -> Ride 3-deck glass Elevator UP to Deck 3 (Ops Command Spire)
            WalkerLeg::Walk {
                target: [0.0, D2, 3.8],
                dwell: 1.4,
                look_yaw: Some(PI - 0.35),
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -1.5],
                dwell: 0.0,
                look_yaw: Some(PI),
                room: "DECK-2 ELEVATOR LOBBY",
            },
            WalkerLeg::WaitElevator {
                deck: ElevatorDeck::Deck2Atrium,
                room: "BOARDING ELEVATOR",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -6.0],
                dwell: 0.5,
                look_yaw: Some(0.0), // face +Z out the glass elevator doors
                room: "ELEVATOR CAR",
            },
            WalkerLeg::RideElevator {
                target_deck: ElevatorDeck::Deck3Ops,
                room: "ELEVATOR -> DECK-3 OPS",
            },
            // 10..=18: Patrol Ops Command Deck (wide perimeter loop clear of Central Holotable) -> Ride DOWN to Deck 1
            WalkerLeg::Walk {
                target: [0.0, D3, -2.2],
                dwell: 0.0,
                look_yaw: None,
                room: "OPS COMMAND DECK",
            },
            WalkerLeg::Walk {
                target: [-5.2, D3, -2.0],
                dwell: 0.0,
                look_yaw: None,
                room: "OPS COMMAND DECK",
            },
            WalkerLeg::Walk {
                target: [-5.2, D3, 4.8],
                dwell: 2.2,
                look_yaw: Some(0.22), // look out +Z armored viewports at ships & space!
                room: "OPS DECK VIEWPORT",
            },
            WalkerLeg::Walk {
                target: [5.2, D3, 4.8],
                dwell: 1.6,
                look_yaw: Some(-0.25),
                room: "OPS COMMAND DECK",
            },
            WalkerLeg::Walk {
                target: [5.2, D3, -2.0],
                dwell: 0.0,
                look_yaw: None,
                room: "OPS COMMAND DECK",
            },
            WalkerLeg::Walk {
                target: [0.0, D3, -2.2],
                dwell: 0.0,
                look_yaw: None,
                room: "OPS ELEVATOR LOBBY",
            },
            WalkerLeg::WaitElevator {
                deck: ElevatorDeck::Deck3Ops,
                room: "BOARDING ELEVATOR",
            },
            WalkerLeg::Walk {
                target: [0.0, D3, -6.0],
                dwell: 0.5,
                look_yaw: Some(0.0),
                room: "ELEVATOR CAR",
            },
            WalkerLeg::RideElevator {
                target_deck: ElevatorDeck::Deck1Reactor,
                room: "ELEVATOR -> DECK-1 REACTOR",
            },
            // 19..=29: Deck 1 Reactor Room — orthogonal exit cleanly clear of Elevator Shaft & shifted Reactor Core (Z=+1.5)
            WalkerLeg::Walk {
                target: [0.0, D1, -2.30],
                dwell: 0.0,
                look_yaw: None,
                room: "REACTOR CHAMBER",
            },
            WalkerLeg::Walk {
                target: [4.8, D1, -2.30],
                dwell: 0.0,
                look_yaw: None,
                room: "REACTOR CATWALK",
            },
            WalkerLeg::Walk {
                target: [5.6, D1, 1.5],
                dwell: 0.0,
                look_yaw: None,
                room: "REACTOR CATWALK",
            },
            WalkerLeg::Walk {
                target: [5.6, D1, 5.4],
                dwell: 2.0,
                look_yaw: Some(-0.7), // look back across pulsing plasma core & space window
                room: "CONCOURSE & REACTOR",
            },
            WalkerLeg::Walk {
                target: [-5.6, D1, 5.4],
                dwell: 1.4,
                look_yaw: Some(0.55),
                room: "DECK-1 CONCOURSE",
            },
            WalkerLeg::Walk {
                target: [-5.6, D1, 1.5],
                dwell: 0.0,
                look_yaw: None,
                room: "REACTOR CATWALK",
            },
            WalkerLeg::Walk {
                target: [-4.8, D1, -2.30],
                dwell: 0.0,
                look_yaw: None,
                room: "REACTOR CATWALK",
            },
            WalkerLeg::Walk {
                target: [0.0, D1, -2.30],
                dwell: 0.0,
                look_yaw: None,
                room: "DECK-1 ELEVATOR LOBBY",
            },
            WalkerLeg::WaitElevator {
                deck: ElevatorDeck::Deck1Reactor,
                room: "BOARDING ELEVATOR",
            },
            WalkerLeg::Walk {
                target: [0.0, D1, -6.0],
                dwell: 0.5,
                look_yaw: Some(0.0),
                room: "ELEVATOR CAR",
            },
            WalkerLeg::RideElevator {
                target_deck: ElevatorDeck::Deck2Atrium,
                room: "ELEVATOR -> DECK-2 ATRIUM",
            },
            // 30..=41: Exit Elevator -> Port Pad B Sloped Hallway & Pressurized Airlock -> Inspect Pad B -> Return
            WalkerLeg::Walk {
                target: [0.0, D2, -1.5],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [-4.8, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "ATRIUM PORT ACCESS",
            },
            WalkerLeg::Walk {
                target: [-12.2, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "PORT PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [-21.2, PAD, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "PORT PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [-25.9, PAD, 0.0],
                dwell: 0.2,
                look_yaw: Some(-FRAC_PI_2),
                room: "PORT AIRLOCK",
            },
            WalkerLeg::CycleAirlock {
                airlock_idx: 0,
                depressurize: true,
                pad_check: Some(LandingPadId::PadBPort),
                room: "PORT AIRLOCK",
            },
            WalkerLeg::Walk {
                target: [-35.8, PAD, 0.0],
                dwell: 2.4,
                look_yaw: Some(-FRAC_PI_2), // inspect Port Freight Deck & deep space!
                room: "PAD-B PORT INSPECTION",
            },
            WalkerLeg::Walk {
                target: [-25.9, PAD, 0.0],
                dwell: 0.2,
                look_yaw: Some(FRAC_PI_2),
                room: "PORT AIRLOCK",
            },
            WalkerLeg::CycleAirlock {
                airlock_idx: 0,
                depressurize: false,
                pad_check: None,
                room: "PORT AIRLOCK",
            },
            WalkerLeg::Walk {
                target: [-21.2, PAD, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "PORT PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [-12.2, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "PORT PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [-4.8, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            // 42..=52: Starboard Pad C Sloped Hallway & Pressurized Airlock -> Inspect Pad C -> Return
            WalkerLeg::Walk {
                target: [4.8, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "ATRIUM STBD ACCESS",
            },
            WalkerLeg::Walk {
                target: [12.2, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "STBD PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [21.2, PAD, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "STBD PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [25.9, PAD, 0.0],
                dwell: 0.2,
                look_yaw: Some(FRAC_PI_2),
                room: "STBD AIRLOCK",
            },
            WalkerLeg::CycleAirlock {
                airlock_idx: 1,
                depressurize: true,
                pad_check: Some(LandingPadId::PadCStarboard),
                room: "STBD AIRLOCK",
            },
            WalkerLeg::Walk {
                target: [35.8, PAD, 0.0],
                dwell: 2.4,
                look_yaw: Some(FRAC_PI_2), // inspect Starboard Utility Deck & orbit!
                room: "PAD-C STBD INSPECTION",
            },
            WalkerLeg::Walk {
                target: [25.9, PAD, 0.0],
                dwell: 0.2,
                look_yaw: Some(-FRAC_PI_2),
                room: "STBD AIRLOCK",
            },
            WalkerLeg::CycleAirlock {
                airlock_idx: 1,
                depressurize: false,
                pad_check: None,
                room: "STBD AIRLOCK",
            },
            WalkerLeg::Walk {
                target: [21.2, PAD, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "STBD PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [12.2, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "STBD PAD CORRIDOR",
            },
            WalkerLeg::Walk {
                target: [4.8, D2, 0.0],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            // 53..=58: Orthogonal route around Atrium Elevator Shaft -> Axial Pipe Tunnel -> Solar Bay
            WalkerLeg::Walk {
                target: [4.8, D2, -11.2],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -11.2],
                dwell: 0.3,
                look_yaw: None,
                room: "AFT AXIAL AIRLOCK",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -20.0],
                dwell: 2.2,
                look_yaw: Some(FRAC_PI_2), // look right out side porthole at rotating ring spokes!
                room: "AXIAL PIPE TUNNEL",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -28.0],
                dwell: 0.0,
                look_yaw: None,
                room: "SOLAR ENGINEERING BAY",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -37.0],
                dwell: 2.4,
                look_yaw: Some(-FRAC_PI_2), // look out huge viewport at glowing solar wings!
                room: "SOLAR ENGINEERING BAY",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, -28.0],
                dwell: 0.0,
                look_yaw: None,
                room: "SOLAR ENGINEERING BAY",
            },
            // 59..=63: Return through Pipe Tunnel & West Atrium Corridor (orthogonal around Elevator Shaft) to Hangar
            WalkerLeg::Walk {
                target: [0.0, D2, -11.2],
                dwell: 0.0,
                look_yaw: None,
                room: "AXIAL PIPE TUNNEL",
            },
            WalkerLeg::Walk {
                target: [-4.8, D2, -11.2],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [-4.8, D2, -1.5],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, 3.8],
                dwell: 0.0,
                look_yaw: None,
                room: "CENTRAL CORE ATRIUM",
            },
            WalkerLeg::Walk {
                target: [0.0, D2, 13.8],
                dwell: 0.0,
                look_yaw: None,
                room: "HANGAR AIRLOCK",
            },
        ];

        let turn_yaw = |curr: f32, target: f32, rate: f32, dt: f32| -> f32 {
            let mut diff = (target - curr).rem_euclid(TAU);
            if diff > PI {
                diff -= TAU;
            }
            curr + diff * (1.0 - (-rate * dt).exp())
        };

        let idx = self.walker.route_index % ROUTE.len();
        let leg = ROUTE[idx];
        let mut want_elev_door_open = false;
        let mut want_airlock_inner = [false; 2];
        let mut want_airlock_outer = [false; 2];
        let mut airlock_cycling = [false; 2];
        let mut target_pitch = 0.0f32;

        match leg {
            WalkerLeg::Walk {
                target,
                dwell,
                look_yaw,
                room,
            } => {
                self.walker.riding_elevator = false;
                self.walker.current_room = room.to_string();

                let using_elevator_shaft = (target[0].abs() < 2.4
                    && (target[2] - (-6.0)).abs() < 2.8)
                    || (self.walker.pos[0].abs() < 2.5
                        && (self.walker.pos[2] - (-6.0)).abs() < 2.8);

                // Keep elevator doors open while stepping in or out of the shaft
                if using_elevator_shaft
                    || ((self.walker.pos[2] - (-6.0)).abs() < 4.8 && self.walker.pos[0].abs() < 2.4)
                {
                    want_elev_door_open = true;
                }

                // Proximity triggers for Port (`[0]`) and Starboard (`[1]`) Pad Airlock doors
                let on_deck2_or_pad = self.walker.pos[1] > -9.0 && self.walker.pos[1] < 5.0;
                if on_deck2_or_pad && self.walker.pos[2].abs() < 3.0 {
                    let ax = self.walker.pos[0].abs();
                    let air_idx = if self.walker.pos[0] < 0.0 { 0 } else { 1 };
                    if (18.5..=24.8).contains(&ax) {
                        want_airlock_inner[air_idx] = true;
                    } else if (26.6..=32.5).contains(&ax) {
                        want_airlock_outer[air_idx] = true;
                    }
                }

                let walker_in_pipe = (self.walker.pos[1] - D2).abs() < 1.0
                    && (-25.6..=-14.2).contains(&self.walker.pos[2]);
                let heading_into_pipe = (target[1] - D2).abs() < 1.0
                    && ((-25.6..=-14.2).contains(&target[2])
                        || (self.walker.pos[2] < -25.6 && target[2] > -14.2)
                        || (self.walker.pos[2] > -14.2 && target[2] < -25.6));
                let other_physically_in_pipe = self.agents.iter().any(|a| {
                    a.kind != AgentKind::HullWelder
                        && (a.pos[1] - D2).abs() < 1.2
                        && (-25.6..=-14.2).contains(&a.pos[2])
                });
                let waiting_for_pipe =
                    heading_into_pipe && !walker_in_pipe && other_physically_in_pipe;
                if waiting_for_pipe {
                    self.walker.current_room = "AFT AIRLOCK (YIELDING)".to_string();
                    self.walker.pos[0] = ease_scalar(self.walker.pos[0], 2.1, 3.2, dt);
                } else if heading_into_pipe {
                    self.pipe_tunnel_owner = Some(0);
                }

                let dx = target[0] - self.walker.pos[0];
                let dz = target[2] - self.walker.pos[2];
                let dist = (dx * dx + dz * dz).sqrt();
                let walk_speed = if waiting_for_pipe { 0.0 } else { 3.8 };
                let in_pad_corridor = on_deck2_or_pad
                    && self.walker.pos[0].abs() > 10.5
                    && self.walker.pos[2].abs() < 3.0;
                let in_confined_shaft = using_elevator_shaft
                    || in_pad_corridor
                    || ((self.walker.pos[1] - D2).abs() < 1.0
                        && (-25.6..=-14.2).contains(&self.walker.pos[2]));
                let target_occupied = !in_confined_shaft
                    && self.agents.iter().any(|a| {
                        a.kind != AgentKind::HullWelder
                            && (a.pos[1] - target[1]).abs() < 1.5
                            && ((a.pos[0] - target[0]).powi(2) + (a.pos[2] - target[2]).powi(2))
                                < 2.1 * 2.1
                    });
                let arrive_r = if in_confined_shaft {
                    0.22
                } else if target_occupied {
                    2.1
                } else if dwell > 0.0 {
                    0.45
                } else {
                    0.65
                };

                if !waiting_for_pipe && dist > arrive_r {
                    self.walker.leg_timer += dt;
                    // Safety net: if BOT-01 is ever wedged on a non-shaft leg > 7.5s, advance cleanly
                    if !in_confined_shaft && self.walker.leg_timer > 7.5 {
                        self.walker.leg_timer = 0.0;
                        self.walker.dwell_timer = 0.0;
                        self.walker.route_index = (idx + 1) % ROUTE.len();
                        return;
                    }
                }

                if dist > arrive_r {
                    let dir_x = dx / dist;
                    let dir_z = dz / dist;
                    let mut avoid_x = 0.0f32;
                    let mut avoid_z = 0.0f32;
                    if !in_confined_shaft {
                        for a in &self.agents {
                            if a.kind == AgentKind::HullWelder
                                || (a.pos[1] - self.walker.pos[1]).abs() > 1.8
                            {
                                continue;
                            }
                            let ox = a.pos[0] - self.walker.pos[0];
                            let oz = a.pos[2] - self.walker.pos[2];
                            let d = (ox * ox + oz * oz).sqrt();
                            let safe_r = 0.85 + a.kind.radius() + 0.45;
                            let min_sep = 0.85 + a.kind.radius() + 1.4;
                            if d < min_sep && d > 1e-4 {
                                let forward_dot = (ox * dir_x + oz * dir_z) / d;
                                if forward_dot > 0.2 {
                                    let strength = ((min_sep - d) / (min_sep - safe_r).max(0.25))
                                        .clamp(0.35, 1.5);
                                    avoid_x += dir_z * strength;
                                    avoid_z += -dir_x * strength;
                                }
                                if d < safe_r {
                                    let push = (safe_r - d) / safe_r;
                                    avoid_x -= (ox / d) * push * 1.6;
                                    avoid_z -= (oz / d) * push * 1.6;
                                }
                            }
                        }
                    }
                    let mut move_x = dir_x + avoid_x;
                    let mut move_z = dir_z + avoid_z;
                    let m_len = (move_x * move_x + move_z * move_z).sqrt();
                    if m_len > 1e-4 {
                        move_x /= m_len;
                        move_z /= m_len;
                    }
                    let step = (walk_speed * dt).min(dist);
                    let mut new_x = self.walker.pos[0] + move_x * step;
                    let mut new_z = self.walker.pos[2] + move_z * step;
                    if !in_confined_shaft {
                        for a in &self.agents {
                            if a.kind == AgentKind::HullWelder
                                || (a.pos[1] - self.walker.pos[1]).abs() > 1.8
                            {
                                continue;
                            }
                            let rx = new_x - a.pos[0];
                            let rz = new_z - a.pos[2];
                            let rd = (rx * rx + rz * rz).sqrt();
                            let hard_r = 0.85 + a.kind.radius() + 0.12;
                            if rd < hard_r && rd > 1e-4 {
                                new_x = a.pos[0] + (rx / rd) * hard_r;
                                new_z = a.pos[2] + (rz / rd) * hard_r;
                            }
                        }
                        // Keep BOT-01 strictly clear of Ops Holotable, shifted Reactor Core & 3-Deck Elevator Shaft
                        if (target[1] - D3).abs() < 0.5 {
                            let hx = new_x;
                            let hz = new_z - 1.0;
                            let hd = (hx * hx + hz * hz).sqrt();
                            if hd < 2.95 && hd > 1e-4 {
                                new_x = (hx / hd) * 2.95;
                                new_z = 1.0 + (hz / hd) * 2.95;
                            }
                        } else if (target[1] - D1).abs() < 0.5 {
                            let rx = new_x;
                            let rz = new_z - REACTOR_CORE_Z;
                            let hd = (rx * rx + rz * rz).sqrt();
                            if hd < 3.65 && hd > 1e-4 {
                                new_x = (rx / hd) * 3.65;
                                new_z = REACTOR_CORE_Z + (rz / hd) * 3.65;
                            }
                        }
                        let (ex, ez) = clamp_outside_elevator_shaft(new_x, new_z, 0.85);
                        new_x = ex;
                        new_z = ez;
                    }
                    self.walker.pos[0] = new_x;
                    self.walker.pos[2] = new_z;
                    self.walker.pos[1] = if target[1] > -9.0 && target[1] < 5.0 {
                        hallway_floor_y(new_x)
                    } else {
                        target[1]
                    };
                    if target[1] > -9.0
                        && target[1] < 5.0
                        && new_x.abs() > 12.2
                        && new_x.abs() < 23.2
                    {
                        target_pitch = if new_x.signum() * move_x > 0.0 {
                            0.217
                        } else {
                            -0.217
                        };
                    }
                    self.walker.walk_phase = (self.walker.walk_phase + dt * 7.2) % TAU;
                    let travel_yaw = move_x.atan2(move_z);
                    self.walker.yaw = turn_yaw(self.walker.yaw, travel_yaw, 5.0, dt);
                    self.walker.dwell_timer = dwell;
                } else {
                    if in_confined_shaft {
                        self.walker.pos = target;
                    } else {
                        self.walker.pos[1] = if target[1] > -9.0 && target[1] < 5.0 {
                            hallway_floor_y(self.walker.pos[0])
                        } else {
                            target[1]
                        };
                    }
                    if let Some(ly) = look_yaw {
                        self.walker.yaw = turn_yaw(self.walker.yaw, ly, 3.2, dt);
                    }
                    if self.walker.dwell_timer > 0.0 {
                        self.walker.dwell_timer = (self.walker.dwell_timer - dt).max(0.0);
                    } else {
                        self.walker.leg_timer = 0.0;
                        self.walker.route_index = (idx + 1) % ROUTE.len();
                    }
                }
            }
            WalkerLeg::WaitPadClear {
                pad,
                look_yaw,
                room,
            } => {
                self.walker.riding_elevator = false;
                self.walker.yaw = turn_yaw(self.walker.yaw, look_yaw, 3.2, dt);
                self.walker.leg_timer = 0.0;
                if self.is_pad_clear(pad) {
                    self.walker.current_room = format!("{} CLEAR", pad.label());
                    self.walker.route_index = (idx + 1) % ROUTE.len();
                } else {
                    self.walker.current_room = room.to_string();
                }
            }
            WalkerLeg::CycleAirlock {
                airlock_idx,
                depressurize,
                pad_check,
                room,
            } => {
                self.walker.riding_elevator = false;
                self.walker.leg_timer = 0.0;
                let side = if airlock_idx == 0 { -1.0f32 } else { 1.0f32 };
                let look = if depressurize {
                    side * FRAC_PI_2
                } else {
                    -side * FRAC_PI_2
                };
                self.walker.yaw = turn_yaw(self.walker.yaw, look, 3.5, dt);

                let pad_ready = pad_check.is_none_or(|p| self.is_pad_clear(p));
                let al = &mut self.pad_airlocks[airlock_idx];
                airlock_cycling[airlock_idx] = true;

                if depressurize {
                    // 1. Close inner door first
                    want_airlock_inner[airlock_idx] = false;
                    if al.inner_door > 0.06 {
                        self.walker.current_room = format!("{room} // SEALING");
                    } else if !pad_ready {
                        // Hold inside sealed airlock looking out at the busy landing pad
                        let pad_lbl = pad_check.map(|p| p.label()).unwrap_or("PAD");
                        self.walker.current_room = format!("{room} // {pad_lbl} BUSY");
                    } else if al.pressure > 0.02 {
                        // 2. Vent chamber atmosphere to vacuum
                        al.pressure = (al.pressure - dt * 0.85).max(0.0);
                        self.walker.current_room = format!("{room} // VENTING");
                    } else {
                        // 3. Open outer door onto the clear landing pad
                        al.pressure = 0.0;
                        want_airlock_outer[airlock_idx] = true;
                        self.walker.current_room = format!("{room} // EXITING");
                        if al.outer_door > 0.88 {
                            self.walker.route_index = (idx + 1) % ROUTE.len();
                        }
                    }
                } else {
                    // Returning from vacuum pad into pressurized station
                    want_airlock_outer[airlock_idx] = false;
                    if al.outer_door > 0.06 {
                        self.walker.current_room = format!("{room} // SEALING");
                    } else if al.pressure < 0.98 {
                        al.pressure = (al.pressure + dt * 0.85).min(1.0);
                        self.walker.current_room = format!("{room} // PRESSURIZING");
                    } else {
                        al.pressure = 1.0;
                        want_airlock_inner[airlock_idx] = true;
                        self.walker.current_room = format!("{room} // ENTERING");
                        if al.inner_door > 0.88 {
                            self.walker.route_index = (idx + 1) % ROUTE.len();
                        }
                    }
                }
            }
            WalkerLeg::WaitElevator { deck, room } => {
                self.walker.riding_elevator = false;
                self.walker.current_room = room.to_string();
                self.elevator.target_deck = deck;
                let tgt_y = deck.floor_y();
                let dy = tgt_y - self.elevator.current_y;
                if dy.abs() > 0.06 {
                    self.elevator.moving = true;
                    want_elev_door_open = false;
                    if self.elevator.door_open < 0.08 {
                        let step = (6.8 * dt).min(dy.abs());
                        self.elevator.current_y += dy.signum() * step;
                    }
                } else {
                    self.elevator.current_y = tgt_y;
                    self.elevator.moving = false;
                    want_elev_door_open = true;
                    if self.elevator.door_open > 0.88 {
                        self.walker.route_index = (idx + 1) % ROUTE.len();
                    }
                }
            }
            WalkerLeg::RideElevator { target_deck, room } => {
                self.walker.riding_elevator = true;
                self.walker.current_room = room.to_string();
                self.walker.yaw = turn_yaw(self.walker.yaw, 0.0, 3.5, dt);
                self.elevator.target_deck = target_deck;
                let tgt_y = target_deck.floor_y();
                let dy = tgt_y - self.elevator.current_y;
                if dy.abs() > 0.06 {
                    want_elev_door_open = false;
                    if self.elevator.door_open < 0.08 {
                        self.elevator.moving = true;
                        let step = (6.4 * dt).min(dy.abs());
                        self.elevator.current_y += dy.signum() * step;
                    }
                } else {
                    self.elevator.current_y = tgt_y;
                    self.elevator.moving = false;
                    want_elev_door_open = true;
                    if self.elevator.door_open > 0.88 {
                        self.walker.riding_elevator = false;
                        self.walker.route_index = (idx + 1) % ROUTE.len();
                    }
                }
                self.walker.pos = [0.0, self.elevator.current_y, -6.0];
            }
        }

        // Smoothly pitch BOT-01 along sloped Pad Access Hallways
        self.walker.pitch = ease_scalar(self.walker.pitch, target_pitch, 6.0, dt);

        // Smoothly animate elevator doors
        let elev_door_target = if want_elev_door_open { 1.0 } else { 0.0 };
        self.elevator.door_open = ease_scalar(self.elevator.door_open, elev_door_target, 5.0, dt);

        // Smoothly animate Port & Starboard Pad Airlocks with strict mechanical interlock:
        // inner and outer doors can NEVER be open simultaneously!
        for i in 0..2 {
            let al = &mut self.pad_airlocks[i];
            al.cycling = airlock_cycling[i];
            let inner_tgt = if want_airlock_inner[i] && al.outer_door < 0.04 && al.pressure > 0.92 {
                1.0
            } else {
                0.0
            };
            let outer_tgt = if want_airlock_outer[i] && al.inner_door < 0.04 && al.pressure < 0.08 {
                1.0
            } else {
                0.0
            };
            al.inner_door = ease_scalar(al.inner_door, inner_tgt, 5.5, dt);
            al.outer_door = ease_scalar(al.outer_door, outer_tgt, 5.5, dt);
        }

        // Proximity-triggered sliding blast doors on Deck 2 (`Z = 11.6`, `Z = -14.6`, `Z = -25.0`)
        // Opens smoothly for BOT-01, HERMES-01, AND any Deck-2 rolling robot!
        let door_zs = [11.6f32, -14.6f32, -25.0f32];
        let walker_on_d2 = (self.walker.pos[1] - D2).abs() < 1.5;
        let hermes_on_d2 = (-4.2..=4.2).contains(&self.hermes.pos[1]);
        for (i, &dz) in door_zs.iter().enumerate() {
            let walker_near = walker_on_d2
                && self.walker.pos[0].abs() < 3.5
                && (self.walker.pos[2] - dz).abs() < 4.5;
            let hermes_near = hermes_on_d2
                && !self.hermes.docked
                && self.hermes.pos[0].abs() < 4.2
                && (self.hermes.pos[2] - dz).abs() < 5.5;
            let any_bot_near = self.agents.iter().any(|a| {
                (a.pos[1] - D2).abs() < 1.5 && a.pos[0].abs() < 3.8 && (a.pos[2] - dz).abs() < 4.5
            });
            let tgt = if walker_near || hermes_near || any_bot_near {
                1.0
            } else {
                0.0
            };
            self.door_open[i] = ease_scalar(self.door_open[i], tgt, 4.8, dt);
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

        if self.camera_rig.auto_director
            && !self.camera_rig.primary_feed.is_dynamic()
            && !self.hermes.active
            && self.camera_rig.primary_feed != CameraFeedId::HermesFlyCam
        {
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

    /// Assign a specific camera feed to a PiP mini-monitor slot (`0` or `1`).
    pub fn set_pip_feed(&mut self, slot: usize, feed: CameraFeedId) {
        if slot < 2 && self.camera_rig.pip_feeds[slot] != feed {
            self.camera_rig.pip_feeds[slot] = feed;
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        }
    }

    /// Assign a specific camera feed to a 2x2 Grid quadrant (`0..=3`: TL, TR, BL, BR).
    pub fn set_grid_feed(&mut self, slot: usize, feed: CameraFeedId) {
        if slot < 4 && self.camera_rig.grid_feeds[slot] != feed {
            self.camera_rig.grid_feeds[slot] = feed;
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
        if self.camera_rig.auto_director
            && !self.camera_rig.primary_feed.is_dynamic()
            && !self.hermes.active
            && self.camera_rig.primary_feed != CameraFeedId::HermesFlyCam
        {
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

    // =========================================================================
    // Hermes (`HERMES-01`) Free-Flying Camera Drone
    // =========================================================================

    /// Activate Hermes when entering Direct Interaction Mode: spawns at his Atrium home alcove
    /// and jumps the primary camera feed to his first-person view.
    pub fn activate_hermes(&mut self) {
        if !self.hermes.active {
            if self.camera_rig.primary_feed != CameraFeedId::HermesFlyCam {
                self.hermes.saved_feed = Some(self.camera_rig.primary_feed);
            }
            if self.camera_rig.layout == CameraLayout::Grid2x2 {
                self.hermes.saved_layout = Some(self.camera_rig.layout);
                self.camera_rig.layout = CameraLayout::Single;
            }
        }
        self.hermes.pos = HERMES_HOME_POS;
        self.hermes.vel = [0.0, 0.0, 0.0];
        self.hermes.yaw = HERMES_HOME_YAW;
        self.hermes.pitch = HERMES_HOME_PITCH;
        self.hermes.active = true;
        self.hermes.docked = false;
        self.hermes.returning_home = false;
        self.hermes.thrust_level = 0.35;

        self.camera_rig.primary_feed = CameraFeedId::HermesFlyCam;
        self.camera_rig.manual_pan = [0.0, 0.0];
        self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
    }

    /// Deactivate Hermes when exiting Direct Interaction Mode: restores the previous camera feed
    /// and layout, and sends Hermes smoothly flying back to dock in his Atrium home bay.
    pub fn deactivate_hermes(&mut self) {
        if !self.hermes.active {
            return;
        }
        self.hermes.active = false;
        self.hermes.returning_home = true;
        self.hermes.vel = [0.0, 0.0, 0.0];

        if let Some(prev_feed) = self.hermes.saved_feed.take() {
            self.camera_rig.primary_feed = prev_feed;
            self.camera_rig.static_glitch_timer = STATIC_GLITCH_DURATION;
        }
        if let Some(prev_layout) = self.hermes.saved_layout.take() {
            self.camera_rig.layout = prev_layout;
        }
        self.camera_rig.director_timer = DIRECTOR_INTERVAL_SECS;
    }

    /// Apply Minecraft-creative-style flight controls to Hermes:
    /// - `move_fwd` (`+1` Up Arrow, `-1` Down Arrow) and `move_right` (`+1` Right Arrow, `-1` Left Arrow)
    ///   move horizontally in the XZ plane relative to Hermes's current yaw.
    /// - `move_up` (`+1` Spacebar, `-1` Shift) moves purely vertically along Y.
    /// - `fly_along_look` (Left Mouse Button held) propels Hermes smoothly along his exact 3D look vector.
    /// - `look_dyaw` / `look_dpitch` rotate the camera freely (360° yaw, ±85° pitch).
    #[allow(clippy::too_many_arguments)]
    pub fn apply_hermes_controls(
        &mut self,
        dt: f32,
        move_fwd: f32,
        move_right: f32,
        move_up: f32,
        fly_along_look: bool,
        look_dyaw: f32,
        look_dpitch: f32,
    ) {
        if !self.hermes.active {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);

        // Mouse look: unbounded yaw, clamped pitch (±85°)
        self.hermes.yaw = (self.hermes.yaw + look_dyaw).rem_euclid(TAU);
        self.hermes.pitch = (self.hermes.pitch + look_dpitch).clamp(-1.48, 1.48);

        // Horizontal XZ basis matching right-handed `build_perspective_view_proj`:
        // forward = [sin(yaw), 0, cos(yaw)], right = cross(forward, up) = [-cos(yaw), 0, sin(yaw)]
        let (sy, cy) = self.hermes.yaw.sin_cos();
        let (sp, cp) = self.hermes.pitch.sin_cos();
        let fwd_x = sy;
        let fwd_z = cy;
        let right_x = -cy;
        let right_z = sy;

        let mut horiz_x = fwd_x * move_fwd + right_x * move_right;
        let mut horiz_z = fwd_z * move_fwd + right_z * move_right;
        let mut vert_y = move_up.clamp(-1.0, 1.0);

        if fly_along_look {
            horiz_x += sy * cp;
            vert_y += sp;
            horiz_z += cy * cp;
        }

        let h_len = (horiz_x * horiz_x + horiz_z * horiz_z).sqrt();
        if h_len > 1.0 {
            horiz_x /= h_len;
            horiz_z /= h_len;
        }
        vert_y = vert_y.clamp(-1.0, 1.0);

        const HORIZ_SPEED: f32 = 14.0;
        const VERT_SPEED: f32 = 10.0;
        let target_vx = horiz_x * HORIZ_SPEED;
        let target_vy = vert_y
            * if fly_along_look {
                HORIZ_SPEED
            } else {
                VERT_SPEED
            };
        let target_vz = horiz_z * HORIZ_SPEED;

        let moving = h_len > 0.01 || vert_y.abs() > 0.01;
        let rate = if moving { 10.0 } else { 8.5 };
        self.hermes.vel[0] = ease_scalar(self.hermes.vel[0], target_vx, rate, dt);
        self.hermes.vel[1] = ease_scalar(self.hermes.vel[1], target_vy, rate, dt);
        self.hermes.vel[2] = ease_scalar(self.hermes.vel[2], target_vz, rate, dt);

        self.hermes.pos[0] += self.hermes.vel[0] * dt;
        self.hermes.pos[1] += self.hermes.vel[1] * dt;
        self.hermes.pos[2] += self.hermes.vel[2] * dt;

        // Soft outer orbital boundary so Hermes can explore all of HELIOS-09 and nearby space
        // without drifting infinitely into deep space.
        const MAX_ORBIT_RADIUS: f32 = 220.0;
        let r = len3(self.hermes.pos);
        if r > MAX_ORBIT_RADIUS {
            let scale = MAX_ORBIT_RADIUS / r;
            self.hermes.pos[0] *= scale;
            self.hermes.pos[1] *= scale;
            self.hermes.pos[2] *= scale;
        }

        let speed_norm = (len3(self.hermes.vel) / HORIZ_SPEED).clamp(0.0, 1.0);
        self.hermes.thrust_level =
            ease_scalar(self.hermes.thrust_level, 0.25 + 0.75 * speed_norm, 6.0, dt);
    }

    fn step_hermes(&mut self, dt: f32) {
        self.hermes.hover_phase = (self.hermes.hover_phase + dt * 2.8) % TAU;

        if self.hermes.active {
            return;
        }

        if self.hermes.returning_home {
            let to_home = sub3(HERMES_HOME_POS, self.hermes.pos);
            let dist = len3(to_home);
            if dist < 0.15 {
                self.hermes.pos = HERMES_HOME_POS;
                self.hermes.vel = [0.0, 0.0, 0.0];
                self.hermes.yaw = HERMES_HOME_YAW;
                self.hermes.pitch = HERMES_HOME_PITCH;
                self.hermes.returning_home = false;
                self.hermes.docked = true;
                self.hermes.thrust_level = 0.0;
            } else {
                let dir = norm3(to_home, [0.0, 0.0, -1.0]);
                let speed = (dist * 2.4).clamp(4.0, 18.0);
                let step_len = (speed * dt).min(dist);
                self.hermes.pos[0] += dir[0] * step_len;
                self.hermes.pos[1] += dir[1] * step_len;
                self.hermes.pos[2] += dir[2] * step_len;

                let desired_yaw = if dist > 1.2 {
                    dir[0].atan2(dir[2])
                } else {
                    HERMES_HOME_YAW
                };
                // Smooth shortest-arc yaw interpolation
                let mut dy = (desired_yaw - self.hermes.yaw) % TAU;
                if dy > std::f32::consts::PI {
                    dy -= TAU;
                } else if dy < -std::f32::consts::PI {
                    dy += TAU;
                }
                self.hermes.yaw =
                    (self.hermes.yaw + dy * (1.0 - (-5.0 * dt).exp())).rem_euclid(TAU);
                self.hermes.pitch = ease_scalar(self.hermes.pitch, HERMES_HOME_PITCH, 4.5, dt);
                self.hermes.thrust_level = ease_scalar(self.hermes.thrust_level, 0.65, 5.0, dt);
            }
        } else if self.hermes.docked {
            self.hermes.pos = HERMES_HOME_POS;
            self.hermes.thrust_level = ease_scalar(self.hermes.thrust_level, 0.0, 5.0, dt);
        }
    }

    /// Returns `true` if world position `p` lies inside one of Station HELIOS-09's interior rooms
    /// (used to switch atmospheric fog cleanly when Hermes flies out into open space).
    pub fn is_pos_interior(p: [f32; 3]) -> bool {
        let [x, y, z] = p;
        // Hangar Bay (`X:-12..12, Y:-4..10, Z:12..42`)
        if x.abs() <= 12.2 && (-4.5..=10.5).contains(&y) && (11.5..=42.0).contains(&z) {
            return true;
        }
        // Central Core Atrium (`X:-12..12, Y:-4.2..11.2, Z:-14.6..11.6`)
        if x.abs() <= 12.2 && (-4.5..=11.4).contains(&y) && (-14.8..=11.6).contains(&z) {
            return true;
        }
        // Ops Command Deck (`X:-11..11, Y:11.4..21.4, Z:-11..11`)
        if x.abs() <= 11.0 && (11.4..=21.4).contains(&y) && (-11.0..=11.0).contains(&z) {
            return true;
        }
        // Reactor & Concourse (`X:-19..19, Y:-15.6..-4.8, Z:-10..10`)
        if x.abs() <= 19.0 && (-15.8..=-4.8).contains(&y) && (-10.0..=10.0).contains(&z) {
            return true;
        }
        // Axial Pipe Tunnel (`X:-3..3, Y:-4.5..2.5, Z:-25.2..-14.6`)
        if x.abs() <= 3.2 && (-4.5..=2.5).contains(&y) && (-25.2..=-14.6).contains(&z) {
            return true;
        }
        // Aft Solar Engineering Bay (`X:-7..7, Y:-4.5..6.8, Z:-42.8..-25.0`)
        if x.abs() <= 7.0 && (-4.5..=6.8).contains(&y) && (-42.8..=-25.0).contains(&z) {
            return true;
        }
        false
    }
}
