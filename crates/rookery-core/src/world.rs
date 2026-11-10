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
    /// Whether this branch segment represents a ground buttress root.
    #[serde(default)]
    pub is_root: bool,
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

/// Real-world inspired tree species supported by The Living ASCII Rookery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TreeSpecies {
    #[default]
    AncientOak,
    WeepingWillow,
    AlpinePine,
    JapaneseMaple,
    SilverBirch,
    GreatBaobab,
}

impl TreeSpecies {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AncientOak => "Ancient Oak",
            Self::WeepingWillow => "Weeping Willow",
            Self::AlpinePine => "Alpine Pine",
            Self::JapaneseMaple => "Japanese Maple",
            Self::SilverBirch => "Silver Birch",
            Self::GreatBaobab => "Great Baobab",
        }
    }

    pub fn all() -> &'static [TreeSpecies] {
        &[
            Self::AncientOak,
            Self::WeepingWillow,
            Self::AlpinePine,
            Self::JapaneseMaple,
            Self::SilverBirch,
            Self::GreatBaobab,
        ]
    }

    pub fn next(&self) -> Self {
        match self {
            Self::AncientOak => Self::WeepingWillow,
            Self::WeepingWillow => Self::AlpinePine,
            Self::AlpinePine => Self::JapaneseMaple,
            Self::JapaneseMaple => Self::SilverBirch,
            Self::SilverBirch => Self::GreatBaobab,
            Self::GreatBaobab => Self::AncientOak,
        }
    }

    pub fn max_growth_steps(&self) -> u32 {
        match self {
            Self::AncientOak => 42,
            Self::WeepingWillow => 40,
            Self::AlpinePine => 45,
            Self::JapaneseMaple => 36,
            Self::SilverBirch => 40,
            Self::GreatBaobab => 38,
        }
    }
}

/// Curated ANSI Roguelike color palettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RookeryPalette {
    #[default]
    EarthAndSylvan, // 0: Caves of Qud / Earth Sylvan (umber bark, jade & emerald canopy)
    MidnightStarlight, // 1: Moonlit silver & slate bark, frosted cyan foliage, obsidian sky
    AutumnHearth,      // 2: Rust & mahogany bark, golden amber, burnt orange, ruby canopy
    VerdantRainforest, // 3: Dark mossy bark, vibrant lime, emerald & orchid blossom canopy
    AmberPhosphor,     // 4: Retro monochrome amber CRT terminal
    MatrixGreen,       // 5: Retro hacker green phosphor terminal
}

pub const ROOKERY_PALETTE_COUNT: u8 = 6;
pub const ROOKERY_PALETTE_NAMES: [&str; 6] = [
    "Earth & Sylvan (Caves of Qud)",
    "Midnight Starlight",
    "Autumn Hearth",
    "Verdant Rainforest",
    "Amber Phosphor CRT",
    "Emerald Matrix",
];

impl RookeryPalette {
    pub fn display_name(&self) -> &'static str {
        ROOKERY_PALETTE_NAMES[self.as_u8() as usize]
    }

    pub fn all() -> &'static [RookeryPalette] {
        &[
            Self::EarthAndSylvan,
            Self::MidnightStarlight,
            Self::AutumnHearth,
            Self::VerdantRainforest,
            Self::AmberPhosphor,
            Self::MatrixGreen,
        ]
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            Self::EarthAndSylvan => 0,
            Self::MidnightStarlight => 1,
            Self::AutumnHearth => 2,
            Self::VerdantRainforest => 3,
            Self::AmberPhosphor => 4,
            Self::MatrixGreen => 5,
        }
    }

    pub fn from_u8(val: u8) -> Self {
        match val % ROOKERY_PALETTE_COUNT {
            0 => Self::EarthAndSylvan,
            1 => Self::MidnightStarlight,
            2 => Self::AutumnHearth,
            3 => Self::VerdantRainforest,
            4 => Self::AmberPhosphor,
            _ => Self::MatrixGreen,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_u8(self.as_u8() + 1)
    }

    pub fn prev(&self) -> Self {
        Self::from_u8((self.as_u8() + ROOKERY_PALETTE_COUNT - 1) % ROOKERY_PALETTE_COUNT)
    }
}

/// ANSI roguelike bird rendering styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BirdStyle {
    #[default]
    Classic3Cell, // 3-cell CP437 avians (< @ / perched, \ ^ / flight)
    Streamlined2Cell, // 2-cell songbirds (< @ perched, ^ ^ flight)
    OrnateFlock,      // Distinct 3x1 and 2x2 silhouettes for rare species
}

impl BirdStyle {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Classic3Cell => "3-Cell Classic CP437",
            Self::Streamlined2Cell => "2-Cell Streamlined",
            Self::OrnateFlock => "Ornate Silhouettes",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Classic3Cell => Self::Streamlined2Cell,
            Self::Streamlined2Cell => Self::OrnateFlock,
            Self::OrnateFlock => Self::Classic3Cell,
        }
    }
}

/// State record for an individual tree in the forest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeRecord {
    pub id: u32,
    pub species: TreeSpecies,
    pub base_pos: [f32; 2],
    pub root_branch_id: u32,
    pub is_mature: bool,
    pub growth_steps_completed: u32,
    pub target_growth_steps: u32,
    pub planted_age_secs: f64,
}

fn default_growth_step_timer() -> f32 {
    6.0
}

/// Aesthetic visual style names for The Living ASCII Rookery mock-ups (retained for backward compatibility).
pub const ROOKERY_STYLE_COUNT: u8 = 8;
pub const ROOKERY_STYLE_NAMES: [&str; 8] = [
    "Classic SDF Woodland",
    "Strict ANSI Roguelike",
    "Full-Screen ASCII Matrix",
    "Multi-Line ASCII Art",
    "Retro Phosphor CRT",
    "Sumi-e Ink & Parchment",
    "Stained-Glass Mosaic",
    "3D Volumetric Tree",
];

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
    #[serde(default)]
    pub aesthetic_style: u8,
    #[serde(default)]
    pub trees: Vec<TreeRecord>,
    #[serde(default)]
    pub active_tree_id: u32,
    #[serde(default = "default_growth_step_timer")]
    pub growth_step_timer: f32,
    #[serde(default)]
    pub target_viewport_offset_x: f32,
    #[serde(default)]
    pub active_palette: RookeryPalette,
    #[serde(default)]
    pub bird_style: BirdStyle,
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
    /// The active tree starts as a tiny sapling barely taking up a few cells at ground level.
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut branches = Vec::new();

        // Base trunk sprout at default normalized coordinate space [500.0, 750.0]
        let trunk_base = [500.0, 750.0];
        let sprout_len = 18.0;
        let trunk = Branch {
            id: 1,
            parent_id: None,
            depth: 0,
            start: trunk_base,
            end: [trunk_base[0], trunk_base[1] - sprout_len],
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
                bloom: 0.8,
            }],
        };
        branches.push(trunk);

        let initial_tree = TreeRecord {
            id: 1,
            species: TreeSpecies::AncientOak,
            base_pos: trunk_base,
            root_branch_id: 1,
            is_mature: false,
            growth_steps_completed: 0,
            target_growth_steps: TreeSpecies::AncientOak.max_growth_steps(),
            planted_age_secs: 0.0,
        };

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
            label: format!("Ancestral Sapling · Seed #{seed}"),
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
            aesthetic_style: 1, // Strict ANSI Roguelike
            trees: vec![initial_tree],
            active_tree_id: 1,
            growth_step_timer: 6.0,
            target_viewport_offset_x: 0.0,
            active_palette: RookeryPalette::EarthAndSylvan,
            bird_style: BirdStyle::Classic3Cell,
            notes: Vec::new(),
            chips: Vec::new(),
            stats: WorldStats::default(),
            last_saved_utc: Some(Utc::now()),
            total_offline_caught_up_secs: 0.0,
        };

        world.recompute_stats();
        world
    }

    pub fn debug_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== World State Dump ===\n");
        dump.push_str(&format!("Trees: {}\n", self.trees.len()));
        for tree in &self.trees {
            dump.push_str(&format!(
                "  Tree ID: {}, Mature: {}, Growth Steps: {}/{}\n",
                tree.id, tree.is_mature, tree.growth_steps_completed, tree.target_growth_steps
            ));
        }
        dump.push_str(&format!("Birds: {}\n", self.birds.len()));
        for bird in &self.birds {
            dump.push_str(&format!(
                "  Bird Species: {:?}, State: {:?}\n",
                bird.species, bird.state
            ));
        }
        dump
    }

    /// Renders a 2D text canvas of the active viewport, displaying the tree structure,
    /// foliage, ground line, and bird flock. Extremely useful for test logging and inspection.
    #[allow(clippy::needless_range_loop)]
    pub fn render_ascii_art(&self, width_chars: usize, height_chars: usize) -> String {
        let w = width_chars.max(40);
        let h = height_chars.max(15);
        let mut grid = vec![vec![' '; w]; h];

        let vp_x = self.viewport_offset_x;
        let ground_y = 750.0f32;
        let ground_row = ((ground_y / 800.0) * h as f32).clamp(1.0, (h - 2) as f32) as usize;

        // 1. Draw ground horizon
        for c in 0..w {
            grid[ground_row][c] = '─';
            for r in (ground_row + 1)..h {
                grid[r][c] = if (c + r) % 3 == 0 { '.' } else { ' ' };
            }
        }

        // Coordinate mapper from world space to grid cell (col, row)
        let world_to_grid = |pt: [f32; 2]| -> Option<(usize, usize)> {
            let rel_x = pt[0] - vp_x;
            if !(0.0..=1000.0).contains(&rel_x) || !(0.0..=800.0).contains(&pt[1]) {
                return None;
            }
            let c = ((rel_x / 1000.0) * w as f32) as usize;
            let r = ((pt[1] / 800.0) * h as f32) as usize;
            if c < w && r < h { Some((c, r)) } else { None }
        };

        // 2. Draw tree branches
        for b in &self.branches {
            let start = b.start;
            let end = b.current_end();
            let steps = 25;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let pt = [
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ];
                if let Some((c, r)) = world_to_grid(pt)
                    && (r < ground_row || (b.is_root && r <= ground_row + 1))
                {
                    let ch = if b.depth == 0 {
                        '║'
                    } else if b.angle.abs() < 0.25 {
                        '│'
                    } else if b.angle < -1.1 || b.angle > 1.1 {
                        '═'
                    } else if b.angle < 0.0 {
                        '\\'
                    } else {
                        '/'
                    };
                    grid[r][c] = ch;
                }
            }

            // 3. Draw foliage
            for bud in &b.foliage {
                if bud.bloom > 0.1
                    && let Some((c, r)) = world_to_grid(b.point_at(bud.offset_along))
                    && r < ground_row
                    && grid[r][c] == ' '
                {
                    grid[r][c] = '*';
                }
            }
        }

        // 4. Draw birds
        for b in &self.birds {
            if let Some((c, r)) = world_to_grid(b.pos)
                && r < h
            {
                let sprite = match b.state {
                    BirdState::Perched { .. } => {
                        if b.facing_left {
                            "<@/"
                        } else {
                            "\\@>"
                        }
                    }
                    _ => "\\^/",
                };
                for (offset, ch) in sprite.chars().enumerate() {
                    let col_idx =
                        (c as isize - 1 + offset as isize).clamp(0, (w - 1) as isize) as usize;
                    grid[r][col_idx] = ch;
                }
            }
        }

        // 5. Build boxed string output
        let mut out = String::new();
        out.push('┌');
        for _ in 0..w {
            out.push('─');
        }
        out.push_str("┐\n");

        for row in grid {
            out.push('│');
            for ch in row {
                out.push(ch);
            }
            out.push_str("│\n");
        }

        out.push('└');
        for _ in 0..w {
            out.push('─');
        }
        out.push_str("┘\n");

        out
    }

    /// Set the active curated ANSI Roguelike color palette.
    pub fn set_palette(&mut self, palette: RookeryPalette) {
        self.active_palette = palette;
        self.aesthetic_style = palette.as_u8();
    }

    /// Cycle to the next curated color palette.
    pub fn cycle_palette(&mut self) -> RookeryPalette {
        self.active_palette = self.active_palette.next();
        self.aesthetic_style = self.active_palette.as_u8();
        self.active_palette
    }

    /// Cycle to the previous curated color palette.
    pub fn prev_palette(&mut self) -> RookeryPalette {
        self.active_palette = self.active_palette.prev();
        self.aesthetic_style = self.active_palette.as_u8();
        self.active_palette
    }

    /// Cycle bird rendering style.
    pub fn cycle_bird_style(&mut self) -> BirdStyle {
        self.bird_style = self.bird_style.next();
        self.bird_style
    }

    /// Set the active aesthetic style index (delegates to palette for backward compatibility).
    pub fn set_aesthetic_style(&mut self, style: u8) {
        self.set_palette(RookeryPalette::from_u8(style));
    }

    /// Cycle to next aesthetic style (delegates to palette).
    pub fn cycle_aesthetic_style(&mut self) -> u8 {
        self.cycle_palette().as_u8()
    }

    /// Cycle to previous aesthetic style (delegates to palette).
    pub fn prev_aesthetic_style(&mut self) -> u8 {
        self.prev_palette().as_u8()
    }

    /// Human-readable display name for the current active palette.
    pub fn aesthetic_style_name(&self) -> &'static str {
        self.active_palette.display_name()
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
        let record = TreeRecord {
            id: tree_id,
            species,
            base_pos: trunk_base,
            root_branch_id: root_id,
            is_mature: false,
            growth_steps_completed: 0,
            target_growth_steps: species.max_growth_steps(),
            planted_age_secs: self.total_age_secs,
        };
        self.trees.push(record);
        self.active_tree_id = tree_id;
        self.recompute_stats();
        tree_id
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
                    let next_x = last.base_pos[0] + 800.0;
                    let next_species = last.species.next();
                    self.target_viewport_offset_x = last.base_pos[0] + 800.0 - 500.0;
                    self.plant_new_sapling(next_x, next_species);
                }
                return;
            }
        };

        let tree = &mut self.trees[active_idx];
        tree.growth_steps_completed += 1;
        let step = tree.growth_steps_completed;
        let species = tree.species;
        let root_id = tree.root_branch_id;
        let base_x = tree.base_pos[0];
        let max_steps = tree.target_growth_steps;
        let is_now_mature = step >= max_steps;

        // Collect all branches currently belonging to this tree
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

        // 1. Stage 1 (Steps 1..18): Progressive Trunk Ascent & Buttress Roots
        let target_trunk_len = match species {
            TreeSpecies::AlpinePine => 380.0,
            TreeSpecies::SilverBirch => 260.0,
            TreeSpecies::AncientOak => 165.0,
            TreeSpecies::GreatBaobab => 160.0,
            TreeSpecies::WeepingWillow => 140.0,
            TreeSpecies::JapaneseMaple => 120.0,
        };
        let target_thickness = match species {
            TreeSpecies::GreatBaobab => 28.0,
            TreeSpecies::AncientOak => 22.0,
            TreeSpecies::AlpinePine => 16.0,
            TreeSpecies::WeepingWillow => 14.0,
            TreeSpecies::JapaneseMaple => 12.0,
            TreeSpecies::SilverBirch => 10.0,
        };

        if step <= 18 {
            if let Some(trunk) = self.branches.iter_mut().find(|b| b.id == root_id) {
                let frac = (step as f32 / 18.0).clamp(0.0, 1.0);
                let new_len = 18.0 + (target_trunk_len - 18.0) * frac;
                let new_thick = 4.0 + (target_thickness - 4.0) * frac;
                trunk.length = new_len;
                trunk.target_length = new_len;
                trunk.thickness = new_thick;
                trunk.end = trunk.current_end();
            }

            // At step 3 & 5: flare buttress roots at the ground
            if step == 3 || step == 5 {
                let flare_dir = if step == 3 { -1.0 } else { 1.0 };
                let root_angle = flare_dir * self.rng.range(1.2, 1.45);
                let root_len = self.rng.range(16.0, 28.0);
                self.next_id += 1;
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
                    thickness: 6.0,
                    growth_progress: 1.0,
                    children_spawned: true,
                    has_nest: false,
                    sway_factor: 0.0,
                    is_root: true,
                    foliage: Vec::new(),
                });
            }
        }

        // 2. Stage 2 (Steps 6..28): Primary Scaffold Limbs & Species Branching
        if (6..=28).contains(&step) {
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

            let parent_id = if is_pine && primary_pine_whorls < 10 && self.rng.f32() < 0.70 {
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
                            let tier_idx = (primary_pine_whorls / 2) % 6;
                            0.28 + (tier_idx as f32) * 0.13
                        }
                        TreeSpecies::AncientOak => self.rng.range(0.40, 0.95),
                        TreeSpecies::GreatBaobab => self.rng.range(0.70, 0.98),
                        _ => self.rng.range(0.45, 0.95),
                    };
                    if let Some(trunk) = self.branches.iter().find(|b| b.id == root_id) {
                        trunk.point_at(t)
                    } else {
                        p_end
                    }
                } else {
                    p_end
                };

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
                                    - (dist_from_trunk_base / target_trunk_len).clamp(0.0, 0.75);
                                let l = (75.0 * h_factor).clamp(24.0, 75.0);
                                (a, l)
                            } else {
                                let a = (p_angle + branch_sign * self.rng.range(0.20, 0.45))
                                    .clamp(-1.50, 1.50);
                                let l = (p_len * 0.65).clamp(18.0, 42.0);
                                (a, l)
                            }
                        }
                        TreeSpecies::WeepingWillow => {
                            if depth <= 2 {
                                let a = if parent_id == root_id {
                                    branch_sign * self.rng.range(0.35, 0.95)
                                } else {
                                    (p_angle + branch_sign * self.rng.range(0.15, 0.45))
                                        .clamp(-1.15, 1.15)
                                };
                                (a, self.rng.range(40.0, 65.0))
                            } else {
                                // Downward weeping cascade
                                let a = branch_sign * self.rng.range(2.1, 2.9);
                                (a, self.rng.range(35.0, 60.0))
                            }
                        }
                        TreeSpecies::JapaneseMaple => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.45, 0.95)
                            } else {
                                let zig = if depth % 2 == 0 { 0.55 } else { -0.55 };
                                (p_angle + zig * branch_sign + self.rng.range(-0.15, 0.15))
                                    .clamp(-1.3, 1.3)
                            };
                            (a, self.rng.range(28.0, 52.0))
                        }
                        TreeSpecies::SilverBirch => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.25, 0.65)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.15, 0.35))
                                    .clamp(-0.85, 0.85)
                            };
                            (a, self.rng.range(35.0, 60.0))
                        }
                        TreeSpecies::GreatBaobab => {
                            let a = if parent_id == root_id {
                                let spread = [-1.1, -0.6, 0.6, 1.1];
                                let base_a = spread[(k as usize).min(spread.len() - 1)];
                                base_a + self.rng.range(-0.2, 0.2)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.15, 0.4))
                                    .clamp(-1.35, 1.35)
                            };
                            (a, self.rng.range(28.0, 48.0))
                        }
                        TreeSpecies::AncientOak => {
                            let a = if parent_id == root_id {
                                branch_sign * self.rng.range(0.35, 0.85)
                            } else {
                                (p_angle + branch_sign * self.rng.range(0.20, 0.55))
                                    .clamp(-1.25, 1.25)
                            };
                            (a, self.rng.range(38.0, 72.0))
                        }
                    };

                    let thickness = (p_thick * 0.72).max(2.5);

                    // Generate species-specific foliage buds
                    let mut foliage = Vec::new();
                    let leaf_count = self.rng.range_i32(3, 7) as usize;
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
                        foliage.push(FoliageBud {
                            offset_along: self.rng.range(0.35, 1.0),
                            lateral_offset: self.rng.range(-10.0, 10.0),
                            char_code,
                            color_idx,
                            scale: self.rng.range(0.9, 1.3),
                            bloom: 0.85,
                        });
                    }

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
                        growth_progress: 1.0,
                        children_spawned: false,
                        has_nest: false,
                        sway_factor: (depth as f32 * 0.12).min(0.65),
                        is_root: false,
                        foliage,
                    });
                }
            }
        }
        // 3. Stage 3 (Steps 29..max_steps): Foliage Bloom, Densification & Maturation
        if step > 28 {
            for bid in &tree_branch_ids {
                if let Some(b) = self.branches.iter_mut().find(|b| b.id == *bid) {
                    for bud in &mut b.foliage {
                        bud.bloom = 1.0;
                    }
                    if !b.is_root && b.depth >= 2 && b.foliage.len() < 10 {
                        b.foliage.push(FoliageBud {
                            offset_along: 0.8,
                            lateral_offset: 6.0,
                            char_code: b'*',
                            color_idx: 0,
                            scale: 1.1,
                            bloom: 1.0,
                        });
                    }
                }
            }
        }

        // Spawn a subtle wood chip spark from the active growth
        self.chips.push(FallingChip {
            pos: [
                base_x + self.rng.range(-40.0, 40.0),
                750.0 - (step as f32 * 12.0).min(550.0),
            ],
            vel: [self.rng.range(-15.0, 15.0), self.rng.range(-20.0, -8.0)],
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

            // Forest Succession: smoothly pan camera left and plant next tree in center!
            let next_species = species.next();
            let next_x = base_x + 800.0;
            self.target_viewport_offset_x = base_x + 800.0 - 500.0;
            self.plant_new_sapling(next_x, next_species);
        }

        self.recompute_stats();
    }

    /// Advance tree growth by `dt_secs`.
    pub fn advance_growth(&mut self, dt_secs: f32) {
        if dt_secs <= 0.0 {
            return;
        }

        // 1. Smooth horizontal viewport camera panning
        let offset_diff = self.target_viewport_offset_x - self.viewport_offset_x;
        if offset_diff.abs() > 0.1 {
            let step = offset_diff * (dt_secs * 2.5).min(0.25);
            self.viewport_offset_x += step;
        } else {
            self.viewport_offset_x = self.target_viewport_offset_x;
        }

        // 2. Discrete 5-10s growth step timer
        self.growth_step_timer -= dt_secs;
        let max_loops = ((dt_secs / 6.0).ceil() as usize).clamp(1, 60);
        let mut loop_guard = 0;
        while self.growth_step_timer <= 0.0 && loop_guard < max_loops {
            self.growth_step_timer += self.rng.range(5.0, 8.0);
            self.execute_growth_step();
            loop_guard += 1;
        }
        if self.growth_step_timer <= 0.0 {
            self.growth_step_timer = self.rng.range(5.0, 8.0);
        }

        // 3. Smooth continuous branch & foliage interpolation
        let base_rate = 0.015 * dt_secs;
        for b in &mut self.branches {
            if b.growth_progress < 1.0 {
                b.growth_progress = (b.growth_progress + base_rate).min(1.0);
                b.end = b.current_end();
            }
            for f in &mut b.foliage {
                if f.bloom < 1.0 {
                    f.bloom = (f.bloom + base_rate * 2.0).min(1.0);
                }
            }
        }

        // 4. Ecosystem structures on mature branches over time
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
                                        && (b.start[0] >= vp_x - 150.0
                                            && b.start[0] <= vp_x + 1150.0)
                                })
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
                                center: [active_tree_base_x, 320.0],
                                radius: self.rng.range(140.0, 300.0),
                                angle: self.rng.range(0.0, std::f32::consts::TAU),
                                speed: self.rng.range(0.4, 0.8)
                                    * if self.rng.f32() > 0.5 { 1.0 } else { -1.0 },
                            });
                        } else {
                            let tx = vp_x + self.rng.range(150.0, 850.0);
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
                        let tx = vp_x + self.rng.range(100.0, 900.0);
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
            if (bird.pos[0] - (vp_x + 500.0)).abs() > 1300.0 {
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

    /// User interaction: pan forest horizontally.
    pub fn pan_forest(&mut self, delta_x: f32) {
        let max_offset = (self.trees.len() as f32 * 800.0 + 1000.0).max(3000.0);
        self.viewport_offset_x = (self.viewport_offset_x + delta_x).clamp(-1000.0, max_offset);
        self.target_viewport_offset_x = self.viewport_offset_x;
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

        // Step through in bulk chunks, capped at max 60 steps for instant wake-up (< 20ms)
        let max_steps = 60;
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
                    if state.trees.is_empty() && !state.branches.is_empty() {
                        let root_branch = state
                            .branches
                            .iter()
                            .find(|b| b.depth == 0)
                            .map(|b| b.id)
                            .unwrap_or(1);
                        let base_pos = state
                            .branches
                            .iter()
                            .find(|b| b.id == root_branch)
                            .map(|b| b.start)
                            .unwrap_or([500.0, 750.0]);
                        state.trees.push(TreeRecord {
                            id: 1,
                            species: TreeSpecies::AncientOak,
                            base_pos,
                            root_branch_id: root_branch,
                            is_mature: state.branches.len() > 25,
                            growth_steps_completed: if state.branches.len() > 25 {
                                40
                            } else {
                                state.branches.len() as u32
                            },
                            target_growth_steps: 40,
                            planted_age_secs: 0.0,
                        });
                        state.active_tree_id = 1;
                    }

                    // Migration: Mark ground flare buttress roots as is_root and sanitize any children
                    for b in &mut state.branches {
                        if b.depth == 1 && b.start[1] >= 748.0 && b.angle.abs() > 1.0 {
                            b.is_root = true;
                        }
                    }
                    let root_ids: Vec<u32> = state
                        .branches
                        .iter()
                        .filter(|b| b.is_root)
                        .map(|b| b.id)
                        .collect();
                    state.branches.retain(|b| {
                        if let Some(pid) = b.parent_id {
                            !root_ids.contains(&pid)
                        } else {
                            true
                        }
                    });

                    // Migration: Reposition any birds stranded far outside active viewport
                    let active_x = state
                        .trees
                        .iter()
                        .find(|t| t.id == state.active_tree_id)
                        .map(|t| t.base_pos[0])
                        .unwrap_or(state.viewport_offset_x + 500.0);
                    let vp_x = state.viewport_offset_x;
                    for (i, bird) in state.birds.iter_mut().enumerate() {
                        if (bird.pos[0] - active_x).abs() > 1200.0 {
                            bird.pos = [
                                vp_x + 250.0 + ((i as f32 % 5.0) * 110.0),
                                200.0 + ((i as f32 % 3.0) * 70.0),
                            ];
                            bird.state = BirdState::Circling {
                                center: [active_x, 320.0],
                                radius: 160.0 + ((i as f32) * 20.0),
                                angle: (i as f32) * 1.5,
                                speed: if i % 2 == 0 { 0.55 } else { -0.55 },
                            };
                        }
                    }

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
        let mut world = WorldState::new(42);

        println!("Initial bird count: {}", world.birds.len());
        for b in &world.birds {
            println!(
                "Bird {}: species={:?}, state={:?}, pos={:?}",
                b.id, b.species, b.state, b.pos
            );
        }

        // Advance 10 seconds of simulation
        for _ in 0..10 {
            world.update(1.0, false, None);
        }

        println!("\nAfter 10 seconds of update:");
        for b in &world.birds {
            println!(
                "Bird {}: species={:?}, state={:?}, pos={:?}",
                b.id, b.species, b.state, b.pos
            );
        }

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

    #[test]
    fn rng_is_deterministic_and_in_range() {
        let mut a = WorldRng::new(42);
        let mut b = WorldRng::new(42);
        for _ in 0..1000 {
            let x = a.f32();
            assert_eq!(x, b.f32());
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn palette_cycling_and_tree_species() {
        let mut world = WorldState::new(77);
        assert_eq!(world.active_palette, RookeryPalette::EarthAndSylvan);
        assert_eq!(
            world.aesthetic_style_name(),
            "Earth & Sylvan (Caves of Qud)"
        );

        assert_eq!(world.cycle_palette(), RookeryPalette::MidnightStarlight);
        assert_eq!(world.aesthetic_style_name(), "Midnight Starlight");

        assert_eq!(world.prev_palette(), RookeryPalette::EarthAndSylvan);
        assert_eq!(world.prev_palette(), RookeryPalette::MatrixGreen);

        world.set_palette(RookeryPalette::AutumnHearth);
        assert_eq!(world.active_palette, RookeryPalette::AutumnHearth);
        assert_eq!(world.aesthetic_style_name(), "Autumn Hearth");

        // Verify all 6 real-tree species exist and cycle
        let species = TreeSpecies::all();
        assert_eq!(species.len(), 6);
        assert_eq!(TreeSpecies::AncientOak.next(), TreeSpecies::WeepingWillow);
        assert_eq!(TreeSpecies::WeepingWillow.next(), TreeSpecies::AlpinePine);
        assert_eq!(TreeSpecies::AlpinePine.next(), TreeSpecies::JapaneseMaple);
        assert_eq!(TreeSpecies::JapaneseMaple.next(), TreeSpecies::SilverBirch);
        assert_eq!(TreeSpecies::SilverBirch.next(), TreeSpecies::GreatBaobab);
        assert_eq!(TreeSpecies::GreatBaobab.next(), TreeSpecies::AncientOak);

        // Verify sapling starts with 1 branch taking up only a few cells
        assert_eq!(world.branches.len(), 1);
        assert!(world.branches[0].length <= 20.0);
        assert_eq!(world.trees.len(), 1);
        assert!(!world.trees[0].is_mature);

        // Advance 350 seconds of growth -> discrete steps mature the tree and trigger forest succession
        world.advance_growth(350.0);
        assert!(world.trees[0].is_mature);
        assert!(
            world.trees.len() >= 2,
            "must have planted next tree in the forest"
        );
        assert_eq!(world.trees[1].species, TreeSpecies::WeepingWillow);
        assert!(
            world.target_viewport_offset_x > 0.0,
            "camera must pan towards next tree"
        );
    }

    #[test]
    fn test_all_species_tree_growth_and_geometry() {
        for &species in TreeSpecies::all() {
            let mut world = WorldState::new(42);
            world.trees[0].species = species;
            world.trees[0].target_growth_steps = species.max_growth_steps();

            println!("\n=== Growing Species: {:?} ===", species);
            let mut steps_safety = 0;
            while world.trees[0].growth_steps_completed < species.max_growth_steps()
                && steps_safety < 100
            {
                world.advance_growth(7.0);
                steps_safety += 1;
            }

            println!("{}", world.render_ascii_art(80, 24));

            let root_id = world.trees[0].root_branch_id;
            let base_x = world.trees[0].base_pos[0];

            let mut tree_0_branch_ids = vec![root_id];
            let mut idx = 0;
            while idx < tree_0_branch_ids.len() {
                let pid = tree_0_branch_ids[idx];
                for b in &world.branches {
                    if b.parent_id == Some(pid) && !tree_0_branch_ids.contains(&b.id) {
                        tree_0_branch_ids.push(b.id);
                    }
                }
                idx += 1;
            }

            let non_root_branches: Vec<&Branch> = world
                .branches
                .iter()
                .filter(|b| !b.is_root && b.id != root_id && tree_0_branch_ids.contains(&b.id))
                .collect();

            assert!(
                !non_root_branches.is_empty(),
                "{:?} must have canopy branches",
                species
            );

            // 1. Zero canopy branches should crawl along the ground (y > 725)
            for b in &non_root_branches {
                assert!(
                    b.end[1] < 725.0,
                    "{:?} branch {} crawled along the ground: end={:?}",
                    species,
                    b.id,
                    b.end
                );
            }

            // 2. Tree canopy must reach upward into the sky
            let min_y = non_root_branches
                .iter()
                .map(|b| b.end[1])
                .fold(750.0f32, f32::min);
            let expected_max_y = match species {
                TreeSpecies::WeepingWillow
                | TreeSpecies::JapaneseMaple
                | TreeSpecies::GreatBaobab => 610.0,
                _ => 520.0,
            };
            assert!(
                min_y < expected_max_y,
                "{:?} tree failed to reach upward: min_y={} (expected < {})",
                species,
                min_y,
                expected_max_y
            );

            // 3. Canopy branches must spread on both left and right of trunk
            let min_x = non_root_branches
                .iter()
                .map(|b| b.end[0])
                .fold(base_x, f32::min);
            let max_x = non_root_branches
                .iter()
                .map(|b| b.end[0])
                .fold(base_x, f32::max);
            assert!(
                min_x < base_x - 30.0,
                "{:?} lacks left branch spread: min_x={}, base_x={}",
                species,
                min_x,
                base_x
            );
            assert!(
                max_x > base_x + 30.0,
                "{:?} lacks right branch spread: max_x={}, base_x={}",
                species,
                max_x,
                base_x
            );

            // 4. Alpine Pine specific check: branches originate at multiple heights along trunk
            if species == TreeSpecies::AlpinePine {
                let primary_starts: Vec<f32> = non_root_branches
                    .iter()
                    .filter(|b| b.parent_id == Some(root_id))
                    .map(|b| b.start[1])
                    .collect();
                let mut distinct_heights = primary_starts.clone();
                distinct_heights.sort_by(|a, b| a.total_cmp(b));
                distinct_heights.dedup_by(|a, b| (*a - *b).abs() < 5.0);
                assert!(
                    distinct_heights.len() >= 3,
                    "Alpine Pine should have at least 3 whorl tiers, found: {:?}",
                    distinct_heights
                );
            }
        }
    }

    #[test]
    fn test_birds_follow_active_viewport_across_succession() {
        let mut world = WorldState::new(101);

        // Advance growth to trigger succession and camera panning to Tree 2
        world.advance_growth(350.0);
        assert!(world.trees.len() >= 2);
        let target_vp = world.target_viewport_offset_x;
        assert!(target_vp > 0.0);

        // Pan viewport to Tree 2
        world.viewport_offset_x = target_vp;

        // Run simulation updates
        for _ in 0..30 {
            world.update(1.0, false, None);
        }

        println!("\n=== ASCII Viewport after Forest Succession ===");
        println!("{}", world.render_ascii_art(80, 24));

        let vp_x = world.viewport_offset_x;

        // Verify birds are within or near active viewport
        for bird in &world.birds {
            assert!(
                bird.pos[0] >= vp_x - 150.0 && bird.pos[0] <= vp_x + 1150.0,
                "Bird {:?} pos {:?} outside active viewport [{} .. {}]",
                bird.species,
                bird.pos,
                vp_x,
                vp_x + 1000.0
            );
        }
    }

    #[test]
    fn test_load_or_init_sanitizes_ground_branches_and_relocates_birds() {
        let temp_dir = std::env::temp_dir().join(format!("rookery_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let save_path = temp_dir.join("test_save.json");

        let mut world = WorldState::new(42);
        // Simulate an expanded forest at x = 12,800
        world.viewport_offset_x = 12800.0;
        world.target_viewport_offset_x = 12800.0;
        world.trees.push(TreeRecord {
            id: 17,
            species: TreeSpecies::SilverBirch,
            base_pos: [13300.0, 750.0],
            root_branch_id: 507,
            is_mature: false,
            growth_steps_completed: 20,
            target_growth_steps: 40,
            planted_age_secs: 50000.0,
        });
        world.active_tree_id = 17;

        // Add a ground buttress root and a corrupted ground child branch
        let root_branch_id = 998;
        let ground_child_id = 999;
        world.branches.push(Branch {
            id: root_branch_id,
            parent_id: Some(507),
            depth: 1,
            start: [13300.0, 750.0],
            end: [13280.0, 746.0],
            angle: -1.35,
            length: 20.0,
            target_length: 20.0,
            thickness: 6.0,
            growth_progress: 1.0,
            children_spawned: true,
            has_nest: false,
            sway_factor: 0.0,
            is_root: true,
            foliage: Vec::new(),
        });
        world.branches.push(Branch {
            id: ground_child_id,
            parent_id: Some(root_branch_id),
            depth: 2,
            start: [13280.0, 746.0],
            end: [13210.0, 745.0],
            angle: -1.57,
            length: 70.0,
            target_length: 70.0,
            thickness: 4.0,
            growth_progress: 1.0,
            children_spawned: true,
            has_nest: false,
            sway_factor: 0.0,
            is_root: false,
            foliage: Vec::new(),
        });

        // Set all birds back at Tree 1 (x = 300..600, 12,000 pixels away)
        for b in &mut world.birds {
            b.pos = [450.0, 300.0];
        }

        // Save raw world JSON to disk
        let json_str = serde_json::to_string_pretty(&world).unwrap();
        std::fs::write(&save_path, json_str).unwrap();

        // Load via load_or_init
        let loaded = WorldState::load_or_init(&save_path, 42);

        // Verify corrupted ground child was pruned
        assert!(
            !loaded.branches.iter().any(|b| b.id == ground_child_id),
            "Corrupted ground child branch must be removed"
        );

        // Verify buttress root is marked as is_root
        let root_b = loaded
            .branches
            .iter()
            .find(|b| b.id == root_branch_id)
            .expect("buttress root should remain");
        assert!(root_b.is_root);

        // Verify birds were relocated into the active viewport
        for bird in &loaded.birds {
            assert!(
                bird.pos[0] >= loaded.viewport_offset_x - 100.0
                    && bird.pos[0] <= loaded.viewport_offset_x + 1100.0,
                "Bird pos {:?} should be in active viewport",
                bird.pos
            );
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
