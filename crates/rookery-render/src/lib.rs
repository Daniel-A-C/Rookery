//! # rookery-render
//!
//! The wgpu side of the Rookery host:
//!
//! - [`Gpu`]: one shared instance/adapter/device/queue (low-power adapter preferred).
//! - [`Renderer`]: a single render thread driving N display surfaces, each with its own
//!   [`rookery_core::governor::FrameBudget`]. When every display is paused the thread
//!   blocks on its command channel and consumes no CPU.
//! - [`Scene`]: the trait Rookery Apps implement (Milestone 2's ASCII Rookery will be one).
//! - [`demo::DemoScene`]: a cheap placeholder scene (aurora bands + drifting motes) used to
//!   exercise the render loop and governor in Milestone 1.

pub mod ascii_scene;
pub mod atlas;
pub mod demo;
mod fullscreen;
mod gpu;
pub mod offscreen;
mod renderer;
mod scene;
mod vignette;

pub use ascii_scene::LivingRookeryScene;
pub use gpu::Gpu;
pub use renderer::{DisplayStats, Renderer, SceneFactory, SurfaceSize};
pub use scene::{Pointer, Scene, SceneInput};
