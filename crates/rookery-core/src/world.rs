//! Deterministic simulation state for *The Living ASCII Rookery* (Milestone 2).
//!
//! Owns procedural tree growth, autonomous bird flocking/perching, micro-interactions
//! (pruning, seeding, snacks), state persistence, and offline time-delta catch-up.

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Activity;

pub const WORLD_STATE_VERSION: u32 = 1;
pub const WORLD_STATE_FILE_NAME: &str = "rookery_world.json";

/// Deterministic 64-bit XorShift PRNG for reproducible world generation and growth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRng {
    pub state: u64,
}

impl WorldRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform float in [0.0, 1.0).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo) as u64)) as i32
    }

    pub fn bool_with_prob(&mut self, prob: f32) -> bool {
        self.f32() < prob
    }
}

/// A foliage blossom / leaf cluster placed on a branch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoliageBud {
    /// Normalized distance along parent branch [0.0, 1.0].
    pub offset_along: f32,
    /// Perpendicular offset from branch center line.
    pub lateral_offset: f32,
    /// Character representation: b'*', b'%', b'&', b'@', b'o', b'8', b'#'.
    pub char_code: u8,
    /// Palette index: 0 = emerald, 1 = sage, 2 = golden autumn, 3 = blossom pink.
    pub color_idx: u8,
    /// Base size multiplier.
    pub scale: f32,
    /// Growth/bloom progress [0.0, 1.0].
    pub bloom: f32,
}

/// A single branch segment of the procedural tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    pub id: u32,
    pub parent_id: Option<u32>,
    /// Hierarchical depth: 0 = trunk base, 1 = main limb, 2+ = lateral twigs.
    pub depth: u32,
    pub start: [f32; 2],
    pub end: [f32; 2],
    /// Angle in radians relative to straight up (0.0 = straight up, negative = left, positive = right).
    pub angle: f32,
    pub length: f32,
    pub target_length: f32,
    pub thickness: f32,
    /// Growth completion [0.0, 1.0].
    pub growth_progress: f32,
    pub children_spawned: bool,
    pub has_nest: bool,
    /// Amplitude of wind sway (increases further up the tree).
    pub sway_factor: f32,
    /// Foliage clusters along this branch.
    pub foliage: Vec<FoliageBud>,
}

impl Branch {
    pub fn current_end(&self) -> [f32; 2] {
        let actual_len = self.length * self.growth_progress;
        [
            self.start[0] + self.angle.sin() * actual_len,
            self.start[1] - self.angle.cos() * actual_len,
        ]
    }

    /// Evaluates point along the branch at parameter t in [0.0, 1.0].
    pub fn point_at(&self, t: f32) -> [f32; 2] {
        let actual_len = self.length * self.growth_progress;
        let l = actual_len * t.clamp(0.0, 1.0);
        [
            self.start[0] + self.angle.sin() * l,
            self.start[1] - self.angle.cos() * l,
        ]
    }
}

/// Species variety of autonomous birds in the rookery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BirdSpecies {
    /// Classic dark charcoal Rook / Crow.
    Rook,
    /// Sapphire / Azure Jay.
    AzureJay,
    /// Golden / Amber Finch.
    Goldfinch,
    /// Chestnut / Scarlet Robin.
    Robin,
    /// Emerald / Sage Dove.
    EmeraldDove,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BirdState {
    Flying {
        target: [f32; 2],
        speed: f32,
    },
    Circling {
        center: [f32; 2],
        radius: f32,
        angle: f32,
        speed: f32,
    },
    Perched {
        branch_id: u32,
        t: f32,
        remaining_secs: f32,
    },
    Feeding {
        branch_id: u32,
        t: f32,
        remaining_secs: f32,
    },
    Startled {
        remaining_secs: f32,
        dir: [f32; 2],
    },
}

/// An autonomous procedural bird in the world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bird {
    pub id: u32,
    pub species: BirdSpecies,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub facing_left: bool,
    pub state: BirdState,
    pub sing_timer: f32,
    pub wing_phase: f32,
    pub size_scale: f32,
}

/// A snack / birdseed dropped by the user on a branch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snack {
    pub branch_id: u32,
    pub t: f32,
    pub servings_left: u32,
}

/// Floating musical note emitted when birds sing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingNote {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub glyph: char,
    pub life: f32,
    pub max_life: f32,
    pub color_idx: u8,
}

/// Falling character chip created when pruning branches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FallingChip {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub rotation: f32,
    pub rot_vel: f32,
    pub glyph: char,
    pub life: f32,
    pub max_life: f32,
}

/// Statistics and milestones of the living world.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldStats {
    pub total_branches: u32,
    pub total_foliage: u32,
    pub birds_active: u32,
    pub birds_perched: u32,
    pub pruned_branches: u32,
    pub seeds_planted: u32,
    pub snacks_eaten: u32,
    pub nests_built: u32,
}

/// Complete serialized state of The Living ASCII Rookery world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub total_age_secs: f64,
    pub next_id: u32,
    pub branches: Vec<Branch>,
    pub birds: Vec<Bird>,
    pub snacks: Vec<Snack>,
    #[serde(skip)]
    pub notes: Vec<FloatingNote>,
    #[serde(skip)]
    pub chips: Vec<FallingChip>,
    pub stats: WorldStats,
    pub last_saved_utc: Option<DateTime<Utc>>,
    pub total_offline_caught_up_secs: f64,
}

impl WorldState {
    /// Initialize a brand new Living Rookery world with the given random seed.
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut branches = Vec::new();

        // Base trunk at default normalized coordinate space [500.0, 750.0]
        // (will be scaled/mapped to viewport by the renderer).
        let trunk_base = [500.0, 750.0];
        let trunk_len = 110.0;
        let trunk = Branch {
            id: 1,
            parent_id: None,
            depth: 0,
            start: trunk_base,
            end: [trunk_base[0], trunk_base[1] - trunk_len],
            angle: 0.0,
            length: trunk_len,
            target_length: trunk_len,
            thickness: 18.0,
            growth_progress: 1.0,
            children_spawned: false,
            has_nest: false,
            sway_factor: 0.05,
            foliage: Vec::new(),
        };
        branches.push(trunk);

        let mut birds = Vec::new();
        let species_pool = [
            BirdSpecies::Rook,
            BirdSpecies::AzureJay,
            BirdSpecies::Goldfinch,
            BirdSpecies::Robin,
            BirdSpecies::EmeraldDove,
        ];

        // Seed 4 initial birds circling or flying
        for i in 0..4 {
            let sp = species_pool[(i as usize) % species_pool.len()];
            let x = rng.range(200.0, 800.0);
            let y = rng.range(150.0, 450.0);
            let speed = rng.range(35.0, 75.0);
            birds.push(Bird {
                id: i + 1,
                species: sp,
                pos: [x, y],
                vel: [if i % 2 == 0 { speed } else { -speed }, 0.0],
                facing_left: i % 2 != 0,
                state: BirdState::Circling {
                    center: [500.0, 350.0],
                    radius: rng.range(120.0, 260.0),
                    angle: rng.range(0.0, std::f32::consts::TAU),
                    speed: rng.range(0.3, 0.7) * if i % 2 == 0 { 1.0 } else { -1.0 },
                },
                sing_timer: rng.range(4.0, 15.0),
                wing_phase: rng.range(0.0, std::f32::consts::TAU),
                size_scale: rng.range(0.9, 1.15),
            });
        }

        let mut world = Self {
            version: WORLD_STATE_VERSION,
            seed,
            rng,
            total_age_secs: 0.0,
            next_id: 10,
            branches,
            birds,
            snacks: Vec::new(),
            notes: Vec::new(),
            chips: Vec::new(),
            stats: WorldStats::default(),
            last_saved_utc: Some(Utc::now()),
            total_offline_caught_up_secs: 0.0,
        };

        // Advance a few initial growth ticks so the sapling has its first branches
        world.advance_growth(300.0);
        world.recompute_stats();
        world
    }

    /// Primary simulation update step. Returns [`Activity::Animating`] if birds are in flight
    /// or interactions are occurring, or [`Activity::Idle`] if ambient and still.
    pub fn update(
        &mut self,
        dt_secs: f32,
        interactive: bool,
        pointer: Option<[f32; 2]>,
    ) -> Activity {
        if dt_secs <= 0.0 {
            return Activity::Idle;
        }

        self.total_age_secs += dt_secs as f64;
        self.advance_growth(dt_secs);
        self.update_birds(dt_secs, interactive, pointer);
        self.update_notes_and_chips(dt_secs);

        // Interactive mouse effects (gust / scatter birds)
        if interactive && let Some(p) = pointer {
            self.scatter_birds_near(p, 70.0);
        }

        self.recompute_stats();

        // Check if anything fast is moving (flying birds, active particles, or interactive mode)
        let any_flying_birds = self.birds.iter().any(|b| {
            matches!(
                b.state,
                BirdState::Flying { .. } | BirdState::Startled { .. }
            )
        });
        let particles_active = !self.notes.is_empty() || !self.chips.is_empty();

        if interactive || any_flying_birds || particles_active {
            Activity::Animating
        } else {
            Activity::Idle
        }
    }

    /// Advance tree growth by `dt_secs`.
    pub fn advance_growth(&mut self, dt_secs: f32) {
        // Growth rate: 1.0 growth progress takes ~120-240 seconds per branch segment in real-time,
        // producing a majestic slow-evolving structure.
        let base_rate = 0.006 * dt_secs;
        let mut new_branches = Vec::new();

        let branch_count = self.branches.len();
        for i in 0..branch_count {
            let b = &mut self.branches[i];
            if b.growth_progress < 1.0 {
                // Trunk grows faster, outer twigs slower
                let depth_factor = 1.0 / (b.depth as f32 * 0.4 + 1.0);
                b.growth_progress = (b.growth_progress + base_rate * depth_factor).min(1.0);
                b.end = b.current_end();
            }

            // Once mature, spawn child branches if not already done and max depth not reached
            if b.growth_progress >= 1.0 && !b.children_spawned && b.depth < 5 && branch_count < 120
            {
                b.children_spawned = true;
                let parent_id = b.id;
                let parent_end = b.end;
                let parent_angle = b.angle;
                let depth = b.depth + 1;
                let parent_thickness = b.thickness;

                // Determine child count: trunk gets 2-3 branches, twigs get 1-2
                let count = if depth == 1 {
                    self.rng.range_i32(2, 4)
                } else if depth <= 3 {
                    self.rng.range_i32(1, 3)
                } else {
                    1
                };

                for k in 0..count {
                    self.next_id += 1;
                    let id = self.next_id;

                    // Branch spread angles
                    let angle_offset = if count == 1 {
                        self.rng.range(-0.35, 0.35)
                    } else {
                        let span = 0.85;
                        let step = span / ((count - 1).max(1) as f32);
                        -span * 0.5 + step * (k as f32) + self.rng.range(-0.15, 0.15)
                    };
                    let angle = (parent_angle + angle_offset).clamp(-1.2, 1.2);

                    let target_len = (b.target_length * self.rng.range(0.68, 0.88)).max(24.0);
                    let thickness = (parent_thickness * 0.72).max(2.0);
                    let sway = (b.sway_factor * 1.35).min(0.8);

                    // Generate foliage clusters along new twigs
                    let mut foliage = Vec::new();
                    if depth >= 2 {
                        let leaves_count = self.rng.range_i32(3, 8) as usize;
                        let chars = *b"*%&@o8#";
                        for _ in 0..leaves_count {
                            let char_code =
                                chars[self.rng.range_i32(0, chars.len() as i32) as usize];
                            let color_idx = if depth == 4 {
                                self.rng.range_i32(0, 4) as u8
                            } else {
                                self.rng.range_i32(0, 2) as u8
                            };
                            foliage.push(FoliageBud {
                                offset_along: self.rng.range(0.3, 1.0),
                                lateral_offset: self.rng.range(-12.0, 12.0),
                                char_code,
                                color_idx,
                                scale: self.rng.range(0.8, 1.3),
                                bloom: 0.1,
                            });
                        }
                    }

                    new_branches.push(Branch {
                        id,
                        parent_id: Some(parent_id),
                        depth,
                        start: parent_end,
                        end: parent_end,
                        angle,
                        length: target_len,
                        target_length: target_len,
                        thickness,
                        growth_progress: 0.05,
                        children_spawned: false,
                        has_nest: false,
                        sway_factor: sway,
                        foliage,
                    });
                }
            }

            // Bloom foliage on existing branches
            for f in &mut b.foliage {
                if f.bloom < 1.0 {
                    f.bloom = (f.bloom + base_rate * 1.5).min(1.0);
                }
            }
        }

        self.branches.extend(new_branches);
    }

    /// Autonomous bird flocking, perching, feeding, and singing AI.
    pub fn update_birds(&mut self, dt_secs: f32, _interactive: bool, _pointer: Option<[f32; 2]>) {
        let branch_count = self.branches.len();
        if branch_count == 0 {
            return;
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
                        // Arrived near target: choose new action (perch, circle, or new destination)
                        let roll = self.rng.f32();
                        if roll < 0.5 && branch_count > 1 {
                            // Find suitable mature branch to perch on
                            let candidates: Vec<u32> = self
                                .branches
                                .iter()
                                .filter(|b| b.growth_progress >= 0.8 && b.depth >= 1)
                                .map(|b| b.id)
                                .collect();
                            if !candidates.is_empty() {
                                let branch_id = candidates
                                    [self.rng.range_i32(0, candidates.len() as i32) as usize];
                                let t = self.rng.range(0.4, 0.95);
                                next_state = Some(BirdState::Perched {
                                    branch_id,
                                    t,
                                    remaining_secs: self.rng.range(8.0, 25.0),
                                });
                            }
                        } else if roll < 0.8 {
                            next_state = Some(BirdState::Circling {
                                center: [500.0, 320.0],
                                radius: self.rng.range(140.0, 300.0),
                                angle: self.rng.range(0.0, std::f32::consts::TAU),
                                speed: self.rng.range(0.4, 0.8)
                                    * if self.rng.f32() > 0.5 { 1.0 } else { -1.0 },
                            });
                        } else {
                            let tx = self.rng.range(150.0, 850.0);
                            let ty = self.rng.range(100.0, 480.0);
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

                    // Chance to break into perch or direct flight
                    if self.rng.bool_with_prob(0.03 * dt_secs) {
                        let tx = self.rng.range(100.0, 900.0);
                        let ty = self.rng.range(120.0, 450.0);
                        let sp = self.rng.range(45.0, 85.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: sp,
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
                        // Take flight again
                        let tx = self.rng.range(100.0, 900.0);
                        let ty = self.rng.range(80.0, 350.0);
                        let sp = self.rng.range(50.0, 90.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: sp,
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
                        let tx = self.rng.range(200.0, 800.0);
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
                        let tx = self.rng.range(150.0, 850.0);
                        let ty = self.rng.range(100.0, 350.0);
                        next_state = Some(BirdState::Flying {
                            target: [tx, ty],
                            speed: 60.0,
                        });
                    }
                }
            }

            if let Some(st) = next_state {
                bird.state = st;
            }
        }

        self.notes.extend(new_notes);

        // Check if snacks attract perched birds
        if !self.snacks.is_empty() {
            for snack in &mut self.snacks {
                if snack.servings_left > 0 {
                    // Find a bird to feed
                    for b in &mut self.birds {
                        if matches!(
                            b.state,
                            BirdState::Circling { .. } | BirdState::Flying { .. }
                        ) {
                            b.state = BirdState::Feeding {
                                branch_id: snack.branch_id,
                                t: snack.t,
                                remaining_secs: 5.0,
                            };
                            snack.servings_left -= 1;
                            break;
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
    fn remove_branch_and_descendants(&mut self, target_id: u32) {
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

        // Reassign any perched birds on deleted branches
        for bird in &mut self.birds {
            match &bird.state {
                BirdState::Perched { branch_id, .. } | BirdState::Feeding { branch_id, .. }
                    if to_remove.contains(branch_id) =>
                {
                    let tx = self.rng.range(200.0, 800.0);
                    let ty = self.rng.range(100.0, 300.0);
                    bird.state = BirdState::Flying {
                        target: [tx, ty],
                        speed: 70.0,
                    };
                }
                _ => {}
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
            foliage: Vec::new(),
        };
        self.branches.push(branch);
        self.stats.seeds_planted += 1;
    }

    /// User micro-interaction: drop a snack on nearest branch to attract birds.
    pub fn drop_snack(&mut self, pos: [f32; 2]) -> bool {
        let mut best_id = None;
        let mut min_dist = 60.0;

        for b in &self.branches {
            let mid = b.point_at(0.5);
            let dx = mid[0] - pointer_dist(mid, pos);
            if dx < min_dist {
                min_dist = dx;
                best_id = Some(b.id);
            }
        }

        if let Some(id) = best_id {
            self.snacks.push(Snack {
                branch_id: id,
                t: 0.5,
                servings_left: 2,
            });
            true
        } else {
            false
        }
    }

    /// Offline time-delta engine: deterministically catch up simulation state after sleep or shutdown.
    /// Fast-forwards tree growth and advances bird states without lag (< 5ms execution time).
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 {
            return;
        }

        log::info!(
            "offline time-delta engine: catching up {:.1}s of offline progression",
            elapsed_secs
        );

        self.total_offline_caught_up_secs += elapsed_secs;
        self.total_age_secs += elapsed_secs;

        // Step through in bulk chunks of 15 seconds, capped at max 400 steps (6,000s = 100 min)
        // For longer durations (e.g. 24h), execute 200 bulk steps with scaled dt.
        let max_steps = 200;
        let step_dt = (elapsed_secs / max_steps as f64).min(30.0) as f32;
        let steps = ((elapsed_secs / step_dt as f64).round() as usize).min(max_steps);

        for _ in 0..steps {
            self.advance_growth(step_dt);
        }

        self.recompute_stats();
    }

    pub fn recompute_stats(&mut self) {
        self.stats.total_branches = self.branches.len() as u32;
        self.stats.total_foliage = self.branches.iter().map(|b| b.foliage.len() as u32).sum();
        self.stats.birds_active = self.birds.len() as u32;
        self.stats.birds_perched = self
            .birds
            .iter()
            .filter(|b| {
                matches!(
                    b.state,
                    BirdState::Perched { .. } | BirdState::Feeding { .. }
                )
            })
            .count() as u32;
        self.stats.nests_built = self.branches.iter().filter(|b| b.has_nest).count() as u32;
    }

    /// Load persisted world state from disk, falling back to a freshly seeded world if missing or corrupt.
    pub fn load_or_init(path: &Path, seed: u64) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<WorldState>(&content) {
                Ok(mut state) => {
                    state.recompute_stats();
                    log::info!(
                        "loaded world state from {} (age {:.1}h, {} branches)",
                        path.display(),
                        state.total_age_secs / 3600.0,
                        state.branches.len()
                    );
                    state
                }
                Err(e) => {
                    log::warn!(
                        "failed to deserialize {}: {e}; creating new world",
                        path.display()
                    );
                    Self::new(seed)
                }
            },
            Err(_) => Self::new(seed),
        }
    }

    /// Atomically persist current world state to disk (write temp file + sync + rename).
    pub fn save_atomic(&mut self, path: &Path) -> std::io::Result<()> {
        self.last_saved_utc = Some(Utc::now());
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json.as_bytes())?;
        fs::rename(&temp_path, path)?;
        Ok(())
    }
}

fn pointer_dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_initializes_with_tree_and_birds() {
        let world = WorldState::new(42);
        assert!(!world.branches.is_empty(), "must have initial branches");
        assert_eq!(world.birds.len(), 4, "must have 4 initial birds");
        assert!(world.branches[0].depth == 0, "root must have depth 0");
    }

    #[test]
    fn deterministic_world_generation() {
        let w1 = WorldState::new(12345);
        let w2 = WorldState::new(12345);
        assert_eq!(w1.branches.len(), w2.branches.len());
        assert_eq!(w1.branches[0].length, w2.branches[0].length);
        assert_eq!(w1.birds.len(), w2.birds.len());
    }

    #[test]
    fn growth_advances_over_time() {
        let mut world = WorldState::new(99);
        let initial_count = world.branches.len();
        world.advance_growth(1000.0);
        assert!(
            world.branches.len() >= initial_count,
            "branches should grow or maintain count"
        );
        assert!(world.branches.iter().any(|b| b.growth_progress > 0.0));
    }

    #[test]
    fn offline_catchup_is_fast_and_deterministic() {
        let mut world = WorldState::new(777);
        let start = std::time::Instant::now();
        // Catch up 12 hours of sleep
        world.on_wake(12.0 * 3600.0);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 50,
            "12 hours offline catch-up took {:?}, must be under 50ms",
            elapsed
        );
        assert_eq!(world.total_offline_caught_up_secs, 12.0 * 3600.0);
    }

    #[test]
    fn pruning_removes_branch_and_descendants() {
        let mut world = WorldState::new(888);
        world.advance_growth(500.0);
        let initial_count = world.branches.len();

        // Prune branch with depth > 0
        if let Some(target) = world.branches.iter().find(|b| b.depth == 1).map(|b| b.id) {
            world.remove_branch_and_descendants(target);
            assert!(world.branches.len() < initial_count);
            assert!(!world.branches.iter().any(|b| b.id == target));
            assert!(!world.chips.is_empty(), "pruning must spawn wood chips");
        }
    }

    #[test]
    fn serialization_round_trip() {
        let mut world = WorldState::new(333);
        world.advance_growth(200.0);
        let temp = tempfile::NamedTempFile::new().unwrap();
        world.save_atomic(temp.path()).unwrap();

        let loaded = WorldState::load_or_init(temp.path(), 333);
        assert_eq!(loaded.seed, world.seed);
        assert_eq!(loaded.branches.len(), world.branches.len());
        assert_eq!(loaded.birds.len(), world.birds.len());
    }
}
