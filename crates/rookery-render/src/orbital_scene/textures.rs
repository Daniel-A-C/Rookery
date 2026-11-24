//! Procedural Y2K pixel-art texture atlas (`512x512`, `8x8` grid of `64x64` tiles).
//!
//! Sampled with `wgpu::FilterMode::Nearest` to evoke late-90s Unreal Engine 1 / Dark Engine
//! industrial sci-fi textures (Deus Ex 2000 & System Shock 2).

pub const ATLAS_SIZE: u32 = 512;
pub const TILE_SIZE: u32 = 64;
pub const TILES_PER_ROW: u32 = ATLAS_SIZE / TILE_SIZE; // 8

/// Tile indices packed into the 8x8 procedural atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Tile {
    HullPanel = 0,
    DarkSteelRib = 1,
    HazardStripe = 2,
    FloorGrate = 3,
    CrtConsole = 4,
    BlastDoor = 5,
    LandingPad = 6,
    SolarCells = 7,
    CargoCrate = 8,
    ForcefieldHex = 9,
    CorridorWall = 10,
    CeilingLight = 11,
    ReactorCoil = 12,
    ServerRack = 13,
    RadiatorLouvers = 14,
    ShipArmor = 15,
}

impl Tile {
    /// Returns `[u_min, v_min, u_size, v_size]` in normalized `[0, 1]` atlas coordinates,
    /// inset by a half-texel to prevent bleeding at tile edges.
    pub fn rect(self) -> [f32; 4] {
        let idx = self as u32;
        let col = idx % TILES_PER_ROW;
        let row = idx / TILES_PER_ROW;
        let inv = 1.0 / TILES_PER_ROW as f32;
        let pad = 0.5 / ATLAS_SIZE as f32;
        [
            col as f32 * inv + pad,
            row as f32 * inv + pad,
            inv - 2.0 * pad,
            inv - 2.0 * pad,
        ]
    }
}

#[inline]
fn hash2(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

#[inline]
fn noise_i32(x: u32, y: u32, seed: u32, amp: i32) -> i32 {
    let h = hash2(x, y, seed) & 0xFF;
    ((h as i32) * (amp * 2 + 1) / 256) - amp
}

#[inline]
fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

fn put_px(buf: &mut [u8], tile: Tile, x: u32, y: u32, rgba: [u8; 4]) {
    let idx = tile as u32;
    let tx = (idx % TILES_PER_ROW) * TILE_SIZE + x;
    let ty = (idx / TILES_PER_ROW) * TILE_SIZE + y;
    let off = ((ty * ATLAS_SIZE + tx) * 4) as usize;
    buf[off..off + 4].copy_from_slice(&rgba);
}

fn fill_tile(buf: &mut [u8], tile: Tile, mut f: impl FnMut(u32, u32) -> [u8; 4]) {
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let rgba = f(x, y);
            put_px(buf, tile, x, y, rgba);
        }
    }
}

/// Generate the `512x512` RGBA8 procedural retro texture atlas.
pub fn generate_atlas_rgba() -> Vec<u8> {
    let mut buf = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];

    // 0: Riveted gunmetal hull panel (2x2 sub-panels with bevels and corner rivets)
    fill_tile(&mut buf, Tile::HullPanel, |x, y| {
        let sx = x % 32;
        let sy = y % 32;
        let n = noise_i32(x / 2, y / 2, 1, 9);
        let mut v = 136 + n;
        if sx == 0 || sy == 0 {
            v -= 48;
        } else if sx == 1 || sy == 1 {
            v += 22;
        } else if sx == 31 || sy == 31 {
            v -= 24;
        }
        // Corner rivets
        let rivet = (sx == 4 || sx == 27) && (sy == 4 || sy == 27);
        if rivet {
            v += 34;
        }
        [clamp_u8(v - 6), clamp_u8(v), clamp_u8(v + 8), 255]
    });

    // 1: Dark structural bulkhead rib / heavy steel truss
    fill_tile(&mut buf, Tile::DarkSteelRib, |x, y| {
        let n = noise_i32(x / 2, y / 2, 2, 7);
        let mut v = 86 + n;
        if !(4..60).contains(&x) || !(4..60).contains(&y) {
            v -= 22;
        }
        if (x % 16 == 0) || (y % 16 == 0) {
            v -= 16;
        }
        if (x + y) % 16 == 8 && x > 6 && x < 58 {
            v += 12;
        }
        [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 6), 255]
    });

    // 2: Black-and-yellow industrial hazard stripes
    fill_tile(&mut buf, Tile::HazardStripe, |x, y| {
        let diag = (x + y) % 16;
        let n = noise_i32(x / 2, y / 2, 3, 10);
        if diag < 8 {
            // Weathered industrial yellow/amber
            [
                clamp_u8(218 + n),
                clamp_u8(168 + n),
                clamp_u8(34 + n / 2),
                255,
            ]
        } else {
            // Scuffed dark charcoal
            [
                clamp_u8(34 + n / 2),
                clamp_u8(36 + n / 2),
                clamp_u8(40 + n / 2),
                255,
            ]
        }
    });

    // 3: Perforated industrial floor grating (catwalks & hangar bays)
    fill_tile(&mut buf, Tile::FloorGrate, |x, y| {
        let gx = x % 8;
        let gy = y % 8;
        let n = noise_i32(x, y, 4, 6);
        let is_bar = gx == 0 || gy == 0 || gx == 1 || (x % 32 < 2) || (y % 32 < 2);
        if is_bar {
            let v = 128 + n;
            [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 6), 255]
        } else {
            let v = 38 + n / 2;
            [clamp_u8(v), clamp_u8(v + 4), clamp_u8(v + 8), 255]
        }
    });

    // 4: Glowing Y2K CRT terminal console bank
    fill_tile(&mut buf, Tile::CrtConsole, |x, y| {
        let in_screen = (6..58).contains(&x) && (8..46).contains(&y);
        if in_screen {
            let scanline = if y % 2 == 0 { 16 } else { -12 };
            let grid = if x % 8 == 0 || y % 8 == 0 { 22 } else { 0 };
            // Stylized telemetry bars & waveform inside screen
            let wave_y = 26 + (((x as f32 * 0.35).sin() * 8.0) as i32);
            let on_wave = (y as i32 - wave_y).abs() <= 1;
            let bar = y > 36 && y < 42 && (x % 6) < 4;
            if on_wave || bar {
                [190, 255, 245, 255]
            } else {
                [
                    clamp_u8(28 + grid / 2),
                    clamp_u8(142 + scanline + grid),
                    clamp_u8(168 + scanline + grid),
                    255,
                ]
            }
        } else if (50..=57).contains(&y) && (x % 6) < 4 && (8..56).contains(&x) {
            // Glowing keyboard / indicator buttons below screen
            if x > 44 {
                [235, 130, 40, 255]
            } else {
                [75, 195, 180, 255]
            }
        } else {
            // Chunky CRT bezel casing
            let n = noise_i32(x, y, 5, 6);
            [clamp_u8(52 + n), clamp_u8(58 + n), clamp_u8(66 + n), 255]
        }
    });

    // 5: Reinforced blast-door chevrons & lock bolts
    fill_tile(&mut buf, Tile::BlastDoor, |x, y| {
        let n = noise_i32(x / 2, y / 2, 6, 8);
        let mut v = 112 + n;
        let seam = x == 31 || x == 32;
        if seam {
            v = 24;
        } else if !(6..58).contains(&x) || !(6..58).contains(&y) {
            v = 74 + n;
        } else if y > 26 && y < 38 && ((x + y) % 12) < 5 {
            // Subtle warning band across center of blast door
            return [clamp_u8(185 + n), clamp_u8(120 + n), clamp_u8(36), 255];
        }
        [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 8), 255]
    });

    // 6: Landing pad deck composite with corner guide stripes
    fill_tile(&mut buf, Tile::LandingPad, |x, y| {
        let n = noise_i32(x / 2, y / 2, 7, 10);
        let border = !(4..60).contains(&x) || !(4..60).contains(&y);
        if border && ((x / 8 + y / 8) % 2 == 0) {
            return [clamp_u8(215 + n), clamp_u8(165 + n), clamp_u8(42), 255];
        }
        let mut v = 96 + n;
        if x % 16 == 0 || y % 16 == 0 {
            v -= 18;
        }
        [clamp_u8(v - 2), clamp_u8(v), clamp_u8(v + 4), 255]
    });

    // 7: Deep cobalt photovoltaic solar cells with gold busbars
    fill_tile(&mut buf, Tile::SolarCells, |x, y| {
        let cx = x % 16;
        let cy = y % 8;
        if cx == 0 || cy == 0 {
            [178, 146, 72, 255] // gold busbar
        } else {
            let n = noise_i32(x / 2, y / 2, 8, 8);
            [clamp_u8(28 + n), clamp_u8(62 + n), clamp_u8(138 + n), 255]
        }
    });

    // 8: Industrial cargo crate with braced edges & stencil band
    fill_tile(&mut buf, Tile::CargoCrate, |x, y| {
        let n = noise_i32(x / 2, y / 2, 9, 8);
        let frame = !(6..58).contains(&x) || !(6..58).contains(&y);
        if frame {
            let v = 78 + n;
            [clamp_u8(v + 8), clamp_u8(v + 2), clamp_u8(v - 6), 255]
        } else if (26..34).contains(&y) {
            [clamp_u8(198 + n), clamp_u8(122 + n), clamp_u8(38), 255]
        } else {
            let rib = if x % 12 == 0 { -16 } else { 0 };
            let v = 132 + n + rib;
            [clamp_u8(v + 12), clamp_u8(v + 6), clamp_u8(v - 8), 255]
        }
    });

    // 9: Translucent atmospheric containment forcefield hex/grid
    fill_tile(&mut buf, Tile::ForcefieldHex, |x, y| {
        let gx = x % 16;
        let gy = y % 16;
        let diag = (x + y) % 16 == 0 || (x + 64 - y) % 16 == 0;
        let edge = gx == 0 || gy == 0 || diag;
        if edge {
            [115, 235, 255, 145]
        } else {
            [40, 175, 225, 42]
        }
    });

    // 10: Deus Ex / System Shock 2 interior corridor wall (upper panel, waist rail, dark kickplate)
    fill_tile(&mut buf, Tile::CorridorWall, |x, y| {
        let n = noise_i32(x / 2, y / 2, 10, 7);
        if y < 8 {
            // Top dark soffit trim
            let v = 64 + n;
            [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 6), 255]
        } else if (28..34).contains(&y) {
            // Waist conduit rail with subtle accent stripe
            if y == 30 || y == 31 {
                [88, 162, 178, 255]
            } else {
                [54, 62, 70, 255]
            }
        } else if y >= 50 {
            // Dark scuffed floor kickplate
            let v = 56 + n;
            [clamp_u8(v), clamp_u8(v + 2), clamp_u8(v + 4), 255]
        } else {
            let seam = if x % 32 == 0 {
                -28
            } else if x % 32 == 1 {
                14
            } else {
                0
            };
            let v = 124 + n + seam;
            [clamp_u8(v - 6), clamp_u8(v + 2), clamp_u8(v + 8), 255]
        }
    });

    // 11: Overhead ceiling duct & glowing fluorescent strip
    fill_tile(&mut buf, Tile::CeilingLight, |x, y| {
        if (22..42).contains(&x) && (8..56).contains(&y) {
            let rib = if y % 12 == 0 { -35 } else { 0 };
            [
                clamp_u8(205 + rib),
                clamp_u8(240 + rib),
                clamp_u8(255 + rib),
                255,
            ]
        } else {
            let n = noise_i32(x / 2, y / 2, 11, 6);
            let v = 68 + n;
            [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 6), 255]
        }
    });

    // 12: Reactor plasma coil
    fill_tile(&mut buf, Tile::ReactorCoil, |x, y| {
        let band = y % 12;
        if band < 7 {
            let core = 255 - ((x as i32 - 32).abs() * 3).min(90);
            [clamp_u8(core / 2), clamp_u8(core), 255, 255]
        } else {
            [42, 54, 66, 255]
        }
    });

    // 13: Blinking server rack / mainframe bay
    fill_tile(&mut buf, Tile::ServerRack, |x, y| {
        let n = noise_i32(x, y, 13, 5);
        if (8..56).contains(&x) && (y % 8 == 3 || y % 8 == 4) {
            let led = hash2(x / 4, y / 8, 99) % 7;
            if led == 0 {
                return [245, 90, 50, 255];
            } else if led == 1 || led == 2 {
                return [70, 235, 190, 255];
            } else if led == 3 {
                return [235, 185, 55, 255];
            }
        }
        let v = if y % 8 == 0 { 28 } else { 54 + n };
        [clamp_u8(v - 2), clamp_u8(v), clamp_u8(v + 6), 255]
    });

    // 14: Thermal radiator fin louvers
    fill_tile(&mut buf, Tile::RadiatorLouvers, |x, y| {
        let louver = y % 6;
        let n = noise_i32(x / 2, y, 14, 6);
        if louver < 3 {
            [clamp_u8(168 + n), clamp_u8(74 + n), clamp_u8(48), 255]
        } else {
            [clamp_u8(62 + n), clamp_u8(36 + n), clamp_u8(32), 255]
        }
    });

    // 15: Spacecraft hull armor plating
    fill_tile(&mut buf, Tile::ShipArmor, |x, y| {
        let n = noise_i32(x / 2, y / 2, 15, 7);
        let mut v = 166 + n;
        if x % 32 == 0 || y % 16 == 0 {
            v -= 38;
        }
        if (28..34).contains(&y) && x > 8 && x < 56 {
            return [clamp_u8(196 + n), clamp_u8(138 + n), clamp_u8(44), 255];
        }
        [clamp_u8(v - 4), clamp_u8(v), clamp_u8(v + 6), 255]
    });

    buf
}
