//! Water ripples, drag swirls, and floating food pellets for Tranquil Koi Pond.

use serde::{Deserialize, Serialize};

/// A circular water ripple expanding across the pond surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaterRipple {
    pub center: [f32; 2],
    pub radius: f32,
    pub max_radius: f32,
    pub amplitude: f32,
    pub speed: f32,
    /// Number of concentric crests (1 for raindrop, 2-3 for pebble/gulp).
    pub ring_count: u8,
}

impl WaterRipple {
    pub fn new(
        center: [f32; 2],
        max_radius: f32,
        amplitude: f32,
        speed: f32,
        ring_count: u8,
    ) -> Self {
        Self {
            center,
            radius: 3.0,
            max_radius,
            amplitude,
            speed,
            ring_count: ring_count.clamp(1, 3),
        }
    }

    /// Normalized lifecycle progress in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        (self.radius / self.max_radius.max(1.0)).clamp(0.0, 1.0)
    }
}

/// A decaying surface water current/swirl created by cursor drag or fish tail flicks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaterSwirl {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub radius: f32,
    pub strength: f32,
}

/// A floating food pellet dropped for the koi fish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoodPellet {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    /// `0.0` (buoyant at surface) to `1.0` (dissolved/sunk).
    pub sink_depth: f32,
    pub life: f32,
    pub bob_phase: f32,
}
