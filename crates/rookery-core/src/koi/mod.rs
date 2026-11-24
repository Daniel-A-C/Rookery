//! Tranquil Koi Pond Simulation State v2 (Painterly Japanese Water Garden).
//!
//! Deterministic ecosystem featuring 7 authentic Nishikigoi varieties with 12-joint
//! articulated spine kinematics and depth-diving behavior, veined lily pads with
//! blooming lotuses, skimming dragonflies, lily-pad frogs, pond turtles, gentle rain,
//! drifting seasonal petals/leaves, floating tōrō paper lanterns, mossy margin rocks,
//! and an animated bamboo shishi-odoshi fountain.

pub mod fauna;
pub mod fish;
pub mod flora;
pub mod water;

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

pub use fauna::{Dragonfly, DragonflyMode, FrogMode, PondFrog, PondTurtle};
pub use fish::{KOI_SPINE_SEGMENTS, KoiFish, KoiVariety};
pub use flora::{BambooFountain, DriftPetal, FloatingLantern, LilyPad, PondRock, SpoutPhase};
pub use water::{FoodPellet, WaterRipple, WaterSwirl};

use crate::Activity;
use crate::world::WorldRng;

pub const KOI_POND_STATE_VERSION: u32 = 2;
pub const POND_W: f32 = 1000.0;
pub const POND_H: f32 = 750.0;
pub const MAX_KOI_COUNT: usize = 18;
pub const MAX_ACTIVE_RIPPLES: usize = 32;

/// Seasonal & time-of-day atmospheric themes for the koi pond.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PondTheme {
    /// Mist-kissed morning teal, warm dawn light, drifting pink cherry blossom petals.
    SakuraMorning,
    /// Lush emerald-turquoise sunlit garden afternoon with golden caustics.
    #[default]
    SunlitJade,
    /// Amber-bronze twilight reflections, drifting crimson/gold maple leaves, warm lanterns.
    AutumnMapleDusk,
    /// Deep indigo-jade water, silver moonlight shimmer, glowing tōrō lanterns, and fireflies.
    MoonlitLanternNight,
}

pub const KOI_THEME_COUNT: usize = 4;

pub const KOI_THEME_NAMES: [&str; KOI_THEME_COUNT] = [
    "Sakura Morning",
    "Sunlit Jade",
    "Autumn Maple Dusk",
    "Moonlit Lantern Night",
];

impl PondTheme {
    pub fn from_u8(idx: u8) -> Self {
        match idx % KOI_THEME_COUNT as u8 {
            0 => PondTheme::SakuraMorning,
            1 => PondTheme::SunlitJade,
            2 => PondTheme::AutumnMapleDusk,
            _ => PondTheme::MoonlitLanternNight,
        }
    }

    pub fn index(self) -> u8 {
        match self {
            PondTheme::SakuraMorning => 0,
            PondTheme::SunlitJade => 1,
            PondTheme::AutumnMapleDusk => 2,
            PondTheme::MoonlitLanternNight => 3,
        }
    }

    pub fn name(self) -> &'static str {
        KOI_THEME_NAMES[self.index() as usize]
    }

    pub fn next(self) -> Self {
        Self::from_u8((self.index() + 1) % KOI_THEME_COUNT as u8)
    }

    /// Whether floating lanterns glow naturally by default in this theme.
    pub fn default_lanterns(self) -> bool {
        matches!(
            self,
            PondTheme::AutumnMapleDusk | PondTheme::MoonlitLanternNight
        )
    }
}

/// One-shot audio cues produced by the simulation for `KoiPondScene` to trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KoiSoundCue {
    WaterRipple,
    BambooClack,
    FeedGulp,
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
    pub time: f32,

    pub theme: PondTheme,
    pub prev_theme: PondTheme,
    /// `1.0` = fully settled on `theme`, `0.0..1.0` = smoothly cross-fading from `prev_theme`.
    pub theme_blend: f32,
    /// Continuous theme color shift rate (`0.0` = manual/off, up to `3.0`x).
    #[serde(default)]
    pub color_shift_rate: f32,

    pub rain_enabled: bool,
    pub rain_intensity: f32,
    pub rain_spawn_accum: f32,
    pub lanterns_enabled: bool,
    pub lantern_glow: f32,

    pub fish: Vec<KoiFish>,
    pub lilies: Vec<LilyPad>,
    pub rocks: Vec<PondRock>,
    pub petals: Vec<DriftPetal>,
    pub lanterns: Vec<FloatingLantern>,
    pub fountain: BambooFountain,
    pub dragonflies: Vec<Dragonfly>,
    pub frogs: Vec<PondFrog>,
    pub turtles: Vec<PondTurtle>,

    pub pellets: Vec<FoodPellet>,
    pub ripples: Vec<WaterRipple>,
    pub swirls: Vec<WaterSwirl>,
    pub stats: KoiPondStats,
    #[serde(skip)]
    pub pending_sfx: Vec<KoiSoundCue>,
}

impl KoiPondState {
    pub fn new(seed: u64) -> Self {
        let mut rng = WorldRng::new(seed);
        let mut fish = Vec::new();

        let starter_varieties = [
            KoiVariety::Kohaku,
            KoiVariety::YamabukiOgon,
            KoiVariety::Showa,
            KoiVariety::Tancho,
            KoiVariety::TaishoSanke,
            KoiVariety::Asagi,
            KoiVariety::PlatinumOgon,
            KoiVariety::Kohaku,
            KoiVariety::YamabukiOgon,
            KoiVariety::TaishoSanke,
        ];

        for (i, &var) in starter_varieties.iter().enumerate() {
            let x = rng.range(160.0, POND_W - 160.0);
            let y = rng.range(130.0, POND_H - 130.0);
            let angle = rng.range(0.0, TAU);
            let len = rng.range(64.0, 96.0);
            let girth = rng.range(0.90, 1.14);
            let depth = rng.range(0.18, 0.72);
            let pseed = rng.range(0.0, 100.0);
            fish.push(KoiFish::new(
                i as u32 + 1,
                var,
                [x, y],
                angle,
                len,
                girth,
                depth,
                pseed,
            ));
        }

        let rocks = flora::spawn_pond_rocks(&mut rng);
        let lilies = flora::spawn_lily_pads(&mut rng);
        let petals = flora::spawn_drift_petals(&mut rng, 26);
        let lanterns = flora::spawn_lanterns(&mut rng);
        let fountain = BambooFountain::new();
        let dragonflies = fauna::spawn_dragonflies(&mut rng);
        let frogs = fauna::spawn_frogs(&lilies, &mut rng);
        let turtles = fauna::spawn_turtles(&mut rng);

        let total_fish = fish.len() as u32;
        let total_lilies = lilies.len() as u32;
        let theme = PondTheme::SunlitJade;

        Self {
            version: KOI_POND_STATE_VERSION,
            seed,
            rng,
            age_secs: 0.0,
            time: 0.0,
            theme,
            prev_theme: theme,
            theme_blend: 1.0,
            color_shift_rate: 0.0,
            rain_enabled: false,
            rain_intensity: 0.0,
            rain_spawn_accum: 0.0,
            lanterns_enabled: false,
            lantern_glow: 0.0,
            fish,
            lilies,
            rocks,
            petals,
            lanterns,
            fountain,
            dragonflies,
            frogs,
            turtles,
            pellets: Vec::new(),
            ripples: Vec::new(),
            swirls: Vec::new(),
            stats: KoiPondStats {
                pond_age_secs: 0.0,
                total_fish,
                total_lilies,
                pellets_eaten: 0,
                ripples_created: 0,
            },
            pending_sfx: Vec::new(),
        }
    }

    /// Drop floating fish food pellets into the pond at `pos`.
    pub fn feed_koi(&mut self, pos: [f32; 2]) {
        for _ in 0..5 {
            let ox = self.rng.range(-26.0, 26.0);
            let oy = self.rng.range(-22.0, 22.0);
            self.pellets.push(FoodPellet {
                pos: [
                    (pos[0] + ox).clamp(40.0, POND_W - 40.0),
                    (pos[1] + oy).clamp(40.0, POND_H - 40.0),
                ],
                vel: [self.rng.range(-4.0, 4.0), self.rng.range(-4.0, 4.0)],
                sink_depth: 0.0,
                life: 0.0,
                bob_phase: self.rng.range(0.0, TAU),
            });
        }
        self.add_ripple_rings(pos, 0.75, 2);
    }

    /// Trigger a water ripple at `center`. Strong ripples gently startle nearby koi.
    pub fn add_ripple(&mut self, center: [f32; 2], amplitude: f32) {
        let rings = if amplitude >= 0.8 { 3 } else { 2 };
        self.add_ripple_rings(center, amplitude, rings);
        if amplitude >= 0.85 {
            for koi in &mut self.fish {
                koi.startle_away_from(center, amplitude);
            }
        }
    }

    fn add_ripple_rings(&mut self, center: [f32; 2], amplitude: f32, rings: u8) {
        if self.ripples.len() >= MAX_ACTIVE_RIPPLES {
            self.ripples.remove(0);
        }
        let max_r = self.rng.range(72.0, 155.0) * (0.7 + amplitude.min(1.5) * 0.35);
        let speed = self.rng.range(42.0, 68.0);
        self.ripples
            .push(WaterRipple::new(center, max_r, amplitude, speed, rings));
        self.stats.ripples_created += 1;
    }

    /// Create a decaying swirl current from cursor dragging.
    pub fn swirl_water(&mut self, pos: [f32; 2], vel: [f32; 2]) {
        if self.swirls.len() >= 12 {
            self.swirls.remove(0);
        }
        self.swirls.push(WaterSwirl {
            pos,
            vel: [vel[0].clamp(-220.0, 220.0), vel[1].clamp(-220.0, 220.0)],
            radius: 130.0,
            strength: 1.0,
        });
        if self.rng.f32() < 0.35 {
            self.add_ripple_rings(pos, 0.38, 1);
        }
    }

    /// Spawn a new Nishikigoi entering gracefully from a pond edge (up to `MAX_KOI_COUNT`).
    pub fn spawn_koi(&mut self) -> bool {
        if self.fish.len() >= MAX_KOI_COUNT {
            return false;
        }
        let id = self.fish.iter().map(|f| f.id).max().unwrap_or(0) + 1;
        let variety = KoiVariety::from_rng(&mut self.rng);
        let edge = (self.rng.f32() * 4.0) as u8 % 4;
        let (pos, angle) = match edge {
            0 => ([60.0, self.rng.range(140.0, POND_H - 140.0)], 0.0),
            1 => (
                [POND_W - 60.0, self.rng.range(140.0, POND_H - 140.0)],
                std::f32::consts::PI,
            ),
            2 => (
                [self.rng.range(160.0, POND_W - 160.0), 60.0],
                std::f32::consts::FRAC_PI_2,
            ),
            _ => (
                [self.rng.range(160.0, POND_W - 160.0), POND_H - 60.0],
                -std::f32::consts::FRAC_PI_2,
            ),
        };
        let len = self.rng.range(62.0, 94.0);
        let girth = self.rng.range(0.92, 1.12);
        let depth = self.rng.range(0.22, 0.65);
        let pseed = self.rng.range(0.0, 100.0);
        self.fish.push(KoiFish::new(
            id, variety, pos, angle, len, girth, depth, pseed,
        ));
        self.stats.total_fish = self.fish.len() as u32;
        self.add_ripple_rings(pos, 0.6, 2);
        true
    }

    /// Open lotus blossoms across the pond in a serene blooming wave.
    pub fn bloom_all_lotuses(&mut self) {
        let mut bloomed_any = false;
        for lily in &mut self.lilies {
            if !lily.has_flower && self.rng.f32() < 0.45 {
                lily.has_flower = true;
                lily.flower_bloom = 0.08;
            }
            if lily.has_flower {
                lily.target_bloom = 1.0;
                bloomed_any = true;
            }
        }
        if !bloomed_any && let Some(first) = self.lilies.first_mut() {
            first.has_flower = true;
            first.flower_bloom = 0.1;
            first.target_bloom = 1.0;
        }
    }

    /// Toggle gentle pond rain shower.
    pub fn toggle_rain(&mut self) -> bool {
        self.rain_enabled = !self.rain_enabled;
        self.rain_enabled
    }

    /// Toggle floating paper lanterns (*tōrō*).
    pub fn toggle_lanterns(&mut self) -> bool {
        self.lanterns_enabled = !self.lanterns_enabled;
        self.lanterns_enabled
    }

    /// Immediately tip the bamboo *shishi-odoshi* water spout.
    pub fn trigger_bamboo_spout(&mut self) {
        self.fountain.trigger_tip();
    }

    /// Smoothly transition to a specific seasonal/time-of-day theme.
    pub fn set_theme(&mut self, next: PondTheme) {
        if next == self.theme {
            return;
        }
        self.prev_theme = self.theme;
        self.theme = next;
        self.theme_blend = 0.0;
        if next.default_lanterns() {
            self.lanterns_enabled = true;
        }
    }

    /// Cycle to the next seasonal/time-of-day theme.
    pub fn cycle_theme(&mut self) -> PondTheme {
        let next = self.theme.next();
        self.set_theme(next);
        next
    }

    /// Set the continuous theme color shift rate (`0.0` = manual/off, up to `3.0`x).
    pub fn set_color_shift_rate(&mut self, rate: f32) {
        self.color_shift_rate = rate.clamp(0.0, 3.0);
    }

    /// Drain queued simulation sound cues.
    pub fn take_pending_sfx(&mut self) -> Vec<KoiSoundCue> {
        std::mem::take(&mut self.pending_sfx)
    }

    /// Offline time catchup: deterministically advance pond age and lotus bloom cycles.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let capped = elapsed_secs.min(86_400.0);
        self.age_secs += capped;
        self.stats.pond_age_secs = self.age_secs;

        for (i, lily) in self.lilies.iter_mut().enumerate() {
            if lily.has_flower {
                let cycle = ((self.age_secs * 0.004 + i as f64 * 0.17) % 1.0) as f32;
                let bloom = (cycle * TAU).sin().abs() * 0.45 + 0.55;
                lily.flower_bloom = bloom;
                lily.target_bloom = bloom;
            }
        }
    }

    /// Advance the entire koi pond ecosystem by `dt_secs`.
    pub fn update(
        &mut self,
        dt_secs: f32,
        interactive: bool,
        pointer: Option<[f32; 2]>,
    ) -> Activity {
        if dt_secs <= 0.0 {
            return Activity::Idle;
        }
        let dt = dt_secs.min(0.15);

        self.age_secs += dt as f64;
        self.time += dt;
        self.stats.pond_age_secs = self.age_secs;

        // Smooth theme cross-fade & continuous color shift cycling
        if self.color_shift_rate > 0.0 {
            if self.theme_blend >= 1.0 {
                self.prev_theme = self.theme;
                self.theme = self.theme.next();
                self.theme_blend = 0.0;
            }
            let rate = 0.05 * self.color_shift_rate.clamp(0.0, 3.0);
            self.theme_blend += dt * rate;
            while self.theme_blend >= 1.0 {
                self.theme_blend -= 1.0;
                self.prev_theme = self.theme;
                self.theme = self.theme.next();
            }
        } else if self.theme_blend < 1.0 {
            self.theme_blend = (self.theme_blend + dt * 0.72).min(1.0);
        }

        // Smooth rain & lantern intensity ramps
        let rain_target = if self.rain_enabled { 1.0 } else { 0.0 };
        self.rain_intensity += (rain_target - self.rain_intensity) * (dt * 1.4).min(1.0);

        let lantern_target = if self.lanterns_enabled { 1.0 } else { 0.0 };
        self.lantern_glow += (lantern_target - self.lantern_glow) * (dt * 1.6).min(1.0);

        // Spawn gentle raindrop ripples when raining
        if self.rain_intensity > 0.05 {
            self.rain_spawn_accum += dt * (4.5 * self.rain_intensity);
            while self.rain_spawn_accum >= 1.0 {
                self.rain_spawn_accum -= 1.0;
                let rx = self.rng.range(45.0, POND_W - 45.0);
                let ry = self.rng.range(45.0, POND_H - 45.0);
                self.add_ripple_rings([rx, ry], 0.34, 1);
            }
        }

        // Subtle interactive cursor hover ripples
        if interactive
            && let Some(p) = pointer
            && self.rng.f32() < 0.09
        {
            self.add_ripple_rings(p, 0.30, 1);
        }

        // Update bamboo shishi-odoshi fountain
        let (splashed, clacked) = self.fountain.update(dt);
        if splashed {
            self.add_ripple_rings(self.fountain.splash_pos, 0.85, 3);
            self.pending_sfx.push(KoiSoundCue::WaterRipple);
        }
        if clacked {
            self.pending_sfx.push(KoiSoundCue::BambooClack);
        }

        // Decay cursor swirls
        for s in &mut self.swirls {
            s.strength = (s.strength - dt * 0.95).max(0.0);
        }
        self.swirls.retain(|s| s.strength > 0.02);

        // Expand and attenuate ripples
        for r in &mut self.ripples {
            r.radius += r.speed * dt;
            r.amplitude = (r.amplitude - dt * 0.28).max(0.0);
        }
        self.ripples
            .retain(|r| r.radius < r.max_radius && r.amplitude > 0.015);

        // Update food pellets (gentle swirl advection + slow dissolving)
        for p in &mut self.pellets {
            p.life += dt;
            p.bob_phase = (p.bob_phase + dt * 3.0) % TAU;
            p.sink_depth = (p.sink_depth + dt * 0.055).min(1.0);
            for s in &self.swirls {
                let dx = p.pos[0] - s.pos[0];
                let dy = p.pos[1] - s.pos[1];
                let d = (dx * dx + dy * dy).sqrt();
                if d < s.radius {
                    let f = (1.0 - d / s.radius) * s.strength;
                    p.vel[0] += s.vel[0] * f * dt * 1.8;
                    p.vel[1] += s.vel[1] * f * dt * 1.8;
                }
            }
            p.vel[0] *= (1.0 - dt * 1.8).max(0.0);
            p.vel[1] *= (1.0 - dt * 1.8).max(0.0);
            p.pos[0] = (p.pos[0] + p.vel[0] * dt).clamp(32.0, POND_W - 32.0);
            p.pos[1] = (p.pos[1] + p.vel[1] * dt).clamp(32.0, POND_H - 32.0);
        }
        self.pellets.retain(|p| p.sink_depth < 0.96);

        // Update articulated koi school
        let wakes = fish::update_koi_school(&mut self.fish, &self.pellets, dt, &mut self.rng);
        for w in wakes {
            if self.ripples.len() < MAX_ACTIVE_RIPPLES {
                self.ripples.push(w);
            }
        }

        // Check if any koi reached a food pellet -> surface gulp + splash ripple
        let mut eaten = Vec::new();
        for (p_idx, p) in self.pellets.iter().enumerate() {
            for koi in &mut self.fish {
                let dx = koi.pos[0] - p.pos[0];
                let dy = koi.pos[1] - p.pos[1];
                if dx * dx + dy * dy < 28.0 * 28.0 {
                    eaten.push(p_idx);
                    koi.gulp_timer = 0.45;
                    self.stats.pellets_eaten += 1;
                    break;
                }
            }
        }
        for idx in eaten.into_iter().rev() {
            if idx < self.pellets.len() {
                let p = self.pellets.remove(idx);
                self.add_ripple_rings(p.pos, 0.62, 2);
                self.pending_sfx.push(KoiSoundCue::FeedGulp);
            }
        }

        // Update flora, drifting petals, and lanterns
        flora::update_flora_and_props(
            &mut self.lilies,
            &mut self.petals,
            &mut self.lanterns,
            &self.ripples,
            &self.swirls,
            self.theme,
            self.time,
            dt,
        );

        // Update pond fauna (dragonflies, frogs, turtles)
        let fauna_splashes = fauna::update_fauna(
            &mut self.dragonflies,
            &mut self.frogs,
            &mut self.turtles,
            &self.lilies,
            dt,
            &mut self.rng,
        );
        for s in fauna_splashes {
            if self.ripples.len() < MAX_ACTIVE_RIPPLES {
                self.ripples.push(s);
                self.stats.ripples_created += 1;
            }
        }

        // Keep scene smoothly animating since fish, caustics, and dragonflies are always alive
        Activity::Animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koi_pond_initialization() {
        let pond = KoiPondState::new(42);
        assert_eq!(pond.version, KOI_POND_STATE_VERSION);
        assert_eq!(pond.fish.len(), 10);
        assert_eq!(pond.lilies.len(), 11);
        assert_eq!(pond.stats.total_fish, 10);
        assert!(!pond.rocks.is_empty());
        assert!(!pond.petals.is_empty());
        assert!(!pond.dragonflies.is_empty());
        assert!(!pond.frogs.is_empty());
        assert!(!pond.turtles.is_empty());
    }

    #[test]
    fn koi_pond_feeding_and_ripple() {
        let mut pond = KoiPondState::new(42);
        pond.feed_koi([500.0, 375.0]);
        assert_eq!(pond.pellets.len(), 5);
        assert!(!pond.ripples.is_empty());

        let act = pond.update(0.1, false, None);
        assert_eq!(act, Activity::Animating);
    }

    #[test]
    fn koi_pond_theme_cycle_and_blend() {
        let mut pond = KoiPondState::new(42);
        assert_eq!(pond.theme, PondTheme::SunlitJade);
        pond.cycle_theme();
        assert_eq!(pond.theme, PondTheme::AutumnMapleDusk);
        assert_eq!(pond.theme_blend, 0.0);
        assert!(pond.lanterns_enabled);

        for _ in 0..30 {
            pond.update(0.1, false, None);
        }
        assert!((pond.theme_blend - 1.0).abs() < 1e-4);
    }

    #[test]
    fn koi_pond_spawn_cap_and_rain() {
        let mut pond = KoiPondState::new(7);
        for _ in 0..20 {
            pond.spawn_koi();
        }
        assert_eq!(pond.fish.len(), MAX_KOI_COUNT);

        assert!(pond.toggle_rain());
        for _ in 0..30 {
            pond.update(0.1, false, None);
        }
        assert!(pond.rain_intensity > 0.5);
        assert!(pond.stats.ripples_created > 0);
    }

    #[test]
    fn koi_pond_bamboo_fountain_tips_and_clacks() {
        let mut pond = KoiPondState::new(99);
        pond.trigger_bamboo_spout();
        let mut saw_ripple = false;
        let mut saw_clack = false;
        for _ in 0..40 {
            pond.update(0.05, false, None);
            for cue in pond.take_pending_sfx() {
                if cue == KoiSoundCue::WaterRipple {
                    saw_ripple = true;
                }
                if cue == KoiSoundCue::BambooClack {
                    saw_clack = true;
                }
            }
        }
        assert!(saw_ripple, "expected bamboo spout splash ripple");
        assert!(saw_clack, "expected bamboo spout clack cue");
    }

    #[test]
    fn koi_pond_offline_catchup() {
        let mut pond = KoiPondState::new(42);
        pond.on_wake(3600.0);
        assert!(pond.age_secs >= 3600.0);
    }

    #[test]
    fn koi_pond_color_shift_rate_cycles_smoothly() {
        let mut pond = KoiPondState::new(42);
        assert_eq!(pond.color_shift_rate, 0.0);
        assert_eq!(pond.theme, PondTheme::SunlitJade);

        // At 0.0x, theme stays settled.
        for _ in 0..20 {
            pond.update(0.1, false, None);
        }
        assert_eq!(pond.theme, PondTheme::SunlitJade);
        assert!((pond.theme_blend - 1.0).abs() < 1e-4);

        // At 2.0x (rate = 0.10/s), 5s advances halfway through the next theme transition.
        pond.set_color_shift_rate(2.0);
        for _ in 0..50 {
            pond.update(0.1, false, None);
        }
        assert_eq!(pond.prev_theme, PondTheme::SunlitJade);
        assert_eq!(pond.theme, PondTheme::AutumnMapleDusk);
        assert!((pond.theme_blend - 0.5).abs() < 1e-3);

        // Returning to 0.0x finishes settling onto the target theme.
        pond.set_color_shift_rate(0.0);
        for _ in 0..20 {
            pond.update(0.1, false, None);
        }
        assert!((pond.theme_blend - 1.0).abs() < 1e-4);
    }
}
