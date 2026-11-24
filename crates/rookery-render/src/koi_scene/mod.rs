//! Painterly Japanese Garden Koi Pond Scene (Zero ASCII, Custom wgpu Pipelines).
//!
//! Two-pass GPU renderer:
//! 1. Fullscreen procedural water & riverbed pass (`koi_water.wgsl`): Voronoi riverbed pebbles,
//!    moss patches, live UV-refracting water ripples, dual-octave caustics, canopy dappled light,
//!    and shimmering floating lantern reflections.
//! 2. Layered painterly 2D entity mesh pass (`koi_entities.wgsl`): depth-offset shadows,
//!    sculpted 12-segment Nishikigoi (7 varieties) with translucent flowing fins, mossy margin
//!    boulders, concentric ripple crests, veined lily pads, multi-petal lotuses, frogs, turtles,
//!    bamboo shishi-odoshi fountain, floating tōrō lanterns, seasonal drifting petals/fireflies,
//!    and skimming dragonflies.

mod mesh;
mod theme;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::koi::{KoiPondState, KoiSoundCue};

use crate::audio::{AudioEngine, SoundEffect};
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

use mesh::{KoiMeshBuilder, KoiVertex};
use theme::{lerp_palette, palette_for};

const INITIAL_VERTS: usize = 8192;
const INITIAL_INDICES: usize = 12288;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterUniforms {
    resolution_and_time: [f32; 4],
    water_deep: [f32; 4],
    water_mid: [f32; 4],
    water_shallow: [f32; 4],
    caustic_color: [f32; 4],
    pebble_tint: [f32; 4],
    pebble_moss: [f32; 4],
    sun_tint: [f32; 4],
    ripples: [[f32; 4]; 32],
    lanterns: [[f32; 4]; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EntityUniforms {
    resolution_and_time: [f32; 4],
    water_absorb: [f32; 4],
    sun_tint: [f32; 4],
}

pub struct KoiPondScene {
    pub pond: Arc<Mutex<KoiPondState>>,
    pub audio: Option<AudioEngine>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,

    water_pipeline: wgpu::RenderPipeline,
    water_uniforms_buf: wgpu::Buffer,
    water_bind_group: wgpu::BindGroup,

    entity_pipeline: wgpu::RenderPipeline,
    entity_uniforms_buf: wgpu::Buffer,
    entity_bind_group: wgpu::BindGroup,

    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    vbuf_cap: usize,
    ibuf_cap: usize,
    index_count: u32,

    verts_scratch: Vec<KoiVertex>,
    indices_scratch: Vec<u32>,

    last_pointer_world: Option<[f32; 2]>,
}

impl KoiPondScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, pond: Arc<Mutex<KoiPondState>>) -> Self {
        let device = &gpu.device;

        // 1. Fullscreen procedural water & riverbed pipeline
        let water_source = include_str!("koi_water.wgsl");
        let water_layout = fullscreen::uniform_layout(device, "koi-water-layout");
        let water_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("koi-water-uniforms"),
            size: std::mem::size_of::<WaterUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let water_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("koi-water-bind-group"),
            layout: &water_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: water_uniforms_buf.as_entire_binding(),
            }],
        });
        let water_pipeline = fullscreen::pipeline(
            device,
            "koi-water-pipeline",
            water_source,
            format,
            Some(&water_layout),
            None,
        );

        // 2. Painterly 2D entity mesh pipeline
        let entity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("koi-entities-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("koi_entities.wgsl").into()),
        });
        let entity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("koi-entities-layout"),
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
        let entity_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("koi-entities-pipeline-layout"),
                bind_group_layouts: &[Some(&entity_layout)],
                immediate_size: 0,
            });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<KoiVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2,
                1 => Float32x2,
                2 => Float32x4,
                3 => Float32x4,
                4 => Float32x4,
            ],
        };
        let entity_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("koi-entities-pipeline"),
            layout: Some(&entity_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &entity_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(vertex_layout)],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &entity_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let entity_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("koi-entities-uniforms"),
            size: std::mem::size_of::<EntityUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("koi-entities-bind-group"),
            layout: &entity_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: entity_uniforms_buf.as_entire_binding(),
            }],
        });

        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("koi-entity-vbuf"),
            size: (INITIAL_VERTS * std::mem::size_of::<KoiVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("koi-entity-ibuf"),
            size: (INITIAL_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut scene = Self {
            pond,
            audio: None,
            width: 1280.0,
            height: 720.0,
            scale_factor: 1.0,
            time: 0.0,
            water_pipeline,
            water_uniforms_buf,
            water_bind_group,
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
        };
        scene.upload_frame(gpu);
        scene
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn screen_to_world(&self, pt: Pointer) -> [f32; 2] {
        [
            (pt.x / self.width.max(1.0)) * 1000.0,
            (pt.y / self.height.max(1.0)) * 750.0,
        ]
    }

    fn upload_frame(&mut self, gpu: &Gpu) {
        let pond_arc = self.pond.clone();
        let Ok(guard) = pond_arc.lock() else {
            return;
        };

        let pal_prev = palette_for(guard.prev_theme);
        let pal_curr = palette_for(guard.theme);
        let pal = if guard.theme_blend < 0.999 {
            lerp_palette(&pal_prev, &pal_curr, guard.theme_blend)
        } else {
            pal_curr
        };

        // Pack up to 32 live water ripples into the water refraction uniform
        let mut ripples = [[0.0f32; 4]; 32];
        let r_count = guard.ripples.len().min(32);
        for (i, r) in guard.ripples.iter().take(r_count).enumerate() {
            let fade = (1.0 - r.progress()).powf(0.8);
            ripples[i] = [r.center[0], r.center[1], r.radius, r.amplitude * fade];
        }

        // Pack up to 6 floating lantern lights into the water reflection uniform
        let mut lanterns = [[0.0f32; 4]; 6];
        let l_count = guard.lanterns.len().min(6);
        for (i, l) in guard.lanterns.iter().take(l_count).enumerate() {
            let flicker = 0.88 + 0.12 * (guard.time * 4.2 + l.flicker_phase).sin();
            lanterns[i] = [l.pos[0], l.pos[1], l.size * 3.8, flicker];
        }

        let water_u = WaterUniforms {
            resolution_and_time: [
                self.width,
                self.height,
                self.time as f32,
                guard.rain_intensity,
            ],
            water_deep: [
                pal.water_deep[0],
                pal.water_deep[1],
                pal.water_deep[2],
                pal.vignette_strength,
            ],
            water_mid: [
                pal.water_mid[0],
                pal.water_mid[1],
                pal.water_mid[2],
                pal.dapple_strength,
            ],
            water_shallow: [
                pal.water_shallow[0],
                pal.water_shallow[1],
                pal.water_shallow[2],
                guard.lantern_glow,
            ],
            caustic_color: [
                pal.caustic_color[0],
                pal.caustic_color[1],
                pal.caustic_color[2],
                pal.ambient_mul,
            ],
            pebble_tint: [
                pal.pebble_tint[0],
                pal.pebble_tint[1],
                pal.pebble_tint[2],
                r_count as f32,
            ],
            pebble_moss: [
                pal.pebble_moss[0],
                pal.pebble_moss[1],
                pal.pebble_moss[2],
                l_count as f32,
            ],
            sun_tint: [pal.sun_tint[0], pal.sun_tint[1], pal.sun_tint[2], 0.0],
            ripples,
            lanterns,
        };
        gpu.queue
            .write_buffer(&self.water_uniforms_buf, 0, bytemuck::bytes_of(&water_u));

        let entity_u = EntityUniforms {
            resolution_and_time: [self.width, self.height, self.time as f32, pal.ambient_mul],
            water_absorb: [
                pal.water_absorb[0],
                pal.water_absorb[1],
                pal.water_absorb[2],
                guard.lantern_glow,
            ],
            sun_tint: [pal.sun_tint[0], pal.sun_tint[1], pal.sun_tint[2], 0.0],
        };
        gpu.queue
            .write_buffer(&self.entity_uniforms_buf, 0, bytemuck::bytes_of(&entity_u));

        // Build all layered entity meshes reusing scratch Vecs
        let mut mb = KoiMeshBuilder::from_scratch(
            std::mem::take(&mut self.verts_scratch),
            std::mem::take(&mut self.indices_scratch),
            self.width,
            self.height,
        );
        mb.build_scene(&guard, &pal);
        let (verts, indices) = mb.into_scratch();

        if verts.len() > self.vbuf_cap {
            self.vbuf_cap = verts.len().next_power_of_two();
            self.vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("koi-entity-vbuf"),
                size: (self.vbuf_cap * std::mem::size_of::<KoiVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() > self.ibuf_cap {
            self.ibuf_cap = indices.len().next_power_of_two();
            self.ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("koi-entity-ibuf"),
                size: (self.ibuf_cap * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if !verts.is_empty() && !indices.is_empty() {
            gpu.queue
                .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(&verts));
            gpu.queue
                .write_buffer(&self.ibuf, 0, bytemuck::cast_slice(&indices));
        }
        self.index_count = indices.len() as u32;
        self.verts_scratch = verts;
        self.indices_scratch = indices;
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
        let (activity, sfx_cues) = {
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
                            a.play_sfx(SoundEffect::FeedKoi);
                        }
                    }
                    PointerAction::RightClick => {
                        guard.add_ripple(pw, 1.35);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(SoundEffect::WaterRipple);
                        }
                    }
                    PointerAction::None => {
                        if input.left_held {
                            if let Some(prev) = self.last_pointer_world {
                                let vx = (pw[0] - prev[0]) / dt_secs.max(0.01);
                                let vy = (pw[1] - prev[1]) / dt_secs.max(0.01);
                                if vx * vx + vy * vy > 40.0 {
                                    guard.swirl_water(pw, [vx * 0.45, vy * 0.45]);
                                }
                            } else {
                                guard.add_ripple(pw, 0.45);
                            }
                        }
                    }
                }
            }

            let act = guard.update(dt_secs, input.interactive, pointer_world);
            let cues = guard.take_pending_sfx();
            (act, cues)
        };

        self.last_pointer_world = if input.interactive {
            pointer_world
        } else {
            None
        };

        if let Some(ref audio) = self.audio {
            for cue in sfx_cues {
                match cue {
                    KoiSoundCue::BambooClack => audio.play_sfx(SoundEffect::BambooClack),
                    KoiSoundCue::WaterRipple => audio.play_sfx(SoundEffect::WaterRipple),
                    KoiSoundCue::FeedGulp => audio.play_sfx(SoundEffect::WaterRipple),
                }
            }
        }

        self.upload_frame(gpu);
        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("koi-pond-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.08,
                        b: 0.11,
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

        // Pass 1: Procedural riverbed, refracted ripples, caustics & canopy dappling
        rpass.set_pipeline(&self.water_pipeline);
        rpass.set_bind_group(0, &self.water_bind_group, &[]);
        rpass.draw(0..3, 0..1);

        // Pass 2: Layered painterly entities (rocks, koi, ripples, lily pads, lotuses, fauna, props)
        if self.index_count > 0 {
            rpass.set_pipeline(&self.entity_pipeline);
            rpass.set_bind_group(0, &self.entity_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vbuf.slice(..));
            rpass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }
}
