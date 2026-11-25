//! Dynamic 3D machinery, visual progress props, and cycle-reset finales for all five
//! main interior rooms of Station HELIOS-09.

use std::f32::consts::TAU;

use rookery_core::orbital::simulation::REACTOR_CORE_Z;
use rookery_core::orbital::{
    ATRIUM_PILLARS, ATRIUM_SECTOR_COUNT, ATRIUM_SECTORS, HANGAR_CRATES_PER_CYCLE, OPS_HOLO_LAYERS,
    OrbitalHavenState,
};

use super::mesh_builder::MeshBuilder;
use super::textures::Tile;
use super::theme::OrbitalTheme;

#[inline]
fn lerp_col(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let u = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        a[3] + (b[3] - a[3]) * u,
    ]
}

pub fn build_room_activity_meshes(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    build_hangar_compactor(state, theme, time, op, tr);
    build_atrium_maintenance(state, theme, time, op, tr);
    build_ops_holo_diorama(state, theme, time, op, tr);
    build_reactor_cryo_cycle(state, theme, time, op, tr);
    build_solar_grid_charge(state, theme, time, op, tr);
}

// =============================================================================
// 1. MAIN HANGAR BAY — Hydraulic Compactor Press, Bale Pallet & Gantry Crane
// =============================================================================

fn build_hangar_compactor(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    let h = &state.room_activities.hangar;
    let floor_y = -3.88f32;
    let accent = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let sec = [
        theme.accent_secondary[0],
        theme.accent_secondary[1],
        theme.accent_secondary[2],
        1.0,
    ];
    let green = [0.22, 1.0, 0.45, 1.0];
    let white = [0.95, 0.97, 1.0, 1.0];
    let dark_steel = [0.42, 0.47, 0.54, 1.0];
    let alloy_bale = [0.68, 0.76, 0.84, 1.0];

    // Twin Compactor Presses, Bale Pallets & Staging Racks on Starboard (+9.5) and Port (-9.5)
    for side in [-1.0f32, 1.0f32] {
        let wall_x = side * 9.5;
        let inward = -side;

        // A. Hydraulic Cargo Compactor Press at Z = 22.0
        let cz = 22.0f32;
        // Base anvil & hazard frame
        op.push_box(
            [wall_x, floor_y + 0.35, cz],
            [2.4, 0.70, 2.6],
            Tile::HazardStripe,
            2.0,
            white,
            0.0,
        );
        // Rear & side press uprights
        op.push_box(
            [wall_x + side * 0.95, floor_y + 2.35, cz],
            [0.50, 3.3, 2.6],
            Tile::DarkSteelRib,
            2.5,
            theme.rib_tint,
            0.0,
        );
        for dz in [-1.15f32, 1.15f32] {
            op.push_box(
                [wall_x, floor_y + 2.35, cz + dz],
                [2.2, 3.3, 0.32],
                Tile::DarkSteelRib,
                2.2,
                dark_steel,
                0.0,
            );
        }
        // Heavy top hydraulic head
        op.push_box(
            [wall_x, floor_y + 4.15, cz],
            [2.6, 0.65, 2.8],
            Tile::HullPanel,
            2.5,
            theme.hull_tint,
            0.0,
        );
        // 6-LED cycle progress strip facing inward toward Pad A & CAM-04
        for i in 0..HANGAR_CRATES_PER_CYCLE {
            let lz = cz - 0.95 + i as f32 * 0.38;
            let lit = i < h.compacted_bales;
            let col = if lit { green } else { [0.22, 0.26, 0.30, 1.0] };
            let em = if lit { 1.35 } else { 0.1 };
            op.push_box(
                [wall_x + inward * 1.32, floor_y + 4.15, lz],
                [0.10, 0.24, 0.26],
                Tile::CeilingLight,
                1.0,
                col,
                em,
            );
        }
        // Flashing caution strobe when ram is actively crushing
        let crushing = h.ram_depth > 0.05;
        if crushing && (time * 12.0).sin() > 0.0 {
            op.push_box(
                [wall_x + inward * 0.6, floor_y + 4.65, cz],
                [0.42, 0.32, 0.42],
                Tile::CeilingLight,
                1.0,
                sec,
                1.5,
            );
        }
        // Animated hydraulic piston & descending ram plate
        let ram_drop = h.ram_depth * 1.55;
        let ram_y = floor_y + 3.25 - ram_drop;
        op.push_box(
            [wall_x, (ram_y + floor_y + 3.85) * 0.5, cz],
            [0.55, (floor_y + 3.85 - ram_y).max(0.2), 0.55],
            Tile::DarkSteelRib,
            2.0,
            dark_steel,
            0.0,
        );
        op.push_box(
            [wall_x, ram_y, cz],
            [1.95, 0.34, 1.95],
            Tile::HazardStripe,
            1.8,
            white,
            0.0,
        );
        // Crate inside hopper visibly squashing from height 1.45 -> 0.42
        if h.crate_in_hopper {
            let sq = h.hopper_squash.clamp(0.0, 1.0);
            let ch = 1.45 - sq * 1.03;
            let cw = 1.35 + sq * 0.18;
            let col = lerp_col(white, alloy_bale, sq);
            let tile = if sq > 0.7 {
                Tile::ShipArmor
            } else {
                Tile::CargoCrate
            };
            op.push_box(
                [wall_x, floor_y + 0.72 + ch * 0.5, cz],
                [cw, ch, cw],
                tile,
                2.0,
                col,
                0.0,
            );
            // Glowing plasma compression seam while ram presses down
            if h.ram_depth > 0.25 {
                tr.push_box(
                    [wall_x, floor_y + 0.72 + ch, cz],
                    [cw + 0.25, 0.18, cw + 0.25],
                    Tile::ForcefieldHex,
                    1.5,
                    [sec[0], sec[1], sec[2], 0.58 * h.ram_depth],
                    1.8,
                );
            }
        }

        // B. Output Alloy Bale Pallet at Z = 25.0 (stacks 0..=6 gleaming compacted cubes)
        let pz = 25.0f32;
        op.push_box(
            [wall_x, floor_y + 0.14, pz],
            [2.3, 0.28, 2.4],
            Tile::FloorGrate,
            2.0,
            theme.hull_tint,
            0.0,
        );
        for b in 0..h.compacted_bales {
            let layer = (b / 2) as f32;
            let slot = (b % 2) as f32;
            let bz = pz - 0.56 + slot * 1.12;
            let by = floor_y + 0.52 + layer * 0.48;
            op.push_box(
                [wall_x, by, bz],
                [1.45, 0.42, 0.96],
                Tile::ShipArmor,
                1.8,
                alloy_bale,
                0.0,
            );
            // Glowing recycled-alloy seal band
            op.push_box(
                [wall_x + inward * 0.74, by, bz],
                [0.06, 0.14, 0.64],
                Tile::CeilingLight,
                1.0,
                accent,
                0.95,
            );
        }

        // C. Input Raw Crate Staging Rack at Z = 31.4 (depletes 6..=0 over the 5-minute cycle)
        let rz = 31.4f32;
        op.push_box(
            [wall_x, floor_y + 0.14, rz],
            [2.4, 0.28, 3.0],
            Tile::HazardStripe,
            2.0,
            white,
            0.0,
        );
        for c in 0..h.raw_crates_remaining {
            let layer = (c / 2) as f32;
            let slot = (c % 2) as f32;
            let cz_slot = rz - 0.72 + slot * 1.44;
            let cy = floor_y + 0.95 + layer * 1.34;
            op.push_box(
                [wall_x, cy, cz_slot],
                [1.42, 1.28, 1.28],
                Tile::CargoCrate,
                2.0,
                white,
                0.0,
            );
        }
    }

    // D. Animated Overhead Gantry Crane (`Y = 8.2`, traveling along Z = 18..34)
    let gz = h.gantry_z;
    op.push_box(
        [0.0, 8.2, gz],
        [21.4, 0.9, 1.25],
        Tile::HazardStripe,
        2.4,
        white,
        0.0,
    );
    // Sliding magnetic trolley carriage
    let tx = h.gantry_trolley_x;
    op.push_box(
        [tx, 7.55, gz],
        [2.2, 0.65, 1.8],
        Tile::DarkSteelRib,
        2.0,
        dark_steel,
        0.0,
    );
    op.push_box(
        [tx, 7.18, gz],
        [0.6, 0.18, 0.6],
        Tile::CeilingLight,
        1.0,
        if h.in_finale { accent } else { sec },
        1.25,
    );

    // Telescoping hoist cable, grapple head & carried payload during the 45s finale
    if h.gantry_hoist > 0.02 || h.gantry_payload > 0 {
        let drop_len = 0.8 + h.gantry_hoist * 7.6;
        let hook_y = 7.1 - drop_len;
        op.push_box(
            [tx, 7.1 - drop_len * 0.5, gz],
            [0.18, drop_len, 0.18],
            Tile::DarkSteelRib,
            2.0,
            dark_steel,
            0.0,
        );
        // Magnetic grapple crossbar
        op.push_box(
            [tx, hook_y, gz],
            [2.0, 0.32, 2.2],
            Tile::HazardStripe,
            1.8,
            white,
            0.0,
        );
        // Translucent cyan tractor-lock field under grapple
        tr.push_box(
            [tx, hook_y - 0.55, gz],
            [1.8, 0.95, 2.0],
            Tile::ForcefieldHex,
            1.6,
            [accent[0], accent[1], accent[2], 0.34],
            1.7,
        );
        if h.gantry_payload == 1 {
            // Hoisting completed 6-bale alloy pallet up to ceiling cargo bay!
            op.push_box(
                [tx, hook_y - 0.92, gz],
                [1.8, 1.35, 2.0],
                Tile::ShipArmor,
                2.0,
                alloy_bale,
                0.0,
            );
        } else if h.gantry_payload == 2 {
            // Lowering fresh 6-crate supply pod onto the staging rack!
            op.push_box(
                [tx, hook_y - 1.35, gz],
                [1.8, 2.2, 2.4],
                Tile::CargoCrate,
                2.0,
                white,
                0.0,
            );
        }
    }
}

// =============================================================================
// 2. CENTRAL CORE ATRIUM — 12-Sector Floor Scrub, Wash Dock & 4 Pillar Beacons
// =============================================================================

fn build_atrium_maintenance(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    let at = &state.room_activities.atrium;
    let floor_y = -3.84f32;
    let accent = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let sec = [
        theme.accent_secondary[0],
        theme.accent_secondary[1],
        theme.accent_secondary[2],
        1.0,
    ];
    let green = [0.22, 1.0, 0.45, 1.0];
    let white = [0.95, 0.97, 1.0, 1.0];
    let dark_steel = [0.42, 0.47, 0.54, 1.0];

    // A. 12 Dynamic Deck Maintenance Sectors (dirty amber oil-film vs gleaming sanitized cyan)
    for (idx, &([sx, sz], _)) in ATRIUM_SECTORS.iter().enumerate() {
        let cleaned = (idx as u8) < at.cleaned_sectors;
        if !cleaned && at.grime_fade_in > 0.04 {
            let alpha = 0.34 * at.grime_fade_in;
            tr.push_box(
                [sx, floor_y + 0.04, sz],
                [2.4, 0.05, 2.4],
                Tile::HazardStripe,
                2.2,
                [0.62, 0.42, 0.16, alpha],
                0.25,
            );
        } else if cleaned && (idx as u8) + 2 >= at.cleaned_sectors {
            // Recently scrubbed sector shimmers with a clean cyan sanitized sheen
            let pulse = 0.16 + 0.10 * (time * 3.2 + idx as f32).sin().abs();
            tr.push_box(
                [sx, floor_y + 0.04, sz],
                [2.2, 0.04, 2.2],
                Tile::ForcefieldHex,
                2.0,
                [accent[0], accent[1], accent[2], pulse],
                1.35,
            );
        }
    }

    // B. 4 Server Pillar Status Beacons (`±6.8, 5.2` & `±7.2, -10.2`, Y = 3.05)
    for (i, &([px, pz], _, _)) in ATRIUM_PILLARS.iter().enumerate() {
        let verified = (i as u8) < at.verified_pillars;
        let col = if verified { green } else { sec };
        let em = if verified {
            1.35
        } else {
            0.75 + 0.35 * (time * 4.0 + i as f32).sin().abs()
        };
        op.push_box(
            [px, 3.08, pz],
            [1.65, 0.34, 1.65],
            Tile::CeilingLight,
            1.5,
            col,
            em,
        );
    }

    // C. 12-Segment Cycle Progress Strip on the +Z front fascia of the Elevator Shaft (`Z = -3.28`)
    op.push_box(
        [0.0, 1.72, -3.28],
        [4.8, 0.46, 0.18],
        Tile::DarkSteelRib,
        2.0,
        dark_steel,
        0.0,
    );
    for i in 0..ATRIUM_SECTOR_COUNT {
        let bx = -2.05 + i as f32 * 0.372;
        let lit = (i as u8) < at.cleaned_sectors;
        let col = if lit { accent } else { [0.18, 0.22, 0.28, 1.0] };
        op.push_box(
            [bx, 1.72, -3.17],
            [0.26, 0.24, 0.08],
            Tile::CeilingLight,
            1.0,
            col,
            if lit { 1.3 } else { 0.1 },
        );
    }

    // D. Atrium Wash & Waste-Flush Dock (`X = 9.9, Z = 8.2`, clear of Pad C portal & server pillar)
    op.push_box(
        [9.9, floor_y + 1.35, 8.2],
        [1.5, 2.7, 2.8],
        Tile::DarkSteelRib,
        2.2,
        theme.rib_tint,
        0.0,
    );
    op.push_box(
        [9.05, floor_y + 0.12, 8.2],
        [1.4, 0.16, 2.4],
        Tile::HazardStripe,
        1.8,
        white,
        0.0,
    );
    let dock_col = if at.purging_waste { accent } else { green };
    op.push_box(
        [9.12, floor_y + 2.35, 8.2],
        [0.14, 0.42, 1.8],
        Tile::CeilingLight,
        1.2,
        dock_col,
        1.3,
    );
    if at.purging_waste {
        // Rising translucent cyan-white purge steam plume
        for k in 0..3 {
            let phase = (time * 1.4 + k as f32 * 0.33).fract();
            let sy = floor_y + 0.8 + phase * 3.4;
            let sz_s = 0.65 + phase * 0.9;
            tr.push_box(
                [9.1, sy, 8.2 + (time * 3.0 + k as f32).sin() * 0.25],
                [sz_s, sz_s * 0.7, sz_s],
                Tile::ForcefieldHex,
                1.5,
                [0.75, 0.96, 1.0, (1.0 - phase) * 0.36],
                1.4,
            );
        }
    }
}

// =============================================================================
// 3. OPS COMMAND SPIRE — 6-Layer 3D Holographic Orbital Star-Chart Diorama
// =============================================================================

fn build_ops_holo_diorama(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    let ops = &state.room_activities.ops;
    let cx = 0.0f32;
    let cz = 1.0f32;
    let base_y = 13.85f32;
    let accent = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let sec = [
        theme.accent_secondary[0],
        theme.accent_secondary[1],
        theme.accent_secondary[2],
        1.0,
    ];
    let green = [0.25, 1.0, 0.55, 1.0];

    // 6 Radial Emitter Nodes around the Central Holotable Pedestal lighting up per completed layer
    for i in 0..OPS_HOLO_LAYERS {
        let a = (i as f32 / OPS_HOLO_LAYERS as f32) * TAU;
        let ex = cx + a.cos() * 2.15;
        let ez = cz + a.sin() * 2.15;
        let lit = i < ops.completed_layers;
        let col = if ops.broadcasting {
            green
        } else if lit {
            accent
        } else {
            [0.20, 0.24, 0.30, 1.0]
        };
        op.push_box(
            [ex, base_y, ez],
            [0.28, 0.16, 0.28],
            Tile::CeilingLight,
            1.0,
            col,
            if lit { 1.4 } else { 0.15 },
        );
    }

    // Archive-reset flash pulse when compressing down into table
    if ops.archive_collapse > 0.05 {
        let flash_a = (1.0 - ops.archive_collapse) * 0.55;
        tr.push_cylinder(
            [cx, base_y + 0.35, cz],
            2.1,
            2.1,
            0.35,
            10,
            Tile::ForcefieldHex,
            [accent[0], accent[1], accent[2], flash_a],
            1.95,
        );
    }

    let spin = time * 0.55;
    let (ss, sc) = spin.sin_cos();
    let hr = [sc, 0.0, -ss];
    let hu = [0.0, 1.0, 0.0];
    let hf = [ss, 0.0, sc];

    let holo_cyan = [accent[0], accent[1], accent[2], 0.52];
    let holo_gold = [sec[0], sec[1], sec[2], 0.56];
    let holo_emerald = [0.25, 1.0, 0.58, 0.45];

    // Base faint wireframe calibration ring (always visible)
    tr.push_cylinder(
        [cx, base_y + 0.18, cz],
        1.95,
        1.95,
        0.08,
        12,
        Tile::ForcefieldHex,
        [accent[0], accent[1], accent[2], 0.28],
        1.6,
    );

    // Layer 0: Miniature Planetary Core & Tilted Orbital Ring
    let s0 = ops.layer_scale(0);
    if s0 > 0.04 {
        tr.push_sphere(
            [cx, base_y + 1.05, cz],
            0.55 * s0,
            8,
            12,
            Tile::ForcefieldHex,
            holo_cyan,
            holo_cyan,
            1.8,
        );
        tr.push_oriented_box(
            [cx, base_y + 1.05, cz],
            hr,
            hu,
            hf,
            [0.0, 0.0, 0.0],
            [2.4 * s0, 0.06, 2.4 * s0],
            Tile::ForcefieldHex,
            [accent[0], accent[1], accent[2], 0.26 * s0],
            1.7,
        );
    }

    // Layer 1: Mini Station HELIOS-09 Central Spine & Forward Hangar
    let s1 = ops.layer_scale(1);
    if s1 > 0.04 {
        let sy = base_y + 2.05;
        tr.push_oriented_box(
            [cx, sy, cz],
            hr,
            hu,
            hf,
            [0.0, 0.0, 0.0],
            [0.48 * s1, 0.48 * s1, 2.2 * s1],
            Tile::ForcefieldHex,
            holo_cyan,
            1.85,
        );
        tr.push_oriented_box(
            [cx, sy, cz],
            hr,
            hu,
            hf,
            [0.0, 0.42 * s1, 0.0],
            [0.52 * s1, 0.85 * s1, 0.52 * s1],
            Tile::ForcefieldHex,
            holo_cyan,
            1.85,
        );
    }

    // Layer 2: Spinning Miniature Habitation Counterweight Ring
    let s2 = ops.layer_scale(2);
    if s2 > 0.04 {
        let sy = base_y + 2.05;
        let ring_spin = time * 1.4;
        for i in 0..8 {
            let a = ring_spin + (i as f32 / 8.0) * TAU;
            let lx = a.cos() * 0.88 * s2;
            let ly = a.sin() * 0.88 * s2;
            tr.push_oriented_box(
                [cx, sy, cz],
                hr,
                hu,
                hf,
                [lx, ly, -0.45 * s2],
                [0.18 * s2, 0.36 * s2, 0.20 * s2],
                Tile::ForcefieldHex,
                holo_gold,
                1.9,
            );
        }
    }

    // Layer 3: Twin Aft Solar Wing Arrays & External Pad Markers
    let s3 = ops.layer_scale(3);
    if s3 > 0.04 {
        let sy = base_y + 2.05;
        for side in [-1.0f32, 1.0f32] {
            tr.push_oriented_box(
                [cx, sy, cz],
                hr,
                hu,
                hf,
                [side * 1.15 * s3, 0.08, -0.95 * s3],
                [1.25 * s3, 0.08, 0.52 * s3],
                Tile::ForcefieldHex,
                holo_cyan,
                1.85,
            );
            tr.push_oriented_box(
                [cx, sy, cz],
                hr,
                hu,
                hf,
                [side * 1.10 * s3, -0.10, 0.25 * s3],
                [0.48 * s3, 0.12, 0.48 * s3],
                Tile::ForcefieldHex,
                holo_gold,
                1.85,
            );
        }
    }

    // Layer 4: Live-Orbiting Miniature Spacecraft Blips
    let s4 = ops.layer_scale(4);
    if s4 > 0.04 {
        let sy = base_y + 2.15;
        for i in 0..3 {
            let a = time * (1.2 + i as f32 * 0.45) + i as f32 * 2.1;
            let rad = (1.45 + i as f32 * 0.24) * s4;
            let bx = cx + a.cos() * rad;
            let bz = cz + a.sin() * rad;
            let by = sy + (a * 2.0).sin() * 0.35 * s4;
            op.push_box(
                [bx, by, bz],
                [0.18 * s4, 0.12 * s4, 0.24 * s4],
                Tile::CeilingLight,
                1.0,
                sec,
                1.5,
            );
        }
    }

    // Layer 5: Tactical Sensor Dome & Rising Broadcast Telemetry Rings
    let s5 = ops.layer_scale(5);
    if s5 > 0.04 {
        let dome_col = if ops.broadcasting {
            holo_emerald
        } else {
            [accent[0], accent[1], accent[2], 0.28 * s5]
        };
        tr.push_cylinder(
            [cx, base_y + 2.0, cz],
            1.85 * s5,
            1.25 * s5,
            2.3 * s5,
            10,
            Tile::ForcefieldHex,
            dome_col,
            1.7,
        );
        if ops.broadcasting {
            for k in 0..2 {
                let p = (time * 0.65 + k as f32 * 0.5).fract();
                let ry = base_y + 0.5 + p * 3.8;
                let rr = 1.9 - p * 0.6;
                tr.push_cylinder(
                    [cx, ry, cz],
                    rr,
                    rr,
                    0.10,
                    12,
                    Tile::ForcefieldHex,
                    [0.35, 1.0, 0.65, (1.0 - p) * 0.45],
                    1.9,
                );
            }
        }
    }
}

// =============================================================================
// 4. CONCOURSE & REACTOR CORE — 4 Cryo-Rod Manifolds, Fresh Rack & Cooling Vat
// =============================================================================

fn build_reactor_cryo_cycle(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    let rc = &state.room_activities.reactor;
    let floor_y = -14.95f32;
    let accent = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let hot_orange = [1.0, 0.34, 0.08, 1.0];
    let white = [0.95, 0.97, 1.0, 1.0];
    let dark_steel = [0.42, 0.47, 0.54, 1.0];

    // A. 4 Vertical Cryo-Rod Manifolds mounted around the Reactor Collar (R = 3.25 around (0, REACTOR_CORE_Z))
    let angles = [
        std::f32::consts::FRAC_PI_4,
        3.0 * std::f32::consts::FRAC_PI_4,
        5.0 * std::f32::consts::FRAC_PI_4,
        7.0 * std::f32::consts::FRAC_PI_4,
    ];
    for (i, &ang) in angles.iter().enumerate() {
        let mx = ang.cos() * 3.25;
        let mz = REACTOR_CORE_Z + ang.sin() * 3.25;
        let heat = rc.rod_heat[i].clamp(0.0, 1.0);
        let rod_col = lerp_col(accent, hot_orange, heat);

        // Lower & upper steel manifold sockets
        op.push_box(
            [mx, floor_y + 0.65, mz],
            [0.72, 1.3, 0.72],
            Tile::DarkSteelRib,
            1.8,
            dark_steel,
            0.0,
        );
        op.push_box(
            [mx, floor_y + 8.4, mz],
            [0.72, 1.0, 0.72],
            Tile::DarkSteelRib,
            1.8,
            dark_steel,
            0.0,
        );
        // Glowing vertical Cryo/Fuel Rod shifting from cool cyan -> warm amber -> hot orange
        op.push_box(
            [mx, floor_y + 4.55, mz],
            [0.42, 6.4, 0.42],
            Tile::ReactorCoil,
            1.6,
            rod_col,
            1.15 + heat * 0.4,
        );
    }

    // B. Starboard Fresh Cryo-Cell Rack (`X = 16.0, Z = 0.0`)
    let rx = 16.0f32;
    op.push_box(
        [rx, floor_y + 0.35, 0.0],
        [2.2, 0.70, 5.2],
        Tile::HazardStripe,
        2.0,
        white,
        0.0,
    );
    op.push_box(
        [rx + 0.85, floor_y + 2.4, 0.0],
        [0.45, 4.1, 5.2],
        Tile::DarkSteelRib,
        2.2,
        theme.rib_tint,
        0.0,
    );
    op.push_box(
        [rx, floor_y + 4.55, 0.0],
        [2.2, 0.42, 5.4],
        Tile::HullPanel,
        2.4,
        theme.hull_tint,
        0.0,
    );
    for slot in 0..rc.fresh_rack_count {
        let sz = -1.8 + slot as f32 * 1.2;
        op.push_box(
            [rx - 0.15, floor_y + 2.1, sz],
            [0.52, 2.7, 0.52],
            Tile::ReactorCoil,
            1.5,
            accent,
            1.35,
        );
    }

    // C. Port Spent-Rod Cooling Vat (`X = -16.0, Z = 0.0`)
    let vx = -16.0f32;
    op.push_box(
        [vx, floor_y + 0.85, 0.0],
        [2.4, 1.7, 5.4],
        Tile::DarkSteelRib,
        2.4,
        theme.rib_tint,
        0.0,
    );
    op.push_box(
        [vx, floor_y + 1.78, 0.0],
        [2.5, 0.22, 5.5],
        Tile::HazardStripe,
        2.0,
        white,
        0.0,
    );
    let quench_col = lerp_col(hot_orange, accent, rc.vat_quench_t);
    for slot in 0..rc.vat_rod_count {
        let sz = -1.8 + slot as f32 * 1.2;
        op.push_box(
            [vx + 0.15, floor_y + 2.65, sz],
            [0.52, 2.1, 0.52],
            Tile::ReactorCoil,
            1.5,
            quench_col,
            1.35,
        );
    }
    // Animated translucent cyan Cryo-Quench Shroud during finale (`255..300s`)
    if rc.vat_quench_t > 0.04 {
        let h_shroud = 3.2 * rc.vat_quench_t;
        tr.push_box(
            [vx, floor_y + 1.9 + h_shroud * 0.5, 0.0],
            [2.6, h_shroud, 5.6],
            Tile::ForcefieldHex,
            2.0,
            [accent[0], accent[1], accent[2], 0.42 * rc.vat_quench_t],
            1.8,
        );
    }

    // D. Overhead Pneumatic Cryo-Cell Return Conduit (`Y = -6.6, Z = 0.0, X = -16..+16`)
    op.push_box(
        [0.0, -6.45, 0.0],
        [32.0, 0.42, 0.52],
        Tile::DarkSteelRib,
        3.0,
        dark_steel,
        0.0,
    );
    if rc.quenching && rc.pneumatic_pulse_t > 0.01 {
        let px = -15.5 + rc.pneumatic_pulse_t * 31.0;
        op.push_box(
            [px, -6.45, 0.0],
            [1.6, 0.62, 0.72],
            Tile::CeilingLight,
            1.2,
            accent,
            1.6,
        );
        // Pulse glow halo
        let glow = 0.4 + 0.2 * (time * 12.0).sin().abs();
        tr.push_box(
            [px, -6.45, 0.0],
            [2.6, 1.1, 1.1],
            Tile::ForcefieldHex,
            1.5,
            [accent[0], accent[1], accent[2], glow],
            1.85,
        );
    }
}

// =============================================================================
// 5. AFT SOLAR ENGINEERING BAY — 5-Tier Capacitor Charge Bars & Grid Discharge
// =============================================================================

fn build_solar_grid_charge(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    let sol = &state.room_activities.solar;
    let accent = [
        theme.accent_primary[0],
        theme.accent_primary[1],
        theme.accent_primary[2],
        1.0,
    ];
    let sec = [
        theme.accent_secondary[0],
        theme.accent_secondary[1],
        theme.accent_secondary[2],
        1.0,
    ];
    let dark_steel = [0.42, 0.47, 0.54, 1.0];

    let towers: [([f32; 2], f32); 4] = [
        ([-4.4, -29.8], 1.0),
        ([4.4, -29.8], -1.0),
        ([-4.4, -37.2], 1.0),
        ([4.4, -37.2], -1.0),
    ];

    // A. 5-Tier Vertical LED Charge Bars + Spinning Induction Rings on each of the 4 Towers
    for (i, &([tx, tz], inward)) in towers.iter().enumerate() {
        let chg = sol.tower_charge[i].clamp(0.0, 1.0);
        for tier in 0..5 {
            let threshold = (tier as f32 + 0.5) / 5.0;
            let lit = chg >= threshold;
            let ty = -3.15 + tier as f32 * 0.74;
            let col = if lit {
                lerp_col(sec, accent, tier as f32 / 4.0)
            } else {
                [0.16, 0.19, 0.24, 1.0]
            };
            op.push_box(
                [tx + inward * 1.02, ty, tz],
                [0.12, 0.48, 1.35],
                Tile::CeilingLight,
                1.2,
                col,
                if lit { 1.35 } else { 0.08 },
            );
        }
        // Spinning induction coil halo atop each tower (rotates faster as charge rises!)
        let ring_a = time * (0.8 + chg * 4.5) + i as f32 * 1.4;
        let (rs, rc) = ring_a.sin_cos();
        let rr = [rc, 0.0, -rs];
        let ru = [0.0, 1.0, 0.0];
        let rf = [rs, 0.0, rc];
        let coil_col = lerp_col(sec, accent, chg);
        op.push_oriented_box(
            [tx, 1.72, tz],
            rr,
            ru,
            rf,
            [0.0, 0.0, 0.0],
            [1.65, 0.24, 0.34],
            Tile::CeilingLight,
            coil_col,
            0.7 + chg * 0.9,
        );

        // Overhead vertical bus riser up to ceiling power-bus conduit (`Y = 4.6`)
        op.push_box(
            [tx, 3.25, tz],
            [0.28, 2.8, 0.28],
            Tile::DarkSteelRib,
            2.0,
            dark_steel,
            0.0,
        );

        // High-voltage discharge energy arc from tower top to overhead bus during finale!
        if sol.discharge_intensity > 0.05 {
            let flicker = 0.65 + 0.35 * (time * 24.0 + i as f32 * 2.3).sin().abs();
            let a = sol.discharge_intensity * flicker * 0.62;
            tr.push_box(
                [tx, 3.25, tz],
                [0.65, 3.0, 0.65],
                Tile::ForcefieldHex,
                1.5,
                [sec[0], sec[1], sec[2], a],
                1.95,
            );
        }
    }

    // B. Overhead Power-Bus Rails (`Y = 4.6`) linking Port/Stbd towers aft to Rear Accumulator (`Z = -41.4`)
    for side in [-1.0f32, 1.0f32] {
        op.push_box(
            [side * 4.4, 4.65, -35.6],
            [0.36, 0.36, 12.2],
            Tile::DarkSteelRib,
            2.5,
            dark_steel,
            0.0,
        );
        if sol.discharge_intensity > 0.05 {
            let a = sol.discharge_intensity * 0.55;
            tr.push_box(
                [side * 4.4, 4.65, -35.6],
                [0.68, 0.68, 12.2],
                Tile::ForcefieldHex,
                2.0,
                [accent[0], accent[1], accent[2], a],
                1.95,
            );
            // Outward golden-cyan beam pulsing through the side viewports toward the Solar Wings!
            let pulse = 0.5 + 0.5 * (time * 10.0 + side).sin().abs();
            tr.push_box(
                [side * 5.8, 1.72, -33.5],
                [2.4, 0.72, 1.4],
                Tile::ForcefieldHex,
                1.6,
                [
                    sec[0],
                    sec[1],
                    sec[2],
                    sol.discharge_intensity * pulse * 0.55,
                ],
                1.95,
            );
        }
    }

    // C. Rear Super-Capacitor Accumulator & 8-Segment Main Grid Meter (`Z = -41.3, Y = 3.3`)
    op.push_box(
        [0.0, 3.35, -41.45],
        [7.6, 0.95, 0.55],
        Tile::DarkSteelRib,
        2.4,
        theme.rib_tint,
        0.0,
    );
    for seg in 0..8 {
        let sx = -3.15 + seg as f32 * 0.90;
        let threshold = (seg as f32 + 0.5) / 8.0;
        let lit = sol.grid_charge >= threshold;
        let col = if sol.discharging {
            sec
        } else if lit {
            accent
        } else {
            [0.16, 0.20, 0.26, 1.0]
        };
        op.push_box(
            [sx, 3.35, -41.12],
            [0.66, 0.52, 0.14],
            Tile::CeilingLight,
            1.0,
            col,
            if lit || sol.discharging { 1.45 } else { 0.1 },
        );
    }
}
