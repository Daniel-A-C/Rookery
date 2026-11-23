//! Core data types, palettes, and entity definitions for *Gothic Metropolis*.

use serde::{Deserialize, Serialize};

/// Display names for the 4 curated gothic palettes in Gothic Metropolis.
pub const GOTHIC_PALETTE_NAMES: [&str; 4] = [
    "1989 Furst Amber & Obsidian",
    "Returns Ice & Gargoyle Slate",
    "Crimson Noir & Brass",
    "Axis Chemical Verdigris",
];

/// Curated gothic color palettes for Gothic Metropolis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GothicPalette {
    #[default]
    GothamAmber = 0,
    CathedralMoonlight = 1,
    CrimsonDeco = 2,
    EmeraldChem = 3,
}

impl GothicPalette {
    pub fn from_u8(v: u8) -> Self {
        match v % 4 {
            0 => Self::GothamAmber,
            1 => Self::CathedralMoonlight,
            2 => Self::CrimsonDeco,
            _ => Self::EmeraldChem,
        }
    }

    pub fn next(self) -> Self {
        Self::from_u8((self as u8) + 1)
    }

    pub fn name(self) -> &'static str {
        GOTHIC_PALETTE_NAMES[self as usize]
    }
}

/// Deterministic xorshift64* PRNG for reproducible metropolis dynamics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GothicRng {
    state: u64,
}

impl GothicRng {
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

/// Modular building archetypes inspired by 1990s Batman / Anton Furst gothic architecture.
///
/// New building types can be added here and placed in [`CityBlueprint`] to expand or evolve
/// the city over time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingArchetype {
    /// Towering Neo-Gothic cathedral keep with twin belfry towers, flying buttresses,
    /// pointed arch recesses, and a needle central spire.
    CathedralKeep,
    /// Exaggerated Art-Deco stepped ziggurat skyscraper with deep vertical ribs,
    /// setback terraces, eagle/gargoyle ledges, and a crown beacon.
    ArtDecoZiggurat,
    /// Slender gothic clockwork belfry with glowing clock faces on all 4 sides,
    /// arched louver vents, and a steep pyramidal roof.
    ClockworkBelfry,
    /// Heavy buttressed industrial foundry with glowing furnace slits, boiler tanks,
    /// and twin smokestacks emitting rising steam plumes.
    IndustrialFoundry,
    /// Brutalist-gothic corporate monolith with cantilevered skybridge portals,
    /// heavy cornice ledges, and rooftop searchlight batteries.
    SyndicateTower,
    /// Multi-level vaulted transit station that the elevated train viaduct passes
    /// directly through at `y = 2.6`, topped by a gothic tower.
    ViaductStation,
    /// Eight-sided octagonal gothic rotunda keep with 45-degree chamfered drum tiers,
    /// diagonal buttress fins, and a pointed octagonal spire.
    OctagonalCitadel,
    /// Open old-world gothic civic piazza with corner colonnade pavilions, glowing
    /// braziers, a stepped octagonal fountain dais, and a central monument obelisk.
    GothicMonumentPlaza,
    /// Soaring slender Art-Deco supertall monolith with continuous vertical recessed
    /// light strips, cantilevered titan/gargoyle shoulder buttresses, and a sunburst spire.
    AtlasDecoTower,
    /// Asymmetric twin-spire gothic chancery citadel with an integral high-altitude
    /// vaulted sky-arch bridge joining its twin towers, stepped gables, and lancet windows.
    ChanceryArcadeKeep,
    /// Brooding stepped gothic-industrial bastion crowned by an 8-sided ribbed copper
    /// observatory dome, glowing astronomical aperture, and corner lightning-coil pinnacles.
    AlchemicalObservatory,
}

/// A single building placement in the city blueprint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingPlacement {
    pub id: u32,
    pub archetype: BuildingArchetype,
    /// World X center coordinate (in city units, roughly `-10.0 .. 10.0`).
    pub x: f32,
    /// World Z center coordinate (in city units, roughly `-10.0 .. 10.0`).
    pub z: f32,
    /// Base X width of the building footprint.
    pub width: f32,
    /// Base Z depth of the building footprint.
    pub depth: f32,
    /// Primary structural height (excluding top needle spires).
    pub height: f32,
    /// Quarter-turn orientation (`0..=3`, i.e. `0°`, `90°`, `180°`, `270°`).
    pub quarter_turns: u8,
    /// Deterministic architectural variation seed for window lighting & trim.
    pub style_variant: u8,
}

impl BuildingPlacement {
    /// Return the solid axis-aligned `(min_x, max_x, min_z, max_z)` structural facade bounds
    /// of this building at vertical elevation `y`.
    ///
    /// Accounts for each archetype's tier setbacks, frustum tapers, and octagonal drums so
    /// elevated walkways and skybridge vestibules anchor flush into the actual 3D facade at `y`.
    pub fn cross_section_bounds_at(&self, y: f32) -> Option<(f32, f32, f32, f32)> {
        if y < 0.0 || y > self.height {
            return None;
        }
        let h = self.height.max(0.5);
        let y_frac = (y / h).clamp(0.0, 1.0);
        let (w_eff, d_eff) = match self.archetype {
            BuildingArchetype::CathedralKeep => {
                if y_frac < 0.34 {
                    (self.width, self.depth)
                } else {
                    (self.width * 0.72, self.depth * 0.72)
                }
            }
            BuildingArchetype::ArtDecoZiggurat => {
                let scale = if y_frac < 0.42 {
                    1.00
                } else if y_frac < 0.70 {
                    0.78
                } else if y_frac < 0.88 {
                    0.56
                } else {
                    0.36
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::ClockworkBelfry => {
                let scale = if y_frac < 0.74 {
                    0.86
                } else if y_frac < 0.92 {
                    0.96
                } else {
                    0.86
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::IndustrialFoundry => {
                let scale = if y_frac < 0.62 {
                    let t = (y_frac / 0.62).clamp(0.0, 1.0);
                    1.04 - 0.14 * t
                } else if y_frac < 0.86 {
                    0.72
                } else {
                    0.52
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::SyndicateTower => {
                let scale = if y_frac < 0.58 {
                    1.00
                } else if y_frac < 0.88 {
                    0.82
                } else {
                    0.56
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::ViaductStation => {
                if y < 3.85 {
                    (self.width, self.depth)
                } else {
                    (self.width * 0.92, self.depth * 0.86)
                }
            }
            BuildingArchetype::OctagonalCitadel => {
                let r_base = self.width.min(self.depth) * 0.86;
                let r = if y_frac < 0.38 {
                    r_base
                } else if y_frac < 0.72 {
                    r_base * 0.74
                } else {
                    r_base * 0.50
                };
                (r, r)
            }
            BuildingArchetype::GothicMonumentPlaza => {
                let r = self.width.min(self.depth) * 0.48;
                (r, r)
            }
            BuildingArchetype::AtlasDecoTower => {
                let scale = if y_frac < 0.34 {
                    1.00
                } else if y_frac < 0.68 {
                    0.82
                } else if y_frac < 0.88 {
                    0.64
                } else {
                    0.44
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::ChanceryArcadeKeep => {
                let scale = if y_frac < 0.38 {
                    1.00
                } else if y_frac < 0.66 {
                    0.86
                } else {
                    0.76
                };
                (self.width * scale, self.depth * scale)
            }
            BuildingArchetype::AlchemicalObservatory => {
                let scale = if y_frac < 0.44 {
                    1.00
                } else if y_frac < 0.76 {
                    0.80
                } else {
                    0.62
                };
                (self.width * scale, self.depth * scale)
            }
        };

        let hx = w_eff * 0.5;
        let hz = d_eff * 0.5;
        Some((self.x - hx, self.x + hx, self.z - hz, self.z + hz))
    }

    /// Snap a walkway connection point `[x, z]` onto the outer facade wall of this building
    /// at elevation `y` facing toward `other_xz`, ensuring the walkway is inset from building corners.
    pub fn snap_facade_anchor(
        &self,
        y: f32,
        hint_xz: [f32; 2],
        other_xz: [f32; 2],
        walkway_width: f32,
    ) -> [f32; 2] {
        let Some((x0, x1, z0, z1)) = self.cross_section_bounds_at(y) else {
            return hint_xz;
        };
        let cx = (x0 + x1) * 0.5;
        let cz = (z0 + z1) * 0.5;
        let hx = ((x1 - x0) * 0.5).max(0.1);
        let hz = ((z1 - z0) * 0.5).max(0.1);

        let dx = other_xz[0] - cx;
        let dz = other_xz[1] - cz;

        if (dx / hx).abs() >= (dz / hz).abs() {
            let wall_x = if dx >= 0.0 { x1 } else { x0 };
            let margin = (walkway_width * 0.65).min(hz * 0.55);
            let wall_z = hint_xz[1].clamp(z0 + margin, z1 - margin);
            [wall_x, wall_z]
        } else {
            let wall_z = if dz >= 0.0 { z1 } else { z0 };
            let margin = (walkway_width * 0.65).min(hx * 0.55);
            let wall_x = hint_xz[0].clamp(x0 + margin, x1 - margin);
            [wall_x, wall_z]
        }
    }
}

/// Visual style of an elevated walkway / skybridge spanning between two towers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalkwayStyle {
    /// Stone arch bridge with gothic parapet railings and warm walkway lamps.
    StoneArchBridge,
    /// Riveted iron truss skybridge with overhead lattice portal frames.
    IronTrussSkybridge,
    /// Covered gothic gallery bridge with pitched roof and slit windows.
    CoveredGallery,
    /// High-altitude vaulted cathedral skybridge with flying-buttress under-ribs and lantern pinnacles.
    GrandVaultedArcade,
}

/// An elevated walkway connecting two building facades above the street canyons.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkwaySpan {
    pub id: u32,
    /// ID of the building where `start` anchors into the facade at `elevation`.
    #[serde(default)]
    pub start_building_id: u32,
    /// ID of the building where `end` anchors into the facade at `elevation`.
    #[serde(default)]
    pub end_building_id: u32,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub elevation: f32,
    pub width: f32,
    pub style: WalkwayStyle,
}

impl WalkwaySpan {
    pub fn length(&self) -> f32 {
        let dx = self.end[0] - self.start[0];
        let dz = self.end[1] - self.start[1];
        (dx * dx + dz * dz).sqrt().max(0.1)
    }

    pub fn point_at(&self, t: f32) -> [f32; 3] {
        let clamped = t.clamp(0.0, 1.0);
        [
            self.start[0] + (self.end[0] - self.start[0]) * clamped,
            self.elevation,
            self.start[1] + (self.end[1] - self.start[1]) * clamped,
        ]
    }
}

/// A street-level canyon avenue or outer perimeter highway segment where gothic vehicles cruise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreetAvenue {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub width: f32,
    /// True if this road segment is part of the outer perimeter ring highway.
    #[serde(default)]
    pub is_highway: bool,
}

/// An underground parking garage entrance recessed into the base of a building,
/// connected to an adjacent street avenue or highway via a sloped driveway apron across the sidewalk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GaragePortal {
    pub id: u32,
    pub building_id: u32,
    /// Point `[x, z]` on the street centerline where vehicles turn into/out of the garage.
    pub street_entry: [f32; 2],
    /// Point `[x, z]` at the building facade portal mouth where the ramp descends underground.
    pub portal_pos: [f32; 2],
    /// Unit inward direction `[dx, dz]` pointing from the street into the building garage.
    pub inward_dir: [f32; 2],
    /// Width of the garage portal tunnel and ramp.
    pub width: f32,
}

fn default_true() -> bool {
    true
}

/// A 2nd-level elevated highway bridge/overpass spanning above an outer perimeter edge or city avenue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevatedHighwaySpan {
    pub id: u32,
    /// Start `[x, z]` of the full span.
    pub start: [f32; 2],
    /// End `[x, z]` of the full span.
    pub end: [f32; 2],
    /// Length along the span occupied by each sloped ramp at `start` and `end` (when enabled).
    pub ramp_length: f32,
    /// Elevated deck height above ground level.
    pub elevation: f32,
    /// Roadway deck width.
    pub width: f32,
    /// Whether `start` has a sloped ramp down to ground level (`false` = elevated junction).
    #[serde(default = "default_true")]
    pub start_has_ramp: bool,
    /// Whether `end` has a sloped ramp down to ground level (`false` = elevated junction).
    #[serde(default = "default_true")]
    pub end_has_ramp: bool,
}

/// A sunken roadside/highway portal where vehicles descend into an unsimulated underground section.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndergroundPortal {
    pub id: u32,
    /// Point `[x, z]` at ground level where the ramp begins descending.
    pub entry_pos: [f32; 2],
    /// Point `[x, z]` at the subterranean tunnel mouth (`y < 0`).
    pub portal_pos: [f32; 2],
    /// Unit horizontal direction `[dx, dz]` pointing into the tunnel mouth.
    pub inward_dir: [f32; 2],
    /// Width of the sunken ramp and tunnel portal.
    pub width: f32,
}

/// A connected multi-segment route for street and highway vehicles.
///
/// Either forms a continuous closed circuit around the connected avenues & outer ring highway
/// (`is_closed_loop = true`) or connects two underground building parking garages
/// (`is_closed_loop = false`), ensuring vehicles never drive off the edge or vanish in open air.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleRoute {
    pub id: u32,
    pub waypoints: Vec<[f32; 2]>,
    pub is_closed_loop: bool,
    /// Optional per-waypoint vertical elevation (`y`). Empty defaults to `0.025` (street level).
    #[serde(default)]
    pub waypoint_elevations: Vec<f32>,
    /// Segment indices `[i -> (i + 1) % n]` where vehicles are underground and hidden from rendering.
    #[serde(default)]
    pub underground_segments: Vec<usize>,
}

/// A steam emitter source (either a street grate or a high rooftop smokestack/vent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteamSource {
    pub pos: [f32; 3],
    pub rate_scale: f32,
    pub is_rooftop: bool,
}

/// A raised stone city block or courtyard plot in the metropolis blueprint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CityBlockPlot {
    pub id: u32,
    /// Minimum X coordinate of the raised sidewalk curb.
    pub x0: f32,
    /// Minimum Z coordinate of the raised sidewalk curb.
    pub z0: f32,
    /// Maximum X coordinate of the raised sidewalk curb.
    pub x1: f32,
    /// Maximum Z coordinate of the raised sidewalk curb.
    pub z1: f32,
}

/// An articulated elevated train running on the viaduct loop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevatedGothicTrain {
    pub id: u32,
    /// Distance of the lead locomotive along the closed viaduct track.
    pub track_dist: f32,
    pub speed: f32,
    pub base_speed: f32,
    pub car_count: u8,
    pub station_dwell_secs: f32,
}

/// A stylized pedestrian walking across an elevated skybridge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkwayPedestrian {
    pub id: u32,
    pub walkway_idx: usize,
    /// Normalized progress `0.0 ..= 1.0` along the walkway span.
    pub progress: f32,
    /// Direction (`1.0` or `-1.0`).
    pub dir: f32,
    pub speed: f32,
    /// Lateral offset across the walkway width (`-0.12 .. 0.12`).
    pub lateral_offset: f32,
    pub coat_variant: u8,
}

/// Style of street-level gothic vehicle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GothicVehicleKind {
    /// Exaggerated long-hood 1990s-Batman art-deco coupe.
    NoirCoupe,
    /// Heavy gothic armored cruiser / patrol car with flashing roof lightbar.
    ArmoredCruiser,
    /// Double-length vintage gothic street omnibus with lit passenger windows.
    TransitOmnibus,
    /// Hero jet-turbine noir interceptor with swept bat-fins and rear afterburner.
    VigilanteInterceptor,
    /// Elongated obsidian formal steam-limousine with brass carriage lanterns.
    SteamLimousine,
}

/// A vehicle cruising through the deep canyon avenues at street level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GothicVehicle {
    pub id: u32,
    pub avenue_idx: usize,
    /// Progress `0.0 ..= 1.0` along the avenue.
    pub progress: f32,
    /// Direction (`1.0` or `-1.0`).
    pub dir: f32,
    pub speed: f32,
    pub kind: GothicVehicleKind,
    /// Instantaneous route speed (modulated by traffic, corners, signals, and highway boosts).
    #[serde(default)]
    pub current_speed: f32,
    /// True when decelerating or stopped at a red signal / behind traffic.
    #[serde(default)]
    pub is_braking: bool,
    /// Active turn indicator: `-1` = left, `0` = off, `1` = right.
    #[serde(default)]
    pub turn_signal: i8,
    /// Rear turbine afterburner intensity (`0.0 ..= 1.0`), used by `VigilanteInterceptor`.
    #[serde(default)]
    pub afterburner: f32,
    /// Accumulator for periodic tailpipe steam/smoke puffs.
    #[serde(default)]
    pub exhaust_accum: f32,
}

/// A wrought-iron gothic intersection traffic signal controlling cross-traffic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrafficSignal {
    pub id: u32,
    pub pos: [f32; 2],
    pub phase_offset: f32,
    /// True when North-South (+/-Z) avenues have a green light (and East-West has red).
    pub ns_green: bool,
    /// True during the brief amber clearance phase before switching axes.
    pub is_amber: bool,
}

/// Street traffic density preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrafficDensity {
    #[default]
    NocturnalCruise,
    MidnightRushHour,
}

/// A high-altitude gothic patrol airship / dirigible drifting above the spires.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GothicAirship {
    pub id: u32,
    pub orbit_center: [f32; 2],
    pub orbit_rx: f32,
    pub orbit_rz: f32,
    pub altitude: f32,
    pub angle: f32,
    pub angular_speed: f32,
    pub hull_length: f32,
    pub searchlight_phase: f32,
}

impl GothicAirship {
    /// Current 3D world position `[x, y, z]` and forward unit direction `[fx, fz]`.
    pub fn pos_and_forward(&self, time: f32) -> ([f32; 3], [f32; 2]) {
        let x = self.orbit_center[0] + self.angle.cos() * self.orbit_rx;
        let z = self.orbit_center[1] + self.angle.sin() * self.orbit_rz;
        let bob = (time * 0.30 + self.id as f32 * 1.7).sin() * 0.08;
        let vx = -self.angle.sin() * self.orbit_rx * self.angular_speed;
        let vz = self.angle.cos() * self.orbit_rz * self.angular_speed;
        let len = (vx * vx + vz * vz).sqrt().max(0.0001);
        ([x, self.altitude + bob, z], [vx / len, vz / len])
    }
}

/// A rising puff of steam or industrial smoke from a rooftop smokestack or street grate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteamPlume {
    pub pos: [f32; 3],
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub age: f32,
    pub max_age: f32,
    pub base_radius: f32,
}

/// A rooftop searchlight battery sweeping a volumetric beam into the dark void.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchlightBeam {
    pub origin: [f32; 3],
    pub base_yaw: f32,
    pub sweep_amp: f32,
    pub sweep_speed: f32,
    pub tilt_from_vertical: f32,
    pub length: f32,
    pub phase_offset: f32,
}

impl SearchlightBeam {
    /// Compute the normalized 3D direction vector `[dx, dy, dz]` of the beam at `time`.
    pub fn direction_at(&self, time: f32) -> [f32; 3] {
        let yaw =
            self.base_yaw + (time * self.sweep_speed + self.phase_offset).sin() * self.sweep_amp;
        let tilt = self.tilt_from_vertical
            + (time * self.sweep_speed * 0.63 + self.phase_offset).cos() * 0.12;
        let sin_t = tilt.sin();
        let cos_t = tilt.cos();
        [yaw.cos() * sin_t, cos_t, yaw.sin() * sin_t]
    }
}

/// Live telemetry stats reported to the Popover HUD.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GothicMetropolisStats {
    pub active_trains: u32,
    pub trains_dispatched: u32,
    pub active_airships: u32,
    pub street_vehicles: u32,
    pub walkway_pedestrians: u32,
    pub palette_name: String,
    pub searchlights_lit: bool,
}
