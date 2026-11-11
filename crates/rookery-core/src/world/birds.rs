//! Autonomous bird flocking, flight, perching, nesting, and feeding simulation.

use super::*;

impl WorldState {
    /// Spawn an autonomous bird of a specific species for debugging fauna.
    pub fn spawn_bird_of_species(&mut self, species: BirdSpecies) {
        let vp_x = self.viewport_offset_x;
        let mut bird_rng = WorldRng::new(self.rng.next_u64());
        let spawn_x = vp_x + bird_rng.range(200.0, 800.0);
        let spawn_y = bird_rng.range(150.0, 450.0);
        let speed = bird_rng.range(40.0, 80.0);

        self.birds.push(Bird {
            id: self.next_id + 1,
            species,
            pos: [spawn_x, spawn_y],
            vel: [bird_rng.range(-20.0, 20.0), bird_rng.range(-15.0, 15.0)],
            facing_left: bird_rng.bool_with_prob(0.5),
            state: BirdState::Flying {
                target: [vp_x + 500.0, 350.0],
                speed,
            },
            sing_timer: bird_rng.range(2.0, 6.0),
            wing_phase: bird_rng.range(0.0, std::f32::consts::TAU),
            size_scale: if species.is_rare() {
                1.25
            } else {
                bird_rng.range(0.9, 1.15)
            },
        });
        self.next_id += 1;
        self.recompute_stats();
    }

    /// Autonomous bird flocking, perching, feeding, nesting, and singing AI.
    pub fn update_birds(&mut self, dt_secs: f32, _interactive: bool, _pointer: Option<[f32; 2]>) {
        let branch_count = self.branches.len();
        if branch_count == 0 {
            return;
        }

        let vp_x = self.viewport_offset_x;
        let active_tree_base_x = self
            .trees
            .iter()
            .find(|t| t.id == self.active_tree_id)
            .map(|t| t.base_pos[0])
            .unwrap_or(vp_x + 500.0);

        // Decay water splash timers on bird baths
        for bb in &mut self.bird_baths {
            bb.splash_timer = (bb.splash_timer - dt_secs).max(0.0);
        }

        let mut new_notes = Vec::new();

        for i in 0..self.birds.len() {
            let mut next_state = None;
            let bird = &mut self.birds[i];

            bird.wing_phase = (bird.wing_phase + dt_secs * 12.0) % std::f32::consts::TAU;

            match &mut bird.state {
                BirdState::Flying { target, speed } => {
                    let dx = target[0] - bird.pos[0];
                    let dy = target[1] - bird.pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();

                    bird.facing_left = dx < 0.0;

                    if dist < 15.0 {
                        // Arrived near target: choose new action
                        let active_nests: Vec<&Nest> = self
                            .nests
                            .iter()
                            .filter(|n| {
                                if let Some(b) =
                                    self.branches.iter().find(|br| br.id == n.branch_id)
                                {
                                    let pt = b.point_at(n.t);
                                    pt[0] >= vp_x - 100.0 && pt[0] <= vp_x + 1100.0
                                } else {
                                    false
                                }
                            })
                            .collect();
                        let active_baths: Vec<u32> = self
                            .bird_baths
                            .iter()
                            .filter(|b| b.pos[0] >= vp_x - 100.0 && b.pos[0] <= vp_x + 1100.0)
                            .map(|b| b.id)
                            .collect();

                        let roll = self.rng.f32();
                        if roll < 0.40 && branch_count > 1 {
                            // Find suitable mature branch to perch on within visible viewport
                            let candidates: Vec<u32> = self
                                .branches
                                .iter()
                                .filter(|b| {
                                    !b.is_root
                                        && b.growth_progress >= 0.8
                                        && b.depth >= 1
                                        && (b.start[0].max(b.current_end()[0]) >= vp_x
                                            && b.start[0].min(b.current_end()[0]) <= vp_x + 1000.0)
                                })
                                .map(|b| b.id)
                                .collect();
                            if !candidates.is_empty() {
                                let branch_id = candidates
                                    [self.rng.range_i32(0, candidates.len() as i32) as usize];
                                let branch = self.branches.iter().find(|b| b.id == branch_id);
                                let t = if let Some(b) = branch {
                                    let x0 = b.start[0];
                                    let x1 = b.current_end()[0];
                                    if (x1 - x0).abs() > 1.0 {
                                        let t_min_vp =
                                            ((vp_x + 10.0 - x0) / (x1 - x0)).clamp(0.0, 1.0);
                                        let t_max_vp =
                                            ((vp_x + 990.0 - x0) / (x1 - x0)).clamp(0.0, 1.0);
                                        let t_low = t_min_vp.min(t_max_vp).max(0.35);
                                        let t_high = t_min_vp.max(t_max_vp).min(0.95);
                                        if t_low < t_high {
                                            self.rng.range(t_low, t_high)
                                        } else {
                                            self.rng.range(0.35, 0.95)
                                        }
                                    } else {
                                        self.rng.range(0.35, 0.95)
                                    }
                                } else {
                                    self.rng.range(0.35, 0.95)
                                };
                                next_state = Some(BirdState::ApproachingPerch {
                                    branch_id,
                                    t,
                                    speed: self.rng.range(50.0, 85.0),
                                });
                            }
                        } else if roll < 0.55 && !active_nests.is_empty() {
                            let n_idx = self.rng.range_i32(0, active_nests.len() as i32) as usize;
                            let nest = active_nests[n_idx];
                            if nest.progress < 1.0 {
                                next_state = Some(BirdState::ApproachingNest {
                                    nest_id: nest.id,
                                    is_brooding: false,
                                    speed: self.rng.range(50.0, 75.0),
                                });
                            } else if nest.eggs_count > 0 {
                                next_state = Some(BirdState::ApproachingNest {
                                    nest_id: nest.id,
                                    is_brooding: true,
                                    speed: self.rng.range(50.0, 75.0),
                                });
                            }
                        } else if roll < 0.68 && !active_baths.is_empty() {
                            let bath_id = active_baths
                                [self.rng.range_i32(0, active_baths.len() as i32) as usize];
                            next_state = Some(BirdState::ApproachingBath {
                                bath_id,
                                speed: self.rng.range(50.0, 75.0),
                            });
                        } else if roll < 0.82 {
                            let center = [active_tree_base_x, 300.0];
                            let rel_x = bird.pos[0] - center[0];
                            let rel_y = (bird.pos[1] - center[1]) / 0.45;
                            let cur_r = (rel_x * rel_x + rel_y * rel_y).sqrt();
                            let angle = rel_y.atan2(rel_x);
                            let target_r = cur_r.clamp(140.0, 360.0);
                            if (cur_r - target_r).abs() > 35.0 {
                                let entry_target = [
                                    center[0] + angle.cos() * target_r,
                                    center[1] + angle.sin() * target_r * 0.45,
                                ];
                                next_state = Some(BirdState::Flying {
                                    target: entry_target,
                                    speed: self.rng.range(50.0, 80.0),
                                });
                            } else {
                                let sp = self.rng.range(0.35, 0.75)
                                    * if self.rng.f32() > 0.5 { 1.0 } else { -1.0 };
                                next_state = Some(BirdState::Circling {
                                    center,
                                    radius: cur_r,
                                    angle,
                                    speed: sp,
                                });
                            }
                        } else {
                            let tx = vp_x + self.rng.range(120.0, 880.0);
                            let ty = self.rng.range(80.0, 480.0);
                            let sp = self.rng.range(40.0, 80.0);
                            next_state = Some(BirdState::Flying {
                                target: [tx, ty],
                                speed: sp,
                            });
                        }
                    } else {
                        let step = (*speed * dt_secs).min(dist);
                        bird.pos[0] += (dx / dist) * step;
                        bird.pos[1] += (dy / dist) * step;
                        bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                    }
                }
                BirdState::Circling {
                    center,
                    radius,
                    angle,
                    speed,
                } => {
                    *angle += *speed * dt_secs;
                    bird.pos[0] = center[0] + angle.cos() * *radius;
                    bird.pos[1] = center[1] + angle.sin() * *radius * 0.45;
                    bird.facing_left = *speed < 0.0;
                    bird.vel = [
                        -*speed * angle.sin() * *radius,
                        *speed * angle.cos() * *radius * 0.45,
                    ];

                    // Chance to break into direct flight from current orbit point
                    if self.rng.bool_with_prob(0.03 * dt_secs) {
                        let tx = vp_x + self.rng.range(100.0, 900.0);
                        let ty = self.rng.range(120.0, 450.0);
                        let sp = self.rng.range(45.0, 85.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: sp,
                        });
                    }
                }
                BirdState::ApproachingPerch {
                    branch_id,
                    t,
                    speed,
                } => {
                    let target_pos = self
                        .branches
                        .iter()
                        .find(|br| br.id == *branch_id)
                        .map(|b| b.point_at(*t));
                    if let Some(target) = target_pos {
                        let dx = target[0] - bird.pos[0];
                        let dy = target[1] - bird.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        bird.facing_left = dx < 0.0;

                        if dist < 12.0 {
                            bird.pos = target;
                            bird.vel = [0.0, 0.0];
                            next_state = Some(BirdState::Perched {
                                branch_id: *branch_id,
                                t: *t,
                                remaining_secs: self.rng.range(8.0, 22.0),
                            });
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                            bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                        }
                    } else {
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
                BirdState::ApproachingBath { bath_id, speed } => {
                    let bath_pos = self
                        .bird_baths
                        .iter()
                        .find(|b| b.id == *bath_id)
                        .map(|b| [b.pos[0], b.pos[1] - 8.0]);
                    if let Some(target) = bath_pos {
                        let dx = target[0] - bird.pos[0];
                        let dy = target[1] - bird.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        bird.facing_left = dx < 0.0;

                        if dist < 12.0 {
                            bird.pos = target;
                            bird.vel = [0.0, 0.0];
                            next_state = Some(BirdState::Bathing {
                                bath_id: *bath_id,
                                remaining_secs: self.rng.range(4.0, 7.0),
                            });
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                            bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                        }
                    } else {
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
                BirdState::ApproachingNest {
                    nest_id,
                    is_brooding,
                    speed,
                } => {
                    let nest_pos = self.nests.iter().find(|n| n.id == *nest_id).and_then(|n| {
                        self.branches
                            .iter()
                            .find(|b| b.id == n.branch_id)
                            .map(|b| b.point_at(n.t))
                    });
                    if let Some(target) = nest_pos {
                        let dx = target[0] - bird.pos[0];
                        let dy = target[1] - bird.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        bird.facing_left = dx < 0.0;

                        if dist < 12.0 {
                            bird.pos = target;
                            bird.vel = [0.0, 0.0];
                            if *is_brooding {
                                next_state = Some(BirdState::BroodingEggs {
                                    nest_id: *nest_id,
                                    remaining_secs: 7.0,
                                });
                            } else {
                                next_state = Some(BirdState::BuildingNest {
                                    nest_id: *nest_id,
                                    remaining_secs: 3.5,
                                });
                            }
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                            bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                        }
                    } else {
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
                BirdState::ApproachingFeed {
                    branch_id,
                    t,
                    speed,
                } => {
                    let branch_pos = self
                        .branches
                        .iter()
                        .find(|b| b.id == *branch_id)
                        .map(|b| b.point_at(*t));
                    if let Some(target) = branch_pos {
                        let dx = target[0] - bird.pos[0];
                        let dy = target[1] - bird.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        bird.facing_left = dx < 0.0;

                        if dist < 12.0 {
                            bird.pos = target;
                            bird.vel = [0.0, 0.0];
                            // Consume one serving on arrival
                            if let Some(snack) = self
                                .snacks
                                .iter_mut()
                                .find(|s| s.branch_id == *branch_id && (s.t - *t).abs() < 0.02)
                            {
                                snack.servings_left = snack.servings_left.saturating_sub(1);
                            }
                            next_state = Some(BirdState::Feeding {
                                branch_id: *branch_id,
                                t: *t,
                                remaining_secs: 5.0,
                            });
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                            bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                        }
                    } else {
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
                BirdState::Perched {
                    branch_id,
                    t,
                    remaining_secs,
                } => {
                    *remaining_secs -= dt_secs;

                    // Snap position to perched branch point
                    if let Some(b) = self.branches.iter().find(|br| br.id == *branch_id) {
                        bird.pos = b.point_at(*t);
                    }

                    // Sing periodically
                    bird.sing_timer -= dt_secs;
                    if bird.sing_timer <= 0.0 {
                        bird.sing_timer = self.rng.range(6.0, 20.0);
                        let glyphs = ['♪', '♫', '*', '~'];
                        let note_char = glyphs[self.rng.range_i32(0, glyphs.len() as i32) as usize];
                        new_notes.push(FloatingNote {
                            pos: [bird.pos[0], bird.pos[1] - 8.0],
                            vel: [self.rng.range(-8.0, 8.0), -self.rng.range(18.0, 32.0)],
                            glyph: note_char,
                            life: 0.0,
                            max_life: self.rng.range(1.8, 3.2),
                            color_idx: self.rng.range_i32(0, 3) as u8,
                        });
                    }

                    if *remaining_secs <= 0.0 {
                        // Check if an incomplete nest needs twigs
                        let incomplete_nest = self.nests.iter().find(|n| n.progress < 1.0);
                        if let Some(nest) = incomplete_nest
                            && self.rng.f32() < 0.45
                        {
                            next_state = Some(BirdState::GatheringTwig {
                                branch_id: *branch_id,
                                t: *t,
                                remaining_secs: 2.2,
                                target_nest_id: nest.id,
                            });
                        } else {
                            let tx = vp_x + self.rng.range(100.0, 900.0);
                            let ty = self.rng.range(80.0, 350.0);
                            let sp = self.rng.range(50.0, 90.0);
                            next_state = Some(BirdState::Flying {
                                target: [tx, ty],
                                speed: sp,
                            });
                        }
                    }
                }
                BirdState::GatheringTwig {
                    branch_id,
                    t,
                    remaining_secs,
                    target_nest_id,
                } => {
                    *remaining_secs -= dt_secs;
                    if let Some(b) = self.branches.iter().find(|br| br.id == *branch_id) {
                        bird.pos = b.point_at(*t);
                    }
                    if *remaining_secs <= 0.0 {
                        next_state = Some(BirdState::CarryingTwig {
                            target_nest_id: *target_nest_id,
                            speed: 70.0,
                        });
                    }
                }
                BirdState::CarryingTwig {
                    target_nest_id,
                    speed,
                } => {
                    let nest_pos = self
                        .nests
                        .iter()
                        .find(|n| n.id == *target_nest_id)
                        .and_then(|n| {
                            self.branches
                                .iter()
                                .find(|b| b.id == n.branch_id)
                                .map(|b| b.point_at(n.t))
                        });

                    if let Some(target) = nest_pos {
                        let dx = target[0] - bird.pos[0];
                        let dy = target[1] - bird.pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();
                        bird.facing_left = dx < 0.0;

                        if dist < 12.0 {
                            bird.pos = target;
                            bird.vel = [0.0, 0.0];
                            next_state = Some(BirdState::BuildingNest {
                                nest_id: *target_nest_id,
                                remaining_secs: 3.5,
                            });
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                            bird.vel = [(dx / dist) * *speed, (dy / dist) * *speed];
                        }
                    } else {
                        // Nest was pruned or missing
                        let tx = vp_x + self.rng.range(200.0, 800.0);
                        let ty = self.rng.range(100.0, 300.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 65.0,
                        });
                    }
                }
                BirdState::BuildingNest {
                    nest_id,
                    remaining_secs,
                } => {
                    *remaining_secs -= dt_secs;
                    let target_nest_id = *nest_id;
                    if let Some(n) = self.nests.iter().find(|n| n.id == target_nest_id)
                        && let Some(b) = self.branches.iter().find(|br| br.id == n.branch_id)
                    {
                        bird.pos = b.point_at(n.t);
                    }
                    if *remaining_secs <= 0.0 {
                        if let Some(nest) = self.nests.iter_mut().find(|n| n.id == target_nest_id) {
                            nest.progress = (nest.progress + 0.35).min(1.0);
                            if nest.progress >= 1.0 {
                                if let Some(b) =
                                    self.branches.iter_mut().find(|br| br.id == nest.branch_id)
                                {
                                    b.has_nest = true;
                                }
                                self.stats.nests_built += 1;
                            }
                        }
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 65.0,
                        });
                    }
                }
                BirdState::BroodingEggs {
                    nest_id,
                    remaining_secs,
                } => {
                    *remaining_secs -= dt_secs;
                    let target_nest_id = *nest_id;
                    if let Some(n) = self.nests.iter().find(|n| n.id == target_nest_id)
                        && let Some(b) = self.branches.iter().find(|br| br.id == n.branch_id)
                    {
                        bird.pos = b.point_at(n.t);
                    }
                    if *remaining_secs <= 0.0 {
                        if let Some(nest) = self.nests.iter_mut().find(|n| n.id == target_nest_id)
                            && nest.eggs_count > 0
                            && nest.chicks_count == 0
                        {
                            let hatched = nest.eggs_count.min(2);
                            nest.chicks_count = hatched;
                            nest.eggs_count = 0;
                            self.stats.chicks_hatched += hatched as u32;

                            for _ in 0..2 {
                                new_notes.push(FloatingNote {
                                    pos: [
                                        bird.pos[0] + self.rng.range(-4.0, 4.0),
                                        bird.pos[1] - 8.0,
                                    ],
                                    vel: [self.rng.range(-12.0, 12.0), -self.rng.range(18.0, 30.0)],
                                    glyph: '♥',
                                    life: 0.0,
                                    max_life: 2.5,
                                    color_idx: 0,
                                });
                            }
                        }
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 65.0,
                        });
                    }
                }
                BirdState::Bathing {
                    bath_id,
                    remaining_secs,
                } => {
                    *remaining_secs -= dt_secs;
                    let target_bath_id = *bath_id;
                    if let Some(bb) = self.bird_baths.iter_mut().find(|b| b.id == target_bath_id) {
                        bird.pos = [bb.pos[0] + self.rng.range(-5.0, 5.0), bb.pos[1] - 8.0];
                        bb.splash_timer = 0.5;
                    }
                    if *remaining_secs <= 0.0 {
                        let tx = vp_x + self.rng.range(200.0, 800.0);
                        let ty = self.rng.range(120.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 65.0,
                        });
                    }
                }
                BirdState::Feeding {
                    branch_id,
                    t,
                    remaining_secs,
                } => {
                    *remaining_secs -= dt_secs;
                    if let Some(b) = self.branches.iter().find(|br| br.id == *branch_id) {
                        bird.pos = b.point_at(*t);
                    }
                    if *remaining_secs <= 0.0 {
                        self.stats.snacks_eaten += 1;
                        let tx = vp_x + self.rng.range(200.0, 800.0);
                        let ty = self.rng.range(100.0, 300.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 65.0,
                        });
                    }
                }
                BirdState::Startled {
                    remaining_secs,
                    dir,
                } => {
                    *remaining_secs -= dt_secs;
                    bird.pos[0] += dir[0] * 120.0 * dt_secs;
                    bird.pos[1] += dir[1] * 120.0 * dt_secs;
                    bird.facing_left = dir[0] < 0.0;

                    if *remaining_secs <= 0.0 {
                        let tx = vp_x + self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
            }

            // Redirect stranded birds (e.g. from camera panning or succession) back toward active flock
            if (bird.pos[0] - (vp_x + 500.0)).abs() > 620.0 {
                next_state = Some(BirdState::Flying {
                    target: [
                        vp_x + self.rng.range(250.0, 750.0),
                        self.rng.range(150.0, 400.0),
                    ],
                    speed: 95.0,
                });
            }

            if let Some(st) = next_state {
                bird.state = st;
            }
        }

        self.notes.extend(new_notes);

        // Wildlife arrivals from off-screen margins
        if self.birds.len() < 8 && self.rng.bool_with_prob(0.015 * dt_secs) {
            let from_left = self.rng.f32() < 0.5;
            let start_x = if from_left {
                vp_x - 60.0
            } else {
                vp_x + 1060.0
            };
            let start_y = self.rng.range(120.0, 360.0);
            let species_pool = [
                BirdSpecies::Rook,
                BirdSpecies::AzureJay,
                BirdSpecies::Goldfinch,
                BirdSpecies::Robin,
                BirdSpecies::EmeraldDove,
                BirdSpecies::CelestialRaven,
                BirdSpecies::RubyTanager,
                BirdSpecies::SnowyOwl,
            ];
            let species = species_pool[self.rng.range_i32(0, species_pool.len() as i32) as usize];
            self.next_id += 1;
            self.birds.push(Bird {
                id: self.next_id,
                species,
                pos: [start_x, start_y],
                vel: [if from_left { 65.0 } else { -65.0 }, 0.0],
                facing_left: !from_left,
                state: BirdState::Flying {
                    target: [
                        vp_x + self.rng.range(300.0, 700.0),
                        self.rng.range(150.0, 400.0),
                    ],
                    speed: self.rng.range(45.0, 80.0),
                },
                sing_timer: self.rng.range(5.0, 15.0),
                wing_phase: 0.0,
                size_scale: if species == BirdSpecies::SnowyOwl {
                    1.35
                } else {
                    1.0
                },
            });
        }

        // Check if snacks attract perched or flying birds
        if !self.snacks.is_empty() {
            for snack in &mut self.snacks {
                if snack.servings_left > 0 {
                    let branch_id = snack.branch_id;
                    let t = snack.t;
                    let targeted_count = self
                        .birds
                        .iter()
                        .filter(|b| match b.state {
                            BirdState::ApproachingFeed {
                                branch_id: bid,
                                t: bt,
                                ..
                            }
                            | BirdState::Feeding {
                                branch_id: bid,
                                t: bt,
                                ..
                            } => bid == branch_id && (bt - t).abs() < 0.02,
                            _ => false,
                        })
                        .count();

                    if targeted_count < snack.servings_left as usize {
                        for b in &mut self.birds {
                            if matches!(
                                b.state,
                                BirdState::Circling { .. } | BirdState::Flying { .. }
                            ) {
                                b.state = BirdState::ApproachingFeed {
                                    branch_id,
                                    t,
                                    speed: 75.0,
                                };
                                break;
                            }
                        }
                    }
                }
            }
            self.snacks.retain(|s| s.servings_left > 0);
        }
    }

    /// Particle updates (musical notes and wood chips from pruning).
    pub fn update_notes_and_chips(&mut self, dt_secs: f32) {
        // Floating notes rise, gently drift, and fade out
        for n in &mut self.notes {
            n.life += dt_secs;
            n.pos[0] += n.vel[0] * dt_secs;
            n.pos[1] += n.vel[1] * dt_secs;
        }
        self.notes.retain(|n| n.life < n.max_life);

        // Falling chips flutter downward under gravity
        for c in &mut self.chips {
            c.life += dt_secs;
            c.pos[0] += c.vel[0] * dt_secs;
            c.pos[1] += c.vel[1] * dt_secs;
            c.vel[1] += 120.0 * dt_secs; // gravity
            c.rotation += c.rot_vel * dt_secs;
        }
        self.chips
            .retain(|c| c.life < c.max_life && c.pos[1] < 900.0);
    }

    /// Scatter perched or circling birds if cursor is nearby in Direct Interaction Mode.
    pub fn scatter_birds_near(&mut self, pointer: [f32; 2], radius: f32) {
        let r2 = radius * radius;
        for b in &mut self.birds {
            let dx = b.pos[0] - pointer[0];
            let dy = b.pos[1] - pointer[1];
            let d2 = dx * dx + dy * dy;
            if d2 < r2 {
                let dist = d2.sqrt().max(1.0);
                let dir = [dx / dist, (dy / dist) - 0.4]; // boost upwards
                b.state = BirdState::Startled {
                    remaining_secs: 1.5,
                    dir,
                };
            }
        }
    }

    /// User micro-interaction: drop a snack on nearest branch to attract birds.
    pub fn drop_snack(&mut self, pos: [f32; 2]) -> bool {
        self.drop_snack_typed(pos, SnackType::Seeds)
    }

    /// User micro-interaction: drop a specific snack type on nearest branch.
    pub fn drop_snack_typed(&mut self, pos: [f32; 2], snack_type: SnackType) -> bool {
        let mut best_id = None;
        let mut min_dist = 85.0;

        for b in &self.branches {
            let mid = b.point_at(0.5);
            let dist = pointer_dist(mid, pos);
            if dist < min_dist {
                min_dist = dist;
                best_id = Some(b.id);
            }
        }

        if let Some(id) = best_id {
            self.snacks.push(Snack {
                branch_id: id,
                t: 0.5,
                servings_left: 3,
                snack_type,
            });

            // Special berries or nectar attract visiting or rare birds from off-screen margins
            if matches!(snack_type, SnackType::SweetBerries | SnackType::StarNectar)
                || self.rng.f32() < 0.35
            {
                let from_left = self.rng.f32() < 0.5;
                let start_x = if from_left { -70.0 } else { 1070.0 };
                let start_y = self.rng.range(100.0, 320.0);
                let species = match snack_type {
                    SnackType::SweetBerries => BirdSpecies::RubyTanager,
                    SnackType::StarNectar => BirdSpecies::CelestialRaven,
                    SnackType::Seeds => {
                        if self.rng.f32() < 0.4 {
                            BirdSpecies::SnowyOwl
                        } else {
                            BirdSpecies::Goldfinch
                        }
                    }
                };
                self.next_id += 1;
                self.birds.push(Bird {
                    id: self.next_id,
                    species,
                    pos: [start_x, start_y],
                    vel: [if from_left { 70.0 } else { -70.0 }, 0.0],
                    facing_left: !from_left,
                    state: BirdState::Flying {
                        target: pos,
                        speed: 75.0,
                    },
                    sing_timer: self.rng.range(3.0, 8.0),
                    wing_phase: 0.0,
                    size_scale: if species == BirdSpecies::SnowyOwl {
                        1.35
                    } else {
                        1.05
                    },
                });
            }

            true
        } else {
            false
        }
    }
}
