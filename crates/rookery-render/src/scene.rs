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
