//! Curated painterly color palettes for the 4 seasonal & time-of-day Koi Pond themes.

use rookery_core::koi::PondTheme;

/// Full color & lighting palette for a pond atmosphere.
#[derive(Clone, Copy, Debug)]
pub struct PondPalette {
    pub water_deep: [f32; 3],
    pub water_mid: [f32; 3],
    pub water_shallow: [f32; 3],
    pub caustic_color: [f32; 3],
    pub pebble_tint: [f32; 3],
    pub pebble_moss: [f32; 3],
    pub sun_tint: [f32; 3],
    pub dapple_strength: f32,
    pub vignette_strength: f32,
    pub ripple_tint: [f32; 3],
    pub lily_base: [f32; 3],
    pub lily_vein: [f32; 3],
    pub lily_rim: [f32; 3],
    pub rock_base: [f32; 3],
    pub rock_moss: [f32; 3],
    pub water_absorb: [f32; 3],
    pub shadow_color: [f32; 4],
    pub ambient_mul: f32,
    pub petal_kind: f32, // 0.0 = sakura, 1.0 = summer blossom, 2.0 = maple leaf, 3.0 = firefly
}

pub fn palette_for(theme: PondTheme) -> PondPalette {
    match theme {
        PondTheme::SakuraMorning => PondPalette {
            water_deep: [0.032, 0.115, 0.155],
            water_mid: [0.062, 0.195, 0.228],
            water_shallow: [0.125, 0.295, 0.305],
            caustic_color: [0.62, 0.84, 0.82],
            pebble_tint: [0.32, 0.36, 0.36],
            pebble_moss: [0.14, 0.34, 0.22],
            sun_tint: [1.0, 0.86, 0.84],
            dapple_strength: 0.30,
            vignette_strength: 0.34,
            ripple_tint: [0.88, 0.94, 0.96],
            lily_base: [0.16, 0.44, 0.25],
            lily_vein: [0.30, 0.59, 0.35],
            lily_rim: [0.10, 0.30, 0.18],
            rock_base: [0.27, 0.30, 0.31],
            rock_moss: [0.22, 0.45, 0.23],
            water_absorb: [0.05, 0.17, 0.21],
            shadow_color: [0.015, 0.055, 0.08, 0.46],
            ambient_mul: 0.98,
            petal_kind: 0.0,
        },
        PondTheme::SunlitJade => PondPalette {
            water_deep: [0.016, 0.095, 0.118],
            water_mid: [0.038, 0.185, 0.188],
            water_shallow: [0.088, 0.295, 0.255],
            caustic_color: [0.56, 0.88, 0.72],
            pebble_tint: [0.34, 0.34, 0.28],
            pebble_moss: [0.14, 0.38, 0.18],
            sun_tint: [1.0, 0.94, 0.74],
            dapple_strength: 0.38,
            vignette_strength: 0.32,
            ripple_tint: [0.80, 0.96, 0.92],
            lily_base: [0.14, 0.47, 0.23],
            lily_vein: [0.28, 0.64, 0.32],
            lily_rim: [0.08, 0.30, 0.15],
            rock_base: [0.28, 0.29, 0.26],
            rock_moss: [0.20, 0.48, 0.21],
            water_absorb: [0.03, 0.16, 0.17],
            shadow_color: [0.01, 0.045, 0.06, 0.50],
            ambient_mul: 1.04,
            petal_kind: 1.0,
        },
        PondTheme::AutumnMapleDusk => PondPalette {
            water_deep: [0.048, 0.058, 0.092],
            water_mid: [0.105, 0.098, 0.122],
            water_shallow: [0.185, 0.142, 0.128],
            caustic_color: [0.92, 0.58, 0.34],
            pebble_tint: [0.32, 0.25, 0.21],
            pebble_moss: [0.22, 0.28, 0.15],
            sun_tint: [1.0, 0.66, 0.40],
            dapple_strength: 0.30,
            vignette_strength: 0.42,
            ripple_tint: [0.96, 0.78, 0.60],
            lily_base: [0.23, 0.35, 0.18],
            lily_vein: [0.40, 0.45, 0.23],
            lily_rim: [0.18, 0.22, 0.11],
            rock_base: [0.24, 0.21, 0.20],
            rock_moss: [0.28, 0.33, 0.16],
            water_absorb: [0.08, 0.08, 0.11],
            shadow_color: [0.02, 0.02, 0.04, 0.52],
            ambient_mul: 0.90,
            petal_kind: 2.0,
        },
        PondTheme::MoonlitLanternNight => PondPalette {
            water_deep: [0.010, 0.026, 0.062],
            water_mid: [0.020, 0.054, 0.108],
            water_shallow: [0.038, 0.092, 0.155],
            caustic_color: [0.30, 0.52, 0.78],
            pebble_tint: [0.14, 0.18, 0.26],
            pebble_moss: [0.06, 0.16, 0.18],
            sun_tint: [0.68, 0.82, 1.0],
            dapple_strength: 0.22,
            vignette_strength: 0.48,
            ripple_tint: [0.64, 0.82, 1.0],
            lily_base: [0.07, 0.21, 0.18],
            lily_vein: [0.14, 0.32, 0.27],
            lily_rim: [0.04, 0.13, 0.11],
            rock_base: [0.12, 0.15, 0.20],
            rock_moss: [0.09, 0.22, 0.19],
            water_absorb: [0.018, 0.045, 0.095],
            shadow_color: [0.005, 0.012, 0.035, 0.58],
            ambient_mul: 0.76,
            petal_kind: 3.0,
        },
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

#[inline]
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}

pub fn lerp_palette(a: &PondPalette, b: &PondPalette, t: f32) -> PondPalette {
    let s = t.clamp(0.0, 1.0);
    let smooth = s * s * (3.0 - 2.0 * s);
    PondPalette {
        water_deep: lerp3(a.water_deep, b.water_deep, smooth),
        water_mid: lerp3(a.water_mid, b.water_mid, smooth),
        water_shallow: lerp3(a.water_shallow, b.water_shallow, smooth),
        caustic_color: lerp3(a.caustic_color, b.caustic_color, smooth),
        pebble_tint: lerp3(a.pebble_tint, b.pebble_tint, smooth),
        pebble_moss: lerp3(a.pebble_moss, b.pebble_moss, smooth),
        sun_tint: lerp3(a.sun_tint, b.sun_tint, smooth),
        dapple_strength: lerp(a.dapple_strength, b.dapple_strength, smooth),
        vignette_strength: lerp(a.vignette_strength, b.vignette_strength, smooth),
        ripple_tint: lerp3(a.ripple_tint, b.ripple_tint, smooth),
        lily_base: lerp3(a.lily_base, b.lily_base, smooth),
        lily_vein: lerp3(a.lily_vein, b.lily_vein, smooth),
        lily_rim: lerp3(a.lily_rim, b.lily_rim, smooth),
        rock_base: lerp3(a.rock_base, b.rock_base, smooth),
        rock_moss: lerp3(a.rock_moss, b.rock_moss, smooth),
        water_absorb: lerp3(a.water_absorb, b.water_absorb, smooth),
        shadow_color: lerp4(a.shadow_color, b.shadow_color, smooth),
        ambient_mul: lerp(a.ambient_mul, b.ambient_mul, smooth),
        petal_kind: if smooth < 0.5 {
            a.petal_kind
        } else {
            b.petal_kind
        },
    }
}
