//! Dynamic 2D painterly mesh builder for all Koi Pond entities.
//!
//! Uses pre-allocated scratch buffers so steady-state frames perform zero heap allocations.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bytemuck::{Pod, Zeroable};
use rookery_core::koi::{
    BambooFountain, Dragonfly, DriftPetal, FloatingLantern, FoodPellet, KOI_SPINE_SEGMENTS,
    KoiFish, KoiPondState, LilyPad, PondFrog, PondRock, PondTheme, PondTurtle, WaterRipple,
};

use super::theme::PondPalette;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct KoiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub accent: [f32; 4],
    pub params: [f32; 4],
}

pub struct KoiMeshBuilder {
    pub verts: Vec<KoiVertex>,
    pub indices: Vec<u32>,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

impl KoiMeshBuilder {
    pub fn from_scratch(
        mut verts: Vec<KoiVertex>,
        mut indices: Vec<u32>,
        width: f32,
        height: f32,
    ) -> Self {
        verts.clear();
        indices.clear();
        let scale = (height / 750.0).clamp(0.45, 2.6);
        Self {
            verts,
            indices,
            width,
            height,
            scale,
        }
    }

    pub fn into_scratch(self) -> (Vec<KoiVertex>, Vec<u32>) {
        (self.verts, self.indices)
    }

    #[inline]
    pub fn w2s(&self, p: [f32; 2]) -> [f32; 2] {
        [(p[0] / 1000.0) * self.width, (p[1] / 750.0) * self.height]
    }

    /// Push a rotated quad centered at screen `center` with half-extents `(hx, hy)`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_quad(
        &mut self,
        center: [f32; 2],
        hx: f32,
        hy: f32,
        angle: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        color: [f32; 4],
        accent: [f32; 4],
        params: [f32; 4],
    ) {
        let (s, c) = angle.sin_cos();
        let base = self.verts.len() as u32;
        let corners = [
            ([-hx, -hy], [uv_min[0], uv_min[1]]),
            ([hx, -hy], [uv_max[0], uv_min[1]]),
            ([hx, hy], [uv_max[0], uv_max[1]]),
            ([-hx, hy], [uv_min[0], uv_max[1]]),
        ];
        for (local, uv) in corners {
            let rx = local[0] * c - local[1] * s;
            let ry = local[0] * s + local[1] * c;
            self.verts.push(KoiVertex {
                pos: [center[0] + rx, center[1] + ry],
                uv,
                color,
                accent,
                params,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Push an oriented quad extending from `root` along `angle` for `length` and width `2*half_w`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_fin_quad(
        &mut self,
        root: [f32; 2],
        angle: f32,
        length: f32,
        half_w: f32,
        color: [f32; 4],
        depth: f32,
    ) {
        let (s, c) = angle.sin_cos();
        let mid = [root[0] + c * (length * 0.5), root[1] + s * (length * 0.5)];
        self.push_quad(
            mid,
            length * 0.5,
            half_w,
            angle,
            [0.0, -1.0],
            [1.0, 1.0],
            color,
            color,
            [2.0, 0.0, 0.0, depth],
        );
    }

    /// Push a soft Gaussian shadow or glow disc.
    pub fn push_soft_disc(
        &mut self,
        center: [f32; 2],
        rx: f32,
        ry: f32,
        angle: f32,
        color: [f32; 4],
        softness: f32,
    ) {
        self.push_quad(
            center,
            rx,
            ry,
            angle,
            [-1.0, -1.0],
            [1.0, 1.0],
            color,
            color,
            [0.0, softness, 0.0, 0.0],
        );
    }

    /// Build all layered painterly geometry for the entire pond scene.
    pub fn build_scene(&mut self, pond: &KoiPondState, pal: &PondPalette) {
        // ---------------------------------------------------------------------
        // LAYER 1: Submerged Riverbed Shadows & Mossy Margin Rocks
        // ---------------------------------------------------------------------
        for rock in &pond.rocks {
            self.push_rock(rock, pal);
        }

        // ---------------------------------------------------------------------
        // LAYER 2: Underwater Koi Shadows + Sculpted Articulated Nishikigoi
        // Sorted deepest-first so shallower koi swim cleanly over deeper ones!
        // ---------------------------------------------------------------------
        let mut fish_order: Vec<usize> = (0..pond.fish.len()).collect();
        fish_order.sort_by(|&a, &b| {
            pond.fish[b]
                .swim_depth
                .partial_cmp(&pond.fish[a].swim_depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for &idx in &fish_order {
            self.push_koi_shadow(&pond.fish[idx], pal);
        }
        for &idx in &fish_order {
            self.push_koi_fish(&pond.fish[idx]);
        }

        // Pond turtles swim gently in the upper water column
        for turtle in &pond.turtles {
            self.push_turtle(turtle, pal);
        }

        // Sinking food pellets
        for pellet in &pond.pellets {
            self.push_pellet(pellet);
        }

        // ---------------------------------------------------------------------
        // LAYER 3: Water Surface Ripple Crests (over underwater koi, under flora)
        // ---------------------------------------------------------------------
        for ripple in &pond.ripples {
            self.push_ripple(ripple, pal);
        }

        // ---------------------------------------------------------------------
        // LAYER 4: Floating Botanical Lily Pads, Lotuses, Frogs, Petals, Props
        // ---------------------------------------------------------------------
        for lily in &pond.lilies {
            self.push_lily_pad_shadow(lily, pal);
        }
        for lily in &pond.lilies {
            self.push_lily_pad(lily, pal, pond.time);
        }
        for frog in &pond.frogs {
            self.push_frog(frog, pal);
        }
        for petal in &pond.petals {
            self.push_drift_petal(petal, pond.theme, pal, pond.time);
        }

        // Bamboo shishi-odoshi fountain in the upper-right mossy bank
        self.push_bamboo_fountain(&pond.fountain, pal, pond.time);

        // Floating glowing tōrō paper lanterns
        if pond.lantern_glow > 0.02 {
            for lantern in &pond.lanterns {
                self.push_lantern(lantern, pond.lantern_glow, pond.time);
            }
        }

        // Skimming dragonflies above the water surface
        for df in &pond.dragonflies {
            self.push_dragonfly(df, pal);
        }
    }

    fn push_rock(&mut self, rock: &PondRock, pal: &PondPalette) {
        let c = self.w2s(rock.pos);
        let rx = rock.radius_x * self.scale;
        let ry = rock.radius_y * self.scale;
        // Soft underwater shadow around rock base
        self.push_soft_disc(
            [c[0] + 6.0 * self.scale, c[1] + 8.0 * self.scale],
            rx * 1.18,
            ry * 1.18,
            rock.rotation,
            pal.shadow_color,
            0.85,
        );
        let base_col = [
            (pal.rock_base[0] * rock.shade).min(1.0),
            (pal.rock_base[1] * rock.shade).min(1.0),
            (pal.rock_base[2] * rock.shade).min(1.0),
            0.98,
        ];
        let moss_col = [pal.rock_moss[0], pal.rock_moss[1], pal.rock_moss[2], 1.0];
        self.push_quad(
            c,
            rx,
            ry,
            rock.rotation,
            [-1.0, -1.0],
            [1.0, 1.0],
            base_col,
            moss_col,
            [6.0, 0.0, rock.rotation * 3.1, rock.moss_amount],
        );
    }

    fn push_koi_shadow(&mut self, koi: &KoiFish, pal: &PondPalette) {
        // Deeper fish cast softer, slightly closer shadows; shallow fish cast crisper offset shadows
        let elevation = (1.0 - koi.swim_depth).clamp(0.15, 1.0);
        let off_x = (12.0 + elevation * 22.0) * self.scale;
        let off_y = (16.0 + elevation * 26.0) * self.scale;
        let mut sh = pal.shadow_color;
        sh[3] *= 0.55 + 0.45 * elevation;

        let mid = self.w2s(koi.spine[4]);
        let tail = self.w2s(koi.spine[9]);
        let half_len = koi.length * 0.52 * self.scale;
        let half_w = koi.length * 0.19 * koi.girth * self.scale;

        self.push_soft_disc(
            [mid[0] + off_x, mid[1] + off_y],
            half_len,
            half_w,
            koi.angle,
            sh,
            0.72 + (1.0 - elevation) * 0.35,
        );
        self.push_soft_disc(
            [tail[0] + off_x, tail[1] + off_y],
            half_len * 0.42,
            half_w * 0.65,
            koi.angle,
            [sh[0], sh[1], sh[2], sh[3] * 0.65],
            0.85,
        );
    }

    fn push_koi_fish(&mut self, koi: &KoiFish) {
        let fin_col = match koi.variety {
            rookery_core::koi::KoiVariety::YamabukiOgon => [0.98, 0.86, 0.42, 0.58],
            rookery_core::koi::KoiVariety::Showa => [0.85, 0.84, 0.82, 0.52],
            rookery_core::koi::KoiVariety::Asagi => [0.94, 0.54, 0.38, 0.56],
            _ => [0.98, 0.96, 0.93, 0.56],
        };

        // 1. Pectoral & pelvic translucent flowing fins (drawn beneath body)
        let shoulder = self.w2s(koi.spine[2]);
        let s_next = self.w2s(koi.spine[3]);
        let shoulder_ang = (shoulder[1] - s_next[1]).atan2(shoulder[0] - s_next[0]);
        let fin_wave = koi.fin_phase.sin() * 0.22;
        let pec_len = koi.length * 0.30 * self.scale;
        let pec_w = koi.length * 0.13 * self.scale;

        self.push_fin_quad(
            shoulder,
            shoulder_ang + 1.12 + fin_wave,
            pec_len,
            pec_w,
            fin_col,
            koi.swim_depth,
        );
        self.push_fin_quad(
            shoulder,
            shoulder_ang - 1.12 - fin_wave,
            pec_len,
            pec_w,
            fin_col,
            koi.swim_depth,
        );

        let pelvis = self.w2s(koi.spine[6]);
        let p_next = self.w2s(koi.spine[7]);
        let pelv_ang = (pelvis[1] - p_next[1]).atan2(pelvis[0] - p_next[0]);
        self.push_fin_quad(
            pelvis,
            pelv_ang + 1.25 - fin_wave * 0.6,
            pec_len * 0.58,
            pec_w * 0.62,
            [fin_col[0], fin_col[1], fin_col[2], fin_col[3] * 0.8],
            koi.swim_depth,
        );
        self.push_fin_quad(
            pelvis,
            pelv_ang - 1.25 + fin_wave * 0.6,
            pec_len * 0.58,
            pec_w * 0.62,
            [fin_col[0], fin_col[1], fin_col[2], fin_col[3] * 0.8],
            koi.swim_depth,
        );

        // 2. Sweeping forked caudal (tail) fin trailing from spine[11]
        let tail_root = self.w2s(koi.spine[KOI_SPINE_SEGMENTS - 1]);
        let tail_prev = self.w2s(koi.spine[KOI_SPINE_SEGMENTS - 2]);
        let tail_ang = (tail_root[1] - tail_prev[1]).atan2(tail_root[0] - tail_prev[0]);
        let tail_flick = (koi.spine_phase - 6.2).sin() * 0.24;
        let tail_len = koi.length * 0.34 * self.scale;
        let tail_w = koi.length * 0.115 * self.scale;

        self.push_fin_quad(
            tail_root,
            tail_ang + 0.28 + tail_flick,
            tail_len,
            tail_w,
            [fin_col[0], fin_col[1], fin_col[2], fin_col[3] * 1.05],
            koi.swim_depth,
        );
        self.push_fin_quad(
            tail_root,
            tail_ang - 0.28 + tail_flick,
            tail_len,
            tail_w,
            [fin_col[0], fin_col[1], fin_col[2], fin_col[3] * 1.05],
            koi.swim_depth,
        );

        // 3. Sculpted 12-segment organic koi body ribbon
        let base_idx = self.verts.len() as u32;
        let params = [
            1.0,
            koi.variety.index() as f32,
            koi.pattern_seed,
            koi.swim_depth,
        ];
        let body_scale_w = koi.length * 0.175 * koi.girth * self.scale;

        for s in 0..KOI_SPINE_SEGMENTS {
            let t = s as f32 / (KOI_SPINE_SEGMENTS - 1) as f32;
            let curr = self.w2s(koi.spine[s]);

            // Smooth tangent along spine
            let dir = if s == 0 {
                let nxt = self.w2s(koi.spine[1]);
                [curr[0] - nxt[0], curr[1] - nxt[1]]
            } else if s == KOI_SPINE_SEGMENTS - 1 {
                let prv = self.w2s(koi.spine[s - 1]);
                [prv[0] - curr[0], prv[1] - curr[1]]
            } else {
                let prv = self.w2s(koi.spine[s - 1]);
                let nxt = self.w2s(koi.spine[s + 1]);
                [prv[0] - nxt[0], prv[1] - nxt[1]]
            };
            let dlen = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt().max(0.001);
            let perp = [-dir[1] / dlen, dir[0] / dlen];

            // Authentic koi dorsal silhouette: rounded snout -> broad shoulders at t~0.24 -> graceful taper
            let profile = if t < 0.24 {
                let k = t / 0.24;
                0.22 + 0.78 * (k * FRAC_PI_2).sin().powf(0.68)
            } else {
                let k = (t - 0.24) / 0.76;
                (1.0 - k * k * 0.86).max(0.11)
            };
            let half_w = body_scale_w * profile;

            self.verts.push(KoiVertex {
                pos: [curr[0] + perp[0] * half_w, curr[1] + perp[1] * half_w],
                uv: [t, 1.0],
                color: [1.0, 1.0, 1.0, 0.99],
                accent: [1.0, 1.0, 1.0, 1.0],
                params,
            });
            self.verts.push(KoiVertex {
                pos: [curr[0] - perp[0] * half_w, curr[1] - perp[1] * half_w],
                uv: [t, -1.0],
                color: [1.0, 1.0, 1.0, 0.99],
                accent: [1.0, 1.0, 1.0, 1.0],
                params,
            });
        }

        for s in 0..(KOI_SPINE_SEGMENTS - 1) as u32 {
            let i0 = base_idx + s * 2;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;
            self.indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
        }

        // 4. Translucent flowing dorsal fin ridge along spine[3..8]
        let d_root = self.w2s(koi.spine[4]);
        let d_tail = self.w2s(koi.spine[8]);
        let d_ang = (d_tail[1] - d_root[1]).atan2(d_tail[0] - d_root[0]);
        let d_len = ((d_tail[0] - d_root[0]).hypot(d_tail[1] - d_root[1])).max(4.0);
        let d_sway = (koi.spine_phase - 2.5).sin() * 0.14;
        self.push_fin_quad(
            d_root,
            d_ang + d_sway,
            d_len * 1.15,
            koi.length * 0.055 * self.scale,
            [fin_col[0], fin_col[1], fin_col[2], fin_col[3] * 0.72],
            koi.swim_depth,
        );
    }

    fn push_pellet(&mut self, pellet: &FoodPellet) {
        let c = self.w2s(pellet.pos);
        let bob = 1.0 + pellet.bob_phase.sin() * 0.12;
        let r = 3.8 * self.scale * bob;
        let alpha = (1.0 - pellet.sink_depth * 0.8).clamp(0.15, 0.98);
        self.push_quad(
            c,
            r,
            r,
            0.0,
            [-1.0, -1.0],
            [1.0, 1.0],
            [0.86, 0.64, 0.30, alpha],
            [0.54, 0.36, 0.14, alpha],
            [6.0, 1.0, 0.0, 0.0],
        );
    }

    fn push_ripple(&mut self, r: &WaterRipple, pal: &PondPalette) {
        let c = self.w2s(r.center);
        let rad = (r.radius / 750.0) * self.height;
        let fade = (1.0 - r.progress()).powf(0.75);
        let alpha = (r.amplitude * 0.42 * fade).clamp(0.0, 0.52);
        if alpha < 0.01 || rad < 2.0 {
            return;
        }
        self.push_quad(
            c,
            rad,
            rad,
            0.0,
            [-1.0, -1.0],
            [1.0, 1.0],
            [
                pal.ripple_tint[0],
                pal.ripple_tint[1],
                pal.ripple_tint[2],
                alpha,
            ],
            [1.0, 1.0, 1.0, alpha],
            [5.0, r.ring_count as f32, 0.0, 0.0],
        );
    }

    fn push_lily_pad_shadow(&mut self, lily: &LilyPad, pal: &PondPalette) {
        let c = self.w2s(lily.pos);
        let r = lily.radius * self.scale;
        self.push_soft_disc(
            [c[0] + 9.0 * self.scale, c[1] + 12.0 * self.scale],
            r * 1.10,
            r * 1.10,
            0.0,
            pal.shadow_color,
            0.78,
        );
    }

    fn push_lily_pad(&mut self, lily: &LilyPad, pal: &PondPalette, time: f32) {
        let c = self.w2s(lily.pos);
        let jostle = 1.0 + lily.ripple_jostle * 0.04 * (time * 9.0).sin();
        let r = lily.radius * self.scale * jostle;
        let rot = lily.rotation + lily.ripple_jostle * 0.06 * (time * 6.0).cos();

        self.push_quad(
            c,
            r,
            r,
            rot,
            [-1.0, -1.0],
            [1.0, 1.0],
            [pal.lily_base[0], pal.lily_base[1], pal.lily_base[2], 0.96],
            [pal.lily_vein[0], pal.lily_vein[1], pal.lily_vein[2], 0.96],
            [3.0, 0.0, lily.id as f32 * 1.9, 0.0],
        );

        // Multi-petal blooming lotus flower
        if lily.has_flower && lily.flower_bloom > 0.06 {
            let bloom = lily.flower_bloom;
            let (outer_tip, outer_base, inner_tip) = match lily.flower_color_idx {
                0 => (
                    [0.98, 0.78, 0.86, 0.96],
                    [0.86, 0.38, 0.56, 0.96],
                    [1.00, 0.89, 0.94, 0.98],
                ),
                1 => (
                    [0.98, 0.96, 0.89, 0.96],
                    [0.90, 0.78, 0.64, 0.96],
                    [1.00, 0.99, 0.95, 0.98],
                ),
                2 => (
                    [0.96, 0.54, 0.74, 0.96],
                    [0.74, 0.18, 0.44, 0.96],
                    [0.99, 0.76, 0.86, 0.98],
                ),
                _ => (
                    [0.99, 0.92, 0.66, 0.96],
                    [0.88, 0.66, 0.28, 0.96],
                    [1.00, 0.96, 0.80, 0.98],
                ),
            };

            let fl_r = (14.0 + bloom * 13.0) * self.scale;
            let outer_count = 8;
            for i in 0..outer_count {
                let ang = rot * 0.5 + (i as f32 / outer_count as f32) * TAU;
                let px = c[0] + ang.cos() * (fl_r * 0.46);
                let py = c[1] + ang.sin() * (fl_r * 0.46);
                self.push_quad(
                    [px, py],
                    fl_r * 0.56,
                    fl_r * 0.27,
                    ang,
                    [-1.0, -1.0],
                    [1.0, 1.0],
                    outer_tip,
                    outer_base,
                    [4.0, 0.0, 0.0, 0.0],
                );
            }

            let inner_count = 6;
            for i in 0..inner_count {
                let ang = rot * 0.5 + ((i as f32 + 0.5) / inner_count as f32) * TAU;
                let px = c[0] + ang.cos() * (fl_r * 0.28);
                let py = c[1] + ang.sin() * (fl_r * 0.28);
                self.push_quad(
                    [px, py],
                    fl_r * 0.40,
                    fl_r * 0.20,
                    ang,
                    [-1.0, -1.0],
                    [1.0, 1.0],
                    inner_tip,
                    outer_base,
                    [4.0, 0.0, 0.0, 0.0],
                );
            }

            // Golden stamen center
            let st_r = fl_r * 0.24;
            self.push_quad(
                c,
                st_r,
                st_r,
                0.0,
                [-1.0, -1.0],
                [1.0, 1.0],
                [0.99, 0.85, 0.28, 0.98],
                [0.86, 0.58, 0.12, 0.98],
                [6.0, 1.0, 0.0, 0.0],
            );
        }
    }

    fn push_frog(&mut self, frog: &PondFrog, pal: &PondPalette) {
        let c = self.w2s(frog.pos);
        let hop_arc = if frog.mode == rookery_core::koi::FrogMode::Hopping {
            1.0 + (frog.hop_progress * PI).sin() * 0.32
        } else {
            1.0
        };
        let breath = 1.0 + frog.breath_phase.sin() * 0.06;
        let s = self.scale * hop_arc;
        let body_col = [
            0.22 * pal.ambient_mul,
            0.54 * pal.ambient_mul,
            0.24 * pal.ambient_mul,
            0.96,
        ];
        let dark_col = [
            0.12 * pal.ambient_mul,
            0.34 * pal.ambient_mul,
            0.14 * pal.ambient_mul,
            0.96,
        ];

        // Soft shadow under frog
        self.push_soft_disc(
            [c[0] + 2.5 * s, c[1] + 3.0 * s],
            9.0 * s,
            7.0 * s,
            frog.angle,
            pal.shadow_color,
            0.7,
        );
        // Folded hind legs
        let (sa, ca) = frog.angle.sin_cos();
        for side in [-1.0f32, 1.0f32] {
            let lx = c[0] - ca * 2.5 * s - sa * 5.0 * side * s;
            let ly = c[1] - sa * 2.5 * s + ca * 5.0 * side * s;
            self.push_quad(
                [lx, ly],
                4.2 * s,
                2.5 * s,
                frog.angle + side * 0.45,
                [-1.0, -1.0],
                [1.0, 1.0],
                dark_col,
                dark_col,
                [6.0, 1.0, 0.0, 0.0],
            );
        }
        // Main plump emerald body
        self.push_quad(
            c,
            7.2 * s * breath,
            5.4 * s * breath,
            frog.angle,
            [-1.0, -1.0],
            [1.0, 1.0],
            body_col,
            dark_col,
            [6.0, 1.0, 0.0, 0.0],
        );
        // Tiny golden-rimmed eyes
        for side in [-1.0f32, 1.0f32] {
            let ex = c[0] + ca * 4.5 * s - sa * 2.4 * side * s;
            let ey = c[1] + sa * 4.5 * s + ca * 2.4 * side * s;
            self.push_quad(
                [ex, ey],
                1.5 * s,
                1.5 * s,
                0.0,
                [-1.0, -1.0],
                [1.0, 1.0],
                [0.92, 0.84, 0.34, 0.96],
                [0.10, 0.12, 0.08, 0.96],
                [6.0, 1.0, 0.0, 0.0],
            );
        }
    }

    fn push_turtle(&mut self, turtle: &PondTurtle, pal: &PondPalette) {
        let c = self.w2s(turtle.pos);
        let s = turtle.size * self.scale;
        let (sa, ca) = turtle.angle.sin_cos();
        let stroke = turtle.stroke_phase.sin() * 0.42;

        // Underwater shadow
        self.push_soft_disc(
            [c[0] + 10.0 * self.scale, c[1] + 12.0 * self.scale],
            s * 1.15,
            s * 0.92,
            turtle.angle,
            pal.shadow_color,
            0.8,
        );

        let limb_col = [
            0.22 * pal.ambient_mul,
            0.34 * pal.ambient_mul,
            0.22 * pal.ambient_mul,
            0.92,
        ];
        // 4 paddling flippers
        for (fx, fy, fl_ang) in [
            (0.45f32, 0.72f32, 0.75 + stroke),
            (0.45, -0.72, -0.75 - stroke),
            (-0.55, 0.65, 2.25 - stroke * 0.6),
            (-0.55, -0.65, -2.25 + stroke * 0.6),
        ] {
            let px = c[0] + (ca * fx - sa * fy) * s;
            let py = c[1] + (sa * fx + ca * fy) * s;
            self.push_quad(
                [px, py],
                s * 0.42,
                s * 0.20,
                turtle.angle + fl_ang,
                [-1.0, -1.0],
                [1.0, 1.0],
                limb_col,
                [
                    limb_col[0] * 0.7,
                    limb_col[1] * 0.7,
                    limb_col[2] * 0.7,
                    0.92,
                ],
                [6.0, 1.0, 0.0, 0.0],
            );
        }

        // Head extending forward
        let hx = c[0] + ca * s * 1.15;
        let hy = c[1] + sa * s * 1.15;
        self.push_quad(
            [hx, hy],
            s * 0.36,
            s * 0.24,
            turtle.angle,
            [-1.0, -1.0],
            [1.0, 1.0],
            limb_col,
            [
                limb_col[0] * 0.7,
                limb_col[1] * 0.7,
                limb_col[2] * 0.7,
                0.95,
            ],
            [6.0, 1.0, 0.0, 0.0],
        );

        // Domed olive-bronze carapace shell
        self.push_quad(
            c,
            s * 0.95,
            s * 0.76,
            turtle.angle,
            [-1.0, -1.0],
            [1.0, 1.0],
            [
                0.30 * pal.ambient_mul,
                0.36 * pal.ambient_mul,
                0.20 * pal.ambient_mul,
                0.96,
            ],
            [
                0.14 * pal.ambient_mul,
                0.20 * pal.ambient_mul,
                0.12 * pal.ambient_mul,
                0.96,
            ],
            [6.0, 1.0, 0.0, 0.0],
        );
    }

    fn push_drift_petal(&mut self, p: &DriftPetal, theme: PondTheme, pal: &PondPalette, time: f32) {
        let c = self.w2s(p.pos);
        let sz = p.size * self.scale;

        if theme == PondTheme::MoonlitLanternNight {
            // Soft glowing golden-jade nocturnal fireflies drifting above the water
            let pulse = 0.45 + 0.55 * (time * 2.4 + p.phase).sin().abs();
            self.push_soft_disc(
                c,
                sz * 2.1 * pulse,
                sz * 2.1 * pulse,
                0.0,
                [0.98, 0.88, 0.42, 0.42 * pulse],
                0.65,
            );
            self.push_soft_disc(
                c,
                sz * 0.52,
                sz * 0.52,
                0.0,
                [1.0, 0.98, 0.78, 0.92 * pulse],
                0.4,
            );
            return;
        }

        let (kind, tip_col, base_col) = match theme {
            PondTheme::SakuraMorning => match p.color_variant {
                0 => (1.0, [0.99, 0.82, 0.89, 0.92], [0.93, 0.58, 0.72, 0.92]),
                1 => (1.0, [1.00, 0.90, 0.94, 0.90], [0.95, 0.72, 0.82, 0.90]),
                _ => (1.0, [0.97, 0.74, 0.84, 0.92], [0.88, 0.48, 0.64, 0.92]),
            },
            PondTheme::SunlitJade => match p.color_variant {
                0 => (1.0, [0.99, 0.96, 0.88, 0.88], [0.94, 0.84, 0.62, 0.88]),
                1 => (1.0, [0.98, 0.84, 0.88, 0.88], [0.92, 0.62, 0.72, 0.88]),
                _ => (0.0, [0.36, 0.64, 0.34, 0.84], [0.18, 0.42, 0.22, 0.84]),
            },
            PondTheme::AutumnMapleDusk => match p.color_variant {
                0 => (2.0, [0.86, 0.20, 0.14, 0.94], [0.58, 0.10, 0.08, 0.94]),
                1 => (2.0, [0.94, 0.44, 0.14, 0.94], [0.68, 0.24, 0.08, 0.94]),
                2 => (2.0, [0.95, 0.70, 0.18, 0.94], [0.74, 0.44, 0.10, 0.94]),
                _ => (2.0, [0.76, 0.16, 0.20, 0.94], [0.46, 0.08, 0.12, 0.94]),
            },
            PondTheme::MoonlitLanternNight => unreachable!(),
        };

        let mult = if kind > 1.5 { 1.45 } else { 1.0 };
        self.push_soft_disc(
            [c[0] + 3.0 * self.scale, c[1] + 4.0 * self.scale],
            sz * mult * 0.9,
            sz * mult * 0.7,
            p.rotation,
            [
                pal.shadow_color[0],
                pal.shadow_color[1],
                pal.shadow_color[2],
                0.28,
            ],
            0.7,
        );
        self.push_quad(
            c,
            sz * mult,
            sz * mult * if kind > 1.5 { 1.0 } else { 0.68 },
            p.rotation,
            [-1.0, -1.0],
            [1.0, 1.0],
            tip_col,
            base_col,
            [4.0, kind, 0.0, 0.0],
        );
    }

    fn push_bamboo_fountain(&mut self, f: &BambooFountain, pal: &PondPalette, time: f32) {
        let pivot = self.w2s(f.pivot);
        let splash = self.w2s(f.splash_pos);
        let s = self.scale;

        let bamboo_light = [
            0.46 * pal.ambient_mul,
            0.54 * pal.ambient_mul,
            0.28 * pal.ambient_mul,
            0.98,
        ];
        let bamboo_dark = [
            0.26 * pal.ambient_mul,
            0.34 * pal.ambient_mul,
            0.16 * pal.ambient_mul,
            0.98,
        ];

        // Pivoting hollow bamboo clapper tube
        let rest_ang = 2.42; // pointing down-left toward pond
        let tip_ang = rest_ang - f.tilt * 0.48;
        let tube_len = 74.0 * s;
        let tube_w = 8.5 * s;

        // Shadow below bamboo tube
        self.push_soft_disc(
            [pivot[0] - 10.0 * s, pivot[1] + 18.0 * s],
            tube_len * 0.55,
            tube_w * 1.4,
            tip_ang,
            pal.shadow_color,
            0.75,
        );

        // Bamboo tube body
        self.push_quad(
            pivot,
            tube_len * 0.5,
            tube_w,
            tip_ang,
            [-1.0, -1.0],
            [1.0, 1.0],
            bamboo_light,
            bamboo_dark,
            [6.0, 1.0, 0.0, 0.0],
        );

        // Bamboo support posts flanking the pivot
        for off in [-10.0f32, 10.0f32] {
            self.push_quad(
                [pivot[0] + off * s, pivot[1] + 2.0 * s],
                4.2 * s,
                4.2 * s,
                0.0,
                [-1.0, -1.0],
                [1.0, 1.0],
                bamboo_dark,
                [
                    bamboo_dark[0] * 0.7,
                    bamboo_dark[1] * 0.7,
                    bamboo_dark[2] * 0.7,
                    0.98,
                ],
                [6.0, 1.0, 0.0, 0.0],
            );
        }

        // Gentle water trickle feeding the spout + splash foam when tipping
        let trickle_alpha = 0.45 + 0.15 * (time * 8.0).sin();
        self.push_quad(
            [pivot[0] - 24.0 * s, pivot[1] + 22.0 * s],
            14.0 * s,
            2.4 * s,
            2.38,
            [-1.0, -1.0],
            [1.0, 1.0],
            [0.82, 0.94, 0.96, trickle_alpha],
            [0.62, 0.84, 0.88, trickle_alpha],
            [6.0, 1.0, 0.0, 0.0],
        );

        if f.splash_glow > 0.02 {
            let foam_r = (14.0 + (1.0 - f.splash_glow) * 18.0) * s;
            self.push_soft_disc(
                splash,
                foam_r,
                foam_r,
                0.0,
                [0.88, 0.96, 0.98, f.splash_glow * 0.55],
                0.7,
            );
        }
    }

    fn push_lantern(&mut self, l: &FloatingLantern, glow: f32, time: f32) {
        let c = self.w2s(l.pos);
        let sz = l.size * self.scale;
        let flicker = 0.88 + 0.12 * (time * 4.2 + l.flicker_phase).sin();

        // Warm outer glow aura over the water
        self.push_soft_disc(
            c,
            sz * 3.2,
            sz * 3.2,
            0.0,
            [1.0, 0.66, 0.25, 0.38 * glow * flicker],
            0.75,
        );

        // Floating wooden raft base
        self.push_quad(
            c,
            sz * 0.96,
            sz * 0.96,
            l.rotation,
            [-1.0, -1.0],
            [1.0, 1.0],
            [0.28, 0.17, 0.10, 0.95 * glow],
            [0.18, 0.10, 0.06, 0.95 * glow],
            [6.0, 2.0, 0.0, 0.0],
        );

        // Warm backlit washi paper lantern square
        self.push_quad(
            c,
            sz * 0.72,
            sz * 0.72,
            l.rotation,
            [-1.0, -1.0],
            [1.0, 1.0],
            [0.98, 0.72, 0.34, 0.98 * glow],
            [1.00, 0.94, 0.68, 0.98 * glow],
            [6.0, 2.0, 0.0, 0.0],
        );
    }

    fn push_dragonfly(&mut self, df: &Dragonfly, pal: &PondPalette) {
        let c = self.w2s(df.pos);
        let s = self.scale;
        let (sa, ca) = df.angle.sin_cos();
        let flutter = df.wing_phase.sin() * 0.26;

        let body_col = match df.color_idx {
            0 => [0.22, 0.66, 0.96, 0.95],
            1 => [0.22, 0.86, 0.68, 0.95],
            _ => [0.96, 0.66, 0.24, 0.95],
        };

        // Faint airborne shadow offset below dragonfly
        self.push_soft_disc(
            [c[0] + 18.0 * s, c[1] + 24.0 * s],
            10.0 * s,
            4.5 * s,
            df.angle,
            [
                pal.shadow_color[0],
                pal.shadow_color[1],
                pal.shadow_color[2],
                0.22,
            ],
            0.75,
        );

        // 4 iridescent translucent wings
        let wing_col = [0.86, 0.95, 1.0, 0.44];
        for (fwd, base_ang) in [
            (3.0f32, 1.38 + flutter),
            (3.0, -1.38 - flutter),
            (-1.0, 1.72 - flutter * 0.7),
            (-1.0, -1.72 + flutter * 0.7),
        ] {
            let root = [c[0] + ca * fwd * s, c[1] + sa * fwd * s];
            self.push_fin_quad(root, df.angle + base_ang, 15.0 * s, 3.2 * s, wing_col, 0.0);
        }

        // Slender iridescent abdomen & thorax
        self.push_quad(
            [c[0] - ca * 3.0 * s, c[1] - sa * 3.0 * s],
            10.5 * s,
            1.7 * s,
            df.angle,
            [-1.0, -1.0],
            [1.0, 1.0],
            body_col,
            [
                body_col[0] * 0.6,
                body_col[1] * 0.6,
                body_col[2] * 0.6,
                0.95,
            ],
            [6.0, 1.0, 0.0, 0.0],
        );
    }
}
