//! Data structures, enums, and constants for the Living ASCII Rookery world simulation.

use serde::{Deserialize, Serialize};

pub const WORLD_STATE_VERSION: u32 = 1;
pub const WORLD_STATE_FILE_NAME: &str = "rookery_world.json";

/// Standard horizontal distance between successive trees in the living forest.
/// Tuned so that after camera succession pans to the new tree, approximately ~35% of the
/// previous mature tree's eastern canopy and boughs remain gracefully visible framing the left viewport,
/// forming an unbroken, continuous woodland canopy.
pub const FOREST_TREE_SPACING: f32 = 540.0;

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
    ApproachingPerch {
        branch_id: u32,
        t: f32,
        speed: f32,
    },
    Perched {
        branch_id: u32,
        t: f32,
        remaining_secs: f32,
    },
    ApproachingFeed {
        branch_id: u32,
        t: f32,
        speed: f32,
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
    ApproachingNest {
        nest_id: u32,
        is_brooding: bool,
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
    ApproachingBath {
        bath_id: u32,
        speed: f32,
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

/// Woodland fauna species roaming the forest floor and lower boughs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaunaSpecies {
    /// Curious Red Fox roaming the forest floor.
    Fox,
    /// Nimble Squirrel foraging on ground and scampering up the lower trunk.
    Squirrel,
    /// Sturdy European Badger foraging around root buttresses.
    Badger,
    /// Gentle prickly Hedgehog snuffling among fallen leaves.
    Hedgehog,
    /// Gentle wild Cottontail Rabbit hopping across the clover meadow.
    Rabbit,
}

impl FaunaSpecies {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Fox => "Red Fox",
            Self::Squirrel => "Squirrel",
            Self::Badger => "Badger",
            Self::Hedgehog => "Hedgehog",
            Self::Rabbit => "Cottontail Rabbit",
        }
    }

    pub fn base_speed(&self) -> f32 {
        match self {
            Self::Fox => 36.0,
            Self::Squirrel => 44.0,
            Self::Badger => 24.0,
            Self::Hedgehog => 16.0,
            Self::Rabbit => 32.0,
        }
    }
}

/// Autonomous behavioral states for forest floor fauna.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FaunaState {
    /// Trotting smoothly across the meadow ground plane toward a 2D destination.
    Trotting { target: [f32; 2], speed: f32 },
    /// Pausing to sniff the soil, clovers, or mushroom caps.
    Sniffing { remaining_secs: f32 },
    /// Foraging for fallen snacks or acorns near buttress roots or across grass.
    Foraging {
        remaining_secs: f32,
        target_snack: Option<[f32; 2]>,
    },
    /// Resting peacefully under branch shelter.
    Resting { remaining_secs: f32 },
    /// Deep sleep curled up near tree trunk.
    Sleeping { remaining_secs: f32 },
    /// Alert posture upon sensing motion or sounds.
    Alert { remaining_secs: f32 },
    /// Startled flee when direct mode cursor clicks nearby.
    Startled { remaining_secs: f32, dir: [f32; 2] },
    /// Scampering vertically up or down the lower trunk (Squirrel).
    ClimbingTrunk {
        branch_id: u32,
        height: f32,
        speed: f32,
        ascending: bool,
    },
    /// Sitting upright to groom ears, face, or tail (Rabbit, Squirrel).
    Grooming { remaining_secs: f32 },
    /// Stalking low and deliberately toward a snack or low-perched bird (Fox).
    Prowling { target: [f32; 2], speed: f32 },
    /// Short playful or curious arc leap (Fox).
    Pouncing { remaining_secs: f32, dir: [f32; 2] },
    /// Rooting/digging around tree buttresses or caching nuts (Badger, Squirrel).
    Digging { remaining_secs: f32 },
    /// Curled into a tight defensive spiky ball (Hedgehog).
    Curled { remaining_secs: f32 },
}

/// Autonomous procedural woodland creature in the world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WoodlandFauna {
    pub id: u32,
    pub species: FaunaSpecies,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub facing_left: bool,
    pub state: FaunaState,
    pub anim_phase: f32,
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
    GiantRedwood,
    CherryBlossom,
    AncientBanyan,
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
            Self::GiantRedwood => "Giant Redwood",
            Self::CherryBlossom => "Cherry Blossom",
            Self::AncientBanyan => "Ancient Banyan",
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
            Self::GiantRedwood,
            Self::CherryBlossom,
            Self::AncientBanyan,
        ]
    }

    pub fn next(&self) -> Self {
        match self {
            Self::AncientOak => Self::WeepingWillow,
            Self::WeepingWillow => Self::AlpinePine,
            Self::AlpinePine => Self::JapaneseMaple,
            Self::JapaneseMaple => Self::SilverBirch,
            Self::SilverBirch => Self::GreatBaobab,
            Self::GreatBaobab => Self::GiantRedwood,
            Self::GiantRedwood => Self::CherryBlossom,
            Self::CherryBlossom => Self::AncientBanyan,
            Self::AncientBanyan => Self::AncientOak,
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
            Self::GiantRedwood => 46,
            Self::CherryBlossom => 38,
            Self::AncientBanyan => 44,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "ancientoak" | "oak" => Some(Self::AncientOak),
            "weepingwillow" | "willow" => Some(Self::WeepingWillow),
            "alpinepine" | "pine" => Some(Self::AlpinePine),
            "japanesemaple" | "maple" => Some(Self::JapaneseMaple),
            "silverbirch" | "birch" => Some(Self::SilverBirch),
            "greatbaobab" | "baobab" => Some(Self::GreatBaobab),
            "giantredwood" | "redwood" | "sequoia" => Some(Self::GiantRedwood),
            "cherryblossom" | "cherry" | "sakura" => Some(Self::CherryBlossom),
            "ancientbanyan" | "banyan" => Some(Self::AncientBanyan),
            _ => None,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::AncientOak => "ancient_oak",
            Self::WeepingWillow => "weeping_willow",
            Self::AlpinePine => "alpine_pine",
            Self::JapaneseMaple => "japanese_maple",
            Self::SilverBirch => "silver_birch",
            Self::GreatBaobab => "great_baobab",
            Self::GiantRedwood => "giant_redwood",
            Self::CherryBlossom => "cherry_blossom",
            Self::AncientBanyan => "ancient_banyan",
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
    pub fn id(&self) -> &'static str {
        match self {
            Self::Classic3Cell => "classic",
            Self::Streamlined2Cell => "streamlined",
            Self::OrnateFlock => "ornate",
        }
    }

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
    #[serde(default)]
    pub seed: u64,
}

/// Summary snapshot of an individual tree for UI inspection and debugging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeSummary {
    pub id: u32,
    pub species: String,
    pub species_code: String,
    pub is_mature: bool,
    pub growth_steps: u32,
    pub target_steps: u32,
    pub base_x: f32,
    pub seed: u64,
    pub branch_count: u32,
    pub foliage_count: u32,
}

pub(crate) fn default_growth_step_timer() -> f32 {
    6.0
}

pub(crate) fn default_sim_speed() -> f32 {
    1.0
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
