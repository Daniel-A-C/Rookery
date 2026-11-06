//! Font glyph atlas & Signed Distance Field (SDF) generator.
//!
//! Generates a crisp, continuous 512x512 R8Unorm SDF atlas for ASCII characters 32..=126
//! plus custom symbols (musical notes `♪`, `♫`, leaves `☘`) from an embedded 8x16 bitmap font.
//!
//! Pure Rust, zero external binary fonts, 100% MIT-compatible.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::Gpu;

pub const ATLAS_WIDTH: u32 = 512;
pub const ATLAS_HEIGHT: u32 = 512;
pub const CELL_SIZE: u32 = 32;
pub const COLS_PER_ROW: u32 = ATLAS_WIDTH / CELL_SIZE; // 16
pub const GLYPH_COUNT: u32 = 98; // 95 ASCII printable + 3 custom (music notes & leaf)

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GlyphUniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub wind_sway: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

/// Shared instanced SDF glyph renderer used across all ASCII scenes.
pub struct GlyphPass {
    pub instances: Vec<GlyphInstance>,
    max_instances: usize,
    quad_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    uniforms_buf: wgpu::Buffer,
    bind_group0: wgpu::BindGroup,
    bind_group1: wgpu::BindGroup,
    _atlas_texture: wgpu::Texture,
    _atlas_view: wgpu::TextureView,
}

impl GlyphPass {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, max_instances: usize) -> Self {
        let device = &gpu.device;
        let (_atlas_texture, _atlas_view, sampler) = create_glyph_atlas(gpu);

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
            label: Some("glyph-quad-buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-instance-buffer"),
            size: (max_instances * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let glyph_source = include_str!("ascii.wgsl");
        let glyph_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glyph-shader"),
            source: wgpu::ShaderSource::Wgsl(glyph_source.into()),
        });

        let layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-layout0"),
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

        let uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-uniforms"),
            size: std::mem::size_of::<GlyphUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-bg0"),
            layout: &layout0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buf.as_entire_binding(),
            }],
        });

        let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-layout1"),
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

        let bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-bg1"),
            layout: &layout1,
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
            label: Some("glyph-pipeline-layout"),
            bind_group_layouts: &[Some(&layout0), Some(&layout1)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph-pipeline"),
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
            instances: Vec::with_capacity(max_instances.min(2048)),
            max_instances,
            quad_buffer,
            instance_buffer,
            pipeline,
            uniforms_buf,
            bind_group0,
            bind_group1,
            _atlas_texture,
            _atlas_view,
        }
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        c: char,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        depth: f32,
        color: [f32; 4],
        sway: f32,
    ) {
        if self.instances.len() >= self.max_instances {
            return;
        }
        let glyph_idx = char_to_glyph_index(c);
        let uv_bounds = glyph_uv_bounds(glyph_idx);
        self.instances.push(GlyphInstance {
            pos,
            size,
            rotation,
            depth,
            uv_bounds,
            color,
            sway,
        });
    }

    pub fn upload(&self, gpu: &Gpu, resolution: [f32; 2], time: f32, wind_sway: f32) {
        let uniforms = GlyphUniforms {
            resolution,
            time,
            wind_sway,
            _pad: 0.0,
        };
        gpu.queue
            .write_buffer(&self.uniforms_buf, 0, bytemuck::bytes_of(&uniforms));
        if !self.instances.is_empty() {
            gpu.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }
    }

    pub fn render(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        if self.instances.is_empty() {
            return;
        }
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glyph-pass"),
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
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group0, &[]);
        rpass.set_bind_group(1, &self.bind_group1, &[]);
        rpass.set_vertex_buffer(0, self.quad_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.draw(0..6, 0..self.instances.len() as u32);
    }
}

/// Maps a character to its glyph index in the atlas.
pub fn char_to_glyph_index(c: char) -> u32 {
    let u = c as u32;
    match c {
        '♪' => 95,
        '♫' => 96,
        '☘' => 97,
        ' '..='~' => u - 32,
        _ => 0, // space fallback
    }
}

/// UV bounds for a glyph in the atlas.
pub fn glyph_uv_bounds(glyph_idx: u32) -> [f32; 4] {
    let idx = glyph_idx.min(GLYPH_COUNT - 1);
    let col = idx % COLS_PER_ROW;
    let row = idx / COLS_PER_ROW;

    let u0 = (col * CELL_SIZE) as f32 / ATLAS_WIDTH as f32;
    let v0 = (row * CELL_SIZE) as f32 / ATLAS_HEIGHT as f32;
    let u1 = ((col + 1) * CELL_SIZE) as f32 / ATLAS_WIDTH as f32;
    let v1 = ((row + 1) * CELL_SIZE) as f32 / ATLAS_HEIGHT as f32;

    [u0, v0, u1, v1]
}

/// Creates the wgpu Texture and TextureView for the SDF glyph atlas.
pub fn create_glyph_atlas(gpu: &Gpu) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
    let sdf_data = generate_sdf_atlas();

    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph-sdf-atlas"),
        size: wgpu::Extent3d {
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &sdf_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(ATLAS_WIDTH),
            rows_per_image: Some(ATLAS_HEIGHT),
        },
        wgpu::Extent3d {
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("glyph-sdf-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    (texture, view, sampler)
}

/// Generates a 512x512 8-bit Signed Distance Field atlas from font bitmaps.
fn generate_sdf_atlas() -> Vec<u8> {
    let mut atlas = vec![0u8; (ATLAS_WIDTH * ATLAS_HEIGHT) as usize];

    for glyph_idx in 0..GLYPH_COUNT {
        let bitmap = get_glyph_bitmap_16x32(glyph_idx);
        let col = glyph_idx % COLS_PER_ROW;
        let row = glyph_idx / COLS_PER_ROW;
        let cell_ox = col * CELL_SIZE;
        let cell_oy = row * CELL_SIZE;

        // In the 32x32 cell, the 16x32 glyph is centered with x offset = 8
        let ox = 8;
        let search_radius = 5.0f32;

        for cy in 0..CELL_SIZE {
            for cx in 0..CELL_SIZE {
                let inside = if cx >= ox && cx < ox + 16 {
                    let gx = (cx - ox) as usize;
                    let gy = cy as usize;
                    bitmap[gy * 16 + gx]
                } else {
                    false
                };

                let mut min_dist_sq = search_radius * search_radius;

                // Search neighborhood for boundary transition
                let r_int = search_radius.ceil() as i32;
                let cxi = cx as i32;
                let cyi = cy as i32;

                for dy in -r_int..=r_int {
                    let ny = cyi + dy;
                    if ny < 0 || ny >= CELL_SIZE as i32 {
                        continue;
                    }
                    for dx in -r_int..=r_int {
                        let nx = cxi + dx;
                        if nx < 0 || nx >= CELL_SIZE as i32 {
                            continue;
                        }

                        let n_inside = if nx >= ox as i32 && nx < (ox + 16) as i32 {
                            let gx = (nx - ox as i32) as usize;
                            let gy = ny as usize;
                            bitmap[gy * 16 + gx]
                        } else {
                            false
                        };

                        if n_inside != inside {
                            let d2 = (dx * dx + dy * dy) as f32;
                            if d2 < min_dist_sq {
                                min_dist_sq = d2;
                            }
                        }
                    }
                }

                let dist = min_dist_sq.sqrt();
                let signed_dist = if inside { dist } else { -dist };

                // Map [-search_radius, +search_radius] to [0, 255] with 128 as edge
                let normalized = (0.5 + 0.5 * (signed_dist / search_radius)).clamp(0.0, 1.0);
                let byte_val = (normalized * 255.0).round() as u8;

                let px = cell_ox + cx;
                let py = cell_oy + cy;
                atlas[(py * ATLAS_WIDTH + px) as usize] = byte_val;
            }
        }
    }

    atlas
}

/// Returns a 16x32 boolean grid for a glyph (scaled 2x from 8x16 base font).
fn get_glyph_bitmap_16x32(glyph_idx: u32) -> [bool; 16 * 32] {
    let mut grid = [false; 16 * 32];
    let base_8x16 = get_glyph_8x16(glyph_idx);

    for (y, &row_bits) in base_8x16.iter().enumerate() {
        for x in 0..8 {
            let is_on = (row_bits & (0x80 >> x)) != 0;
            if is_on {
                let gx = x * 2;
                let gy = y * 2;
                grid[gy * 16 + gx] = true;
                grid[gy * 16 + (gx + 1)] = true;
                grid[(gy + 1) * 16 + gx] = true;
                grid[(gy + 1) * 16 + (gx + 1)] = true;
            }
        }
    }

    grid
}

/// Embedded 8x16 font table for characters.
fn get_glyph_8x16(glyph_idx: u32) -> [u8; 16] {
    if glyph_idx >= FONT_8X16.len() as u32 {
        return [0; 16];
    }
    FONT_8X16[glyph_idx as usize]
}

// 8x16 bitmap font: 95 ASCII characters (space..~) + 3 custom symbols (♪, ♫, ☘)
const FONT_8X16: [[u8; 16]; 98] = [
    // 0: ' ' (space)
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 1: '!'
    [
        0x00, 0x00, 0x18, 0x3C, 0x3C, 0x3C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 2: '"'
    [
        0x00, 0x66, 0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 3: '#'
    [
        0x00, 0x00, 0x00, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 4: '$'
    [
        0x18, 0x18, 0x7C, 0xC6, 0xC2, 0xC0, 0x78, 0x06, 0x86, 0x86, 0x7C, 0x18, 0x18, 0x00, 0x00,
        0x00,
    ],
    // 5: '%'
    [
        0x00, 0x00, 0x00, 0xC6, 0xCC, 0x18, 0x30, 0x60, 0xC6, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 6: '&'
    [
        0x00, 0x00, 0x38, 0x6C, 0x68, 0x70, 0x3C, 0x66, 0xC6, 0xCE, 0x7B, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 7: '\''
    [
        0x00, 0x30, 0x30, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 8: '('
    [
        0x00, 0x0C, 0x18, 0x30, 0x30, 0x60, 0x60, 0x60, 0x30, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 9: ')'
    [
        0x00, 0x30, 0x18, 0x0C, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x0C, 0x18, 0x30, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 10: '*'
    [
        0x00, 0x00, 0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 11: '+'
    [
        0x00, 0x00, 0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 12: ','
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x08, 0x10, 0x00, 0x00,
        0x00,
    ],
    // 13: '-'
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 14: '.'
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 15: '/'
    [
        0x00, 0x00, 0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 16: '0'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xDB, 0xC3, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 17: '1'
    [
        0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 18: '2'
    [
        0x00, 0x00, 0x7C, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC6, 0xFE, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 19: '3'
    [
        0x00, 0x00, 0x7C, 0xC6, 0x06, 0x06, 0x3C, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 20: '4'
    [
        0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C, 0x1E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 21: '5'
    [
        0x00, 0x00, 0xFE, 0xC0, 0xC0, 0xF8, 0x0C, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 22: '6'
    [
        0x00, 0x00, 0x38, 0x60, 0xC0, 0xF8, 0xCC, 0xC6, 0xC6, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 23: '7'
    [
        0x00, 0x00, 0xFE, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 24: '8'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0x66, 0x3C, 0x66, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 25: '9'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0x7E, 0x06, 0x0C, 0x18, 0x70, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 26: ':'
    [
        0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 27: ';'
    [
        0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 28: '<'
    [
        0x00, 0x00, 0x06, 0x18, 0x60, 0x80, 0x60, 0x18, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 29: '='
    [
        0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 30: '>'
    [
        0x00, 0x00, 0x60, 0x18, 0x06, 0x01, 0x06, 0x18, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 31: '?'
    [
        0x00, 0x00, 0x7C, 0xC6, 0x0C, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 32: '@'
    [
        0x00, 0x00, 0x3C, 0x66, 0xCE, 0xDA, 0xDB, 0xCE, 0xC0, 0x60, 0x3E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 33: 'A'
    [
        0x00, 0x00, 0x18, 0x3C, 0x66, 0xC3, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 34: 'B'
    [
        0x00, 0x00, 0xFC, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0xFC, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 35: 'C'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC0, 0xC0, 0xC0, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 36: 'D'
    [
        0x00, 0x00, 0xF8, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x6C, 0xF8, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 37: 'E'
    [
        0x00, 0x00, 0xFE, 0x62, 0x60, 0x7C, 0x60, 0x60, 0x62, 0x62, 0xFE, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 38: 'F'
    [
        0x00, 0x00, 0xFE, 0x62, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 39: 'G'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC0, 0xCF, 0xC3, 0xC3, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 40: 'H'
    [
        0x00, 0x00, 0xC3, 0xC3, 0xC3, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 41: 'I'
    [
        0x00, 0x00, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 42: 'J'
    [
        0x00, 0x00, 0x0E, 0x06, 0x06, 0x06, 0x06, 0x06, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 43: 'K'
    [
        0x00, 0x00, 0xC6, 0xCC, 0xD8, 0xF0, 0xF0, 0xD8, 0xCC, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 44: 'L'
    [
        0x00, 0x00, 0xF0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x62, 0x66, 0xFE, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 45: 'M'
    [
        0x00, 0x00, 0xC3, 0xE7, 0xFF, 0xDB, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 46: 'N'
    [
        0x00, 0x00, 0xC3, 0xE3, 0xF3, 0xDB, 0xCF, 0xC7, 0xC3, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 47: 'O'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 48: 'P'
    [
        0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 49: 'Q'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0xC3, 0xDB, 0x66, 0x3C, 0x0E, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 50: 'R'
    [
        0x00, 0x00, 0xFC, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x66, 0xC6, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 51: 'S'
    [
        0x00, 0x00, 0x3C, 0x66, 0xC0, 0x78, 0x0C, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 52: 'T'
    [
        0x00, 0x00, 0x7E, 0x5A, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 53: 'U'
    [
        0x00, 0x00, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 54: 'V'
    [
        0x00, 0x00, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 55: 'W'
    [
        0x00, 0x00, 0xC3, 0xC3, 0xC3, 0xC3, 0xDB, 0xFF, 0xE7, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 56: 'X'
    [
        0x00, 0x00, 0xC3, 0xC3, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 57: 'Y'
    [
        0x00, 0x00, 0xC3, 0xC3, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 58: 'Z'
    [
        0x00, 0x00, 0xFF, 0xC3, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC3, 0xFF, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 59: '['
    [
        0x00, 0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 60: '\'
    [
        0x00, 0x00, 0x80, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 61: ']'
    [
        0x00, 0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 62: '^'
    [
        0x18, 0x3C, 0x66, 0xC3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 63: '_'
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
        0x00,
    ],
    // 64: '`'
    [
        0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 65: 'a'
    [
        0x00, 0x00, 0x00, 0x00, 0x78, 0x0C, 0x7C, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 66: 'b'
    [
        0x00, 0xE0, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 67: 'c'
    [
        0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0xC0, 0xC0, 0xC0, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 68: 'd'
    [
        0x00, 0x0E, 0x06, 0x06, 0x3E, 0x66, 0xC6, 0xC6, 0xC6, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 69: 'e'
    [
        0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0xC3, 0xFF, 0xC0, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 70: 'f'
    [
        0x00, 0x1C, 0x36, 0x30, 0x78, 0x30, 0x30, 0x30, 0x30, 0x30, 0x78, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 71: 'g'
    [
        0x00, 0x00, 0x00, 0x00, 0x3E, 0x66, 0xC6, 0xC6, 0x7E, 0x06, 0x0C, 0x78, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 72: 'h'
    [
        0x00, 0xE0, 0x60, 0x60, 0x6C, 0x76, 0x66, 0x66, 0x66, 0x66, 0xE7, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 73: 'i'
    [
        0x00, 0x18, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 74: 'j'
    [
        0x00, 0x0C, 0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x4C, 0x38, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 75: 'k'
    [
        0x00, 0xE0, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0xE7, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 76: 'l'
    [
        0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 77: 'm'
    [
        0x00, 0x00, 0x00, 0x00, 0xE6, 0xDB, 0xDB, 0xDB, 0xDB, 0xDB, 0xE7, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 78: 'n'
    [
        0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 79: 'o'
    [
        0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 80: 'p'
    [
        0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x7C, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 81: 'q'
    [
        0x00, 0x00, 0x00, 0x00, 0x3B, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x0F, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 82: 'r'
    [
        0x00, 0x00, 0x00, 0x00, 0xDC, 0x76, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 83: 's'
    [
        0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0x60, 0x38, 0x0C, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 84: 't'
    [
        0x00, 0x10, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x34, 0x18, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 85: 'u'
    [
        0x00, 0x00, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 86: 'v'
    [
        0x00, 0x00, 0x00, 0x00, 0xC3, 0xC3, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 87: 'w'
    [
        0x00, 0x00, 0x00, 0x00, 0xC3, 0xC3, 0xDB, 0xDB, 0xFF, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 88: 'x'
    [
        0x00, 0x00, 0x00, 0x00, 0xC3, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0xC3, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 89: 'y'
    [
        0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xC6, 0x7E, 0x06, 0x0C, 0x78, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 90: 'z'
    [
        0x00, 0x00, 0x00, 0x00, 0xFE, 0xCE, 0x18, 0x30, 0x60, 0x73, 0xFE, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 91: '{'
    [
        0x00, 0x0E, 0x18, 0x18, 0x18, 0x70, 0x18, 0x18, 0x18, 0x18, 0x0E, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 92: '|'
    [
        0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 93: '}'
    [
        0x00, 0x70, 0x18, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x18, 0x18, 0x70, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 94: '~'
    [
        0x00, 0x76, 0xDC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 95: '♪' (musical eighth note)
    [
        0x00, 0x08, 0x18, 0x18, 0x18, 0x18, 0x19, 0x3B, 0x3B, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 96: '♫' (musical beamed notes)
    [
        0x00, 0x1F, 0x1B, 0x1B, 0x1B, 0x1B, 0x3B, 0x3B, 0x1B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    // 97: '☘' (three-leaf clover / sprig)
    [
        0x00, 0x38, 0x7C, 0x7C, 0xFE, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, 0x18, 0x38, 0x00, 0x00, 0x00,
        0x00,
    ],
];
