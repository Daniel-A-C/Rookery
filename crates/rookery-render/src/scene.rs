//! The [`Scene`] trait implemented by everything that draws on a Rookery wallpaper.

use std::time::Duration;

use rookery_core::Activity;

use crate::Gpu;

/// Cursor position over a display in surface pixels (top-left origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    pub x: f32,
    pub y: f32,
}

/// Instantaneous pointer button action occurring on this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PointerAction {
    #[default]
    None,
    /// Left button was clicked (pressed down this frame)
    LeftClick,
    /// Right button was clicked (pressed down this frame)
    RightClick,
}

/// Raw cursor and button state before display-local coordinate mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawPointer {
    pub position: Option<(f64, f64)>,
    pub left_down: bool,
    pub right_down: bool,
}

/// Per-frame input passed to [`Scene::update`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SceneInput {
    /// True while Direct Interaction Mode is active.
    pub interactive: bool,
    /// Cursor position, only provided while interactive and over this display.
    pub pointer: Option<Pointer>,
    /// Instantaneous click action on this frame.
    pub action: PointerAction,
    /// True if left mouse button is held down over this display.
    pub left_held: bool,
    /// True if right mouse button is held down over this display.
    pub right_held: bool,
}

/// A renderable, steppable world. One instance is created per display.
pub trait Scene: Send {
    /// Called once after creation and whenever the surface size or scale factor changes.
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, scale_factor: f64);

    /// Advance the simulation by `dt` and report how much motion is happening.
    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity;

    /// Record draw commands targeting `view` (the surface texture for this frame).
    fn render(&mut self, gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder);
}

/// A minimal idle passthrough scene that simply clears the surface texture to an ambient dark color.
///
/// Used as the wgpu render thread backing when an external webview or canvas app is active.
pub struct BlankScene;

impl Scene for BlankScene {
    fn resize(&mut self, _gpu: &Gpu, _width: u32, _height: u32, _scale_factor: f64) {}

    fn update(&mut self, _gpu: &Gpu, _dt: Duration, _input: &SceneInput) -> Activity {
        Activity::Idle
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blank_passthrough_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.04,
                        g: 0.04,
                        b: 0.07,
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
    }
}
