//! # rookery-core
//!
//! Platform-agnostic, GPU-agnostic logic for the Rookery desktop host.
//!
//! Everything in this crate is deterministic and unit-testable on any CI runner:
//!
//! - [`governor`]: maps OS/user state to a per-display [`governor::FrameBudget`].
//! - [`interaction`]: the Passive ⇄ Interactive (Direct Interaction Mode) state machine.
//! - [`occlusion`]: rectangle-union coverage math used by the OS occlusion probes.
//! - [`settings`]: versioned, atomically persisted host settings.
//!
//! OS-specific code lives in `rookery-platform`; rendering lives in `rookery-render`.

pub mod geometry;
pub mod governor;
pub mod interaction;
pub mod occlusion;
pub mod settings;

pub use geometry::{DisplayId, Rect};

/// A scene's hint to the governor about how much motion it currently has.
///
/// Ambient scenes report [`Activity::Idle`] most of the time (e.g. a tree growing
/// imperceptibly) and [`Activity::Animating`] when something visually fast happens
/// (e.g. a bird flies across the screen), letting the governor ramp the frame rate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    #[default]
    Idle,
    Animating,
}
