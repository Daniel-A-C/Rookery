//! Per-frame dynamic 3D mesh generation for Station HELIOS-09:
//! - 4 low-poly spacecraft classes with landing gear, strobes & plasma thrusters
//! - 2 physical orbiting 3D camera drones
//! - Cargo loaders, interior security droids, ops crew & exterior welders
//! - Rotating habitation counterweight ring, spinning radar dishes, reactor plasma & runway chase lights

use std::f32::consts::TAU;

use rookery_core::orbital::{AgentKind, OrbitalHavenState, ShipClass, StationEventKind};

use super::cameras::basis_from_forward;
use super::mesh_builder::MeshBuilder;
use super::textures::Tile;
use super::theme::OrbitalTheme;

pub fn build_dynamic_entities(
    state: &OrbitalHavenState,
    theme: &OrbitalTheme,
    time: f32,
    op: &mut MeshBuilder,
    tr: &mut MeshBuilder,
) {
    op.clear();
    tr.clear();

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
    let red_strobe = [1.0, 0.18, 0.14, 1.0];
    let green_strobe = [0.22, 1.0, 0.45, 1.0];
    let white = [0.95, 0.97, 1.0, 1.0];
    let dark_steel = [0.45, 0.50, 0.56, 1.0];

    // =========================================================================
    // 1. ROTATING HABITATION COUNTERWEIGHT RING (seated in bearing collar at Z = -20)
    // =========================================================================
    let ring_segs = 12;
    let ring_r = 25.0;
    let ring_z = -20.0;
    // Rotating central bearing drum seated between the static fore/aft hazard collar flanges
    let ba = state.hab_ring_angle;
    let b_rad = [ba.cos(), ba.sin(), 0.0];
    let b_tan = [-ba.sin(), ba.cos(), 0.0];
    let fwd = [0.0, 0.0, 1.0];
    op.push_oriented_box(
        [0.0, 0.0, ring_z],
        b_rad,
        b_tan,
        fwd,
        [0.0, 0.0, 0.0],
        [13.4, 13.4, 5.4],
        Tile::DarkSteelRib,
        theme.rib_tint,
        0.0,
    );

    for i in 0..ring_segs {
        let a = state.hab_ring_angle + (i as f32 / ring_segs as f32) * TAU;
        let cx = a.cos() * ring_r;
        let cy = a.sin() * ring_r;
        // Strictly right-handed basis: cross(radial, tangent) == +Z (fwd)
        let radial = [a.cos(), a.sin(), 0.0];
        let tangent = [-a.sin(), a.cos(), 0.0];

        // Outer habitation module room (radial = 3.4 thick, tangent = 11.6 wide, Z = 5.0 deep)
        op.push_oriented_box(
            [cx, cy, ring_z],
            radial,
            tangent,
            fwd,
            [0.0, 0.0, 0.0],
            [3.4, 11.6, 5.0],
            Tile::HullPanel,
            theme.hull_tint,
            0.0,
        );
        // Continuous inner structural steel track girder uniting all 12 sectors
        let track_r = ring_r - 2.3;
        op.push_oriented_box(
            [a.cos() * track_r, a.sin() * track_r, ring_z],
            radial,
            tangent,
            fwd,
            [0.0, 0.0, 0.0],
            [1.2, 13.2, 3.2],
            Tile::DarkSteelRib,
            theme.rib_tint,
            0.0,
        );
        // Warm glowing crew viewport strip facing forward (+Z)
        op.push_oriented_box(
            [cx, cy, ring_z],
            radial,
            tangent,
            fwd,
            [0.0, 0.0, 2.58],
            [1.05, 7.6, 0.16],
            Tile::CeilingLight,
            sec,
            0.95,
        );
        // Heavy radial tension spoke anchored cleanly into the rotating central bearing drum
        if i % 3 == 0 {
            let spoke_mid_r = (6.4 + track_r) * 0.5;
            let spoke_len = (track_r - 6.4).max(4.0);
            op.push_oriented_box(
                [a.cos() * spoke_mid_r, a.sin() * spoke_mid_r, ring_z],
                radial,
                tangent,
                fwd,
                [0.0, 0.0, 0.0],
                [spoke_len, 2.0, 2.4],
                Tile::DarkSteelRib,
                theme.rib_tint,
                0.0,
            );
        }
    }

    // =========================================================================
    // 2. SPINNING DORSAL RADAR DISH, REACTOR PLASMA & OPS HOLOTABLE
    // =========================================================================
    let ra = state.radar_angle;
    let r_fwd = [ra.cos(), 0.0, ra.sin()];
    let r_right = [-ra.sin(), 0.0, ra.cos()];
    let r_up = [0.0, 1.0, 0.0];
    op.push_oriented_box(
        [0.0, 34.0, -3.0],
        r_right,
        r_up,
        r_fwd,
        [0.0, 0.0, 1.2],
        [7.6, 2.4, 0.7],
        Tile::DarkSteelRib,
        theme.hull_tint,
        0.0,
    );
    op.push_oriented_box(
        [0.0, 34.0, -3.0],
        r_right,
        r_up,
        r_fwd,
        [0.0, 0.0, 1.65],
        [6.4, 0.45, 0.25],
        Tile::CeilingLight,
        accent,
        1.0,
    );

    // Pulsing Reactor Plasma Core Column (Y = -14.2..-6.3)
    let core_pulse = 1.0 + 0.12 * (time * 4.2).sin();
    let core_col = if state.event == StationEventKind::SecurityAlert {
        red_strobe
    } else {
        accent
    };
    op.push_cylinder(
        [0.0, -10.2, 0.0],
        1.55 * core_pulse,
        1.55 * core_pulse,
        7.8,
        8,
        Tile::ReactorCoil,
        core_col,
        1.2,
    );
    tr.push_cylinder(
        [0.0, -10.2, 0.0],
        2.45,
        2.45,
        7.4,
        8,
        Tile::ForcefieldHex,
        [core_col[0], core_col[1], core_col[2], 0.36],
        1.8,
    );

    // Rotating Ops Deck Holographic Station Projection above Holotable (Y = 14.9, Z = 1.0)
    let holo_col = [accent[0], accent[1], accent[2], 0.55];
    tr.push_oriented_box(
        [0.0, 15.0, 1.0],
        r_right,
        r_up,
        r_fwd,
        [0.0, 0.0, 0.0],
        [2.8, 0.55, 0.85],
        Tile::ForcefieldHex,
        holo_col,
        1.9,
    );
    tr.push_oriented_box(
        [0.0, 15.0, 1.0],
        r_right,
        r_up,
        r_fwd,
        [0.0, 0.0, 0.0],
        [0.7, 1.8, 0.7],
        Tile::ForcefieldHex,
        holo_col,
        1.9,
    );

    // Hangar Approach Runway Chase Lights (along Z = 18..41 inside Hangar Bay)
    let chase_idx = ((time * 5.0) as i32).rem_euclid(5);
    for i in 0..5 {
        let z = 39.5 - i as f32 * 4.8;
        let lit = i == chase_idx || state.event == StationEventKind::DockingOps;
        let col = if state.event == StationEventKind::SecurityAlert {
            red_strobe
        } else if i == chase_idx {
            sec
        } else {
            [sec[0] * 0.4, sec[1] * 0.4, sec[2] * 0.4, 1.0]
        };
        let em = if lit { 1.15 } else { 0.6 };
        op.push_box(
            [-7.3, -3.65, z],
            [0.5, 0.25, 0.9],
            Tile::CeilingLight,
            0.5,
            col,
            em,
        );
        op.push_box(
            [7.3, -3.65, z],
            [0.5, 0.25, 0.9],
            Tile::CeilingLight,
            0.5,
            col,
            em,
        );
    }

    // =========================================================================
    // 3. PHYSICAL ORBITING 3D CAMERA DRONES (visible from other feeds!)
    // =========================================================================
    for drone in &state.drones {
        let look_dir = [
            drone.look_target[0] - drone.pos[0],
            drone.look_target[1] - drone.pos[1],
            drone.look_target[2] - drone.pos[2],
        ];
        let (r, u, f) = basis_from_forward(look_dir);
        // Armored sensor body
        op.push_oriented_box(
            drone.pos,
            r,
            u,
            f,
            [0.0, 0.0, 0.0],
            [2.0, 1.4, 2.6],
            Tile::ShipArmor,
            theme.hull_tint,
            0.0,
        );
        // Side stabilizing sensor fins
        op.push_oriented_box(
            drone.pos,
            r,
            u,
            f,
            [0.0, 0.0, -0.3],
            [4.2, 0.35, 1.1],
            Tile::DarkSteelRib,
            dark_steel,
            0.0,
        );
        // Glowing red camera optic lens on front
        op.push_oriented_box(
            drone.pos,
            r,
            u,
            f,
            [0.0, 0.0, 1.38],
            [0.75, 0.75, 0.25],
            Tile::CeilingLight,
            red_strobe,
            1.3,
        );
    }

    // =========================================================================
    // 4. 3D SPACECRAFT FLEET (4 Low-Poly Y2K Hull Classes)
    // =========================================================================
    let strobe_on = (time * 3.2).sin() > 0.3;
    for ship in &state.ships {
        let (r, u, f) = basis_from_forward(ship.forward);
        let p = ship.pos;

        match ship.class {
            ShipClass::CorporateShuttle => {
                // Sleek angular wedge fuselage
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.6, 0.0],
                    [3.6, 1.8, 7.6],
                    Tile::ShipArmor,
                    white,
                    0.0,
                );
                // Tapered forward nose & glowing cockpit slit
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.45, 4.6],
                    [2.4, 1.2, 2.4],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.1, 3.8],
                    [2.1, 0.45, 1.4],
                    Tile::CrtConsole,
                    accent,
                    1.1,
                );
                // Swept wing pods
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-3.0, 0.3, -1.2],
                    [2.8, 0.55, 4.2],
                    Tile::DarkSteelRib,
                    theme.rib_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [3.0, 0.3, -1.2],
                    [2.8, 0.55, 4.2],
                    Tile::DarkSteelRib,
                    theme.rib_tint,
                    0.0,
                );
                // Twin aft engine nacelles
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-1.5, 0.6, -4.1],
                    [1.1, 1.1, 1.6],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [1.5, 0.6, -4.1],
                    [1.1, 1.1, 1.6],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
            }
            ShipClass::BulkFreighter => {
                // Heavy boxy industrial twin-cargo-pod freighter
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.8, 0.0],
                    [3.2, 2.6, 12.0],
                    Tile::DarkSteelRib,
                    theme.rib_tint,
                    0.0,
                );
                // Forward bridge deck
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 3.3, 4.2],
                    [4.2, 1.6, 3.2],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 3.5, 5.85],
                    [3.4, 0.55, 0.25],
                    Tile::CrtConsole,
                    sec,
                    1.1,
                );
                // Port & Starboard massive industrial cargo containers
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-3.4, 1.5, -0.5],
                    [3.2, 2.8, 8.8],
                    Tile::CargoCrate,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [3.4, 1.5, -0.5],
                    [3.2, 2.8, 8.8],
                    Tile::CargoCrate,
                    white,
                    0.0,
                );
                // Aft thruster block
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.8, -6.6],
                    [6.4, 2.2, 1.8],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
            }
            ShipClass::PatrolCutter => {
                // Fast angular delta-wing security interceptor
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.5, 0.5],
                    [2.2, 1.1, 6.8],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.05, 1.8],
                    [1.4, 0.5, 1.8],
                    Tile::CrtConsole,
                    red_strobe,
                    1.15,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.35, -1.2],
                    [8.8, 0.35, 2.8],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.4, -2.2],
                    [0.35, 1.9, 2.0],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
            }
            ShipClass::MaintenanceTug => {
                // Chunky boxy utility tug with yellow hazard stripes & work lamp
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.2, 0.0],
                    [4.4, 2.4, 5.4],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.8, 2.85],
                    [3.2, 0.9, 0.3],
                    Tile::CrtConsole,
                    sec,
                    1.15,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-2.7, 1.2, -1.0],
                    [1.2, 1.6, 4.2],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [2.7, 1.2, -1.0],
                    [1.2, 1.6, 4.2],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
            }
        }

        // Articulated Landing Gear Struts when deployed
        if ship.gear_deploy > 0.08 {
            let h = 1.1 * ship.gear_deploy;
            for gx in [-1.8f32, 1.8f32] {
                for gz in [-2.2f32, 2.2f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [gx, -h * 0.4, gz],
                        [0.35, h, 0.35],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                }
            }
        }

        // Wingtip navigation strobes (Port = Red, Starboard = Green)
        if strobe_on {
            op.push_oriented_box(
                p,
                r,
                u,
                f,
                [-4.2, 0.5, -1.0],
                [0.42, 0.42, 0.42],
                Tile::CeilingLight,
                red_strobe,
                1.3,
            );
            op.push_oriented_box(
                p,
                r,
                u,
                f,
                [4.2, 0.5, -1.0],
                [0.42, 0.42, 0.42],
                Tile::CeilingLight,
                green_strobe,
                1.3,
            );
        }

        // Translucent Main Aft Plasma Thruster Plumes (only when main engines fire — never clips deck!)
        if ship.thruster_power > 0.08 {
            let plume_len =
                1.8 + ship.thruster_power * 5.2 + (time * 18.0 + ship.id as f32).sin() * 0.42;
            let aft_z = match ship.class {
                ShipClass::BulkFreighter => -7.8 - plume_len * 0.5,
                _ => -4.8 - plume_len * 0.5,
            };
            let plume_col = [accent[0], accent[1], accent[2], 0.68 * ship.thruster_power];
            tr.push_oriented_box(
                p,
                r,
                u,
                f,
                [-1.4, 0.7, aft_z],
                [0.95, 0.95, plume_len],
                Tile::ReactorCoil,
                plume_col,
                1.9,
            );
            tr.push_oriented_box(
                p,
                r,
                u,
                f,
                [1.4, 0.7, aft_z],
                [0.95, 0.95, plume_len],
                Tile::ReactorCoil,
                plume_col,
                1.9,
            );
        }

        // Downward Belly VTOL Hover Jets (for smooth vertical liftoff & touchdown)
        if ship.vtol_thrust > 0.08 {
            let v_len =
                (0.55 + ship.vtol_thrust * 1.15 + (time * 24.0 + ship.id as f32).sin() * 0.12)
                    .clamp(0.4, 1.8);
            let v_col = [sec[0], sec[1], sec[2], 0.58 * ship.vtol_thrust];
            let (vx, vz) = match ship.class {
                ShipClass::BulkFreighter => (2.6, 3.6),
                _ => (1.7, 2.0),
            };
            for sx in [-vx, vx] {
                for sz in [-vz, vz] {
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, -v_len * 0.45, sz],
                        [0.48, v_len, 0.48],
                        Tile::ReactorCoil,
                        v_col,
                        1.85,
                    );
                }
            }
        }

        // Forward Retro / RCS Braking & Hangar Reverse-Out Jets
        if ship.retro_thrust > 0.08 {
            let r_len = 0.7 + ship.retro_thrust * 1.9 + (time * 22.0 + ship.id as f32).sin() * 0.16;
            let r_col = [white[0], accent[1], accent[2], 0.50 * ship.retro_thrust];
            let fwd_z = match ship.class {
                ShipClass::BulkFreighter => 6.2 + r_len * 0.5,
                _ => 5.2 + r_len * 0.5,
            };
            for sx in [-1.45f32, 1.45f32] {
                tr.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [sx, 0.6, fwd_z],
                    [0.42, 0.42, r_len],
                    Tile::ReactorCoil,
                    r_col,
                    1.85,
                );
            }
        }
    }

    // =========================================================================
    // 5. AUTONOMOUS BOTS & INTERIOR CREW FIGURES
    // =========================================================================
    for agent in &state.agents {
        let p = agent.pos;
        let (sy, cy) = agent.yaw.sin_cos();
        let f = [sy, 0.0, cy];
        let r = [cy, 0.0, -sy];
        let u = [0.0, 1.0, 0.0];

        match agent.kind {
            AgentKind::CargoLoader => {
                // Detailed Y2K tracked industrial forklift / cargo-loader bot
                // 1. Twin heavy caterpillar tread pods (left & right)
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.78, 0.28, -0.05],
                    [0.36, 0.56, 2.15],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.78, 0.28, -0.05],
                    [0.36, 0.56, 2.15],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                // 2. Low-slung yellow/black hazard main chassis & rear counterweight engine block
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.56, -0.10],
                    [1.24, 0.62, 1.85],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.88, -0.82],
                    [1.18, 0.56, 0.55],
                    Tile::HullPanel,
                    theme.rib_tint,
                    0.0,
                );
                // 3. Armored sensor cab & glowing cyan optical scanner bar
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.24, -0.18],
                    [0.92, 0.72, 0.96],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.32, 0.33],
                    [0.68, 0.22, 0.08],
                    Tile::CrtConsole,
                    accent,
                    1.15,
                );
                // 4. Twin forward work headlights
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.45, 0.68, 0.85],
                    [0.20, 0.16, 0.10],
                    Tile::CeilingLight,
                    white,
                    1.2,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.45, 0.68, 0.85],
                    [0.20, 0.16, 0.10],
                    Tile::CeilingLight,
                    white,
                    1.2,
                );
                // 5. Twin vertical hydraulic lift mast rails
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.50, 1.05, 0.92],
                    [0.14, 1.75, 0.16],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.50, 1.05, 0.92],
                    [0.14, 1.75, 0.16],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                // 6. Articulated lifting carriage & twin steel forklift prongs
                let fork_y = 0.24 + agent.lift_height.clamp(0.0, 1.0) * 1.15;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, fork_y + 0.14, 0.98],
                    [1.12, 0.24, 0.12],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.36, fork_y, 1.46],
                    [0.14, 0.09, 0.96],
                    Tile::DarkSteelRib,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.36, fork_y, 1.46],
                    [0.14, 0.09, 0.96],
                    Tile::DarkSteelRib,
                    theme.hull_tint,
                    0.0,
                );
                // 7. Dynamic cargo crate resting on the raised forks when hauling!
                if agent.carrying_cargo {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, fork_y + 0.56, 1.50],
                        [1.15, 0.98, 1.05],
                        Tile::CargoCrate,
                        white,
                        0.0,
                    );
                }
                // 8. Flashing rooftop amber warning strobe
                if (time * 4.5 + agent.id as f32).sin() > 0.0 {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 1.72, -0.22],
                        [0.28, 0.24, 0.28],
                        Tile::CeilingLight,
                        sec,
                        1.35,
                    );
                }
            }
            AgentKind::SecurityDroid => {
                // System Shock 2 inspired boxy armored security patrol droid with glowing visor
                let bob = (agent.phase * 2.0).sin() * 0.12;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.2 + bob, 0.0],
                    [1.1, 1.6, 0.9],
                    Tile::DarkSteelRib,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 2.25 + bob, 0.0],
                    [0.85, 0.65, 0.85],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 2.3 + bob, 0.46],
                    [0.65, 0.22, 0.1],
                    Tile::CeilingLight,
                    red_strobe,
                    1.35,
                );
            }
            AgentKind::OpsCrew => {
                // Stylized low-poly Deus Ex station officer silhouette (legs + torso + head)
                let stride = agent.phase.sin() * 0.22;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.22, 0.55, stride],
                    [0.32, 1.1, 0.35],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.22, 0.55, -stride],
                    [0.32, 1.1, 0.35],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.55, 0.0],
                    [0.82, 1.05, 0.45],
                    Tile::CorridorWall,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 2.32, 0.0],
                    [0.46, 0.48, 0.46],
                    Tile::ShipArmor,
                    white,
                    0.0,
                );
            }
            AgentKind::HullWelder => {
                // Exterior hull maintenance welder drone with flickering arc spark
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.0, 0.0],
                    [1.3, 1.0, 1.5],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                if (time * 11.0 + agent.id as f32).sin() > 0.35 {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, -0.3, 1.0],
                        [0.55, 0.55, 0.55],
                        Tile::CeilingLight,
                        accent,
                        1.4,
                    );
                }
            }
        }
    }
}
