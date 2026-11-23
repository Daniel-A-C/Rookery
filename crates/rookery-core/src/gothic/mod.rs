//! Deterministic simulation state and modular city blueprint for *Gothic Metropolis* (`gothic-metropolis`).
//!
//! Models an exaggerated 1990s-Batman-style gothic art-deco metropolis in simplified 3D isometric
//! space (`x, z` horizontal footprint in `[-12.0..12.0]`, `y` vertical elevation in `[0.0..14.0]`).
//!
//! Key architectural properties:
//! - **Consistent Authored Blueprint (`CityBlueprint`)**: The city layout is deterministic and
//!   consistent every time you view it, defined by modular `BuildingPlacement` slots, multi-level
//!   `WalkwaySpan` skybridges, a closed `ViaductTrack` loop, and deep `StreetAvenue` canyons.
//! - **Extensible Building System (`BuildingArchetype`)**: New building archetypes or future
//!   district layouts can be added declaratively without changing the renderer or simulation loop.
//! - **Five Vertical Layers of Action**:
//!   1. Canyon street vehicles (`GothicVehicle`) & street steam grates (`y = 0.05`).
//!   2. Multi-level elevated walkways with pedestrians (`WalkwayPedestrian`) (`y = 1.6 .. 5.2`).
//!   3. Elevated commuter train network (`ElevatedGothicTrain`) weaving around & through towers (`y = 2.6`).
//!   4. Towering gothic spires with rising steam plumes (`SteamPlume`).
//!   5. High-altitude patrol airships (`GothicAirship`) with downward spotlights (`y = 9.2 .. 11.8`).

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

use serde::{Deserialize, Serialize};

use crate::Activity;

pub mod blueprint;
pub mod traffic;
pub mod types;

#[cfg(test)]
mod tests;

pub use blueprint::*;
pub use types::*;

/// Default overview look-at target. Raised slightly above the geometric center (`5.20`) so the
/// city sits a little lower in the viewport, centering it cleanly in the visible area below a
/// top camera cutout / menu bar.
const OVERVIEW_CAMERA_TARGET: [f32; 3] = [0.0, 5.95, 0.0];
const DEFAULT_CAMERA_PITCH: f32 = 0.54;

fn default_palette_cycle_speed() -> f32 {
    1.0
}

/// Complete deterministic simulation state for *Gothic Metropolis* (`gothic-metropolis`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GothicMetropolisState {
    pub seed: u64,
    pub rng: GothicRng,
    pub time: f32,
    pub palette: GothicPalette,
    /// When true, the city smoothly crossfades through all curated palettes over time.
    #[serde(default)]
    pub palette_cycle_enabled: bool,
    /// Speed multiplier for smooth palette cycling (`0.1` .. `5.0`, default `1.0`).
    #[serde(default = "default_palette_cycle_speed")]
    pub palette_cycle_speed: f32,
    /// Current interpolation progress `[0.0, 1.0)` from `palette` toward `palette.next()`.
    #[serde(default)]
    pub palette_blend: f32,
    pub blueprint: CityBlueprint,
    pub trains: Vec<ElevatedGothicTrain>,
    pub pedestrians: Vec<WalkwayPedestrian>,
    pub vehicles: Vec<GothicVehicle>,
    pub airships: Vec<GothicAirship>,
    pub steam_plumes: Vec<SteamPlume>,
    pub searchlights: Vec<SearchlightBeam>,
    pub searchlights_enabled: bool,
    pub trains_dispatched: u32,
    /// Current isometric camera yaw angle in radians (smoothly interpolates toward `target_camera_yaw`).
    pub camera_yaw: f32,
    pub target_camera_yaw: f32,
    /// Current isometric camera pitch angle in radians (`0.38 .. 0.78`, default `0.54`).
    pub camera_pitch: f32,
    /// Fraction of the screen occupied by the city island (`0.60` = three-fifths default).
    pub screen_occupancy: f32,
    /// Functional wrought-iron intersection traffic signals.
    #[serde(default)]
    pub traffic_signals: Vec<TrafficSignal>,
    /// Current street traffic density preset.
    #[serde(default)]
    pub traffic_density: TrafficDensity,
    /// Remaining seconds of all-green express wave (`> 0.0` forces all signals green).
    #[serde(default)]
    pub green_wave_timer: f32,
    /// Remaining seconds of active high-speed Vigilante Interceptor patrol.
    #[serde(default)]
    pub pursuit_timer: f32,
    /// Countdown in seconds until the next automatic 5-minute Vigilante patrol.
    #[serde(default)]
    pub vigilante_cooldown_secs: f32,
    /// When true, the isometric camera smoothly tracks the active Vigilante or lead cruiser.
    #[serde(default)]
    pub follow_car: bool,
    /// Current smoothed camera look-at target `[x, y, z]`.
    #[serde(default)]
    pub camera_target: [f32; 3],
    /// Current smoothed camera zoom multiplier (`1.0` = overview, `2.35` = street follow).
    #[serde(default)]
    pub camera_zoom: f32,
    /// Dark-winged spire fliers circling the tallest cathedral and belfry spires.
    #[serde(default)]
    pub spire_fliers: Vec<SpireFlier>,
    /// Ground-level pedestrians strolling along the raised block sidewalk loops.
    #[serde(default)]
    pub sidewalk_walkers: Vec<SidewalkWalker>,
    /// Glowing ember sparks rising from industrial foundry smokestacks.
    #[serde(default)]
    pub ember_sparks: Vec<EmberSpark>,
    steam_accum: f32,
    #[serde(default)]
    ember_accum: f32,
}

impl GothicMetropolisState {
    pub fn new(seed: u64) -> Self {
        let mut rng = GothicRng::new(seed);
        let blueprint = CityBlueprint::default_gothic_metropolis();
        let track_len = blueprint.viaduct_total_length();

        // Spawn 3 articulated elevated trains spaced around the viaduct loop
        // so at least one train is always visible in the foreground viaduct!
        let trains = vec![
            ElevatedGothicTrain {
                id: 1,
                track_dist: track_len * 0.44,
                speed: 2.35,
                base_speed: 2.35,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
            ElevatedGothicTrain {
                id: 2,
                track_dist: track_len * 0.78,
                speed: 2.25,
                base_speed: 2.25,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
            ElevatedGothicTrain {
                id: 3,
                track_dist: track_len * 0.12,
                speed: 2.30,
                base_speed: 2.30,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
        ];

        // Spawn stylized pedestrians across all 12 elevated walkways
        let mut pedestrians = Vec::new();
        let mut ped_id = 1u32;
        for (w_idx, span) in blueprint.walkways.iter().enumerate() {
            let count = if span.elevation < 2.0 { 3 } else { 2 };
            for k in 0..count {
                let progress = ((k as f32 + 0.25) / count as f32 + rng.f32() * 0.18).fract();
                let dir = if (w_idx + k) % 2 == 0 { 1.0 } else { -1.0 };
                let lateral_offset = if dir > 0.0 { 0.07 } else { -0.07 };
                pedestrians.push(WalkwayPedestrian {
                    id: ped_id,
                    walkway_idx: w_idx,
                    progress,
                    dir,
                    speed: rng.range(0.11, 0.19) / span.length().max(1.0),
                    lateral_offset,
                    coat_variant: (ped_id % 3) as u8,
                });
                ped_id += 1;
            }
        }

        // Spawn 20 gothic vehicles across the 10 connected vehicle routes
        // (5 continuous highway/canyon loops + 5 subterranean garage-to-garage commuter routes)
        let mut vehicles = Vec::new();
        let mut veh_id = 1u32;
        let route_count = blueprint.vehicle_routes.len().max(1);
        for route_idx in 0..route_count {
            let is_closed = blueprint
                .vehicle_routes
                .get(route_idx)
                .map(|r| r.is_closed_loop)
                .unwrap_or(true);
            for slot in 0..2 {
                let dir = if is_closed || slot == 0 { 1.0 } else { -1.0 };
                let kind = match (route_idx + slot) % 4 {
                    0 => GothicVehicleKind::NoirCoupe,
                    1 => GothicVehicleKind::ArmoredCruiser,
                    2 => GothicVehicleKind::TransitOmnibus,
                    _ => GothicVehicleKind::SteamLimousine,
                };
                let base_prog = if slot == 0 {
                    0.18 + rng.f32() * 0.22
                } else {
                    0.62 + rng.f32() * 0.22
                };
                let speed = if is_closed {
                    rng.range(0.026, 0.040)
                } else {
                    rng.range(0.052, 0.078)
                };
                vehicles.push(GothicVehicle {
                    id: veh_id,
                    avenue_idx: route_idx,
                    progress: base_prog.fract(),
                    dir,
                    speed,
                    kind,
                    current_speed: speed,
                    is_braking: false,
                    turn_signal: 0,
                    afterburner: 0.0,
                    exhaust_accum: rng.range(0.0, 0.4),
                });
                veh_id += 1;
            }
        }

        // Spawn the hero VigilanteInterceptor parked inside Garage 1 (Route 5) ready for patrol
        let vigilante_route = 5.min(route_count.saturating_sub(1));
        vehicles.push(GothicVehicle {
            id: veh_id,
            avenue_idx: vigilante_route,
            progress: 0.0,
            dir: 1.0,
            speed: 0.085,
            kind: GothicVehicleKind::VigilanteInterceptor,
            current_speed: 0.0,
            is_braking: false,
            turn_signal: 0,
            afterburner: 0.0,
            exhaust_accum: 0.0,
        });

        // Functional wrought-iron traffic signals at the 6 primary inner-city junctions
        let signal_coords: [([f32; 2], f32); 6] = [
            ([-5.2, 0.2], 0.0),
            ([5.4, 0.2], 6.0),
            ([-0.4, -5.4], 3.0),
            ([0.7, 4.6], 9.0),
            ([-5.2, -5.4], 4.5),
            ([-5.2, 5.4], 1.5),
        ];
        let traffic_signals = signal_coords
            .iter()
            .enumerate()
            .map(|(i, &(pos, phase_offset))| TrafficSignal {
                id: i as u32 + 1,
                pos,
                phase_offset,
                ns_green: phase_offset < 6.0,
                is_amber: false,
            })
            .collect();

        // Spawn 2 gothic patrol airships cruising above the upper spires
        let airships = vec![
            GothicAirship {
                id: 1,
                orbit_center: [-0.4, -0.3],
                orbit_rx: 8.6,
                orbit_rz: 7.4,
                altitude: 14.2,
                angle: 0.65,
                angular_speed: 0.022,
                hull_length: 2.25,
                searchlight_phase: 0.0,
            },
            GothicAirship {
                id: 2,
                orbit_center: [0.5, 0.4],
                orbit_rx: 6.8,
                orbit_rz: 8.2,
                altitude: 15.6,
                angle: 3.8,
                angular_speed: -0.018,
                hull_length: 1.95,
                searchlight_phase: 2.1,
            },
        ];

        // No rooftop searchlights on buildings (only patrol airships project searchlights)
        let searchlights = Vec::new();

        // Spawn 15 gothic spire fliers (bats/ravens) circling the tallest spires
        let spire_anchors: [([f32; 2], f32); 3] = [
            ([-2.80, -2.35], 13.8), // Central CathedralKeep
            ([2.10, -2.05], 13.2),  // ArtDecoZiggurat
            ([7.10, -7.10], 11.4),  // NE CathedralKeep
        ];
        let mut spire_fliers = Vec::with_capacity(15);
        for i in 0..15u32 {
            let (center, base_alt) = spire_anchors[(i as usize) % spire_anchors.len()];
            let dir = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
            spire_fliers.push(SpireFlier {
                id: i + 1,
                center,
                radius: rng.range(1.35, 2.55),
                altitude: base_alt + rng.range(-0.9, 1.8),
                angle: rng.range(0.0, TAU),
                angular_speed: dir * rng.range(0.32, 0.58),
                flap_phase: rng.range(0.0, TAU),
                bob_phase: rng.range(0.0, TAU),
            });
        }

        // Spawn 32 ground-level sidewalk pedestrians across the 16 city blocks (2 per block)
        let mut sidewalk_walkers = Vec::with_capacity(blueprint.blocks.len() * 2);
        let mut sw_id = 1u32;
        for (b_idx, block) in blueprint.blocks.iter().enumerate() {
            let perim = ((block.x1 - block.x0) + (block.z1 - block.z0)) * 2.0;
            for k in 0..2 {
                sidewalk_walkers.push(SidewalkWalker {
                    id: sw_id,
                    block_idx: b_idx,
                    perimeter_t: (k as f32 * 0.5 + rng.f32() * 0.3).fract(),
                    dir: if (b_idx + k) % 2 == 0 { 1.0 } else { -1.0 },
                    speed: rng.range(0.24, 0.38) / perim.max(4.0),
                    coat_variant: (sw_id % 3) as u8,
                });
                sw_id += 1;
            }
        }

        let mut state = Self {
            seed,
            rng,
            time: 0.0,
            palette: GothicPalette::GothamAmber,
            palette_cycle_enabled: false,
            palette_cycle_speed: 1.0,
            palette_blend: 0.0,
            blueprint,
            trains,
            pedestrians,
            vehicles,
            airships,
            steam_plumes: Vec::with_capacity(64),
            searchlights,
            searchlights_enabled: true,
            trains_dispatched: 2,
            camera_yaw: FRAC_PI_4,
            target_camera_yaw: FRAC_PI_4,
            camera_pitch: DEFAULT_CAMERA_PITCH,
            screen_occupancy: 0.675,
            traffic_signals,
            traffic_density: TrafficDensity::NocturnalCruise,
            green_wave_timer: 0.0,
            pursuit_timer: 0.0,
            vigilante_cooldown_secs: 300.0,
            follow_car: false,
            camera_target: OVERVIEW_CAMERA_TARGET,
            camera_zoom: 1.0,
            spire_fliers,
            sidewalk_walkers,
            ember_sparks: Vec::with_capacity(48),
            steam_accum: 0.0,
            ember_accum: 0.0,
        };

        // Pre-warm steam plumes so frame 0 already has atmospheric rising steam
        for _ in 0..24 {
            state.step_simulation(0.25);
        }

        state
    }

    pub fn tick(&mut self, dt_secs: f32) -> Activity {
        let clamped_dt = dt_secs.clamp(0.0, 0.25);
        self.step_simulation(clamped_dt);
        Activity::Animating
    }

    /// Deterministic offline catch-up when waking from sleep.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 {
            return;
        }
        let steps = ((elapsed_secs / 1.0).floor() as usize).clamp(1, 60);
        for _ in 0..steps {
            self.step_simulation(0.25);
        }
    }

    /// Dispatch an express elevated train (or boost existing trains if at max capacity).
    pub fn dispatch_train(&mut self) {
        self.trains_dispatched += 1;
        let track_len = self.blueprint.viaduct_total_length();
        if self.trains.len() < 4 {
            let new_id = self.trains.len() as u32 + 1;
            // Place the new train at the midpoint of the largest gap around the loop
            let offset = if self.trains.is_empty() {
                0.0
            } else {
                let mut sorted: Vec<f32> = self.trains.iter().map(|t| t.track_dist).collect();
                sorted.sort_by(|a, b| a.total_cmp(b));
                let mut best_gap = -1.0f32;
                let mut best_mid = 0.0f32;
                for i in 0..sorted.len() {
                    let a = sorted[i];
                    let b = sorted[(i + 1) % sorted.len()];
                    let gap = (b - a).rem_euclid(track_len);
                    if gap > best_gap {
                        best_gap = gap;
                        best_mid = (a + gap * 0.5).rem_euclid(track_len);
                    }
                }
                best_mid
            };
            self.trains.push(ElevatedGothicTrain {
                id: new_id,
                track_dist: offset,
                speed: 3.1,
                base_speed: 2.75,
                car_count: 5,
                station_dwell_secs: 0.0,
            });
        } else {
            for t in &mut self.trains {
                t.station_dwell_secs = 0.0;
                t.speed = 3.4;
            }
        }
    }

    /// Launch the Vigilante Interceptor on a high-speed afterburner patrol across the city.
    pub fn dispatch_interceptor(&mut self) {
        self.pursuit_timer = 26.0;
        self.vigilante_cooldown_secs = 300.0;
        if let Some(v) = self
            .vehicles
            .iter_mut()
            .find(|v| v.kind == GothicVehicleKind::VigilanteInterceptor)
        {
            v.afterburner = 1.0;
            v.current_speed = v.speed * 2.0;
            if v.progress <= 0.02 {
                v.dir = 1.0;
            }
        } else {
            let next_id = self.vehicles.iter().map(|v| v.id).max().unwrap_or(0) + 1;
            let route = 5.min(self.blueprint.vehicle_routes.len().saturating_sub(1));
            self.vehicles.push(GothicVehicle {
                id: next_id,
                avenue_idx: route,
                progress: 0.04,
                dir: 1.0,
                speed: 0.085,
                kind: GothicVehicleKind::VigilanteInterceptor,
                current_speed: 0.17,
                is_braking: false,
                turn_signal: 0,
                afterburner: 1.0,
                exhaust_accum: 0.0,
            });
        }
    }

    /// Toggle between Nocturnal Cruise and dense Midnight Rush Hour street traffic.
    pub fn toggle_rush_hour(&mut self) {
        match self.traffic_density {
            TrafficDensity::NocturnalCruise => {
                self.traffic_density = TrafficDensity::MidnightRushHour;
                let route_count = self.blueprint.vehicle_routes.len().max(1);
                let base_id = self.vehicles.iter().map(|v| v.id).max().unwrap_or(0) + 1;
                for route_idx in 0..route_count {
                    let is_closed = self
                        .blueprint
                        .vehicle_routes
                        .get(route_idx)
                        .map(|r| r.is_closed_loop)
                        .unwrap_or(true);
                    let kind = match route_idx % 4 {
                        0 => GothicVehicleKind::NoirCoupe,
                        1 => GothicVehicleKind::ArmoredCruiser,
                        2 => GothicVehicleKind::TransitOmnibus,
                        _ => GothicVehicleKind::SteamLimousine,
                    };
                    let speed = if is_closed {
                        self.rng.range(0.028, 0.042)
                    } else {
                        self.rng.range(0.050, 0.072)
                    };
                    self.vehicles.push(GothicVehicle {
                        id: base_id + route_idx as u32,
                        avenue_idx: route_idx,
                        progress: self.rng.range(0.32, 0.54),
                        dir: if is_closed || route_idx % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        },
                        speed,
                        kind,
                        current_speed: speed,
                        is_braking: false,
                        turn_signal: 0,
                        afterburner: 0.0,
                        exhaust_accum: 0.0,
                    });
                }
            }
            TrafficDensity::MidnightRushHour => {
                self.traffic_density = TrafficDensity::NocturnalCruise;
                let base_non_vigilante = self.blueprint.vehicle_routes.len().max(1) * 2;
                let mut kept = 0usize;
                self.vehicles.retain(|v| {
                    if v.kind == GothicVehicleKind::VigilanteInterceptor {
                        return true;
                    }
                    kept += 1;
                    kept <= base_non_vigilante
                });
            }
        }
    }

    /// Force all inner-city traffic signals green for 10 seconds so queued cars surge forward.
    pub fn trigger_green_wave(&mut self) {
        self.green_wave_timer = 10.0;
        for sig in &mut self.traffic_signals {
            sig.ns_green = true;
            sig.is_amber = false;
        }
    }

    /// Toggle smooth Follow-Car camera tracking of the Vigilante Interceptor or lead cruiser.
    pub fn toggle_follow_car(&mut self) {
        self.follow_car = !self.follow_car;
    }

    /// Summon an additional patrol airship (or boost airship patrol speed and emit steam).
    pub fn summon_airship(&mut self) {
        if self.airships.len() < 4 {
            let new_id = self.airships.len() as u32 + 1;
            let dir = if new_id.is_multiple_of(2) { -1.0 } else { 1.0 };
            self.airships.push(GothicAirship {
                id: new_id,
                orbit_center: [0.0, 0.0],
                orbit_rx: 6.2 + (new_id as f32 * 0.55),
                orbit_rz: 5.8 + (new_id as f32 * 0.45),
                altitude: 13.4 + (new_id as f32 * 0.65),
                angle: self.rng.range(0.0, TAU),
                angular_speed: dir * 0.026,
                hull_length: 2.0,
                searchlight_phase: self.rng.range(0.0, TAU),
            });
        } else {
            for ship in &mut self.airships {
                ship.angle = (ship.angle + 0.18) % TAU;
            }
        }
    }

    /// Toggle rooftop and airship searchlights on or off.
    pub fn toggle_searchlights(&mut self) {
        self.searchlights_enabled = !self.searchlights_enabled;
    }

    /// Rotate the isometric camera by 90 degrees around the city island.
    pub fn rotate_camera_quarter_turn(&mut self) {
        self.target_camera_yaw += FRAC_PI_2;
    }

    /// Orbit camera directly during Direct Interaction Mode drag.
    pub fn orbit_camera(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.camera_yaw += delta_yaw;
        self.target_camera_yaw = self.camera_yaw;
        self.camera_pitch = (self.camera_pitch + delta_pitch).clamp(0.36, 0.82);
    }

    /// Smoothly reset the camera back to its default isometric angle and overview framing.
    pub fn reset_camera(&mut self) {
        let wrapped_offset = (self.camera_yaw - FRAC_PI_4 + std::f32::consts::PI).rem_euclid(TAU)
            - std::f32::consts::PI;
        self.camera_yaw = FRAC_PI_4 + wrapped_offset;
        self.target_camera_yaw = FRAC_PI_4;
        self.camera_pitch = DEFAULT_CAMERA_PITCH;
        self.follow_car = false;
    }

    /// Cycle to the next gothic color palette.
    pub fn cycle_palette(&mut self) {
        self.palette = self.palette.next();
        self.palette_blend = 0.0;
    }

    /// Set a specific gothic color palette.
    pub fn set_palette(&mut self, palette: GothicPalette) {
        self.palette = palette;
        self.palette_blend = 0.0;
    }

    /// Toggle smooth automatic palette cycling on or off.
    pub fn toggle_palette_cycle(&mut self) {
        self.palette_cycle_enabled = !self.palette_cycle_enabled;
    }

    /// Set the smooth palette cycling speed multiplier (`0.1` .. `5.0`).
    pub fn set_palette_cycle_speed(&mut self, speed: f32) {
        self.palette_cycle_speed = speed.clamp(0.1, 5.0);
    }

    /// Trigger a dramatic burst of rooftop and street steam plumes.
    pub fn trigger_steam_burst(&mut self) {
        let sources = self.blueprint.steam_sources.clone();
        for src in &sources {
            for _ in 0..2 {
                if self.steam_plumes.len() >= 80 {
                    break;
                }
                let vx = self.rng.range(-0.16, 0.16);
                let vz = self.rng.range(-0.16, 0.16);
                let vy = self.rng.range(0.55, 0.95);
                self.steam_plumes.push(SteamPlume {
                    pos: src.pos,
                    vx,
                    vy,
                    vz,
                    age: 0.0,
                    max_age: self.rng.range(2.2, 3.6),
                    base_radius: 0.18,
                });
            }
        }
    }

    pub fn stats(&self) -> GothicMetropolisStats {
        GothicMetropolisStats {
            active_trains: self.trains.len() as u32,
            trains_dispatched: self.trains_dispatched,
            active_airships: self.airships.len() as u32,
            street_vehicles: self.vehicles.len() as u32,
            walkway_pedestrians: self.pedestrians.len() as u32,
            palette_name: self.palette.name().to_string(),
            searchlights_lit: self.searchlights_enabled,
        }
    }
}
