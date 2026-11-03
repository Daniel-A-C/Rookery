//! Single render thread driving every display surface with its own frame budget.
//!
//! Scheduling: each display tracks a `next_due` instant derived from its current
//! [`FrameBudget`]. The thread waits on its command channel with a timeout equal to the
//! earliest deadline. If every display is paused it blocks indefinitely on the channel,
//! consuming zero CPU until the governor sends a new budget.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use rookery_core::governor::FrameBudget;
use rookery_core::{Activity, DisplayId, Rect};
use serde::Serialize;

use crate::vignette::Vignette;
use crate::{Gpu, Pointer, Scene, SceneInput};

/// Creates one scene per display.
pub type SceneFactory = Box<dyn Fn(&Gpu, wgpu::TextureFormat) -> Box<dyn Scene> + Send>;

/// Returns the global cursor position in the platform's native display coordinate space
/// (the same space as `SurfaceSize::bounds_native`), or `None` if unavailable.
pub type PointerSource = Arc<dyn Fn() -> Option<(f64, f64)> + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    /// Display bounds in the platform's native coordinate space (for pointer mapping).
    pub bounds_native: Rect,
}

/// Live per-display statistics, published for the tray popover.
#[derive(Clone, Debug, Serialize)]
pub struct DisplayStats {
    pub budget: FrameBudget,
    pub actual_fps: f32,
    pub frame_cpu_ms: f32,
    pub activity: Activity,
    pub width: u32,
    pub height: u32,
}

enum Command {
    AddSurface {
        id: DisplayId,
        surface: wgpu::Surface<'static>,
        size: SurfaceSize,
    },
    RemoveSurface {
        id: DisplayId,
        ack: Sender<()>,
    },
    Resize {
        id: DisplayId,
        size: SurfaceSize,
    },
    SetBudget {
        id: DisplayId,
        budget: FrameBudget,
    },
    SetInteractive(bool),
    SwitchScene(SceneFactory),
    Shutdown,
}

struct Display {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: SurfaceSize,
    scene: Box<dyn Scene>,
    budget: FrameBudget,
    next_due: Option<Instant>,
    last_update: Instant,
    fps_ema: f32,
    cpu_ms_ema: f32,
    last_frame_at: Option<Instant>,
    activity: Activity,
}

/// Handle to the render thread. Dropping it shuts the thread down.
pub struct Renderer {
    tx: Sender<Command>,
    stats: Arc<Mutex<BTreeMap<DisplayId, DisplayStats>>>,
    gpu: Arc<Gpu>,
    thread: Option<JoinHandle<()>>,
}

impl Renderer {
    pub fn spawn(gpu: Arc<Gpu>, factory: SceneFactory, pointer: PointerSource) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let stats = Arc::new(Mutex::new(BTreeMap::new()));
        let thread = {
            let gpu = gpu.clone();
            let stats = stats.clone();
            std::thread::Builder::new()
                .name("rookery-render".into())
                .spawn(move || RenderThread::new(gpu, factory, pointer, rx, stats).run())
                .context("failed to spawn render thread")?
        };
        Ok(Self {
            tx,
            stats,
            gpu,
            thread: Some(thread),
        })
    }

    pub fn gpu(&self) -> &Arc<Gpu> {
        &self.gpu
    }

    /// Create a wgpu surface for a native window.
    ///
    /// Must be called on the thread that owns the window (the main thread on macOS).
    ///
    /// # Safety
    /// The window must outlive the surface: call [`Renderer::remove_surface`] (which waits for
    /// the render thread to drop the surface) before destroying the window.
    pub unsafe fn create_surface(
        &self,
        window: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<wgpu::Surface<'static>> {
        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(window.display_handle()?.as_raw()),
            raw_window_handle: window.window_handle()?.as_raw(),
        };
        // SAFETY: upheld by the caller (see doc comment).
        let surface = unsafe { self.gpu.instance.create_surface_unsafe(target)? };
        Ok(surface)
    }

    pub fn add_surface(&self, id: DisplayId, surface: wgpu::Surface<'static>, size: SurfaceSize) {
        let _ = self.tx.send(Command::AddSurface { id, surface, size });
    }

    /// Remove a surface and block until the render thread has dropped it.
    pub fn remove_surface(&self, id: DisplayId) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self
            .tx
            .send(Command::RemoveSurface { id, ack: ack_tx })
            .is_ok()
        {
            let _ = ack_rx.recv_timeout(Duration::from_secs(2));
        }
    }

    pub fn resize(&self, id: DisplayId, size: SurfaceSize) {
        let _ = self.tx.send(Command::Resize { id, size });
    }

    pub fn set_budget(&self, id: DisplayId, budget: FrameBudget) {
        let _ = self.tx.send(Command::SetBudget { id, budget });
    }

    pub fn set_interactive(&self, interactive: bool) {
        let _ = self.tx.send(Command::SetInteractive(interactive));
    }

    /// Snapshot of per-display stats.
    pub fn stats(&self) -> BTreeMap<DisplayId, DisplayStats> {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Hot-swaps the active scene factory across all active display surfaces.
    pub fn switch_scene(&self, factory: SceneFactory) {
        let _ = self.tx.send(Command::SwitchScene(factory));
    }

    pub fn shutdown(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct RenderThread {
    gpu: Arc<Gpu>,
    factory: SceneFactory,
    pointer: PointerSource,
    rx: Receiver<Command>,
    stats: Arc<Mutex<BTreeMap<DisplayId, DisplayStats>>>,
    displays: BTreeMap<DisplayId, Display>,
    interactive: bool,
    vignette: Option<(wgpu::TextureFormat, Vignette)>,
    started: Instant,
}

impl RenderThread {
    fn new(
        gpu: Arc<Gpu>,
        factory: SceneFactory,
        pointer: PointerSource,
        rx: Receiver<Command>,
        stats: Arc<Mutex<BTreeMap<DisplayId, DisplayStats>>>,
    ) -> Self {
        Self {
            gpu,
            factory,
            pointer,
            rx,
            stats,
            displays: BTreeMap::new(),
            interactive: false,
            vignette: None,
            started: Instant::now(),
        }
    }

    fn run(mut self) {
        log::info!("render thread started");
        loop {
            let next = self.displays.values().filter_map(|d| d.next_due).min();
            let cmd = match next {
                None => match self.rx.recv() {
                    Ok(c) => Some(c),
                    Err(_) => break,
                },
                Some(due) => {
                    let wait = due.saturating_duration_since(Instant::now());
                    match self.rx.recv_timeout(wait) {
                        Ok(c) => Some(c),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            };
            if let Some(cmd) = cmd {
                if !self.handle(cmd) {
                    break;
                }
                // Drain any further queued commands before rendering.
                while let Ok(cmd) = self.rx.try_recv() {
                    if !self.handle(cmd) {
                        self.displays.clear();
                        return;
                    }
                }
            }
            self.render_due();
            self.publish_stats();
        }
        self.displays.clear();
        log::info!("render thread stopped");
    }

    /// Returns false on shutdown.
    fn handle(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::AddSurface { id, surface, size } => {
                match self.add(id.clone(), surface, size) {
                    Ok(()) => log::info!("surface added for display {id}"),
                    Err(e) => log::error!("failed to add surface for {id}: {e:#}"),
                }
            }
            Command::RemoveSurface { id, ack } => {
                self.displays.remove(&id);
                if let Ok(mut s) = self.stats.lock() {
                    s.remove(&id);
                }
                let _ = ack.send(());
            }
            Command::Resize { id, size } => {
                if let Some(d) = self.displays.get_mut(&id) {
                    d.size = size;
                    d.config.width = size.width.max(1);
                    d.config.height = size.height.max(1);
                    d.surface.configure(&self.gpu.device, &d.config);
                    d.scene
                        .resize(&self.gpu, size.width, size.height, size.scale_factor);
                    if !d.budget.is_paused() {
                        d.next_due = Some(Instant::now());
                    }
                }
            }
            Command::SetBudget { id, budget } => {
                if let Some(d) = self.displays.get_mut(&id) {
                    Self::apply_budget(d, budget);
                }
            }
            Command::SetInteractive(on) => {
                self.interactive = on;
                for d in self.displays.values_mut() {
                    if !d.budget.is_paused() {
                        d.next_due = Some(Instant::now());
                    }
                }
            }
            Command::SwitchScene(factory) => {
                self.factory = factory;
                for d in self.displays.values_mut() {
                    let mut scene = (self.factory)(&self.gpu, d.config.format);
                    scene.resize(&self.gpu, d.size.width, d.size.height, d.size.scale_factor);
                    d.scene = scene;
                    if !d.budget.is_paused() {
                        d.next_due = Some(Instant::now());
                    }
                }
            }
            Command::Shutdown => return false,
        }
        true
    }

    fn apply_budget(d: &mut Display, budget: FrameBudget) {
        if d.budget == budget {
            return;
        }
        let was_paused = d.budget.is_paused();
        d.budget = budget;
        match budget {
            FrameBudget::Paused(_) => {
                d.next_due = None;
                d.fps_ema = 0.0;
                d.last_frame_at = None;
            }
            FrameBudget::Fps(_) => {
                let now = Instant::now();
                if was_paused {
                    // Simulation is frozen while paused (offline catch-up is Milestone 2).
                    d.last_update = now;
                }
                d.next_due = Some(now);
            }
        }
    }

    fn add(
        &mut self,
        id: DisplayId,
        surface: wgpu::Surface<'static>,
        size: SurfaceSize,
    ) -> Result<()> {
        let caps = surface.get_capabilities(&self.gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .context("surface reports no supported formats")?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes[0]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 1,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            ..surface
                .get_default_config(&self.gpu.adapter, size.width.max(1), size.height.max(1))
                .context("surface not supported by adapter")?
        };
        surface.configure(&self.gpu.device, &config);

        let mut scene = (self.factory)(&self.gpu, format);
        scene.resize(&self.gpu, size.width, size.height, size.scale_factor);
        if self.vignette.as_ref().is_none_or(|(f, _)| *f != format) {
            self.vignette = Some((format, Vignette::new(&self.gpu, format)));
        }
        let now = Instant::now();
        self.displays.insert(
            id,
            Display {
                surface,
                config,
                size,
                scene,
                // Render one frame immediately so the wallpaper never shows garbage; the
                // governor will send the real budget within a second.
                budget: FrameBudget::Fps(15),
                next_due: Some(now),
                last_update: now,
                fps_ema: 0.0,
                cpu_ms_ema: 0.0,
                last_frame_at: None,
                activity: Activity::Idle,
            },
        );
        Ok(())
    }

    fn render_due(&mut self) {
        let now = Instant::now();
        let global_pointer = if self.interactive {
            (self.pointer)()
        } else {
            None
        };
        let elapsed = self.started.elapsed().as_secs_f32() % 4096.0;
        for d in self.displays.values_mut() {
            let Some(due) = d.next_due else { continue };
            if due > now {
                continue;
            }
            let FrameBudget::Fps(fps) = d.budget else {
                d.next_due = None;
                continue;
            };
            let interval = Duration::from_secs_f64(1.0 / fps.max(1) as f64);
            let frame_start = Instant::now();

            let pointer = global_pointer.and_then(|(gx, gy)| {
                let b = d.size.bounds_native;
                let (fx, fy) = ((gx - b.x) / b.width, (gy - b.y) / b.height);
                ((0.0..1.0).contains(&fx) && (0.0..1.0).contains(&fy)).then_some(Pointer {
                    x: (fx * d.size.width as f64) as f32,
                    y: (fy * d.size.height as f64) as f32,
                })
            });
            let input = SceneInput {
                interactive: self.interactive,
                pointer,
            };
            let dt = frame_start.duration_since(d.last_update);
            d.last_update = frame_start;
            d.activity = d.scene.update(&self.gpu, dt, &input);

            let texture = match d.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) => Some(t),
                wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                    d.surface.configure(&self.gpu.device, &d.config);
                    Some(t)
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    d.surface.configure(&self.gpu.device, &d.config);
                    None
                }
                other => {
                    log::debug!("skipping frame: {other:?}");
                    None
                }
            };
            if let Some(frame) = texture {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    self.gpu
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("rookery-frame"),
                        });
                d.scene.render(&self.gpu, &view, &mut encoder);
                if self.interactive
                    && let Some((_, vignette)) = &self.vignette
                {
                    vignette.render(
                        &self.gpu,
                        &view,
                        &mut encoder,
                        (d.config.width, d.config.height),
                        d.size.scale_factor,
                        elapsed,
                    );
                }
                self.gpu.queue.submit(Some(encoder.finish()));
                self.gpu.queue.present(frame);
            }

            let cpu_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            d.cpu_ms_ema = ema(d.cpu_ms_ema, cpu_ms, 0.1);
            if let Some(prev) = d.last_frame_at {
                let inst = 1.0 / frame_start.duration_since(prev).as_secs_f32().max(1e-4);
                d.fps_ema = ema(d.fps_ema, inst, 0.1);
            }
            d.last_frame_at = Some(frame_start);

            // Advance deadline; if we fell behind by more than a frame, resync to now.
            let mut next = due + interval;
            if next < frame_start {
                next = frame_start + interval;
            }
            d.next_due = Some(next);
        }
    }

    fn publish_stats(&self) {
        if let Ok(mut s) = self.stats.lock() {
            for (id, d) in &self.displays {
                s.insert(
                    id.clone(),
                    DisplayStats {
                        budget: d.budget,
                        actual_fps: d.fps_ema,
                        frame_cpu_ms: d.cpu_ms_ema,
                        activity: d.activity,
                        width: d.config.width,
                        height: d.config.height,
                    },
                );
            }
        }
    }
}

fn ema(prev: f32, sample: f32, alpha: f32) -> f32 {
    if prev == 0.0 {
        sample
    } else {
        prev + alpha * (sample - prev)
    }
}
