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
            let turn_rate = match ship.class {
                ShipClass::PatrolCutter => 2.6,
                ShipClass::CorporateShuttle => 2.0,
                ShipClass::MaintenanceTug => 1.7,
                ShipClass::BulkFreighter => 1.25,
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
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
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

                    if ship.leg_progress >= 1.0 {
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
                        ship.leg_duration = 20.0;
                    }
                }
                ShipFlightPhase::Docked => {
                    ship.leg_progress += dt / ship.leg_duration.max(1.0);
                    let pad_fwd = pad_docked_forward(ship.assigned_pad);
                    ship.forward = turn_toward(ship.forward, pad_fwd, 2.0, dt);
                    ship.up = turn_toward(ship.up, [0.0, 1.0, 0.0], 3.0, dt);
                    ship.gear_deploy = 1.0;
                    ship.thruster_power = ease_scalar(ship.thruster_power, 0.0, 5.0, dt);
                    ship.retro_thrust = ease_scalar(ship.retro_thrust, 0.0, 5.0, dt);
                    // Gentle VTOL pre-ignition warm-up in the last 14% of docked time before liftoff
                    let pre_ignite = smoothstep((ship.leg_progress - 0.86) / 0.14) * 0.55;
                    ship.vtol_thrust = ease_scalar(ship.vtol_thrust, pre_ignite, 4.0, dt);

                    if ship.leg_progress >= 1.0 {
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
            vtol_thrust: 0.0,
            retro_thrust: 0.0,
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
