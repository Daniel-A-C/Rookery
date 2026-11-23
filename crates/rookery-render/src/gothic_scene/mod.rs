//! Gothic Metropolis Scene (`gothic-metropolis`).
//!
//! Renders an exaggerated 1990s-Batman-style (Anton Furst / Tim Burton) gothic city in a
//! simplified 3D isometric art style, floating in a stark black void and occupying roughly
//! three-fifths (~60%) of the viewport.
//!
//! Features 5 vertical layers of action:
//! 1. Deep canyon avenues with long-hood noir coupes, armored cruisers, and steam grates.
//! 2. Multi-level elevated gothic walkways/skybridges with strolling pedestrians.
//! 3. Elevated steel-and-stone train viaduct looping through the city and ViaductStation.
//! 4. Towering gothic setbacks, cathedral spires, flying buttresses, clockwork belfries,
//!    rising industrial steam plumes, and sweeping rooftop searchlights.
//! 5. High-altitude gothic patrol airships (dirigibles) with downward spotlights.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::gothic::{GothicMetropolisState, GothicPalette};
use wgpu::util::DeviceExt;

use crate::audio::AudioEngine;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const INITIAL_DYNAMIC_VERTS: usize = 16_384;
const INITIAL_DYNAMIC_INDICES: usize = 24_576;

mod buildings;
mod city_layout;
mod mesh_builder;
mod theme;
mod vehicles;

use city_layout::build_static_city_mesh;
use mesh_builder::{GothicVertex, MeshBuilder};
use rookery_core::gothic::CityBlueprint;
use theme::{lerp_theme, palette_theme};
use vehicles::push_vehicle_mesh;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GothicUniforms {
    view_proj: [[f32; 4]; 4],
    light_and_time: [f32; 4],
    bounce_and_beam: [f32; 4],
    palette_params: [f32; 4],
}

fn build_blended_static_mesh(
    bp: &CityBlueprint,
    palette: GothicPalette,
    include_next: bool,
) -> (Vec<GothicVertex>, Vec<u32>) {
    let theme_a = palette_theme(palette);
    let (mut verts, indices) = build_static_city_mesh(bp, &theme_a);
    if include_next {
        let theme_b = palette_theme(palette.next());
        let (verts_b, _) = build_static_city_mesh(bp, &theme_b);
        for (va, vb) in verts.iter_mut().zip(verts_b.iter()) {
            va.color_b = vb.color;
        }
    }
    (verts, indices)
}

/// Build the orthographic isometric 4x4 column-major view-projection matrix.
///
/// Configured so the expanded 4x4 city island occupies roughly **three-fifths (~60%)** of the
/// viewport centered in a stark black void.
fn build_isometric_view_proj(
    width: f32,
    height: f32,
    yaw: f32,
    pitch: f32,
    occupancy: f32,
    target: [f32; 3],
    zoom: f32,
) -> [[f32; 4]; 4] {
    let aspect = (width.max(1.0) / height.max(1.0)).clamp(0.4, 3.5);
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();

    // Expanded 4x4 city horizontal radius is ~12.2 world units; vertical span is ~17.5 world units.
    // Sizing `scale_y` so that the projected 4x4 city fills the vertical viewport with a tiny ~0.3cm buffer:
    let occ = (occupancy * zoom.clamp(0.8, 2.8)).clamp(0.40, 2.50);
    // Fit smoothly across both 16:9 widescreen and narrower aspect ratios
    let base_scale = occ / 13.0;
    let scale_y = base_scale * (aspect.min(1.65)).sqrt();
    let scale_x = scale_y / aspect;
    let scale_z = 1.0 / 64.0;

    // Row 0: vx = scale_x * (x * cy - z * sy)
    // Row 1: vy = scale_y * (y * cp - (x * sy + z * cy) * sp)
    // Row 2: vz = scale_z * (-(y * sp + (x * sy + z * cy) * cp)) + 0.5
    let r0 = [scale_x * cy, 0.0, -scale_x * sy, 0.0];
    let r1 = [-scale_y * sy * sp, scale_y * cp, -scale_y * cy * sp, 0.0];
    let r2 = [-scale_z * sy * cp, -scale_z * sp, -scale_z * cy * cp, 0.0];

    let tx = -(r0[0] * target[0] + r0[1] * target[1] + r0[2] * target[2]);
    let ty = -(r1[0] * target[0] + r1[1] * target[1] + r1[2] * target[2]);
    let tz = -(r2[0] * target[0] + r2[1] * target[1] + r2[2] * target[2]) + 0.5;

    // Column-major layout for WGSL `mat4x4<f32>`
    [
        [r0[0], r1[0], r2[0], 0.0],
        [r0[1], r1[1], r2[1], 0.0],
        [r0[2], r1[2], r2[2], 0.0],
        [tx, ty, tz, 1.0],
    ]
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gothic-depth-tex"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

/// GPU scene for *Gothic Metropolis* (`gothic-metropolis`).
pub struct GothicMetropolisScene {
    pub state: Arc<Mutex<GothicMetropolisState>>,
    pub audio: Option<AudioEngine>,
    width: u32,
    height: u32,
    scale_factor: f64,

    opaque_pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,

    static_vbuf: wgpu::Buffer,
    static_ibuf: wgpu::Buffer,
    static_index_count: u32,
    cached_palette: GothicPalette,
    cached_palette_blend_active: bool,

    opaque_dyn_vbuf: wgpu::Buffer,
    opaque_dyn_ibuf: wgpu::Buffer,
    opaque_dyn_vbuf_cap: usize,
    opaque_dyn_ibuf_cap: usize,
    opaque_dyn_index_count: u32,
    translucent_dyn_vbuf: wgpu::Buffer,
    translucent_dyn_ibuf: wgpu::Buffer,
    translucent_dyn_vbuf_cap: usize,
    translucent_dyn_ibuf_cap: usize,
    translucent_dyn_index_count: u32,

    opaque_verts_scratch: Vec<GothicVertex>,
    opaque_indices_scratch: Vec<u32>,
    trans_verts_scratch: Vec<GothicVertex>,
    trans_indices_scratch: Vec<u32>,

    last_pointer: Option<Pointer>,
    is_dragging: bool,
}

impl GothicMetropolisScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        state: Arc<Mutex<GothicMetropolisState>>,
    ) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gothic-isometric-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../gothic.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gothic-bind-layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gothic-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GothicVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x4,
                3 => Float32,
                4 => Float32x4,
            ],
        };

        let make_pipeline = |label: &str, depth_write: bool, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(vertex_layout.clone())],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let opaque_pipeline = make_pipeline("gothic-opaque-pipeline", true, None);
        let translucent_pipeline = make_pipeline(
            "gothic-translucent-pipeline",
            false,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-uniform-buf"),
            size: std::mem::size_of::<GothicUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gothic-bind-group"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let (init_palette, init_blend_active, (init_verts, init_indices)) = {
            let guard = state.lock().expect("gothic state lock");
            let need_next = guard.palette_cycle_enabled || guard.palette_blend > 0.0;
            (
                guard.palette,
                need_next,
                build_blended_static_mesh(&guard.blueprint, guard.palette, need_next),
            )
        };

        let static_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gothic-static-vbuf"),
            contents: bytemuck::cast_slice(&init_verts),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let static_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gothic-static-ibuf"),
            contents: bytemuck::cast_slice(&init_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let static_index_count = init_indices.len() as u32;

        let dyn_vbuf_bytes = (INITIAL_DYNAMIC_VERTS * std::mem::size_of::<GothicVertex>()) as u64;
        let dyn_ibuf_bytes = (INITIAL_DYNAMIC_INDICES * std::mem::size_of::<u32>()) as u64;
        let opaque_dyn_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-opaque-dyn-vbuf"),
            size: dyn_vbuf_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let opaque_dyn_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-opaque-dyn-ibuf"),
            size: dyn_ibuf_bytes,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let translucent_dyn_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-translucent-dyn-vbuf"),
            size: dyn_vbuf_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let translucent_dyn_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-translucent-dyn-ibuf"),
            size: dyn_ibuf_bytes,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_view = create_depth_texture(device, 1280, 720);

        let mut scene = Self {
            state,
            audio: None,
            width: 1280,
            height: 720,
            scale_factor: 1.0,
            opaque_pipeline,
            translucent_pipeline,
            uniform_buf,
            bind_group,
            depth_view,
            static_vbuf,
            static_ibuf,
            static_index_count,
            cached_palette: init_palette,
            cached_palette_blend_active: init_blend_active,
            opaque_dyn_vbuf,
            opaque_dyn_ibuf,
            opaque_dyn_vbuf_cap: INITIAL_DYNAMIC_VERTS,
            opaque_dyn_ibuf_cap: INITIAL_DYNAMIC_INDICES,
            opaque_dyn_index_count: 0,
            translucent_dyn_vbuf,
            translucent_dyn_ibuf,
            translucent_dyn_vbuf_cap: INITIAL_DYNAMIC_VERTS,
            translucent_dyn_ibuf_cap: INITIAL_DYNAMIC_INDICES,
            translucent_dyn_index_count: 0,
            opaque_verts_scratch: Vec::with_capacity(INITIAL_DYNAMIC_VERTS),
            opaque_indices_scratch: Vec::with_capacity(INITIAL_DYNAMIC_INDICES),
            trans_verts_scratch: Vec::with_capacity(4096),
            trans_indices_scratch: Vec::with_capacity(6144),
            last_pointer: None,
            is_dragging: false,
        };

        scene.rebuild_dynamic_buffers(gpu);
        scene
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn rebuild_dynamic_buffers(&mut self, gpu: &Gpu) {
        let Ok(guard) = self.state.lock() else {
            return;
        };
        let need_next = guard.palette_cycle_enabled || guard.palette_blend > 0.0;

        // Rebuild static architecture mesh if palette changed or blend target needs populating
        if guard.palette != self.cached_palette || (need_next && !self.cached_palette_blend_active)
        {
            let (new_static_verts, new_static_indices) =
                build_blended_static_mesh(&guard.blueprint, guard.palette, need_next);
            self.static_vbuf = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gothic-static-vbuf"),
                    contents: bytemuck::cast_slice(&new_static_verts),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            self.static_ibuf = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gothic-static-ibuf"),
                    contents: bytemuck::cast_slice(&new_static_indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
            self.static_index_count = new_static_indices.len() as u32;
            self.cached_palette = guard.palette;
            self.cached_palette_blend_active = need_next;
        }

        let theme_a = palette_theme(guard.palette);
        let theme = if guard.palette_blend > 0.0 {
            let theme_b = palette_theme(guard.palette.next());
            lerp_theme(&theme_a, &theme_b, guard.palette_blend)
        } else {
            theme_a
        };

        // Update uniform buffer
        let view_proj = build_isometric_view_proj(
            self.width as f32,
            self.height as f32,
            guard.camera_yaw,
            guard.camera_pitch,
            guard.screen_occupancy,
            guard.camera_target,
            guard.camera_zoom,
        );
        let uniforms = GothicUniforms {
            view_proj,
            light_and_time: [
                theme.key_light_dir[0],
                theme.key_light_dir[1],
                theme.key_light_dir[2],
                guard.time,
            ],
            bounce_and_beam: [
                theme.bounce_rgb[0],
                theme.bounce_rgb[1],
                theme.bounce_rgb[2],
                if guard.searchlights_enabled { 1.0 } else { 0.0 },
            ],
            palette_params: [guard.palette_blend.clamp(0.0, 1.0), 0.0, 0.0, 0.0],
        };
        gpu.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        // Build dynamic opaque & translucent geometry for all 5 vertical layers of action
        // Reusing scratch buffers avoids per-frame heap allocations.
        let mut opaque_mb = MeshBuilder::from_scratch(
            std::mem::take(&mut self.opaque_verts_scratch),
            std::mem::take(&mut self.opaque_indices_scratch),
        );
        let mut trans_mb = MeshBuilder::from_scratch(
            std::mem::take(&mut self.trans_verts_scratch),
            std::mem::take(&mut self.trans_indices_scratch),
        );

        // --- LAYER 1: Connected Street & Highway Vehicles + Functional Traffic Signals ---
        for sig in &guard.traffic_signals {
            let hx = sig.pos[0] + 0.40;
            let hz = sig.pos[1] + 0.54;
            let ns_color = if guard.green_wave_timer > 0.0 || (sig.ns_green && !sig.is_amber) {
                theme.copper_roof
            } else if sig.ns_green && sig.is_amber {
                theme.window_warm
            } else {
                theme.beacon_red
            };
            let ew_color = if guard.green_wave_timer > 0.0 || (!sig.ns_green && !sig.is_amber) {
                theme.copper_roof
            } else if !sig.ns_green && sig.is_amber {
                theme.window_warm
            } else {
                theme.beacon_red
            };
            opaque_mb.push_box(hx, 0.59, hz + 0.05, 0.055, 0.06, 0.025, ns_color, 1.35);
            opaque_mb.push_box(hx - 0.05, 0.59, hz, 0.025, 0.06, 0.055, ew_color, 1.35);
        }

        for veh in &guard.vehicles {
            if let Some((pos, car_fwd, slope, is_subterranean)) =
                guard.blueprint.sample_vehicle_pose_3d(veh)
            {
                if is_subterranean {
                    continue;
                }
                let opaque_start = opaque_mb.verts.len();
                let trans_start = trans_mb.verts.len();
                push_vehicle_mesh(
                    &mut opaque_mb,
                    &mut trans_mb,
                    veh,
                    pos,
                    car_fwd,
                    guard.time,
                    &theme,
                );
                if slope.abs() > 1e-4 {
                    let pitch = slope.atan();
                    let (sin_p, cos_p) = pitch.sin_cos();
                    for verts in [
                        &mut opaque_mb.verts[opaque_start..],
                        &mut trans_mb.verts[trans_start..],
                    ] {
                        for v in verts {
                            let dx = v.pos[0] - pos[0];
                            let dy = v.pos[1] - pos[1];
                            let dz = v.pos[2] - pos[2];
                            let along = dx * car_fwd[0] + dz * car_fwd[1];
                            let new_along = along * cos_p - dy * sin_p;
                            let new_dy = along * sin_p + dy * cos_p;
                            let d_along = new_along - along;
                            v.pos[0] += car_fwd[0] * d_along;
                            v.pos[2] += car_fwd[1] * d_along;
                            v.pos[1] = pos[1] + new_dy;

                            let n_along = v.normal[0] * car_fwd[0] + v.normal[2] * car_fwd[1];
                            let n_y = v.normal[1];
                            let new_n_along = n_along * cos_p - n_y * sin_p;
                            let new_n_y = n_along * sin_p + n_y * cos_p;
                            let dn = new_n_along - n_along;
                            v.normal[0] += car_fwd[0] * dn;
                            v.normal[2] += car_fwd[1] * dn;
                            v.normal[1] = new_n_y;
                        }
                    }
                }
            }
        }

        // --- LAYER 2: Elevated Walkway Pedestrians ---
        for ped in &guard.pedestrians {
            if let Some(span) = guard.blueprint.walkways.get(ped.walkway_idx) {
                let dx = span.end[0] - span.start[0];
                let dz = span.end[1] - span.start[1];
                let len = (dx * dx + dz * dz).sqrt().max(0.1);
                let right = [-dz / len, dx / len];
                let base = span.point_at(ped.progress);
                let px = base[0] + right[0] * ped.lateral_offset;
                let pz = base[2] + right[1] * ped.lateral_offset;
                let py = base[1];

                let coat_color = match ped.coat_variant {
                    0 => theme.accent_trim,
                    1 => theme.stone_secondary,
                    _ => theme.window_dim,
                };
                // Stylized gothic trenchcoat silhouette + hat/lantern
                opaque_mb.push_box(px, py, pz, 0.08, 0.16, 0.08, coat_color, 0.0);
                opaque_mb.push_box(px, py + 0.16, pz, 0.06, 0.06, 0.06, theme.window_warm, 0.65);
            }
        }

        // --- LAYER 3: Articulated Elevated Gothic Trains ---
        let car_spacing = 0.88f32;
        for train in &guard.trains {
            for car_idx in 0..train.car_count {
                let dist = train.track_dist - (car_idx as f32) * car_spacing;
                let (pos, fwd) = guard.blueprint.sample_viaduct(dist);
                let is_loco = car_idx == 0;
                let is_caboose = car_idx + 1 == train.car_count;
                let car_len = if is_loco { 0.80 } else { 0.72 };
                let car_w = 0.42;
                let car_h = 0.32;

                // Articulated accordion gangway coupler linking to the preceding car
                if car_idx > 0 {
                    let (gang_pos, gang_fwd) =
                        guard.blueprint.sample_viaduct(dist + car_spacing * 0.5);
                    opaque_mb.push_oriented_box(
                        [gang_pos[0], gang_pos[1] + 0.06, gang_pos[2]],
                        gang_fwd,
                        0.18,
                        0.24,
                        car_w * 0.62,
                        theme.plinth_dark,
                        0.0,
                    );
                }

                // Twin undercarriage bogie wheel trucks resting on the rails
                for truck_off in [-car_len * 0.28, car_len * 0.28] {
                    opaque_mb.push_oriented_box(
                        [
                            pos[0] + fwd[0] * truck_off,
                            pos[1] + 0.02,
                            pos[2] + fwd[1] * truck_off,
                        ],
                        fwd,
                        0.18,
                        0.05,
                        car_w * 0.78,
                        theme.plinth_dark,
                        0.0,
                    );
                }

                // Main lower steel body shell
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.06, pos[2]],
                    fwd,
                    car_len,
                    car_h,
                    car_w,
                    theme.stone_dark,
                    0.0,
                );
                // Brass/steel art-deco belt-rail trim along the lower flanks
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.11, pos[2]],
                    fwd,
                    car_len * 0.96,
                    0.035,
                    car_w + 0.025,
                    theme.accent_trim,
                    0.0,
                );

                if is_loco {
                    // Art-deco streamlined prow & lower cowcatcher pilot
                    let prow = [
                        pos[0] + fwd[0] * (car_len * 0.52),
                        pos[1] + 0.06,
                        pos[2] + fwd[1] * (car_len * 0.52),
                    ];
                    opaque_mb.push_oriented_box(
                        prow,
                        fwd,
                        0.16,
                        0.26,
                        car_w * 0.84,
                        theme.stone_secondary,
                        0.0,
                    );
                    opaque_mb.push_oriented_box(
                        [
                            pos[0] + fwd[0] * (car_len * 0.60),
                            pos[1] + 0.03,
                            pos[2] + fwd[1] * (car_len * 0.60),
                        ],
                        fwd,
                        0.10,
                        0.10,
                        car_w * 0.70,
                        theme.accent_trim,
                        0.0,
                    );

                    // Glowing front cab windshield & side cab windows
                    opaque_mb.push_oriented_box(
                        [
                            pos[0] + fwd[0] * (car_len * 0.34),
                            pos[1] + 0.21,
                            pos[2] + fwd[1] * (car_len * 0.34),
                        ],
                        fwd,
                        0.20,
                        0.11,
                        car_w + 0.03,
                        theme.window_warm,
                        0.95,
                    );

                    // Stepped locomotive roof + twin art-deco exhaust/steam stacks
                    opaque_mb.push_oriented_box(
                        [pos[0], pos[1] + 0.06 + car_h, pos[2]],
                        fwd,
                        car_len * 0.88,
                        0.06,
                        car_w * 0.82,
                        theme.copper_roof,
                        0.0,
                    );
                    for stack_off in [-0.14f32, 0.06f32] {
                        opaque_mb.push_oriented_box(
                            [
                                pos[0] + fwd[0] * stack_off,
                                pos[1] + 0.12 + car_h,
                                pos[2] + fwd[1] * stack_off,
                            ],
                            fwd,
                            0.09,
                            0.07,
                            0.10,
                            theme.steel_viaduct,
                            0.0,
                        );
                    }

                    // Lead locomotive brass headlamp bezel + cyclops lens + volumetric beam
                    let nose = [
                        pos[0] + fwd[0] * (car_len * 0.61),
                        pos[1] + 0.21,
                        pos[2] + fwd[1] * (car_len * 0.61),
                    ];
                    opaque_mb.push_oriented_box(
                        nose,
                        fwd,
                        0.07,
                        0.11,
                        0.14,
                        theme.window_warm,
                        1.0,
                    );
                    trans_mb.push_light_cone(
                        nose,
                        [fwd[0], -0.05, fwd[1]],
                        2.15,
                        0.48,
                        theme.searchlight,
                    );
                } else {
                    // Segmented glowing gothic passenger windows (3 bays per coach)
                    for win_idx in [-1.0f32, 0.0, 1.0] {
                        let wx = pos[0] + fwd[0] * (win_idx * car_len * 0.28);
                        let wz = pos[2] + fwd[1] * (win_idx * car_len * 0.28);
                        opaque_mb.push_oriented_box(
                            [wx, pos[1] + 0.17, wz],
                            fwd,
                            car_len * 0.20,
                            0.13,
                            car_w + 0.03,
                            theme.window_warm,
                            0.95,
                        );
                    }

                    // Arched copper car roof + illuminated clerestory strip
                    opaque_mb.push_oriented_box(
                        [pos[0], pos[1] + 0.06 + car_h, pos[2]],
                        fwd,
                        car_len * 0.94,
                        0.06,
                        car_w * 0.84,
                        theme.copper_roof,
                        0.0,
                    );
                    opaque_mb.push_oriented_box(
                        [pos[0], pos[1] + 0.12 + car_h, pos[2]],
                        fwd,
                        car_len * 0.72,
                        0.035,
                        car_w * 0.38,
                        theme.window_warm,
                        0.90,
                    );
                }

                if is_caboose {
                    // Rear ruby tail marker lamps + brass observation trim
                    let right = [-fwd[1], fwd[0]];
                    let tail_base = [
                        pos[0] - fwd[0] * (car_len * 0.51),
                        pos[1] + 0.16,
                        pos[2] - fwd[1] * (car_len * 0.51),
                    ];
                    opaque_mb.push_oriented_box(
                        [tail_base[0], pos[1] + 0.10, tail_base[2]],
                        fwd,
                        0.06,
                        0.05,
                        car_w * 0.82,
                        theme.accent_trim,
                        0.0,
                    );
                    for side in [-1.0f32, 1.0f32] {
                        opaque_mb.push_oriented_box(
                            [
                                tail_base[0] + right[0] * (car_w * 0.30) * side,
                                tail_base[1] + 0.02,
                                tail_base[2] + right[1] * (car_w * 0.30) * side,
                            ],
                            fwd,
                            0.05,
                            0.07,
                            0.07,
                            theme.beacon_red,
                            1.0,
                        );
                    }
                }
            }
        }

        // --- LAYER 4: Rising Steam Plumes & Rooftop Searchlights ---
        for plume in &guard.steam_plumes {
            let life_frac = (plume.age / plume.max_age).clamp(0.0, 1.0);
            let alpha = (1.0 - life_frac) * (life_frac * 4.0).min(1.0) * theme.steam[3];
            let radius = plume.base_radius * (1.0 + life_frac * 1.85);
            let steam_col = [theme.steam[0], theme.steam[1], theme.steam[2], alpha];
            trans_mb.push_box(
                plume.pos[0],
                plume.pos[1],
                plume.pos[2],
                radius,
                radius * 0.85,
                radius,
                steam_col,
                2.0,
            );
        }

        if guard.searchlights_enabled {
            for beam in &guard.searchlights {
                let dir = beam.direction_at(guard.time);
                // Searchlight housing & glowing lens
                opaque_mb.push_box(
                    beam.origin[0],
                    beam.origin[1] - 0.12,
                    beam.origin[2],
                    0.22,
                    0.16,
                    0.22,
                    theme.steel_viaduct,
                    0.0,
                );
                opaque_mb.push_box(
                    beam.origin[0],
                    beam.origin[1],
                    beam.origin[2],
                    0.16,
                    0.10,
                    0.16,
                    theme.window_warm,
                    1.0,
                );
                trans_mb.push_light_cone(beam.origin, dir, beam.length, 1.25, theme.searchlight);
            }
        }

        // --- LAYER 5: High-Altitude Gothic Patrol Airships (Dirigibles) ---
        for ship in &guard.airships {
            let (pos, fwd) = ship.pos_and_forward(guard.time);
            let right = [-fwd[1], fwd[0]];
            let hl = ship.hull_length;
            let hw = hl * 0.28;
            let hh = hl * 0.26;

            // Main dirigible envelope (3 stepped segments for tapered streamlined look)
            opaque_mb.push_oriented_box(pos, fwd, hl * 0.58, hh, hw, theme.stone_secondary, 0.0);
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] + hh * 0.12, pos[2]],
                fwd,
                hl * 0.86,
                hh * 0.74,
                hw * 0.74,
                theme.stone_primary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] + hh * 0.22, pos[2]],
                fwd,
                hl * 1.04,
                hh * 0.44,
                hw * 0.44,
                theme.accent_trim,
                0.0,
            );

            // Cruciform tail fins at the stern
            let stern = [
                pos[0] - fwd[0] * (hl * 0.40),
                pos[1] + hh * 0.15,
                pos[2] - fwd[1] * (hl * 0.40),
            ];
            opaque_mb.push_oriented_box(
                stern,
                fwd,
                hl * 0.22,
                hh * 1.15,
                0.06,
                theme.copper_roof,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [stern[0], stern[1] + hh * 0.45, stern[2]],
                fwd,
                hl * 0.22,
                0.06,
                hw * 1.25,
                theme.copper_roof,
                0.0,
            );

            // Underslung illuminated gondola cabin + blinking red beacon
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] - 0.14, pos[2]],
                fwd,
                hl * 0.34,
                0.14,
                hw * 0.42,
                theme.window_warm,
                1.0,
            );
            opaque_mb.push_oriented_box(
                [stern[0], stern[1] + hh * 1.15, stern[2]],
                fwd,
                0.09,
                0.09,
                0.09,
                theme.beacon_red,
                1.0,
            );

            // Downward sweeping airship searchlight scanning the gothic city below
            if guard.searchlights_enabled {
                let sweep = ship.searchlight_phase.sin() * 0.36;
                let spot_dir = [
                    fwd[0] * 0.32 + right[0] * sweep,
                    -0.88,
                    fwd[1] * 0.32 + right[1] * sweep,
                ];
                trans_mb.push_light_cone(
                    [
                        pos[0] + fwd[0] * (hl * 0.16),
                        pos[1] - 0.14,
                        pos[2] + fwd[1] * (hl * 0.16),
                    ],
                    spot_dir,
                    5.4,
                    1.10,
                    theme.searchlight,
                );
            }
        }

        // Release simulation lock before submitting GPU buffer writes
        drop(guard);

        let (opaque_verts, opaque_indices) = opaque_mb.into_parts();
        self.opaque_dyn_index_count = opaque_indices.len() as u32;
        if !opaque_indices.is_empty() {
            if opaque_verts.len() > self.opaque_dyn_vbuf_cap {
                self.opaque_dyn_vbuf_cap = opaque_verts.len().next_power_of_two();
                self.opaque_dyn_vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gothic-opaque-dyn-vbuf"),
                    size: (self.opaque_dyn_vbuf_cap * std::mem::size_of::<GothicVertex>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            if opaque_indices.len() > self.opaque_dyn_ibuf_cap {
                self.opaque_dyn_ibuf_cap = opaque_indices.len().next_power_of_two();
                self.opaque_dyn_ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gothic-opaque-dyn-ibuf"),
                    size: (self.opaque_dyn_ibuf_cap * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            gpu.queue.write_buffer(
                &self.opaque_dyn_vbuf,
                0,
                bytemuck::cast_slice(&opaque_verts),
            );
            gpu.queue.write_buffer(
                &self.opaque_dyn_ibuf,
                0,
                bytemuck::cast_slice(&opaque_indices),
            );
        }
        self.opaque_verts_scratch = opaque_verts;
        self.opaque_indices_scratch = opaque_indices;

        let (trans_verts, trans_indices) = trans_mb.into_parts();
        self.translucent_dyn_index_count = trans_indices.len() as u32;
        if !trans_indices.is_empty() {
            if trans_verts.len() > self.translucent_dyn_vbuf_cap {
                self.translucent_dyn_vbuf_cap = trans_verts.len().next_power_of_two();
                self.translucent_dyn_vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gothic-translucent-dyn-vbuf"),
                    size: (self.translucent_dyn_vbuf_cap * std::mem::size_of::<GothicVertex>())
                        as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            if trans_indices.len() > self.translucent_dyn_ibuf_cap {
                self.translucent_dyn_ibuf_cap = trans_indices.len().next_power_of_two();
                self.translucent_dyn_ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gothic-translucent-dyn-ibuf"),
                    size: (self.translucent_dyn_ibuf_cap * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            gpu.queue.write_buffer(
                &self.translucent_dyn_vbuf,
                0,
                bytemuck::cast_slice(&trans_verts),
            );
            gpu.queue.write_buffer(
                &self.translucent_dyn_ibuf,
                0,
                bytemuck::cast_slice(&trans_indices),
            );
        }
        self.trans_verts_scratch = trans_verts;
        self.trans_indices_scratch = trans_indices;
    }
}

impl Scene for GothicMetropolisScene {
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.scale_factor = scale_factor;
        self.depth_view = create_depth_texture(&gpu.device, self.width, self.height);
        self.rebuild_dynamic_buffers(gpu);
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let mut drag_delta = None;
        if input.action == PointerAction::LeftClick {
            self.is_dragging = true;
            self.last_pointer = input.pointer;
        }
        if input.left_held {
            if let (Some(curr), Some(prev)) = (input.pointer, self.last_pointer) {
                let dx = (curr.x - prev.x) / (self.width.max(1) as f32);
                let dy = (curr.y - prev.y) / (self.height.max(1) as f32);
                if dx.abs() > 0.0001 || dy.abs() > 0.0001 {
                    drag_delta = Some((-dx * PI * 1.4, dy * 1.1));
                }
            }
            self.last_pointer = input.pointer;
        } else {
            self.is_dragging = false;
            self.last_pointer = input.pointer;
        }

        let activity = if let Ok(mut guard) = self.state.lock() {
            if input.action == PointerAction::LeftClick {
                guard.trigger_steam_burst();
            } else if input.action == PointerAction::RightClick {
                guard.rotate_camera_quarter_turn();
            }
            if let Some((yaw_d, pitch_d)) = drag_delta {
                guard.orbit_camera(yaw_d, pitch_d);
            }
            guard.tick(dt.as_secs_f32())
        } else {
            Activity::Animating
        };

        self.rebuild_dynamic_buffers(gpu);
        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gothic-metropolis-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Pure stark black void background (#000000)
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
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

        // 1. Opaque 3D Architecture + Trains + Vehicles + Pedestrians + Airship Hulls
        rpass.set_pipeline(&self.opaque_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        if self.static_index_count > 0 {
            rpass.set_vertex_buffer(0, self.static_vbuf.slice(..));
            rpass.set_index_buffer(self.static_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.static_index_count, 0, 0..1);
        }
        if self.opaque_dyn_index_count > 0 {
            rpass.set_vertex_buffer(0, self.opaque_dyn_vbuf.slice(..));
            rpass.set_index_buffer(self.opaque_dyn_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.opaque_dyn_index_count, 0, 0..1);
        }

        // 2. Translucent Volumetric Searchlights, Headlight Cones & Rising Steam Plumes
        if self.translucent_dyn_index_count > 0 {
            rpass.set_pipeline(&self.translucent_pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.translucent_dyn_vbuf.slice(..));
            rpass.set_index_buffer(
                self.translucent_dyn_ibuf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rpass.draw_indexed(0..self.translucent_dyn_index_count, 0, 0..1);
        }
    }
}
