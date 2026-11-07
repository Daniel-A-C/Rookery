//! Overlay drawn on top of the scene while Direct Interaction Mode is active (VISION.md §2.2).

use bytemuck::{Pod, Zeroable};

use crate::{Gpu, fullscreen};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    scale: f32,
    color: [f32; 4],
}

pub struct Vignette {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Vignette {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let layout = fullscreen::uniform_layout(device, "vignette");
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vignette"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vignette"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        // Premultiplied "over" blending.
        let pipeline = fullscreen::pipeline(
            device,
            "vignette",
            include_str!("vignette.wgsl"),
            format,
            Some(&layout),
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        Self {
            pipeline,
            uniforms,
            bind_group,
        }
    }

    pub fn render(
        &self,
        gpu: &Gpu,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        scale: f64,
        time: f32,
        color: [f32; 3],
    ) {
        gpu.queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::bytes_of(&Uniforms {
                resolution: [size.0 as f32, size.1 as f32],
                time,
                scale: scale as f32,
                color: [color[0], color[1], color[2], 1.0],
            }),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vignette"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
