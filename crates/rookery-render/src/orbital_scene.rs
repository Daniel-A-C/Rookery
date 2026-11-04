//! Orbital Haven Scene (Milestone 4 - Slow-Burn Idle World).
//!
//! A modular orbital station assembling modules and habitat rings over real time,
//! serviced by automated cargo transport shuttles against a planetary horizon.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::orbital::{ConstructionFocus, OrbitalHavenState};
use wgpu::util::DeviceExt;

use crate::ascii_scene::GlyphInstance;
use crate::atlas::{self, char_to_glyph_index, glyph_uv_bounds};
use crate::fullscreen;
use crate::{Gpu, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OrbitalUniforms {
    resolution: [f32; 2],
    time: f32,
    orbit_angle: f32,
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

pub struct OrbitalHavenScene {
    pub station: Arc<Mutex<OrbitalHavenState>>,
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

impl OrbitalHavenScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        station: Arc<Mutex<OrbitalHavenState>>,
    ) -> Self {
        let device = &gpu.device;

        // 1. Orbital planet background setup
        let bg_source = include_str!("orbital.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "orbital-bg-layout");
        let bg_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-bg-uniforms"),
            size: std::mem::size_of::<OrbitalUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms_buf.as_entire_binding(),
            }],
        });
        let bg_pipeline = fullscreen::pipeline(
            device,
            "orbital-bg-pipeline",
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
            label: Some("orbital-quad-buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-instance-buffer"),
            size: (MAX_INSTANCES * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let glyph_source = include_str!("ascii.wgsl");
        let glyph_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orbital-glyph-shader"),
            source: wgpu::ShaderSource::Wgsl(glyph_source.into()),
        });

        let glyph_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("orbital-glyph-layout0"),
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
            label: Some("orbital-glyph-uniforms"),
            size: std::mem::size_of::<InstanceUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let glyph_bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-glyph-bg0"),
            layout: &glyph_layout0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: glyph_uniforms_buf.as_entire_binding(),
            }],
        });

        let glyph_layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("orbital-glyph-layout1"),
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
            label: Some("orbital-glyph-bg1"),
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
            label: Some("orbital-glyph-pipeline-layout"),
            bind_group_layouts: &[Some(&glyph_layout0), Some(&glyph_layout1)],
            immediate_size: 0,
        });

        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orbital-glyph-pipeline"),
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
            station,
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

        let st_arc = self.station.clone();
        let st_guard = match st_arc.lock() {
            Ok(s) => s,
            Err(_) => return,
        };

        let scale = (self.height / 750.0).clamp(0.6, 2.0);
        let station_pos = self.world_to_screen([500.0, 340.0]);

        let hull_col = [0.85, 0.88, 0.95, 0.98];
        let dark_hull = [0.35, 0.38, 0.45, 0.95];
        let solar_col = [0.15, 0.45, 0.88, 0.98];
        let window_glow = [0.98, 0.85, 0.35, 0.95];

        // 1. Central Core Hub
        let hub_size = [28.0 * scale, 28.0 * scale];
        self.push_glyph('O', station_pos, hub_size, 0.0, 0.1, hull_col, 0.0);
        self.push_glyph(
            '+',
            station_pos,
            [18.0 * scale, 18.0 * scale],
            0.0,
            0.11,
            dark_hull,
            0.0,
        );

        // 2. Rotating Centrifugal Habitat Ring Sectors
        let ring_radius = 68.0 * scale;
        let rot = st_guard.ring_rotation;
        let active_sectors = st_guard.habitat_sectors.min(12);
        for i in 0..12 {
            let angle = rot + (i as f32 / 12.0) * std::f32::consts::TAU;
            let sx = station_pos[0] + angle.cos() * ring_radius;
            let sy = station_pos[1] + angle.sin() * ring_radius * 0.45;

            // Spokes connecting to hub
            if i % 3 == 0 {
                let mid_x = (station_pos[0] + sx) * 0.5;
                let mid_y = (station_pos[1] + sy) * 0.5;
                self.push_glyph(
                    '-',
                    [mid_x, mid_y],
                    [24.0 * scale, 12.0 * scale],
                    angle,
                    0.08,
                    dark_hull,
                    0.0,
                );
            }

            // Ring sector module
            if i < active_sectors {
                self.push_glyph(
                    'o',
                    [sx, sy],
                    [14.0 * scale, 14.0 * scale],
                    angle,
                    0.12,
                    hull_col,
                    0.0,
                );
                self.push_glyph(
                    '.',
                    [sx, sy],
                    [8.0 * scale, 8.0 * scale],
                    0.0,
                    0.13,
                    window_glow,
                    0.0,
                );
            } else {
                self.push_glyph(
                    ':',
                    [sx, sy],
                    [10.0 * scale, 10.0 * scale],
                    angle,
                    0.06,
                    [0.45, 0.50, 0.55, 0.55],
                    0.0,
                );
            }
        }

        // 3. Solar Array Wings extending outward horizontally
        let wing_count = st_guard.solar_wings.min(8);
        for w in 0..wing_count {
            let side = if w % 2 == 0 { 1.0 } else { -1.0 };
            let seg = (w / 2) as f32 + 1.0;
            let wing_x = station_pos[0] + side * (ring_radius + seg * 24.0 * scale);
            let wing_y = station_pos[1];

            self.push_glyph(
                '#',
                [wing_x, wing_y],
                [20.0 * scale, 16.0 * scale],
                0.0,
                0.09,
                solar_col,
                0.0,
            );
        }

        // 4. Communication & Sensor Array Dishes
        let dish_count = st_guard.comm_arrays.min(6);
        for d in 0..dish_count {
            let dy = station_pos[1] - (hub_size[1] * 0.7 + (d as f32) * 14.0 * scale);
            let dish_col = [0.92, 0.95, 0.98, 0.95];
            self.push_glyph(
                ')',
                [station_pos[0], dy],
                [14.0 * scale, 14.0 * scale],
                1.57,
                0.15,
                dish_col,
                0.0,
            );
        }

        // 5. Autonomous Transport Shuttles & Thruster Plumes
        for shuttle in &st_guard.shuttles {
            let s_pos = self.world_to_screen(shuttle.pos);
            let rot = shuttle.vel[1].atan2(shuttle.vel[0]);
            let s_size = [16.0 * scale, 16.0 * scale];

            if shuttle.vel[0].abs() > 1.0 || shuttle.vel[1].abs() > 1.0 {
                let plume_x = s_pos[0] - shuttle.vel[0] * 0.25 * scale;
                let plume_y = s_pos[1] - shuttle.vel[1] * 0.25 * scale;
                self.push_glyph(
                    '*',
                    [plume_x, plume_y],
                    [12.0 * scale, 12.0 * scale],
                    rot,
                    0.14,
                    [0.35, 0.75, 0.98, 0.85],
                    0.0,
                );
            }

            self.push_glyph('>', s_pos, s_size, rot, 0.16, [0.95, 0.92, 0.88, 0.98], 0.0);
        }
    }
}

impl Scene for OrbitalHavenScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);
        self.time += dt_secs as f64;

        let (activity, orbit_angle) = {
            let mut guard = match self.station.lock() {
                Ok(g) => g,
                Err(_) => return Activity::Idle,
            };
            if input.interactive {
                match input.action {
                    PointerAction::LeftClick => {
                        guard.launch_shuttle();
                    }
                    PointerAction::RightClick => {
                        let next_focus = match guard.focus {
                            ConstructionFocus::SolarPower => ConstructionFocus::HabitatRing,
                            ConstructionFocus::HabitatRing => ConstructionFocus::CommsGrid,
                            ConstructionFocus::CommsGrid => ConstructionFocus::SolarPower,
                        };
                        guard.set_focus(next_focus);
                    }
                    PointerAction::None => {}
                }
            }
            let act = guard.update(dt_secs, input.interactive, None);
            (act, guard.orbit_angle)
        };

        let orb_uniforms = OrbitalUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            orbit_angle,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&orb_uniforms));

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
        // Pass 1: Planet horizon background
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("orbital-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.008,
                            g: 0.012,
                            b: 0.022,
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
                label: Some("orbital-glyph-pass"),
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
