//! Cosmic Nebula Simulation State (Milestone 4 - Generative Physics & Shaders).
//!
//! Algorithmic deep-space simulation with glowing celestial particles, shifting gravitational
//! attractor/repulsor vortexes, and customizable stellar palettes and turbulence.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::Activity;
use crate::world::WorldRng;

pub const COSMIC_NEBULA_STATE_VERSION: u32 = 1;

/// Color palette presets for the Cosmic Nebula.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NebulaPalette {
    /// Deep cyan, sapphire, and brilliant golden starlight.
    StellarGold,
    /// Royal purple, magenta, and amethyst ion glow.
    DeepAmethyst,
    /// Solar flare orange, crimson, and radiant white heat.
    SolarFlare,
    /// Ethereal green, jade, and turquoise aurora veil.
    AuroraVeil,
}

impl NebulaPalette {
    pub fn name(&self) -> &'static str {
        match self {
            Self::StellarGold => "Stellar Gold",
            Self::DeepAmethyst => "Deep Amethyst",
            Self::SolarFlare => "Solar Flare",
            Self::AuroraVeil => "Aurora Veil",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::StellarGold => Self::DeepAmethyst,
            Self::DeepAmethyst => Self::SolarFlare,
            Self::SolarFlare => Self::AuroraVeil,
            Self::AuroraVeil => Self::StellarGold,
        }
    }
}

/// A glowing celestial particle/star orbiting in the nebula.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StellarParticle {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub base_speed: f32,
    pub mass: f32,
    pub size: f32,
    pub color_t: f32, // [0.0, 1.0] parameter along the active palette
    pub phase: f32,
}

/// An interactive gravitational well (attractor or repulsor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GravityWell {
    pub id: u32,
    pub pos: [f32; 2],
    pub strength: f32, // positive = attract, negative = repel
    pub radius: f32,
    pub is_repulsor: bool,
    pub life: f32,
    pub max_life: f32,
}

/// Telemetry and stats for Cosmic Nebula.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CosmicNebulaStats {
    pub total_particles: u32,
    pub active_vortices: u32,
    pub simulation_age_secs: f64,
}

/// Deterministic simulation state for Cosmic Nebula.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CosmicNebulaState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub age_secs: f64,
    pub palette: NebulaPalette,
    pub turbulence: f32, // [0.1, 2.0]
    pub flow_speed: f32, // [0.2, 3.0]
    pub particles: Vec<StellarParticle>,
    pub wells: Vec<GravityWell>,
    pub stats: CosmicNebulaStats,
    pub next_id: u32,
}

impl CosmicNebulaState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let particle_count = 600; // Balanced for CPU tick < 0.2ms and gorgeous visuals
        let mut particles = Vec::with_capacity(particle_count);

        for _ in 0..particle_count {
            let x = rng.range(50.0, 950.0);
            let y = rng.range(50.0, 700.0);
            let speed = rng.range(15.0, 45.0);
            let angle = rng.range(0.0, TAU);
            particles.push(StellarParticle {
                pos: [x, y],
                vel: [angle.cos() * speed, angle.sin() * speed],
                base_speed: speed,
                mass: rng.range(0.8, 1.5),
                size: rng.range(1.2, 3.5),
                color_t: rng.range(0.0, 1.0),
                phase: rng.range(0.0, TAU),
            });
        }

        // Initial gentle central gravitational vortex
        let mut wells = Vec::new();
        wells.push(GravityWell {
            id: 1,
            pos: [500.0, 375.0],
            strength: 420.0,
            radius: 350.0,
            is_repulsor: false,
            life: 0.0,
            max_life: 999999.0, // standing anchor
        });

        Self {
            version: COSMIC_NEBULA_STATE_VERSION,
            seed,
            rng,
            age_secs: 0.0,
            palette: NebulaPalette::StellarGold,
            turbulence: 1.0,
            flow_speed: 1.0,
            particles,
            wells,
            stats: CosmicNebulaStats {
                total_particles: particle_count as u32,
                active_vortices: 1,
                simulation_age_secs: 0.0,
            },
            next_id: 10,
        }
    }

    pub fn cycle_palette(&mut self) -> NebulaPalette {
        self.palette = self.palette.next();
        self.palette
    }

    pub fn add_gravity_well(&mut self, pos: [f32; 2], is_repulsor: bool) {
        self.next_id += 1;
        let strength = if is_repulsor { -800.0 } else { 750.0 };
        self.wells.push(GravityWell {
            id: self.next_id,
            pos,
            strength,
            radius: 280.0,
            is_repulsor,
            life: 0.0,
            max_life: 25.0, // fades over 25 seconds
        });
        self.stats.active_vortices = self.wells.len() as u32;
    }

    pub fn on_wake(&mut self, elapsed_secs: f64) {
        self.age_secs += elapsed_secs.min(86400.0);
        self.stats.simulation_age_secs = self.age_secs;
    }

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
        self.stats.simulation_age_secs = self.age_secs;

        // Interactive gravity cursor in active mode
        if interactive && let Some(p) = pointer {
            if self.rng.f32() < 0.08 && self.wells.len() < 5 {
                self.add_gravity_well(p, false);
            }
        }

        // Update gravitational wells
        for w in &mut self.wells {
            w.life += dt_secs;
        }
        self.wells.retain(|w| w.life < w.max_life);
        self.stats.active_vortices = self.wells.len() as u32;

        let bounds = [1000.0f32, 750.0f32];
        let turb = self.turbulence;
        let flow = self.flow_speed;

        for p in &mut self.particles {
            p.phase += dt_secs * 1.5;

            // Compute gravitational acceleration from all active wells
            let mut acc = [0.0f32, 0.0f32];
            for w in &self.wells {
                let dx = w.pos[0] - p.pos[0];
                let dy = w.pos[1] - p.pos[1];
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt().max(10.0);

                if dist < w.radius {
                    // Softened inverse-square force
                    let f = (w.strength / (dist_sq + 400.0)) * 25.0;
                    acc[0] += (dx / dist) * f;
                    acc[1] += (dy / dist) * f;

                    // Add tangential vortex swirl
                    let perp = [-dy / dist, dx / dist];
                    let swirl = f * 0.45 * if w.is_repulsor { -1.0 } else { 1.0 };
                    acc[0] += perp[0] * swirl;
                    acc[1] += perp[1] * swirl;
                }
            }

            // Pseudo curl-noise turbulence
            let freq = 0.005;
            let noise_x = (p.pos[1] * freq + self.age_secs as f32 * 0.2).sin();
            let noise_y = (p.pos[0] * freq + self.age_secs as f32 * 0.2).cos();
            acc[0] += noise_x * 12.0 * turb;
            acc[1] += noise_y * 12.0 * turb;

            p.vel[0] = (p.vel[0] + acc[0] * dt_secs) * 0.985;
            p.vel[1] = (p.vel[1] + acc[1] * dt_secs) * 0.985;

            // Clamp velocity
            let spd = (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt();
            let max_spd = 160.0 * flow;
            if spd > max_spd {
                p.vel[0] = (p.vel[0] / spd) * max_spd;
                p.vel[1] = (p.vel[1] / spd) * max_spd;
            }

            p.pos[0] += p.vel[0] * dt_secs * flow;
            p.pos[1] += p.vel[1] * dt_secs * flow;

            // Toroidal wrapping around viewport edges
            if p.pos[0] < -20.0 {
                p.pos[0] = bounds[0] + 20.0;
            } else if p.pos[0] > bounds[0] + 20.0 {
                p.pos[0] = -20.0;
            }
            if p.pos[1] < -20.0 {
                p.pos[1] = bounds[1] + 20.0;
            } else if p.pos[1] > bounds[1] + 20.0 {
                p.pos[1] = -20.0;
            }
        }

        Activity::Animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosmic_nebula_initialization() {
        let nebula = CosmicNebulaState::new(42);
        assert_eq!(nebula.particles.len(), 600);
        assert_eq!(nebula.wells.len(), 1);
        assert_eq!(nebula.palette, NebulaPalette::StellarGold);
    }

    #[test]
    fn cosmic_nebula_add_well_and_cycle() {
        let mut nebula = CosmicNebulaState::new(42);
        nebula.add_gravity_well([400.0, 300.0], true);
        assert_eq!(nebula.wells.len(), 2);

        let pal = nebula.cycle_palette();
        assert_eq!(pal, NebulaPalette::DeepAmethyst);

        let act = nebula.update(0.1, false, None);
        assert_eq!(act, Activity::Animating);
    }
}
