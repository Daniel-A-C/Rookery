//! Color palette helpers and stream geometry for *The Living ASCII Rookery*.

use rookery_core::world::RookeryPalette;

pub(super) fn palette_bark_colors(palette: RookeryPalette) -> ([f32; 4], [f32; 4]) {
    match palette {
        RookeryPalette::MidnightStarlight => ([0.12, 0.16, 0.22, 0.95], [0.72, 0.82, 0.92, 0.98]),
        RookeryPalette::AutumnHearth => ([0.22, 0.12, 0.08, 0.95], [0.85, 0.52, 0.28, 0.98]),
        RookeryPalette::VerdantRainforest => ([0.12, 0.18, 0.10, 0.95], [0.55, 0.82, 0.42, 0.98]),
        RookeryPalette::AmberPhosphor => ([0.32, 0.18, 0.04, 0.95], [0.98, 0.75, 0.20, 0.98]),
        RookeryPalette::MatrixGreen => ([0.05, 0.24, 0.08, 0.95], [0.28, 0.95, 0.38, 0.98]),
        RookeryPalette::EarthAndSylvan => ([0.24, 0.16, 0.10, 0.92], [0.76, 0.56, 0.36, 0.98]),
    }
}

/// Returns `(bg_col, fg_col, tile_char)` for canopy and bush/sapling foliage matching the tree aesthetic.
pub(super) fn palette_foliage_colors(
    palette: RookeryPalette,
    color_idx: u8,
    idx: usize,
) -> ([f32; 4], [f32; 4], char) {
    match (palette, color_idx) {
        (RookeryPalette::AutumnHearth, 0) => (
            [0.32, 0.14, 0.06, 0.90],
            [0.98, 0.48, 0.15, 0.98],
            if idx.is_multiple_of(2) { '▓' } else { '☘' },
        ),
        (RookeryPalette::AutumnHearth, 1) => (
            [0.35, 0.22, 0.05, 0.90],
            [0.98, 0.82, 0.20, 0.98],
            if idx.is_multiple_of(2) { '▒' } else { '*' },
        ),
        (RookeryPalette::AutumnHearth, 2) => {
            ([0.38, 0.08, 0.08, 0.92], [0.98, 0.28, 0.22, 0.98], '◆')
        }
        (RookeryPalette::AutumnHearth, _) => {
            ([0.28, 0.12, 0.06, 0.90], [0.85, 0.58, 0.22, 0.98], '✦')
        }
        (RookeryPalette::MidnightStarlight, 0) => (
            [0.06, 0.18, 0.28, 0.90],
            [0.35, 0.88, 0.98, 0.98],
            if idx.is_multiple_of(2) { '☘' } else { '▓' },
        ),
        (RookeryPalette::MidnightStarlight, 1) => (
            [0.10, 0.22, 0.32, 0.90],
            [0.72, 0.92, 1.0, 0.98],
            if idx.is_multiple_of(2) { '▒' } else { '*' },
        ),
        (RookeryPalette::MidnightStarlight, _) => {
            ([0.18, 0.12, 0.28, 0.90], [0.88, 0.78, 0.98, 0.98], '✦')
        }
        (RookeryPalette::AmberPhosphor, _) => (
            [0.24, 0.14, 0.02, 0.90],
            [1.0, 0.78, 0.22, 0.98],
            if idx.is_multiple_of(2) { '▓' } else { '*' },
        ),
        (RookeryPalette::MatrixGreen, _) => (
            [0.04, 0.22, 0.06, 0.90],
            [0.25, 0.98, 0.38, 0.98],
            if idx.is_multiple_of(2) { '▒' } else { '☘' },
        ),
        _ => match color_idx {
            0 => (
                [0.06, 0.28, 0.14, 0.88],
                [0.28, 0.95, 0.54, 0.98],
                if idx.is_multiple_of(2) { '☘' } else { '▓' },
            ),
            1 => (
                [0.10, 0.34, 0.16, 0.85],
                [0.52, 0.96, 0.60, 0.98],
                if idx.is_multiple_of(2) { '▒' } else { '*' },
            ),
            2 => ([0.34, 0.24, 0.05, 0.88], [0.98, 0.84, 0.25, 0.98], '✦'),
            _ => ([0.36, 0.10, 0.20, 0.88], [0.98, 0.58, 0.78, 0.98], '◆'),
        },
    }
}

/// Returns `(water_bg, water_fg, bank_stone)` color tokens for the forest stream.
pub(super) fn palette_water_colors(palette: RookeryPalette) -> ([f32; 4], [f32; 4], [f32; 4]) {
    match palette {
        RookeryPalette::MidnightStarlight => (
            [0.04, 0.14, 0.26, 0.92],
            [0.42, 0.78, 0.98, 0.94],
            [0.36, 0.44, 0.54, 0.92],
        ),
        RookeryPalette::AutumnHearth => (
            [0.07, 0.13, 0.20, 0.92],
            [0.46, 0.72, 0.88, 0.92],
            [0.48, 0.38, 0.30, 0.92],
        ),
        RookeryPalette::VerdantRainforest => (
            [0.03, 0.18, 0.22, 0.92],
            [0.35, 0.88, 0.85, 0.94],
            [0.34, 0.48, 0.40, 0.92],
        ),
        RookeryPalette::AmberPhosphor => (
            [0.14, 0.10, 0.02, 0.92],
            [0.92, 0.72, 0.22, 0.92],
            [0.54, 0.40, 0.16, 0.92],
        ),
        RookeryPalette::MatrixGreen => (
            [0.02, 0.14, 0.06, 0.92],
            [0.24, 0.88, 0.46, 0.92],
            [0.18, 0.44, 0.24, 0.92],
        ),
        RookeryPalette::EarthAndSylvan => (
            [0.05, 0.16, 0.26, 0.92],
            [0.44, 0.82, 0.96, 0.94],
            [0.44, 0.48, 0.46, 0.92],
        ),
    }
}

/// Returns `(world_center_col, half_width)` for the meandering forest stream at row `r`.
pub(super) fn stream_geometry(r: i32, ground_row: i32, cols: i32) -> (f32, f32) {
    let dr = (r - ground_row) as f32;
    let center =
        (cols as f32 * 0.74) - dr * 0.85 + (dr * 0.42).sin() * 5.0 + (dr * 0.18).cos() * 2.5;
    let depth_norm = (dr / 14.0).clamp(0.0, 1.0);
    let half_w = 1.6 + depth_norm * 1.6;
    (center, half_w)
}

#[derive(Clone, Copy)]
pub(super) struct ShadowPalette {
    pub(super) umbral: [f32; 4],
    pub(super) deep: [f32; 4],
    pub(super) soft: [f32; 4],
}
