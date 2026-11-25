//! Per-frame dynamic 3D mesh generation for Station HELIOS-09:
//! - 4 low-poly spacecraft classes with landing gear, strobes & plasma thrusters
//! - 2 physical orbiting 3D camera drones
//! - Cargo loaders, interior security droids, ops crew & exterior welders
//! - Rotating habitation counterweight ring, spinning radar dishes, reactor plasma & runway chase lights

use std::f32::consts::TAU;

use rookery_core::orbital::simulation::REACTOR_CORE_Z;
use rookery_core::orbital::{
    AgentKind, AgentWorkAction, CargoVisualKind, HERMES_HOME_POS, OrbitalHavenState, PAD_A_POS,
    PAD_B_POS, PAD_C_POS, ServiceKind, ShipClass, StationEventKind,
};

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
    // 1. ROTATING HABITATION COUNTERWEIGHT RING (hollow annular drum around Axial Pipe Tunnel at Z = -20)
    // =========================================================================
    let ring_segs = 12;
    let ring_r = 25.0;
    let ring_z = -20.0;
    let fwd = [0.0, 0.0, 1.0];
    // Hollow 12-segment annular rotating bearing drum (`R = 6.3`, inner clearance `5.4`) so the Pipe Tunnel passes right through!
    for i in 0..12 {
        let a = state.hab_ring_angle + (i as f32 / 12.0) * TAU;
        let rad = [a.cos(), a.sin(), 0.0];
        let tan = [-a.sin(), a.cos(), 0.0];
        op.push_oriented_box(
            [a.cos() * 6.35, a.sin() * 6.35, ring_z],
            rad,
            tan,
            fwd,
            [0.0, 0.0, 0.0],
            [1.4, 3.2, 5.2],
            Tile::DarkSteelRib,
            theme.rib_tint,
            0.0,
        );
    }

    for i in 0..ring_segs {
        let a = state.hab_ring_angle + (i as f32 / ring_segs as f32) * TAU;
        let cx = a.cos() * ring_r;
        let cy = a.sin() * ring_r;
        // Strictly right-handed basis: cross(radial, tangent) == +Z (fwd)
        let radial = [a.cos(), a.sin(), 0.0];
        let tangent = [-a.sin(), a.cos(), 0.0];

        // Outer habitation module room (radial = 3.4 thick, tangent = 11.4 wide, Z = 5.0 deep — clean gap, zero overlap)
        op.push_oriented_box(
            [cx, cy, ring_z],
            radial,
            tangent,
            fwd,
            [0.0, 0.0, 0.0],
            [3.4, 11.4, 5.0],
            Tile::HullPanel,
            theme.hull_tint,
            0.0,
        );
        // Inner structural steel track girder trimmed cleanly to chord length (11.55 < 11.75) so adjacent girders never Z-fight
        let track_r = ring_r - 2.3;
        op.push_oriented_box(
            [a.cos() * track_r, a.sin() * track_r, ring_z],
            radial,
            tangent,
            fwd,
            [0.0, 0.0, 0.0],
            [1.15, 11.55, 3.1],
            Tile::DarkSteelRib,
            theme.rib_tint,
            0.0,
        );
        // Proud bolted structural coupler bracket at the midpoint between adjacent girders (wider Z = 3.55 -> zero coplanar face!)
        let joint_a = a + (0.5 / ring_segs as f32) * TAU;
        let j_rad = [joint_a.cos(), joint_a.sin(), 0.0];
        let j_tan = [-joint_a.sin(), joint_a.cos(), 0.0];
        let joint_r = track_r - 0.15;
        op.push_oriented_box(
            [joint_a.cos() * joint_r, joint_a.sin() * joint_r, ring_z],
            j_rad,
            j_tan,
            fwd,
            [0.0, 0.0, 0.0],
            [1.65, 1.5, 3.55],
            Tile::HazardStripe,
            white,
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
        // Heavy radial tension spoke anchored cleanly between the cylindrical bearing drum (R=6.7) and inner track (R=22.1)
        if i % 3 == 0 {
            let r_start = 6.7;
            let r_end = track_r - 0.55;
            let spoke_mid_r = (r_start + r_end) * 0.5;
            let spoke_len = (r_end - r_start).max(4.0);
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
        [0.0, -10.2, REACTOR_CORE_Z],
        1.55 * core_pulse,
        1.55 * core_pulse,
        7.8,
        8,
        Tile::ReactorCoil,
        core_col,
        1.2,
    );
    tr.push_cylinder(
        [0.0, -10.2, REACTOR_CORE_Z],
        2.45,
        2.45,
        7.4,
        8,
        Tile::ForcefieldHex,
        [core_col[0], core_col[1], core_col[2], 0.36],
        1.8,
    );

    // Bespoke 5-minute looping room activity machinery & progress visuals across all 5 rooms
    super::room_activities_mesh::build_room_activity_meshes(state, theme, time, op, tr);

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
            ShipClass::MedicalCourier => {
                // Crisp white-and-crimson triangular bio-rescue transport with flashing med-beacon & cryo-pods
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.05, 0.2],
                    [3.6, 1.5, 7.6],
                    Tile::ShipArmor,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.0, 4.6],
                    [2.2, 1.1, 2.2],
                    Tile::ShipArmor,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.48, 3.8],
                    [1.8, 0.45, 1.5],
                    Tile::CrtConsole,
                    accent,
                    1.2,
                );
                // Non-overlapping dorsal crimson medical cross (center longitudinal bar + 2 flush side arms)
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.88, -0.4],
                    [0.72, 0.14, 2.2],
                    Tile::CeilingLight,
                    red_strobe,
                    0.85,
                );
                for sx in [-0.73f32, 0.73f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 1.88, -0.4],
                        [0.74, 0.14, 0.72],
                        Tile::CeilingLight,
                        red_strobe,
                        0.85,
                    );
                }
                for sx in [-2.65f32, 2.65f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 0.95, -0.8],
                        [1.25, 1.15, 4.6],
                        Tile::ReactorCoil,
                        accent,
                        0.75,
                    );
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx * 1.48, 0.85, -1.6],
                        [1.8, 0.32, 3.2],
                        Tile::ShipArmor,
                        white,
                        0.0,
                    );
                }
                let med_col = if (time * 6.0 + ship.id as f32).sin() > 0.0 {
                    red_strobe
                } else {
                    white
                };
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 2.22, -2.6],
                    [0.52, 0.42, 0.52],
                    Tile::CeilingLight,
                    med_col,
                    1.4,
                );
            }
            ShipClass::SurveyProbeCarrier => {
                // Asymmetric Y2K science/recon vessel with spinning dorsal dish & glowing telemetry pods
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.4, 1.15, 0.0],
                    [3.4, 1.7, 7.2],
                    Tile::HullPanel,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [2.1, 1.05, 1.2],
                    [1.4, 1.25, 5.8],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [2.1, 1.12, 4.25],
                    [1.05, 0.72, 0.25],
                    Tile::CrtConsole,
                    accent,
                    1.35,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-3.2, 1.0, -0.8],
                    [2.4, 0.28, 4.2],
                    Tile::SolarCells,
                    white,
                    0.0,
                );
                let da = time * 2.2 + ship.id as f32;
                let (ds, dc) = da.sin_cos();
                let df = [
                    r[0] * ds + f[0] * dc,
                    r[1] * ds + f[1] * dc,
                    r[2] * ds + f[2] * dc,
                ];
                let dr = [
                    r[0] * dc - f[0] * ds,
                    r[1] * dc - f[1] * ds,
                    r[2] * dc - f[2] * ds,
                ];
                op.push_oriented_box(
                    p,
                    dr,
                    u,
                    df,
                    [-0.4, 2.35, -0.8],
                    [0.32, 0.75, 0.32],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    dr,
                    u,
                    df,
                    [-0.4, 2.82, -0.6],
                    [2.2, 0.92, 0.22],
                    Tile::HullPanel,
                    white,
                    0.25,
                );
                op.push_oriented_box(
                    p,
                    dr,
                    u,
                    df,
                    [-0.4, 2.82, -0.38],
                    [0.42, 0.42, 0.35],
                    Tile::CeilingLight,
                    accent,
                    1.3,
                );
            }
            ShipClass::FuelTanker => {
                // Heavy girder-spine plasma fuel tanker with 4 ribbed glowing fuel drums & quad VTOL pods
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.35, 0.0],
                    [1.6, 1.3, 11.4],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.65, 5.4],
                    [3.6, 1.9, 2.8],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.95, 6.88],
                    [2.6, 0.58, 0.2],
                    Tile::CrtConsole,
                    sec,
                    1.25,
                );
                for sx in [-2.25f32, 2.25f32] {
                    for sz in [-2.4f32, 1.8f32] {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 1.45, sz],
                            [2.1, 2.1, 3.4],
                            Tile::ReactorCoil,
                            sec,
                            0.85,
                        );
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 1.45, sz],
                            [2.28, 2.28, 0.55],
                            Tile::DarkSteelRib,
                            dark_steel,
                            0.0,
                        );
                    }
                }
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.4, -5.8],
                    [5.2, 2.2, 1.8],
                    Tile::ShipArmor,
                    theme.hull_tint,
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
                ShipClass::BulkFreighter | ShipClass::FuelTanker => -7.8 - plume_len * 0.5,
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
                ShipClass::BulkFreighter | ShipClass::FuelTanker => (2.6, 3.6),
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
                ShipClass::BulkFreighter | ShipClass::FuelTanker => 6.2 + r_len * 0.5,
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
    // 4B. DYNAMIC PAD SERVICE HARDWARE & DOCKED-SHIP CHOREOGRAPHY (Pads A, B, C)
    // =========================================================================
    let pad_anchors: [([f32; 3], f32, f32); 3] = [
        (PAD_A_POS, 1.0, -3.88),
        (PAD_B_POS, 1.0, -1.42),
        (PAD_C_POS, -1.0, -1.42),
    ];

    for (pad_idx, &(pad_pos, side, floor_y)) in pad_anchors.iter().enumerate() {
        let svc = &state.pad_services[pad_idx];
        let active = svc.active_kind();

        // 1. Articulated 3-Joint Gantry Repair / Welding Arm
        let base_x = pad_pos[0] + side * 7.8;
        let base_z = pad_pos[2] - 2.6;
        let base_y = floor_y + 1.8;
        op.push_box(
            [base_x, floor_y + 0.95, base_z],
            [1.4, 1.9, 1.4],
            Tile::HazardStripe,
            2.0,
            white,
            0.0,
        );
        let arm_t = svc.repair_arm_ext.clamp(0.0, 1.0);
        let sweep_z = (time * 1.9 + pad_idx as f32).sin() * 2.2 * arm_t;
        let elbow_x = base_x - side * (0.6 + arm_t * 3.1);
        let elbow_y = base_y + 1.4 + arm_t * 1.6;
        let elbow_z = base_z + sweep_z * 0.4;
        let tip_x = base_x - side * (1.0 + arm_t * 5.6);
        let tip_y = floor_y + 2.5 + (1.0 - arm_t) * 1.8;
        let tip_z = pad_pos[2] + sweep_z;

        let d1 = [elbow_x - base_x, elbow_y - base_y, elbow_z - base_z];
        let l1 = (d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2])
            .sqrt()
            .max(0.4);
        let (r1, u1, f1) = basis_from_forward(d1);
        op.push_oriented_box(
            [
                (base_x + elbow_x) * 0.5,
                (base_y + elbow_y) * 0.5,
                (base_z + elbow_z) * 0.5,
            ],
            r1,
            u1,
            f1,
            [0.0, 0.0, 0.0],
            [0.42, 0.42, l1],
            Tile::DarkSteelRib,
            dark_steel,
            0.0,
        );
        op.push_box(
            [elbow_x, elbow_y, elbow_z],
            [0.72, 0.72, 0.72],
            Tile::HazardStripe,
            1.5,
            white,
            0.0,
        );
        let d2 = [tip_x - elbow_x, tip_y - elbow_y, tip_z - elbow_z];
        let l2 = (d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2])
            .sqrt()
            .max(0.4);
        let (r2, u2, f2) = basis_from_forward(d2);
        op.push_oriented_box(
            [
                (elbow_x + tip_x) * 0.5,
                (elbow_y + tip_y) * 0.5,
                (elbow_z + tip_z) * 0.5,
            ],
            r2,
            u2,
            f2,
            [0.0, 0.0, 0.0],
            [0.32, 0.32, l2],
            Tile::ShipArmor,
            theme.hull_tint,
            0.0,
        );
        if arm_t > 0.72 && (time * 18.0 + pad_idx as f32).sin() > -0.1 {
            let spark_s = 0.45 + ((time * 31.0).sin().abs()) * 0.35;
            op.push_box(
                [tip_x, tip_y - 0.15, tip_z],
                [spark_s, spark_s, spark_s],
                Tile::CeilingLight,
                1.0,
                accent,
                1.6,
            );
            tr.push_box(
                [tip_x, tip_y - 0.15, tip_z],
                [spark_s * 2.1, spark_s * 2.1, spark_s * 2.1],
                Tile::ForcefieldHex,
                1.0,
                [accent[0], accent[1], accent[2], 0.42],
                1.9,
            );
        }

        // 2. Cryo-Fuel Umbilical Boom with chasing plasma-flow pulse nodes
        let fuel_base_x = pad_pos[0] + side * 7.8;
        let fuel_base_z = pad_pos[2] + 2.4;
        let fuel_y = floor_y + 2.2;
        op.push_box(
            [fuel_base_x, floor_y + 1.1, fuel_base_z],
            [1.2, 2.2, 1.2],
            Tile::DarkSteelRib,
            2.0,
            theme.rib_tint,
            0.0,
        );
        let fuel_t = svc.fuel_boom_ext.clamp(0.0, 1.0);
        if fuel_t > 0.04 {
            let reach = fuel_t * 5.2;
            let mid_x = fuel_base_x - side * (reach * 0.5);
            op.push_box(
                [mid_x, fuel_y, fuel_base_z],
                [reach.max(0.3), 0.36, 0.36],
                Tile::DarkSteelRib,
                2.0,
                dark_steel,
                0.0,
            );
            let nozzle_x = fuel_base_x - side * reach;
            op.push_box(
                [nozzle_x, fuel_y, fuel_base_z],
                [0.52, 0.58, 0.58],
                Tile::ReactorCoil,
                1.5,
                sec,
                1.15,
            );
            if fuel_t > 0.72 {
                for k in 0..3 {
                    let flow = ((time * 1.4 + k as f32 * 0.33) % 1.0).clamp(0.0, 1.0);
                    let px = fuel_base_x - side * (flow * reach);
                    op.push_box(
                        [px, fuel_y + 0.02, fuel_base_z],
                        [0.44, 0.46, 0.46],
                        Tile::CeilingLight,
                        1.0,
                        accent,
                        1.45,
                    );
                }
            }
        }

        // 3. Holographic Laser Bio-Scan Curtain
        if active == Some(ServiceKind::BioScanSweep) {
            let sweep_u = (svc.scan_sweep_t * 2.0).sin().abs().clamp(0.0, 1.0);
            let scan_z = pad_pos[2] - 5.8 + sweep_u * 11.6;
            let scan_col = [accent[0], 1.0, accent[2], 0.28];
            tr.push_box(
                [pad_pos[0], floor_y + 2.4, scan_z],
                [9.4, 4.4, 0.28],
                Tile::ForcefieldHex,
                2.0,
                scan_col,
                1.75,
            );
            for sx in [-5.0f32, 5.0f32] {
                op.push_box(
                    [pad_pos[0] + sx, floor_y + 1.8, scan_z],
                    [0.32, 3.6, 0.32],
                    Tile::CeilingLight,
                    1.5,
                    accent,
                    1.35,
                );
            }
        }

        // 4. Pair of Hovering Pad Resupply Micro-Drones
        let drone_t = svc.drone_deploy.clamp(0.0, 1.0);
        if drone_t > 0.05 {
            for d_side in [-1.0f32, 1.0f32] {
                let orbit_a = time * 1.6 * d_side + pad_idx as f32;
                let dx =
                    pad_pos[0] + d_side * (5.6 - drone_t * 1.9) + orbit_a.cos() * 0.5 * drone_t;
                let dy = floor_y + 0.9 + drone_t * 2.4 + (time * 3.8 + d_side).sin() * 0.22;
                let dz = pad_pos[2] + orbit_a.sin() * 2.2 * drone_t;
                op.push_box(
                    [dx, dy, dz],
                    [0.82, 0.44, 0.92],
                    Tile::HazardStripe,
                    1.5,
                    white,
                    0.0,
                );
                op.push_box(
                    [dx, dy - 0.34, dz],
                    [0.44, 0.22, 0.44],
                    Tile::CeilingLight,
                    1.0,
                    accent,
                    1.35,
                );
                if svc.step_progress < 0.55 {
                    op.push_box(
                        [dx, dy + 0.34, dz],
                        [0.42, 0.32, 0.56],
                        Tile::ReactorCoil,
                        1.0,
                        sec,
                        1.2,
                    );
                }
            }
        }

        // 5. Illuminated Ship Boarding Ramp & Wheeled Courier / Med-Cart Exchange
        let ramp_t = svc.ramp_deploy.clamp(0.0, 1.0);
        if ramp_t > 0.05 {
            let ramp_dir = -side;
            let ramp_x = pad_pos[0] + ramp_dir * (2.6 + ramp_t * 1.2);
            let ramp_z = pad_pos[2] - 1.2;
            op.push_box(
                [ramp_x, floor_y + 0.35, ramp_z],
                [2.2 * ramp_t, 0.16, 1.6],
                Tile::FloorGrate,
                1.8,
                white,
                0.0,
            );
            op.push_box(
                [ramp_x, floor_y + 0.46, ramp_z - 0.82],
                [2.2 * ramp_t, 0.14, 0.12],
                Tile::CeilingLight,
                1.0,
                accent,
                1.2,
            );
            op.push_box(
                [ramp_x, floor_y + 0.46, ramp_z + 0.82],
                [2.2 * ramp_t, 0.14, 0.12],
                Tile::CeilingLight,
                1.0,
                accent,
                1.2,
            );
            let cart_u = (svc.step_progress * std::f32::consts::PI)
                .sin()
                .clamp(0.0, 1.0);
            let cart_x = pad_pos[0] + ramp_dir * (7.2 - cart_u * 2.4);
            op.push_box(
                [cart_x, floor_y + 0.42, ramp_z],
                [1.05, 0.46, 0.78],
                Tile::ShipArmor,
                1.5,
                white,
                0.0,
            );
            op.push_box(
                [cart_x, floor_y + 0.80, ramp_z],
                [0.52, 0.36, 0.48],
                Tile::CeilingLight,
                1.0,
                if svc.step_progress < 0.5 { accent } else { sec },
                1.3,
            );
        }
    }

    // =========================================================================
    // 5. AUTONOMOUS WHEELED & TRACKED STATION ROBOTS
    // =========================================================================
    for agent in &state.agents {
        let p = agent.pos;
        let (sy, cy) = agent.yaw.sin_cos();
        let f = [sy, 0.0, cy];
        let r = [cy, 0.0, -sy];
        let u = [0.0, 1.0, 0.0];

        match agent.kind {
            AgentKind::CargoForklift => {
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
                // 7. Dynamic cargo crate or compacted alloy bale resting on the raised forks when hauling
                if agent.carrying_cargo || agent.cargo_kind != CargoVisualKind::None {
                    if agent.cargo_kind == CargoVisualKind::CompactedBale {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [0.0, fork_y + 0.28, 1.50],
                            [1.22, 0.42, 1.05],
                            Tile::ShipArmor,
                            [0.70, 0.78, 0.86, 1.0],
                            0.0,
                        );
                    } else {
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
                }
                // 8. Flashing rooftop amber warning strobe (faster when yielding)
                let strobe_rate = if agent.yielding { 10.0 } else { 4.5 };
                if (time * strobe_rate + agent.id as f32).sin() > 0.0 {
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
            AgentKind::FloorScrubber => {
                // Compact 3-wheel custodial floor scrubber with spinning front brush roller
                // 1. Rear twin drive wheels & front caster wheel
                for sx in [-0.52f32, 0.52f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 0.22, -0.32],
                        [0.18, 0.44, 0.44],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                }
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.16, 0.42],
                    [0.22, 0.32, 0.32],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                // 2. Low octagonal-style industrial drum body + hazard bumper
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.48, -0.05],
                    [0.98, 0.42, 1.05],
                    Tile::HullPanel,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.36, 0.48],
                    [1.06, 0.18, 0.16],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                // 3. Spinning yellow front scrubber brush roller
                let spin_osc = (time * 16.0 + agent.id as f32).sin() * 0.05;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.16 + spin_osc.abs() * 0.4, 0.64],
                    [0.96, 0.24, 0.24],
                    Tile::HazardStripe,
                    sec,
                    0.25,
                );
                // 4. Rear waste tank & glowing cyan optical sensor dome
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.86, -0.26],
                    [0.72, 0.44, 0.52],
                    Tile::ShipArmor,
                    theme.rib_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.82, 0.20],
                    [0.36, 0.26, 0.36],
                    Tile::CeilingLight,
                    accent,
                    1.15,
                );
                // 5. Active cyan sanitizer foam/mist ring underneath chassis while scrubbing
                if agent.dwell_timer > 0.0
                    && matches!(
                        agent.work_action,
                        AgentWorkAction::Scrubbing | AgentWorkAction::PurgingWaste
                    )
                {
                    let foam_r = 1.18 + (time * 8.0 + agent.id as f32).sin().abs() * 0.26;
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 0.08, 0.32],
                        [foam_r, 0.10, foam_r],
                        Tile::ForcefieldHex,
                        [accent[0], accent[1], accent[2], 0.38],
                        1.5,
                    );
                }
            }
            AgentKind::PanelTechBot => {
                // Nimble 4-wheel Atrium control-panel calibration robot with twin articulated arms
                let is_working = agent.work_action == AgentWorkAction::CalibratingPanel;
                let is_celebrating = agent.work_action == AgentWorkAction::Celebrating;
                let bounce_y = if is_celebrating {
                    (time * 9.0 + agent.id as f32).sin().abs() * 0.24
                } else if is_working {
                    (time * 11.0 + agent.id as f32).sin().abs() * 0.06
                } else {
                    0.0
                };

                // Multicolor cycling helper for active work & celebration
                let palette = [
                    [0.12, 0.88, 1.0, 1.0],
                    [1.0, 0.22, 0.82, 1.0],
                    [1.0, 0.84, 0.14, 1.0],
                    [0.20, 1.0, 0.45, 1.0],
                    [1.0, 0.46, 0.12, 1.0],
                ];
                let mc_col =
                    palette[((time * 10.0 + agent.id as f32 * 1.7) as usize) % palette.len()];
                let visor_col = if is_working || is_celebrating {
                    mc_col
                } else {
                    [0.16, 0.62, 1.0, 1.0]
                };

                // 1. Four compact omni-wheels
                for sx in [-0.42f32, 0.42f32] {
                    for sz in [-0.38f32, 0.38f32] {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 0.16, sz],
                            [0.16, 0.32, 0.32],
                            Tile::DarkSteelRib,
                            dark_steel,
                            0.0,
                        );
                    }
                }
                // 2. Lower hazard bumper & nimble upright tech chassis
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.34 + bounce_y * 0.4, 0.0],
                    [0.88, 0.24, 0.86],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.94 + bounce_y, -0.04],
                    [0.74, 0.96, 0.66],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                // 3. Glowing chest diagnostic visor
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.08 + bounce_y, 0.32],
                    [0.54, 0.36, 0.08],
                    Tile::CrtConsole,
                    visor_col,
                    1.38,
                );
                // 4. Articulated twin calibration arms (rapidly tap/tune panel while working, raised while celebrating!)
                for (side_idx, sx) in [-0.46f32, 0.46f32].iter().enumerate() {
                    let tap = if is_working {
                        (time * 15.0 + side_idx as f32 * std::f32::consts::PI + agent.id as f32)
                            .sin()
                    } else {
                        0.0
                    };
                    let arm_y = if is_celebrating {
                        1.52 + bounce_y + (time * 12.0 + side_idx as f32 * 2.0).sin() * 0.16
                    } else if is_working {
                        1.14 + bounce_y + tap * 0.20
                    } else {
                        0.92 + bounce_y
                    };
                    let arm_z = if is_working {
                        0.48 + tap.abs() * 0.22
                    } else if is_celebrating {
                        0.18
                    } else {
                        0.14
                    };
                    let arm_len = if is_working { 0.78 } else { 0.46 };
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [*sx, arm_y, arm_z],
                        [0.14, 0.16, arm_len],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                    // Glowing fingertip tool probe
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [*sx, arm_y, arm_z + arm_len * 0.52],
                        [0.14, 0.14, 0.16],
                        Tile::CeilingLight,
                        if is_working || is_celebrating {
                            mc_col
                        } else {
                            accent
                        },
                        1.4,
                    );
                }
                // 5. Rotating rooftop diagnostic beacon dome
                let beacon_yaw = agent.yaw
                    + time
                        * if is_working || is_celebrating {
                            8.0
                        } else {
                            2.0
                        };
                let (bsy, bcy) = beacon_yaw.sin_cos();
                let bf = [bsy, 0.0, bcy];
                let br = [bcy, 0.0, -bsy];
                op.push_oriented_box(
                    p,
                    br,
                    u,
                    bf,
                    [0.0, 1.56 + bounce_y, -0.04],
                    [0.34, 0.28, 0.34],
                    Tile::CeilingLight,
                    visor_col,
                    1.42,
                );
                // 6. Multicolor holographic interface beam & sparks while working on a panel!
                if is_working {
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 1.18 + bounce_y, 1.12],
                        [0.78, 0.56, 0.88],
                        Tile::ForcefieldHex,
                        [mc_col[0], mc_col[1], mc_col[2], 0.36],
                        1.8,
                    );
                } else if is_celebrating {
                    tr.push_oriented_box(
                        p,
                        br,
                        u,
                        bf,
                        [0.0, 1.98 + bounce_y, 0.0],
                        [0.95, 0.18, 0.95],
                        Tile::ForcefieldHex,
                        [mc_col[0], mc_col[1], mc_col[2], 0.48],
                        1.9,
                    );
                }
            }
            AgentKind::OpsHoloDroid => {
                // 4-wheel UNATCO telemetry droid with 3 glowing CRT displays & spinning radar dish
                // 1. 4 omni-wheel pods at corners
                for sx in [-0.46f32, 0.46f32] {
                    for sz in [-0.44f32, 0.44f32] {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 0.18, sz],
                            [0.18, 0.36, 0.36],
                            Tile::DarkSteelRib,
                            dark_steel,
                            0.0,
                        );
                    }
                }
                // 2. Low base chassis & central telemetry pedestal
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.38, 0.0],
                    [0.96, 0.28, 0.96],
                    Tile::DarkSteelRib,
                    theme.rib_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.08, 0.0],
                    [0.64, 1.12, 0.64],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                // 3. Three glowing CRT telemetry screens (front, left, right)
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.24, 0.35],
                    [0.50, 0.42, 0.08],
                    Tile::CrtConsole,
                    accent,
                    1.25,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [-0.35, 1.24, 0.0],
                    [0.08, 0.38, 0.46],
                    Tile::CrtConsole,
                    accent,
                    1.1,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.35, 1.24, 0.0],
                    [0.08, 0.38, 0.46],
                    Tile::CrtConsole,
                    accent,
                    1.1,
                );
                // 4. Spinning rooftop micro-radar dish
                let dish_yaw = agent.yaw + time * 2.4 + agent.id as f32;
                let (dsy, dcy) = dish_yaw.sin_cos();
                let df = [dsy, 0.0, dcy];
                let dr = [dcy, 0.0, -dsy];
                op.push_oriented_box(
                    p,
                    dr,
                    u,
                    df,
                    [0.0, 1.76, 0.0],
                    [0.14, 0.26, 0.14],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    dr,
                    u,
                    df,
                    [0.0, 1.94, 0.10],
                    [0.54, 0.26, 0.10],
                    Tile::HullPanel,
                    white,
                    0.35,
                );
                // 5. Floating holographic Data-Packet Cube above rooftop dish when transporting telemetry
                if agent.cargo_kind == CargoVisualKind::HoloDataCube {
                    let cube_bob = 2.42 + (time * 4.5 + agent.id as f32).sin() * 0.12;
                    tr.push_oriented_box(
                        p,
                        dr,
                        u,
                        df,
                        [0.0, cube_bob, 0.0],
                        [0.42, 0.42, 0.42],
                        Tile::ForcefieldHex,
                        [accent[0], accent[1], accent[2], 0.68],
                        1.85,
                    );
                }
                // 6. Translucent data-sync / holotable upload projection beam while dwelling
                if agent.dwell_timer > 0.0 {
                    let beam_col = if agent.work_action == AgentWorkAction::UploadingHolo {
                        [sec[0], sec[1], sec[2], 0.36]
                    } else {
                        [accent[0], accent[1], accent[2], 0.30]
                    };
                    let beam_len = if agent.work_action == AgentWorkAction::UploadingHolo {
                        2.1
                    } else {
                        1.15
                    };
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 1.38, 0.55 + beam_len * 0.5],
                        [0.46, 0.38, beam_len],
                        Tile::ForcefieldHex,
                        beam_col,
                        1.65,
                    );
                }
            }
            AgentKind::SecurityEnforcer => {
                // Heavy twin-tracked System Shock 2 security enforcer with sweeping turret torso
                // 1. Wide armored tank treads (left & right)
                for sx in [-0.64f32, 0.64f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 0.28, 0.0],
                        [0.34, 0.56, 1.62],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                }
                // 2. Armored lower track hull
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.48, 0.0],
                    [0.98, 0.44, 1.38],
                    Tile::ShipArmor,
                    theme.rib_tint,
                    0.0,
                );
                // 3. Slowly sweeping upper turret torso + glowing crimson visor slit
                let sweep = (time * 1.35 + agent.id as f32 * 1.7).sin() * 0.42;
                let (tsy, tcy) = (agent.yaw + sweep).sin_cos();
                let tf = [tsy, 0.0, tcy];
                let tr_vec = [tcy, 0.0, -tsy];
                op.push_oriented_box(
                    p,
                    tr_vec,
                    u,
                    tf,
                    [0.0, 1.26, 0.0],
                    [0.96, 1.08, 0.88],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    tr_vec,
                    u,
                    tf,
                    [0.0, 1.92, 0.04],
                    [0.72, 0.44, 0.72],
                    Tile::DarkSteelRib,
                    dark_steel,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    tr_vec,
                    u,
                    tf,
                    [0.0, 1.94, 0.42],
                    [0.62, 0.16, 0.08],
                    Tile::CeilingLight,
                    red_strobe,
                    1.45,
                );
                // Twin side sensor / stun-emitter pods
                for sx in [-0.62f32, 0.62f32] {
                    op.push_oriented_box(
                        p,
                        tr_vec,
                        u,
                        tf,
                        [sx, 1.34, 0.24],
                        [0.24, 0.28, 0.78],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                }
                // Volumetric vertical laser audit fan while scanning Atrium server pillars
                if agent.dwell_timer > 0.0 && agent.work_action == AgentWorkAction::Scanning {
                    let scan_y = 1.55 + (time * 3.8 + agent.id as f32).sin() * 0.75;
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, scan_y, 1.45],
                        [1.35, 0.16, 2.0],
                        Tile::ForcefieldHex,
                        [red_strobe[0], red_strobe[1], red_strobe[2], 0.36],
                        1.75,
                    );
                }
            }
            AgentKind::ReactorCoolantBot => {
                // 4-wheel heavy reactor coolant tanker with twin glowing cyan cryo canisters
                // 1. 4 heavy industrial wheels
                for sx in [-0.64f32, 0.64f32] {
                    for sz in [-0.56f32, 0.56f32] {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 0.28, sz],
                            [0.24, 0.56, 0.56],
                            Tile::DarkSteelRib,
                            dark_steel,
                            0.0,
                        );
                    }
                }
                // 2. Squat yellow/black hazard tanker chassis
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.58, 0.0],
                    [1.12, 0.56, 1.56],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                // 3. Twin glowing cyan cryo-coolant pressure tanks mounted on top
                for sx in [-0.30f32, 0.30f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 1.12, -0.14],
                        [0.42, 0.54, 1.18],
                        Tile::ReactorCoil,
                        accent,
                        1.15,
                    );
                }
                // 4. Extending front thermal inspection / coolant nozzle arm
                let ext = 0.85 + agent.lift_height.clamp(0.0, 1.0) * 0.65;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.86, ext],
                    [0.22, 0.22, 0.76],
                    Tile::DarkSteelRib,
                    theme.hull_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.86, ext + 0.42],
                    [0.26, 0.26, 0.14],
                    Tile::CeilingLight,
                    accent,
                    1.2,
                );
                // 5. Dorsal carried Fresh Cryo-Cell (bright cyan) or Spent Fuel Rod (glowing orange)
                if matches!(
                    agent.cargo_kind,
                    CargoVisualKind::FreshCryoCell | CargoVisualKind::SpentFuelRod
                ) {
                    let rod_col = if agent.cargo_kind == CargoVisualKind::SpentFuelRod {
                        [1.0, 0.34, 0.08, 1.0]
                    } else {
                        accent
                    };
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 1.62, -0.08],
                        [0.36, 0.36, 1.38],
                        Tile::ReactorCoil,
                        rod_col,
                        1.45,
                    );
                }
                // 6. Translucent cryo-vapor mist burst when swapping a reactor rod
                if agent.dwell_timer > 0.0 && agent.work_action == AgentWorkAction::SwappingCryoRod
                {
                    let puff = 0.65 + (time * 9.0 + agent.id as f32).sin().abs() * 0.45;
                    tr.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 1.05, ext + 0.85],
                        [puff, puff * 0.9, puff],
                        Tile::ForcefieldHex,
                        [0.72, 0.96, 1.0, 0.38],
                        1.65,
                    );
                }
            }
            AgentKind::SolarCapacitorBot => {
                // Twin-tracked high-voltage solar calibration bot with copper coil pack & probe arm
                // 1. Narrow twin tracks
                for sx in [-0.50f32, 0.50f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 0.24, 0.0],
                        [0.26, 0.48, 1.38],
                        Tile::DarkSteelRib,
                        dark_steel,
                        0.0,
                    );
                }
                // 2. Tall insulated high-voltage body
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.92, 0.0],
                    [0.78, 1.08, 1.02],
                    Tile::ShipArmor,
                    theme.hull_tint,
                    0.0,
                );
                // 3. Amber copper-coil capacitor backpack
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 1.08, -0.58],
                    [0.64, 0.76, 0.34],
                    Tile::ReactorCoil,
                    sec,
                    0.95,
                );
                // 4. Extending calibration probe arm & flickering amber arc when calibrating
                let arm_h = 1.15 + agent.lift_height.clamp(0.0, 1.0) * 0.55;
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, arm_h, 0.76],
                    [0.18, 0.18, 0.78],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                if agent.dwell_timer > 0.2 && (time * 14.0 + agent.id as f32).sin() > 0.15 {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, arm_h, 1.24],
                        [0.34, 0.34, 0.34],
                        Tile::CeilingLight,
                        sec,
                        1.5,
                    );
                    if agent.work_action == AgentWorkAction::CalibratingCapacitor {
                        tr.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [0.0, arm_h + 0.15, 1.82],
                            [0.24, 0.24, 1.15],
                            Tile::ForcefieldHex,
                            [sec[0], sec[1], sec[2], 0.52],
                            1.9,
                        );
                    }
                }
            }
            AgentKind::CourierRunner => {
                // Zippy low-profile 4-wheel cross-room data & parts courier rover
                // 1. 4 sporty dark wheel pods
                for sx in [-0.48f32, 0.48f32] {
                    for sz in [-0.46f32, 0.46f32] {
                        op.push_oriented_box(
                            p,
                            r,
                            u,
                            f,
                            [sx, 0.20, sz],
                            [0.18, 0.40, 0.40],
                            Tile::DarkSteelRib,
                            dark_steel,
                            0.0,
                        );
                    }
                }
                // 2. Low-slung angular wedge chassis with amber hazard stripe
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.44, 0.0],
                    [0.84, 0.34, 1.32],
                    Tile::DarkSteelRib,
                    theme.rib_tint,
                    0.0,
                );
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.64, -0.08],
                    [0.72, 0.20, 0.96],
                    Tile::HazardStripe,
                    white,
                    0.0,
                );
                // 3. Forward white headlights & twin red rear micro-taillights
                op.push_oriented_box(
                    p,
                    r,
                    u,
                    f,
                    [0.0, 0.48, 0.68],
                    [0.54, 0.14, 0.08],
                    Tile::CeilingLight,
                    white,
                    1.25,
                );
                for sx in [-0.28f32, 0.28f32] {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [sx, 0.48, -0.68],
                        [0.16, 0.12, 0.08],
                        Tile::CeilingLight,
                        red_strobe,
                        1.3,
                    );
                }
                // 4. Glowing cyan data-core canister strapped on top when carrying a delivery
                if agent.carrying_cargo {
                    op.push_oriented_box(
                        p,
                        r,
                        u,
                        f,
                        [0.0, 0.92, -0.08],
                        [0.44, 0.36, 0.56],
                        Tile::ReactorCoil,
                        accent,
                        1.35,
                    );
                }
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

    // =========================================================================
    // 6. FUNCTIONING 3-DECK GLASS ELEVATOR CAR, SLIDING BLAST DOORS & WALKER BOT
    // =========================================================================
    // 6A. Animated 3-Deck Elevator Car (`X = 0, Z = -6.0, Y = state.elevator.current_y`)
    let ey = state.elevator.current_y;
    let ez = -6.0;
    // Grated elevator floor platform & hazard trim
    op.push_box(
        [0.0, ey - 0.16, ez],
        [4.6, 0.32, 4.6],
        Tile::FloorGrate,
        2.5,
        theme.hull_tint,
        0.0,
    );
    op.push_box(
        [0.0, ey - 0.16, ez + 2.35],
        [4.6, 0.34, 0.2],
        Tile::HazardStripe,
        2.0,
        white,
        0.0,
    );
    // Elevator roof cap & glowing interior overhead strip
    op.push_box(
        [0.0, ey + 3.55, ez],
        [4.6, 0.35, 4.6],
        Tile::DarkSteelRib,
        2.5,
        theme.rib_tint,
        0.0,
    );
    op.push_box(
        [0.0, ey + 3.32, ez],
        [2.4, 0.14, 2.4],
        Tile::CeilingLight,
        2.4,
        accent,
        1.1,
    );
    // 4 vertical cage corner struts
    for cx in [-2.15f32, 2.15f32] {
        for cz in [-2.15f32, 2.15f32] {
            op.push_box(
                [cx, ey + 1.7, ez + cz],
                [0.24, 3.4, 0.24],
                Tile::DarkSteelRib,
                2.0,
                dark_steel,
                0.0,
            );
        }
    }
    // Translucent glass side & rear panels on the elevator car so you see the Atrium/Ops/Reactor while riding!
    let glass_col = [accent[0], accent[1], accent[2], 0.22];
    tr.push_quad(
        [-2.1, ey + 0.1, ez - 2.1],
        [2.1, ey + 0.1, ez - 2.1],
        [2.1, ey + 3.3, ez - 2.1],
        [-2.1, ey + 3.3, ez - 2.1],
        [0.0, 0.0, 1.0],
        [2.0, 1.5],
        Tile::ForcefieldHex,
        glass_col,
        1.6,
    );
    // Twin animated sliding elevator doors on the `+Z` front side (`ez + 2.15`)
    let e_slide = state.elevator.door_open.clamp(0.0, 1.0) * 1.95;
    let e_door_w = (2.1 - e_slide * 0.4).max(0.25);
    op.push_box(
        [-1.08 - e_slide * 0.52, ey + 1.68, ez + 2.15],
        [e_door_w, 3.25, 0.16],
        Tile::BlastDoor,
        3.2,
        white,
        0.0,
    );
    op.push_box(
        [1.08 + e_slide * 0.52, ey + 1.68, ez + 2.15],
        [e_door_w, 3.25, 0.16],
        Tile::BlastDoor,
        3.2,
        white,
        0.0,
    );

    // 6B. 3 Pairs of Proximity-Triggered Sliding Blast Doors on Deck 2
    let door_specs = [
        // (z, half_open_w, height, open_t)
        (11.6f32, 3.15f32, 7.0f32, state.door_open[0]),
        (-14.6f32, 2.55f32, 5.6f32, state.door_open[1]),
        (-25.1f32, 2.55f32, 5.6f32, state.door_open[2]),
    ];
    for (dz, half_w, dh, open_t) in door_specs {
        let o = open_t.clamp(0.0, 1.0);
        let leaf_w = (half_w * (1.0 - o * 0.84)).max(0.18);
        let cx_off = half_w - leaf_w * 0.5;
        let cy = -3.85 + dh * 0.5;
        op.push_box(
            [-cx_off, cy, dz],
            [leaf_w, dh, 0.36],
            Tile::BlastDoor,
            4.5,
            white,
            0.0,
        );
        op.push_box(
            [cx_off, cy, dz],
            [leaf_w, dh, 0.36],
            Tile::BlastDoor,
            4.5,
            white,
            0.0,
        );
    }

    // 6B-2. Port (`[0]`) & Starboard (`[1]`) Pressurized Pad Airlock Sliding Doors & Status Beacons
    for (air_idx, side) in [-1.0f32, 1.0f32].into_iter().enumerate() {
        let al = &state.pad_airlocks[air_idx];
        let airlock_doors = [(side * 23.2, al.inner_door), (side * 28.6, al.outer_door)];
        let half_w = 2.25f32;
        let dh = 3.80f32;
        let cy = -1.42 + dh * 0.5;
        for (dx, open_t) in airlock_doors {
            let o = open_t.clamp(0.0, 1.0);
            let leaf_w = (half_w * (1.0 - o * 0.84)).max(0.18);
            let cz_off = half_w - leaf_w * 0.5 + o * 0.12;
            op.push_box(
                [dx, cy, -cz_off],
                [0.38, dh, leaf_w],
                Tile::BlastDoor,
                4.2,
                white,
                0.0,
            );
            op.push_box(
                [dx, cy, cz_off],
                [0.38, dh, leaf_w],
                Tile::BlastDoor,
                4.2,
                white,
                0.0,
            );
        }
        // Diegetic pressure status beacons (cyan = pressurized, pulsing amber = cycling/busy, red = vacuum)
        let beacon_col = if al.cycling && (time * 6.0).sin() > 0.0 {
            sec
        } else if al.pressure > 0.85 {
            accent
        } else {
            red_strobe
        };
        for bx in [side * 22.8, side * 25.9, side * 29.0] {
            for bz in [-1.75f32, 1.75f32] {
                op.push_box(
                    [bx, 2.52, bz],
                    [0.24, 0.24, 0.32],
                    Tile::CeilingLight,
                    1.0,
                    beacon_col,
                    1.35,
                );
            }
        }
    }

    // 6C. Physical 3D Interior Tracked / 6-Wheel Camera Rover (`BOT-01`, visible on fixed interior CCTV feeds!)
    {
        let wp = state.walker.pos;
        let (sy, cy) = state.walker.yaw.sin_cos();
        let (sp, cp) = state.walker.pitch.sin_cos();
        let wf = [sy * cp, sp, cy * cp];
        let wr = [cy, 0.0, -sy];
        let wu = [-sy * sp, cp, -cy * sp];
        let rolling = !state.walker.riding_elevator && state.walker.dwell_timer <= 0.0;
        let suspension_bob = if rolling {
            (state.walker.walk_phase * 2.5).sin().abs() * 0.03
        } else {
            0.0
        };
        // 1. Twin armored side track bogies + 6 visible rugged wheel hubs (3 per side)
        for sx in [-0.58f32, 0.58f32] {
            op.push_oriented_box(
                wp,
                wr,
                wu,
                wf,
                [sx, 0.24, 0.0],
                [0.24, 0.42, 1.48],
                Tile::DarkSteelRib,
                dark_steel,
                0.0,
            );
            for sz in [-0.52f32, 0.0, 0.52f32] {
                op.push_oriented_box(
                    wp,
                    wr,
                    wu,
                    wf,
                    [sx * 1.12, 0.22, sz],
                    [0.10, 0.34, 0.34],
                    Tile::HullPanel,
                    theme.rib_tint,
                    0.0,
                );
            }
        }
        // 2. Low-slung armored reconnaissance rover hull with hazard stripe belt
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.0, 0.56 + suspension_bob, -0.02],
            [0.98, 0.52, 1.32],
            Tile::ShipArmor,
            theme.hull_tint,
            0.0,
        );
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.0, 0.44 + suspension_bob, 0.0],
            [1.02, 0.18, 1.36],
            Tile::HazardStripe,
            white,
            0.0,
        );
        // Twin forward rover headlights
        for sx in [-0.34f32, 0.34f32] {
            op.push_oriented_box(
                wp,
                wr,
                wu,
                wf,
                [sx, 0.62 + suspension_bob, 0.66],
                [0.18, 0.14, 0.08],
                Tile::CeilingLight,
                white,
                1.25,
            );
        }
        // 3. Telescoping vertical optical mast & rotating camera turret head + glowing red REC lens
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.0, 1.24 + suspension_bob, -0.06],
            [0.24, 0.88, 0.24],
            Tile::DarkSteelRib,
            dark_steel,
            0.0,
        );
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.0, 1.78 + suspension_bob, 0.04],
            [0.64, 0.46, 0.68],
            Tile::DarkSteelRib,
            theme.rib_tint,
            0.0,
        );
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.0, 1.78 + suspension_bob, 0.40],
            [0.34, 0.26, 0.10],
            Tile::CeilingLight,
            red_strobe,
            1.35,
        );
        // Rooftop cyan telemetry antenna
        op.push_oriented_box(
            wp,
            wr,
            wu,
            wf,
            [0.24, 2.18 + suspension_bob, -0.10],
            [0.08, 0.44, 0.08],
            Tile::CeilingLight,
            accent,
            1.1,
        );
    }

    // 6D. HERMES-01 Free-Flying Winged Camera Drone & Atrium Home Alcove Status Strip
    {
        let h = &state.hermes;
        // Dynamic status beacon strip on Hermes's Atrium home docking hood
        let dock_col = if h.docked {
            sec
        } else {
            let pulse = 0.6 + 0.4 * (time * 4.5).sin().abs();
            [accent[0] * pulse, accent[1] * pulse, accent[2] * pulse, 1.0]
        };
        op.push_box(
            [HERMES_HOME_POS[0], 8.38, -12.34],
            [2.4, 0.12, 0.12],
            Tile::CeilingLight,
            1.5,
            dock_col,
            1.25,
        );

        let (sy, cy) = h.yaw.sin_cos();
        let (sp, cp) = h.pitch.sin_cos();
        let hf = [sy * cp, sp, cy * cp];
        let hr = [cy, 0.0, -sy];
        let hu = [-sy * sp, cp, -cy * sp];
        let bob = if h.docked {
            0.0
        } else {
            (h.hover_phase).sin() * 0.07
        };
        let hp = [h.pos[0], h.pos[1] + bob, h.pos[2]];

        // 1. Sleek armored central camera-drone chassis
        op.push_oriented_box(
            hp,
            hr,
            hu,
            hf,
            [0.0, 0.0, 0.0],
            [0.72, 0.46, 0.92],
            Tile::ShipArmor,
            white,
            0.0,
        );
        // Amber/gold dorsal trim plate
        op.push_oriented_box(
            hp,
            hr,
            hu,
            hf,
            [0.0, 0.25, -0.04],
            [0.54, 0.10, 0.68],
            Tile::HazardStripe,
            sec,
            0.0,
        );
        // 2. Signature Hermes winged stabilizer fins (port & starboard)
        for side in [-1.0f32, 1.0f32] {
            op.push_oriented_box(
                hp,
                hr,
                hu,
                hf,
                [side * 0.64, 0.04, -0.10],
                [0.62, 0.10, 0.36],
                Tile::ShipArmor,
                sec,
                0.0,
            );
            op.push_oriented_box(
                hp,
                hr,
                hu,
                hf,
                [side * 0.96, 0.10, -0.22],
                [0.24, 0.08, 0.24],
                Tile::CeilingLight,
                accent,
                if h.docked { 0.45 } else { 1.1 },
            );
        }
        // 3. Quad corner ducted micro-thruster pods with dynamic hover-glow rings
        let thr_em = if h.docked {
            0.25
        } else {
            0.85 + h.thrust_level * 0.75
        };
        let thr_col = if h.docked { sec } else { accent };
        for (tx, tz) in [(-0.46, 0.34), (0.46, 0.34), (-0.46, -0.34), (0.46, -0.34)] {
            op.push_oriented_box(
                hp,
                hr,
                hu,
                hf,
                [tx, -0.10, tz],
                [0.26, 0.22, 0.26],
                Tile::DarkSteelRib,
                dark_steel,
                0.0,
            );
            op.push_oriented_box(
                hp,
                hr,
                hu,
                hf,
                [tx, -0.22, tz],
                [0.18, 0.07, 0.18],
                Tile::CeilingLight,
                thr_col,
                thr_em,
            );
        }
        // 4. Front gimballed camera optic housing & glowing REC lens
        op.push_oriented_box(
            hp,
            hr,
            hu,
            hf,
            [0.0, -0.02, 0.48],
            [0.44, 0.32, 0.18],
            Tile::DarkSteelRib,
            theme.rib_tint,
            0.0,
        );
        op.push_oriented_box(
            hp,
            hr,
            hu,
            hf,
            [0.0, -0.02, 0.59],
            [0.24, 0.20, 0.06],
            Tile::CeilingLight,
            if h.active { accent } else { red_strobe },
            1.35,
        );
        // 5. Dorsal whip telemetry antenna & status beacon
        op.push_oriented_box(
            hp,
            hr,
            hu,
            hf,
            [0.18, 0.44, -0.22],
            [0.06, 0.38, 0.06],
            Tile::CeilingLight,
            dock_col,
            1.15,
        );
    }
}
