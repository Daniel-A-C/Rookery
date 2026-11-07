//! Generic runtime runner for external WGSL community shader apps.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use wgpu::util::DeviceExt;

use crate::fullscreen;
use crate::{Gpu, PointerAction, Scene, SceneInput};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CustomShaderUniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub interactive: f32,
    pub pointer: [f32; 2],
    pub action: f32,      // 0.0 = none, 1.0 = left click / button trigger
    pub left_held: f32,   // 1.0 if held, 0.0 otherwise
    pub params: [f32; 4], // Up to 4 declarative slider/select control values from rookery.toml
}

pub struct CustomShaderScene {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f32,
    color_phase: f32,
    pulse: f32,
    params: Arc<Mutex<[f32; 4]>>,
}

impl CustomShaderScene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, wgsl_source: &str) -> anyhow::Result<Self> {
        Self::new_with_params(
            gpu,
            format,
            wgsl_source,
            Arc::new(Mutex::new([1.0, 1.0, 0.0, 0.0])),
        )
    }

    pub fn new_with_params(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        wgsl_source: &str,
        params: Arc<Mutex<[f32; 4]>>,
    ) -> anyhow::Result<Self> {
        let full_source = if wgsl_source.contains("fn vs_main") {
            wgsl_source.to_string()
        } else {
            format!(
                r#"
struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {{
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(in_vertex_index) % 2) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}}

{wgsl_source}
"#
            )
        };

        let uniform_layout = fullscreen::uniform_layout(&gpu.device, "CustomShaderUniforms");
        let pipeline = fullscreen::pipeline(
            &gpu.device,
            "CustomShaderScene",
            &full_source,
            format,
            Some(&uniform_layout),
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );

        let initial_params = params.lock().map(|p| *p).unwrap_or([1.0, 1.0, 0.0, 0.0]);
        let initial_uniforms = CustomShaderUniforms {
            resolution: [1920.0, 1080.0],
            time: 0.0,
            interactive: 0.0,
            pointer: [0.0, 0.0],
            action: 0.0,
            left_held: 0.0,
            params: initial_params,
        };

        let uniform_buf = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CustomShaderUniformsBuf"),
                contents: bytemuck::bytes_of(&initial_uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CustomShaderBindGroup"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        Ok(Self {
            pipeline,
            bind_group,
            uniform_buf,
            width: 1920.0,
            height: 1080.0,
            scale_factor: 1.0,
            time: 0.0,
            color_phase: initial_params[3],
            pulse: 0.0,
            params,
        })
    }
}

impl Scene for CustomShaderScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);

        let mut cur_params = self
            .params
            .lock()
            .map(|p| *p)
            .unwrap_or([1.0, 1.0, 0.0, 0.0]);

        let speed_mult = cur_params[0].max(0.01);
        self.time += dt_secs * speed_mult;

        let color_rate = cur_params[2].max(0.0);
        self.color_phase = (self.color_phase + dt_secs * color_rate).rem_euclid(1000.0);
        cur_params[3] = self.color_phase;
        if let Ok(mut guard) = self.params.lock() {
            guard[3] = self.color_phase;
        }

        if input.action != PointerAction::None {
            self.pulse = 1.0;
        } else {
            self.pulse = (self.pulse - dt_secs * 1.2).max(0.0);
        }

        let (px, py) = input
            .pointer
            .map(|p| (p.x, p.y))
            .unwrap_or((-1000.0, -1000.0));

        let uniforms = CustomShaderUniforms {
            resolution: [self.width, self.height],
            time: self.time,
            interactive: if input.interactive { 1.0 } else { 0.0 },
            pointer: [px, py],
            action: self.pulse,
            left_held: if input.left_held { 1.0 } else { 0.0 },
            params: cur_params,
        };

        gpu.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        if input.interactive || input.left_held || self.pulse > 0.0 {
            Activity::Animating
        } else {
            Activity::Idle
        }
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("CustomShaderRenderPass"),
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

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
