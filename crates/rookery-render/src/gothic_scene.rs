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
    BuildingArchetype, BuildingPlacement, CityBlockPlot, CityBlueprint, ElevatedHighwaySpan,
    GaragePortal, GothicMetropolisState, GothicPalette, GothicVehicle, GothicVehicleKind,
    StreetAvenue, UndergroundPortal, WalkwaySpan, WalkwayStyle,
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
    sidewalk_curb: [f32; 4],
    sidewalk_paver: [f32; 4],
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
            sidewalk: [0.25, 0.23, 0.26, 1.0],
            sidewalk_curb: [0.35, 0.31, 0.33, 1.0],
            sidewalk_paver: [0.30, 0.27, 0.30, 1.0],
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
            sidewalk: [0.21, 0.27, 0.36, 1.0],
            sidewalk_curb: [0.31, 0.39, 0.50, 1.0],
            sidewalk_paver: [0.26, 0.33, 0.43, 1.0],
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
            sidewalk: [0.27, 0.20, 0.21, 1.0],
            sidewalk_curb: [0.38, 0.28, 0.28, 1.0],
            sidewalk_paver: [0.32, 0.24, 0.25, 1.0],
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
            sidewalk: [0.21, 0.29, 0.25, 1.0],
            sidewalk_curb: [0.30, 0.40, 0.34, 1.0],
            sidewalk_paver: [0.25, 0.34, 0.29, 1.0],
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

    /// Sloped 3D box whose bottom centerline runs from `start_bottom` to `end_bottom`,
    /// with vertical thickness `height` and horizontal `width`. Includes underside face.
    #[allow(clippy::too_many_arguments)]
    fn push_sloped_box(
        &mut self,
        start_bottom: [f32; 3],
        end_bottom: [f32; 3],
        height: f32,
        width: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let dx = end_bottom[0] - start_bottom[0];
        let dy = end_bottom[1] - start_bottom[1];
        let dz = end_bottom[2] - start_bottom[2];
        let horiz_len = (dx * dx + dz * dz).sqrt().max(0.001);
        let len_3d = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
        let fx = dx / horiz_len;
        let fz = dz / horiz_len;
        let rx = -fz;
        let rz = fx;
        let hw = width * 0.5;

        let f3 = [dx / len_3d, dy / len_3d, dz / len_3d];
        let up_n = [
            -f3[1] * fx,
            (dx * dx + dz * dz).sqrt() / len_3d,
            -f3[1] * fz,
        ];

        let s_l_b = [
            start_bottom[0] - rx * hw,
            start_bottom[1],
            start_bottom[2] - rz * hw,
        ];
        let s_r_b = [
            start_bottom[0] + rx * hw,
            start_bottom[1],
            start_bottom[2] + rz * hw,
        ];
        let s_l_t = [s_l_b[0], s_l_b[1] + height, s_l_b[2]];
        let s_r_t = [s_r_b[0], s_r_b[1] + height, s_r_b[2]];

        let e_l_b = [
            end_bottom[0] - rx * hw,
            end_bottom[1],
            end_bottom[2] - rz * hw,
        ];
        let e_r_b = [
            end_bottom[0] + rx * hw,
            end_bottom[1],
            end_bottom[2] + rz * hw,
        ];
        let e_l_t = [e_l_b[0], e_l_b[1] + height, e_l_b[2]];
        let e_r_t = [e_r_b[0], e_r_b[1] + height, e_r_b[2]];

        // Top
        self.push_quad(s_l_t, e_l_t, e_r_t, s_r_t, up_n, color, emissive);
        // Bottom
        self.push_quad(
            s_r_b,
            e_r_b,
            e_l_b,
            s_l_b,
            [-up_n[0], -up_n[1], -up_n[2]],
            color,
            emissive,
        );
        // End (+fwd)
        self.push_quad(e_l_b, e_r_b, e_r_t, e_l_t, f3, color, emissive);
        // Start (-fwd)
        self.push_quad(
            s_r_b,
            s_l_b,
            s_l_t,
            s_r_t,
            [-f3[0], -f3[1], -f3[2]],
            color,
            emissive,
        );
        // Right (+right)
        self.push_quad(e_r_b, s_r_b, s_r_t, e_r_t, [rx, 0.0, rz], color, emissive);
        // Left (-right)
        self.push_quad(s_l_b, e_l_b, e_l_t, s_l_t, [-rx, 0.0, -rz], color, emissive);
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
    let mut mb = MeshBuilder::with_capacity(64_000);

    // 1. Stepped Citadel Foundation Plinth floating in the black void
    let ext = bp.plinth_half_extent;
    // Lower dark obsidian/slate deck slab (y = -0.26 .. 0.0)
    mb.push_box(
        0.0,
        -0.26,
        0.0,
        ext * 2.0,
        0.26,
        ext * 2.0,
        theme.plinth_dark,
        0.0,
    );
    // Outer fortified parapet wall around the city island perimeter
    let wall_t = 0.24;
    let wall_h = 0.32;
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
    // Corner bastion turrets + mid-wall gothic buttress bastions around the expanded floor plate
    for sx in [-1.0f32, 1.0f32] {
        for sz in [-1.0f32, 1.0f32] {
            mb.push_box(
                sx * (ext - 0.22),
                -0.15,
                sz * (ext - 0.22),
                0.56,
                0.62,
                0.56,
                theme.stone_secondary,
                0.0,
            );
            mb.push_box(
                sx * (ext - 0.22),
                0.47,
                sz * (ext - 0.22),
                0.62,
                0.10,
                0.62,
                theme.accent_trim,
                0.0,
            );
        }
        for mid in [-4.4f32, 4.4f32] {
            mb.push_box(
                mid,
                -0.12,
                sx * (ext - 0.18),
                0.44,
                0.50,
                0.44,
                theme.stone_secondary,
                0.0,
            );
            mb.push_box(
                sx * (ext - 0.18),
                -0.12,
                mid,
                0.44,
                0.50,
                0.44,
                theme.stone_secondary,
                0.0,
            );
        }
    }

    // Tapered under-city substructure fading into the black abyss
    mb.push_frustum(
        0.0,
        -1.06,
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
        -1.86,
        0.0,
        ext * 1.45,
        ext * 1.45,
        ext * 1.84,
        ext * 1.84,
        0.80,
        theme.plinth_dark,
        0.0,
    );

    // 2. Raised Stone Sidewalks, Curbs, Flagstone Pavers & Outer Highway Promenade
    build_sidewalks_and_curbs_mesh(&mut mb, bp, theme);

    // 3. Connected Canyon Avenues & Outer Perimeter Ring Highway
    build_roads_and_highway_mesh(&mut mb, bp, theme);

    // 4. Modular Gothic Buildings
    for b in &bp.buildings {
        build_archetype_mesh(&mut mb, b, theme);
    }

    // 5. Subterranean Building Parking Garage Portals & Ramps
    for garage in &bp.garages {
        build_garage_portal_mesh(&mut mb, garage, theme);
    }

    // 5b. 2nd-Level Outer Elevated Highway Overpasses & Straddling Arch Piers
    for span in &bp.elevated_highways {
        build_elevated_highway_mesh(&mut mb, bp, span, theme);
    }

    // 5c. Sunken Roadside Underground Tunnel Portals
    for portal in &bp.underground_portals {
        build_underground_portal_mesh(&mut mb, portal, theme);
    }

    // 6. Multi-Level Elevated Walkways / Skybridges
    for span in &bp.walkways {
        build_walkway_mesh(&mut mb, span, theme);
    }

    // 7. Elevated Train Viaduct Loop & Support Arches
    build_viaduct_mesh(&mut mb, bp, theme);

    mb.verts
}

/// Build raised stone sidewalks, distinct curb borders, flagstone paving courses, and corner bollards
/// around all city blocks in the blueprint and along the outer perimeter ring highway promenade.
fn build_sidewalks_and_curbs_mesh(
    mb: &mut MeshBuilder,
    bp: &CityBlueprint,
    theme: &GothicThemeColors,
) {
    let sw_h = 0.075f32;
    let curb_h = 0.095f32;
    let curb_w = 0.085f32;
    let walk_band = 0.42f32;

    let fallback_blocks: Vec<CityBlockPlot>;
    let blocks: &[CityBlockPlot] = if !bp.blocks.is_empty() {
        &bp.blocks
    } else {
        let spans: [(f32, f32); 3] = [(-7.56, -2.95), (-1.85, 1.55), (2.65, 7.56)];
        let mut v = Vec::with_capacity(9);
        let mut id = 1;
        for &(x0, x1) in &spans {
            for &(z0, z1) in &spans {
                v.push(CityBlockPlot { id, x0, z0, x1, z1 });
                id += 1;
            }
        }
        fallback_blocks = v;
        &fallback_blocks
    };

    for block in blocks {
        let (x0, z0, x1, z1) = (block.x0, block.z0, block.x1, block.z1);
        let bw = x1 - x0;
        let bd = z1 - z0;
        let cx = (x0 + x1) * 0.5;
        let cz = (z0 + z1) * 0.5;

        // Main raised block sidewalk & plaza platform
        mb.push_box(cx, 0.0, cz, bw, sw_h, bd, theme.sidewalk, 0.0);

        // Inner slightly recessed dark stone building plinth pad so the surrounding
        // 0.42m sidewalk walkway reads as a crisp perimeter pedestrian belt!
        if bw > walk_band * 2.2 && bd > walk_band * 2.2 {
            mb.push_box(
                cx,
                sw_h,
                cz,
                bw - walk_band * 2.0,
                0.015,
                bd - walk_band * 2.0,
                theme.stone_dark,
                0.0,
            );
        }

        // Raised stone curbs along the 4 street-facing edges of the city block
        mb.push_box(
            cx,
            0.0,
            z0 + curb_w * 0.5,
            bw,
            curb_h,
            curb_w,
            theme.sidewalk_curb,
            0.0,
        );
        mb.push_box(
            cx,
            0.0,
            z1 - curb_w * 0.5,
            bw,
            curb_h,
            curb_w,
            theme.sidewalk_curb,
            0.0,
        );
        mb.push_box(
            x0 + curb_w * 0.5,
            0.0,
            cz,
            curb_w,
            curb_h,
            bd,
            theme.sidewalk_curb,
            0.0,
        );
        mb.push_box(
            x1 - curb_w * 0.5,
            0.0,
            cz,
            curb_w,
            curb_h,
            bd,
            theme.sidewalk_curb,
            0.0,
        );

        // Textured flagstone paving slabs along North & South block sidewalks
        let x_pavers = ((bw - 0.4) / 0.54).floor().max(2.0) as usize;
        let x_step = (bw - 0.36) / x_pavers as f32;
        for i in 0..x_pavers {
            let px = x0 + 0.18 + (i as f32 + 0.5) * x_step;
            let paver_w = (x_step * 0.76).min(0.42);
            for pz in [z0 + walk_band * 0.52, z1 - walk_band * 0.52] {
                mb.push_box(
                    px,
                    sw_h,
                    pz,
                    paver_w,
                    0.012,
                    walk_band * 0.68,
                    theme.sidewalk_paver,
                    0.0,
                );
            }
        }

        // Textured flagstone paving slabs along West & East block sidewalks
        let z_pavers = ((bd - 0.4) / 0.54).floor().max(2.0) as usize;
        let z_step = (bd - 0.36) / z_pavers as f32;
        for i in 0..z_pavers {
            let pz = z0 + 0.18 + (i as f32 + 0.5) * z_step;
            let paver_d = (z_step * 0.76).min(0.42);
            for px in [x0 + walk_band * 0.52, x1 - walk_band * 0.52] {
                mb.push_box(
                    px,
                    sw_h,
                    pz,
                    walk_band * 0.68,
                    0.012,
                    paver_d,
                    theme.sidewalk_paver,
                    0.0,
                );
            }
        }

        // Gothic corner curb bollards at all 4 corners of the block
        for bx in [x0 + 0.14, x1 - 0.14] {
            for bz in [z0 + 0.14, z1 - 0.14] {
                mb.push_box(bx, sw_h, bz, 0.10, 0.16, 0.10, theme.sidewalk_curb, 0.0);
                mb.push_box(
                    bx,
                    sw_h + 0.16,
                    bz,
                    0.12,
                    0.04,
                    0.12,
                    theme.accent_trim,
                    0.0,
                );
            }
        }
    }

    // Outer Highway Perimeter Sidewalk Promenade (derived from outer highway position & plinth extent)
    let ext = bp.plinth_half_extent;
    let hwy_outer = bp
        .avenues
        .iter()
        .filter(|a| a.is_highway)
        .map(|a| a.start[0].abs().max(a.start[1].abs()) + a.width * 0.5)
        .fold(0.0f32, f32::max)
        .max(ext - 0.96);
    let prom_outer = (ext - 0.24).max(hwy_outer + 0.35);
    let prom_w = prom_outer - hwy_outer;
    let prom_mid = (hwy_outer + prom_outer) * 0.5;
    let span_full = prom_outer * 2.0;

    // 4 outer promenade sidewalk strips + inner highway curbs
    for sign in [-1.0f32, 1.0f32] {
        // North & South outer highway sidewalk promenades
        mb.push_box(
            0.0,
            0.0,
            sign * prom_mid,
            span_full,
            sw_h,
            prom_w,
            theme.sidewalk,
            0.0,
        );
        mb.push_box(
            0.0,
            0.0,
            sign * (hwy_outer + curb_w * 0.5),
            span_full - prom_w * 2.0,
            curb_h,
            curb_w,
            theme.sidewalk_curb,
            0.0,
        );
        // West & East outer highway sidewalk promenades
        mb.push_box(
            sign * prom_mid,
            0.0,
            0.0,
            prom_w,
            sw_h,
            span_full,
            theme.sidewalk,
            0.0,
        );
        mb.push_box(
            sign * (hwy_outer + curb_w * 0.5),
            0.0,
            0.0,
            curb_w,
            curb_h,
            span_full - prom_w * 2.0,
            theme.sidewalk_curb,
            0.0,
        );

        // Flagstone pavers & highway light standards along outer promenade
        let steps = 28usize;
        for i in 0..steps {
            let coord = -hwy_outer + (i as f32 + 0.5) * ((hwy_outer * 2.0) / steps as f32);
            mb.push_box(
                coord,
                sw_h,
                sign * prom_mid,
                0.52,
                0.012,
                prom_w * 0.65,
                theme.sidewalk_paver,
                0.0,
            );
            mb.push_box(
                sign * prom_mid,
                sw_h,
                coord,
                prom_w * 0.65,
                0.012,
                0.52,
                theme.sidewalk_paver,
                0.0,
            );

            // Outer highway art-deco light standards every 3rd paver
            if i % 3 == 1 {
                let lamp_pos = sign * (hwy_outer + 0.16);
                let blocks_ew_elev = bp.elevated_highways.iter().any(|eh| {
                    let min_x = eh.start[0].min(eh.end[0]) - 0.6;
                    let max_x = eh.start[0].max(eh.end[0]) + 0.6;
                    (eh.start[1] - lamp_pos).abs() < 1.1
                        && (eh.end[1] - lamp_pos).abs() < 1.1
                        && coord >= min_x
                        && coord <= max_x
                });
                if !blocks_ew_elev {
                    mb.push_box(
                        coord,
                        sw_h,
                        lamp_pos,
                        0.07,
                        0.42,
                        0.07,
                        theme.stone_dark,
                        0.0,
                    );
                    mb.push_box(
                        coord,
                        sw_h + 0.42,
                        lamp_pos,
                        0.13,
                        0.10,
                        0.13,
                        theme.window_warm,
                        1.0,
                    );
                }

                let blocks_ns_elev = bp.elevated_highways.iter().any(|eh| {
                    let min_z = eh.start[1].min(eh.end[1]) - 0.6;
                    let max_z = eh.start[1].max(eh.end[1]) + 0.6;
                    (eh.start[0] - lamp_pos).abs() < 1.1
                        && (eh.end[0] - lamp_pos).abs() < 1.1
                        && coord >= min_z
                        && coord <= max_z
                });
                if !blocks_ns_elev {
                    mb.push_box(
                        lamp_pos,
                        sw_h,
                        coord,
                        0.07,
                        0.42,
                        0.07,
                        theme.stone_dark,
                        0.0,
                    );
                    mb.push_box(
                        lamp_pos,
                        sw_h + 0.42,
                        coord,
                        0.13,
                        0.10,
                        0.13,
                        theme.window_warm,
                        1.0,
                    );
                }
            }
        }
    }
}

/// Shortest 2D distance from point `(px, pz)` to avenue segment `ave`.
fn point_to_avenue_dist(px: f32, pz: f32, ave: &StreetAvenue) -> f32 {
    let vx = ave.end[0] - ave.start[0];
    let vz = ave.end[1] - ave.start[1];
    let len_sq = vx * vx + vz * vz;
    if len_sq < 1e-4 {
        return ((px - ave.start[0]).powi(2) + (pz - ave.start[1]).powi(2)).sqrt();
    }
    let t = (((px - ave.start[0]) * vx + (pz - ave.start[1]) * vz) / len_sq).clamp(0.0, 1.0);
    let qx = ave.start[0] + vx * t;
    let qz = ave.start[1] + vz * t;
    ((px - qx).powi(2) + (pz - qz).powi(2)).sqrt()
}

/// Build the connected canyon avenues, the 4-sided outer perimeter ring highway,
/// highway median dividers, intersection crosswalks, and canyon streetlamps.
fn build_roads_and_highway_mesh(
    mb: &mut MeshBuilder,
    bp: &CityBlueprint,
    theme: &GothicThemeColors,
) {
    for (ave_idx, ave) in bp.avenues.iter().enumerate() {
        let dx = ave.end[0] - ave.start[0];
        let dz = ave.end[1] - ave.start[1];
        let len = (dx * dx + dz * dz).sqrt().max(0.1);
        let cx = (ave.start[0] + ave.end[0]) * 0.5;
        let cz = (ave.start[1] + ave.end[1]) * 0.5;
        let fwd = [dx / len, dz / len];

        // Extend segments by `ave.width` so all T-junctions, dogleg corners, and outer highway
        // corners overlap cleanly with zero asphalt gaps!
        let ext_len = len + ave.width;

        // Dark wet-look asphalt roadbed
        mb.push_oriented_box(
            [cx, 0.005, cz],
            fwd,
            ext_len,
            0.022,
            ave.width,
            theme.asphalt,
            0.0,
        );

        // Centerline dashed lane markings & highway median reflector studs
        let dash_steps = (len / 0.85).floor().max(3.0) as usize;
        for i in 0..dash_steps {
            let t = (i as f32 + 0.5) / dash_steps as f32;
            let mx = ave.start[0] + dx * t;
            let mz = ave.start[1] + dz * t;

            // Skip road intersections & T-junctions against all other avenue segments
            let near_junction = bp.avenues.iter().enumerate().any(|(other_idx, other)| {
                other_idx != ave_idx
                    && point_to_avenue_dist(mx, mz, other) < other.width * 0.5 + 0.28
            });
            if near_junction {
                continue;
            }
            // Also skip garage turnoff points
            let near_garage = bp.garages.iter().any(|g| {
                (mx - g.street_entry[0]).abs() < 0.55 && (mz - g.street_entry[1]).abs() < 0.55
            });
            if near_garage {
                continue;
            }

            if ave.is_highway {
                // Highway double-brass median strip + glowing amber cat's-eye reflector
                mb.push_oriented_box(
                    [mx, 0.026, mz],
                    fwd,
                    0.46,
                    0.012,
                    0.08,
                    theme.accent_trim,
                    0.0,
                );
                if i % 2 == 0 {
                    mb.push_oriented_box(
                        [mx, 0.036, mz],
                        fwd,
                        0.10,
                        0.016,
                        0.06,
                        theme.window_dim,
                        0.75,
                    );
                }
            } else {
                // Canyon avenue center dashed stripe
                mb.push_oriented_box(
                    [mx, 0.026, mz],
                    fwd,
                    0.34,
                    0.008,
                    0.05,
                    theme.sidewalk_paver,
                    0.0,
                );
            }
        }

        // Place miniature gothic streetlamps along both sides of inner canyon avenues
        if !ave.is_highway {
            let steps = (len / 1.9).round().clamp(3.0, 9.0) as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let lx = ave.start[0] + dx * t;
                let lz = ave.start[1] + dz * t;
                // Skip street intersections & T-junctions
                let near_junction = bp.avenues.iter().enumerate().any(|(other_idx, other)| {
                    other_idx != ave_idx
                        && point_to_avenue_dist(lx, lz, other) < other.width * 0.5 + 0.35
                });
                if near_junction {
                    continue;
                }
                for side in [-1.0f32, 1.0f32] {
                    let px = lx + (-fwd[1]) * (ave.width * 0.5 + 0.16) * side;
                    let pz = lz + (fwd[0]) * (ave.width * 0.5 + 0.16) * side;
                    // Ensure the streetlamp sits on a raised city block sidewalk
                    let on_block_sidewalk = bp.blocks.is_empty()
                        || bp.blocks.iter().any(|b| {
                            px >= b.x0 + 0.06
                                && px <= b.x1 - 0.06
                                && pz >= b.z0 + 0.06
                                && pz <= b.z1 - 0.06
                        });
                    if !on_block_sidewalk {
                        continue;
                    }
                    // Skip if a garage driveway apron occupies this sidewalk spot
                    let blocks_garage = bp.garages.iter().any(|g| {
                        (px - g.portal_pos[0]).abs() < 0.75 && (pz - g.portal_pos[1]).abs() < 0.75
                    });
                    if blocks_garage {
                        continue;
                    }
                    mb.push_box(px, 0.075, pz, 0.06, 0.34, 0.06, theme.stone_dark, 0.0);
                    mb.push_box(px, 0.415, pz, 0.11, 0.10, 0.11, theme.window_warm, 1.0);
                }
            }
        }
    }

    // Stone crosswalk stripes at key staggered interior plaza junctions
    let crosswalk_junctions: [([f32; 2], bool); 8] = [
        ([-5.2, 0.2], true),
        ([-0.4, 0.2], false),
        ([0.7, 0.2], false),
        ([5.4, 0.2], true),
        ([-5.2, -5.4], false),
        ([-0.4, -4.3], false),
        ([0.7, 4.6], false),
        ([-5.2, 5.4], false),
    ];
    for &(jpos, both_axes) in &crosswalk_junctions {
        for offset in [-0.64f32, 0.64f32] {
            for stripe in [-0.30f32, -0.10, 0.10, 0.30] {
                mb.push_box(
                    jpos[0] + stripe,
                    0.025,
                    jpos[1] + offset,
                    0.09,
                    0.008,
                    0.22,
                    theme.sidewalk_paver,
                    0.0,
                );
                if both_axes {
                    mb.push_box(
                        jpos[0] + offset,
                        0.025,
                        jpos[1] + stripe,
                        0.22,
                        0.008,
                        0.09,
                        theme.sidewalk_paver,
                        0.0,
                    );
                }
            }
        }
    }

    // Wrought-iron gothic traffic signal posts at the 6 primary inner-city junctions
    let signal_posts: [[f32; 2]; 6] = [
        [-5.2, 0.2],
        [5.4, 0.2],
        [-0.4, -5.4],
        [0.7, 4.6],
        [-5.2, -5.4],
        [-5.2, 5.4],
    ];
    for &[sx, sz] in &signal_posts {
        let px = sx + 0.54;
        let pz = sz + 0.54;
        // Post base + vertical iron mast + overhanging signal arm
        mb.push_box(px, 0.06, pz, 0.09, 0.08, 0.09, theme.stone_dark, 0.0);
        mb.push_box(px, 0.14, pz, 0.045, 0.52, 0.045, theme.plinth_dark, 0.0);
        mb.push_box(
            px - 0.14,
            0.63,
            pz,
            0.32,
            0.04,
            0.045,
            theme.plinth_dark,
            0.0,
        );
        mb.push_box(
            px - 0.28,
            0.57,
            pz,
            0.09,
            0.14,
            0.09,
            theme.stone_secondary,
            0.0,
        );
    }
}

/// Build an underground parking garage entrance recessed into a building base, complete with
/// a downward-sloping asphalt driveway cutting through the sidewalk, stone ramp wing-walls,
/// a heavy gothic portal arch, a pitch-black subterranean tunnel void, and glowing clearance lights.
fn build_garage_portal_mesh(
    mb: &mut MeshBuilder,
    garage: &GaragePortal,
    theme: &GothicThemeColors,
) {
    let fwd = garage.inward_dir;
    let right = [-fwd[1], fwd[0]];
    let w = garage.width;

    let dx = garage.portal_pos[0] - garage.street_entry[0];
    let dz = garage.portal_pos[1] - garage.street_entry[1];
    let dist = (dx * dx + dz * dz).sqrt().max(0.2);

    // 1. Asphalt driveway apron cutting across the raised sidewalk into the building portal
    let apron_len = (dist - 0.32).max(0.35);
    let apron_cx = garage.portal_pos[0] - fwd[0] * (apron_len * 0.5 - 0.12);
    let apron_cz = garage.portal_pos[1] - fwd[1] * (apron_len * 0.5 - 0.12);
    mb.push_oriented_box(
        [apron_cx, 0.002, apron_cz],
        fwd,
        apron_len + 0.35,
        0.078,
        w * 0.92,
        theme.asphalt,
        0.0,
    );
    // Amber center guide stripe leading into the subterranean ramp
    mb.push_oriented_box(
        [apron_cx, 0.076, apron_cz],
        fwd,
        apron_len * 0.85,
        0.008,
        0.06,
        theme.accent_trim,
        0.0,
    );

    // 2. Flanking sloped stone curb wing-walls bordering the garage ramp across the sidewalk
    let cheek_w = 0.09f32;
    for side in [-1.0f32, 1.0f32] {
        let wx = apron_cx + right[0] * (w * 0.48 + cheek_w * 0.5) * side;
        let wz = apron_cz + right[1] * (w * 0.48 + cheek_w * 0.5) * side;
        mb.push_oriented_box(
            [wx, 0.075, wz],
            fwd,
            apron_len * 0.88,
            0.09,
            cheek_w,
            theme.sidewalk_curb,
            0.0,
        );
    }

    // 3. Protruding Gothic Stone Garage Portal Archway & Hood on the building facade
    let portal_h = 0.72f32;
    let portal_depth = 0.24f32;
    let arch_cx = garage.portal_pos[0] - fwd[0] * 0.04;
    let arch_cz = garage.portal_pos[1] - fwd[1] * 0.04;

    // Outer stone portal surround
    mb.push_oriented_box(
        [arch_cx, 0.0, arch_cz],
        fwd,
        portal_depth,
        portal_h,
        w + 0.28,
        theme.stone_secondary,
        0.0,
    );
    // Overhanging gothic cornice lintel above garage entrance
    mb.push_oriented_box(
        [
            arch_cx - fwd[0] * 0.03,
            portal_h - 0.08,
            arch_cz - fwd[1] * 0.03,
        ],
        fwd,
        portal_depth + 0.10,
        0.11,
        w + 0.36,
        theme.accent_trim,
        0.0,
    );

    // 4. Pitch-black subterranean tunnel mouth & underground vault recess
    let void_color = [0.01, 0.01, 0.018, 1.0];
    mb.push_oriented_box(
        [
            garage.portal_pos[0] + fwd[0] * 0.14,
            0.0,
            garage.portal_pos[1] + fwd[1] * 0.14,
        ],
        fwd,
        0.64,
        portal_h - 0.16,
        w * 0.86,
        void_color,
        0.0,
    );

    // 5. Illuminated overhead clearance light bar + twin ruby entrance marker beacons
    let sign_x = garage.portal_pos[0] - fwd[0] * 0.15;
    let sign_z = garage.portal_pos[1] - fwd[1] * 0.15;
    mb.push_oriented_box(
        [sign_x, portal_h - 0.18, sign_z],
        fwd,
        0.05,
        0.07,
        w * 0.72,
        theme.window_warm,
        0.95,
    );
    for side in [-1.0f32, 1.0f32] {
        let bx = sign_x + right[0] * (w * 0.52) * side;
        let bz = sign_z + right[1] * (w * 0.52) * side;
        mb.push_oriented_box([bx, 0.34, bz], fwd, 0.06, 0.08, 0.06, theme.beacon_red, 1.0);
    }
}

/// Shortest 2D distance from point `(px, pz)` to elevated highway span centerline `span`.
fn point_to_elev_span_dist(px: f32, pz: f32, span: &ElevatedHighwaySpan) -> f32 {
    let vx = span.end[0] - span.start[0];
    let vz = span.end[1] - span.start[1];
    let len_sq = vx * vx + vz * vz;
    if len_sq < 1e-4 {
        return ((px - span.start[0]).powi(2) + (pz - span.start[1]).powi(2)).sqrt();
    }
    let t = (((px - span.start[0]) * vx + (pz - span.start[1]) * vz) / len_sq).clamp(0.0, 1.0);
    let qx = span.start[0] + vx * t;
    let qz = span.start[1] + vz * t;
    ((px - qx).powi(2) + (pz - qz).powi(2)).sqrt()
}

/// Emit an overhead gothic steel-and-brass highway gantry arch with glowing signage and beacons.
fn push_highway_gantry(
    mb: &mut MeshBuilder,
    center: [f32; 3],
    fwd: [f32; 2],
    right: [f32; 2],
    width: f32,
    clearance_h: f32,
    theme: &GothicThemeColors,
) {
    let base_y = center[1];
    let post_offset = width * 0.54;
    for side in [-1.0f32, 1.0f32] {
        let px = center[0] + right[0] * post_offset * side;
        let pz = center[2] + right[1] * post_offset * side;
        // Stepped stone-and-iron pylon
        mb.push_box(px, base_y, pz, 0.16, 0.14, 0.16, theme.stone_secondary, 0.0);
        mb.push_box(
            px,
            base_y + 0.14,
            pz,
            0.10,
            clearance_h - 0.14,
            0.10,
            theme.stone_dark,
            0.0,
        );
        mb.push_box(
            px,
            base_y + clearance_h,
            pz,
            0.14,
            0.06,
            0.14,
            theme.accent_trim,
            0.0,
        );
        mb.push_pyramid(
            px,
            base_y + clearance_h + 0.06,
            pz,
            0.13,
            0.18,
            0.13,
            theme.stone_secondary,
            0.0,
        );
        // Glowing clearance beacon
        mb.push_box(
            px,
            base_y + clearance_h + 0.24,
            pz,
            0.06,
            0.05,
            0.06,
            theme.beacon_red,
            1.0,
        );
    }

    // Overhead steel cross-beam & brass cornice
    let beam_y = base_y + clearance_h - 0.06;
    mb.push_oriented_box(
        [center[0], beam_y, center[2]],
        fwd,
        0.10,
        0.10,
        post_offset * 2.0 + 0.08,
        theme.steel_viaduct,
        0.0,
    );
    mb.push_oriented_box(
        [center[0], beam_y + 0.10, center[2]],
        fwd,
        0.12,
        0.03,
        post_offset * 2.0 + 0.10,
        theme.accent_trim,
        0.0,
    );

    // Illuminated highway directional signplate centered on the gantry
    mb.push_oriented_box(
        [center[0], beam_y - 0.04, center[2]],
        fwd,
        0.13,
        0.14,
        width * 0.56,
        theme.stone_dark,
        0.0,
    );
    mb.push_oriented_box(
        [center[0], beam_y - 0.02, center[2]],
        fwd,
        0.14,
        0.08,
        width * 0.46,
        theme.window_dim,
        0.85,
    );
}

/// Emit an ornate cantilevered gothic streetlamp on an elevated deck or ramp parapet.
fn push_elevated_cantilever_lamp(
    mb: &mut MeshBuilder,
    px: f32,
    base_y: f32,
    pz: f32,
    right: [f32; 2],
    side: f32,
    theme: &GothicThemeColors,
) {
    mb.push_box(px, base_y, pz, 0.07, 0.06, 0.07, theme.stone_secondary, 0.0);
    mb.push_box(
        px,
        base_y + 0.06,
        pz,
        0.045,
        0.28,
        0.045,
        theme.stone_dark,
        0.0,
    );
    // Brass cantilever arm reaching slightly inward over the roadway
    let inward_x = px - right[0] * 0.08 * side;
    let inward_z = pz - right[1] * 0.08 * side;
    mb.push_oriented_box(
        [(px + inward_x) * 0.5, base_y + 0.34, (pz + inward_z) * 0.5],
        [-right[1] * side, right[0] * side],
        0.04,
        0.03,
        0.14,
        theme.accent_trim,
        0.0,
    );
    mb.push_box(
        inward_x,
        base_y + 0.28,
        inward_z,
        0.08,
        0.07,
        0.08,
        theme.window_warm,
        1.0,
    );
    mb.push_pyramid(
        inward_x,
        base_y + 0.35,
        inward_z,
        0.09,
        0.06,
        0.09,
        theme.stone_dark,
        0.0,
    );
}

/// Build a 2nd-level elevated highway span with optional sloped approach/exit ramps,
/// road-grade entrance gantries & markings, ornate upper-level gothic lamps & pilasters,
/// seamless elevated corner/T-junction openings, and straddling arch piers.
fn build_elevated_highway_mesh(
    mb: &mut MeshBuilder,
    bp: &CityBlueprint,
    span: &ElevatedHighwaySpan,
    theme: &GothicThemeColors,
) {
    let dx = span.end[0] - span.start[0];
    let dz = span.end[1] - span.start[1];
    let total_len = (dx * dx + dz * dz).sqrt().max(0.5);
    let fwd = [dx / total_len, dz / total_len];
    let right = [-fwd[1], fwd[0]];
    let start_ramp_len = if span.start_has_ramp {
        span.ramp_length.min(total_len * 0.42)
    } else {
        0.0
    };
    let end_ramp_len = if span.end_has_ramp {
        span.ramp_length.min(total_len * 0.42)
    } else {
        0.0
    };
    let deck_len = (total_len - start_ramp_len - end_ramp_len).max(0.2);
    let elev = span.elevation;
    let w = span.width;

    let start_top = [
        span.start[0] + fwd[0] * start_ramp_len,
        span.start[1] + fwd[1] * start_ramp_len,
    ];
    let end_top = [
        span.end[0] - fwd[0] * end_ramp_len,
        span.end[1] - fwd[1] * end_ramp_len,
    ];

    struct RampSegment {
        p0: [f32; 2],
        y0: f32,
        p1: [f32; 2],
        y1: f32,
        ground_entry: [f32; 2],
    }

    // 1. Sloped approach & exit ramps (with normal-road lane markings, brass rails & entrance gantries)
    let mut ramps = Vec::new();
    if span.start_has_ramp {
        ramps.push(RampSegment {
            p0: span.start,
            y0: 0.0,
            p1: start_top,
            y1: elev,
            ground_entry: span.start,
        });
    }
    if span.end_has_ramp {
        ramps.push(RampSegment {
            p0: end_top,
            y0: elev,
            p1: span.end,
            y1: 0.0,
            ground_entry: span.end,
        });
    }

    for RampSegment {
        p0,
        y0,
        p1,
        y1,
        ground_entry,
    } in ramps
    {
        let rdx = p1[0] - p0[0];
        let rdz = p1[1] - p0[1];
        let rlen = (rdx * rdx + rdz * rdz).sqrt().max(0.2);

        // Steel/stone structural ramp girder
        mb.push_sloped_box(
            [p0[0], y0 - 0.13, p0[1]],
            [p1[0], y1 - 0.13, p1[1]],
            0.13,
            w + 0.12,
            theme.steel_viaduct,
            0.0,
        );
        // Asphalt ramp deck
        mb.push_sloped_box(
            [p0[0], y0 - 0.005, p0[1]],
            [p1[0], y1 - 0.005, p1[1]],
            0.03,
            w * 0.92,
            theme.asphalt,
            0.0,
        );

        // Flanking ramp parapet guardrails + brass top trim
        for side in [-1.0f32, 1.0f32] {
            let ox = right[0] * (w * 0.50) * side;
            let oz = right[1] * (w * 0.50) * side;
            mb.push_sloped_box(
                [p0[0] + ox, y0, p0[1] + oz],
                [p1[0] + ox, y1, p1[1] + oz],
                0.15,
                0.08,
                theme.stone_secondary,
                0.0,
            );
            mb.push_sloped_box(
                [p0[0] + ox, y0 + 0.15, p0[1] + oz],
                [p1[0] + ox, y1 + 0.15, p1[1] + oz],
                0.035,
                0.10,
                theme.accent_trim,
                0.0,
            );
        }

        // Sloped edge lane stripes along both sides of the ramp asphalt
        for side in [-1.0f32, 1.0f32] {
            let ox = right[0] * (w * 0.38) * side;
            let oz = right[1] * (w * 0.38) * side;
            mb.push_sloped_box(
                [p0[0] + ox, y0 + 0.022, p0[1] + oz],
                [p1[0] + ox, y1 + 0.022, p1[1] + oz],
                0.008,
                0.04,
                theme.sidewalk_paver,
                0.0,
            );
        }

        // Ramp centerline double-brass dashes & glowing amber cat's-eye reflectors
        let ramp_dashes = (rlen / 0.72).floor().max(3.0) as usize;
        for i in 0..ramp_dashes {
            let t0 = (i as f32 + 0.20) / ramp_dashes as f32;
            let t1 = (i as f32 + 0.78) / ramp_dashes as f32;
            let d0 = [p0[0] + rdx * t0, p0[1] + rdz * t0];
            let d1 = [p0[0] + rdx * t1, p0[1] + rdz * t1];
            let dy0 = y0 + (y1 - y0) * t0;
            let dy1 = y0 + (y1 - y0) * t1;
            mb.push_sloped_box(
                [d0[0], dy0 + 0.024, d0[1]],
                [d1[0], dy1 + 0.024, d1[1]],
                0.012,
                0.07,
                theme.accent_trim,
                0.0,
            );
            let tm = 0.5 * (t0 + t1);
            let mx = p0[0] + rdx * tm;
            let mz = p0[1] + rdz * tm;
            let my = y0 + (y1 - y0) * tm;
            mb.push_oriented_box(
                [mx, my + 0.036, mz],
                fwd,
                0.09,
                0.014,
                0.05,
                theme.window_dim,
                0.80,
            );
        }

        // Ramp-side gothic streetlamps at midpoint of the ramp
        let mid_x = (p0[0] + p1[0]) * 0.5;
        let mid_z = (p0[1] + p1[1]) * 0.5;
        let mid_y = (y0 + y1) * 0.5;
        for side in [-1.0f32, 1.0f32] {
            let lx = mid_x + right[0] * (w * 0.50) * side;
            let lz = mid_z + right[1] * (w * 0.50) * side;
            push_elevated_cantilever_lamp(mb, lx, mid_y + 0.185, lz, right, side, theme);
        }

        // Ground-level entrance threshold & monumental gothic highway gantry arch
        mb.push_oriented_box(
            [ground_entry[0], 0.022, ground_entry[1]],
            fwd,
            0.16,
            0.016,
            w * 0.96,
            theme.accent_trim,
            0.0,
        );
        push_highway_gantry(
            mb,
            [ground_entry[0], 0.02, ground_entry[1]],
            fwd,
            right,
            w,
            0.58,
            theme,
        );
    }

    // 2. Flat elevated bridge deck (extended by w * 0.5 at non-ramp ends for seamless corners/T-junctions)
    let start_ext = if span.start_has_ramp { 0.0 } else { w * 0.5 };
    let end_ext = if span.end_has_ramp { 0.0 } else { w * 0.5 };
    let deck_ext_len = deck_len + start_ext + end_ext + 0.08;
    let deck_cx = (start_top[0] + end_top[0]) * 0.5 + fwd[0] * (end_ext - start_ext) * 0.5;
    let deck_cz = (start_top[1] + end_top[1]) * 0.5 + fwd[1] * (end_ext - start_ext) * 0.5;

    mb.push_oriented_box(
        [deck_cx, elev - 0.15, deck_cz],
        fwd,
        deck_ext_len,
        0.15,
        w + 0.14,
        theme.steel_viaduct,
        0.0,
    );
    mb.push_oriented_box(
        [deck_cx, elev - 0.005, deck_cz],
        fwd,
        deck_ext_len,
        0.03,
        w * 0.92,
        theme.asphalt,
        0.0,
    );

    // Flanking gothic parapet walls + brass top trim + pilasters (opened cleanly at elevated junctions)
    let parapet_steps = (deck_ext_len / 0.48).ceil().max(4.0) as usize;
    let seg_len = deck_ext_len / parapet_steps as f32 + 0.02;
    for side in [-1.0f32, 1.0f32] {
        for i in 0..parapet_steps {
            let t = (i as f32 + 0.5) / parapet_steps as f32 - 0.5;
            let px = deck_cx + fwd[0] * (t * deck_ext_len) + right[0] * (w * 0.50) * side;
            let pz = deck_cz + fwd[1] * (t * deck_ext_len) + right[1] * (w * 0.50) * side;

            let inside_other_deck = bp.elevated_highways.iter().any(|other| {
                other.id != span.id
                    && point_to_elev_span_dist(px, pz, other) < other.width * 0.5 - 0.02
            });
            if inside_other_deck {
                continue;
            }

            mb.push_oriented_box(
                [px, elev, pz],
                fwd,
                seg_len,
                0.16,
                0.09,
                theme.stone_secondary,
                0.0,
            );
            mb.push_oriented_box(
                [px, elev + 0.16, pz],
                fwd,
                seg_len,
                0.04,
                0.11,
                theme.accent_trim,
                0.0,
            );

            // Gothic stone pilaster & miniature spire finial every 4th segment
            if i % 4 == 2 {
                mb.push_box(px, elev, pz, 0.13, 0.22, 0.13, theme.stone_secondary, 0.0);
                mb.push_pyramid(
                    px,
                    elev + 0.22,
                    pz,
                    0.11,
                    0.12,
                    0.11,
                    theme.accent_trim,
                    0.0,
                );
            }
        }
    }

    // 3. Upper-level edge lane stripes, median reflector dashes, cantilevered lamps & sign gantries
    let dash_steps = (deck_len / 0.90).floor().max(4.0) as usize;
    for i in 0..dash_steps {
        let t = (i as f32 + 0.5) / dash_steps as f32;
        let mx = start_top[0] + (end_top[0] - start_top[0]) * t;
        let mz = start_top[1] + (end_top[1] - start_top[1]) * t;

        let near_elev_junction = bp.elevated_highways.iter().any(|other| {
            other.id != span.id && point_to_elev_span_dist(mx, mz, other) < other.width * 0.5 + 0.28
        });
        if near_elev_junction {
            continue;
        }

        // Edge lane markings
        for side in [-1.0f32, 1.0f32] {
            let ex = mx + right[0] * (w * 0.38) * side;
            let ez = mz + right[1] * (w * 0.38) * side;
            mb.push_oriented_box(
                [ex, elev + 0.022, ez],
                fwd,
                0.68,
                0.008,
                0.038,
                theme.sidewalk_paver,
                0.0,
            );
        }

        // Center double-brass median & glowing amber cat's-eye reflector
        mb.push_oriented_box(
            [mx, elev + 0.022, mz],
            fwd,
            0.44,
            0.012,
            0.07,
            theme.accent_trim,
            0.0,
        );
        if i % 2 == 0 {
            mb.push_oriented_box(
                [mx, elev + 0.032, mz],
                fwd,
                0.10,
                0.014,
                0.05,
                theme.window_dim,
                0.80,
            );
        }

        // Ornate cantilevered gothic streetlamps every 3rd dash
        if i % 3 == 1 {
            for side in [-1.0f32, 1.0f32] {
                let lx = mx + right[0] * (w * 0.50) * side;
                let lz = mz + right[1] * (w * 0.50) * side;
                let lamp_blocked = bp.elevated_highways.iter().any(|other| {
                    other.id != span.id
                        && point_to_elev_span_dist(lx, lz, other) < other.width * 0.5 + 0.15
                });
                if !lamp_blocked {
                    push_elevated_cantilever_lamp(mb, lx, elev + 0.20, lz, right, side, theme);
                }
            }
        }

        // Overhead gothic highway sign gantries along the upper deck
        if i == dash_steps / 3 || (dash_steps >= 10 && i == (dash_steps * 2) / 3) {
            push_highway_gantry(mb, [mx, elev + 0.02, mz], fwd, right, w, 0.58, theme);
        }
    }

    // 4. Straddling gothic arch piers (clearing ground-level highway & cross avenues)
    let pier_count = (total_len / 2.6).ceil().max(4.0) as usize;
    for i in 1..pier_count {
        let t = i as f32 / pier_count as f32;
        let dist = t * total_len;
        let px = span.start[0] + dx * t;
        let pz = span.start[1] + dz * t;

        // Compute deck underside height at `dist`
        let local_elev = if span.start_has_ramp && dist < start_ramp_len {
            elev * (dist / start_ramp_len)
        } else if span.end_has_ramp && dist > total_len - end_ramp_len {
            elev * ((total_len - dist) / end_ramp_len)
        } else {
            elev
        };
        if local_elev < 0.42 {
            continue;
        }

        // Skip piers that would land inside an intersecting cross street or another span's junction
        let hits_cross_street = bp.avenues.iter().any(|ave| {
            let adx = ave.end[0] - ave.start[0];
            let adz = ave.end[1] - ave.start[1];
            let alen = (adx * adx + adz * adz).sqrt().max(0.1);
            let dot = ((adx / alen) * fwd[0] + (adz / alen) * fwd[1]).abs();
            let is_perpendicular = dot < 0.5;
            (!ave.is_highway || is_perpendicular)
                && point_to_avenue_dist(px, pz, ave) < ave.width * 0.5 + 0.35
        });
        let near_other_junction = bp.elevated_highways.iter().any(|other| {
            other.id != span.id && point_to_elev_span_dist(px, pz, other) < other.width * 0.5 + 0.55
        });
        if hits_cross_street || near_other_junction {
            continue;
        }

        let col_offset = (w * 0.62).max(0.72);
        for side in [-1.0f32, 1.0f32] {
            let cx = px + right[0] * col_offset * side;
            let cz = pz + right[1] * col_offset * side;
            mb.push_box(
                cx,
                0.0,
                cz,
                0.17,
                local_elev - 0.12,
                0.17,
                theme.stone_dark,
                0.0,
            );
            mb.push_box(cx, 0.0, cz, 0.24, 0.16, 0.24, theme.stone_secondary, 0.0);
            mb.push_box(cx, 0.16, cz, 0.21, 0.04, 0.21, theme.accent_trim, 0.0);
        }
        // Overhead cross-girder connecting twin columns beneath the elevated deck
        mb.push_oriented_box(
            [px, local_elev - 0.26, pz],
            fwd,
            0.20,
            0.14,
            col_offset * 2.0 + 0.18,
            theme.stone_secondary,
            0.0,
        );
    }
}

/// Build a sunken roadside underground tunnel portal with a sloped descent ramp,
/// gothic stone retaining wing-walls, overhanging tunnel archway, pitch-black tunnel void,
/// and clearance beacons.
fn build_underground_portal_mesh(
    mb: &mut MeshBuilder,
    portal: &UndergroundPortal,
    theme: &GothicThemeColors,
) {
    let fwd = portal.inward_dir;
    let right = [-fwd[1], fwd[0]];
    let w = portal.width;

    // 1. Sloped asphalt ramp descending from street level into the sunken tunnel mouth
    mb.push_sloped_box(
        [portal.entry_pos[0], -0.02, portal.entry_pos[1]],
        [portal.portal_pos[0], -0.24, portal.portal_pos[1]],
        0.04,
        w * 0.90,
        theme.asphalt,
        0.0,
    );
    // Amber center guide stripe descending into the tunnel
    mb.push_sloped_box(
        [portal.entry_pos[0], 0.022, portal.entry_pos[1]],
        [portal.portal_pos[0], -0.195, portal.portal_pos[1]],
        0.01,
        0.06,
        theme.accent_trim,
        0.0,
    );

    // 2. Flanking stone retaining walls bordering the sunken ramp
    let dx = portal.portal_pos[0] - portal.entry_pos[0];
    let dz = portal.portal_pos[1] - portal.entry_pos[1];
    let ramp_len = (dx * dx + dz * dz).sqrt().max(0.4);
    let mid_x = (portal.entry_pos[0] + portal.portal_pos[0]) * 0.5;
    let mid_z = (portal.entry_pos[1] + portal.portal_pos[1]) * 0.5;
    for side in [-1.0f32, 1.0f32] {
        let wx = mid_x + right[0] * (w * 0.50 + 0.06) * side;
        let wz = mid_z + right[1] * (w * 0.50 + 0.06) * side;
        mb.push_oriented_box(
            [wx, 0.0, wz],
            fwd,
            ramp_len * 0.92,
            0.22,
            0.12,
            theme.stone_secondary,
            0.0,
        );
        mb.push_oriented_box(
            [wx, 0.22, wz],
            fwd,
            ramp_len * 0.92,
            0.04,
            0.14,
            theme.accent_trim,
            0.0,
        );
    }

    // 3. Heavy gothic stone tunnel portal housing & overhanging cornice lintel
    let portal_h = 0.78f32;
    let arch_cx = portal.portal_pos[0] + fwd[0] * 0.18;
    let arch_cz = portal.portal_pos[1] + fwd[1] * 0.18;
    mb.push_oriented_box(
        [arch_cx, 0.0, arch_cz],
        fwd,
        0.52,
        portal_h,
        w + 0.36,
        theme.stone_secondary,
        0.0,
    );
    mb.push_oriented_box(
        [arch_cx - fwd[0] * 0.04, portal_h, arch_cz - fwd[1] * 0.04],
        fwd,
        0.60,
        0.12,
        w + 0.44,
        theme.accent_trim,
        0.0,
    );

    // 4. Pitch-black subterranean tunnel mouth void
    let void_color = [0.01, 0.01, 0.018, 1.0];
    mb.push_oriented_box(
        [
            portal.portal_pos[0] - fwd[0] * 0.05,
            -0.18,
            portal.portal_pos[1] - fwd[1] * 0.05,
        ],
        fwd,
        0.38,
        portal_h - 0.04,
        w * 0.86,
        void_color,
        0.0,
    );

    // 5. Overhead illuminated clearance bar & twin ruby tunnel beacons
    let sign_x = portal.portal_pos[0] - fwd[0] * 0.12;
    let sign_z = portal.portal_pos[1] - fwd[1] * 0.12;
    mb.push_oriented_box(
        [sign_x, portal_h - 0.16, sign_z],
        fwd,
        0.05,
        0.07,
        w * 0.72,
        theme.window_warm,
        0.95,
    );
    for side in [-1.0f32, 1.0f32] {
        let bx = sign_x + right[0] * (w * 0.54) * side;
        let bz = sign_z + right[1] * (w * 0.54) * side;
        mb.push_oriented_box([bx, 0.36, bz], fwd, 0.06, 0.09, 0.06, theme.beacon_red, 1.0);
    }
}

/// Per-building masonry, trim, and roof color palette derived deterministically from
/// the active [`GothicThemeColors`] and the building's `id` / `style_variant`.
#[derive(Clone, Copy)]
struct BuildingMaterial {
    stone_main: [f32; 4],
    stone_alt: [f32; 4],
    stone_base: [f32; 4],
    trim: [f32; 4],
    roof: [f32; 4],
}

fn mix_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let u = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        a[3],
    ]
}

fn scale_color(c: [f32; 4], s: f32) -> [f32; 4] {
    [
        (c[0] * s).clamp(0.0, 1.0),
        (c[1] * s).clamp(0.0, 1.0),
        (c[2] * s).clamp(0.0, 1.0),
        c[3],
    ]
}

fn building_material(b: &BuildingPlacement, theme: &GothicThemeColors) -> BuildingMaterial {
    let finish = (b.id as u8).wrapping_add(b.style_variant.wrapping_mul(3)) % 4;
    match finish {
        // 0: Obsidian Basalt — deep charcoal stone with high-contrast brass/amber trim
        0 => BuildingMaterial {
            stone_main: scale_color(theme.stone_primary, 0.84),
            stone_alt: theme.stone_secondary,
            stone_base: scale_color(theme.stone_dark, 0.86),
            trim: scale_color(theme.accent_trim, 1.14),
            roof: mix_color(theme.copper_roof, theme.stone_dark, 0.28),
        },
        // 1: Weathered Gargoyle Limestone — lighter ash-stone with crisp dark mullions & verdigris roof
        1 => BuildingMaterial {
            stone_main: scale_color(theme.stone_secondary, 1.12),
            stone_alt: theme.stone_primary,
            stone_base: theme.stone_dark,
            trim: mix_color(theme.accent_trim, theme.sidewalk_curb, 0.35),
            roof: scale_color(theme.copper_roof, 1.18),
        },
        // 2: Bronzed Art-Deco Slate — warm bronze-infused masonry with gilded deco ledges
        2 => BuildingMaterial {
            stone_main: mix_color(theme.stone_primary, theme.accent_trim, 0.24),
            stone_alt: mix_color(theme.stone_secondary, theme.accent_trim, 0.32),
            stone_base: mix_color(theme.stone_dark, theme.accent_trim, 0.16),
            trim: mix_color(theme.accent_trim, theme.window_warm, 0.20),
            roof: mix_color(theme.copper_roof, theme.accent_trim, 0.35),
        },
        // 3: Soot-Stained Foundry Ironstone — gunmetal slate with oxidized copper upper works
        _ => BuildingMaterial {
            stone_main: mix_color(theme.stone_primary, theme.steel_viaduct, 0.45),
            stone_alt: scale_color(theme.stone_secondary, 0.94),
            stone_base: scale_color(theme.plinth_dark, 1.12),
            trim: theme.accent_trim,
            roof: scale_color(theme.copper_roof, 1.08),
        },
    }
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
    let usable_d = (d - pad_z * 0.2 * 2.0 - pad_z * 1.6).max(0.2);
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

/// Emit continuous multi-story Art-Deco vertical window channels with dark spandrel dividers
/// and projecting vertical stone mullions (inspired by 1990s Batman / Anton Furst skyscrapers).
#[allow(clippy::too_many_arguments)]
fn push_vertical_deco_light_strips(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    strips: usize,
    seed_variant: u8,
    mat: &BuildingMaterial,
    theme: &GothicThemeColors,
) {
    if h < 0.8 || strips == 0 {
        return;
    }
    let usable_w = (w * 0.68).max(0.24);
    let usable_d = (d * 0.68).max(0.24);
    let strip_h = (h * 0.82).max(0.4);
    let sy = y0 + h * 0.09;
    let step_w = usable_w / strips as f32;
    let step_d = usable_d / strips as f32;
    let chan_w = (step_w * 0.38).clamp(0.06, 0.16);
    let chan_d = (step_d * 0.38).clamp(0.06, 0.16);

    for i in 0..strips {
        let hash = (i as u32)
            .wrapping_mul(29)
            .wrapping_add((seed_variant as u32).wrapping_mul(19));
        let col = if hash.is_multiple_of(3) {
            theme.window_dim
        } else {
            theme.window_warm
        };

        let wx = (cx - usable_w * 0.5) + (i as f32 + 0.5) * step_w;
        let wz = (cz - usable_d * 0.5) + (i as f32 + 0.5) * step_d;

        // Vertical glowing light channel on all 4 faces
        mb.push_box(wx, sy, cz + d * 0.503, chan_w, strip_h, 0.03, col, 0.95);
        mb.push_box(wx, sy, cz - d * 0.503, chan_w, strip_h, 0.03, col, 0.95);
        mb.push_box(cx + w * 0.503, sy, wz, 0.03, strip_h, chan_d, col, 0.95);
        mb.push_box(cx - w * 0.503, sy, wz, 0.03, strip_h, chan_d, col, 0.95);

        // Flanking vertical Art-Deco stone flute rib next to each light channel
        let rib_off_x = step_w * 0.44;
        let rib_off_z = step_d * 0.44;
        mb.push_box(
            wx + rib_off_x,
            y0,
            cz + d * 0.508,
            0.06,
            h - 0.10,
            0.05,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            wx - rib_off_x,
            y0,
            cz - d * 0.508,
            0.06,
            h - 0.10,
            0.05,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            cx + w * 0.508,
            y0,
            wz + rib_off_z,
            0.05,
            h - 0.10,
            0.06,
            mat.stone_alt,
            0.0,
        );
        mb.push_box(
            cx - w * 0.508,
            y0,
            wz - rib_off_z,
            0.05,
            h - 0.10,
            0.06,
            mat.stone_alt,
            0.0,
        );
    }

    // Horizontal dark bronze spandrel bars interrupting the vertical light strips every ~1.1m
    let spandrels = (strip_h / 1.05).floor().max(1.0) as usize;
    for s in 1..spandrels {
        let by = sy + (s as f32 / spandrels as f32) * strip_h;
        mb.push_box(cx, by, cz, w + 0.05, 0.07, d + 0.05, mat.stone_base, 0.0);
    }
}

/// Emit tall double-height pointed-arch gothic lancet windows with glowing tracery heads.
#[allow(clippy::too_many_arguments)]
fn push_gothic_lancet_windows(
    mb: &mut MeshBuilder,
    cx: f32,
    y0: f32,
    cz: f32,
    w: f32,
    h: f32,
    d: f32,
    cols: usize,
    tiers: usize,
    seed_variant: u8,
    theme: &GothicThemeColors,
) {
    if h < 0.8 || cols == 0 || tiers == 0 {
        return;
    }
    let usable_w = (w * 0.68).max(0.24);
    let usable_d = (d * 0.68).max(0.24);
    let usable_h = (h * 0.80).max(0.4);
    let start_y = y0 + h * 0.10;
    let step_w = usable_w / cols as f32;
    let step_d = usable_d / cols as f32;
    let tier_h = usable_h / tiers as f32;
    let win_w = (step_w * 0.44).clamp(0.08, 0.22);
    let win_d = (step_d * 0.44).clamp(0.08, 0.22);
    let body_h = (tier_h * 0.60).clamp(0.18, 0.75);
    let arch_h = (tier_h * 0.18).clamp(0.06, 0.22);

    for t in 0..tiers {
        let wy = start_y + (t as f32 + 0.10) * tier_h;
        for c in 0..cols {
            let hash = (c as u32)
                .wrapping_mul(41)
                .wrapping_add((t as u32).wrapping_mul(23))
                .wrapping_add((seed_variant as u32).wrapping_mul(11));
            let col = if hash.is_multiple_of(3) {
                theme.window_dim
            } else {
                theme.window_warm
            };

            let wx = (cx - usable_w * 0.5) + (c as f32 + 0.5) * step_w;
            let wz = (cz - usable_d * 0.5) + (c as f32 + 0.5) * step_d;

            // Main lancet shaft + narrower pointed arch head on +Z / -Z
            mb.push_box(wx, wy, cz + d * 0.503, win_w, body_h, 0.03, col, 0.95);
            mb.push_box(
                wx,
                wy + body_h,
                cz + d * 0.503,
                win_w * 0.56,
                arch_h,
                0.03,
                theme.window_warm,
                1.0,
            );
            mb.push_box(wx, wy, cz - d * 0.503, win_w, body_h, 0.03, col, 0.95);
            mb.push_box(
                wx,
                wy + body_h,
                cz - d * 0.503,
                win_w * 0.56,
                arch_h,
                0.03,
                theme.window_warm,
                1.0,
            );

            // Main lancet shaft + narrower pointed arch head on +X / -X
            mb.push_box(cx + w * 0.503, wy, wz, 0.03, body_h, win_d, col, 0.95);
            mb.push_box(
                cx + w * 0.503,
                wy + body_h,
                wz,
                0.03,
                arch_h,
                win_d * 0.56,
                theme.window_warm,
                1.0,
            );
            mb.push_box(cx - w * 0.503, wy, wz, 0.03, body_h, win_d, col, 0.95);
            mb.push_box(
                cx - w * 0.503,
                wy + body_h,
                wz,
                0.03,
                arch_h,
                win_d * 0.56,
                theme.window_warm,
                1.0,
            );
        }
    }
}

/// Emit a grand illuminated street-level gothic entrance colonnade / vaulted narthex at the base of a tower.
fn push_street_lobby_colonnade(
    mb: &mut MeshBuilder,
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    mat: &BuildingMaterial,
    theme: &GothicThemeColors,
) {
    if w < 1.3 || d < 1.3 {
        return;
    }
    let lobby_h = 0.68f32;
    let portal_w = (w * 0.36).clamp(0.32, 0.72);
    let portal_d = (d * 0.36).clamp(0.32, 0.72);

    // Street-level projecting stone plinth belt
    mb.push_box(cx, 0.0, cz, w + 0.10, 0.14, d + 0.10, mat.stone_base, 0.0);
    mb.push_box(cx, lobby_h, cz, w + 0.14, 0.08, d + 0.14, mat.trim, 0.0);

    // Warm recessed entrance lobby portals on all 4 street facades
    for sign in [-1.0f32, 1.0f32] {
        mb.push_box(
            cx,
            0.08,
            cz + sign * (d * 0.504),
            portal_w,
            lobby_h - 0.12,
            0.04,
            theme.window_warm,
            0.95,
        );
        mb.push_box(
            cx + sign * (w * 0.504),
            0.08,
            cz,
            0.04,
            lobby_h - 0.12,
            portal_d,
            theme.window_warm,
            0.95,
        );
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
    let mat = building_material(b, theme);

    match b.archetype {
        BuildingArchetype::CathedralKeep => {
            // 1. Heavy buttressed podium base + grand street narthex colonnade
            let h_base = h * 0.34;
            mb.push_box(cx, 0.0, cz, w, h_base, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h_base, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.75,
                cz,
                w,
                h_base - 0.9,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // 2. Flying buttress fins flanking the soaring mid nave
            let h_nave = h * 0.42;
            let nave_w = w * 0.72;
            let nave_d = d * 0.72;
            mb.push_box(cx, h_base, cz, nave_w, h_nave, nave_d, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                h_base,
                cz,
                nave_w,
                h_nave,
                nave_d,
                mat.stone_alt,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                h_base + 0.15,
                cz,
                nave_w,
                h_nave - 0.3,
                nave_d,
                3,
                3,
                b.style_variant + 1,
                theme,
            );

            let rotated = !b.quarter_turns.is_multiple_of(2);
            for side in [-1.0f32, 1.0f32] {
                for offset in [-0.55f32, 0.55f32] {
                    let (px, pz, sx_pier, sz_pier, sx_strut, sz_strut, ax_x, ax_z) = if !rotated {
                        (
                            cx + side * (w * 0.46),
                            cz + offset * d * 0.38,
                            0.20,
                            0.24,
                            cx + side * (w * 0.38),
                            cz + offset * d * 0.38,
                            w * 0.22,
                            0.16,
                        )
                    } else {
                        (
                            cx + offset * w * 0.38,
                            cz + side * (d * 0.46),
                            0.24,
                            0.20,
                            cx + offset * w * 0.38,
                            cz + side * (d * 0.38),
                            0.16,
                            d * 0.22,
                        )
                    };
                    // Outer buttress pier + double-tier flying arch struts
                    mb.push_box(
                        px,
                        h_base,
                        pz,
                        sx_pier,
                        h_nave * 0.78,
                        sz_pier,
                        mat.stone_alt,
                        0.0,
                    );
                    for arch_frac in [0.36f32, 0.66f32] {
                        mb.push_box(
                            sx_strut,
                            h_base + h_nave * arch_frac,
                            sz_strut,
                            ax_x,
                            0.14,
                            ax_z,
                            mat.trim,
                            0.0,
                        );
                    }
                    // Pinnacle tip on outer pier
                    mb.push_pyramid(
                        px,
                        h_base + h_nave * 0.78,
                        pz,
                        0.24,
                        0.68,
                        0.24,
                        mat.roof,
                        0.0,
                    );
                }
            }

            // 3. Twin Gothic Belfry Towers + Rose Window Facade
            let y_top = h_base + h_nave;
            let tower_h = h * 0.24;
            for s in [-1.0f32, 1.0f32] {
                let (tx, tz, tw_x, tw_z) = if !rotated {
                    (cx + s * (nave_w * 0.34), cz, nave_w * 0.36, nave_d * 0.82)
                } else {
                    (cx, cz + s * (nave_d * 0.34), nave_w * 0.82, nave_d * 0.36)
                };
                mb.push_box(tx, y_top, tz, tw_x, tower_h, tw_z, mat.stone_alt, 0.0);
                push_gothic_lancet_windows(
                    mb,
                    tx,
                    y_top + 0.1,
                    tz,
                    tw_x,
                    tower_h - 0.2,
                    tw_z,
                    1,
                    2,
                    b.style_variant + 2,
                    theme,
                );
                mb.push_pyramid(
                    tx,
                    y_top + tower_h,
                    tz,
                    tw_x * 1.08,
                    h * 0.22,
                    tw_z * 1.08,
                    mat.roof,
                    0.0,
                );
            }

            // Glowing Rose Window on front & back of nave
            let rose_y = h_base + h_nave * 0.62;
            if !rotated {
                mb.push_box(
                    cx,
                    rose_y,
                    cz + nave_d * 0.51,
                    0.56,
                    0.56,
                    0.04,
                    theme.window_warm,
                    1.0,
                );
                mb.push_box(
                    cx,
                    rose_y,
                    cz - nave_d * 0.51,
                    0.56,
                    0.56,
                    0.04,
                    theme.window_warm,
                    1.0,
                );
            } else {
                mb.push_box(
                    cx + nave_w * 0.51,
                    rose_y,
                    cz,
                    0.04,
                    0.56,
                    0.56,
                    theme.window_warm,
                    1.0,
                );
                mb.push_box(
                    cx - nave_w * 0.51,
                    rose_y,
                    cz,
                    0.04,
                    0.56,
                    0.56,
                    theme.window_warm,
                    1.0,
                );
            }

            // 4. Soaring Central Needle Spire + Red Warning Beacon
            let spire_base = y_top;
            let spire_h = h * 0.36;
            mb.push_pyramid(
                cx,
                spire_base,
                cz,
                nave_w * 0.40,
                spire_h,
                nave_d * 0.40,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                spire_base + spire_h,
                cz,
                0.12,
                0.15,
                0.12,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ArtDecoZiggurat => {
            // 4-tier stepped Furst/Art-Deco skyscraper with vertical light strips, setback floodlights, and eagle ledges
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            let tiers = [
                (0.0, 0.42, 1.00, 4, 4),
                (0.42, 0.28, 0.78, 3, 4),
                (0.70, 0.18, 0.56, 2, 3),
                (0.88, 0.12, 0.36, 1, 2),
            ];
            for (idx, &(y_frac, h_frac, scale, cols, rows)) in tiers.iter().enumerate() {
                let ty = y_frac * h;
                let th = h_frac * h;
                let tw = w * scale;
                let td = d * scale;
                let col = if idx % 2 == 0 {
                    mat.stone_main
                } else {
                    mat.stone_alt
                };
                mb.push_box(cx, ty, cz, tw, th, td, col, 0.0);
                push_gothic_ribs_and_cornice(mb, cx, ty, cz, tw, th, td, mat.stone_base, mat.trim);
                if idx == 1 || idx == 2 {
                    push_vertical_deco_light_strips(
                        mb,
                        cx,
                        ty,
                        cz,
                        tw,
                        th,
                        td,
                        cols,
                        b.style_variant.wrapping_add(idx as u8),
                        &mat,
                        theme,
                    );
                } else {
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
                }

                // Upward-glowing Art-Deco setback terrace floodlight collar at each step
                if idx > 0 {
                    mb.push_box(
                        cx,
                        ty + 0.02,
                        cz,
                        tw + 0.12,
                        0.08,
                        td + 0.12,
                        theme.window_warm,
                        0.9,
                    );
                }

                // Jutting gargoyle/eagle ledges on the 4 corners of tiers 0, 1, and 2
                if idx <= 2 {
                    let gy = ty + th - 0.08;
                    for sx in [-1.0f32, 1.0f32] {
                        for sz in [-1.0f32, 1.0f32] {
                            mb.push_box(
                                cx + sx * (tw * 0.56),
                                gy,
                                cz + sz * (td * 0.56),
                                0.28,
                                0.12,
                                0.28,
                                mat.trim,
                                0.0,
                            );
                        }
                    }
                }
            }
            // Crown needle spire + glowing lantern
            mb.push_box(cx, h, cz, w * 0.24, 0.36, d * 0.24, theme.window_warm, 1.0);
            mb.push_pyramid(
                cx,
                h + 0.36,
                cz,
                w * 0.28,
                h * 0.22,
                d * 0.28,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                h + 0.36 + h * 0.22,
                cz,
                0.11,
                0.14,
                0.11,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ClockworkBelfry => {
            // Slender campanile shaft with vertical deco lancet channels
            let shaft_h = h * 0.74;
            mb.push_box(
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                mat.stone_main,
                0.0,
            );
            push_street_lobby_colonnade(mb, cx, cz, w * 0.86, d * 0.86, &mat, theme);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                0.0,
                cz,
                w * 0.86,
                shaft_h,
                d * 0.86,
                mat.stone_base,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                0.7,
                cz,
                w * 0.86,
                shaft_h - 0.9,
                d * 0.86,
                2,
                4,
                b.style_variant,
                theme,
            );

            // Flared Clock Chamber at top of shaft
            let clock_h = h * 0.18;
            let cw = w * 0.96;
            let cd = d * 0.96;
            mb.push_box(cx, shaft_h, cz, cw, clock_h, cd, mat.stone_alt, 0.0);
            mb.push_box(
                cx,
                shaft_h + clock_h - 0.10,
                cz,
                cw + 0.20,
                0.12,
                cd + 0.20,
                mat.trim,
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
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz + cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                mat.stone_base,
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
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + face_size * 0.16,
                f_mid_y,
                cz - cd * 0.525,
                face_size * 0.34,
                0.06,
                0.04,
                mat.stone_base,
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
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx + cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                mat.stone_base,
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
                mat.stone_base,
                0.0,
            );
            mb.push_box(
                cx - cw * 0.525,
                f_mid_y,
                cz + face_size * 0.16,
                0.04,
                0.06,
                face_size * 0.34,
                mat.stone_base,
                0.0,
            );

            // Steep belfry spire roof + tall needle finial + corner pinnacles
            let roof_y = shaft_h + clock_h;
            mb.push_pyramid(
                cx,
                roof_y,
                cz,
                cw * 0.90,
                h * 0.32,
                cd * 0.90,
                mat.roof,
                0.0,
            );
            mb.push_pyramid(
                cx,
                roof_y + h * 0.28,
                cz,
                cw * 0.18,
                h * 0.16,
                cd * 0.18,
                mat.trim,
                0.0,
            );
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    mb.push_pyramid(
                        cx + sx * (cw * 0.42),
                        roof_y,
                        cz + sz * (cd * 0.42),
                        0.20,
                        0.52,
                        0.20,
                        mat.trim,
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
                mat.stone_base,
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
                mat.stone_main,
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
            // Brutalist-gothic corporate citadel with style_variant crown differentiation
            let h1 = h * 0.58;
            let h2 = h * 0.30;
            let h3 = h * 0.12;
            mb.push_box(cx, 0.0, cz, w, h1, d, mat.stone_main, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h1, d, mat.stone_base, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                0.75,
                cz,
                w,
                h1 - 0.90,
                d,
                3,
                b.style_variant,
                &mat,
                theme,
            );

            mb.push_box(cx, h1, cz, w * 0.82, h2, d * 0.82, mat.stone_alt, 0.0);
            push_gothic_ribs_and_cornice(
                mb,
                cx,
                h1,
                cz,
                w * 0.82,
                h2,
                d * 0.82,
                mat.stone_base,
                mat.trim,
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

            mb.push_box(cx, h1 + h2, cz, w * 0.56, h3, d * 0.56, mat.stone_base, 0.0);

            if b.style_variant.is_multiple_of(2) {
                // Variant A: Brutalist twin-turret corporate crown + tall radio spire
                for s in [-1.0f32, 1.0f32] {
                    mb.push_box(
                        cx + s * (w * 0.16),
                        h,
                        cz,
                        w * 0.20,
                        h * 0.12,
                        d * 0.42,
                        mat.stone_alt,
                        0.0,
                    );
                    mb.push_pyramid(
                        cx + s * (w * 0.16),
                        h + h * 0.12,
                        cz,
                        w * 0.22,
                        h * 0.14,
                        d * 0.44,
                        mat.roof,
                        0.0,
                    );
                }
            } else {
                // Variant B: Steep chateau mansard frustum roof + glowing attic lantern
                mb.push_frustum(
                    cx,
                    h,
                    cz,
                    w * 0.58,
                    d * 0.58,
                    w * 0.26,
                    d * 0.26,
                    h * 0.16,
                    mat.roof,
                    0.0,
                );
                mb.push_box(
                    cx,
                    h + h * 0.16,
                    cz,
                    w * 0.28,
                    0.12,
                    d * 0.28,
                    theme.window_warm,
                    1.0,
                );
                mb.push_pyramid(
                    cx,
                    h + h * 0.16 + 0.12,
                    cz,
                    w * 0.26,
                    h * 0.12,
                    d * 0.26,
                    mat.trim,
                    0.0,
                );
            }
        }

        BuildingArchetype::ViaductStation => {
            // Grand Gothic Transit Station straddling the elevated train track at y = 2.6!
            // Build West and East flanking pier walls so the center (x = cx) has an open portal
            // tunnel from y = 2.4 to y = 3.85!
            let pier_w = w * 0.28;
            let portal_top_y = 3.85;
            // Lower solid station base below track level (0.0 .. 2.55)
            mb.push_box(cx, 0.0, cz, w, 2.55, d, mat.stone_base, 0.0);
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
                    mat.stone_alt,
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
                mat.stone_main,
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
                mat.stone_alt,
                mat.trim,
            );
            push_gothic_lancet_windows(
                mb,
                cx,
                portal_top_y + 0.15,
                cz,
                w * 0.92,
                upper_h * 0.65 - 0.3,
                d * 0.86,
                3,
                2,
                b.style_variant,
                theme,
            );

            let top_y = portal_top_y + upper_h * 0.65;
            mb.push_box(
                cx,
                top_y,
                cz,
                w * 0.56,
                upper_h * 0.35,
                d * 0.56,
                mat.stone_alt,
                0.0,
            );
            mb.push_pyramid(cx, h, cz, w * 0.60, upper_h * 0.42, d * 0.60, mat.roof, 0.0);
        }

        BuildingArchetype::OctagonalCitadel => {
            // Eight-sided Gothic Rotunda Keep built from overlapping 0° and 45° rotated drums,
            // breaking up the strict 90-degree rectangular silhouette of standard city blocks!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];
            let r_base = w.min(d) * 0.86;
            let h1 = h * 0.38;
            let h2 = h * 0.34;
            let h3 = h * 0.28;

            // Tier 1: Heavy Octagonal Bastion Podium + 45° Diagonal Buttress Fins
            mb.push_box(cx, 0.0, cz, r_base, h1, r_base, mat.stone_base, 0.0);
            mb.push_oriented_box(
                [cx, 0.0, cz],
                diag,
                r_base,
                h1 - 0.04,
                r_base,
                mat.stone_main,
                0.0,
            );
            mb.push_box(
                cx,
                h1 - 0.10,
                cz,
                r_base + 0.18,
                0.12,
                r_base + 0.18,
                mat.trim,
                0.0,
            );
            mb.push_oriented_box(
                [cx, h1 - 0.10, cz],
                diag,
                r_base + 0.18,
                0.12,
                r_base + 0.18,
                mat.trim,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                0.22,
                cz,
                r_base,
                h1 - 0.35,
                r_base,
                3,
                3,
                b.style_variant,
                theme,
            );

            // Tier 2: Stepped Octagonal Rotunda Drum + Radial Flying Buttresses
            let r_mid = r_base * 0.74;
            mb.push_box(cx, h1, cz, r_mid, h2, r_mid, mat.stone_main, 0.0);
            mb.push_oriented_box(
                [cx, h1, cz],
                diag,
                r_mid,
                h2 - 0.03,
                r_mid,
                mat.stone_alt,
                0.0,
            );
            push_gothic_window_bands(
                mb,
                cx,
                h1 + 0.15,
                cz,
                r_mid,
                h2 - 0.30,
                r_mid,
                2,
                3,
                b.style_variant + 1,
                theme,
            );
            // 45-degree diagonal buttress ribs & corner turrets on the setback terrace
            for sign_u in [-1.0f32, 1.0f32] {
                for sign_v in [-1.0f32, 1.0f32] {
                    let bx = cx + sign_u * (r_base * 0.42);
                    let bz = cz + sign_v * (r_base * 0.42);
                    mb.push_oriented_box(
                        [bx, h1, bz],
                        diag,
                        0.24,
                        h2 * 0.65,
                        0.24,
                        mat.stone_alt,
                        0.0,
                    );
                    mb.push_pyramid(bx, h1 + h2 * 0.65, bz, 0.26, 0.45, 0.26, mat.roof, 0.0);
                }
            }

            // Tier 3: Upper Octagonal Lantern Keep & Pointed Spire
            let y2 = h1 + h2;
            let r_top = r_base * 0.50;
            mb.push_box(cx, y2, cz, r_top, h3, r_top, mat.stone_alt, 0.0);
            mb.push_oriented_box(
                [cx, y2, cz],
                diag,
                r_top,
                h3 - 0.02,
                r_top,
                mat.stone_main,
                0.0,
            );
            // Glowing clerestory lantern band around all 8 faces
            mb.push_box(
                cx,
                y2 + h3 * 0.45,
                cz,
                r_top + 0.04,
                0.20,
                r_top + 0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_oriented_box(
                [cx, y2 + h3 * 0.45, cz],
                diag,
                r_top + 0.04,
                0.20,
                r_top + 0.04,
                theme.window_warm,
                1.0,
            );
            // Octagonal spire crown + ruby beacon
            mb.push_pyramid(
                cx,
                h,
                cz,
                r_top * 1.04,
                h * 0.28,
                r_top * 1.04,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                h + h * 0.28,
                cz,
                0.11,
                0.12,
                0.11,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::GothicMonumentPlaza => {
            // Open Old-World Gothic Civic Piazza with a stepped octagonal monument dais,
            // central obelisk spire, glowing corner colonnade pavilions, and warm braziers.
            // Leaves dramatic open sightlines across the middle of the 4x4 city!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];

            // 1. Stepped Octagonal Courtyard Dais in the center of the piazza
            let dais_w = w.min(d) * 0.76;
            mb.push_box(cx, 0.08, cz, dais_w, 0.14, dais_w, mat.stone_alt, 0.0);
            mb.push_oriented_box(
                [cx, 0.08, cz],
                diag,
                dais_w,
                0.13,
                dais_w,
                mat.stone_main,
                0.0,
            );
            mb.push_box(
                cx,
                0.22,
                cz,
                dais_w * 0.64,
                0.18,
                dais_w * 0.64,
                mat.trim,
                0.0,
            );
            mb.push_oriented_box(
                [cx, 0.22, cz],
                diag,
                dais_w * 0.64,
                0.17,
                dais_w * 0.64,
                mat.trim,
                0.0,
            );

            // 2. Illuminated Fountain Basin / Sanctuary Pedestal
            let ped_w = dais_w * 0.38;
            mb.push_box(cx, 0.40, cz, ped_w, 0.55, ped_w, mat.stone_base, 0.0);
            mb.push_box(
                cx,
                0.56,
                cz,
                ped_w + 0.08,
                0.14,
                ped_w + 0.08,
                theme.window_warm,
                0.95,
            );

            // 3. Soaring Gothic Obelisk Needle in the center of the Piazza
            let obelisk_h = (h - 0.95).max(2.2);
            mb.push_frustum(
                cx,
                0.95,
                cz,
                ped_w * 0.58,
                ped_w * 0.58,
                ped_w * 0.24,
                ped_w * 0.24,
                obelisk_h * 0.72,
                mat.stone_alt,
                0.0,
            );
            let pyramidion_y = 0.95 + obelisk_h * 0.72;
            mb.push_pyramid(
                cx,
                pyramidion_y,
                cz,
                ped_w * 0.30,
                obelisk_h * 0.28,
                ped_w * 0.30,
                mat.roof,
                0.0,
            );
            mb.push_box(
                cx,
                0.95 + obelisk_h,
                cz,
                0.12,
                0.12,
                0.12,
                theme.window_warm,
                1.0,
            );

            // 4. Four Corner Gothic Colonnade Pavilions & Glowing Braziers framing the square
            let pav_w = (w.min(d) * 0.22).clamp(0.28, 0.52);
            let pav_h = (h * 0.34).clamp(1.1, 1.65);
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    let px = cx + sx * (w * 0.5 - pav_w * 0.55);
                    let pz = cz + sz * (d * 0.5 - pav_w * 0.55);
                    mb.push_box(px, 0.08, pz, pav_w, pav_h, pav_w, mat.stone_main, 0.0);
                    mb.push_box(
                        px,
                        0.08 + pav_h * 0.58,
                        pz,
                        pav_w + 0.04,
                        0.16,
                        pav_w + 0.04,
                        theme.window_warm,
                        1.0,
                    );
                    mb.push_pyramid(
                        px,
                        0.08 + pav_h,
                        pz,
                        pav_w * 1.12,
                        0.48,
                        pav_w * 1.12,
                        mat.roof,
                        0.0,
                    );
                }
            }
        }

        BuildingArchetype::AtlasDecoTower => {
            // Soaring slender Art-Deco supertall monolith with continuous vertical light strips,
            // cantilevered titan/gargoyle shoulder buttresses, and a Chrysler/Furst sunburst spire!
            let h0 = h * 0.34;
            let h1 = h * 0.34;
            let h2 = h * 0.20;
            let h3 = h * 0.12;

            // Tier 0: Grand Podium Base (0.0 .. h * 0.34, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                0.78,
                cz,
                w,
                h0 - 0.92,
                d,
                4,
                b.style_variant,
                &mat,
                theme,
            );

            // Tier 1: Soaring Fluted Deco Shaft (h * 0.34 .. h * 0.68, scale = 0.82)
            let y1 = h0;
            let w1 = w * 0.82;
            let d1 = d * 0.82;
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y1, cz, w1, h1, d1, mat.stone_alt, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                y1 + 0.12,
                cz,
                w1,
                h1 - 0.28,
                d1,
                3,
                b.style_variant + 1,
                &mat,
                theme,
            );

            // Tier 2: Cantilevered Shoulder Lantern Tier (h * 0.68 .. h * 0.88, scale = 0.64)
            let y2 = h0 + h1;
            let w2 = w * 0.64;
            let d2 = d * 0.64;
            mb.push_box(cx, y2, cz, w2, h2, d2, mat.stone_alt, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y2, cz, w2, h2, d2, mat.stone_base, mat.trim);
            push_vertical_deco_light_strips(
                mb,
                cx,
                y2 + 0.10,
                cz,
                w2,
                h2 - 0.24,
                d2,
                2,
                b.style_variant + 2,
                &mat,
                theme,
            );

            // 4 Cantilevered Titan/Gargoyle Shoulder Buttresses on the 4 cardinal faces at y2
            for sign in [-1.0f32, 1.0f32] {
                mb.push_frustum(
                    cx + sign * (w1 * 0.44),
                    y2,
                    cz,
                    0.34,
                    d2 * 0.42,
                    0.18,
                    d2 * 0.26,
                    h2 * 0.82,
                    mat.trim,
                    0.0,
                );
                mb.push_frustum(
                    cx,
                    y2,
                    cz + sign * (d1 * 0.44),
                    w2 * 0.42,
                    0.34,
                    w2 * 0.26,
                    0.18,
                    h2 * 0.82,
                    mat.trim,
                    0.0,
                );
            }

            // Tier 3: Sunburst Crown Observatory (h * 0.88 .. h, scale = 0.44)
            let y3 = y2 + h2;
            let w3 = w * 0.44;
            let d3 = d * 0.44;
            mb.push_box(cx, y3, cz, w3, h3, d3, mat.stone_main, 0.0);
            mb.push_box(
                cx,
                y3 + h3 * 0.35,
                cz,
                w3 + 0.06,
                h3 * 0.38,
                d3 + 0.06,
                theme.window_warm,
                1.0,
            );
            // Sunburst Art-Deco Spire + Ruby Beacon
            mb.push_pyramid(cx, h, cz, w3 * 1.12, h * 0.24, d3 * 1.12, mat.roof, 0.0);
            mb.push_pyramid(
                cx,
                h + h * 0.18,
                cz,
                w3 * 0.32,
                h * 0.14,
                d3 * 0.32,
                mat.trim,
                0.0,
            );
            mb.push_box(
                cx,
                h + h * 0.32,
                cz,
                0.12,
                0.14,
                0.12,
                theme.beacon_red,
                1.0,
            );
        }

        BuildingArchetype::ChanceryArcadeKeep => {
            // Twin-Spire Gothic Chancery Citadel with an integral high-altitude vaulted sky-arch
            // bridge joining its asymmetric twin towers!
            let h0 = h * 0.38;
            let h1 = h * 0.28;
            let h2 = h * 0.34;

            // Tier 0: Heavy Vaulted Chancery Hall (0.0 .. h * 0.38, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.76,
                cz,
                w,
                h0 - 0.90,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // Tier 1: Stepped Mid-Keep Envelope (h * 0.38 .. h * 0.66, scale = 0.86)
            let y1 = h0;
            let w1 = w * 0.86;
            let d1 = d * 0.86;
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            push_gothic_ribs_and_cornice(mb, cx, y1, cz, w1, h1, d1, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                y1 + 0.12,
                cz,
                w1,
                h1 - 0.26,
                d1,
                3,
                2,
                b.style_variant + 1,
                theme,
            );

            // Tier 2: Twin Asymmetric Gothic Towers (h * 0.66 .. h, outer envelope scale = 0.76)
            // joined near the top by an integral vaulted stone Sky-Arch Bridge!
            let y2 = h0 + h1;
            let w2 = w * 0.76;
            let d2 = d * 0.76;
            let rotated = !b.quarter_turns.is_multiple_of(2);

            for (idx, side) in [-1.0f32, 1.0f32].iter().enumerate() {
                // Slight height asymmetry between the twin spires for organic gothic drama
                let spire_scale = if idx == 0 { 1.06 } else { 0.92 };
                let tw_h = h2 * spire_scale;
                let (tx, tz, tw_x, tw_z) = if !rotated {
                    (cx + side * (w2 * 0.31), cz, w2 * 0.38, d2)
                } else {
                    (cx, cz + side * (d2 * 0.31), w2, d2 * 0.38)
                };
                mb.push_box(tx, y2, tz, tw_x, tw_h, tw_z, mat.stone_alt, 0.0);
                push_gothic_ribs_and_cornice(
                    mb,
                    tx,
                    y2,
                    tz,
                    tw_x,
                    tw_h,
                    tw_z,
                    mat.stone_base,
                    mat.trim,
                );
                push_gothic_lancet_windows(
                    mb,
                    tx,
                    y2 + 0.12,
                    tz,
                    tw_x,
                    tw_h - 0.26,
                    tw_z,
                    1,
                    2,
                    b.style_variant + idx as u8,
                    theme,
                );
                // Steep copper gable/pyramid spire on each twin tower
                mb.push_pyramid(
                    tx,
                    y2 + tw_h,
                    tz,
                    tw_x * 1.10,
                    h * 0.20 * spire_scale,
                    tw_z * 1.10,
                    mat.roof,
                    0.0,
                );
            }

            // Integral High-Altitude Sky-Arch Bridge spanning between the twin towers
            let arch_y = y2 + h2 * 0.56;
            let (arch_w, arch_d) = if !rotated {
                (w2 * 0.52, d2 * 0.44)
            } else {
                (w2 * 0.44, d2 * 0.52)
            };
            mb.push_box(cx, arch_y, cz, arch_w, 0.38, arch_d, mat.trim, 0.0);
            mb.push_box(
                cx,
                arch_y + 0.10,
                cz,
                arch_w + 0.04,
                0.16,
                arch_d + 0.04,
                theme.window_warm,
                1.0,
            );
            mb.push_pyramid(cx, arch_y + 0.38, cz, 0.26, 0.55, 0.26, mat.roof, 0.0);
        }

        BuildingArchetype::AlchemicalObservatory => {
            // Stepped Gothic-Industrial Bastion crowned by an 8-sided ribbed verdigris copper
            // observatory dome, glowing astronomical aperture, and corner lightning-coil pinnacles!
            let diag = [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ];
            let h0 = h * 0.44;
            let h1 = h * 0.32;
            let h2 = h * 0.24;

            // Tier 0: Buttressed Laboratory Citadel (0.0 .. h * 0.44, scale = 1.00)
            mb.push_box(cx, 0.0, cz, w, h0, d, mat.stone_base, 0.0);
            push_street_lobby_colonnade(mb, cx, cz, w, d, &mat, theme);
            push_gothic_ribs_and_cornice(mb, cx, 0.0, cz, w, h0, d, mat.stone_alt, mat.trim);
            push_gothic_lancet_windows(
                mb,
                cx,
                0.76,
                cz,
                w,
                h0 - 0.90,
                d,
                3,
                2,
                b.style_variant,
                theme,
            );

            // Tier 1: Stepped Octagonal Instrument Drum (h * 0.44 .. h * 0.76, scale = 0.80)
            let y1 = h0;
            let w1 = w * 0.80;
            let d1 = d * 0.80;
            let r1 = w1.min(d1);
            mb.push_box(cx, y1, cz, w1, h1, d1, mat.stone_main, 0.0);
            mb.push_oriented_box([cx, y1, cz], diag, r1, h1 - 0.04, r1, mat.stone_alt, 0.0);
            push_gothic_window_bands(
                mb,
                cx,
                y1 + 0.14,
                cz,
                w1,
                h1 - 0.28,
                d1,
                2,
                2,
                b.style_variant + 2,
                theme,
            );

            // 4 Corner Tesla / Lightning-Coil Pinnacles on the Tier 1 terrace
            for sx in [-1.0f32, 1.0f32] {
                for sz in [-1.0f32, 1.0f32] {
                    let px = cx + sx * (w * 0.42);
                    let pz = cz + sz * (d * 0.42);
                    mb.push_box(px, y1, pz, 0.20, h1 * 0.55, 0.20, mat.stone_alt, 0.0);
                    mb.push_box(
                        px,
                        y1 + h1 * 0.55,
                        pz,
                        0.16,
                        0.12,
                        0.16,
                        theme.window_dim,
                        0.95,
                    );
                    mb.push_pyramid(
                        px,
                        y1 + h1 * 0.55 + 0.12,
                        pz,
                        0.14,
                        0.48,
                        0.14,
                        mat.trim,
                        0.0,
                    );
                }
            }

            // Tier 2: 8-Sided Ribbed Copper Observatory Dome (h * 0.76 .. h, scale = 0.62)
            let y2 = h0 + h1;
            let r2 = w.min(d) * 0.62;
            // Lower dome drum + glowing clerestory ring
            mb.push_box(cx, y2, cz, r2, h2 * 0.42, r2, mat.stone_alt, 0.0);
            mb.push_oriented_box([cx, y2, cz], diag, r2, h2 * 0.40, r2, mat.stone_main, 0.0);
            mb.push_box(
                cx,
                y2 + h2 * 0.18,
                cz,
                r2 + 0.05,
                0.14,
                r2 + 0.05,
                theme.window_warm,
                1.0,
            );

            // Stepped octagonal copper dome hemisphere + meridian ribs
            let dome_y = y2 + h2 * 0.42;
            let dome_h = h2 * 0.78;
            mb.push_frustum(
                cx,
                dome_y,
                cz,
                r2 * 0.98,
                r2 * 0.98,
                r2 * 0.62,
                r2 * 0.62,
                dome_h * 0.58,
                mat.roof,
                0.0,
            );
            mb.push_pyramid(
                cx,
                dome_y + dome_h * 0.58,
                cz,
                r2 * 0.64,
                dome_h * 0.55,
                r2 * 0.64,
                mat.roof,
                0.0,
            );

            // Glowing Astronomical Telescope Shutter Slit & Projecting Cupola
            mb.push_box(
                cx,
                dome_y + dome_h * 0.12,
                cz + r2 * 0.42,
                r2 * 0.28,
                dome_h * 0.52,
                0.26,
                mat.trim,
                0.0,
            );
            mb.push_box(
                cx,
                dome_y + dome_h * 0.16,
                cz + r2 * 0.56,
                r2 * 0.18,
                dome_h * 0.40,
                0.06,
                theme.window_warm,
                1.0,
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

    // Penetrate 0.28m deep inside both connected building walls so neither orthogonal
    // nor oblique skybridges ever show a gap against stepped facades!
    let embed = 0.28f32;
    let deck_len = len + embed * 2.0;

    // 1. Bridge deck slab + lower molded stone soffit beam
    mb.push_oriented_box(
        [cx, y - 0.11, cz],
        fwd,
        deck_len,
        0.11,
        span.width,
        theme.stone_secondary,
        0.0,
    );
    mb.push_oriented_box(
        [cx, y - 0.17, cz],
        fwd,
        deck_len,
        0.06,
        span.width * 0.78,
        theme.stone_dark,
        0.0,
    );

    // 2. Left and right gothic parapet railings + warm walkway balustrade lamps
    let rail_w = 0.055;
    let rail_h = 0.15;
    for side in [-1.0f32, 1.0f32] {
        let rx = cx + right[0] * (span.width * 0.5 - rail_w * 0.5) * side;
        let rz = cz + right[1] * (span.width * 0.5 - rail_w * 0.5) * side;
        mb.push_oriented_box(
            [rx, y, rz],
            fwd,
            deck_len - 0.04,
            rail_h,
            rail_w,
            theme.accent_trim,
            0.0,
        );
    }

    // 3. Cantilevered Gothic Skybridge Vestibules & Under-Deck Stone Corbels at both building walls
    for (end_pt, into_wall_sign) in [(span.start, -1.0f32), (span.end, 1.0f32)] {
        let wall_dir = [fwd[0] * into_wall_sign, fwd[1] * into_wall_sign];
        // Stepped stone corbel bracket supporting the bridge haunch from the building wall
        let corbel_cx = end_pt[0] - wall_dir[0] * 0.10;
        let corbel_cz = end_pt[1] - wall_dir[1] * 0.10;
        mb.push_oriented_box(
            [corbel_cx, y - 0.42, corbel_cz],
            fwd,
            0.38,
            0.26,
            span.width * 0.90,
            theme.stone_dark,
            0.0,
        );
        mb.push_oriented_box(
            [
                corbel_cx + wall_dir[0] * 0.06,
                y - 0.62,
                corbel_cz + wall_dir[1] * 0.06,
            ],
            fwd,
            0.24,
            0.20,
            span.width * 0.72,
            theme.stone_secondary,
            0.0,
        );

        // Protruding Gothic Stone Vestibule Surround on the building facade
        let vest_cx = end_pt[0] + wall_dir[0] * 0.04;
        let vest_cz = end_pt[1] + wall_dir[1] * 0.04;
        mb.push_oriented_box(
            [vest_cx, y - 0.06, vest_cz],
            fwd,
            0.30,
            0.48,
            span.width + 0.20,
            theme.stone_secondary,
            0.0,
        );
        // Overhanging copper/brass portal pediment hood above the skybridge entrance
        mb.push_oriented_box(
            [
                vest_cx - wall_dir[0] * 0.03,
                y + 0.40,
                vest_cz - wall_dir[1] * 0.03,
            ],
            fwd,
            0.34,
            0.10,
            span.width + 0.28,
            theme.accent_trim,
            0.0,
        );
        // Warm glowing arched portal doorway where pedestrians enter/exit the building
        mb.push_oriented_box(
            [
                end_pt[0] - wall_dir[0] * 0.08,
                y,
                end_pt[1] - wall_dir[1] * 0.08,
            ],
            fwd,
            0.12,
            0.36,
            span.width * 0.68,
            theme.window_warm,
            0.95,
        );
    }

    // 4. Style-specific arch ribs, iron trusses, covered galleries, or grand vaulted arcades
    match span.style {
        WalkwayStyle::StoneArchBridge => {
            // Underneath stepped stone arch haunches & central keystone rib
            for t in [0.22f32, 0.78f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                mb.push_oriented_box(
                    [px, y - 0.34, pz],
                    fwd,
                    len * 0.24,
                    0.20,
                    span.width * 0.84,
                    theme.stone_dark,
                    0.0,
                );
            }
            // Mid-span gothic lamp standards on both parapets
            for side in [-1.0f32, 1.0f32] {
                let lx = cx + right[0] * (span.width * 0.46) * side;
                let lz = cz + right[1] * (span.width * 0.46) * side;
                mb.push_box(lx, y + rail_h, lz, 0.06, 0.18, 0.06, theme.stone_dark, 0.0);
                mb.push_box(
                    lx,
                    y + rail_h + 0.18,
                    lz,
                    0.10,
                    0.09,
                    0.10,
                    theme.window_warm,
                    1.0,
                );
            }
        }
        WalkwayStyle::IronTrussSkybridge => {
            // Overhead riveted iron portal frames + longitudinal top chords along the bridge
            for t in [0.22f32, 0.50f32, 0.78f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                for side in [-1.0f32, 1.0f32] {
                    let post_x = px + right[0] * (span.width * 0.48) * side;
                    let post_z = pz + right[1] * (span.width * 0.48) * side;
                    mb.push_oriented_box(
                        [post_x, y, post_z],
                        fwd,
                        0.06,
                        0.32,
                        0.06,
                        theme.steel_viaduct,
                        0.0,
                    );
                }
                mb.push_oriented_box(
                    [px, y + 0.30, pz],
                    fwd,
                    0.07,
                    0.06,
                    span.width + 0.08,
                    theme.steel_viaduct,
                    0.0,
                );
            }
        }
        WalkwayStyle::CoveredGallery => {
            // Covered gothic gallery roof with side buttress mullions and warm clerestory slits
            for t in [0.25f32, 0.50f32, 0.75f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                for side in [-1.0f32, 1.0f32] {
                    let mx = px + right[0] * (span.width * 0.46) * side;
                    let mz = pz + right[1] * (span.width * 0.46) * side;
                    mb.push_oriented_box(
                        [mx, y + rail_h, mz],
                        fwd,
                        0.08,
                        0.20,
                        0.05,
                        theme.stone_secondary,
                        0.0,
                    );
                }
            }
            mb.push_oriented_box(
                [cx, y + 0.32, cz],
                fwd,
                deck_len - 0.06,
                0.11,
                span.width + 0.10,
                theme.copper_roof,
                0.0,
            );
        }
        WalkwayStyle::GrandVaultedArcade => {
            // High-altitude vaulted cathedral skybridge with flying-buttress under-ribs,
            // arched colonnade sides, pitched copper roof, and central lantern pinnacle!
            for t in [0.20f32, 0.50f32, 0.80f32] {
                let px = span.start[0] + dx * t;
                let pz = span.start[1] + dz * t;
                mb.push_oriented_box(
                    [px, y - 0.36, pz],
                    fwd,
                    len * 0.18,
                    0.22,
                    span.width * 0.86,
                    theme.stone_dark,
                    0.0,
                );
                for side in [-1.0f32, 1.0f32] {
                    let mx = px + right[0] * (span.width * 0.48) * side;
                    let mz = pz + right[1] * (span.width * 0.48) * side;
                    mb.push_oriented_box(
                        [mx, y + rail_h, mz],
                        fwd,
                        0.10,
                        0.26,
                        0.06,
                        theme.accent_trim,
                        0.0,
                    );
                }
            }
            // Vaulted copper gallery roof + central glowing lantern turret
            mb.push_oriented_box(
                [cx, y + 0.38, cz],
                fwd,
                deck_len - 0.04,
                0.12,
                span.width + 0.12,
                theme.copper_roof,
                0.0,
            );
            mb.push_box(cx, y + 0.50, cz, 0.18, 0.16, 0.18, theme.window_warm, 1.0);
            mb.push_pyramid(cx, y + 0.66, cz, 0.22, 0.34, 0.22, theme.copper_roof, 0.0);
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

        // Junction turret cap at each waypoint corner so angled/dogleg viaduct turns join seamlessly
        mb.push_box(
            a[0],
            y - 0.18,
            a[1],
            deck_w * 1.06,
            0.18,
            deck_w * 1.06,
            theme.steel_viaduct,
            0.0,
        );

        // Main steel viaduct girder deck
        mb.push_oriented_box(
            [cx, y - 0.16, cz],
            fwd,
            len + 0.44,
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
                len + 0.36,
                0.09,
                0.07,
                theme.accent_trim,
                0.0,
            );
        }

        // Support columns descending from viaduct to street level (skipping inside buildings/stations)
        let pier_steps = (len / 2.2).ceil().max(1.0) as usize;
        for p in 0..pier_steps {
            let t = (p as f32 + 0.5) / pier_steps as f32;
            let px = a[0] + dx * t;
            let pz = a[1] + dz * t;
            // Skip piers inside any building footprint (such as ViaductStation)
            let inside_building = bp.buildings.iter().any(|bld| {
                (px - bld.x).abs() < bld.width * 0.5 + 0.35
                    && (pz - bld.z).abs() < bld.depth * 0.5 + 0.35
            });
            if inside_building {
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
/// Configured so the expanded 4x4 city island occupies roughly **three-fifths (~60%)** of the
/// viewport centered in a stark black void.
fn push_vehicle_mesh(
    opaque_mb: &mut MeshBuilder,
    trans_mb: &mut MeshBuilder,
    veh: &GothicVehicle,
    pos: [f32; 3],
    fwd: [f32; 2],
    time: f32,
    theme: &GothicThemeColors,
) {
    let vx = pos[0];
    let vy = pos[1];
    let vz = pos[2];
    let right = [-fwd[1], fwd[0]];

    let (car_len, car_w, car_h) = match veh.kind {
        GothicVehicleKind::NoirCoupe => (0.60, 0.22, 0.14),
        GothicVehicleKind::ArmoredCruiser => (0.64, 0.25, 0.18),
        GothicVehicleKind::TransitOmnibus => (0.88, 0.26, 0.24),
        GothicVehicleKind::VigilanteInterceptor => (0.72, 0.26, 0.13),
        GothicVehicleKind::SteamLimousine => (0.84, 0.23, 0.16),
    };

    let body_color = match veh.kind {
        GothicVehicleKind::VigilanteInterceptor => theme.plinth_dark,
        GothicVehicleKind::ArmoredCruiser => theme.stone_dark,
        GothicVehicleKind::TransitOmnibus => theme.stone_secondary,
        GothicVehicleKind::SteamLimousine => theme.plinth_dark,
        GothicVehicleKind::NoirCoupe => theme.plinth_dark,
    };

    // Main lower chassis
    opaque_mb.push_oriented_box(
        [vx, vy + 0.015, vz],
        fwd,
        car_len,
        car_h * 0.56,
        car_w,
        body_color,
        0.0,
    );

    // Four sculpted wheel arches / tires
    for &ax in &[-car_len * 0.30, car_len * 0.30] {
        for &side in &[-1.0f32, 1.0f32] {
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * ax + right[0] * (car_w * 0.48 * side),
                    vy,
                    vz + fwd[1] * ax + right[1] * (car_w * 0.48 * side),
                ],
                fwd,
                0.13,
                0.06,
                0.045,
                theme.stone_dark,
                0.0,
            );
        }
    }

    // Kind-specific upper silhouette & details
    match veh.kind {
        GothicVehicleKind::NoirCoupe => {
            // Swept-back art-deco cabin + chrome grille + twin rear tailfins
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.12),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.12),
                ],
                fwd,
                car_len * 0.44,
                car_h * 0.44,
                car_w * 0.82,
                theme.stone_secondary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.49),
                    vy + 0.03,
                    vz + fwd[1] * (car_len * 0.49),
                ],
                fwd,
                0.03,
                0.055,
                car_w * 0.42,
                theme.steel_viaduct,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.42) + right[0] * (car_w * 0.42 * side),
                        vy + car_h * 0.56,
                        vz - fwd[1] * (car_len * 0.42) + right[1] * (car_w * 0.42 * side),
                    ],
                    fwd,
                    0.14,
                    0.06,
                    0.028,
                    theme.accent_trim,
                    0.0,
                );
            }
        }
        GothicVehicleKind::ArmoredCruiser => {
            // Heavy ram-bumper + armored cabin + alternating flashing roof lightbar
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.51),
                    vy + 0.02,
                    vz + fwd[1] * (car_len * 0.51),
                ],
                fwd,
                0.04,
                0.05,
                car_w * 1.04,
                theme.steel_viaduct,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.06),
                ],
                fwd,
                car_len * 0.54,
                car_h * 0.46,
                car_w * 0.88,
                theme.stone_primary,
                0.0,
            );
            let flash = ((time * 9.0 + veh.id as f32 * 1.3).sin()) >= 0.0;
            let left_col = if flash {
                theme.beacon_red
            } else {
                theme.searchlight
            };
            let right_col = if flash {
                theme.searchlight
            } else {
                theme.beacon_red
            };
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06) - right[0] * (car_w * 0.20),
                    vy + car_h * 1.04,
                    vz - fwd[1] * (car_len * 0.06) - right[1] * (car_w * 0.20),
                ],
                fwd,
                0.06,
                0.038,
                car_w * 0.34,
                left_col,
                1.5,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.06) + right[0] * (car_w * 0.20),
                    vy + car_h * 1.04,
                    vz - fwd[1] * (car_len * 0.06) + right[1] * (car_w * 0.20),
                ],
                fwd,
                0.06,
                0.038,
                car_w * 0.34,
                right_col,
                1.5,
            );
        }
        GothicVehicleKind::TransitOmnibus => {
            // Double-length upper cabin + glowing passenger window strip + roof clerestory + front marquee
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 0.56, vz],
                fwd,
                car_len * 0.90,
                car_h * 0.46,
                car_w * 0.92,
                theme.stone_primary,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 0.68, vz],
                fwd,
                car_len * 0.78,
                0.055,
                car_w * 0.95,
                theme.window_warm,
                0.78,
            );
            opaque_mb.push_oriented_box(
                [vx, vy + car_h * 1.02, vz],
                fwd,
                car_len * 0.68,
                0.035,
                car_w * 0.52,
                theme.accent_trim,
                0.0,
            );
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.46),
                    vy + car_h * 0.92,
                    vz + fwd[1] * (car_len * 0.46),
                ],
                fwd,
                0.03,
                0.04,
                car_w * 0.58,
                theme.window_warm,
                1.1,
            );
        }
        GothicVehicleKind::VigilanteInterceptor => {
            // Turbine nose + low armored canopy + tall swept bat-fins + glowing afterburner
            opaque_mb.push_oriented_box(
                [
                    vx + fwd[0] * (car_len * 0.48),
                    vy + 0.03,
                    vz + fwd[1] * (car_len * 0.48),
                ],
                fwd,
                0.08,
                0.05,
                car_w * 0.34,
                theme.accent_trim,
                0.4,
            );
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.05),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.05),
                ],
                fwd,
                car_len * 0.40,
                car_h * 0.42,
                car_w * 0.68,
                theme.stone_dark,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.34) + right[0] * (car_w * 0.44 * side),
                        vy + car_h * 0.72,
                        vz - fwd[1] * (car_len * 0.34) + right[1] * (car_w * 0.44 * side),
                    ],
                    fwd,
                    0.26,
                    0.11,
                    0.03,
                    theme.plinth_dark,
                    0.0,
                );
            }
            if veh.afterburner > 0.05 {
                let flame_len = 0.14 + veh.afterburner * 0.34;
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.54 + flame_len * 0.4),
                        vy + 0.04,
                        vz - fwd[1] * (car_len * 0.54 + flame_len * 0.4),
                    ],
                    fwd,
                    flame_len,
                    0.055,
                    0.065,
                    theme.searchlight,
                    1.8,
                );
                trans_mb.push_light_cone(
                    [
                        vx - fwd[0] * (car_len * 0.52),
                        vy + 0.04,
                        vz - fwd[1] * (car_len * 0.52),
                    ],
                    [-fwd[0], 0.02, -fwd[1]],
                    0.45 + veh.afterburner * 0.55,
                    0.14,
                    theme.searchlight,
                );
            }
        }
        GothicVehicleKind::SteamLimousine => {
            // Extended formal cabin + brass carriage-lantern pips on B-pillars
            opaque_mb.push_oriented_box(
                [
                    vx - fwd[0] * (car_len * 0.08),
                    vy + car_h * 0.58,
                    vz - fwd[1] * (car_len * 0.08),
                ],
                fwd,
                car_len * 0.58,
                car_h * 0.46,
                car_w * 0.84,
                theme.stone_dark,
                0.0,
            );
            for &side in &[-1.0f32, 1.0f32] {
                opaque_mb.push_oriented_box(
                    [
                        vx - fwd[0] * (car_len * 0.10) + right[0] * (car_w * 0.46 * side),
                        vy + car_h * 0.64,
                        vz - fwd[1] * (car_len * 0.10) + right[1] * (car_w * 0.46 * side),
                    ],
                    fwd,
                    0.04,
                    0.045,
                    0.04,
                    theme.window_warm,
                    1.2,
                );
            }
        }
    }

    // Twin separated headlights
    for &side in &[-1.0f32, 1.0f32] {
        opaque_mb.push_oriented_box(
            [
                vx + fwd[0] * (car_len * 0.50) + right[0] * (car_w * 0.30 * side),
                vy + 0.032,
                vz + fwd[1] * (car_len * 0.50) + right[1] * (car_w * 0.30 * side),
            ],
            fwd,
            0.035,
            0.04,
            0.045,
            theme.window_warm,
            1.25,
        );
    }

    // Twin ruby taillights that flare brightly when braking
    let brake_emissive = if veh.is_braking { 1.75 } else { 0.85 };
    for &side in &[-1.0f32, 1.0f32] {
        opaque_mb.push_oriented_box(
            [
                vx - fwd[0] * (car_len * 0.50) + right[0] * (car_w * 0.30 * side),
                vy + 0.032,
                vz - fwd[1] * (car_len * 0.50) + right[1] * (car_w * 0.30 * side),
            ],
            fwd,
            0.035,
            0.04,
            0.05,
            theme.beacon_red,
            brake_emissive,
        );
    }

    // Blinking amber turn-signal pip before corners
    if veh.turn_signal != 0 && (time * 7.0).sin() > 0.0 {
        let side = if veh.turn_signal > 0 { 1.0f32 } else { -1.0f32 };
        opaque_mb.push_oriented_box(
            [
                vx + fwd[0] * (car_len * 0.44) + right[0] * (car_w * 0.44 * side),
                vy + 0.035,
                vz + fwd[1] * (car_len * 0.44) + right[1] * (car_w * 0.44 * side),
            ],
            fwd,
            0.04,
            0.035,
            0.035,
            theme.accent_trim,
            1.5,
        );
    }

    // Translucent headlight cone on the street asphalt ahead
    let cone_len = if veh.kind == GothicVehicleKind::VigilanteInterceptor {
        1.45
    } else {
        1.05
    };
    trans_mb.push_light_cone(
        [
            vx + fwd[0] * (car_len * 0.52),
            vy + 0.04,
            vz + fwd[1] * (car_len * 0.52),
        ],
        [fwd[0], -0.04, fwd[1]],
        cone_len,
        0.26,
        theme.searchlight,
    );
}

fn build_isometric_view_proj(
    width: f32,
    height: f32,
    yaw: f32,
    pitch: f32,
    occupancy: f32,
    target: [f32; 3],
    zoom: f32,
) -> [[f32; 4]; 4] {
    let aspect = (width.max(1.0) / height.max(1.0)).clamp(0.4, 3.5);
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();

    // Expanded 4x4 city horizontal radius is ~12.2 world units; vertical span is ~17.5 world units.
    // Sizing `scale_y` so that the projected 4x4 city fills the vertical viewport with a tiny ~0.3cm buffer:
    let occ = (occupancy * zoom.clamp(0.8, 2.8)).clamp(0.40, 2.50);
    // Fit smoothly across both 16:9 widescreen and narrower aspect ratios
    let base_scale = occ / 13.0;
    let scale_y = base_scale * (aspect.min(1.65)).sqrt();
    let scale_x = scale_y / aspect;
    let scale_z = 1.0 / 64.0;

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
            guard.camera_target,
            guard.camera_zoom,
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

        // --- LAYER 1: Connected Street & Highway Vehicles + Functional Traffic Signals ---
        for sig in &guard.traffic_signals {
            let hx = sig.pos[0] + 0.40;
            let hz = sig.pos[1] + 0.54;
            let ns_color = if guard.green_wave_timer > 0.0 || (sig.ns_green && !sig.is_amber) {
                theme.copper_roof
            } else if sig.ns_green && sig.is_amber {
                theme.window_warm
            } else {
                theme.beacon_red
            };
            let ew_color = if guard.green_wave_timer > 0.0 || (!sig.ns_green && !sig.is_amber) {
                theme.copper_roof
            } else if !sig.ns_green && sig.is_amber {
                theme.window_warm
            } else {
                theme.beacon_red
            };
            opaque_mb.push_box(hx, 0.59, hz + 0.05, 0.055, 0.06, 0.025, ns_color, 1.35);
            opaque_mb.push_box(hx - 0.05, 0.59, hz, 0.025, 0.06, 0.055, ew_color, 1.35);
        }

        for veh in &guard.vehicles {
            if let Some((pos, car_fwd, slope, is_subterranean)) =
                guard.blueprint.sample_vehicle_pose_3d(veh)
            {
                if is_subterranean {
                    continue;
                }
                let opaque_start = opaque_mb.verts.len();
                let trans_start = trans_mb.verts.len();
                push_vehicle_mesh(
                    &mut opaque_mb,
                    &mut trans_mb,
                    veh,
                    pos,
                    car_fwd,
                    guard.time,
                    &theme,
                );
                if slope.abs() > 1e-4 {
                    let pitch = slope.atan();
                    let (sin_p, cos_p) = pitch.sin_cos();
                    for verts in [
                        &mut opaque_mb.verts[opaque_start..],
                        &mut trans_mb.verts[trans_start..],
                    ] {
                        for v in verts {
                            let dx = v.pos[0] - pos[0];
                            let dy = v.pos[1] - pos[1];
                            let dz = v.pos[2] - pos[2];
                            let along = dx * car_fwd[0] + dz * car_fwd[1];
                            let new_along = along * cos_p - dy * sin_p;
                            let new_dy = along * sin_p + dy * cos_p;
                            let d_along = new_along - along;
                            v.pos[0] += car_fwd[0] * d_along;
                            v.pos[2] += car_fwd[1] * d_along;
                            v.pos[1] = pos[1] + new_dy;

                            let n_along = v.normal[0] * car_fwd[0] + v.normal[2] * car_fwd[1];
                            let n_y = v.normal[1];
                            let new_n_along = n_along * cos_p - n_y * sin_p;
                            let new_n_y = n_along * sin_p + n_y * cos_p;
                            let dn = new_n_along - n_along;
                            v.normal[0] += car_fwd[0] * dn;
                            v.normal[2] += car_fwd[1] * dn;
                            v.normal[1] = new_n_y;
                        }
                    }
                }
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

        // Release simulation lock before submitting GPU buffer writes
        drop(guard);

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
        let mut drag_delta = None;
        if input.action == PointerAction::LeftClick {
            self.is_dragging = true;
            self.last_pointer = input.pointer;
        }
        if input.left_held {
            if let (Some(curr), Some(prev)) = (input.pointer, self.last_pointer) {
                let dx = (curr.x - prev.x) / (self.width.max(1) as f32);
                let dy = (curr.y - prev.y) / (self.height.max(1) as f32);
                if dx.abs() > 0.0001 || dy.abs() > 0.0001 {
                    drag_delta = Some((dx * PI * 1.4, dy * 1.1));
                }
            }
            self.last_pointer = input.pointer;
        } else {
            self.is_dragging = false;
            self.last_pointer = input.pointer;
        }

        let activity = if let Ok(mut guard) = self.state.lock() {
            if input.action == PointerAction::LeftClick {
                guard.trigger_steam_burst();
            } else if input.action == PointerAction::RightClick {
                guard.rotate_camera_quarter_turn();
            }
            if let Some((yaw_d, pitch_d)) = drag_delta {
                guard.orbit_camera(yaw_d, pitch_d);
            }
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
