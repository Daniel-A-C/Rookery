//! Offscreen rendering of a [`Scene`] to RGBA8 pixels.
//!
//! Used by the headless smoke test (and an early seed for Milestone 3's `rookery-bench`).

use std::time::Duration;

use anyhow::{Context, Result};

use crate::{Gpu, Scene, SceneInput};

pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Owns an offscreen render target and readback buffer.
pub struct Offscreen {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    readback: wgpu::Buffer,
    padded_row: u32,
}

impl Offscreen {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("offscreen-readback"),
            size: (padded_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            width,
            height,
            texture,
            readback,
            padded_row,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn create_view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Step `scene` by `dt` with `input`, and submit render commands to the offscreen texture.
    pub fn step_and_render(
        &self,
        gpu: &Gpu,
        scene: &mut dyn Scene,
        dt: Duration,
        input: &SceneInput,
    ) {
        scene.update(gpu, dt, input);
        let view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen-render"),
            });
        scene.render(gpu, &view, &mut encoder);
        gpu.queue.submit(Some(encoder.finish()));
    }

    /// Read back the current texture contents into a tightly packed RGBA8 buffer.
    pub fn readback_rgba(&self, gpu: &Gpu) -> Result<Vec<u8>> {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen-readback"),
            });
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(Some(encoder.finish()));

        let slice = self.readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("device poll failed")?;
        rx.recv()
            .context("map callback dropped")?
            .context("buffer map failed")?;

        let data = slice
            .get_mapped_range()
            .context("failed to map readback range")?;
        let row = (self.width * 4) as usize;
        let mut out = Vec::with_capacity(row * self.height as usize);
        for y in 0..self.height as usize {
            let start = y * self.padded_row as usize;
            out.extend_from_slice(&data[start..start + row]);
        }
        drop(data);
        self.readback.unmap();
        Ok(out)
    }

    /// Step `scene` `ticks` times at `dt`, render one frame, and return tightly packed RGBA8.
    pub fn run(
        &self,
        gpu: &Gpu,
        scene: &mut dyn Scene,
        ticks: u32,
        dt: Duration,
    ) -> Result<Vec<u8>> {
        let input = SceneInput::default();
        for _ in 0..ticks.saturating_sub(1) {
            scene.update(gpu, dt, &input);
        }
        self.step_and_render(gpu, scene, dt, &input);
        self.readback_rgba(gpu)
    }
}
