//! Botanical flora and Zen garden props: mossy rocks, lily pads, lotus blooms,
//! drifting petals/leaves, floating tōrō lanterns, and the bamboo shishi-odoshi fountain.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use super::water::{WaterRipple, WaterSwirl};
use super::{POND_H, POND_W, PondTheme};
use crate::world::WorldRng;

/// A mossy boulder resting along the pond margin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PondRock {
    pub pos: [f32; 2],
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation: f32,
    pub moss_amount: f32,
    pub shade: f32,
}

/// A floating lily pad with radial venation and an optional blooming lotus flower.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LilyPad {
    pub id: u32,
    pub pos: [f32; 2],
    pub base_pos: [f32; 2],
    pub radius: f32,
    pub rotation: f32,
    pub bob_phase: f32,
    pub ripple_jostle: f32,
    pub has_flower: bool,
    pub flower_bloom: f32,
    pub target_bloom: f32,
    /// `0` = soft sakura pink, `1` = warm ivory cream, `2` = rose magenta, `3` = pale gold.
    pub flower_color_idx: u8,
}

/// A drifting surface petal, autumn maple leaf, or nocturnal firefly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriftPetal {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub rotation: f32,
    pub spin: f32,
    pub size: f32,
    pub color_variant: u8,
    pub phase: f32,
}

/// A floating wooden-framed Japanese paper lantern (*tōrō*).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingLantern {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub rotation: f32,
    pub flicker_phase: f32,
    pub size: f32,
}

/// State machine for the animated bamboo *shishi-odoshi* (deer-scarer) water spout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpoutPhase {
    Filling,
    Tipping,
    Returning,
}

/// A bamboo *shishi-odoshi* fountain mounted on the upper-right mossy bank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BambooFountain {
    /// Anchor position in pond world space.
    pub pivot: [f32; 2],
    /// Where the tipped water splashes into the pond.
    pub splash_pos: [f32; 2],
    pub phase: SpoutPhase,
    /// `0.0` (resting back) to `1.0` (fully tipped forward).
    pub tilt: f32,
    /// `0.0` (empty) to `1.0` (full, triggers tip).
    pub fill: f32,
    /// Cooldown timer for splash foam glow.
    pub splash_glow: f32,
}

impl BambooFountain {
    pub fn new() -> Self {
        Self {
            pivot: [918.0, 96.0],
            splash_pos: [854.0, 146.0],
            phase: SpoutPhase::Filling,
            tilt: 0.0,
            fill: 0.35,
            splash_glow: 0.0,
        }
    }

    /// Force the bamboo tube to tip immediately.
    pub fn trigger_tip(&mut self) {
        if self.phase == SpoutPhase::Filling {
            self.fill = 1.0;
            self.phase = SpoutPhase::Tipping;
        }
    }

    /// Advance the fountain by `dt`. Returns `(tipped_splash, clacked)`.
    pub fn update(&mut self, dt: f32) -> (bool, bool) {
        self.splash_glow = (self.splash_glow - dt * 1.2).max(0.0);
        let mut splashed = false;
        let mut clacked = false;

        match self.phase {
            SpoutPhase::Filling => {
                // Fills peacefully over ~13 seconds
                self.fill += dt * 0.077;
                // Slight gradual lean as water accumulates
                self.tilt = (self.fill * 0.16).min(0.16);
                if self.fill >= 1.0 {
                    self.fill = 1.0;
                    self.phase = SpoutPhase::Tipping;
                }
            }
            SpoutPhase::Tipping => {
                self.tilt += dt * 2.4;
                if self.tilt >= 1.0 {
                    self.tilt = 1.0;
                    self.fill = 0.0;
                    self.splash_glow = 1.0;
                    self.phase = SpoutPhase::Returning;
                    splashed = true;
                }
            }
            SpoutPhase::Returning => {
                self.tilt -= dt * 1.9;
                if self.tilt <= 0.0 {
                    self.tilt = 0.0;
                    self.phase = SpoutPhase::Filling;
                    clacked = true;
                }
            }
        }

        (splashed, clacked)
    }
}

impl Default for BambooFountain {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn curated clusters of mossy margin boulders.
pub fn spawn_pond_rocks(rng: &mut WorldRng) -> Vec<PondRock> {
    let anchors = [
        // Top-right corner around the bamboo shishi-odoshi
        ([940.0, 74.0], 54.0, 40.0, 0.85),
        ([895.0, 56.0], 42.0, 32.0, 0.72),
        ([962.0, 128.0], 46.0, 34.0, 0.65),
        // Bottom-left mossy bank
        ([58.0, 688.0], 62.0, 44.0, 0.80),
        ([116.0, 712.0], 40.0, 28.0, 0.58),
        ([34.0, 628.0], 36.0, 30.0, 0.70),
        // Top-left accent stones
        ([64.0, 72.0], 48.0, 35.0, 0.68),
        ([112.0, 46.0], 32.0, 24.0, 0.52),
        // Bottom-right quiet stone pair
        ([932.0, 682.0], 52.0, 38.0, 0.75),
        ([882.0, 710.0], 34.0, 26.0, 0.50),
    ];

    anchors
        .iter()
        .map(|&(pos, rx, ry, moss)| PondRock {
            pos: [pos[0] + rng.range(-8.0, 8.0), pos[1] + rng.range(-6.0, 6.0)],
            radius_x: rx * rng.range(0.9, 1.12),
            radius_y: ry * rng.range(0.9, 1.12),
            rotation: rng.range(0.0, TAU),
            moss_amount: moss,
            shade: rng.range(0.85, 1.15),
        })
        .collect()
}

/// Spawn natural clusters of floating lily pads with blooming lotus flowers.
pub fn spawn_lily_pads(rng: &mut WorldRng) -> Vec<LilyPad> {
    // Three organic botanical clusters leaving the center water open for koi viewing
    let layouts: [([f32; 2], f32, bool); 11] = [
        // Left-upper cluster
        ([168.0, 185.0], 46.0, true),
        ([232.0, 148.0], 34.0, false),
        ([138.0, 255.0], 38.0, false),
        ([218.0, 232.0], 29.0, true),
        // Bottom-left cluster
        ([210.0, 588.0], 48.0, true),
        ([280.0, 622.0], 35.0, false),
        ([156.0, 530.0], 31.0, false),
        // Right-lower cluster
        ([808.0, 540.0], 50.0, true),
        ([745.0, 596.0], 38.0, false),
        ([860.0, 476.0], 33.0, true),
        // Mid-top subtle accent
        ([560.0, 112.0], 36.0, false),
    ];

    layouts
        .iter()
        .enumerate()
        .map(|(i, &(pos, r, has_flower))| {
            let p = [
                pos[0] + rng.range(-12.0, 12.0),
                pos[1] + rng.range(-10.0, 10.0),
            ];
            let bloom = if has_flower {
                rng.range(0.65, 1.0)
            } else {
                0.0
            };
            LilyPad {
                id: i as u32 + 1,
                pos: p,
                base_pos: p,
                radius: r * rng.range(0.92, 1.08),
                rotation: rng.range(0.0, TAU),
                bob_phase: rng.range(0.0, TAU),
                ripple_jostle: 0.0,
                has_flower,
                flower_bloom: bloom,
                target_bloom: bloom,
                flower_color_idx: (i % 4) as u8,
            }
        })
        .collect()
}

/// Spawn drifting surface petals / leaves / nocturnal fireflies.
pub fn spawn_drift_petals(rng: &mut WorldRng, count: usize) -> Vec<DriftPetal> {
    (0..count)
        .map(|i| DriftPetal {
            pos: [
                rng.range(40.0, POND_W - 40.0),
                rng.range(40.0, POND_H - 40.0),
            ],
            vel: [rng.range(-4.0, 8.0), rng.range(-3.0, 5.0)],
            rotation: rng.range(0.0, TAU),
            spin: rng.range(-0.45, 0.45),
            size: rng.range(5.5, 10.5),
            color_variant: (i % 4) as u8,
            phase: rng.range(0.0, TAU),
        })
        .collect()
}

/// Spawn 4 floating Japanese paper lanterns (*tōrō*).
pub fn spawn_lanterns(rng: &mut WorldRng) -> Vec<FloatingLantern> {
    let spots = [
        [330.0, 290.0],
        [670.0, 260.0],
        [430.0, 535.0],
        [710.0, 440.0],
    ];
    spots
        .iter()
        .enumerate()
        .map(|(i, &pos)| FloatingLantern {
            id: i as u32 + 1,
            pos: [
                pos[0] + rng.range(-25.0, 25.0),
                pos[1] + rng.range(-20.0, 20.0),
            ],
            vel: [rng.range(-2.5, 2.5), rng.range(-2.0, 2.0)],
            rotation: rng.range(-0.25, 0.25),
            flicker_phase: rng.range(0.0, TAU),
            size: rng.range(18.0, 22.0),
        })
        .collect()
}

/// Update lily pads, drifting petals, and floating lanterns with ripple/swirl physics.
#[allow(clippy::too_many_arguments)]
pub fn update_flora_and_props(
    lilies: &mut [LilyPad],
    petals: &mut [DriftPetal],
    lanterns: &mut [FloatingLantern],
    ripples: &[WaterRipple],
    swirls: &[WaterSwirl],
    theme: PondTheme,
    time: f32,
    dt: f32,
) {
    // 1. Lily pads: gentle anchor spring + ripple jostling + lotus bloom smoothing
    for lily in lilies.iter_mut() {
        lily.bob_phase = (lily.bob_phase + dt * 1.1) % TAU;
        lily.ripple_jostle = (lily.ripple_jostle - dt * 1.6).max(0.0);

        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;
        for r in ripples {
            let dx = lily.pos[0] - r.center[0];
            let dy = lily.pos[1] - r.center[1];
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let ring_dist = (d - r.radius).abs();
            if ring_dist < 22.0 {
                let impact = (1.0 - ring_dist / 22.0) * r.amplitude;
                lily.ripple_jostle = (lily.ripple_jostle + impact * 0.45).min(1.0);
                push_x += (dx / d) * impact * 7.5;
                push_y += (dy / d) * impact * 7.5;
            }
        }
        for s in swirls {
            let dx = lily.pos[0] - s.pos[0];
            let dy = lily.pos[1] - s.pos[1];
            let d = (dx * dx + dy * dy).sqrt();
            if d < s.radius {
                let f = (1.0 - d / s.radius) * s.strength;
                push_x += s.vel[0] * f * 0.14;
                push_y += s.vel[1] * f * 0.14;
            }
        }

        // Elastic spring back toward base anchor
        let sway_x = (time * 0.45 + lily.bob_phase).sin() * 3.0;
        let sway_y = (time * 0.38 + lily.bob_phase * 1.3).cos() * 2.5;
        let goal_x = lily.base_pos[0] + sway_x;
        let goal_y = lily.base_pos[1] + sway_y;

        lily.pos[0] += ((goal_x - lily.pos[0]) * 1.6 + push_x) * dt;
        lily.pos[1] += ((goal_y - lily.pos[1]) * 1.6 + push_y) * dt;

        if lily.has_flower {
            lily.flower_bloom += (lily.target_bloom - lily.flower_bloom) * (dt * 0.8).min(1.0);
        }
    }

    // 2. Drifting petals / maple leaves / nocturnal fireflies
    let breeze = match theme {
        PondTheme::SakuraMorning => [6.5f32, 3.2f32],
        PondTheme::SunlitJade => [3.5, 1.8],
        PondTheme::AutumnMapleDusk => [8.0, 4.0],
        PondTheme::MoonlitLanternNight => [2.2, -1.6],
    };

    for p in petals.iter_mut() {
        p.phase = (p.phase + dt * 1.4) % TAU;
        let drift_x = breeze[0] + (time * 0.5 + p.phase).sin() * 4.5;
        let drift_y = breeze[1] + (time * 0.4 + p.phase * 1.3).cos() * 3.5;

        p.vel[0] += (drift_x - p.vel[0]) * (dt * 1.2).min(1.0);
        p.vel[1] += (drift_y - p.vel[1]) * (dt * 1.2).min(1.0);

        for r in ripples {
            let dx = p.pos[0] - r.center[0];
            let dy = p.pos[1] - r.center[1];
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let ring_dist = (d - r.radius).abs();
            if ring_dist < 26.0 {
                let push = (1.0 - ring_dist / 26.0) * r.amplitude * 46.0;
                p.vel[0] += (dx / d) * push * dt;
                p.vel[1] += (dy / d) * push * dt;
                p.spin += (push * 0.02 * dt).clamp(-0.6, 0.6);
            }
        }

        for s in swirls {
            let dx = p.pos[0] - s.pos[0];
            let dy = p.pos[1] - s.pos[1];
            let d = (dx * dx + dy * dy).sqrt();
            if d < s.radius {
                let f = (1.0 - d / s.radius) * s.strength;
                p.vel[0] += s.vel[0] * f * dt * 2.4;
                p.vel[1] += s.vel[1] * f * dt * 2.4;
            }
        }

        p.pos[0] += p.vel[0] * dt;
        p.pos[1] += p.vel[1] * dt;
        p.rotation = (p.rotation + p.spin * dt) % TAU;
        p.spin *= (1.0 - dt * 0.5).max(0.0);

        // Wrap smoothly around pond edges
        if p.pos[0] > POND_W + 20.0 {
            p.pos[0] = -18.0;
        } else if p.pos[0] < -20.0 {
            p.pos[0] = POND_W + 18.0;
        }
        if p.pos[1] > POND_H + 20.0 {
            p.pos[1] = -18.0;
        } else if p.pos[1] < -20.0 {
            p.pos[1] = POND_H + 18.0;
        }
    }

    // 3. Floating paper lanterns (tōrō)
    let lantern_positions: Vec<[f32; 2]> = lanterns.iter().map(|l| l.pos).collect();
    for (i, l) in lanterns.iter_mut().enumerate() {
        l.flicker_phase = (l.flicker_phase + dt * 3.1) % TAU;
        let gentle_x = (time * 0.22 + l.id as f32 * 1.7).sin() * 3.2;
        let gentle_y = (time * 0.19 + l.id as f32 * 2.3).cos() * 2.6;
        l.vel[0] += (gentle_x - l.vel[0]) * (dt * 0.8).min(1.0);
        l.vel[1] += (gentle_y - l.vel[1]) * (dt * 0.8).min(1.0);

        for r in ripples {
            let dx = l.pos[0] - r.center[0];
            let dy = l.pos[1] - r.center[1];
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let ring_dist = (d - r.radius).abs();
            if ring_dist < 28.0 {
                let push = (1.0 - ring_dist / 28.0) * r.amplitude * 24.0;
                l.vel[0] += (dx / d) * push * dt;
                l.vel[1] += (dy / d) * push * dt;
            }
        }
        for s in swirls {
            let dx = l.pos[0] - s.pos[0];
            let dy = l.pos[1] - s.pos[1];
            let d = (dx * dx + dy * dy).sqrt();
            if d < s.radius {
                let f = (1.0 - d / s.radius) * s.strength;
                l.vel[0] += s.vel[0] * f * dt * 1.4;
                l.vel[1] += s.vel[1] * f * dt * 1.4;
            }
        }
        // Soft repulsion between lanterns
        for (j, other) in lantern_positions.iter().enumerate() {
            if i == j {
                continue;
            }
            let dx = l.pos[0] - other[0];
            let dy = l.pos[1] - other[1];
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            if d < 110.0 {
                let push = (110.0 - d) / 110.0 * 14.0;
                l.vel[0] += (dx / d) * push * dt;
                l.vel[1] += (dy / d) * push * dt;
            }
        }

        // Keep lanterns inside comfortable central pond margins
        if l.pos[0] < 130.0 {
            l.vel[0] += 18.0 * dt;
        } else if l.pos[0] > POND_W - 130.0 {
            l.vel[0] -= 18.0 * dt;
        }
        if l.pos[1] < 120.0 {
            l.vel[1] += 18.0 * dt;
        } else if l.pos[1] > POND_H - 120.0 {
            l.vel[1] -= 18.0 * dt;
        }

        l.pos[0] += l.vel[0] * dt;
        l.pos[1] += l.vel[1] * dt;
        l.rotation = (time * 0.25 + l.id as f32).sin() * 0.18;
    }
}
