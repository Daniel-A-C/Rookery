//! Cosmic Nebula Scene (Milestone 4 - Generative Physics & Shaders).
//!
//! Algorithmic deep-space simulation with glowing celestial particles, shifting gravitational
//! attractor/repulsor vortexes, and customizable stellar palettes and turbulence.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::nebula::{CosmicNebulaState, NebulaPalette};
use wgpu::util::DeviceExt;

use crate::ascii_scene::GlyphInstance;
use crate::atlas::{self, char_to_glyph_index, glyph_uv_bounds};
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceUniforms {
    resolution: [f32; 2],
    time: f32,
    wind_sway: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct CosmicNebulaScene {
    pub nebula: Arc<Mutex<CosmicNebulaState>>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,

    instances: Vec<GlyphInstance>,
    quad_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,

    bg_pipeline: wgpu::RenderPipeline,
    bg_uniforms_buf: wgpu::Buffer,
    bg_bind_group: wgpu::BindGroup,

    glyph_pipeline: wgpu::RenderPipeline,
    glyph_uniforms_buf: wgpu::Buffer,
    glyph_bind_group0: wgpu::BindGroup,
    glyph_bind_group1: wgpu::BindGroup,

    _atlas_texture: wgpu::Texture,
    _atlas_view: wgpu::TextureView,
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

        // 2. Atlas texture & sampler
        let (_atlas_texture, _atlas_view, sampler) = atlas::create_glyph_atlas(gpu);

        let quad_vertices = [
            QuadVertex {
                pos: [-0.5, -0.5],
                uv: [0.0, 0.0],
            },
            QuadVertex {
                pos: [0.5, -0.5],
                uv: [1.0, 0.0],
            },
            QuadVertex {
                pos: [0.5, 0.5],
                uv: [1.0, 1.0],
            },
            QuadVertex {
                pos: [-0.5, -0.5],
                uv: [0.0, 0.0],
            },
            QuadVertex {
                pos: [0.5, 0.5],
                uv: [1.0, 1.0],
            },
            QuadVertex {
                pos: [-0.5, 0.5],
                uv: [0.0, 1.0],
            },
        ];
        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nebula-quad-buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-instance-buffer"),
            size: (MAX_INSTANCES * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let glyph_source = include_str!("ascii.wgsl");
        let glyph_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nebula-glyph-shader"),
            source: wgpu::ShaderSource::Wgsl(glyph_source.into()),
        });

        let glyph_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nebula-glyph-layout0"),
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

        let glyph_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nebula-glyph-uniforms"),
            size: std::mem::size_of::<InstanceUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let glyph_bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nebula-glyph-bg0"),
            layout: &glyph_layout0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: glyph_uniforms_buf.as_entire_binding(),
            }],
        });

        let glyph_layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nebula-glyph-layout1"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let glyph_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nebula-glyph-bg1"),
            layout: &glyph_layout1,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nebula-glyph-pipeline-layout"),
            bind_group_layouts: &[Some(&glyph_layout0), Some(&glyph_layout1)],
            immediate_size: 0,
        });

        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nebula-glyph-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &glyph_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                                shader_location: 1,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 16,
                                shader_location: 4,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 20,
                                shader_location: 5,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 24,
                                shader_location: 6,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 40,
                                shader_location: 7,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 56,
                                shader_location: 8,
                            },
                        ],
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &glyph_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            nebula,
            width: 800.0,
            height: 600.0,
            scale_factor: 1.0,
            time: 0.0,
            instances: Vec::with_capacity(1024),
            quad_buffer,
            instance_buffer,
            bg_pipeline,
            bg_uniforms_buf,
            bg_bind_group,
            glyph_pipeline,
            glyph_uniforms_buf,
            glyph_bind_group0,
            glyph_bind_group1,
            _atlas_texture,
            _atlas_view,
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
        if self.instances.len() >= MAX_INSTANCES {
            return;
        }
        let glyph_idx = char_to_glyph_index(c);
        let uv = glyph_uv_bounds(glyph_idx);
        self.instances.push(GlyphInstance {
            pos,
            size,
            rotation,
            depth,
            uv_bounds: uv,
            color,
            sway,
        });
    }

    fn generate_instances(&mut self) {
        self.instances.clear();

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
                    }
                    PointerAction::RightClick => {
                        guard.add_gravity_well(pw, true);
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

        let glyph_uniforms = InstanceUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            wind_sway: 0.0,
            _pad: 0.0,
        };
        gpu.queue.write_buffer(
            &self.glyph_uniforms_buf,
            0,
            bytemuck::bytes_of(&glyph_uniforms),
        );

        self.generate_instances();
        gpu.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );

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
        if !self.instances.is_empty() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nebula-glyph-pass"),
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
            rpass.set_pipeline(&self.glyph_pipeline);
            rpass.set_bind_group(0, &self.glyph_bind_group0, &[]);
            rpass.set_bind_group(1, &self.glyph_bind_group1, &[]);
            rpass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rpass.draw(0..6, 0..self.instances.len() as u32);
        }
    }
}
