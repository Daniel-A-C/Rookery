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
    // Hazard border strips along Pad A edges (offset cleanly from deck edge at ±6.75)
    op.push_box(
        [-7.35, -3.78, 26.0],
        [1.1, 0.24, 17.5],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [7.35, -3.78, 26.0],
        [1.1, 0.24, 17.5],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [0.0, -3.77, 41.0],
        [21.6, 0.26, 1.4],
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

    // Hangar Rear Bulkhead (Z = 11.6) split around a real 6.4x7.1 doorway opening into the Central Core Atrium!
    op.push_box(
        [-7.6, 3.0, 11.6],
        [8.8, 14.2, 1.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [7.6, 3.0, 11.6],
        [8.8, 14.2, 1.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 6.65, 11.6],
        [6.4, 6.9, 1.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    // Hazard-striped doorway frame around the Hangar <-> Atrium sliding blast door
    op.push_box(
        [0.0, 3.25, 12.25],
        [6.8, 0.5, 0.4],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    // Hangar control booth window above doorway
    op.push_box(
        [0.0, 7.6, 12.45],
        [9.0, 2.2, 0.45],
        Tile::CrtConsole,
        3.0,
        accent_em,
        0.8,
    );

    // Heavy Y2K structural bulkhead ribs arching over the Hangar Bay (side ribs terminate cleanly under top rib)
    for z_rib in [15.0, 21.5, 28.0, 34.5, 40.6] {
        op.push_box(
            [-11.2, 2.35, z_rib],
            [1.4, 12.5, 1.44],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
        op.push_box(
            [11.2, 2.35, z_rib],
            [1.4, 12.5, 1.44],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
        op.push_box(
            [0.0, 9.3, z_rib],
            [23.8, 1.4, 1.5],
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
    // Floor slab split around the 3-Deck Elevator Shaft cutout (`X:-2.6..2.6, Z:-8.6..-3.4`)
    op.push_box(
        [0.0, 11.4, 3.8],
        [22.0, 1.2, 14.4],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, 11.4, -9.8],
        [22.0, 1.2, 2.4],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [-6.8, 11.4, -6.0],
        [8.4, 1.2, 5.2],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [6.8, 11.4, -6.0],
        [8.4, 1.2, 5.2],
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
    // Side sills/headers trimmed cleanly between front (+10.1) and rear (-10.2) to prevent corner Z-fighting
    op.push_box(
        [0.0, 12.9, 10.6],
        [22.2, 1.8, 1.0],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [0.0, 20.0, 10.6],
        [22.2, 1.5, 1.0],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );
    op.push_box(
        [-10.6, 12.9, -0.05],
        [1.0, 1.8, 20.1],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [-10.6, 20.0, -0.05],
        [1.0, 1.5, 20.1],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );
    op.push_box(
        [10.6, 12.9, -0.05],
        [1.0, 1.8, 20.1],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [10.6, 20.0, -0.05],
        [1.0, 1.5, 20.1],
        Tile::DarkSteelRib,
        4.0,
        rib,
        0.0,
    );

    // Chunky vertical window mullions seated cleanly between lower sill (Y=13.8) and upper header (Y=19.25)
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
            [mx, 16.525, mz],
            [1.24, 5.45, 1.24],
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
    // Lower deck ceiling split around the 3-Deck Elevator Shaft cutout (`X:-2.6..2.6, Z:-8.6..-3.4`)
    op.push_box(
        [0.0, -4.9, 3.3],
        [38.0, 1.2, 13.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, -4.9, -9.3],
        [38.0, 1.2, 1.4],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [-10.8, -4.9, -6.0],
        [16.4, 1.2, 5.2],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [10.8, -4.9, -6.0],
        [16.4, 1.2, 5.2],
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
            [36.2, 1.8, 0.9],
            Tile::CorridorWall,
            4.0,
            corr,
            0.0,
        );
        op.push_box(
            [0.0, -6.2, z_side],
            [36.2, 1.6, 0.9],
            Tile::DarkSteelRib,
            4.0,
            rib,
            0.0,
        );
        // Vertical mullions seated cleanly between sill (Y=-13.2) and header (Y=-7.0)
        for x_col in [-17.5, -12.0, -6.0, 0.0, 6.0, 12.0, 17.5] {
            op.push_box(
                [x_col, -10.1, z_side],
                [1.1, 6.2, 1.14],
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

        // Twin sodium floodlight pylons at outer pad corners (poles terminate cleanly under lamp heads)
        for z_off in [-8.2f32, 8.2f32] {
            let tx = px + side * 8.2;
            op.push_box(
                [tx, py + 4.2, pz + z_off],
                [1.1, 7.7, 1.1],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
            op.push_box(
                [tx - side * 0.45, py + 8.6, pz + z_off],
                [1.5, 1.1, 1.8],
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
    // 5. HOLLOWED CENTRAL CORE ATRIUM, 3-DECK ELEVATOR SHAFT, AXIAL PIPE TUNNEL & SOLAR BAY
    // =========================================================================
    // 5A. Central Core Atrium (2-story interior `X:-11.4..11.4, Y:-3.9..10.8, Z:-14.2..10.9`)
    // Atrium Floor (`Y = -4.25..-3.85`) split cleanly around the Elevator Shaft (`X:-2.6..2.6, Z:-8.6..-3.4`)
    op.push_box(
        [0.0, -4.10, 3.7],
        [23.2, 0.45, 14.2],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, -4.10, -11.6],
        [23.2, 0.45, 6.0],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [-7.1, -4.10, -6.0],
        [9.0, 0.45, 5.2],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [7.1, -4.10, -6.0],
        [9.0, 0.45, 5.2],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    // Atrium Port & Starboard windowed walls (`X = ±11.7`) looking out toward Pads B & C!
    for x_side in [-11.7f32, 11.7f32] {
        op.push_box(
            [x_side, -1.7, -1.8],
            [1.0, 4.4, 25.4],
            Tile::CorridorWall,
            4.5,
            corr,
            0.0,
        );
        op.push_box(
            [x_side, 8.6, -1.8],
            [1.0, 4.4, 25.4],
            Tile::HullPanel,
            5.0,
            hull,
            0.0,
        );
        // Vertical armored window mullions between sill (Y=0.5) and upper wall (Y=6.4)
        for z_m in [-13.8, -7.5, -1.5, 4.5, 10.3] {
            op.push_box(
                [x_side, 3.45, z_m],
                [1.15, 5.9, 1.3],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
        }
        // Mid-level industrial mezzanine catwalk inside the 2-story Atrium (`Y = 3.0`)
        let cat_x = if x_side < 0.0 { -8.8 } else { 8.8 };
        op.push_box(
            [cat_x, 3.0, -1.5],
            [4.2, 0.45, 22.0],
            Tile::FloorGrate,
            2.8,
            hull,
            0.0,
        );
        let rail_x = if x_side < 0.0 { -6.8 } else { 6.8 };
        op.push_box(
            [rail_x, 3.65, -1.5],
            [0.24, 0.85, 22.0],
            Tile::HazardStripe,
            2.2,
            bright_white,
            0.0,
        );
    }
    // Atrium Rear Bulkhead (`Z = -14.6`) split around the Axial Pipe Tunnel doorway (`X:-2.6..2.6, Y:-3.9..1.8`)
    op.push_box(
        [-7.3, 3.4, -14.6],
        [9.4, 14.8, 0.9],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [7.3, 3.4, -14.6],
        [9.4, 14.8, 0.9],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 6.3, -14.6],
        [5.2, 9.0, 0.9],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 2.0, -14.1],
        [5.8, 0.45, 0.35],
        Tile::HazardStripe,
        2.0,
        bright_white,
        0.0,
    );
    // Atrium Server & CRT Status Pillars + Ceiling Fluorescent Strips
    for (px, pz) in [(-6.8, 5.2), (6.8, 5.2), (-7.2, -10.2), (7.2, -10.2)] {
        op.push_box(
            [px, -0.5, pz],
            [2.0, 6.6, 2.0],
            Tile::ServerRack,
            2.2,
            bright_white,
            0.72,
        );
    }
    for z_l in [-10.0, -1.0, 7.0] {
        op.push_box(
            [-4.8, 10.65, z_l],
            [3.2, 0.2, 1.1],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.92,
        );
        op.push_box(
            [4.8, 10.65, z_l],
            [3.2, 0.2, 1.1],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.92,
        );
    }
    // Watertight Atrium Roof / Inter-Deck Ceiling Collar sealing the perimeter between the
    // wider Central Core Atrium (X:-12.2..12.2, Z:-14.6..11.6) and the Ops Command Spire floor
    // slab (X:-11.0..11.0, Z:-11.0..11.0, Y:10.8..12.0), keeping the Elevator Shaft open.
    op.push_box(
        [0.0, 11.3, -12.8],
        [24.4, 1.0, 3.6],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 11.1, 11.3],
        [24.4, 0.6, 0.6],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    for x_eave in [-11.6f32, 11.6f32] {
        op.push_box(
            [x_eave, 11.2, 0.0],
            [1.2, 0.8, 22.0],
            Tile::HullPanel,
            5.0,
            dark_hull,
            0.0,
        );
    }

    // 5B. Continuous 3-Deck Glass Elevator Shaft Frame (`X = 0, Z = -6.0`, spanning `Y = -15.0..+20.6`)
    for sx in [-2.65f32, 2.65f32] {
        for sz in [-8.65f32, -3.35f32] {
            op.push_box(
                [sx, 2.8, sz],
                [0.45, 35.6, 0.45],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
        }
    }
    // Glowing cyan deck-level guide bands on the elevator shaft rear rail at Deck 1, Deck 2, and Deck 3
    for deck_y in [-14.2f32, -3.1f32, 12.8f32] {
        op.push_box(
            [0.0, deck_y, -8.55],
            [4.8, 0.35, 0.22],
            Tile::CeilingLight,
            2.4,
            accent_em,
            1.15,
        );
    }

    // 5C. Hollowed-Out Axial Pipe Connector Tunnel (`Z = -15.0..-25.0`, inside the rotating ring core!)
    // Floor walkway & overhead glowing fluorescent conduit ceiling
    op.push_box(
        [0.0, -4.25, -20.0],
        [5.4, 0.75, 10.2],
        Tile::FloorGrate,
        2.5,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, 2.25, -20.0],
        [5.4, 0.8, 10.2],
        Tile::DarkSteelRib,
        3.0,
        rib,
        0.0,
    );
    op.push_box(
        [0.0, 1.78, -20.0],
        [1.1, 0.16, 8.6],
        Tile::CeilingLight,
        2.5,
        bright_white,
        0.95,
    );
    // Tunnel side walls with armored side portholes at Z = -20.0 so the Walker Bot sees rotating ring spokes sweep past!
    for x_side in [-2.85f32, 2.85f32] {
        // Fore & aft solid tunnel wall segments
        op.push_box(
            [x_side, -1.0, -16.3],
            [0.5, 5.7, 2.6],
            Tile::CorridorWall,
            3.0,
            corr,
            0.0,
        );
        op.push_box(
            [x_side, -1.0, -23.7],
            [0.5, 5.7, 2.6],
            Tile::CorridorWall,
            3.0,
            corr,
            0.0,
        );
        // Porthole lower sill & upper header at Z = -20.0 (leaving open window `Y = -2.2..+0.8, Z = -22.4..-17.6`)
        op.push_box(
            [x_side, -3.05, -20.0],
            [0.5, 1.7, 4.8],
            Tile::CorridorWall,
            3.0,
            corr,
            0.0,
        );
        op.push_box(
            [x_side, 1.35, -20.0],
            [0.5, 1.1, 4.8],
            Tile::DarkSteelRib,
            3.0,
            rib,
            0.0,
        );
    }
    // Outer hollow annular hazard collar flanges (`R = 7.4`, leaving the inner tunnel completely open!)
    for z_col in [-16.4f32, -23.6f32] {
        for i in 0..12 {
            let a = (i as f32 / 12.0) * TAU;
            let rad = [a.cos(), a.sin(), 0.0];
            let tan = [-a.sin(), a.cos(), 0.0];
            let fwd = [0.0, 0.0, 1.0];
            op.push_oriented_box(
                [a.cos() * 7.2, a.sin() * 7.2, z_col],
                rad,
                tan,
                fwd,
                [0.0, 0.0, 0.0],
                [1.8, 3.8, 1.4],
                Tile::HazardStripe,
                bright_white,
                0.0,
            );
        }
    }

    // 5D. Hollowed-Out Aft Solar & Power Engineering Bay (`X:-6.6..6.6, Y:-3.9..6.4, Z:-42.6..-25.0`)
    op.push_box(
        [0.0, -4.3, -33.8],
        [14.0, 0.85, 17.8],
        Tile::FloorGrate,
        3.0,
        hull,
        0.0,
    );
    op.push_box(
        [0.0, 6.7, -33.8],
        [14.0, 0.85, 17.8],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    for z_l in [-29.0, -34.5, -39.5] {
        op.push_box(
            [0.0, 6.18, z_l],
            [4.2, 0.2, 1.1],
            Tile::CeilingLight,
            3.0,
            bright_white,
            0.95,
        );
    }
    // Front doorway wall (`Z = -25.1`) split around Pipe Tunnel entrance (`X:-2.6..2.6, Y:-3.9..1.8`)
    op.push_box(
        [-4.8, 1.2, -25.1],
        [4.4, 10.2, 0.8],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [4.8, 1.2, -25.1],
        [4.4, 10.2, 0.8],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    op.push_box(
        [0.0, 4.1, -25.1],
        [5.2, 4.4, 0.8],
        Tile::CorridorWall,
        4.0,
        corr,
        0.0,
    );
    // Rear solid bulkhead (`Z = -42.5`) with glowing Solar Grid CRT Bank
    op.push_box(
        [0.0, 1.2, -42.5],
        [14.0, 10.2, 0.9],
        Tile::HullPanel,
        5.0,
        dark_hull,
        0.0,
    );
    op.push_box(
        [0.0, 0.6, -41.8],
        [7.4, 3.8, 0.5],
        Tile::CrtConsole,
        2.4,
        accent_em,
        0.95,
    );
    // Port & Starboard huge armored viewports (`X = ±6.7`) looking straight out at the Solar Collector Wings!
    for x_side in [-6.7f32, 6.7f32] {
        op.push_box(
            [x_side, -2.75, -33.8],
            [0.8, 2.3, 17.2],
            Tile::CorridorWall,
            3.5,
            corr,
            0.0,
        );
        op.push_box(
            [x_side, 5.35, -33.8],
            [0.8, 1.9, 17.2],
            Tile::DarkSteelRib,
            3.5,
            rib,
            0.0,
        );
        for z_m in [-25.6, -31.2, -36.8, -42.0] {
            op.push_box(
                [x_side, 1.4, z_m],
                [0.95, 6.0, 0.95],
                Tile::DarkSteelRib,
                3.0,
                rib,
                0.0,
            );
        }
    }
    // High-voltage solar power bus capacitor towers flanking the Solar Bay walkway
    for (tx, tz) in [(-4.4, -29.8), (4.4, -29.8), (-4.4, -37.2), (4.4, -37.2)] {
        op.push_box(
            [tx, -1.6, tz],
            [1.9, 4.4, 2.2],
            Tile::ServerRack,
            2.2,
            bright_white,
            0.75,
        );
        op.push_box(
            [tx, 1.0, tz],
            [1.3, 0.8, 1.4],
            Tile::ReactorCoil,
            1.4,
            sec_em,
            1.1,
        );
    }

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

    // Exterior Hangar Mouth armored collar (side pillars seated cleanly between bottom sill Y=-3.6 and top lintel Y=+9.9)
    op.push_box(
        [-13.2, 3.15, 42.2],
        [2.2, 13.5, 2.2],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );
    op.push_box(
        [13.2, 3.15, 42.2],
        [2.2, 13.5, 2.2],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );
    op.push_box(
        [0.0, 11.0, 42.2],
        [28.8, 2.2, 2.5],
        Tile::HazardStripe,
        3.0,
        bright_white,
        0.0,
    );
    op.push_box(
        [0.0, -4.6, 42.2],
        [28.8, 2.0, 2.5],
        Tile::DarkSteelRib,
        3.5,
        rib,
        0.0,
    );

    // =========================================================================
    // 6. DISTANT NATURAL PLANET, ATMOSPHERIC LIMB, ICE/DUST RING & DEEP STARFIELD
    // =========================================================================
    // Positioned ~1,550 units away with a monumental 620-unit radius so it feels genuinely
    // far below in low orbit with subtle, majestic parallax.
    let planet_center = [-480.0, -920.0, -1150.0];
    let planet_radius = 620.0;
    op.push_sphere(
        planet_center,
        planet_radius,
        18,
        32,
        Tile::PlanetSurface,
        theme.planet_north,
        theme.planet_equator,
        0.0, // Real directional sun & day/night terminator lighting!
    );

    // Subtle translucent atmospheric limb haze shell enveloping the planet
    tr.push_sphere(
        planet_center,
        planet_radius + 16.0,
        14,
        24,
        Tile::PlanetSurface,
        theme.planet_atmo,
        theme.planet_atmo,
        1.6,
    );

    // Natural concentric ice-and-dust planetary ring (with Cassini-style gap, zero hex grid)
    let ring_segs = 44;
    let r_in = 760.0;
    let r_out = 1090.0;
    for i in 0..ring_segs {
        let a0 = (i as f32 / ring_segs as f32) * TAU;
        let a1 = ((i + 1) as f32 / ring_segs as f32) * TAU;
        let pt = |r: f32, a: f32| -> [f32; 3] {
            let x = a.cos() * r;
            let z = a.sin() * r;
            let y = x * 0.30 + z * 0.18;
            [
                planet_center[0] + x,
                planet_center[1] + y,
                planet_center[2] + z,
            ]
        };
        // Note: `u` runs radially from inner edge (0.0) to outer edge (1.0) to sample Tile::PlanetRing bands
        tr.push_quad(
            pt(r_in, a0),
            pt(r_out, a0),
            pt(r_out, a1),
            pt(r_in, a1),
            [0.0, 1.0, 0.0],
            [0.98, 1.0],
            Tile::PlanetRing,
            theme.planet_ring,
            0.0,
        );
    }

    // Deterministic 3D deep-space starfield at radius ~1650..2100
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next_f = || -> f32 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s & 0xFFFF) as f32 / 65535.0
    };
    for _ in 0..220 {
        let th = next_f() * TAU;
        let z_u = next_f() * 2.0 - 1.0;
        let r_xy = (1.0 - z_u * z_u).max(0.0).sqrt();
        let dist = 1650.0 + next_f() * 450.0;
        let pos = [r_xy * th.cos() * dist, z_u * dist, r_xy * th.sin() * dist];
        let sz = 3.2 + next_f() * 4.5;
        let tint = if next_f() > 0.75 {
            [0.78, 0.90, 1.0, 1.0]
        } else if next_f() > 0.8 {
            [1.0, 0.88, 0.72, 1.0]
        } else {
            [0.90, 0.93, 0.98, 1.0]
        };
        op.push_box(pos, [sz, sz, sz], Tile::CeilingLight, sz, tint, 1.15);
    }

    StaticStationMeshes {
        opaque_verts: op.verts,
        opaque_indices: op.indices,
        trans_verts: tr.verts,
        trans_indices: tr.indices,
    }
}
