//! The Living ASCII Rookery: Strict ANSI Roguelike Engine & Forest Succession.
//!
//! Monospace character-cell woodland rendering in the tradition of Caves of Qud and Dwarf Fortress.
//! Features 2-3 cell bird sprites, real-tree species architectures (Oak, Willow, Pine, Maple, Birch, Baobab),
//! atmospheric celestial skies with twinkling starlight, curated ANSI palettes, and continuous forest succession.

use std::sync::{Arc, Mutex};

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::world::{
    BirdSpecies, BirdState, FaunaSpecies, FaunaState, RookeryPalette, SnackType, WorldState,
};

pub use crate::atlas::{GlyphInstance, GlyphPass};
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 65_536;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BgUniforms {
    resolution: [f32; 2],
    time: f32,
    palette_id: f32,
}

pub struct LivingRookeryScene {
    pub world: Arc<Mutex<WorldState>>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,
    interactive_mode: bool,

    bg_pipeline: wgpu::RenderPipeline,
    bg_uniforms_buf: wgpu::Buffer,
    bg_bind_group: wgpu::BindGroup,

    glyphs: GlyphPass,
    pub audio: Option<crate::audio::AudioEngine>,

    drag_start_screen: Option<Pointer>,
    last_screen_pointer: Option<Pointer>,
    is_dragging_panning: bool,
    drag_distance_accum: f32,
    snack_click_count: u32,
}

impl LivingRookeryScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, world: Arc<Mutex<WorldState>>) -> Self {
        let device = &gpu.device;

        // 1. Fullscreen background setup
        let bg_source = include_str!("ascii_bg.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "ascii-bg-layout");
        let bg_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ascii-bg-uniforms"),
            size: std::mem::size_of::<BgUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ascii-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms_buf.as_entire_binding(),
            }],
        });
        let bg_pipeline = fullscreen::pipeline(
            device,
            "ascii-bg-pipeline",
            bg_source,
            format,
            Some(&bg_layout),
            None,
        );

        let glyphs = GlyphPass::new(gpu, format, MAX_INSTANCES);

        Self {
            world,
            width: 1920.0,
            height: 1080.0,
            scale_factor: 1.0,
            time: 0.0,
            interactive_mode: false,
            bg_pipeline,
            bg_uniforms_buf,
            bg_bind_group,
            glyphs,
            audio: None,
            drag_start_screen: None,
            last_screen_pointer: None,
            is_dragging_panning: false,
            drag_distance_accum: 0.0,
            snack_click_count: 0,
        }
    }

    pub fn with_audio(mut self, audio: Option<crate::audio::AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    /// Converts normalized world coordinates [0..1000, 0..800] to current screen viewport pixels.
    fn world_to_screen(&self, pt: [f32; 2], offset_x: f32) -> [f32; 2] {
        let x = ((pt[0] - offset_x) / 1000.0) * self.width;
        let ground_y = self.height * 0.88;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let y = ground_y - (750.0 - pt[1]) * tree_scale;
        [x, y]
    }

    /// Converts screen pixel pointer coordinates to world coordinates.
    fn screen_to_world(&self, pt: Pointer) -> [f32; 2] {
        let offset_x = self
            .world
            .lock()
            .map(|w| w.viewport_offset_x)
            .unwrap_or(0.0);
        let x = (pt.x / self.width) * 1000.0 + offset_x;
        let ground_y = self.height * 0.88;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let y = 750.0 - (ground_y - pt.y) / tree_scale;
        [x, y]
    }

    #[allow(clippy::too_many_arguments)]
    fn push_glyph(
        &mut self,
        c: char,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        z_layer: f32,
        color: [f32; 4],
        sway_factor: f32,
    ) {
        self.glyphs
            .push(c, pos, size, rotation, z_layer, color, sway_factor);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_text(
        &mut self,
        text: &str,
        start_pos: [f32; 2],
        char_size: [f32; 2],
        advance_x: f32,
        z_layer: f32,
        color: [f32; 4],
        sway_factor: f32,
    ) {
        for (i, c) in text.chars().enumerate() {
            let pos = [start_pos[0] + i as f32 * advance_x, start_pos[1]];
            self.push_glyph(c, pos, char_size, 0.0, z_layer, color, sway_factor);
        }
    }

    fn species_color(sp: BirdSpecies) -> [f32; 4] {
        match sp {
            BirdSpecies::Rook => [0.72, 0.75, 0.82, 0.98],
            BirdSpecies::AzureJay => [0.28, 0.72, 0.98, 0.98],
            BirdSpecies::Goldfinch => [0.98, 0.86, 0.22, 0.98],
            BirdSpecies::Robin => [0.98, 0.42, 0.28, 0.98],
            BirdSpecies::EmeraldDove => [0.35, 0.88, 0.58, 0.98],
            BirdSpecies::CelestialRaven => [0.55, 0.88, 1.0, 0.98],
            BirdSpecies::RubyTanager => [0.98, 0.25, 0.28, 0.98],
            BirdSpecies::SnowyOwl => [0.95, 0.98, 1.0, 0.98],
        }
    }

    fn generate_scene_instances(&mut self) {
        self.glyphs.clear();
        let world_snapshot = match self.world.lock() {
            Ok(w) => w.clone(),
            Err(_) => return,
        };

        self.generate_roguelike_scene(&world_snapshot);
    }

    // =========================================================================
    // STRICT ANSI ROGUELIKE ENGINE (Caves of Qud / Dwarf Fortress Aesthetic)
    // =========================================================================
    fn generate_roguelike_scene(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        // Refined character cell sizing for higher grid density (~50% more cells)
        let cell_w = 9.6f32;
        let cell_h = 16.0f32;
        let cell_size = [cell_w, cell_h];
        let cols = (self.width / cell_w).floor() as i32;
        let rows = (self.height / cell_h).floor() as i32;
        let ground_row = ((self.height * 0.88) / cell_h).floor() as i32;

        let snap = |pt: [f32; 2]| -> [f32; 2] {
            let c = (pt[0] / cell_w).floor();
            let r = (pt[1] / cell_h).floor();
            [(c + 0.5) * cell_w, (r + 0.5) * cell_h]
        };
        let cell_pos =
            |c: i32, r: i32| -> [f32; 2] { [(c as f32 + 0.5) * cell_w, (r as f32 + 0.5) * cell_h] };

        let palette = world_guard.active_palette;

        // Palette color tokens:
        let (turf_color, soil_main, soil_crystal, border_color, star_color) = match palette {
            RookeryPalette::MidnightStarlight => (
                [0.45, 0.72, 0.95, 0.95],
                [0.22, 0.28, 0.38, 0.75],
                [0.65, 0.88, 1.0, 0.95],
                [0.45, 0.75, 0.95, 0.92],
                [0.85, 0.92, 1.0, 0.85],
            ),
            RookeryPalette::AutumnHearth => (
                [0.82, 0.52, 0.22, 0.95],
                [0.48, 0.28, 0.18, 0.75],
                [0.98, 0.75, 0.25, 0.95],
                [0.88, 0.55, 0.22, 0.92],
                [1.0, 0.88, 0.72, 0.85],
            ),
            RookeryPalette::VerdantRainforest => (
                [0.20, 0.95, 0.45, 0.95],
                [0.25, 0.35, 0.20, 0.75],
                [0.98, 0.45, 0.85, 0.95],
                [0.25, 0.88, 0.48, 0.92],
                [0.82, 0.98, 0.85, 0.85],
            ),
            RookeryPalette::AmberPhosphor => (
                [0.98, 0.75, 0.18, 0.95],
                [0.45, 0.28, 0.05, 0.75],
                [1.0, 0.88, 0.35, 0.95],
                [0.98, 0.72, 0.15, 0.92],
                [0.98, 0.82, 0.25, 0.85],
            ),
            RookeryPalette::MatrixGreen => (
                [0.25, 0.98, 0.35, 0.95],
                [0.08, 0.38, 0.15, 0.75],
                [0.55, 1.0, 0.65, 0.95],
                [0.18, 0.95, 0.28, 0.92],
                [0.45, 0.98, 0.55, 0.85],
            ),
            RookeryPalette::EarthAndSylvan => (
                [0.25, 0.85, 0.42, 0.95],
                [0.42, 0.30, 0.20, 0.65],
                [0.35, 0.85, 0.95, 0.90],
                [0.28, 0.72, 0.58, 0.92],
                [0.88, 0.92, 0.98, 0.85],
            ),
        };

        // 1. Atmospheric Celestial Sky & Twinkling Stars
        for r in 1..(ground_row - 2) {
            for c in 1..(cols - 1) {
                let star_hash =
                    (c as u32).wrapping_mul(374_761_393) ^ (r as u32).wrapping_mul(668_265_263);
                if star_hash.is_multiple_of(113) {
                    let twinkle = 0.45 + 0.55 * ((self.time as f32 * 2.2 + star_hash as f32).sin());
                    let mut col = star_color;
                    col[3] *= twinkle;
                    let star_ch = match star_hash % 3 {
                        0 => '✦',
                        1 => '*',
                        _ => '·',
                    };
                    self.push_glyph(star_ch, cell_pos(c, r), cell_size, 0.0, 0.02, col, 0.0);
                }
            }
        }

        // Celestial Crescent Moon in upper sky with glowing ambient halo
        let moon_c = (cols - 10).max(4);
        let moon_r = 3.min(rows - 1);
        let shadow_dir: i32 = if moon_c >= cols / 2 { -1 } else { 1 };
        let shadow_color = [
            turf_color[0] * 0.22,
            turf_color[1] * 0.28,
            turf_color[2] * 0.20,
            0.88,
        ];

        // Soft glowing halo in adjacent cells
        for dr in -1..=1 {
            for dc in -2..=2 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let hc = moon_c + dc;
                let hr = moon_r + dr;
                if hc >= 0 && hc < cols && hr >= 0 && hr < ground_row - 2 {
                    let halo_col = [star_color[0], star_color[1], star_color[2], 0.22];
                    self.push_glyph('·', cell_pos(hc, hr), cell_size, 0.0, 0.02, halo_col, 0.0);
                }
            }
        }

        // Celestial Moon body
        self.push_glyph(
            '☾',
            cell_pos(moon_c, moon_r),
            cell_size,
            0.0,
            0.03,
            [star_color[0], star_color[1], star_color[2], 0.95],
            0.0,
        );

        // Rare Shooting Star / Meteor streak (appears periodically in upper sky)
        let meteor_period = 22.0f32;
        let meteor_time = (self.time as f32) % meteor_period;
        let meteor_duration = 1.8f32;
        if meteor_time < meteor_duration {
            let progress = meteor_time / meteor_duration;
            let meteor_start_c = cols - 4;
            let meteor_start_r = 2;
            let meteor_end_c = (cols as f32 * 0.30) as i32;
            let meteor_end_r = 9.min(ground_row - 3);

            let cur_c = (meteor_start_c as f32 + (meteor_end_c - meteor_start_c) as f32 * progress)
                .round() as i32;
            let cur_r = (meteor_start_r as f32 + (meteor_end_r - meteor_start_r) as f32 * progress)
                .round() as i32;

            // Tail trail behind the head
            for tail_i in 1..=4 {
                let tc = cur_c + tail_i * 2;
                let tr = cur_r - (tail_i as f32 * 0.5).round() as i32;
                if tc >= 0 && tc < cols && tr >= 0 && tr < ground_row - 2 {
                    let tail_alpha = (0.70 - tail_i as f32 * 0.15).max(0.1);
                    let tail_col = [star_color[0], star_color[1], star_color[2], tail_alpha];
                    let tail_ch = if tail_i <= 2 { '─' } else { '·' };
                    self.push_glyph(
                        tail_ch,
                        cell_pos(tc, tr),
                        cell_size,
                        0.0,
                        0.03,
                        tail_col,
                        0.0,
                    );
                }
            }

            // Radiant meteor head
            if cur_c >= 0 && cur_c < cols && cur_r >= 0 && cur_r < ground_row - 2 {
                self.push_glyph(
                    '✦',
                    cell_pos(cur_c, cur_r),
                    cell_size,
                    0.0,
                    0.04,
                    [1.0, 1.0, 1.0, 0.98],
                    0.0,
                );
            }
        }

        // 1b. Atmospheric Woodland Backdrop & Midground Forest Canopy (Milestone 9)
        let (backdrop_color, midground_color) = match palette {
            RookeryPalette::MidnightStarlight => {
                ([0.10, 0.16, 0.26, 0.38], [0.16, 0.24, 0.36, 0.58])
            }
            RookeryPalette::AutumnHearth => ([0.26, 0.14, 0.08, 0.40], [0.38, 0.20, 0.12, 0.60]),
            RookeryPalette::VerdantRainforest => {
                ([0.10, 0.24, 0.14, 0.40], [0.16, 0.38, 0.20, 0.62])
            }
            RookeryPalette::AmberPhosphor => ([0.28, 0.18, 0.05, 0.38], [0.48, 0.30, 0.08, 0.58]),
            RookeryPalette::MatrixGreen => ([0.05, 0.22, 0.08, 0.38], [0.10, 0.35, 0.14, 0.58]),
            RookeryPalette::EarthAndSylvan => ([0.12, 0.20, 0.15, 0.38], [0.20, 0.34, 0.24, 0.60]),
        };

        // Layer 1: Distant Woodland Backdrop (Far Ridge / Canopy) with 0.25x Parallax (~1.5x scale)
        let backdrop_c_offset = (offset_x * 0.25) / cell_w;
        for c in 1..cols.saturating_sub(1) {
            let world_c = (c as f32) + backdrop_c_offset;
            // 1.5x broader and taller rolling mountain / pine ridge
            let ridge_wave = (world_c * 0.028).sin() * 5.4
                + (world_c * 0.058 + 1.3).sin() * 3.0
                + (world_c * 0.113 + 2.7).sin() * 1.6;
            let ridge_height = (11.0 + ridge_wave).round() as i32;
            let top_r = (ground_row - ridge_height).clamp(2, ground_row - 2);

            for r in top_r..ground_row {
                let p = cell_pos(c, r);
                if r == top_r {
                    let is_pine_peak = (world_c * 0.21).sin() > 0.55;
                    let peak_ch = if is_pine_peak { '▲' } else { '░' };
                    self.push_glyph(peak_ch, p, cell_size, 0.0, 0.032, backdrop_color, 0.0);
                } else if r == top_r + 1 {
                    self.push_glyph('░', p, cell_size, 0.0, 0.032, backdrop_color, 0.0);
                } else {
                    let fill_hash =
                        (c as u32).wrapping_mul(314_159) ^ (r as u32).wrapping_mul(271_828);
                    if fill_hash.is_multiple_of(6) {
                        self.push_glyph('░', p, cell_size, 0.0, 0.032, backdrop_color, 0.0);
                    } else if fill_hash.is_multiple_of(9) {
                        self.push_glyph('·', p, cell_size, 0.0, 0.032, backdrop_color, 0.0);
                    }
                }
            }
        }

        // Layer 2: Midground Evergreen Forest Canopy Stands with 0.55x Parallax (~1.5x scale)
        let mid_c_offset = (offset_x * 0.55) / cell_w;
        let stand_spacing = 32;
        let min_stand = ((mid_c_offset - 20.0) / stand_spacing as f32).floor() as i32;
        let max_stand = ((mid_c_offset + cols as f32 + 20.0) / stand_spacing as f32).ceil() as i32;

        let render_evergreen =
            |scene: &mut Self, root_c: i32, height_rows: i32, trunk_color: [f32; 4]| {
                let crown_top_r = (ground_row - height_rows).clamp(2, ground_row - 5);
                let canopy_len = (height_rows - 4).max(6);

                // Tiered conical evergreen boughs
                for dr in 0..=canopy_len {
                    let rr = crown_top_r + dr;
                    if rr >= ground_row {
                        break;
                    }

                    if dr == 0 {
                        // Apical spire tip
                        if root_c >= 0 && root_c < cols {
                            scene.push_glyph(
                                '▲',
                                cell_pos(root_c, rr),
                                cell_size,
                                0.0,
                                0.042,
                                trunk_color,
                                0.0,
                            );
                        }
                        continue;
                    }

                    let tier = dr / 3;
                    let step_in_tier = dr % 3;
                    let half_w = (1 + tier + if step_in_tier > 0 { 1 } else { 0 }).min(5);

                    for dc in -half_w..=half_w {
                        let cc = root_c + dc;
                        if cc < 0 || cc >= cols {
                            continue;
                        }

                        let p = cell_pos(cc, rr);
                        let needle_hash =
                            (cc as u32).wrapping_mul(79) ^ (rr as u32).wrapping_mul(43);

                        let ch = if dc == 0 {
                            if dr % 2 == 0 { '║' } else { '▲' }
                        } else if dc == -half_w {
                            '/'
                        } else if dc == half_w {
                            '\\'
                        } else if needle_hash.is_multiple_of(3) {
                            '▒'
                        } else if needle_hash.is_multiple_of(2) {
                            '░'
                        } else {
                            '▲'
                        };

                        scene.push_glyph(ch, p, cell_size, 0.0, 0.042, trunk_color, 0.0);
                    }
                }

                // Bare trunk anchored to ground
                let trunk_start_r = (crown_top_r + canopy_len + 1).min(ground_row - 1);
                for tr in trunk_start_r..ground_row {
                    if root_c >= 0 && root_c < cols {
                        scene.push_glyph(
                            '║',
                            cell_pos(root_c, tr),
                            cell_size,
                            0.0,
                            0.042,
                            trunk_color,
                            0.0,
                        );
                    }
                }
            };

        for k in min_stand..=max_stand {
            let stand_world_c = k * stand_spacing + ((k * 37).rem_euclid(7) - 3);
            let tree_c = (stand_world_c as f32 - mid_c_offset).round() as i32;

            if tree_c >= -8 && tree_c < cols + 8 {
                let h_rows = 18 + (k.unsigned_abs() * 19 % 8) as i32;
                render_evergreen(self, tree_c, h_rows, midground_color);

                // Occasional companion sibling evergreen for natural clustered forest look
                if k % 2 == 0 {
                    let comp_offset = if (k / 2) % 2 == 0 { 6 } else { -6 };
                    let comp_c = tree_c + comp_offset;
                    if comp_c >= -6 && comp_c < cols + 6 {
                        let comp_h = 12 + (k.unsigned_abs() * 7 % 4) as i32;
                        render_evergreen(self, comp_c, comp_h, midground_color);
                    }
                }
            }
        }

        // 2. Multi-Depth Ground: Grass Tufts, Wildflowers, Turf & Subterranean Strata
        // 2a. Upper grass depth layer (ground_row - 1): grass tufts & wildflowers poking into lower air
        for c in 1..cols.saturating_sub(1) {
            let grass_hash = (c as u32).wrapping_mul(127_412_617) ^ 0x9e3779b9;
            if grass_hash % 100 < 36 {
                let (grass_ch, grass_alpha, is_flower) = match grass_hash % 6 {
                    0 => ('"', 0.92, false),
                    1 => ('☘', 0.88, false),
                    2 => ('v', 0.80, false),
                    3 => ('w', 0.90, false),
                    4 => ('*', 0.95, true),
                    _ => (',', 0.75, false),
                };
                let g_col = if is_flower {
                    [
                        soil_crystal[0],
                        soil_crystal[1],
                        soil_crystal[2],
                        grass_alpha,
                    ]
                } else {
                    [
                        turf_color[0] * 1.05,
                        turf_color[1] * 1.05,
                        turf_color[2] * 1.02,
                        grass_alpha,
                    ]
                };
                self.push_glyph(
                    grass_ch,
                    cell_pos(c, ground_row - 1),
                    cell_size,
                    0.0,
                    0.048,
                    g_col,
                    0.0,
                );
            }
        }

        // 2b. Primary turf row (ground_row) with natural forest floor objects & cast shadows
        for c in 1..cols.saturating_sub(1) {
            let hash = (c as u32).wrapping_mul(73_856_093)
                ^ ((ground_row as u32).wrapping_mul(19_349_663));
            let turf_ch = match hash % 4 {
                0 => '☘',
                1 => '"',
                2 => '▀',
                _ => ',',
            };
            self.push_glyph(
                turf_ch,
                cell_pos(c, ground_row),
                cell_size,
                0.0,
                0.05,
                turf_color,
                0.0,
            );

            // Natural ground objects with cast shadows every ~18 columns
            let obj_hash = (c as u32).wrapping_mul(179_424_673) ^ 0x45d9f3b;
            if obj_hash % 100 < 6 {
                let is_boulder = obj_hash.is_multiple_of(2);
                let obj_pos = cell_pos(c, ground_row);
                if is_boulder {
                    // Mossy river boulder
                    self.push_glyph(
                        'o',
                        obj_pos,
                        cell_size,
                        0.0,
                        0.065,
                        [0.55, 0.60, 0.65, 0.95],
                        0.0,
                    );
                } else {
                    // Wild woodland mushroom
                    self.push_glyph(
                        'T',
                        obj_pos,
                        cell_size,
                        0.0,
                        0.065,
                        [0.92, 0.45, 0.28, 0.98],
                        0.0,
                    );
                }
                // Noticeable directional cast shadow away from moon
                for sk in 1..=2 {
                    let sc = c + shadow_dir * sk;
                    if sc >= 0 && sc < cols {
                        self.push_glyph(
                            '░',
                            cell_pos(sc, ground_row),
                            cell_size,
                            0.0,
                            0.052,
                            shadow_color,
                            0.0,
                        );
                    }
                }
            }
        }

        // 2c. Subterranean Strata
        for r in (ground_row + 1)..rows.saturating_sub(1) {
            for c in 1..cols.saturating_sub(1) {
                let hash =
                    (c as u32).wrapping_mul(73_856_093) ^ (r as u32).wrapping_mul(19_349_663);
                let soil_ch = match (r - ground_row, hash % 9) {
                    (1, 0..=4) => '▓',
                    (1, _) => '▒',
                    (_, 0) => '◆',
                    (_, 1..=3) => '▒',
                    _ => '░',
                };
                let col = if soil_ch == '◆' {
                    soil_crystal
                } else {
                    soil_main
                };
                self.push_glyph(soil_ch, cell_pos(c, r), cell_size, 0.0, 0.04, col, 0.0);
            }
        }

        // 3. Stepped Monospace Box-Drawing Tree Boughs & Multi-Column Bark
        for branch in &world_guard.branches {
            // Viewport culling: only render branches within or near the visible screen area
            let min_x = branch.start[0].min(branch.current_end()[0]);
            let max_x = branch.start[0].max(branch.current_end()[0]);
            if max_x < offset_x - 500.0 || min_x > offset_x + 1500.0 {
                continue;
            }

            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let c0 = (start[0] / cell_w).round() as i32;
            let r0 = (start[1] / cell_h).round() as i32;
            let c1 = (end[0] / cell_w).round() as i32;
            let r1 = (end[1] / cell_h).round() as i32;

            let (bark_bg, bark_fg) = match palette {
                RookeryPalette::MidnightStarlight => {
                    ([0.12, 0.16, 0.22, 0.95], [0.72, 0.82, 0.92, 0.98])
                }
                RookeryPalette::AutumnHearth => {
                    ([0.22, 0.12, 0.08, 0.95], [0.85, 0.52, 0.28, 0.98])
                }
                RookeryPalette::VerdantRainforest => {
                    ([0.12, 0.18, 0.10, 0.95], [0.55, 0.82, 0.42, 0.98])
                }
                RookeryPalette::AmberPhosphor => {
                    ([0.32, 0.18, 0.04, 0.95], [0.98, 0.75, 0.20, 0.98])
                }
                RookeryPalette::MatrixGreen => ([0.05, 0.24, 0.08, 0.95], [0.28, 0.95, 0.38, 0.98]),
                RookeryPalette::EarthAndSylvan => {
                    ([0.24, 0.16, 0.10, 0.92], [0.76, 0.56, 0.36, 0.98])
                }
            };

            let steps = (c1 - c0).abs().max((r1 - r0).abs()).max(1);

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cc = (c0 as f32 + (c1 - c0) as f32 * t).round() as i32;
                let rr = (r0 as f32 + (r1 - r0) as f32 * t).round() as i32;
                if cc < 0 || cc >= cols || rr < 0 || rr >= rows {
                    continue;
                }

                if branch.depth == 0 {
                    // Wide ANSI trunk with double-line box-drawing edges and organic bark knots
                    // Subtle trunk ring expansion and bark widening happening cell-by-cell
                    let span = if branch.thickness < 8.0 {
                        0 // Sapling single-column stem: 1 cell wide (║)
                    } else if branch.thickness < 18.0 {
                        1 // Young tree: 3 cells wide (├ ║ ┤)
                    } else if branch.thickness < 32.0 {
                        2 // Developing tree: 5 cells wide (├ █ ║ █ ┤)
                    } else if branch.thickness < 44.0 {
                        3 // Grand mature tree: 7 cells wide
                    } else {
                        4 // Massive ancient trunk: 9 cells wide
                    };

                    // Noticeable cast shadow on the ground turf at the base of the trunk
                    if rr == ground_row {
                        for sk in 1..=3 {
                            let sc = if shadow_dir < 0 {
                                cc - span + shadow_dir * sk
                            } else {
                                cc + span + shadow_dir * sk
                            };
                            if sc >= 0 && sc < cols {
                                let shadow_ch = if sk <= 2 { '▒' } else { '░' };
                                self.push_glyph(
                                    shadow_ch,
                                    cell_pos(sc, ground_row),
                                    cell_size,
                                    0.0,
                                    0.052,
                                    shadow_color,
                                    0.0,
                                );
                            }
                        }
                    }

                    let is_trunk_apical_tip = rr == r1
                        && (branch.growth_progress < 0.99
                            || branch.length < branch.target_length - 2.0);

                    for dx in -span..=span {
                        let col_idx = cc + dx;
                        if col_idx < 0 || col_idx >= cols {
                            continue;
                        }
                        let tp = cell_pos(col_idx, rr);
                        self.push_glyph('█', tp, cell_size, 0.0, 0.08, bark_bg, 0.0);

                        // Apical shoot at the actively growing top of the trunk
                        if is_trunk_apical_tip && dx == 0 {
                            let shoot_fg = match palette {
                                RookeryPalette::AutumnHearth => [0.98, 0.78, 0.28, 0.98],
                                RookeryPalette::MidnightStarlight => [0.45, 0.92, 0.98, 0.98],
                                _ => [0.48, 0.95, 0.42, 0.98],
                            };
                            self.push_glyph('▲', tp, cell_size, 0.0, 0.12, shoot_fg, 0.0);
                            continue;
                        }

                        let knot_hash = (col_idx as u32).wrapping_mul(2654435761)
                            ^ (rr as u32).wrapping_mul(2246822519);
                        let ch = if span == 0 {
                            '║'
                        } else if dx == 0 {
                            if knot_hash.is_multiple_of(29) {
                                'O' // Weathered knot
                            } else if knot_hash.is_multiple_of(41) {
                                'o' // Small hollow
                            } else {
                                '║'
                            }
                        } else if dx == -span {
                            '├'
                        } else if dx == span {
                            '┤'
                        } else if knot_hash.is_multiple_of(37) {
                            '0' // Striated wood grain
                        } else {
                            '█'
                        };
                        self.push_glyph(ch, tp, cell_size, 0.0, 0.10, bark_fg, 0.0);
                    }
                } else {
                    // Depth-based bough shading: Deeper background branches have subdued tone
                    let branch_fg = if branch.depth >= 3 {
                        [
                            bark_fg[0] * 0.82,
                            bark_fg[1] * 0.82,
                            bark_fg[2] * 0.82,
                            bark_fg[3],
                        ]
                    } else {
                        bark_fg
                    };

                    let dc = (c1 - c0).abs();
                    let dr = (r1 - r0).abs();
                    let is_tip = s == steps;
                    let is_actively_growing = branch.growth_progress < 0.96;

                    if is_tip && is_actively_growing {
                        // Tender growing shoot / bud at the extending branch front
                        let tip_fg = match palette {
                            RookeryPalette::AutumnHearth => [0.98, 0.78, 0.28, 0.98],
                            RookeryPalette::MidnightStarlight => [0.45, 0.92, 0.98, 0.98],
                            RookeryPalette::AmberPhosphor => [0.98, 0.82, 0.25, 0.98],
                            RookeryPalette::MatrixGreen => [0.35, 0.98, 0.45, 0.98],
                            _ => [0.48, 0.95, 0.42, 0.98],
                        };
                        let tip_ch = if steps <= 1 {
                            '·'
                        } else if dc > dr * 2 {
                            '─'
                        } else if dr > dc * 2 {
                            '│'
                        } else if (c1 - c0) * (r1 - r0) < 0 {
                            '/'
                        } else {
                            '\\'
                        };
                        let pos = cell_pos(cc, rr);
                        self.push_glyph(tip_ch, pos, cell_size, 0.0, 0.12, tip_fg, 0.0);
                    } else {
                        let box_ch = if is_tip {
                            if branch.children_spawned {
                                '┼'
                            } else if dc > dr * 2 {
                                '═'
                            } else if dr > dc * 2 {
                                '║'
                            } else if (c1 - c0) * (r1 - r0) < 0 {
                                '┌'
                            } else {
                                '┐'
                            }
                        } else if dc > dr * 2 {
                            '═'
                        } else if dr > dc * 2 {
                            '║'
                        } else if (c1 - c0) * (r1 - r0) < 0 {
                            '┌'
                        } else {
                            '┐'
                        };
                        let pos = cell_pos(cc, rr);
                        self.push_glyph('█', pos, cell_size, 0.0, 0.09, bark_bg, 0.0);
                        self.push_glyph(box_ch, pos, cell_size, 0.0, 0.11, branch_fg, 0.0);

                        // Moss & lichen on shaded lower contours of older branches
                        if branch.depth <= 2 && (s as u32).is_multiple_of(4) && rr < ground_row - 1
                        {
                            let moss_col = [0.28, 0.72, 0.36, 0.75];
                            let moss_pos = cell_pos(cc, (rr + 1).min(rows - 1));
                            self.push_glyph('░', moss_pos, cell_size, 0.0, 0.095, moss_col, 0.0);
                        }
                    }
                }
            }

            // 4. Roguelike Canopy Foliage Clusters & Cellular Bud Swelling
            for (idx, bud) in branch.foliage.iter().enumerate() {
                // Dormant bud: branch has not reached it yet, or barely sprouted
                if bud.offset_along > branch.growth_progress || bud.bloom <= 0.03 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset;
                let center_c = (pt[0] / cell_w).floor() as i32;
                let center_r = (pt[1] / cell_h).floor() as i32;
                if center_c < -1 || center_c > cols || center_r < 0 || center_r >= rows {
                    continue;
                }

                // Stage 1: Single-cell tiny bud dot (· / •)
                if bud.bloom < 0.28 {
                    let bud_col = match palette {
                        RookeryPalette::AutumnHearth => [0.98, 0.72, 0.25, 0.95],
                        RookeryPalette::MidnightStarlight => [0.55, 0.92, 0.98, 0.95],
                        RookeryPalette::AmberPhosphor => [0.98, 0.75, 0.20, 0.95],
                        RookeryPalette::MatrixGreen => [0.35, 0.98, 0.45, 0.95],
                        _ => [0.45, 0.92, 0.40, 0.95],
                    };
                    let ch = if idx % 2 == 0 { '·' } else { '•' };
                    let cp = cell_pos(center_c, center_r);
                    self.push_glyph(ch, cp, cell_size, 0.0, 0.15, bud_col, 0.0);
                    continue;
                }

                // Stage 2: Single-cell young flower star / leaflet (* / + / ✦)
                if bud.bloom < 0.62 {
                    let star_fg = match palette {
                        RookeryPalette::AutumnHearth => [0.98, 0.58, 0.20, 0.98],
                        RookeryPalette::MidnightStarlight => [0.72, 0.88, 1.0, 0.98],
                        RookeryPalette::AmberPhosphor => [1.0, 0.78, 0.22, 0.98],
                        RookeryPalette::MatrixGreen => [0.28, 0.98, 0.40, 0.98],
                        _ => [0.38, 0.96, 0.52, 0.98],
                    };
                    let star_bg = match palette {
                        RookeryPalette::MidnightStarlight => [0.06, 0.14, 0.22, 0.75],
                        RookeryPalette::AutumnHearth => [0.24, 0.10, 0.04, 0.75],
                        _ => [0.05, 0.20, 0.10, 0.75],
                    };
                    let ch = if idx % 2 == 0 { '*' } else { '+' };
                    let cp = cell_pos(center_c, center_r);
                    self.push_glyph('█', cp, cell_size, 0.0, 0.13, star_bg, 0.0);
                    self.push_glyph(ch, cp, cell_size, 0.0, 0.15, star_fg, 0.0);
                    continue;
                }

                let (bg_col, fg_col, tile_char) = match (palette, bud.color_idx) {
                    (RookeryPalette::AutumnHearth, 0) => (
                        [0.32, 0.14, 0.06, 0.90],
                        [0.98, 0.48, 0.15, 0.98],
                        if idx % 2 == 0 { '▓' } else { '☘' },
                    ),
                    (RookeryPalette::AutumnHearth, 1) => (
                        [0.35, 0.22, 0.05, 0.90],
                        [0.98, 0.82, 0.20, 0.98],
                        if idx % 2 == 0 { '▒' } else { '*' },
                    ),
                    (RookeryPalette::AutumnHearth, 2) => {
                        ([0.38, 0.08, 0.08, 0.92], [0.98, 0.28, 0.22, 0.98], '◆')
                    }
                    (RookeryPalette::AutumnHearth, _) => {
                        ([0.28, 0.12, 0.06, 0.90], [0.85, 0.58, 0.22, 0.98], '✦')
                    }
                    (RookeryPalette::MidnightStarlight, 0) => (
                        [0.06, 0.18, 0.28, 0.90],
                        [0.35, 0.88, 0.98, 0.98],
                        if idx % 2 == 0 { '☘' } else { '▓' },
                    ),
                    (RookeryPalette::MidnightStarlight, 1) => (
                        [0.10, 0.22, 0.32, 0.90],
                        [0.72, 0.92, 1.0, 0.98],
                        if idx % 2 == 0 { '▒' } else { '*' },
                    ),
                    (RookeryPalette::MidnightStarlight, _) => {
                        ([0.18, 0.12, 0.28, 0.90], [0.88, 0.78, 0.98, 0.98], '✦')
                    }
                    (RookeryPalette::AmberPhosphor, _) => (
                        [0.24, 0.14, 0.02, 0.90],
                        [1.0, 0.78, 0.22, 0.98],
                        if idx % 2 == 0 { '▓' } else { '*' },
                    ),
                    (RookeryPalette::MatrixGreen, _) => (
                        [0.04, 0.22, 0.06, 0.90],
                        [0.25, 0.98, 0.38, 0.98],
                        if idx % 2 == 0 { '▒' } else { '☘' },
                    ),
                    _ => match bud.color_idx {
                        0 => (
                            [0.06, 0.28, 0.14, 0.88],
                            [0.28, 0.95, 0.54, 0.98],
                            if idx % 2 == 0 { '☘' } else { '▓' },
                        ),
                        1 => (
                            [0.10, 0.34, 0.16, 0.85],
                            [0.52, 0.96, 0.60, 0.98],
                            if idx % 2 == 0 { '▒' } else { '*' },
                        ),
                        2 => ([0.34, 0.24, 0.05, 0.88], [0.98, 0.84, 0.25, 0.98], '✦'),
                        _ => ([0.36, 0.10, 0.20, 0.88], [0.98, 0.58, 0.78, 0.98], '◆'),
                    },
                };

                // Stage 3 (0.62 <= bloom < 0.88): 2-cell expanding cluster
                // Stage 4 (bloom >= 0.88): 4-cell mature canopy cross
                let is_expanding = bud.bloom < 0.88;
                let offsets: &[(i32, i32)] = if is_expanding {
                    &[(0, 0), (0, -1)]
                } else {
                    &[(0, 0), (-1, 0), (1, 0), (0, -1)]
                };

                for (oi, &(dc, dr)) in offsets.iter().enumerate() {
                    if oi > 0 && idx % 2 != 0 && !is_expanding {
                        continue;
                    }
                    let (cc, rr) = (center_c + dc, center_r + dr);
                    if cc < 0 || cc >= cols || rr < 0 || rr >= rows {
                        continue;
                    }
                    let cp = cell_pos(cc, rr);
                    let ch = if oi == 0 {
                        if is_expanding { '▒' } else { tile_char }
                    } else {
                        '░'
                    };
                    self.push_glyph('█', cp, cell_size, 0.0, 0.14, bg_col, 0.0);
                    self.push_glyph(ch, cp, cell_size, 0.0, 0.16, fg_col, 0.0);
                }
            }
        }

        // 5. Multi-Cell Grid-Snapped Birds (2–3 cells)
        let bird_bg = [0.05, 0.07, 0.10, 0.92];
        for bird in &world_guard.birds {
            let sp = self.world_to_screen(bird.pos, offset_x);
            let center_c = (sp[0] / cell_w).round() as i32;
            let center_r = (sp[1] / cell_h).round() as i32;
            if center_c < -2 || center_c > cols + 2 || center_r < 0 || center_r >= rows {
                continue;
            }
            let col = Self::species_color(bird.species);

            match bird.state {
                BirdState::Perched { .. }
                | BirdState::Feeding { .. }
                | BirdState::GatheringTwig { .. }
                | BirdState::BuildingNest { .. }
                | BirdState::BroodingEggs { .. }
                | BirdState::Bathing { .. } => {
                    // 3-cell perched posture: Beak, Body, Tail
                    let is_feeding = matches!(bird.state, BirdState::Feeding { .. });
                    let (head_ch, body_ch, tail_ch) = if bird.facing_left {
                        (if is_feeding { 'v' } else { '<' }, '@', '/')
                    } else {
                        (if is_feeding { 'v' } else { '>' }, '@', '\\')
                    };

                    let (c_head, c_body, c_tail) = if bird.facing_left {
                        (center_c - 1, center_c, center_c + 1)
                    } else {
                        (center_c + 1, center_c, center_c - 1)
                    };

                    // Tail
                    if c_tail >= 0 && c_tail < cols {
                        let p_tail = cell_pos(c_tail, center_r);
                        self.push_glyph('█', p_tail, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(tail_ch, p_tail, cell_size, 0.0, 0.25, col, 0.0);
                    }

                    // Body
                    if c_body >= 0 && c_body < cols {
                        let p_body = cell_pos(c_body, center_r);
                        self.push_glyph('█', p_body, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(body_ch, p_body, cell_size, 0.0, 0.26, col, 0.0);
                    }

                    // Head/Beak
                    if c_head >= 0 && c_head < cols {
                        let p_head = cell_pos(c_head, center_r);
                        self.push_glyph('█', p_head, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(head_ch, p_head, cell_size, 0.0, 0.25, col, 0.0);
                    }
                }
                _ => {
                    // 3-cell flight posture: Wing left, Body/Direction, Wing right
                    let flap = bird.wing_phase.sin();
                    let (wing_l, wing_r) = if flap > 0.3 {
                        ('\\', '/')
                    } else if flap < -0.3 {
                        ('/', '\\')
                    } else {
                        ('-', '-')
                    };
                    let dir_ch = if bird.facing_left { '<' } else { '>' };

                    // Wing left
                    if center_c >= 1 && center_c - 1 < cols {
                        let p_wl = cell_pos(center_c - 1, center_r);
                        self.push_glyph('█', p_wl, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(wing_l, p_wl, cell_size, 0.0, 0.25, col, 0.0);
                    }

                    // Center
                    if center_c >= 0 && center_c < cols {
                        let p_mid = cell_pos(center_c, center_r);
                        self.push_glyph('█', p_mid, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(dir_ch, p_mid, cell_size, 0.0, 0.26, col, 0.0);
                    }

                    // Wing right
                    if center_c >= -1 && center_c + 1 < cols {
                        let p_wr = cell_pos(center_c + 1, center_r);
                        self.push_glyph('█', p_wr, cell_size, 0.0, 0.22, bird_bg, 0.0);
                        self.push_glyph(wing_r, p_wr, cell_size, 0.0, 0.25, col, 0.0);
                    }

                    // Celestial Raven stardust trail
                    if bird.species == BirdSpecies::CelestialRaven {
                        let trail_c = if bird.facing_left {
                            center_c + 2
                        } else {
                            center_c - 2
                        };
                        if trail_c >= 0 && trail_c < cols {
                            let p_trail = cell_pos(trail_c, center_r);
                            self.push_glyph(
                                '✦',
                                p_trail,
                                cell_size,
                                0.0,
                                0.20,
                                [0.45, 0.85, 1.0, 0.70],
                                0.0,
                            );
                        }
                    }
                }
            }
        }

        // 5b. Multi-Cell Forest Floor Woodland Fauna (Milestone 10)
        for fauna in &world_guard.fauna {
            let sp = self.world_to_screen(fauna.pos, offset_x);
            let center_c = (sp[0] / cell_w).round() as i32;
            let center_r = (sp[1] / cell_h).round() as i32;
            if center_c < -4 || center_c > cols + 4 || center_r < 0 || center_r >= rows {
                continue;
            }

            // Directional cast shadow on ground turf away from celestial moon
            if fauna.pos[1] >= 740.0 {
                for sk in 1..=2 {
                    let sc = center_c + shadow_dir * sk;
                    if sc >= 0 && sc < cols {
                        self.push_glyph(
                            '░',
                            cell_pos(sc, ground_row),
                            cell_size,
                            0.0,
                            0.052,
                            shadow_color,
                            0.0,
                        );
                    }
                }
            }

            let is_startled = matches!(fauna.state, FaunaState::Startled { .. });
            let is_sleeping = matches!(fauna.state, FaunaState::Sleeping { .. });
            let is_sniffing = matches!(fauna.state, FaunaState::Sniffing { .. });

            // Exclamation indicator when startled
            if is_startled && center_r >= 1 {
                self.push_glyph(
                    '!',
                    cell_pos(center_c, center_r - 1),
                    [cell_w * 0.85, cell_h * 0.85],
                    0.0,
                    0.28,
                    [0.98, 0.88, 0.25, 0.98],
                    0.0,
                );
            }

            // Sleep indicator notes
            if is_sleeping && center_r >= 1 && (self.time as f32 * 1.5).sin() > 0.0 {
                self.push_glyph(
                    'z',
                    cell_pos(center_c, center_r - 1),
                    [cell_w * 0.75, cell_h * 0.75],
                    0.0,
                    0.27,
                    [0.85, 0.90, 1.0, 0.85],
                    0.0,
                );
            }

            match fauna.species {
                FaunaSpecies::Fox => {
                    let coat_col = [0.94, 0.46, 0.16, 0.98];
                    let white_col = [0.98, 0.95, 0.88, 0.98];
                    let bg_col = [0.14, 0.06, 0.02, 0.88];

                    if is_sleeping {
                        // Curled ball: ~ o ~
                        let cells = [('~', coat_col), ('o', coat_col), ('~', white_col)];
                        for (i, &(ch, col)) in cells.iter().enumerate() {
                            let cc = center_c - 1 + i as i32;
                            if cc >= 0 && cc < cols {
                                let p = cell_pos(cc, center_r);
                                self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                                self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                            }
                        }
                    } else {
                        // 4-cell sleek fox: Tail, Body, Head, Snout
                        let head_ch = if is_sniffing { '.' } else { 'o' };
                        let snout_ch = if is_sniffing {
                            'v'
                        } else if fauna.facing_left {
                            '<'
                        } else {
                            '>'
                        };
                        let tail_ch = if fauna.anim_phase.sin() > 0.0 {
                            '~'
                        } else {
                            '≈'
                        };

                        let sprite_chars = if fauna.facing_left {
                            [
                                (snout_ch, white_col),
                                (head_ch, coat_col),
                                ('▄', coat_col),
                                (tail_ch, white_col),
                            ]
                        } else {
                            [
                                (tail_ch, white_col),
                                ('▄', coat_col),
                                (head_ch, coat_col),
                                (snout_ch, white_col),
                            ]
                        };

                        for (i, &(ch, col)) in sprite_chars.iter().enumerate() {
                            let cc = center_c - 1 + i as i32;
                            if cc >= 0 && cc < cols {
                                let p = cell_pos(cc, center_r);
                                self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                                self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                            }
                        }
                    }
                }
                FaunaSpecies::Squirrel => {
                    let coat_col = [0.78, 0.48, 0.22, 0.98];
                    let chest_col = [0.96, 0.90, 0.82, 0.98];
                    let bg_col = [0.12, 0.08, 0.04, 0.88];

                    if matches!(fauna.state, FaunaState::ClimbingTrunk { .. }) {
                        // Vertical climbing sprite
                        let p_head = cell_pos(center_c, center_r);
                        self.push_glyph('█', p_head, cell_size, 0.0, 0.22, bg_col, 0.0);
                        self.push_glyph('^', p_head, cell_size, 0.0, 0.26, chest_col, 0.0);

                        if center_r + 1 < rows {
                            let p_body = cell_pos(center_c, center_r + 1);
                            self.push_glyph('█', p_body, cell_size, 0.0, 0.22, bg_col, 0.0);
                            self.push_glyph('"', p_body, cell_size, 0.0, 0.25, coat_col, 0.0);
                        }
                    } else if matches!(fauna.state, FaunaState::Foraging { .. }) {
                        // Upright foraging with nut: ? o *
                        let sprite = if fauna.facing_left {
                            [
                                ('*', [0.98, 0.82, 0.25, 0.98]),
                                ('o', chest_col),
                                ('c', coat_col),
                            ]
                        } else {
                            [
                                ('?', coat_col),
                                ('o', chest_col),
                                ('*', [0.98, 0.82, 0.25, 0.98]),
                            ]
                        };
                        for (i, &(ch, col)) in sprite.iter().enumerate() {
                            let cc = center_c - 1 + i as i32;
                            if cc >= 0 && cc < cols {
                                let p = cell_pos(cc, center_r);
                                self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                                self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                            }
                        }
                    } else {
                        // Trotting 3 cells: Tail, Body, Nose
                        let sprite = if fauna.facing_left {
                            [('<', chest_col), ('o', coat_col), ('c', coat_col)]
                        } else {
                            [('?', coat_col), ('o', coat_col), ('>', chest_col)]
                        };
                        for (i, &(ch, col)) in sprite.iter().enumerate() {
                            let cc = center_c - 1 + i as i32;
                            if cc >= 0 && cc < cols {
                                let p = cell_pos(cc, center_r);
                                self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                                self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                            }
                        }
                    }
                }
                FaunaSpecies::Badger => {
                    let coat_col = [0.34, 0.36, 0.40, 0.98];
                    let stripe_col = [0.98, 0.98, 1.0, 0.98];
                    let bg_col = [0.08, 0.08, 0.10, 0.88];

                    let muzzle_ch = if is_sniffing {
                        '.'
                    } else if fauna.facing_left {
                        '<'
                    } else {
                        '>'
                    };
                    let sprite = if fauna.facing_left {
                        [(muzzle_ch, stripe_col), ('≡', coat_col), ('~', coat_col)]
                    } else {
                        [('~', coat_col), ('≡', coat_col), (muzzle_ch, stripe_col)]
                    };

                    for (i, &(ch, col)) in sprite.iter().enumerate() {
                        let cc = center_c - 1 + i as i32;
                        if cc >= 0 && cc < cols {
                            let p = cell_pos(cc, center_r);
                            self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                            self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                        }
                    }
                }
                FaunaSpecies::Hedgehog => {
                    let quill_col = [0.64, 0.52, 0.38, 0.98];
                    let snout_col = [0.92, 0.82, 0.72, 0.98];
                    let bg_col = [0.10, 0.08, 0.06, 0.88];

                    if is_sleeping || (is_startled && fauna.anim_phase.sin() > 0.0) {
                        // Curled prickly ball: ○
                        let p = cell_pos(center_c, center_r);
                        self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                        self.push_glyph('○', p, cell_size, 0.0, 0.25, quill_col, 0.0);
                    } else {
                        let sprite = if fauna.facing_left {
                            [('<', snout_col), ('*', quill_col)]
                        } else {
                            [('*', quill_col), ('>', snout_col)]
                        };
                        for (i, &(ch, col)) in sprite.iter().enumerate() {
                            let cc = center_c + i as i32;
                            if cc >= 0 && cc < cols {
                                let p = cell_pos(cc, center_r);
                                self.push_glyph('█', p, cell_size, 0.0, 0.22, bg_col, 0.0);
                                self.push_glyph(ch, p, cell_size, 0.0, 0.25, col, 0.0);
                            }
                        }
                    }
                }
            }
        }

        // 6. Forest Infrastructure: Plant Signs, Lanterns, Baths & Perches
        for sign in &world_guard.signs {
            let sp = snap(self.world_to_screen(sign.pos, offset_x));
            let c = (sp[0] / cell_w).round() as i32;
            if c >= 0 && c < cols {
                // Directional cast shadow on ground turf
                for sk in 1..=2 {
                    let sc = c + shadow_dir * sk;
                    if sc >= 0 && sc < cols {
                        self.push_glyph(
                            '░',
                            cell_pos(sc, ground_row),
                            cell_size,
                            0.0,
                            0.052,
                            shadow_color,
                            0.0,
                        );
                    }
                }
                self.push_glyph('█', sp, cell_size, 0.0, 0.30, [0.28, 0.18, 0.10, 0.95], 0.0);
                self.push_glyph('i', sp, cell_size, 0.0, 0.32, [0.98, 0.92, 0.55, 0.98], 0.0);
            }
        }

        for bath in &world_guard.bird_baths {
            let sp = snap(self.world_to_screen(bath.pos, offset_x));
            let c = (sp[0] / cell_w).round() as i32;
            if c >= 0 && c < cols {
                // Directional cast shadow on ground turf
                for sk in 1..=2 {
                    let sc = c + shadow_dir * sk;
                    if sc >= 0 && sc < cols {
                        self.push_glyph(
                            '▒',
                            cell_pos(sc, ground_row),
                            cell_size,
                            0.0,
                            0.052,
                            shadow_color,
                            0.0,
                        );
                    }
                }
                self.push_glyph('█', sp, cell_size, 0.0, 0.28, [0.18, 0.22, 0.26, 0.95], 0.0);
                self.push_glyph('~', sp, cell_size, 0.0, 0.30, [0.45, 0.85, 0.98, 0.98], 0.0);
            }
        }

        for perch in &world_guard.perches {
            let sp = snap(self.world_to_screen(perch.pos, offset_x));
            let c = (sp[0] / cell_w).round() as i32;
            if c >= 0 && c < cols {
                // Directional cast shadow on ground turf
                for sk in 1..=2 {
                    let sc = c + shadow_dir * sk;
                    if sc >= 0 && sc < cols {
                        self.push_glyph(
                            '░',
                            cell_pos(sc, ground_row),
                            cell_size,
                            0.0,
                            0.052,
                            shadow_color,
                            0.0,
                        );
                    }
                }
                self.push_glyph('═', sp, cell_size, 0.0, 0.28, [0.68, 0.52, 0.35, 0.95], 0.0);
            }
        }

        for lantern in &world_guard.lanterns {
            if let Some(b) = world_guard
                .branches
                .iter()
                .find(|b| b.id == lantern.branch_id)
            {
                let pt = b.point_at(lantern.t);
                if pt[0] >= offset_x - 50.0 && pt[0] <= offset_x + 1050.0 {
                    let sp = snap(self.world_to_screen(pt, offset_x));
                    let c = (sp[0] / cell_w).round() as i32;
                    if c >= 0 && c < cols {
                        self.push_glyph(
                            '█',
                            sp,
                            cell_size,
                            0.0,
                            0.30,
                            [0.22, 0.14, 0.04, 0.95],
                            0.0,
                        );
                        self.push_glyph(
                            '¤',
                            sp,
                            cell_size,
                            0.0,
                            0.32,
                            [1.0, 0.88, 0.25, 0.98],
                            0.0,
                        );
                    }
                }
            }
        }

        // 7. ANSI Double-Line Terminal Border & Status Bar at Bottom
        let bottom_r = rows.saturating_sub(1);
        for c in 1..cols.saturating_sub(1) {
            let ch = if c == 1 {
                '╚'
            } else if c == cols - 2 {
                '╝'
            } else {
                '═'
            };
            self.push_glyph(
                ch,
                cell_pos(c, bottom_r),
                cell_size,
                0.0,
                0.50,
                border_color,
                0.0,
            );
        }

        let active_species_name = world_guard
            .trees
            .iter()
            .find(|t| t.id == world_guard.active_tree_id)
            .map(|t| t.species.display_name())
            .unwrap_or("Ancient Oak");
        let active_tree_num = world_guard.trees.len();

        let status_str = format!(
            "╡ FORT SYLVAN · SEED #{} · TREE #{}: {} · BOUGHS:{} · BUDS:{} · FLOCK:{} · FAUNA:{} ╞",
            world_guard.seed,
            active_tree_num,
            active_species_name,
            world_guard.stats.total_branches,
            world_guard.stats.total_foliage,
            world_guard.birds.len(),
            world_guard.fauna.len()
        );
        self.push_text(
            &status_str,
            cell_pos(4, bottom_r),
            [cell_w * 0.85, cell_h * 0.85],
            cell_w,
            0.55,
            [0.95, 0.92, 0.72, 0.98],
            0.0,
        );

        if world_guard.debug_mode {
            let active_tree = world_guard
                .trees
                .iter()
                .find(|t| t.id == world_guard.active_tree_id);
            let active_species_name = active_tree
                .map(|t| t.species.display_name())
                .unwrap_or("Ancient Oak");
            let step = active_tree.map(|t| t.growth_steps_completed).unwrap_or(0);
            let target_step = active_tree.map(|t| t.target_growth_steps).unwrap_or(42);
            let is_mature = active_tree.map(|t| t.is_mature).unwrap_or(false);
            let tree_seed = active_tree.map(|t| t.seed).unwrap_or(world_guard.seed);

            let debug_banner = format!(
                "DEBUG MODE · CAM X:{:.0} · TREE #{}: {} [SEED #{}] · STEP {}/{} ({}) · BOUGHS:{} · BUDS:{} · SIM:{:.1}x",
                offset_x,
                world_guard.active_tree_id,
                active_species_name,
                tree_seed,
                step,
                target_step,
                if is_mature { "MATURE" } else { "GROWING" },
                world_guard.stats.total_branches,
                world_guard.stats.total_foliage,
                world_guard.sim_speed,
            );

            self.push_text(
                &debug_banner,
                cell_pos(2, 1),
                [cell_w * 0.85, cell_h * 0.85],
                cell_w,
                0.90,
                [0.98, 0.45, 0.25, 0.98],
                0.0,
            );
        }
    }
}

impl Scene for LivingRookeryScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width as f32;
        self.height = height as f32;
        self.scale_factor = scale_factor as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: std::time::Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32();
        self.time += dt.as_secs_f64();
        self.interactive_mode = input.interactive;

        let pointer_world = input.pointer.map(|p| self.screen_to_world(p));

        let (activity, current_palette) = if let Ok(mut w) = self.world.lock() {
            if input.interactive {
                if let Some(sp) = input.pointer {
                    if input.action == PointerAction::RightClick
                        && let Some(pw) = pointer_world
                    {
                        w.prune_nearest(pw, 80.0);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::Prune);
                        }
                    }

                    if input.action == PointerAction::LeftClick {
                        self.drag_start_screen = Some(sp);
                        self.last_screen_pointer = Some(sp);
                        self.is_dragging_panning = false;
                        self.drag_distance_accum = 0.0;
                    }

                    if input.left_held {
                        if let Some(last) = self.last_screen_pointer {
                            let dx = sp.x - last.x;
                            let dy = sp.y - last.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            self.drag_distance_accum += dist;

                            if self.drag_distance_accum > 7.0 {
                                self.is_dragging_panning = true;
                                let pan_scale = 1000.0 / self.width.max(1.0);
                                w.pan_forest(-dx * pan_scale);
                            }
                        }
                        self.last_screen_pointer = Some(sp);
                    }
                }

                if !input.left_held && self.drag_start_screen.is_some() {
                    if !self.is_dragging_panning {
                        let clicked_top_left_pill = self
                            .drag_start_screen
                            .map(|sp| sp.x < 360.0 && sp.y < 48.0)
                            .unwrap_or(false);

                        if clicked_top_left_pill {
                            w.cycle_palette();
                            if let Some(ref a) = self.audio {
                                a.play_sfx(crate::audio::SoundEffect::PaletteCycle);
                            }
                        } else if let Some(pw) = pointer_world {
                            if pw[1] >= 720.0 {
                                w.plant_seed(pw);
                                if let Some(ref a) = self.audio {
                                    a.play_sfx(crate::audio::SoundEffect::PlantSeed);
                                }
                            } else {
                                let snack_type = match self.snack_click_count % 3 {
                                    0 => SnackType::Seeds,
                                    1 => SnackType::SweetBerries,
                                    _ => SnackType::StarNectar,
                                };
                                self.snack_click_count += 1;
                                w.drop_snack_typed(pw, snack_type);
                                if let Some(ref a) = self.audio {
                                    a.play_sfx(crate::audio::SoundEffect::BirdChirp);
                                }
                            }
                        }
                    }
                    self.drag_start_screen = None;
                    self.last_screen_pointer = None;
                    self.is_dragging_panning = false;
                }
            } else {
                self.drag_start_screen = None;
                self.last_screen_pointer = None;
                self.is_dragging_panning = false;
            }

            let pal = w.active_palette.as_u8();
            let mut act = w.update(dt_secs, input.interactive, pointer_world);
            if self.is_dragging_panning {
                act = Activity::Animating;
            }
            (act, pal)
        } else {
            (Activity::Idle, 0)
        };

        // Update background uniforms with active palette_id
        let bg_uniforms = BgUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            palette_id: current_palette as f32,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&bg_uniforms));

        // Generate and upload glyph instances
        self.generate_scene_instances();
        let wind_sway = 0.0; // strict roguelike monospace grid
        self.glyphs
            .upload(gpu, [self.width, self.height], self.time as f32, wind_sway);

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Render pass 1: Background atmospheric monospace shader
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ascii-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.016,
                            g: 0.024,
                            b: 0.038,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.bg_pipeline);
            pass.set_bind_group(0, &self.bg_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Render pass 2: Overlapping ASCII glyph instances with alpha blending
        self.glyphs.render(view, encoder);
    }
}

impl LivingRookeryScene {
    pub fn glyph_instance_count(&self) -> usize {
        self.glyphs.instance_count()
    }

    pub fn generate_scene_instances_for_test(&mut self) {
        self.generate_scene_instances();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rookery_core::world::TreeSpecies;

    #[test]
    fn test_viewport_culling_prevents_glyph_buffer_overflow() {
        let gpu = match Gpu::new_blocking() {
            Ok(g) => g,
            Err(_) => return, // Headless environment without GPU adapter
        };

        let mut world = WorldState::new(42);
        // Simulate succession with 15 trees and hundreds of branches
        for i in 1..=15 {
            let next_x = 500.0 + i as f32 * rookery_core::world::FOREST_TREE_SPACING;
            world.plant_new_sapling(next_x, TreeSpecies::AncientOak);
            for _ in 0..20 {
                world.execute_growth_step();
            }
        }
        // Pan camera to the last tree
        world.viewport_offset_x = 500.0 + 15.0 * rookery_core::world::FOREST_TREE_SPACING - 500.0;
        let total_branches = world.branches.len();
        assert!(
            total_branches > 150,
            "Should have accumulated many branches across trees"
        );

        let world_arc = Arc::new(Mutex::new(world));
        let mut scene = LivingRookeryScene::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, world_arc);
        scene.resize(&gpu, 1920, 1080, 1.0);
        scene.generate_scene_instances_for_test();

        let count = scene.glyph_instance_count();
        assert!(
            count > 0 && count < 15_000,
            "Glyph count ({count}) should be bounded by viewport culling, well under MAX_INSTANCES"
        );
    }
}
