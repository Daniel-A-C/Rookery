//! The Living ASCII Rookery: Signature Launch App Scene (Milestone 2 + Aesthetic Mock-Ups).
//!
//! Supports 8 selectable ASCII aesthetic mock-up styles (0..=7):
//! 0. Classic SDF Woodland (Baseline)
//! 1. Strict ANSI Roguelike Grid
//! 2. Full-Screen ASCII Matrix & Dithering
//! 3. Multi-Line Classic ASCII Art & Parallax
//! 4. Retro Phosphor CRT Terminal
//! 5. Sumi-e Ink & Botanical Field Journal
//! 6. Jewel-Box Stained-Glass Mosaic
//! 7. 3D Volumetric ASCII Tree Visualizer

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::world::{BirdSpecies, BirdState, ROOKERY_STYLE_COUNT, SnackType, WorldState};

pub use crate::atlas::{GlyphInstance, GlyphPass};
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 24_576;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BgUniforms {
    resolution: [f32; 2],
    time: f32,
    style_id: f32,
}

#[derive(Clone, Copy)]
struct Projected3dGlyph {
    c: char,
    screen_pos: [f32; 2],
    size: [f32; 2],
    z_cam: f32,
    color: [f32; 4],
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
        let ground_y = self.height * 0.90;
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
        let ground_y = self.height * 0.90;
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
        depth: f32,
        color: [f32; 4],
        sway: f32,
    ) {
        self.glyphs.push(c, pos, size, rotation, depth, color, sway);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_text(
        &mut self,
        text: &str,
        start_pos: [f32; 2],
        char_size: [f32; 2],
        step_x: f32,
        depth: f32,
        color: [f32; 4],
        sway: f32,
    ) {
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            self.push_glyph(
                ch,
                [start_pos[0] + (i as f32) * step_x, start_pos[1]],
                char_size,
                0.0,
                depth,
                color,
                sway,
            );
        }
    }

    fn species_color(species: BirdSpecies) -> [f32; 4] {
        match species {
            BirdSpecies::Rook => [0.85, 0.88, 0.95, 0.95],
            BirdSpecies::AzureJay => [0.32, 0.68, 0.98, 0.95],
            BirdSpecies::Goldfinch => [0.98, 0.82, 0.25, 0.95],
            BirdSpecies::Robin => [0.95, 0.45, 0.30, 0.95],
            BirdSpecies::EmeraldDove => [0.35, 0.85, 0.65, 0.95],
            BirdSpecies::CelestialRaven => [0.75, 0.45, 0.98, 0.98],
            BirdSpecies::RubyTanager => [0.98, 0.26, 0.32, 0.98],
            BirdSpecies::SnowyOwl => [0.95, 0.96, 1.0, 0.98],
        }
    }

    fn draw_style_switcher_overlay(&mut self, style: u8, style_name: &str) {
        let scale = (self.height / 900.0).clamp(0.65, 1.6);
        let char_size = [9.5 * scale, 16.0 * scale];
        let step_x = 7.8 * scale;
        let label = if self.interactive_mode {
            format!(
                "[ STYLE {}/{}: {} > CLICK TO CYCLE ]",
                style + 1,
                ROOKERY_STYLE_COUNT,
                style_name
            )
        } else {
            format!("[ {}/{}: {} ]", style + 1, ROOKERY_STYLE_COUNT, style_name)
        };

        let is_light_bg = style == 5;
        let bg_col = if is_light_bg {
            [0.18, 0.15, 0.12, 0.82]
        } else {
            [0.04, 0.08, 0.06, 0.78]
        };
        let fg_col = if is_light_bg {
            [0.96, 0.92, 0.84, 0.96]
        } else if style == 4 {
            [1.0, 0.72, 0.12, 0.95]
        } else {
            [0.45, 0.95, 0.72, 0.92]
        };

        let start_x = 18.0 * scale;
        let start_y = 22.0 * scale;

        // Backing bar for readability
        let pill_chars = label.chars().count();
        for i in 0..pill_chars {
            self.push_glyph(
                '█',
                [start_x + (i as f32) * step_x, start_y],
                [step_x * 1.15, char_size[1] * 1.25],
                0.0,
                0.90,
                bg_col,
                0.0,
            );
        }
        self.push_text(
            &label,
            [start_x, start_y],
            char_size,
            step_x,
            0.92,
            fg_col,
            0.0,
        );
    }

    fn generate_scene_instances(&mut self) {
        self.glyphs.clear();

        let world_arc = self.world.clone();
        let world_snapshot = match world_arc.lock() {
            Ok(w) => w.clone(),
            Err(_) => return,
        };

        let style = world_snapshot.aesthetic_style % ROOKERY_STYLE_COUNT;
        let style_name = world_snapshot.aesthetic_style_name();

        match style {
            1 => self.generate_style_1_roguelike(&world_snapshot),
            2 => self.generate_style_2_matrix_dither(&world_snapshot),
            3 => self.generate_style_3_multiline_art(&world_snapshot),
            4 => self.generate_style_4_crt_phosphor(&world_snapshot),
            5 => self.generate_style_5_sumie_journal(&world_snapshot),
            6 => self.generate_style_6_stained_glass(&world_snapshot),
            7 => self.generate_style_7_volumetric_3d(&world_snapshot),
            _ => self.generate_style_0_classic(&world_snapshot),
        }

        self.draw_style_switcher_overlay(style, style_name);
    }

    // =========================================================================
    // STYLE 0: CLASSIC SDF WOODLAND (Current Baseline)
    // =========================================================================
    fn generate_style_0_classic(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let base_glyph_size = [12.0 * tree_scale, 20.0 * tree_scale];

        // 1. Draw Tree Branches with Multi-Column Bark Textures & Base Buttress Roots
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let current_end_world = branch.current_end();
            let end = self.world_to_screen(current_end_world, offset_x);

            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len = (dx * dx + dy * dy).sqrt();
            let steps = (len / (base_glyph_size[1] * 0.52)).ceil().max(1.0) as usize;

            let angle_deg = branch.angle.to_degrees();
            let branch_char = if branch.depth == 0 {
                '#'
            } else if angle_deg.abs() < 18.0 {
                '|'
            } else if angle_deg < 0.0 {
                '/'
            } else {
                '\\'
            };

            let bark_color = if branch.depth == 0 {
                [0.48, 0.36, 0.26, 0.95]
            } else if branch.depth <= 2 {
                [0.55, 0.42, 0.30, 0.90]
            } else {
                [0.45, 0.50, 0.35, 0.85]
            };

            let perp = if len > 0.01 {
                [-dy / len, dx / len]
            } else {
                [1.0, 0.0]
            };

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let px = start[0] + dx * t;
                let py = start[1] + dy * t;
                let glyph_scale = (branch.thickness / 8.0).clamp(1.0, 2.6);

                self.push_glyph(
                    branch_char,
                    [px, py],
                    [
                        base_glyph_size[0] * glyph_scale,
                        base_glyph_size[1] * glyph_scale,
                    ],
                    branch.angle * 0.4,
                    branch.depth as f32 * 0.1,
                    bark_color,
                    branch.sway_factor,
                );

                if branch.depth == 0 {
                    let col_inner = 7.0 * tree_scale;
                    let col_outer = 14.0 * tree_scale;

                    self.push_glyph(
                        '|',
                        [px - col_inner, py],
                        [base_glyph_size[0] * 0.9, base_glyph_size[1]],
                        branch.angle * 0.3,
                        0.05,
                        [0.42, 0.32, 0.22, 0.90],
                        branch.sway_factor,
                    );
                    self.push_glyph(
                        '|',
                        [px + col_inner, py],
                        [base_glyph_size[0] * 0.9, base_glyph_size[1]],
                        branch.angle * 0.3,
                        0.05,
                        [0.42, 0.32, 0.22, 0.90],
                        branch.sway_factor,
                    );

                    self.push_glyph(
                        '!',
                        [px - col_outer, py],
                        [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.95],
                        branch.angle * 0.2,
                        0.05,
                        [0.38, 0.28, 0.18, 0.85],
                        branch.sway_factor,
                    );
                    self.push_glyph(
                        '!',
                        [px + col_outer, py],
                        [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.95],
                        branch.angle * 0.2,
                        0.05,
                        [0.38, 0.28, 0.18, 0.85],
                        branch.sway_factor,
                    );

                    if t > 0.65 {
                        let flare = ((t - 0.65) / 0.35) * 18.0 * tree_scale;
                        let root_moss_col = [0.35, 0.42, 0.25, 0.92];
                        self.push_glyph(
                            '/',
                            [px - col_outer - flare, py],
                            [base_glyph_size[0] * 0.85, base_glyph_size[1]],
                            -0.25,
                            0.04,
                            root_moss_col,
                            branch.sway_factor * 0.5,
                        );
                        self.push_glyph(
                            '\\',
                            [px + col_outer + flare, py],
                            [base_glyph_size[0] * 0.85, base_glyph_size[1]],
                            0.25,
                            0.04,
                            root_moss_col,
                            branch.sway_factor * 0.5,
                        );
                    }
                } else if branch.depth == 1 || branch.thickness >= 7.0 {
                    let p_off = (branch.thickness * 0.38).clamp(3.5, 7.5) * tree_scale;
                    let side_col = [0.46, 0.35, 0.24, 0.85];
                    self.push_glyph(
                        if branch_char == '#' { '|' } else { branch_char },
                        [px - perp[0] * p_off, py - perp[1] * p_off],
                        [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.85],
                        branch.angle * 0.4,
                        branch.depth as f32 * 0.1,
                        side_col,
                        branch.sway_factor,
                    );
                    self.push_glyph(
                        if branch_char == '#' { '|' } else { branch_char },
                        [px + perp[0] * p_off, py + perp[1] * p_off],
                        [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.85],
                        branch.angle * 0.4,
                        branch.depth as f32 * 0.1,
                        side_col,
                        branch.sway_factor,
                    );
                }
            }

            // 2. Draw Foliage along branch
            for bud in &branch.foliage {
                if bud.bloom <= 0.05 {
                    continue;
                }
                let pt = branch.point_at(bud.offset_along);
                let mut screen_pt = self.world_to_screen(pt, offset_x);
                screen_pt[0] += bud.lateral_offset * tree_scale;

                let c = bud.char_code as char;
                let leaf_color = match bud.color_idx {
                    0 => [0.22, 0.68, 0.38, 0.88 * bud.bloom],
                    1 => [0.35, 0.78, 0.45, 0.85 * bud.bloom],
                    2 => [0.82, 0.65, 0.22, 0.90 * bud.bloom],
                    _ => [0.92, 0.52, 0.68, 0.92 * bud.bloom],
                };

                let leaf_size = [
                    base_glyph_size[0] * bud.scale * bud.bloom,
                    base_glyph_size[1] * bud.scale * bud.bloom,
                ];

                self.push_glyph(
                    c,
                    screen_pt,
                    leaf_size,
                    0.0,
                    0.15,
                    leaf_color,
                    branch.sway_factor * 1.2,
                );
            }
        }

        // 3. Nests, Birds, Notes, Chips, Houses, Infrastructure, Signs, Floor
        self.draw_common_props_classic(world_guard, offset_x, tree_scale, base_glyph_size);
    }

    fn draw_common_props_classic(
        &mut self,
        world_guard: &WorldState,
        offset_x: f32,
        tree_scale: f32,
        base_glyph_size: [f32; 2],
    ) {
        for nest in &world_guard.nests {
            if let Some(b) = world_guard
                .branches
                .iter()
                .find(|br| br.id == nest.branch_id)
            {
                let nest_pos = self.world_to_screen(b.point_at(nest.t), offset_x);
                let nest_col = [0.65, 0.52, 0.35, 0.95];

                self.push_glyph(
                    '\\',
                    [nest_pos[0] - 8.0 * tree_scale, nest_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.06,
                    nest_col,
                    b.sway_factor,
                );
                self.push_glyph(
                    '=',
                    [nest_pos[0], nest_pos[1] + 2.0 * tree_scale],
                    base_glyph_size,
                    0.0,
                    0.06,
                    nest_col,
                    b.sway_factor,
                );
                self.push_glyph(
                    '/',
                    [nest_pos[0] + 8.0 * tree_scale, nest_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.06,
                    nest_col,
                    b.sway_factor,
                );

                if nest.chicks_count > 0 {
                    self.push_glyph(
                        'v',
                        [
                            nest_pos[0] - 3.5 * tree_scale,
                            nest_pos[1] - 4.0 * tree_scale,
                        ],
                        [base_glyph_size[0] * 0.75, base_glyph_size[1] * 0.75],
                        0.0,
                        0.07,
                        [0.98, 0.88, 0.30, 0.95],
                        b.sway_factor,
                    );
                } else if nest.eggs_count > 0 {
                    self.push_glyph(
                        'o',
                        [nest_pos[0], nest_pos[1] - 2.5 * tree_scale],
                        [base_glyph_size[0] * 0.65, base_glyph_size[1] * 0.65],
                        0.0,
                        0.07,
                        [0.95, 0.95, 0.90, 0.95],
                        b.sway_factor,
                    );
                }
            }
        }

        for bird in &world_guard.birds {
            let screen_pos = self.world_to_screen(bird.pos, offset_x);
            let bird_color = Self::species_color(bird.species);
            let b_scale = bird.size_scale * tree_scale;
            let b_size = [base_glyph_size[0] * b_scale, base_glyph_size[1] * b_scale];

            match &bird.state {
                BirdState::Perched { .. }
                | BirdState::Feeding { .. }
                | BirdState::GatheringTwig { .. }
                | BirdState::BuildingNest { .. }
                | BirdState::BroodingEggs { .. }
                | BirdState::Bathing { .. } => {
                    let head_char = if bird.facing_left { '<' } else { '>' };
                    let tail_char = if bird.facing_left { '/' } else { '\\' };
                    let ox = if bird.facing_left {
                        -6.0 * b_scale
                    } else {
                        6.0 * b_scale
                    };
                    self.push_glyph(
                        head_char,
                        [screen_pos[0] + ox, screen_pos[1] - 4.0],
                        b_size,
                        0.0,
                        0.2,
                        bird_color,
                        0.0,
                    );
                    self.push_glyph('o', screen_pos, b_size, 0.0, 0.2, bird_color, 0.0);
                    self.push_glyph(
                        tail_char,
                        [screen_pos[0] - ox, screen_pos[1] + 4.0],
                        b_size,
                        0.0,
                        0.2,
                        bird_color,
                        0.0,
                    );
                }
                _ => {
                    let flap = bird.wing_phase.sin();
                    let (wing_left, body, wing_right) = if flap > 0.3 {
                        ('^', 'v', '^')
                    } else if flap < -0.3 {
                        ('v', '-', 'v')
                    } else {
                        ('-', 'o', '-')
                    };
                    let span = 8.0 * b_scale;
                    self.push_glyph(
                        wing_left,
                        [screen_pos[0] - span, screen_pos[1]],
                        b_size,
                        0.0,
                        0.2,
                        bird_color,
                        0.0,
                    );
                    self.push_glyph(body, screen_pos, b_size, 0.0, 0.2, bird_color, 0.0);
                    self.push_glyph(
                        wing_right,
                        [screen_pos[0] + span, screen_pos[1]],
                        b_size,
                        0.0,
                        0.2,
                        bird_color,
                        0.0,
                    );
                }
            }
        }

        for note in &world_guard.notes {
            let screen_pos = self.world_to_screen(note.pos, offset_x);
            let alpha = (1.0 - (note.life / note.max_life)).clamp(0.0, 1.0);
            self.push_glyph(
                note.glyph,
                screen_pos,
                [base_glyph_size[0] * 1.1, base_glyph_size[1] * 1.1],
                0.0,
                0.3,
                [0.95, 0.85, 0.40, alpha],
                0.0,
            );
        }

        for chip in &world_guard.chips {
            let screen_pos = self.world_to_screen(chip.pos, offset_x);
            let alpha = (1.0 - (chip.life / chip.max_life)).clamp(0.0, 1.0);
            self.push_glyph(
                chip.glyph,
                screen_pos,
                base_glyph_size,
                chip.rotation,
                0.3,
                [0.72, 0.55, 0.38, alpha],
                0.0,
            );
        }

        for house in &world_guard.houses {
            if let Some(b) = world_guard
                .branches
                .iter()
                .find(|br| br.id == house.branch_id)
            {
                let h_pos = self.world_to_screen(b.point_at(house.t), offset_x);
                let house_color = [0.75, 0.55, 0.35, 0.95];
                self.push_glyph(
                    '^',
                    [h_pos[0], h_pos[1] - 12.0 * tree_scale],
                    base_glyph_size,
                    0.0,
                    0.08,
                    [0.85, 0.45, 0.35, 0.95],
                    b.sway_factor,
                );
                self.push_glyph(
                    '[',
                    [h_pos[0] - 6.0 * tree_scale, h_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.08,
                    house_color,
                    b.sway_factor,
                );
                self.push_glyph(
                    'o',
                    [h_pos[0], h_pos[1]],
                    [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.8],
                    0.0,
                    0.08,
                    [0.25, 0.18, 0.12, 0.95],
                    b.sway_factor,
                );
                self.push_glyph(
                    ']',
                    [h_pos[0] + 6.0 * tree_scale, h_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.08,
                    house_color,
                    b.sway_factor,
                );
            }
        }

        for lantern in &world_guard.lanterns {
            if let Some(b) = world_guard
                .branches
                .iter()
                .find(|br| br.id == lantern.branch_id)
            {
                let l_pos = self.world_to_screen(b.point_at(lantern.t), offset_x);
                self.push_glyph(
                    '|',
                    [l_pos[0], l_pos[1] + 6.0 * tree_scale],
                    [base_glyph_size[0] * 0.7, base_glyph_size[1] * 0.7],
                    0.0,
                    0.07,
                    [0.55, 0.48, 0.35, 0.90],
                    b.sway_factor,
                );
                self.push_glyph(
                    'o',
                    [l_pos[0], l_pos[1] + 14.0 * tree_scale],
                    [base_glyph_size[0] * 0.75, base_glyph_size[1] * 0.75],
                    0.0,
                    0.07,
                    [1.0, 0.92, 0.50, 0.98],
                    b.sway_factor,
                );
                self.push_glyph(
                    '*',
                    [l_pos[0], l_pos[1] + 14.0 * tree_scale],
                    [base_glyph_size[0] * 1.5, base_glyph_size[1] * 1.5],
                    0.0,
                    0.06,
                    [1.0, 0.85, 0.40, 0.45],
                    b.sway_factor,
                );
            }
        }

        for bath in &world_guard.bird_baths {
            let bb_pos = self.world_to_screen(bath.pos, offset_x);
            let stone_col = [0.65, 0.68, 0.72, 0.95];
            let water_col = [0.35, 0.78, 0.96, 0.90];
            self.push_glyph(
                '|',
                [bb_pos[0], bb_pos[1] + 8.0 * tree_scale],
                base_glyph_size,
                0.0,
                0.02,
                stone_col,
                0.0,
            );
            self.push_glyph(
                '(',
                [bb_pos[0] - 10.0 * tree_scale, bb_pos[1]],
                base_glyph_size,
                0.0,
                0.02,
                stone_col,
                0.0,
            );
            self.push_glyph(
                '~',
                [bb_pos[0], bb_pos[1] - 1.0 * tree_scale],
                base_glyph_size,
                0.0,
                0.03,
                water_col,
                0.0,
            );
            self.push_glyph(
                ')',
                [bb_pos[0] + 10.0 * tree_scale, bb_pos[1]],
                base_glyph_size,
                0.0,
                0.02,
                stone_col,
                0.0,
            );
        }

        for perch in &world_guard.perches {
            let p_pos = self.world_to_screen(perch.pos, offset_x);
            let timber_col = [0.58, 0.44, 0.30, 0.95];
            self.push_glyph(
                '|',
                [p_pos[0], p_pos[1] + 9.0 * tree_scale],
                base_glyph_size,
                0.0,
                0.02,
                timber_col,
                0.0,
            );
            self.push_glyph(
                '=',
                [p_pos[0], p_pos[1]],
                base_glyph_size,
                0.0,
                0.02,
                timber_col,
                0.0,
            );
        }

        for snack in &world_guard.snacks {
            if let Some(b) = world_guard
                .branches
                .iter()
                .find(|br| br.id == snack.branch_id)
            {
                let sn_pos = self.world_to_screen(b.point_at(snack.t), offset_x);
                let (snack_char, snack_col) = match snack.snack_type {
                    SnackType::Seeds => ('.', [0.95, 0.85, 0.35, 0.95]),
                    SnackType::SweetBerries => ('8', [0.98, 0.28, 0.35, 0.95]),
                    SnackType::StarNectar => ('*', [0.70, 0.88, 1.0, 0.98]),
                };
                self.push_glyph(
                    snack_char,
                    [sn_pos[0], sn_pos[1] - 4.0 * tree_scale],
                    [base_glyph_size[0] * 0.9, base_glyph_size[1] * 0.9],
                    0.0,
                    0.08,
                    snack_col,
                    b.sway_factor,
                );
            }
        }

        for sign in &world_guard.signs {
            let s_pos = self.world_to_screen(sign.pos, offset_x);
            let sign_col = [0.72, 0.54, 0.32, 0.95];
            self.push_glyph(
                '|',
                [s_pos[0], s_pos[1] + 8.0 * tree_scale],
                base_glyph_size,
                0.0,
                0.02,
                [0.48, 0.38, 0.24, 0.95],
                0.0,
            );
            self.push_glyph(
                '[',
                [s_pos[0] - 6.0 * tree_scale, s_pos[1]],
                [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.8],
                0.0,
                0.02,
                sign_col,
                0.0,
            );
            self.push_glyph(
                'i',
                [s_pos[0], s_pos[1]],
                [base_glyph_size[0] * 0.7, base_glyph_size[1] * 0.7],
                0.0,
                0.02,
                [0.98, 0.92, 0.75, 0.98],
                0.0,
            );
            self.push_glyph(
                ']',
                [s_pos[0] + 6.0 * tree_scale, s_pos[1]],
                [base_glyph_size[0] * 0.8, base_glyph_size[1] * 0.8],
                0.0,
                0.02,
                sign_col,
                0.0,
            );

            let age_str = sign.age_display(world_guard.total_age_secs);
            let inscription = format!("{} ({})", sign.label, age_str);
            self.push_text(
                &inscription,
                [s_pos[0] + 16.0 * tree_scale, s_pos[1]],
                [base_glyph_size[0] * 0.65, base_glyph_size[1] * 0.65],
                7.5 * tree_scale,
                0.02,
                [0.90, 0.82, 0.62, 0.92],
                0.0,
            );
        }

        let ground_y = self.height * 0.90;
        let ground_chars = ['"', ',', '.', '\'', '~', '☘', '%', '^', 'o', '*'];
        let items_count = 48;
        for i in 0..items_count {
            let fx = (i as f32 / items_count as f32) * self.width;
            let c = ground_chars[i % ground_chars.len()];
            let item_col = match i % 5 {
                0 => [0.24, 0.68, 0.32, 0.80],
                1 => [0.38, 0.75, 0.42, 0.75],
                2 => [0.88, 0.32, 0.25, 0.90],
                3 => [0.72, 0.76, 0.80, 0.75],
                _ => [0.55, 0.48, 0.32, 0.70],
            };
            let y_wiggle = ((i as f32 * 1.7).sin() * 2.5) * tree_scale;
            self.push_glyph(
                c,
                [fx, ground_y + 2.0 * tree_scale + y_wiggle],
                [base_glyph_size[0] * 0.72, base_glyph_size[1] * 0.72],
                0.0,
                0.01,
                item_col,
                0.04,
            );
        }
    }

    // =========================================================================
    // STYLE 1: STRICT ANSI ROGUELIKE GRID (Dwarf Fortress / Caves of Qud)
    // =========================================================================
    fn generate_style_1_roguelike(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let cell_w = 12.0f32;
        let cell_h = 20.0f32;
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

        // 1. Subterranean Dwarf-Fortress soil, roots & crystal strata
        for r in ground_row..rows.saturating_sub(1) {
            for c in 1..cols.saturating_sub(1) {
                let hash =
                    (c as u32).wrapping_mul(73_856_093) ^ (r as u32).wrapping_mul(19_349_663);
                if r == ground_row {
                    let turf_ch = match hash % 4 {
                        0 => '☘',
                        1 => '"',
                        2 => '▀',
                        _ => ',',
                    };
                    self.push_glyph(
                        turf_ch,
                        cell_pos(c, r),
                        cell_size,
                        0.0,
                        0.05,
                        [0.25, 0.85, 0.42, 0.95],
                        0.0,
                    );
                } else {
                    let soil_ch = match (r - ground_row, hash % 9) {
                        (1, 0..=4) => '▓',
                        (1, _) => '▒',
                        (_, 0) => '◆',
                        (_, 1..=3) => '▒',
                        _ => '░',
                    };
                    let col = if soil_ch == '◆' {
                        [0.35, 0.85, 0.95, 0.90]
                    } else {
                        [0.42, 0.30, 0.20, 0.65]
                    };
                    self.push_glyph(soil_ch, cell_pos(c, r), cell_size, 0.0, 0.04, col, 0.0);
                }
            }
        }

        // 2. Stepped Monospace Box-Drawing Tree Boughs
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let c0 = (start[0] / cell_w).round() as i32;
            let r0 = (start[1] / cell_h).round() as i32;
            let c1 = (end[0] / cell_w).round() as i32;
            let r1 = (end[1] / cell_h).round() as i32;

            let steps = (c1 - c0).abs().max((r1 - r0).abs()).max(1);
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cc = (c0 as f32 + (c1 - c0) as f32 * t).round() as i32;
                let rr = (r0 as f32 + (r1 - r0) as f32 * t).round() as i32;
                let pos = cell_pos(cc, rr);

                if branch.depth == 0 {
                    // Wide ANSI trunk with colored background cell tiles
                    for dx in -1..=1 {
                        let tp = cell_pos(cc + dx, rr);
                        self.push_glyph(
                            '█',
                            tp,
                            cell_size,
                            0.0,
                            0.08,
                            [0.24, 0.16, 0.10, 0.92],
                            0.0,
                        );
                        let ch = if dx == 0 {
                            '║'
                        } else if dx < 0 {
                            '├'
                        } else {
                            '┤'
                        };
                        self.push_glyph(
                            ch,
                            tp,
                            cell_size,
                            0.0,
                            0.10,
                            [0.76, 0.56, 0.36, 0.98],
                            0.0,
                        );
                    }
                } else {
                    let dc = (c1 - c0).abs();
                    let dr = (r1 - r0).abs();
                    let box_ch = if s == steps {
                        '┼'
                    } else if dc > dr * 2 {
                        '═'
                    } else if dr > dc * 2 {
                        '║'
                    } else if (c1 - c0) * (r1 - r0) < 0 {
                        '┌'
                    } else {
                        '┐'
                    };
                    self.push_glyph(
                        '█',
                        pos,
                        cell_size,
                        0.0,
                        0.09,
                        [0.18, 0.13, 0.08, 0.85],
                        0.0,
                    );
                    self.push_glyph(
                        box_ch,
                        pos,
                        cell_size,
                        0.0,
                        0.11,
                        [0.68, 0.50, 0.32, 0.95],
                        0.0,
                    );
                }
            }

            // 3. Roguelike Tile Canopy (5-cell diamond cluster of background cell fills + foreground CP437 glyphs)
            for (idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.05 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset;
                let center_c = (pt[0] / cell_w).floor() as i32;
                let center_r = (pt[1] / cell_h).floor() as i32;

                let (bg_col, fg_col, tile_char) = match bud.color_idx {
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
                };

                let offsets = [(0, 0), (-1, 0), (1, 0), (0, -1)];
                for (oi, &(dc, dr)) in offsets.iter().enumerate() {
                    if oi > 0 && idx % 2 != 0 {
                        continue;
                    }
                    let cp = cell_pos(center_c + dc, center_r + dr);
                    let ch = if oi == 0 { tile_char } else { '░' };
                    self.push_glyph('█', cp, cell_size, 0.0, 0.14, bg_col, 0.0);
                    self.push_glyph(ch, cp, cell_size, 0.0, 0.16, fg_col, 0.0);
                }
            }
        }

        // 4. Grid-Snapped Birds & Entities
        for bird in &world_guard.birds {
            let p = snap(self.world_to_screen(bird.pos, offset_x));
            let col = Self::species_color(bird.species);
            self.push_glyph('█', p, cell_size, 0.0, 0.22, [0.08, 0.12, 0.18, 0.92], 0.0);
            let bird_glyph = match bird.state {
                BirdState::Perched { .. } | BirdState::Feeding { .. } => '@',
                _ => {
                    if bird.wing_phase.sin() >= 0.0 {
                        '^'
                    } else {
                        'v'
                    }
                }
            };
            self.push_glyph(bird_glyph, p, cell_size, 0.0, 0.25, col, 0.0);
        }

        // 5. ANSI Double-Line Terminal Border & Status Bar at Bottom
        let border_col = [0.28, 0.72, 0.58, 0.92];
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
                border_col,
                0.0,
            );
        }
        let status_str = format!(
            "╡ FORT SYLVAN · SEED #{} · BOUGHS:{} · BUDS:{} · FLOCK:{} ╞",
            world_guard.seed,
            world_guard.stats.total_branches,
            world_guard.stats.total_foliage,
            world_guard.birds.len()
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
    }

    // =========================================================================
    // STYLE 2: FULL-SCREEN ASCII MATRIX & DEMOSCENE DITHERING
    // =========================================================================
    fn generate_style_2_matrix_dither(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let cell_w = (14.0 * tree_scale).max(10.0);
        let cell_h = (20.0 * tree_scale).max(14.0);
        let cols = (self.width / cell_w).ceil() as usize;
        let rows = (self.height / cell_h).ceil() as usize;

        // Collect screen-space branch segments and foliage centers for fast field evaluation
        let mut segments = Vec::with_capacity(world_guard.branches.len());
        let mut foliage_pts = Vec::new();
        for b in &world_guard.branches {
            let s = self.world_to_screen(b.start, offset_x);
            let e = self.world_to_screen(b.current_end(), offset_x);
            segments.push((s, e, b.thickness * tree_scale, b.depth));
            for bud in &b.foliage {
                if bud.bloom > 0.1 {
                    let mut fp = self.world_to_screen(b.point_at(bud.offset_along), offset_x);
                    fp[0] += bud.lateral_offset * tree_scale;
                    foliage_pts.push((
                        fp,
                        34.0 * bud.scale * bud.bloom * tree_scale,
                        bud.color_idx,
                    ));
                }
            }
        }

        let moon_px = [self.width * 0.78, self.height * 0.20];
        let moon_r = 52.0 * tree_scale;
        let ramp: [char; 10] = ['.', '∙', ':', '-', '=', '+', '*', '#', '%', '@'];
        let t = self.time as f32;

        for r in 0..rows {
            let py = (r as f32 + 0.5) * cell_h;
            let uv_y = py / self.height;
            for c in 0..cols {
                let px = (c as f32 + 0.5) * cell_w;
                let uv_x = px / self.width;

                // 1. Check Canopy Metaball Field
                let mut accum_foliage = 0.0f32;
                let mut best_w = 0.0f32;
                let mut best_color_idx = 0u8;
                for &(fp, rad, cidx) in &foliage_pts {
                    let dx = px - fp[0];
                    let dy = py - fp[1];
                    let d2 = dx * dx + dy * dy;
                    if d2 < rad * rad {
                        let w = 1.0 - (d2.sqrt() / rad);
                        accum_foliage += w * 0.65;
                        if w > best_w {
                            best_w = w;
                            best_color_idx = cidx;
                        }
                    }
                }
                let best_foliage = accum_foliage.min(1.0);

                // 2. Check Branch Distance Field
                let mut branch_hit = 0.0f32;
                for &(s, e, thick, _depth) in &segments {
                    let vx = e[0] - s[0];
                    let vy = e[1] - s[1];
                    let len2 = (vx * vx + vy * vy).max(1.0);
                    let proj = (((px - s[0]) * vx + (py - s[1]) * vy) / len2).clamp(0.0, 1.0);
                    let nx = s[0] + vx * proj;
                    let ny = s[1] + vy * proj;
                    let dist = ((px - nx).powi(2) + (py - ny).powi(2)).sqrt();
                    let radius = (thick * 1.15).max(10.0 * tree_scale);
                    if dist < radius {
                        branch_hit = branch_hit.max(1.0 - dist / radius);
                    }
                }

                // Moon illumination directional factor
                let dir_to_moon_x = (moon_px[0] - px).signum();

                if best_foliage > 0.12 {
                    let rim_light =
                        ((px - self.width * 0.5) * dir_to_moon_x * 0.002).clamp(0.0, 0.35);
                    let luma = (best_foliage * 0.78 + rim_light).clamp(0.15, 0.99);
                    let idx = ((luma * 9.0).round() as usize).min(9);
                    let col = match best_color_idx {
                        0 => [0.18, 0.85 * luma + 0.15, 0.48 * luma, 0.95],
                        1 => [0.35 * luma, 0.95 * luma + 0.05, 0.55 * luma, 0.95],
                        2 => [0.95 * luma, 0.78 * luma, 0.25, 0.95],
                        _ => [0.95 * luma, 0.52 * luma, 0.75 * luma, 0.95],
                    };
                    self.push_glyph(ramp[idx], [px, py], [cell_w, cell_h], 0.0, 0.16, col, 0.08);
                } else if branch_hit > 0.10 {
                    let luma = (0.35 + branch_hit * 0.62).clamp(0.2, 0.98);
                    let idx = ((luma * 8.0).round() as usize).min(9);
                    self.push_glyph(
                        ramp[idx],
                        [px, py],
                        [cell_w, cell_h],
                        0.0,
                        0.12,
                        [0.72 * luma, 0.52 * luma, 0.34 * luma, 0.95],
                        0.04,
                    );
                } else {
                    // Sky, Moon, Clouds & Ground Matrix Dithering
                    let d_moon = ((px - moon_px[0]).powi(2) + (py - moon_px[1]).powi(2)).sqrt();
                    let ground_h = 0.89 + 0.015 * (uv_x * 6.0).sin();

                    if d_moon < moon_r {
                        let m_luma = (1.0 - d_moon / moon_r).sqrt();
                        let idx = (5 + (m_luma * 4.0) as usize).min(9);
                        self.push_glyph(
                            ramp[idx],
                            [px, py],
                            [cell_w, cell_h],
                            0.0,
                            0.04,
                            [0.98, 0.95, 0.78, 0.95],
                            0.0,
                        );
                    } else if uv_y > ground_h {
                        let depth_ground = ((uv_y - ground_h) / 0.11).clamp(0.0, 1.0);
                        let wave = ((uv_x * 18.0 + r as f32 * 1.3).sin() * 0.5 + 0.5)
                            * (1.0 - depth_ground * 0.6);
                        let idx = ((wave * 6.0) as usize).min(9);
                        self.push_glyph(
                            ramp[idx],
                            [px, py],
                            [cell_w, cell_h],
                            0.0,
                            0.05,
                            [0.18, 0.58 * (1.0 - depth_ground * 0.5), 0.32, 0.85],
                            0.0,
                        );
                    } else {
                        // Flowing aurora / cloud dither waves + moon halo
                        let cloud1 = (uv_x * 7.5 - t * 0.18 + (uv_y * 5.0).sin()).sin() * 0.5 + 0.5;
                        let cloud2 = (uv_x * 13.0 + t * 0.11 - uv_y * 8.0).cos() * 0.5 + 0.5;
                        let halo = (1.0 - (d_moon / (moon_r * 4.5)).min(1.0)) * 0.45;
                        let sky_luma =
                            (cloud1 * cloud2 * 0.55 + halo + (1.0 - uv_y) * 0.12).clamp(0.0, 0.85);
                        if sky_luma > 0.14 {
                            let idx = ((sky_luma * 6.0) as usize).min(7);
                            self.push_glyph(
                                ramp[idx],
                                [px, py],
                                [cell_w * 0.9, cell_h * 0.9],
                                0.0,
                                0.02,
                                [
                                    0.22 + halo * 0.6,
                                    0.45 + sky_luma * 0.35,
                                    0.68 + sky_luma * 0.25,
                                    0.45 + sky_luma * 0.4,
                                ],
                                0.0,
                            );
                        }
                    }
                }
            }
        }

        // High-contrast foreground birds & notes
        for bird in &world_guard.birds {
            let p = self.world_to_screen(bird.pos, offset_x);
            let col = Self::species_color(bird.species);
            self.push_glyph('@', p, [cell_w * 1.25, cell_h * 1.25], 0.0, 0.30, col, 0.0);
            self.push_glyph(
                '^',
                [p[0] - cell_w, p[1]],
                [cell_w * 1.1, cell_h * 1.1],
                0.0,
                0.30,
                col,
                0.0,
            );
            self.push_glyph(
                '^',
                [p[0] + cell_w, p[1]],
                [cell_w * 1.1, cell_h * 1.1],
                0.0,
                0.30,
                col,
                0.0,
            );
        }
    }

    // =========================================================================
    // STYLE 3: MULTI-LINE CLASSIC ASCII ART & PARALLAX LANDSCAPE
    // =========================================================================
    fn generate_style_3_multiline_art(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let cw = 10.0 * tree_scale;
        let ch = 17.0 * tree_scale;
        let csize = [cw, ch];
        let ground_y = self.height * 0.895;

        // 1. Multi-Row ASCII Moon Stamp in Upper Sky
        let moon_art = [
            "   _.._   ",
            " .' .-'`  ",
            "/  /      ",
            "|  |      ",
            "\\  \\___   ",
            " '._  _.' ",
            "    ``    ",
        ];
        let moon_x = self.width * 0.80 - offset_x * 0.08;
        let moon_y = self.height * 0.10;
        for (row_idx, line) in moon_art.iter().enumerate() {
            self.push_text(
                line,
                [moon_x, moon_y + row_idx as f32 * ch],
                csize,
                cw,
                0.02,
                [0.98, 0.94, 0.72, 0.90],
                0.0,
            );
        }

        // 2. Drifting Multi-Line ASCII Clouds
        let cloud_stamp = ["   .--.   ", ".-(    ). ", "(___.__)__)"];
        for i in 0..4 {
            let cx = ((i as f32 * 380.0 + self.time as f32 * 8.0 - offset_x * 0.15)
                .rem_euclid(self.width + 200.0))
                - 100.0;
            let cy = self.height * (0.14 + (i % 2) as f32 * 0.09);
            for (r, line) in cloud_stamp.iter().enumerate() {
                self.push_text(
                    line,
                    [cx, cy + r as f32 * ch],
                    csize,
                    cw,
                    0.03,
                    [0.72, 0.78, 0.92, 0.65],
                    0.0,
                );
            }
        }

        // 3. Multi-Row Parallax ASCII Mountain Range along the Horizon
        let mountain_peaks = [
            "      /\\          /^^\\        /\\      ",
            "     /  \\    /\\  /    \\      /^^\\     ",
            "    / /\\ \\  /  \\/  /\\  \\    /    \\    ",
            "   / /  \\ \\/    \\ /  \\  \\  /  /\\  \\   ",
            "__/ /____\\ \\_____/____\\__\\/__/  \\__\\__",
        ];
        let mtn_block_w = 38.0 * cw;
        let repeats = (self.width / mtn_block_w).ceil() as i32 + 2;
        let mtn_parallax = (-offset_x * 0.25).rem_euclid(mtn_block_w);
        for rep in -1..repeats {
            let base_x = rep as f32 * mtn_block_w + mtn_parallax;
            for (r, line) in mountain_peaks.iter().enumerate() {
                let y = ground_y - (mountain_peaks.len() as f32 - r as f32) * ch;
                self.push_text(
                    line,
                    [base_x, y],
                    csize,
                    cw,
                    0.04,
                    [0.35, 0.38, 0.56, 0.65],
                    0.0,
                );
            }
        }

        // 4. Structured Multi-Character ASCII Trunk & Boughs
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let steps = ((dx * dx + dy * dy).sqrt() / (ch * 0.75)).ceil().max(1.0) as usize;

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let px = start[0] + dx * t;
                let py = start[1] + dy * t;
                if branch.depth == 0 {
                    let row_str = if s > steps * 2 / 3 {
                        "/| || || |\\"
                    } else {
                        " | || || | "
                    };
                    self.push_text(
                        row_str,
                        [px - 5.0 * cw, py],
                        csize,
                        cw,
                        0.10,
                        [0.68, 0.50, 0.34, 0.96],
                        branch.sway_factor,
                    );
                } else if branch.depth <= 2 {
                    let b_str = if branch.angle < -0.25 {
                        "\\\\||"
                    } else if branch.angle > 0.25 {
                        "||//"
                    } else {
                        "|/\\|"
                    };
                    self.push_text(
                        b_str,
                        [px - 1.5 * cw, py],
                        csize,
                        cw,
                        0.11,
                        [0.62, 0.46, 0.30, 0.94],
                        branch.sway_factor,
                    );
                }
            }

            // 5. Multi-Row Woven ASCII Canopy Crown Stamps
            for (idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.1 || idx % 3 != 0 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset * tree_scale;

                let canopy_stamp = [
                    "  _.-*~*-._  ",
                    " (  * @ #  ) ",
                    "( # & * % @ )",
                    " `-._____.-' ",
                ];
                let col = match bud.color_idx {
                    0 => [0.26, 0.82, 0.45, 0.94],
                    1 => [0.42, 0.88, 0.54, 0.92],
                    2 => [0.92, 0.76, 0.28, 0.95],
                    _ => [0.95, 0.58, 0.75, 0.95],
                };
                for (r, line) in canopy_stamp.iter().enumerate() {
                    self.push_text(
                        line,
                        [pt[0] - 6.0 * cw, pt[1] + (r as f32 - 1.5) * ch * 0.85],
                        [cw * 0.9, ch * 0.9],
                        cw * 0.85,
                        0.16,
                        col,
                        branch.sway_factor,
                    );
                }
            }
        }

        // 6. Multi-Row ASCII Bird Sprites
        for bird in &world_guard.birds {
            let p = self.world_to_screen(bird.pos, offset_x);
            let col = Self::species_color(bird.species);
            match bird.state {
                BirdState::Perched { .. } | BirdState::Feeding { .. } => {
                    let sprite = [" _ ", "(v)", "//\\"];
                    for (r, line) in sprite.iter().enumerate() {
                        self.push_text(
                            line,
                            [p[0] - cw, p[1] + (r as f32 - 1.0) * ch * 0.8],
                            csize,
                            cw * 0.85,
                            0.25,
                            col,
                            0.0,
                        );
                    }
                }
                _ => {
                    let top = if bird.wing_phase.sin() >= 0.0 {
                        "\\^/"
                    } else {
                        "_v_"
                    };
                    self.push_text(top, [p[0] - cw, p[1]], csize, cw * 0.85, 0.25, col, 0.0);
                    self.push_text(
                        " ' ",
                        [p[0] - cw, p[1] + ch * 0.75],
                        csize,
                        cw * 0.85,
                        0.25,
                        col,
                        0.0,
                    );
                }
            }
        }

        // 7. Classic Text-Mode Meadow Horizon Line
        let cols_total = (self.width / cw).ceil() as usize;
        for c in 0..cols_total {
            let ch_ground = match c % 6 {
                0 => '☘',
                1 | 3 => '~',
                2 => '^',
                4 => '"',
                _ => '_',
            };
            self.push_glyph(
                ch_ground,
                [c as f32 * cw, ground_y + 4.0],
                csize,
                0.0,
                0.08,
                [0.36, 0.82, 0.48, 0.90],
                0.0,
            );
        }
    }

    // =========================================================================
    // STYLE 4: RETRO PHOSPHOR CRT / CYBER-BOTANICAL TERMINAL
    // =========================================================================
    fn generate_style_4_crt_phosphor(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let cw = 10.5 * tree_scale;
        let ch = 18.0 * tree_scale;
        let csize = [cw, ch];

        let amber_bright = [1.0, 0.72, 0.10, 0.96];
        let amber_dim = [0.68, 0.44, 0.06, 0.72];
        let phos_green = [0.22, 0.98, 0.45, 0.94];
        let phos_green_dim = [0.12, 0.58, 0.26, 0.65];

        // 1. CRT Oscilloscope Horizon Baseline & Coordinate Ticks
        let ground_y = self.height * 0.90;
        let tick_cols = (self.width / cw).ceil() as usize;
        for i in 0..tick_cols {
            let x = i as f32 * cw;
            let glyph = if i % 10 == 0 { '┼' } else { '─' };
            self.push_glyph(glyph, [x, ground_y], csize, 0.0, 0.05, amber_dim, 0.0);
        }

        // 2. Vector Circuit Tree Boughs & Node Callouts
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len = (dx * dx + dy * dy).sqrt();
            let steps = (len / (ch * 0.55)).ceil().max(1.0) as usize;

            let trace_char = if branch.depth == 0 {
                '║'
            } else if branch.angle.abs() < 0.3 {
                '│'
            } else if branch.angle < 0.0 {
                '/'
            } else {
                '\\'
            };

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let px = start[0] + dx * t;
                let py = start[1] + dy * t;
                self.push_glyph(
                    trace_char,
                    [px, py],
                    csize,
                    branch.angle * 0.25,
                    0.10,
                    amber_bright,
                    branch.sway_factor * 0.5,
                );
                if branch.depth == 0 {
                    self.push_glyph(
                        '│',
                        [px - cw, py],
                        csize,
                        0.0,
                        0.09,
                        amber_dim,
                        branch.sway_factor * 0.5,
                    );
                    self.push_glyph(
                        '│',
                        [px + cw, py],
                        csize,
                        0.0,
                        0.09,
                        amber_dim,
                        branch.sway_factor * 0.5,
                    );
                }
            }

            // Vector Node Junction Box at Branch Tip
            self.push_text(
                "[+]",
                [end[0] - cw, end[1]],
                [cw * 0.85, ch * 0.85],
                cw * 0.8,
                0.14,
                amber_bright,
                0.0,
            );

            // Telemetry Leader Callouts on Primary Boughs
            if branch.depth == 1 {
                let tag = format!(
                    "───> BIO_NODE_{:02} [{:.0}%]",
                    branch.id,
                    branch.growth_progress * 100.0
                );
                self.push_text(
                    &tag,
                    [end[0] + cw * 1.5, end[1]],
                    [cw * 0.72, ch * 0.72],
                    cw * 0.68,
                    0.15,
                    phos_green,
                    0.0,
                );
            }

            // 3. Phosphor Point-Cloud Canopy
            for (idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.05 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset * tree_scale;
                let g = match idx % 4 {
                    0 => '✦',
                    1 => '+',
                    2 => '░',
                    _ => ':',
                };
                let col = if bud.color_idx >= 2 {
                    amber_bright
                } else if idx % 2 == 0 {
                    phos_green
                } else {
                    phos_green_dim
                };
                self.push_glyph(g, pt, csize, 0.0, 0.16, col, branch.sway_factor);
            }
        }

        // 4. Radar-Targeted Avian Entities with Trajectory Trails
        for bird in &world_guard.birds {
            let p = self.world_to_screen(bird.pos, offset_x);
            let reticle = format!("[>{}<] ID:{:02}", 'o', bird.id);
            self.push_text(
                &reticle,
                [p[0] - cw * 2.0, p[1]],
                [cw * 0.82, ch * 0.82],
                cw * 0.75,
                0.25,
                phos_green,
                0.0,
            );
            // Trailing radar dots behind velocity vector
            let vx = bird.vel[0].signum();
            for trail in 1..=4 {
                self.push_glyph(
                    '∙',
                    [p[0] - vx * (trail as f32 * cw * 1.4), p[1] + 2.0],
                    csize,
                    0.0,
                    0.20,
                    amber_dim,
                    0.0,
                );
            }
        }

        // 5. Cyber-Botanical CRT Telemetry HUD Banner at Bottom
        let hud_line = format!(
            "┌──[ CRT_VECTOR_BIOS // SEED:{:#X} // NODES:{} // FLOCK_TARGETS:{} // CLOCK:{:.1}s ]──┐",
            world_guard.seed,
            world_guard.stats.total_branches,
            world_guard.birds.len(),
            world_guard.total_age_secs
        );
        self.push_text(
            &hud_line,
            [24.0 * tree_scale, self.height - 24.0 * tree_scale],
            [cw * 0.78, ch * 0.78],
            cw * 0.72,
            0.50,
            amber_bright,
            0.0,
        );
    }

    // =========================================================================
    // STYLE 5: SUMI-E INK & BOTANICAL FIELD JOURNAL (Light Washi Parchment)
    // =========================================================================
    fn generate_style_5_sumie_journal(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let cw = 11.0 * tree_scale;
        let ch = 19.0 * tree_scale;
        let csize = [cw, ch];

        let sumi_dark = [0.10, 0.09, 0.08, 0.96];
        let sumi_mid = [0.25, 0.22, 0.19, 0.85];
        let sumi_wash = [0.45, 0.40, 0.35, 0.55];
        let sage_ink = [0.16, 0.36, 0.24, 0.88];
        let vermilion = [0.78, 0.20, 0.15, 0.94];

        // 1. Botanical Plate Header & Ruled Margin Frame
        let header = format!(
            "PLATE V. — ARBOR ROOKERIA (Specimen #{}) · Botanical Ink Study",
            world_guard.seed
        );
        self.push_text(
            &header,
            [self.width * 0.5 - 240.0 * tree_scale, 56.0 * tree_scale],
            [cw * 0.78, ch * 0.78],
            cw * 0.72,
            0.10,
            sumi_mid,
            0.0,
        );

        // 2. Calligraphic Dry-Brush Cross-Hatched Tree Trunk & Boughs
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len = (dx * dx + dy * dy).sqrt();
            let steps = (len / (ch * 0.42)).ceil().max(1.0) as usize;
            let perp = if len > 0.01 {
                [-dy / len, dx / len]
            } else {
                [1.0, 0.0]
            };

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let px = start[0] + dx * t;
                let py = start[1] + dy * t;
                let half_w = (branch.thickness * 0.65 * tree_scale).max(3.5);

                // Heavy Sumi ink contour on right (shadowed) edge
                self.push_glyph(
                    if branch.angle < -0.2 { '/' } else { '#' },
                    [px + perp[0] * half_w, py + perp[1] * half_w],
                    [cw * 1.15, ch * 1.15],
                    branch.angle * 0.5,
                    0.12,
                    sumi_dark,
                    branch.sway_factor,
                );
                // Center ink brush core
                self.push_glyph(
                    '|',
                    [px, py],
                    [cw * 1.1, ch * 1.1],
                    branch.angle * 0.4,
                    0.11,
                    sumi_dark,
                    branch.sway_factor,
                );
                // Dry-brush stipple contour on left (sunlit) edge
                self.push_glyph(
                    if s % 2 == 0 { '/' } else { ':' },
                    [px - perp[0] * half_w, py - perp[1] * half_w],
                    csize,
                    branch.angle * 0.5,
                    0.11,
                    sumi_mid,
                    branch.sway_factor,
                );
                // Cross-hatch interior shading strokes
                if branch.depth <= 1 {
                    let hatch = match s % 3 {
                        0 => '#',
                        1 => '~',
                        _ => ';',
                    };
                    self.push_glyph(
                        hatch,
                        [px + perp[0] * half_w * 0.4, py],
                        csize,
                        branch.angle * 0.3,
                        0.10,
                        sumi_wash,
                        branch.sway_factor,
                    );
                }
            }

            // 3. Stippled Ink & Vermilion Blossom Canopy
            for (idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.05 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset * tree_scale;

                let (ch_leaf, col_leaf) = match bud.color_idx {
                    0 => (if idx % 2 == 0 { '☘' } else { '*' }, sage_ink),
                    1 => (if idx % 2 == 0 { '@' } else { '~' }, sumi_dark),
                    2 => ('✦', [0.68, 0.42, 0.10, 0.94]),
                    _ => ('✦', vermilion),
                };
                self.push_glyph(
                    ch_leaf,
                    pt,
                    [cw * bud.scale * 1.25, ch * bud.scale * 1.25],
                    0.0,
                    0.16,
                    col_leaf,
                    branch.sway_factor,
                );
                // Ink stipple halo
                self.push_glyph(
                    ';',
                    [pt[0] + cw * 0.7, pt[1] + 3.0],
                    csize,
                    0.0,
                    0.15,
                    sumi_mid,
                    branch.sway_factor,
                );
            }
        }

        // Sumi-e Ink Horizon Brush Stroke across the Washi Floor
        let ground_y = self.height * 0.90;
        let floor_cols = (self.width / cw).ceil() as usize;
        for c in 0..floor_cols {
            let fx = c as f32 * cw;
            let wave = ((c as f32 * 0.25).sin() * 4.0) * tree_scale;
            let ch_ink = if c % 3 == 0 { '~' } else { '_' };
            self.push_glyph(
                ch_ink,
                [fx, ground_y + wave],
                [cw * 1.2, ch],
                0.0,
                0.08,
                sumi_dark,
                0.0,
            );
        }

        // 4. Brush-Stroke Ink Birds & Marginalia Labels
        for (idx, bird) in world_guard.birds.iter().enumerate() {
            let p = self.world_to_screen(bird.pos, offset_x);
            let bird_ink = if bird.species.is_rare() {
                vermilion
            } else {
                sumi_dark
            };
            let stroke = if bird.wing_phase.sin() >= 0.0 {
                "~^~"
            } else {
                "-v-"
            };
            self.push_text(
                stroke,
                [p[0] - cw, p[1]],
                csize,
                cw * 0.8,
                0.24,
                bird_ink,
                0.0,
            );
            if idx < 2 {
                let note = format!("· Fig. {}: {}", idx + 1, bird.species.display_name());
                self.push_text(
                    &note,
                    [p[0] + cw * 1.8, p[1]],
                    [cw * 0.65, ch * 0.65],
                    cw * 0.60,
                    0.22,
                    sumi_mid,
                    0.0,
                );
            }
        }

        // 5. Red Vermilion Hanko Seal Stamp in Bottom-Right Corner
        let seal_x = self.width - 110.0 * tree_scale;
        let seal_y = self.height - 95.0 * tree_scale;
        let seal_lines = ["╔═══╗", "║✦☘✦║", "║#┼#║", "╚═══╝"];
        for (r, line) in seal_lines.iter().enumerate() {
            self.push_text(
                line,
                [seal_x, seal_y + r as f32 * ch * 0.85],
                [cw * 0.9, ch * 0.9],
                cw * 0.85,
                0.40,
                vermilion,
                0.0,
            );
        }
    }

    // =========================================================================
    // STYLE 6: JEWEL-BOX STAINED-GLASS MOSAIC
    // =========================================================================
    fn generate_style_6_stained_glass(&mut self, world_guard: &WorldState) {
        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let tile_w = 14.0 * tree_scale;
        let tile_h = 20.0 * tree_scale;
        // Inset backing tile by 15% so dark leaded-glass borders remain between every mosaic cell
        let backing_size = [tile_w * 0.84, tile_h * 0.84];
        let fg_size = [tile_w * 0.76, tile_h * 0.76];

        let snap_mosaic = |pt: [f32; 2]| -> [f32; 2] {
            let c = (pt[0] / tile_w).round();
            let r = (pt[1] / tile_h).round();
            [c * tile_w, r * tile_h]
        };

        // 1. Cathedral Rose-Window Concentric Stained-Glass Arch Rings behind Canopy
        let arch_center = self.world_to_screen([500.0, 440.0], offset_x);
        for ring in 1..=3 {
            let radius = ring as f32 * 88.0 * tree_scale;
            let count = ring * 18;
            for k in 0..count {
                let angle =
                    (k as f32 / count as f32) * std::f32::consts::TAU + self.time as f32 * 0.04;
                let p = snap_mosaic([
                    arch_center[0] + angle.cos() * radius,
                    arch_center[1] + angle.sin() * radius * 0.85,
                ]);
                let bg = match (ring + k) % 3 {
                    0 => [0.10, 0.25, 0.58, 0.38],
                    1 => [0.35, 0.12, 0.52, 0.38],
                    _ => [0.08, 0.42, 0.48, 0.38],
                };
                let fg = [0.65, 0.85, 1.0, 0.55];
                self.push_glyph('█', p, backing_size, 0.0, 0.02, bg, 0.0);
                self.push_glyph('◇', p, fg_size, 0.0, 0.03, fg, 0.0);
            }
        }

        // 2. Stained-Glass Mosaic Tree Trunk, Boughs & Jewel Canopy
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let end = self.world_to_screen(branch.current_end(), offset_x);
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let steps = ((dx * dx + dy * dy).sqrt() / (tile_h * 0.7))
                .ceil()
                .max(1.0) as usize;

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let p = snap_mosaic([start[0] + dx * t, start[1] + dy * t]);
                self.push_glyph(
                    '█',
                    p,
                    backing_size,
                    0.0,
                    0.09,
                    [0.42, 0.24, 0.10, 0.88],
                    branch.sway_factor * 0.4,
                );
                self.push_glyph(
                    '◆',
                    p,
                    fg_size,
                    0.0,
                    0.10,
                    [0.95, 0.72, 0.35, 0.96],
                    branch.sway_factor * 0.4,
                );
            }

            for (idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.05 {
                    continue;
                }
                let mut pt = self.world_to_screen(branch.point_at(bud.offset_along), offset_x);
                pt[0] += bud.lateral_offset * tree_scale;
                let p = snap_mosaic(pt);

                let (tile_bg, facet_fg, facet_ch) = match bud.color_idx {
                    0 => (
                        [0.06, 0.48, 0.28, 0.85],
                        [0.45, 1.0, 0.72, 0.98],
                        if idx % 2 == 0 { '◆' } else { '▲' },
                    ),
                    1 => (
                        [0.08, 0.56, 0.52, 0.85],
                        [0.55, 0.98, 0.95, 0.98],
                        if idx % 2 == 0 { '◇' } else { '✦' },
                    ),
                    2 => ([0.62, 0.38, 0.06, 0.88], [1.0, 0.92, 0.38, 0.98], '✦'),
                    _ => ([0.58, 0.12, 0.32, 0.88], [1.0, 0.62, 0.82, 0.98], '◆'),
                };
                self.push_glyph(
                    '█',
                    p,
                    backing_size,
                    0.0,
                    0.14,
                    tile_bg,
                    branch.sway_factor * 0.6,
                );
                self.push_glyph(
                    facet_ch,
                    p,
                    fg_size,
                    0.0,
                    0.16,
                    facet_fg,
                    branch.sway_factor * 0.6,
                );
            }
        }

        // 3. Stained-Glass Mosaic Floor Border
        let ground_y = (self.height * 0.90 / tile_h).round() * tile_h;
        let cols = (self.width / tile_w).ceil() as i32;
        for c in 0..cols {
            let p = [c as f32 * tile_w, ground_y];
            let bg = if c % 2 == 0 {
                [0.08, 0.38, 0.26, 0.85]
            } else {
                [0.12, 0.28, 0.46, 0.85]
            };
            self.push_glyph('█', p, backing_size, 0.0, 0.06, bg, 0.0);
            self.push_glyph(
                if c % 2 == 0 { '▲' } else { '▼' },
                p,
                fg_size,
                0.0,
                0.07,
                [0.65, 0.95, 0.82, 0.92],
                0.0,
            );
        }

        // 4. Origami Gem Birds
        for bird in &world_guard.birds {
            let p = snap_mosaic(self.world_to_screen(bird.pos, offset_x));
            let col = Self::species_color(bird.species);
            self.push_glyph(
                '█',
                p,
                backing_size,
                0.0,
                0.24,
                [col[0] * 0.45, col[1] * 0.45, col[2] * 0.45, 0.92],
                0.0,
            );
            self.push_glyph('✦', p, fg_size, 0.0, 0.26, col, 0.0);
            self.push_glyph('<', [p[0] - tile_w, p[1]], fg_size, 0.0, 0.25, col, 0.0);
            self.push_glyph('>', [p[0] + tile_w, p[1]], fg_size, 0.0, 0.25, col, 0.0);
        }
    }

    // =========================================================================
    // STYLE 7: 3D VOLUMETRIC ASCII TREE VISUALIZER
    // =========================================================================
    fn generate_style_7_volumetric_3d(&mut self, world_guard: &WorldState) {
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let base_size = [11.5 * tree_scale, 18.5 * tree_scale];

        // Camera turntable orbit angle (driven by time + horizontal pan offset)
        let yaw = (self.time as f32) * 0.28 + world_guard.viewport_offset_x * 0.006;
        let pitch = 0.22f32; // slight elevated 3D look-down angle
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let cos_p = pitch.cos();
        let sin_p = pitch.sin();

        let cam_dist = 520.0f32;
        let fov_scale = 760.0 * tree_scale;
        let screen_cx = self.width * 0.5;
        let screen_cy = self.height * 0.52;

        // Directional light vector in world 3D space (normalized)
        let light_dir = [0.62f32, 0.58, -0.52];
        let ramp: [char; 10] = ['.', ':', '-', '=', '+', '*', '#', '%', '@', '█'];

        let mut projected: Vec<Projected3dGlyph> = Vec::with_capacity(4096);

        let project_3d = |pt3: [f32; 3]| -> Option<([f32; 2], f32, f32)> {
            // Rotate around Y (turntable) then X (pitch)
            let rx = pt3[0] * cos_y + pt3[2] * sin_y;
            let rz0 = -pt3[0] * sin_y + pt3[2] * cos_y;
            let ry = pt3[1] * cos_p - rz0 * sin_p;
            let rz = pt3[1] * sin_p + rz0 * cos_p + cam_dist;
            if rz < 80.0 {
                return None;
            }
            let persp = fov_scale / rz;
            let sx = screen_cx + rx * persp;
            let sy = screen_cy - ry * persp;
            Some(([sx, sy], rz, persp))
        };

        // 1. 3D Circular Turntable Pedestal Ring & Radial Spokes at Y = -220
        let base_y = -220.0f32;
        for ring_r in [120.0f32, 240.0, 340.0] {
            let steps = 48;
            for k in 0..steps {
                let a = (k as f32 / steps as f32) * std::f32::consts::TAU;
                let p3 = [a.cos() * ring_r, base_y, a.sin() * ring_r];
                if let Some((sp, rz, persp)) = project_3d(p3) {
                    let depth_fade = ((780.0 - rz) / 480.0).clamp(0.25, 1.0);
                    let ch = if k % 12 == 0 { '+' } else { '·' };
                    projected.push(Projected3dGlyph {
                        c: ch,
                        screen_pos: sp,
                        size: [base_size[0] * persp * 0.9, base_size[1] * persp * 0.9],
                        z_cam: rz,
                        color: [
                            0.20,
                            0.72 * depth_fade,
                            0.88 * depth_fade,
                            0.75 * depth_fade,
                        ],
                    });
                }
            }
        }

        // 2. Reconstruct 3D Branch Skeleton & Cylindrical 3D Surface Voxels
        // Map each 2D branch into 3D by deterministic azimuth angle derived from branch id
        let branch_3d_coords = |b_id: u32, pt2: [f32; 2], depth: u32| -> [f32; 3] {
            let rel_x = pt2[0] - 500.0;
            let rel_y = (750.0 - pt2[1]) - 220.0;
            if depth == 0 {
                [rel_x, rel_y, 0.0]
            } else {
                let az = (b_id as f32 * 2.399963) + (depth as f32 * 0.7);
                let r_horiz = rel_x.abs() * 0.95 + (depth as f32 * 18.0);
                [az.cos() * r_horiz, rel_y, az.sin() * r_horiz]
            }
        };

        for branch in &world_guard.branches {
            let p0 = branch_3d_coords(branch.id, branch.start, branch.depth.saturating_sub(1));
            let p1 = branch_3d_coords(branch.id, branch.current_end(), branch.depth);
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let dz = p1[2] - p0[2];
            let seg_len = (dx * dx + dy * dy + dz * dz).sqrt();
            let steps = (seg_len / 11.0).ceil().max(1.0) as usize;
            let radius = (branch.thickness * 1.15).max(4.0);

            // Number of circumferential 3D cylinder samples
            let ring_samples = if branch.depth == 0 {
                8
            } else if branch.depth == 1 {
                5
            } else {
                2
            };

            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cx = p0[0] + dx * t;
                let cy = p0[1] + dy * t;
                let cz = p0[2] + dz * t;

                for k in 0..ring_samples {
                    let phi = (k as f32 / ring_samples as f32) * std::f32::consts::TAU;
                    let nx = phi.cos();
                    let nz = phi.sin();
                    let vx = cx + nx * radius;
                    let vz = cz + nz * radius;

                    if let Some((sp, rz, persp)) = project_3d([vx, cy, vz]) {
                        // Lambertian surface normal shading N · L
                        let ndotl = (nx * light_dir[0] + 0.25 * light_dir[1] + nz * light_dir[2])
                            .clamp(0.0, 1.0);
                        let depth_cue = ((820.0 - rz) / 520.0).clamp(0.25, 1.0);
                        let luma = (0.20 + ndotl * 0.78) * depth_cue;
                        let idx = ((luma * 9.0).round() as usize).min(9);

                        projected.push(Projected3dGlyph {
                            c: ramp[idx],
                            screen_pos: sp,
                            size: [
                                (base_size[0] * persp * 1.05).clamp(6.0, 28.0),
                                (base_size[1] * persp * 1.05).clamp(10.0, 42.0),
                            ],
                            z_cam: rz,
                            color: [
                                0.30 + luma * 0.58,
                                0.22 + luma * 0.42,
                                0.16 + (1.0 - depth_cue) * 0.25,
                                0.94,
                            ],
                        });
                    }
                }
            }

            // 3. 3D Volumetric Spherical Foliage Clusters with Normal Shading
            for (b_idx, bud) in branch.foliage.iter().enumerate() {
                if bud.bloom <= 0.08 {
                    continue;
                }
                let t = bud.offset_along;
                let bx = p0[0] + dx * t;
                let by = p0[1] + dy * t;
                let bz = p0[2] + dz * t;

                // Golden-spiral 3D sphere points around each foliage bud
                let sphere_pts = 8;
                let sphere_r = 24.0 * bud.scale * bud.bloom;
                for sp_i in 0..sphere_pts {
                    let fi = sp_i as f32 + (b_idx as f32 * 0.37);
                    let ny = 1.0 - 2.0 * ((sp_i as f32 + 0.5) / sphere_pts as f32);
                    let r_xy = (1.0 - ny * ny).max(0.0).sqrt();
                    let theta = fi * 2.399963;
                    let nx = theta.cos() * r_xy;
                    let nz = theta.sin() * r_xy;

                    let fx = bx + nx * sphere_r;
                    let fy = by + ny * sphere_r;
                    let fz = bz + nz * sphere_r;

                    if let Some((sp, rz, persp)) = project_3d([fx, fy, fz]) {
                        let ndotl = (nx * light_dir[0] + ny * light_dir[1] + nz * light_dir[2])
                            .clamp(0.0, 1.0);
                        let depth_cue = ((760.0 - rz) / 480.0).clamp(0.28, 1.0);
                        let luma = (0.26 + ndotl * 0.74) * depth_cue;
                        let idx = ((luma * 9.0).round() as usize).min(9);

                        let col = match bud.color_idx {
                            0 => [
                                0.14 + luma * 0.28,
                                0.36 + luma * 0.64,
                                0.24 + luma * 0.38,
                                0.95,
                            ],
                            1 => [
                                0.22 + luma * 0.38,
                                0.44 + luma * 0.56,
                                0.28 + luma * 0.34,
                                0.95,
                            ],
                            2 => [
                                0.42 + luma * 0.58,
                                0.34 + luma * 0.54,
                                0.14 + luma * 0.18,
                                0.96,
                            ],
                            _ => [
                                0.44 + luma * 0.56,
                                0.24 + luma * 0.44,
                                0.36 + luma * 0.54,
                                0.96,
                            ],
                        };

                        projected.push(Projected3dGlyph {
                            c: ramp[idx],
                            screen_pos: sp,
                            size: [
                                (base_size[0] * persp * 1.25).clamp(7.0, 30.0),
                                (base_size[1] * persp * 1.25).clamp(12.0, 46.0),
                            ],
                            z_cam: rz,
                            color: col,
                        });
                    }
                }
            }
        }

        // 4. 3D Orbiting Flock in Perspective Space
        for bird in &world_guard.birds {
            let orbit_a = (self.time as f32 * 0.65) + (bird.id as f32 * 1.57);
            let orbit_r = 160.0 + (bird.id % 3) as f32 * 55.0;
            let by = (750.0 - bird.pos[1]) - 220.0;
            let p3 = [orbit_a.cos() * orbit_r, by, orbit_a.sin() * orbit_r];
            if let Some((sp, rz, persp)) = project_3d(p3) {
                let col = Self::species_color(bird.species);
                let bsz = [
                    (base_size[0] * persp * 1.3).clamp(8.0, 30.0),
                    (base_size[1] * persp * 1.3).clamp(12.0, 44.0),
                ];
                projected.push(Projected3dGlyph {
                    c: '@',
                    screen_pos: sp,
                    size: bsz,
                    z_cam: rz - 2.0,
                    color: col,
                });
                projected.push(Projected3dGlyph {
                    c: '^',
                    screen_pos: [sp[0] - bsz[0] * 0.8, sp[1]],
                    size: bsz,
                    z_cam: rz - 2.0,
                    color: col,
                });
                projected.push(Projected3dGlyph {
                    c: '^',
                    screen_pos: [sp[0] + bsz[0] * 0.8, sp[1]],
                    size: bsz,
                    z_cam: rz - 2.0,
                    color: col,
                });
            }
        }

        // 5. Back-to-Front Painter's Depth Sort (far z_cam drawn first, near z_cam drawn on top)
        projected.sort_by(|a, b| {
            b.z_cam
                .partial_cmp(&a.z_cam)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_verts = projected.len();
        for item in projected {
            self.push_glyph(
                item.c,
                item.screen_pos,
                item.size,
                0.0,
                0.20,
                item.color,
                0.0,
            );
        }

        // 6. 3D Orientation Gizmo [X / Y / Z] in Bottom-Left Corner & Viewport Header
        let gizmo_origin = [75.0 * tree_scale, self.height - 75.0 * tree_scale];
        let axis_len = 38.0 * tree_scale;
        let axes = [
            ([1.0f32, 0.0, 0.0], 'X', [0.98, 0.35, 0.35, 0.95]),
            ([0.0f32, 1.0, 0.0], 'Y', [0.35, 0.98, 0.48, 0.95]),
            ([0.0f32, 0.0, 1.0], 'Z', [0.38, 0.72, 1.0, 0.95]),
        ];
        for (vec3, label, col) in axes {
            let rx = vec3[0] * cos_y + vec3[2] * sin_y;
            let rz0 = -vec3[0] * sin_y + vec3[2] * cos_y;
            let ry = vec3[1] * cos_p - rz0 * sin_p;
            let end_pt = [
                gizmo_origin[0] + rx * axis_len,
                gizmo_origin[1] - ry * axis_len,
            ];
            for step in 1..=4 {
                let f = step as f32 / 5.0;
                self.push_glyph(
                    '·',
                    [
                        gizmo_origin[0] + (end_pt[0] - gizmo_origin[0]) * f,
                        gizmo_origin[1] + (end_pt[1] - gizmo_origin[1]) * f,
                    ],
                    [base_size[0] * 0.8, base_size[1] * 0.8],
                    0.0,
                    0.80,
                    col,
                    0.0,
                );
            }
            self.push_glyph(label, end_pt, base_size, 0.0, 0.85, col, 0.0);
        }

        let deg = (yaw.to_degrees()).rem_euclid(360.0);
        let hud = format!(
            "[ 3D ASCII TURNTABLE · YAW: {:03.0} deg · VOXELS: {} ]",
            deg, total_verts
        );
        self.push_text(
            &hud,
            [self.width - 390.0 * tree_scale, 22.0 * tree_scale],
            [base_size[0] * 0.75, base_size[1] * 0.75],
            7.4 * tree_scale,
            0.85,
            [0.45, 0.88, 0.98, 0.92],
            0.0,
        );
    }
}

impl Scene for LivingRookeryScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);
        self.time += dt_secs as f64;
        self.interactive_mode = input.interactive;

        let pointer_world = input.pointer.map(|p| self.screen_to_world(p));

        let (activity, current_style) = if let Ok(mut w) = self.world.lock() {
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
                            .map(|sp| sp.x < 420.0 && sp.y < 52.0)
                            .unwrap_or(false);

                        if clicked_top_left_pill {
                            w.cycle_aesthetic_style();
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

            let style = w.aesthetic_style % ROOKERY_STYLE_COUNT;
            let mut act = w.update(dt_secs, input.interactive, pointer_world);
            if self.is_dragging_panning || style == 7 || style == 2 {
                act = Activity::Animating;
            }
            (act, style)
        } else {
            (Activity::Idle, 0)
        };

        // Update background uniforms with active style_id
        let bg_uniforms = BgUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            style_id: current_style as f32,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&bg_uniforms));

        // Generate and upload glyph instances
        self.generate_scene_instances();
        let wind_sway = if current_style == 1 || current_style == 7 {
            0.0
        } else {
            1.0
        };
        self.glyphs
            .upload(gpu, [self.width, self.height], self.time as f32, wind_sway);

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Render pass 1: Background atmospheric shader per style
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ascii-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.04,
                            b: 0.06,
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
