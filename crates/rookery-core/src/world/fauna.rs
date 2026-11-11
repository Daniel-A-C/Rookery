//! Autonomous terrestrial woodland fauna simulation (Milestone 10).
//!
//! Owns ground creature behaviors (foxes, squirrels, badgers, hedgehogs),
//! horizontal patrolling, ground foraging, squirrel trunk climbing,
//! direct-mode cursor scattering, and inter-species bird startling.

use super::*;

impl WorldState {
    /// Advance woodland fauna simulation by `dt_secs`.
    pub fn update_fauna(&mut self, dt_secs: f32, interactive: bool, pointer: Option<[f32; 2]>) {
        if dt_secs <= 0.0 || self.fauna.is_empty() {
            return;
        }

        let ground_y = 746.0f32;
        let trunk_base_x = self
            .trees
            .iter()
            .find(|t| t.id == self.active_tree_id)
            .map(|t| t.base_pos[0])
            .unwrap_or(500.0);

        let vp_x = self.viewport_offset_x;
        let grove_min_x = vp_x - 250.0;
        let grove_max_x = vp_x + 1250.0;

        // Check for ground-level snacks that can attract hungry foragers
        let ground_snack_x = self.snacks.first().and_then(|snack| {
            self.branches
                .iter()
                .find(|b| b.id == snack.branch_id)
                .map(|b| b.point_at(snack.t)[0])
        });

        // Collect fox positions for bird startling
        let mut fox_positions = Vec::new();

        for fauna in &mut self.fauna {
            fauna.anim_phase += dt_secs * 4.0;

            // Direct-mode pointer interaction: clicking or moving close startles creatures
            if interactive
                && let Some(p) = pointer
                && (p[0] - fauna.pos[0]).abs() < 65.0
                && (p[1] - fauna.pos[1]).abs() < 60.0
                && !matches!(fauna.state, FaunaState::Startled { .. })
            {
                let flee_dir = if fauna.pos[0] < p[0] { -1.0 } else { 1.0 };
                fauna.facing_left = flee_dir < 0.0;
                fauna.state = FaunaState::Startled {
                    remaining_secs: 2.2,
                    dir: flee_dir,
                };
            }

            match &mut fauna.state {
                FaunaState::Trotting { target_x, speed } => {
                    let dx = *target_x - fauna.pos[0];
                    if dx.abs() < 6.0 {
                        // Arrived at destination: choose next activity
                        let roll = self.rng.f32();
                        if let Some(sx) = ground_snack_x
                            && (fauna.pos[0] - sx).abs() < 180.0
                            && roll < 0.45
                        {
                            // Attracted to investigate nearby dropped snack
                            fauna.state = FaunaState::Foraging {
                                remaining_secs: self.rng.range(3.0, 7.0),
                                target_snack_x: Some(sx),
                            };
                        } else if fauna.species == FaunaSpecies::Squirrel
                            && (fauna.pos[0] - trunk_base_x).abs() < 30.0
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
                            // Forage around roots
                            fauna.state = FaunaState::Foraging {
                                remaining_secs: self.rng.range(2.5, 5.0),
                                target_snack_x: None,
                            };
                        } else if roll < 0.70 {
                            // Rest under branch shelter
                            fauna.state = FaunaState::Resting {
                                remaining_secs: self.rng.range(3.0, 6.0),
                            };
                        } else if roll < 0.82 && fauna.species != FaunaSpecies::Squirrel {
                            // Curl up to sleep
                            fauna.state = FaunaState::Sleeping {
                                remaining_secs: self.rng.range(6.0, 14.0),
                            };
                        } else {
                            // Pick a new target along the woodland floor
                            let new_target =
                                self.rng.range(trunk_base_x - 300.0, trunk_base_x + 300.0);
                            let new_speed = fauna.species.base_speed() * self.rng.range(0.85, 1.20);
                            *target_x = new_target.clamp(grove_min_x, grove_max_x);
                            *speed = new_speed;
                        }
                        fauna.vel = [0.0, 0.0];
                    } else {
                        let step = *speed * dt_secs;
                        let dir = if dx > 0.0 { 1.0 } else { -1.0 };
                        fauna.facing_left = dir < 0.0;
                        fauna.pos[0] += dir * step.min(dx.abs());
                        fauna.pos[1] = ground_y;
                        fauna.vel = [dir * *speed, 0.0];
                    }
                }
                FaunaState::Sniffing { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let new_target = self.rng.range(trunk_base_x - 320.0, trunk_base_x + 320.0);
                        fauna.state = FaunaState::Trotting {
                            target_x: new_target.clamp(grove_min_x, grove_max_x),
                            speed: fauna.species.base_speed() * self.rng.range(0.9, 1.15),
                        };
                    }
                }
                FaunaState::Foraging {
                    remaining_secs,
                    target_snack_x,
                } => {
                    *remaining_secs -= dt_secs;
                    if let Some(sx) = target_snack_x {
                        let dx = *sx - fauna.pos[0];
                        if dx.abs() > 4.0 {
                            let step = fauna.species.base_speed() * 0.6 * dt_secs;
                            let dir = if dx > 0.0 { 1.0 } else { -1.0 };
                            fauna.pos[0] += dir * step.min(dx.abs());
                            fauna.facing_left = dir < 0.0;
                        }
                    }
                    if *remaining_secs <= 0.0 {
                        let new_target = self.rng.range(trunk_base_x - 280.0, trunk_base_x + 280.0);
                        fauna.state = FaunaState::Trotting {
                            target_x: new_target.clamp(grove_min_x, grove_max_x),
                            speed: fauna.species.base_speed(),
                        };
                    }
                }
                FaunaState::Resting { remaining_secs } => {
                    *remaining_secs -= dt_secs;
                    fauna.vel = [0.0, 0.0];
                    if *remaining_secs <= 0.0 {
                        let new_target = self.rng.range(trunk_base_x - 300.0, trunk_base_x + 300.0);
                        fauna.state = FaunaState::Trotting {
                            target_x: new_target.clamp(grove_min_x, grove_max_x),
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
                        let new_target = self.rng.range(trunk_base_x - 320.0, trunk_base_x + 320.0);
                        fauna.state = FaunaState::Trotting {
                            target_x: new_target.clamp(grove_min_x, grove_max_x),
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
                    fauna.pos[0] += *dir * speed * dt_secs;
                    fauna.pos[1] = ground_y;
                    fauna.vel = [*dir * speed, 0.0];
                    fauna.facing_left = *dir < 0.0;

                    // Bound within grove
                    if fauna.pos[0] < grove_min_x {
                        fauna.pos[0] = grove_min_x;
                        *dir = 1.0;
                    } else if fauna.pos[0] > grove_max_x {
                        fauna.pos[0] = grove_max_x;
                        *dir = -1.0;
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
                        fauna.pos[1] = ground_y - *height;
                        fauna.pos[0] = trunk_base_x + (fauna.anim_phase.sin() * 2.0);
                        fauna.vel = [0.0, -*speed];
                        if *height >= 130.0 {
                            *ascending = false;
                        }
                    } else {
                        *height -= *speed * dt_secs;
                        fauna.pos[1] = ground_y - *height;
                        fauna.pos[0] = trunk_base_x - (fauna.anim_phase.sin() * 2.0);
                        fauna.vel = [0.0, *speed];
                        if *height <= 0.0 {
                            fauna.pos[1] = ground_y;
                            fauna.vel = [0.0, 0.0];
                            let new_target =
                                self.rng.range(trunk_base_x - 260.0, trunk_base_x + 260.0);
                            fauna.state = FaunaState::Trotting {
                                target_x: new_target.clamp(grove_min_x, grove_max_x),
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
