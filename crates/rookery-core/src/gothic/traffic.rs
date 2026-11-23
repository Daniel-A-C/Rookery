//! Per-tick simulation stepping: elevated trains, traffic signals, vehicle AI, pedestrians, airships, and steam.

use std::f32::consts::TAU;

use super::types::*;
use super::{GothicMetropolisState, OVERVIEW_CAMERA_TARGET};

impl GothicMetropolisState {
    pub(super) fn step_simulation(&mut self, dt: f32) {
        self.time += dt;

        // Smooth automatic palette cycling (0.05/sec at 1.0x = 20s per palette transition)
        if self.palette_cycle_enabled {
            let rate = 0.05 * self.palette_cycle_speed.clamp(0.1, 5.0);
            self.palette_blend += dt * rate;
            while self.palette_blend >= 1.0 {
                self.palette_blend -= 1.0;
                self.palette = self.palette.next();
            }
        }

        // Smooth camera yaw interpolation toward target_camera_yaw
        let yaw_diff = self.target_camera_yaw - self.camera_yaw;
        if yaw_diff.abs() > 0.0005 {
            self.camera_yaw += yaw_diff * (dt * 7.5).min(1.0);
        } else {
            self.camera_yaw = self.target_camera_yaw;
        }

        // 1. Advance elevated trains along the closed viaduct loop with station dwell & safe headway
        let track_len = self.blueprint.viaduct_total_length();
        let station_center_dist = self.blueprint.viaduct_station_dist();
        let train_snapshot: Vec<(u32, f32, f32, u8)> = self
            .trains
            .iter()
            .map(|t| (t.id, t.track_dist, t.speed, t.car_count))
            .collect();

        for train in &mut self.trains {
            if train.station_dwell_secs > 0.0 {
                train.station_dwell_secs = (train.station_dwell_secs - dt).max(0.0);
                train.speed = 0.0;
                if train.station_dwell_secs == 0.0 {
                    // Nudge just past the station trigger point without a visible teleport
                    train.track_dist = (station_center_dist + 0.02).rem_euclid(track_len);
                    train.speed = train.base_speed * 0.35;
                }
                continue;
            }

            let dist_to_station = (station_center_dist - train.track_dist).rem_euclid(track_len);
            let mut desired_speed =
                if dist_to_station < 0.16 && train.speed < train.base_speed * 0.55 {
                    // Pause briefly at the ViaductStation platform
                    train.station_dwell_secs = 1.4;
                    train.track_dist = station_center_dist;
                    train.speed = 0.0;
                    continue;
                } else if dist_to_station < 1.8 {
                    // Smooth deceleration approaching the station
                    let factor = (dist_to_station / 1.8).clamp(0.32, 1.0);
                    train.base_speed * factor
                } else {
                    train.base_speed
                };

            // Maintain safe headway behind the next train ahead on the loop
            for &(other_id, other_dist, other_speed, other_cars) in &train_snapshot {
                if other_id == train.id {
                    continue;
                }
                let center_gap = (other_dist - train.track_dist).rem_euclid(track_len);
                let ahead_tail_len = (other_cars.saturating_sub(1) as f32) * 0.88 + 0.90;
                let clear_gap = center_gap - ahead_tail_len;
                if clear_gap < 2.0 {
                    if clear_gap <= 0.35 {
                        desired_speed = 0.0;
                    } else {
                        let t = ((clear_gap - 0.35) / 1.65).clamp(0.0, 1.0);
                        desired_speed =
                            desired_speed.min(other_speed * 0.85 + train.base_speed * 0.5 * t);
                    }
                }
            }

            train.speed += (desired_speed - train.speed) * (dt * 3.5).min(1.0);
            train.track_dist = (train.track_dist + train.speed * dt).rem_euclid(track_len);
        }

        // 2. Advance elevated walkway pedestrians
        for ped in &mut self.pedestrians {
            ped.progress += ped.dir * ped.speed * dt;
            if ped.progress >= 1.0 {
                ped.progress = 1.0;
                ped.dir = -1.0;
                ped.lateral_offset = -ped.lateral_offset.abs();
            } else if ped.progress <= 0.0 {
                ped.progress = 0.0;
                ped.dir = 1.0;
                ped.lateral_offset = ped.lateral_offset.abs();
            }
        }

        // 3. Update traffic signals, Vigilante 5-minute patrol timer, and street vehicle AI
        if self.green_wave_timer > 0.0 {
            self.green_wave_timer = (self.green_wave_timer - dt).max(0.0);
        }
        if self.pursuit_timer > 0.0 {
            self.pursuit_timer = (self.pursuit_timer - dt).max(0.0);
        } else {
            self.vigilante_cooldown_secs -= dt;
            if self.vigilante_cooldown_secs <= 0.0 {
                self.dispatch_interceptor();
            }
        }

        for sig in &mut self.traffic_signals {
            if self.green_wave_timer > 0.0 {
                sig.ns_green = true;
                sig.is_amber = false;
            } else {
                let cycle = (self.time + sig.phase_offset).rem_euclid(12.0);
                sig.ns_green = cycle < 6.0;
                sig.is_amber = (5.0..6.0).contains(&cycle) || (11.0..12.0).contains(&cycle);
            }
        }

        // Pre-sample world poses and slightly-ahead tangents for spacing, signals, and cornering
        let poses: Vec<_> = self
            .vehicles
            .iter()
            .map(|v| self.blueprint.sample_vehicle_pose(v))
            .collect();
        let ahead_tangents: Vec<Option<[f32; 2]>> = self
            .vehicles
            .iter()
            .map(|v| {
                let mut probe = v.clone();
                probe.progress = (probe.progress + probe.dir * 0.025).clamp(0.0, 1.0);
                self.blueprint
                    .sample_vehicle_pose(&probe)
                    .map(|(_, fwd, _)| fwd)
            })
            .collect();

        let route_count = self.blueprint.vehicle_routes.len();
        let veh_count = self.vehicles.len();
        let mut exhaust_spawns: Vec<([f32; 3], [f32; 2])> = Vec::new();

        for i in 0..veh_count {
            let is_closed = if route_count > 0 {
                self.blueprint
                    .vehicle_routes
                    .get(self.vehicles[i].avenue_idx % route_count)
                    .map(|r| r.is_closed_loop)
                    .unwrap_or(true)
            } else {
                true
            };

            let kind = self.vehicles[i].kind;
            let base_speed = self.vehicles[i].speed;

            // VigilanteInterceptor rests hidden inside a subterranean garage between 5-minute patrols
            if kind == GothicVehicleKind::VigilanteInterceptor
                && self.pursuit_timer <= 0.0
                && !is_closed
                && ((self.vehicles[i].progress <= 0.02
                    && (self.vehicles[i].dir < 0.0 || self.vehicles[i].current_speed == 0.0))
                    || (self.vehicles[i].progress >= 0.98 && self.vehicles[i].dir > 0.0))
            {
                if self.vehicles[i].progress >= 0.98 {
                    self.vehicles[i].progress = 1.0;
                    self.vehicles[i].dir = -1.0;
                } else {
                    self.vehicles[i].progress = 0.0;
                    self.vehicles[i].dir = 1.0;
                }
                self.vehicles[i].current_speed = 0.0;
                self.vehicles[i].is_braking = false;
                self.vehicles[i].turn_signal = 0;
                self.vehicles[i].afterburner = 0.0;
                continue;
            }

            let mut target_speed = base_speed;
            let mut braking = false;
            let mut turn_sig: i8 = 0;

            if let Some((pos, fwd, is_subterranean)) = poses[i]
                && !is_subterranean
            {
                // Highway speed boost on outer perimeter ring
                if pos[0].abs() > 9.8 || pos[2].abs() > 9.8 {
                    target_speed *= 1.35;
                }

                // Cornering deceleration & turn signal indicator
                if let Some(ahead_fwd) = ahead_tangents[i] {
                    let dot = fwd[0] * ahead_fwd[0] + fwd[1] * ahead_fwd[1];
                    if dot < 0.94 {
                        target_speed *= 0.62;
                        if self.vehicles[i].current_speed > target_speed * 1.12 {
                            braking = true;
                        }
                        let cross = fwd[0] * ahead_fwd[1] - fwd[1] * ahead_fwd[0];
                        turn_sig = if cross > 0.0 { 1 } else { -1 };
                    }
                }

                // Active Vigilante pursuit override
                let is_active_vigilante =
                    kind == GothicVehicleKind::VigilanteInterceptor && self.pursuit_timer > 0.0;
                if is_active_vigilante {
                    target_speed = base_speed * 2.05;
                }

                // Functional gothic traffic signals (only on ground-level streets; Vigilante blasts through)
                if pos[1] < 0.4 && !is_active_vigilante && self.green_wave_timer <= 0.0 {
                    let moving_ns = fwd[1].abs() >= fwd[0].abs();
                    for sig in &self.traffic_signals {
                        let dx = sig.pos[0] - pos[0];
                        let dz = sig.pos[1] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.38..=1.35).contains(&along) && perp < 0.55 {
                            let has_green = if moving_ns {
                                sig.ns_green && !sig.is_amber
                            } else {
                                !sig.ns_green && !sig.is_amber
                            };
                            if !has_green {
                                target_speed = 0.0;
                                braking = true;
                                break;
                            }
                        }
                    }
                }

                // Car-following collision avoidance (only yield to same-direction traffic,
                // never deadlock head-to-head on single-lane garage ramps)
                for (j, other_pose) in poses.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    if let Some((opos, ofwd, o_sub)) = other_pose {
                        if *o_sub || (opos[1] - pos[1]).abs() > 0.65 {
                            continue;
                        }
                        let same_dir = fwd[0] * ofwd[0] + fwd[1] * ofwd[1];
                        if same_dir <= 0.35 {
                            continue;
                        }
                        let dx = opos[0] - pos[0];
                        let dz = opos[2] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.15..=1.28).contains(&along) && perp < 0.34 {
                            target_speed = 0.0;
                            braking = true;
                            break;
                        }
                    }
                }

                // Periodic tailpipe steam puffs for vintage steam/turbine vehicles
                if matches!(
                    kind,
                    GothicVehicleKind::SteamLimousine
                        | GothicVehicleKind::TransitOmnibus
                        | GothicVehicleKind::VigilanteInterceptor
                ) {
                    self.vehicles[i].exhaust_accum += dt;
                    let interval = if kind == GothicVehicleKind::VigilanteInterceptor {
                        0.18
                    } else {
                        0.55
                    };
                    if self.vehicles[i].exhaust_accum >= interval {
                        self.vehicles[i].exhaust_accum = 0.0;
                        exhaust_spawns.push((pos, fwd));
                    }
                }
            }

            let veh = &mut self.vehicles[i];
            let accel_rate = if braking { 8.5 } else { 4.2 };
            veh.current_speed += (target_speed - veh.current_speed) * (dt * accel_rate).min(1.0);
            veh.is_braking = braking && veh.current_speed < base_speed * 0.9;
            veh.turn_signal = turn_sig;

            if kind == GothicVehicleKind::VigilanteInterceptor {
                let target_ab = if self.pursuit_timer > 0.0 { 1.0 } else { 0.15 };
                veh.afterburner += (target_ab - veh.afterburner) * (dt * 5.0).min(1.0);
            } else {
                veh.afterburner = 0.0;
            }

            if is_closed {
                veh.progress = (veh.progress + veh.dir * veh.current_speed * dt).rem_euclid(1.0);
            } else {
                veh.progress += veh.dir * veh.current_speed * dt;
                if veh.progress >= 1.0 {
                    veh.progress = 1.0;
                    veh.dir = -1.0;
                } else if veh.progress <= 0.0 {
                    veh.progress = 0.0;
                    veh.dir = 1.0;
                }
            }
        }

        for (pos, fwd) in exhaust_spawns {
            if self.steam_plumes.len() < 78 {
                self.steam_plumes.push(SteamPlume {
                    pos: [
                        pos[0] - fwd[0] * 0.38,
                        pos[1] + 0.06,
                        pos[2] - fwd[1] * 0.38,
                    ],
                    vx: -fwd[0] * 0.18 + self.rng.range(-0.04, 0.04),
                    vy: self.rng.range(0.16, 0.28),
                    vz: -fwd[1] * 0.18 + self.rng.range(-0.04, 0.04),
                    age: 0.0,
                    max_age: 0.75,
                    base_radius: 0.055,
                });
            }
        }

        // Smooth Follow-Car camera tracking
        let (desired_target, desired_zoom) = if self.follow_car {
            let tracked = self
                .vehicles
                .iter()
                .find(|v| {
                    v.kind == GothicVehicleKind::VigilanteInterceptor && self.pursuit_timer > 0.0
                })
                .or_else(|| {
                    self.vehicles
                        .iter()
                        .find(|v| v.kind == GothicVehicleKind::ArmoredCruiser)
                })
                .or_else(|| self.vehicles.first());
            if let Some(v) = tracked
                && let Some((pos, _, is_sub)) = self.blueprint.sample_vehicle_pose(v)
                && !is_sub
            {
                ([pos[0], 0.45, pos[2]], 2.35)
            } else {
                (OVERVIEW_CAMERA_TARGET, 1.0)
            }
        } else {
            (OVERVIEW_CAMERA_TARGET, 1.0)
        };
        let cam_blend = (dt * 4.5).min(1.0);
        for (tgt, desired) in self.camera_target.iter_mut().zip(desired_target) {
            *tgt += (desired - *tgt) * cam_blend;
        }
        self.camera_zoom += (desired_zoom - self.camera_zoom) * cam_blend;

        // 4. Advance high-altitude patrol airships
        for ship in &mut self.airships {
            ship.angle = (ship.angle + ship.angular_speed * dt) % TAU;
            ship.searchlight_phase = (ship.searchlight_phase + dt * 0.22) % TAU;
        }

        // 5. Update and spawn rising steam/smoke plumes
        for plume in &mut self.steam_plumes {
            plume.age += dt;
            plume.pos[0] += plume.vx * dt;
            plume.pos[1] += plume.vy * dt;
            plume.pos[2] += plume.vz * dt;
        }
        self.steam_plumes.retain(|p| p.age < p.max_age);

        self.steam_accum += dt;
        while self.steam_accum >= 0.14 {
            self.steam_accum -= 0.14;
            if self.steam_plumes.len() < 60 && !self.blueprint.steam_sources.is_empty() {
                let idx = (self.rng.next_u64() as usize) % self.blueprint.steam_sources.len();
                let src = &self.blueprint.steam_sources[idx];
                let vx = self.rng.range(-0.08, 0.14);
                let vz = self.rng.range(-0.10, 0.10);
                let vy = if src.is_rooftop {
                    self.rng.range(0.42, 0.68) * src.rate_scale
                } else {
                    self.rng.range(0.28, 0.45) * src.rate_scale
                };
                let max_age = if src.is_rooftop {
                    self.rng.range(2.6, 4.0)
                } else {
                    self.rng.range(1.8, 2.8)
                };
                let base_radius = if src.is_rooftop { 0.16 } else { 0.12 };
                self.steam_plumes.push(SteamPlume {
                    pos: src.pos,
                    vx,
                    vy,
                    vz,
                    age: 0.0,
                    max_age,
                    base_radius,
                });
            }
        }

        // 6. Advance circling gothic spire fliers (bats/ravens)
        for flier in &mut self.spire_fliers {
            flier.angle = (flier.angle + flier.angular_speed * dt).rem_euclid(TAU);
            flier.flap_phase = (flier.flap_phase + dt * 11.5).rem_euclid(TAU);
            flier.bob_phase = (flier.bob_phase + dt * 1.4).rem_euclid(TAU);
        }

        // 7. Advance ground-level sidewalk pedestrians around city block loops
        for walker in &mut self.sidewalk_walkers {
            walker.perimeter_t =
                (walker.perimeter_t + walker.dir * walker.speed * dt).rem_euclid(1.0);
        }

        // 8. Update and spawn warm glowing ember sparks from rooftop industrial stacks
        for spark in &mut self.ember_sparks {
            spark.age += dt;
            spark.pos[0] += spark.vx * dt;
            spark.pos[1] += spark.vy * dt;
            spark.pos[2] += spark.vz * dt;
        }
        self.ember_sparks.retain(|s| s.age < s.max_age);

        self.ember_accum += dt;
        while self.ember_accum >= 0.12 {
            self.ember_accum -= 0.12;
            if self.ember_sparks.len() < 40 {
                // Spawn from rooftop steam sources (industrial smokestacks)
                let rooftop_count = self
                    .blueprint
                    .steam_sources
                    .iter()
                    .filter(|s| s.is_rooftop)
                    .count();
                if rooftop_count > 0 {
                    let pick = (self.rng.next_u64() as usize) % rooftop_count;
                    if let Some(src) = self
                        .blueprint
                        .steam_sources
                        .iter()
                        .filter(|s| s.is_rooftop)
                        .nth(pick)
                    {
                        self.ember_sparks.push(EmberSpark {
                            pos: [
                                src.pos[0] + self.rng.range(-0.12, 0.12),
                                src.pos[1] + 0.05,
                                src.pos[2] + self.rng.range(-0.12, 0.12),
                            ],
                            vx: self.rng.range(-0.22, 0.22),
                            vy: self.rng.range(0.65, 1.15),
                            vz: self.rng.range(-0.22, 0.22),
                            age: 0.0,
                            max_age: self.rng.range(1.3, 2.2),
                        });
                    }
                }
            }
        }
    }
}
