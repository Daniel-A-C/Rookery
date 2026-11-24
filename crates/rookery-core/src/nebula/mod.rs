//! Cosmic Nebula v2.0 — Stellar Nursery & Proto-Star Forge Simulation State.
//!
//! Deterministic, GPU-agnostic deep-space nursery simulation featuring:
//! - Living stellar lifecycles (Protostar collapse -> Main-Sequence JWST star -> Supergiant -> Supernova)
//! - Expanding supernova shockwaves that sweep stardust and seed new starbirth along their rim
//! - Dual-tail comets, periodic meteor showers, and a sweeping neutron-star pulsar beacon
//! - Cursor-sculpted gas/plasma stir wakes and 450 physics-driven stardust motes
//! - 5 curated JWST / Hubble region palettes with smooth crossfades
//! - Toggleable Astral Cartography constellation graph

mod lifecycle;

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::Activity;
use crate::world::WorldRng;

pub const COSMIC_NEBULA_STATE_VERSION: u32 = 2;
pub const WORLD_W: f32 = 1000.0;
pub const WORLD_H: f32 = 750.0;

pub const MAX_NURSERY_STARS: usize = 20;
pub const MAX_SHOCKWAVES: usize = 6;
pub const MAX_COMETS: usize = 2;
pub const MAX_METEORS: usize = 16;
pub const MAX_STIR_WAKES: usize = 12;
pub const DUST_MOTE_COUNT: usize = 450;

pub const NEBULA_PALETTE_COUNT: usize = 5;
pub const NEBULA_PALETTE_NAMES: [&str; NEBULA_PALETTE_COUNT] = [
    "Carina Cliffs",
    "Pillars of Creation",
    "Horsehead Crimson",
    "Helix Violet",
    "Veil Aurora",
];

/// Curated JWST / Hubble deep-space region palettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NebulaPalette {
    /// JWST Cosmic Cliffs: midnight indigo-teal, copper/amber dust ridges, champagne-gold rims.
    CarinaCliffs = 0,
    /// Eagle Nebula: abyssal teal-slate, emerald/cyan ionized veil, towering bronze-ochre pillars.
    PillarsOfCreation = 1,
    /// Orion molecular cloud: obsidian-plum, H-alpha crimson/magenta veil, icy sapphire stars.
    HorseheadCrimson = 2,
    /// Planetary nebula jewel: cosmic ultraviolet, electric orchid/amethyst shells, cyan core glow.
    HelixViolet = 3,
    /// Supernova remnant filaments: void navy, gossamer emerald-mint & cyan ribbons, warm peach dust.
    VeilAurora = 4,
}

impl NebulaPalette {
    pub fn index(self) -> u8 {
        self as u8
    }

    pub fn from_u8(idx: u8) -> Self {
        match idx % (NEBULA_PALETTE_COUNT as u8) {
            0 => Self::CarinaCliffs,
            1 => Self::PillarsOfCreation,
            2 => Self::HorseheadCrimson,
            3 => Self::HelixViolet,
            _ => Self::VeilAurora,
        }
    }

    pub fn name(self) -> &'static str {
        NEBULA_PALETTE_NAMES[self.index() as usize]
    }

    pub fn next(self) -> Self {
        Self::from_u8(self.index() + 1)
    }
}

/// Evolutionary stage of a star inside the stellar nursery.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StarStage {
    /// Dense molecular knot collapsing under gravity (`collapse` 0.0 -> 1.0).
    Protostar { collapse: f32 },
    /// Luminous main-sequence star with JWST 6-point diffraction spikes.
    MainSequence,
    /// Massive aging star pulsing unstable prior to natural supernova detonation.
    Supergiant { instability: f32 },
}

/// A living star inside the nursery that illuminates surrounding gas and dust.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NurseryStar {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub stage: StarStage,
    /// Spectral temperature: 0.0 = warm amber-gold, 0.5 = ivory white, 1.0 = icy O-class blue.
    pub temp_t: f32,
    /// Visual brightness multiplier (0.55 .. 1.85).
    pub luminosity: f32,
    /// Base visual scale in world units.
    pub radius: f32,
    pub age: f32,
    pub lifespan: f32,
    pub phase: f32,
    /// Transient burst right when a protostar ignites (1.0 -> 0.0).
    pub ignition_flash: f32,
}

/// An expanding supernova ionization shockwave ring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shockwave {
    pub id: u32,
    pub pos: [f32; 2],
    pub radius: f32,
    pub max_radius: f32,
    pub age: f32,
    pub max_age: f32,
    /// Spectral accent tint along the ionization front.
    pub hue_t: f32,
    /// Whether this shockwave has already triggered secondary starbirth along its rim.
    pub seeded_birth: bool,
}

/// A sweeping deep-space comet with a straight cyan ion tail and curved golden dust tail.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comet {
    pub id: u32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub age: f32,
    pub max_age: f32,
    pub brightness: f32,
    /// Recent positions used to sculpt the gently curved dust tail.
    pub trail: Vec<[f32; 2]>,
}

/// A crisp micro-meteor / shooting star streaking across the deep field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Meteor {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub age: f32,
    pub max_age: f32,
    pub length: f32,
    pub brightness: f32,
}

/// A spinning neutron-star pulsar tucked in the nursery, sweeping twin synchrotron beams.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pulsar {
    pub pos: [f32; 2],
    pub angle: f32,
    pub spin_speed: f32,
    pub pulse_phase: f32,
    pub intensity: f32,
}

/// A short-lived cursor sculpting/stirring wake that warps the gas shader and nudges dust motes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StirWake {
    pub pos: [f32; 2],
    pub dir: [f32; 2],
    pub strength: f32,
    pub age: f32,
    pub max_age: f32,
}

/// A lightweight luminous stardust mote drifting through the nebula currents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DustMote {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub size: f32,
    /// Parallax depth factor (0.35 = far, 1.0 = foreground).
    pub depth: f32,
    pub color_t: f32,
    pub phase: f32,
}

/// An edge in the toggleable Astral Cartography constellation graph.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstellationEdge {
    pub a: usize,
    pub b: usize,
    pub strength: f32,
}

/// Discrete audio cues emitted by the simulation for the renderer's `AudioEngine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NebulaSoundCue {
    StarIgnite,
    SupernovaBoom,
    CometSweep,
    NebulaPulse,
    PaletteCycle,
}

/// Live telemetry and statistics for Cosmic Nebula v2.0.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CosmicNebulaStats {
    pub stars_active: u32,
    pub protostars: u32,
    pub stars_born_total: u32,
    pub supernovae_total: u32,
    pub active_shockwaves: u32,
    pub active_comets: u32,
    pub active_meteors: u32,
    pub dust_motes: u32,
    pub simulation_age_secs: f64,
}

/// Deterministic simulation state for Cosmic Nebula v2.0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CosmicNebulaState {
    pub version: u32,
    pub seed: u64,
    pub rng: WorldRng,
    pub age_secs: f64,

    pub palette: NebulaPalette,
    pub prev_palette: NebulaPalette,
    pub palette_blend: f32,

    pub star_chart_enabled: bool,
    pub star_chart_alpha: f32,

    pub stars: Vec<NurseryStar>,
    pub shockwaves: Vec<Shockwave>,
    pub comets: Vec<Comet>,
    pub meteors: Vec<Meteor>,
    pub pulsar: Pulsar,
    pub wakes: Vec<StirWake>,
    pub dust: Vec<DustMote>,
    pub constellation_edges: Vec<ConstellationEdge>,

    pub stats: CosmicNebulaStats,
    pub next_id: u32,

    pub(crate) next_starbirth_in: f32,
    pub(crate) next_comet_in: f32,
    pub(crate) next_meteor_in: f32,
    pub(crate) comet_trail_accum: f32,
    pub(crate) graph_dirty: bool,

    #[serde(skip)]
    pub pending_sounds: Vec<NebulaSoundCue>,
}

impl CosmicNebulaState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut next_id = 1u32;

        // Seed 11 well-composed initial nursery stars (9 mature + 1 supergiant + 1 collapsing protostar)
        let mut stars = Vec::with_capacity(MAX_NURSERY_STARS);
        let anchor_positions: [[f32; 2]; 11] = [
            [185.0, 210.0],
            [335.0, 155.0],
            [510.0, 245.0],
            [690.0, 175.0],
            [835.0, 265.0],
            [250.0, 465.0],
            [435.0, 395.0],
            [620.0, 445.0],
            [795.0, 520.0],
            [365.0, 595.0],
            [575.0, 610.0],
        ];

        for (i, base) in anchor_positions.iter().enumerate() {
            let jitter_x = rng.range(-38.0, 38.0);
            let jitter_y = rng.range(-30.0, 30.0);
            let pos = [
                (base[0] + jitter_x).clamp(70.0, WORLD_W - 70.0),
                (base[1] + jitter_y).clamp(60.0, WORLD_H - 60.0),
            ];
            let drift_angle = rng.range(0.0, TAU);
            let drift_speed = rng.range(1.2, 3.8);
            let stage = if i == 10 {
                StarStage::Protostar { collapse: 0.25 }
            } else if i == 2 {
                StarStage::Supergiant { instability: 0.15 }
            } else {
                StarStage::MainSequence
            };
            let is_hero = i == 2 || i == 6 || i == 3;
            let luminosity = if is_hero {
                rng.range(1.35, 1.75)
            } else {
                rng.range(0.65, 1.25)
            };
            let radius = if is_hero {
                rng.range(14.0, 20.0)
            } else {
                rng.range(7.0, 13.0)
            };
            stars.push(NurseryStar {
                id: next_id,
                pos,
                vel: [
                    drift_angle.cos() * drift_speed,
                    drift_angle.sin() * drift_speed,
                ],
                stage,
                temp_t: rng.range(0.0, 1.0),
                luminosity,
                radius,
                age: rng.range(10.0, 60.0),
                lifespan: rng.range(140.0, 280.0),
                phase: rng.range(0.0, TAU),
                ignition_flash: 0.0,
            });
            next_id += 1;
        }

        // Seed 450 luminous stardust motes across parallax depths
        let mut dust = Vec::with_capacity(DUST_MOTE_COUNT);
        for _ in 0..DUST_MOTE_COUNT {
            let angle = rng.range(0.0, TAU);
            let speed = rng.range(4.0, 16.0);
            dust.push(DustMote {
                pos: [rng.range(0.0, WORLD_W), rng.range(0.0, WORLD_H)],
                vel: [angle.cos() * speed, angle.sin() * speed],
                size: rng.range(1.2, 3.8),
                depth: rng.range(0.35, 1.0),
                color_t: rng.range(0.0, 1.0),
                phase: rng.range(0.0, TAU),
            });
        }

        // Place a serene spinning pulsar in the upper-right nebula quadrant
        let pulsar = Pulsar {
            pos: [rng.range(680.0, 860.0), rng.range(120.0, 260.0)],
            angle: rng.range(0.0, TAU),
            spin_speed: 0.42,
            pulse_phase: 0.0,
            intensity: 0.85,
        };

        let mut state = Self {
            version: COSMIC_NEBULA_STATE_VERSION,
            seed,
            rng,
            age_secs: 0.0,
            palette: NebulaPalette::CarinaCliffs,
            prev_palette: NebulaPalette::CarinaCliffs,
            palette_blend: 1.0,
            star_chart_enabled: false,
            star_chart_alpha: 0.0,
            stars,
            shockwaves: Vec::with_capacity(MAX_SHOCKWAVES),
            comets: Vec::with_capacity(MAX_COMETS),
            meteors: Vec::with_capacity(MAX_METEORS),
            pulsar,
            wakes: Vec::with_capacity(MAX_STIR_WAKES),
            dust,
            constellation_edges: Vec::new(),
            stats: CosmicNebulaStats {
                stars_active: 11,
                protostars: 1,
                stars_born_total: 11,
                supernovae_total: 0,
                active_shockwaves: 0,
                active_comets: 0,
                active_meteors: 0,
                dust_motes: DUST_MOTE_COUNT as u32,
                simulation_age_secs: 0.0,
            },
            next_id,
            next_starbirth_in: 14.0,
            next_comet_in: 22.0,
            next_meteor_in: 7.5,
            comet_trail_accum: 0.0,
            graph_dirty: true,
            pending_sounds: Vec::new(),
        };

        state.rebuild_constellations();
        state.refresh_stats();
        state
    }

    /// Cycle smoothly to the next JWST / Hubble cosmic region palette.
    pub fn cycle_palette(&mut self) -> NebulaPalette {
        let next = self.palette.next();
        self.set_palette(next);
        next
    }

    /// Smoothly transition to a specific cosmic region palette.
    pub fn set_palette(&mut self, palette: NebulaPalette) {
        if palette == self.palette {
            return;
        }
        self.prev_palette = self.palette;
        self.palette = palette;
        self.palette_blend = 0.0;
        self.pending_sounds.push(NebulaSoundCue::PaletteCycle);
    }

    /// Toggle the delicate Astral Cartography constellation overlay.
    pub fn toggle_star_chart(&mut self) -> bool {
        self.star_chart_enabled = !self.star_chart_enabled;
        self.star_chart_enabled
    }

    /// Drain all queued sound cues emitted since the last frame.
    pub fn drain_sounds(&mut self) -> Vec<NebulaSoundCue> {
        std::mem::take(&mut self.pending_sounds)
    }

    /// Advance offline catch-up gracefully after system sleep or occlusion.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let clamped = elapsed_secs.clamp(0.0, 86_400.0);
        self.age_secs += clamped;

        // Mature any in-progress protostars if enough time elapsed
        if clamped > 5.0 {
            for star in &mut self.stars {
                if let StarStage::Protostar { .. } = star.stage {
                    star.stage = StarStage::MainSequence;
                    star.ignition_flash = 0.6;
                }
            }
            self.graph_dirty = true;
        }

        // Advance pulsar angle cleanly
        self.pulsar.angle = (self.pulsar.angle + (clamped as f32) * self.pulsar.spin_speed) % TAU;
        self.palette_blend = 1.0;
        self.refresh_stats();
    }

    /// Advance the entire stellar nursery by `dt_secs`.
    pub fn update(
        &mut self,
        dt_secs: f32,
        _interactive: bool,
        _pointer: Option<[f32; 2]>,
    ) -> Activity {
        if dt_secs <= 0.0 {
            return Activity::Idle;
        }

        let dt = dt_secs.min(0.15);
        self.age_secs += dt as f64;

        // Smooth palette crossfade (~1.4s)
        if self.palette_blend < 1.0 {
            self.palette_blend = (self.palette_blend + dt * 0.72).min(1.0);
        }

        // Smooth Star Chart fade in/out
        let target_chart = if self.star_chart_enabled { 1.0 } else { 0.0 };
        self.star_chart_alpha += (target_chart - self.star_chart_alpha) * (dt * 4.0).min(1.0);
        if (self.star_chart_alpha - target_chart).abs() < 0.002 {
            self.star_chart_alpha = target_chart;
        }

        self.step_pulsar(dt);
        self.step_stars(dt);
        self.step_shockwaves(dt);
        self.step_comets(dt);
        self.step_meteors(dt);
        self.step_wakes(dt);
        self.step_dust(dt);
        self.step_ambient_timers(dt);

        if self.graph_dirty {
            self.rebuild_constellations();
            self.graph_dirty = false;
        }

        self.refresh_stats();
        Activity::Animating
    }

    pub(crate) fn refresh_stats(&mut self) {
        let protostars = self
            .stars
            .iter()
            .filter(|s| matches!(s.stage, StarStage::Protostar { .. }))
            .count() as u32;
        self.stats.stars_active = self.stars.len() as u32;
        self.stats.protostars = protostars;
        self.stats.active_shockwaves = self.shockwaves.len() as u32;
        self.stats.active_comets = self.comets.len() as u32;
        self.stats.active_meteors = self.meteors.len() as u32;
        self.stats.dust_motes = self.dust.len() as u32;
        self.stats.simulation_age_secs = self.age_secs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization_and_determinism() {
        let a = CosmicNebulaState::new(42);
        let b = CosmicNebulaState::new(42);
        assert_eq!(a, b);
        assert_eq!(a.stars.len(), 11);
        assert_eq!(a.dust.len(), DUST_MOTE_COUNT);
        assert_eq!(a.palette, NebulaPalette::CarinaCliffs);
        assert!(!a.constellation_edges.is_empty());
    }

    #[test]
    fn protostar_ignites_into_main_sequence() {
        let mut state = CosmicNebulaState::new(7);
        state.ignite_star(Some([500.0, 375.0]));
        assert!(
            state
                .stars
                .iter()
                .any(|s| matches!(s.stage, StarStage::Protostar { .. }))
        );

        let mut saw_ignite_cue = false;
        for _ in 0..160 {
            state.update(0.033, false, None);
            if state.drain_sounds().contains(&NebulaSoundCue::StarIgnite) {
                saw_ignite_cue = true;
            }
        }
        assert!(
            saw_ignite_cue,
            "protostar should emit StarIgnite cue upon collapse"
        );
    }

    #[test]
    fn supernova_triggers_shockwave_and_seeds_starbirth() {
        let mut state = CosmicNebulaState::new(99);
        let initial_born = state.stats.stars_born_total;
        state.trigger_supernova(Some([480.0, 360.0]));
        assert_eq!(state.shockwaves.len(), 1);
        assert_eq!(state.stats.supernovae_total, 1);

        for _ in 0..120 {
            state.update(0.033, false, None);
        }
        assert!(
            state.stats.stars_born_total > initial_born,
            "supernova shockwave should seed at least one new protostar"
        );
    }

    #[test]
    fn events_wakes_palettes_and_chart_work() {
        let mut state = CosmicNebulaState::new(123);
        state.summon_comet();
        assert_eq!(state.comets.len(), 1);

        state.trigger_meteor_shower();
        assert!(state.meteors.len() >= 6);

        state.stir_at([500.0, 375.0], [18.0, -10.0]);
        assert_eq!(state.wakes.len(), 1);

        assert!(state.toggle_star_chart());
        for _ in 0..40 {
            state.update(0.05, false, None);
        }
        assert!(state.star_chart_alpha > 0.8);

        for expected in 1..=NEBULA_PALETTE_COUNT {
            let p = state.cycle_palette();
            assert_eq!(p.index() as usize, expected % NEBULA_PALETTE_COUNT);
        }
    }
}
