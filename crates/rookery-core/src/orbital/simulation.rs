//! Simulation stepping, construction, robotics, fleet dispatch, and telemetry for *Orbital Haven*.

use std::f32::consts::TAU;

use crate::Activity;

use super::types::*;
use super::{OrbitalHavenState, STATION_CENTER};

impl OrbitalHavenState {
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
