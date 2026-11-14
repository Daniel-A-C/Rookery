//! Deterministic simulation state for *Shinjuku Rain* (`shinjuku-rain`).
//!
//! Models a heavy-rain Japanese city street in normalized `[0..1000, 0..1000]`
//! world coordinates: wind-slanted rain, puddle splashes, pedestrians with
//! umbrellas, passing taxis/kei trucks, an elevated commuter train, alley cats,
//! rising ramen-shop steam, and distant lightning.

use serde::{Deserialize, Serialize};

use crate::Activity;

/// Display names for the 4 curated ANSI palettes in Shinjuku Rain.
pub const SHINJUKU_PALETTE_NAMES: [&str; 4] = [
    "Shinjuku Neon (Kabukicho)",
    "Kyoto Gion (Lantern Alley)",
    "Midnight Downpour (Slate)",
    "Cyber Terminal (Phosphor)",
];

/// Curated ANSI color palettes for Shinjuku Rain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ShinjukuPalette {
    #[default]
    ShinjukuNeon = 0,
    KyotoGion = 1,
    MidnightDownpour = 2,
    CyberTerminal = 3,
}

impl ShinjukuPalette {
    pub fn from_u8(v: u8) -> Self {
        match v % 4 {
            0 => Self::ShinjukuNeon,
            1 => Self::KyotoGion,
            2 => Self::MidnightDownpour,
            _ => Self::CyberTerminal,
        }
    }

    pub fn next(self) -> Self {
        Self::from_u8((self as u8) + 1)
    }

    pub fn name(self) -> &'static str {
        SHINJUKU_PALETTE_NAMES[self as usize]
    }
}

/// Deterministic xorshift64* PRNG for reproducible street simulation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RainRng {
    state: u64,
}

impl RainRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }
}

/// Depth layer for rain streaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RainLayer {
    Far,
    Mid,
    Near,
}

/// A single falling raindrop streak.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Raindrop {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
    pub layer: RainLayer,
}

/// Expanding puddle ripple or splash crown on the wet street/sidewalk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PuddleSplash {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub max_age: f32,
    pub scale: f32,
}

/// Autonomous pedestrian carrying an umbrella along the sidewalk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pedestrian {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub umbrella_color: u8,
    pub pause_secs: f32,
    pub walk_phase: f32,
}

/// Type of vehicle cruising the wet street.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleKind {
    Taxi,
    KeiTruck,
}

/// Passing vehicle on the asphalt street.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub kind: VehicleKind,
}

/// Elevated commuter train gliding across the overhead steel viaduct.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevatedTrain {
    pub x: f32,
    pub vx: f32,
    pub car_count: u8,
}

/// Stray alley cat sheltering under awnings or darting across puddles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlleyCat {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub facing_right: bool,
    pub darting: bool,
    pub timer: f32,
}

/// Rising steam particle from the ramen shop or street grate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteamParticle {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub max_age: f32,
    pub drift: f32,
}

/// Live telemetry summary for the popover HUD.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShinjukuStats {
    pub rain_rate_mm_hr: u32,
    pub pedestrian_count: usize,
    pub trains_passed: u64,
    pub palette_name: String,
    pub signs_lit: bool,
}

/// Complete simulation state for *Shinjuku Rain*.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShinjukuRainState {
    pub seed: u64,
    pub rng: RainRng,
    pub time: f64,
    pub rain_intensity: f32,
    pub wind_gust: f32,
    pub target_wind: f32,
    pub gust_timer: f32,
    pub signs_lit: bool,
    pub lightning_flash: f32,
    pub next_lightning_secs: f32,
    pub palette: ShinjukuPalette,
    pub raindrops: Vec<Raindrop>,
    pub splashes: Vec<PuddleSplash>,
    pub pedestrians: Vec<Pedestrian>,
    pub vehicles: Vec<Vehicle>,
    pub train: Option<ElevatedTrain>,
    pub next_train_secs: f32,
    pub trains_passed: u64,
    pub cats: Vec<AlleyCat>,
    pub steam: Vec<SteamParticle>,
    pub next_id: u32,
}

impl ShinjukuRainState {
    /// Initialize a new rainy Shinjuku street with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut rng = RainRng::new(seed);
        let mut raindrops = Vec::with_capacity(140);
        for i in 0..140 {
            let layer = match i % 3 {
                0 => RainLayer::Far,
                1 => RainLayer::Mid,
                _ => RainLayer::Near,
            };
            let speed = match layer {
                RainLayer::Far => rng.range(520.0, 680.0),
                RainLayer::Mid => rng.range(720.0, 920.0),
                RainLayer::Near => rng.range(980.0, 1250.0),
            };
            raindrops.push(Raindrop {
                x: rng.range(0.0, 1000.0),
                y: rng.range(0.0, 950.0),
                speed,
                layer,
            });
        }

        let mut pedestrians = Vec::with_capacity(6);
        for i in 0..6 {
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            pedestrians.push(Pedestrian {
                id: i + 1,
                x: rng.range(60.0, 940.0),
                y: rng.range(755.0, 785.0),
                vx: dir * rng.range(24.0, 42.0),
                umbrella_color: (i % 5) as u8,
                pause_secs: 0.0,
                walk_phase: rng.range(0.0, std::f32::consts::TAU),
            });
        }

        let vehicles = vec![
            Vehicle {
                id: 100,
                x: 180.0,
                y: 865.0,
                vx: 145.0,
                kind: VehicleKind::Taxi,
            },
            Vehicle {
                id: 101,
                x: 820.0,
                y: 915.0,
                vx: -125.0,
                kind: VehicleKind::KeiTruck,
            },
        ];

        let cats = vec![
            AlleyCat {
                id: 200,
                x: 245.0,
                y: 775.0,
                facing_right: true,
                darting: false,
                timer: 4.5,
            },
            AlleyCat {
                id: 201,
                x: 715.0,
                y: 780.0,
                facing_right: false,
                darting: false,
                timer: 7.0,
            },
        ];

        Self {
            seed,
            rng,
            time: 0.0,
            rain_intensity: 1.0,
            wind_gust: 0.25,
            target_wind: 0.25,
            gust_timer: 6.0,
            signs_lit: true,
            lightning_flash: 0.0,
            next_lightning_secs: 14.0,
            palette: ShinjukuPalette::ShinjukuNeon,
            raindrops,
            splashes: Vec::with_capacity(64),
            pedestrians,
            vehicles,
            train: Some(ElevatedTrain {
                x: -180.0,
                vx: 230.0,
                car_count: 5,
            }),
            next_train_secs: 16.0,
            trains_passed: 0,
            cats,
            steam: Vec::with_capacity(24),
            next_id: 300,
        }
    }

    /// Advance the street simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32, _interactive: bool, _pointer: Option<[f32; 2]>) -> Activity {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt as f64;

        // Smooth wind gust modulation
        self.gust_timer -= dt;
        if self.gust_timer <= 0.0 {
            self.target_wind = self.rng.range(0.12, 0.72);
            self.gust_timer = self.rng.range(4.0, 9.0);
        }
        self.wind_gust += (self.target_wind - self.wind_gust) * (dt * 1.4).min(1.0);

        // Lightning decay & ambient distant storm timer
        if self.lightning_flash > 0.0 {
            self.lightning_flash = (self.lightning_flash - dt * 2.8).max(0.0);
        }
        self.next_lightning_secs -= dt;
        if self.next_lightning_secs <= 0.0 {
            self.lightning_flash = self.rng.range(0.55, 0.95);
            self.next_lightning_secs = self.rng.range(16.0, 34.0);
        }

        // Update falling raindrops & spawn puddle splashes
        let wind_vx = self.wind_gust * 260.0;
        let speed_scale = 0.75 + self.rain_intensity * 0.45;
        let mut new_splashes = Vec::new();
        for drop in &mut self.raindrops {
            let layer_wind = match drop.layer {
                RainLayer::Far => 0.65,
                RainLayer::Mid => 0.95,
                RainLayer::Near => 1.25,
            };
            drop.x += wind_vx * layer_wind * dt;
            drop.y += drop.speed * speed_scale * dt;

            if drop.x > 1030.0 {
                drop.x -= 1060.0;
            } else if drop.x < -30.0 {
                drop.x += 1060.0;
            }

            let ground_y = match drop.layer {
                RainLayer::Far => 775.0,
                RainLayer::Mid => 845.0,
                RainLayer::Near => 965.0,
            };

            if drop.y >= ground_y {
                if self.splashes.len() + new_splashes.len() < 72 && self.rng.f32() < 0.38 {
                    new_splashes.push(PuddleSplash {
                        x: drop.x.clamp(10.0, 990.0),
                        y: self.rng.range(765.0, 965.0),
                        age: 0.0,
                        max_age: self.rng.range(0.32, 0.62),
                        scale: match drop.layer {
                            RainLayer::Far => 0.7,
                            RainLayer::Mid => 1.0,
                            RainLayer::Near => 1.35,
                        },
                    });
                }
                drop.y = self.rng.range(-80.0, -5.0);
                drop.x = self.rng.range(-20.0, 1020.0);
            }
        }
        self.splashes.extend(new_splashes);

        // Age & prune puddle splashes
        self.splashes.retain_mut(|s| {
            s.age += dt;
            s.age < s.max_age
        });

        // Update pedestrians with umbrellas
        for ped in &mut self.pedestrians {
            if ped.pause_secs > 0.0 {
                ped.pause_secs -= dt;
                continue;
            }
            ped.x += ped.vx * dt;
            ped.walk_phase += dt * 5.0;

            // Occasional pause near ramen shop (x ~ 220) or vending machine (x ~ 670)
            if ((ped.x - 220.0).abs() < 4.0 || (ped.x - 670.0).abs() < 4.0)
                && self.rng.f32() < 0.015
            {
                ped.pause_secs = self.rng.range(2.0, 4.5);
            }

            if ped.x > 1040.0 {
                ped.x = -40.0;
            } else if ped.x < -40.0 {
                ped.x = 1040.0;
            }
        }

        // Update passing vehicles
        for v in &mut self.vehicles {
            v.x += v.vx * dt;
            if v.vx > 0.0 && v.x > 1180.0 {
                v.x = -self.rng.range(180.0, 420.0);
            } else if v.vx < 0.0 && v.x < -180.0 {
                v.x = self.rng.range(1120.0, 1380.0);
            }
        }

        // Update elevated train
        if let Some(ref mut t) = self.train {
            t.x += t.vx * dt;
            let train_len = (t.car_count as f32) * 110.0;
            if (t.vx > 0.0 && t.x > 1080.0) || (t.vx < 0.0 && t.x + train_len < -80.0) {
                self.train = None;
                self.trains_passed += 1;
                self.next_train_secs = self.rng.range(14.0, 28.0);
            }
        } else {
            self.next_train_secs -= dt;
            if self.next_train_secs <= 0.0 {
                self.dispatch_train();
            }
        }

        // Update alley cats
        for cat in &mut self.cats {
            cat.timer -= dt;
            if cat.darting {
                let speed = if cat.facing_right { 85.0 } else { -85.0 };
                cat.x += speed * dt;
                if cat.timer <= 0.0 || cat.x < 40.0 || cat.x > 960.0 {
                    cat.darting = false;
                    cat.x = cat.x.clamp(45.0, 955.0);
                    cat.timer = self.rng.range(5.0, 12.0);
                }
            } else if cat.timer <= 0.0 {
                cat.darting = true;
                cat.facing_right = self.rng.f32() < 0.5;
                cat.timer = self.rng.range(1.2, 2.6);
            }
        }

        // Spawn & age steam particles from ramen shop & street grate
        if self.steam.len() < 22 && self.rng.f32() < dt * 7.5 {
            let source_x = if self.rng.f32() < 0.65 { 215.0 } else { 540.0 };
            let source_y = if source_x < 300.0 { 700.0 } else { 785.0 };
            self.steam.push(SteamParticle {
                x: source_x + self.rng.range(-14.0, 14.0),
                y: source_y,
                age: 0.0,
                max_age: self.rng.range(1.6, 2.8),
                drift: self.rng.range(-12.0, 12.0),
            });
        }
        for s in &mut self.steam {
            s.age += dt;
            s.y -= 26.0 * dt;
            s.x += (s.drift + self.wind_gust * 45.0) * dt;
        }
        self.steam.retain(|s| s.age < s.max_age);

        Activity::Animating
    }

    /// Create a cluster of puddle splashes at normalized coordinates `(x, y)`.
    pub fn splash_at(&mut self, x: f32, y: f32) {
        let clamped_x = x.clamp(20.0, 980.0);
        let clamped_y = y.clamp(755.0, 965.0);
        for i in 0..5 {
            let ox = self.rng.range(-28.0, 28.0);
            let oy = self.rng.range(-12.0, 12.0);
            if self.splashes.len() < 96 {
                self.splashes.push(PuddleSplash {
                    x: (clamped_x + ox).clamp(15.0, 985.0),
                    y: (clamped_y + oy).clamp(750.0, 975.0),
                    age: (i as f32) * 0.04,
                    max_age: 0.65,
                    scale: 1.5,
                });
            }
        }
    }

    /// Trigger a strong wind gust that slants the rain and sways lanterns.
    pub fn trigger_wind_gust(&mut self) {
        self.target_wind = 0.92;
        self.wind_gust = 0.78;
        self.gust_timer = 4.5;
    }

    /// Dispatch an elevated commuter train across the overhead viaduct.
    pub fn dispatch_train(&mut self) {
        if self.train.is_some() {
            return;
        }
        let go_right = self.rng.f32() < 0.5;
        let car_count = if self.rng.f32() < 0.5 { 5 } else { 6 };
        let start_x = if go_right {
            -(car_count as f32) * 110.0 - 40.0
        } else {
            1040.0
        };
        let vx = if go_right { 245.0 } else { -245.0 };
        self.train = Some(ElevatedTrain {
            x: start_x,
            vx,
            car_count,
        });
    }

    /// Toggle the glowing neon storefront signs on or off.
    pub fn toggle_signs(&mut self) {
        self.signs_lit = !self.signs_lit;
    }

    /// Trigger a distant lightning flash across the rainy skyline.
    pub fn trigger_lightning(&mut self) {
        self.lightning_flash = 1.0;
        self.next_lightning_secs = self.rng.range(16.0, 32.0);
    }

    /// Cycle to the next curated ANSI palette.
    pub fn cycle_palette(&mut self) {
        self.palette = self.palette.next();
    }

    /// Set a specific ANSI palette.
    pub fn set_palette(&mut self, palette: ShinjukuPalette) {
        self.palette = palette;
    }

    /// Deterministic catch-up when resuming from sleep or offline time.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let clamped = elapsed_secs.clamp(0.0, 3600.0);
        let steps = (clamped / 0.1).min(60.0) as usize;
        for _ in 0..steps {
            self.update(0.1, false, None);
        }
    }

    /// Return current live telemetry stats.
    pub fn stats(&self) -> ShinjukuStats {
        let rain_rate_mm_hr = (42.0 * self.rain_intensity + self.wind_gust * 18.0).round() as u32;
        ShinjukuStats {
            rain_rate_mm_hr,
            pedestrian_count: self.pedestrians.len(),
            trains_passed: self.trains_passed,
            palette_name: self.palette.name().to_string(),
            signs_lit: self.signs_lit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shinjuku_seed_determinism() {
        let mut a = ShinjukuRainState::new(42);
        let mut b = ShinjukuRainState::new(42);
        for _ in 0..30 {
            a.update(1.0 / 30.0, false, None);
            b.update(1.0 / 30.0, false, None);
        }
        assert_eq!(a.raindrops, b.raindrops);
        assert_eq!(a.pedestrians, b.pedestrians);
    }

    #[test]
    fn shinjuku_interactions_work() {
        let mut s = ShinjukuRainState::new(7);
        s.splash_at(500.0, 850.0);
        assert!(!s.splashes.is_empty());

        s.trigger_wind_gust();
        assert!(s.wind_gust > 0.7);

        let was_lit = s.signs_lit;
        s.toggle_signs();
        assert_ne!(s.signs_lit, was_lit);

        s.trigger_lightning();
        assert!(s.lightning_flash > 0.9);

        let p0 = s.palette;
        s.cycle_palette();
        assert_ne!(s.palette, p0);
    }

    #[test]
    fn shinjuku_on_wake_advances_time() {
        let mut s = ShinjukuRainState::new(99);
        let t0 = s.time;
        s.on_wake(10.0);
        assert!(s.time > t0);
    }
}
