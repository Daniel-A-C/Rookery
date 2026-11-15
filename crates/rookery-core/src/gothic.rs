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
//!   4. Towering gothic spires with rising steam plumes (`SteamPlume`).
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

/// A 2nd-level elevated highway bridge/overpass spanning above an outer perimeter edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevatedHighwaySpan {
    pub id: u32,
    /// Start `[x, z]` of the full span (including approach ramp at ground level `y = 0.0`).
    pub start: [f32; 2],
    /// End `[x, z]` of the full span (including exit ramp back to `y = 0.0`).
    pub end: [f32; 2],
    /// Length along the span occupied by each sloped ramp at `start` and `end`.
    pub ramp_length: f32,
    /// Elevated deck height above ground level.
    pub elevation: f32,
    /// Roadway deck width.
    pub width: f32,
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
    #[serde(default)]
    pub blocks: Vec<CityBlockPlot>,
    pub buildings: Vec<BuildingPlacement>,
    pub walkways: Vec<WalkwaySpan>,
    /// Closed loop of `(x, z)` waypoints for the elevated train viaduct at `viaduct_elevation`.
    pub viaduct_waypoints: Vec<[f32; 2]>,
    pub viaduct_elevation: f32,
    /// Index of the waypoint segment where the `ViaductStation` platform is located.
    pub station_waypoint_idx: usize,
    pub avenues: Vec<StreetAvenue>,
    #[serde(default)]
    pub garages: Vec<GaragePortal>,
    #[serde(default)]
    pub elevated_highways: Vec<ElevatedHighwaySpan>,
    #[serde(default)]
    pub underground_portals: Vec<UndergroundPortal>,
    #[serde(default)]
    pub vehicle_routes: Vec<VehicleRoute>,
    pub steam_sources: Vec<SteamSource>,
}

impl CityBlueprint {
    /// Construct the canonical Gothic Metropolis layout.
    ///
    /// Arranged across an organic, staggered **4x4** city district topology (16 raised stone
    /// city blocks of varied proportions) where **no interior street runs straight from edge
    /// to edge**: avenues meet at old-world gothic doglegs, T-junctions, and stepped plazas,
    /// bounded by an outer perimeter ring highway (`x = ±10.6`, `z = ±10.6`) on an expanded
    /// foundation floor plate (`plinth_half_extent = 12.2`). Features 22 buildings (including
    /// multi-structure compound blocks), 16 orthogonal and diagonal skybridges, a winding
    /// elevated train viaduct loop, and 10 subterranean building parking garages.
    pub fn default_gothic_metropolis() -> Self {
        // 16 Staggered City Block Plots (4 rows x 4 columns) with dogleg canyon avenues between them
        let blocks = vec![
            // Row 0 (North Skyline Ward)
            CityBlockPlot {
                id: 1,
                x0: -9.92,
                z0: -9.92,
                x1: -6.40,
                z1: -6.00,
            },
            CityBlockPlot {
                id: 2,
                x0: -5.20,
                z0: -9.92,
                x1: -1.00,
                z1: -6.00,
            },
            CityBlockPlot {
                id: 3,
                x0: 0.20,
                z0: -9.92,
                x1: 3.90,
                z1: -4.90,
            },
            CityBlockPlot {
                id: 4,
                x0: 5.10,
                z0: -9.92,
                x1: 9.92,
                z1: -4.90,
            },
            // Row 1 (Upper-Mid Sovereign Core)
            CityBlockPlot {
                id: 5,
                x0: -9.92,
                z0: -4.80,
                x1: -5.80,
                z1: -1.30,
            },
            CityBlockPlot {
                id: 6,
                x0: -4.60,
                z0: -4.80,
                x1: -1.00,
                z1: -0.40,
            },
            CityBlockPlot {
                id: 7,
                x0: 0.20,
                z0: -3.70,
                x1: 4.80,
                z1: -0.40,
            },
            CityBlockPlot {
                id: 8,
                x0: 6.00,
                z0: -3.70,
                x1: 9.92,
                z1: 0.50,
            },
            // Row 2 (Lower-Mid Plaza & Viaduct Core)
            CityBlockPlot {
                id: 9,
                x0: -9.92,
                z0: -0.10,
                x1: -5.80,
                z1: 4.80,
            },
            CityBlockPlot {
                id: 10,
                x0: -4.60,
                z0: 0.80,
                x1: 0.10,
                z1: 4.80,
            },
            CityBlockPlot {
                id: 11,
                x0: 1.30,
                z0: 0.80,
                x1: 4.80,
                z1: 4.00,
            },
            CityBlockPlot {
                id: 12,
                x0: 6.00,
                z0: 1.70,
                x1: 9.92,
                z1: 4.00,
            },
            // Row 3 (South Foreground & Industrial Ward)
            CityBlockPlot {
                id: 13,
                x0: -9.92,
                z0: 6.00,
                x1: -4.90,
                z1: 9.92,
            },
            CityBlockPlot {
                id: 14,
                x0: -3.70,
                z0: 6.00,
                x1: 0.10,
                z1: 9.92,
            },
            CityBlockPlot {
                id: 15,
                x0: 1.30,
                z0: 5.20,
                x1: 5.60,
                z1: 9.92,
            },
            CityBlockPlot {
                id: 16,
                x0: 6.80,
                z0: 5.20,
                x1: 9.92,
                z1: 9.92,
            },
        ];

        // 22 Gothic Buildings across the 16 asymmetric blocks (including multi-building compounds),
        // spanning 11 distinct archetypes and a dramatic vertical hierarchy (heights 4.4 .. 16.2).
        let buildings = vec![
            // --- Row 0 (North Skyline Ward: Blocks 1..4) ---
            BuildingPlacement {
                id: 1,
                archetype: BuildingArchetype::ClockworkBelfry,
                x: -8.16,
                z: -7.96,
                width: 2.1,
                depth: 2.1,
                height: 10.6,
                quarter_turns: 1,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 2,
                archetype: BuildingArchetype::ChanceryArcadeKeep,
                x: -3.55,
                z: -7.96,
                width: 2.2,
                depth: 2.4,
                height: 11.8,
                quarter_turns: 0,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 3,
                archetype: BuildingArchetype::AtlasDecoTower,
                x: 2.05,
                z: -6.85,
                width: 2.4,
                depth: 2.4,
                height: 14.4,
                quarter_turns: 0,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 4,
                archetype: BuildingArchetype::CathedralKeep,
                x: 7.10,
                z: -7.10,
                width: 2.6,
                depth: 2.6,
                height: 12.6,
                quarter_turns: 1,
                style_variant: 3,
            },
            // --- Row 1 (Upper-Mid Sovereign Core: Blocks 5..8) ---
            BuildingPlacement {
                id: 5,
                archetype: BuildingArchetype::SyndicateTower,
                x: -7.86,
                z: -3.05,
                width: 2.5,
                depth: 2.2,
                height: 11.0,
                quarter_turns: 1,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 6,
                archetype: BuildingArchetype::CathedralKeep,
                x: -2.80,
                z: -2.35,
                width: 2.6,
                depth: 2.8,
                height: 16.2,
                quarter_turns: 0,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 7,
                archetype: BuildingArchetype::ArtDecoZiggurat,
                x: 2.10,
                z: -2.05,
                width: 2.5,
                depth: 2.3,
                height: 15.4,
                quarter_turns: 1,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 8,
                archetype: BuildingArchetype::AlchemicalObservatory,
                x: 7.96,
                z: -1.60,
                width: 2.3,
                depth: 2.3,
                height: 10.6,
                quarter_turns: 1,
                style_variant: 3,
            },
            // --- Row 2 (Lower-Mid Plaza & Viaduct Core: Blocks 9..12) ---
            BuildingPlacement {
                id: 9,
                archetype: BuildingArchetype::ViaductStation,
                x: -7.86,
                z: 2.35,
                width: 2.4,
                depth: 3.0,
                height: 9.6,
                quarter_turns: 0,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 10,
                archetype: BuildingArchetype::AtlasDecoTower,
                x: -2.75,
                z: 2.80,
                width: 2.3,
                depth: 2.5,
                height: 13.6,
                quarter_turns: 2,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 11,
                archetype: BuildingArchetype::GothicMonumentPlaza,
                x: 3.05,
                z: 2.40,
                width: 2.6,
                depth: 2.3,
                height: 5.4,
                quarter_turns: 0,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 12,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: 7.96,
                z: 2.85,
                width: 2.6,
                depth: 1.5,
                height: 6.4,
                quarter_turns: 1,
                style_variant: 3,
            },
            // --- Row 3 (South Foreground & Industrial Ward: Blocks 13..16) ---
            BuildingPlacement {
                id: 13,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: -7.90,
                z: 7.96,
                width: 2.4,
                depth: 2.3,
                height: 6.8,
                quarter_turns: 0,
                style_variant: 2,
            },
            BuildingPlacement {
                id: 14,
                archetype: BuildingArchetype::OctagonalCitadel,
                x: -1.80,
                z: 7.96,
                width: 2.2,
                depth: 2.2,
                height: 9.8,
                quarter_turns: 3,
                style_variant: 0,
            },
            BuildingPlacement {
                id: 15,
                archetype: BuildingArchetype::ChanceryArcadeKeep,
                x: 3.45,
                z: 7.56,
                width: 2.5,
                depth: 2.7,
                height: 11.2,
                quarter_turns: 2,
                style_variant: 1,
            },
            BuildingPlacement {
                id: 16,
                archetype: BuildingArchetype::ArtDecoZiggurat,
                x: 8.36,
                z: 7.56,
                width: 2.0,
                depth: 2.5,
                height: 9.4,
                quarter_turns: 1,
                style_variant: 3,
            },
            // --- Compound Annexes & Asymmetric Secondary Spires on Larger Blocks ---
            // Building 17: East Octagonal Turret Annex on Block 2 (beside Building 2)
            BuildingPlacement {
                id: 17,
                archetype: BuildingArchetype::OctagonalCitadel,
                x: -1.85,
                z: -8.25,
                width: 1.25,
                depth: 1.25,
                height: 8.2,
                quarter_turns: 2,
                style_variant: 3,
            },
            // Building 18: North Citadel Bastion on Block 3 (behind Building 3)
            BuildingPlacement {
                id: 18,
                archetype: BuildingArchetype::SyndicateTower,
                x: 2.05,
                z: -8.85,
                width: 1.8,
                depth: 1.2,
                height: 8.8,
                quarter_turns: 1,
                style_variant: 0,
            },
            // Building 19: NE Civic Shrine Plaza on Block 4 (beside CathedralKeep Building 4)
            BuildingPlacement {
                id: 19,
                archetype: BuildingArchetype::GothicMonumentPlaza,
                x: 8.95,
                z: -8.55,
                width: 1.3,
                depth: 1.4,
                height: 4.4,
                quarter_turns: 3,
                style_variant: 1,
            },
            // Building 20: East Herald Campanile on Block 7 (flanking Crown Ziggurat Building 7)
            BuildingPlacement {
                id: 20,
                archetype: BuildingArchetype::ClockworkBelfry,
                x: 3.92,
                z: -2.05,
                width: 1.15,
                depth: 1.5,
                height: 10.2,
                quarter_turns: 0,
                style_variant: 2,
            },
            // Building 21: Central Plaza Alchemical Observatory on Block 10 (flanking Building 10)
            BuildingPlacement {
                id: 21,
                archetype: BuildingArchetype::AlchemicalObservatory,
                x: -0.85,
                z: 2.80,
                width: 1.35,
                depth: 1.8,
                height: 8.6,
                quarter_turns: 0,
                style_variant: 1,
            },
            // Building 22: Axis Chemical Boiler Annex on Block 13 (beside Foundry Building 13)
            BuildingPlacement {
                id: 22,
                archetype: BuildingArchetype::IndustrialFoundry,
                x: -5.85,
                z: 7.96,
                width: 1.3,
                depth: 1.8,
                height: 5.4,
                quarter_turns: 2,
                style_variant: 1,
            },
        ];

        // Helper that snaps both walkway endpoints onto the exact structural facade walls
        // of `start_b_id` and `end_b_id` at `elevation`, guaranteeing 100% solid connections.
        let make_walkway = |id: u32,
                            start_b_id: u32,
                            end_b_id: u32,
                            hint_start: [f32; 2],
                            hint_end: [f32; 2],
                            elevation: f32,
                            width: f32,
                            style: WalkwayStyle|
         -> WalkwaySpan {
            let b_start = buildings
                .iter()
                .find(|b| b.id == start_b_id)
                .expect("valid start_building_id");
            let b_end = buildings
                .iter()
                .find(|b| b.id == end_b_id)
                .expect("valid end_building_id");
            let start =
                b_start.snap_facade_anchor(elevation, hint_start, [b_end.x, b_end.z], width);
            let end = b_end.snap_facade_anchor(elevation, hint_end, [b_start.x, b_start.z], width);
            WalkwaySpan {
                id,
                start_building_id: start_b_id,
                end_building_id: end_b_id,
                start,
                end,
                elevation,
                width,
                style,
            }
        };

        // 18 Multi-layered elevated walkways across 4 vertical tiers (y = 1.65 .. 8.10),
        // every span anchored directly into the elevation-specific cross-sections of its buildings.
        let walkways = vec![
            // Tier 1: Lower Plaza Stone Arch Bridges (y = 1.65 .. 1.75, on canyons without elevated train)
            make_walkway(
                1,
                6,
                7,
                [-1.50, -2.15],
                [0.85, -2.05],
                1.65,
                0.42,
                WalkwayStyle::StoneArchBridge,
            ),
            make_walkway(
                2,
                1,
                2,
                [-7.25, -7.96],
                [-4.65, -7.96],
                1.65,
                0.40,
                WalkwayStyle::StoneArchBridge,
            ),
            make_walkway(
                3,
                3,
                4,
                [3.25, -6.95],
                [5.80, -7.05],
                1.65,
                0.40,
                WalkwayStyle::StoneArchBridge,
            ),
            make_walkway(
                4,
                14,
                15,
                [-0.85, 7.80],
                [2.20, 7.65],
                1.70,
                0.42,
                WalkwayStyle::StoneArchBridge,
            ),
            make_walkway(
                5,
                15,
                16,
                [4.70, 7.56],
                [7.36, 7.56],
                1.70,
                0.40,
                WalkwayStyle::StoneArchBridge,
            ),
            // Tier 2: Mid-Level Gothic Skybridges (y = 3.60 .. 4.25, clearing the y = 2.6 elevated train!)
            make_walkway(
                6,
                5,
                6,
                [-6.61, -2.85],
                [-3.74, -2.55],
                3.70,
                0.38,
                WalkwayStyle::CoveredGallery,
            ),
            make_walkway(
                7,
                6,
                10,
                [-2.80, -0.95],
                [-2.75, 1.55],
                3.75,
                0.38,
                WalkwayStyle::IronTrussSkybridge,
            ),
            make_walkway(
                8,
                10,
                21,
                [-1.60, 2.80],
                [-1.52, 2.80],
                3.65,
                0.36,
                WalkwayStyle::CoveredGallery,
            ),
            make_walkway(
                9,
                4,
                8,
                [7.45, -5.80],
                [7.75, -2.75],
                3.80,
                0.36,
                WalkwayStyle::IronTrussSkybridge,
            ),
            make_walkway(
                10,
                20,
                8,
                [4.41, -1.95],
                [6.81, -1.70],
                3.65,
                0.36,
                WalkwayStyle::IronTrussSkybridge,
            ),
            make_walkway(
                11,
                9,
                10,
                [-6.75, 2.55],
                [-3.90, 2.75],
                4.25,
                0.38,
                WalkwayStyle::StoneArchBridge,
            ),
            make_walkway(
                12,
                8,
                12,
                [7.96, -0.45],
                [7.96, 2.12],
                3.60,
                0.36,
                WalkwayStyle::IronTrussSkybridge,
            ),
            make_walkway(
                13,
                10,
                14,
                [-2.45, 4.05],
                [-1.95, 7.01],
                3.65,
                0.36,
                WalkwayStyle::IronTrussSkybridge,
            ),
            // Tier 3: High Spire Skybridges (y = 5.35 .. 5.65)
            make_walkway(
                14,
                2,
                6,
                [-3.25, -6.92],
                [-2.95, -3.36],
                5.35,
                0.34,
                WalkwayStyle::CoveredGallery,
            ),
            make_walkway(
                15,
                3,
                7,
                [2.05, -5.86],
                [2.10, -3.20],
                5.55,
                0.34,
                WalkwayStyle::IronTrussSkybridge,
            ),
            make_walkway(
                16,
                6,
                7,
                [-1.50, -2.55],
                [0.85, -2.25],
                5.25,
                0.36,
                WalkwayStyle::CoveredGallery,
            ),
            // Tier 4: Sovereign High-Altitude Vaulted Skybridges (y = 7.45 .. 8.10)
            make_walkway(
                17,
                6,
                7,
                [-1.86, -2.20],
                [1.12, -2.05],
                8.10,
                0.38,
                WalkwayStyle::GrandVaultedArcade,
            ),
            make_walkway(
                18,
                6,
                10,
                [-2.78, -1.34],
                [-2.75, 1.77],
                7.45,
                0.36,
                WalkwayStyle::GrandVaultedArcade,
            ),
        ];

        // Winding non-grid elevated train viaduct loop at y = 2.6.
        // Passes straight through the West ViaductStation (Building 9 at x = -7.86, z = 2.35),
        // angles along the SW canyon into the West Mid Canyon (x = -5.20), cuts across the
        // Central Grand Plaza (z = 0.20), jogs down the South Central Spine (x = 0.70) to the
        // SE canyon (z = 4.60), climbs the East Mid Canyon (x = 5.40) to the NE canyon (z = -4.30),
        // jogs at the North Spine (x = -0.40) into the NW canyon (z = -5.40), and returns via
        // the staggered Mid-West Outer Avenue (z = -0.70).
        let unique_viaduct_waypoints = vec![
            [-7.86, -0.70], // 0: North portal approach to ViaductStation
            [-7.86, 4.90],  // 1: South portal exit of ViaductStation (platform segment 0 -> 1)
            [-5.20, 5.40],  // 2: Oblique viaduct span into West Mid Canyon junction
            [-5.20, 0.20],  // 3: Northward along West Mid Canyon to Central Plaza
            [0.70, 0.20],   // 4: Eastward along Central Grand Plaza Avenue
            [0.70, 4.60],   // 5: Southward jog along South Central Spine
            [5.40, 4.60],   // 6: Eastward along South-East Avenue
            [5.40, -4.30],  // 7: Northward along East Mid Canyon Avenue
            [-0.40, -4.30], // 8: Westward along North-East Avenue
            [-0.40, -5.40], // 9: Northward dogleg at North Central Spine
            [-5.20, -5.40], // 10: Westward along North-West Avenue
            [-5.20, -0.70], // 11: Southward to Mid-West Outer Avenue dogleg
        ];

        // Staggered non-grid street network: 15 dogleg/T-junction canyon segments + 4 outer ring highway segments
        let avenues = vec![
            // --- Staggered East-West Canyon Avenues ---
            StreetAvenue {
                start: [-10.6, -5.4],
                end: [-0.4, -5.4],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-0.4, -4.3],
                end: [10.6, -4.3],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-10.6, -0.7],
                end: [-5.2, -0.7],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-5.2, 0.2],
                end: [5.4, 0.2],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [5.4, 1.1],
                end: [10.6, 1.1],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-10.6, 5.4],
                end: [0.7, 5.4],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [0.7, 4.6],
                end: [10.6, 4.6],
                width: 1.10,
                is_highway: false,
            },
            // --- Staggered North-South Canyon Avenues ---
            StreetAvenue {
                start: [-5.8, -10.6],
                end: [-5.8, -5.4],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-5.2, -5.4],
                end: [-5.2, 5.4],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-4.3, 5.4],
                end: [-4.3, 10.6],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [-0.4, -10.6],
                end: [-0.4, 0.2],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [0.7, 0.2],
                end: [0.7, 10.6],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [4.5, -10.6],
                end: [4.5, -4.3],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [5.4, -4.3],
                end: [5.4, 4.6],
                width: 1.10,
                is_highway: false,
            },
            StreetAvenue {
                start: [6.2, 4.6],
                end: [6.2, 10.6],
                width: 1.10,
                is_highway: false,
            },
            // --- Outer Perimeter Ring Highway (4-sided loop encircling the 4x4 city at ±10.6) ---
            StreetAvenue {
                start: [-10.6, -10.6],
                end: [10.6, -10.6],
                width: 1.28,
                is_highway: true,
            },
            StreetAvenue {
                start: [10.6, -10.6],
                end: [10.6, 10.6],
                width: 1.28,
                is_highway: true,
            },
            StreetAvenue {
                start: [10.6, 10.6],
                end: [-10.6, 10.6],
                width: 1.28,
                is_highway: true,
            },
            StreetAvenue {
                start: [-10.6, 10.6],
                end: [-10.6, -10.6],
                width: 1.28,
                is_highway: true,
            },
        ];

        // 10 Subterranean Parking Garage Entrances recessed into major building bases across the 4x4 districts
        let garages = vec![
            // Garage 1: CathedralKeep (Building 6) East facade onto North Central Spine (x = -0.4)
            GaragePortal {
                id: 1,
                building_id: 6,
                street_entry: [-0.40, -1.65],
                portal_pos: [-1.50, -1.65],
                inward_dir: [-1.0, 0.0],
                width: 0.68,
            },
            // Garage 2: ArtDecoZiggurat (Building 7) South facade onto Central Grand Plaza Avenue (z = 0.2)
            GaragePortal {
                id: 2,
                building_id: 7,
                street_entry: [2.10, 0.20],
                portal_pos: [2.10, -0.90],
                inward_dir: [0.0, -1.0],
                width: 0.68,
            },
            // Garage 3: SyndicateTower (Building 2) South facade onto North-West Avenue (z = -5.4)
            GaragePortal {
                id: 3,
                building_id: 2,
                street_entry: [-3.55, -5.40],
                portal_pos: [-3.55, -6.76],
                inward_dir: [0.0, -1.0],
                width: 0.68,
            },
            // Garage 4: CathedralKeep (Building 4) North facade onto Outer North Highway (z = -10.6)
            GaragePortal {
                id: 4,
                building_id: 4,
                street_entry: [7.10, -10.60],
                portal_pos: [7.10, -8.40],
                inward_dir: [0.0, 1.0],
                width: 0.68,
            },
            // Garage 5: ClockworkBelfry (Building 8) East facade onto Outer East Highway (x = 10.6)
            GaragePortal {
                id: 5,
                building_id: 8,
                street_entry: [10.60, -1.60],
                portal_pos: [9.06, -1.60],
                inward_dir: [-1.0, 0.0],
                width: 0.68,
            },
            // Garage 6: SyndicateTower (Building 5) West facade onto Outer West Highway (x = -10.6)
            GaragePortal {
                id: 6,
                building_id: 5,
                street_entry: [-10.60, -3.05],
                portal_pos: [-9.11, -3.05],
                inward_dir: [1.0, 0.0],
                width: 0.68,
            },
            // Garage 7: ArtDecoZiggurat (Building 10) North facade onto Central Grand Plaza Avenue (z = 0.2)
            GaragePortal {
                id: 7,
                building_id: 10,
                street_entry: [-2.75, 0.20],
                portal_pos: [-2.75, 1.55],
                inward_dir: [0.0, 1.0],
                width: 0.68,
            },
            // Garage 8: SyndicateTower (Building 16) West facade onto SE Alley (x = 6.2)
            GaragePortal {
                id: 8,
                building_id: 16,
                street_entry: [6.20, 7.56],
                portal_pos: [7.36, 7.56],
                inward_dir: [1.0, 0.0],
                width: 0.68,
            },
            // Garage 9: IndustrialFoundry (Building 13) North facade onto South-West Avenue (z = 5.4)
            GaragePortal {
                id: 9,
                building_id: 13,
                street_entry: [-7.90, 5.40],
                portal_pos: [-7.90, 6.81],
                inward_dir: [0.0, 1.0],
                width: 0.68,
            },
            // Garage 10: CathedralKeep (Building 15) West facade onto South Central Spine (x = 0.7)
            GaragePortal {
                id: 10,
                building_id: 15,
                street_entry: [0.70, 7.56],
                portal_pos: [2.20, 7.56],
                inward_dir: [1.0, 0.0],
                width: 0.68,
            },
        ];

        // 2nd-Level Elevated Highway Overpass along the South foreground outer edge (`z = +10.6`)
        let elevated_highways = vec![ElevatedHighwaySpan {
            id: 1,
            start: [-9.8, 10.6],
            end: [9.8, 10.6],
            ramp_length: 3.4,
            elevation: 1.85,
            width: 1.18,
        }];

        // 4 Sunken Underground Tunnel Portals for subterranean crosstown shortcuts
        let underground_portals = vec![
            // Portal 1: East Outer Highway -> Block 8 South Plaza
            UndergroundPortal {
                id: 1,
                entry_pos: [10.60, 0.05],
                portal_pos: [8.80, 0.05],
                inward_dir: [-1.0, 0.0],
                width: 0.78,
            },
            // Portal 2: South-West Avenue -> Block 14 NW Plaza
            UndergroundPortal {
                id: 2,
                entry_pos: [-3.35, 5.40],
                portal_pos: [-3.35, 7.20],
                inward_dir: [0.0, 1.0],
                width: 0.78,
            },
            // Portal 3: East Outer Highway -> Block 4 SE Plaza
            UndergroundPortal {
                id: 3,
                entry_pos: [10.60, -5.60],
                portal_pos: [8.85, -5.60],
                inward_dir: [-1.0, 0.0],
                width: 0.78,
            },
            // Portal 4: West Outer Highway -> Block 1 SW Plaza
            UndergroundPortal {
                id: 4,
                entry_pos: [-10.60, -6.50],
                portal_pos: [-8.85, -6.50],
                inward_dir: [1.0, 0.0],
                width: 0.78,
            },
        ];

        // 13 Connected Vehicle Routes:
        // - Routes 0..5 are continuous closed loops navigating the Outer Ring Highway & dogleg inner avenues.
        // - Routes 5..10 are subterranean Garage-to-Garage commuter routes across the 4x4 city.
        // - Route 10 is the 2nd-Level South Overpass Express Loop (climbs to y = 1.875).
        // - Routes 11..13 are Subterranean Express Tunnel Loops connecting sunken UndergroundPortals.
        let vehicle_routes = vec![
            // Route 1: Outer Perimeter Ring Highway (Clockwise Loop)
            VehicleRoute {
                id: 1,
                waypoints: vec![[-10.6, -10.6], [10.6, -10.6], [10.6, 10.6], [-10.6, 10.6]],
                is_closed_loop: true,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 2: Outer Perimeter Ring Highway (Counter-Clockwise Loop)
            VehicleRoute {
                id: 2,
                waypoints: vec![[-10.6, -10.6], [-10.6, 10.6], [10.6, 10.6], [10.6, -10.6]],
                is_closed_loop: true,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 3: Grand Staggered Inner Citadel Circuit (navigating the dogleg central blocks)
            VehicleRoute {
                id: 3,
                waypoints: vec![
                    [-5.2, 0.2],
                    [5.4, 0.2],
                    [5.4, 4.6],
                    [0.7, 4.6],
                    [0.7, 5.4],
                    [-5.2, 5.4],
                ],
                is_closed_loop: true,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 4: North-East & North-West Dogleg Canyon Circuit
            VehicleRoute {
                id: 4,
                waypoints: vec![
                    [-5.2, -5.4],
                    [-0.4, -5.4],
                    [-0.4, -4.3],
                    [5.4, -4.3],
                    [5.4, 0.2],
                    [-5.2, 0.2],
                ],
                is_closed_loop: true,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 5: Cross-Metropolis Winding Boulevard & South Highway Loop
            VehicleRoute {
                id: 5,
                waypoints: vec![
                    [-10.6, -5.4],
                    [-0.4, -5.4],
                    [-0.4, 0.2],
                    [0.7, 0.2],
                    [0.7, 4.6],
                    [10.6, 4.6],
                    [10.6, 10.6],
                    [-10.6, 10.6],
                ],
                is_closed_loop: true,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 6: Underground Garage 1 (CathedralKeep) <-> Underground Garage 5 (East Highway Belfry)
            VehicleRoute {
                id: 6,
                waypoints: vec![
                    [-2.15, -1.65],
                    [-1.50, -1.65],
                    [-0.40, -1.65],
                    [-0.40, -4.30],
                    [10.60, -4.30],
                    [10.60, -1.60],
                    [9.06, -1.60],
                    [8.40, -1.60],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 7: Underground Garage 2 (Crown Ziggurat) <-> Underground Garage 3 (NW Syndicate Tower)
            VehicleRoute {
                id: 7,
                waypoints: vec![
                    [2.10, -1.55],
                    [2.10, -0.90],
                    [2.10, 0.20],
                    [-0.40, 0.20],
                    [-0.40, -5.40],
                    [-3.55, -5.40],
                    [-3.55, -6.76],
                    [-3.55, -7.40],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 8: Underground Garage 7 (Central Ziggurat) <-> Underground Garage 9 (SW Chemical Foundry)
            VehicleRoute {
                id: 8,
                waypoints: vec![
                    [-2.75, 2.20],
                    [-2.75, 1.55],
                    [-2.75, 0.20],
                    [-5.20, 0.20],
                    [-5.20, 5.40],
                    [-7.90, 5.40],
                    [-7.90, 6.81],
                    [-7.90, 7.45],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 9: Underground Garage 8 (SE Syndicate Bastion) <-> Underground Garage 4 (NE Cathedral Citadel)
            VehicleRoute {
                id: 9,
                waypoints: vec![
                    [8.00, 7.56],
                    [7.36, 7.56],
                    [6.20, 7.56],
                    [6.20, 4.60],
                    [5.40, 4.60],
                    [5.40, -4.30],
                    [10.60, -4.30],
                    [10.60, -10.60],
                    [7.10, -10.60],
                    [7.10, -8.40],
                    [7.10, -7.75],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 10: Underground Garage 6 (West Syndicate) <-> Underground Garage 10 (SE Cathedral Keep)
            VehicleRoute {
                id: 10,
                waypoints: vec![
                    [-8.45, -3.05],
                    [-9.11, -3.05],
                    [-10.60, -3.05],
                    [-10.60, 5.40],
                    [0.70, 5.40],
                    [0.70, 7.56],
                    [2.20, 7.56],
                    [2.85, 7.56],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 11: 2nd-Level South Overpass & Outer Ring Express Loop
            VehicleRoute {
                id: 11,
                waypoints: vec![
                    [-10.6, 10.6],
                    [-9.8, 10.6],
                    [-6.4, 10.6],
                    [6.4, 10.6],
                    [9.8, 10.6],
                    [10.6, 10.6],
                    [10.6, -10.6],
                    [-10.6, -10.6],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![0.025, 0.025, 1.875, 1.875, 0.025, 0.025, 0.025, 0.025],
                underground_segments: Vec::new(),
            },
            // Route 12: Subterranean Tunnel Loop (Portal 1 East Outer -> Portal 2 SW Avenue)
            VehicleRoute {
                id: 12,
                waypoints: vec![
                    [10.60, 0.05],
                    [8.80, 0.05],
                    [-3.35, 7.20],
                    [-3.35, 5.40],
                    [0.70, 5.40],
                    [0.70, 4.60],
                    [10.60, 4.60],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![0.025, -0.22, -0.22, 0.025, 0.025, 0.025, 0.025],
                underground_segments: vec![1],
            },
            // Route 13: Subterranean Crosstown Loop (Portal 3 NE Outer -> Portal 4 NW Outer)
            VehicleRoute {
                id: 13,
                waypoints: vec![
                    [10.60, -5.60],
                    [8.85, -5.60],
                    [-8.85, -6.50],
                    [-10.60, -6.50],
                    [-10.60, -10.60],
                    [10.60, -10.60],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![0.025, -0.22, -0.22, 0.025, 0.025, 0.025],
                underground_segments: vec![1],
            },
        ];

        // Street-level steam grates and rooftop industrial smokestacks across the 4x4 metropolis
        let steam_sources = vec![
            // Rooftop foundry smokestacks (aligned with Buildings 12, 13, 22)
            SteamSource {
                pos: [-8.48, 8.30, 7.63],
                rate_scale: 1.25,
                is_rooftop: true,
            },
            SteamSource {
                pos: [-7.32, 8.30, 8.29],
                rate_scale: 1.15,
                is_rooftop: true,
            },
            SteamSource {
                pos: [-6.16, 6.59, 7.70],
                rate_scale: 1.05,
                is_rooftop: true,
            },
            SteamSource {
                pos: [7.34, 7.81, 2.63],
                rate_scale: 1.15,
                is_rooftop: true,
            },
            SteamSource {
                pos: [8.58, 7.81, 3.07],
                rate_scale: 1.05,
                is_rooftop: true,
            },
            // Canyon street steam grates at dogleg junctions
            SteamSource {
                pos: [-5.20, 0.08, 0.20],
                rate_scale: 0.85,
                is_rooftop: false,
            },
            SteamSource {
                pos: [-0.40, 0.08, -2.10],
                rate_scale: 0.85,
                is_rooftop: false,
            },
            SteamSource {
                pos: [0.70, 0.08, 2.40],
                rate_scale: 0.80,
                is_rooftop: false,
            },
            SteamSource {
                pos: [5.40, 0.08, -1.20],
                rate_scale: 0.80,
                is_rooftop: false,
            },
            SteamSource {
                pos: [-2.60, 0.08, 5.40],
                rate_scale: 0.75,
                is_rooftop: false,
            },
        ];

        Self {
            name: "Gotham Spire Core".into(),
            plinth_half_extent: 12.2,
            blocks,
            buildings,
            walkways,
            viaduct_waypoints: unique_viaduct_waypoints,
            viaduct_elevation: 2.6,
            station_waypoint_idx: 0,
            avenues,
            garages,
            elevated_highways,
            underground_portals,
            vehicle_routes,
            steam_sources,
        }
    }

    /// Sample a vehicle's 3D world position `[x, y, z]`, unit forward tangent `[fx, fz]`,
    /// and whether the vehicle is currently deep inside a subterranean parking garage vault
    /// or underground tunnel (`is_subterranean_hidden`).
    ///
    /// Applies right-hand lane offsets along street and highway segments, centers vehicles on
    /// underground garage/tunnel ramps, and rounds 90-degree corners with smooth Quadratic Bezier curves.
    pub fn sample_vehicle_pose(&self, veh: &GothicVehicle) -> Option<([f32; 3], [f32; 2], bool)> {
        self.sample_vehicle_pose_3d(veh)
            .map(|(pos, fwd, _slope, hidden)| (pos, fwd, hidden))
    }

    /// Extended variant of [`Self::sample_vehicle_pose`] that also returns the vertical ramp
    /// slope `dy / ds` along the vehicle's forward direction for realistic chassis pitching.
    pub fn sample_vehicle_pose_3d(
        &self,
        veh: &GothicVehicle,
    ) -> Option<([f32; 3], [f32; 2], f32, bool)> {
        if self.vehicle_routes.is_empty() {
            // Fallback for custom minimal blueprints with only straight `avenues`
            let ave = self.avenues.get(veh.avenue_idx)?;
            let dx = ave.end[0] - ave.start[0];
            let dz = ave.end[1] - ave.start[1];
            let len = (dx * dx + dz * dz).sqrt().max(0.1);
            let u_fwd = [dx / len, dz / len];
            let u_right = [-u_fwd[1], u_fwd[0]];
            let lane_offset = if veh.dir >= 0.0 { 0.22 } else { -0.22 };
            let vx = ave.start[0] + dx * veh.progress + u_right[0] * lane_offset;
            let vz = ave.start[1] + dz * veh.progress + u_right[1] * lane_offset;
            let car_fwd = [u_fwd[0] * veh.dir.signum(), u_fwd[1] * veh.dir.signum()];
            return Some(([vx, 0.025, vz], car_fwd, 0.0, false));
        }

        let route = self
            .vehicle_routes
            .get(veh.avenue_idx % self.vehicle_routes.len())?;
        let n = route.waypoints.len();
        if n < 2 {
            return None;
        }

        let dir_sign = if veh.dir >= 0.0 { 1.0f32 } else { -1.0f32 };
        let lane_mag = 0.23f32 * dir_sign;

        // Check if waypoint `i` is a portal mouth or street approach for an underground tunnel segment.
        let is_tunnel_mouth = |idx: usize| -> bool {
            route
                .underground_segments
                .iter()
                .any(|&s| s == idx || (s + 1) % n == idx)
        };
        let is_tunnel_approach = |idx: usize| -> bool {
            route
                .underground_segments
                .iter()
                .any(|&s| (s + n - 1) % n == idx || (s + 2) % n == idx)
        };

        // Build lane-offset waypoints so vehicles stay in their right-hand lane on roads
        // and align with the ramp centerline when entering/exiting underground garages or tunnels.
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let p = route.waypoints[i];
            let is_garage_pt = !route.is_closed_loop && (i <= 1 || i + 2 >= n);
            if is_garage_pt || is_tunnel_mouth(i) || is_tunnel_approach(i) {
                pts.push(p);
                continue;
            }

            let prev = if i > 0 {
                route.waypoints[i - 1]
            } else if route.is_closed_loop {
                route.waypoints[n - 1]
            } else {
                p
            };
            let next = if i + 1 < n {
                route.waypoints[i + 1]
            } else if route.is_closed_loop {
                route.waypoints[0]
            } else {
                p
            };

            let d_in = [p[0] - prev[0], p[1] - prev[1]];
            let l_in = (d_in[0] * d_in[0] + d_in[1] * d_in[1]).sqrt();
            let r_in = if l_in > 0.01 {
                [-d_in[1] / l_in, d_in[0] / l_in]
            } else {
                [0.0, 0.0]
            };

            let d_out = [next[0] - p[0], next[1] - p[1]];
            let l_out = (d_out[0] * d_out[0] + d_out[1] * d_out[1]).sqrt();
            let r_out = if l_out > 0.01 {
                [-d_out[1] / l_out, d_out[0] / l_out]
            } else {
                [0.0, 0.0]
            };

            // For garage turnoff waypoints (i == 2 or i == n - 3), only offset perpendicular to the street
            let offset = if !route.is_closed_loop && i == 2 {
                [r_out[0] * lane_mag, r_out[1] * lane_mag]
            } else if !route.is_closed_loop && i + 3 == n {
                [r_in[0] * lane_mag, r_in[1] * lane_mag]
            } else {
                [
                    (r_in[0] + r_out[0]) * lane_mag,
                    (r_in[1] + r_out[1]) * lane_mag,
                ]
            };

            pts.push([p[0] + offset[0], p[1] + offset[1]]);
        }

        let seg_count = if route.is_closed_loop { n } else { n - 1 };
        let mut seg_lens = Vec::with_capacity(seg_count);
        let mut total_len = 0.0f32;
        for i in 0..seg_count {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2))
                .sqrt()
                .max(0.001);
            seg_lens.push(len);
            total_len += len;
        }

        let target_dist = if route.is_closed_loop {
            veh.progress.rem_euclid(1.0) * total_len
        } else {
            veh.progress.clamp(0.0, 1.0) * total_len
        };

        // Hide the vehicle while it is deep inside the subterranean building garage vault
        if !route.is_closed_loop {
            let start_vault_len = seg_lens[0] * 0.68;
            let end_vault_len = seg_lens[seg_count - 1] * 0.68;
            if target_dist < start_vault_len || target_dist > total_len - end_vault_len {
                return Some(([pts[0][0], -0.10, pts[0][1]], [1.0, 0.0], 0.0, true));
            }
        }

        // Locate segment and apply Quadratic Bezier corner rounding near intersections
        let mut rem = target_dist;
        let mut seg_idx = 0;
        for (i, &slen) in seg_lens.iter().enumerate() {
            if rem <= slen || i + 1 == seg_count {
                seg_idx = i;
                break;
            }
            rem -= slen;
        }

        // Hide vehicles while they traverse an unsimulated underground tunnel segment
        if route.underground_segments.contains(&seg_idx) {
            return Some((
                [pts[seg_idx][0], -0.45, pts[seg_idx][1]],
                [1.0, 0.0],
                0.0,
                true,
            ));
        }

        // Hide vehicles while they traverse an unsimulated underground tunnel segment
        if route.underground_segments.contains(&seg_idx) {
            return Some(([pts[seg_idx][0], -0.45, pts[seg_idx][1]], [1.0, 0.0], 0.0, true));
        }

        let a = pts[seg_idx];
        let b = pts[(seg_idx + 1) % n];
        let slen = seg_lens[seg_idx];
        let dir_curr = [(b[0] - a[0]) / slen, (b[1] - a[1]) / slen];

        let corner_r = 0.36f32;
        let can_round_start = (route.is_closed_loop || seg_idx > 0) && !is_tunnel_mouth(seg_idx);
        let can_round_end = (route.is_closed_loop || (seg_idx + 1 < seg_count))
            && !is_tunnel_mouth((seg_idx + 1) % n);

        let (pos_xz, mut tangent) = if can_round_start && rem < corner_r.min(slen * 0.45) {
            let prev_idx = (seg_idx + n - 1) % n;
            let prev_len = seg_lens[if seg_idx > 0 {
                seg_idx - 1
            } else {
                seg_count - 1
            }];
            let r = corner_r.min(slen * 0.45).min(prev_len * 0.45);
            let p_prev = pts[prev_idx];
            let dir_prev = [(a[0] - p_prev[0]) / prev_len, (a[1] - p_prev[1]) / prev_len];
            let p0 = [a[0] - dir_prev[0] * r, a[1] - dir_prev[1] * r];
            let p1 = a;
            let p2 = [a[0] + dir_curr[0] * r, a[1] + dir_curr[1] * r];
            let u = 0.5 + 0.5 * (rem / r.max(0.001)).clamp(0.0, 1.0);
            let om = 1.0 - u;
            let px = om * om * p0[0] + 2.0 * om * u * p1[0] + u * u * p2[0];
            let pz = om * om * p0[1] + 2.0 * om * u * p1[1] + u * u * p2[1];
            let tx = (1.0 - u) * dir_prev[0] + u * dir_curr[0];
            let tz = (1.0 - u) * dir_prev[1] + u * dir_curr[1];
            let tlen = (tx * tx + tz * tz).sqrt().max(0.001);
            ([px, pz], [tx / tlen, tz / tlen])
        } else if can_round_end && (slen - rem) < corner_r.min(slen * 0.45) {
            let next_seg = (seg_idx + 1) % seg_count;
            let next_len = seg_lens[next_seg];
            let r = corner_r.min(slen * 0.45).min(next_len * 0.45);
            let c = pts[(seg_idx + 2) % n];
            let dir_next = [(c[0] - b[0]) / next_len, (c[1] - b[1]) / next_len];
            let p0 = [b[0] - dir_curr[0] * r, b[1] - dir_curr[1] * r];
            let p1 = b;
            let p2 = [b[0] + dir_next[0] * r, b[1] + dir_next[1] * r];
            let u = 0.5 * ((rem - (slen - r)) / r.max(0.001)).clamp(0.0, 1.0);
            let om = 1.0 - u;
            let px = om * om * p0[0] + 2.0 * om * u * p1[0] + u * u * p2[0];
            let pz = om * om * p0[1] + 2.0 * om * u * p1[1] + u * u * p2[1];
            let tx = (1.0 - u) * dir_curr[0] + u * dir_next[0];
            let tz = (1.0 - u) * dir_curr[1] + u * dir_next[1];
            let tlen = (tx * tx + tz * tz).sqrt().max(0.001);
            ([px, pz], [tx / tlen, tz / tlen])
        } else {
            let t = (rem / slen).clamp(0.0, 1.0);
            (
                [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
                dir_curr,
            )
        };

        if !route.is_closed_loop && dir_sign < 0.0 {
            tangent = [-tangent[0], -tangent[1]];
        }

        // Compute vertical elevation `y` and ramp slope `dy / ds`
        let mut y = 0.025f32;
        let mut slope = 0.0f32;
        if route.waypoint_elevations.len() == n {
            let y0 = route.waypoint_elevations[seg_idx];
            let y1 = route.waypoint_elevations[(seg_idx + 1) % n];
            let t = (rem / slen).clamp(0.0, 1.0);
            y = y0 + (y1 - y0) * t;
            slope = ((y1 - y0) / slen) * dir_sign;
        } else if !route.is_closed_loop {
            // Smoothly slope elevation down (`y = 0.025 -> -0.06`) when on a subterranean garage ramp
            let ramp_span = seg_lens[0] + seg_lens[1];
            let end_ramp_span = seg_lens[seg_count - 1] + seg_lens[seg_count - 2];
            if target_dist < ramp_span {
                let ramp_t = (1.0 - (target_dist / ramp_span.max(0.01))).clamp(0.0, 1.0);
                y = 0.025 - ramp_t * 0.09;
            } else if target_dist > total_len - end_ramp_span {
                let ramp_t = ((target_dist - (total_len - end_ramp_span))
                    / end_ramp_span.max(0.01))
                .clamp(0.0, 1.0);
                y = 0.025 - ramp_t * 0.09;
            }
        }

        Some(([pos_xz[0], y, pos_xz[1]], tangent, slope, false))
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
    /// Functional wrought-iron intersection traffic signals.
    #[serde(default)]
    pub traffic_signals: Vec<TrafficSignal>,
    /// Current street traffic density preset.
    #[serde(default)]
    pub traffic_density: TrafficDensity,
    /// Remaining seconds of all-green express wave (`> 0.0` forces all signals green).
    #[serde(default)]
    pub green_wave_timer: f32,
    /// Remaining seconds of active high-speed Vigilante Interceptor patrol.
    #[serde(default)]
    pub pursuit_timer: f32,
    /// Countdown in seconds until the next automatic 5-minute Vigilante patrol.
    #[serde(default)]
    pub vigilante_cooldown_secs: f32,
    /// When true, the isometric camera smoothly tracks the active Vigilante or lead cruiser.
    #[serde(default)]
    pub follow_car: bool,
    /// Current smoothed camera look-at target `[x, y, z]`.
    #[serde(default)]
    pub camera_target: [f32; 3],
    /// Current smoothed camera zoom multiplier (`1.0` = overview, `2.35` = street follow).
    #[serde(default)]
    pub camera_zoom: f32,
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

        // Spawn 20 gothic vehicles across the 10 connected vehicle routes
        // (5 continuous highway/canyon loops + 5 subterranean garage-to-garage commuter routes)
        let mut vehicles = Vec::new();
        let mut veh_id = 1u32;
        let route_count = blueprint.vehicle_routes.len().max(1);
        for route_idx in 0..route_count {
            let is_closed = blueprint
                .vehicle_routes
                .get(route_idx)
                .map(|r| r.is_closed_loop)
                .unwrap_or(true);
            for slot in 0..2 {
                let dir = if is_closed || slot == 0 { 1.0 } else { -1.0 };
                let kind = match (route_idx + slot) % 4 {
                    0 => GothicVehicleKind::NoirCoupe,
                    1 => GothicVehicleKind::ArmoredCruiser,
                    2 => GothicVehicleKind::TransitOmnibus,
                    _ => GothicVehicleKind::SteamLimousine,
                };
                let base_prog = if slot == 0 {
                    0.18 + rng.f32() * 0.22
                } else {
                    0.62 + rng.f32() * 0.22
                };
                let speed = if is_closed {
                    rng.range(0.026, 0.040)
                } else {
                    rng.range(0.052, 0.078)
                };
                vehicles.push(GothicVehicle {
                    id: veh_id,
                    avenue_idx: route_idx,
                    progress: base_prog.fract(),
                    dir,
                    speed,
                    kind,
                    current_speed: speed,
                    is_braking: false,
                    turn_signal: 0,
                    afterburner: 0.0,
                    exhaust_accum: rng.range(0.0, 0.4),
                });
                veh_id += 1;
            }
        }

        // Spawn the hero VigilanteInterceptor parked inside Garage 1 (Route 5) ready for patrol
        let vigilante_route = 5.min(route_count.saturating_sub(1));
        vehicles.push(GothicVehicle {
            id: veh_id,
            avenue_idx: vigilante_route,
            progress: 0.0,
            dir: 1.0,
            speed: 0.085,
            kind: GothicVehicleKind::VigilanteInterceptor,
            current_speed: 0.0,
            is_braking: false,
            turn_signal: 0,
            afterburner: 0.0,
            exhaust_accum: 0.0,
        });

        // Functional wrought-iron traffic signals at the 6 primary inner-city junctions
        let signal_coords: [([f32; 2], f32); 6] = [
            ([-5.2, 0.2], 0.0),
            ([5.4, 0.2], 6.0),
            ([-0.4, -5.4], 3.0),
            ([0.7, 4.6], 9.0),
            ([-5.2, -5.4], 4.5),
            ([-5.2, 5.4], 1.5),
        ];
        let traffic_signals = signal_coords
            .iter()
            .enumerate()
            .map(|(i, &(pos, phase_offset))| TrafficSignal {
                id: i as u32 + 1,
                pos,
                phase_offset,
                ns_green: phase_offset < 6.0,
                is_amber: false,
            })
            .collect();

        // Spawn 2 gothic patrol airships cruising above the upper spires
        let airships = vec![
            GothicAirship {
                id: 1,
                orbit_center: [-0.4, -0.3],
                orbit_rx: 8.6,
                orbit_rz: 7.4,
                altitude: 14.2,
                angle: 0.65,
                angular_speed: 0.022,
                hull_length: 2.25,
                searchlight_phase: 0.0,
            },
            GothicAirship {
                id: 2,
                orbit_center: [0.5, 0.4],
                orbit_rx: 6.8,
                orbit_rz: 8.2,
                altitude: 15.6,
                angle: 3.8,
                angular_speed: -0.018,
                hull_length: 1.95,
                searchlight_phase: 2.1,
            },
        ];

        // No rooftop searchlights on buildings (only patrol airships project searchlights)
        let searchlights = Vec::new();

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
            screen_occupancy: 0.675,
            traffic_signals,
            traffic_density: TrafficDensity::NocturnalCruise,
            green_wave_timer: 0.0,
            pursuit_timer: 0.0,
            vigilante_cooldown_secs: 300.0,
            follow_car: false,
            camera_target: [0.0, 5.20, 0.0],
            camera_zoom: 1.0,
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
        // Station platform is along segment 0 (track_dist 1.8 .. 4.3, centered at z = 2.35 -> dist 3.05)
        let station_center_dist = 3.05f32;
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

        // 3. Update traffic signals, Vigilante 5-minute patrol timer, and street vehicle AI
        if self.green_wave_timer > 0.0 {
            self.green_wave_timer = (self.green_wave_timer - dt).max(0.0);
        }
        if self.pursuit_timer > 0.0 {
            self.pursuit_timer = (self.pursuit_timer - dt).max(0.0);
        } else {
            self.vigilante_cooldown_secs -= dt;
            if self.vigilante_cooldown_secs <= 0.0 {
                self.dispatch_interceptor();
            }
        }

        for sig in &mut self.traffic_signals {
            if self.green_wave_timer > 0.0 {
                sig.ns_green = true;
                sig.is_amber = false;
            } else {
                let cycle = (self.time + sig.phase_offset).rem_euclid(12.0);
                sig.ns_green = cycle < 6.0;
                sig.is_amber = (5.0..6.0).contains(&cycle) || (11.0..12.0).contains(&cycle);
            }
        }

        // Pre-sample world poses and slightly-ahead tangents for spacing, signals, and cornering
        let poses: Vec<_> = self
            .vehicles
            .iter()
            .map(|v| self.blueprint.sample_vehicle_pose(v))
            .collect();
        let ahead_tangents: Vec<Option<[f32; 2]>> = self
            .vehicles
            .iter()
            .map(|v| {
                let mut probe = v.clone();
                probe.progress = (probe.progress + probe.dir * 0.025).clamp(0.0, 1.0);
                self.blueprint
                    .sample_vehicle_pose(&probe)
                    .map(|(_, fwd, _)| fwd)
            })
            .collect();

        let route_count = self.blueprint.vehicle_routes.len();
        let veh_count = self.vehicles.len();
        let mut exhaust_spawns: Vec<([f32; 3], [f32; 2])> = Vec::new();

        for i in 0..veh_count {
            let is_closed = if route_count > 0 {
                self.blueprint
                    .vehicle_routes
                    .get(self.vehicles[i].avenue_idx % route_count)
                    .map(|r| r.is_closed_loop)
                    .unwrap_or(true)
            } else {
                true
            };

            let kind = self.vehicles[i].kind;
            let base_speed = self.vehicles[i].speed;

            // VigilanteInterceptor rests hidden inside a subterranean garage between 5-minute patrols
            if kind == GothicVehicleKind::VigilanteInterceptor
                && self.pursuit_timer <= 0.0
                && !is_closed
                && ((self.vehicles[i].progress <= 0.02
                    && (self.vehicles[i].dir < 0.0 || self.vehicles[i].current_speed == 0.0))
                    || (self.vehicles[i].progress >= 0.98 && self.vehicles[i].dir > 0.0))
            {
                if self.vehicles[i].progress >= 0.98 {
                    self.vehicles[i].progress = 1.0;
                    self.vehicles[i].dir = -1.0;
                } else {
                    self.vehicles[i].progress = 0.0;
                    self.vehicles[i].dir = 1.0;
                }
                self.vehicles[i].current_speed = 0.0;
                self.vehicles[i].is_braking = false;
                self.vehicles[i].turn_signal = 0;
                self.vehicles[i].afterburner = 0.0;
                continue;
            }

            let mut target_speed = base_speed;
            let mut braking = false;
            let mut turn_sig: i8 = 0;

            if let Some((pos, fwd, is_subterranean)) = poses[i]
                && !is_subterranean
            {
                // Highway speed boost on outer perimeter ring
                if pos[0].abs() > 9.8 || pos[2].abs() > 9.8 {
                    target_speed *= 1.35;
                }

                // Cornering deceleration & turn signal indicator
                if let Some(ahead_fwd) = ahead_tangents[i] {
                    let dot = fwd[0] * ahead_fwd[0] + fwd[1] * ahead_fwd[1];
                    if dot < 0.94 {
                        target_speed *= 0.62;
                        if self.vehicles[i].current_speed > target_speed * 1.12 {
                            braking = true;
                        }
                        let cross = fwd[0] * ahead_fwd[1] - fwd[1] * ahead_fwd[0];
                        turn_sig = if cross > 0.0 { 1 } else { -1 };
                    }
                }

                // Active Vigilante pursuit override
                let is_active_vigilante =
                    kind == GothicVehicleKind::VigilanteInterceptor && self.pursuit_timer > 0.0;
                if is_active_vigilante {
                    target_speed = base_speed * 2.05;
                }

                // Functional gothic traffic signals (only on ground-level streets; Vigilante blasts through)
                if pos[1] < 0.4 && !is_active_vigilante && self.green_wave_timer <= 0.0 {
                    let moving_ns = fwd[1].abs() >= fwd[0].abs();
                    for sig in &self.traffic_signals {
                        let dx = sig.pos[0] - pos[0];
                        let dz = sig.pos[1] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.38..=1.35).contains(&along) && perp < 0.55 {
                            let has_green = if moving_ns {
                                sig.ns_green && !sig.is_amber
                            } else {
                                !sig.ns_green && !sig.is_amber
                            };
                            if !has_green {
                                target_speed = 0.0;
                                braking = true;
                                break;
                            }
                        }
                    }
                }

                // Car-following collision avoidance
                for (j, other_pose) in poses.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    if let Some((opos, _, o_sub)) = other_pose {
                        if *o_sub || (opos[1] - pos[1]).abs() > 0.65 {
                            continue;
                        }
                        let dx = opos[0] - pos[0];
                        let dz = opos[2] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.15..=1.28).contains(&along) && perp < 0.34 {
                            target_speed = 0.0;
                            braking = true;
                            break;
                        }
                    }
                }

                // Periodic tailpipe steam puffs for vintage steam/turbine vehicles
                if matches!(
                    kind,
                    GothicVehicleKind::SteamLimousine
                        | GothicVehicleKind::TransitOmnibus
                        | GothicVehicleKind::VigilanteInterceptor
                ) {
                    self.vehicles[i].exhaust_accum += dt;
                    let interval = if kind == GothicVehicleKind::VigilanteInterceptor {
                        0.18
                    } else {
                        0.55
                    };
                    if self.vehicles[i].exhaust_accum >= interval {
                        self.vehicles[i].exhaust_accum = 0.0;
                        exhaust_spawns.push((pos, fwd));
                    }
                }
            }

            let veh = &mut self.vehicles[i];
            let accel_rate = if braking { 8.5 } else { 4.2 };
            veh.current_speed += (target_speed - veh.current_speed) * (dt * accel_rate).min(1.0);
            veh.is_braking = braking && veh.current_speed < base_speed * 0.9;
            veh.turn_signal = turn_sig;

            if kind == GothicVehicleKind::VigilanteInterceptor {
                let target_ab = if self.pursuit_timer > 0.0 { 1.0 } else { 0.15 };
                veh.afterburner += (target_ab - veh.afterburner) * (dt * 5.0).min(1.0);
            } else {
                veh.afterburner = 0.0;
            }

            let kind = self.vehicles[i].kind;
            let base_speed = self.vehicles[i].speed;

            // VigilanteInterceptor rests hidden inside a subterranean garage between 5-minute patrols
            if kind == GothicVehicleKind::VigilanteInterceptor
                && self.pursuit_timer <= 0.0
                && !is_closed
                && ((self.vehicles[i].progress <= 0.02
                    && (self.vehicles[i].dir < 0.0 || self.vehicles[i].current_speed == 0.0))
                    || (self.vehicles[i].progress >= 0.98 && self.vehicles[i].dir > 0.0))
            {
                if self.vehicles[i].progress >= 0.98 {
                    self.vehicles[i].progress = 1.0;
                    self.vehicles[i].dir = -1.0;
                } else {
                    self.vehicles[i].progress = 0.0;
                    self.vehicles[i].dir = 1.0;
                }
                self.vehicles[i].current_speed = 0.0;
                self.vehicles[i].is_braking = false;
                self.vehicles[i].turn_signal = 0;
                self.vehicles[i].afterburner = 0.0;
                continue;
            }

            let mut target_speed = base_speed;
            let mut braking = false;
            let mut turn_sig: i8 = 0;

            if let Some((pos, fwd, is_subterranean)) = poses[i]
                && !is_subterranean
            {
                // Highway speed boost on outer perimeter ring
                if pos[0].abs() > 9.8 || pos[2].abs() > 9.8 {
                    target_speed *= 1.35;
                }

                // Cornering deceleration & turn signal indicator
                if let Some(ahead_fwd) = ahead_tangents[i] {
                    let dot = fwd[0] * ahead_fwd[0] + fwd[1] * ahead_fwd[1];
                    if dot < 0.94 {
                        target_speed *= 0.62;
                        if self.vehicles[i].current_speed > target_speed * 1.12 {
                            braking = true;
                        }
                        let cross = fwd[0] * ahead_fwd[1] - fwd[1] * ahead_fwd[0];
                        turn_sig = if cross > 0.0 { 1 } else { -1 };
                    }
                }

                // Active Vigilante pursuit override
                let is_active_vigilante =
                    kind == GothicVehicleKind::VigilanteInterceptor && self.pursuit_timer > 0.0;
                if is_active_vigilante {
                    target_speed = base_speed * 2.05;
                }

                // Functional gothic traffic signals (only on ground-level streets; Vigilante blasts through)
                if pos[1] < 0.4 && !is_active_vigilante && self.green_wave_timer <= 0.0 {
                    let moving_ns = fwd[1].abs() >= fwd[0].abs();
                    for sig in &self.traffic_signals {
                        let dx = sig.pos[0] - pos[0];
                        let dz = sig.pos[1] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.38..=1.35).contains(&along) && perp < 0.55 {
                            let has_green = if moving_ns {
                                sig.ns_green && !sig.is_amber
                            } else {
                                !sig.ns_green && !sig.is_amber
                            };
                            if !has_green {
                                target_speed = 0.0;
                                braking = true;
                                break;
                            }
                        }
                    }
                }

                // Car-following collision avoidance (only yield to same-direction traffic,
                // never deadlock head-to-head on single-lane garage ramps)
                for (j, other_pose) in poses.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    if let Some((opos, ofwd, o_sub)) = other_pose {
                        if *o_sub {
                            continue;
                        }
                        let same_dir = fwd[0] * ofwd[0] + fwd[1] * ofwd[1];
                        if same_dir <= 0.35 {
                            continue;
                        }
                        let dx = opos[0] - pos[0];
                        let dz = opos[2] - pos[2];
                        let along = dx * fwd[0] + dz * fwd[1];
                        let perp = (-dx * fwd[1] + dz * fwd[0]).abs();
                        if (0.15..=1.28).contains(&along) && perp < 0.34 {
                            target_speed = 0.0;
                            braking = true;
                            break;
                        }
                    }
                }

                // Periodic tailpipe steam puffs for vintage steam/turbine vehicles
                if matches!(
                    kind,
                    GothicVehicleKind::SteamLimousine
                        | GothicVehicleKind::TransitOmnibus
                        | GothicVehicleKind::VigilanteInterceptor
                ) {
                    self.vehicles[i].exhaust_accum += dt;
                    let interval = if kind == GothicVehicleKind::VigilanteInterceptor {
                        0.18
                    } else {
                        0.55
                    };
                    if self.vehicles[i].exhaust_accum >= interval {
                        self.vehicles[i].exhaust_accum = 0.0;
                        exhaust_spawns.push((pos, fwd));
                    }
                }
            }

            let veh = &mut self.vehicles[i];
            let accel_rate = if braking { 8.5 } else { 4.2 };
            veh.current_speed += (target_speed - veh.current_speed) * (dt * accel_rate).min(1.0);
            veh.is_braking = braking && veh.current_speed < base_speed * 0.9;
            veh.turn_signal = turn_sig;

            if kind == GothicVehicleKind::VigilanteInterceptor {
                let target_ab = if self.pursuit_timer > 0.0 { 1.0 } else { 0.15 };
                veh.afterburner += (target_ab - veh.afterburner) * (dt * 5.0).min(1.0);
            } else {
                veh.afterburner = 0.0;
            }

            if is_closed {
                veh.progress = (veh.progress + veh.dir * veh.current_speed * dt).rem_euclid(1.0);
            } else {
                veh.progress += veh.dir * veh.current_speed * dt;
                if veh.progress >= 1.0 {
                    veh.progress = 1.0;
                    veh.dir = -1.0;
                } else if veh.progress <= 0.0 {
                    veh.progress = 0.0;
                    veh.dir = 1.0;
                }
            }
        }

        for (pos, fwd) in exhaust_spawns {
            if self.steam_plumes.len() < 78 {
                self.steam_plumes.push(SteamPlume {
                    pos: [
                        pos[0] - fwd[0] * 0.38,
                        pos[1] + 0.06,
                        pos[2] - fwd[1] * 0.38,
                    ],
                    vx: -fwd[0] * 0.18 + self.rng.range(-0.04, 0.04),
                    vy: self.rng.range(0.16, 0.28),
                    vz: -fwd[1] * 0.18 + self.rng.range(-0.04, 0.04),
                    age: 0.0,
                    max_age: 0.75,
                    base_radius: 0.055,
                });
            }
        }

        // Smooth Follow-Car camera tracking
        let (desired_target, desired_zoom) = if self.follow_car {
            let tracked = self
                .vehicles
                .iter()
                .find(|v| {
                    v.kind == GothicVehicleKind::VigilanteInterceptor && self.pursuit_timer > 0.0
                })
                .or_else(|| {
                    self.vehicles
                        .iter()
                        .find(|v| v.kind == GothicVehicleKind::ArmoredCruiser)
                })
                .or_else(|| self.vehicles.first());
            if let Some(v) = tracked
                && let Some((pos, _, is_sub)) = self.blueprint.sample_vehicle_pose(v)
                && !is_sub
            {
                ([pos[0], 0.45, pos[2]], 2.35)
            } else {
                ([0.0, 5.20, 0.0], 1.0)
            }
        } else {
            ([0.0, 5.20, 0.0], 1.0)
        };
        let cam_blend = (dt * 4.5).min(1.0);
        for (tgt, desired) in self.camera_target.iter_mut().zip(desired_target) {
            *tgt += (desired - *tgt) * cam_blend;
        }
        self.camera_zoom += (desired_zoom - self.camera_zoom) * cam_blend;

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

    /// Launch the Vigilante Interceptor on a high-speed afterburner patrol across the city.
    pub fn dispatch_interceptor(&mut self) {
        self.pursuit_timer = 26.0;
        self.vigilante_cooldown_secs = 300.0;
        if let Some(v) = self
            .vehicles
            .iter_mut()
            .find(|v| v.kind == GothicVehicleKind::VigilanteInterceptor)
        {
            v.afterburner = 1.0;
            v.current_speed = v.speed * 2.0;
            if v.progress <= 0.02 {
                v.dir = 1.0;
            }
        } else {
            let next_id = self.vehicles.iter().map(|v| v.id).max().unwrap_or(0) + 1;
            let route = 5.min(self.blueprint.vehicle_routes.len().saturating_sub(1));
            self.vehicles.push(GothicVehicle {
                id: next_id,
                avenue_idx: route,
                progress: 0.04,
                dir: 1.0,
                speed: 0.085,
                kind: GothicVehicleKind::VigilanteInterceptor,
                current_speed: 0.17,
                is_braking: false,
                turn_signal: 0,
                afterburner: 1.0,
                exhaust_accum: 0.0,
            });
        }
    }

    /// Toggle between Nocturnal Cruise and dense Midnight Rush Hour street traffic.
    pub fn toggle_rush_hour(&mut self) {
        match self.traffic_density {
            TrafficDensity::NocturnalCruise => {
                self.traffic_density = TrafficDensity::MidnightRushHour;
                let route_count = self.blueprint.vehicle_routes.len().max(1);
                let base_id = self.vehicles.iter().map(|v| v.id).max().unwrap_or(0) + 1;
                for route_idx in 0..route_count {
                    let is_closed = self
                        .blueprint
                        .vehicle_routes
                        .get(route_idx)
                        .map(|r| r.is_closed_loop)
                        .unwrap_or(true);
                    let kind = match route_idx % 4 {
                        0 => GothicVehicleKind::NoirCoupe,
                        1 => GothicVehicleKind::ArmoredCruiser,
                        2 => GothicVehicleKind::TransitOmnibus,
                        _ => GothicVehicleKind::SteamLimousine,
                    };
                    let speed = if is_closed {
                        self.rng.range(0.028, 0.042)
                    } else {
                        self.rng.range(0.050, 0.072)
                    };
                    self.vehicles.push(GothicVehicle {
                        id: base_id + route_idx as u32,
                        avenue_idx: route_idx,
                        progress: self.rng.range(0.32, 0.54),
                        dir: if is_closed || route_idx % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        },
                        speed,
                        kind,
                        current_speed: speed,
                        is_braking: false,
                        turn_signal: 0,
                        afterburner: 0.0,
                        exhaust_accum: 0.0,
                    });
                }
            }
            TrafficDensity::MidnightRushHour => {
                self.traffic_density = TrafficDensity::NocturnalCruise;
                let base_non_vigilante = self.blueprint.vehicle_routes.len().max(1) * 2;
                let mut kept = 0usize;
                self.vehicles.retain(|v| {
                    if v.kind == GothicVehicleKind::VigilanteInterceptor {
                        return true;
                    }
                    kept += 1;
                    kept <= base_non_vigilante
                });
            }
        }
    }

    /// Force all inner-city traffic signals green for 10 seconds so queued cars surge forward.
    pub fn trigger_green_wave(&mut self) {
        self.green_wave_timer = 10.0;
        for sig in &mut self.traffic_signals {
            sig.ns_green = true;
            sig.is_amber = false;
        }
    }

    /// Toggle smooth Follow-Car camera tracking of the Vigilante Interceptor or lead cruiser.
    pub fn toggle_follow_car(&mut self) {
        self.follow_car = !self.follow_car;
    }

    /// Summon an additional patrol airship (or boost airship patrol speed and emit steam).
    pub fn summon_airship(&mut self) {
        if self.airships.len() < 4 {
            let new_id = self.airships.len() as u32 + 1;
            let dir = if new_id.is_multiple_of(2) { -1.0 } else { 1.0 };
            self.airships.push(GothicAirship {
                id: new_id,
                orbit_center: [0.0, 0.0],
                orbit_rx: 6.2 + (new_id as f32 * 0.55),
                orbit_rz: 5.8 + (new_id as f32 * 0.45),
                altitude: 13.4 + (new_id as f32 * 0.65),
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
        assert_eq!(bp1.blocks.len(), 16, "4x4 layout must have 16 city blocks");
        assert!(bp1.buildings.len() >= 20);
        assert!(bp1.walkways.len() >= 14);
        assert!(bp1.viaduct_waypoints.len() >= 10);
        assert!(bp1.avenues.len() >= 16);
        assert_eq!(bp1.avenues.iter().filter(|a| a.is_highway).count(), 4);
        assert!(bp1.garages.len() >= 8);
        assert!(!bp1.elevated_highways.is_empty());
        assert!(bp1.underground_portals.len() >= 2);
        assert!(bp1.vehicle_routes.len() >= 11);
        assert!(bp1.viaduct_total_length() > 35.0);
    }

    #[test]
    fn vehicles_stay_within_plinth_and_use_garages_and_highway() {
        let mut state = GothicMetropolisState::new(42);
        let max_bound = state.blueprint.plinth_half_extent - 0.5;
        let mut saw_garage_subterranean = false;
        let mut saw_elevated_highway_car = false;
        let mut saw_tunnel_portal_subterranean = false;

        for _ in 0..360 {
            state.tick(0.1);
            for veh in &state.vehicles {
                let (pos, fwd, is_subterranean) = state
                    .blueprint
                    .sample_vehicle_pose(veh)
                    .expect("valid vehicle pose");
                let route_idx = veh.avenue_idx % state.blueprint.vehicle_routes.len();
                let has_underground_seg = !state.blueprint.vehicle_routes[route_idx]
                    .underground_segments
                    .is_empty();
                if is_subterranean {
                    saw_garage_subterranean = true;
                    if has_underground_seg {
                        saw_tunnel_portal_subterranean = true;
                    }
                } else {
                    if pos[1] > 1.5 {
                        saw_elevated_highway_car = true;
                    }
                    assert!(
                        pos[0].abs() <= max_bound && pos[2].abs() <= max_bound,
                        "Vehicle {} left city bounds at {:?}",
                        veh.id,
                        pos
                    );
                    let flen = (fwd[0] * fwd[0] + fwd[1] * fwd[1]).sqrt();
                    assert!((flen - 1.0).abs() < 0.05);
                }
            }
        }
        assert!(
            saw_garage_subterranean,
            "Garage commuter vehicles should enter and exit subterranean building garages"
        );
        assert!(
            saw_elevated_highway_car,
            "Vehicles on the elevated highway route should climb onto the 2nd-level overpass"
        );
        assert!(
            saw_tunnel_portal_subterranean,
            "Vehicles on underground tunnel routes should enter and exit sunken portals"
        );
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

    #[test]
    fn walkways_connect_to_building_facades_at_elevation() {
        let bp = CityBlueprint::default_gothic_metropolis();
        assert!(
            bp.walkways.len() >= 18,
            "Expected at least 18 multi-tier walkways"
        );

        for span in &bp.walkways {
            let b_start = bp
                .buildings
                .iter()
                .find(|b| b.id == span.start_building_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Walkway {} missing start_building_id {}",
                        span.id, span.start_building_id
                    )
                });
            let b_end = bp
                .buildings
                .iter()
                .find(|b| b.id == span.end_building_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Walkway {} missing end_building_id {}",
                        span.id, span.end_building_id
                    )
                });

            assert!(
                span.elevation + 0.4 <= b_start.height,
                "Walkway {} elevation {} is too close to or above start building {} height {}",
                span.id,
                span.elevation,
                b_start.id,
                b_start.height
            );
            assert!(
                span.elevation + 0.4 <= b_end.height,
                "Walkway {} elevation {} is too close to or above end building {} height {}",
                span.id,
                span.elevation,
                b_end.id,
                b_end.height
            );

            for (label, pt, bld) in [("start", span.start, b_start), ("end", span.end, b_end)] {
                let (x0, x1, z0, z1) = bld
                    .cross_section_bounds_at(span.elevation)
                    .expect("building cross section at walkway elevation");
                let eps = 0.03;
                let inside_x = pt[0] >= x0 - eps && pt[0] <= x1 + eps;
                let inside_z = pt[1] >= z0 - eps && pt[1] <= z1 + eps;
                let on_wall_x = (pt[0] - x0).abs() <= eps || (pt[0] - x1).abs() <= eps;
                let on_wall_z = (pt[1] - z0).abs() <= eps || (pt[1] - z1).abs() <= eps;
                assert!(
                    inside_x && inside_z && (on_wall_x || on_wall_z),
                    "Walkway {} {} point {:?} does not touch building {} ({:?}) facade bounds ({:.2}..{:.2}, {:.2}..{:.2}) at y={:.2}",
                    span.id,
                    label,
                    pt,
                    bld.id,
                    bld.archetype,
                    x0,
                    x1,
                    z0,
                    z1,
                    span.elevation
                );
            }
        }
    }

    #[test]
    fn buildings_exhibit_height_and_archetype_variety() {
        let bp = CityBlueprint::default_gothic_metropolis();
        let min_h = bp
            .buildings
            .iter()
            .map(|b| b.height)
            .fold(f32::INFINITY, f32::min);
        let max_h = bp.buildings.iter().map(|b| b.height).fold(0.0f32, f32::max);
        assert!(
            max_h >= 15.0 && (max_h - min_h) >= 10.5,
            "Buildings must span a dramatic vertical range (got {min_h:.1}..{max_h:.1})"
        );

        let required_archetypes = [
            BuildingArchetype::CathedralKeep,
            BuildingArchetype::ArtDecoZiggurat,
            BuildingArchetype::ClockworkBelfry,
            BuildingArchetype::IndustrialFoundry,
            BuildingArchetype::SyndicateTower,
            BuildingArchetype::ViaductStation,
            BuildingArchetype::OctagonalCitadel,
            BuildingArchetype::GothicMonumentPlaza,
            BuildingArchetype::AtlasDecoTower,
            BuildingArchetype::ChanceryArcadeKeep,
            BuildingArchetype::AlchemicalObservatory,
        ];
        for arch in required_archetypes {
            assert!(
                bp.buildings.iter().any(|b| b.archetype == arch),
                "Missing building archetype {:?}",
                arch
            );
        }
    }

    #[test]
    fn vigilante_auto_patrol_signals_rush_hour_and_follow_car() {
        let mut state = GothicMetropolisState::new(42);
        assert_eq!(state.traffic_signals.len(), 6);
        assert_eq!(state.traffic_density, TrafficDensity::NocturnalCruise);
        assert!(!state.follow_car);

        // Rush hour toggles density up and back down
        let base_count = state.vehicles.len();
        state.toggle_rush_hour();
        assert_eq!(state.traffic_density, TrafficDensity::MidnightRushHour);
        assert!(state.vehicles.len() > base_count);
        state.toggle_rush_hour();
        assert_eq!(state.traffic_density, TrafficDensity::NocturnalCruise);
        assert_eq!(state.vehicles.len(), base_count);

        // Green wave forces all signals green
        state.trigger_green_wave();
        assert!(state.green_wave_timer > 9.0);
        assert!(
            state
                .traffic_signals
                .iter()
                .all(|s| s.ns_green && !s.is_amber)
        );

        // Follow car smoothly shifts camera zoom & target
        state.toggle_follow_car();
        assert!(state.follow_car);
        for _ in 0..20 {
            state.tick(0.1);
        }
        assert!(state.camera_zoom > 1.5);

        // Vigilante auto-launches when 5-minute cooldown expires
        state.pursuit_timer = 0.0;
        state.vigilante_cooldown_secs = 0.05;
        state.tick(0.1);
        assert!(state.pursuit_timer > 20.0);
        let vigilante = state
            .vehicles
            .iter()
            .find(|v| v.kind == GothicVehicleKind::VigilanteInterceptor)
            .expect("vigilante vehicle exists");
        assert!(vigilante.afterburner > 0.5);
    }
}
