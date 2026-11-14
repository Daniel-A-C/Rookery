//! Autonomous terrestrial woodland fauna simulation (Milestone 10 & Verdant Meadow).
//!
//! Owns ground creature behaviors (foxes, squirrels, badgers, hedgehogs, rabbits),
//! 2D meadow roaming, ground foraging, squirrel trunk climbing,
//! direct-mode cursor scattering, and inter-species bird startling.

use super::*;

/// Maximum number of terrestrial woodland creatures roaming the floor simultaneously.
/// Keeps the floor uncluttered so the main focus remains on tree growth and birds.
pub const MAX_FLOOR_FAUNA: usize = 2;

impl WorldState {
    /// Populate default starter woodland creatures roaming the 2D verdant meadow.
    /// Strictly limits floor population to 1 or 2 quiet woodland creatures at a time.
    pub fn seed_default_fauna(&mut self) {
        let trunk_base_x = self
            .trees
            .iter()
            .find(|t| t.id == self.active_tree_id)
            .or_else(|| self.trees.last())
            .map(|t| t.base_pos[0])
            .unwrap_or(500.0);

        self.fauna = vec![
            WoodlandFauna {
                id: 1,
                species: FaunaSpecies::Fox,
                pos: [trunk_base_x - 140.0, 758.0],
                vel: [0.0, 0.0],
                facing_left: false,
                state: FaunaState::Trotting {
                    target: [trunk_base_x + 160.0, 775.0],
                    speed: FaunaSpecies::Fox.base_speed(),
                },
                anim_phase: 0.0,
                size_scale: 1.0,
            },
            WoodlandFauna {
                id: 2,
                species: FaunaSpecies::Squirrel,
                pos: [trunk_base_x + 35.0, 748.0],
                vel: [0.0, 0.0],
                facing_left: true,
                state: FaunaState::Foraging {
                    remaining_secs: 4.0,
                    target_snack: None,
                },
                anim_phase: 1.2,
                size_scale: 0.95,
            },
        ];
    }

    /// Spawn a new woodland creature of the given species roaming the 2D ground meadow,
    /// strictly enforcing that at most 1 or 2 creatures roam the floor at a time.
    pub fn spawn_fauna_of_species(&mut self, species: FaunaSpecies) -> u32 {
        while self.fauna.len() >= MAX_FLOOR_FAUNA {
            self.fauna.remove(0);
        }

        let id = self.next_id;
        self.next_id += 1;

        let trunk_base_x = self
            .trees
            .iter()
            .find(|t| t.id == self.active_tree_id)
            .map(|t| t.base_pos[0])
            .unwrap_or(500.0);

        let ground_min_y = 746.0f32;
        let ground_max_y = 835.0f32;

        let pos_x = self.rng.range(trunk_base_x - 280.0, trunk_base_x + 280.0);
        let pos_y = self.rng.range(ground_min_y, ground_max_y);
        let target_x = self.rng.range(trunk_base_x - 280.0, trunk_base_x + 280.0);
        let target_y = self.rng.range(ground_min_y, ground_max_y);

        let facing_left = target_x < pos_x;
        self.fauna.push(WoodlandFauna {
            id,
            species,
            pos: [pos_x, pos_y],
            vel: [0.0, 0.0],
            facing_left,
            state: FaunaState::Trotting {
                target: [target_x, target_y],
                speed: species.base_speed() * self.rng.range(0.9, 1.15),
            },
            anim_phase: self.rng.range(0.0, std::f32::consts::TAU),
            size_scale: self.rng.range(0.92, 1.08),
        });

        id
    }

    /// Advance woodland fauna simulation by `dt_secs`.
    pub fn update_fauna(&mut self, dt_secs: f32, interactive: bool, pointer: Option<[f32; 2]>) {
        if dt_secs <= 0.0 {
            return;
        }

        // Strictly enforce at most MAX_FLOOR_FAUNA (1 or 2 creatures) on the floor
        if self.fauna.len() > MAX_FLOOR_FAUNA {
            self.fauna.truncate(MAX_FLOOR_FAUNA);
        }

        let ground_min_y = 746.0f32;
        let ground_max_y = 835.0f32;
        let trunk_base_x = self
            .trees
            .iter()
            .find(|t| t.id == self.active_tree_id)
            .map(|t| t.base_pos[0])
            .unwrap_or(500.0);

        let vp_x = self.viewport_offset_x;
        let grove_min_x = vp_x - 280.0;
        let grove_max_x = vp_x + 1280.0;

        // Ambient visitor arrival: if fewer than MAX_FLOOR_FAUNA creatures are present,
        // a peaceful woodland creature from an unrepresented species occasionally enters from the deep woods.
        if self.fauna.len() < MAX_FLOOR_FAUNA && self.rng.f32() < (dt_secs * 0.012) {
            let current_species: Vec<_> = self.fauna.iter().map(|f| f.species).collect();
            let all_species = [
                FaunaSpecies::Fox,
                FaunaSpecies::Squirrel,
                FaunaSpecies::Badger,
                FaunaSpecies::Hedgehog,
                FaunaSpecies::Rabbit,
            ];
            let species = all_species
                .iter()
                .copied()
                .find(|s| !current_species.contains(s))
                .unwrap_or(FaunaSpecies::Fox);

            let from_left = self.rng.bool_with_prob(0.5);
            let start_x = if from_left {
                grove_min_x - 20.0
            } else {
                grove_max_x + 20.0
            };
            let start_y = self.rng.range(ground_min_y, ground_max_y);
            let target_x = self.rng.range(trunk_base_x - 180.0, trunk_base_x + 180.0);
            let target_y = self.rng.range(ground_min_y, ground_max_y);
            let id = self.next_id;
            self.next_id += 1;
            self.fauna.push(WoodlandFauna {
                id,
                species,
                pos: [start_x, start_y],
                vel: [0.0, 0.0],
                facing_left: start_x > target_x,
                state: FaunaState::Trotting {
                    target: [target_x, target_y],
                    speed: species.base_speed() * self.rng.range(0.9, 1.15),
                },
                anim_phase: self.rng.range(0.0, std::f32::consts::TAU),
                size_scale: self.rng.range(0.95, 1.05),
            });
        }

        if self.fauna.is_empty() {
            return;
        }

        // Check for ground-level points of interest (snacks, bird baths, lanterns)
        let ground_snack = self.snacks.first().and_then(|snack| {
            self.branches
                .iter()
                .find(|b| b.id == snack.branch_id)
                .map(|b| b.point_at(snack.t))
        });
        let bath_pos = self.bird_baths.first().map(|b| [b.pos[0], 756.0]);
        let lantern_pos = self.lanterns.first().and_then(|l| {
            self.branches
                .iter()
                .find(|b| b.id == l.branch_id)
                .map(|b| [b.point_at(l.t)[0], 760.0])
        });
        let low_bird_pos = self.birds.iter().find(|b| b.pos[1] > 690.0).map(|b| b.pos);

        // Collect fox positions for bird & prey startling
        let mut fox_positions = Vec::new();
        let mut fox_alert_positions = Vec::new();

        for fauna in &mut self.fauna {
            let phase_rate = match &fauna.state {
                FaunaState::Startled { .. } | FaunaState::Pouncing { .. } => 9.5,
                FaunaState::ClimbingTrunk { .. } => 7.5,
                FaunaState::Trotting { .. } => match fauna.species {
                    FaunaSpecies::Rabbit | FaunaSpecies::Squirrel => 5.6,
                    FaunaSpecies::Hedgehog => 5.0,
                    FaunaSpecies::Fox => 4.8,
                    FaunaSpecies::Badger => 3.8,
                },
                FaunaState::Digging { .. } | FaunaState::Grooming { .. } => 5.0,
                FaunaState::Foraging { .. } | FaunaState::Sniffing { .. } => 3.0,
                FaunaState::Prowling { .. } => 2.8,
                FaunaState::Alert { .. } | FaunaState::Curled { .. } => 1.8,
                FaunaState::Resting { .. } | FaunaState::Sleeping { .. } => 1.0,
            };
            fauna.anim_phase =
                (fauna.anim_phase + dt_secs * phase_rate) % (std::f32::consts::TAU * 16.0);

            // Direct-mode pointer interaction: clicking or moving close startles creatures in 2D
            if interactive
                && let Some(p) = pointer
                && (p[0] - fauna.pos[0]).abs() < 65.0
                && (p[1] - fauna.pos[1]).abs() < 60.0
                && !matches!(
                    fauna.state,
                    FaunaState::Startled { .. } | FaunaState::Curled { .. }
                )
            {
                if fauna.species == FaunaSpecies::Hedgehog {
                    // Hedgehogs curl into a tight defensive spiky ball in place
                    fauna.vel = [0.0, 0.0];
                    fauna.state = FaunaState::Curled {
                        remaining_secs: 2.6,
                    };
                } else if fauna.species == FaunaSpecies::Squirrel
                    && (fauna.pos[0] - trunk_base_x).abs() < 90.0
                    && fauna.pos[1] <= ground_min_y + 35.0
                    && !matches!(fauna.state, FaunaState::ClimbingTrunk { .. })
                {
                    // Squirrels near the trunk dart straight up to safety
                    fauna.state = FaunaState::ClimbingTrunk {
                        branch_id: 1,
                        height: 0.0,
                        speed: 68.0,
                        ascending: true,
                    };
                } else {
                    let mut flee_dir = [fauna.pos[0] - p[0], fauna.pos[1] - p[1]];
                    let len = (flee_dir[0] * flee_dir[0] + flee_dir[1] * flee_dir[1])
                        .sqrt()
                        .max(0.001);
                    flee_dir[0] /= len;
                    flee_dir[1] /= len;
                    fauna.facing_left = flee_dir[0] < 0.0;
                    fauna.state = FaunaState::Startled {
                        remaining_secs: 2.2,
                        dir: flee_dir,
                    };
                }
            }

            // Helper closure for picking a meadow wander target (biased toward POIs occasionally)
            let pick_wander_target = |species: FaunaSpecies, rng: &mut WorldRng| -> [f32; 2] {
                let poi_roll = rng.f32();
                if poi_roll < 0.14
                    && let Some(bp) = bath_pos
                {
                    return [
                        (bp[0] + rng.range(-24.0, 24.0)).clamp(grove_min_x, grove_max_x),
                        rng.range(ground_min_y, ground_min_y + 20.0),
                    ];
                }
                if poi_roll < 0.26
                    && let Some(lp) = lantern_pos
                {
                    return [
                        (lp[0] + rng.range(-30.0, 30.0)).clamp(grove_min_x, grove_max_x),
                        rng.range(ground_min_y, ground_min_y + 25.0),
                    ];
                }
                if matches!(species, FaunaSpecies::Badger | FaunaSpecies::Squirrel)
                    && poi_roll < 0.45
                {
                    return [
                        (trunk_base_x + rng.range(-85.0, 85.0)).clamp(grove_min_x, grove_max_x),
                        rng.range(ground_min_y, ground_min_y + 24.0),
                    ];
                }
                let span = if species == FaunaSpecies::Hedgehog {
                    180.0
                } else {
                    300.0
                };
                [
                    (trunk_base_x + rng.range(-span, span)).clamp(grove_min_x, grove_max_x),
                    rng.range(ground_min_y, ground_max_y),
                ]
            };

            match &mut fauna.state {
                FaunaState::Trotting { target, speed } => {
                    let dx = target[0] - fauna.pos[0];
                    let dy = target[1] - fauna.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist < 6.0 {
                        // Arrived at destination: choose species-authentic next activity
                        let roll = self.rng.f32();
                        if let Some(snack_pt) = ground_snack
                            && (fauna.pos[0] - snack_pt[0]).abs() < 180.0
                            && roll < 0.45
                        {
                            if fauna.species == FaunaSpecies::Fox {
                                fauna.state = FaunaState::Prowling {
                                    target: [
                                        snack_pt[0].clamp(grove_min_x, grove_max_x),
                                        snack_pt[1].clamp(ground_min_y, ground_max_y),
                                    ],
                                    speed: FaunaSpecies::Fox.base_speed() * 0.55,
                                };
                            } else {
                                fauna.state = FaunaState::Foraging {
                                    remaining_secs: self.rng.range(3.0, 6.5),
                                    target_snack: Some(snack_pt),
                                };
                            }
                        } else if fauna.species == FaunaSpecies::Squirrel
                            && (fauna.pos[0] - trunk_base_x).abs() < 36.0
                            && fauna.pos[1] <= ground_min_y + 14.0
                            && roll < 0.38
                        {
                            fauna.state = FaunaState::ClimbingTrunk {
                                branch_id: 1,
                                height: 0.0,
                                speed: 42.0,
                                ascending: true,
                            };
                        } else if fauna.species == FaunaSpecies::Fox
                            && let Some(bird_pt) = low_bird_pos
                            && (fauna.pos[0] - bird_pt[0]).abs() < 200.0
                            && roll < 0.35
                        {
                            fauna.state = FaunaState::Prowling {
                                target: [
                                    bird_pt[0].clamp(grove_min_x, grove_max_x),
                                    ground_min_y + 4.0,
                                ],
                                speed: FaunaSpecies::Fox.base_speed() * 0.52,
                            };
                        } else if matches!(
                            fauna.species,
                            FaunaSpecies::Badger | FaunaSpecies::Squirrel
                        ) && (fauna.pos[0] - trunk_base_x).abs() < 95.0
                            && roll < 0.30
                        {
                            fauna.state = FaunaState::Digging {
                                remaining_secs: self.rng.range(2.2, 4.2),
                            };
                        } else if matches!(
                            fauna.species,
                            FaunaSpecies::Rabbit | FaunaSpecies::Squirrel
                        ) && roll < 0.28
                        {
                            fauna.state = FaunaState::Grooming {
                                remaining_secs: self.rng.range(2.2, 4.0),
                            };
                        } else if roll < 0.42 {
                            fauna.state = FaunaState::Sniffing {
                                remaining_secs: self.rng.range(1.8, 4.0),
                            };
                        } else if roll < 0.60 {
                            fauna.state = FaunaState::Foraging {
                                remaining_secs: self.rng.range(2.5, 5.0),
                                target_snack: None,
                            };
                        } else if roll < 0.74 {
                            fauna.state = FaunaState::Resting {
                                remaining_secs: self.rng.range(3.0, 6.0),
                            };
                        } else if roll < 0.84
                            && fauna.species != FaunaSpecies::Squirrel
                            && fauna.species != FaunaSpecies::Rabbit
                        {
                            fauna.state = FaunaState::Sleeping {
                                remaining_secs: self.rng.range(6.0, 14.0),
                            };
                        } else if roll < 0.96 {
                            let next_t = pick_wander_target(fauna.species, &mut self.rng);
                            let new_speed = fauna.species.base_speed() * self.rng.range(0.85, 1.20);
                            *target = next_t;
                            *speed = new_speed;
                        } else {
                            let exit_x = if self.rng.bool_with_prob(0.5) {
                                grove_min_x - 65.0
                            } else {
                                grove_max_x + 65.0
                            };
                            let exit_y = self.rng.range(ground_min_y, ground_max_y);
                            *target = [exit_x, exit_y];
                            *speed = fauna.species.base_speed() * 0.9;
                        }
                        fauna.vel = [0.0, 0.0];
                    } else {
                        // Species-specific gait modulation (hop bursts for rabbits/squirrels)
                        let gait_mult = match fauna.species {
                            FaunaSpecies::Rabbit => {
                                if fauna.anim_phase.sin() > -0.20 {
                                    1.35
                                } else {
                                    0.15
                                }
                            }
                            FaunaSpecies::Squirrel => {
                                if fauna.anim_phase.sin() > -0.30 {
                                    1.45
                                } else {
                                    0.10
                                }
                            }
                            FaunaSpecies::Hedgehog => 0.75 + 0.35 * fauna.anim_phase.sin(),
                            FaunaSpecies::Fox | FaunaSpecies::Badger => 1.0,
                        };
                        let eff_speed = *speed * gait_mult;
                        let step = (eff_speed * dt_secs).min(dist);
                        let dir_x = dx / dist;
                        let dir_y = dy / dist;
                        fauna.facing_left = dir_x < 0.0;
                        fauna.pos[0] += dir_x * step;
                        fauna.pos[1] =
                            (fauna.pos[1] + dir_y * step).clamp(ground_min_y, ground_max_y);
                        fauna.vel = [dir_x * eff_speed, dir_y * eff_speed];
                    }
                }
                FaunaState::Prowling { target, speed } => {
                    let dx = target[0] - fauna.pos[0];
                    let dy = target[1] - fauna.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 24.0 {
                        let dir_x = if dist > 0.01 {
                            dx / dist
                        } else if fauna.facing_left {
                            -1.0
                        } else {
                            1.0
                        };
                        let dir_y = if dist > 0.01 { dy / dist } else { 0.0 };
                        fauna.state = FaunaState::Pouncing {
                            remaining_secs: 0.55,
                            dir: [dir_x, dir_y],
                        };
                    } else {
                        let step = (*speed * dt_secs).min(dist);
                        let dir_x = dx / dist;
                        let dir_y = dy / dist;
                        fauna.facing_left = dir_x < 0.0;
                        fauna.pos[0] += dir_x * step;
                        fauna.pos[1] =
                            (fauna.pos[1] + dir_y * step).clamp(ground_min_y, ground_max_y);
                        fauna.vel = [dir_x * *speed, dir_y * *speed];
                    }
                }
                FaunaState::Pouncing {
                    remaining_secs,
                    dir,
                } => {
                    *remaining_secs -= dt_secs;
                    let pounce_speed = fauna.species.base_speed() * 1.65;
                    fauna.pos[0] = (fauna.pos[0] + dir[0] * pounce_speed * dt_secs)
                        .clamp(grove_min_x, grove_max_x);
                    fauna.pos[1] = (fauna.pos[1] + dir[1] * pounce_speed * dt_secs)
                        .clamp(ground_min_y, ground_max_y);
                    fauna.vel = [dir[0] * pounce_speed, dir[1] * pounce_speed];
                    if *remaining_secs <= 0.0 {
                        fauna.vel = [0.0, 0.0];
                        fauna.state = FaunaState::Sniffing {
                            remaining_secs: self.rng.range(1.4, 2.6),
                        };
                    }
                }
                FaunaState::Digging { remaining_secs }
                | FaunaState::Grooming { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let next_t = pick_wander_target(fauna.species, &mut self.rng);
                        fauna.state = FaunaState::Trotting {
                            target: next_t,
                            speed: fauna.species.base_speed() * self.rng.range(0.9, 1.15),
                        };
                    }
                }
                FaunaState::Curled { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        fauna.state = FaunaState::Alert {
                            remaining_secs: 1.2,
                        };
                    }
                }
                FaunaState::Sniffing { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let next_t = pick_wander_target(fauna.species, &mut self.rng);
                        fauna.state = FaunaState::Trotting {
                            target: next_t,
                            speed: fauna.species.base_speed() * self.rng.range(0.9, 1.15),
                        };
                    }
                }
                FaunaState::Foraging {
                    remaining_secs,
                    target_snack,
                } => {
                    *remaining_secs -= dt_secs;
                    if let Some(snack_pt) = target_snack {
                        let dx = snack_pt[0] - fauna.pos[0];
                        let dy = snack_pt[1] - fauna.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 4.0 {
                            let step = fauna.species.base_speed() * 0.6 * dt_secs;
                            let dir_x = dx / dist;
                            let dir_y = dy / dist;
                            fauna.pos[0] += dir_x * step.min(dist);
                            fauna.pos[1] = (fauna.pos[1] + dir_y * step.min(dist))
                                .clamp(ground_min_y, ground_max_y);
                            fauna.facing_left = dir_x < 0.0;
                            fauna.vel = [
                                dir_x * fauna.species.base_speed() * 0.6,
                                dir_y * fauna.species.base_speed() * 0.6,
                            ];
                        } else {
                            fauna.vel = [0.0, 0.0];
                        }
                    } else {
                        fauna.vel = [0.0, 0.0];
                    }
                    if *remaining_secs <= 0.0 {
                        let next_t = pick_wander_target(fauna.species, &mut self.rng);
                        fauna.state = FaunaState::Trotting {
                            target: next_t,
                            speed: fauna.species.base_speed(),
                        };
                    }
                }
                FaunaState::Resting { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let next_t = pick_wander_target(fauna.species, &mut self.rng);
                        fauna.state = FaunaState::Trotting {
                            target: next_t,
                            speed: fauna.species.base_speed(),
                        };
                    }
                }
                FaunaState::Sleeping { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        fauna.state = FaunaState::Alert {
                            remaining_secs: 1.5,
                        };
                    }
                }
                FaunaState::Alert { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let next_t = pick_wander_target(fauna.species, &mut self.rng);
                        fauna.state = FaunaState::Trotting {
                            target: next_t,
                            speed: fauna.species.base_speed(),
                        };
                    }
                }
                FaunaState::Startled {
                    remaining_secs,
                    dir,
                } => {
                    *remaining_secs -= dt_secs;
                    let speed = fauna.species.base_speed() * 1.85;
                    fauna.pos[0] += dir[0] * speed * dt_secs;
                    fauna.pos[1] += dir[1] * speed * dt_secs;
                    fauna.vel = [dir[0] * speed, dir[1] * speed];
                    fauna.facing_left = dir[0] < 0.0;

                    // Bound within meadow
                    if fauna.pos[0] < grove_min_x {
                        fauna.pos[0] = grove_min_x;
                        dir[0] = dir[0].abs();
                    } else if fauna.pos[0] > grove_max_x {
                        fauna.pos[0] = grove_max_x;
                        dir[0] = -dir[0].abs();
                    }
                    if fauna.pos[1] < ground_min_y {
                        fauna.pos[1] = ground_min_y;
                        dir[1] = dir[1].abs();
                    } else if fauna.pos[1] > ground_max_y {
                        fauna.pos[1] = ground_max_y;
                        dir[1] = -dir[1].abs();
                    }

                    if *remaining_secs <= 0.0 {
                        fauna.state = FaunaState::Alert {
                            remaining_secs: 1.2,
                        };
                        fauna.vel = [0.0, 0.0];
                    }
                }
                FaunaState::ClimbingTrunk {
                    branch_id: _,
                    height,
                    speed,
                    ascending,
                } => {
                    if *ascending {
                        *height += *speed * dt_secs;
                        fauna.pos[1] = ground_min_y - *height;
                        fauna.pos[0] = trunk_base_x + (fauna.anim_phase.sin() * 2.0);
                        fauna.vel = [0.0, -*speed];
                        if *height >= 130.0 {
                            *ascending = false;
                        }
                    } else {
                        *height -= *speed * dt_secs;
                        fauna.pos[1] = ground_min_y - *height;
                        fauna.pos[0] = trunk_base_x - (fauna.anim_phase.sin() * 2.0);
                        fauna.vel = [0.0, *speed];
                        if *height <= 0.0 {
                            fauna.pos[1] = ground_min_y;
                            fauna.vel = [0.0, 0.0];
                            let next_t = pick_wander_target(fauna.species, &mut self.rng);
                            fauna.state = FaunaState::Trotting {
                                target: next_t,
                                speed: fauna.species.base_speed(),
                            };
                        }
                    }
                }
            }

            // Record fox position for bird & prey startling
            if fauna.species == FaunaSpecies::Fox
                && matches!(
                    fauna.state,
                    FaunaState::Trotting { .. }
                        | FaunaState::Prowling { .. }
                        | FaunaState::Pouncing { .. }
                        | FaunaState::Startled { .. }
                )
            {
                fox_positions.push(fauna.pos);
                fox_alert_positions.push(fauna.pos);
            }
        }

        // Retain creatures that are within the meadow and grove bounds;
        // any creature that has wandered off into deep woods retires gracefully.
        self.fauna
            .retain(|f| f.pos[0] >= grove_min_x - 50.0 && f.pos[0] <= grove_max_x + 50.0);

        // Inter-species interaction: rabbits and squirrels perk up or scurry when a fox approaches
        for fox_pos in &fox_alert_positions {
            for fauna in &mut self.fauna {
                if matches!(fauna.species, FaunaSpecies::Rabbit | FaunaSpecies::Squirrel) {
                    let dx = fauna.pos[0] - fox_pos[0];
                    let dy = fauna.pos[1] - fox_pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 65.0
                        && !matches!(
                            fauna.state,
                            FaunaState::Startled { .. } | FaunaState::ClimbingTrunk { .. }
                        )
                    {
                        if fauna.species == FaunaSpecies::Squirrel
                            && (fauna.pos[0] - trunk_base_x).abs() < 90.0
                            && fauna.pos[1] <= ground_min_y + 35.0
                        {
                            fauna.state = FaunaState::ClimbingTrunk {
                                branch_id: 1,
                                height: 0.0,
                                speed: 65.0,
                                ascending: true,
                            };
                        } else {
                            let len = dist.max(0.001);
                            let flee_dir = [dx / len, dy / len];
                            fauna.facing_left = flee_dir[0] < 0.0;
                            fauna.state = FaunaState::Startled {
                                remaining_secs: 1.6,
                                dir: flee_dir,
                            };
                        }
                    } else if dist < 115.0
                        && matches!(
                            fauna.state,
                            FaunaState::Trotting { .. }
                                | FaunaState::Sniffing { .. }
                                | FaunaState::Foraging { .. }
                                | FaunaState::Resting { .. }
                                | FaunaState::Grooming { .. }
                        )
                    {
                        fauna.facing_left = fox_pos[0] < fauna.pos[0];
                        fauna.vel = [0.0, 0.0];
                        fauna.state = FaunaState::Alert {
                            remaining_secs: 1.4,
                        };
                    }
                }
            }
        }

        // Inter-species interaction: foxes trotting beneath low perched birds startle them
        for fox_pos in fox_positions {
            for bird in &mut self.birds {
                if bird.pos[1] > 690.0 && (bird.pos[0] - fox_pos[0]).abs() < 55.0 {
                    match bird.state {
                        BirdState::Perched { .. }
                        | BirdState::Feeding { .. }
                        | BirdState::Bathing { .. } => {
                            let flee_dir = if bird.pos[0] > fox_pos[0] { 0.7 } else { -0.7 };
                            bird.state = BirdState::Startled {
                                remaining_secs: 1.8,
                                dir: [flee_dir, -1.0],
                            };
                            bird.vel = [flee_dir * 45.0, -90.0];
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
