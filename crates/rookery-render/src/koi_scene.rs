//! Tranquil Koi Pond Scene (Milestone 4 - Cozy Living Ecosystem).
//!
//! A tranquil water garden ecosystem featuring autonomous articulated koi fish,
//! floating lily pads with blooming lotus flowers, interactive water ripples,
//! and feeding micro-interactions.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::koi::{KoiPondState, KoiVariety};

use crate::atlas::GlyphPass;
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterUniforms {
    resolution: [f32; 2],
    time: f32,
    _pad: f32,
}

pub struct KoiPondScene {
    pub pond: Arc<Mutex<KoiPondState>>,
    pub audio: Option<crate::audio::AudioEngine>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,

    bg_pipeline: wgpu::RenderPipeline,
    bg_uniforms_buf: wgpu::Buffer,
    bg_bind_group: wgpu::BindGroup,

    glyphs: GlyphPass,
}

impl KoiPondScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, pond: Arc<Mutex<KoiPondState>>) -> Self {
        let device = &gpu.device;

        // 1. Water caustics background setup
        let bg_source = include_str!("koi.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "koi-bg-layout");
        let bg_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("koi-bg-uniforms"),
            size: std::mem::size_of::<WaterUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("koi-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms_buf.as_entire_binding(),
            }],
        });
        let bg_pipeline = fullscreen::pipeline(
            device,
            "koi-bg-pipeline",
            bg_source,
            format,
            Some(&bg_layout),
            None,
        );

        let glyphs = GlyphPass::new(gpu, format, MAX_INSTANCES);

        Self {
            pond,
            audio: None,
            width: 800.0,
            height: 600.0,
            scale_factor: 1.0,
            time: 0.0,
            bg_pipeline,
            bg_uniforms_buf,
            bg_bind_group,
            glyphs,
        }
    }

    fn world_to_screen(&self, pt: [f32; 2]) -> [f32; 2] {
        [(pt[0] / 1000.0) * self.width, (pt[1] / 750.0) * self.height]
    }

    fn screen_to_world(&self, pt: Pointer) -> [f32; 2] {
        [(pt.x / self.width) * 1000.0, (pt.y / self.height) * 750.0]
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

    fn generate_instances(&mut self) {
        self.glyphs.clear();

        let pond_arc = self.pond.clone();
        let pond_guard = match pond_arc.lock() {
            Ok(p) => p,
            Err(_) => return,
        };

        let scale = (self.height / 750.0).clamp(0.6, 2.0);

        // 1. Draw Water Ripples
        for r in &pond_guard.ripples {
            let center = self.world_to_screen(r.center);
            let r_screen = (r.radius / 750.0) * self.height;
            let alpha = (r.amplitude * 0.45).clamp(0.0, 0.6);
            let ripple_col = [0.35, 0.75, 0.85, alpha];
            self.push_glyph(
                'O',
                center,
                [r_screen * 2.0, r_screen * 1.4],
                0.0,
                0.02,
                ripple_col,
                0.0,
            );
        }

        // 2. Draw Floating Lily Pads and Blooming Lotus Flowers
        for lily in &pond_guard.lilies {
            let center = self.world_to_screen(lily.pos);
            let pad_size = [lily.radius * 2.1 * scale, lily.radius * 1.8 * scale];
            let pad_col = [0.12, 0.48, 0.28, 0.88];

            self.push_glyph('O', center, pad_size, lily.rotation, 0.05, pad_col, 0.02);
            self.push_glyph(
                '☘',
                [center[0] - 4.0 * scale, center[1]],
                [pad_size[0] * 0.65, pad_size[1] * 0.65],
                lily.rotation + 0.4,
                0.06,
                [0.18, 0.58, 0.35, 0.95],
                0.02,
            );

            if lily.has_flower && lily.flower_bloom > 0.1 {
                let bloom = lily.flower_bloom;
                let fl_size = [24.0 * scale * bloom, 24.0 * scale * bloom];
                let fl_col = match lily.flower_color_idx {
                    0 => [0.95, 0.72, 0.82, 0.95],
                    1 => [0.92, 0.38, 0.65, 0.95],
                    _ => [0.98, 0.92, 0.65, 0.95],
                };
                self.push_glyph(
                    '*',
                    center,
                    fl_size,
                    lily.rotation * 2.0,
                    0.08,
                    fl_col,
                    0.03,
                );
                self.push_glyph(
                    'o',
                    center,
                    [fl_size[0] * 0.5, fl_size[1] * 0.5],
                    0.0,
                    0.09,
                    [0.98, 0.88, 0.35, 0.98],
                    0.03,
                );
            }
        }

        // 3. Draw Articulated Koi Fish
        for fish in &pond_guard.fish {
            let (primary_col, secondary_col, accent_col) = match fish.variety {
                KoiVariety::Kohaku => (
                    [0.96, 0.95, 0.92, 0.95],
                    [0.92, 0.25, 0.18, 0.95],
                    [0.92, 0.25, 0.18, 0.95],
                ),
                KoiVariety::YamabukiOgon => (
                    [0.95, 0.78, 0.22, 0.98],
                    [0.98, 0.88, 0.35, 0.95],
                    [0.85, 0.65, 0.15, 0.95],
                ),
                KoiVariety::Showa => (
                    [0.18, 0.18, 0.20, 0.98],
                    [0.90, 0.22, 0.15, 0.95],
                    [0.95, 0.92, 0.88, 0.95],
                ),
                KoiVariety::Tancho => (
                    [0.96, 0.95, 0.92, 0.95],
                    [0.96, 0.95, 0.92, 0.95],
                    [0.92, 0.18, 0.18, 0.98],
                ),
                KoiVariety::Karasugoi => (
                    [0.12, 0.14, 0.18, 0.95],
                    [0.22, 0.26, 0.32, 0.92],
                    [0.35, 0.45, 0.55, 0.85],
                ),
            };

            let seg_count = fish.spine.len();
            for (idx, joint) in fish.spine.iter().enumerate() {
                let screen_pos = self.world_to_screen(*joint);
                let t = idx as f32 / seg_count.max(1) as f32;

                let body_profile = (1.0 - (t - 0.35).abs() * 1.5).clamp(0.25, 1.0);
                let w = (fish.length * 0.38 * body_profile) * scale;
                let h = (fish.length * 0.38 * body_profile) * scale;

                let col = if idx == 0 {
                    if fish.variety == KoiVariety::Tancho {
                        accent_col
                    } else {
                        primary_col
                    }
                } else if idx % 2 == 1 {
                    secondary_col
                } else {
                    primary_col
                };

                let c = if idx == 0 {
                    'O'
                } else if idx == seg_count - 1 {
                    '>'
                } else {
                    'o'
                };

                let rot = fish.angle + std::f32::consts::FRAC_PI_2;
                self.push_glyph(c, screen_pos, [w, h], rot, 0.12, col, 0.0);

                if idx == 1 {
                    let fin_angle = (fish.fin_phase).sin() * 0.35;
                    let fin_w = w * 0.85;
                    let fin_h = h * 0.75;
                    let perp = [-fish.angle.sin(), fish.angle.cos()];
                    let left_fin = [
                        screen_pos[0] + perp[0] * w * 0.7,
                        screen_pos[1] + perp[1] * h * 0.7,
                    ];
                    let right_fin = [
                        screen_pos[0] - perp[0] * w * 0.7,
                        screen_pos[1] - perp[1] * h * 0.7,
                    ];

                    self.push_glyph(
                        '~',
                        left_fin,
                        [fin_w, fin_h],
                        rot + fin_angle,
                        0.11,
                        [col[0], col[1], col[2], 0.75],
                        0.0,
                    );
                    self.push_glyph(
                        '~',
                        right_fin,
                        [fin_w, fin_h],
                        rot - fin_angle,
                        0.11,
                        [col[0], col[1], col[2], 0.75],
                        0.0,
                    );
                }
            }
        }

        // 4. Draw Food Pellets
        for pellet in &pond_guard.pellets {
            let p_pos = self.world_to_screen(pellet.pos);
            let alpha = (1.0 - pellet.sink_depth * 0.7).clamp(0.2, 1.0);
            let pel_col = [0.85, 0.65, 0.32, alpha];
            self.push_glyph(
                'o',
                p_pos,
                [8.0 * scale, 8.0 * scale],
                0.0,
                0.07,
                pel_col,
                0.0,
            );
        }
    }

    pub fn with_audio(mut self, audio: Option<crate::audio::AudioEngine>) -> Self {
        self.audio = audio;
        self
    }
}

impl Scene for KoiPondScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);
        self.time += dt_secs as f64;

        let pointer_world = input.pointer.map(|pt| self.screen_to_world(pt));
        let activity = {
            let mut guard = match self.pond.lock() {
                Ok(g) => g,
                Err(_) => return Activity::Idle,
            };
            if input.interactive
                && let Some(pw) = pointer_world
            {
                match input.action {
                    PointerAction::LeftClick => {
                        guard.feed_koi(pw);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::FeedKoi);
                        }
                    }
                    PointerAction::RightClick => {
                        guard.add_ripple(pw, 1.5);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::WaterRipple);
                        }
                    }
                    PointerAction::None => {
                        if input.left_held {
                            guard.add_ripple(pw, 0.6);
                        }
                    }
                }
            }
            guard.update(dt_secs, input.interactive, pointer_world)
        };

        let water_uniforms = WaterUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            _pad: 0.0,
        };
        gpu.queue.write_buffer(
            &self.bg_uniforms_buf,
            0,
            bytemuck::bytes_of(&water_uniforms),
        );

        self.generate_instances();
        self.glyphs
            .upload(gpu, [self.width, self.height], self.time as f32, 0.0);

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Pass 1: Water background
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("koi-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.08,
                            b: 0.12,
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
            rpass.set_pipeline(&self.bg_pipeline);
            rpass.set_bind_group(0, &self.bg_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Pass 2: Overlapping Glyphs
        self.glyphs.render(view, encoder);
    }
}
