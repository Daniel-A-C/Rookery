//! Shared GPU context.

use std::sync::Arc;

use anyhow::{Context, Result};

/// Instance, adapter, device and queue shared by every surface and scene.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Create the GPU context, preferring the integrated/low-power adapter so a wallpaper
    /// never wakes a discrete GPU on dual-GPU laptops.
    pub fn new_blocking() -> Result<Arc<Self>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .context("no compatible GPU adapter found")?;
        let info = adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            info.name,
            info.backend,
            info.device_type
        );

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rookery-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            ..Default::default()
        }))
        .context("failed to create GPU device")?;

        device.on_uncaptured_error(Arc::new(|e| log::error!("wgpu error: {e}")));

        Ok(Arc::new(Self {
            instance,
            adapter,
            device,
            queue,
        }))
    }
}
