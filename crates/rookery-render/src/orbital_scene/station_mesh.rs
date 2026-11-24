//! Static 3D architecture for Station HELIOS-09 (exterior hull + 3 seamless live-windowed interiors).
//!
//! All rooms exist inside the true 3D station coordinate space so windows and the open hangar bay
//! look directly out onto live spacecraft, orbiting camera drones, and the planetary horizon.

use std::f32::consts::TAU;

use rookery_core::orbital::{PAD_A_POS, PAD_B_POS, PAD_C_POS};

use super::mesh_builder::{MeshBuilder, OrbitalVertex};
use super::textures::Tile;
use super::theme::OrbitalTheme;

pub struct StaticStationMeshes {
    pub opaque_verts: Vec<OrbitalVertex>,
    pub opaque_indices: Vec<u32>,
    pub trans_verts: Vec<OrbitalVertex>,
    pub trans_indices: Vec<u32>,
}

pub fn build_static_station(theme: &OrbitalTheme) -> StaticStationMeshes {
    let mut op = MeshBuilder::with_capacity(14_000, 22_000);
    let mut tr = MeshBuilder::with_capacity(1_024, 1_536);

    let hull = theme.hull_tint;
    let rib = theme.rib_tint;
    let corr = theme.corridor_tint;
    let dark_hull = [hull[0] * 0.62, hull[1] * 0.62, hull[2] * 0.65, 1.0];
    let bright_white = [0.96, 0.98, 1.0, 1.0];
    let accent_em = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let sec_em = [
        theme.accent_secondary[0],
        theme.accent_secondary[1],
        theme.accent_secondary[2],
        1.0,
    ];

    // =========================================================================
    // 1. MAIN HANGAR BAY (PAD A) — Seamless Interior (`X:-12..12, Y:-4..10, Z:12..42`)
    // =========================================================================
    // Hangar floor slab & grated deck
    op.push_box(
        [0.0, -4.5, 27.0],
        [24.0, 1.2, 30.0],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    // Pad A recessed landing deck markings
    op.push_box(
        [PAD_A_POS[0], -3.82, PAD_A_POS[2]],
        [13.5, 0.22, 16.5],
        Tile::LandingPad,
        4.5,
        bright_white,
        0.0,
    );
    // Hazard border strips along Pad A edges
    op.push_box(
        [-7.3, -3.80, 26.0],
        [1.1, 0.24, 17.5],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [7.3, -3.80, 26.0],
        [1.1, 0.24, 17.5],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [0.0, -3.80, 41.2],
        [22.0, 0.26, 1.4],
        Tile::HazardStripe,
        2.2,
        bright_white,
        0.0,
    );

    // Hangar roof slab & ceiling fluorescent light strips
    op.push_box(
        [0.0, 10.6, 27.0],
        [25.6, 1.4, 30.0],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    for z_light in [17.0, 24.0, 31.0, 38.0] {
        op.push_box(
            [-6.0, 9.82, z_light],
            [3.5, 0.2, 1.4],
            Tile::CeilingLight,
            3.5,
            bright_white,
            0.9,
        );
        op.push_box(
            [6.0, 9.82, z_light],
            [3.5, 0.2, 1.4],
            Tile::CeilingLight,
            3.5,
            bright_white,
            0.9,
        );
    }

    // Hangar Port (-X) and Starboard (+X) armored side walls
    op.push_box(
        [-12.5, 3.0, 27.0],
        [1.4, 14.2, 30.0],
        Tile::CorridorWall,
        5.0,
        corr,
        0.0,
    );
    op.push_box(
        [12.5, 3.0, 27.0],
        [1.4, 14.2, 30.0],
        Tile::CorridorWall,
        5.0,
        corr,
        0.0,
    );

    // Hangar Rear Bulkhead (Z = 12) with giant reinforced Blast Door
    op.push_box(
        [0.0, 3.0, 11.6],
        [24.0, 14.2, 1.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 1.2, 12.45],
        [10.0, 9.2, 0.6],
        Tile::BlastDoor,
        9.5,
        bright_white,
        0.0,
    );
    // Hangar control booth window above blast door
    op.push_box(
        [0.0, 7.6, 12.5],
        [9.0, 2.2, 0.5],
        Tile::CrtConsole,
        3.0,
        accent_em,
        0.8,
    );

    // Heavy Y2K structural bulkhead ribs arching over the Hangar Bay
    for z_rib in [15.0, 21.5, 28.0, 34.5, 41.0] {
        op.push_box(
            [-11.2, 3.0, z_rib],
            [1.4, 13.8, 1.5],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
        op.push_box(
            [11.2, 3.0, z_rib],
            [1.4, 13.8, 1.5],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
        op.push_box(
            [0.0, 9.3, z_rib],
            [23.6, 1.4, 1.5],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
    }

    // Overhead yellow industrial gantry crane beam inside hangar
    op.push_box(
        [0.0, 8.2, 25.0],
        [21.0, 0.9, 1.2],
        Tile::HazardStripe,
        2.4,
        bright_white,
        0.0,
    );

    // Stacked industrial cargo crates in hangar side alcoves
    let crate_spots = [
        ([-9.6, -2.9, 16.5], [2.2, 2.0, 2.4]),
        ([-9.6, -2.9, 19.6], [2.0, 1.8, 2.0]),
        ([-9.6, -1.2, 16.8], [1.8, 1.4, 1.8]),
        ([9.6, -2.9, 17.5], [2.4, 2.1, 2.4]),
        ([9.6, -2.9, 34.5], [2.2, 1.9, 2.2]),
        ([-9.6, -2.9, 36.0], [2.0, 1.7, 2.2]),
    ];
    for (pos, sz) in crate_spots {
        op.push_box(pos, sz, Tile::CargoCrate, 2.2, bright_white, 0.0);
    }

    // Translucent Cyan Atmospheric Containment Forcefield across the open Hangar Mouth (Z = 41.8)
    tr.push_quad(
        [-11.5, -3.7, 41.8],
        [11.5, -3.7, 41.8],
        [11.5, 9.6, 41.8],
        [-11.5, 9.6, 41.8],
        [0.0, 0.0, 1.0],
        [6.0, 3.5],
        Tile::ForcefieldHex,
        theme.forcefield_tint,
        1.8,
    );
    tr.push_quad(
        [11.5, -3.7, 41.8],
        [-11.5, -3.7, 41.8],
        [-11.5, 9.6, 41.8],
        [11.5, 9.6, 41.8],
        [0.0, 0.0, -1.0],
        [6.0, 3.5],
        Tile::ForcefieldHex,
        theme.forcefield_tint,
        1.8,
    );

    // =========================================================================
    // 2. CENTRAL OPS & COMMAND SPIRE — Seamless Windowed Interior (`Y: +12..+21`)
    // =========================================================================
    // Floor slab & ceiling slab
    op.push_box(
        [0.0, 11.4, 0.0],
        [22.0, 1.2, 22.0],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, 21.4, 0.0],
        [23.0, 1.4, 23.0],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    // Ceiling fluorescent strips inside Ops Deck
    for x_l in [-5.0, 5.0] {
        op.push_box(
            [x_l, 20.62, 0.0],
            [1.2, 0.18, 12.0],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.9,
        );
    }

    // Rear (-Z) solid bulkhead wall with Server Racks & Blast Door
    op.push_box(
        [0.0, 16.4, -10.8],
        [22.0, 9.4, 1.2],
        Tile::CorridorWall,
        4.5,
        corr,
        0.0,
    );
    op.push_box(
        [0.0, 15.2, -10.0],
        [4.2, 6.2, 0.4],
        Tile::BlastDoor,
        4.5,
        bright_white,
        0.0,
    );
    for sx in [-7.2, -4.4, 4.4, 7.2] {
        op.push_box(
            [sx, 15.4, -9.8],
            [2.1, 6.4, 0.9],
            Tile::ServerRack,
            2.2,
            bright_white,
            0.72,
        );
    }

    // Open Armored Viewports on +Z (front), -X (port), and +X (starboard) looking out to space & pads!
    // Lower sill walls (Y = 12.0..13.8) + upper header fascia (Y = 19.3..20.8) + corner & mullion pillars
    op.push_box(
        [0.0, 12.9, 10.6],
        [21.6, 1.8, 1.0],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [0.0, 20.0, 10.6],
        [21.6, 1.5, 1.0],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );
    op.push_box(
        [-10.6, 12.9, 0.0],
        [1.0, 1.8, 21.6],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [-10.6, 20.0, 0.0],
        [1.0, 1.5, 21.6],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );
    op.push_box(
        [10.6, 12.9, 0.0],
        [1.0, 1.8, 21.6],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [10.6, 20.0, 0.0],
        [1.0, 1.5, 21.6],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );

    // Chunky vertical window mullions around Ops Deck (leaving wide open glass sightlines!)
    let mullions = [
        [-10.6, 10.6],
        [-3.6, 10.6],
        [3.6, 10.6],
        [10.6, 10.6],
        [-10.6, 0.0],
        [10.6, 0.0],
        [-10.6, -10.6],
        [10.6, -10.6],
    ];
    for [mx, mz] in mullions {
        op.push_box(
            [mx, 16.5, mz],
            [1.2, 8.8, 1.2],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
    }

    // Central Holographic Tactical Table Pedestal (lifted cleanly above floor Y=12.0 to prevent Z-fighting)
    op.push_cylinder(
        [0.0, 12.82, 1.0],
        2.9,
        2.5,
        1.56,
        8,
        Tile::DarkSteelRib,
        rib,
        0.0,
    );
    op.push_cylinder(
        [0.0, 13.70, 1.0],
        2.2,
        2.2,
        0.16,
        8,
        Tile::CrtConsole,
        accent_em,
        1.0,
    );

    // Curved ring of glowing Y2K CRT console desks facing the front viewport
    let desks = [
        ([-5.4, 13.2, 6.8], [3.6, 2.2, 1.6]),
        ([0.0, 13.2, 7.6], [4.2, 2.2, 1.6]),
        ([5.4, 13.2, 6.8], [3.6, 2.2, 1.6]),
        ([-7.2, 13.2, 1.2], [1.6, 2.2, 3.8]),
        ([7.2, 13.2, 1.2], [1.6, 2.2, 3.8]),
    ];
    for (pos, sz) in desks {
        op.push_box(pos, sz, Tile::DarkSteelRib, 2.0, rib, 0.0);
        // Glowing angled CRT bank sitting on top of the desk
        op.push_box(
            [pos[0], pos[1] + 1.35, pos[2]],
            [sz[0] * 0.86, 1.15, sz[2] * 0.65],
            Tile::CrtConsole,
            1.6,
            accent_em,
            0.95,
        );
    }

    // =========================================================================
    // 3. CONCOURSE CORRIDOR & REACTOR CORE — Seamless Interior (`Y: -15..-5.5`)
    // =========================================================================
    // Lower deck floor & ceiling
    op.push_box(
        [0.0, -15.6, 0.0],
        [38.0, 1.2, 20.0],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, -4.9, 0.0],
        [38.0, 1.2, 20.0],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    // End walls at X = -19 and X = +19 with Blast Doors
    op.push_box(
        [-18.8, -10.2, 0.0],
        [1.2, 10.0, 20.0],
        Tile::CorridorWall,
        4.5,
        corr,
        0.0,
    );
    op.push_box(
        [18.8, -10.2, 0.0],
        [1.2, 10.0, 20.0],
        Tile::CorridorWall,
        4.5,
        corr,
        0.0,
    );
    op.push_box(
        [-18.0, -11.6, 0.0],
        [0.4, 6.4, 5.0],
        Tile::BlastDoor,
        5.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [18.0, -11.6, 0.0],
        [0.4, 6.4, 5.0],
        Tile::BlastDoor,
        5.0,
        bright_white,
        0.0,
    );

    // Tall observation windows along Z = +9.8 and Z = -9.8 looking out to deep space & passing ships!
    for z_side in [-9.8f32, 9.8f32] {
        op.push_box(
            [0.0, -14.1, z_side],
            [37.0, 1.8, 0.9],
            Tile::CorridorWall,
            4.0,
            corr,
            0.0,
        );
        op.push_box(
            [0.0, -6.2, z_side],
            [37.0, 1.6, 0.9],
            Tile::DarkSteelRib,
            4.0,
            rib,
            0.0,
        );
        for x_col in [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0] {
            op.push_box(
                [x_col, -10.2, z_side],
                [1.1, 9.4, 1.1],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
        }
    }

    // Overhead corridor fluorescent light strips & exposed pipes
    for x_l in [-13.0, -6.5, 6.5, 13.0] {
        op.push_box(
            [x_l, -5.65, 5.2],
            [3.6, 0.2, 0.9],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.9,
        );
        op.push_box(
            [x_l, -5.65, -5.2],
            [3.6, 0.2, 0.9],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.9,
        );
    }
    // Reactor containment collars (top & bottom around central plasma column)
    op.push_cylinder(
        [0.0, -14.4, 0.0],
        3.8,
        3.4,
        1.2,
        8,
        Tile::HazardStripe,
        bright_white,
        0.0,
    );
    op.push_cylinder(
        [0.0, -6.1, 0.0],
        3.4,
        3.8,
        1.2,
        8,
        Tile::HazardStripe,
        bright_white,
        0.0,
    );
    // Mainframe server racks flanking the reactor chamber
    for x_r in [-10.5, 10.5] {
        op.push_box(
            [x_r, -11.5, -2.5],
            [1.8, 6.4, 2.8],
            Tile::ServerRack,
            2.2,
            bright_white,
            0.72,
        );
        op.push_box(
            [x_r, -11.5, 2.5],
            [1.8, 6.4, 2.8],
            Tile::ServerRack,
            2.2,
            bright_white,
            0.72,
        );
    }

    // =========================================================================
    // 4. EXTERNAL LANDING PADS B (PORT) & C (STARBOARD) + TRUSSES
    // =========================================================================
    for (pad_pos, side) in [(PAD_B_POS, -1.0f32), (PAD_C_POS, 1.0f32)] {
        let px = pad_pos[0];
        let py = pad_pos[1] - 0.8;
        let pz = pad_pos[2];

        // Cantilevered horizontal main steel truss connecting central core to pad
        op.push_box(
            [side * 22.0, -2.2, 0.0],
            [18.0, 3.2, 6.5],
            Tile::DarkSteelRib,
            4.0,
            rib,
            0.0,
        );
        op.push_box(
            [side * 22.0, -0.3, 0.0],
            [18.0, 0.6, 4.2],
            Tile::FloorGrate,
            3.0,
            hull,
            0.0,
        );

        // Octagonal-profile cantilevered Landing Pad platform
        op.push_cylinder(
            [px, py - 0.6, pz],
            11.8,
            12.2,
            2.0,
            8,
            Tile::DarkSteelRib,
            rib,
            0.0,
        );
        op.push_box(
            [px, py + 0.45, pz],
            [16.0, 0.25, 16.0],
            Tile::LandingPad,
            5.0,
            bright_white,
            0.0,
        );
        // Hazard perimeter strips
        op.push_box(
            [px, py + 0.48, pz + 8.3],
            [15.0, 0.26, 1.1],
            Tile::HazardStripe,
            2.0,
            bright_white,
            0.0,
        );
        op.push_box(
            [px, py + 0.48, pz - 8.3],
            [15.0, 0.26, 1.1],
            Tile::HazardStripe,
            2.0,
            bright_white,
            0.0,
        );

        // Blast deflector shield on inner edge
        op.push_box(
            [px - side * 9.5, py + 2.6, pz],
            [1.4, 4.4, 11.0],
            Tile::BlastDoor,
            4.5,
            hull,
            0.0,
        );

        // Twin sodium floodlight pylons at outer pad corners
        for z_off in [-8.2f32, 8.2f32] {
            let tx = px + side * 8.2;
            op.push_box(
                [tx, py + 4.5, pz + z_off],
                [1.1, 8.5, 1.1],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
            op.push_box(
                [tx - side * 0.5, py + 8.6, pz + z_off],
                [1.4, 1.1, 1.8],
                Tile::CeilingLight,
                1.5,
                sec_em,
                1.2,
            );
        }

        // Pad-side cargo crates
        op.push_box(
            [px - side * 6.8, py + 1.4, pz + 6.5],
            [2.2, 1.9, 2.2],
            Tile::CargoCrate,
            2.2,
            bright_white,
            0.0,
        );
    }

    // =========================================================================
    // 5. EXTERIOR SUPERSTRUCTURE, COMMS SPIRE, SOLAR WINGS & RADIATORS
    // =========================================================================
    // Central core hull casing between Concourse and Ops Deck (around X:-12..12, Y:-4..11, Z:-14..12)
    op.push_box(
        [0.0, 3.5, -2.0],
        [24.0, 15.0, 26.0],
        Tile::HullPanel,
        6.0,
        hull,
        0.0,
    );
    // Aft Rotating-Ring Bearing Hub & Recessed Collar Track (Z = -15..-26, ring spins at Z = -20)
    // Heavy central axle core passing through the ring bearing
    op.push_box(
        [0.0, 0.0, -20.0],
        [11.5, 11.5, 11.0],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );
    // Fore & aft armored collar flanges framing the rotating ring's track slot (Z = -16.4 and -23.6)
    op.push_box(
        [0.0, 0.0, -16.4],
        [14.8, 14.8, 1.4],
        Tile::HazardStripe,
        3.2,
        bright_white,
        0.0,
    );
    op.push_box(
        [0.0, 0.0, -23.6],
        [14.8, 14.8, 1.4],
        Tile::HazardStripe,
        3.2,
        bright_white,
        0.0,
    );

    // Aft Engineering & Solar Spine Block (Z = -25..-43 — completely clear behind the rotating ring!)
    op.push_box(
        [0.0, 0.5, -34.0],
        [14.0, 12.0, 18.0],
        Tile::HullPanel,
        6.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 7.0, -34.0],
        [11.0, 1.2, 15.0],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );

    // Dorsal Communications & Sensor Mast atop Ops Spire (Y = 22..44)
    op.push_cylinder(
        [0.0, 28.0, -3.0],
        3.4,
        1.8,
        12.0,
        8,
        Tile::DarkSteelRib,
        rib,
        0.0,
    );
    op.push_cylinder(
        [0.0, 38.0, -3.0],
        1.1,
        0.5,
        10.0,
        6,
        Tile::HullPanel,
        hull,
        0.0,
    );
    // Crown warning strobe housing
    op.push_box(
        [0.0, 43.3, -3.0],
        [1.4, 1.4, 1.4],
        Tile::CeilingLight,
        1.4,
        sec_em,
        1.3,
    );

    // Ventral Sensor Keel below Reactor Deck (Y = -16..-34)
    op.push_cylinder(
        [0.0, -23.0, 0.0],
        6.5,
        2.6,
        14.0,
        8,
        Tile::HullPanel,
        dark_hull,
        0.0,
    );
    op.push_cylinder(
        [0.0, -32.0, 0.0],
        1.4,
        0.6,
        6.0,
        6,
        Tile::DarkSteelRib,
        rib,
        0.0,
    );

    // Massive Articulated Solar Collector Wings mounted far aft at Z = -38 (clear of the Z = -20 ring!)
    for side in [-1.0f32, 1.0f32] {
        op.push_box(
            [side * 24.0, 2.5, -38.0],
            [32.0, 1.4, 1.8],
            Tile::DarkSteelRib,
            4.0,
            rib,
            0.0,
        );
        for wing_i in 0..2 {
            let wx = side * (18.5 + wing_i as f32 * 13.5);
            op.push_box(
                [wx, 2.5, -38.0],
                [10.5, 0.35, 16.0],
                Tile::SolarCells,
                4.5,
                bright_white,
                0.0,
            );
        }
        // Glowing Thermal Radiator Fin Arrays mounted aft at Z = -34 (clear of the ring!)
        op.push_box(
            [side * 12.0, -7.5, -34.0],
            [10.5, 0.35, 10.0],
            Tile::RadiatorLouvers,
            4.0,
            sec_em,
            0.65,
        );
    }

    // Exterior Hangar Mouth armored collar & glowing docking guide rails
    op.push_box(
        [-13.2, 3.0, 42.2],
        [2.2, 15.8, 2.2],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );
    op.push_box(
        [13.2, 3.0, 42.2],
        [2.2, 15.8, 2.2],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );
    op.push_box(
        [0.0, 11.0, 42.2],
        [28.6, 2.2, 2.4],
        Tile::HazardStripe,
        3.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [0.0, -4.6, 42.2],
        [28.6, 2.0, 2.4],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );

    // =========================================================================
    // 6. STYLIZED LOW-POLY PLANET, ORBITAL RING & DEEP-SPACE STARFIELD
    // =========================================================================
    let planet_center = [-160.0, -190.0, -260.0];
    op.push_sphere(
        planet_center,
        155.0,
        12,
        20,
        Tile::HullPanel,
        theme.planet_north,
        theme.planet_equator,
        0.55,
    );

    // Stylized faceted planetary ring band
    let ring_segs = 28;
    let r_in = 190.0;
    let r_out = 235.0;
    for i in 0..ring_segs {
        let a0 = (i as f32 / ring_segs as f32) * TAU;
        let a1 = ((i + 1) as f32 / ring_segs as f32) * TAU;
        let pt = |r: f32, a: f32| -> [f32; 3] {
            let x = a.cos() * r;
            let z = a.sin() * r;
            let y = x * 0.28 + z * 0.14;
            [
                planet_center[0] + x,
                planet_center[1] + y,
                planet_center[2] + z,
            ]
        };
        tr.push_quad(
            pt(r_in, a0),
            pt(r_out, a0),
            pt(r_out, a1),
            pt(r_in, a1),
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            Tile::ForcefieldHex,
            [
                theme.accent_primary[0] * 0.55,
                theme.accent_primary[1] * 0.55,
                theme.accent_primary[2] * 0.65,
                0.24,
            ],
            1.6,
        );
    }

    // Deterministic 3D deep-space starfield cubes at radius ~360..420
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next_f = || -> f32 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s & 0xFFFF) as f32 / 65535.0
    };
    for _ in 0..160 {
        let th = next_f() * TAU;
        let z_u = next_f() * 2.0 - 1.0;
        let r_xy = (1.0 - z_u * z_u).max(0.0).sqrt();
        let dist = 360.0 + next_f() * 60.0;
        let pos = [r_xy * th.cos() * dist, z_u * dist, r_xy * th.sin() * dist];
        let sz = 1.1 + next_f() * 1.4;
        let tint = if next_f() > 0.75 {
            [0.75, 0.90, 1.0, 1.0]
        } else if next_f() > 0.8 {
            [1.0, 0.88, 0.68, 1.0]
        } else {
            [0.88, 0.92, 0.98, 1.0]
        };
        op.push_box(pos, [sz, sz, sz], Tile::CeilingLight, sz, tint, 1.1);
    }

    StaticStationMeshes {
        opaque_verts: op.verts,
        opaque_indices: op.indices,
        trans_verts: tr.verts,
        trans_indices: tr.indices,
    }
}
