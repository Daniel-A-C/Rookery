//! Tranquil Koi Pond Simulation State (Milestone 4 - Cozy Living Ecosystem).
//!
//! A tranquil water garden ecosystem featuring autonomous articulated koi fish boids,
//! floating lily pads with blooming lotus flowers, interactive water ripples,
//! and feeding micro-interactions with offline time-catchup.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::Activity;
use crate::world::WorldRng;

pub const KOI_POND_STATE_VERSION: u32 = 1;

/// Color variety of Japanese Nishikigoi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KoiVariety {
    /// White body with vibrant red (hi) patterns.
    Kohaku,
    /// Black body with red and white markings.
    Showa,
    /// Solid lustrous metallic gold.
    YamabukiOgon,
    /// Pure white body with a solitary red circle on the head.
    Tancho,
    /// Deep ink black and indigo ghost koi.
    Karasugoi,
}

/// An autonomous, articulated koi fish swimming in the pond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KoiFish {
    pub id: u32,
    pub variety: KoiVariety,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub angle: f32,
    pub target_angle: f32,
    pub length: f32,
    pub speed: f32,
    pub target_speed: f32,
    pub spine_phase: f32,
    pub fin_phase: f32,
    pub turn_timer: f32,
    /// Relative spine segment positions for smooth multi-joint undulation.
    pub spine: Vec<[f32; 2]>,
}

impl KoiFish {
    pub fn new(id: u32, variety: KoiVariety, pos: [f32; 2], angle: f32, length: f32) -> Self {
        let seg_count = 7;
        let mut spine = Vec::with_capacity(seg_count);
        for i in 0..seg_count {
            let offset = (i as f32) * (length / seg_count as f32);
            spine.push([pos[0] - angle.cos() * offset, pos[1] - angle.sin() * offset]);
        }

        Self {
            id,
            variety,
            pos,
            vel: [angle.cos() * 25.0, angle.sin() * 25.0],
            angle,
            target_angle: angle,
            length,
            speed: 25.0,
            target_speed: 25.0,
            spine_phase: 0.0,
            fin_phase: 0.0,
            turn_timer: 2.0,
            spine,
        }
    }
}

/// A floating lily pad with optional blooming lotus flower.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LilyPad {
    pub id: u32,
    pub pos: [f32; 2],
    pub radius: f32,
    pub rotation: f32,
    pub has_flower: bool,
    pub flower_bloom: f32,
    pub flower_color_idx: u8, // 0 = white/pink, 1 = magenta, 2 = pale golden
}

/// A food pellet dropped for the koi fish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoodPellet {
    pub pos: [f32; 2],
    pub sink_depth: f32, // 0.0 (surface) to 1.0 (bottom)
    pub life: f32,
}

/// A circular water ripple expanding across the pond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaterRipple {
    pub center: [f32; 2],
    pub radius: f32,
    pub max_radius: f32,
    pub amplitude: f32,
    pub speed: f32,
}

/// Statistics for the tranquil koi pond.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KoiPondStats {
    pub pond_age_secs: f64,
    pub total_fish: u32,
    pub total_lilies: u32,
    pub pellets_eaten: u32,
    pub ripples_created: u32,
}

/// Complete deterministic simulation state for Tranquil Koi Pond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KoiPondState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub age_secs: f64,
    pub fish: Vec<KoiFish>,
    pub lilies: Vec<LilyPad>,
    pub pellets: Vec<FoodPellet>,
    pub ripples: Vec<WaterRipple>,
    pub stats: KoiPondStats,
}

impl KoiPondState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut fish = Vec::new();

        let varieties = [
            KoiVariety::Kohaku,
            KoiVariety::YamabukiOgon,
            KoiVariety::Showa,
            KoiVariety::Tancho,
            KoiVariety::Kohaku,
            KoiVariety::YamabukiOgon,
            KoiVariety::Karasugoi,
            KoiVariety::Showa,
        ];

        for (i, &var) in varieties.iter().enumerate() {
            let x = rng.range(150.0, 850.0);
            let y = rng.range(100.0, 650.0);
            let angle = rng.range(0.0, TAU);
            let len = rng.range(42.0, 65.0);
            fish.push(KoiFish::new(i as u32 + 1, var, [x, y], angle, len));
        }

        let mut lilies = Vec::new();
        for i in 0..9 {
            let x = rng.range(120.0, 880.0);
            let y = rng.range(120.0, 620.0);
            let r = rng.range(34.0, 60.0);
            let rot = rng.range(0.0, TAU);
            let has_flower = i % 2 == 0;
            lilies.push(LilyPad {
                id: i as u32 + 1,
                pos: [x, y],
                radius: r,
                rotation: rot,
                has_flower,
                flower_bloom: if has_flower { rng.range(0.6, 1.0) } else { 0.0 },
                flower_color_idx: (i % 3) as u8,
            });
        }

        let total_lilies = lilies.len() as u32;
        let total_fish = fish.len() as u32;

        Self {
            version: KOI_POND_STATE_VERSION,
            seed,
            rng,
            age_secs: 0.0,
            fish,
            lilies,
            pellets: Vec::new(),
            ripples: Vec::new(),
            stats: KoiPondStats {
                pond_age_secs: 0.0,
                total_fish,
                total_lilies,
                pellets_eaten: 0,
                ripples_created: 0,
            },
        }
    }

    /// User interaction: drop fish food pellets into the pond.
    pub fn feed_koi(&mut self, pos: [f32; 2]) {
        for _ in 0..4 {
            let ox = self.rng.range(-18.0, 18.0);
            let oy = self.rng.range(-18.0, 18.0);
            self.pellets.push(FoodPellet {
                pos: [pos[0] + ox, pos[1] + oy],
                sink_depth: 0.0,
                life: 0.0,
            });
        }
        self.add_ripple(pos, 0.7);
    }

    /// Trigger a gentle water ripple at the given coordinate.
    pub fn add_ripple(&mut self, center: [f32; 2], amplitude: f32) {
        self.ripples.push(WaterRipple {
            center,
            radius: 4.0,
            max_radius: self.rng.range(60.0, 140.0),
            amplitude,
            speed: self.rng.range(38.0, 65.0),
        });
        self.stats.ripples_created += 1;
    }

    /// Offline time catchup: deterministically progress lotus bloom, pond age, and fish.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let capped = elapsed_secs.min(86400.0); // cap at 24 hours
        self.age_secs += capped;
        self.stats.pond_age_secs += capped;

        for lily in &mut self.lilies {
            if lily.has_flower {
                // Lotus flowers gently bloom and cycle
                let cycle = ((self.age_secs * 0.005) % 1.0) as f32;
                lily.flower_bloom = (cycle * TAU).sin().abs() * 0.5 + 0.5;
            }
        }
    }

    /// Advance pond simulation by `dt_secs`.
    pub fn update(
        &mut self,
        dt_secs: f32,
        interactive: bool,
        pointer: Option<[f32; 2]>,
    ) -> Activity {
        if dt_secs <= 0.0 {
            return Activity::Idle;
        }

        self.age_secs += dt_secs as f64;
        self.stats.pond_age_secs = self.age_secs;

        // Interactive cursor ripples
        if interactive && let Some(p) = pointer {
            if self.rng.f32() < 0.15 {
                self.add_ripple(p, 0.4);
            }
        }

        // Update ripples
        for r in &mut self.ripples {
            r.radius += r.speed * dt_secs;
            r.amplitude = (r.amplitude - dt_secs * 0.35).max(0.0);
        }
        self.ripples.retain(|r| r.radius < r.max_radius && r.amplitude > 0.02);

        // Update food pellets
        for p in &mut self.pellets {
            p.life += dt_secs;
            p.sink_depth = (p.sink_depth + dt_secs * 0.12).min(1.0);
        }
        self.pellets.retain(|p| p.sink_depth < 0.95);

        // Update fish boids
        let bounds = [1000.0f32, 750.0f32];
        let pellet_coords: Vec<[f32; 2]> = self.pellets.iter().map(|p| p.pos).collect();

        for fish in &mut self.fish {
            fish.turn_timer -= dt_secs;
            fish.spine_phase += fish.speed * 0.15 * dt_secs;
            fish.fin_phase += fish.speed * 0.22 * dt_secs;

            // Check if attracted to nearby food pellet
            let mut attracted = false;
            if !pellet_coords.is_empty() {
                let mut closest_d = 220.0f32;
                let mut closest_p = None;
                for p in &pellet_coords {
                    let dx = p[0] - fish.pos[0];
                    let dy = p[1] - fish.pos[1];
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < closest_d {
                        closest_d = d;
                        closest_p = Some(*p);
                    }
                }
                if let Some(target) = closest_p {
                    let angle_to = (target[1] - fish.pos[1]).atan2(target[0] - fish.pos[0]);
                    fish.target_angle = angle_to;
                    fish.target_speed = 48.0;
                    attracted = true;
                }
            }

            if !attracted && fish.turn_timer <= 0.0 {
                fish.turn_timer = self.rng.range(2.0, 6.0);
                let turn = self.rng.range(-0.8, 0.8);
                fish.target_angle = (fish.angle + turn) % TAU;
                fish.target_speed = self.rng.range(18.0, 36.0);
            }

            // Boundary avoidance
            let margin = 70.0;
            if fish.pos[0] < margin {
                fish.target_angle = 0.0;
            } else if fish.pos[0] > bounds[0] - margin {
                fish.target_angle = std::f32::consts::PI;
            }
            if fish.pos[1] < margin {
                fish.target_angle = std::f32::consts::FRAC_PI_2;
            } else if fish.pos[1] > bounds[1] - margin {
                fish.target_angle = -std::f32::consts::FRAC_PI_2;
            }

            // Smooth angular rotation towards target
            let mut diff = fish.target_angle - fish.angle;
            while diff < -std::f32::consts::PI {
                diff += TAU;
            }
            while diff > std::f32::consts::PI {
                diff -= TAU;
            }
            fish.angle += diff * (dt_secs * 2.2).min(1.0);

            fish.speed += (fish.target_speed - fish.speed) * (dt_secs * 1.5).min(1.0);
            fish.vel = [fish.angle.cos() * fish.speed, fish.angle.sin() * fish.speed];

            fish.pos[0] += fish.vel[0] * dt_secs;
            fish.pos[1] += fish.vel[1] * dt_secs;

            // Propagate spine joints along swimming trajectory with sine wave lateral offset
            let head = fish.pos;
            if !fish.spine.is_empty() {
                fish.spine[0] = head;
                let seg_len = fish.length / fish.spine.len() as f32;
                for s in 1..fish.spine.len() {
                    let prev = fish.spine[s - 1];
                    let curr = fish.spine[s];
                    let dx = curr[0] - prev[0];
                    let dy = curr[1] - prev[1];
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);

                    // Desired position trailing behind previous joint
                    let nx = dx / dist;
                    let ny = dy / dist;
                    let mut joint = [prev[0] + nx * seg_len, prev[1] + ny * seg_len];

                    // Add lateral serpentine oscillation
                    let lateral_wave = (fish.spine_phase - (s as f32) * 0.65).sin()
                        * (s as f32)
                        * 0.85;
                    let perp = [-ny, nx];
                    joint[0] += perp[0] * lateral_wave * dt_secs * 10.0;
                    joint[1] += perp[1] * lateral_wave * dt_secs * 10.0;

                    fish.spine[s] = joint;
                }
            }
        }

        // Check if any fish reached food pellets
        let mut eaten_indices = Vec::new();
        for (p_idx, p) in self.pellets.iter().enumerate() {
            for fish in &self.fish {
                let dx = fish.pos[0] - p.pos[0];
                let dy = fish.pos[1] - p.pos[1];
                if dx * dx + dy * dy < 28.0 * 28.0 {
                    eaten_indices.push(p_idx);
                    self.stats.pellets_eaten += 1;
                    break;
                }
            }
        }
        for idx in eaten_indices.into_iter().rev() {
            if idx < self.pellets.len() {
                let p = self.pellets.remove(idx);
                self.add_ripple(p.pos, 0.5);
            }
        }

        if !self.pellets.is_empty() || !self.ripples.is_empty() || interactive {
            Activity::Animating
        } else {
            Activity::Idle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koi_pond_initialization() {
        let pond = KoiPondState::new(42);
        assert_eq!(pond.fish.len(), 8);
        assert_eq!(pond.lilies.len(), 9);
        assert_eq!(pond.stats.total_fish, 8);
    }

    #[test]
    fn koi_pond_feeding_and_ripple() {
        let mut pond = KoiPondState::new(42);
        pond.feed_koi([500.0, 400.0]);
        assert_eq!(pond.pellets.len(), 4);
        assert!(!pond.ripples.is_empty());

        let act = pond.update(0.1, false, None);
        assert_eq!(act, Activity::Animating);
    }

    #[test]
    fn koi_pond_offline_catchup() {
        let mut pond = KoiPondState::new(42);
        pond.on_wake(3600.0);
        assert!(pond.age_secs >= 3600.0);
    }
}
