//! The Living ASCII Rookery: Signature Launch App Scene (Milestone 2).
//!
//! An algorithmic, layered ASCII tree that grows imperceptibly slowly over days and weeks,
//! inhabited by autonomous procedural birds that glide, flock, perch, sing, and nest.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::world::{BirdSpecies, BirdState, WorldState};
use wgpu::util::DeviceExt;

use crate::atlas::{self, char_to_glyph_index, glyph_uv_bounds};
use crate::fullscreen;
use crate::{Gpu, Pointer, Scene, SceneInput};

const MAX_INSTANCES: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BgUniforms {
    resolution: [f32; 2],
    time: f32,
    time_of_day: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AsciiUniforms {
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct GlyphInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub depth: f32,
    pub uv_bounds: [f32; 4],
    pub color: [f32; 4],
    pub sway: f32,
}

pub struct LivingRookeryScene {
    pub world: Arc<Mutex<WorldState>>,
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

    ascii_pipeline: wgpu::RenderPipeline,
    ascii_uniforms_buf: wgpu::Buffer,
    ascii_bind_group0: wgpu::BindGroup,
    ascii_bind_group1: wgpu::BindGroup,

    _atlas_texture: wgpu::Texture,
    _atlas_view: wgpu::TextureView,
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

        // 2. Atlas texture & sampler
        let (_atlas_texture, _atlas_view, sampler) = atlas::create_glyph_atlas(gpu);

        // 3. Ascii glyph uniform buffer & bind group 0
        let ascii_source = include_str!("ascii.wgsl");
        let ascii_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ascii-shader-module"),
            source: wgpu::ShaderSource::Wgsl(ascii_source.into()),
        });

        let ascii_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ascii-layout0"),
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

        let ascii_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ascii-uniforms"),
            size: std::mem::size_of::<AsciiUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ascii_bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ascii-bg0"),
            layout: &ascii_layout0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ascii_uniforms_buf.as_entire_binding(),
            }],
        });

        // 4. Atlas texture bind group 1
        let ascii_layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ascii-layout1"),
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

        let ascii_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ascii-bg1"),
            layout: &ascii_layout1,
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
            label: Some("ascii-pipeline-layout"),
            bind_group_layouts: &[Some(&ascii_layout0), Some(&ascii_layout1)],
            immediate_size: 0,
        });

        // Vertex buffer for quad: 2 triangles
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
            label: Some("ascii-quad-buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ascii-instance-buffer"),
            size: (MAX_INSTANCES * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ascii_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ascii-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ascii_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    // Quad vertex buffer
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
                    // Instance buffer
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 2, // pos
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                                shader_location: 3, // size
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 16,
                                shader_location: 4, // rotation
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 20,
                                shader_location: 5, // depth
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 24,
                                shader_location: 6, // uv_bounds
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 40,
                                shader_location: 7, // color
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 56,
                                shader_location: 8, // sway
                            },
                        ],
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &ascii_module,
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
            world,
            width: 1920.0,
            height: 1080.0,
            scale_factor: 1.0,
            time: 0.0,
            instances: Vec::with_capacity(2048),
            quad_buffer,
            instance_buffer,
            bg_pipeline,
            bg_uniforms_buf,
            bg_bind_group,
            ascii_pipeline,
            ascii_uniforms_buf,
            ascii_bind_group0,
            ascii_bind_group1,
            _atlas_texture,
            _atlas_view,
        }
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

    fn generate_scene_instances(&mut self) {
        self.instances.clear();

        let world_arc = self.world.clone();
        let world_guard = match world_arc.lock() {
            Ok(w) => w,
            Err(_) => return,
        };

        let offset_x = world_guard.viewport_offset_x;
        let tree_scale = (self.height / 900.0).clamp(0.6, 2.0);
        let base_glyph_size = [12.0 * tree_scale, 20.0 * tree_scale];

        // 1. Draw Tree Branches
        for branch in &world_guard.branches {
            let start = self.world_to_screen(branch.start, offset_x);
            let current_end_world = branch.current_end();
            let end = self.world_to_screen(current_end_world, offset_x);

            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len = (dx * dx + dy * dy).sqrt();
            let steps = (len / (base_glyph_size[1] * 0.55)).ceil().max(1.0) as usize;

            // Character choice based on angle and depth
            let angle_deg = branch.angle.to_degrees();
            let branch_char = if branch.depth == 0 {
                '#'
            } else if angle_deg.abs() < 18.0 {
                '|'
            } else if angle_deg < -45.0 {
                '/'
            } else if angle_deg > 45.0 {
                '\\'
            } else if angle_deg < 0.0 {
                '/'
            } else {
                '\\'
            };

            // Bark color: rich warm bark tones darkening near roots
            let bark_color = if branch.depth == 0 {
                [0.48, 0.36, 0.26, 0.95]
            } else if branch.depth <= 2 {
                [0.55, 0.42, 0.30, 0.90]
            } else {
                [0.45, 0.50, 0.35, 0.85] // twig green-brown
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

                // Multi-column trunk reinforcement for sturdy thick trees
                if branch.depth == 0 {
                    let col_offset = 7.0 * tree_scale;
                    self.push_glyph(
                        '|',
                        [px - col_offset, py],
                        [base_glyph_size[0] * 0.9, base_glyph_size[1]],
                        branch.angle * 0.4,
                        0.05,
                        [0.42, 0.32, 0.22, 0.90],
                        branch.sway_factor,
                    );
                    self.push_glyph(
                        '|',
                        [px + col_offset, py],
                        [base_glyph_size[0] * 0.9, base_glyph_size[1]],
                        branch.angle * 0.4,
                        0.05,
                        [0.42, 0.32, 0.22, 0.90],
                        branch.sway_factor,
                    );
                }
            }

            // Draw nest if present on this branch
            if branch.has_nest {
                let nest_pos = self.world_to_screen(branch.point_at(0.4), offset_x);
                self.push_glyph(
                    '\\',
                    [nest_pos[0] - 8.0, nest_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.05,
                    [0.65, 0.52, 0.35, 0.95],
                    branch.sway_factor,
                );
                self.push_glyph(
                    '=',
                    [nest_pos[0], nest_pos[1] + 2.0],
                    base_glyph_size,
                    0.0,
                    0.05,
                    [0.65, 0.52, 0.35, 0.95],
                    branch.sway_factor,
                );
                self.push_glyph(
                    '/',
                    [nest_pos[0] + 8.0, nest_pos[1]],
                    base_glyph_size,
                    0.0,
                    0.05,
                    [0.65, 0.52, 0.35, 0.95],
                    branch.sway_factor,
                );
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
                    0 => [0.22, 0.68, 0.38, 0.88 * bud.bloom], // emerald green
                    1 => [0.35, 0.78, 0.45, 0.85 * bud.bloom], // lush spring sage
                    2 => [0.82, 0.65, 0.22, 0.90 * bud.bloom], // golden autumn highlight
                    _ => [0.92, 0.52, 0.68, 0.92 * bud.bloom], // blossom pink
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

        // 3. Draw Autonomous Birds
        for bird in &world_guard.birds {
            let screen_pos = self.world_to_screen(bird.pos, offset_x);
            let bird_color = match bird.species {
                BirdSpecies::Rook => [0.85, 0.88, 0.95, 0.95], // bright silver-white
                BirdSpecies::AzureJay => [0.32, 0.68, 0.98, 0.95], // sapphire azure
                BirdSpecies::Goldfinch => [0.98, 0.82, 0.25, 0.95], // bright goldfinch
                BirdSpecies::Robin => [0.95, 0.45, 0.30, 0.95], // warm scarlet robin
                BirdSpecies::EmeraldDove => [0.35, 0.85, 0.65, 0.95], // emerald dove
            };

            let b_scale = bird.size_scale * tree_scale;
            let b_size = [base_glyph_size[0] * b_scale, base_glyph_size[1] * b_scale];

            match &bird.state {
                BirdState::Perched { .. } | BirdState::Feeding { .. } => {
                    // Perched resting bird: head, body, tail
                    let head_char = if bird.facing_left { '<' } else { '>' };
                    let body_char = 'o';
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
                    self.push_glyph(body_char, screen_pos, b_size, 0.0, 0.2, bird_color, 0.0);
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
                BirdState::Flying { .. }
                | BirdState::Circling { .. }
                | BirdState::Startled { .. } => {
                    // Flying animated bird: flapping wings based on wing_phase
                    let flap = bird.wing_phase.sin();
                    let (wing_left, body, wing_right) = if flap > 0.3 {
                        ('^', 'v', '^') // wings up
                    } else if flap < -0.3 {
                        ('v', '-', 'v') // wings down
                    } else {
                        ('-', 'o', '-') // gliding flat
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

        // 4. Draw Floating Musical Notes
        for note in &world_guard.notes {
            let screen_pos = self.world_to_screen(note.pos, offset_x);
            let alpha = (1.0 - (note.life / note.max_life)).clamp(0.0, 1.0);
            let note_color = match note.color_idx {
                0 => [0.95, 0.85, 0.40, alpha], // gold
                1 => [0.45, 0.85, 0.95, alpha], // sky blue
                _ => [0.95, 0.55, 0.75, alpha], // lavender
            };
            self.push_glyph(
                note.glyph,
                screen_pos,
                [base_glyph_size[0] * 1.1, base_glyph_size[1] * 1.1],
                0.0,
                0.3,
                note_color,
                0.0,
            );
        }

        // 5. Draw Falling Pruning Chips
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

        // 6. Draw Birdhouses on mature branches
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
                self.push_glyph(
                    '_',
                    [h_pos[0], h_pos[1] + 6.0 * tree_scale],
                    base_glyph_size,
                    0.0,
                    0.08,
                    [0.60, 0.45, 0.30, 0.95],
                    b.sway_factor,
                );
            }
        }

        // 7. Draw Planted Marker Signs
        for sign in &world_guard.signs {
            let s_pos = self.world_to_screen(sign.pos, offset_x);
            let sign_col = [0.68, 0.52, 0.32, 0.92];
            self.push_glyph(
                '|',
                [s_pos[0], s_pos[1] + 8.0 * tree_scale],
                base_glyph_size,
                0.0,
                0.02,
                [0.45, 0.35, 0.22, 0.95],
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
                [0.95, 0.90, 0.75, 0.95],
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
        }

        // 8. Forest Floor Moss and Grass Sprouts
        let ground_y = self.height * 0.90;
        let tuft_chars = ['"', ',', '.', '\'', '~', '☘'];
        let tufts_count = 24;
        for i in 0..tufts_count {
            let fx = (i as f32 / tufts_count as f32) * self.width;
            let c = tuft_chars[i % tuft_chars.len()];
            let moss_col = match i % 3 {
                0 => [0.28, 0.55, 0.30, 0.75],
                1 => [0.38, 0.65, 0.35, 0.70],
                _ => [0.45, 0.40, 0.28, 0.65],
            };
            self.push_glyph(
                c,
                [fx, ground_y + 2.0 * tree_scale],
                [base_glyph_size[0] * 0.75, base_glyph_size[1] * 0.75],
                0.0,
                0.01,
                moss_col,
                0.05,
            );
        }
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

        // Map pointer to world coordinates if interactive
        let pointer_world = input.pointer.map(|p| self.screen_to_world(p));

        let activity = if let Ok(mut w) = self.world.lock() {
            w.update(dt_secs, input.interactive, pointer_world)
        } else {
            Activity::Idle
        };

        // Update uniforms
        let bg_uniforms = BgUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            time_of_day: 0.15, // twilight/dusk
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&bg_uniforms));

        let ascii_uniforms = AsciiUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            wind_sway: 1.0,
            _pad: 0.0,
        };
        gpu.queue.write_buffer(
            &self.ascii_uniforms_buf,
            0,
            bytemuck::bytes_of(&ascii_uniforms),
        );

        // Generate and upload glyph instances
        self.generate_scene_instances();
        if !self.instances.is_empty() {
            gpu.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Render pass 1: Background atmospheric sky + stars + ground
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
        if !self.instances.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ascii-glyph-pass"),
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
            pass.set_pipeline(&self.ascii_pipeline);
            pass.set_bind_group(0, &self.ascii_bind_group0, &[]);
            pass.set_bind_group(1, &self.ascii_bind_group1, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.draw(0..6, 0..self.instances.len() as u32);
        }
    }
}
