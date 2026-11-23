//! Dynamic 3D mesh generation for Gothic street and highway vehicles.

use rookery_core::gothic::{GothicVehicle, GothicVehicleKind};

use super::mesh_builder::MeshBuilder;
use super::theme::GothicThemeColors;

pub(super) fn push_vehicle_mesh(
    opaque_mb: &mut MeshBuilder,
    trans_mb: &mut MeshBuilder,
    veh: &GothicVehicle,
    pos: [f32; 3],
    fwd: [f32; 2],
    time: f32,
    theme: &GothicThemeColors,
) {
    let vx = pos[0];
    let vy = pos[1];
    let vz = pos[2];
    let right = [-fwd[1], fwd[0]];

    let (car_len, car_w, car_h) = match veh.kind {
        GothicVehicleKind::NoirCoupe => (0.60, 0.22, 0.14),
        GothicVehicleKind::ArmoredCruiser => (0.64, 0.25, 0.18),
        GothicVehicleKind::TransitOmnibus => (0.88, 0.26, 0.24),
        GothicVehicleKind::VigilanteInterceptor => (0.72, 0.26, 0.13),
        GothicVehicleKind::SteamLimousine => (0.84, 0.23, 0.16),
    };

    let body_color = match veh.kind {
        GothicVehicleKind::VigilanteInterceptor => theme.plinth_dark,
        GothicVehicleKind::ArmoredCruiser => theme.stone_dark,
        GothicVehicleKind::TransitOmnibus => theme.stone_secondary,
        GothicVehicleKind::SteamLimousine => theme.plinth_dark,
        GothicVehicleKind::NoirCoupe => theme.plinth_dark,
    };

    // Main lower chassis
    opaque_mb.push_oriented_box(
        [vx, vy + 0.015, vz],
        fwd,
        car_len,
        car_h * 0.56,
        car_w,
        body_color,
        0.0,
    );

    // Four sculpted wheel arches / tires
    for &ax in &[-car_len * 0.30, car_len * 0.30] {
        for &side in &[-1.0f32, 1.0f32] {
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * ax + right[0] * (car_w * 0.48 * side),
                    vy,
                    vz + fwd[1] * ax + right[1] * (car_w * 0.48 * side),
                ],
                fwd,
                0.13,
                0.06,
                0.045,
                theme.stone_dark,
                0.0,
            );
        }
    }

    // Kind-specific upper silhouette & details
    match veh.kind {
        GothicVehicleKind::NoirCoupe => {
            // Swept-back art-deco cabin + chrome grille + twin rear tailfins
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.12),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.12),
                ],
                fwd,
                car_len * 0.44,
                car_h * 0.44,
                car_w * 0.82,
                theme.stone_secondary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.49),
                    vy + 0.03,
                    vz + fwd[1] * (car_len * 0.49),
                ],
                fwd,
                0.03,
                0.055,
                car_w * 0.42,
                theme.steel_viaduct,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.42) + right[0] * (car_w * 0.42 * side),
                        vy + car_h * 0.56,
                        vz - fwd[1] * (car_len * 0.42) + right[1] * (car_w * 0.42 * side),
                    ],
                    fwd,
                    0.14,
                    0.06,
                    0.028,
                    theme.accent_trim,
                    0.0,
                );
            }
        }
        GothicVehicleKind::ArmoredCruiser => {
            // Heavy ram-bumper + armored cabin + alternating flashing roof lightbar
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.51),
                    vy + 0.02,
                    vz + fwd[1] * (car_len * 0.51),
                ],
                fwd,
                0.04,
                0.05,
                car_w * 1.04,
                theme.steel_viaduct,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.06),
                ],
                fwd,
                car_len * 0.54,
                car_h * 0.46,
                car_w * 0.88,
                theme.stone_primary,
                0.0,
            );
            let flash = ((time * 9.0 + veh.id as f32 * 1.3).sin()) >= 0.0;
            let left_col = if flash {
                theme.beacon_red
            } else {
                theme.searchlight
            };
            let right_col = if flash {
                theme.searchlight
            } else {
                theme.beacon_red
            };
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06) - right[0] * (car_w * 0.20),
                    vy + car_h * 1.04,
                    vz - fwd[1] * (car_len * 0.06) - right[1] * (car_w * 0.20),
                ],
                fwd,
                0.06,
                0.038,
                car_w * 0.34,
                left_col,
                1.5,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06) + right[0] * (car_w * 0.20),
                    vy + car_h * 1.04,
                    vz - fwd[1] * (car_len * 0.06) + right[1] * (car_w * 0.20),
                ],
                fwd,
                0.06,
                0.038,
                car_w * 0.34,
                right_col,
                1.5,
            );
        }
        GothicVehicleKind::TransitOmnibus => {
            // Double-length upper cabin + glowing passenger window strip + roof clerestory + front marquee
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 0.56, vz],
                fwd,
                car_len * 0.90,
                car_h * 0.46,
                car_w * 0.92,
                theme.stone_primary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 0.68, vz],
                fwd,
                car_len * 0.78,
                0.055,
                car_w * 0.95,
                theme.window_warm,
                0.78,
            );
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 1.02, vz],
                fwd,
                car_len * 0.68,
                0.035,
                car_w * 0.52,
                theme.accent_trim,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.46),
                    vy + car_h * 0.92,
                    vz + fwd[1] * (car_len * 0.46),
                ],
                fwd,
                0.03,
                0.04,
                car_w * 0.58,
                theme.window_warm,
                1.1,
            );
        }
        GothicVehicleKind::VigilanteInterceptor => {
            // Turbine nose + low armored canopy + tall swept bat-fins + glowing afterburner
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.48),
                    vy + 0.03,
                    vz + fwd[1] * (car_len * 0.48),
                ],
                fwd,
                0.08,
                0.05,
                car_w * 0.34,
                theme.accent_trim,
                0.4,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.05),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.05),
                ],
                fwd,
                car_len * 0.40,
                car_h * 0.42,
                car_w * 0.68,
                theme.stone_dark,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.34) + right[0] * (car_w * 0.44 * side),
                        vy + car_h * 0.72,
                        vz - fwd[1] * (car_len * 0.34) + right[1] * (car_w * 0.44 * side),
                    ],
                    fwd,
                    0.26,
                    0.11,
                    0.03,
                    theme.plinth_dark,
                    0.0,
                );
            }
            if veh.afterburner > 0.05 {
                let flame_len = 0.14 + veh.afterburner * 0.34;
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.54 + flame_len * 0.4),
                        vy + 0.04,
                        vz - fwd[1] * (car_len * 0.54 + flame_len * 0.4),
                    ],
                    fwd,
                    flame_len,
                    0.055,
                    0.065,
                    theme.searchlight,
                    1.8,
                );
                trans_mb.push_light_cone(
                    [
                        vx - fwd[0] * (car_len * 0.52),
                        vy + 0.04,
                        vz - fwd[1] * (car_len * 0.52),
                    ],
                    [-fwd[0], 0.02, -fwd[1]],
                    0.45 + veh.afterburner * 0.55,
                    0.14,
                    theme.searchlight,
                );
            }
        }
        GothicVehicleKind::SteamLimousine => {
            // Extended formal cabin + brass carriage-lantern pips on B-pillars
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.08),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.08),
                ],
                fwd,
                car_len * 0.58,
                car_h * 0.46,
                car_w * 0.84,
                theme.stone_dark,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.10) + right[0] * (car_w * 0.46 * side),
                        vy + car_h * 0.64,
                        vz - fwd[1] * (car_len * 0.10) + right[1] * (car_w * 0.46 * side),
                    ],
                    fwd,
                    0.04,
                    0.045,
                    0.04,
                    theme.window_warm,
                    1.2,
                );
            }
        }
    }

    // Twin separated headlights
    for &side in &[-1.0f32, 1.0f32] {
        opaque_mb.push_oriented_box(
            [
                vx + fwd[0] * (car_len * 0.50) + right[0] * (car_w * 0.30 * side),
                vy + 0.032,
                vz + fwd[1] * (car_len * 0.50) + right[1] * (car_w * 0.30 * side),
            ],
            fwd,
            0.035,
            0.04,
            0.045,
            theme.window_warm,
            1.25,
        );
    }

    // Twin ruby taillights that flare brightly when braking
    let brake_emissive = if veh.is_braking { 1.75 } else { 0.85 };
    for &side in &[-1.0f32, 1.0f32] {
        opaque_mb.push_oriented_box(
            [
                vx - fwd[0] * (car_len * 0.50) + right[0] * (car_w * 0.30 * side),
                vy + 0.032,
                vz - fwd[1] * (car_len * 0.50) + right[1] * (car_w * 0.30 * side),
            ],
            fwd,
            0.035,
            0.04,
            0.05,
            theme.beacon_red,
            brake_emissive,
        );
    }

    // Blinking amber turn-signal pip before corners
    if veh.turn_signal != 0 && (time * 7.0).sin() > 0.0 {
        let side = if veh.turn_signal > 0 { 1.0f32 } else { -1.0f32 };
        opaque_mb.push_oriented_box(
            [
                vx + fwd[0] * (car_len * 0.44) + right[0] * (car_w * 0.44 * side),
                vy + 0.035,
                vz + fwd[1] * (car_len * 0.44) + right[1] * (car_w * 0.44 * side),
            ],
            fwd,
            0.04,
            0.035,
            0.035,
            theme.accent_trim,
            1.5,
        );
    }

    // Translucent headlight cone on the street asphalt ahead
    let cone_len = if veh.kind == GothicVehicleKind::VigilanteInterceptor {
        1.45
    } else {
        1.05
    };
    trans_mb.push_light_cone(
        [
            vx + fwd[0] * (car_len * 0.52),
            vy + 0.04,
            vz + fwd[1] * (car_len * 0.52),
        ],
        [fwd[0], -0.04, fwd[1]],
        cone_len,
        0.26,
        theme.searchlight,
    );
}
