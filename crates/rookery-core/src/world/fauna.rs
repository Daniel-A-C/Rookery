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

        // Check for ground-level snacks that can attract hungry foragers
        let ground_snack = self.snacks.first().and_then(|snack| {
            self.branches
                .iter()
                .find(|b| b.id == snack.branch_id)
                .map(|b| b.point_at(snack.t))
        });

        // Collect fox positions for bird startling
        let mut fox_positions = Vec::new();

        for fauna in &mut self.fauna {
            fauna.anim_phase += dt_secs * 4.0;

            // Direct-mode pointer interaction: clicking or moving close startles creatures in 2D
            if interactive
                && let Some(p) = pointer
                && (p[0] - fauna.pos[0]).abs() < 65.0
                && (p[1] - fauna.pos[1]).abs() < 60.0
                && !matches!(fauna.state, FaunaState::Startled { .. })
            {
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

            match &mut fauna.state {
                FaunaState::Trotting { target, speed } => {
                    let dx = target[0] - fauna.pos[0];
                    let dy = target[1] - fauna.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist < 6.0 {
                        // Arrived at destination: choose next activity
                        let roll = self.rng.f32();
                        if let Some(snack_pt) = ground_snack
                            && (fauna.pos[0] - snack_pt[0]).abs() < 180.0
                            && roll < 0.45
                        {
                            // Attracted to investigate nearby dropped snack
                            fauna.state = FaunaState::Foraging {
                                remaining_secs: self.rng.range(3.0, 7.0),
                                target_snack: Some(snack_pt),
                            };
                        } else if fauna.species == FaunaSpecies::Squirrel
                            && (fauna.pos[0] - trunk_base_x).abs() < 30.0
                            && fauna.pos[1] <= ground_min_y + 12.0
                            && roll < 0.35
                        {
                            // Squirrel scampers up the active tree trunk!
                            fauna.state = FaunaState::ClimbingTrunk {
                                branch_id: 1,
                                height: 0.0,
                                speed: 38.0,
                                ascending: true,
                            };
                        } else if roll < 0.35 {
                            // Stop and sniff the soil / clovers
                            fauna.state = FaunaState::Sniffing {
                                remaining_secs: self.rng.range(2.0, 4.5),
                            };
                        } else if roll < 0.55 {
                            // Forage around roots or through grass
                            fauna.state = FaunaState::Foraging {
                                remaining_secs: self.rng.range(2.5, 5.0),
                                target_snack: None,
                            };
                        } else if roll < 0.70 {
                            // Rest under branch shelter
                            fauna.state = FaunaState::Resting {
                                remaining_secs: self.rng.range(3.0, 6.0),
                            };
                        } else if roll < 0.82
                            && fauna.species != FaunaSpecies::Squirrel
                            && fauna.species != FaunaSpecies::Rabbit
                        {
                            // Curl up to sleep
                            fauna.state = FaunaState::Sleeping {
                                remaining_secs: self.rng.range(6.0, 14.0),
                            };
                        } else if roll < 0.96 {
                            // Pick a new 2D target along the verdant meadow
                            let new_x = self.rng.range(trunk_base_x - 300.0, trunk_base_x + 300.0);
                            let new_y = self.rng.range(ground_min_y, ground_max_y);
                            let new_speed = fauna.species.base_speed() * self.rng.range(0.85, 1.20);
                            *target = [new_x.clamp(grove_min_x, grove_max_x), new_y];
                            *speed = new_speed;
                        } else {
                            // Occasionally wander off into the deep surrounding woods to let another creature visit
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
                FaunaState::Sniffing { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let new_x = self.rng.range(trunk_base_x - 320.0, trunk_base_x + 320.0);
                        let new_y = self.rng.range(ground_min_y, ground_max_y);
                        fauna.state = FaunaState::Trotting {
                            target: [new_x.clamp(grove_min_x, grove_max_x), new_y],
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
                        }
                    }
                    if *remaining_secs <= 0.0 {
                        let new_x = self.rng.range(trunk_base_x - 280.0, trunk_base_x + 280.0);
                        let new_y = self.rng.range(ground_min_y, ground_max_y);
                        fauna.state = FaunaState::Trotting {
                            target: [new_x.clamp(grove_min_x, grove_max_x), new_y],
                            speed: fauna.species.base_speed(),
                        };
                    }
                }
                FaunaState::Resting { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let new_x = self.rng.range(trunk_base_x - 300.0, trunk_base_x + 300.0);
                        let new_y = self.rng.range(ground_min_y, ground_max_y);
                        fauna.state = FaunaState::Trotting {
                            target: [new_x.clamp(grove_min_x, grove_max_x), new_y],
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
                        let new_x = self.rng.range(trunk_base_x - 320.0, trunk_base_x + 320.0);
                        let new_y = self.rng.range(ground_min_y, ground_max_y);
                        fauna.state = FaunaState::Trotting {
                            target: [new_x.clamp(grove_min_x, grove_max_x), new_y],
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
                            let new_x = self.rng.range(trunk_base_x - 260.0, trunk_base_x + 260.0);
                            let new_y = self.rng.range(ground_min_y, ground_max_y);
                            fauna.state = FaunaState::Trotting {
                                target: [new_x.clamp(grove_min_x, grove_max_x), new_y],
                                speed: fauna.species.base_speed(),
                            };
                        }
                    }
                }
            }

            // Record fox position for bird startling
            if fauna.species == FaunaSpecies::Fox
                && matches!(
                    fauna.state,
                    FaunaState::Trotting { .. } | FaunaState::Startled { .. }
                )
            {
                fox_positions.push(fauna.pos);
            }
        }

        // Retain creatures that are within the meadow and grove bounds;
        // any creature that has wandered off into deep woods retires gracefully.
        self.fauna
            .retain(|f| f.pos[0] >= grove_min_x - 50.0 && f.pos[0] <= grove_max_x + 50.0);

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
