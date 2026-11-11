//! Procedural tree growth, species profiles, and forest succession simulation.

use super::*;

impl WorldState {
    /// Plant a brand-new sapling of a specific species at `base_x`.
    pub fn plant_new_sapling(&mut self, base_x: f32, species: TreeSpecies) -> u32 {
        self.next_id += 1;
        let root_id = self.next_id;
        let trunk_base = [base_x, 750.0];
        let sprout_len = 18.0;
        let trunk = Branch {
            id: root_id,
            parent_id: None,
            depth: 0,
            start: trunk_base,
            end: [base_x, 750.0 - sprout_len],
            angle: 0.0,
            length: sprout_len,
            target_length: sprout_len,
            thickness: 4.0,
            growth_progress: 1.0,
            children_spawned: false,
            has_nest: false,
            sway_factor: 0.04,
            is_root: false,
            foliage: vec![FoliageBud {
                offset_along: 1.0,
                lateral_offset: 0.0,
                char_code: b'*',
                color_idx: 0,
                scale: 1.0,
                bloom: 0.85,
            }],
        };
        self.branches.push(trunk);

        self.next_id += 1;
        let tree_id = self.next_id;
        let tree_seed = self
            .seed
            .wrapping_add((self.trees.len() as u64 + 1) * 31337);
        let record = TreeRecord {
            id: tree_id,
            species,
            base_pos: trunk_base,
            root_branch_id: root_id,
            is_mature: false,
            growth_steps_completed: 0,
            target_growth_steps: species.max_growth_steps(),
            planted_age_secs: self.total_age_secs,
            seed: tree_seed,
        };
        self.trees.push(record);
        self.active_tree_id = tree_id;
        self.recompute_stats();
        tree_id
    }

    /// Collect all branch IDs descended from `root_id` (including `root_id`).
    pub fn tree_branch_ids(&self, root_id: u32) -> Vec<u32> {
        let mut tree_branch_ids = vec![root_id];
        let mut idx = 0;
        while idx < tree_branch_ids.len() {
            let pid = tree_branch_ids[idx];
            for b in &self.branches {
                if b.parent_id == Some(pid) && !tree_branch_ids.contains(&b.id) {
                    tree_branch_ids.push(b.id);
                }
            }
            idx += 1;
        }
        tree_branch_ids
    }

    /// Discrete growth engine: fires every 5-10 seconds to visibly grow a new element on the active tree.
    pub fn execute_growth_step(&mut self) {
        // Find active tree that is not yet mature
        let active_idx = self
            .trees
            .iter()
            .position(|t| t.id == self.active_tree_id && !t.is_mature)
            .or_else(|| self.trees.iter().position(|t| !t.is_mature));

        let active_idx = match active_idx {
            Some(idx) => idx,
            None => {
                // If all trees are mature, check if we need to pan and plant the next tree
                if let Some(last) = self.trees.last() {
                    let next_x = last.base_pos[0] + FOREST_TREE_SPACING;
                    let next_species = last.species.next();
                    self.target_viewport_offset_x = next_x - 500.0;
                    self.plant_new_sapling(next_x, next_species);
                }
                return;
            }
        };

        self.execute_growth_step_for(active_idx, true);
    }

    /// Execute a single growth step for a specific tree index.
    pub fn execute_growth_step_for(&mut self, active_idx: usize, auto_succession: bool) {
        if active_idx >= self.trees.len() {
            return;
        }
        let tree = &mut self.trees[active_idx];
        tree.growth_steps_completed += 1;
        let step = tree.growth_steps_completed;
        let species = tree.species;
        let root_id = tree.root_branch_id;
        let base_x = tree.base_pos[0];
        let max_steps = tree.target_growth_steps;
        let is_now_mature = step >= max_steps;

        // Collect all branches currently belonging to this tree
        let tree_branch_ids = self.tree_branch_ids(root_id);

        // 1. Stage 1 (Steps 1..18): Progressive Trunk Ascent & Buttress Roots
        // Heights scaled up significantly to command ~5/6ths of screen height (ground at y=750.0)
        let target_trunk_len = match species {
            TreeSpecies::AlpinePine => 600.0,
            TreeSpecies::SilverBirch => 540.0,
            TreeSpecies::AncientOak => 330.0,
            TreeSpecies::GreatBaobab => 340.0,
            TreeSpecies::WeepingWillow => 290.0,
            TreeSpecies::JapaneseMaple => 260.0,
        };
        let target_thickness = match species {
            TreeSpecies::GreatBaobab => 54.0,
            TreeSpecies::AncientOak => 36.0,
            TreeSpecies::AlpinePine => 22.0,
            TreeSpecies::WeepingWillow => 24.0,
            TreeSpecies::JapaneseMaple => 22.0,
            TreeSpecies::SilverBirch => 18.0,
        };

        if step <= 18 {
            let frac = (step as f32 / 18.0).clamp(0.0, 1.0);
            let target_step_len = 18.0 + (target_trunk_len - 18.0) * frac;
            let target_step_thick = 4.0 + (target_thickness - 4.0) * frac;

            if let Some(trunk) = self.branches.iter_mut().find(|b| b.id == root_id) {
                trunk.target_length = target_step_len;
                if !auto_succession || is_now_mature {
                    trunk.length = target_step_len;
                    trunk.thickness = target_step_thick;
                    trunk.end = trunk.current_end();
                }
            }

            // At step 3 & 5: flare buttress roots at the ground
            if step == 3 || step == 5 || step == 7 {
                let flare_dir = if step.is_multiple_of(2) { 1.0 } else { -1.0 };
                let root_angle = flare_dir * self.rng.range(1.15, 1.45);
                let root_len = match species {
                    TreeSpecies::GreatBaobab => self.rng.range(38.0, 68.0),
                    TreeSpecies::AncientOak => self.rng.range(32.0, 58.0),
                    _ => self.rng.range(22.0, 42.0),
                };
                let root_thick = match species {
                    TreeSpecies::GreatBaobab => 16.0,
                    TreeSpecies::AncientOak => 12.0,
                    _ => 8.0,
                };
                self.next_id += 1;
                let init_progress = if !auto_succession || is_now_mature {
                    1.0
                } else {
                    0.05
                };
                self.branches.push(Branch {
                    id: self.next_id,
                    parent_id: Some(root_id),
                    depth: 1,
                    start: [base_x, 750.0],
                    end: [
                        base_x + root_angle.sin() * root_len,
                        750.0 - root_angle.cos() * root_len,
                    ],
                    angle: root_angle,
                    length: root_len,
                    target_length: root_len,
                    thickness: root_thick,
                    growth_progress: init_progress,
                    children_spawned: true,
                    has_nest: false,
                    sway_factor: 0.0,
                    is_root: true,
                    foliage: Vec::new(),
                });
            }
        }

        // 2. Stage 2 (Steps 6..30): Primary Scaffold Limbs & Species Branching
        if (6..=30).contains(&step) {
            let mut candidates: Vec<u32> = tree_branch_ids
                .iter()
                .copied()
                .filter(|&bid| {
                    if let Some(b) = self.branches.iter().find(|b| b.id == bid) {
                        !b.is_root && b.depth < 4
                    } else {
                        false
                    }
                })
                .collect();

            if candidates.is_empty() {
                candidates.push(root_id);
            }

            let is_pine = species == TreeSpecies::AlpinePine;
            let primary_pine_whorls = self
                .branches
                .iter()
                .filter(|b| b.parent_id == Some(root_id) && !b.is_root)
                .count();

            let parent_id = if is_pine && primary_pine_whorls < 12 && self.rng.f32() < 0.70 {
                root_id
            } else {
                candidates[self.rng.range_i32(0, candidates.len() as i32) as usize]
            };

            let parent_data = self
                .branches
                .iter()
                .find(|b| b.id == parent_id)
                .map(|b| (b.start, b.end, b.angle, b.depth, b.thickness, b.length));

            if let Some((_p_start, p_end, p_angle, p_depth, p_thick, p_len)) = parent_data {
                let depth = p_depth + 1;
                let count = if depth == 1 {
                    match species {
                        TreeSpecies::AlpinePine => 2, // symmetrical horizontal whorl
                        TreeSpecies::GreatBaobab => self.rng.range_i32(2, 4),
                        _ => self.rng.range_i32(1, 3),
                    }
                } else {
                    1
                };

                let attach_pos = if parent_id == root_id {
                    let t = match species {
                        TreeSpecies::AlpinePine => {
                            let tier_idx = (primary_pine_whorls / 2) % 7;
                            0.20 + (tier_idx as f32) * 0.11
                        }
                        TreeSpecies::AncientOak => self.rng.range(0.40, 0.95),
                        TreeSpecies::GreatBaobab => self.rng.range(0.72, 0.98),
                        TreeSpecies::SilverBirch => self.rng.range(0.50, 0.95),
                        _ => self.rng.range(0.40, 0.95),
                    };
                    if let Some(trunk) = self.branches.iter().find(|b| b.id == root_id) {
                        trunk.point_at(t)
                    } else {
                        p_end
                    }
                } else {
                    p_end
                };

                if let Some(parent) = self.branches.iter_mut().find(|b| b.id == parent_id) {
                    parent.children_spawned = true;
                }

                for k in 0..count {
                    self.next_id += 1;
                    let id = self.next_id;

                    let branch_sign = if count == 1 {
                        if self.rng.f32() > 0.5 { 1.0 } else { -1.0 }
                    } else if k % 2 == 0 {
                        -1.0
                    } else {
                        1.0
                    };

                    let (angle, target_len) = match species {
                        TreeSpecies::AlpinePine => {
                            if depth == 1 {
                                let a = branch_sign * self.rng.range(1.22, 1.45);
                                let dist_from_trunk_base = (750.0 - attach_pos[1]).max(10.0);
                                let h_factor = 1.0
                                    - (dist_from_trunk_base / target_trunk_len).clamp(0.0, 0.78);
                                let l = (210.0 * h_factor).clamp(55.0, 210.0);
                                (a, l)
                            } else {
                                let a = (p_angle + branch_sign * self.rng.range(0.18, 0.40))
                                    .clamp(-1.50, 1.50);
                                let l = (p_len * 0.60).clamp(32.0, 85.0);
                                (a, l)
                            }
                        }
                        TreeSpecies::WeepingWillow => {
                            if depth <= 2 {
                                let a = if parent_id == root_id {
                                    branch_sign * self.rng.range(0.30, 0.85)
                                } else {
                                    (p_angle + branch_sign * self.rng.range(0.15, 0.45))
                                        .clamp(-1.10, 1.10)
                                };
                                let l = if depth == 1 {
                                    self.rng.range(160.0, 240.0)
                                } else {
                                    self.rng.range(100.0, 160.0)
                                };
                                (a, l)
                            } else {
                                // Downward weeping cascade
                                let a = branch_sign * self.rng.range(2.1, 2.8);
                                let raw_l = self.rng.range(110.0, 180.0);
                                // Ensure cascading branches never pierce the ground
                                let max_down_len = (715.0 - attach_pos[1]).max(20.0);
                                (a, raw_l.min(max_down_len))
                            }
                        }
                        TreeSpecies::JapaneseMaple => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.42, 0.95)
                            } else {
                                let zig = if depth % 2 == 0 { 0.55 } else { -0.55 };
                                (p_angle + zig * branch_sign + self.rng.range(-0.15, 0.15))
                                    .clamp(-1.3, 1.3)
                            };
                            let l = if depth == 1 {
                                self.rng.range(150.0, 230.0)
                            } else if depth == 2 {
                                self.rng.range(100.0, 160.0)
                            } else {
                                self.rng.range(65.0, 110.0)
                            };
                            (a, l)
                        }
                        TreeSpecies::SilverBirch => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.22, 0.60)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.15, 0.35))
                                    .clamp(-0.85, 0.85)
                            };
                            let l = if depth == 1 {
                                self.rng.range(130.0, 200.0)
                            } else {
                                self.rng.range(75.0, 130.0)
                            };
                            (a, l)
                        }
                        TreeSpecies::GreatBaobab => {
                            let a = if parent_id == root_id {
                                let spread = [-1.15, -0.65, 0.65, 1.15];
                                let base_a = spread[(k as usize).min(spread.len() - 1)];
                                base_a + self.rng.range(-0.18, 0.18)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.15, 0.40))
                                    .clamp(-1.35, 1.35)
                            };
                            let l = if depth == 1 {
                                self.rng.range(150.0, 240.0)
                            } else {
                                self.rng.range(80.0, 140.0)
                            };
                            (a, l)
                        }
                        TreeSpecies::AncientOak => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.35, 0.90)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.20, 0.55))
                                    .clamp(-1.25, 1.25)
                            };
                            let l = if depth == 1 {
                                self.rng.range(170.0, 260.0)
                            } else if depth == 2 {
                                self.rng.range(110.0, 180.0)
                            } else {
                                self.rng.range(70.0, 120.0)
                            };
                            (a, l)
                        }
                    };

                    let thickness = (p_thick * 0.72).max(2.5);

                    // Generate species-specific foliage buds
                    let mut foliage = Vec::new();
                    let leaf_count = match depth {
                        1 => self.rng.range_i32(4, 8) as usize,
                        2 => self.rng.range_i32(6, 12) as usize,
                        _ => self.rng.range_i32(8, 15) as usize,
                    };
                    let chars: &[u8] = match species {
                        TreeSpecies::AlpinePine => b"^*+^#",
                        TreeSpecies::WeepingWillow => b"|~;:,",
                        TreeSpecies::JapaneseMaple => b"*@&#",
                        TreeSpecies::SilverBirch => b"\"'`*",
                        _ => b"*%&@o#",
                    };

                    for _ in 0..leaf_count {
                        let char_code = chars[self.rng.range_i32(0, chars.len() as i32) as usize];
                        let color_idx = match species {
                            TreeSpecies::JapaneseMaple => self.rng.range_i32(2, 4) as u8,
                            TreeSpecies::SilverBirch => {
                                if self.rng.f32() > 0.5 {
                                    1
                                } else {
                                    2
                                }
                            }
                            _ => self.rng.range_i32(0, 3) as u8,
                        };
                        let init_bloom = if !auto_succession || is_now_mature {
                            0.85
                        } else {
                            0.0
                        };
                        foliage.push(FoliageBud {
                            offset_along: self.rng.range(0.25, 1.0),
                            lateral_offset: self.rng.range(-16.0, 16.0),
                            char_code,
                            color_idx,
                            scale: self.rng.range(0.9, 1.3),
                            bloom: init_bloom,
                        });
                    }

                    // Advance previous branches of this tree so discrete step calls progress
                    for bid in &tree_branch_ids {
                        if let Some(b) = self.branches.iter_mut().find(|b| b.id == *bid)
                            && !b.is_root
                            && b.id != root_id
                        {
                            b.growth_progress = (b.growth_progress + 0.35).min(1.0);
                        }
                    }

                    let init_progress = if !auto_succession || is_now_mature {
                        1.0
                    } else {
                        0.02
                    };

                    self.branches.push(Branch {
                        id,
                        parent_id: Some(parent_id),
                        depth,
                        start: attach_pos,
                        end: [
                            attach_pos[0] + angle.sin() * target_len,
                            attach_pos[1] - angle.cos() * target_len,
                        ],
                        angle,
                        length: target_len,
                        target_length: target_len,
                        thickness,
                        growth_progress: init_progress,
                        children_spawned: false,
                        has_nest: false,
                        sway_factor: (depth as f32 * 0.12).min(0.65),
                        is_root: false,
                        foliage,
                    });
                }
            }
        }
        // 3. Stage 3 (Steps 31..max_steps): Foliage Bloom, Densification & Maturation
        if step > 30 {
            for bid in &tree_branch_ids {
                if let Some(b) = self.branches.iter_mut().find(|b| b.id == *bid) {
                    if !auto_succession || is_now_mature {
                        for bud in &mut b.foliage {
                            bud.bloom = 1.0;
                        }
                    }
                    if !b.is_root && b.depth >= 2 && b.foliage.len() < 16 {
                        let init_bloom = if !auto_succession || is_now_mature {
                            1.0
                        } else {
                            0.15
                        };
                        b.foliage.push(FoliageBud {
                            offset_along: self.rng.range(0.35, 0.98),
                            lateral_offset: self.rng.range(-15.0, 15.0),
                            char_code: b'*',
                            color_idx: 0,
                            scale: 1.1,
                            bloom: init_bloom,
                        });
                    }
                }
            }
        }

        // Spawn a subtle wood chip spark from the active growth
        self.chips.push(FallingChip {
            pos: [
                base_x + self.rng.range(-60.0, 60.0),
                750.0 - (step as f32 * 18.0).min(640.0),
            ],
            vel: [self.rng.range(-18.0, 18.0), self.rng.range(-22.0, -8.0)],
            rotation: 0.0,
            rot_vel: self.rng.range(-2.0, 2.0),
            glyph: '+',
            life: 0.0,
            max_life: 1.0,
        });

        // Check if tree has reached full maturity
        if is_now_mature {
            self.trees[active_idx].is_mature = true;
            let forest_num = self.trees.len();

            // Commemorative plant sign marker
            let sign_id = self.next_id + 1;
            self.next_id += 1;
            self.signs.push(PlantSign {
                id: sign_id,
                tree_root_id: root_id,
                pos: [base_x + 36.0, 752.0],
                planted_age_secs: self.total_age_secs,
                label: format!("Grand {} · Forest #{}", species.display_name(), forest_num),
                lineage_gen: forest_num as u32,
            });

            if auto_succession {
                // Forest Succession: smoothly pan camera left and plant next tree in center!
                let next_species = species.next();
                let next_x = base_x + FOREST_TREE_SPACING;
                self.target_viewport_offset_x = next_x - 500.0;
                self.plant_new_sapling(next_x, next_species);
                self.next_id += 1;
                self.bird_baths.push(BirdBath {
                    id: self.next_id,
                    pos: [next_x - 160.0, 754.0],
                    water_level: 1.0,
                    splash_timer: 0.0,
                });
            }
        }

        self.recompute_stats();
    }

    /// Grow a specific tree to full maturity immediately.
    pub fn mature_tree_by_id(&mut self, tree_id: u32, auto_succession: bool) -> bool {
        let tree_idx = match self.trees.iter().position(|t| t.id == tree_id) {
            Some(idx) => idx,
            None => return false,
        };

        let target_steps = self.trees[tree_idx].target_growth_steps;
        while self.trees[tree_idx].growth_steps_completed < target_steps
            && !self.trees[tree_idx].is_mature
        {
            self.execute_growth_step_for(tree_idx, auto_succession);
        }

        // Set all branch growth_progress and foliage bloom to 1.0 for visual completion
        let root_id = self.trees[tree_idx].root_branch_id;
        let branch_ids = self.tree_branch_ids(root_id);
        for b in &mut self.branches {
            if branch_ids.contains(&b.id) {
                b.growth_progress = 1.0;
                b.end = b.current_end();
                for f in &mut b.foliage {
                    f.bloom = 1.0;
                }
            }
        }

        self.recompute_stats();
        true
    }

    /// Plant and immediately mature a tree of a specific species with a deterministic seed.
    pub fn plant_and_mature_tree(&mut self, base_x: f32, species: TreeSpecies, seed: u64) -> u32 {
        let saved_rng = self.rng;
        self.rng = WorldRng::new(seed);

        let tree_id = self.plant_new_sapling(base_x, species);
        if let Some(t) = self.trees.iter_mut().find(|t| t.id == tree_id) {
            t.seed = seed;
        }

        self.mature_tree_by_id(tree_id, false);

        self.rng = saved_rng;
        tree_id
    }

    /// Debug mode: Clears the forest and spawns a single centered mature specimen of `species` with `seed`.
    pub fn view_mature_tree(&mut self, species: TreeSpecies, seed: u64) -> u32 {
        self.branches.clear();
        self.trees.clear();
        self.signs.clear();
        self.houses.clear();
        self.lanterns.clear();
        self.nests.clear();
        self.chips.clear();
        self.notes.clear();
        self.next_id = 0;

        let base_x = 500.0;
        let tid = self.plant_and_mature_tree(base_x, species, seed);
        self.active_tree_id = tid;
        self.target_viewport_offset_x = 0.0;
        self.viewport_offset_x = 0.0;
        self.recompute_stats();
        tid
    }

    /// Debug mode: Generate all 6 species fully mature in a grand panorama showcase.
    pub fn generate_mature_showcase(&mut self, base_seed: u64) {
        self.branches.clear();
        self.trees.clear();
        self.signs.clear();
        self.houses.clear();
        self.lanterns.clear();
        self.nests.clear();
        self.chips.clear();
        self.notes.clear();
        self.next_id = 0;

        let all_species = TreeSpecies::all();
        let spacing = FOREST_TREE_SPACING;
        let mut first_id = 0;

        for (i, &species) in all_species.iter().enumerate() {
            let base_x = 500.0 + (i as f32) * spacing;
            let tree_seed = base_seed.wrapping_add((i as u64) * 1000 + 42);
            let tid = self.plant_and_mature_tree(base_x, species, tree_seed);
            if i == 0 {
                first_id = tid;
            }
        }

        self.active_tree_id = first_id;
        self.target_viewport_offset_x = 0.0;
        self.viewport_offset_x = 0.0;
        self.recompute_stats();
    }

    /// Debug mode: Generate 4 mature variations of `species` side-by-side to compare procedural variety.
    pub fn generate_species_variety_grove(&mut self, species: TreeSpecies, base_seed: u64) {
        self.branches.clear();
        self.trees.clear();
        self.signs.clear();
        self.houses.clear();
        self.lanterns.clear();
        self.nests.clear();
        self.chips.clear();
        self.notes.clear();
        self.next_id = 0;

        let spacing = FOREST_TREE_SPACING;
        let mut first_id = 0;

        for i in 0..4 {
            let base_x = 500.0 + (i as f32) * spacing;
            let tree_seed = base_seed.wrapping_add((i as u64) * 31337 + 7);
            let tid = self.plant_and_mature_tree(base_x, species, tree_seed);
            if i == 0 {
                first_id = tid;
            }
        }

        self.active_tree_id = first_id;
        self.target_viewport_offset_x = 0.0;
        self.viewport_offset_x = 0.0;
        self.recompute_stats();
    }

    /// Debug mode: Rewind a tree back to its initial sapling stage.
    pub fn rewind_tree_to_sapling(&mut self, tree_id: u32) -> bool {
        let tree_idx = match self.trees.iter().position(|t| t.id == tree_id) {
            Some(idx) => idx,
            None => return false,
        };

        let root_id = self.trees[tree_idx].root_branch_id;
        let branch_ids = self.tree_branch_ids(root_id);

        // Remove all non-root branches
        self.branches
            .retain(|b| b.id == root_id || !branch_ids.contains(&b.id));

        // Remove any signs, houses, lanterns, nests on this tree's branches
        self.signs.retain(|s| s.tree_root_id != root_id);
        self.houses
            .retain(|h| h.branch_id == root_id || !branch_ids.contains(&h.branch_id));
        self.lanterns
            .retain(|l| l.branch_id == root_id || !branch_ids.contains(&l.branch_id));
        self.nests
            .retain(|n| n.branch_id == root_id || !branch_ids.contains(&n.branch_id));

        // Reset root trunk to sapling state
        let base_pos = self.trees[tree_idx].base_pos;
        if let Some(trunk) = self.branches.iter_mut().find(|b| b.id == root_id) {
            let sprout_len = 18.0;
            trunk.start = base_pos;
            trunk.end = [base_pos[0], base_pos[1] - sprout_len];
            trunk.angle = 0.0;
            trunk.length = sprout_len;
            trunk.target_length = sprout_len;
            trunk.thickness = 4.0;
            trunk.growth_progress = 1.0;
            trunk.children_spawned = false;
            trunk.has_nest = false;
            trunk.foliage = vec![FoliageBud {
                offset_along: 1.0,
                lateral_offset: 0.0,
                char_code: b'*',
                color_idx: 0,
                scale: 1.0,
                bloom: 0.85,
            }];
        }

        let tree = &mut self.trees[tree_idx];
        tree.growth_steps_completed = 0;
        tree.is_mature = false;

        self.recompute_stats();
        true
    }

    /// Focus viewport camera smoothly on a specific tree.
    pub fn focus_tree(&mut self, tree_id: u32) -> bool {
        if let Some(tree) = self.trees.iter().find(|t| t.id == tree_id) {
            self.target_viewport_offset_x = tree.base_pos[0] - 500.0;
            self.viewport_offset_x = self.target_viewport_offset_x;
            self.active_tree_id = tree.id;
            true
        } else {
            false
        }
    }

    /// Summary snapshot list of all trees for UI and telemetry.
    pub fn tree_summary_list(&self) -> Vec<TreeSummary> {
        self.trees
            .iter()
            .map(|t| {
                let branch_ids = self.tree_branch_ids(t.root_branch_id);
                let mut branch_count = 0;
                let mut foliage_count = 0;
                for b in &self.branches {
                    if branch_ids.contains(&b.id) {
                        branch_count += 1;
                        foliage_count += b.foliage.len() as u32;
                    }
                }
                TreeSummary {
                    id: t.id,
                    species: t.species.display_name().to_string(),
                    species_code: t.species.code().to_string(),
                    is_mature: t.is_mature,
                    growth_steps: t.growth_steps_completed,
                    target_steps: t.target_growth_steps,
                    base_x: t.base_pos[0],
                    seed: t.seed,
                    branch_count,
                    foliage_count,
                }
            })
            .collect()
    }

    /// Advance tree growth by `dt_secs`.
    pub fn advance_growth(&mut self, dt_secs: f32) {
        if dt_secs <= 0.0 || self.sim_speed <= 0.0 {
            return;
        }

        let effective_dt = dt_secs * self.sim_speed.clamp(0.0, 100.0);

        // 1. Smooth horizontal viewport camera panning
        let offset_diff = self.target_viewport_offset_x - self.viewport_offset_x;
        if offset_diff.abs() > 0.1 {
            let step = offset_diff * (effective_dt * 2.5).min(0.25);
            self.viewport_offset_x += step;
        } else {
            self.viewport_offset_x = self.target_viewport_offset_x;
        }

        // 2. Discrete 5-10s growth step timer
        self.growth_step_timer -= effective_dt;
        let max_loops = ((effective_dt / 6.0).ceil() as usize).clamp(1, 60);
        let mut loop_guard = 0;
        while self.growth_step_timer <= 0.0 && loop_guard < max_loops {
            self.growth_step_timer += self.rng.range(5.0, 8.0);
            self.execute_growth_step();
            loop_guard += 1;
        }
        if self.growth_step_timer <= 0.0 {
            self.growth_step_timer = self.rng.range(5.0, 8.0);
        }

        // 3. Fine-Grained Cell-by-Cell Procedural Growth Simulation
        // A. Trunk upward ascent & radial thickening
        for tree in &self.trees {
            if tree.is_mature {
                continue;
            }
            let root_id = tree.root_branch_id;
            let target_thickness = match tree.species {
                TreeSpecies::GreatBaobab => 54.0,
                TreeSpecies::AncientOak => 36.0,
                TreeSpecies::AlpinePine => 22.0,
                TreeSpecies::WeepingWillow => 24.0,
                TreeSpecies::JapaneseMaple => 22.0,
                TreeSpecies::SilverBirch => 18.0,
            };
            let step = tree.growth_steps_completed;
            let target_thick =
                4.0 + (target_thickness - 4.0) * (step as f32 / 18.0).clamp(0.0, 1.0);

            if let Some(trunk) = self.branches.iter_mut().find(|b| b.id == root_id) {
                // Smooth trunk height extension
                if trunk.length < trunk.target_length {
                    let len_diff = trunk.target_length - trunk.length;
                    let grow_step = (len_diff * 0.45).max(4.0) * effective_dt;
                    trunk.length = (trunk.length + grow_step).min(trunk.target_length);
                    trunk.end = trunk.current_end();
                }
                // Smooth trunk radial thickening
                if trunk.thickness < target_thick {
                    let thick_diff = target_thick - trunk.thickness;
                    let thick_step = (thick_diff * 0.35).max(0.6) * effective_dt;
                    trunk.thickness = (trunk.thickness + thick_step).min(target_thick);
                }
            }
        }

        // B. Continuous glyph-by-glyph branch extension & bud swelling
        let branch_grow_rate = 0.22 * effective_dt;
        let bud_swell_rate = 0.16 * effective_dt;

        for b in &mut self.branches {
            if b.growth_progress < 1.0 {
                b.growth_progress = (b.growth_progress + branch_grow_rate).min(1.0);
                b.end = b.current_end();
            }

            // Buds swell from single dot -> young star -> expanding cluster -> mature canopy
            for f in &mut b.foliage {
                if b.growth_progress >= f.offset_along && f.bloom < 1.0 {
                    f.bloom = (f.bloom + bud_swell_rate).min(1.0);
                }
            }
        }

        // 4. Ecosystem structures on mature branches over time
        if self.houses.len() < 12 && self.rng.f32() < 0.008 * (effective_dt / 10.0).clamp(0.01, 1.0)
        {
            let candidates: Vec<u32> = self
                .branches
                .iter()
                .filter(|b| b.growth_progress >= 0.95 && (b.depth == 1 || b.depth == 2))
                .map(|b| b.id)
                .collect();
            if !candidates.is_empty() {
                let branch_id = candidates[self.rng.range_i32(0, candidates.len() as i32) as usize];
                let t = self.rng.range(0.35, 0.75);
                let house_style = self.rng.range_i32(0, 3) as u8;
                self.next_id += 1;
                self.houses.push(BirdHouse {
                    id: self.next_id,
                    branch_id,
                    t,
                    house_style,
                    built_age_secs: self.total_age_secs,
                });
                self.stats.houses_built += 1;
            }
        }

        // Hanging lanterns along mature primary branches
        if self.lanterns.len() < 4 && self.rng.f32() < 0.006 * (dt_secs / 10.0).clamp(0.01, 1.0) {
            let candidates: Vec<u32> = self
                .branches
                .iter()
                .filter(|b| b.growth_progress >= 0.9 && b.depth == 1)
                .map(|b| b.id)
                .collect();
            if !candidates.is_empty() {
                let branch_id = candidates[self.rng.range_i32(0, candidates.len() as i32) as usize];
                self.next_id += 1;
                self.lanterns.push(Lantern {
                    id: self.next_id,
                    branch_id,
                    t: self.rng.range(0.4, 0.8),
                    light_intensity: 1.0,
                });
            }
        }

        // Nest establishment: start new twig-built nests
        if self.nests.len() < 6 && self.rng.f32() < 0.012 * (dt_secs / 10.0).clamp(0.01, 1.0) {
            let candidates: Vec<u32> = self
                .branches
                .iter()
                .filter(|b| {
                    b.growth_progress >= 0.85 && b.depth >= 1 && b.depth <= 3 && !b.has_nest
                })
                .map(|b| b.id)
                .collect();
            if !candidates.is_empty() {
                let branch_id = candidates[self.rng.range_i32(0, candidates.len() as i32) as usize];
                self.next_id += 1;
                self.nests.push(Nest {
                    id: self.next_id,
                    branch_id,
                    t: self.rng.range(0.35, 0.65),
                    progress: 0.1,
                    eggs_count: 0,
                    chicks_count: 0,
                    age_secs: self.total_age_secs,
                });
            }
        }

        // Egg laying in mature nests
        for nest in &mut self.nests {
            nest.age_secs += dt_secs as f64;
            if nest.progress >= 1.0
                && nest.eggs_count == 0
                && nest.chicks_count == 0
                && self.rng.f32() < 0.005 * dt_secs
            {
                let eggs = self.rng.range_i32(1, 4) as u8;
                nest.eggs_count = eggs;
                self.stats.eggs_laid += eggs as u32;
            }
        }
    }

    /// User micro-interaction: Prune nearest branch to pointer within `radius`.
    /// Generates fluttering falling wood chips and redirects growth energy.
    pub fn prune_nearest(&mut self, pointer: [f32; 2], max_radius: f32) -> Option<u32> {
        let mut best_id = None;
        let mut min_dist = max_radius;

        for b in &self.branches {
            // Cannot prune trunk base (depth 0)
            if b.depth == 0 {
                continue;
            }
            let mid = b.point_at(0.5);
            let dx = mid[0] - pointer[0];
            let dy = mid[1] - pointer[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < min_dist {
                min_dist = dist;
                best_id = Some(b.id);
            }
        }

        if let Some(id) = best_id {
            self.remove_branch_and_descendants(id);
            self.stats.pruned_branches += 1;
            Some(id)
        } else {
            None
        }
    }

    /// Recursively remove branch and its descendants, spawning falling wood chips.
    pub(crate) fn remove_branch_and_descendants(&mut self, target_id: u32) {
        let mut to_remove = vec![target_id];
        let mut idx = 0;
        while idx < to_remove.len() {
            let pid = to_remove[idx];
            for b in &self.branches {
                if b.parent_id == Some(pid) && !to_remove.contains(&b.id) {
                    to_remove.push(b.id);
                }
            }
            idx += 1;
        }

        // Spawn falling wood chips for removed branches
        let chip_chars = ['/', '\\', '|', '-', '~', '%', '*'];
        for b in &self.branches {
            if to_remove.contains(&b.id) {
                let p = b.point_at(0.5);
                for _ in 0..4 {
                    let glyph = chip_chars[self.rng.range_i32(0, chip_chars.len() as i32) as usize];
                    self.chips.push(FallingChip {
                        pos: [
                            p[0] + self.rng.range(-8.0, 8.0),
                            p[1] + self.rng.range(-8.0, 8.0),
                        ],
                        vel: [self.rng.range(-40.0, 40.0), self.rng.range(-50.0, -10.0)],
                        rotation: self.rng.range(0.0, std::f32::consts::TAU),
                        rot_vel: self.rng.range(-6.0, 6.0),
                        glyph,
                        life: 0.0,
                        max_life: self.rng.range(1.5, 3.0),
                    });
                }
            }
        }

        self.branches.retain(|b| !to_remove.contains(&b.id));
        self.nests.retain(|n| !to_remove.contains(&n.branch_id));
        self.houses.retain(|h| !to_remove.contains(&h.branch_id));
        self.lanterns.retain(|l| !to_remove.contains(&l.branch_id));
        self.snacks.retain(|s| !to_remove.contains(&s.branch_id));

        // Reassign any birds active on deleted branches
        for bird in &mut self.birds {
            let needs_flight = match &bird.state {
                BirdState::Perched { branch_id, .. }
                | BirdState::Feeding { branch_id, .. }
                | BirdState::GatheringTwig { branch_id, .. }
                | BirdState::ApproachingPerch { branch_id, .. }
                | BirdState::ApproachingFeed { branch_id, .. } => to_remove.contains(branch_id),
                _ => false,
            };
            if needs_flight {
                let tx = self.rng.range(200.0, 800.0);
                let ty = self.rng.range(100.0, 300.0);
                bird.state = BirdState::Flying {
                    target: [tx, ty],
                    speed: 70.0,
                };
            }
        }
    }

    /// User micro-interaction: plant a seed at the ground to sprout a new shoot or sapling.
    pub fn plant_seed(&mut self, pos: [f32; 2]) {
        self.next_id += 1;
        let id = self.next_id;
        let len = self.rng.range(45.0, 75.0);
        let branch = Branch {
            id,
            parent_id: None,
            depth: 1,
            start: pos,
            end: [pos[0], pos[1] - len],
            angle: self.rng.range(-0.15, 0.15),
            length: len,
            target_length: len,
            thickness: 8.0,
            growth_progress: 0.1,
            children_spawned: false,
            has_nest: false,
            sway_factor: 0.1,
            is_root: false,
            foliage: Vec::new(),
        };
        self.branches.push(branch);
        self.stats.seeds_planted += 1;

        // Erect a commemorative plant sign marker with timestamp and lineage
        let sign_id = self.next_id;
        self.next_id += 1;
        self.signs.push(PlantSign {
            id: sign_id,
            tree_root_id: id,
            pos: [pos[0] + 28.0, pos[1] + 2.0],
            planted_age_secs: self.total_age_secs,
            label: format!("Sapling #{}", self.stats.seeds_planted),
            lineage_gen: 2,
        });
    }
}
