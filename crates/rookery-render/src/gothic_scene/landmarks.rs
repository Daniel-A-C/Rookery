//! Eight bespoke one-of-a-kind micro-landmarks placed in specific curated spots
//! across *Gothic Metropolis*.

use super::mesh_builder::MeshBuilder;
use super::theme::GothicThemeColors;

/// Build the static geometry for all 8 bespoke city landmarks.
/// Animated components (orrery rings, carillon bell, mooring beacon, precinct projector cone)
/// are emitted per-frame in [`super::GothicMetropolisScene::rebuild_dynamic_buffers`].
pub(super) fn build_bespoke_landmarks_mesh(mb: &mut MeshBuilder, theme: &GothicThemeColors) {
    // 1. Grand Clockwork Orrery Dais (Block 15 front courtyard)
    push_orrery_dais(mb, 3.45, 0.09, 5.70, theme);

    // 2. Nocturne Botanical Conservatory (Block 14 NE terrace)
    push_botanical_conservatory(mb, -0.38, 0.09, 6.50, theme);

    // 3. "Monarch" Art-Deco Cinema & Marquee (Block 10 SW corner)
    push_monarch_cinema(mb, -4.10, 0.09, 4.35, theme);

    // 4. Zeppelin Mooring Mast & Gantry (Crown of Building 3 AtlasDecoTower at y = 14.4)
    push_zeppelin_mooring_mast(mb, 2.05, 14.4, -6.85, theme);

    // 5. Foundry Overhead Gantry Crane (Spanning between Building 13 & Building 22 on Block 13)
    push_foundry_gantry_crane(mb, -6.88, 4.60, 7.96, theme);

    // 6. Gargoyle Carillon Pavilion (Block 14 NW courtyard)
    push_carillon_pavilion(mb, -3.22, 0.09, 6.52, theme);

    // 7. Art-Deco Metro Kiosk (Block 9 SE sidewalk near ViaductStation)
    push_metro_kiosk(mb, -6.30, 0.09, 4.25, theme);

    // 8. Precinct Rooftop Signal Housing & Water Tower (Block 16 foreground setback)
    push_precinct_rooftop_signal(mb, 8.95, 3.95, 6.85, theme);
}

fn push_orrery_dais(mb: &mut MeshBuilder, cx: f32, y0: f32, cz: f32, theme: &GothicThemeColors) {
    // Stepped stone dais with inlaid brass star ring
    mb.push_box(cx, y0, cz, 0.88, 0.06, 0.88, theme.stone_dark, 0.0);
    mb.push_box(
        cx,
        y0 + 0.06,
        cz,
        0.66,
        0.06,
        0.66,
        theme.stone_secondary,
        0.0,
    );
    mb.push_box(cx, y0 + 0.12, cz, 0.48, 0.03, 0.48, theme.accent_trim, 0.0);
    // Central brass pedestal
    mb.push_box(cx, y0 + 0.15, cz, 0.12, 0.24, 0.12, theme.accent_trim, 0.0);
}

fn push_botanical_conservatory(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    theme: &GothicThemeColors,
) {
    // Stone kneewall base
    mb.push_box(cx, y0, cz, 0.72, 0.14, 0.72, theme.stone_dark, 0.0);
    // Warm glowing amber-emerald glasshouse body
    mb.push_box(cx, y0 + 0.14, cz, 0.62, 0.34, 0.62, theme.window_warm, 0.78);
    // Wrought-iron corner mullions & mid-ribs
    for sx in [-0.31f32, 0.31] {
        for sz in [-0.31f32, 0.31] {
            mb.push_box(
                cx + sx,
                y0 + 0.14,
                cz + sz,
                0.05,
                0.36,
                0.05,
                theme.plinth_dark,
                0.0,
            );
        }
    }
    // Interior gothic palm/fern crown silhouette
    mb.push_box(cx, y0 + 0.14, cz, 0.16, 0.38, 0.16, theme.copper_roof, 0.0);
    // Ribbed copper-and-glass conservatory cupola
    mb.push_pyramid(cx, y0 + 0.48, cz, 0.68, 0.28, 0.68, theme.copper_roof, 0.0);
    mb.push_box(cx, y0 + 0.76, cz, 0.10, 0.08, 0.10, theme.accent_trim, 0.0);
}

fn push_monarch_cinema(mb: &mut MeshBuilder, cx: f32, y0: f32, cz: f32, theme: &GothicThemeColors) {
    // Stepped Art-Deco theater vestibule
    mb.push_box(cx, y0, cz, 0.76, 0.56, 0.64, theme.stone_dark, 0.0);
    mb.push_box(
        cx,
        y0 + 0.56,
        cz,
        0.58,
        0.24,
        0.48,
        theme.stone_secondary,
        0.0,
    );
    // Glowing ticket booth & entrance doors
    mb.push_box(
        cx,
        y0 + 0.04,
        cz + 0.33,
        0.24,
        0.28,
        0.06,
        theme.window_warm,
        0.95,
    );
    // Projecting illuminated triangular Art-Deco marquee
    mb.push_box(
        cx,
        y0 + 0.34,
        cz + 0.40,
        0.64,
        0.12,
        0.22,
        theme.accent_trim,
        0.0,
    );
    mb.push_box(
        cx,
        y0 + 0.36,
        cz + 0.42,
        0.56,
        0.07,
        0.20,
        theme.window_warm,
        1.0,
    );
    // Vertical illuminated "MONARCH" blade sign
    mb.push_box(
        cx + 0.26,
        y0 + 0.44,
        cz + 0.38,
        0.07,
        0.46,
        0.10,
        theme.beacon_red,
        0.95,
    );
}

fn push_zeppelin_mooring_mast(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    theme: &GothicThemeColors,
) {
    // Lattice steel mooring tower rising from tower crown
    mb.push_frustum(
        cx,
        y0,
        cz,
        0.44,
        0.44,
        0.18,
        0.18,
        1.35,
        theme.steel_viaduct,
        0.0,
    );
    // Cantilevered boarding gantry catwalk
    mb.push_box(
        cx + 0.34,
        y0 + 0.72,
        cz,
        0.62,
        0.08,
        0.18,
        theme.accent_trim,
        0.0,
    );
    mb.push_box(
        cx + 0.62,
        y0 + 0.80,
        cz,
        0.06,
        0.10,
        0.18,
        theme.plinth_dark,
        0.0,
    );
    // Upper mooring ring collar
    mb.push_box(cx, y0 + 1.35, cz, 0.24, 0.10, 0.24, theme.accent_trim, 0.0);
}

fn push_foundry_gantry_crane(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    theme: &GothicThemeColors,
) {
    // Twin riveted iron bridge girders spanning between Foundry 13 & Annex 22
    for dz in [-0.22f32, 0.22] {
        mb.push_box(cx, y0, cz + dz, 1.85, 0.14, 0.09, theme.steel_viaduct, 0.0);
    }
    // Trolley carriage & suspended molten crucible bucket
    mb.push_box(
        cx + 0.12,
        y0 + 0.14,
        cz,
        0.34,
        0.10,
        0.56,
        theme.accent_trim,
        0.0,
    );
    mb.push_box(
        cx + 0.12,
        y0 - 0.42,
        cz,
        0.04,
        0.42,
        0.04,
        theme.plinth_dark,
        0.0,
    );
    mb.push_box(
        cx + 0.12,
        y0 - 0.64,
        cz,
        0.24,
        0.22,
        0.24,
        theme.stone_dark,
        0.0,
    );
    // Glowing molten slag top rim
    mb.push_box(
        cx + 0.12,
        y0 - 0.44,
        cz,
        0.18,
        0.04,
        0.18,
        theme.window_warm,
        1.0,
    );
}

fn push_carillon_pavilion(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    theme: &GothicThemeColors,
) {
    // Stone plinth + 4 corner gothic pillars
    mb.push_box(cx, y0, cz, 0.66, 0.06, 0.66, theme.stone_secondary, 0.0);
    for sx in [-0.24f32, 0.24] {
        for sz in [-0.24f32, 0.24] {
            mb.push_box(
                cx + sx,
                y0 + 0.06,
                cz + sz,
                0.08,
                0.48,
                0.08,
                theme.stone_primary,
                0.0,
            );
        }
    }
    // Steep copper carillon roof
    mb.push_box(cx, y0 + 0.54, cz, 0.64, 0.06, 0.64, theme.accent_trim, 0.0);
    mb.push_pyramid(cx, y0 + 0.60, cz, 0.66, 0.36, 0.66, theme.copper_roof, 0.0);
}

fn push_metro_kiosk(mb: &mut MeshBuilder, cx: f32, y0: f32, cz: f32, theme: &GothicThemeColors) {
    // Sunken glowing stairwell opening + wrought-iron Art-Deco canopy
    mb.push_box(cx, y0, cz, 0.64, 0.04, 0.46, theme.window_dim, 0.85);
    for sx in [-0.28f32, 0.28] {
        for sz in [-0.19f32, 0.19] {
            mb.push_box(
                cx + sx,
                y0 + 0.04,
                cz + sz,
                0.05,
                0.34,
                0.05,
                theme.plinth_dark,
                0.0,
            );
        }
    }
    mb.push_box(cx, y0 + 0.38, cz, 0.70, 0.06, 0.52, theme.copper_roof, 0.0);
    // Glowing "M" metro lantern crest
    mb.push_box(
        cx,
        y0 + 0.44,
        cz + 0.22,
        0.16,
        0.10,
        0.06,
        theme.window_warm,
        1.0,
    );
}

fn push_precinct_rooftop_signal(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    theme: &GothicThemeColors,
) {
    // Stilted rooftop water tower next to the brass signal projector housing
    let wt_x = cx - 0.32;
    for sx in [-0.09f32, 0.09] {
        for sz in [-0.09f32, 0.09] {
            mb.push_box(
                wt_x + sx,
                y0,
                cz + sz,
                0.035,
                0.22,
                0.035,
                theme.plinth_dark,
                0.0,
            );
        }
    }
    mb.push_box(
        wt_x,
        y0 + 0.22,
        cz,
        0.24,
        0.26,
        0.24,
        theme.stone_secondary,
        0.0,
    );
    mb.push_pyramid(
        wt_x,
        y0 + 0.48,
        cz,
        0.28,
        0.14,
        0.28,
        theme.copper_roof,
        0.0,
    );

    // Brass-rimmed rooftop signal projector base
    mb.push_box(cx, y0, cz, 0.28, 0.14, 0.28, theme.steel_viaduct, 0.0);
    mb.push_box(cx, y0 + 0.14, cz, 0.20, 0.08, 0.20, theme.accent_trim, 0.0);
    mb.push_box(cx, y0 + 0.22, cz, 0.16, 0.05, 0.16, theme.window_warm, 1.0);
}
