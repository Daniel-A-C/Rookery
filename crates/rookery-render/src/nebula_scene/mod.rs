//! Cosmic Nebula v2.0 Scene (Zero ASCII, Custom 2-Pass wgpu Stellar Nursery).
//!
//! Two-pass GPU renderer:
//! 1. Fullscreen volumetric JWST stellar nursery backdrop (`nebula_backdrop.wgsl`):
//!    deep-field stars & galaxies, 3-layer domain-warped emission gas, backlit dark dust pillars,
//!    live star light scattering, expanding supernova shockwaves, pulsar beams, and cursor stir warps.
//! 2. Premultiplied/additive painterly space entity mesh pass (`nebula_entities.wgsl`):
//!    6-point JWST diffraction-spiked stars, collapsing protostars, luminous stardust motes,
//!    dual-tail comets, shooting meteors, supernova remnant filaments, and toggleable Astral Cartography.

mod mesh;
mod theme;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::nebula::{CosmicNebulaState, NebulaSoundCue, StarStage, WORLD_H, WORLD_W};

use crate::audio::{AudioEngine, SoundEffect};
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

use mesh::{NebulaMeshBuilder, NebulaVertex};
use theme::{lerp_palette, palette_for};

const INITIAL_VERTS: usize = 8192;
const INITIAL_INDICES: usize = 12288;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BackdropUniforms {
    resolution_and_time: [f32; 4],
    counts_and_flags: [f32; 4],
    pulsar_data: [f32; 4],
    deep_space: [f32; 4],
    gas_primary: [f32; 4],
    gas_secondary: [f32; 4],
    dust_dark: [f32; 4],
    rim_glow: [f32; 4],
    star_warm: [f32; 4],
    star_cool: [f32; 4],
    stars: [[f32; 4]; 16],
    shockwaves: [[f32; 4]; 6],
    wakes: [[f32; 4]; 12],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EntityUniforms {
    resolution_and_time: [f32; 4],
}

pub struct CosmicNebulaScene {
    pub nebula: Arc<Mutex<CosmicNebulaState>>,
    pub audio: Option<AudioEngine>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,

    backdrop_pipeline: wgpu::RenderPipeline,
    backdrop_uniforms_buf: wgpu::Buffer,
    backdrop_bind_group: wgpu::BindGroup,

    entity_pipeline: wgpu::RenderPipeline,
    entity_uniforms_buf: wgpu::Buffer,
    entity_bind_group: wgpu::BindGroup,

    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    vbuf_cap: usize,
    ibuf_cap: usize,
    index_count: u32,

    verts_scratch: Vec<NebulaVertex>,
    indices_scratch: Vec<u32>,

    last_pointer_world: Option<[f32; 2]>,
}

impl CosmicNebulaScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        nebula: Arc<Mutex<CosmicNebulaState>>,
    ) -> Self {
        let device = &gpu.device;

        // 1. Fullscreen volumetric backdrop pipeline
        let bg_source = include_str!("nebula_backdrop.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "nebula-backdrop-layout");
        let backdrop_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-backdrop-uniforms"),
            size: std::mem::size_of::<BackdropUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let backdrop_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nebula-backdrop-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: backdrop_uniforms_buf.as_entire_binding(),
            }],
        });
        let backdrop_pipeline = fullscreen::pipeline(
            device,
            "nebula-backdrop-pipeline",
            bg_source,
            format,
            Some(&bg_layout),
            None,
        );

        // 2. Premultiplied/additive space entity mesh pipeline
        let entity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nebula-entities-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nebula_entities.wgsl").into()),
        });
        let entity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nebula-entities-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let entity_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-entities-uniforms"),
            size: std::mem::size_of::<EntityUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nebula-entities-bind-group"),
            layout: &entity_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: entity_uniforms_buf.as_entire_binding(),
            }],
        });
        let entity_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("nebula-entities-pipeline-layout"),
                bind_group_layouts: &[Some(&entity_layout)],
                immediate_size: 0,
            });
        let entity_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nebula-entities-pipeline"),
            layout: Some(&entity_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &entity_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(NebulaVertex::layout())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &entity_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-entity-vbuf"),
            size: (INITIAL_VERTS * std::mem::size_of::<NebulaVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-entity-ibuf"),
            size: (INITIAL_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            nebula,
            audio: None,
            width: 1280.0,
            height: 720.0,
            scale_factor: 1.0,
            time: 0.0,
            backdrop_pipeline,
            backdrop_uniforms_buf,
            backdrop_bind_group,
            entity_pipeline,
            entity_uniforms_buf,
            entity_bind_group,
            vbuf,
            ibuf,
            vbuf_cap: INITIAL_VERTS,
            ibuf_cap: INITIAL_INDICES,
            index_count: 0,
            verts_scratch: Vec::with_capacity(INITIAL_VERTS),
            indices_scratch: Vec::with_capacity(INITIAL_INDICES),
            last_pointer_world: None,
        }
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn screen_to_world(&self, pt: Pointer) -> [f32; 2] {
        [
            (pt.x / self.width) * WORLD_W,
            (pt.y / self.height) * WORLD_H,
        ]
    }

    fn upload_mesh(&mut self, gpu: &Gpu) {
        self.index_count = self.indices_scratch.len() as u32;
        if self.index_count == 0 {
            return;
        }
        if self.verts_scratch.len() > self.vbuf_cap {
            self.vbuf_cap = (self.verts_scratch.len() * 3 / 2).max(INITIAL_VERTS);
            self.vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nebula-entity-vbuf"),
                size: (self.vbuf_cap * std::mem::size_of::<NebulaVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.indices_scratch.len() > self.ibuf_cap {
            self.ibuf_cap = (self.indices_scratch.len() * 3 / 2).max(INITIAL_INDICES);
            self.ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nebula-entity-ibuf"),
                size: (self.ibuf_cap * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        gpu.queue
            .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(&self.verts_scratch));
        gpu.queue
            .write_buffer(&self.ibuf, 0, bytemuck::cast_slice(&self.indices_scratch));
    }
    fn sync_gpu(&mut self, gpu: &Gpu) {
        let t = self.time as f32;
        let backdrop_uniforms = {
            let guard = match self.nebula.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let pal = lerp_palette(
                palette_for(guard.prev_palette),
                palette_for(guard.palette),
                guard.palette_blend,
            );

            // Pack live star lighting (up to 16)
            let mut stars_u = [[0.0f32; 4]; 16];
            let star_count = guard.stars.len().min(16);
            for (slot, s) in stars_u.iter_mut().zip(guard.stars.iter().take(16)) {
                let lum = match s.stage {
                    StarStage::Protostar { collapse } => 0.45 + 0.65 * collapse,
                    StarStage::MainSequence => s.luminosity + s.ignition_flash * 1.1,
                    StarStage::Supergiant { instability } => s.luminosity + instability * 0.8,
                };
                *slot = [s.pos[0] / WORLD_W, s.pos[1] / WORLD_H, lum, s.temp_t];
            }

            // Pack live supernova shockwaves (up to 6)
            let mut shockwaves_u = [[0.0f32; 4]; 6];
            let sw_count = guard.shockwaves.len().min(6);
            for (slot, sw) in shockwaves_u.iter_mut().zip(guard.shockwaves.iter().take(6)) {
                let progress = (sw.age / sw.max_age).clamp(0.0, 1.0);
                let fade = (1.0 - progress).powf(1.3);
                let r_norm = sw.radius / WORLD_H;
                *slot = [sw.pos[0] / WORLD_W, sw.pos[1] / WORLD_H, r_norm, fade];
            }

            // Pack live cursor stir wakes (up to 12)
            let mut wakes_u = [[0.0f32; 4]; 12];
            let wake_count = guard.wakes.len().min(12);
            for (slot, w) in wakes_u.iter_mut().zip(guard.wakes.iter().take(12)) {
                let fade = 1.0 - (w.age / w.max_age).clamp(0.0, 1.0);
                let s = w.strength * fade;
                *slot = [
                    w.pos[0] / WORLD_W,
                    w.pos[1] / WORLD_H,
                    w.dir[0] * s,
                    w.dir[1] * s,
                ];
            }

            let uniforms = BackdropUniforms {
                resolution_and_time: [self.width, self.height, t, star_count as f32],
                counts_and_flags: [
                    sw_count as f32,
                    wake_count as f32,
                    guard.pulsar.intensity,
                    0.0,
                ],
                pulsar_data: [
                    guard.pulsar.pos[0] / WORLD_W,
                    guard.pulsar.pos[1] / WORLD_H,
                    guard.pulsar.angle,
                    guard.pulsar.pulse_phase,
                ],
                deep_space: pal.deep_space,
                gas_primary: pal.gas_primary,
                gas_secondary: pal.gas_secondary,
                dust_dark: pal.dust_dark,
                rim_glow: pal.rim_glow,
                star_warm: pal.star_warm,
                star_cool: pal.star_cool,
                stars: stars_u,
                shockwaves: shockwaves_u,
                wakes: wakes_u,
            };

            let mut builder = NebulaMeshBuilder::new(
                &mut self.verts_scratch,
                &mut self.indices_scratch,
                self.width,
                self.height,
            );
            builder.push_dust_motes(&guard.dust, &pal, t);
            builder.push_shockwave_filaments(&guard.shockwaves, &pal);
            builder.push_star_chart(
                &guard.stars,
                &guard.constellation_edges,
                &guard.pulsar,
                &pal,
                guard.star_chart_alpha,
                t,
            );
            builder.push_comets(&guard.comets, &pal);
            builder.push_meteors(&guard.meteors);
            builder.push_pulsar(&guard.pulsar, &pal);
            builder.push_nursery_stars(&guard.stars, &pal, t);

            uniforms
        };

        gpu.queue.write_buffer(
            &self.backdrop_uniforms_buf,
            0,
            bytemuck::bytes_of(&backdrop_uniforms),
        );
        let entity_uniforms = EntityUniforms {
            resolution_and_time: [self.width, self.height, t, 0.0],
        };
        gpu.queue.write_buffer(
            &self.entity_uniforms_buf,
            0,
            bytemuck::bytes_of(&entity_uniforms),
        );
        self.upload_mesh(gpu);
    }
}

impl Scene for CosmicNebulaScene {
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
        self.sync_gpu(gpu);
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.15);
        self.time += dt_secs as f64;

        let pointer_world = input.pointer.map(|pt| self.screen_to_world(pt));

        let (activity, sounds) = {
            let mut guard = match self.nebula.lock() {
                Ok(g) => g,
                Err(_) => return Activity::Idle,
            };

            if input.interactive {
                if let Some(pw) = pointer_world {
                    match input.action {
                        PointerAction::LeftClick => guard.ignite_star(Some(pw)),
                        PointerAction::RightClick => guard.trigger_supernova(Some(pw)),
                        PointerAction::None => {}
                    }
                    if input.left_held
                        && let Some(prev) = self.last_pointer_world
                    {
                        let delta = [pw[0] - prev[0], pw[1] - prev[1]];
                        guard.stir_at(pw, delta);
                    }
                    self.last_pointer_world = Some(pw);
                } else {
                    self.last_pointer_world = None;
                }
            } else {
                self.last_pointer_world = None;
            }

            let act = guard.update(dt_secs, input.interactive, pointer_world);
            let cues = guard.drain_sounds();
            (act, cues)
        };

        // Dispatch audio cues outside mutex lock
        if let Some(ref audio) = self.audio {
            for cue in sounds {
                let sfx = match cue {
                    NebulaSoundCue::StarIgnite => SoundEffect::StarIgnite,
                    NebulaSoundCue::SupernovaBoom => SoundEffect::SupernovaBoom,
                    NebulaSoundCue::CometSweep => SoundEffect::CometSweep,
                    NebulaSoundCue::NebulaPulse => SoundEffect::NebulaPulse,
                    NebulaSoundCue::PaletteCycle => SoundEffect::PaletteCycle,
                };
                audio.play_sfx(sfx);
            }
        }

        self.sync_gpu(gpu);
        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Pass 1: Volumetric JWST Nebula Backdrop
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nebula-backdrop-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.015,
                            b: 0.035,
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
            rpass.set_pipeline(&self.backdrop_pipeline);
            rpass.set_bind_group(0, &self.backdrop_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Pass 2: Painterly Space Entities (JWST Stars, Motes, Comets, Meteors, Star Chart)
        if self.index_count > 0 {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nebula-entities-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.entity_pipeline);
            rpass.set_bind_group(0, &self.entity_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vbuf.slice(..));
            rpass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }
}
