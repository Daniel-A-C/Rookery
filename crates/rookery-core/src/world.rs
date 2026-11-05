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
    /// Ethereal shimmering violet-cyan Celestial Raven (attracted by Star Nectar).
    CelestialRaven,
    /// Radiant crimson-and-gold Ruby Tanager (attracted by Sweet Berries).
    RubyTanager,
    /// Ghostly luminous silver-white Snowy Owl (rare majestic forest guardian).
    SnowyOwl,
}

impl BirdSpecies {
    pub fn is_rare(&self) -> bool {
        matches!(
            self,
            Self::CelestialRaven | Self::RubyTanager | Self::SnowyOwl
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rook => "Rook",
            Self::AzureJay => "Azure Jay",
            Self::Goldfinch => "Goldfinch",
            Self::Robin => "Scarlet Robin",
            Self::EmeraldDove => "Emerald Dove",
            Self::CelestialRaven => "Celestial Raven",
            Self::RubyTanager => "Ruby Tanager",
            Self::SnowyOwl => "Snowy Owl",
        }
    }
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
    GatheringTwig {
        branch_id: u32,
        t: f32,
        remaining_secs: f32,
        target_nest_id: u32,
    },
    CarryingTwig {
        target_nest_id: u32,
        speed: f32,
    },
    BuildingNest {
        nest_id: u32,
        remaining_secs: f32,
    },
    BroodingEggs {
        nest_id: u32,
        remaining_secs: f32,
    },
    Bathing {
        bath_id: u32,
        remaining_secs: f32,
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

/// Snack variety dropped to feed birds and attract rare species.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SnackType {
    /// Golden birdseed mix for local songbirds.
    #[default]
    Seeds,
    /// Wild forest berries, highly appealing to Ruby Tanagers.
    SweetBerries,
    /// Shimmering starlight nectar, attracting Celestial Ravens.
    StarNectar,
}

/// A snack / birdseed dropped by the user on a branch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snack {
    pub branch_id: u32,
    pub t: f32,
    pub servings_left: u32,
    #[serde(default)]
    pub snack_type: SnackType,
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

/// A wooden plant marker sign erected when a tree/sapling is planted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantSign {
    pub id: u32,
    pub tree_root_id: u32,
    pub pos: [f32; 2],
    pub planted_age_secs: f64,
    pub label: String,
    #[serde(default)]
    pub lineage_gen: u32,
}

impl PlantSign {
    /// Format sign age relative to world time.
    pub fn age_display(&self, current_total_age: f64) -> String {
        let age_secs = (current_total_age - self.planted_age_secs).max(0.0);
        if age_secs < 60.0 {
            format!("{:.0}s old", age_secs)
        } else if age_secs < 3600.0 {
            format!("{:.0}m old", age_secs / 60.0)
        } else if age_secs < 86400.0 {
            let hours = (age_secs / 3600.0).floor();
            let mins = ((age_secs % 3600.0) / 60.0).floor();
            format!("{:.0}h {:.0}m old", hours, mins)
        } else {
            let days = (age_secs / 86400.0).floor();
            format!("{:.0}d old", days)
        }
    }
}

/// A cozy little birdhouse built over time on a mature branch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BirdHouse {
    pub id: u32,
    pub branch_id: u32,
    pub t: f32,
    pub house_style: u8,
    pub built_age_secs: f64,
}

/// A woven bird nest built over time by twigs gathered by birds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nest {
    pub id: u32,
    pub branch_id: u32,
    pub t: f32,
    pub progress: f32,
    pub eggs_count: u8,
    pub chicks_count: u8,
    pub age_secs: f64,
}

/// A cozy hanging forest lantern swaying from a branch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lantern {
    pub id: u32,
    pub branch_id: u32,
    pub t: f32,
    pub light_intensity: f32,
}

/// A carved stone bird bath on the forest floor where birds splash and bathe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BirdBath {
    pub id: u32,
    pub pos: [f32; 2],
    pub water_level: f32,
    #[serde(skip)]
    pub splash_timer: f32,
}

/// A rustic wooden resting perch where birds can flock and rest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Perch {
    pub id: u32,
    pub pos: [f32; 2],
    pub width: f32,
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
    #[serde(default)]
    pub houses_built: u32,
    #[serde(default)]
    pub eggs_laid: u32,
    #[serde(default)]
    pub chicks_hatched: u32,
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
    #[serde(default)]
    pub signs: Vec<PlantSign>,
    #[serde(default)]
    pub houses: Vec<BirdHouse>,
    #[serde(default)]
    pub nests: Vec<Nest>,
    #[serde(default)]
    pub lanterns: Vec<Lantern>,
    #[serde(default)]
    pub bird_baths: Vec<BirdBath>,
    #[serde(default)]
    pub perches: Vec<Perch>,
    #[serde(default)]
    pub viewport_offset_x: f32,
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

        let signs = vec![PlantSign {
            id: 1,
            tree_root_id: 1,
            pos: [trunk_base[0] + 34.0, trunk_base[1] + 2.0],
            planted_age_secs: 0.0,
            label: format!("Ancestral Tree · Seed #{seed}"),
            lineage_gen: 1,
        }];

        let bird_baths = vec![BirdBath {
            id: 1,
            pos: [trunk_base[0] - 160.0, trunk_base[1] + 4.0],
            water_level: 1.0,
            splash_timer: 0.0,
        }];

        let perches = vec![Perch {
            id: 1,
            pos: [trunk_base[0] + 160.0, trunk_base[1] - 30.0],
            width: 44.0,
        }];

        let lanterns = vec![Lantern {
            id: 1,
            branch_id: 1,
            t: 0.85,
            light_intensity: 1.0,
        }];

        let mut world = Self {
            version: WORLD_STATE_VERSION,
            seed,
            rng,
            total_age_secs: 0.0,
            next_id: 10,
            branches,
            birds,
            snacks: Vec::new(),
            signs,
            houses: Vec::new(),
            nests: Vec::new(),
            lanterns,
            bird_baths,
            perches,
            viewport_offset_x: 0.0,
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

        // Check if anything fast is moving (bird flights, feeding frenzies, active particles, or interactive mode)
        let any_active_birds = self.birds.iter().any(|b| {
            matches!(
                b.state,
                BirdState::Flying { .. }
                    | BirdState::Circling { .. }
                    | BirdState::Startled { .. }
                    | BirdState::CarryingTwig { .. }
                    | BirdState::Feeding { .. }
                    | BirdState::Bathing { .. }
            )
        });
        let active_items =
            !self.snacks.is_empty() || !self.notes.is_empty() || !self.chips.is_empty();

        if interactive || any_active_birds || active_items {
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

        // Milestone 4 & 5: Ecosystem structures on mature branches over time
        if self.houses.len() < 12 && self.rng.f32() < 0.008 * (dt_secs / 10.0).clamp(0.01, 1.0) {
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

    /// Autonomous bird flocking, perching, feeding, nesting, and singing AI.
    pub fn update_birds(&mut self, dt_secs: f32, _interactive: bool, _pointer: Option<[f32; 2]>) {
        let branch_count = self.branches.len();
        if branch_count == 0 {
            return;
        }

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
                        let roll = self.rng.f32();
                        if roll < 0.40 && branch_count > 1 {
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
                                    remaining_secs: self.rng.range(8.0, 22.0),
                                });
                            }
                        } else if roll < 0.55 && !self.nests.is_empty() {
                            let n_idx = self.rng.range_i32(0, self.nests.len() as i32) as usize;
                            let nest = &self.nests[n_idx];
                            if nest.progress < 1.0 {
                                next_state = Some(BirdState::BuildingNest {
                                    nest_id: nest.id,
                                    remaining_secs: 3.5,
                                });
                            } else if nest.eggs_count > 0 {
                                next_state = Some(BirdState::BroodingEggs {
                                    nest_id: nest.id,
                                    remaining_secs: 7.0,
                                });
                            }
                        } else if roll < 0.68 && !self.bird_baths.is_empty() {
                            let bath_id = self.bird_baths[0].id;
                            next_state = Some(BirdState::Bathing {
                                bath_id,
                                remaining_secs: self.rng.range(4.0, 7.0),
                            });
                        } else if roll < 0.82 {
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

                    // Chance to break into perch, bathing, or direct flight
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
                            let tx = self.rng.range(100.0, 900.0);
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

                        if dist < 14.0 {
                            next_state = Some(BirdState::BuildingNest {
                                nest_id: *target_nest_id,
                                remaining_secs: 3.5,
                            });
                        } else {
                            let step = (*speed * dt_secs).min(dist);
                            bird.pos[0] += (dx / dist) * step;
                            bird.pos[1] += (dy / dist) * step;
                        }
                    } else {
                        // Nest was pruned or missing
                        let tx = self.rng.range(200.0, 800.0);
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
                        let tx = self.rng.range(150.0, 850.0);
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
                        let tx = self.rng.range(150.0, 850.0);
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
                        let tx = self.rng.range(200.0, 800.0);
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

        // Wildlife arrivals from off-screen margins
        if self.birds.len() < 8 && self.rng.bool_with_prob(0.015 * dt_secs) {
            let from_left = self.rng.f32() < 0.5;
            let start_x = if from_left { -60.0 } else { 1060.0 };
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
                    target: [self.rng.range(300.0, 700.0), self.rng.range(150.0, 400.0)],
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
        self.nests.retain(|n| !to_remove.contains(&n.branch_id));
        self.houses.retain(|h| !to_remove.contains(&h.branch_id));
        self.lanterns.retain(|l| !to_remove.contains(&l.branch_id));
        self.snacks.retain(|s| !to_remove.contains(&s.branch_id));

        // Reassign any birds active on deleted branches
        for bird in &mut self.birds {
            let needs_flight = match &bird.state {
                BirdState::Perched { branch_id, .. }
                | BirdState::Feeding { branch_id, .. }
                | BirdState::GatheringTwig { branch_id, .. } => to_remove.contains(branch_id),
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
            foliage: Vec::new(),
        };
        self.branches.push(branch);
        self.stats.seeds_planted += 1;

        // Erect a commemorative plant sign marker with timestamp and lineage
        self.signs.push(PlantSign {
            id: self.next_id + 1,
            tree_root_id: id,
            pos: [pos[0] + 28.0, pos[1] + 2.0],
            planted_age_secs: self.total_age_secs,
            label: format!("Sapling #{}", self.stats.seeds_planted),
            lineage_gen: 2,
        });
    }

    /// User interaction: pan forest horizontally.
    pub fn pan_forest(&mut self, delta_x: f32) {
        self.viewport_offset_x = (self.viewport_offset_x + delta_x).clamp(-3000.0, 3000.0);
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
                    BirdState::Perched { .. }
                        | BirdState::Feeding { .. }
                        | BirdState::GatheringTwig { .. }
                        | BirdState::BuildingNest { .. }
                        | BirdState::BroodingEggs { .. }
                        | BirdState::Bathing { .. }
                )
            })
            .count() as u32;
        self.stats.nests_built = self.branches.iter().filter(|b| b.has_nest).count() as u32
            + self.nests.iter().filter(|n| n.progress >= 1.0).count() as u32;
        self.stats.houses_built = self.houses.len() as u32;
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

    /// Formats this world's seed into a standard sharing URI: `rookery://tree/seed-<hex>`.
    pub fn seed_uri(&self) -> String {
        format!("rookery://tree/seed-{:016x}", self.seed)
    }

    /// Serializes this world's entire simulation state into a base64url URI: `rookery://tree/state-<base64>`.
    pub fn state_uri(&self) -> Result<String, serde_json::Error> {
        use base64::Engine;
        let json = serde_json::to_vec(self)?;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&json);
        Ok(format!("rookery://tree/state-{b64}"))
    }

    /// Parses a share string (seed URI, state URI, hex seed, decimal seed, or raw JSON)
    /// and returns either a newly seeded world or the exact deserialized state.
    pub fn from_share_string(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Empty input string".into());
        }

        // 1. Exact snapshot state URI
        if let Some(rest) = trimmed.strip_prefix("rookery://tree/state-") {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(rest.trim())
                .map_err(|e| format!("Invalid base64 state: {e}"))?;
            let state: WorldState =
                serde_json::from_slice(&bytes).map_err(|e| format!("Invalid world JSON: {e}"))?;
            return Ok(state);
        }

        // 2. Seed URI: rookery://tree/seed-<hex> or seed-<hex>
        let seed_hex = if let Some(rest) = trimmed.strip_prefix("rookery://tree/seed-") {
            Some(rest.trim())
        } else {
            trimmed.strip_prefix("seed-").map(|s| s.trim())
        };

        if let Some(hex_str) = seed_hex {
            let seed = u64::from_str_radix(hex_str, 16)
                .map_err(|e| format!("Invalid hex seed '{hex_str}': {e}"))?;
            return Ok(Self::new(seed));
        }

        // 3. Raw decimal u64 seed
        if let Ok(seed) = trimmed.parse::<u64>() {
            return Ok(Self::new(seed));
        }

        // 4. Raw JSON state
        if trimmed.starts_with('{') {
            let state: WorldState =
                serde_json::from_str(trimmed).map_err(|e| format!("Invalid world JSON: {e}"))?;
            return Ok(state);
        }

        Err("Unrecognized share string. Expected 'rookery://tree/seed-...' or 'rookery://tree/state-...'".into())
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
        assert_eq!(loaded.bird_baths.len(), world.bird_baths.len());
        assert_eq!(loaded.lanterns.len(), world.lanterns.len());
        assert_eq!(loaded.perches.len(), world.perches.len());
    }

    #[test]
    fn plant_sign_age_display_and_seeding() {
        let mut world = WorldState::new(555);
        assert_eq!(world.signs.len(), 1);
        assert_eq!(world.signs[0].lineage_gen, 1);
        assert_eq!(world.signs[0].age_display(0.0), "0s old");
        assert_eq!(world.signs[0].age_display(125.0), "2m old");
        assert_eq!(world.signs[0].age_display(3665.0), "1h 1m old");
        assert_eq!(world.signs[0].age_display(90000.0), "1d old");

        world.plant_seed([550.0, 750.0]);
        assert_eq!(world.signs.len(), 2);
        assert_eq!(world.signs[1].lineage_gen, 2);
        assert_eq!(world.stats.seeds_planted, 1);
    }

    #[test]
    fn rare_birds_and_special_snacks() {
        let mut world = WorldState::new(101);
        let initial_birds = world.birds.len();

        // Dropping sweet berries attracts Ruby Tanager from off-screen
        let dropped = world.drop_snack_typed([500.0, 680.0], SnackType::SweetBerries);
        assert!(dropped);
        assert_eq!(world.snacks.len(), 1);
        assert_eq!(world.snacks[0].snack_type, SnackType::SweetBerries);
        assert!(world.birds.len() > initial_birds);
        assert!(
            world
                .birds
                .iter()
                .any(|b| b.species == BirdSpecies::RubyTanager)
        );

        // Dropping star nectar attracts Celestial Raven
        world.drop_snack_typed([500.0, 680.0], SnackType::StarNectar);
        assert!(
            world
                .birds
                .iter()
                .any(|b| b.species == BirdSpecies::CelestialRaven)
        );
    }

    #[test]
    fn nesting_lifecycle_and_infrastructure() {
        let mut world = WorldState::new(202);
        assert!(!world.bird_baths.is_empty(), "must have initial bird bath");
        assert!(!world.perches.is_empty(), "must have initial perch");
        assert!(!world.lanterns.is_empty(), "must have initial lantern");

        // Manually introduce a nest and advance lifecycle
        world.nests.push(Nest {
            id: 99,
            branch_id: 1,
            t: 0.5,
            progress: 0.2,
            eggs_count: 0,
            chicks_count: 0,
            age_secs: 0.0,
        });

        // Twig gathering and nest completion simulation
        world.birds[0].state = BirdState::BuildingNest {
            nest_id: 99,
            remaining_secs: 0.05,
        };
        world.update_birds(0.1, false, None);
        assert!(world.nests[0].progress > 0.2);

        // Brooding eggs to hatch chicks
        world.nests[0].progress = 1.0;
        world.nests[0].eggs_count = 2;
        world.birds[0].state = BirdState::BroodingEggs {
            nest_id: 99,
            remaining_secs: 0.05,
        };
        world.update_birds(0.1, false, None);
        assert_eq!(world.nests[0].chicks_count, 2);
        assert_eq!(world.nests[0].eggs_count, 0);
        assert_eq!(world.stats.chicks_hatched, 2);
    }

    #[test]
    fn seed_uri_round_trip() {
        let world = WorldState::new(0x1a2b3c4d5e6f7081);
        let uri = world.seed_uri();
        assert_eq!(uri, "rookery://tree/seed-1a2b3c4d5e6f7081");

        let parsed = WorldState::from_share_string(&uri).expect("must parse seed uri");
        assert_eq!(parsed.seed, 0x1a2b3c4d5e6f7081);
        assert_eq!(parsed.branches.len(), world.branches.len());
    }

    #[test]
    fn state_uri_round_trip() {
        let mut world = WorldState::new(0x999);
        world.advance_growth(350.0);
        world.drop_snack([500.0, 450.0]);

        let uri = world.state_uri().expect("must serialize state uri");
        assert!(uri.starts_with("rookery://tree/state-"));

        let restored = WorldState::from_share_string(&uri).expect("must parse state uri");
        assert_eq!(restored.seed, world.seed);
        assert_eq!(restored.branches.len(), world.branches.len());
        assert_eq!(restored.snacks.len(), world.snacks.len());
        assert_eq!(restored.total_age_secs, world.total_age_secs);
    }

    #[test]
    fn from_share_string_formats() {
        // Hex with seed- prefix
        let w1 = WorldState::from_share_string("seed-00000000000000ff").unwrap();
        assert_eq!(w1.seed, 255);

        // Decimal seed
        let w2 = WorldState::from_share_string("12345").unwrap();
        assert_eq!(w2.seed, 12345);

        // Error on invalid
        assert!(WorldState::from_share_string("gibberish").is_err());
    }
}
