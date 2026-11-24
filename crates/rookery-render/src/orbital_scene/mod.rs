//! Orbital Haven v3 Scene (Station HELIOS-09 — Y2K 3D Multi-Camera CCTV Surveillance).
//!
//! Renders a unified low-poly 3D orbital station with procedural pixelated textures,
//! seamless live-windowed interiors, autonomous 3D spacecraft & bots, and switchable
//! multi-feed CCTV layouts (Single / Main+PiP / 2x2 Grid) with diegetic telemetry HUD.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::orbital::{
    CameraLayout, OrbitalHavenState, OrbitalPalette, PAD_B_POS, PAD_C_POS, StationEventKind,
};
use wgpu::util::DeviceExt;

use crate::atlas::GlyphPass;
use crate::audio::{AudioEngine, SoundEffect};
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

mod cameras;
mod entities_mesh;
mod mesh_builder;
mod station_mesh;
mod textures;
mod theme;

use cameras::{build_perspective_view_proj, compute_viewports, resolve_camera};
use entities_mesh::build_dynamic_entities;
use mesh_builder::{MeshBuilder, OrbitalVertex};
use station_mesh::build_static_station;
use textures::{ATLAS_SIZE, generate_atlas_rgba};
use theme::palette_theme;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const MAX_FEEDS: usize = 4;
const MAX_HUD_GLYPHS: usize = 2048;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct PointLightGpu {
    pos_radius: [f32; 4],
    color_intensity: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Orbital3dUniforms {
    view_proj: [[f32; 4]; 4],
    eye_and_time: [f32; 4],
    sun_dir_and_power: [f32; 4],
    ambient_and_fog: [f32; 4],
    fog_color_and_pulse: [f32; 4],
    lights: [PointLightGpu; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ClearQuadUniforms {
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct PostUniforms {
    res_time_glitch: [f32; 4],
    layout_and_accent: [f32; 4],
    pip0: [f32; 4],
    pip1: [f32; 4],
}

const CLEAR_QUAD_WGSL: &str = r#"
struct ClearU { color: vec4<f32>, };
@group(0) @binding(0) var<uniform> u: ClearU;
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    let x = f32((idx << 1u) & 2u);
    let y = f32(idx & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.9999, 1.0);
}
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
"#;

fn create_scene_targets(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::TextureView) {
    let w = width.max(1);
    let h = height.max(1);
    let color_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("orbital-scene-color"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("orbital-scene-depth"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    (
        color_tex.create_view(&wgpu::TextureViewDescriptor::default()),
        depth_tex.create_view(&wgpu::TextureViewDescriptor::default()),
    )
}

pub struct OrbitalHavenScene {
    pub station: Arc<Mutex<OrbitalHavenState>>,
    pub audio: Option<AudioEngine>,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    scale_factor: f64,
    time: f64,

    // 3D pipelines & per-viewport uniform slots (up to 4 simultaneous CCTV feeds)
    opaque_pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    clear_pipeline: wgpu::RenderPipeline,
    clear_uniform_buf: wgpu::Buffer,
    clear_bind_group: wgpu::BindGroup,

    feed_uniform_bufs: [wgpu::Buffer; MAX_FEEDS],
    feed_bind_groups: [wgpu::BindGroup; MAX_FEEDS],

    // Static station geometry (rebuilt only on palette change)
    cached_palette: OrbitalPalette,
    static_op_vbuf: wgpu::Buffer,
    static_op_ibuf: wgpu::Buffer,
    static_op_indices: u32,
    static_tr_vbuf: wgpu::Buffer,
    static_tr_ibuf: wgpu::Buffer,
    static_tr_indices: u32,

    // Per-frame dynamic entity buffers
    dyn_op_builder: MeshBuilder,
    dyn_tr_builder: MeshBuilder,
    dyn_op_vbuf: wgpu::Buffer,
    dyn_op_ibuf: wgpu::Buffer,
    dyn_op_vcap: usize,
    dyn_op_icap: usize,
    dyn_op_indices: u32,
    dyn_tr_vbuf: wgpu::Buffer,
    dyn_tr_ibuf: wgpu::Buffer,
    dyn_tr_vcap: usize,
    dyn_tr_icap: usize,
    dyn_tr_indices: u32,

    // Offscreen render targets & CCTV post-processing compositor
    scene_color_view: wgpu::TextureView,
    scene_depth_view: wgpu::TextureView,
    post_pipeline: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    post_sampler: wgpu::Sampler,
    post_uniform_buf: wgpu::Buffer,
    post_bind_group: wgpu::BindGroup,

    // Diegetic CCTV HUD text overlay
    hud_glyphs: GlyphPass,

    // Direct Interaction Mode tracking
    last_pointer: Option<Pointer>,
    is_dragging: bool,
    drag_dist: f32,
}

impl OrbitalHavenScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        station: Arc<Mutex<OrbitalHavenState>>,
    ) -> Self {
        let device = &gpu.device;
        let queue = &gpu.queue;

        // 1. Upload procedural 512x512 Y2K pixel texture atlas with Nearest filtering
        let atlas_rgba = generate_atlas_rgba();
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("orbital-y2k-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE * 4),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("orbital-atlas-nearest"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // 2. 3D Textured Opaque & Translucent Pipelines
        let bgl_3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("orbital-3d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader_3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orbital-3d-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("orbital_3d.wgsl").into()),
        });
        let pl_3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("orbital-3d-pl"),
            bind_group_layouts: &[Some(&bgl_3d)],
            immediate_size: 0,
        });

        let make_3d_pipeline = |label: &str, translucent: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl_3d),
                vertex: wgpu::VertexState {
                    module: &shader_3d,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(OrbitalVertex::LAYOUT)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_3d,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(if translucent {
                            wgpu::BlendState::ALPHA_BLENDING
                        } else {
                            wgpu::BlendState::REPLACE
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: if translucent {
                        None
                    } else {
                        Some(wgpu::Face::Back)
                    },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(!translucent),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let opaque_pipeline = make_3d_pipeline("orbital-3d-opaque", false);
        let translucent_pipeline = make_3d_pipeline("orbital-3d-translucent", true);

        // 3. Per-viewport clear-backdrop quad pipeline (cleanly fills PiP mini-monitor space background)
        let clear_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("orbital-clear-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orbital-clear-shader"),
            source: wgpu::ShaderSource::Wgsl(CLEAR_QUAD_WGSL.into()),
        });
        let clear_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("orbital-clear-pl"),
            bind_group_layouts: &[Some(&clear_bgl)],
            immediate_size: 0,
        });
        let clear_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orbital-clear-pipeline"),
            layout: Some(&clear_pl),
            vertex: wgpu::VertexState {
                module: &clear_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &clear_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let clear_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-clear-uniform"),
            size: std::mem::size_of::<ClearQuadUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let clear_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-clear-bg"),
            layout: &clear_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: clear_uniform_buf.as_entire_binding(),
            }],
        });

        // 4. Allocate 4 independent uniform buffers & bind groups for simultaneous CCTV feeds
        let feed_uniform_bufs: [wgpu::Buffer; MAX_FEEDS] = std::array::from_fn(|i| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("orbital-feed-u-{i}")),
                size: std::mem::size_of::<Orbital3dUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let feed_bind_groups: [wgpu::BindGroup; MAX_FEEDS] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("orbital-feed-bg-{i}")),
                layout: &bgl_3d,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: feed_uniform_bufs[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                    },
                ],
            })
        });

        // 5. Build initial static station mesh
        let initial_palette = station
            .lock()
            .map(|s| s.palette)
            .unwrap_or(OrbitalPalette::UnatcoCobalt);
        let st_mesh = build_static_station(&palette_theme(initial_palette));
        let static_op_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-op-v"),
            contents: bytemuck::cast_slice(&st_mesh.opaque_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let static_op_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-op-i"),
            contents: bytemuck::cast_slice(&st_mesh.opaque_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let static_tr_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-tr-v"),
            contents: bytemuck::cast_slice(&st_mesh.trans_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let static_tr_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-tr-i"),
            contents: bytemuck::cast_slice(&st_mesh.trans_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // 6. Dynamic entity buffers
        let dyn_op_vcap = 8_192usize;
        let dyn_op_icap = 12_288usize;
        let dyn_tr_vcap = 2_048usize;
        let dyn_tr_icap = 3_072usize;
        let dyn_op_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-dyn-op-v"),
            size: (dyn_op_vcap * std::mem::size_of::<OrbitalVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dyn_op_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-dyn-op-i"),
            size: (dyn_op_icap * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dyn_tr_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-dyn-tr-v"),
            size: (dyn_tr_vcap * std::mem::size_of::<OrbitalVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dyn_tr_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-dyn-tr-i"),
            size: (dyn_tr_icap * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 7. Offscreen Scene Targets & CCTV Post-Process Pipeline
        let (scene_color_view, scene_depth_view) = create_scene_targets(device, format, 1280, 720);
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("orbital-post-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("orbital-post-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let post_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-post-uniform"),
            size: std::mem::size_of::<PostUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-post-bg"),
            layout: &post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: post_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&post_sampler),
                },
            ],
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orbital-post-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("orbital_post.wgsl").into()),
        });
        let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("orbital-post-pl"),
            bind_group_layouts: &[Some(&post_layout)],
            immediate_size: 0,
        });
        let post_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orbital-post-pipeline"),
            layout: Some(&post_pl),
            vertex: wgpu::VertexState {
                module: &post_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let hud_glyphs = GlyphPass::new(gpu, format, MAX_HUD_GLYPHS);

        Self {
            station,
            audio: None,
            format,
            width: 1280,
            height: 720,
            scale_factor: 1.0,
            time: 0.0,
            opaque_pipeline,
            translucent_pipeline,
            clear_pipeline,
            clear_uniform_buf,
            clear_bind_group,
            feed_uniform_bufs,
            feed_bind_groups,
            cached_palette: initial_palette,
            static_op_vbuf,
            static_op_ibuf,
            static_op_indices: st_mesh.opaque_indices.len() as u32,
            static_tr_vbuf,
            static_tr_ibuf,
            static_tr_indices: st_mesh.trans_indices.len() as u32,
            dyn_op_builder: MeshBuilder::with_capacity(4_096, 6_144),
            dyn_tr_builder: MeshBuilder::with_capacity(1_024, 1_536),
            dyn_op_vbuf,
            dyn_op_ibuf,
            dyn_op_vcap,
            dyn_op_icap,
            dyn_op_indices: 0,
            dyn_tr_vbuf,
            dyn_tr_ibuf,
            dyn_tr_vcap,
            dyn_tr_icap,
            dyn_tr_indices: 0,
            scene_color_view,
            scene_depth_view,
            post_pipeline,
            post_layout,
            post_sampler,
            post_uniform_buf,
            post_bind_group,
            hud_glyphs,
            last_pointer: None,
            is_dragging: false,
            drag_dist: 0.0,
        }
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn rebuild_static_if_needed(&mut self, device: &wgpu::Device, palette: OrbitalPalette) {
        if self.cached_palette == palette {
            return;
        }
        self.cached_palette = palette;
        let st_mesh = build_static_station(&palette_theme(palette));
        self.static_op_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-op-v"),
            contents: bytemuck::cast_slice(&st_mesh.opaque_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.static_op_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-op-i"),
            contents: bytemuck::cast_slice(&st_mesh.opaque_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.static_op_indices = st_mesh.opaque_indices.len() as u32;
        self.static_tr_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-tr-v"),
            contents: bytemuck::cast_slice(&st_mesh.trans_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.static_tr_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("orbital-static-tr-i"),
            contents: bytemuck::cast_slice(&st_mesh.trans_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.static_tr_indices = st_mesh.trans_indices.len() as u32;
    }

    fn push_hud_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let step_x = size * 0.64;
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            self.hud_glyphs.push(
                ch,
                [x + i as f32 * step_x, y],
                [size, size * 1.15],
                0.0,
                0.9,
                color,
                0.0,
            );
        }
    }

    fn build_lights(state: &OrbitalHavenState, t: f32) -> [PointLightGpu; 8] {
        let th = palette_theme(state.palette);
        let alert = state.event == StationEventKind::SecurityAlert;
        let docking = state.event == StationEventKind::DockingOps;
        let alert_pulse = 0.45 + 0.55 * (t * 5.2).sin().abs();

        let hangar_col = if alert {
            [1.0, 0.16, 0.12]
        } else {
            th.accent_secondary
        };
        let corr_col = if alert {
            [1.0, 0.18, 0.14]
        } else {
            th.accent_primary
        };
        let flood_boost = if docking { 1.55 } else { 1.15 };

        [
            // 0: Hangar Bay overhead sodium floodlight
            PointLightGpu {
                pos_radius: [0.0, 7.2, 26.0, 38.0],
                color_intensity: [
                    hangar_col[0],
                    hangar_col[1],
                    hangar_col[2],
                    1.35 * flood_boost,
                ],
            },
            // 1: Hangar Mouth cyan forcefield glow
            PointLightGpu {
                pos_radius: [0.0, 3.0, 41.0, 28.0],
                color_intensity: [
                    th.accent_primary[0],
                    th.accent_primary[1],
                    th.accent_primary[2],
                    1.15,
                ],
            },
            // 2: Port Landing Pad B floodlight
            PointLightGpu {
                pos_radius: [PAD_B_POS[0], 7.0, PAD_B_POS[2], 32.0],
                color_intensity: [
                    hangar_col[0],
                    hangar_col[1],
                    hangar_col[2],
                    1.25 * flood_boost,
                ],
            },
            // 3: Starboard Landing Pad C floodlight
            PointLightGpu {
                pos_radius: [PAD_C_POS[0], 7.0, PAD_C_POS[2], 32.0],
                color_intensity: [
                    hangar_col[0],
                    hangar_col[1],
                    hangar_col[2],
                    1.25 * flood_boost,
                ],
            },
            // 4: Ops Command Deck interior CRT & overhead glow
            PointLightGpu {
                pos_radius: [0.0, 17.5, 2.0, 24.0],
                color_intensity: [
                    corr_col[0],
                    corr_col[1],
                    corr_col[2],
                    if alert { 1.4 * alert_pulse } else { 1.25 },
                ],
            },
            // 5: Reactor Core pulsing plasma light
            PointLightGpu {
                pos_radius: [0.0, -10.2, 0.0, 26.0],
                color_intensity: [
                    corr_col[0],
                    corr_col[1],
                    corr_col[2],
                    1.55 + 0.25 * (t * 4.2).sin(),
                ],
            },
            // 6: Central Core Atrium overhead + elevator shaft light
            PointLightGpu {
                pos_radius: [0.0, 6.5, -2.0, 32.0],
                color_intensity: [
                    corr_col[0],
                    corr_col[1],
                    corr_col[2],
                    if alert { 1.35 * alert_pulse } else { 1.25 },
                ],
            },
            // 7: Aft Solar Engineering Bay & Axial Pipe Tunnel warm/cyan work light
            PointLightGpu {
                pos_radius: [0.0, 2.5, -31.0, 30.0],
                color_intensity: [hangar_col[0], hangar_col[1], hangar_col[2], 1.25],
            },
        ]
    }
}

impl Scene for OrbitalHavenScene {
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.scale_factor = scale_factor;

        let (color_view, depth_view) =
            create_scene_targets(&gpu.device, self.format, self.width, self.height);
        self.scene_color_view = color_view;
        self.scene_depth_view = depth_view;
        self.post_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-post-bg"),
            layout: &self.post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.post_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                },
            ],
        });
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_s = dt.as_secs_f32().clamp(0.0, 0.1);
        self.time += dt_s as f64;

        let mut switched_cam = false;
        let state_snapshot = if let Ok(mut st) = self.station.lock() {
            st.step(dt_s);

            if input.interactive {
                let w = self.width as f32;
                let h = self.height as f32;

                if input.left_held {
                    if let (Some(curr), Some(prev)) = (input.pointer, self.last_pointer) {
                        let dx = curr.x - prev.x;
                        let dy = curr.y - prev.y;
                        self.drag_dist += (dx * dx + dy * dy).sqrt();
                        if self.drag_dist > 5.0 {
                            self.is_dragging = true;
                            st.nudge_pan(dx * 0.003, -dy * 0.0025);
                        }
                    }
                } else {
                    self.is_dragging = false;
                    self.drag_dist = 0.0;
                }

                if input.action == PointerAction::LeftClick
                    && !self.is_dragging
                    && let Some(pt) = input.pointer
                {
                    let viewports = compute_viewports(
                        st.camera_rig.layout,
                        w,
                        h,
                        st.camera_rig.primary_feed,
                        st.camera_rig.pip_feeds,
                        st.camera_rig.grid_feeds,
                    );
                    // Check front-to-back (PiP mini-feeds first!)
                    let hit = viewports.iter().rev().find(|vp| {
                        pt.x >= vp.x && pt.x <= vp.x + vp.w && pt.y >= vp.y && pt.y <= vp.y + vp.h
                    });
                    if let Some(vp) = hit {
                        match vp.slot_tag {
                            0 => st.cycle_camera(),
                            1 => st.promote_pip_slot(0),
                            2 => st.promote_pip_slot(1),
                            10..=13 => st.cycle_grid_slot((vp.slot_tag - 10) as usize),
                            _ => st.cycle_camera(),
                        }
                        switched_cam = true;
                    }
                } else if input.action == PointerAction::RightClick {
                    st.cycle_layout();
                    switched_cam = true;
                }

                self.last_pointer = input.pointer;
            } else {
                self.last_pointer = None;
                self.is_dragging = false;
                self.drag_dist = 0.0;
            }

            st.clone()
        } else {
            return Activity::Idle;
        };

        if switched_cam && let Some(ref audio) = self.audio {
            audio.play_sfx(SoundEffect::CameraSwitch);
        }

        // Rebuild static station if palette changed
        self.rebuild_static_if_needed(&gpu.device, state_snapshot.palette);

        // Rebuild dynamic entities (ships, drones, bots, rotating ring, plasma)
        let th = palette_theme(state_snapshot.palette);
        let t = self.time as f32;
        build_dynamic_entities(
            &state_snapshot,
            &th,
            t,
            &mut self.dyn_op_builder,
            &mut self.dyn_tr_builder,
        );

        // Upload dynamic opaque buffers (grow if needed)
        if self.dyn_op_builder.verts.len() > self.dyn_op_vcap {
            self.dyn_op_vcap = self.dyn_op_builder.verts.len().next_power_of_two();
            self.dyn_op_vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("orbital-dyn-op-v"),
                size: (self.dyn_op_vcap * std::mem::size_of::<OrbitalVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.dyn_op_builder.indices.len() > self.dyn_op_icap {
            self.dyn_op_icap = self.dyn_op_builder.indices.len().next_power_of_two();
            self.dyn_op_ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("orbital-dyn-op-i"),
                size: (self.dyn_op_icap * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !self.dyn_op_builder.verts.is_empty() {
            gpu.queue.write_buffer(
                &self.dyn_op_vbuf,
                0,
                bytemuck::cast_slice(&self.dyn_op_builder.verts),
            );
            gpu.queue.write_buffer(
                &self.dyn_op_ibuf,
                0,
                bytemuck::cast_slice(&self.dyn_op_builder.indices),
            );
        }
        self.dyn_op_indices = self.dyn_op_builder.indices.len() as u32;

        // Upload dynamic translucent buffers
        if self.dyn_tr_builder.verts.len() > self.dyn_tr_vcap {
            self.dyn_tr_vcap = self.dyn_tr_builder.verts.len().next_power_of_two();
            self.dyn_tr_vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("orbital-dyn-tr-v"),
                size: (self.dyn_tr_vcap * std::mem::size_of::<OrbitalVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.dyn_tr_builder.indices.len() > self.dyn_tr_icap {
            self.dyn_tr_icap = self.dyn_tr_builder.indices.len().next_power_of_two();
            self.dyn_tr_ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("orbital-dyn-tr-i"),
                size: (self.dyn_tr_icap * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !self.dyn_tr_builder.verts.is_empty() {
            gpu.queue.write_buffer(
                &self.dyn_tr_vbuf,
                0,
                bytemuck::cast_slice(&self.dyn_tr_builder.verts),
            );
            gpu.queue.write_buffer(
                &self.dyn_tr_ibuf,
                0,
                bytemuck::cast_slice(&self.dyn_tr_builder.indices),
            );
        }
        self.dyn_tr_indices = self.dyn_tr_builder.indices.len() as u32;

        // Update clear quad uniform
        gpu.queue.write_buffer(
            &self.clear_uniform_buf,
            0,
            bytemuck::bytes_of(&ClearQuadUniforms {
                color: [
                    th.space_clear[0] as f32,
                    th.space_clear[1] as f32,
                    th.space_clear[2] as f32,
                    1.0,
                ],
            }),
        );

        // Compute viewports, upload per-feed 3D uniforms, and build Diegetic CCTV HUD
        let w = self.width as f32;
        let h = self.height as f32;
        let viewports = compute_viewports(
            state_snapshot.camera_rig.layout,
            w,
            h,
            state_snapshot.camera_rig.primary_feed,
            state_snapshot.camera_rig.pip_feeds,
            state_snapshot.camera_rig.grid_feeds,
        );

        let lights = Self::build_lights(&state_snapshot, t);
        let sun_power = if state_snapshot.event == StationEventKind::SolarEclipse {
            0.16
        } else {
            1.0
        };
        let sun_dir = [
            state_snapshot.orbit_angle.cos() * 0.72,
            0.55,
            state_snapshot.orbit_angle.sin() * 0.72,
        ];

        self.hud_glyphs.clear();
        let rec_blink = (t * 2.4).sin() > 0.0;
        let total_s = state_snapshot.total_age_secs as u64;
        let clock_str = format!(
            "T+{:02}:{:02}:{:02}",
            (total_s / 3600) % 100,
            (total_s / 60) % 60,
            total_s % 60
        );

        for (i, vp) in viewports.iter().take(MAX_FEEDS).enumerate() {
            let cam = resolve_camera(&state_snapshot, vp.feed, t, vp.slot_tag == 0);
            let aspect = (vp.w / vp.h.max(1.0)).clamp(0.3, 4.0);
            let view_proj = build_perspective_view_proj(
                cam.eye,
                cam.target,
                cam.up,
                cam.fov_deg,
                aspect,
                0.4,
                3200.0,
            );

            let fog_density = if cam.is_interior { 0.011 } else { 0.00018 };
            let u3d = Orbital3dUniforms {
                view_proj,
                eye_and_time: [cam.eye[0], cam.eye[1], cam.eye[2], t],
                sun_dir_and_power: [sun_dir[0], sun_dir[1], sun_dir[2], sun_power],
                ambient_and_fog: [th.ambient[0], th.ambient[1], th.ambient[2], fog_density],
                fog_color_and_pulse: [th.fog_color[0], th.fog_color[1], th.fog_color[2], 1.0],
                lights,
            };
            gpu.queue
                .write_buffer(&self.feed_uniform_bufs[i], 0, bytemuck::bytes_of(&u3d));

            // Diegetic CCTV HUD text per viewport
            if state_snapshot.hud_enabled {
                let is_mini = vp.slot_tag == 1 || vp.slot_tag == 2;
                let char_sz = if is_mini {
                    (vp.h * 0.072).clamp(10.0, 14.0)
                } else {
                    (vp.h * 0.025).clamp(12.0, 19.0)
                };
                let pad_x = (vp.w * 0.022).clamp(10.0, 26.0);
                let pad_y = (vp.h * 0.036).clamp(12.0, 28.0);

                let header = if is_mini {
                    format!("{} {}", vp.feed.code(), vp.feed.short_label())
                } else {
                    cam.title.clone()
                };
                let hdr_col = if cam.is_uplink {
                    th.hud_secondary
                } else {
                    th.hud_primary
                };
                self.push_hud_text(&header, vp.x + pad_x, vp.y + pad_y, char_sz, hdr_col);

                if !is_mini {
                    self.push_hud_text(
                        &cam.subtitle,
                        vp.x + pad_x,
                        vp.y + pad_y + char_sz * 1.4,
                        char_sz * 0.84,
                        [hdr_col[0], hdr_col[1], hdr_col[2], 0.72],
                    );

                    // Top-right REC indicator + mission clock
                    let rec_col = if rec_blink {
                        [1.0, 0.24, 0.20, 0.95]
                    } else {
                        [0.55, 0.16, 0.14, 0.55]
                    };
                    let right_x = vp.x + vp.w - pad_x - char_sz * 8.2;
                    self.push_hud_text("* REC", right_x, vp.y + pad_y, char_sz, rec_col);
                    self.push_hud_text(
                        &clock_str,
                        right_x - char_sz * 1.4,
                        vp.y + pad_y + char_sz * 1.4,
                        char_sz * 0.82,
                        th.hud_primary,
                    );

                    // Bottom-left station telemetry bar on primary/grid feeds
                    if vp.slot_tag == 0 || vp.slot_tag == 12 {
                        let footer = format!(
                            "HELIOS-09  |  {}  |  {} SHIPS  |  {}",
                            state_snapshot.event.name(),
                            state_snapshot.stats.active_ships,
                            state_snapshot.camera_rig.layout.short_name()
                        );
                        self.push_hud_text(
                            &footer,
                            vp.x + pad_x,
                            vp.y + vp.h - pad_y * 0.95,
                            char_sz * 0.84,
                            [
                                th.hud_primary[0],
                                th.hud_primary[1],
                                th.hud_primary[2],
                                0.76,
                            ],
                        );
                    }
                } else if rec_blink {
                    let rx = vp.x + vp.w - pad_x - char_sz * 2.4;
                    self.push_hud_text(
                        "REC",
                        rx,
                        vp.y + pad_y,
                        char_sz * 0.9,
                        [1.0, 0.25, 0.2, 0.92],
                    );
                }
            }
        }

        // Upload CCTV Post-Process Uniforms
        let layout_idx = match state_snapshot.camera_rig.layout {
            CameraLayout::Single => 0.0,
            CameraLayout::Pip => 1.0,
            CameraLayout::Grid2x2 => 2.0,
        };
        let norm_rect = |vp: Option<&cameras::ViewportRect>| -> [f32; 4] {
            vp.map(|v| [v.x / w, v.y / h, v.w / w, v.h / h])
                .unwrap_or([0.0; 4])
        };
        let post_u = PostUniforms {
            res_time_glitch: [
                w,
                h,
                t,
                (state_snapshot.camera_rig.static_glitch_timer / 0.18).clamp(0.0, 1.0),
            ],
            layout_and_accent: [
                layout_idx,
                th.accent_primary[0],
                th.accent_primary[1],
                th.accent_primary[2],
            ],
            pip0: norm_rect(viewports.get(1)),
            pip1: norm_rect(viewports.get(2)),
        };
        gpu.queue
            .write_buffer(&self.post_uniform_buf, 0, bytemuck::bytes_of(&post_u));

        self.hud_glyphs.upload(gpu, [w, h], t, 0.0);

        Activity::Animating
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let (layout, primary, pip_feeds, grid_feeds, palette) = self
            .station
            .lock()
            .map(|s| {
                (
                    s.camera_rig.layout,
                    s.camera_rig.primary_feed,
                    s.camera_rig.pip_feeds,
                    s.camera_rig.grid_feeds,
                    s.palette,
                )
            })
            .unwrap_or((
                CameraLayout::Single,
                rookery_core::orbital::CameraFeedId::DroneAlphaWide,
                [
                    rookery_core::orbital::CameraFeedId::PadAHangarInterior,
                    rookery_core::orbital::CameraFeedId::ShipNoseUplink,
                ],
                [
                    rookery_core::orbital::CameraFeedId::DroneAlphaWide,
                    rookery_core::orbital::CameraFeedId::ShipNoseUplink,
                    rookery_core::orbital::CameraFeedId::PadAHangarInterior,
                    rookery_core::orbital::CameraFeedId::IntOpsDeck,
                ],
                OrbitalPalette::UnatcoCobalt,
            ));

        let th = palette_theme(palette);
        let w = self.width as f32;
        let h = self.height as f32;
        let viewports = compute_viewports(layout, w, h, primary, pip_feeds, grid_feeds);

        // PASS 1..N: Render each active 3D camera feed into the offscreen scene texture
        for (i, vp) in viewports.iter().take(MAX_FEEDS).enumerate() {
            let vx = vp.x.max(0.0).min(w - 1.0);
            let vy = vp.y.max(0.0).min(h - 1.0);
            let vw = vp.w.max(1.0).min(w - vx);
            let vh = vp.h.max(1.0).min(h - vy);

            let color_load = if i == 0 {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: th.space_clear[0],
                    g: th.space_clear[1],
                    b: th.space_clear[2],
                    a: 1.0,
                })
            } else {
                wgpu::LoadOp::Load
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("orbital-3d-feed-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.scene_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
            pass.set_scissor_rect(vx as u32, vy as u32, vw as u32, vh as u32);

            // For secondary viewports (PiP mini-monitors or Grid quadrants 1..3), clear space backdrop cleanly
            if i > 0 {
                pass.set_pipeline(&self.clear_pipeline);
                pass.set_bind_group(0, &self.clear_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            // Opaque 3D pass: static station + dynamic spacecraft/drones/bots
            pass.set_pipeline(&self.opaque_pipeline);
            pass.set_bind_group(0, &self.feed_bind_groups[i], &[]);

            if self.static_op_indices > 0 {
                pass.set_vertex_buffer(0, self.static_op_vbuf.slice(..));
                pass.set_index_buffer(self.static_op_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.static_op_indices, 0, 0..1);
            }
            if self.dyn_op_indices > 0 {
                pass.set_vertex_buffer(0, self.dyn_op_vbuf.slice(..));
                pass.set_index_buffer(self.dyn_op_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.dyn_op_indices, 0, 0..1);
            }

            // Translucent 3D pass: containment forcefield, planetary ring, thruster plumes, holotable
            pass.set_pipeline(&self.translucent_pipeline);
            pass.set_bind_group(0, &self.feed_bind_groups[i], &[]);

            if self.static_tr_indices > 0 {
                pass.set_vertex_buffer(0, self.static_tr_vbuf.slice(..));
                pass.set_index_buffer(self.static_tr_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.static_tr_indices, 0, 0..1);
            }
            if self.dyn_tr_indices > 0 {
                pass.set_vertex_buffer(0, self.dyn_tr_vbuf.slice(..));
                pass.set_index_buffer(self.dyn_tr_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.dyn_tr_indices, 0, 0..1);
            }
        }

        // FINAL COMPOSITE PASS: CCTV Bayer dithering + bezels + static glitch + Diegetic HUD glyphs
        let mut final_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("orbital-cctv-composite-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        final_pass.set_pipeline(&self.post_pipeline);
        final_pass.set_bind_group(0, &self.post_bind_group, &[]);
        final_pass.draw(0..3, 0..1);
        drop(final_pass);

        self.hud_glyphs.render(view, encoder);
    }
}
