//! Shinjuku Rain: Monospace Character-Cell Japanese City Street in a Heavy Downpour.
//!
//! Shares the strict ANSI roguelike + WGSL atmospheric shader aesthetic of
//! *The Living ASCII Rookery*, featuring layered Tokyo skyscrapers, an overhead
//! rail viaduct with passing commuter trains, glowing ramen/izakaya storefronts,
//! vending machines, utility poles, pedestrians with umbrellas, stray alley cats,
//! heavy slanted ASCII rain, and wet-asphalt neon reflections.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::shinjuku::{RainLayer, ShinjukuPalette, ShinjukuRainState, VehicleKind};
use wgpu::util::DeviceExt;

use crate::atlas::GlyphPass;
use crate::audio::{AudioEngine, SoundEffect};
use crate::{Gpu, PointerAction, Scene, SceneInput, fullscreen};

const MAX_INSTANCES: usize = 32_768;
const CELL_W: f32 = 9.6;
const CELL_H: f32 = 16.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BgUniforms {
    resolution: [f32; 2],
    time: f32,
    palette_id: f32,
    lightning: f32,
    signs_lit: f32,
    wind: f32,
    _pad: f32,
}

#[derive(Clone, Copy)]
struct PaletteColors {
    skyline: [f32; 4],
    window_warm: [f32; 4],
    window_cool: [f32; 4],
    neon_a: [f32; 4],
    neon_b: [f32; 4],
    neon_c: [f32; 4],
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
            skyline: [0.14, 0.14, 0.24, 0.92],
            window_warm: [0.98, 0.76, 0.34, 0.92],
            window_cool: [0.36, 0.82, 0.98, 0.86],
            neon_a: [0.98, 0.24, 0.58, 0.98],
            neon_b: [0.18, 0.90, 0.98, 0.98],
            neon_c: [0.98, 0.68, 0.20, 0.98],
            rain_far: [0.32, 0.42, 0.62, 0.42],
            rain_mid: [0.50, 0.68, 0.88, 0.66],
            rain_near: [0.76, 0.90, 1.00, 0.86],
            splash: [0.64, 0.86, 1.00, 0.84],
            steel: [0.34, 0.38, 0.48, 0.94],
            wood: [0.58, 0.40, 0.26, 0.95],
        },
        ShinjukuPalette::KyotoGion => PaletteColors {
            skyline: [0.14, 0.16, 0.22, 0.92],
            window_warm: [0.98, 0.78, 0.36, 0.94],
            window_cool: [0.46, 0.74, 0.70, 0.82],
            neon_a: [0.98, 0.32, 0.22, 0.98],
            neon_b: [0.32, 0.84, 0.74, 0.96],
            neon_c: [0.98, 0.80, 0.30, 0.98],
            rain_far: [0.34, 0.44, 0.56, 0.42],
            rain_mid: [0.52, 0.68, 0.78, 0.66],
            rain_near: [0.78, 0.90, 0.94, 0.85],
            splash: [0.68, 0.86, 0.92, 0.82],
            steel: [0.36, 0.38, 0.42, 0.94],
            wood: [0.66, 0.44, 0.28, 0.96],
        },
        ShinjukuPalette::MidnightDownpour => PaletteColors {
            skyline: [0.12, 0.16, 0.24, 0.92],
            window_warm: [0.94, 0.76, 0.42, 0.88],
            window_cool: [0.54, 0.78, 0.98, 0.88],
            neon_a: [0.60, 0.80, 1.00, 0.96],
            neon_b: [0.82, 0.90, 1.00, 0.96],
            neon_c: [0.96, 0.74, 0.36, 0.96],
            rain_far: [0.36, 0.48, 0.64, 0.44],
            rain_mid: [0.58, 0.74, 0.92, 0.68],
            rain_near: [0.84, 0.94, 1.00, 0.88],
            splash: [0.74, 0.88, 1.00, 0.84],
            steel: [0.38, 0.44, 0.54, 0.94],
            wood: [0.52, 0.44, 0.36, 0.94],
        },
        ShinjukuPalette::CyberTerminal => PaletteColors {
            skyline: [0.06, 0.24, 0.12, 0.92],
            window_warm: [0.96, 0.76, 0.20, 0.94],
            window_cool: [0.24, 0.94, 0.46, 0.90],
            neon_a: [0.22, 0.98, 0.46, 0.98],
            neon_b: [0.98, 0.74, 0.18, 0.98],
            neon_c: [0.46, 0.98, 0.64, 0.98],
            rain_far: [0.12, 0.48, 0.22, 0.44],
            rain_mid: [0.20, 0.74, 0.36, 0.68],
            rain_near: [0.42, 0.98, 0.58, 0.88],
            splash: [0.36, 0.96, 0.54, 0.86],
            steel: [0.16, 0.52, 0.26, 0.94],
            wood: [0.34, 0.68, 0.28, 0.94],
        },
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
            signs_lit: 1.0,
            wind: 0.25,
            _pad: 0.0,
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

    fn build_ascii_frame(&mut self, snapshot: &ShinjukuRainState) {
        self.glyphs.clear();
        let (cols, rows) = self.grid_dims();
        let ground_row = ((rows as f32) * 0.76).round() as i32;
        let viaduct_row = ((rows as f32) * 0.30).round() as i32;
        let pal = palette_colors(snapshot.palette);
        let t = snapshot.time as f32;

        // 1. Far Tokyo Skyline silhouettes & windows
        let bldg_specs: [(f32, f32, i32); 9] = [
            (0.02, 0.12, 18),
            (0.13, 0.22, 24),
            (0.24, 0.34, 15),
            (0.36, 0.46, 27),
            (0.48, 0.58, 20),
            (0.60, 0.70, 25),
            (0.72, 0.81, 17),
            (0.83, 0.92, 22),
            (0.93, 0.99, 14),
        ];
        for (idx, (x0, x1, h_rows)) in bldg_specs.iter().enumerate() {
            let c0 = ((*x0) * cols as f32) as i32;
            let c1 = ((*x1) * cols as f32) as i32;
            let top = (ground_row - h_rows).max(2);
            // Roofline & blinking red aviation beacon
            for c in c0..=c1 {
                self.push_cell(c, top, '─', pal.skyline, 12.0, 0.0, 0.0);
            }
            if idx % 2 == 1 {
                let mid = (c0 + c1) / 2;
                self.push_cell(mid, top - 1, '│', pal.skyline, 12.0, 0.0, 0.0);
                let blink = ((t * 2.2 + idx as f32).sin() > 0.2) as u8;
                if blink == 1 {
                    self.push_cell(mid, top - 2, '*', [0.98, 0.22, 0.24, 0.95], 12.0, 0.0, 0.0);
                }
            }
            for r in (top + 1)..ground_row {
                self.push_cell(c0, r, '│', pal.skyline, 12.0, 0.0, 0.0);
                self.push_cell(c1, r, '│', pal.skyline, 12.0, 0.0, 0.0);
                for c in (c0 + 1)..c1 {
                    if (c + r) % 2 == 0 && (r - top) % 2 == 1 {
                        let hash = ((c * 31 + r * 17 + idx as i32 * 13) & 7) as usize;
                        if hash < 4 {
                            let w_col = if hash.is_multiple_of(2) {
                                pal.window_warm
                            } else {
                                pal.window_cool
                            };
                            let ch = if hash == 0 { '▪' } else { '░' };
                            self.push_cell(c, r, ch, w_col, 13.0, 0.0, 0.0);
                        }
                    }
                }
            }
        }

        // 2. Overhead Steel Rail Viaduct & Passing Commuter Train
        for c in 0..cols {
            self.push_cell(c, viaduct_row, '═', pal.steel, 24.0, 0.0, 0.0);
            self.push_cell(c, viaduct_row + 2, '═', pal.steel, 24.0, 0.0, 0.0);
            if c % 4 == 0 {
                self.push_cell(c, viaduct_row + 1, '│', pal.steel, 24.0, 0.0, 0.0);
            } else if c % 4 == 2 {
                self.push_cell(c, viaduct_row + 1, '/', pal.steel, 24.0, 0.0, 0.0);
            }
            if c % 18 == 9 {
                for r in (viaduct_row + 3)..ground_row {
                    self.push_cell(c, r, '║', pal.steel, 22.0, 0.0, 0.0);
                }
            }
        }
        if let Some(ref train) = snapshot.train {
            let car_cols = 11i32;
            let base_col = ((train.x / 1000.0) * cols as f32).round() as i32;
            for car in 0..train.car_count as i32 {
                let cc = base_col + car * (car_cols + 1);
                self.push_str(cc, viaduct_row - 2, "┌─────────┐", pal.steel, 26.0, 0.0);
                self.push_str(cc, viaduct_row - 1, "│         │", pal.steel, 26.0, 0.0);
                self.push_str(
                    cc + 2,
                    viaduct_row - 1,
                    "▪▪▪▪▪▪▪",
                    pal.window_warm,
                    27.0,
                    0.0,
                );
                self.push_str(cc, viaduct_row, "└─o─────o─┘", pal.steel, 26.0, 0.0);
            }
        }

        // 3. Japanese Storefronts & Street Architecture
        let sign_alpha = if snapshot.signs_lit { 1.0 } else { 0.25 };
        let mut na = pal.neon_a;
        na[3] *= sign_alpha;
        let mut nb = pal.neon_b;
        nb[3] *= sign_alpha;
        let mut nc = pal.neon_c;
        nc[3] *= sign_alpha;

        // 3a. Ramen & Izakaya Shop (left-center)
        let ramen_col = ((cols as f32) * 0.14).round() as i32;
        self.push_str(
            ramen_col,
            ground_row - 6,
            "╔═══════════════╗",
            pal.wood,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 5,
            "║  RAMEN  居酒屋 ║",
            nc,
            35.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 4,
            "╠═══════════════╣",
            pal.wood,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 3,
            "│_│_│_│_│_│_│_│_│",
            na,
            35.0,
            0.25,
        );
        self.push_str(
            ramen_col,
            ground_row - 2,
            "│ ░░░░░░░░░░░░░ │",
            pal.window_warm,
            34.0,
            0.0,
        );
        self.push_str(
            ramen_col,
            ground_row - 1,
            "│ ▒▒▒  OPEN  ▒▒ │",
            pal.window_warm,
            34.0,
            0.0,
        );
        // Swaying red chochin paper lanterns
        self.push_cell(ramen_col - 2, ground_row - 4, '(', na, 36.0, 0.6, 0.0);
        self.push_cell(ramen_col - 1, ground_row - 4, '●', na, 36.0, 0.6, 0.0);
        self.push_cell(ramen_col, ground_row - 4, ')', na, 36.0, 0.6, 0.0);
        self.push_cell(ramen_col + 17, ground_row - 4, '(', na, 36.0, 0.6, 0.0);
        self.push_cell(ramen_col + 18, ground_row - 4, '●', na, 36.0, 0.6, 0.0);
        self.push_cell(ramen_col + 19, ground_row - 4, ')', na, 36.0, 0.6, 0.0);

        // 3b. Neon Arcade / Bar Facade (center)
        let bar_col = ((cols as f32) * 0.43).round() as i32;
        let flicker = if snapshot.signs_lit && ((t * 7.0).sin() > -0.88) {
            1.0
        } else {
            0.3
        };
        let mut bar_na = na;
        bar_na[3] *= flicker;
        self.push_str(bar_col, ground_row - 9, "╔═══════════╗", bar_na, 34.0, 0.0);
        self.push_str(bar_col, ground_row - 8, "║  BAR 24H  ║", bar_na, 35.0, 0.0);
        self.push_str(bar_col, ground_row - 7, "║   SHIN    ║", nb, 35.0, 0.0);
        self.push_str(bar_col, ground_row - 6, "║   JUKU    ║", nb, 35.0, 0.0);
        self.push_str(bar_col, ground_row - 5, "╚═══════════╝", bar_na, 34.0, 0.0);
        self.push_str(
            bar_col + 1,
            ground_row - 4,
            "│ ▓▓▓▓▓▓▓ │",
            pal.steel,
            33.0,
            0.0,
        );
        self.push_str(
            bar_col + 1,
            ground_row - 3,
            "│ ░░░░░░░ │",
            pal.steel,
            33.0,
            0.0,
        );
        self.push_str(bar_col + 1, ground_row - 2, "│  [███]  │", nc, 33.0, 0.0);
        self.push_str(
            bar_col + 1,
            ground_row - 1,
            "│  │   │  │",
            pal.steel,
            33.0,
            0.0,
        );

        // 3c. Glowing Vending Machines (right-center)
        let vend_col = ((cols as f32) * 0.68).round() as i32;
        for vm in 0..2 {
            let vc = vend_col + vm * 6;
            self.push_str(vc, ground_row - 4, "┌───┐", nb, 34.0, 0.0);
            self.push_str(vc, ground_row - 3, "│▓▓▓│", nb, 35.0, 0.0);
            self.push_str(vc, ground_row - 2, "│▪▪▪│", nc, 35.0, 0.0);
            self.push_str(vc, ground_row - 1, "└───┘", pal.steel, 34.0, 0.0);
        }

        // 3d. Utility Poles & Overhead Catenary Power Lines
        let pole1 = ((cols as f32) * 0.08).round() as i32;
        let pole2 = ((cols as f32) * 0.88).round() as i32;
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

        // Curb & Sidewalk divider
        for c in 0..cols {
            let ch = if c % 6 == 0 { '╤' } else { '═' };
            self.push_cell(c, ground_row, ch, pal.steel, 40.0, 0.0, 0.0);
        }

        // 4. Rising Steam from Ramen Shop & Street Grate
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
            self.push_cell(sc, sr, ch, [0.88, 0.92, 0.98, 0.45 * life], 44.0, 0.5, 0.0);
        }

        // 5. Alley Cats
        for cat in &snapshot.cats {
            let cc = ((cat.x / 1000.0) * cols as f32).round() as i32;
            let cr = ((cat.y / 1000.0) * rows as f32).round() as i32;
            let sprite = if cat.darting {
                if cat.facing_right { "=^.^>~" } else { "~<^.^=" }
            } else {
                "/\\_/\\"
            };
            self.push_str(cc - 2, cr, sprite, [0.96, 0.78, 0.52, 0.95], 50.0, 0.0);
        }

        // 6. Pedestrians with Umbrellas
        let umbrella_palette = [
            [0.96, 0.28, 0.46, 0.96],
            [0.24, 0.88, 0.98, 0.96],
            [0.98, 0.82, 0.24, 0.96],
            [0.86, 0.92, 0.98, 0.92],
            [0.74, 0.52, 0.98, 0.96],
        ];
        for ped in &snapshot.pedestrians {
            let pc = ((ped.x / 1000.0) * cols as f32).round() as i32;
            let pr = ((ped.y / 1000.0) * rows as f32).round() as i32;
            let ucol = umbrella_palette[(ped.umbrella_color as usize) % umbrella_palette.len()];
            self.push_str(pc - 2, pr - 2, ".---.", ucol, 55.0, 0.15);
            self.push_cell(pc, pr - 1, 'o', [0.92, 0.92, 0.96, 0.96], 55.0, 0.0, 0.0);
            let legs = if ped.pause_secs > 0.0 {
                "||"
            } else if ped.walk_phase.sin() > 0.0 {
                "/\\"
            } else {
                "||"
            };
            self.push_str(pc, pr, legs, [0.78, 0.82, 0.92, 0.94], 55.0, 0.0);
        }

        // 7. Passing Vehicles (Taxis & Kei Trucks)
        for v in &snapshot.vehicles {
            let vc = ((v.x / 1000.0) * cols as f32).round() as i32;
            let vr = ((v.y / 1000.0) * rows as f32).round() as i32;
            let (body, beam_col) = match v.kind {
                VehicleKind::Taxi => ([0.98, 0.80, 0.18, 0.96], [0.98, 0.96, 0.72, 0.75]),
                VehicleKind::KeiTruck => ([0.84, 0.88, 0.94, 0.96], [0.92, 0.96, 1.00, 0.75]),
            };
            if v.vx > 0.0 {
                self.push_str(vc - 3, vr - 1, "┌────┐", body, 60.0, 0.0);
                self.push_str(vc - 3, vr, "└o──o┘", body, 60.0, 0.0);
                self.push_str(vc + 3, vr, ">>", beam_col, 60.0, 0.0);
                self.push_cell(vc - 4, vr, '*', [0.98, 0.22, 0.22, 0.95], 60.0, 0.0, 0.0);
            } else {
                self.push_str(vc - 3, vr - 1, "┌────┐", body, 60.0, 0.0);
                self.push_str(vc - 3, vr, "└o──o┘", body, 60.0, 0.0);
                self.push_str(vc - 5, vr, "<<", beam_col, 60.0, 0.0);
                self.push_cell(vc + 3, vr, '*', [0.98, 0.22, 0.22, 0.95], 60.0, 0.0, 0.0);
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

        // 9. Heavy Slanted ASCII Rain Streaks
        let slant_angle = snapshot.wind_gust * 0.28;
        for drop in &snapshot.raindrops {
            let dc = ((drop.x / 1000.0) * cols as f32).round() as i32;
            let dr = ((drop.y / 1000.0) * rows as f32).round() as i32;
            let (ch, col, depth) = match drop.layer {
                RainLayer::Far => (':', pal.rain_far, 20.0),
                RainLayer::Mid => ('/', pal.rain_mid, 70.0),
                RainLayer::Near => ('│', pal.rain_near, 85.0),
            };
            self.push_cell(dc, dr, ch, col, depth, 0.0, slant_angle);
            if matches!(drop.layer, RainLayer::Near) {
                self.push_cell(dc, dr - 1, '\'', col, depth, 0.0, slant_angle);
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
        let mut clicked = false;
        let mut click_pos = None;
        let pointer_norm = input.pointer.map(|p| {
            let nx = (p.x / (self.width as f32).max(1.0)) * 1000.0;
            let ny = (p.y / (self.height as f32).max(1.0)) * 1000.0;
            [nx, ny]
        });

        if input.interactive && input.action == PointerAction::LeftClick {
            clicked = true;
            click_pos = pointer_norm;
        }

        let snapshot = if let Ok(mut st) = self.state.lock() {
            if clicked && let Some([nx, ny]) = click_pos {
                if ny < 350.0 {
                    st.trigger_lightning();
                    if let Some(ref audio) = self.audio {
                        audio.play_sfx(SoundEffect::ThunderRumble);
                    }
                } else {
                    st.splash_at(nx, ny);
                    if let Some(ref audio) = self.audio {
                        audio.play_sfx(SoundEffect::RainSplash);
                    }
                }
            }
            st.update(dt.as_secs_f32(), input.interactive, pointer_norm);
            st.clone()
        } else {
            return Activity::Idle;
        };

        let bg_u = BgUniforms {
            resolution: [self.width as f32, self.height as f32],
            time: snapshot.time as f32,
            palette_id: snapshot.palette as u8 as f32,
            lightning: snapshot.lightning_flash,
            signs_lit: if snapshot.signs_lit { 1.0 } else { 0.0 },
            wind: snapshot.wind_gust,
            _pad: 0.0,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms, 0, bytemuck::bytes_of(&bg_u));

        self.build_ascii_frame(&snapshot);
        self.glyphs.upload(
            gpu,
            [self.width as f32, self.height as f32],
            snapshot.time as f32,
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
                            r: 0.02,
                            g: 0.02,
                            b: 0.05,
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
