//! Authored 4x4 Gothic Metropolis city blueprint, precomputed route splines, and path sampling.

use serde::{Deserialize, Serialize};

use super::types::*;

/// Precomputed lane-offset polyline and segment lengths for one travel direction of a [`VehicleRoute`].
#[derive(Clone, Debug, PartialEq, Default)]
struct CachedLaneSpline {
    pts: Vec<[f32; 2]>,
    seg_lens: Vec<f32>,
    total_len: f32,
}

/// Cached forward (`dir >= 0`) and reverse (`dir < 0`) lane splines for a [`VehicleRoute`].
#[derive(Clone, Debug, PartialEq, Default)]
struct CachedRouteGeometry {
    forward: CachedLaneSpline,
    reverse: CachedLaneSpline,
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
    #[serde(skip)]
    route_cache: Vec<CachedRouteGeometry>,
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

        // 2nd-Level Elevated Highway Network: wraps around the West, South, and East outer
        // perimeter edges at realistic overpass height (y = 0.82) with one branch into the city.
        let elev_y = 0.82f32;
        let elevated_highways = vec![
            // Span 1: West outer edge (ramps up from y=0 at z=1.4, joins SW corner at [-10.6, 10.6])
            ElevatedHighwaySpan {
                id: 1,
                start: [-10.6, 1.4],
                end: [-10.6, 10.6],
                ramp_length: 2.4,
                elevation: elev_y,
                width: 1.18,
                start_has_ramp: true,
                end_has_ramp: false,
            },
            // Span 2: South outer edge (full elevated deck from SW corner to SE corner)
            ElevatedHighwaySpan {
                id: 2,
                start: [-10.6, 10.6],
                end: [10.6, 10.6],
                ramp_length: 2.4,
                elevation: elev_y,
                width: 1.18,
                start_has_ramp: false,
                end_has_ramp: false,
            },
            // Span 3: East outer edge (joins SE corner at [10.6, 10.6], ramps down to y=0 at z=1.6)
            ElevatedHighwaySpan {
                id: 3,
                start: [10.6, 10.6],
                end: [10.6, 1.6],
                ramp_length: 2.4,
                elevation: elev_y,
                width: 1.18,
                start_has_ramp: false,
                end_has_ramp: true,
            },
            // Span 4: City branch (T-junction at [0.7, 10.6], runs north over South Central Spine,
            // ramps down to y=0 at [0.7, 1.8] before Central Grand Plaza)
            ElevatedHighwaySpan {
                id: 4,
                start: [0.7, 10.6],
                end: [0.7, 1.8],
                ramp_length: 2.4,
                elevation: elev_y,
                width: 1.06,
                start_has_ramp: false,
                end_has_ramp: true,
            },
        ];

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
        let elev_car_y = elev_y + 0.025;
        let vehicle_routes = vec![
            // Route 1: Outer Perimeter Ring Highway (Clockwise Loop — climbs East ramp onto 2nd-level
            // wrap-around highway and descends West ramp, never driving through ramps on the ground)
            VehicleRoute {
                id: 1,
                waypoints: vec![
                    [-10.6, -10.6],
                    [10.6, -10.6],
                    [10.6, 1.6],
                    [10.6, 4.0],
                    [10.6, 10.6],
                    [-10.6, 10.6],
                    [-10.6, 3.8],
                    [-10.6, 1.4],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![
                    0.025, 0.025, 0.025, elev_car_y, elev_car_y, elev_car_y, elev_car_y, 0.025,
                ],
                underground_segments: Vec::new(),
            },
            // Route 2: Outer Perimeter Ring Highway (Counter-Clockwise Loop — climbs West ramp onto
            // 2nd-level wrap-around highway and descends East ramp)
            VehicleRoute {
                id: 2,
                waypoints: vec![
                    [-10.6, -10.6],
                    [-10.6, 1.4],
                    [-10.6, 3.8],
                    [-10.6, 10.6],
                    [10.6, 10.6],
                    [10.6, 4.0],
                    [10.6, 1.6],
                    [10.6, -10.6],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![
                    0.025, 0.025, elev_car_y, elev_car_y, elev_car_y, elev_car_y, 0.025, 0.025,
                ],
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
            // Route 5: Cross-Metropolis Boulevard -> City Branch Ramp -> South/West 2nd-Level Highway
            VehicleRoute {
                id: 5,
                waypoints: vec![
                    [-10.6, -5.4],
                    [-0.4, -5.4],
                    [-0.4, 0.2],
                    [0.7, 0.2],
                    [0.7, 1.8],
                    [0.7, 4.2],
                    [0.7, 10.6],
                    [-10.6, 10.6],
                    [-10.6, 3.8],
                    [-10.6, 1.4],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![
                    0.025, 0.025, 0.025, 0.025, 0.025, elev_car_y, elev_car_y, elev_car_y,
                    elev_car_y, 0.025,
                ],
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
            // Uses Mid-West Avenue (z = -0.70) to stay clear of the West elevated highway ramp.
            VehicleRoute {
                id: 10,
                waypoints: vec![
                    [-8.45, -3.05],
                    [-9.11, -3.05],
                    [-10.60, -3.05],
                    [-10.60, -0.70],
                    [-5.20, -0.70],
                    [-5.20, 5.40],
                    [0.70, 5.40],
                    [0.70, 7.56],
                    [2.20, 7.56],
                    [2.85, 7.56],
                ],
                is_closed_loop: false,
                waypoint_elevations: Vec::new(),
                underground_segments: Vec::new(),
            },
            // Route 11: 2nd-Level East Ramp -> South Deck -> City Branch Express Loop
            VehicleRoute {
                id: 11,
                waypoints: vec![
                    [10.6, -10.6],
                    [10.6, 1.6],
                    [10.6, 4.0],
                    [10.6, 10.6],
                    [0.7, 10.6],
                    [0.7, 4.2],
                    [0.7, 1.8],
                    [0.7, 0.2],
                    [-0.4, 0.2],
                    [-0.4, -10.6],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![
                    0.025, 0.025, elev_car_y, elev_car_y, elev_car_y, elev_car_y, 0.025, 0.025,
                    0.025, 0.025,
                ],
                underground_segments: Vec::new(),
            },
            // Route 12: Subterranean Tunnel Loop (Portal 1 East Outer -> Portal 2 SW Avenue)
            // Returns via East Mid Canyon (x = 5.40, z = 1.10) to avoid the East elevated highway ramp.
            VehicleRoute {
                id: 12,
                waypoints: vec![
                    [10.60, 0.05],
                    [8.80, 0.05],
                    [-3.35, 7.20],
                    [-3.35, 5.40],
                    [0.70, 5.40],
                    [0.70, 4.60],
                    [5.40, 4.60],
                    [5.40, 1.10],
                    [10.60, 1.10],
                ],
                is_closed_loop: true,
                waypoint_elevations: vec![
                    0.025, -0.22, -0.22, 0.025, 0.025, 0.025, 0.025, 0.025, 0.025,
                ],
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

        let mut bp = Self {
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
            route_cache: Vec::new(),
        };
        bp.rebuild_route_cache();
        bp
    }

    /// Precompute forward and reverse lane-offset splines for all `vehicle_routes`.
    pub fn rebuild_route_cache(&mut self) {
        self.route_cache = self
            .vehicle_routes
            .iter()
            .map(|route| CachedRouteGeometry {
                forward: Self::build_lane_spline(route, 1.0),
                reverse: Self::build_lane_spline(route, -1.0),
            })
            .collect();
    }

    fn build_lane_spline(route: &VehicleRoute, dir_sign: f32) -> CachedLaneSpline {
        let n = route.waypoints.len();
        if n < 2 {
            return CachedLaneSpline::default();
        }
        let lane_mag = 0.23f32 * dir_sign;
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

        CachedLaneSpline {
            pts,
            seg_lens,
            total_len,
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

        let route_idx = veh.avenue_idx % self.vehicle_routes.len();
        let route = self.vehicle_routes.get(route_idx)?;
        let n = route.waypoints.len();
        if n < 2 {
            return None;
        }

        let dir_sign = if veh.dir >= 0.0 { 1.0f32 } else { -1.0f32 };

        // Check if waypoint `i` is a portal mouth for an underground tunnel segment.
        let is_tunnel_mouth = |idx: usize| -> bool {
            route
                .underground_segments
                .iter()
                .any(|&s| s == idx || (s + 1) % n == idx)
        };

        let fallback_spline;
        let spline = if let Some(cached) = self.route_cache.get(route_idx) {
            if dir_sign >= 0.0 {
                &cached.forward
            } else {
                &cached.reverse
            }
        } else {
            fallback_spline = Self::build_lane_spline(route, dir_sign);
            &fallback_spline
        };

        let pts = &spline.pts;
        let seg_lens = &spline.seg_lens;
        let seg_count = seg_lens.len();
        let total_len = spline.total_len;

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
