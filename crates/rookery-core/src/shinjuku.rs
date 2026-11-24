//! Deterministic simulation state for *Shinjuku Rain* (`shinjuku-rain`).
//!
//! Models a heavy-rain Japanese city street in normalized `[0..1000, 0..1000]`
//! world coordinates: wind-slanted multi-depth rain, awning drips, puddle splashes,
//! pedestrians with umbrellas, passing taxis/kei trucks/delivery scooters obeying
//! a crosswalk traffic signal, an elevated commuter train with pantograph sparks,
//! alley cats, rising ramen-shop steam, and localized forked lightning.

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

/// Droplet dripping from a storefront awning or viaduct edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AwningDrip {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
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
    #[serde(default)]
    pub coat_color: u8,
    pub pause_secs: f32,
    pub walk_phase: f32,
}

/// Type of vehicle cruising the wet street.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleKind {
    Taxi,
    KeiTruck,
    Scooter,
}

/// Passing vehicle on the asphalt street.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    #[serde(default)]
    pub target_vx: f32,
    #[serde(default)]
    pub brake_glow: f32,
    pub kind: VehicleKind,
}

/// Crosswalk traffic signal phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficLightState {
    #[default]
    Green,
    Yellow,
    Red,
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

/// Outcome of a context-aware click interaction in the street.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShinjukuInteraction {
    Lightning,
    Train,
    Storefront,
    Vending,
    TrafficSignal,
    Cat,
    Splash,
}

/// Live telemetry summary for the popover HUD.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShinjukuStats {
    pub rain_rate_mm_hr: u32,
    pub weather_label: String,
    pub pedestrian_count: usize,
    pub trains_passed: u64,
    pub palette_name: String,
    pub signs_lit: bool,
}

fn default_palette_blend() -> f32 {
    1.0
}

fn default_lightning_x() -> f32 {
    500.0
}

fn default_traffic_timer() -> f32 {
    11.0
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
    #[serde(default = "default_lightning_x")]
    pub lightning_x: f32,
    #[serde(default)]
    pub lightning_seed: u32,
    pub next_lightning_secs: f32,
    pub palette: ShinjukuPalette,
    #[serde(default)]
    pub prev_palette: ShinjukuPalette,
    /// `1.0` = settled on `palette`, `0.0..1.0` = smoothly cross-fading from `prev_palette`.
    #[serde(default = "default_palette_blend")]
    pub palette_blend: f32,
    /// Continuous palette color shift rate (`0.0` = manual/off, up to `3.0`x).
    #[serde(default)]
    pub color_shift_rate: f32,
    #[serde(default)]
    pub traffic_light: TrafficLightState,
    #[serde(default = "default_traffic_timer")]
    pub traffic_timer: f32,
    #[serde(default)]
    pub train_spark_flash: f32,
    #[serde(default = "default_lightning_x")]
    pub train_spark_x: f32,
    #[serde(default)]
    pub ramen_steam_boost: f32,
    #[serde(default)]
    pub vending_pulse: f32,
    #[serde(default)]
    pub bar_flicker_boost: f32,
    #[serde(default)]
    pub konbini_pulse: f32,
    pub raindrops: Vec<Raindrop>,
    #[serde(default)]
    pub drips: Vec<AwningDrip>,
    pub splashes: Vec<PuddleSplash>,
    pub pedestrians: Vec<Pedestrian>,
    pub vehicles: Vec<Vehicle>,
    pub train: Option<ElevatedTrain>,
    pub next_train_secs: f32,
    pub trains_passed: u64,
    pub cats: Vec<AlleyCat>,
    pub steam: Vec<SteamParticle>,
    pub next_id: u32,
    #[serde(skip)]
    pub pointer_pos: Option<[f32; 2]>,
    #[serde(skip)]
    pub pointer_active: bool,
    #[serde(skip)]
    pub ambient_train_sfx: bool,
    #[serde(skip)]
    pub ambient_thunder_sfx: bool,
}

impl ShinjukuRainState {
    /// Initialize a new rainy Shinjuku street with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut rng = RainRng::new(seed);
        let mut raindrops = Vec::with_capacity(260);
        for i in 0..260 {
            let layer = match i % 3 {
                0 => RainLayer::Far,
                1 => RainLayer::Mid,
                _ => RainLayer::Near,
            };
            let speed = match layer {
                RainLayer::Far => rng.range(520.0, 680.0),
                RainLayer::Mid => rng.range(740.0, 960.0),
                RainLayer::Near => rng.range(1020.0, 1320.0),
            };
            raindrops.push(Raindrop {
                x: rng.range(0.0, 1000.0),
                y: rng.range(0.0, 960.0),
                speed,
                layer,
            });
        }

        let mut pedestrians = Vec::with_capacity(7);
        for i in 0..7 {
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            pedestrians.push(Pedestrian {
                id: i + 1,
                x: rng.range(50.0, 950.0),
                y: rng.range(734.0, 750.0),
                vx: dir * rng.range(24.0, 44.0),
                umbrella_color: (i % 5) as u8,
                coat_color: ((i + 2) % 4) as u8,
                pause_secs: 0.0,
                walk_phase: rng.range(0.0, std::f32::consts::TAU),
            });
        }

        let vehicles = vec![
            Vehicle {
                id: 100,
                x: 180.0,
                y: 842.0,
                vx: 150.0,
                target_vx: 150.0,
                brake_glow: 0.0,
                kind: VehicleKind::Taxi,
            },
            Vehicle {
                id: 101,
                x: 820.0,
                y: 912.0,
                vx: -125.0,
                target_vx: -125.0,
                brake_glow: 0.0,
                kind: VehicleKind::KeiTruck,
            },
            Vehicle {
                id: 102,
                x: -240.0,
                y: 864.0,
                vx: 175.0,
                target_vx: 175.0,
                brake_glow: 0.0,
                kind: VehicleKind::Scooter,
            },
        ];

        let cats = vec![
            AlleyCat {
                id: 200,
                x: 195.0,
                y: 744.0,
                facing_right: true,
                darting: false,
                timer: 4.5,
            },
            AlleyCat {
                id: 201,
                x: 745.0,
                y: 746.0,
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
            wind_gust: 0.28,
            target_wind: 0.28,
            gust_timer: 6.0,
            signs_lit: true,
            lightning_flash: 0.0,
            lightning_x: 520.0,
            lightning_seed: 1,
            next_lightning_secs: 14.0,
            palette: ShinjukuPalette::ShinjukuNeon,
            prev_palette: ShinjukuPalette::ShinjukuNeon,
            palette_blend: 1.0,
            color_shift_rate: 0.0,
            traffic_light: TrafficLightState::Green,
            traffic_timer: 12.0,
            train_spark_flash: 0.0,
            train_spark_x: 500.0,
            ramen_steam_boost: 0.0,
            vending_pulse: 0.0,
            bar_flicker_boost: 0.0,
            konbini_pulse: 0.0,
            raindrops,
            drips: Vec::with_capacity(20),
            splashes: Vec::with_capacity(80),
            pedestrians,
            vehicles,
            train: Some(ElevatedTrain {
                x: -160.0,
                vx: 235.0,
                car_count: 5,
            }),
            next_train_secs: 15.0,
            trains_passed: 0,
            cats,
            steam: Vec::with_capacity(32),
            next_id: 300,
            pointer_pos: None,
            pointer_active: false,
            ambient_train_sfx: false,
            ambient_thunder_sfx: false,
        }
    }

    /// Number of active raindrops scaled by `rain_intensity`.
    pub fn active_raindrop_count(&self) -> usize {
        let frac = ((self.rain_intensity - 0.1) / 2.4).clamp(0.12, 1.0);
        ((self.raindrops.len() as f32) * frac)
            .round()
            .clamp(28.0, self.raindrops.len() as f32) as usize
    }

    /// Human-readable weather classification for the current `rain_intensity`.
    pub fn weather_label(&self) -> &'static str {
        if self.rain_intensity < 0.55 {
            "Light Drizzle"
        } else if self.rain_intensity < 1.15 {
            "Steady Rain"
        } else if self.rain_intensity < 1.85 {
            "Heavy Downpour"
        } else {
            "Typhoon Squall"
        }
    }

    /// Advance the street simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32, interactive: bool, pointer: Option<[f32; 2]>) -> Activity {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt as f64;
        self.pointer_active = interactive && pointer.is_some();
        let prev_pointer = self.pointer_pos;
        self.pointer_pos = if interactive { pointer } else { None };

        // Decay interactive storefront pulses & train sparks
        self.train_spark_flash = (self.train_spark_flash - dt * 6.5).max(0.0);
        self.ramen_steam_boost = (self.ramen_steam_boost - dt * 0.65).max(0.0);
        self.vending_pulse = (self.vending_pulse - dt * 1.1).max(0.0);
        self.bar_flicker_boost = (self.bar_flicker_boost - dt * 1.2).max(0.0);
        self.konbini_pulse = (self.konbini_pulse - dt * 0.9).max(0.0);

        // Smooth palette crossfade & continuous color shift cycling
        if self.color_shift_rate > 0.0 {
            if self.palette_blend >= 1.0 {
                self.prev_palette = self.palette;
                self.palette = self.palette.next();
                self.palette_blend = 0.0;
            }
            let rate = 0.05 * self.color_shift_rate.clamp(0.0, 3.0);
            self.palette_blend += dt * rate;
            while self.palette_blend >= 1.0 {
                self.palette_blend -= 1.0;
                self.prev_palette = self.palette;
                self.palette = self.palette.next();
            }
        } else if self.palette_blend < 1.0 {
            self.palette_blend = (self.palette_blend + dt * 0.72).min(1.0);
        }

        // Smooth wind gust modulation (stronger base gusts during heavy rain)
        self.gust_timer -= dt;
        if self.gust_timer <= 0.0 {
            let storm_bias = ((self.rain_intensity - 0.8).max(0.0) * 0.22).min(0.35);
            self.target_wind = (self.rng.range(0.14, 0.72) + storm_bias).min(0.98);
            self.gust_timer = self.rng.range(4.0, 9.0);
        }
        self.wind_gust += (self.target_wind - self.wind_gust) * (dt * 1.4).min(1.0);

        // Crosswalk traffic signal cycle
        self.traffic_timer -= dt;
        if self.traffic_timer <= 0.0 {
            match self.traffic_light {
                TrafficLightState::Green => {
                    self.traffic_light = TrafficLightState::Yellow;
                    self.traffic_timer = 2.4;
                }
                TrafficLightState::Yellow => {
                    self.traffic_light = TrafficLightState::Red;
                    self.traffic_timer = 7.5;
                }
                TrafficLightState::Red => {
                    self.traffic_light = TrafficLightState::Green;
                    self.traffic_timer = 13.0;
                }
            }
        }

        // Lightning decay & ambient distant storm timer
        if self.lightning_flash > 0.0 {
            self.lightning_flash = (self.lightning_flash - dt * 2.6).max(0.0);
        }
        self.next_lightning_secs -= dt * (0.7 + self.rain_intensity * 0.4);
        if self.next_lightning_secs <= 0.0 {
            self.lightning_flash = self.rng.range(0.62, 0.96);
            self.lightning_x = self.rng.range(160.0, 860.0);
            self.lightning_seed = self.lightning_seed.wrapping_add(1).max(1);
            self.next_lightning_secs = self.rng.range(15.0, 32.0);
            self.ambient_thunder_sfx = true;
        }

        // Pointer puddle ripple wake when moving across wet asphalt
        if let (Some([px, py]), Some([prev_x, prev_y])) = (self.pointer_pos, prev_pointer) {
            let moved = (px - prev_x).hypot(py - prev_y);
            if py > 760.0 && moved > 6.0 && self.splashes.len() < 92 && self.rng.f32() < 0.45 {
                self.splashes.push(PuddleSplash {
                    x: px.clamp(15.0, 985.0),
                    y: py.clamp(765.0, 975.0),
                    age: 0.0,
                    max_age: 0.58,
                    scale: 1.35,
                });
            }
        }

        // Update falling raindrops, invisible pointer umbrella deflection & puddle splashes
        let wind_vx = self.wind_gust * 280.0;
        let speed_scale = 0.72 + self.rain_intensity * 0.42;
        let active_drops = self.active_raindrop_count();
        let splash_chance = (0.28 + self.rain_intensity * 0.16).min(0.65);
        let mut new_splashes = Vec::new();
        for drop in self.raindrops.iter_mut().take(active_drops) {
            let layer_wind = match drop.layer {
                RainLayer::Far => 0.65,
                RainLayer::Mid => 0.95,
                RainLayer::Near => 1.28,
            };
            drop.x += wind_vx * layer_wind * dt;
            drop.y += drop.speed * speed_scale * dt;

            // Invisible umbrella deflection around interactive cursor
            if let Some([px, py]) = self.pointer_pos {
                let dx = drop.x - px;
                let dy = drop.y - py;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < 54.0 * 54.0 && dy > -45.0 && dy < 24.0 {
                    let push = if dx >= 0.0 { 1.0 } else { -1.0 };
                    drop.x += push * 280.0 * dt;
                }
            }

            if drop.x > 1030.0 {
                drop.x -= 1060.0;
            } else if drop.x < -30.0 {
                drop.x += 1060.0;
            }

            let ground_y = match drop.layer {
                RainLayer::Far => 765.0,
                RainLayer::Mid => 845.0,
                RainLayer::Near => 965.0,
            };

            if drop.y >= ground_y {
                if self.splashes.len() + new_splashes.len() < 84 && self.rng.f32() < splash_chance {
                    new_splashes.push(PuddleSplash {
                        x: drop.x.clamp(10.0, 990.0),
                        y: self.rng.range(768.0, 968.0),
                        age: 0.0,
                        max_age: self.rng.range(0.32, 0.64),
                        scale: match drop.layer {
                            RainLayer::Far => 0.7,
                            RainLayer::Mid => 1.0,
                            RainLayer::Near => 1.35,
                        },
                    });
                }
                drop.y = self.rng.range(-85.0, -5.0);
                drop.x = self.rng.range(-20.0, 1020.0);
            }
        }
        self.splashes.extend(new_splashes);

        // Awning drips from Ramen shop & Konbini overhangs
        let drip_spawn_rate = dt * (4.0 + self.rain_intensity * 4.0);
        if self.drips.len() < 18 && self.rng.f32() < drip_spawn_rate {
            let x = if self.rng.f32() < 0.55 {
                self.rng.range(95.0, 265.0)
            } else {
                self.rng.range(785.0, 955.0)
            };
            self.drips.push(AwningDrip {
                x,
                y: 668.0,
                vy: self.rng.range(160.0, 220.0),
            });
        }
        let mut drip_splashes = Vec::new();
        self.drips.retain_mut(|d| {
            d.vy += 420.0 * dt;
            d.y += d.vy * dt;
            if d.y >= 756.0 {
                if self.splashes.len() + drip_splashes.len() < 92 {
                    drip_splashes.push(PuddleSplash {
                        x: d.x,
                        y: 766.0,
                        age: 0.0,
                        max_age: 0.36,
                        scale: 0.85,
                    });
                }
                false
            } else {
                true
            }
        });
        self.splashes.extend(drip_splashes);

        // Age & prune puddle splashes
        self.splashes.retain_mut(|s| {
            s.age += dt;
            s.age < s.max_age
        });

        // Update pedestrians with umbrellas on the sidewalk
        for ped in &mut self.pedestrians {
            if ped.pause_secs > 0.0 {
                ped.pause_secs -= dt;
                continue;
            }
            ped.x += ped.vx * dt;
            ped.walk_phase += dt * 5.2;

            // Occasional pause near ramen shop (x ~ 180), vending machine (x ~ 665), or konbini (x ~ 865)
            if ((ped.x - 180.0).abs() < 4.0
                || (ped.x - 665.0).abs() < 4.0
                || (ped.x - 865.0).abs() < 4.0)
                && self.rng.f32() < 0.018
            {
                ped.pause_secs = self.rng.range(2.0, 4.5);
            }

            if ped.x > 1040.0 {
                ped.x = -40.0;
            } else if ped.x < -40.0 {
                ped.x = 1040.0;
            }
        }

        // Update passing vehicles (obey crosswalk red/yellow signal + spawn tire spray)
        let stop_for_signal = matches!(
            self.traffic_light,
            TrafficLightState::Red | TrafficLightState::Yellow
        );
        for v in &mut self.vehicles {
            if v.target_vx.abs() < 1.0 {
                v.target_vx = if v.vx >= 0.0 { 145.0 } else { -125.0 };
            }
            let approaching_crosswalk = if v.target_vx > 0.0 {
                v.x > 475.0 && v.x < 575.0
            } else {
                v.x < 775.0 && v.x > 675.0
            };

            let desired_vx = if stop_for_signal && approaching_crosswalk {
                0.0
            } else {
                v.target_vx
            };

            let accel = if desired_vx.abs() < v.vx.abs() {
                3.8
            } else {
                2.2
            };
            v.vx += (desired_vx - v.vx) * (dt * accel).min(1.0);
            if v.vx.abs() < 2.0 && desired_vx == 0.0 {
                v.vx = 0.0;
            }
            let braking = desired_vx == 0.0 || (v.vx.abs() < v.target_vx.abs() * 0.72);
            let target_brake = if braking { 1.0 } else { 0.0 };
            v.brake_glow += (target_brake - v.brake_glow) * (dt * 6.0).min(1.0);

            v.x += v.vx * dt;

            // Wet tire wake spray behind fast-moving vehicles
            if v.vx.abs() > 70.0 && self.splashes.len() < 90 && self.rng.f32() < dt * 5.0 {
                let rear_x = if v.vx > 0.0 { v.x - 42.0 } else { v.x + 42.0 };
                if (15.0..=985.0).contains(&rear_x) {
                    self.splashes.push(PuddleSplash {
                        x: rear_x,
                        y: (v.y + 10.0).clamp(770.0, 970.0),
                        age: 0.0,
                        max_age: 0.34,
                        scale: 0.9,
                    });
                }
            }

            if v.target_vx > 0.0 && v.x > 1180.0 {
                v.x = -self.rng.range(180.0, 420.0);
            } else if v.target_vx < 0.0 && v.x < -180.0 {
                v.x = self.rng.range(1120.0, 1380.0);
            }
        }

        // Update elevated train & occasional overhead pantograph sparks
        if let Some(ref mut t) = self.train {
            t.x += t.vx * dt;
            let train_len = (t.car_count as f32) * 110.0;
            if self.train_spark_flash <= 0.0 && self.rng.f32() < dt * (0.8 + self.rain_intensity) {
                let spark_x = t.x + self.rng.range(40.0, (train_len - 40.0).max(50.0));
                if (40.0..=960.0).contains(&spark_x) {
                    self.train_spark_flash = 1.0;
                    self.train_spark_x = spark_x;
                }
            }
            if (t.vx > 0.0 && t.x > 1080.0) || (t.vx < 0.0 && t.x + train_len < -80.0) {
                self.train = None;
                self.trains_passed += 1;
                self.next_train_secs = self.rng.range(13.0, 26.0);
            }
        } else {
            self.next_train_secs -= dt;
            if self.next_train_secs <= 0.0 {
                self.dispatch_train();
                self.ambient_train_sfx = true;
            }
        }

        // Update alley cats (shelter more during heavy downpours, startle on close pointer)
        for cat in &mut self.cats {
            if let Some([px, py]) = self.pointer_pos
                && !cat.darting
                && (px - cat.x).hypot(py - cat.y) < 42.0
            {
                cat.darting = true;
                cat.facing_right = cat.x >= px;
                cat.timer = 1.4;
            }
            cat.timer -= dt;
            if cat.darting {
                let speed = if cat.facing_right { 90.0 } else { -90.0 };
                cat.x += speed * dt;
                if cat.timer <= 0.0 || cat.x < 45.0 || cat.x > 955.0 {
                    cat.darting = false;
                    cat.x = cat.x.clamp(45.0, 955.0);
                    let shelter_bonus = if self.rain_intensity > 1.4 { 5.0 } else { 0.0 };
                    cat.timer = self.rng.range(5.0, 12.0) + shelter_bonus;
                }
            } else if cat.timer <= 0.0 {
                cat.darting = true;
                cat.facing_right = self.rng.f32() < 0.5;
                cat.timer = self.rng.range(1.2, 2.5);
            }
        }

        // Spawn & age steam particles from ramen shop & street grate
        let steam_rate = dt * (8.5 + self.ramen_steam_boost * 18.0);
        if self.steam.len() < 30 && self.rng.f32() < steam_rate {
            let from_ramen = self.ramen_steam_boost > 0.1 || self.rng.f32() < 0.68;
            let source_x = if from_ramen { 185.0 } else { 545.0 };
            let source_y = if from_ramen { 668.0 } else { 756.0 };
            self.steam.push(SteamParticle {
                x: source_x + self.rng.range(-18.0, 18.0),
                y: source_y,
                age: 0.0,
                max_age: self.rng.range(1.6, 2.8),
                drift: self.rng.range(-12.0, 12.0),
            });
        }
        for s in &mut self.steam {
            s.age += dt;
            s.y -= 28.0 * dt;
            s.x += (s.drift + self.wind_gust * 46.0) * dt;
        }
        self.steam.retain(|s| s.age < s.max_age);

        Activity::Animating
    }

    /// Context-aware click handler across sky, viaduct, storefronts, cats, signal, and wet street.
    pub fn interact_at(&mut self, x: f32, y: f32) -> ShinjukuInteraction {
        // 1. Sky -> localized forked lightning
        if y < 255.0 {
            self.trigger_lightning_at(x);
            return ShinjukuInteraction::Lightning;
        }
        // 2. Viaduct -> dispatch train or pantograph spark
        if (255.0..=355.0).contains(&y) {
            if self.train.is_none() {
                self.dispatch_train();
            } else {
                self.train_spark_flash = 1.0;
                self.train_spark_x = x.clamp(60.0, 940.0);
            }
            return ShinjukuInteraction::Train;
        }
        // 3. Alley cats -> playful dart
        for cat in &mut self.cats {
            if (x - cat.x).hypot(y - cat.y) < 38.0 {
                cat.darting = true;
                cat.facing_right = x < cat.x;
                cat.timer = 1.8;
                return ShinjukuInteraction::Cat;
            }
        }
        // 4. Crosswalk traffic signal post (x ~ 580..625)
        if (565.0..=625.0).contains(&x) && (560.0..=760.0).contains(&y) {
            self.toggle_traffic_light();
            return ShinjukuInteraction::TrafficSignal;
        }
        // 5. Storefronts on the sidewalk
        if (520.0..=760.0).contains(&y) {
            if (70.0..=295.0).contains(&x) {
                self.ramen_steam_boost = 1.0;
                self.wind_gust = (self.wind_gust + 0.25).min(0.95);
                return ShinjukuInteraction::Storefront;
            }
            if (365.0..=555.0).contains(&x) {
                self.bar_flicker_boost = 1.0;
                return ShinjukuInteraction::Storefront;
            }
            if (625.0..=755.0).contains(&x) {
                self.vending_pulse = 1.0;
                return ShinjukuInteraction::Vending;
            }
            if (770.0..=970.0).contains(&x) {
                self.konbini_pulse = 1.0;
                return ShinjukuInteraction::Storefront;
            }
        }
        // 6. Wet street -> heavy puddle splash cluster
        self.splash_at(x, y);
        ShinjukuInteraction::Splash
    }

    /// Create a cluster of puddle splashes at normalized coordinates `(x, y)`.
    pub fn splash_at(&mut self, x: f32, y: f32) {
        let clamped_x = x.clamp(20.0, 980.0);
        let clamped_y = y.clamp(765.0, 965.0);
        for i in 0..5 {
            let ox = self.rng.range(-28.0, 28.0);
            let oy = self.rng.range(-12.0, 12.0);
            if self.splashes.len() < 104 {
                self.splashes.push(PuddleSplash {
                    x: (clamped_x + ox).clamp(15.0, 985.0),
                    y: (clamped_y + oy).clamp(762.0, 975.0),
                    age: (i as f32) * 0.04,
                    max_age: 0.68,
                    scale: 1.55,
                });
            }
        }
    }

    /// Set the rain intensity multiplier (`0.2` drizzle to `2.5` typhoon).
    pub fn set_rain_intensity(&mut self, intensity: f32) {
        self.rain_intensity = intensity.clamp(0.2, 2.5);
    }

    /// Toggle the crosswalk traffic signal between Green and Red.
    pub fn toggle_traffic_light(&mut self) {
        self.traffic_light = match self.traffic_light {
            TrafficLightState::Green => {
                self.traffic_timer = 8.0;
                TrafficLightState::Red
            }
            TrafficLightState::Yellow | TrafficLightState::Red => {
                self.traffic_timer = 13.0;
                TrafficLightState::Green
            }
        };
    }

    /// Trigger a strong wind gust that slants the rain and sways lanterns.
    pub fn trigger_wind_gust(&mut self) {
        self.target_wind = 0.94;
        self.wind_gust = 0.80;
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
        let x = self.rng.range(180.0, 820.0);
        self.trigger_lightning_at(x);
    }

    /// Trigger a localized lightning flash and forked bolt at normalized `x`.
    pub fn trigger_lightning_at(&mut self, x: f32) {
        self.lightning_flash = 1.0;
        self.lightning_x = x.clamp(100.0, 900.0);
        self.lightning_seed = self.lightning_seed.wrapping_add(1).max(1);
        self.next_lightning_secs = self.rng.range(16.0, 32.0);
    }

    /// Cycle to the next curated ANSI palette.
    pub fn cycle_palette(&mut self) {
        let next = self.palette.next();
        self.set_palette(next);
    }

    /// Set a specific ANSI palette with a smooth crossfade.
    pub fn set_palette(&mut self, palette: ShinjukuPalette) {
        if palette == self.palette {
            return;
        }
        self.prev_palette = self.palette;
        self.palette = palette;
        self.palette_blend = 0.0;
    }

    /// Set the continuous palette color shift rate (`0.0` = manual/off, up to `3.0`x).
    pub fn set_color_shift_rate(&mut self, rate: f32) {
        self.color_shift_rate = rate.clamp(0.0, 3.0);
    }

    /// Deterministic catch-up when resuming from sleep or offline time.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        let clamped = elapsed_secs.clamp(0.0, 3600.0);
        let steps = (clamped / 0.1).min(60.0) as usize;
        for _ in 0..steps {
            self.update(0.1, false, None);
        }
        self.ambient_train_sfx = false;
        self.ambient_thunder_sfx = false;
    }

    /// Return current live telemetry stats.
    pub fn stats(&self) -> ShinjukuStats {
        let rain_rate_mm_hr = (42.0 * self.rain_intensity + self.wind_gust * 18.0).round() as u32;
        ShinjukuStats {
            rain_rate_mm_hr,
            weather_label: self.weather_label().to_string(),
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

        s.set_rain_intensity(2.2);
        assert_eq!(s.weather_label(), "Typhoon Squall");
        assert!(s.active_raindrop_count() > 200);

        assert_eq!(s.interact_at(450.0, 120.0), ShinjukuInteraction::Lightning);
        assert_eq!(s.interact_at(180.0, 680.0), ShinjukuInteraction::Storefront);
        assert_eq!(s.interact_at(690.0, 680.0), ShinjukuInteraction::Vending);
        assert_eq!(
            s.interact_at(595.0, 680.0),
            ShinjukuInteraction::TrafficSignal
        );
        assert_eq!(s.interact_at(400.0, 880.0), ShinjukuInteraction::Splash);
    }

    #[test]
    fn shinjuku_traffic_signal_brakes_vehicles() {
        let mut s = ShinjukuRainState::new(11);
        s.traffic_light = TrafficLightState::Red;
        s.traffic_timer = 10.0;
        s.vehicles[0].x = 510.0;
        s.vehicles[0].vx = 150.0;
        s.vehicles[0].target_vx = 150.0;

        for _ in 0..30 {
            s.update(0.05, false, None);
        }
        assert!(s.vehicles[0].vx < 20.0);
        assert!(s.vehicles[0].brake_glow > 0.5);
    }

    #[test]
    fn shinjuku_on_wake_advances_time() {
        let mut s = ShinjukuRainState::new(99);
        let t0 = s.time;
        s.on_wake(10.0);
        assert!(s.time > t0);
    }

    #[test]
    fn shinjuku_color_shift_rate_cycles_smoothly() {
        let mut s = ShinjukuRainState::new(42);
        assert_eq!(s.color_shift_rate, 0.0);
        assert_eq!(s.palette, ShinjukuPalette::ShinjukuNeon);

        // At 0.0x, palette stays settled.
        for _ in 0..20 {
            s.update(0.1, false, None);
        }
        assert_eq!(s.palette, ShinjukuPalette::ShinjukuNeon);
        assert!((s.palette_blend - 1.0).abs() < 1e-4);

        // At 2.0x (rate = 0.10/s), 5s advances halfway through KyotoGion transition.
        s.set_color_shift_rate(2.0);
        for _ in 0..50 {
            s.update(0.1, false, None);
        }
        assert_eq!(s.prev_palette, ShinjukuPalette::ShinjukuNeon);
        assert_eq!(s.palette, ShinjukuPalette::KyotoGion);
        assert!((s.palette_blend - 0.5).abs() < 1e-3);
    }
}
