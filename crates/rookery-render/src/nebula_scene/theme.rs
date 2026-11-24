//! Curated JWST / Hubble visual color palettes for Cosmic Nebula v2.0.

use rookery_core::nebula::NebulaPalette;

#[derive(Clone, Copy, Debug)]
pub struct NebulaVisualPalette {
    pub deep_space: [f32; 4],
    pub gas_primary: [f32; 4],
    pub gas_secondary: [f32; 4],
    pub dust_dark: [f32; 4],
    pub rim_glow: [f32; 4],
    pub star_warm: [f32; 4],
    pub star_cool: [f32; 4],
    pub chart_accent: [f32; 4],
}

pub fn palette_for(p: NebulaPalette) -> NebulaVisualPalette {
    match p {
        NebulaPalette::CarinaCliffs => NebulaVisualPalette {
            deep_space: [0.012, 0.018, 0.042, 1.0],
            gas_primary: [0.09, 0.27, 0.44, 1.0],
            gas_secondary: [0.76, 0.36, 0.16, 1.0],
            dust_dark: [0.045, 0.022, 0.018, 1.0],
            rim_glow: [1.00, 0.78, 0.40, 1.0],
            star_warm: [1.00, 0.86, 0.58, 1.0],
            star_cool: [0.68, 0.89, 1.00, 1.0],
            chart_accent: [0.98, 0.80, 0.46, 1.0],
        },
        NebulaPalette::PillarsOfCreation => NebulaVisualPalette {
            deep_space: [0.008, 0.022, 0.036, 1.0],
            gas_primary: [0.07, 0.36, 0.40, 1.0],
            gas_secondary: [0.64, 0.44, 0.18, 1.0],
            dust_dark: [0.038, 0.026, 0.016, 1.0],
            rim_glow: [0.46, 0.96, 0.88, 1.0],
            star_warm: [1.00, 0.88, 0.52, 1.0],
            star_cool: [0.58, 0.94, 1.00, 1.0],
            chart_accent: [0.48, 0.94, 0.86, 1.0],
        },
        NebulaPalette::HorseheadCrimson => NebulaVisualPalette {
            deep_space: [0.020, 0.008, 0.026, 1.0],
            gas_primary: [0.56, 0.09, 0.24, 1.0],
            gas_secondary: [0.88, 0.28, 0.38, 1.0],
            dust_dark: [0.035, 0.010, 0.018, 1.0],
            rim_glow: [1.00, 0.54, 0.68, 1.0],
            star_warm: [1.00, 0.76, 0.62, 1.0],
            star_cool: [0.62, 0.82, 1.00, 1.0],
            chart_accent: [0.98, 0.62, 0.74, 1.0],
        },
        NebulaPalette::HelixViolet => NebulaVisualPalette {
            deep_space: [0.016, 0.009, 0.038, 1.0],
            gas_primary: [0.32, 0.11, 0.56, 1.0],
            gas_secondary: [0.80, 0.28, 0.74, 1.0],
            dust_dark: [0.024, 0.012, 0.044, 1.0],
            rim_glow: [0.46, 0.92, 1.00, 1.0],
            star_warm: [0.98, 0.74, 0.98, 1.0],
            star_cool: [0.52, 0.94, 1.00, 1.0],
            chart_accent: [0.76, 0.64, 1.00, 1.0],
        },
        NebulaPalette::VeilAurora => NebulaVisualPalette {
            deep_space: [0.008, 0.016, 0.034, 1.0],
            gas_primary: [0.08, 0.44, 0.38, 1.0],
            gas_secondary: [0.22, 0.52, 0.84, 1.0],
            dust_dark: [0.014, 0.024, 0.036, 1.0],
            rim_glow: [0.48, 1.00, 0.78, 1.0],
            star_warm: [1.00, 0.84, 0.64, 1.0],
            star_cool: [0.56, 1.00, 0.90, 1.0],
            chart_accent: [0.52, 0.98, 0.80, 1.0],
        },
    }
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

pub fn lerp_palette(a: NebulaVisualPalette, b: NebulaVisualPalette, t: f32) -> NebulaVisualPalette {
    let s = t.clamp(0.0, 1.0);
    let smooth = s * s * (3.0 - 2.0 * s);
    NebulaVisualPalette {
        deep_space: lerp4(a.deep_space, b.deep_space, smooth),
        gas_primary: lerp4(a.gas_primary, b.gas_primary, smooth),
        gas_secondary: lerp4(a.gas_secondary, b.gas_secondary, smooth),
        dust_dark: lerp4(a.dust_dark, b.dust_dark, smooth),
        rim_glow: lerp4(a.rim_glow, b.rim_glow, smooth),
        star_warm: lerp4(a.star_warm, b.star_warm, smooth),
        star_cool: lerp4(a.star_cool, b.star_cool, smooth),
        chart_accent: lerp4(a.chart_accent, b.chart_accent, smooth),
    }
}
