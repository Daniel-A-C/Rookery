//! Cosmic Nebula Scene (Milestone 4 - Generative Physics & Shaders).
//!
//! Algorithmic deep-space simulation with glowing celestial particles, shifting gravitational
//! attractor/repulsor vortexes, and customizable stellar palettes and turbulence.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::nebula::{CosmicNebulaState, NebulaPalette};

use crate::atlas::GlyphPass;
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct NebulaUniforms {
    resolution: [f32; 2],
    time: f32,
    palette_idx: f32,
    vortex_pos: [f32; 2],
    turbulence: f32,
    _pad: f32,
}

pub struct CosmicNebulaScene {
    pub nebula: Arc<Mutex<CosmicNebulaState>>,
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

impl CosmicNebulaScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        nebula: Arc<Mutex<CosmicNebulaState>>,
    ) -> Self {
        let device = &gpu.device;

        // 1. Nebula background setup
        let bg_source = include_str!("nebula.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "nebula-bg-layout");
        let bg_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-bg-uniforms"),
            size: std::mem::size_of::<NebulaUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nebula-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms_buf.as_entire_binding(),
            }],
        });
        let bg_pipeline = fullscreen::pipeline(
            device,
            "nebula-bg-pipeline",
            bg_source,
            format,
            Some(&bg_layout),
            None,
        );

        let glyphs = GlyphPass::new(gpu, format, MAX_INSTANCES);

        Self {
            nebula,
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

        let nebula_arc = self.nebula.clone();
        let nebula_guard = match nebula_arc.lock() {
            Ok(n) => n,
            Err(_) => return,
        };

        let scale = (self.height / 750.0).clamp(0.6, 2.0);

        // 1. Draw Active Gravitational Wells / Vortex cores
        for w in &nebula_guard.wells {
            let center = self.world_to_screen(w.pos);
            let alpha = (1.0 - w.life / w.max_life).clamp(0.2, 0.95);
            let (v_char, v_col) = if w.is_repulsor {
                ('O', [0.95, 0.35, 0.35, alpha])
            } else {
                ('@', [0.35, 0.85, 0.98, alpha])
            };
            let sz = [28.0 * scale, 28.0 * scale];
            self.push_glyph(v_char, center, sz, self.time as f32 * 2.0, 0.05, v_col, 0.0);
        }

        // 2. Draw Stellar Particles / Celestial Boids
        let pal = nebula_guard.palette;
        for p in &nebula_guard.particles {
            let screen_pos = self.world_to_screen(p.pos);
            let speed = (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt();
            let rot = p.vel[1].atan2(p.vel[0]);

            let col = match pal {
                NebulaPalette::StellarGold => {
                    if p.color_t > 0.6 {
                        [0.98, 0.88, 0.35, 0.95]
                    } else if p.color_t > 0.3 {
                        [0.45, 0.82, 0.98, 0.90]
                    } else {
                        [0.95, 0.98, 1.0, 0.95]
                    }
                }
                NebulaPalette::DeepAmethyst => {
                    if p.color_t > 0.6 {
                        [0.92, 0.45, 0.88, 0.95]
                    } else if p.color_t > 0.3 {
                        [0.55, 0.32, 0.88, 0.90]
                    } else {
                        [0.98, 0.85, 0.95, 0.95]
                    }
                }
                NebulaPalette::SolarFlare => {
                    if p.color_t > 0.6 {
                        [0.98, 0.45, 0.15, 0.95]
                    } else if p.color_t > 0.3 {
                        [0.95, 0.82, 0.22, 0.95]
                    } else {
                        [0.98, 0.95, 0.88, 0.95]
                    }
                }
                NebulaPalette::AuroraVeil => {
                    if p.color_t > 0.6 {
                        [0.35, 0.95, 0.65, 0.95]
                    } else if p.color_t > 0.3 {
                        [0.25, 0.75, 0.85, 0.90]
                    } else {
                        [0.85, 1.0, 0.92, 0.95]
                    }
                }
            };

            let char_choice = if speed > 60.0 {
                '-'
            } else if p.size > 2.5 {
                '*'
            } else if p.size > 1.8 {
                'o'
            } else {
                '.'
            };

            let sz = [p.size * 5.5 * scale, p.size * 5.5 * scale];
            self.push_glyph(char_choice, screen_pos, sz, rot, 0.1, col, 0.0);
        }
    }

    pub fn with_audio(mut self, audio: Option<crate::audio::AudioEngine>) -> Self {
        self.audio = audio;
        self
    }
}

impl Scene for CosmicNebulaScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);
        self.time += dt_secs as f64;

        let pointer_world = input.pointer.map(|pt| self.screen_to_world(pt));
        let (activity, pal_idx, turb, vortex_pos) = {
            let mut guard = match self.nebula.lock() {
                Ok(g) => g,
                Err(_) => return Activity::Idle,
            };
            if input.interactive
                && let Some(pw) = pointer_world
            {
                match input.action {
                    PointerAction::LeftClick => {
                        guard.add_gravity_well(pw, false);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::NebulaPulse);
                        }
                    }
                    PointerAction::RightClick => {
                        guard.add_gravity_well(pw, true);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::NebulaPulse);
                        }
                    }
                    PointerAction::None => {}
                }
            }
            let act = guard.update(dt_secs, input.interactive, pointer_world);
            let p_idx = match guard.palette {
                NebulaPalette::StellarGold => 0.0,
                NebulaPalette::DeepAmethyst => 1.0,
                NebulaPalette::SolarFlare => 2.0,
                NebulaPalette::AuroraVeil => 3.0,
            };
            let v_pos = guard
                .wells
                .first()
                .map(|w| {
                    [
                        (w.pos[0] / 1000.0) * self.width,
                        (w.pos[1] / 750.0) * self.height,
                    ]
                })
                .unwrap_or([self.width * 0.5, self.height * 0.5]);

            (act, p_idx, guard.turbulence, v_pos)
        };

        let neb_uniforms = NebulaUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            palette_idx: pal_idx,
            vortex_pos,
            turbulence: turb,
            _pad: 0.0,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&neb_uniforms));

        self.generate_instances();
        self.glyphs
            .upload(gpu, [self.width, self.height], self.time as f32, 0.0);

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Pass 1: Background nebula pass
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nebula-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.015,
                            g: 0.02,
                            b: 0.04,
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

        // Pass 2: Foreground glyphs
        self.glyphs.render(view, encoder);
    }
}
