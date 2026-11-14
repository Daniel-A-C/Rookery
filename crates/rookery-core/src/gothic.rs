//! Deterministic simulation state and modular city blueprint for *Gothic Metropolis* (`gothic-metropolis`).
//!
//! Models an exaggerated 1990s-Batman-style gothic art-deco metropolis in simplified 3D isometric
//! space (`x, z` horizontal footprint in `[-12.0..12.0]`, `y` vertical elevation in `[0.0..14.0]`).
//!
//! Key architectural properties:
//! - **Consistent Authored Blueprint (`CityBlueprint`)**: The city layout is deterministic and
//!   consistent every time you view it, defined by modular `BuildingPlacement` slots, multi-level
//!   `WalkwaySpan` skybridges, a closed `ViaductTrack` loop, and deep `StreetAvenue` canyons.
//! - **Extensible Building System (`BuildingArchetype`)**: New building archetypes or future
//!   district layouts can be added declaratively without changing the renderer or simulation loop.
//! - **Five Vertical Layers of Action**:
//!   1. Canyon street vehicles (`GothicVehicle`) & street steam grates (`y = 0.05`).
//!   2. Multi-level elevated walkways with pedestrians (`WalkwayPedestrian`) (`y = 1.6 .. 5.2`).
//!   3. Elevated commuter train network (`ElevatedGothicTrain`) weaving around & through towers (`y = 2.6`).
//!   4. Towering gothic spires with rising steam plumes (`SteamPlume`) & sweeping searchlights (`SearchlightBeam`).
//!   5. High-altitude patrol airships (`GothicAirship`) with downward spotlights (`y = 9.2 .. 11.8`).

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

use serde::{Deserialize, Serialize};

use crate::Activity;

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

/// Visual style of an elevated walkway / skybridge spanning between two towers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalkwayStyle {
    /// Stone arch bridge with gothic parapet railings and warm walkway lamps.
    StoneArchBridge,
    /// Riveted iron truss skybridge with overhead lattice portal frames.
    IronTrussSkybridge,
    /// Covered gothic gallery bridge with pitched roof and slit windows.
    CoveredGallery,
}

/// An elevated walkway connecting two building facades above the street canyons.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkwaySpan {
    pub id: u32,
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

/// A street-level canyon avenue where gothic vehicles cruise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreetAvenue {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub width: f32,
}

/// A steam emitter source (either a street grate or a high rooftop smokestack/vent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteamSource {
    pub pos: [f32; 3],
    pub rate_scale: f32,
    pub is_rooftop: bool,
}

/// Declarative, consistent blueprint of the Gothic Metropolis.
///
/// Because this blueprint is authored deterministically, the city layout is always
/// consistent when opened, while making it easy to add new building archetypes or
/// district layouts in future updates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CityBlueprint {
    pub name: String,
    /// Half-extent of the octagonal/stepped city plinth in world units.
    pub plinth_half_extent: f32,
    pub buildings: Vec<BuildingPlacement>,
    pub walkways: Vec<WalkwaySpan>,
    /// Closed loop of `(x, z)` waypoints for the elevated train viaduct at `viaduct_elevation`.
    pub viaduct_waypoints: Vec<[f32; 2]>,
    pub viaduct_elevation: f32,
    /// Index of the waypoint segment where the `ViaductStation` platform is located.
    pub station_waypoint_idx: usize,
    pub avenues: Vec<StreetAvenue>,
    pub steam_sources: Vec<SteamSource>,
}

impl CityBlueprint {
    /// Construct the canonical Gothic Metropolis layout.
    ///
    /// Arranged around two intersecting deep canyon avenues (`x = -2.2`, `x = 2.4`, `z = -2.2`, `z = 2.4`)
    /// with a towering central Cathedral & Ziggurat core, a mid-level elevated train loop at `y = 2.6`
    /// passing directly through the west `ViaductStation`, and 9 multi-tiered elevated walkways
    /// spanning across three vertical tiers (`y = 1.65`, `y = 3.55`, `y = 5.35`).
    pub fn default_gothic_metropolis() -> Self {
        let buildings = vec![
            // --- Central Crown District ---
            BuildingPlacement {
                id: 1,
                archetype: BuildingArchetype::CathedralKeep,
                x: -0.2,
                z: -0.2,
                width: 2.7,
                depth: 2.7,
                height: 9.8,
                quarter_turns: 0,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 2,
                archetype: BuildingArchetype::ArtDecoZiggurat,
                x: 4.3,
                z: -0.2,
                width: 2.5,
                depth: 2.5,
                height: 8.6,
                quarter_turns: 1,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 3,
                archetype: BuildingArchetype::ViaductStation,
                x: -4.6,
                z: -0.2,
                width: 2.4,
                depth: 3.0,
                height: 6.4,
                quarter_turns: 0,
                style_variant: 2,
            },
            // --- North District (Back Skyline) ---
            BuildingPlacement {
                id: 4,
                archetype: BuildingArchetype::ArtDecoZiggurat,
                x: -0.2,
                z: -4.6,
                width: 2.6,
                depth: 2.4,
                height: 9.0,
                quarter_turns: 0,
                style_variant: 3,
            },
            BuildingPlacement {
                id: 5,
                archetype: BuildingArchetype::SyndicateTower,
                x: -4.6,
                z: -4.6,
                width: 2.3,
                depth: 2.3,
                height: 7.4,
                quarter_turns: 0,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 6,
                archetype: BuildingArchetype::ClockworkBelfry,
                x: 4.3,
                z: -4.6,
                width: 2.1,
                depth: 2.1,
                height: 8.2,
                quarter_turns: 2,
                style_variant: 1,
            },
            // --- South District (Foreground Canyon) ---
            BuildingPlacement {
                id: 7,
                archetype: BuildingArchetype::SyndicateTower,
                x: -0.2,
                z: 4.3,
                width: 2.4,
                depth: 2.3,
                height: 7.2,
                quarter_turns: 2,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 8,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: -4.6,
                z: 4.3,
                width: 2.4,
                depth: 2.3,
                height: 5.8,
                quarter_turns: 1,
                style_variant: 3,
            },
            BuildingPlacement {
                id: 9,
                archetype: BuildingArchetype::ClockworkBelfry,
                x: 4.3,
                z: 4.3,
                width: 2.1,
                depth: 2.1,
                height: 6.9,
                quarter_turns: 3,
                style_variant: 0,
            },
            // --- Outer Perimeter Bastions & Flanking Spires ---
            BuildingPlacement {
                id: 10,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: -7.7,
                z: -2.4,
                width: 1.9,
                depth: 2.1,
                height: 4.9,
                quarter_turns: 0,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 11,
                archetype: BuildingArchetype::SyndicateTower,
                x: 7.5,
                z: -2.4,
                width: 1.9,
                depth: 2.1,
                height: 5.6,
                quarter_turns: 1,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 12,
                archetype: BuildingArchetype::ArtDecoZiggurat,
                x: -2.4,
                z: -7.6,
                width: 2.1,
                depth: 1.9,
                height: 6.8,
                quarter_turns: 0,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 13,
                archetype: BuildingArchetype::CathedralKeep,
                x: 2.1,
                z: -7.6,
                width: 2.2,
                depth: 2.0,
                height: 7.5,
                quarter_turns: 2,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 14,
                archetype: BuildingArchetype::ClockworkBelfry,
                x: -7.7,
                z: 2.1,
                width: 1.8,
                depth: 1.8,
                height: 5.4,
                quarter_turns: 1,
                style_variant: 3,
            },
            BuildingPlacement {
                id: 15,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: 7.5,
                z: 2.1,
                width: 1.9,
                depth: 2.0,
                height: 4.8,
                quarter_turns: 3,
                style_variant: 2,
            },
        ];

        // Multi-layered elevated walkways spanning across the canyon avenues at 3 distinct heights
        let walkways = vec![
            // Tier 1: Lower Plaza Bridges (y = 1.65)
            WalkwaySpan {
                id: 1,
                start: [-3.4, -0.2],
                end: [-1.5, -0.2],
                elevation: 1.65,
                width: 0.42,
                style: WalkwayStyle::StoneArchBridge,
            },
            WalkwaySpan {
                id: 2,
                start: [1.1, -0.2],
                end: [3.0, -0.2],
                elevation: 1.65,
                width: 0.42,
                style: WalkwayStyle::StoneArchBridge,
            },
            WalkwaySpan {
                id: 3,
                start: [-0.2, 1.1],
                end: [-0.2, 3.1],
                elevation: 1.65,
                width: 0.42,
                style: WalkwayStyle::StoneArchBridge,
            },
            // Tier 2: Mid-Level Gothic Skybridges (y = 3.65, clearing the y = 2.6 elevated train!)
            WalkwaySpan {
                id: 4,
                start: [-0.2, -3.4],
                end: [-0.2, -1.5],
                elevation: 3.65,
                width: 0.38,
                style: WalkwayStyle::CoveredGallery,
            },
            WalkwaySpan {
                id: 5,
                start: [-3.4, -4.6],
                end: [-1.5, -4.6],
                elevation: 3.65,
                width: 0.36,
                style: WalkwayStyle::IronTrussSkybridge,
            },
            WalkwaySpan {
                id: 6,
                start: [1.1, -4.6],
                end: [3.2, -4.6],
                elevation: 3.65,
                width: 0.36,
                style: WalkwayStyle::IronTrussSkybridge,
            },
            WalkwaySpan {
                id: 7,
                start: [-3.4, 4.3],
                end: [-1.4, 4.3],
                elevation: 3.55,
                width: 0.36,
                style: WalkwayStyle::IronTrussSkybridge,
            },
            WalkwaySpan {
                id: 8,
                start: [1.0, 4.3],
                end: [3.2, 4.3],
                elevation: 3.55,
                width: 0.36,
                style: WalkwayStyle::StoneArchBridge,
            },
            // Tier 3: High Spire Skybridges (y = 5.45)
            WalkwaySpan {
                id: 9,
                start: [1.1, 0.35],
                end: [3.0, 0.35],
                elevation: 5.45,
                width: 0.34,
                style: WalkwayStyle::CoveredGallery,
            },
            WalkwaySpan {
                id: 10,
                start: [-0.75, -3.4],
                end: [-0.75, -1.5],
                elevation: 5.65,
                width: 0.34,
                style: WalkwayStyle::IronTrussSkybridge,
            },
            WalkwaySpan {
                id: 11,
                start: [4.3, -3.5],
                end: [4.3, -1.4],
                elevation: 4.85,
                width: 0.34,
                style: WalkwayStyle::IronTrussSkybridge,
            },
            WalkwaySpan {
                id: 12,
                start: [4.3, 1.0],
                end: [4.3, 3.2],
                elevation: 4.65,
                width: 0.34,
                style: WalkwayStyle::StoneArchBridge,
            },
        ];

        // Closed elevated train viaduct loop at y = 2.6.
        // Passes straight through the ViaductStation at x = -4.6, z = -2.2 .. 2.0,
        // then weaves through the southern canyon (z = 2.1), up the eastern avenue (x = 2.1),
        // across the northern canyon (z = -2.4), and back into the station approach!
        let viaduct_waypoints = vec![
            [-4.6, -2.4], // 0: North portal of ViaductStation
            [-4.6, 2.1],  // 1: South portal of ViaductStation (platform segment 0 -> 1)
            [-3.8, 2.1],  // 2: Turn into south canyon
            [2.1, 2.1],   // 3: Eastward along south canyon
            [2.1, 6.4],   // 4: South-east outer loop extension
            [6.1, 6.4],   // 5: Outer south-east corner
            [6.1, -2.4],  // 6: Outer east viaduct run
            [2.1, -2.4],  // 7: Re-enter inner north-east junction
            [-4.6, -2.4], // 8: Close loop at North station portal (handled cyclically)
        ];
        // Note: We store the unique vertices of the closed polygon (0..8).
        let unique_viaduct_waypoints = viaduct_waypoints[..8].to_vec();

        // Deep street-level avenues for gothic vehicles
        let avenues = vec![
            // North-South Avenues
            StreetAvenue {
                start: [-2.4, -7.8],
                end: [-2.4, 7.8],
                width: 1.1,
            },
            StreetAvenue {
                start: [2.1, -7.8],
                end: [2.1, 7.8],
                width: 1.1,
            },
            // East-West Avenues
            StreetAvenue {
                start: [-7.8, -2.4],
                end: [7.8, -2.4],
                width: 1.1,
            },
            StreetAvenue {
                start: [-7.8, 2.1],
                end: [7.8, 2.1],
                width: 1.1,
            },
        ];

        // Street-level steam grates and rooftop industrial smokestacks
        let steam_sources = vec![
            // Rooftop foundry smokestacks
            SteamSource {
                pos: [-5.1, 6.6, 4.7],
                rate_scale: 1.25,
                is_rooftop: true,
            },
            SteamSource {
                pos: [-4.1, 6.6, 3.9],
                rate_scale: 1.1,
                is_rooftop: true,
            },
            SteamSource {
                pos: [-8.1, 5.6, -2.7],
                rate_scale: 1.0,
                is_rooftop: true,
            },
            SteamSource {
                pos: [7.9, 5.5, 2.4],
                rate_scale: 1.0,
                is_rooftop: true,
            },
            // Canyon street steam grates
            SteamSource {
                pos: [-2.4, 0.08, 0.8],
                rate_scale: 0.85,
                is_rooftop: false,
            },
            SteamSource {
                pos: [2.1, 0.08, -1.1],
                rate_scale: 0.85,
                is_rooftop: false,
            },
            SteamSource {
                pos: [-0.8, 0.08, 2.1],
                rate_scale: 0.75,
                is_rooftop: false,
            },
        ];

        Self {
            name: "Gotham Spire Core".into(),
            plinth_half_extent: 8.4,
            buildings,
            walkways,
            viaduct_waypoints: unique_viaduct_waypoints,
            viaduct_elevation: 2.6,
            station_waypoint_idx: 0,
            avenues,
            steam_sources,
        }
    }

    /// Total length of the closed viaduct track loop.
    pub fn viaduct_total_length(&self) -> f32 {
        let n = self.viaduct_waypoints.len();
        if n < 2 {
            return 1.0;
        }
        let mut total = 0.0;
        for i in 0..n {
            let a = self.viaduct_waypoints[i];
            let b = self.viaduct_waypoints[(i + 1) % n];
            let dx = b[0] - a[0];
            let dz = b[1] - a[1];
            total += (dx * dx + dz * dz).sqrt();
        }
        total.max(1.0)
    }

    /// Sample a 3D position `[x, y, z]` and forward unit tangent `[tx, tz]` at distance `dist`
    /// along the closed viaduct loop, with smooth corner rounding.
    pub fn sample_viaduct(&self, dist: f32) -> ([f32; 3], [f32; 2]) {
        let n = self.viaduct_waypoints.len();
        if n < 2 {
            return ([0.0, self.viaduct_elevation, 0.0], [1.0, 0.0]);
        }
        let total = self.viaduct_total_length();
        let mut d = dist.rem_euclid(total);

        for i in 0..n {
            let a = self.viaduct_waypoints[i];
            let b = self.viaduct_waypoints[(i + 1) % n];
            let dx = b[0] - a[0];
            let dz = b[1] - a[1];
            let seg_len = (dx * dx + dz * dz).sqrt().max(0.001);
            if d <= seg_len || i == n - 1 {
                let t = (d / seg_len).clamp(0.0, 1.0);
                let x = a[0] + dx * t;
                let z = a[1] + dz * t;
                return ([x, self.viaduct_elevation, z], [dx / seg_len, dz / seg_len]);
            }
            d -= seg_len;
        }
        (
            [
                self.viaduct_waypoints[0][0],
                self.viaduct_elevation,
                self.viaduct_waypoints[0][1],
            ],
            [0.0, 1.0],
        )
    }
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
    /// Heavy gothic armored cruiser / patrol car.
    ArmoredCruiser,
    /// Double-length vintage gothic street omnibus.
    TransitOmnibus,
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

/// Complete deterministic simulation state for *Gothic Metropolis* (`gothic-metropolis`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GothicMetropolisState {
    pub seed: u64,
    pub rng: GothicRng,
    pub time: f32,
    pub palette: GothicPalette,
    pub blueprint: CityBlueprint,
    pub trains: Vec<ElevatedGothicTrain>,
    pub pedestrians: Vec<WalkwayPedestrian>,
    pub vehicles: Vec<GothicVehicle>,
    pub airships: Vec<GothicAirship>,
    pub steam_plumes: Vec<SteamPlume>,
    pub searchlights: Vec<SearchlightBeam>,
    pub searchlights_enabled: bool,
    pub trains_dispatched: u32,
    /// Current isometric camera yaw angle in radians (smoothly interpolates toward `target_camera_yaw`).
    pub camera_yaw: f32,
    pub target_camera_yaw: f32,
    /// Current isometric camera pitch angle in radians (`0.38 .. 0.78`, default `0.54`).
    pub camera_pitch: f32,
    /// Fraction of the screen occupied by the city island (`0.60` = three-fifths default).
    pub screen_occupancy: f32,
    steam_accum: f32,
}

impl GothicMetropolisState {
    pub fn new(seed: u64) -> Self {
        let mut rng = GothicRng::new(seed);
        let blueprint = CityBlueprint::default_gothic_metropolis();
        let track_len = blueprint.viaduct_total_length();

        // Spawn 3 articulated elevated trains spaced around the viaduct loop
        // so at least one train is always visible in the foreground viaduct!
        let trains = vec![
            ElevatedGothicTrain {
                id: 1,
                track_dist: track_len * 0.44,
                speed: 2.35,
                base_speed: 2.35,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
            ElevatedGothicTrain {
                id: 2,
                track_dist: track_len * 0.78,
                speed: 2.25,
                base_speed: 2.25,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
            ElevatedGothicTrain {
                id: 3,
                track_dist: track_len * 0.12,
                speed: 2.30,
                base_speed: 2.30,
                car_count: 4,
                station_dwell_secs: 0.0,
            },
        ];

        // Spawn stylized pedestrians across all 12 elevated walkways
        let mut pedestrians = Vec::new();
        let mut ped_id = 1u32;
        for (w_idx, span) in blueprint.walkways.iter().enumerate() {
            let count = if span.elevation < 2.0 { 3 } else { 2 };
            for k in 0..count {
                let progress = ((k as f32 + 0.25) / count as f32 + rng.f32() * 0.18).fract();
                let dir = if (w_idx + k) % 2 == 0 { 1.0 } else { -1.0 };
                let lateral_offset = if dir > 0.0 { 0.07 } else { -0.07 };
                pedestrians.push(WalkwayPedestrian {
                    id: ped_id,
                    walkway_idx: w_idx,
                    progress,
                    dir,
                    speed: rng.range(0.11, 0.19) / span.length().max(1.0),
                    lateral_offset,
                    coat_variant: (ped_id % 3) as u8,
                });
                ped_id += 1;
            }
        }

        // Spawn 10 gothic street vehicles across the 4 deep canyon avenues
        let mut vehicles = Vec::new();
        let mut veh_id = 1u32;
        for ave_idx in 0..blueprint.avenues.len() {
            for slot in 0..2 {
                let dir = if slot == 0 { 1.0 } else { -1.0 };
                let kind = match (ave_idx + slot) % 3 {
                    0 => GothicVehicleKind::NoirCoupe,
                    1 => GothicVehicleKind::ArmoredCruiser,
                    _ => GothicVehicleKind::TransitOmnibus,
                };
                vehicles.push(GothicVehicle {
                    id: veh_id,
                    avenue_idx: ave_idx,
                    progress: rng.f32(),
                    dir,
                    speed: rng.range(0.065, 0.105),
                    kind,
                });
                veh_id += 1;
            }
        }
        // Add 2 extra Noir Coupes on the main central avenues
        for ave_idx in 0..2 {
            vehicles.push(GothicVehicle {
                id: veh_id,
                avenue_idx: ave_idx,
                progress: (rng.f32() + 0.5).fract(),
                dir: 1.0,
                speed: rng.range(0.085, 0.115),
                kind: GothicVehicleKind::NoirCoupe,
            });
            veh_id += 1;
        }

        // Spawn 2 gothic patrol airships cruising above the upper spires
        let airships = vec![
            GothicAirship {
                id: 1,
                orbit_center: [-0.4, -0.4],
                orbit_rx: 6.4,
                orbit_rz: 5.4,
                altitude: 10.4,
                angle: 0.65,
                angular_speed: 0.024,
                hull_length: 2.15,
                searchlight_phase: 0.0,
            },
            GothicAirship {
                id: 2,
                orbit_center: [0.6, 0.4],
                orbit_rx: 4.8,
                orbit_rz: 6.2,
                altitude: 11.6,
                angle: 3.8,
                angular_speed: -0.019,
                hull_length: 1.85,
                searchlight_phase: 2.1,
            },
        ];

        // Authored rooftop searchlights on the high towers
        let searchlights = vec![
            // SyndicateTower NW rooftop beacon
            SearchlightBeam {
                origin: [-4.6, 7.55, -4.6],
                base_yaw: 0.8,
                sweep_amp: 0.95,
                sweep_speed: 0.14,
                tilt_from_vertical: 0.34,
                length: 6.2,
                phase_offset: 0.0,
            },
            // ArtDecoZiggurat East crown beacon
            SearchlightBeam {
                origin: [4.3, 8.75, -0.2],
                base_yaw: 2.6,
                sweep_amp: 0.85,
                sweep_speed: 0.12,
                tilt_from_vertical: 0.30,
                length: 6.6,
                phase_offset: 1.7,
            },
            // SyndicateTower South rooftop beacon
            SearchlightBeam {
                origin: [-0.2, 7.35, 4.3],
                base_yaw: -1.4,
                sweep_amp: 0.90,
                sweep_speed: 0.16,
                tilt_from_vertical: 0.32,
                length: 5.8,
                phase_offset: 3.3,
            },
            // North ArtDecoZiggurat beacon
            SearchlightBeam {
                origin: [-0.2, 9.15, -4.6],
                base_yaw: 1.5,
                sweep_amp: 0.78,
                sweep_speed: 0.11,
                tilt_from_vertical: 0.28,
                length: 6.4,
                phase_offset: 4.8,
            },
        ];

        let mut state = Self {
            seed,
            rng,
            time: 0.0,
            palette: GothicPalette::GothamAmber,
            blueprint,
            trains,
            pedestrians,
            vehicles,
            airships,
            steam_plumes: Vec::with_capacity(64),
            searchlights,
            searchlights_enabled: true,
            trains_dispatched: 2,
            camera_yaw: FRAC_PI_4,
            target_camera_yaw: FRAC_PI_4,
            camera_pitch: 0.54,
            screen_occupancy: 0.60,
            steam_accum: 0.0,
        };

        // Pre-warm steam plumes so frame 0 already has atmospheric rising steam
        for _ in 0..24 {
            state.step_simulation(0.25);
        }

        state
    }

    pub fn tick(&mut self, dt_secs: f32) -> Activity {
        let clamped_dt = dt_secs.clamp(0.0, 0.25);
        self.step_simulation(clamped_dt);
        Activity::Animating
    }

    fn step_simulation(&mut self, dt: f32) {
        self.time += dt;

        // Smooth camera yaw interpolation toward target_camera_yaw
        let yaw_diff = self.target_camera_yaw - self.camera_yaw;
        if yaw_diff.abs() > 0.0005 {
            self.camera_yaw += yaw_diff * (dt * 7.5).min(1.0);
        } else {
            self.camera_yaw = self.target_camera_yaw;
        }

        // 1. Advance elevated trains along the closed viaduct loop
        let track_len = self.blueprint.viaduct_total_length();
        // Station platform is along segment 0 (track_dist 1.4 .. 2.8)
        let station_center_dist = 2.25f32;
        for train in &mut self.trains {
            if train.station_dwell_secs > 0.0 {
                train.station_dwell_secs = (train.station_dwell_secs - dt).max(0.0);
                train.speed = 0.0;
                if train.station_dwell_secs == 0.0 {
                    train.speed = train.base_speed * 0.35;
                }
                continue;
            }

            let dist_to_station = (station_center_dist - train.track_dist).rem_euclid(track_len);
            if dist_to_station < 0.18 && train.speed < train.base_speed * 0.55 {
                // Pause briefly at the ViaductStation platform
                train.station_dwell_secs = 1.4;
                train.track_dist = (station_center_dist + 0.19) % track_len;
                continue;
            } else if dist_to_station < 1.8 {
                // Smooth deceleration approaching the station
                let factor = (dist_to_station / 1.8).clamp(0.32, 1.0);
                train.speed = train.base_speed * factor;
            } else {
                // Smooth acceleration back to cruise speed
                train.speed += (train.base_speed - train.speed) * (dt * 3.0).min(1.0);
            }

            train.track_dist = (train.track_dist + train.speed * dt).rem_euclid(track_len);
        }

        // 2. Advance elevated walkway pedestrians
        for ped in &mut self.pedestrians {
            ped.progress += ped.dir * ped.speed * dt;
            if ped.progress >= 1.0 {
                ped.progress = 1.0;
                ped.dir = -1.0;
                ped.lateral_offset = -ped.lateral_offset.abs();
            } else if ped.progress <= 0.0 {
                ped.progress = 0.0;
                ped.dir = 1.0;
                ped.lateral_offset = ped.lateral_offset.abs();
            }
        }

        // 3. Advance canyon street vehicles
        for veh in &mut self.vehicles {
            veh.progress += veh.dir * veh.speed * dt;
            if veh.progress > 0.96 {
                veh.progress = 0.04;
            } else if veh.progress < 0.04 {
                veh.progress = 0.96;
            }
        }

        // 4. Advance high-altitude patrol airships
        for ship in &mut self.airships {
            ship.angle = (ship.angle + ship.angular_speed * dt) % TAU;
            ship.searchlight_phase = (ship.searchlight_phase + dt * 0.22) % TAU;
        }

        // 5. Update and spawn rising steam/smoke plumes
        for plume in &mut self.steam_plumes {
            plume.age += dt;
            plume.pos[0] += plume.vx * dt;
            plume.pos[1] += plume.vy * dt;
            plume.pos[2] += plume.vz * dt;
        }
        self.steam_plumes.retain(|p| p.age < p.max_age);

        self.steam_accum += dt;
        while self.steam_accum >= 0.14 {
            self.steam_accum -= 0.14;
            if self.steam_plumes.len() < 60 && !self.blueprint.steam_sources.is_empty() {
                let idx = (self.rng.next_u64() as usize) % self.blueprint.steam_sources.len();
                let src = &self.blueprint.steam_sources[idx];
                let vx = self.rng.range(-0.08, 0.14);
                let vz = self.rng.range(-0.10, 0.10);
                let vy = if src.is_rooftop {
                    self.rng.range(0.42, 0.68) * src.rate_scale
                } else {
                    self.rng.range(0.28, 0.45) * src.rate_scale
                };
                let max_age = if src.is_rooftop {
                    self.rng.range(2.6, 4.0)
                } else {
                    self.rng.range(1.8, 2.8)
                };
                let base_radius = if src.is_rooftop { 0.16 } else { 0.12 };
                self.steam_plumes.push(SteamPlume {
                    pos: src.pos,
                    vx,
                    vy,
                    vz,
                    age: 0.0,
                    max_age,
                    base_radius,
                });
            }
        }
    }

    /// Deterministic offline catch-up when waking from sleep.
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 {
            return;
        }
        let steps = ((elapsed_secs / 1.0).floor() as usize).clamp(1, 60);
        for _ in 0..steps {
            self.step_simulation(0.25);
        }
    }

    /// Dispatch an express elevated train (or boost existing trains if at max capacity).
    pub fn dispatch_train(&mut self) {
        self.trains_dispatched += 1;
        let track_len = self.blueprint.viaduct_total_length();
        if self.trains.len() < 4 {
            let new_id = self.trains.len() as u32 + 1;
            let offset = self
                .trains
                .last()
                .map(|t| (t.track_dist + track_len * 0.33) % track_len)
                .unwrap_or(0.0);
            self.trains.push(ElevatedGothicTrain {
                id: new_id,
                track_dist: offset,
                speed: 3.1,
                base_speed: 2.75,
                car_count: 5,
                station_dwell_secs: 0.0,
            });
        } else {
            for t in &mut self.trains {
                t.station_dwell_secs = 0.0;
                t.speed = 3.4;
            }
        }
    }

    /// Summon an additional patrol airship (or boost airship patrol speed and emit steam).
    pub fn summon_airship(&mut self) {
        if self.airships.len() < 4 {
            let new_id = self.airships.len() as u32 + 1;
            let dir = if new_id.is_multiple_of(2) { -1.0 } else { 1.0 };
            self.airships.push(GothicAirship {
                id: new_id,
                orbit_center: [0.0, 0.0],
                orbit_rx: 5.5 + (new_id as f32 * 0.55),
                orbit_rz: 5.2 + (new_id as f32 * 0.45),
                altitude: 9.6 + (new_id as f32 * 0.65),
                angle: self.rng.range(0.0, TAU),
                angular_speed: dir * 0.026,
                hull_length: 2.0,
                searchlight_phase: self.rng.range(0.0, TAU),
            });
        } else {
            for ship in &mut self.airships {
                ship.angle = (ship.angle + 0.18) % TAU;
            }
        }
    }

    /// Toggle rooftop and airship searchlights on or off.
    pub fn toggle_searchlights(&mut self) {
        self.searchlights_enabled = !self.searchlights_enabled;
    }

    /// Rotate the isometric camera by 90 degrees around the city island.
    pub fn rotate_camera_quarter_turn(&mut self) {
        self.target_camera_yaw += FRAC_PI_2;
    }

    /// Orbit camera directly during Direct Interaction Mode drag.
    pub fn orbit_camera(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.camera_yaw += delta_yaw;
        self.target_camera_yaw = self.camera_yaw;
        self.camera_pitch = (self.camera_pitch + delta_pitch).clamp(0.36, 0.82);
    }

    /// Cycle to the next gothic color palette.
    pub fn cycle_palette(&mut self) {
        self.palette = self.palette.next();
    }

    /// Set a specific gothic color palette.
    pub fn set_palette(&mut self, palette: GothicPalette) {
        self.palette = palette;
    }

    /// Trigger a dramatic burst of rooftop and street steam plumes.
    pub fn trigger_steam_burst(&mut self) {
        let sources = self.blueprint.steam_sources.clone();
        for src in &sources {
            for _ in 0..2 {
                if self.steam_plumes.len() >= 80 {
                    break;
                }
                let vx = self.rng.range(-0.16, 0.16);
                let vz = self.rng.range(-0.16, 0.16);
                let vy = self.rng.range(0.55, 0.95);
                self.steam_plumes.push(SteamPlume {
                    pos: src.pos,
                    vx,
                    vy,
                    vz,
                    age: 0.0,
                    max_age: self.rng.range(2.2, 3.6),
                    base_radius: 0.18,
                });
            }
        }
    }

    pub fn stats(&self) -> GothicMetropolisStats {
        GothicMetropolisStats {
            active_trains: self.trains.len() as u32,
            trains_dispatched: self.trains_dispatched,
            active_airships: self.airships.len() as u32,
            street_vehicles: self.vehicles.len() as u32,
            walkway_pedestrians: self.pedestrians.len() as u32,
            palette_name: self.palette.name().to_string(),
            searchlights_lit: self.searchlights_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_is_consistent_and_modular() {
        let bp1 = CityBlueprint::default_gothic_metropolis();
        let bp2 = CityBlueprint::default_gothic_metropolis();
        assert_eq!(
            bp1, bp2,
            "CityBlueprint must be 100% consistent across calls"
        );
        assert!(bp1.buildings.len() >= 12);
        assert!(bp1.walkways.len() >= 10);
        assert!(bp1.viaduct_waypoints.len() >= 6);
        assert!(bp1.avenues.len() >= 4);
        assert!(bp1.viaduct_total_length() > 20.0);
    }

    #[test]
    fn simulation_advances_all_vertical_layers() {
        let mut state = GothicMetropolisState::new(42);
        let initial_train_dist = state.trains[0].track_dist;
        let initial_ped_prog = state.pedestrians[0].progress;
        let initial_veh_prog = state.vehicles[0].progress;
        let initial_ship_angle = state.airships[0].angle;

        for _ in 0..10 {
            let act = state.tick(0.05);
            assert_eq!(act, Activity::Animating);
        }

        assert_ne!(state.trains[0].track_dist, initial_train_dist);
        assert_ne!(state.pedestrians[0].progress, initial_ped_prog);
        assert_ne!(state.vehicles[0].progress, initial_veh_prog);
        assert_ne!(state.airships[0].angle, initial_ship_angle);
        assert!(!state.steam_plumes.is_empty());
    }

    #[test]
    fn interactive_actions_mutate_state_cleanly() {
        let mut state = GothicMetropolisState::new(42);
        assert_eq!(state.trains.len(), 3);
        state.dispatch_train();
        assert_eq!(state.trains.len(), 4);

        assert_eq!(state.airships.len(), 2);
        state.summon_airship();
        assert_eq!(state.airships.len(), 3);

        assert!(state.searchlights_enabled);
        state.toggle_searchlights();
        assert!(!state.searchlights_enabled);

        let initial_yaw = state.target_camera_yaw;
        state.rotate_camera_quarter_turn();
        assert!((state.target_camera_yaw - (initial_yaw + FRAC_PI_2)).abs() < 1e-4);

        assert_eq!(state.palette, GothicPalette::GothamAmber);
        state.cycle_palette();
        assert_eq!(state.palette, GothicPalette::CathedralMoonlight);
    }
}
