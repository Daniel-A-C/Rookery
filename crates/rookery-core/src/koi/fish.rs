//! Articulated Nishikigoi fish varieties, boid schooling, depth diving, and spine kinematics.

use std::f32::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use super::water::{FoodPellet, WaterRipple};
use super::{POND_H, POND_W};
use crate::world::WorldRng;

/// Number of articulated spine joints per koi fish.
pub const KOI_SPINE_SEGMENTS: usize = 12;

/// Authentic Japanese Nishikigoi varieties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KoiVariety {
    /// Snow-white body with organic vermilion-crimson (hi) dorsal markings.
    Kohaku,
    /// Ivory-white with crimson saddles and crisp ink-black (sumi) flecks.
    TaishoSanke,
    /// Deep lacquer black with wrapping crimson and cream-white bands.
    Showa,
    /// Luminous metallic gold with warm scale sheen.
    YamabukiOgon,
    /// Pure porcelain white with a solitary crimson crown circle on the forehead.
    Tancho,
    /// Pearlescent silver-white with iridescent fin shimmer.
    PlatinumOgon,
    /// Indigo-slate reticulated dorsal scales with warm coral-orange belly & cheeks.
    Asagi,
}

impl KoiVariety {
    pub const ALL: [KoiVariety; 7] = [
        KoiVariety::Kohaku,
        KoiVariety::TaishoSanke,
        KoiVariety::Showa,
        KoiVariety::YamabukiOgon,
        KoiVariety::Tancho,
        KoiVariety::PlatinumOgon,
        KoiVariety::Asagi,
    ];

    pub fn index(self) -> u32 {
        match self {
            KoiVariety::Kohaku => 0,
            KoiVariety::TaishoSanke => 1,
            KoiVariety::Showa => 2,
            KoiVariety::YamabukiOgon => 3,
            KoiVariety::Tancho => 4,
            KoiVariety::PlatinumOgon => 5,
            KoiVariety::Asagi => 6,
        }
    }

    pub fn from_rng(rng: &mut WorldRng) -> Self {
        let idx = (rng.f32() * Self::ALL.len() as f32) as usize % Self::ALL.len();
        Self::ALL[idx]
    }
}

/// An autonomous, articulated koi fish swimming gracefully in the pond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KoiFish {
    pub id: u32,
    pub variety: KoiVariety,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub angle: f32,
    pub target_angle: f32,
    /// Total head-to-tail length in pond units.
    pub length: f32,
    /// Body width multiplier (`0.88..=1.15`).
    pub girth: f32,
    pub speed: f32,
    pub target_speed: f32,
    /// Depth below surface: `0.0` = right at surface, `1.0` = near riverbed.
    pub swim_depth: f32,
    pub target_depth: f32,
    /// Unique shader seed so every koi has distinct organic markings.
    pub pattern_seed: f32,
    pub spine_phase: f32,
    pub fin_phase: f32,
    pub turn_timer: f32,
    pub depth_timer: f32,
    /// Remaining seconds of startled burst speed.
    pub startle_timer: f32,
    /// Remaining seconds of surface feeding gulp animation.
    pub gulp_timer: f32,
    /// Wake ripple cooldown timer.
    pub wake_timer: f32,
    /// 12 spine joint positions in world space (index 0 = snout, 11 = tail peduncle).
    pub spine: [[f32; 2]; KOI_SPINE_SEGMENTS],
}

impl KoiFish {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        variety: KoiVariety,
        pos: [f32; 2],
        angle: f32,
        length: f32,
        girth: f32,
        depth: f32,
        pattern_seed: f32,
    ) -> Self {
        let mut spine = [[0.0; 2]; KOI_SPINE_SEGMENTS];
        let seg_len = length / (KOI_SPINE_SEGMENTS - 1) as f32;
        for (i, joint) in spine.iter_mut().enumerate() {
            let offset = i as f32 * seg_len;
            *joint = [pos[0] - angle.cos() * offset, pos[1] - angle.sin() * offset];
        }

        let speed = 26.0;
        Self {
            id,
            variety,
            pos,
            vel: [angle.cos() * speed, angle.sin() * speed],
            angle,
            target_angle: angle,
            length,
            girth,
            speed,
            target_speed: speed,
            swim_depth: depth.clamp(0.08, 0.88),
            target_depth: depth.clamp(0.08, 0.88),
            pattern_seed,
            spine_phase: pattern_seed * TAU,
            fin_phase: pattern_seed * 4.3,
            turn_timer: 1.5 + (pattern_seed * 3.7) % 3.5,
            depth_timer: 4.0 + (pattern_seed * 5.1) % 6.0,
            startle_timer: 0.0,
            gulp_timer: 0.0,
            wake_timer: 0.5,
            spine,
        }
    }

    /// Gently startle this koi away from `epicenter`.
    pub fn startle_away_from(&mut self, epicenter: [f32; 2], intensity: f32) {
        let dx = self.pos[0] - epicenter[0];
        let dy = self.pos[1] - epicenter[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 175.0 && dist > 0.1 {
            let away = dy.atan2(dx);
            self.target_angle = away;
            self.target_speed = (68.0 + intensity * 26.0).min(105.0);
            self.target_depth = (self.swim_depth + 0.35).min(0.88);
            self.startle_timer = 1.4;
            self.turn_timer = 1.6;
        }
    }
}

/// Advance all koi fish with boid schooling, pellet attraction, smooth boundary curves,
/// depth diving, and 12-segment undulating spine kinematics.
/// Returns any newly generated surface wake ripples.
pub fn update_koi_school(
    fish: &mut [KoiFish],
    pellets: &[FoodPellet],
    dt: f32,
    rng: &mut WorldRng,
) -> Vec<WaterRipple> {
    let mut wakes = Vec::new();
    let count = fish.len();

    // Snapshot positions and headings for clean O(N^2) boid forces (N <= 18).
    let mut snapshots = [(0.0f32, 0.0f32, 0.0f32, 0.0f32); 24];
    let snap_len = count.min(24);
    for (i, f) in fish.iter().take(snap_len).enumerate() {
        snapshots[i] = (f.pos[0], f.pos[1], f.angle.cos(), f.angle.sin());
    }

    for (i, koi) in fish.iter_mut().enumerate() {
        koi.turn_timer -= dt;
        koi.depth_timer -= dt;
        koi.startle_timer = (koi.startle_timer - dt).max(0.0);
        koi.gulp_timer = (koi.gulp_timer - dt).max(0.0);
        koi.wake_timer = (koi.wake_timer - dt).max(0.0);

        let swim_rate = (koi.speed / 28.0).clamp(0.55, 2.8);
        koi.spine_phase = (koi.spine_phase + swim_rate * 4.2 * dt) % (TAU * 100.0);
        koi.fin_phase = (koi.fin_phase + swim_rate * 3.4 * dt) % (TAU * 100.0);

        // 1. Food pellet attraction (highest priority when not startled)
        let mut attracted = false;
        if koi.startle_timer <= 0.0 && !pellets.is_empty() {
            let mut closest_d = 320.0f32;
            let mut closest_p = None;
            for p in pellets {
                let dx = p.pos[0] - koi.pos[0];
                let dy = p.pos[1] - koi.pos[1];
                let d = (dx * dx + dy * dy).sqrt();
                if d < closest_d {
                    closest_d = d;
                    closest_p = Some(p.pos);
                }
            }
            if let Some(target) = closest_p {
                let angle_to = (target[1] - koi.pos[1]).atan2(target[0] - koi.pos[0]);
                koi.target_angle = angle_to;
                koi.target_speed = if closest_d < 65.0 { 46.0 } else { 62.0 };
                // Rise smoothly to the surface to gulp the pellet
                koi.target_depth = 0.06;
                attracted = true;
            }
        }

        // 2. Tranquil wandering & schooling boid forces
        if !attracted && koi.startle_timer <= 0.0 {
            if koi.turn_timer <= 0.0 {
                koi.turn_timer = rng.range(2.2, 5.8);
                let wander = rng.range(-0.95, 0.95);
                koi.target_angle = wrap_angle(koi.angle + wander);
                koi.target_speed = rng.range(20.0, 38.0);
            }
            if koi.depth_timer <= 0.0 {
                koi.depth_timer = rng.range(4.5, 10.5);
                koi.target_depth = rng.range(0.14, 0.82);
            }

            // Soft boid separation & gentle alignment
            let mut sep_x = 0.0f32;
            let mut sep_y = 0.0f32;
            let mut align_x = 0.0f32;
            let mut align_y = 0.0f32;
            let mut neighbors = 0.0f32;

            for &(ox, oy, odx, ody) in snapshots.iter().take(snap_len) {
                let dx = koi.pos[0] - ox;
                let dy = koi.pos[1] - oy;
                let d2 = dx * dx + dy * dy;
                if d2 < 1.0 {
                    continue;
                }
                let d = d2.sqrt();
                if d < 78.0 {
                    let push = (78.0 - d) / 78.0;
                    sep_x += (dx / d) * push;
                    sep_y += (dy / d) * push;
                } else if d < 175.0 {
                    align_x += odx;
                    align_y += ody;
                    neighbors += 1.0;
                }
            }

            if sep_x.abs() + sep_y.abs() > 0.05 {
                let sep_angle = sep_y.atan2(sep_x);
                let delta = wrap_angle(sep_angle - koi.target_angle);
                koi.target_angle = wrap_angle(koi.target_angle + delta * (dt * 1.4).min(0.4));
            } else if neighbors > 0.0 {
                let align_angle = (align_y / neighbors).atan2(align_x / neighbors);
                let delta = wrap_angle(align_angle - koi.target_angle);
                koi.target_angle = wrap_angle(koi.target_angle + delta * (dt * 0.35).min(0.15));
            }
        }

        // 3. Smooth elliptical pond-edge steering (natural curved turnbacks)
        let margin_x = 95.0;
        let margin_y = 85.0;
        let mut steer_x = 0.0f32;
        let mut steer_y = 0.0f32;
        if koi.pos[0] < margin_x {
            steer_x += (margin_x - koi.pos[0]) / margin_x;
        } else if koi.pos[0] > POND_W - margin_x {
            steer_x -= (koi.pos[0] - (POND_W - margin_x)) / margin_x;
        }
        if koi.pos[1] < margin_y {
            steer_y += (margin_y - koi.pos[1]) / margin_y;
        } else if koi.pos[1] > POND_H - margin_y {
            steer_y -= (koi.pos[1] - (POND_H - margin_y)) / margin_y;
        }
        if steer_x.abs() + steer_y.abs() > 0.02 {
            let inward = steer_y.atan2(steer_x);
            let urgency = (steer_x.abs() + steer_y.abs()).clamp(0.0, 1.5);
            let delta = wrap_angle(inward - koi.target_angle);
            koi.target_angle =
                wrap_angle(koi.target_angle + delta * (dt * 2.8 * urgency).min(0.85));
        }

        // 4. Smooth angular, speed, and depth interpolation
        let turn_rate = if attracted || koi.startle_timer > 0.0 {
            3.6
        } else {
            1.75
        };
        let diff = wrap_angle(koi.target_angle - koi.angle);
        koi.angle = wrap_angle(koi.angle + diff * (dt * turn_rate).min(1.0));

        let speed_rate = if koi.startle_timer > 0.0 { 4.2 } else { 1.6 };
        koi.speed += (koi.target_speed - koi.speed) * (dt * speed_rate).min(1.0);
        koi.swim_depth += (koi.target_depth - koi.swim_depth) * (dt * 1.15).min(1.0);
        koi.swim_depth = koi.swim_depth.clamp(0.04, 0.90);

        koi.vel = [koi.angle.cos() * koi.speed, koi.angle.sin() * koi.speed];
        koi.pos[0] = (koi.pos[0] + koi.vel[0] * dt).clamp(24.0, POND_W - 24.0);
        koi.pos[1] = (koi.pos[1] + koi.vel[1] * dt).clamp(24.0, POND_H - 24.0);

        // Emit subtle surface wake ripple when a koi glides near the surface
        if koi.swim_depth < 0.16 && koi.speed > 34.0 && koi.wake_timer <= 0.0 {
            koi.wake_timer = 0.45 + (i as f32 * 0.07) % 0.25;
            wakes.push(WaterRipple::new(
                koi.pos,
                42.0 + koi.speed * 0.4,
                0.28,
                34.0,
                1,
            ));
        }

        // 5. Propagate 12-segment articulated spine with natural sinusoidal tail undulation
        koi.spine[0] = koi.pos;
        let seg_len = koi.length / (KOI_SPINE_SEGMENTS - 1) as f32;
        let wave_amp = (0.35 + (koi.speed / 55.0).min(1.2) * 0.65) * seg_len * 0.28;

        for s in 1..KOI_SPINE_SEGMENTS {
            let prev = koi.spine[s - 1];
            let curr = koi.spine[s];
            let dx = curr[0] - prev[0];
            let dy = curr[1] - prev[1];
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = dx / dist;
            let ny = dy / dist;

            // Keep segment at exact anatomical spacing behind parent joint
            let mut jx = prev[0] + nx * seg_len;
            let mut jy = prev[1] + ny * seg_len;

            // Progressive lateral swimming wave (stiffer head, sweeping tail)
            let t = s as f32 / (KOI_SPINE_SEGMENTS - 1) as f32;
            let flex = t * t;
            let lateral = (koi.spine_phase - s as f32 * 0.58).sin() * wave_amp * flex;
            let perp = [-ny, nx];
            jx += perp[0] * lateral * (dt * 14.0).min(1.0);
            jy += perp[1] * lateral * (dt * 14.0).min(1.0);

            // Re-enforce exact segment length after lateral wave
            let rx = jx - prev[0];
            let ry = jy - prev[1];
            let rdist = (rx * rx + ry * ry).sqrt().max(0.001);
            koi.spine[s] = [
                prev[0] + (rx / rdist) * seg_len,
                prev[1] + (ry / rdist) * seg_len,
            ];
        }
    }

    wakes
}

#[inline]
pub fn wrap_angle(mut a: f32) -> f32 {
    while a < -PI {
        a += TAU;
    }
    while a > PI {
        a -= TAU;
    }
    a
}
