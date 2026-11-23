//! The Living ASCII Rookery: Strict ANSI Roguelike Engine & Forest Succession.
//!
//! Monospace character-cell woodland rendering in the tradition of Caves of Qud and Dwarf Fortress.
//! Features 2-3 cell bird sprites, real-tree species architectures (Oak, Willow, Pine, Maple, Birch, Baobab),
//! atmospheric celestial skies with twinkling starlight, curated ANSI palettes, and continuous forest succession.

use std::sync::{Arc, Mutex};

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::world::{BirdSpecies, SnackType, WorldState};

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

mod layers;
mod palette;

use palette::ShadowPalette;

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
    pan_remainder_x: f32,
    pan_velocity_x: f32,
    snack_click_count: u32,
}

impl LivingRookeryScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, world: Arc<Mutex<WorldState>>) -> Self {
        let device = &gpu.device;

        // 1. Fullscreen background setup
        let bg_source = include_str!("../ascii_bg.wgsl");
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
            pan_remainder_x: 0.0,
            pan_velocity_x: 0.0,
            snack_click_count: 0,
        }
    }

    pub fn with_audio(mut self, audio: Option<crate::audio::AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    /// Converts normalized world coordinates [0..1000, 0..850] to current screen viewport pixels.
    fn world_to_screen(&self, pt: [f32; 2], offset_x: f32) -> [f32; 2] {
        let x = ((pt[0] - offset_x) / 1000.0) * self.width;
        let ground_y = self.height * 0.80;
        let tree_scale = (self.height / 960.0).clamp(0.6, 2.0);
        let y = ground_y - (750.0 - pt[1]) * tree_scale;
        [x, y]
    }

    /// Converts screen pixel pointer coordinates to world coordinates using a known camera offset.
    fn screen_to_world_with_offset(&self, pt: Pointer, offset_x: f32) -> [f32; 2] {
        let x = (pt.x / self.width) * 1000.0 + offset_x;
        let ground_y = self.height * 0.80;
        let tree_scale = (self.height / 960.0).clamp(0.6, 2.0);
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

    /// Draws a directional moon-cast ground shadow cell with umbral backing and feathered edges.
    /// `density`: 3 = deep contact/umbra, 2 = mid shadow body, 1 = soft penumbra.
    #[allow(clippy::too_many_arguments)]
    fn push_shadow_cell(
        &mut self,
        c: i32,
        r: i32,
        cols: i32,
        rows: i32,
        cell_size: [f32; 2],
        density: u8,
        z_layer: f32,
        pal: ShadowPalette,
    ) {
        if c < 0 || c >= cols || r < 0 || r >= rows {
            return;
        }
        let pos = [
            (c as f32 + 0.5) * cell_size[0],
            (r as f32 + 0.5) * cell_size[1],
        ];
        match density {
            3.. => {
                self.push_glyph('█', pos, cell_size, 0.0, z_layer, pal.umbral, 0.0);
                self.push_glyph('▓', pos, cell_size, 0.0, z_layer + 0.001, pal.deep, 0.0);
            }
            2 => {
                let mid_bg = [pal.umbral[0], pal.umbral[1], pal.umbral[2], 0.72];
                self.push_glyph('█', pos, cell_size, 0.0, z_layer, mid_bg, 0.0);
                self.push_glyph('▒', pos, cell_size, 0.0, z_layer + 0.001, pal.deep, 0.0);
            }
            _ => {
                self.push_glyph('░', pos, cell_size, 0.0, z_layer, pal.soft, 0.0);
            }
        }
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

        let (activity, current_palette) = if let Ok(mut w) = self.world.lock() {
            let pointer_world = input
                .pointer
                .map(|p| self.screen_to_world_with_offset(p, w.viewport_offset_x));
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
                        self.pan_remainder_x = 0.0;
                        self.pan_velocity_x = 0.0;
                    }

                    if input.left_held {
                        let mut frame_pan_dx = 0.0;
                        if let Some(last) = self.last_screen_pointer {
                            let dx = sp.x - last.x;
                            let dy = sp.y - last.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            self.drag_distance_accum += dist;

                            if self.drag_distance_accum > 7.0 {
                                self.is_dragging_panning = true;
                                let pan_scale = 1000.0 / self.width.max(1.0);
                                frame_pan_dx = -dx * pan_scale;
                                self.pan_remainder_x += frame_pan_dx;
                            }
                        }
                        self.last_screen_pointer = Some(sp);

                        if self.is_dragging_panning && dt_secs > 0.0 {
                            let safe_dt = dt_secs.max(1.0 / 240.0);
                            let inst_vel = (frame_pan_dx / safe_dt).clamp(-4500.0, 4500.0);
                            let vel_blend = 1.0 - (-dt_secs * 14.0).exp();
                            self.pan_velocity_x += (inst_vel - self.pan_velocity_x) * vel_blend;
                        }
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

            let mut is_panning_smoothly = false;
            if dt_secs > 0.0 {
                if self.pan_remainder_x.abs() > 0.05 {
                    let follow = 1.0 - (-dt_secs * 22.0).exp();
                    let step = self.pan_remainder_x * follow;
                    w.pan_forest(step);
                    self.pan_remainder_x -= step;
                    is_panning_smoothly = true;
                } else if self.pan_remainder_x != 0.0 {
                    w.pan_forest(self.pan_remainder_x);
                    self.pan_remainder_x = 0.0;
                }

                if !self.is_dragging_panning && self.pan_velocity_x.abs() > 4.0 {
                    let prev_offset = w.viewport_offset_x;
                    w.pan_forest(self.pan_velocity_x * dt_secs);
                    if (w.viewport_offset_x - prev_offset).abs() < 1e-4 {
                        self.pan_velocity_x = 0.0;
                    } else {
                        let friction = (-dt_secs * 4.5).exp();
                        self.pan_velocity_x *= friction;
                        is_panning_smoothly = true;
                    }
                } else if !self.is_dragging_panning {
                    self.pan_velocity_x = 0.0;
                }
            }

            let pal = w.active_palette.as_u8();
            let mut act = w.update(dt_secs, input.interactive, pointer_world);
            if self.is_dragging_panning || is_panning_smoothly {
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
