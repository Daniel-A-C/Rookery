//! Placeholder Milestone 1 scene: aurora bands + ~200 drifting motes.
//!
//! Deliberately cheap. It exists to exercise the render loop, per-display budgets and the
//! governor's FPS ramp (it reports [`Activity::Animating`] while a periodic "gust" blows
//! or while the user is interacting). Replaced by the Living ASCII Rookery in Milestone 2.

use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use wgpu::util::DeviceExt;

use crate::fullscreen;
use crate::{Gpu, Scene, SceneInput};

const MOTE_COUNT: usize = 200;
/// Wrap shader time to keep f32 precision (aurora motion is slow; wrap is imperceptible).
const TIME_WRAP_SECS: f64 = 4096.0;
const GUST_DURATION: f32 = 4.0;
const GUST_INTERVAL: (f32, f32) = (40.0, 90.0);
const POINTER_RADIUS_PX: f32 = 180.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    gust: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct MoteInstance {
    center: [f32; 2],
    radius: f32,
    alpha: f32,
}

#[derive(Clone, Copy, Debug)]
struct Mote {
    pos: [f32; 2],
    vel: [f32; 2],
    radius: f32,
    phase: f32,
}

/// Tiny deterministic xorshift RNG so scenes are reproducible from a seed.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

pub struct DemoScene {
    rng: Rng,
    time: f64,
    width: f32,
    height: f32,
    scale: f32,
    motes: Vec<Mote>,
    instances: Vec<MoteInstance>,
    gust_timer: f32,
    gust_left: f32,
    gust_dir: [f32; 2],
    gust_env: f32,

    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bg_pipeline: wgpu::RenderPipeline,
    mote_pipeline: wgpu::RenderPipeline,
    mote_buffer: wgpu::Buffer,
}

impl DemoScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, seed: u64) -> Self {
        let device = &gpu.device;
        let source = include_str!("demo.wgsl");
        let layout = fullscreen::uniform_layout(device, "demo-uniforms");
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demo-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demo-uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let bg_pipeline =
            fullscreen::pipeline(device, "demo-aurora", source, format, Some(&layout), None);
        let mote_pipeline = mote_pipeline(device, source, format, &layout);
        let mote_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("demo-motes"),
            contents: bytemuck::cast_slice(&[MoteInstance::zeroed(); MOTE_COUNT]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut rng = Rng::new(seed);
        let gust_timer = rng.range(GUST_INTERVAL.0 * 0.25, GUST_INTERVAL.1 * 0.5);
        Self {
            rng,
            time: 0.0,
            width: 1.0,
            height: 1.0,
            scale: 1.0,
            motes: Vec::new(),
            instances: Vec::with_capacity(MOTE_COUNT),
            gust_timer,
            gust_left: 0.0,
            gust_dir: [1.0, 0.0],
            gust_env: 0.0,
            uniforms,
            bind_group,
            bg_pipeline,
            mote_pipeline,
            mote_buffer,
        }
    }

    fn spawn_motes(&mut self) {
        self.motes = (0..MOTE_COUNT)
            .map(|_| Mote {
                pos: [
                    self.rng.range(0.0, self.width),
                    self.rng.range(0.0, self.height),
                ],
                vel: [self.rng.range(-6.0, 6.0), self.rng.range(-3.0, 3.0)],
                radius: self.rng.range(1.0, 2.8),
                phase: self.rng.range(0.0, std::f32::consts::TAU),
            })
            .collect();
    }

    /// Advance the simulation; returns current activity. Separate from GPU upload for tests.
    fn step(&mut self, dt: f32, input: &SceneInput) -> Activity {
        self.time = (self.time + dt as f64) % TIME_WRAP_SECS;

        // Gust scheduling.
        if self.gust_left > 0.0 {
            self.gust_left -= dt;
        } else {
            self.gust_timer -= dt;
            if self.gust_timer <= 0.0 {
                self.gust_left = GUST_DURATION;
                self.gust_timer = self.rng.range(GUST_INTERVAL.0, GUST_INTERVAL.1);
                let angle = self.rng.range(-0.4, 0.4);
                let sign = if self.rng.f32() < 0.5 { -1.0 } else { 1.0 };
                self.gust_dir = [sign * angle.cos(), angle.sin()];
            }
        }
        let target_env = if self.gust_left > 0.0 { 1.0 } else { 0.0 };
        self.gust_env += (target_env - self.gust_env) * (dt * 1.5).min(1.0);

        let pointer = input.pointer.map(|p| [p.x, p.y]);
        let gust_force = 60.0 * self.gust_env * self.scale;
        let t = self.time as f32;
        let (w, h) = (self.width, self.height);

        for m in &mut self.motes {
            // Gentle curl-ish drift.
            let sway = [
                (m.pos[1] * 0.004 + t * 0.15 + m.phase).sin() * 4.0,
                (m.pos[0] * 0.003 + t * 0.11 + m.phase).cos() * 3.0,
            ];
            let mut acc = [
                sway[0] + self.gust_dir[0] * gust_force,
                sway[1] + self.gust_dir[1] * gust_force,
            ];
            if let Some(p) = pointer {
                let dx = m.pos[0] - p[0];
                let dy = m.pos[1] - p[1];
                let d2 = dx * dx + dy * dy;
                let r = POINTER_RADIUS_PX * self.scale;
                if d2 < r * r && d2 > 1e-3 {
                    let d = d2.sqrt();
                    let push = (1.0 - d / r) * 900.0 * self.scale;
                    acc[0] += dx / d * push;
                    acc[1] += dy / d * push;
                }
            }
            // Damped velocity integration.
            let damping = (-dt * 0.8).exp();
            m.vel[0] = m.vel[0] * damping + acc[0] * dt;
            m.vel[1] = m.vel[1] * damping + acc[1] * dt;
            m.pos[0] = (m.pos[0] + m.vel[0] * dt).rem_euclid(w);
            m.pos[1] = (m.pos[1] + m.vel[1] * dt).rem_euclid(h);
        }

        if self.gust_left > 0.0 || self.gust_env > 0.05 || pointer.is_some() {
            Activity::Animating
        } else {
            Activity::Idle
        }
    }

    fn build_instances(&mut self) {
        let t = self.time as f32;
        self.instances.clear();
        self.instances
            .extend(self.motes.iter().map(|m| MoteInstance {
                center: m.pos,
                radius: m.radius * self.scale * 2.0,
                alpha: 0.25 + 0.20 * (t * 0.6 + m.phase).sin(),
            }));
    }
}

fn mote_pipeline(
    device: &wgpu::Device,
    source: &str,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("demo-motes"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("demo-motes"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let additive = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("demo-motes"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_mote"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MoteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32, 2 => Float32],
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_mote"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: additive,
                    alpha: additive,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl Scene for DemoScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        let (old_w, old_h) = (self.width, self.height);
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale = scale_factor.max(0.5) as f32;
        if self.motes.is_empty() {
            self.spawn_motes();
        } else {
            let (sx, sy) = (self.width / old_w, self.height / old_h);
            for m in &mut self.motes {
                m.pos = [m.pos[0] * sx, m.pos[1] * sy];
            }
        }
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let activity = self.step(dt.as_secs_f32().min(0.25), input);
        self.build_instances();
        gpu.queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::bytes_of(&Uniforms {
                resolution: [self.width, self.height],
                time: self.time as f32,
                gust: self.gust_env,
            }),
        );
        gpu.queue
            .write_buffer(&self.mote_buffer, 0, bytemuck::cast_slice(&self.instances));
        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("demo"),
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
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_pipeline(&self.bg_pipeline);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&self.mote_pipeline);
        pass.set_vertex_buffer(0, self.mote_buffer.slice(..));
        pass.draw(0..6, 0..self.instances.len() as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let x = a.f32();
            assert_eq!(x, b.f32());
            assert!((0.0..1.0).contains(&x));
        }
    }
}
