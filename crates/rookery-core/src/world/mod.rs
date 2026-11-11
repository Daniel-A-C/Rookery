//! Deterministic simulation state for *The Living ASCII Rookery* (Milestone 2).
//!
//! Owns procedural tree growth, autonomous bird flocking/perching, micro-interactions
//! (pruning, seeding, snacks), state persistence, and offline time-delta catch-up.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Activity;

pub mod types;
pub use types::*;

pub mod ascii_art;
pub mod birds;
pub mod growth;
pub mod persistence;

#[cfg(test)]
mod tests;

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
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default = "default_sim_speed")]
    pub sim_speed: f32,
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
            seed,
        };

        let mut birds = Vec::new();
        let species_pool = [
            BirdSpecies::Rook,
            BirdSpecies::AzureJay,
            BirdSpecies::Goldfinch,
            BirdSpecies::Robin,
            BirdSpecies::EmeraldDove,
        ];

        // Seed 4 initial birds circling gracefully around the ancestral tree
        for i in 0..4 {
            let sp = species_pool[(i as usize) % species_pool.len()];
            let _dummy_x = rng.range(200.0, 800.0);
            let _dummy_y = rng.range(150.0, 450.0);
            let _dummy_speed = rng.range(35.0, 75.0);
            let center = [500.0, 350.0];
            let radius = rng.range(120.0, 260.0);
            let angle = rng.range(0.0, std::f32::consts::TAU);
            let speed = rng.range(0.3, 0.7) * if i % 2 == 0 { 1.0 } else { -1.0 };
            let x = center[0] + angle.cos() * radius;
            let y = center[1] + angle.sin() * radius * 0.45;
            birds.push(Bird {
                id: i + 1,
                species: sp,
                pos: [x, y],
                vel: [
                    -speed * angle.sin() * radius,
                    speed * angle.cos() * radius * 0.45,
                ],
                facing_left: speed < 0.0,
                state: BirdState::Circling {
                    center,
                    radius,
                    angle,
                    speed,
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
            debug_mode: false,
            sim_speed: 1.0,
        };

        world.recompute_stats();
        world
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
                    | BirdState::ApproachingPerch { .. }
                    | BirdState::ApproachingBath { .. }
                    | BirdState::ApproachingFeed { .. }
                    | BirdState::ApproachingNest { .. }
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

    /// User interaction: pan forest horizontally.
    pub fn pan_forest(&mut self, delta_x: f32) {
        let max_offset = (self.trees.len() as f32 * FOREST_TREE_SPACING + 1000.0).max(3000.0);
        self.viewport_offset_x = (self.viewport_offset_x + delta_x).clamp(-1000.0, max_offset);
        self.target_viewport_offset_x = self.viewport_offset_x;
    }
}

pub(crate) fn pointer_dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    (dx * dx + dy * dy).sqrt()
}
