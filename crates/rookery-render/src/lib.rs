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
pub mod audio;
pub mod capture;
pub mod catalog;
pub mod custom_shader;
pub mod demo;
mod fullscreen;
mod gpu;
pub mod koi_scene;
pub mod nebula_scene;
pub mod offscreen;
pub mod orbital_scene;
mod renderer;
mod scene;
mod vignette;

pub use ascii_scene::LivingRookeryScene;
pub use audio::{AudioEngine, SoundEffect};
pub use catalog::{AppCatalog, CatalogState};
pub use custom_shader::CustomShaderScene;
pub use gpu::Gpu;
pub use koi_scene::KoiPondScene;
pub use nebula_scene::CosmicNebulaScene;
pub use orbital_scene::OrbitalHavenScene;
pub use renderer::{DisplayStats, Renderer, SceneFactory, SurfaceSize};
pub use scene::{Pointer, PointerAction, RawPointer, Scene, SceneInput};
