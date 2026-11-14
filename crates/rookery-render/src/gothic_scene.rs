//! Gothic Metropolis Scene (`gothic-metropolis`).
//!
//! Renders an exaggerated 1990s-Batman-style (Anton Furst / Tim Burton) gothic city in a
//! simplified 3D isometric art style, floating in a stark black void and occupying roughly
//! three-fifths (~60%) of the viewport.
//!
//! Features 5 vertical layers of action:
//! 1. Deep canyon avenues with long-hood noir coupes, armored cruisers, and steam grates.
//! 2. Multi-level elevated gothic walkways/skybridges with strolling pedestrians.
//! 3. Elevated steel-and-stone train viaduct looping through the city and ViaductStation.
//! 4. Towering gothic setbacks, cathedral spires, flying buttresses, clockwork belfries,
//!    rising industrial steam plumes, and sweeping rooftop searchlights.
//! 5. High-altitude gothic patrol airships (dirigibles) with downward spotlights.

use std::f32::consts::{PI, TAU};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::gothic::{
    BuildingArchetype, BuildingPlacement, CityBlueprint, GothicMetropolisState, GothicPalette,
    GothicVehicleKind, WalkwaySpan, WalkwayStyle,
};
use wgpu::util::DeviceExt;

use crate::audio::AudioEngine;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const MAX_DYNAMIC_VERTS: usize = 65_536;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GothicVertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    emissive: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GothicUniforms {
    view_proj: [[f32; 4]; 4],
    light_and_time: [f32; 4],
    bounce_and_beam: [f32; 4],
}

#[derive(Clone, Copy)]
struct GothicThemeColors {
    plinth_dark: [f32; 4],
    plinth_trim: [f32; 4],
    asphalt: [f32; 4],
    sidewalk: [f32; 4],
    stone_primary: [f32; 4],
    stone_secondary: [f32; 4],
    stone_dark: [f32; 4],
    accent_trim: [f32; 4],
    copper_roof: [f32; 4],
    steel_viaduct: [f32; 4],
    window_warm: [f32; 4],
    window_dim: [f32; 4],
    beacon_red: [f32; 4],
    searchlight: [f32; 4],
    steam: [f32; 4],
    bounce_rgb: [f32; 3],
    key_light_dir: [f32; 3],
}

fn palette_theme(palette: GothicPalette) -> GothicThemeColors {
    match palette {
        GothicPalette::GothamAmber => GothicThemeColors {
            plinth_dark: [0.07, 0.07, 0.09, 1.0],
            plinth_trim: [0.14, 0.13, 0.16, 1.0],
            asphalt: [0.05, 0.05, 0.07, 1.0],
            sidewalk: [0.15, 0.14, 0.17, 1.0],
            stone_primary: [0.18, 0.17, 0.21, 1.0],
            stone_secondary: [0.24, 0.22, 0.26, 1.0],
            stone_dark: [0.11, 0.11, 0.14, 1.0],
            accent_trim: [0.36, 0.30, 0.22, 1.0],
            copper_roof: [0.16, 0.22, 0.21, 1.0],
            steel_viaduct: [0.20, 0.19, 0.23, 1.0],
            window_warm: [0.98, 0.74, 0.28, 1.0],
            window_dim: [0.72, 0.46, 0.16, 1.0],
            beacon_red: [0.96, 0.20, 0.18, 1.0],
            searchlight: [1.00, 0.86, 0.52, 0.16],
            steam: [0.68, 0.64, 0.60, 0.26],
            bounce_rgb: [0.95, 0.58, 0.18],
            key_light_dir: [-0.48, 0.76, 0.44],
        },
        GothicPalette::CathedralMoonlight => GothicThemeColors {
            plinth_dark: [0.06, 0.08, 0.12, 1.0],
            plinth_trim: [0.12, 0.16, 0.23, 1.0],
            asphalt: [0.04, 0.06, 0.09, 1.0],
            sidewalk: [0.13, 0.17, 0.24, 1.0],
            stone_primary: [0.16, 0.21, 0.30, 1.0],
            stone_secondary: [0.22, 0.28, 0.39, 1.0],
            stone_dark: [0.10, 0.13, 0.20, 1.0],
            accent_trim: [0.32, 0.40, 0.52, 1.0],
            copper_roof: [0.14, 0.26, 0.32, 1.0],
            steel_viaduct: [0.18, 0.23, 0.31, 1.0],
            window_warm: [0.95, 0.88, 0.68, 1.0],
            window_dim: [0.46, 0.68, 0.88, 1.0],
            beacon_red: [0.98, 0.22, 0.32, 1.0],
            searchlight: [0.62, 0.88, 1.00, 0.17],
            steam: [0.62, 0.74, 0.86, 0.26],
            bounce_rgb: [0.38, 0.68, 0.95],
            key_light_dir: [0.45, 0.78, 0.42],
        },
        GothicPalette::CrimsonDeco => GothicThemeColors {
            plinth_dark: [0.09, 0.06, 0.07, 1.0],
            plinth_trim: [0.18, 0.12, 0.14, 1.0],
            asphalt: [0.06, 0.04, 0.05, 1.0],
            sidewalk: [0.17, 0.13, 0.14, 1.0],
            stone_primary: [0.21, 0.15, 0.17, 1.0],
            stone_secondary: [0.28, 0.20, 0.21, 1.0],
            stone_dark: [0.13, 0.09, 0.10, 1.0],
            accent_trim: [0.48, 0.34, 0.20, 1.0],
            copper_roof: [0.26, 0.14, 0.16, 1.0],
            steel_viaduct: [0.24, 0.18, 0.19, 1.0],
            window_warm: [1.00, 0.70, 0.30, 1.0],
            window_dim: [0.86, 0.28, 0.24, 1.0],
            beacon_red: [1.00, 0.18, 0.22, 1.0],
            searchlight: [1.00, 0.46, 0.38, 0.16],
            steam: [0.76, 0.58, 0.58, 0.26],
            bounce_rgb: [0.92, 0.28, 0.22],
            key_light_dir: [-0.52, 0.74, 0.42],
        },
        GothicPalette::EmeraldChem => GothicThemeColors {
            plinth_dark: [0.06, 0.09, 0.08, 1.0],
            plinth_trim: [0.12, 0.18, 0.16, 1.0],
            asphalt: [0.04, 0.06, 0.05, 1.0],
            sidewalk: [0.13, 0.19, 0.16, 1.0],
            stone_primary: [0.16, 0.22, 0.19, 1.0],
            stone_secondary: [0.22, 0.30, 0.25, 1.0],
            stone_dark: [0.09, 0.14, 0.12, 1.0],
            accent_trim: [0.34, 0.42, 0.26, 1.0],
            copper_roof: [0.16, 0.34, 0.26, 1.0],
            steel_viaduct: [0.18, 0.25, 0.21, 1.0],
            window_warm: [0.96, 0.80, 0.32, 1.0],
            window_dim: [0.32, 0.88, 0.54, 1.0],
            beacon_red: [0.42, 1.00, 0.52, 1.0],
            searchlight: [0.48, 0.98, 0.66, 0.16],
            steam: [0.56, 0.82, 0.66, 0.27],
            bounce_rgb: [0.28, 0.86, 0.48],
            key_light_dir: [-0.44, 0.77, 0.46],
        },
    }
}

/// Low-poly 3D mesh builder for simplified gothic isometric geometry.
struct MeshBuilder {
    verts: Vec<GothicVertex>,
}

impl MeshBuilder {
    fn with_capacity(cap: usize) -> Self {
        Self {
            verts: Vec::with_capacity(cap),
        }
    }

    fn push_tri(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        emissive: f32,
    ) {
        self.verts.push(GothicVertex {
            pos: a,
            normal,
            color,
            emissive,
        });
        self.verts.push(GothicVertex {
            pos: b,
            normal,
            color,
            emissive,
        });
        self.verts.push(GothicVertex {
            pos: c,
            normal,
            color,
            emissive,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn push_quad(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        d: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        emissive: f32,
    ) {
        self.push_tri(a, b, c, normal, color, emissive);
        self.push_tri(a, c, d, normal, color, emissive);
    }

    /// Axis-aligned 3D box centered at `(cx, _, cz)` with base at `y0` and top at `y1`.
    #[allow(clippy::too_many_arguments)]
    fn push_box(
        &mut self,
        cx: f32,
        y0: f32,
        cz: f32,
        w: f32,
        h: f32,
        d: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let hx = w * 0.5;
        let hz = d * 0.5;
        let x0 = cx - hx;
        let x1 = cx + hx;
        let z0 = cz - hz;
        let z1 = cz + hz;
        let y1 = y0 + h;

        // Top (+Y)
        self.push_quad(
            [x0, y1, z0],
            [x0, y1, z1],
            [x1, y1, z1],
            [x1, y1, z0],
            [0.0, 1.0, 0.0],
            color,
            emissive,
        );
        // Front (+Z)
        self.push_quad(
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
            [0.0, 0.0, 1.0],
            color,
            emissive,
        );
        // Back (-Z)
        self.push_quad(
            [x1, y0, z0],
            [x0, y0, z0],
            [x0, y1, z0],
            [x1, y1, z0],
            [0.0, 0.0, -1.0],
            color,
            emissive,
        );
        // Right (+X)
        self.push_quad(
            [x1, y0, z1],
            [x1, y0, z0],
            [x1, y1, z0],
            [x1, y1, z1],
            [1.0, 0.0, 0.0],
            color,
            emissive,
        );
        // Left (-X)
        self.push_quad(
            [x0, y0, z0],
            [x0, y0, z1],
            [x0, y1, z1],
            [x0, y1, z0],
            [-1.0, 0.0, 0.0],
            color,
            emissive,
        );
    }

    /// Oriented 3D box along horizontal unit forward direction `fwd = [fx, fz]`.
    #[allow(clippy::too_many_arguments)]
    fn push_oriented_box(
        &mut self,
        center: [f32; 3],
        fwd: [f32; 2],
        length: f32,
        height: f32,
        width: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let fx = fwd[0];
        let fz = fwd[1];
        let rx = -fz;
        let rz = fx;

        let hl = length * 0.5;
        let hw = width * 0.5;
        let y0 = center[1];
        let y1 = center[1] + height;

        let pt = |l: f32, w: f32, y: f32| -> [f32; 3] {
            [center[0] + fx * l + rx * w, y, center[2] + fz * l + rz * w]
        };

        let f_l_b = pt(hl, -hw, y0);
        let f_r_b = pt(hl, hw, y0);
        let f_r_t = pt(hl, hw, y1);
        let f_l_t = pt(hl, -hw, y1);

        let b_l_b = pt(-hl, -hw, y0);
        let b_r_b = pt(-hl, hw, y0);
        let b_r_t = pt(-hl, hw, y1);
        let b_l_t = pt(-hl, -hw, y1);

        // Top
        self.push_quad(b_l_t, f_l_t, f_r_t, b_r_t, [0.0, 1.0, 0.0], color, emissive);
        // Front (+fwd)
        self.push_quad(f_l_b, f_r_b, f_r_t, f_l_t, [fx, 0.0, fz], color, emissive);
        // Back (-fwd)
        self.push_quad(b_r_b, b_l_b, b_l_t, b_r_t, [-fx, 0.0, -fz], color, emissive);
        // Right (+right)
        self.push_quad(f_r_b, b_r_b, b_r_t, f_r_t, [rx, 0.0, rz], color, emissive);
        // Left (-right)
        self.push_quad(b_l_b, f_l_b, f_l_t, b_l_t, [-rx, 0.0, -rz], color, emissive);
    }

    /// 4-sided pyramid / gothic spire centered at `(cx, cz)` from `y0` to `y0 + h`.
    #[allow(clippy::too_many_arguments)]
    fn push_pyramid(
        &mut self,
        cx: f32,
        y0: f32,
        cz: f32,
        w: f32,
        h: f32,
        d: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let hx = w * 0.5;
        let hz = d * 0.5;
        let apex = [cx, y0 + h, cz];
        let p00 = [cx - hx, y0, cz - hz];
        let p10 = [cx + hx, y0, cz - hz];
        let p11 = [cx + hx, y0, cz + hz];
        let p01 = [cx - hx, y0, cz + hz];

        self.push_tri(p01, p11, apex, [0.0, 0.45, 0.89], color, emissive);
        self.push_tri(p10, p00, apex, [0.0, 0.45, -0.89], color, emissive);
        self.push_tri(p11, p10, apex, [0.89, 0.45, 0.0], color, emissive);
        self.push_tri(p00, p01, apex, [-0.89, 0.45, 0.0], color, emissive);
    }

    /// Tapered 4-sided frustum block (for stepped bases, tapered towers, and sloped buttresses).
    #[allow(clippy::too_many_arguments)]
    fn push_frustum(
        &mut self,
        cx: f32,
        y0: f32,
        cz: f32,
        w0: f32,
        d0: f32,
        w1: f32,
        d1: f32,
        h: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let hx0 = w0 * 0.5;
        let hz0 = d0 * 0.5;
        let hx1 = w1 * 0.5;
        let hz1 = d1 * 0.5;
        let y1 = y0 + h;

        let b00 = [cx - hx0, y0, cz - hz0];
        let b10 = [cx + hx0, y0, cz - hz0];
        let b11 = [cx + hx0, y0, cz + hz0];
        let b01 = [cx - hx0, y0, cz + hz0];

        let t00 = [cx - hx1, y1, cz - hz1];
        let t10 = [cx + hx1, y1, cz - hz1];
        let t11 = [cx + hx1, y1, cz + hz1];
        let t01 = [cx - hx1, y1, cz + hz1];

        // Top
        self.push_quad(t00, t01, t11, t10, [0.0, 1.0, 0.0], color, emissive);
        // Bottom
        self.push_quad(b00, b10, b11, b01, [0.0, -1.0, 0.0], color, emissive);
        // Front (+Z)
        self.push_quad(b01, b11, t11, t01, [0.0, 0.2, 0.98], color, emissive);
        // Back (-Z)
        self.push_quad(b10, b00, t00, t10, [0.0, 0.2, -0.98], color, emissive);
        // Right (+X)
        self.push_quad(b11, b10, t10, t11, [0.98, 0.2, 0.0], color, emissive);
        // Left (-X)
        self.push_quad(b00, b01, t01, t00, [-0.98, 0.2, 0.0], color, emissive);
    }

    /// Translucent 3D volumetric cone from `apex` along `dir` of `length` and `end_radius`.
    fn push_light_cone(
        &mut self,
        apex: [f32; 3],
        dir: [f32; 3],
        length: f32,
        end_radius: f32,
        color: [f32; 4],
    ) {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
            .sqrt()
            .max(0.001);
        let dx = dir[0] / len;
        let dy = dir[1] / len;
        let dz = dir[2] / len;

        // Orthonormal basis `(u, v)` perpendicular to `d`
        let up = if dy.abs() < 0.92 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let ux = up[1] * dz - up[2] * dy;
        let uy = up[2] * dx - up[0] * dz;
        let uz = up[0] * dy - up[1] * dx;
        let ulen = (ux * ux + uy * uy + uz * uz).sqrt().max(0.001);
        let u = [ux / ulen, uy / ulen, uz / ulen];
        let v = [
            dy * u[2] - dz * u[1],
            dz * u[0] - dx * u[2],
            dx * u[1] - dy * u[0],
        ];

        let mid_len = length * 0.52;
        let mid_radius = end_radius * 0.52;
        let mid_center = [
            apex[0] + dx * mid_len,
            apex[1] + dy * mid_len,
            apex[2] + dz * mid_len,
        ];
        let base_center = [
            apex[0] + dx * length,
            apex[1] + dy * length,
            apex[2] + dz * length,
        ];

        const SIDES: usize = 10;
        let tip_color = [color[0], color[1], color[2], color[3] * 1.35];
        let mid_color = [color[0], color[1], color[2], color[3] * 0.42];
        let fade_color = [color[0], color[1], color[2], 0.0];

        for i in 0..SIDES {
            let a0 = (i as f32 / SIDES as f32) * TAU;
            let a1 = ((i + 1) as f32 / SIDES as f32) * TAU;
            let m0 = [
                mid_center[0] + (u[0] * a0.cos() + v[0] * a0.sin()) * mid_radius,
                mid_center[1] + (u[1] * a0.cos() + v[1] * a0.sin()) * mid_radius,
                mid_center[2] + (u[2] * a0.cos() + v[2] * a0.sin()) * mid_radius,
            ];
            let m1 = [
                mid_center[0] + (u[0] * a1.cos() + v[0] * a1.sin()) * mid_radius,
                mid_center[1] + (u[1] * a1.cos() + v[1] * a1.sin()) * mid_radius,
                mid_center[2] + (u[2] * a1.cos() + v[2] * a1.sin()) * mid_radius,
            ];
            let p0 = [
                base_center[0] + (u[0] * a0.cos() + v[0] * a0.sin()) * end_radius,
                base_center[1] + (u[1] * a0.cos() + v[1] * a0.sin()) * end_radius,
                base_center[2] + (u[2] * a0.cos() + v[2] * a0.sin()) * end_radius,
            ];
            let p1 = [
                base_center[0] + (u[0] * a1.cos() + v[0] * a1.sin()) * end_radius,
                base_center[1] + (u[1] * a1.cos() + v[1] * a1.sin()) * end_radius,
                base_center[2] + (u[2] * a1.cos() + v[2] * a1.sin()) * end_radius,
            ];

            // Inner core cone slice (apex -> mid ring)
            self.verts.push(GothicVertex {
                pos: apex,
                normal: [0.0, 1.0, 0.0],
                color: tip_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: m0,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: m1,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
            });

            // Outer feather skirt (mid ring -> zero-alpha outer rim)
            self.verts.push(GothicVertex {
                pos: m0,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: p0,
                normal: [0.0, 1.0, 0.0],
                color: fade_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: p1,
                normal: [0.0, 1.0, 0.0],
                color: fade_color,
                emissive: 2.0,
            });

            self.verts.push(GothicVertex {
                pos: m0,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: p1,
                normal: [0.0, 1.0, 0.0],
                color: fade_color,
                emissive: 2.0,
            });
            self.verts.push(GothicVertex {
                pos: m1,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
            });
        }
    }
}

/// Build the complete static 3D city mesh from [`CityBlueprint`] and [`GothicThemeColors`].
fn build_static_city_mesh(bp: &CityBlueprint, theme: &GothicThemeColors) -> Vec<GothicVertex> {
    let mut mb = MeshBuilder::with_capacity(48_000);

    // 1. Stepped Citadel Foundation Plinth floating in the black void
    let ext = bp.plinth_half_extent;
    // Upper street deck (y = -0.25 .. 0.0)
    mb.push_box(
        0.0,
        -0.25,
        0.0,
        ext * 2.0,
        0.25,
        ext * 2.0,
        theme.sidewalk,
        0.0,
    );
    // Outer parapet wall around the city island perimeter
    let wall_t = 0.22;
    let wall_h = 0.28;
    mb.push_box(
        0.0,
        0.0,
        -ext + wall_t * 0.5,
        ext * 2.0,
        wall_h,
        wall_t,
        theme.plinth_trim,
        0.0,
    );
    mb.push_box(
        0.0,
        0.0,
        ext - wall_t * 0.5,
        ext * 2.0,
        wall_h,
        wall_t,
        theme.plinth_trim,
        0.0,
    );
    mb.push_box(
        -ext + wall_t * 0.5,
        0.0,
        0.0,
        wall_t,
        wall_h,
        ext * 2.0,
        theme.plinth_trim,
        0.0,
    );
    mb.push_box(
        ext - wall_t * 0.5,
        0.0,
        0.0,
        wall_t,
        wall_h,
        ext * 2.0,
        theme.plinth_trim,
        0.0,
    );

    // Tapered under-city substructure fading into the black abyss
    mb.push_frustum(
        0.0,
        -1.05,
        0.0,
        ext * 1.84,
        ext * 1.84,
        ext * 1.98,
        ext * 1.98,
        0.80,
        theme.plinth_dark,
        0.0,
    );
    mb.push_frustum(
        0.0,
        -1.85,
        0.0,
        ext * 1.45,
        ext * 1.45,
        ext * 1.84,
        ext * 1.84,
        0.80,
        theme.plinth_dark,
        0.0,
    );

    // 2. Deep Canyon Street Avenues & Glowing Gothic Streetlamps
    for ave in &bp.avenues {
        let dx = ave.end[0] - ave.start[0];
        let dz = ave.end[1] - ave.start[1];
        let len = (dx * dx + dz * dz).sqrt().max(0.1);
        let cx = (ave.start[0] + ave.end[0]) * 0.5;
        let cz = (ave.start[1] + ave.end[1]) * 0.5;
        let fwd = [dx / len, dz / len];

        // Dark wet-look asphalt road surface slightly above plinth
        mb.push_oriented_box(
            [cx, 0.005, cz],
            fwd,
            len,
            0.02,
            ave.width,
            theme.asphalt,
            0.0,
        );

        // Place miniature gothic streetlamps along both sides of the avenue
        let steps = 7;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let lx = ave.start[0] + dx * t;
            let lz = ave.start[1] + dz * t;
            // Skip intersections
            if ((lx - (-2.4)).abs() < 0.75 || (lx - 2.1).abs() < 0.75)
                && ((lz - (-2.4)).abs() < 0.75 || (lz - 2.1).abs() < 0.75)
            {
                continue;
            }
            for side in [-1.0f32, 1.0f32] {
                let px = lx + (-fwd[1]) * (ave.width * 0.58) * side;
                let pz = lz + (fwd[0]) * (ave.width * 0.58) * side;
                mb.push_box(px, 0.0, pz, 0.06, 0.36, 0.06, theme.stone_dark, 0.0);
                mb.push_box(px, 0.36, pz, 0.11, 0.10, 0.11, theme.window_warm, 1.0);
            }
        }
    }

    // 3. Modular Gothic Buildings
    for b in &bp.buildings {
        build_archetype_mesh(&mut mb, b, theme);
    }

    // 4. Multi-Level Elevated Walkways / Skybridges
    for span in &bp.walkways {
        build_walkway_mesh(&mut mb, span, theme);
    }

    // 5. Elevated Train Viaduct Loop & Support Arches
    build_viaduct_mesh(&mut mb, bp, theme);

    mb.verts
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
    let usable_d = (d - pad_z * 2.0).max(0.2);
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

fn build_archetype_mesh(mb: &mut MeshBuilder, b: &BuildingPlacement, theme: &GothicThemeColors) {
    let cx = b.x;
    let cz = b.z;
    let w = b.width;
    let d = b.depth;
    let h = b.height;

    match b.archetype {
        BuildingArchetype::CathedralKeep => {
            // 1. Heavy buttressed podium base
            let h_base = h * 0.34;
            mb.push_box(cx, 0.0, cz, w, h_base, d, theme.stone_dark, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                0.0,
                cz,
                w,
                h_base,
                d,
                theme.stone_secondary,
                theme.accent_trim,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.2,
                cz,
                w,
                h_base - 0.3,
                d,
                4,
                3,
                b.style_variant,
                theme,
            );

            // 2. Flying buttress fins flanking the mid nave
            let h_nave = h * 0.42;
            let nave_w = w * 0.72;
            let nave_d = d * 0.72;
            mb.push_box(
                cx,
                h_base,
                cz,
                nave_w,
                h_nave,
                nave_d,
                theme.stone_primary,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                h_base,
                cz,
                nave_w,
                h_nave,
                nave_d,
                3,
                5,
                b.style_variant + 1,
                theme,
            );

            for side in [-1.0f32, 1.0f32] {
                for offset in [-0.55f32, 0.55f32] {
                    // Outer buttress pier + sloped flying arch strut
                    mb.push_box(
                        cx + side * (w * 0.46),
                        h_base,
                        cz + offset * d * 0.38,
                        0.18,
                        h_nave * 0.72,
                        0.22,
                        theme.stone_secondary,
                        0.0,
                    );
                    mb.push_box(
                        cx + side * (w * 0.38),
                        h_base + h_nave * 0.55,
                        cz + offset * d * 0.38,
                        w * 0.22,
                        0.14,
                        0.16,
                        theme.accent_trim,
                        0.0,
                    );
                    // Pinnacle tip on outer pier
                    mb.push_pyramid(
                        cx + side * (w * 0.46),
                        h_base + h_nave * 0.72,
                        cz + offset * d * 0.38,
                        0.22,
                        0.55,
                        0.22,
                        theme.copper_roof,
                        0.0,
                    );
                }
            }

            // 3. Twin Gothic Belfry Towers + Rose Window Facade
            let y_top = h_base + h_nave;
            let tower_h = h * 0.24;
            let tw = nave_w * 0.36;
            for sx in [-1.0f32, 1.0f32] {
                let tx = cx + sx * (nave_w * 0.34);
                mb.push_box(
                    tx,
                    y_top,
                    cz,
                    tw,
                    tower_h,
                    nave_d * 0.82,
                    theme.stone_secondary,
                    0.0,
                );
                mb.push_pyramid(
                    tx,
                    y_top + tower_h,
                    cz,
                    tw * 1.08,
                    h * 0.22,
                    nave_d * 0.86,
                    theme.copper_roof,
                    0.0,
                );
            }

            // Glowing Rose Window on front & back of nave
            let rose_y = h_base + h_nave * 0.58;
            mb.push_box(
                cx,
                rose_y,
                cz + nave_d * 0.51,
                0.48,
                0.48,
                0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_box(
                cx,
                rose_y,
                cz - nave_d * 0.51,
                0.48,
                0.48,
                0.04,
                theme.window_warm,
                1.0,
            );

            // 4. Soaring Central Needle Spire + Red Warning Beacon
            let spire_base = y_top;
            let spire_h = h * 0.36;
            mb.push_pyramid(
                cx,
                spire_base,
                cz,
                nave_w * 0.38,
                spire_h,
                nave_d * 0.38,
                theme.copper_roof,
                0.0,
            );
            mb.push_box(
                cx,
                spire_base + spire_h,
                cz,
                0.12,
                0.14,
                0.12,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ArtDecoZiggurat => {
            // 4-tier stepped Furst/Art-Deco skyscraper with dramatic vertical ribs and gargoyle ledges
            let tiers = [
                (0.0, 0.42, 1.00, 4, 4),
                (0.42, 0.28, 0.78, 3, 3),
                (0.70, 0.18, 0.56, 2, 2),
                (0.88, 0.12, 0.36, 1, 2),
            ];
            for (idx, &(y_frac, h_frac, scale, cols, rows)) in tiers.iter().enumerate() {
                let ty = y_frac * h;
                let th = h_frac * h;
                let tw = w * scale;
                let td = d * scale;
                let col = if idx % 2 == 0 {
                    theme.stone_primary
                } else {
                    theme.stone_secondary
                };
                mb.push_box(cx, ty, cz, tw, th, td, col, 0.0);
                push_gothic_ribs_and_cornice(
                    mb,
                    cx,
                    ty,
                    cz,
                    tw,
                    th,
                    td,
                    theme.stone_secondary,
                    theme.accent_trim,
                );
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

                // Jutting gargoyle/eagle ledges on the 4 corners of tiers 1 and 2
                if idx == 0 || idx == 1 {
                    let gy = ty + th - 0.08;
                    for sx in [-1.0f32, 1.0f32] {
                        for sz in [-1.0f32, 1.0f32] {
                            mb.push_box(
                                cx + sx * (tw * 0.56),
                                gy,
                                cz + sz * (td * 0.56),
                                0.24,
                                0.10,
                                0.24,
                                theme.accent_trim,
                                0.0,
                            );
                        }
                    }
                }
            }
            // Crown needle spire + glowing lantern
            mb.push_box(cx, h, cz, w * 0.20, 0.28, d * 0.20, theme.window_warm, 1.0);
            mb.push_pyramid(
                cx,
                h + 0.28,
                cz,
                w * 0.24,
                h * 0.22,
                d * 0.24,
                theme.copper_roof,
                0.0,
            );
        }

        BuildingArchetype::ClockworkBelfry => {
            // Slender campanile shaft
            let shaft_h = h * 0.74;
            mb.push_box(
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                theme.stone_primary,
                0.0,
            );
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                theme.stone_dark,
                theme.accent_trim,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.3,
                cz,
                w * 0.86,
                shaft_h - 0.5,
                d * 0.86,
                2,
                6,
                b.style_variant,
                theme,
            );

            // Flared Clock Chamber at top of shaft
            let clock_h = h * 0.18;
            let cw = w * 0.96;
            let cd = d * 0.96;
            mb.push_box(cx, shaft_h, cz, cw, clock_h, cd, theme.stone_secondary, 0.0);
            mb.push_box(
                cx,
                shaft_h + clock_h - 0.10,
                cz,
                cw + 0.20,
                0.12,
                cd + 0.20,
                theme.accent_trim,
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
                theme.stone_dark,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz + cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                theme.stone_dark,
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
                theme.stone_dark,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz - cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                theme.stone_dark,
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
                theme.stone_dark,
                0.0,
            );
            mb.push_box(
                cx + cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                theme.stone_dark,
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
                theme.stone_dark,
                0.0,
            );
            mb.push_box(
                cx - cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                theme.stone_dark,
                0.0,
            );

            // Steep belfry spire roof + tall needle finial + corner pinnacles
            let roof_y = shaft_h + clock_h;
            mb.push_pyramid(
                cx,
                roof_y,
                cz,
                cw * 0.90,
                h * 0.34,
                cd * 0.90,
                theme.copper_roof,
                0.0,
            );
            mb.push_pyramid(
                cx,
                roof_y + h * 0.30,
                cz,
                cw * 0.18,
                h * 0.18,
                cd * 0.18,
                theme.accent_trim,
                0.0,
            );
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    mb.push_pyramid(
                        cx + sx * (cw * 0.42),
                        roof_y,
                        cz + sz * (cd * 0.42),
                        0.20,
                        0.48,
                        0.20,
                        theme.accent_trim,
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
                theme.stone_dark,
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
                theme.stone_primary,
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
            // Brutalist-gothic corporate citadel with cantilevered skybridge portals
            let h1 = h * 0.58;
            let h2 = h * 0.30;
            let h3 = h * 0.12;
            mb.push_box(cx, 0.0, cz, w, h1, d, theme.stone_primary, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                0.0,
                cz,
                w,
                h1,
                d,
                theme.stone_dark,
                theme.accent_trim,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.2,
                cz,
                w,
                h1 - 0.3,
                d,
                4,
                5,
                b.style_variant,
                theme,
            );

            mb.push_box(
                cx,
                h1,
                cz,
                w * 0.82,
                h2,
                d * 0.82,
                theme.stone_secondary,
                0.0,
            );
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                h1,
                cz,
                w * 0.82,
                h2,
                d * 0.82,
                theme.stone_dark,
                theme.accent_trim,
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

            mb.push_box(
                cx,
                h1 + h2,
                cz,
                w * 0.56,
                h3,
                d * 0.56,
                theme.stone_dark,
                0.0,
            );
            mb.push_pyramid(
                cx,
                h,
                cz,
                w * 0.52,
                h * 0.16,
                d * 0.52,
                theme.copper_roof,
                0.0,
            );
        }

        BuildingArchetype::ViaductStation => {
            // Grand Gothic Transit Station straddling the elevated train track at y = 2.6!
            // Build West and East flanking pier walls so the center (x = cx) has an open portal
            // tunnel from y = 2.4 to y = 3.85!
            let pier_w = w * 0.28;
            let portal_top_y = 3.85;
            // Lower solid station base below track level (0.0 .. 2.55)
            mb.push_box(cx, 0.0, cz, w, 2.55, d, theme.stone_dark, 0.0);
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
                    theme.stone_secondary,
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
                theme.stone_primary,
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
                theme.stone_secondary,
                theme.accent_trim,
            );
            push_gothic_window_bands(
                mb,
                cx,
                portal_top_y,
                cz,
                w * 0.92,
                upper_h * 0.65,
                d * 0.86,
                3,
                2,
                b.style_variant,
                theme,
            );

            let top_y = portal_top_y + upper_h * 0.65;
            mb.push_pyramid(
                cx,
                top_y,
                cz,
                w * 0.68,
                upper_h * 0.55,
                d * 0.68,
                theme.copper_roof,
                0.0,
            );
        }
    }
}

fn build_walkway_mesh(mb: &mut MeshBuilder, span: &WalkwaySpan, theme: &GothicThemeColors) {
    let dx = span.end[0] - span.start[0];
    let dz = span.end[1] - span.start[1];
    let len = (dx * dx + dz * dz).sqrt().max(0.1);
    let cx = (span.start[0] + span.end[0]) * 0.5;
    let cz = (span.start[1] + span.end[1]) * 0.5;
    let fwd = [dx / len, dz / len];
    let right = [-fwd[1], fwd[0]];
    let y = span.elevation;

    // 1. Bridge deck slab
    mb.push_oriented_box(
        [cx, y - 0.10, cz],
        fwd,
        len + 0.16,
        0.10,
        span.width,
        theme.stone_secondary,
        0.0,
    );

    // 2. Left and right gothic parapet railings
    let rail_w = 0.05;
    let rail_h = 0.14;
    for side in [-1.0f32, 1.0f32] {
        let rx = cx + right[0] * (span.width * 0.5 - rail_w * 0.5) * side;
        let rz = cz + right[1] * (span.width * 0.5 - rail_w * 0.5) * side;
        mb.push_oriented_box(
            [rx, y, rz],
            fwd,
            len + 0.12,
            rail_h,
            rail_w,
            theme.accent_trim,
            0.0,
        );
    }

    // 3. Warm glowing portal doorways at both ends where the walkway enters the buildings
    for end_pt in [span.start, span.end] {
        mb.push_oriented_box(
            [end_pt[0], y, end_pt[1]],
            fwd,
            0.12,
            0.34,
            span.width * 0.68,
            theme.window_warm,
            0.95,
        );
    }

    // 4. Style-specific arch ribs or overhead iron truss frames
    match span.style {
        WalkwayStyle::StoneArchBridge => {
            // Underneath stepped stone arch haunches at both ends
            for t in [0.18f32, 0.82f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                mb.push_oriented_box(
                    [px, y - 0.34, pz],
                    fwd,
                    len * 0.24,
                    0.24,
                    span.width * 0.82,
                    theme.stone_dark,
                    0.0,
                );
            }
        }
        WalkwayStyle::IronTrussSkybridge => {
            // Overhead iron portal frames along the bridge
            for t in [0.25f32, 0.50f32, 0.75f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                mb.push_oriented_box(
                    [px, y + 0.28, pz],
                    fwd,
                    0.06,
                    0.06,
                    span.width + 0.06,
                    theme.steel_viaduct,
                    0.0,
                );
            }
        }
        WalkwayStyle::CoveredGallery => {
            // Covered gothic gallery roof with side lancet openings
            mb.push_oriented_box(
                [cx, y + 0.32, cz],
                fwd,
                len + 0.10,
                0.10,
                span.width + 0.08,
                theme.copper_roof,
                0.0,
            );
        }
    }
}

fn build_viaduct_mesh(mb: &mut MeshBuilder, bp: &CityBlueprint, theme: &GothicThemeColors) {
    let n = bp.viaduct_waypoints.len();
    if n < 2 {
        return;
    }
    let y = bp.viaduct_elevation;
    let deck_w = 0.68;

    for i in 0..n {
        let a = bp.viaduct_waypoints[i];
        let b = bp.viaduct_waypoints[(i + 1) % n];
        let dx = b[0] - a[0];
        let dz = b[1] - a[1];
        let len = (dx * dx + dz * dz).sqrt().max(0.1);
        let cx = (a[0] + b[0]) * 0.5;
        let cz = (a[1] + b[1]) * 0.5;
        let fwd = [dx / len, dz / len];
        let right = [-fwd[1], fwd[0]];

        // Main steel viaduct girder deck
        mb.push_oriented_box(
            [cx, y - 0.16, cz],
            fwd,
            len + 0.52,
            0.16,
            deck_w,
            theme.steel_viaduct,
            0.0,
        );

        // Twin raised steel rails + outer guard girders
        for side in [-1.0f32, 1.0f32] {
            let rx = cx + right[0] * (deck_w * 0.44) * side;
            let rz = cz + right[1] * (deck_w * 0.44) * side;
            mb.push_oriented_box(
                [rx, y, rz],
                fwd,
                len + 0.44,
                0.09,
                0.07,
                theme.accent_trim,
                0.0,
            );
        }

        // Support columns descending from viaduct to street level (skipping inside ViaductStation)
        let pier_steps = (len / 2.2).ceil().max(1.0) as usize;
        for p in 0..pier_steps {
            let t = (p as f32 + 0.5) / pier_steps as f32;
            let px = a[0] + dx * t;
            let pz = a[1] + dz * t;
            // Skip piers inside ViaductStation (around x = -4.6, z = -0.2)
            if (px - (-4.6)).abs() < 1.3 && (pz - (-0.2)).abs() < 1.6 {
                continue;
            }
            for side in [-1.0f32, 1.0f32] {
                let col_x = px + right[0] * (deck_w * 0.36) * side;
                let col_z = pz + right[1] * (deck_w * 0.36) * side;
                mb.push_box(
                    col_x,
                    0.0,
                    col_z,
                    0.14,
                    y - 0.16,
                    0.14,
                    theme.stone_dark,
                    0.0,
                );
            }
            // Cross arch header under the viaduct deck
            mb.push_oriented_box(
                [px, y - 0.38, pz],
                fwd,
                0.22,
                0.22,
                deck_w * 0.92,
                theme.stone_secondary,
                0.0,
            );
        }
    }
}

/// Build the orthographic isometric 4x4 column-major view-projection matrix.
///
/// Configured so the city island occupies roughly **three-fifths (~60%)** of the viewport
/// centered in a stark black void.
fn build_isometric_view_proj(
    width: f32,
    height: f32,
    yaw: f32,
    pitch: f32,
    occupancy: f32,
) -> [[f32; 4]; 4] {
    let aspect = (width.max(1.0) / height.max(1.0)).clamp(0.4, 3.5);
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();

    // City horizontal radius is ~10.8 world units; vertical span is ~13.5 world units.
    // Sizing `scale_y` so that the projected city spans ~60% (three-fifths) of the screen:
    let occ = occupancy.clamp(0.40, 0.85);
    // Fit smoothly across both 16:9 widescreen and narrower aspect ratios
    let base_scale = occ / 9.2;
    let scale_y = base_scale * (aspect.min(1.65)).sqrt();
    let scale_x = scale_y / aspect;
    let scale_z = 1.0 / 56.0;

    // Look-at target near the vertical midpoint of the gothic skyline
    let target = [0.0f32, 3.65f32, 0.0f32];

    // Row 0: vx = scale_x * (x * cy - z * sy)
    // Row 1: vy = scale_y * (y * cp - (x * sy + z * cy) * sp)
    // Row 2: vz = scale_z * (-(y * sp + (x * sy + z * cy) * cp)) + 0.5
    let r0 = [scale_x * cy, 0.0, -scale_x * sy, 0.0];
    let r1 = [-scale_y * sy * sp, scale_y * cp, -scale_y * cy * sp, 0.0];
    let r2 = [-scale_z * sy * cp, -scale_z * sp, -scale_z * cy * cp, 0.0];

    let tx = -(r0[0] * target[0] + r0[1] * target[1] + r0[2] * target[2]);
    let ty = -(r1[0] * target[0] + r1[1] * target[1] + r1[2] * target[2]);
    let tz = -(r2[0] * target[0] + r2[1] * target[1] + r2[2] * target[2]) + 0.5;

    // Column-major layout for WGSL `mat4x4<f32>`
    [
        [r0[0], r1[0], r2[0], 0.0],
        [r0[1], r1[1], r2[1], 0.0],
        [r0[2], r1[2], r2[2], 0.0],
        [tx, ty, tz, 1.0],
    ]
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gothic-depth-tex"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

/// GPU scene for *Gothic Metropolis* (`gothic-metropolis`).
pub struct GothicMetropolisScene {
    pub state: Arc<Mutex<GothicMetropolisState>>,
    pub audio: Option<AudioEngine>,
    width: u32,
    height: u32,
    scale_factor: f64,

    opaque_pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,

    static_vbuf: wgpu::Buffer,
    static_vert_count: u32,
    cached_palette: GothicPalette,

    opaque_dyn_vbuf: wgpu::Buffer,
    opaque_dyn_count: u32,
    translucent_dyn_vbuf: wgpu::Buffer,
    translucent_dyn_count: u32,

    last_pointer: Option<Pointer>,
    is_dragging: bool,
}

impl GothicMetropolisScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        state: Arc<Mutex<GothicMetropolisState>>,
    ) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gothic-isometric-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gothic.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gothic-bind-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gothic-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GothicVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x4,
                3 => Float32,
            ],
        };

        let make_pipeline = |label: &str, depth_write: bool, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(vertex_layout.clone())],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let opaque_pipeline = make_pipeline("gothic-opaque-pipeline", true, None);
        let translucent_pipeline = make_pipeline(
            "gothic-translucent-pipeline",
            false,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-uniform-buf"),
            size: std::mem::size_of::<GothicUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gothic-bind-group"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let (init_palette, init_verts) = {
            let guard = state.lock().expect("gothic state lock");
            let theme = palette_theme(guard.palette);
            (
                guard.palette,
                build_static_city_mesh(&guard.blueprint, &theme),
            )
        };

        let static_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gothic-static-vbuf"),
            contents: bytemuck::cast_slice(&init_verts),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let static_vert_count = init_verts.len() as u32;

        let dyn_buf_bytes = (MAX_DYNAMIC_VERTS * std::mem::size_of::<GothicVertex>()) as u64;
        let opaque_dyn_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-opaque-dyn-vbuf"),
            size: dyn_buf_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let translucent_dyn_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gothic-translucent-dyn-vbuf"),
            size: dyn_buf_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_view = create_depth_texture(device, 1280, 720);

        let mut scene = Self {
            state,
            audio: None,
            width: 1280,
            height: 720,
            scale_factor: 1.0,
            opaque_pipeline,
            translucent_pipeline,
            uniform_buf,
            bind_group,
            depth_view,
            static_vbuf,
            static_vert_count,
            cached_palette: init_palette,
            opaque_dyn_vbuf,
            opaque_dyn_count: 0,
            translucent_dyn_vbuf,
            translucent_dyn_count: 0,
            last_pointer: None,
            is_dragging: false,
        };

        scene.rebuild_dynamic_buffers(gpu);
        scene
    }

    pub fn with_audio(mut self, audio: Option<AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn rebuild_dynamic_buffers(&mut self, gpu: &Gpu) {
        let Ok(guard) = self.state.lock() else {
            return;
        };
        let theme = palette_theme(guard.palette);

        // Rebuild static architecture mesh if palette changed
        if guard.palette != self.cached_palette {
            let new_static = build_static_city_mesh(&guard.blueprint, &theme);
            self.static_vbuf = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gothic-static-vbuf"),
                    contents: bytemuck::cast_slice(&new_static),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            self.static_vert_count = new_static.len() as u32;
            self.cached_palette = guard.palette;
        }

        // Update uniform buffer
        let view_proj = build_isometric_view_proj(
            self.width as f32,
            self.height as f32,
            guard.camera_yaw,
            guard.camera_pitch,
            guard.screen_occupancy,
        );
        let uniforms = GothicUniforms {
            view_proj,
            light_and_time: [
                theme.key_light_dir[0],
                theme.key_light_dir[1],
                theme.key_light_dir[2],
                guard.time,
            ],
            bounce_and_beam: [
                theme.bounce_rgb[0],
                theme.bounce_rgb[1],
                theme.bounce_rgb[2],
                if guard.searchlights_enabled { 1.0 } else { 0.0 },
            ],
        };
        gpu.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        // Build dynamic opaque & translucent geometry for all 5 vertical layers of action
        let mut opaque_mb = MeshBuilder::with_capacity(8192);
        let mut trans_mb = MeshBuilder::with_capacity(4096);

        // --- LAYER 1: Deep Canyon Street Vehicles & Headlight Cones ---
        for veh in &guard.vehicles {
            if let Some(ave) = guard.blueprint.avenues.get(veh.avenue_idx) {
                let dx = ave.end[0] - ave.start[0];
                let dz = ave.end[1] - ave.start[1];
                let len = (dx * dx + dz * dz).sqrt().max(0.1);
                let u_fwd = [dx / len, dz / len];
                let u_right = [-u_fwd[1], u_fwd[0]];
                let lane_offset = if veh.dir > 0.0 { 0.22 } else { -0.22 };

                let vx = ave.start[0] + dx * veh.progress + u_right[0] * lane_offset;
                let vz = ave.start[1] + dz * veh.progress + u_right[1] * lane_offset;
                let car_fwd = [u_fwd[0] * veh.dir, u_fwd[1] * veh.dir];

                let (car_len, car_w, car_h) = match veh.kind {
                    GothicVehicleKind::NoirCoupe => (0.58, 0.22, 0.14),
                    GothicVehicleKind::ArmoredCruiser => (0.64, 0.25, 0.18),
                    GothicVehicleKind::TransitOmnibus => (0.86, 0.26, 0.24),
                };

                // Sleek dark chassis + cabin
                opaque_mb.push_oriented_box(
                    [vx, 0.02, vz],
                    car_fwd,
                    car_len,
                    car_h * 0.58,
                    car_w,
                    theme.plinth_dark,
                    0.0,
                );
                opaque_mb.push_oriented_box(
                    [
                        vx - car_fwd[0] * (car_len * 0.10),
                        0.02 + car_h * 0.55,
                        vz - car_fwd[1] * (car_len * 0.10),
                    ],
                    car_fwd,
                    car_len * 0.48,
                    car_h * 0.45,
                    car_w * 0.84,
                    theme.stone_secondary,
                    0.0,
                );
                // Glowing twin headlights & ruby taillights
                opaque_mb.push_oriented_box(
                    [
                        vx + car_fwd[0] * (car_len * 0.50),
                        0.04,
                        vz + car_fwd[1] * (car_len * 0.50),
                    ],
                    car_fwd,
                    0.04,
                    0.05,
                    car_w * 0.76,
                    theme.window_warm,
                    1.0,
                );
                opaque_mb.push_oriented_box(
                    [
                        vx - car_fwd[0] * (car_len * 0.50),
                        0.04,
                        vz - car_fwd[1] * (car_len * 0.50),
                    ],
                    car_fwd,
                    0.04,
                    0.05,
                    car_w * 0.76,
                    theme.beacon_red,
                    1.0,
                );

                // Translucent headlight cone on the street asphalt ahead
                trans_mb.push_light_cone(
                    [
                        vx + car_fwd[0] * (car_len * 0.52),
                        0.06,
                        vz + car_fwd[1] * (car_len * 0.52),
                    ],
                    [car_fwd[0], -0.04, car_fwd[1]],
                    1.05,
                    0.26,
                    theme.searchlight,
                );
            }
        }

        // --- LAYER 2: Elevated Walkway Pedestrians ---
        for ped in &guard.pedestrians {
            if let Some(span) = guard.blueprint.walkways.get(ped.walkway_idx) {
                let dx = span.end[0] - span.start[0];
                let dz = span.end[1] - span.start[1];
                let len = (dx * dx + dz * dz).sqrt().max(0.1);
                let right = [-dz / len, dx / len];
                let base = span.point_at(ped.progress);
                let px = base[0] + right[0] * ped.lateral_offset;
                let pz = base[2] + right[1] * ped.lateral_offset;
                let py = base[1];

                let coat_color = match ped.coat_variant {
                    0 => theme.accent_trim,
                    1 => theme.stone_secondary,
                    _ => theme.window_dim,
                };
                // Stylized gothic trenchcoat silhouette + hat/lantern
                opaque_mb.push_box(px, py, pz, 0.08, 0.16, 0.08, coat_color, 0.0);
                opaque_mb.push_box(px, py + 0.16, pz, 0.06, 0.06, 0.06, theme.window_warm, 0.65);
            }
        }

        // --- LAYER 3: Articulated Elevated Gothic Trains ---
        let car_spacing = 0.88f32;
        for train in &guard.trains {
            for car_idx in 0..train.car_count {
                let dist = train.track_dist - (car_idx as f32) * car_spacing;
                let (pos, fwd) = guard.blueprint.sample_viaduct(dist);
                let is_loco = car_idx == 0;
                let is_caboose = car_idx + 1 == train.car_count;
                let car_len = if is_loco { 0.82 } else { 0.76 };
                let car_w = 0.44;
                let car_h = 0.34;

                // Lower steel bogie & body shell
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.02, pos[2]],
                    fwd,
                    car_len,
                    car_h,
                    car_w,
                    theme.stone_dark,
                    0.0,
                );
                // Warm glowing passenger window band
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.12, pos[2]],
                    fwd,
                    car_len * 0.90,
                    0.14,
                    car_w + 0.03,
                    theme.window_warm,
                    1.0,
                );
                // Arched gothic car roof + illuminated roof clerestory strip
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.02 + car_h, pos[2]],
                    fwd,
                    car_len * 0.94,
                    0.07,
                    car_w * 0.84,
                    theme.copper_roof,
                    0.0,
                );
                opaque_mb.push_oriented_box(
                    [pos[0], pos[1] + 0.09 + car_h, pos[2]],
                    fwd,
                    car_len * 0.76,
                    0.04,
                    car_w * 0.42,
                    theme.window_warm,
                    0.95,
                );

                if is_loco {
                    // Lead locomotive cyclops headlamp + volumetric beam along the viaduct
                    let nose = [
                        pos[0] + fwd[0] * (car_len * 0.52),
                        pos[1] + 0.20,
                        pos[2] + fwd[1] * (car_len * 0.52),
                    ];
                    opaque_mb.push_oriented_box(
                        nose,
                        fwd,
                        0.08,
                        0.12,
                        0.16,
                        theme.window_warm,
                        1.0,
                    );
                    trans_mb.push_light_cone(
                        nose,
                        [fwd[0], -0.05, fwd[1]],
                        2.15,
                        0.48,
                        theme.searchlight,
                    );
                }
                if is_caboose {
                    // Rear ruby tail marker lamps
                    let tail = [
                        pos[0] - fwd[0] * (car_len * 0.51),
                        pos[1] + 0.16,
                        pos[2] - fwd[1] * (car_len * 0.51),
                    ];
                    opaque_mb.push_oriented_box(
                        tail,
                        fwd,
                        0.05,
                        0.08,
                        car_w * 0.72,
                        theme.beacon_red,
                        1.0,
                    );
                }
            }
        }

        // --- LAYER 4: Rising Steam Plumes & Rooftop Searchlights ---
        for plume in &guard.steam_plumes {
            let life_frac = (plume.age / plume.max_age).clamp(0.0, 1.0);
            let alpha = (1.0 - life_frac) * (life_frac * 4.0).min(1.0) * theme.steam[3];
            let radius = plume.base_radius * (1.0 + life_frac * 1.85);
            let steam_col = [theme.steam[0], theme.steam[1], theme.steam[2], alpha];
            trans_mb.push_box(
                plume.pos[0],
                plume.pos[1],
                plume.pos[2],
                radius,
                radius * 0.85,
                radius,
                steam_col,
                2.0,
            );
        }

        if guard.searchlights_enabled {
            for beam in &guard.searchlights {
                let dir = beam.direction_at(guard.time);
                // Searchlight housing & glowing lens
                opaque_mb.push_box(
                    beam.origin[0],
                    beam.origin[1] - 0.12,
                    beam.origin[2],
                    0.22,
                    0.16,
                    0.22,
                    theme.steel_viaduct,
                    0.0,
                );
                opaque_mb.push_box(
                    beam.origin[0],
                    beam.origin[1],
                    beam.origin[2],
                    0.16,
                    0.10,
                    0.16,
                    theme.window_warm,
                    1.0,
                );
                trans_mb.push_light_cone(beam.origin, dir, beam.length, 1.25, theme.searchlight);
            }
        }

        // --- LAYER 5: High-Altitude Gothic Patrol Airships (Dirigibles) ---
        for ship in &guard.airships {
            let (pos, fwd) = ship.pos_and_forward(guard.time);
            let right = [-fwd[1], fwd[0]];
            let hl = ship.hull_length;
            let hw = hl * 0.28;
            let hh = hl * 0.26;

            // Main dirigible envelope (3 stepped segments for tapered streamlined look)
            opaque_mb.push_oriented_box(pos, fwd, hl * 0.58, hh, hw, theme.stone_secondary, 0.0);
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] + hh * 0.12, pos[2]],
                fwd,
                hl * 0.86,
                hh * 0.74,
                hw * 0.74,
                theme.stone_primary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] + hh * 0.22, pos[2]],
                fwd,
                hl * 1.04,
                hh * 0.44,
                hw * 0.44,
                theme.accent_trim,
                0.0,
            );

            // Cruciform tail fins at the stern
            let stern = [
                pos[0] - fwd[0] * (hl * 0.40),
                pos[1] + hh * 0.15,
                pos[2] - fwd[1] * (hl * 0.40),
            ];
            opaque_mb.push_oriented_box(
                stern,
                fwd,
                hl * 0.22,
                hh * 1.15,
                0.06,
                theme.copper_roof,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [stern[0], stern[1] + hh * 0.45, stern[2]],
                fwd,
                hl * 0.22,
                0.06,
                hw * 1.25,
                theme.copper_roof,
                0.0,
            );

            // Underslung illuminated gondola cabin + blinking red beacon
            opaque_mb.push_oriented_box(
                [pos[0], pos[1] - 0.14, pos[2]],
                fwd,
                hl * 0.34,
                0.14,
                hw * 0.42,
                theme.window_warm,
                1.0,
            );
            opaque_mb.push_oriented_box(
                [stern[0], stern[1] + hh * 1.15, stern[2]],
                fwd,
                0.09,
                0.09,
                0.09,
                theme.beacon_red,
                1.0,
            );

            // Downward sweeping airship searchlight scanning the gothic city below
            if guard.searchlights_enabled {
                let sweep = ship.searchlight_phase.sin() * 0.36;
                let spot_dir = [
                    fwd[0] * 0.32 + right[0] * sweep,
                    -0.88,
                    fwd[1] * 0.32 + right[1] * sweep,
                ];
                trans_mb.push_light_cone(
                    [
                        pos[0] + fwd[0] * (hl * 0.16),
                        pos[1] - 0.14,
                        pos[2] + fwd[1] * (hl * 0.16),
                    ],
                    spot_dir,
                    5.4,
                    1.10,
                    theme.searchlight,
                );
            }
        }

        let opaque_slice = &opaque_mb.verts[..opaque_mb.verts.len().min(MAX_DYNAMIC_VERTS)];
        self.opaque_dyn_count = opaque_slice.len() as u32;
        if !opaque_slice.is_empty() {
            gpu.queue
                .write_buffer(&self.opaque_dyn_vbuf, 0, bytemuck::cast_slice(opaque_slice));
        }

        let trans_slice = &trans_mb.verts[..trans_mb.verts.len().min(MAX_DYNAMIC_VERTS)];
        self.translucent_dyn_count = trans_slice.len() as u32;
        if !trans_slice.is_empty() {
            gpu.queue.write_buffer(
                &self.translucent_dyn_vbuf,
                0,
                bytemuck::cast_slice(trans_slice),
            );
        }
    }
}

impl Scene for GothicMetropolisScene {
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.scale_factor = scale_factor;
        self.depth_view = create_depth_texture(&gpu.device, self.width, self.height);
        self.rebuild_dynamic_buffers(gpu);
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        // Handle Direct Interaction Mode camera orbit & clicks
        if input.action == PointerAction::LeftClick {
            self.is_dragging = true;
            self.last_pointer = input.pointer;
            if let Ok(mut guard) = self.state.lock() {
                guard.trigger_steam_burst();
            }
        } else if input.action == PointerAction::RightClick
            && let Ok(mut guard) = self.state.lock()
        {
            guard.rotate_camera_quarter_turn();
        }

        if input.left_held {
            if let (Some(curr), Some(prev)) = (input.pointer, self.last_pointer) {
                let dx = (curr.x - prev.x) / (self.width.max(1) as f32);
                let dy = (curr.y - prev.y) / (self.height.max(1) as f32);
                if (dx.abs() > 0.0001 || dy.abs() > 0.0001)
                    && let Ok(mut guard) = self.state.lock()
                {
                    guard.orbit_camera(dx * PI * 1.4, dy * 1.1);
                }
            }
            self.last_pointer = input.pointer;
        } else {
            self.is_dragging = false;
            self.last_pointer = input.pointer;
        }

        let activity = if let Ok(mut guard) = self.state.lock() {
            guard.tick(dt.as_secs_f32())
        } else {
            Activity::Animating
        };

        self.rebuild_dynamic_buffers(gpu);
        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gothic-metropolis-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Pure stark black void background (#000000)
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // 1. Opaque 3D Architecture + Trains + Vehicles + Pedestrians + Airship Hulls
        rpass.set_pipeline(&self.opaque_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        if self.static_vert_count > 0 {
            rpass.set_vertex_buffer(0, self.static_vbuf.slice(..));
            rpass.draw(0..self.static_vert_count, 0..1);
        }
        if self.opaque_dyn_count > 0 {
            rpass.set_vertex_buffer(0, self.opaque_dyn_vbuf.slice(..));
            rpass.draw(0..self.opaque_dyn_count, 0..1);
        }

        // 2. Translucent Volumetric Searchlights, Headlight Cones & Rising Steam Plumes
        if self.translucent_dyn_count > 0 {
            rpass.set_pipeline(&self.translucent_pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.translucent_dyn_vbuf.slice(..));
            rpass.draw(0..self.translucent_dyn_count, 0..1);
        }
    }
}
