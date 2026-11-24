//! Dynamic 2D procedural mesh builder for Cosmic Nebula v2.0 entities.

use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use rookery_core::nebula::{
    Comet, ConstellationEdge, DustMote, Meteor, NurseryStar, Pulsar, Shockwave, StarStage, WORLD_H,
    WORLD_W,
};

use super::theme::NebulaVisualPalette;

pub const KIND_JWST_STAR: u32 = 0;
pub const KIND_PROTOSTAR: u32 = 1;
pub const KIND_DUST_MOTE: u32 = 2;
pub const KIND_RIBBON: u32 = 3;
pub const KIND_CHART_LINE: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NebulaVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub kind: u32,
    pub param: f32,
}

impl NebulaVertex {
    pub const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Uint32,
        4 => Float32,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

pub struct NebulaMeshBuilder<'a> {
    verts: &'a mut Vec<NebulaVertex>,
    indices: &'a mut Vec<u32>,
    width: f32,
    height: f32,
    scale: f32,
}

impl<'a> NebulaMeshBuilder<'a> {
    pub fn new(
        verts: &'a mut Vec<NebulaVertex>,
        indices: &'a mut Vec<u32>,
        width: f32,
        height: f32,
    ) -> Self {
        verts.clear();
        indices.clear();
        let scale = (height / WORLD_H).clamp(0.55, 2.2);
        Self {
            verts,
            indices,
            width,
            height,
            scale,
        }
    }

    #[inline]
    fn to_screen(&self, world: [f32; 2]) -> [f32; 2] {
        [
            (world[0] / WORLD_W) * self.width,
            (world[1] / WORLD_H) * self.height,
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn push_oriented_quad(
        &mut self,
        center: [f32; 2],
        axis_x: [f32; 2],
        axis_y: [f32; 2],
        half_w: f32,
        half_h: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        color: [f32; 4],
        kind: u32,
        param: f32,
    ) {
        let base = self.verts.len() as u32;
        let dx = [axis_x[0] * half_w, axis_x[1] * half_w];
        let dy = [axis_y[0] * half_h, axis_y[1] * half_h];

        let p0 = [center[0] - dx[0] - dy[0], center[1] - dx[1] - dy[1]];
        let p1 = [center[0] + dx[0] - dy[0], center[1] + dx[1] - dy[1]];
        let p2 = [center[0] + dx[0] + dy[0], center[1] + dx[1] + dy[1]];
        let p3 = [center[0] - dx[0] + dy[0], center[1] - dx[1] + dy[1]];

        self.verts.push(NebulaVertex {
            pos: p0,
            uv: [uv_min[0], uv_min[1]],
            color,
            kind,
            param,
        });
        self.verts.push(NebulaVertex {
            pos: p1,
            uv: [uv_max[0], uv_min[1]],
            color,
            kind,
            param,
        });
        self.verts.push(NebulaVertex {
            pos: p2,
            uv: [uv_max[0], uv_max[1]],
            color,
            kind,
            param,
        });
        self.verts.push(NebulaVertex {
            pos: p3,
            uv: [uv_min[0], uv_max[1]],
            color,
            kind,
            param,
        });

        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn push_segment(
        &mut self,
        a: [f32; 2],
        b: [f32; 2],
        half_thickness: f32,
        color: [f32; 4],
        kind: u32,
        param: f32,
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 0.5 {
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        let nx = -uy;
        let ny = ux;
        let center = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
        self.push_oriented_quad(
            center,
            [ux, uy],
            [nx, ny],
            len * 0.5,
            half_thickness,
            [0.0, -1.0],
            [1.0, 1.0],
            color,
            kind,
            param,
        );
    }

    pub fn push_dust_motes(&mut self, dust: &[DustMote], pal: &NebulaVisualPalette, time: f32) {
        for m in dust {
            let p = self.to_screen(m.pos);
            let spd = m.vel[0].hypot(m.vel[1]);
            let twinkle = 0.62 + 0.38 * (time * 1.9 + m.phase).sin();
            let col = blend_star_color(pal, m.color_t);
            let alpha = (0.28 + 0.52 * m.depth) * twinkle;

            let base_r = m.size * 1.45 * self.scale * (0.50 + 0.55 * m.depth);
            if spd > 28.0 {
                let ux = m.vel[0] / spd;
                let uy = m.vel[1] / spd;
                let stretch = (1.0 + (spd - 28.0) * 0.055).clamp(1.0, 6.0);
                self.push_oriented_quad(
                    p,
                    [ux, uy],
                    [-uy, ux],
                    base_r * stretch,
                    base_r * 0.65,
                    [-1.0, -1.0],
                    [1.0, 1.0],
                    [col[0], col[1], col[2], alpha * 0.92],
                    KIND_DUST_MOTE,
                    0.0,
                );
            } else {
                self.push_oriented_quad(
                    p,
                    [1.0, 0.0],
                    [0.0, 1.0],
                    base_r,
                    base_r,
                    [-1.0, -1.0],
                    [1.0, 1.0],
                    [col[0], col[1], col[2], alpha * 0.85],
                    KIND_DUST_MOTE,
                    0.0,
                );
            }
        }
    }

    pub fn push_nursery_stars(
        &mut self,
        stars: &[NurseryStar],
        pal: &NebulaVisualPalette,
        time: f32,
    ) {
        for s in stars {
            let center = self.to_screen(s.pos);
            let mut col = blend_star_color(pal, s.temp_t);
            let twinkle = 0.90 + 0.10 * (time * 2.6 + s.phase).sin();

            match s.stage {
                StarStage::Protostar { collapse } => {
                    let half = (s.radius * 3.4 * self.scale) * (1.25 - 0.35 * collapse);
                    let intensity = (0.65 + 0.85 * collapse) * twinkle;
                    self.push_oriented_quad(
                        center,
                        [1.0, 0.0],
                        [0.0, 1.0],
                        half,
                        half,
                        [-1.0, -1.0],
                        [1.0, 1.0],
                        [pal.rim_glow[0], pal.rim_glow[1], pal.rim_glow[2], intensity],
                        KIND_PROTOSTAR,
                        collapse,
                    );
                }
                StarStage::MainSequence => {
                    let flash = s.ignition_flash;
                    let half = (s.radius * 4.8 * self.scale) * (1.0 + 0.75 * flash);
                    let intensity = (s.luminosity * twinkle + flash * 1.1).min(2.4);
                    // Subtle warm-to-white shift on ignition flash
                    col[0] = (col[0] + flash * 0.3).min(1.0);
                    col[1] = (col[1] + flash * 0.25).min(1.0);
                    col[2] = (col[2] + flash * 0.2).min(1.0);
                    self.push_oriented_quad(
                        center,
                        [1.0, 0.0],
                        [0.0, 1.0],
                        half,
                        half,
                        [-1.0, -1.0],
                        [1.0, 1.0],
                        [col[0], col[1], col[2], intensity],
                        KIND_JWST_STAR,
                        s.luminosity,
                    );
                }
                StarStage::Supergiant { instability } => {
                    let pulse =
                        1.0 + instability * 0.28 * (time * (4.0 + instability * 10.0)).sin();
                    let half = s.radius * 5.6 * self.scale * pulse;
                    // Shift slightly hotter/crimson-gold as instability climbs
                    let r = (col[0] + instability * 0.35).min(1.0);
                    let g = (col[1] * (1.0 - 0.2 * instability)).max(0.35);
                    let b = (col[2] * (1.0 - 0.1 * instability)).max(0.45);
                    let intensity = (s.luminosity * 1.15 + instability * 0.6) * pulse;
                    self.push_oriented_quad(
                        center,
                        [1.0, 0.0],
                        [0.0, 1.0],
                        half,
                        half,
                        [-1.0, -1.0],
                        [1.0, 1.0],
                        [r, g, b, intensity],
                        KIND_JWST_STAR,
                        s.luminosity + instability,
                    );
                }
            }
        }
    }

    pub fn push_pulsar(&mut self, pulsar: &Pulsar, pal: &NebulaVisualPalette) {
        let center = self.to_screen(pulsar.pos);
        let pulse = 0.78 + 0.22 * pulsar.pulse_phase.sin();
        let half = 24.0 * self.scale * pulse;
        self.push_oriented_quad(
            center,
            [1.0, 0.0],
            [0.0, 1.0],
            half,
            half,
            [-1.0, -1.0],
            [1.0, 1.0],
            [
                pal.star_cool[0],
                pal.star_cool[1],
                pal.star_cool[2],
                pulsar.intensity * 1.15 * pulse,
            ],
            KIND_JWST_STAR,
            1.4,
        );
    }

    pub fn push_shockwave_filaments(
        &mut self,
        shockwaves: &[Shockwave],
        pal: &NebulaVisualPalette,
    ) {
        for sw in shockwaves {
            let progress = (sw.age / sw.max_age).clamp(0.0, 1.0);
            let fade = (1.0 - progress).powf(1.4) * (progress * 8.0).min(1.0);
            if fade < 0.02 {
                continue;
            }
            let center = self.to_screen(sw.pos);
            // Match backdrop shader's circular radius in screen pixels
            let r_px = (sw.radius / WORLD_H) * self.height;
            let segments = 48usize;
            let half_thick = (11.0 - 5.0 * progress).max(3.5) * self.scale;

            let base = self.verts.len() as u32;
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let a = t * TAU + sw.hue_t * 2.0;
                let wobble = 1.0 + 0.018 * (a * 5.0 + sw.age * 1.6).sin();
                let r = r_px * wobble;
                let ca = a.cos();
                let sa = a.sin();
                let shimmer = 0.45 + 0.55 * (a * 3.0 - sw.age * 2.0).cos().abs();
                let col_mix = 0.5 + 0.5 * (a * 2.0 + sw.hue_t * TAU).sin();
                let col = [
                    pal.star_cool[0] + (pal.rim_glow[0] - pal.star_cool[0]) * col_mix,
                    pal.star_cool[1] + (pal.rim_glow[1] - pal.star_cool[1]) * col_mix,
                    pal.star_cool[2] + (pal.rim_glow[2] - pal.star_cool[2]) * col_mix,
                    fade * shimmer * 0.48,
                ];

                let p_in = [
                    center[0] + ca * (r - half_thick),
                    center[1] + sa * (r - half_thick),
                ];
                let p_out = [
                    center[0] + ca * (r + half_thick),
                    center[1] + sa * (r + half_thick),
                ];
                self.verts.push(NebulaVertex {
                    pos: p_in,
                    uv: [t, -1.0],
                    color: col,
                    kind: KIND_RIBBON,
                    param: 1.0, // continuous closed filament (no longitudinal taper)
                });
                self.verts.push(NebulaVertex {
                    pos: p_out,
                    uv: [t, 1.0],
                    color: col,
                    kind: KIND_RIBBON,
                    param: 1.0,
                });
            }
            for i in 0..segments as u32 {
                let v0 = base + i * 2;
                let v1 = v0 + 1;
                let v2 = v0 + 2;
                let v3 = v0 + 3;
                self.indices.extend_from_slice(&[v0, v2, v3, v0, v3, v1]);
            }
        }
    }

    pub fn push_comets(&mut self, comets: &[Comet], pal: &NebulaVisualPalette) {
        for c in comets {
            let head = self.to_screen(c.pos);
            let spd = c.vel[0].hypot(c.vel[1]).max(1.0);
            let ux = c.vel[0] / spd;
            let uy = c.vel[1] / spd;

            let fade = (c.age * 1.4).min(1.0) * ((c.max_age - c.age) * 0.8).clamp(0.0, 1.0);
            if fade < 0.02 {
                continue;
            }

            // 1. Straight cyan-blue ion tail pointing away from velocity
            let ion_len = 155.0 * self.scale * c.brightness;
            let ion_tail = [head[0] - ux * ion_len, head[1] - uy * ion_len];
            self.push_segment(
                head,
                ion_tail,
                7.5 * self.scale,
                [0.45, 0.88, 1.0, fade * 0.72],
                KIND_RIBBON,
                0.0,
            );

            // 2. Seamless continuous curved warm golden dust tail along recent trajectory
            if c.trail.len() >= 2 {
                let mut pts: Vec<[f32; 2]> = Vec::with_capacity(c.trail.len() + 1);
                pts.push(head);
                for pt in c.trail.iter().rev() {
                    pts.push(self.to_screen(*pt));
                }
                let n = pts.len();
                let base = self.verts.len() as u32;
                for i in 0..n {
                    let t_norm = i as f32 / (n - 1).max(1) as f32;
                    let (tx, ty) = if i == 0 {
                        (pts[1][0] - pts[0][0], pts[1][1] - pts[0][1])
                    } else if i + 1 == n {
                        (pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1])
                    } else {
                        (pts[i + 1][0] - pts[i - 1][0], pts[i + 1][1] - pts[i - 1][1])
                    };
                    let len = tx.hypot(ty).max(0.001);
                    let nx = -ty / len;
                    let ny = tx / len;
                    let half_w = (4.5 + 11.0 * t_norm) * self.scale;
                    let col = [
                        pal.star_warm[0],
                        pal.star_warm[1],
                        pal.star_warm[2],
                        fade * 0.62,
                    ];
                    let p_l = [pts[i][0] - nx * half_w, pts[i][1] - ny * half_w];
                    let p_r = [pts[i][0] + nx * half_w, pts[i][1] + ny * half_w];
                    self.verts.push(NebulaVertex {
                        pos: p_l,
                        uv: [t_norm, -1.0],
                        color: col,
                        kind: KIND_RIBBON,
                        param: 0.0,
                    });
                    self.verts.push(NebulaVertex {
                        pos: p_r,
                        uv: [t_norm, 1.0],
                        color: col,
                        kind: KIND_RIBBON,
                        param: 0.0,
                    });
                }
                for i in 0..(n - 1) as u32 {
                    let v0 = base + i * 2;
                    let v1 = v0 + 1;
                    let v2 = v0 + 2;
                    let v3 = v0 + 3;
                    self.indices.extend_from_slice(&[v0, v2, v3, v0, v3, v1]);
                }
            }

            // 3. Incandescent comet coma
            let coma_r = 22.0 * self.scale * c.brightness;
            self.push_oriented_quad(
                head,
                [1.0, 0.0],
                [0.0, 1.0],
                coma_r,
                coma_r,
                [-1.0, -1.0],
                [1.0, 1.0],
                [0.85, 0.96, 1.0, fade * 1.5],
                KIND_JWST_STAR,
                0.8,
            );
        }
    }

    pub fn push_meteors(&mut self, meteors: &[Meteor]) {
        for m in meteors {
            if m.age < 0.0 || m.age >= m.max_age {
                continue;
            }
            let progress = m.age / m.max_age;
            let env = (progress * 5.0).min(1.0) * (1.0 - progress).powf(1.2);
            if env < 0.02 {
                continue;
            }
            let head = self.to_screen(m.pos);
            let spd = m.vel[0].hypot(m.vel[1]).max(1.0);
            let ux = m.vel[0] / spd;
            let uy = m.vel[1] / spd;
            let len = m.length * self.scale;
            let tail = [head[0] - ux * len, head[1] - uy * len];
            self.push_segment(
                head,
                tail,
                2.6 * self.scale,
                [0.92, 0.97, 1.0, env * m.brightness * 0.95],
                KIND_RIBBON,
                0.0,
            );
        }
    }

    /// Delicate Astral Cartography overlay: constellation threads, protostar reticles, pulsar crosshair.
    pub fn push_star_chart(
        &mut self,
        stars: &[NurseryStar],
        edges: &[ConstellationEdge],
        pulsar: &Pulsar,
        pal: &NebulaVisualPalette,
        chart_alpha: f32,
        time: f32,
    ) {
        if chart_alpha < 0.005 {
            return;
        }
        let accent = pal.chart_accent;

        // 1. Constellation threads between nearby mature stars
        for e in edges {
            if e.a >= stars.len() || e.b >= stars.len() {
                continue;
            }
            let a = self.to_screen(stars[e.a].pos);
            let b = self.to_screen(stars[e.b].pos);
            let alpha = chart_alpha * e.strength * 0.42;
            self.push_segment(
                a,
                b,
                1.45 * self.scale,
                [accent[0], accent[1], accent[2], alpha],
                KIND_CHART_LINE,
                0.0,
            );
        }

        // 2. Subtle astrolabe rings around protostars & supergiants
        for s in stars {
            let ring_r = match s.stage {
                StarStage::Protostar { collapse } => Some((32.0 - 10.0 * collapse) * self.scale),
                StarStage::Supergiant { .. } => Some(36.0 * self.scale),
                StarStage::MainSequence => None,
            };
            if let Some(r) = ring_r {
                let c = self.to_screen(s.pos);
                let segs = 18usize;
                for i in 0..segs {
                    let a0 = (i as f32 / segs as f32) * TAU + time * 0.25;
                    let a1 = ((i + 1) as f32 / segs as f32) * TAU + time * 0.25;
                    let p0 = [c[0] + a0.cos() * r, c[1] + a0.sin() * r];
                    let p1 = [c[0] + a1.cos() * r, c[1] + a1.sin() * r];
                    self.push_segment(
                        p0,
                        p1,
                        1.25 * self.scale,
                        [accent[0], accent[1], accent[2], chart_alpha * 0.52],
                        KIND_CHART_LINE,
                        1.0,
                    );
                }
            }
        }

        // 3. Delicate bearing reticle around the pulsar
        let pc = self.to_screen(pulsar.pos);
        let pr = 26.0 * self.scale;
        for k in 0..4 {
            let a = (k as f32) * (TAU * 0.25) + pulsar.angle * 0.5;
            let p0 = [pc[0] + a.cos() * (pr * 0.65), pc[1] + a.sin() * (pr * 0.65)];
            let p1 = [pc[0] + a.cos() * (pr * 1.25), pc[1] + a.sin() * (pr * 1.25)];
            self.push_segment(
                p0,
                p1,
                1.35 * self.scale,
                [
                    pal.star_cool[0],
                    pal.star_cool[1],
                    pal.star_cool[2],
                    chart_alpha * 0.6,
                ],
                KIND_CHART_LINE,
                0.0,
            );
        }
    }
}

#[inline]
fn blend_star_color(pal: &NebulaVisualPalette, t: f32) -> [f32; 3] {
    let s = t.clamp(0.0, 1.0);
    // Warm amber -> crisp ivory -> icy O-class blue
    if s < 0.5 {
        let k = s * 2.0;
        [
            pal.star_warm[0] + (1.0 - pal.star_warm[0]) * k,
            pal.star_warm[1] + (0.98 - pal.star_warm[1]) * k,
            pal.star_warm[2] + (0.96 - pal.star_warm[2]) * k,
        ]
    } else {
        let k = (s - 0.5) * 2.0;
        [
            1.0 + (pal.star_cool[0] - 1.0) * k,
            0.98 + (pal.star_cool[1] - 0.98) * k,
            0.96 + (pal.star_cool[2] - 0.96) * k,
        ]
    }
}
