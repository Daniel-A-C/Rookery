//! Shinjuku Rain v2.0: Monospace Character-Cell Japanese City Street in a Heavy Downpour.
//!
//! Shares the strict ANSI roguelike + WGSL atmospheric shader aesthetic of
//! *The Living ASCII Rookery*, upgraded with solid architectural occlusion,
//! a Tokyo Tower skyline, overhead viaduct with pantograph sparks, full-width
//! Kabukicho storefronts (Ramen/Izakaya, vertical neon alley, 24H Bar, Vending
//! alcove, Traffic Signal crosswalk, and 24H Konbini), smart vehicles, umbrella
//! pedestrians with puddle reflections, stray alley cats, and forked lightning.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::shinjuku::{
    RainLayer, ShinjukuInteraction, ShinjukuPalette, ShinjukuRainState, TrafficLightState,
    VehicleKind,
};
use wgpu::util::DeviceExt;

use crate::atlas::GlyphPass;
use crate::audio::{AudioEngine, SoundEffect};
use crate::{Gpu, PointerAction, Scene, SceneInput, fullscreen};

const MAX_INSTANCES: usize = 48_000;
const CELL_W: f32 = 9.6;
const CELL_H: f32 = 16.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BgUniforms {
    resolution: [f32; 2],
    time: f32,
    palette_id: f32,
    lightning: f32,
    lightning_x: f32,
    signs_lit: f32,
    wind: f32,
    prev_palette_id: f32,
    palette_blend: f32,
    rain_intensity: f32,
    traffic_red: f32,
    train_light: [f32; 4],
    store_fx: [f32; 4],
    pointer: [f32; 4],
    vehicles: [[f32; 4]; 3],
    ripples: [[f32; 4]; 4],
}

#[derive(Clone, Copy)]
struct PaletteColors {
    skyline: [f32; 4],
    skyline_fill: [f32; 4],
    facade_fill: [f32; 4],
    window_warm: [f32; 4],
    window_cool: [f32; 4],
    neon_a: [f32; 4],
    neon_b: [f32; 4],
    neon_c: [f32; 4],
    konbini: [f32; 4],
    rain_far: [f32; 4],
    rain_mid: [f32; 4],
    rain_near: [f32; 4],
    splash: [f32; 4],
    steel: [f32; 4],
    wood: [f32; 4],
}

fn palette_colors(palette: ShinjukuPalette) -> PaletteColors {
    match palette {
        ShinjukuPalette::ShinjukuNeon => PaletteColors {
            skyline: [0.18, 0.18, 0.30, 0.94],
            skyline_fill: [0.022, 0.020, 0.048, 0.92],
            facade_fill: [0.018, 0.016, 0.036, 0.97],
            window_warm: [0.98, 0.78, 0.34, 0.94],
            window_cool: [0.34, 0.84, 0.98, 0.88],
            neon_a: [0.98, 0.22, 0.56, 0.98],
            neon_b: [0.16, 0.90, 0.98, 0.98],
            neon_c: [0.98, 0.68, 0.18, 0.98],
            konbini: [0.46, 0.96, 0.86, 0.96],
            rain_far: [0.32, 0.44, 0.64, 0.42],
            rain_mid: [0.52, 0.70, 0.90, 0.66],
            rain_near: [0.78, 0.92, 1.00, 0.88],
            splash: [0.66, 0.88, 1.00, 0.84],
            steel: [0.36, 0.40, 0.52, 0.95],
            wood: [0.62, 0.42, 0.26, 0.96],
        },
        ShinjukuPalette::KyotoGion => PaletteColors {
            skyline: [0.18, 0.20, 0.28, 0.94],
            skyline_fill: [0.020, 0.022, 0.038, 0.92],
            facade_fill: [0.022, 0.018, 0.028, 0.97],
            window_warm: [0.98, 0.78, 0.34, 0.95],
            window_cool: [0.46, 0.76, 0.72, 0.84],
            neon_a: [0.98, 0.30, 0.20, 0.98],
            neon_b: [0.30, 0.84, 0.74, 0.96],
            neon_c: [0.98, 0.80, 0.28, 0.98],
            konbini: [0.94, 0.86, 0.60, 0.95],
            rain_far: [0.34, 0.44, 0.56, 0.42],
            rain_mid: [0.54, 0.70, 0.80, 0.66],
            rain_near: [0.80, 0.92, 0.96, 0.86],
            splash: [0.70, 0.88, 0.94, 0.82],
            steel: [0.38, 0.40, 0.46, 0.95],
            wood: [0.68, 0.46, 0.28, 0.96],
        },
        ShinjukuPalette::MidnightDownpour => PaletteColors {
            skyline: [0.16, 0.22, 0.32, 0.94],
            skyline_fill: [0.016, 0.022, 0.036, 0.92],
            facade_fill: [0.014, 0.020, 0.032, 0.97],
            window_warm: [0.96, 0.78, 0.42, 0.90],
            window_cool: [0.56, 0.80, 0.98, 0.90],
            neon_a: [0.60, 0.82, 1.00, 0.96],
            neon_b: [0.84, 0.92, 1.00, 0.96],
            neon_c: [0.96, 0.74, 0.34, 0.96],
            konbini: [0.78, 0.92, 1.00, 0.96],
            rain_far: [0.36, 0.48, 0.66, 0.44],
            rain_mid: [0.60, 0.76, 0.94, 0.68],
            rain_near: [0.86, 0.95, 1.00, 0.88],
            splash: [0.76, 0.90, 1.00, 0.84],
            steel: [0.40, 0.46, 0.56, 0.95],
            wood: [0.54, 0.46, 0.38, 0.95],
        },
        ShinjukuPalette::CyberTerminal => PaletteColors {
            skyline: [0.08, 0.30, 0.16, 0.94],
            skyline_fill: [0.006, 0.022, 0.010, 0.94],
            facade_fill: [0.005, 0.018, 0.008, 0.98],
            window_warm: [0.98, 0.76, 0.20, 0.94],
            window_cool: [0.24, 0.96, 0.48, 0.92],
            neon_a: [0.22, 0.98, 0.46, 0.98],
            neon_b: [0.98, 0.74, 0.18, 0.98],
            neon_c: [0.46, 0.98, 0.64, 0.98],
            konbini: [0.56, 1.00, 0.70, 0.96],
            rain_far: [0.12, 0.48, 0.24, 0.44],
            rain_mid: [0.22, 0.76, 0.38, 0.68],
            rain_near: [0.44, 0.98, 0.60, 0.88],
            splash: [0.38, 0.96, 0.56, 0.86],
            steel: [0.18, 0.56, 0.28, 0.95],
            wood: [0.36, 0.70, 0.30, 0.95],
        },
    }
}

#[inline]
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn lerp_palette_colors(a: PaletteColors, b: PaletteColors, t: f32) -> PaletteColors {
    let s = t.clamp(0.0, 1.0);
    if s <= 0.0 {
        return a;
    }
    if s >= 1.0 {
        return b;
    }
    let smooth = s * s * (3.0 - 2.0 * s);
    PaletteColors {
        skyline: lerp4(a.skyline, b.skyline, smooth),
        skyline_fill: lerp4(a.skyline_fill, b.skyline_fill, smooth),
        facade_fill: lerp4(a.facade_fill, b.facade_fill, smooth),
        window_warm: lerp4(a.window_warm, b.window_warm, smooth),
        window_cool: lerp4(a.window_cool, b.window_cool, smooth),
        neon_a: lerp4(a.neon_a, b.neon_a, smooth),
        neon_b: lerp4(a.neon_b, b.neon_b, smooth),
        neon_c: lerp4(a.neon_c, b.neon_c, smooth),
        konbini: lerp4(a.konbini, b.konbini, smooth),
        rain_far: lerp4(a.rain_far, b.rain_far, smooth),
        rain_mid: lerp4(a.rain_mid, b.rain_mid, smooth),
        rain_near: lerp4(a.rain_near, b.rain_near, smooth),
        splash: lerp4(a.splash, b.splash, smooth),
        steel: lerp4(a.steel, b.steel, smooth),
        wood: lerp4(a.wood, b.wood, smooth),
    }
}

/// GPU scene for *Shinjuku Rain*.
pub struct ShinjukuRainScene {
    state: Arc<Mutex<ShinjukuRainState>>,
    audio: Option<AudioEngine>,
    width: u32,
    height: u32,
    scale_factor: f64,
    bg_pipeline: wgpu::RenderPipeline,
    bg_uniforms: wgpu::Buffer,
    bg_bind_group: wgpu::BindGroup,
    glyphs: GlyphPass,
}

impl ShinjukuRainScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        state: Arc<Mutex<ShinjukuRainState>>,
    ) -> Self {
        let device = &gpu.device;
        let bg_layout = fullscreen::uniform_layout(device, "shinjuku-bg-layout");
        let bg_pipeline = fullscreen::pipeline(
            device,
            "shinjuku-bg-pipeline",
            include_str!("shinjuku_bg.wgsl"),
            format,
            Some(&bg_layout),
            None,
        );
        let init_uniforms = BgUniforms {
            resolution: [1280.0, 720.0],
            time: 0.0,
            palette_id: 0.0,
            lightning: 0.0,
            lightning_x: 0.5,
            signs_lit: 1.0,
            wind: 0.25,
            prev_palette_id: 0.0,
            palette_blend: 1.0,
            rain_intensity: 1.0,
            traffic_red: 0.0,
            train_light: [0.0; 4],
            store_fx: [0.0, 1.0, 0.0, 0.0],
            pointer: [0.0; 4],
            vehicles: [[-1.0, 0.0, 1.0, 0.0]; 3],
            ripples: [[0.0; 4]; 4],
        };
        let bg_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shinjuku-bg-uniforms"),
            contents: bytemuck::bytes_of(&init_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shinjuku-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms.as_entire_binding(),
            }],
        });

        let glyphs = GlyphPass::new(gpu, format, MAX_INSTANCES);

        Self {
            state,
            audio: None,
            width: 1280,
            height: 720,
            scale_factor: 1.0,
            bg_pipeline,
            bg_uniforms,
            bg_bind_group,
            glyphs,
        }
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn grid_dims(&self) -> (i32, i32) {
        let w = (self.width as f32 / self.scale_factor as f32).max(320.0);
        let h = (self.height as f32 / self.scale_factor as f32).max(240.0);
        let cols = (w / CELL_W).floor().max(40.0) as i32;
        let rows = (h / CELL_H).floor().max(24.0) as i32;
        (cols, rows)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_cell(
        &mut self,
        col: i32,
        row: i32,
        ch: char,
        color: [f32; 4],
        depth: f32,
        sway: f32,
        rotation: f32,
    ) {
        let scale = self.scale_factor as f32;
        let cw = CELL_W * scale;
        let ch_px = CELL_H * scale;
        let x = (col as f32 + 0.5) * cw;
        let y = (row as f32 + 0.5) * ch_px;
        self.glyphs
            .push(ch, [x, y], [cw, ch_px], rotation, depth, color, sway);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_str(&mut self, col: i32, row: i32, s: &str, color: [f32; 4], depth: f32, sway: f32) {
        for (i, ch) in s.chars().enumerate() {
            if ch != ' ' {
                self.push_cell(col + i as i32, row, ch, color, depth, sway, 0.0);
            }
        }
    }

    /// Emit opaque dark cell backing across `[c0..=c1] x [r0..=r1]` so foreground
    /// architecture, train cars, and storefronts cleanly occlude layers behind them.
    fn fill_rect_bg(&mut self, c0: i32, r0: i32, c1: i32, r1: i32, color: [f32; 4], depth: f32) {
        let scale = self.scale_factor as f32;
        let cw = CELL_W * scale;
        let ch_px = CELL_H * scale;
        // In the 32x32 SDF atlas cell, the 16x32 '█' bitmap occupies the middle 50% horizontally,
        // so scaling width by ~2.12x produces a seamless, stripe-free opaque cell fill.
        let pad_w = cw * 2.14;
        let pad_h = ch_px * 1.06;
        for r in r0..=r1 {
            let y = (r as f32 + 0.5) * ch_px;
            for c in c0..=c1 {
                let x = (c as f32 + 0.5) * cw;
                self.glyphs
                    .push('█', [x, y], [pad_w, pad_h], 0.0, depth, color, 0.0);
            }
        }
    }

    fn build_ascii_frame(&mut self, snapshot: &ShinjukuRainState, bar_flicker: f32) {
        self.glyphs.clear();
        let (cols, rows) = self.grid_dims();
        let ground_row = ((rows as f32) * 0.76).round() as i32;
        let viaduct_row = ((rows as f32) * 0.30).round() as i32;
        let pal = lerp_palette_colors(
            palette_colors(snapshot.prev_palette),
            palette_colors(snapshot.palette),
            snapshot.palette_blend,
        );
        let t = snapshot.time as f32;

        // 0. Forked Lightning Bolt in Upper Sky (behind skyline)
        if snapshot.lightning_flash > 0.42 {
            let bolt_alpha = ((snapshot.lightning_flash - 0.42) / 0.58).clamp(0.0, 1.0);
            let bolt_col = [0.84, 0.92, 1.00, 0.92 * bolt_alpha];
            let mut bc = ((snapshot.lightning_x / 1000.0) * cols as f32).round() as i32;
            let mut h = snapshot.lightning_seed.wrapping_mul(2_654_435_761);
            let bolt_end = (viaduct_row - 1).max(6);
            for r in 1..bolt_end {
                h = h.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let step = match (h >> 24) % 3 {
                    0 => -1,
                    1 => 1,
                    _ => 0,
                };
                let ch = if step < 0 {
                    '╱'
                } else if step > 0 {
                    '╲'
                } else {
                    '│'
                };
                self.push_cell(bc, r, ch, bolt_col, 8.0, 0.0, 0.0);
                // Side fork branch mid-bolt
                if r == bolt_end / 2 {
                    self.push_cell(
                        bc + 1,
                        r + 1,
                        '╲',
                        [0.76, 0.86, 1.0, 0.65 * bolt_alpha],
                        8.0,
                        0.0,
                        0.0,
                    );
                    self.push_cell(
                        bc + 2,
                        r + 2,
                        '╲',
                        [0.72, 0.82, 1.0, 0.45 * bolt_alpha],
                        8.0,
                        0.0,
                        0.0,
                    );
                }
                bc = (bc + step).clamp(3, cols - 4);
            }
        }

        // 1a. Distant Tokyo Tower / Telecom Lattice Spire (far background)
        let tower_col = ((cols as f32) * 0.75).round() as i32;
        let tower_top = (ground_row - 30).max(2);
        let tower_red = [0.88, 0.24, 0.26, 0.78];
        let tower_white = [0.82, 0.86, 0.94, 0.68];
        for r in tower_top..(viaduct_row + 4).min(ground_row) {
            let prog = (r - tower_top) as f32 / 24.0;
            let band_col = if (r / 3) % 2 == 0 {
                tower_red
            } else {
                tower_white
            };
            if prog < 0.22 {
                self.push_cell(tower_col, r, '│', band_col, 9.0, 0.0, 0.0);
            } else if prog < 0.55 {
                self.push_cell(tower_col - 1, r, '╱', band_col, 9.0, 0.0, 0.0);
                self.push_cell(tower_col + 1, r, '╲', band_col, 9.0, 0.0, 0.0);
                if r % 3 == 0 {
                    self.push_cell(tower_col, r, '─', band_col, 9.0, 0.0, 0.0);
                }
            } else {
                let spread = 2 + ((prog - 0.55) * 4.0) as i32;
                self.push_cell(tower_col - spread, r, '╱', band_col, 9.0, 0.0, 0.0);
                self.push_cell(tower_col + spread, r, '╲', band_col, 9.0, 0.0, 0.0);
            }
        }
        if ((t * 2.6).sin()) > 0.1 {
            self.push_cell(
                tower_col,
                (tower_top - 1).max(1),
                '✦',
                [1.0, 0.24, 0.24, 0.98],
                9.0,
                0.0,
                0.0,
            );
        }

        // 1b. Layered Tokyo Skyline with Solid Silhouette Backing & Lit Windows
        let bldg_specs: [(f32, f32, i32); 10] = [
            (0.00, 0.10, 19),
            (0.11, 0.21, 25),
            (0.22, 0.32, 16),
            (0.33, 0.44, 28),
            (0.45, 0.55, 21),
            (0.56, 0.66, 26),
            (0.67, 0.74, 17),
            (0.77, 0.85, 24),
            (0.86, 0.94, 20),
            (0.95, 1.00, 15),
        ];
        for (idx, (x0, x1, h_rows)) in bldg_specs.iter().enumerate() {
            let c0 = ((*x0) * cols as f32) as i32;
            let c1 = (((*x1) * cols as f32) as i32).min(cols - 1);
            if c1 <= c0 + 1 {
                continue;
            }
            let top = (ground_row - h_rows).max(2);

            // Solid dark silhouette fill so clouds & lightning don't bleed through towers
            self.fill_rect_bg(c0, top, c1, ground_row - 1, pal.skyline_fill, 11.0);

            // Roofline, water tank or blinking red aviation beacon
            self.push_cell(c0, top, '┌', pal.skyline, 12.0, 0.0, 0.0);
            for c in (c0 + 1)..c1 {
                self.push_cell(c, top, '─', pal.skyline, 12.0, 0.0, 0.0);
            }
            self.push_cell(c1, top, '┐', pal.skyline, 12.0, 0.0, 0.0);

            if idx % 2 == 1 {
                let mid = (c0 + c1) / 2;
                self.push_cell(mid, top - 1, '│', pal.skyline, 12.0, 0.0, 0.0);
                if (t * 2.2 + idx as f32).sin() > 0.2 {
                    self.push_cell(mid, top - 2, '*', [0.98, 0.22, 0.24, 0.96], 12.0, 0.0, 0.0);
                }
            } else if c1 - c0 >= 6 {
                // Rooftop water tank / AC bulkhead
                self.push_str(c0 + 2, top - 1, "┌─┐", pal.skyline, 12.0, 0.0);
            }

            for r in (top + 1)..ground_row {
                self.push_cell(c0, r, '│', pal.skyline, 12.0, 0.0, 0.0);
                self.push_cell(c1, r, '│', pal.skyline, 12.0, 0.0, 0.0);
                for c in (c0 + 1)..c1 {
                    if (c - c0) % 2 == 1 && (r - top) % 2 == 1 {
                        let hash = ((c * 31 + r * 17 + idx as i32 * 13) & 7) as usize;
                        if hash < 4 {
                            let mut w_col = if hash.is_multiple_of(2) {
                                pal.window_warm
                            } else {
                                pal.window_cool
                            };
                            w_col[3] *= if hash == 0 { 0.88 } else { 0.56 };
                            let ch = if hash == 0 { '▪' } else { '░' };
                            self.push_cell(c, r, ch, w_col, 13.0, 0.0, 0.0);
                        }
                    }
                }
            }
        }

        // 2. Overhead Steel Rail Viaduct, Catenary Wire & Passing Commuter Train
        // Overhead catenary power wire above the train
        let catenary_row = (viaduct_row - 3).max(2);
        for c in 0..cols {
            let ch = if c % 16 == 0 { '┬' } else { '─' };
            self.push_cell(
                c,
                catenary_row,
                ch,
                [pal.steel[0], pal.steel[1], pal.steel[2], 0.48],
                21.0,
                0.0,
                0.0,
            );
        }
        // Steel viaduct support pillars & solid girder deck
        for c in 0..cols {
            if c % 18 == 9 {
                for r in (viaduct_row + 3)..ground_row {
                    self.push_cell(c, r, '║', pal.steel, 22.0, 0.0, 0.0);
                }
            }
        }
        self.fill_rect_bg(
            0,
            viaduct_row,
            cols - 1,
            viaduct_row + 2,
            pal.facade_fill,
            23.0,
        );
        for c in 0..cols {
            self.push_cell(c, viaduct_row, '═', pal.steel, 24.0, 0.0, 0.0);
            self.push_cell(c, viaduct_row + 2, '═', pal.steel, 24.0, 0.0, 0.0);
            let truss = match c % 4 {
                0 => '│',
                1 => '╱',
                2 => '╲',
                _ => '│',
            };
            self.push_cell(c, viaduct_row + 1, truss, pal.steel, 24.0, 0.0, 0.0);
        }

        // Elevated Commuter Train (solid occluded car bodies + glowing warm windows)
        if let Some(ref train) = snapshot.train {
            let car_cols = 12i32;
            let base_col = ((train.x / 1000.0) * cols as f32).round() as i32;
            let stripe_col = pal.neon_b;
            for car in 0..train.car_count as i32 {
                let cc = base_col + car * (car_cols + 1);
                if cc + car_cols < -2 || cc > cols + 2 {
                    continue;
                }
                self.fill_rect_bg(
                    cc,
                    viaduct_row - 2,
                    cc + car_cols - 1,
                    viaduct_row,
                    pal.facade_fill,
                    25.0,
                );
                self.push_str(cc, viaduct_row - 2, "┌──────────┐", pal.steel, 26.0, 0.0);
                self.push_str(cc, viaduct_row - 1, "│          │", pal.steel, 26.0, 0.0);
                self.push_str(
                    cc + 2,
                    viaduct_row - 1,
                    "▪▪ ▪▪ ▪▪",
                    pal.window_warm,
                    27.0,
                    0.0,
                );
                self.push_str(cc, viaduct_row, "└─o──────o─┘", stripe_col, 26.0, 0.0);

                // Coupler between cars
                if car + 1 < train.car_count as i32 {
                    self.push_cell(
                        cc + car_cols,
                        viaduct_row - 1,
                        '═',
                        pal.steel,
                        26.0,
                        0.0,
                        0.0,
                    );
                }

                // Cab headlight & rear red tail markers
                let is_lead = (train.vx > 0.0 && car == train.car_count as i32 - 1)
                    || (train.vx < 0.0 && car == 0);
                let is_tail = (train.vx > 0.0 && car == 0)
                    || (train.vx < 0.0 && car == train.car_count as i32 - 1);
                if is_lead {
                    if train.vx > 0.0 {
                        self.push_str(
                            cc + car_cols,
                            viaduct_row - 1,
                            ">>",
                            [1.0, 0.96, 0.74, 0.85],
                            27.0,
                            0.0,
                        );
                    } else {
                        self.push_str(
                            cc - 2,
                            viaduct_row - 1,
                            "<<",
                            [1.0, 0.96, 0.74, 0.85],
                            27.0,
                            0.0,
                        );
                    }
                }
                if is_tail {
                    let tail_c = if train.vx > 0.0 {
                        cc - 1
                    } else {
                        cc + car_cols
                    };
                    self.push_cell(
                        tail_c,
                        viaduct_row - 1,
                        '●',
                        [1.0, 0.20, 0.22, 0.95],
                        27.0,
                        0.0,
                        0.0,
                    );
                }
            }
        }
        // Pantograph electrical spark on catenary wire
        if snapshot.train_spark_flash > 0.12 {
            let sc = ((snapshot.train_spark_x / 1000.0) * cols as f32).round() as i32;
            let sp_col = [0.76, 0.96, 1.0, snapshot.train_spark_flash];
            self.push_cell(sc, catenary_row + 1, '✦', sp_col, 28.0, 0.0, 0.0);
            self.push_cell(sc - 1, catenary_row + 1, '*', sp_col, 28.0, 0.0, 0.0);
            self.push_cell(sc + 1, catenary_row + 1, '*', sp_col, 28.0, 0.0, 0.0);
        }

        // 3. Full-Width Kabukicho Street Storefronts (with Solid Facade Occlusion!)
        let sign_alpha = if snapshot.signs_lit { 1.0 } else { 0.26 };
        let mut na = pal.neon_a;
        na[3] *= sign_alpha;
        let mut nb = pal.neon_b;
        nb[3] *= sign_alpha;
        let mut nc = pal.neon_c;
        nc[3] *= sign_alpha;
        let mut nk = pal.konbini;
        nk[3] *= sign_alpha;

        // 3a. Ramen & Izakaya Shop (left: ~8%..28%)
        let ramen_col = ((cols as f32) * 0.085).round() as i32;
        self.fill_rect_bg(
            ramen_col,
            ground_row - 7,
            ramen_col + 18,
            ground_row - 1,
            pal.facade_fill,
            32.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 7,
            "╔═════════════════╗",
            pal.wood,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 6,
            "║  RAMEN ラメン 酒  ║",
            nc,
            35.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 5,
            "╠═════════════════╣",
            pal.wood,
            34.0,
            0.0,
        );
        // Swaying noren curtains under awning
        self.push_str(
            ramen_col,
            ground_row - 4,
            "│▌_▌_▌_▌_▌_▌_▌_▌_│",
            na,
            35.0,
            0.30,
        );
        self.push_str(
            ramen_col,
            ground_row - 3,
            "│ ░░░░░░░░░░░░░░░ │",
            pal.window_warm,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 2,
            "│ ▒▒▒  OPEN   ▒▒▒ │",
            pal.window_warm,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 1,
            "└───┴─────────┴───┘",
            pal.wood,
            34.0,
            0.0,
        );
        // Swaying red chochin paper lanterns
        self.push_str(ramen_col - 3, ground_row - 5, "(●)", na, 36.0, 0.70);
        self.push_str(ramen_col + 19, ground_row - 5, "(●)", na, 36.0, 0.70);

        // 3b. Alley Stairwell & Vertical Hanging Neon Signboards (~31%..36%)
        let alley_col = ((cols as f32) * 0.31).round() as i32;
        self.fill_rect_bg(
            alley_col,
            ground_row - 11,
            alley_col + 4,
            ground_row - 1,
            pal.facade_fill,
            32.0,
        );
        self.push_str(alley_col, ground_row - 11, "╔═══╗", na, 34.0, 0.0);
        self.push_str(alley_col, ground_row - 10, "║ 呑 ║", na, 35.0, 0.0);
        self.push_str(alley_col, ground_row - 9, "║ 酒 ║", nc, 35.0, 0.0);
        self.push_str(alley_col, ground_row - 8, "║ 2 ║", nb, 35.0, 0.0);
        self.push_str(alley_col, ground_row - 7, "║ 4 ║", nb, 35.0, 0.0);
        self.push_str(alley_col, ground_row - 6, "║ H ║", nb, 35.0, 0.0);
        self.push_str(alley_col, ground_row - 5, "╚═══╝", na, 34.0, 0.0);
        self.push_str(alley_col, ground_row - 4, "│[=]│", pal.steel, 33.0, 0.0);
        self.push_str(
            alley_col,
            ground_row - 3,
            "│░░░│",
            pal.window_warm,
            33.0,
            0.0,
        );
        self.push_str(alley_col, ground_row - 2, "││ ││", pal.steel, 33.0, 0.0);
        self.push_str(alley_col, ground_row - 1, "┴┴─┴┴", pal.steel, 33.0, 0.0);

        // 3c. Neon Arcade / Bar 24H Facade (center: ~39%..53%)
        let bar_col = ((cols as f32) * 0.39).round() as i32;
        let mut bar_na = na;
        bar_na[3] *= bar_flicker;
        self.fill_rect_bg(
            bar_col,
            ground_row - 10,
            bar_col + 14,
            ground_row - 1,
            pal.facade_fill,
            32.0,
        );
        self.push_str(
            bar_col,
            ground_row - 10,
            "╔═════════════╗",
            bar_na,
            34.0,
            0.0,
        );
        self.push_str(
            bar_col,
            ground_row - 9,
            "║   BAR 24H   ║",
            bar_na,
            35.0,
            0.0,
        );
        self.push_str(bar_col, ground_row - 8, "║  SHIN·JUKU  ║", nb, 35.0, 0.0);
        // Animated marquee chase dots
        let chase = ((t * 5.0) as i32).rem_euclid(3);
        let marquee = match chase {
            0 => "║ ·  ✦   ·  ✦ ║",
            1 => "║  ·  ✦   ·  ✦║",
            _ => "║✦  ·   ✦  ·  ║",
        };
        self.push_str(bar_col, ground_row - 7, marquee, nc, 35.0, 0.0);
        self.push_str(
            bar_col,
            ground_row - 6,
            "╚═════════════╝",
            bar_na,
            34.0,
            0.0,
        );
        self.push_str(
            bar_col + 1,
            ground_row - 5,
            "│ ▓▓▓▓▓▓▓▓▓ │",
            pal.steel,
            33.0,
            0.0,
        );
        self.push_str(
            bar_col + 1,
            ground_row - 4,
            "│ ░░░░░░░░░ │",
            pal.steel,
            33.0,
            0.0,
        );
        self.push_str(bar_col + 1, ground_row - 3, "│  ┌─────┐  │", nc, 33.0, 0.0);
        self.push_str(bar_col + 1, ground_row - 2, "│  │ ███ │  │", nc, 33.0, 0.0);
        self.push_str(
            bar_col + 1,
            ground_row - 1,
            "└──┴─────┴──┘",
            pal.steel,
            33.0,
            0.0,
        );

        // 3d. Crosswalk Traffic Signal Post (~58.5%)
        let sig_col = ((cols as f32) * 0.585).round() as i32;
        for r in (ground_row - 7)..ground_row {
            self.push_cell(sig_col, r, '│', pal.steel, 36.0, 0.0, 0.0);
        }
        self.fill_rect_bg(
            sig_col - 1,
            ground_row - 9,
            sig_col + 1,
            ground_row - 7,
            pal.facade_fill,
            36.0,
        );
        self.push_str(sig_col - 1, ground_row - 9, "┌─┐", pal.steel, 37.0, 0.0);
        let (lamp_col, lamp_row) = match snapshot.traffic_light {
            TrafficLightState::Green => ([0.20, 0.98, 0.52, 0.98], ground_row - 7),
            TrafficLightState::Yellow => ([0.98, 0.84, 0.18, 0.98], ground_row - 8),
            TrafficLightState::Red => ([1.00, 0.22, 0.24, 0.98], ground_row - 8),
        };
        self.push_str(sig_col - 1, ground_row - 8, "│▫│", pal.steel, 37.0, 0.0);
        self.push_str(sig_col - 1, ground_row - 7, "└─┘", pal.steel, 37.0, 0.0);
        self.push_cell(sig_col, lamp_row, '●', lamp_col, 38.0, 0.0, 0.0);

        // 3e. Glowing Vending Machine Alcove (~64%..74%)
        let vend_col = ((cols as f32) * 0.645).round() as i32;
        let vend_flash = 1.0 + snapshot.vending_pulse * 0.5;
        let mut vend_nb = nb;
        vend_nb[0] = (vend_nb[0] * vend_flash).min(1.0);
        vend_nb[1] = (vend_nb[1] * vend_flash).min(1.0);
        vend_nb[2] = (vend_nb[2] * vend_flash).min(1.0);
        for vm in 0..2 {
            let vc = vend_col + vm * 6;
            self.fill_rect_bg(
                vc,
                ground_row - 5,
                vc + 4,
                ground_row - 1,
                pal.facade_fill,
                32.0,
            );
            self.push_str(vc, ground_row - 5, "┌───┐", vend_nb, 34.0, 0.0);
            self.push_str(vc, ground_row - 4, "│▪▪▪│", vend_nb, 35.0, 0.0);
            self.push_str(vc, ground_row - 3, "│▪▪▪│", nc, 35.0, 0.0);
            self.push_str(vc, ground_row - 2, "│═══│", pal.steel, 35.0, 0.0);
            self.push_str(vc, ground_row - 1, "└───┘", pal.steel, 34.0, 0.0);
        }

        // Curved sodium streetlamp between vending machines and Konbini
        let lamp_c = ((cols as f32) * 0.755).round() as i32;
        for r in (ground_row - 9)..ground_row {
            self.push_cell(lamp_c, r, '│', pal.steel, 35.0, 0.0, 0.0);
        }
        self.push_str(lamp_c, ground_row - 10, "┌─●", pal.window_warm, 36.0, 0.0);

        // 3f. 24H Konbini Convenience Store (far right: ~79%..96%)
        let konbini_col = ((cols as f32) * 0.79).round() as i32;
        let konbini_w = (cols - konbini_col - 2).clamp(14, 22);
        self.fill_rect_bg(
            konbini_col,
            ground_row - 7,
            konbini_col + konbini_w,
            ground_row - 1,
            pal.facade_fill,
            32.0,
        );
        // Bright fascia stripes & glowing interior glass
        self.push_cell(konbini_col, ground_row - 7, '╔', nk, 34.0, 0.0, 0.0);
        self.push_cell(
            konbini_col + konbini_w,
            ground_row - 7,
            '╗',
            nk,
            34.0,
            0.0,
            0.0,
        );
        for c in (konbini_col + 1)..(konbini_col + konbini_w) {
            self.push_cell(c, ground_row - 7, '═', nk, 34.0, 0.0, 0.0);
            self.push_cell(c, ground_row - 5, '═', na, 34.0, 0.0, 0.0);
        }
        self.push_cell(konbini_col, ground_row - 6, '║', nk, 34.0, 0.0, 0.0);
        self.push_cell(
            konbini_col + konbini_w,
            ground_row - 6,
            '║',
            nk,
            34.0,
            0.0,
            0.0,
        );
        self.push_str(
            konbini_col + 2,
            ground_row - 6,
            "KONBINI 24H",
            nk,
            35.0,
            0.0,
        );
        self.push_cell(konbini_col, ground_row - 5, '╠', nk, 34.0, 0.0, 0.0);
        self.push_cell(
            konbini_col + konbini_w,
            ground_row - 5,
            '╣',
            nk,
            34.0,
            0.0,
            0.0,
        );
        for r in (ground_row - 4)..=(ground_row - 1) {
            self.push_cell(konbini_col, r, '│', pal.steel, 34.0, 0.0, 0.0);
            self.push_cell(konbini_col + konbini_w, r, '│', pal.steel, 34.0, 0.0, 0.0);
            for c in (konbini_col + 1)..(konbini_col + konbini_w) {
                let is_door = c >= konbini_col + 2 && c <= konbini_col + 5;
                if is_door {
                    let dch = if r == ground_row - 4 { '─' } else { '▒' };
                    self.push_cell(c, r, dch, pal.window_warm, 34.0, 0.0, 0.0);
                } else {
                    let sch = if r == ground_row - 2 { '▪' } else { '░' };
                    self.push_cell(c, r, sch, nk, 34.0, 0.0, 0.0);
                }
            }
        }

        // 3g. Utility Poles & Overhead Catenary Power Lines
        let pole1 = ((cols as f32) * 0.045).round() as i32;
        let pole2 = ((cols as f32) * 0.965).round() as i32;
        for r in (viaduct_row - 5)..ground_row {
            self.push_cell(pole1, r, '│', pal.wood, 38.0, 0.0, 0.0);
            self.push_cell(pole2, r, '│', pal.wood, 38.0, 0.0, 0.0);
        }
        self.push_str(pole1 - 2, viaduct_row - 4, "──┼──", pal.steel, 38.0, 0.0);
        self.push_str(pole2 - 2, viaduct_row - 4, "──┼──", pal.steel, 38.0, 0.0);
        for c in (pole1 + 1)..pole2 {
            let frac = (c - pole1) as f32 / (pole2 - pole1).max(1) as f32;
            let sag = (4.0 * frac * (1.0 - frac) * 3.0).round() as i32;
            self.push_cell(c, viaduct_row - 4 + sag, '~', pal.steel, 37.0, 0.35, 0.0);
        }

        // Curb & Sidewalk divider (with working '╤' drainage grates)
        for c in 0..cols {
            let ch = if c % 7 == 0 { '╤' } else { '═' };
            self.push_cell(c, ground_row, ch, pal.steel, 40.0, 0.0, 0.0);
        }

        // 4. Rising Steam & Awning Drips
        for s in &snapshot.steam {
            let sc = ((s.x / 1000.0) * cols as f32).round() as i32;
            let sr = ((s.y / 1000.0) * rows as f32).round() as i32;
            let life = 1.0 - (s.age / s.max_age).clamp(0.0, 1.0);
            let ch = if life > 0.65 {
                'S'
            } else if life > 0.3 {
                's'
            } else {
                '~'
            };
            self.push_cell(sc, sr, ch, [0.90, 0.94, 1.0, 0.48 * life], 44.0, 0.5, 0.0);
        }
        for d in &snapshot.drips {
            let dc = ((d.x / 1000.0) * cols as f32).round() as i32;
            let dr = ((d.y / 1000.0) * rows as f32).round() as i32;
            self.push_cell(dc, dr, '·', pal.rain_near, 45.0, 0.0, 0.0);
        }

        // 5. Alley Cats (sitting/blinking under awnings or darting across sidewalk)
        for cat in &snapshot.cats {
            let cc = ((cat.x / 1000.0) * cols as f32).round() as i32;
            let cr = (((cat.y / 1000.0) * rows as f32).round() as i32).min(ground_row - 1);
            let sprite = if cat.darting {
                if cat.facing_right { "=^.^>~" } else { "~<^.^=" }
            } else if ((t * 1.4 + cat.id as f32).sin()) > -0.82 {
                "/\\_/\\"
            } else {
                "/-_-\\"
            };
            self.push_str(cc - 2, cr, sprite, [0.98, 0.78, 0.48, 0.96], 50.0, 0.0);
        }

        // 6. Pedestrians with Wind-Tilted Umbrellas + Subtle Puddle Reflections
        let umbrella_palette = [
            [0.98, 0.28, 0.48, 0.96],
            [0.22, 0.90, 0.98, 0.96],
            [0.98, 0.84, 0.22, 0.96],
            [0.88, 0.94, 1.00, 0.92],
            [0.76, 0.52, 0.98, 0.96],
        ];
        let coat_palette = [
            [0.64, 0.70, 0.84, 0.94],
            [0.82, 0.72, 0.62, 0.94],
            [0.54, 0.68, 0.76, 0.94],
            [0.74, 0.64, 0.78, 0.94],
        ];
        for ped in &snapshot.pedestrians {
            let pc = ((ped.x / 1000.0) * cols as f32).round() as i32;
            let pr = (((ped.y / 1000.0) * rows as f32).round() as i32).min(ground_row - 1);
            let ucol = umbrella_palette[(ped.umbrella_color as usize) % umbrella_palette.len()];
            let ccol = coat_palette[(ped.coat_color as usize) % coat_palette.len()];
            let umb_tilt = if snapshot.wind_gust > 0.55 {
                "╱───╲"
            } else {
                ".---."
            };
            self.push_str(pc - 2, pr - 2, umb_tilt, ucol, 55.0, 0.18);
            self.push_cell(pc, pr - 1, 'o', [0.94, 0.94, 0.98, 0.96], 55.0, 0.0, 0.0);
            let legs = if ped.pause_secs > 0.0 {
                "||"
            } else if ped.walk_phase.sin() > 0.0 {
                "/\\"
            } else {
                "||"
            };
            self.push_str(pc, pr, legs, ccol, 55.0, 0.0);

            // Faint inverted umbrella reflection shimmering below the curb
            let refl_col = [ucol[0], ucol[1], ucol[2], 0.20];
            self.push_str(pc - 1, ground_row + 2, "~~~", refl_col, 48.0, 0.25);
        }

        // 7. Passing Vehicles (Tokyo Taxi, Kei Truck, Delivery Scooter)
        for v in &snapshot.vehicles {
            let vc = ((v.x / 1000.0) * cols as f32).round() as i32;
            let vr = ((v.y / 1000.0) * rows as f32).round() as i32;
            let dir_right = v.target_vx >= 0.0;
            let brake_col = if v.brake_glow > 0.3 {
                [1.00, 0.18, 0.18, 0.98]
            } else {
                [0.92, 0.24, 0.22, 0.84]
            };
            match v.kind {
                VehicleKind::Taxi => {
                    let body = [0.98, 0.80, 0.16, 0.97];
                    let beam = [1.00, 0.96, 0.72, 0.82];
                    self.fill_rect_bg(vc - 2, vr - 2, vc + 3, vr - 2, pal.facade_fill, 59.0);
                    self.fill_rect_bg(vc - 4, vr - 1, vc + 3, vr, pal.facade_fill, 59.0);
                    self.push_str(vc - 2, vr - 2, "┌TAXI┐", [1.0, 0.92, 0.45, 0.98], 60.0, 0.0);
                    self.push_str(vc - 4, vr - 1, "┌──▪▪──┐", body, 60.0, 0.0);
                    self.push_str(vc - 4, vr, "└─o──o─┘", body, 60.0, 0.0);
                    if dir_right {
                        self.push_str(vc + 4, vr, ">>>", beam, 60.0, 0.0);
                        self.push_cell(vc - 5, vr, '●', brake_col, 60.0, 0.0, 0.0);
                    } else {
                        self.push_str(vc - 7, vr, "<<<", beam, 60.0, 0.0);
                        self.push_cell(vc + 4, vr, '●', brake_col, 60.0, 0.0, 0.0);
                    }
                }
                VehicleKind::KeiTruck => {
                    let body = [0.84, 0.90, 0.96, 0.97];
                    let beam = [0.94, 0.98, 1.00, 0.80];
                    if dir_right {
                        self.fill_rect_bg(vc - 4, vr - 2, vc + 1, vr - 2, pal.facade_fill, 59.0);
                        self.fill_rect_bg(vc - 4, vr - 1, vc + 4, vr, pal.facade_fill, 59.0);
                        self.push_str(vc - 4, vr - 2, "┌────┐", body, 60.0, 0.0);
                        self.push_str(vc - 4, vr - 1, "│▓▓▓▓├──┐", body, 60.0, 0.0);
                        self.push_str(vc - 4, vr, "└─o───o─┘", body, 60.0, 0.0);
                        self.push_str(vc + 5, vr, ">>", beam, 60.0, 0.0);
                        self.push_cell(vc - 5, vr, '●', brake_col, 60.0, 0.0, 0.0);
                    } else {
                        self.fill_rect_bg(vc - 1, vr - 2, vc + 4, vr - 2, pal.facade_fill, 59.0);
                        self.fill_rect_bg(vc - 4, vr - 1, vc + 4, vr, pal.facade_fill, 59.0);
                        self.push_str(vc - 1, vr - 2, "┌────┐", body, 60.0, 0.0);
                        self.push_str(vc - 4, vr - 1, "┌──┤▓▓▓▓│", body, 60.0, 0.0);
                        self.push_str(vc - 4, vr, "└─o───o─┘", body, 60.0, 0.0);
                        self.push_str(vc - 6, vr, "<<", beam, 60.0, 0.0);
                        self.push_cell(vc + 5, vr, '●', brake_col, 60.0, 0.0, 0.0);
                    }
                }
                VehicleKind::Scooter => {
                    let body = pal.neon_a;
                    let beam = [1.00, 0.96, 0.78, 0.85];
                    if dir_right {
                        self.push_str(vc - 1, vr - 1, "o/", [0.92, 0.94, 0.98, 0.96], 60.0, 0.0);
                        self.push_str(vc - 2, vr, "o=o", body, 60.0, 0.0);
                        self.push_str(vc + 1, vr, ">>", beam, 60.0, 0.0);
                        self.push_cell(vc - 3, vr, '*', brake_col, 60.0, 0.0, 0.0);
                    } else {
                        self.push_str(vc - 1, vr - 1, "\\o", [0.92, 0.94, 0.98, 0.96], 60.0, 0.0);
                        self.push_str(vc - 1, vr, "o=o", body, 60.0, 0.0);
                        self.push_str(vc - 3, vr, "<<", beam, 60.0, 0.0);
                        self.push_cell(vc + 2, vr, '*', brake_col, 60.0, 0.0, 0.0);
                    }
                }
            }
        }

        // 8. Puddle Splashes on Wet Asphalt
        for sp in &snapshot.splashes {
            let sc = ((sp.x / 1000.0) * cols as f32).round() as i32;
            let sr = ((sp.y / 1000.0) * rows as f32).round() as i32;
            let frac = (sp.age / sp.max_age).clamp(0.0, 1.0);
            let mut scol = pal.splash;
            scol[3] *= 1.0 - frac;
            if frac < 0.25 {
                self.push_cell(sc, sr, 'v', scol, 65.0, 0.0, 0.0);
            } else if frac < 0.55 {
                self.push_str(sc - 1, sr, "(o)", scol, 65.0, 0.0);
            } else {
                self.push_str(sc - 2, sr, "( O )", scol, 65.0, 0.0);
            }
        }

        // 9. Multi-Depth Wind-Slanted ASCII Rain Streaks (scaled by rain_intensity)
        let slant_angle = -snapshot.wind_gust * 0.34;
        let active_drops = snapshot.active_raindrop_count();
        for drop in snapshot.raindrops.iter().take(active_drops) {
            let dc = ((drop.x / 1000.0) * cols as f32).round() as i32;
            let dr = ((drop.y / 1000.0) * rows as f32).round() as i32;
            let (ch, col, depth) = match drop.layer {
                RainLayer::Far => (':', pal.rain_far, 20.0),
                RainLayer::Mid => (
                    if snapshot.wind_gust > 0.48 {
                        '╲'
                    } else {
                        '│'
                    },
                    pal.rain_mid,
                    70.0,
                ),
                RainLayer::Near => ('│', pal.rain_near, 85.0),
            };
            self.push_cell(dc, dr, ch, col, depth, 0.0, slant_angle);
            if matches!(drop.layer, RainLayer::Near) {
                let mut tail_col = col;
                tail_col[3] *= 0.45;
                self.push_cell(dc, dr - 1, '│', tail_col, depth, 0.0, slant_angle);
            }
        }
    }
}

impl Scene for ShinjukuRainScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.scale_factor = scale_factor.max(1.0);
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let pointer_norm = input.pointer.map(|p| {
            let nx = (p.x / (self.width as f32).max(1.0)) * 1000.0;
            let ny = (p.y / (self.height as f32).max(1.0)) * 1000.0;
            [nx, ny]
        });

        let clicked = input.interactive && input.action == PointerAction::LeftClick;

        let snapshot = if let Ok(mut st) = self.state.lock() {
            if clicked && let Some([nx, ny]) = pointer_norm {
                let outcome = st.interact_at(nx, ny);
                if let Some(ref audio) = self.audio {
                    match outcome {
                        ShinjukuInteraction::Lightning => {
                            audio.play_sfx(SoundEffect::ThunderRumble);
                        }
                        ShinjukuInteraction::Train => {
                            audio.play_sfx(SoundEffect::TrainPass);
                        }
                        ShinjukuInteraction::Storefront
                        | ShinjukuInteraction::Vending
                        | ShinjukuInteraction::TrafficSignal => {
                            audio.play_sfx(SoundEffect::SignToggle);
                        }
                        ShinjukuInteraction::Cat | ShinjukuInteraction::Splash => {
                            audio.play_sfx(SoundEffect::RainSplash);
                        }
                    }
                }
            }
            st.update(dt.as_secs_f32(), input.interactive, pointer_norm);
            if let Some(ref audio) = self.audio {
                if st.ambient_train_sfx {
                    audio.play_sfx(SoundEffect::TrainPass);
                    st.ambient_train_sfx = false;
                }
                if st.ambient_thunder_sfx {
                    audio.play_sfx(SoundEffect::ThunderRumble);
                    st.ambient_thunder_sfx = false;
                }
            } else {
                st.ambient_train_sfx = false;
                st.ambient_thunder_sfx = false;
            }
            st.clone()
        } else {
            return Activity::Idle;
        };

        let t = snapshot.time as f32;
        let bar_flicker = if !snapshot.signs_lit {
            0.25
        } else if snapshot.bar_flicker_boost > 0.05 {
            if ((t * 28.0).sin()) > -0.2 {
                1.15
            } else {
                0.35
            }
        } else if (t * 7.0).sin() > -0.88 {
            1.0
        } else {
            0.32
        };

        let train_light = if let Some(ref tr) = snapshot.train {
            let center_x = (tr.x + (tr.car_count as f32) * 55.0) / 1000.0;
            [
                center_x,
                1.0,
                snapshot.train_spark_x / 1000.0,
                snapshot.train_spark_flash,
            ]
        } else {
            [
                0.5,
                0.0,
                snapshot.train_spark_x / 1000.0,
                snapshot.train_spark_flash,
            ]
        };

        let mut vehicle_slots = [[-1.0f32, 0.0, 1.0, 0.0]; 3];
        for (slot, v) in vehicle_slots.iter_mut().zip(snapshot.vehicles.iter()) {
            let dir = if v.target_vx >= 0.0 { 1.0 } else { -1.0 };
            *slot = [v.x / 1000.0, v.y / 1000.0, dir, v.brake_glow];
        }

        let mut ripple_slots = [[0.0f32; 4]; 4];
        for (slot, sp) in ripple_slots
            .iter_mut()
            .zip(snapshot.splashes.iter().rev().take(4))
        {
            let age_norm = (sp.age / sp.max_age.max(0.01)).clamp(0.0, 1.0);
            *slot = [sp.x / 1000.0, sp.y / 1000.0, age_norm, sp.scale.min(1.8)];
        }

        let pointer_uniform = match (snapshot.pointer_active, snapshot.pointer_pos) {
            (true, Some([px, py])) => [px / 1000.0, py / 1000.0, 1.0, 0.0],
            _ => [0.0, 0.0, 0.0, 0.0],
        };

        let bg_u = BgUniforms {
            resolution: [self.width as f32, self.height as f32],
            time: t,
            palette_id: snapshot.palette as u8 as f32,
            lightning: snapshot.lightning_flash,
            lightning_x: snapshot.lightning_x / 1000.0,
            signs_lit: if snapshot.signs_lit { 1.0 } else { 0.0 },
            wind: snapshot.wind_gust,
            prev_palette_id: snapshot.prev_palette as u8 as f32,
            palette_blend: snapshot.palette_blend,
            rain_intensity: snapshot.rain_intensity,
            traffic_red: match snapshot.traffic_light {
                TrafficLightState::Green => 0.0,
                TrafficLightState::Yellow => 0.5,
                TrafficLightState::Red => 1.0,
            },
            train_light,
            store_fx: [
                snapshot.ramen_steam_boost,
                bar_flicker,
                snapshot.vending_pulse,
                snapshot.konbini_pulse,
            ],
            pointer: pointer_uniform,
            vehicles: vehicle_slots,
            ripples: ripple_slots,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms, 0, bytemuck::bytes_of(&bg_u));

        self.build_ascii_frame(&snapshot, bar_flicker);
        self.glyphs.upload(
            gpu,
            [self.width as f32, self.height as f32],
            t,
            0.35 + snapshot.wind_gust * 0.85,
        );

        Activity::Animating
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shinjuku-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.bg_pipeline);
            rpass.set_bind_group(0, &self.bg_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
        self.glyphs.render(view, encoder);
    }
}
