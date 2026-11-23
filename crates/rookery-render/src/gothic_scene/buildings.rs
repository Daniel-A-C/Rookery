//! Procedural 3D mesh generation for all 11 Gothic Metropolis building archetypes.

use rookery_core::gothic::{BuildingArchetype, BuildingPlacement};

use super::mesh_builder::MeshBuilder;
use super::theme::GothicThemeColors;

/// Per-building masonry, trim, and roof color palette derived deterministically from
/// the active [`GothicThemeColors`] and the building's `id` / `style_variant`.
#[derive(Clone, Copy)]
struct BuildingMaterial {
    stone_main: [f32; 4],
    stone_alt: [f32; 4],
    stone_base: [f32; 4],
    trim: [f32; 4],
    roof: [f32; 4],
}

fn mix_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let u = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        a[3],
    ]
}

fn scale_color(c: [f32; 4], s: f32) -> [f32; 4] {
    [
        (c[0] * s).clamp(0.0, 1.0),
        (c[1] * s).clamp(0.0, 1.0),
        (c[2] * s).clamp(0.0, 1.0),
        c[3],
    ]
}

fn building_material(b: &BuildingPlacement, theme: &GothicThemeColors) -> BuildingMaterial {
    let finish = (b.id as u8).wrapping_add(b.style_variant.wrapping_mul(3)) % 4;
    match finish {
        // 0: Obsidian Basalt — deep charcoal stone with high-contrast brass/amber trim
        0 => BuildingMaterial {
            stone_main: scale_color(theme.stone_primary, 0.84),
            stone_alt: theme.stone_secondary,
            stone_base: scale_color(theme.stone_dark, 0.86),
            trim: scale_color(theme.accent_trim, 1.14),
            roof: mix_color(theme.copper_roof, theme.stone_dark, 0.28),
        },
        // 1: Weathered Gargoyle Limestone — lighter ash-stone with crisp dark mullions & verdigris roof
        1 => BuildingMaterial {
            stone_main: scale_color(theme.stone_secondary, 1.12),
            stone_alt: theme.stone_primary,
            stone_base: theme.stone_dark,
            trim: mix_color(theme.accent_trim, theme.sidewalk_curb, 0.35),
            roof: scale_color(theme.copper_roof, 1.18),
        },
        // 2: Bronzed Art-Deco Slate — warm bronze-infused masonry with gilded deco ledges
        2 => BuildingMaterial {
            stone_main: mix_color(theme.stone_primary, theme.accent_trim, 0.24),
            stone_alt: mix_color(theme.stone_secondary, theme.accent_trim, 0.32),
            stone_base: mix_color(theme.stone_dark, theme.accent_trim, 0.16),
            trim: mix_color(theme.accent_trim, theme.window_warm, 0.20),
            roof: mix_color(theme.copper_roof, theme.accent_trim, 0.35),
        },
        // 3: Soot-Stained Foundry Ironstone — gunmetal slate with oxidized copper upper works
        _ => BuildingMaterial {
            stone_main: mix_color(theme.stone_primary, theme.steel_viaduct, 0.45),
            stone_alt: scale_color(theme.stone_secondary, 0.94),
            stone_base: scale_color(theme.plinth_dark, 1.12),
            trim: theme.accent_trim,
            roof: scale_color(theme.copper_roof, 1.08),
        },
    }
}

/// Emit recessed gothic window strips along all 4 sides of a tower section.
#[allow(clippy::too_many_arguments)]
fn push_gothic_window_bands(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    cols: usize,
    rows: usize,
    seed_variant: u8,
    theme: &GothicThemeColors,
) {
    if h < 0.5 || cols == 0 || rows == 0 {
        return;
    }
    let pad_x = w * 0.16;
    let pad_z = d * 0.16;
    let usable_w = (w - pad_x * 2.0).max(0.2);
    let usable_d = (d - pad_z * 0.2 * 2.0 - pad_z * 1.6).max(0.2);
    let usable_h = (h * 0.78).max(0.3);
    let start_y = y0 + h * 0.11;

    let col_step_w = usable_w / cols as f32;
    let col_step_d = usable_d / cols as f32;
    let row_step = usable_h / rows as f32;
    let win_w = (col_step_w * 0.46).clamp(0.06, 0.22);
    let win_d = (col_step_d * 0.46).clamp(0.06, 0.22);
    let win_h = (row_step * 0.62).clamp(0.08, 0.34);

    for r in 0..rows {
        let wy = start_y + (r as f32 + 0.18) * row_step;
        for c in 0..cols {
            let hash = (c as u32)
                .wrapping_mul(37)
                .wrapping_add((r as u32).wrapping_mul(17))
                .wrapping_add((seed_variant as u32).wrapping_mul(13));
            // Leave ~25% of windows dark so facades look organic and lived-in
            if hash.is_multiple_of(4) {
                continue;
            }
            let win_color = if hash.is_multiple_of(3) {
                theme.window_dim
            } else {
                theme.window_warm
            };

            let wx = (cx - usable_w * 0.5) + (c as f32 + 0.5) * col_step_w;
            // Front (+Z) & Back (-Z) window panes
            mb.push_box(wx, wy, cz + d * 0.502, win_w, win_h, 0.03, win_color, 0.9);
            mb.push_box(wx, wy, cz - d * 0.502, win_w, win_h, 0.03, win_color, 0.9);

            let wz = (cz - usable_d * 0.5) + (c as f32 + 0.5) * col_step_d;
            // Right (+X) & Left (-X) window panes
            mb.push_box(cx + w * 0.502, wy, wz, 0.03, win_h, win_d, win_color, 0.9);
            mb.push_box(cx - w * 0.502, wy, wz, 0.03, win_h, win_d, win_color, 0.9);
        }
    }
}

/// Emit continuous multi-story Art-Deco vertical window channels with dark spandrel dividers
/// and projecting vertical stone mullions (inspired by 1990s Batman / Anton Furst skyscrapers).
#[allow(clippy::too_many_arguments)]
fn push_vertical_deco_light_strips(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    strips: usize,
    seed_variant: u8,
    mat: &BuildingMaterial,
    theme: &GothicThemeColors,
) {
    if h < 0.8 || strips == 0 {
        return;
    }
    let usable_w = (w * 0.68).max(0.24);
    let usable_d = (d * 0.68).max(0.24);
    let strip_h = (h * 0.82).max(0.4);
    let sy = y0 + h * 0.09;
    let step_w = usable_w / strips as f32;
    let step_d = usable_d / strips as f32;
    let chan_w = (step_w * 0.38).clamp(0.06, 0.16);
    let chan_d = (step_d * 0.38).clamp(0.06, 0.16);

    for i in 0..strips {
        let hash = (i as u32)
            .wrapping_mul(29)
            .wrapping_add((seed_variant as u32).wrapping_mul(19));
        let col = if hash.is_multiple_of(3) {
            theme.window_dim
        } else {
            theme.window_warm
        };

        let wx = (cx - usable_w * 0.5) + (i as f32 + 0.5) * step_w;
        let wz = (cz - usable_d * 0.5) + (i as f32 + 0.5) * step_d;

        // Vertical glowing light channel on all 4 faces
        mb.push_box(wx, sy, cz + d * 0.503, chan_w, strip_h, 0.03, col, 0.95);
        mb.push_box(wx, sy, cz - d * 0.503, chan_w, strip_h, 0.03, col, 0.95);
        mb.push_box(cx + w * 0.503, sy, wz, 0.03, strip_h, chan_d, col, 0.95);
        mb.push_box(cx - w * 0.503, sy, wz, 0.03, strip_h, chan_d, col, 0.95);

        // Flanking vertical Art-Deco stone flute rib next to each light channel
        let rib_off_x = step_w * 0.44;
        let rib_off_z = step_d * 0.44;
        mb.push_box(
            wx + rib_off_x,
            y0,
            cz + d * 0.508,
            0.06,
            h - 0.10,
            0.05,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            wx - rib_off_x,
            y0,
            cz - d * 0.508,
            0.06,
            h - 0.10,
            0.05,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            cx + w * 0.508,
            y0,
            wz + rib_off_z,
            0.05,
            h - 0.10,
            0.06,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            cx - w * 0.508,
            y0,
            wz - rib_off_z,
            0.05,
            h - 0.10,
            0.06,
            mat.stone_alt,
            0.0,
        );
    }

    // Horizontal dark bronze spandrel bars interrupting the vertical light strips every ~1.1m
    let spandrels = (strip_h / 1.05).floor().max(1.0) as usize;
    for s in 1..spandrels {
        let by = sy + (s as f32 / spandrels as f32) * strip_h;
        mb.push_box(cx, by, cz, w + 0.05, 0.07, d + 0.05, mat.stone_base, 0.0);
    }
}

/// Emit tall double-height pointed-arch gothic lancet windows with glowing tracery heads.
#[allow(clippy::too_many_arguments)]
fn push_gothic_lancet_windows(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    cols: usize,
    tiers: usize,
    seed_variant: u8,
    theme: &GothicThemeColors,
) {
    if h < 0.8 || cols == 0 || tiers == 0 {
        return;
    }
    let usable_w = (w * 0.68).max(0.24);
    let usable_d = (d * 0.68).max(0.24);
    let usable_h = (h * 0.80).max(0.4);
    let start_y = y0 + h * 0.10;
    let step_w = usable_w / cols as f32;
    let step_d = usable_d / cols as f32;
    let tier_h = usable_h / tiers as f32;
    let win_w = (step_w * 0.44).clamp(0.08, 0.22);
    let win_d = (step_d * 0.44).clamp(0.08, 0.22);
    let body_h = (tier_h * 0.60).clamp(0.18, 0.75);
    let arch_h = (tier_h * 0.18).clamp(0.06, 0.22);

    for t in 0..tiers {
        let wy = start_y + (t as f32 + 0.10) * tier_h;
        for c in 0..cols {
            let hash = (c as u32)
                .wrapping_mul(41)
                .wrapping_add((t as u32).wrapping_mul(23))
                .wrapping_add((seed_variant as u32).wrapping_mul(11));
            let col = if hash.is_multiple_of(3) {
                theme.window_dim
            } else {
                theme.window_warm
            };

            let wx = (cx - usable_w * 0.5) + (c as f32 + 0.5) * step_w;
            let wz = (cz - usable_d * 0.5) + (c as f32 + 0.5) * step_d;

            // Main lancet shaft + narrower pointed arch head on +Z / -Z
            mb.push_box(wx, wy, cz + d * 0.503, win_w, body_h, 0.03, col, 0.95);
            mb.push_box(
                wx,
                wy + body_h,
                cz + d * 0.503,
                win_w * 0.56,
                arch_h,
                0.03,
                theme.window_warm,
                1.0,
            );
            mb.push_box(wx, wy, cz - d * 0.503, win_w, body_h, 0.03, col, 0.95);
            mb.push_box(
                wx,
                wy + body_h,
                cz - d * 0.503,
                win_w * 0.56,
                arch_h,
                0.03,
                theme.window_warm,
                1.0,
            );

            // Main lancet shaft + narrower pointed arch head on +X / -X
            mb.push_box(cx + w * 0.503, wy, wz, 0.03, body_h, win_d, col, 0.95);
            mb.push_box(
                cx + w * 0.503,
                wy + body_h,
                wz,
                0.03,
                arch_h,
                win_d * 0.56,
                theme.window_warm,
                1.0,
            );
            mb.push_box(cx - w * 0.503, wy, wz, 0.03, body_h, win_d, col, 0.95);
            mb.push_box(
                cx - w * 0.503,
                wy + body_h,
                wz,
                0.03,
                arch_h,
                win_d * 0.56,
                theme.window_warm,
                1.0,
            );
        }
    }
}

/// Emit a grand illuminated street-level gothic entrance colonnade / vaulted narthex at the base of a tower.
fn push_street_lobby_colonnade(
    mb: &mut MeshBuilder,
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    mat: &BuildingMaterial,
    theme: &GothicThemeColors,
) {
    if w < 1.3 || d < 1.3 {
        return;
    }
    let lobby_h = 0.68f32;
    let portal_w = (w * 0.36).clamp(0.32, 0.72);
    let portal_d = (d * 0.36).clamp(0.32, 0.72);

    // Street-level projecting stone plinth belt & heavy ground-floor cornice
    mb.push_box(cx, 0.0, cz, w + 0.10, 0.14, d + 0.10, mat.stone_base, 0.0);
    mb.push_box(cx, lobby_h, cz, w + 0.14, 0.08, d + 0.14, mat.trim, 0.0);

    // Warm recessed entrance lobby portals + flanking pilasters + projecting Art-Deco marquee canopies
    for sign in [-1.0f32, 1.0f32] {
        // Z-facing portals
        let pz = cz + sign * (d * 0.504);
        mb.push_box(
            cx,
            0.08,
            pz,
            portal_w,
            lobby_h - 0.12,
            0.04,
            theme.window_warm,
            0.95,
        );
        for px_off in [-portal_w * 0.54, portal_w * 0.54] {
            mb.push_box(
                cx + px_off,
                0.08,
                pz + sign * 0.02,
                0.07,
                lobby_h - 0.08,
                0.07,
                mat.stone_alt,
                0.0,
            );
        }
        // Projecting Art-Deco canopy awning with glowing underside rim
        mb.push_box(
            cx,
            lobby_h - 0.09,
            cz + sign * (d * 0.5 + 0.09),
            portal_w + 0.14,
            0.06,
            0.18,
            mat.trim,
            0.0,
        );
        mb.push_box(
            cx,
            lobby_h - 0.11,
            cz + sign * (d * 0.5 + 0.09),
            portal_w + 0.08,
            0.02,
            0.14,
            theme.window_warm,
            0.85,
        );

        // X-facing portals
        let px = cx + sign * (w * 0.504);
        mb.push_box(
            px,
            0.08,
            cz,
            0.04,
            lobby_h - 0.12,
            portal_d,
            theme.window_warm,
            0.95,
        );
        for pz_off in [-portal_d * 0.54, portal_d * 0.54] {
            mb.push_box(
                px + sign * 0.02,
                0.08,
                cz + pz_off,
                0.07,
                lobby_h - 0.08,
                0.07,
                mat.stone_alt,
                0.0,
            );
        }
        mb.push_box(
            cx + sign * (w * 0.5 + 0.09),
            lobby_h - 0.09,
            cz,
            0.18,
            0.06,
            portal_d + 0.14,
            mat.trim,
            0.0,
        );
        mb.push_box(
            cx + sign * (w * 0.5 + 0.09),
            lobby_h - 0.11,
            cz,
            0.14,
            0.02,
            portal_d + 0.08,
            theme.window_warm,
            0.85,
        );
    }

    // Flanking ground-floor shopfront vitrines on wider facades
    if w >= 2.2 {
        for sx in [-1.0f32, 1.0f32] {
            let vx = cx + sx * (w * 0.34);
            for sz in [-1.0f32, 1.0f32] {
                mb.push_box(
                    vx,
                    0.16,
                    cz + sz * (d * 0.502),
                    0.22,
                    0.34,
                    0.03,
                    theme.window_dim,
                    0.78,
                );
            }
        }
    }
}

/// Emit a classic 1990s Gotham wooden-stave rooftop water tower on iron stilts.
fn push_rooftop_water_tower(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    scale: f32,
    mat: &BuildingMaterial,
) {
    let leg_h = 0.28 * scale;
    let leg_off = 0.11 * scale;
    for sx in [-1.0f32, 1.0f32] {
        for sz in [-1.0f32, 1.0f32] {
            mb.push_box(
                cx + sx * leg_off,
                y0,
                cz + sz * leg_off,
                0.04 * scale,
                leg_h,
                0.04 * scale,
                mat.stone_base,
                0.0,
            );
        }
    }
    // Cross-bracing platform + cylindrical-ish octagonal tank + iron hoops + conical roof
    mb.push_box(
        cx,
        y0 + leg_h,
        cz,
        0.32 * scale,
        0.04 * scale,
        0.32 * scale,
        mat.trim,
        0.0,
    );
    mb.push_box(
        cx,
        y0 + leg_h + 0.04 * scale,
        cz,
        0.26 * scale,
        0.32 * scale,
        0.26 * scale,
        mat.stone_alt,
        0.0,
    );
    mb.push_box(
        cx,
        y0 + leg_h + 0.14 * scale,
        cz,
        0.28 * scale,
        0.03 * scale,
        0.28 * scale,
        mat.stone_base,
        0.0,
    );
    mb.push_box(
        cx,
        y0 + leg_h + 0.26 * scale,
        cz,
        0.28 * scale,
        0.03 * scale,
        0.28 * scale,
        mat.stone_base,
        0.0,
    );
    mb.push_pyramid(
        cx,
        y0 + leg_h + 0.36 * scale,
        cz,
        0.32 * scale,
        0.16 * scale,
        0.32 * scale,
        mat.roof,
        0.0,
    );
}

/// Emit architectural corner pilasters and overhang cornice ledges on a building tier.
#[allow(clippy::too_many_arguments)]
fn push_gothic_ribs_and_cornice(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    rib_color: [f32; 4],
    cornice_color: [f32; 4],
) {
    let rib_t = 0.14;
    for sx in [-1.0f32, 1.0f32] {
        for sz in [-1.0f32, 1.0f32] {
            mb.push_box(
                cx + sx * (w * 0.5 - rib_t * 0.35),
                y0,
                cz + sz * (d * 0.5 - rib_t * 0.35),
                rib_t,
                h,
                rib_t,
                rib_color,
                0.0,
            );
        }
    }
    // Overhanging gothic cornice ledge at top of tier
    mb.push_box(
        cx,
        y0 + h - 0.12,
        cz,
        w + 0.22,
        0.14,
        d + 0.22,
        cornice_color,
        0.0,
    );
}

pub(super) fn build_archetype_mesh(
    mb: &mut MeshBuilder,
    b: &BuildingPlacement,
    theme: &GothicThemeColors,
) {
    let cx = b.x;
    let cz = b.z;
    let w = b.width;
    let d = b.depth;
    let h = b.height;
    let mat = building_material(b, theme);

    match b.archetype {
        BuildingArchetype::CathedralKeep => {
            // 1. Heavy buttressed podium base + grand street narthex colonnade
            let h_base = h * 0.34;
            mb.push_box(cx, 0.0, cz, w, h_base, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h_base, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.75,
                cz,
                w,
                h_base - 0.9,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // 2. Flying buttress fins flanking the soaring mid nave
            let h_nave = h * 0.42;
            let nave_w = w * 0.72;
            let nave_d = d * 0.72;
            mb.push_box(cx, h_base, cz, nave_w, h_nave, nave_d, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                h_base,
                cz,
                nave_w,
                h_nave,
                nave_d,
                mat.stone_alt,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                h_base + 0.15,
                cz,
                nave_w,
                h_nave - 0.3,
                nave_d,
                3,
                3,
                b.style_variant + 1,
                theme,
            );

            let rotated = !b.quarter_turns.is_multiple_of(2);
            for side in [-1.0f32, 1.0f32] {
                for offset in [-0.55f32, 0.55f32] {
                    let (px, pz, sx_pier, sz_pier, sx_strut, sz_strut, ax_x, ax_z) = if !rotated {
                        (
                            cx + side * (w * 0.46),
                            cz + offset * d * 0.38,
                            0.20,
                            0.24,
                            cx + side * (w * 0.38),
                            cz + offset * d * 0.38,
                            w * 0.22,
                            0.16,
                        )
                    } else {
                        (
                            cx + offset * w * 0.38,
                            cz + side * (d * 0.46),
                            0.24,
                            0.20,
                            cx + offset * w * 0.38,
                            cz + side * (d * 0.38),
                            0.16,
                            d * 0.22,
                        )
                    };
                    // Outer buttress pier + double-tier flying arch struts
                    mb.push_box(
                        px,
                        h_base,
                        pz,
                        sx_pier,
                        h_nave * 0.78,
                        sz_pier,
                        mat.stone_alt,
                        0.0,
                    );
                    for arch_frac in [0.36f32, 0.66f32] {
                        mb.push_box(
                            sx_strut,
                            h_base + h_nave * arch_frac,
                            sz_strut,
                            ax_x,
                            0.14,
                            ax_z,
                            mat.trim,
                            0.0,
                        );
                    }
                    // Pinnacle tip on outer pier
                    mb.push_pyramid(
                        px,
                        h_base + h_nave * 0.78,
                        pz,
                        0.24,
                        0.68,
                        0.24,
                        mat.roof,
                        0.0,
                    );
                }
            }

            // 3. Twin Gothic Belfry Towers + Rose Window Facade
            let y_top = h_base + h_nave;
            let tower_h = h * 0.24;
            for s in [-1.0f32, 1.0f32] {
                let (tx, tz, tw_x, tw_z) = if !rotated {
                    (cx + s * (nave_w * 0.34), cz, nave_w * 0.36, nave_d * 0.82)
                } else {
                    (cx, cz + s * (nave_d * 0.34), nave_w * 0.82, nave_d * 0.36)
                };
                mb.push_box(tx, y_top, tz, tw_x, tower_h, tw_z, mat.stone_alt, 0.0);
                push_gothic_lancet_windows(
                    mb,
                    tx,
                    y_top + 0.1,
                    tz,
                    tw_x,
                    tower_h - 0.2,
                    tw_z,
                    1,
                    2,
                    b.style_variant + 2,
                    theme,
                );
                mb.push_pyramid(
                    tx,
                    y_top + tower_h,
                    tz,
                    tw_x * 1.08,
                    h * 0.22,
                    tw_z * 1.08,
                    mat.roof,
                    0.0,
                );
            }

            // Glowing Rose Window on front & back of nave
            let rose_y = h_base + h_nave * 0.62;
            if !rotated {
                mb.push_box(
                    cx,
                    rose_y,
                    cz + nave_d * 0.51,
                    0.56,
                    0.56,
                    0.04,
                    theme.window_warm,
                    1.0,
                );
                mb.push_box(
                    cx,
                    rose_y,
                    cz - nave_d * 0.51,
                    0.56,
                    0.56,
                    0.04,
                    theme.window_warm,
                    1.0,
                );
            } else {
                mb.push_box(
                    cx + nave_w * 0.51,
                    rose_y,
                    cz,
                    0.04,
                    0.56,
                    0.56,
                    theme.window_warm,
                    1.0,
                );
                mb.push_box(
                    cx - nave_w * 0.51,
                    rose_y,
                    cz,
                    0.04,
                    0.56,
                    0.56,
                    theme.window_warm,
                    1.0,
                );
            }

            // 4. Soaring Central Needle Spire + Red Warning Beacon
            let spire_base = y_top;
            let spire_h = h * 0.36;
            mb.push_pyramid(
                cx,
                spire_base,
                cz,
                nave_w * 0.40,
                spire_h,
                nave_d * 0.40,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                spire_base + spire_h,
                cz,
                0.12,
                0.15,
                0.12,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ArtDecoZiggurat => {
            // 4-tier stepped Furst/Art-Deco skyscraper with vertical light strips, setback floodlights, and eagle ledges
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            let tiers = [
                (0.0, 0.42, 1.00, 4, 4),
                (0.42, 0.28, 0.78, 3, 4),
                (0.70, 0.18, 0.56, 2, 3),
                (0.88, 0.12, 0.36, 1, 2),
            ];
            for (idx, &(y_frac, h_frac, scale, cols, rows)) in tiers.iter().enumerate() {
                let ty = y_frac * h;
                let th = h_frac * h;
                let tw = w * scale;
                let td = d * scale;
                let col = if idx % 2 == 0 {
                    mat.stone_main
                } else {
                    mat.stone_alt
                };
                mb.push_box(cx, ty, cz, tw, th, td, col, 0.0);
                push_gothic_ribs_and_cornice(mb, cx, ty, cz, tw, th, td, mat.stone_base, mat.trim);
                if idx == 1 || idx == 2 {
                    push_vertical_deco_light_strips(
                        mb,
                        cx,
                        ty,
                        cz,
                        tw,
                        th,
                        td,
                        cols,
                        b.style_variant.wrapping_add(idx as u8),
                        &mat,
                        theme,
                    );
                } else {
                    push_gothic_window_bands(
                        mb,
                        cx,
                        ty,
                        cz,
                        tw,
                        th,
                        td,
                        cols,
                        rows,
                        b.style_variant.wrapping_add(idx as u8),
                        theme,
                    );
                }

                // Upward-glowing Art-Deco setback terrace floodlight collar at each step
                if idx > 0 {
                    mb.push_box(
                        cx,
                        ty + 0.02,
                        cz,
                        tw + 0.12,
                        0.08,
                        td + 0.12,
                        theme.window_warm,
                        0.9,
                    );
                }

                // Jutting gargoyle/eagle ledges on the 4 corners of tiers 0, 1, and 2
                if idx <= 2 {
                    let gy = ty + th - 0.08;
                    for sx in [-1.0f32, 1.0f32] {
                        for sz in [-1.0f32, 1.0f32] {
                            mb.push_box(
                                cx + sx * (tw * 0.56),
                                gy,
                                cz + sz * (td * 0.56),
                                0.28,
                                0.12,
                                0.28,
                                mat.trim,
                                0.0,
                            );
                        }
                    }
                }
            }
            // Crown needle spire + glowing lantern
            mb.push_box(cx, h, cz, w * 0.24, 0.36, d * 0.24, theme.window_warm, 1.0);
            mb.push_pyramid(
                cx,
                h + 0.36,
                cz,
                w * 0.28,
                h * 0.22,
                d * 0.28,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                h + 0.36 + h * 0.22,
                cz,
                0.11,
                0.14,
                0.11,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ClockworkBelfry => {
            // Slender campanile shaft with vertical deco lancet channels
            let shaft_h = h * 0.74;
            mb.push_box(
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                mat.stone_main,
                0.0,
            );
            push_street_lobby_colonnade(mb, cx, cz, w * 0.86, d * 0.86, &mat, theme);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                mat.stone_base,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                0.7,
                cz,
                w * 0.86,
                shaft_h - 0.9,
                d * 0.86,
                2,
                4,
                b.style_variant,
                theme,
            );

            // Flared Clock Chamber at top of shaft
            let clock_h = h * 0.18;
            let cw = w * 0.96;
            let cd = d * 0.96;
            mb.push_box(cx, shaft_h, cz, cw, clock_h, cd, mat.stone_alt, 0.0);
            mb.push_box(
                cx,
                shaft_h + clock_h - 0.10,
                cz,
                cw + 0.20,
                0.12,
                cd + 0.20,
                mat.trim,
                0.0,
            );

            // Glowing Gothic Clock Faces on all 4 sides + dark clock hands
            let face_size = (cw * 0.48).min(clock_h * 0.64);
            let fy = shaft_h + (clock_h - face_size) * 0.5;
            let f_mid_y = fy + face_size * 0.48;
            // +Z and -Z clock faces + hands
            mb.push_box(
                cx,
                fy,
                cz + cd * 0.505,
                face_size,
                face_size,
                0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_box(
                cx,
                f_mid_y,
                cz + cd * 0.525,
                0.06,
                face_size * 0.42,
                0.04,
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz + cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                mat.stone_base,
                0.0,
            );

            mb.push_box(
                cx,
                fy,
                cz - cd * 0.505,
                face_size,
                face_size,
                0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_box(
                cx,
                f_mid_y,
                cz - cd * 0.525,
                0.06,
                face_size * 0.42,
                0.04,
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz - cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                mat.stone_base,
                0.0,
            );

            // +X and -X clock faces + hands
            mb.push_box(
                cx + cw * 0.505,
                fy,
                cz,
                0.04,
                face_size,
                face_size,
                theme.window_warm,
                1.0,
            );
            mb.push_box(
                cx + cw * 0.525,
                f_mid_y,
                cz,
                0.04,
                face_size * 0.42,
                0.06,
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                mat.stone_base,
                0.0,
            );

            mb.push_box(
                cx - cw * 0.505,
                fy,
                cz,
                0.04,
                face_size,
                face_size,
                theme.window_warm,
                1.0,
            );
            mb.push_box(
                cx - cw * 0.525,
                f_mid_y,
                cz,
                0.04,
                face_size * 0.42,
                0.06,
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx - cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                mat.stone_base,
                0.0,
            );

            // Steep belfry spire roof + tall needle finial + corner pinnacles
            let roof_y = shaft_h + clock_h;
            mb.push_pyramid(
                cx,
                roof_y,
                cz,
                cw * 0.90,
                h * 0.32,
                cd * 0.90,
                mat.roof,
                0.0,
            );
            mb.push_pyramid(
                cx,
                roof_y + h * 0.28,
                cz,
                cw * 0.18,
                h * 0.16,
                cd * 0.18,
                mat.trim,
                0.0,
            );
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    mb.push_pyramid(
                        cx + sx * (cw * 0.42),
                        roof_y,
                        cz + sz * (cd * 0.42),
                        0.20,
                        0.52,
                        0.20,
                        mat.trim,
                        0.0,
                    );
                }
            }
        }

        BuildingArchetype::IndustrialFoundry => {
            // Heavy sloped masonry foundry base
            let base_h = h * 0.62;
            mb.push_frustum(
                cx,
                0.0,
                cz,
                w * 1.04,
                d * 1.04,
                w * 0.90,
                d * 0.90,
                base_h,
                mat.stone_base,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.25,
                cz,
                w * 0.94,
                base_h * 0.85,
                d * 0.94,
                3,
                3,
                b.style_variant,
                theme,
            );

            // Upper clerestory furnace hall
            let hall_h = h * 0.24;
            mb.push_box(
                cx,
                base_h,
                cz,
                w * 0.72,
                hall_h,
                d * 0.72,
                mat.stone_main,
                0.0,
            );
            mb.push_box(
                cx,
                base_h + hall_h * 0.35,
                cz,
                w * 0.75,
                0.18,
                d * 0.75,
                theme.window_warm,
                1.0,
            );

            // Twin tall industrial smokestacks rising from the roof
            let stack_y = base_h + hall_h;
            let stack_h = h * 0.36;
            for offset in [-0.24f32, 0.24f32] {
                let sx = cx + offset * w;
                let sz = cz + offset * d * 0.6;
                mb.push_frustum(
                    sx,
                    stack_y,
                    sz,
                    0.34,
                    0.34,
                    0.22,
                    0.22,
                    stack_h,
                    theme.steel_viaduct,
                    0.0,
                );
                // Glowing ember rim at top of smokestack
                mb.push_box(
                    sx,
                    stack_y + stack_h - 0.06,
                    sz,
                    0.25,
                    0.08,
                    0.25,
                    theme.window_dim,
                    0.85,
                );
            }
        }

        BuildingArchetype::SyndicateTower => {
            // Brutalist-gothic corporate citadel with style_variant crown differentiation
            let h1 = h * 0.58;
            let h2 = h * 0.30;
            let h3 = h * 0.12;
            mb.push_box(cx, 0.0, cz, w, h1, d, mat.stone_main, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h1, d, mat.stone_base, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                0.75,
                cz,
                w,
                h1 - 0.90,
                d,
                3,
                b.style_variant,
                &mat,
                theme,
            );

            mb.push_box(cx, h1, cz, w * 0.82, h2, d * 0.82, mat.stone_alt, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                h1,
                cz,
                w * 0.82,
                h2,
                d * 0.82,
                mat.stone_base,
                mat.trim,
            );
            push_gothic_window_bands(
                mb,
                cx,
                h1,
                cz,
                w * 0.82,
                h2,
                d * 0.82,
                3,
                3,
                b.style_variant + 2,
                theme,
            );

            mb.push_box(cx, h1 + h2, cz, w * 0.56, h3, d * 0.56, mat.stone_base, 0.0);
            // Rooftop water tower perched on the h1 setback terrace + mechanical condenser
            push_rooftop_water_tower(mb, cx + w * 0.38, h1, cz - d * 0.36, 0.95, &mat);
            mb.push_box(
                cx - w * 0.36,
                h1,
                cz + d * 0.36,
                0.24,
                0.18,
                0.24,
                mat.stone_base,
                0.0,
            );

            if b.style_variant.is_multiple_of(2) {
                // Variant A: Brutalist twin-turret corporate crown + tall radio spire
                for s in [-1.0f32, 1.0f32] {
                    mb.push_box(
                        cx + s * (w * 0.16),
                        h,
                        cz,
                        w * 0.20,
                        h * 0.12,
                        d * 0.42,
                        mat.stone_alt,
                        0.0,
                    );
                    mb.push_pyramid(
                        cx + s * (w * 0.16),
                        h + h * 0.12,
                        cz,
                        w * 0.22,
                        h * 0.14,
                        d * 0.44,
                        mat.roof,
                        0.0,
                    );
                }
            } else {
                // Variant B: Steep chateau mansard frustum roof + glowing attic lantern
                mb.push_frustum(
                    cx,
                    h,
                    cz,
                    w * 0.58,
                    d * 0.58,
                    w * 0.26,
                    d * 0.26,
                    h * 0.16,
                    mat.roof,
                    0.0,
                );
                mb.push_box(
                    cx,
                    h + h * 0.16,
                    cz,
                    w * 0.28,
                    0.12,
                    d * 0.28,
                    theme.window_warm,
                    1.0,
                );
                mb.push_pyramid(
                    cx,
                    h + h * 0.16 + 0.12,
                    cz,
                    w * 0.26,
                    h * 0.12,
                    d * 0.26,
                    mat.trim,
                    0.0,
                );
            }
        }

        BuildingArchetype::ViaductStation => {
            // Grand Gothic Transit Station straddling the elevated train track at y = 2.6!
            // Build West and East flanking pier walls so the center (x = cx) has an open portal
            // tunnel from y = 2.4 to y = 3.85!
            let pier_w = w * 0.28;
            let portal_top_y = 3.85;
            // Lower solid station base below track level (0.0 .. 2.55)
            mb.push_box(cx, 0.0, cz, w, 2.55, d, mat.stone_base, 0.0);
            push_gothic_window_bands(mb, cx, 0.2, cz, w, 2.2, d, 3, 2, b.style_variant, theme);

            // Left & Right vaulted train shed walls (y = 2.55 .. 3.85) leaving center open for the train!
            for side in [-1.0f32, 1.0f32] {
                let px = cx + side * (w * 0.5 - pier_w * 0.5);
                mb.push_box(
                    px,
                    2.55,
                    cz,
                    pier_w,
                    portal_top_y - 2.55,
                    d,
                    mat.stone_alt,
                    0.0,
                );
            }
            // Glowing amber station platform lamps inside the train shed
            for z_off in [-0.9f32, 0.0, 0.9f32] {
                mb.push_box(
                    cx - w * 0.20,
                    2.62,
                    cz + z_off,
                    0.10,
                    0.26,
                    0.16,
                    theme.window_warm,
                    1.0,
                );
                mb.push_box(
                    cx + w * 0.20,
                    2.62,
                    cz + z_off,
                    0.10,
                    0.26,
                    0.16,
                    theme.window_warm,
                    1.0,
                );
            }

            // Over-track station concourse & gothic clock tower (y = 3.85 .. h)
            let upper_h = (h - portal_top_y).max(1.8);
            mb.push_box(
                cx,
                portal_top_y,
                cz,
                w * 0.92,
                upper_h * 0.65,
                d * 0.86,
                mat.stone_main,
                0.0,
            );
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                portal_top_y,
                cz,
                w * 0.92,
                upper_h * 0.65,
                d * 0.86,
                mat.stone_alt,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                portal_top_y + 0.15,
                cz,
                w * 0.92,
                upper_h * 0.65 - 0.3,
                d * 0.86,
                3,
                2,
                b.style_variant,
                theme,
            );

            let top_y = portal_top_y + upper_h * 0.65;
            mb.push_box(
                cx,
                top_y,
                cz,
                w * 0.56,
                upper_h * 0.35,
                d * 0.56,
                mat.stone_alt,
                0.0,
            );
            mb.push_pyramid(cx, h, cz, w * 0.60, upper_h * 0.42, d * 0.60, mat.roof, 0.0);
        }

        BuildingArchetype::OctagonalCitadel => {
            // Eight-sided Gothic Rotunda Keep built from overlapping 0° and 45° rotated drums,
            // breaking up the strict 90-degree rectangular silhouette of standard city blocks!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];
            let r_base = w.min(d) * 0.86;
            let h1 = h * 0.38;
            let h2 = h * 0.34;
            let h3 = h * 0.28;

            // Tier 1: Heavy Octagonal Bastion Podium + 45° Diagonal Buttress Fins
            mb.push_box(cx, 0.0, cz, r_base, h1, r_base, mat.stone_base, 0.0);
            mb.push_oriented_box(
                [cx, 0.0, cz],
                diag,
                r_base,
                h1 - 0.04,
                r_base,
                mat.stone_main,
                0.0,
            );
            mb.push_box(
                cx,
                h1 - 0.10,
                cz,
                r_base + 0.18,
                0.12,
                r_base + 0.18,
                mat.trim,
                0.0,
            );
            mb.push_oriented_box(
                [cx, h1 - 0.10, cz],
                diag,
                r_base + 0.18,
                0.12,
                r_base + 0.18,
                mat.trim,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.22,
                cz,
                r_base,
                h1 - 0.35,
                r_base,
                3,
                3,
                b.style_variant,
                theme,
            );

            // Tier 2: Stepped Octagonal Rotunda Drum + Radial Flying Buttresses
            let r_mid = r_base * 0.74;
            mb.push_box(cx, h1, cz, r_mid, h2, r_mid, mat.stone_main, 0.0);
            mb.push_oriented_box(
                [cx, h1, cz],
                diag,
                r_mid,
                h2 - 0.03,
                r_mid,
                mat.stone_alt,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                h1 + 0.15,
                cz,
                r_mid,
                h2 - 0.30,
                r_mid,
                2,
                3,
                b.style_variant + 1,
                theme,
            );
            // 45-degree diagonal buttress ribs & corner turrets on the setback terrace
            for sign_u in [-1.0f32, 1.0f32] {
                for sign_v in [-1.0f32, 1.0f32] {
                    let bx = cx + sign_u * (r_base * 0.42);
                    let bz = cz + sign_v * (r_base * 0.42);
                    mb.push_oriented_box(
                        [bx, h1, bz],
                        diag,
                        0.24,
                        h2 * 0.65,
                        0.24,
                        mat.stone_alt,
                        0.0,
                    );
                    mb.push_pyramid(bx, h1 + h2 * 0.65, bz, 0.26, 0.45, 0.26, mat.roof, 0.0);
                }
            }

            // Tier 3: Upper Octagonal Lantern Keep & Pointed Spire
            let y2 = h1 + h2;
            let r_top = r_base * 0.50;
            mb.push_box(cx, y2, cz, r_top, h3, r_top, mat.stone_alt, 0.0);
            mb.push_oriented_box(
                [cx, y2, cz],
                diag,
                r_top,
                h3 - 0.02,
                r_top,
                mat.stone_main,
                0.0,
            );
            // Glowing clerestory lantern band around all 8 faces
            mb.push_box(
                cx,
                y2 + h3 * 0.45,
                cz,
                r_top + 0.04,
                0.20,
                r_top + 0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_oriented_box(
                [cx, y2 + h3 * 0.45, cz],
                diag,
                r_top + 0.04,
                0.20,
                r_top + 0.04,
                theme.window_warm,
                1.0,
            );
            // Octagonal spire crown + ruby beacon
            mb.push_pyramid(
                cx,
                h,
                cz,
                r_top * 1.04,
                h * 0.28,
                r_top * 1.04,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                h + h * 0.28,
                cz,
                0.11,
                0.12,
                0.11,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::GothicMonumentPlaza => {
            // Open Old-World Gothic Civic Piazza with a stepped octagonal monument dais,
            // central obelisk spire, glowing corner colonnade pavilions, and warm braziers.
            // Leaves dramatic open sightlines across the middle of the 4x4 city!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];

            // 1. Stepped Octagonal Courtyard Dais in the center of the piazza
            let dais_w = w.min(d) * 0.76;
            mb.push_box(cx, 0.08, cz, dais_w, 0.14, dais_w, mat.stone_alt, 0.0);
            mb.push_oriented_box(
                [cx, 0.08, cz],
                diag,
                dais_w,
                0.13,
                dais_w,
                mat.stone_main,
                0.0,
            );
            mb.push_box(
                cx,
                0.22,
                cz,
                dais_w * 0.64,
                0.18,
                dais_w * 0.64,
                mat.trim,
                0.0,
            );
            mb.push_oriented_box(
                [cx, 0.22, cz],
                diag,
                dais_w * 0.64,
                0.17,
                dais_w * 0.64,
                mat.trim,
                0.0,
            );

            // 2. Illuminated Fountain Basin / Sanctuary Pedestal
            let ped_w = dais_w * 0.38;
            mb.push_box(cx, 0.40, cz, ped_w, 0.55, ped_w, mat.stone_base, 0.0);
            mb.push_box(
                cx,
                0.56,
                cz,
                ped_w + 0.08,
                0.14,
                ped_w + 0.08,
                theme.window_warm,
                0.95,
            );

            // 3. Soaring Gothic Obelisk Needle in the center of the Piazza
            let obelisk_h = (h - 0.95).max(2.2);
            mb.push_frustum(
                cx,
                0.95,
                cz,
                ped_w * 0.58,
                ped_w * 0.58,
                ped_w * 0.24,
                ped_w * 0.24,
                obelisk_h * 0.72,
                mat.stone_alt,
                0.0,
            );
            let pyramidion_y = 0.95 + obelisk_h * 0.72;
            mb.push_pyramid(
                cx,
                pyramidion_y,
                cz,
                ped_w * 0.30,
                obelisk_h * 0.28,
                ped_w * 0.30,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                0.95 + obelisk_h,
                cz,
                0.12,
                0.12,
                0.12,
                theme.window_warm,
                1.0,
            );

            // 4. Four Corner Gothic Colonnade Pavilions & Glowing Braziers framing the square
            let pav_w = (w.min(d) * 0.22).clamp(0.28, 0.52);
            let pav_h = (h * 0.34).clamp(1.1, 1.65);
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    let px = cx + sx * (w * 0.5 - pav_w * 0.55);
                    let pz = cz + sz * (d * 0.5 - pav_w * 0.55);
                    mb.push_box(px, 0.08, pz, pav_w, pav_h, pav_w, mat.stone_main, 0.0);
                    mb.push_box(
                        px,
                        0.08 + pav_h * 0.58,
                        pz,
                        pav_w + 0.04,
                        0.16,
                        pav_w + 0.04,
                        theme.window_warm,
                        1.0,
                    );
                    mb.push_pyramid(
                        px,
                        0.08 + pav_h,
                        pz,
                        pav_w * 1.12,
                        0.48,
                        pav_w * 1.12,
                        mat.roof,
                        0.0,
                    );
                }
            }
        }

        BuildingArchetype::AtlasDecoTower => {
            // Soaring slender Art-Deco supertall monolith with continuous vertical light strips,
            // cantilevered titan/gargoyle shoulder buttresses, and a Chrysler/Furst sunburst spire!
            let h0 = h * 0.34;
            let h1 = h * 0.34;
            let h2 = h * 0.20;
            let h3 = h * 0.12;

            // Tier 0: Grand Podium Base (0.0 .. h * 0.34, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                0.78,
                cz,
                w,
                h0 - 0.92,
                d,
                4,
                b.style_variant,
                &mat,
                theme,
            );

            // Tier 1: Soaring Fluted Deco Shaft (h * 0.34 .. h * 0.68, scale = 0.82)
            let y1 = h0;
            let w1 = w * 0.82;
            let d1 = d * 0.82;
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y1, cz, w1, h1, d1, mat.stone_alt, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                y1 + 0.12,
                cz,
                w1,
                h1 - 0.28,
                d1,
                3,
                b.style_variant + 1,
                &mat,
                theme,
            );

            // Tier 2: Cantilevered Shoulder Lantern Tier (h * 0.68 .. h * 0.88, scale = 0.64)
            let y2 = h0 + h1;
            let w2 = w * 0.64;
            let d2 = d * 0.64;
            mb.push_box(cx, y2, cz, w2, h2, d2, mat.stone_alt, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y2, cz, w2, h2, d2, mat.stone_base, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                y2 + 0.10,
                cz,
                w2,
                h2 - 0.24,
                d2,
                2,
                b.style_variant + 2,
                &mat,
                theme,
            );

            // 4 Cantilevered Titan/Gargoyle Shoulder Buttresses on the 4 cardinal faces at y2
            for sign in [-1.0f32, 1.0f32] {
                mb.push_frustum(
                    cx + sign * (w1 * 0.44),
                    y2,
                    cz,
                    0.34,
                    d2 * 0.42,
                    0.18,
                    d2 * 0.26,
                    h2 * 0.82,
                    mat.trim,
                    0.0,
                );
                mb.push_frustum(
                    cx,
                    y2,
                    cz + sign * (d1 * 0.44),
                    w2 * 0.42,
                    0.34,
                    w2 * 0.26,
                    0.18,
                    h2 * 0.82,
                    mat.trim,
                    0.0,
                );
            }

            // Tier 3: Sunburst Crown Observatory (h * 0.88 .. h, scale = 0.44)
            let y3 = y2 + h2;
            let w3 = w * 0.44;
            let d3 = d * 0.44;
            mb.push_box(cx, y3, cz, w3, h3, d3, mat.stone_main, 0.0);
            mb.push_box(
                cx,
                y3 + h3 * 0.35,
                cz,
                w3 + 0.06,
                h3 * 0.38,
                d3 + 0.06,
                theme.window_warm,
                1.0,
            );
            // Sunburst Art-Deco Spire + Ruby Beacon
            mb.push_pyramid(cx, h, cz, w3 * 1.12, h * 0.24, d3 * 1.12, mat.roof, 0.0);
            mb.push_pyramid(
                cx,
                h + h * 0.18,
                cz,
                w3 * 0.32,
                h * 0.14,
                d3 * 0.32,
                mat.trim,
                0.0,
            );
            mb.push_box(
                cx,
                h + h * 0.32,
                cz,
                0.12,
                0.14,
                0.12,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ChanceryArcadeKeep => {
            // Twin-Spire Gothic Chancery Citadel with an integral high-altitude vaulted sky-arch
            // bridge joining its asymmetric twin towers!
            let h0 = h * 0.38;
            let h1 = h * 0.28;
            let h2 = h * 0.34;

            // Tier 0: Heavy Vaulted Chancery Hall (0.0 .. h * 0.38, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.76,
                cz,
                w,
                h0 - 0.90,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // Tier 1: Stepped Mid-Keep Envelope (h * 0.38 .. h * 0.66, scale = 0.86)
            let y1 = h0;
            let w1 = w * 0.86;
            let d1 = d * 0.86;
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y1, cz, w1, h1, d1, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                y1 + 0.12,
                cz,
                w1,
                h1 - 0.26,
                d1,
                3,
                2,
                b.style_variant + 1,
                theme,
            );

            // Tier 2: Twin Asymmetric Gothic Towers (h * 0.66 .. h, outer envelope scale = 0.76)
            // joined near the top by an integral vaulted stone Sky-Arch Bridge!
            let y2 = h0 + h1;
            let w2 = w * 0.76;
            let d2 = d * 0.76;
            let rotated = !b.quarter_turns.is_multiple_of(2);

            for (idx, side) in [-1.0f32, 1.0f32].iter().enumerate() {
                // Slight height asymmetry between the twin spires for organic gothic drama
                let spire_scale = if idx == 0 { 1.06 } else { 0.92 };
                let tw_h = h2 * spire_scale;
                let (tx, tz, tw_x, tw_z) = if !rotated {
                    (cx + side * (w2 * 0.31), cz, w2 * 0.38, d2)
                } else {
                    (cx, cz + side * (d2 * 0.31), w2, d2 * 0.38)
                };
                mb.push_box(tx, y2, tz, tw_x, tw_h, tw_z, mat.stone_alt, 0.0);
                push_gothic_ribs_and_cornice(
                    mb,
                    tx,
                    y2,
                    tz,
                    tw_x,
                    tw_h,
                    tw_z,
                    mat.stone_base,
                    mat.trim,
                );
                push_gothic_lancet_windows(
                    mb,
                    tx,
                    y2 + 0.12,
                    tz,
                    tw_x,
                    tw_h - 0.26,
                    tw_z,
                    1,
                    2,
                    b.style_variant + idx as u8,
                    theme,
                );
                // Steep copper gable/pyramid spire on each twin tower
                mb.push_pyramid(
                    tx,
                    y2 + tw_h,
                    tz,
                    tw_x * 1.10,
                    h * 0.20 * spire_scale,
                    tw_z * 1.10,
                    mat.roof,
                    0.0,
                );
            }

            // Integral High-Altitude Sky-Arch Bridge spanning between the twin towers
            let arch_y = y2 + h2 * 0.56;
            let (arch_w, arch_d) = if !rotated {
                (w2 * 0.52, d2 * 0.44)
            } else {
                (w2 * 0.44, d2 * 0.52)
            };
            mb.push_box(cx, arch_y, cz, arch_w, 0.38, arch_d, mat.trim, 0.0);
            mb.push_box(
                cx,
                arch_y + 0.10,
                cz,
                arch_w + 0.04,
                0.16,
                arch_d + 0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_pyramid(cx, arch_y + 0.38, cz, 0.26, 0.55, 0.26, mat.roof, 0.0);
        }

        BuildingArchetype::AlchemicalObservatory => {
            // Stepped Gothic-Industrial Bastion crowned by an 8-sided ribbed verdigris copper
            // observatory dome, glowing astronomical aperture, and corner lightning-coil pinnacles!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];
            let h0 = h * 0.44;
            let h1 = h * 0.32;
            let h2 = h * 0.24;

            // Tier 0: Buttressed Laboratory Citadel (0.0 .. h * 0.44, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.76,
                cz,
                w,
                h0 - 0.90,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // Tier 1: Stepped Octagonal Instrument Drum (h * 0.44 .. h * 0.76, scale = 0.80)
            let y1 = h0;
            let w1 = w * 0.80;
            let d1 = d * 0.80;
            let r1 = w1.min(d1);
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            mb.push_oriented_box([cx, y1, cz], diag, r1, h1 - 0.04, r1, mat.stone_alt, 0.0);
            push_gothic_window_bands(
                mb,
                cx,
                y1 + 0.14,
                cz,
                w1,
                h1 - 0.28,
                d1,
                2,
                2,
                b.style_variant + 2,
                theme,
            );

            // 4 Corner Tesla / Lightning-Coil Pinnacles on the Tier 1 terrace
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    let px = cx + sx * (w * 0.42);
                    let pz = cz + sz * (d * 0.42);
                    mb.push_box(px, y1, pz, 0.20, h1 * 0.55, 0.20, mat.stone_alt, 0.0);
                    mb.push_box(
                        px,
                        y1 + h1 * 0.55,
                        pz,
                        0.16,
                        0.12,
                        0.16,
                        theme.window_dim,
                        0.95,
                    );
                    mb.push_pyramid(
                        px,
                        y1 + h1 * 0.55 + 0.12,
                        pz,
                        0.14,
                        0.48,
                        0.14,
                        mat.trim,
                        0.0,
                    );
                }
            }

            // Tier 2: 8-Sided Ribbed Copper Observatory Dome (h * 0.76 .. h, scale = 0.62)
            let y2 = h0 + h1;
            let r2 = w.min(d) * 0.62;
            // Lower dome drum + glowing clerestory ring
            mb.push_box(cx, y2, cz, r2, h2 * 0.42, r2, mat.stone_alt, 0.0);
            mb.push_oriented_box([cx, y2, cz], diag, r2, h2 * 0.40, r2, mat.stone_main, 0.0);
            mb.push_box(
                cx,
                y2 + h2 * 0.18,
                cz,
                r2 + 0.05,
                0.14,
                r2 + 0.05,
                theme.window_warm,
                1.0,
            );

            // Stepped octagonal copper dome hemisphere + meridian ribs
            let dome_y = y2 + h2 * 0.42;
            let dome_h = h2 * 0.78;
            mb.push_frustum(
                cx,
                dome_y,
                cz,
                r2 * 0.98,
                r2 * 0.98,
                r2 * 0.62,
                r2 * 0.62,
                dome_h * 0.58,
                mat.roof,
                0.0,
            );
            mb.push_pyramid(
                cx,
                dome_y + dome_h * 0.58,
                cz,
                r2 * 0.64,
                dome_h * 0.55,
                r2 * 0.64,
                mat.roof,
                0.0,
            );

            // Glowing Astronomical Telescope Shutter Slit & Projecting Cupola
            mb.push_box(
                cx,
                dome_y + dome_h * 0.12,
                cz + r2 * 0.42,
                r2 * 0.28,
                dome_h * 0.52,
                0.26,
                mat.trim,
                0.0,
            );
            mb.push_box(
                cx,
                dome_y + dome_h * 0.16,
                cz + r2 * 0.56,
                r2 * 0.18,
                dome_h * 0.40,
                0.06,
                theme.window_warm,
                1.0,
            );
        }
    }
}
