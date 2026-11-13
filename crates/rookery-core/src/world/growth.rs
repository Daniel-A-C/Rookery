//! Procedural tree growth, species profiles, and forest succession simulation.

use super::*;

/// Species-specific architectural proportions for procedural tree growth.
struct SpeciesProfile {
    trunk_total_len: f32,
    base_thickness: f32,
    trunk_curve_amp: f32,
    root_len: (f32, f32),
    root_thick: f32,
    has_secondary_roots: bool,
}

impl SpeciesProfile {
    fn for_species(species: TreeSpecies) -> Self {
        match species {
            TreeSpecies::GiantRedwood => Self {
                trunk_total_len: 630.0,
                base_thickness: 54.0,
                trunk_curve_amp: 0.03,
                root_len: (38.0, 56.0),
                root_thick: 16.0,
                has_secondary_roots: true,
            },
            TreeSpecies::AlpinePine => Self {
                trunk_total_len: 580.0,
                base_thickness: 28.0,
                trunk_curve_amp: 0.04,
                root_len: (24.0, 38.0),
                root_thick: 10.0,
                has_secondary_roots: false,
            },
            TreeSpecies::SilverBirch => Self {
                trunk_total_len: 490.0,
                base_thickness: 20.0,
                trunk_curve_amp: 0.16,
                root_len: (20.0, 32.0),
                root_thick: 8.0,
                has_secondary_roots: false,
            },
            TreeSpecies::AncientOak => Self {
                trunk_total_len: 330.0,
                base_thickness: 42.0,
                trunk_curve_amp: 0.14,
                root_len: (34.0, 52.0),
                root_thick: 14.0,
                has_secondary_roots: true,
            },
            TreeSpecies::GreatBaobab => Self {
                trunk_total_len: 340.0,
                base_thickness: 66.0,
                trunk_curve_amp: 0.07,
                root_len: (38.0, 58.0),
                root_thick: 18.0,
                has_secondary_roots: true,
            },
            TreeSpecies::AncientBanyan => Self {
                trunk_total_len: 310.0,
                base_thickness: 44.0,
                trunk_curve_amp: 0.15,
                root_len: (36.0, 54.0),
                root_thick: 14.0,
                has_secondary_roots: true,
            },
            TreeSpecies::WeepingWillow => Self {
                trunk_total_len: 290.0,
                base_thickness: 30.0,
                trunk_curve_amp: 0.14,
                root_len: (28.0, 44.0),
                root_thick: 10.0,
                has_secondary_roots: true,
            },
            TreeSpecies::CherryBlossom => Self {
                trunk_total_len: 280.0,
                base_thickness: 26.0,
                trunk_curve_amp: 0.20,
                root_len: (24.0, 38.0),
                root_thick: 9.0,
                has_secondary_roots: false,
            },
            TreeSpecies::JapaneseMaple => Self {
                trunk_total_len: 260.0,
                base_thickness: 24.0,
                trunk_curve_amp: 0.22,
                root_len: (22.0, 36.0),
                root_thick: 9.0,
                has_secondary_roots: false,
            },
        }
    }
}

impl WorldState {
    /// Keep child branches/segments anchored to a parent branch's tip when it extends.
    fn sync_trunk_segment_children(&mut self, parent_id: u32, new_tip: [f32; 2]) {
        let mut child_updates: Vec<(u32, [f32; 2])> = Vec::new();
        for b in &mut self.branches {
            if b.parent_id == Some(parent_id) && b.depth == 0 {
                b.start = new_tip;
                b.end = b.current_end();
                child_updates.push((b.id, b.end));
            }
        }
        for (cid, tip) in child_updates {
            self.sync_trunk_segment_children(cid, tip);
        }
    }

    /// Generate species-specific foliage buds for a canopy branch.
    fn generate_foliage_for_branch(
        &mut self,
        species: TreeSpecies,
        depth: u32,
        mature_now: bool,
    ) -> Vec<FoliageBud> {
        let mut foliage = Vec::new();
        let leaf_count = match depth {
            1 => self.rng.range_i32(4, 7) as usize,
            2 => self.rng.range_i32(6, 11) as usize,
            _ => self.rng.range_i32(8, 14) as usize,
        };
        let chars: &[u8] = match species {
            TreeSpecies::AlpinePine | TreeSpecies::GiantRedwood => b"^*+^#",
            TreeSpecies::WeepingWillow => b"|~;:,",
            TreeSpecies::JapaneseMaple => b"*@&#",
            TreeSpecies::CherryBlossom => b"*@o&#",
            TreeSpecies::SilverBirch => b"\"'`*",
            TreeSpecies::AncientBanyan => b"%&@o#",
            _ => b"*%&@o#",
        };

        for _ in 0..leaf_count {
            let char_code = chars[self.rng.range_i32(0, chars.len() as i32) as usize];
            let color_idx = match species {
                TreeSpecies::JapaneseMaple => self.rng.range_i32(2, 4) as u8,
                TreeSpecies::CherryBlossom => {
                    if self.rng.f32() < 0.80 {
                        3 // Blossom pink
                    } else {
                        0
                    }
                }
                TreeSpecies::SilverBirch => {
                    if self.rng.f32() > 0.5 {
                        1
                    } else {
                        2
                    }
                }
                _ => self.rng.range_i32(0, 3) as u8,
            };
            let init_bloom = if mature_now { 0.85 } else { 0.0 };
            foliage.push(FoliageBud {
                offset_along: self.rng.range(0.25, 1.0),
                lateral_offset: self.rng.range(-14.0, 14.0),
                char_code,
                color_idx,
                scale: self.rng.range(0.9, 1.3),
                bloom: init_bloom,
            });
        }
        foliage
    }

    /// Compute species-appropriate angle and compact, realistic length for a canopy branch.
    #[allow(clippy::too_many_arguments)]
    fn compute_branch_geometry(
        &mut self,
        species: TreeSpecies,
        depth: u32,
        p_angle: f32,
        p_len: f32,
        attach_pos: [f32; 2],
        trunk_total_len: f32,
        branch_sign: f32,
        slot_idx: usize,
    ) -> (f32, f32) {
        match species {
            TreeSpecies::AlpinePine => {
                if depth == 1 {
                    let a = branch_sign * self.rng.range(1.18, 1.42);
                    let dist_from_base = (750.0 - attach_pos[1]).max(10.0);
                    let h_factor = 1.0 - (dist_from_base / trunk_total_len).clamp(0.0, 0.80);
                    let l = (105.0 * h_factor).clamp(32.0, 105.0);
                    (a, l)
                } else {
                    let a = (p_angle + branch_sign * self.rng.range(0.18, 0.42)).clamp(-1.48, 1.48);
                    let l = (p_len * 0.52).clamp(20.0, 48.0);
                    (a, l)
                }
            }
            TreeSpecies::GiantRedwood => {
                if depth == 1 {
                    // Slight downward droop on massive high-altitude boughs
                    let a = branch_sign * self.rng.range(1.28, 1.62);
                    let dist_from_base = (750.0 - attach_pos[1]).max(10.0);
                    let h_factor = 1.0 - (dist_from_base / trunk_total_len).clamp(0.0, 0.72);
                    let l = (95.0 * h_factor).clamp(34.0, 95.0);
                    (a, l)
                } else {
                    let a = (p_angle + branch_sign * self.rng.range(0.15, 0.38)).clamp(-1.68, 1.68);
                    let l = (p_len * 0.55).clamp(22.0, 50.0);
                    (a, l)
                }
            }
            TreeSpecies::WeepingWillow => {
                if depth <= 2 {
                    let a = if depth == 1 {
                        branch_sign * self.rng.range(0.38, 0.88)
                    } else {
                        (p_angle + branch_sign * self.rng.range(0.18, 0.45)).clamp(-1.15, 1.15)
                    };
                    let l = if depth == 1 {
                        self.rng.range(72.0, 108.0)
                    } else {
                        self.rng.range(45.0, 74.0)
                    };
                    (a, l)
                } else {
                    // Downward weeping cascade, clamped safely above the ground
                    let a = branch_sign * self.rng.range(2.25, 2.85);
                    let raw_l = self.rng.range(65.0, 115.0);
                    let max_down_len = (715.0 - attach_pos[1]).max(20.0);
                    (a, raw_l.min(max_down_len))
                }
            }
            TreeSpecies::JapaneseMaple => {
                let a = if depth == 1 {
                    branch_sign * self.rng.range(0.45, 0.98)
                } else {
                    let zig = if depth.is_multiple_of(2) { 0.48 } else { -0.48 };
                    (p_angle + zig * branch_sign + self.rng.range(-0.15, 0.15)).clamp(-1.30, 1.30)
                };
                let l = match depth {
                    1 => self.rng.range(65.0, 100.0),
                    2 => self.rng.range(44.0, 70.0),
                    _ => self.rng.range(28.0, 48.0),
                };
                (a, l)
            }
            TreeSpecies::CherryBlossom => {
                let a = if depth == 1 {
                    branch_sign * self.rng.range(0.48, 1.05)
                } else {
                    (p_angle + branch_sign * self.rng.range(0.15, 0.42)).clamp(-1.35, 1.35)
                };
                let l = match depth {
                    1 => self.rng.range(70.0, 110.0),
                    2 => self.rng.range(45.0, 75.0),
                    _ => self.rng.range(30.0, 50.0),
                };
                (a, l)
            }
            TreeSpecies::SilverBirch => {
                let a = if depth == 1 {
                    branch_sign * self.rng.range(0.24, 0.58)
                } else {
                    (p_angle + branch_sign * self.rng.range(0.15, 0.35)).clamp(-0.88, 0.88)
                };
                let l = if depth == 1 {
                    self.rng.range(55.0, 85.0)
                } else {
                    self.rng.range(35.0, 58.0)
                };
                (a, l)
            }
            TreeSpecies::GreatBaobab => {
                let a = if depth == 1 {
                    let spread = [-1.15, 1.15, -0.62, 0.62];
                    let base_a = spread[slot_idx % spread.len()];
                    base_a + self.rng.range(-0.15, 0.15)
                } else {
                    (p_angle + branch_sign * self.rng.range(0.15, 0.40)).clamp(-1.35, 1.35)
                };
                let l = if depth == 1 {
                    self.rng.range(70.0, 112.0)
                } else {
                    self.rng.range(40.0, 66.0)
                };
                (a, l)
            }
            TreeSpecies::AncientBanyan => {
                let a = if depth == 1 {
                    branch_sign * self.rng.range(0.85, 1.32)
                } else {
                    (p_angle + branch_sign * self.rng.range(0.18, 0.42)).clamp(-1.38, 1.38)
                };
                let l = match depth {
                    1 => self.rng.range(84.0, 128.0),
                    2 => self.rng.range(46.0, 74.0),
                    _ => self.rng.range(30.0, 50.0),
                };
                (a, l)
            }
            TreeSpecies::AncientOak => {
                let a = if depth == 1 {
                    branch_sign * self.rng.range(0.38, 0.92)
                } else {
                    (p_angle + branch_sign * self.rng.range(0.20, 0.50)).clamp(-1.28, 1.28)
                };
                let l = match depth {
                    1 => self.rng.range(75.0, 120.0),
                    2 => self.rng.range(48.0, 80.0),
                    _ => self.rng.range(30.0, 52.0),
                };
                (a, l)
            }
        }
    }

    /// Spawn a single canopy branch on `parent_id`.
    #[allow(clippy::too_many_arguments)]
    fn spawn_canopy_branch(
        &mut self,
        species: TreeSpecies,
        parent_id: u32,
        attach_pos: [f32; 2],
        depth: u32,
        p_angle: f32,
        p_len: f32,
        p_thick: f32,
        trunk_total_len: f32,
        branch_sign: f32,
        slot_idx: usize,
        mature_now: bool,
    ) {
        self.next_id += 1;
        let id = self.next_id;
        let (angle, target_len) = self.compute_branch_geometry(
            species,
            depth,
            p_angle,
            p_len,
            attach_pos,
            trunk_total_len,
            branch_sign,
            slot_idx,
        );
        let thickness = (p_thick * 0.68).max(3.0);
        let foliage = self.generate_foliage_for_branch(species, depth, mature_now);
        let init_progress = if mature_now { 1.0 } else { 0.04 };

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
        let mut sorted = vec![root_id];
        let mut out = vec![root_id];
        for b in &self.branches {
            if let Some(pid) = b.parent_id
                && sorted.binary_search(&pid).is_ok()
                && let Err(pos) = sorted.binary_search(&b.id)
            {
                sorted.insert(pos, b.id);
                out.push(b.id);
            }
        }
        out
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

        let mature_now = !auto_succession || is_now_mature;
        let profile = SpeciesProfile::for_species(species);

        // Collect all branches currently belonging to this tree in O(1) lookup set
        let mut tree_set: std::collections::HashSet<u32> =
            self.tree_branch_ids(root_id).into_iter().collect();

        // 1. Stage 1 (Steps 1..18): Segmented, Curving Trunk Ascent & 2.5D Root Tendrils
        if step <= 18 {
            // Build a 5-segment tapering, subtly curving trunk
            let seg_len = profile.trunk_total_len / 5.0;
            let spawn_steps = [1u32, 4, 7, 11, 15];
            let trunk_seg_ids: Vec<u32> = self
                .branches
                .iter()
                .filter(|b| tree_set.contains(&b.id) && b.depth == 0 && !b.is_root)
                .map(|b| b.id)
                .collect();

            // Spawn the next trunk segment when reaching a milestone step
            if let Some(seg_idx) = spawn_steps.iter().position(|&s| s == step)
                && seg_idx > 0
                && trunk_seg_ids.len() <= seg_idx
                && let Some(&prev_seg_id) = trunk_seg_ids.last()
                && let Some((prev_tip, prev_angle, prev_thick)) = self
                    .branches
                    .iter()
                    .find(|b| b.id == prev_seg_id)
                    .map(|b| (b.current_end(), b.angle, b.thickness))
            {
                // Alternate gentle S-curve bias so the crown stays balanced over the root collar
                let curve_dir = if seg_idx % 2 == 1 { 1.0 } else { -1.0 };
                let lean = if self.rng.bool_with_prob(0.7) {
                    curve_dir
                } else {
                    -curve_dir
                };
                let delta = lean * self.rng.range(0.35, 1.0) * profile.trunk_curve_amp;
                let seg_angle = (prev_angle + delta).clamp(-0.26, 0.26);
                let seg_thick = (prev_thick * 0.84).max(8.0);
                self.next_id += 1;
                let new_seg_id = self.next_id;
                let init_len = if mature_now { seg_len } else { 10.0 };
                let init_prog = if mature_now { 1.0 } else { 0.15 };
                self.branches.push(Branch {
                    id: new_seg_id,
                    parent_id: Some(prev_seg_id),
                    depth: 0,
                    start: prev_tip,
                    end: [
                        prev_tip[0] + seg_angle.sin() * init_len,
                        prev_tip[1] - seg_angle.cos() * init_len,
                    ],
                    angle: seg_angle,
                    length: init_len,
                    target_length: seg_len,
                    thickness: seg_thick,
                    growth_progress: init_prog,
                    children_spawned: false,
                    has_nest: false,
                    sway_factor: (0.04 + seg_idx as f32 * 0.025).min(0.16),
                    is_root: false,
                    foliage: Vec::new(),
                });
                tree_set.insert(new_seg_id);
            }

            // Update target lengths and thicknesses of all existing trunk segments in one pass
            let trunk_frac = (step as f32 / 18.0).clamp(0.0, 1.0);
            let mut seg_idx = 0usize;
            let mut mature_tips: Vec<(u32, [f32; 2])> = Vec::new();
            for seg in &mut self.branches {
                if tree_set.contains(&seg.id) && seg.depth == 0 && !seg.is_root {
                    let taper = (1.0 - 0.16 * seg_idx as f32).max(0.28);
                    let target_seg_thick =
                        (4.0 + (profile.base_thickness - 4.0) * trunk_frac) * taper;
                    let seg_target_len = if seg_idx == 0 {
                        18.0 + (seg_len - 18.0) * (step as f32 / 4.0).clamp(0.0, 1.0)
                    } else {
                        seg_len
                    };
                    seg.target_length = seg_target_len;
                    if mature_now {
                        seg.length = seg_target_len;
                        seg.thickness = target_seg_thick;
                        seg.growth_progress = 1.0;
                        seg.end = seg.current_end();
                        mature_tips.push((seg.id, seg.end));
                    }
                    seg_idx += 1;
                }
            }
            for (sid, tip) in mature_tips {
                self.sync_trunk_segment_children(sid, tip);
            }

            // Spawn balanced left + right 2.5D root tendrils at steps 2, 4, 6
            if step == 2 || step == 4 || step == 6 {
                let collar_offset = (profile.base_thickness * 0.16).min(8.0);
                let init_progress = if mature_now { 1.0 } else { 0.08 };
                for &flare_dir in &[-1.0f32, 1.0f32] {
                    // Angle > PI/2 (1.57) so roots extend outward AND downward into 2.5D ground (y > 750)
                    let root_angle = flare_dir * self.rng.range(1.78, 2.28);
                    let root_len = self.rng.range(profile.root_len.0, profile.root_len.1);
                    let start_pt = [base_x + flare_dir * collar_offset, 750.0];
                    self.next_id += 1;
                    let rid = self.next_id;
                    self.branches.push(Branch {
                        id: rid,
                        parent_id: Some(root_id),
                        depth: 1,
                        start: start_pt,
                        end: [
                            start_pt[0] + root_angle.sin() * root_len,
                            start_pt[1] - root_angle.cos() * root_len,
                        ],
                        angle: root_angle,
                        length: root_len,
                        target_length: root_len,
                        thickness: profile.root_thick,
                        growth_progress: init_progress,
                        children_spawned: true,
                        has_nest: false,
                        sway_factor: 0.0,
                        is_root: true,
                        foliage: Vec::new(),
                    });
                    tree_set.insert(rid);
                }
            }

            // Secondary branching root tendrils at step 7 for heavy-rooted species
            if step == 7 && profile.has_secondary_roots {
                let primary_roots: Vec<(u32, [f32; 2], f32, f32)> = self
                    .branches
                    .iter()
                    .filter(|b| b.is_root && b.depth == 1 && tree_set.contains(&b.id))
                    .take(4)
                    .map(|b| (b.id, b.end, b.angle, b.thickness))
                    .collect();
                let init_progress = if mature_now { 1.0 } else { 0.08 };
                for (r_id, r_end, r_angle, r_thick) in primary_roots {
                    let sign = if r_angle < 0.0 { -1.0 } else { 1.0 };
                    let sub_angle =
                        (r_angle + sign * self.rng.range(-0.22, 0.25)).clamp(-2.45, 2.45);
                    let sub_angle = if sub_angle.abs() < 1.72 {
                        sign * 1.78
                    } else {
                        sub_angle
                    };
                    let sub_len = self.rng.range(18.0, 30.0);
                    self.next_id += 1;
                    let rid = self.next_id;
                    self.branches.push(Branch {
                        id: rid,
                        parent_id: Some(r_id),
                        depth: 2,
                        start: r_end,
                        end: [
                            r_end[0] + sub_angle.sin() * sub_len,
                            r_end[1] - sub_angle.cos() * sub_len,
                        ],
                        angle: sub_angle,
                        length: sub_len,
                        target_length: sub_len,
                        thickness: (r_thick * 0.55).max(3.0),
                        growth_progress: init_progress,
                        children_spawned: true,
                        has_nest: false,
                        sway_factor: 0.0,
                        is_root: true,
                        foliage: Vec::new(),
                    });
                    tree_set.insert(rid);
                }
            }
        }

        // 2. Stage 2 (Steps 5..32): Balanced Cohort Branching
        if (5..=32).contains(&step) {
            let mut trunk_segs = Vec::new();
            let mut primary_boughs_count = 0usize;
            for b in &self.branches {
                if tree_set.contains(&b.id) && !b.is_root {
                    if b.depth == 0 {
                        trunk_segs.push(b.id);
                    } else if b.depth == 1 {
                        primary_boughs_count += 1;
                    }
                }
            }
            let target_primary_boughs = match species {
                TreeSpecies::AlpinePine | TreeSpecies::GiantRedwood => 12,
                TreeSpecies::GreatBaobab => 6,
                _ => 8,
            };

            let min_trunk_seg_for_boughs = match species {
                TreeSpecies::AlpinePine => 1,
                TreeSpecies::GiantRedwood => 2,
                TreeSpecies::GreatBaobab => 3,
                _ => 1,
            };

            let total_trunk_segs = 5usize;
            let eligible_segs = (total_trunk_segs - min_trunk_seg_for_boughs).max(1);
            let total_pairs = (target_primary_boughs / 2).max(1);
            let pair_idx = primary_boughs_count / 2;
            let target_seg_idx =
                min_trunk_seg_for_boughs + (pair_idx * eligible_segs) / total_pairs;

            let target_seg_ready = trunk_segs
                .get(target_seg_idx)
                .and_then(|&id| self.branches.iter().find(|b| b.id == id))
                .is_some_and(|b| b.length >= b.target_length * 0.75);

            let spawn_primary = primary_boughs_count < target_primary_boughs
                && target_seg_ready
                && (step % 2 == 1 || primary_boughs_count < 2);

            let top_seg_ready = trunk_segs
                .get(total_trunk_segs - 1)
                .and_then(|&id| self.branches.iter().find(|b| b.id == id))
                .is_some_and(|b| b.length >= b.target_length * 0.75);
            let spawn_crown_topper =
                primary_boughs_count == target_primary_boughs && top_seg_ready && step % 2 == 1;

            if spawn_primary {
                let parent_seg_id = trunk_segs[target_seg_idx];
                let existing_on_seg = self
                    .branches
                    .iter()
                    .filter(|b| !b.is_root && b.depth == 1 && b.parent_id == Some(parent_seg_id))
                    .count();
                if let Some((p_angle, p_thick, p_len, attach_pt)) = self
                    .branches
                    .iter()
                    .find(|b| b.id == parent_seg_id)
                    .map(|b| {
                        let t = if existing_on_seg >= 2 {
                            self.rng.range(0.75, 0.96)
                        } else if target_seg_idx + 1 == total_trunk_segs {
                            self.rng.range(0.60, 0.94)
                        } else {
                            self.rng.range(0.35, 0.75)
                        };
                        (b.angle, b.thickness, b.length, b.point_at(t))
                    })
                {
                    if let Some(p) = self.branches.iter_mut().find(|b| b.id == parent_seg_id) {
                        p.children_spawned = true;
                    }
                    // Spawn a balanced left & right pair of primary boughs simultaneously
                    for (k, &sign) in [-1.0f32, 1.0f32].iter().enumerate() {
                        self.spawn_canopy_branch(
                            species,
                            parent_seg_id,
                            attach_pt,
                            1,
                            p_angle,
                            p_len,
                            p_thick,
                            profile.trunk_total_len,
                            sign,
                            primary_boughs_count + k,
                            mature_now,
                        );
                    }
                }
            } else if spawn_crown_topper {
                // Cap the top trunk segment with a central crown leader so the silhouette
                // tapers/rounds out naturally instead of ending flat at the trunk tip.
                let top_seg_id = trunk_segs[total_trunk_segs - 1];
                if let Some((top_angle, top_thick, top_tip)) = self
                    .branches
                    .iter()
                    .find(|b| b.id == top_seg_id)
                    .map(|b| (b.angle, b.thickness, b.current_end()))
                {
                    let crown_angle = match species {
                        TreeSpecies::AlpinePine | TreeSpecies::GiantRedwood => {
                            top_angle + self.rng.range(-0.08, 0.08)
                        }
                        _ => top_angle + self.rng.range(-0.16, 0.16),
                    };
                    let crown_len = match species {
                        TreeSpecies::AlpinePine | TreeSpecies::GiantRedwood => {
                            self.rng.range(48.0, 64.0)
                        }
                        TreeSpecies::GreatBaobab => self.rng.range(42.0, 56.0),
                        TreeSpecies::SilverBirch => self.rng.range(48.0, 66.0),
                        _ => self.rng.range(52.0, 74.0),
                    };
                    self.next_id += 1;
                    let id = self.next_id;
                    let foliage = self.generate_foliage_for_branch(species, 1, mature_now);
                    let init_progress = if mature_now { 1.0 } else { 0.04 };
                    self.branches.push(Branch {
                        id,
                        parent_id: Some(top_seg_id),
                        depth: 1,
                        start: top_tip,
                        end: [
                            top_tip[0] + crown_angle.sin() * crown_len,
                            top_tip[1] - crown_angle.cos() * crown_len,
                        ],
                        angle: crown_angle,
                        length: crown_len,
                        target_length: crown_len,
                        thickness: (top_thick * 0.65).max(3.5),
                        growth_progress: init_progress,
                        children_spawned: false,
                        has_nest: false,
                        sway_factor: 0.14,
                        is_root: false,
                        foliage,
                    });
                }
            } else {
                // Balanced secondary/tertiary expansion: prioritize lowest-depth branches that haven't sprouted yet
                let mut parents: Vec<(u32, u32, [f32; 2], f32, f32, f32)> = self
                    .branches
                    .iter()
                    .filter(|b| {
                        !b.is_root
                            && b.depth >= 1
                            && b.depth < 3
                            && !b.children_spawned
                            && tree_set.contains(&b.id)
                    })
                    .map(|b| (b.id, b.depth, b.end, b.angle, b.length, b.thickness))
                    .collect();

                // Sort by depth ascending so all depth-1 boughs fork before depth-2 twigs fork
                parents.sort_by_key(|p| p.1);

                let cohort_limit = if step >= 18 || parents.len() > 6 {
                    2.min(parents.len())
                } else {
                    1.min(parents.len())
                };
                for (idx, (pid, p_depth, p_end, p_angle, p_len, p_thick)) in
                    parents.into_iter().take(cohort_limit).enumerate()
                {
                    if let Some(p) = self.branches.iter_mut().find(|b| b.id == pid) {
                        p.children_spawned = true;
                    }
                    let child_depth = p_depth + 1;
                    // Spawn 2 balanced diverging sub-branches on each limb
                    for (k, &sign) in [-1.0f32, 1.0f32].iter().enumerate() {
                        self.spawn_canopy_branch(
                            species,
                            pid,
                            p_end,
                            child_depth,
                            p_angle,
                            p_len,
                            p_thick,
                            profile.trunk_total_len,
                            sign,
                            idx * 2 + k,
                            mature_now,
                        );
                    }
                }
            }

            // Ancient Banyan: aerial prop roots descending from primary horizontal boughs
            if species == TreeSpecies::AncientBanyan && (22..=28).contains(&step) && step % 3 == 1 {
                let banyan_boughs: Vec<(u32, [f32; 2])> = self
                    .branches
                    .iter()
                    .filter(|b| !b.is_root && b.depth == 1 && tree_set.contains(&b.id))
                    .map(|b| (b.id, b.point_at(0.70)))
                    .collect();
                let prop_count = self
                    .branches
                    .iter()
                    .filter(|b| {
                        b.is_root && b.parent_id != Some(root_id) && tree_set.contains(&b.id)
                    })
                    .count();
                if let Some(&(bough_id, start_pt)) =
                    banyan_boughs.get(prop_count % banyan_boughs.len().max(1))
                    && start_pt[1] < 720.0
                {
                    let prop_len = (750.0 - start_pt[1]).max(30.0);
                    let prop_angle = std::f32::consts::PI + self.rng.range(-0.06, 0.06);
                    self.next_id += 1;
                    self.branches.push(Branch {
                        id: self.next_id,
                        parent_id: Some(bough_id),
                        depth: 2,
                        start: start_pt,
                        end: [
                            start_pt[0] + prop_angle.sin() * prop_len,
                            start_pt[1] - prop_angle.cos() * prop_len,
                        ],
                        angle: prop_angle,
                        length: prop_len,
                        target_length: prop_len,
                        thickness: 6.0,
                        growth_progress: if mature_now { 1.0 } else { 0.05 },
                        children_spawned: true,
                        has_nest: false,
                        sway_factor: 0.02,
                        is_root: true,
                        foliage: Vec::new(),
                    });
                }
            }

            // Advance existing active branches together so the whole crown grows harmoniously
            for b in &mut self.branches {
                if tree_set.contains(&b.id) && !b.is_root && b.depth > 0 {
                    b.growth_progress = (b.growth_progress + 0.28).min(1.0);
                }
            }
        }

        // 3. Stage 3 (Steps 33..max_steps): Foliage Bloom, Densification & Maturation
        if step > 32 {
            for b in &mut self.branches {
                if !tree_set.contains(&b.id) {
                    continue;
                }
                if mature_now {
                    for bud in &mut b.foliage {
                        bud.bloom = 1.0;
                    }
                }
                if !b.is_root && b.depth >= 2 && b.foliage.len() < 16 {
                    let init_bloom = if mature_now { 1.0 } else { 0.15 };
                    let color_idx = if species == TreeSpecies::CherryBlossom {
                        3
                    } else if species == TreeSpecies::JapaneseMaple {
                        self.rng.range_i32(2, 4) as u8
                    } else {
                        0
                    };
                    b.foliage.push(FoliageBud {
                        offset_along: self.rng.range(0.35, 0.98),
                        lateral_offset: self.rng.range(-14.0, 14.0),
                        char_code: b'*',
                        color_idx,
                        scale: 1.1,
                        bloom: init_bloom,
                    });
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

    /// Debug mode: Generate all 9 species fully mature in a grand panorama showcase.
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

        // 2. Discrete 5-10s growth step timer & interleaved continuous growth
        self.growth_step_timer -= effective_dt;
        let max_loops = ((effective_dt / 6.0).ceil() as usize).clamp(1, 60);
        let mut loop_guard = 0;
        let mut remaining_dt = effective_dt;
        while self.growth_step_timer <= 0.0 && loop_guard < max_loops {
            let interval = self.rng.range(5.0, 8.0);
            self.growth_step_timer += interval;
            self.execute_growth_step();
            let step_dt = remaining_dt.min(interval);
            self.step_continuous_growth(step_dt);
            remaining_dt = (remaining_dt - step_dt).max(0.0);
            loop_guard += 1;
        }
        if self.growth_step_timer <= 0.0 {
            self.growth_step_timer = self.rng.range(5.0, 8.0);
        }
        if remaining_dt > 0.0 {
            self.step_continuous_growth(remaining_dt);
        }

        // 4. Ecosystem structures on mature branches over time
        if self.houses.len() < 12 && self.rng.f32() < 0.008 * (effective_dt / 10.0).clamp(0.01, 1.0)
        {
            let candidates: Vec<u32> = self
                .branches
                .iter()
                .filter(|b| {
                    !b.is_root && b.growth_progress >= 0.95 && (b.depth == 1 || b.depth == 2)
                })
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
                .filter(|b| !b.is_root && b.growth_progress >= 0.9 && b.depth == 1)
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
                    !b.is_root
                        && b.growth_progress >= 0.85
                        && b.depth >= 1
                        && b.depth <= 3
                        && !b.has_nest
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

    /// Fine-grained cell-by-cell trunk ascent, radial thickening, branch extension, and bud swelling.
    fn step_continuous_growth(&mut self, effective_dt: f32) {
        // A. Segmented trunk upward ascent & radial thickening
        let active_trunks: Vec<(u32, TreeSpecies, u32)> = self
            .trees
            .iter()
            .filter(|t| !t.is_mature)
            .map(|t| (t.root_branch_id, t.species, t.growth_steps_completed))
            .collect();

        for (root_id, species, step) in active_trunks {
            let profile = SpeciesProfile::for_species(species);
            let frac = (step as f32 / 18.0).clamp(0.0, 1.0);
            let mut seg_ids = Vec::with_capacity(5);
            for b in &self.branches {
                if b.depth == 0
                    && !b.is_root
                    && (b.id == root_id || b.parent_id.is_some_and(|pid| seg_ids.contains(&pid)))
                {
                    seg_ids.push(b.id);
                }
            }

            for (idx, seg_id) in seg_ids.into_iter().enumerate() {
                let taper = (1.0 - 0.16 * idx as f32).max(0.28);
                let target_thick = (4.0 + (profile.base_thickness - 4.0) * frac) * taper;
                let mut extended_tip = None;

                if let Some(seg) = self.branches.iter_mut().find(|b| b.id == seg_id) {
                    if seg.length < seg.target_length {
                        let len_diff = seg.target_length - seg.length;
                        let grow_step = (len_diff * 0.45).max(4.0) * effective_dt;
                        seg.length = (seg.length + grow_step).min(seg.target_length);
                        seg.end = seg.current_end();
                        extended_tip = Some(seg.current_end());
                    }
                    if seg.thickness < target_thick {
                        let thick_diff = target_thick - seg.thickness;
                        let thick_step = (thick_diff * 0.35).max(0.6) * effective_dt;
                        seg.thickness = (seg.thickness + thick_step).min(target_thick);
                    }
                }
                if let Some(tip) = extended_tip {
                    self.sync_trunk_segment_children(seg_id, tip);
                }
            }
        }

        // B. Continuous glyph-by-glyph branch extension & bud swelling
        let branch_grow_rate = 0.26 * effective_dt;
        let bud_swell_rate = 0.20 * effective_dt;

        for b in &mut self.branches {
            let fully_grown = b.growth_progress >= 1.0;
            if !fully_grown {
                b.growth_progress = (b.growth_progress + branch_grow_rate).min(1.0);
                b.end = b.current_end();
            }

            // Skip foliage scan on already-mature branches whose buds have all bloomed
            if fully_grown && b.foliage.iter().all(|f| f.bloom >= 1.0) {
                continue;
            }

            // Buds swell from single dot -> young star -> expanding cluster -> mature canopy
            for f in &mut b.foliage {
                if b.growth_progress >= f.offset_along && f.bloom < 1.0 {
                    f.bloom = (f.bloom + bud_swell_rate).min(1.0);
                }
            }
        }
    }

    /// User micro-interaction: Prune nearest branch to pointer within `radius`.
    /// Generates fluttering falling wood chips and redirects growth energy.
    pub fn prune_nearest(&mut self, pointer: [f32; 2], max_radius: f32) -> Option<u32> {
        let mut best_id = None;
        let mut min_dist = max_radius;

        for b in &self.branches {
            // Cannot prune trunk segments (depth 0) or roots
            if b.depth == 0 || b.is_root {
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
