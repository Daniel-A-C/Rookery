//! Living pond fauna: skimming iridescent dragonflies, lily-pad frogs, and gentle pond turtles.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use super::fish::wrap_angle;
use super::flora::LilyPad;
use super::water::WaterRipple;
use super::{POND_H, POND_W};
use crate::world::WorldRng;

/// Behavioral state of a skimming dragonfly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragonflyMode {
    Cruising,
    Hovering,
    Perching,
}

/// An iridescent dragonfly darting and hovering above the water surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dragonfly {
    pub id: u32,
    pub pos: [f32; 2],
    pub target: [f32; 2],
    pub angle: f32,
    pub wing_phase: f32,
    pub mode: DragonflyMode,
    pub mode_timer: f32,
    /// `0` = cobalt blue, `1` = emerald jade, `2` = warm amber.
    pub color_idx: u8,
}

/// Behavioral state of a small emerald pond frog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrogMode {
    Resting,
    Hopping,
}

/// A small pond frog resting on lily pads and occasionally hopping with a splash.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PondFrog {
    pub id: u32,
    pub lily_idx: usize,
    pub target_lily_idx: usize,
    pub pos: [f32; 2],
    pub start_pos: [f32; 2],
    pub angle: f32,
    pub mode: FrogMode,
    pub hop_progress: f32,
    pub rest_timer: f32,
    pub breath_phase: f32,
}

/// A tranquil pond turtle paddling lazily near the shallows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PondTurtle {
    pub id: u32,
    pub pos: [f32; 2],
    pub angle: f32,
    pub target_angle: f32,
    pub speed: f32,
    pub stroke_phase: f32,
    pub turn_timer: f32,
    pub size: f32,
}

pub fn spawn_dragonflies(rng: &mut WorldRng) -> Vec<Dragonfly> {
    (0..3)
        .map(|i| {
            let pos = [
                rng.range(160.0, POND_W - 160.0),
                rng.range(140.0, POND_H - 140.0),
            ];
            let target = [
                rng.range(160.0, POND_W - 160.0),
                rng.range(140.0, POND_H - 140.0),
            ];
            Dragonfly {
                id: i as u32 + 1,
                pos,
                target,
                angle: rng.range(0.0, TAU),
                wing_phase: rng.range(0.0, TAU),
                mode: DragonflyMode::Cruising,
                mode_timer: rng.range(1.5, 4.0),
                color_idx: i as u8 % 3,
            }
        })
        .collect()
}

pub fn spawn_frogs(lilies: &[LilyPad], rng: &mut WorldRng) -> Vec<PondFrog> {
    if lilies.is_empty() {
        return Vec::new();
    }
    let indices = [0usize, 4usize.min(lilies.len() - 1)];
    indices
        .iter()
        .enumerate()
        .map(|(i, &lily_idx)| {
            let pos = lilies[lily_idx].pos;
            PondFrog {
                id: i as u32 + 1,
                lily_idx,
                target_lily_idx: lily_idx,
                pos,
                start_pos: pos,
                angle: rng.range(0.0, TAU),
                mode: FrogMode::Resting,
                hop_progress: 0.0,
                rest_timer: rng.range(6.0, 14.0),
                breath_phase: rng.range(0.0, TAU),
            }
        })
        .collect()
}

pub fn spawn_turtles(rng: &mut WorldRng) -> Vec<PondTurtle> {
    vec![PondTurtle {
        id: 1,
        pos: [
            rng.range(260.0, POND_W - 260.0),
            rng.range(200.0, POND_H - 200.0),
        ],
        angle: rng.range(0.0, TAU),
        target_angle: rng.range(0.0, TAU),
        speed: 12.0,
        stroke_phase: rng.range(0.0, TAU),
        turn_timer: rng.range(4.0, 8.0),
        size: 16.5,
    }]
}

/// Update dragonflies, pond frogs, and turtles. Returns any splash ripples created by hopping frogs.
pub fn update_fauna(
    dragonflies: &mut [Dragonfly],
    frogs: &mut [PondFrog],
    turtles: &mut [PondTurtle],
    lilies: &[LilyPad],
    dt: f32,
    rng: &mut WorldRng,
) -> Vec<WaterRipple> {
    let mut splashes = Vec::new();

    // 1. Skimming dragonflies
    for df in dragonflies.iter_mut() {
        let wing_speed = match df.mode {
            DragonflyMode::Perching => 2.5,
            DragonflyMode::Hovering => 34.0,
            DragonflyMode::Cruising => 42.0,
        };
        df.wing_phase = (df.wing_phase + dt * wing_speed) % (TAU * 100.0);
        df.mode_timer -= dt;

        match df.mode {
            DragonflyMode::Cruising => {
                let dx = df.target[0] - df.pos[0];
                let dy = df.target[1] - df.pos[1];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 14.0 || df.mode_timer <= 0.0 {
                    df.mode = if rng.f32() < 0.35 {
                        DragonflyMode::Perching
                    } else {
                        DragonflyMode::Hovering
                    };
                    df.mode_timer = rng.range(1.4, 3.8);
                } else {
                    let desired = dy.atan2(dx);
                    let diff = wrap_angle(desired - df.angle);
                    df.angle = wrap_angle(df.angle + diff * (dt * 6.5).min(1.0));
                    let spd = (dist * 1.4).clamp(48.0, 135.0);
                    df.pos[0] += df.angle.cos() * spd * dt;
                    df.pos[1] += df.angle.sin() * spd * dt;
                }
            }
            DragonflyMode::Hovering => {
                // Subtle hovering figure-eight vibration
                df.pos[0] += (df.wing_phase * 0.08).sin() * 4.5 * dt;
                df.pos[1] += (df.wing_phase * 0.06).cos() * 4.5 * dt;
                if df.mode_timer <= 0.0 {
                    df.mode = DragonflyMode::Cruising;
                    df.mode_timer = rng.range(2.2, 4.5);
                    if !lilies.is_empty() && rng.f32() < 0.6 {
                        let idx = (rng.f32() * lilies.len() as f32) as usize % lilies.len();
                        df.target = lilies[idx].pos;
                    } else {
                        df.target = [
                            rng.range(110.0, POND_W - 110.0),
                            rng.range(95.0, POND_H - 95.0),
                        ];
                    }
                }
            }
            DragonflyMode::Perching => {
                if df.mode_timer <= 0.0 {
                    df.mode = DragonflyMode::Cruising;
                    df.mode_timer = rng.range(2.0, 4.2);
                    df.target = [
                        rng.range(120.0, POND_W - 120.0),
                        rng.range(100.0, POND_H - 100.0),
                    ];
                }
            }
        }
    }

    // 2. Pond frogs resting on lily pads and occasionally hopping between them
    if !lilies.is_empty() {
        for frog in frogs.iter_mut() {
            frog.breath_phase = (frog.breath_phase + dt * 2.8) % TAU;
            match frog.mode {
                FrogMode::Resting => {
                    let lily = &lilies[frog.lily_idx.min(lilies.len() - 1)];
                    // Offset slightly off center if lily has a flower
                    let off = if lily.has_flower {
                        lily.radius * 0.42
                    } else {
                        lily.radius * 0.12
                    };
                    frog.pos = [
                        lily.pos[0] + (lily.rotation + 2.1).cos() * off,
                        lily.pos[1] + (lily.rotation + 2.1).sin() * off,
                    ];
                    frog.rest_timer -= dt;
                    if frog.rest_timer <= 0.0 && lilies.len() > 1 {
                        // Pick a nearby lily pad to hop toward
                        let mut best_idx = (frog.lily_idx + 1) % lilies.len();
                        let mut best_d = f32::MAX;
                        for (idx, l) in lilies.iter().enumerate() {
                            if idx == frog.lily_idx {
                                continue;
                            }
                            let dx = l.pos[0] - frog.pos[0];
                            let dy = l.pos[1] - frog.pos[1];
                            let d = (dx * dx + dy * dy).sqrt();
                            if d < best_d && d < 280.0 {
                                best_d = d;
                                best_idx = idx;
                            }
                        }
                        frog.target_lily_idx = best_idx;
                        frog.start_pos = frog.pos;
                        let dest = lilies[best_idx].pos;
                        frog.angle = (dest[1] - frog.pos[1]).atan2(dest[0] - frog.pos[0]);
                        frog.hop_progress = 0.0;
                        frog.mode = FrogMode::Hopping;
                        splashes.push(WaterRipple::new(frog.pos, 58.0, 0.55, 42.0, 2));
                    }
                }
                FrogMode::Hopping => {
                    frog.hop_progress += dt * 1.15;
                    let dest = lilies[frog.target_lily_idx.min(lilies.len() - 1)].pos;
                    let t = frog.hop_progress.clamp(0.0, 1.0);
                    let smooth = t * t * (3.0 - 2.0 * t);
                    frog.pos = [
                        frog.start_pos[0] + (dest[0] - frog.start_pos[0]) * smooth,
                        frog.start_pos[1] + (dest[1] - frog.start_pos[1]) * smooth,
                    ];
                    if frog.hop_progress >= 1.0 {
                        frog.lily_idx = frog.target_lily_idx;
                        frog.mode = FrogMode::Resting;
                        frog.rest_timer = rng.range(9.0, 20.0);
                        splashes.push(WaterRipple::new(dest, 52.0, 0.45, 38.0, 2));
                    }
                }
            }
        }
    }

    // 3. Tranquil pond turtles paddling slowly
    for turtle in turtles.iter_mut() {
        turtle.stroke_phase = (turtle.stroke_phase + dt * 2.2) % TAU;
        turtle.turn_timer -= dt;
        if turtle.turn_timer <= 0.0 {
            turtle.turn_timer = rng.range(4.5, 9.5);
            turtle.target_angle = wrap_angle(turtle.angle + rng.range(-1.1, 1.1));
            turtle.speed = rng.range(8.5, 16.0);
        }

        // Gentle margin steer
        let margin = 120.0;
        if turtle.pos[0] < margin {
            turtle.target_angle = 0.0;
        } else if turtle.pos[0] > POND_W - margin {
            turtle.target_angle = std::f32::consts::PI;
        }
        if turtle.pos[1] < margin {
            turtle.target_angle = std::f32::consts::FRAC_PI_2;
        } else if turtle.pos[1] > POND_H - margin {
            turtle.target_angle = -std::f32::consts::FRAC_PI_2;
        }

        let diff = wrap_angle(turtle.target_angle - turtle.angle);
        turtle.angle = wrap_angle(turtle.angle + diff * (dt * 0.9).min(1.0));
        // Subtle paddling surge
        let paddle_surge = 0.75 + 0.35 * turtle.stroke_phase.sin().max(0.0);
        turtle.pos[0] = (turtle.pos[0] + turtle.angle.cos() * turtle.speed * paddle_surge * dt)
            .clamp(40.0, POND_W - 40.0);
        turtle.pos[1] = (turtle.pos[1] + turtle.angle.sin() * turtle.speed * paddle_surge * dt)
            .clamp(40.0, POND_H - 40.0);
    }

    splashes
}
