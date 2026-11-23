//! Low-poly indexed 3D mesh builder and primitive geometry helpers.

use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct GothicVertex {
    pub(super) pos: [f32; 3],
    pub(super) normal: [f32; 3],
    pub(super) color: [f32; 4],
    pub(super) emissive: f32,
    pub(super) color_b: [f32; 4],
}

/// Low-poly indexed 3D mesh builder for simplified gothic isometric geometry.
pub(super) struct MeshBuilder {
    pub(super) verts: Vec<GothicVertex>,
    pub(super) indices: Vec<u32>,
}

impl MeshBuilder {
    pub(super) fn with_capacity(vert_cap: usize, idx_cap: usize) -> Self {
        Self {
            verts: Vec::with_capacity(vert_cap),
            indices: Vec::with_capacity(idx_cap),
        }
    }

    pub(super) fn from_scratch(mut verts: Vec<GothicVertex>, mut indices: Vec<u32>) -> Self {
        verts.clear();
        indices.clear();
        Self { verts, indices }
    }

    pub(super) fn into_parts(self) -> (Vec<GothicVertex>, Vec<u32>) {
        (self.verts, self.indices)
    }

    #[inline]
    pub(super) fn push_vertex(&mut self, v: GothicVertex) -> u32 {
        let idx = self.verts.len() as u32;
        self.verts.push(v);
        idx
    }

    pub(super) fn push_tri(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        emissive: f32,
    ) {
        let base = self.verts.len() as u32;
        self.verts.push(GothicVertex {
            pos: a,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.verts.push(GothicVertex {
            pos: b,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.verts.push(GothicVertex {
            pos: c,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_quad(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        d: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        emissive: f32,
    ) {
        let base = self.verts.len() as u32;
        self.verts.push(GothicVertex {
            pos: a,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.verts.push(GothicVertex {
            pos: b,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.verts.push(GothicVertex {
            pos: c,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.verts.push(GothicVertex {
            pos: d,
            normal,
            color,
            emissive,
            color_b: color,
        });
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Axis-aligned 3D box centered at `(cx, _, cz)` with base at `y0` and top at `y1`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_box(
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
    pub(super) fn push_oriented_box(
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
    pub(super) fn push_sloped_box(
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
    pub(super) fn push_pyramid(
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
    pub(super) fn push_frustum(
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
    pub(super) fn push_light_cone(
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

        let apex_idx = self.push_vertex(GothicVertex {
            pos: apex,
            normal: [0.0, 1.0, 0.0],
            color: tip_color,
            emissive: 2.0,
            color_b: tip_color,
        });

        let mid_base = self.verts.len() as u32;
        for i in 0..SIDES {
            let a = (i as f32 / SIDES as f32) * TAU;
            let (sa, ca) = a.sin_cos();
            let pos = [
                mid_center[0] + (u[0] * ca + v[0] * sa) * mid_radius,
                mid_center[1] + (u[1] * ca + v[1] * sa) * mid_radius,
                mid_center[2] + (u[2] * ca + v[2] * sa) * mid_radius,
            ];
            self.verts.push(GothicVertex {
                pos,
                normal: [0.0, 1.0, 0.0],
                color: mid_color,
                emissive: 2.0,
                color_b: mid_color,
            });
        }

        let outer_base = self.verts.len() as u32;
        for i in 0..SIDES {
            let a = (i as f32 / SIDES as f32) * TAU;
            let (sa, ca) = a.sin_cos();
            let pos = [
                base_center[0] + (u[0] * ca + v[0] * sa) * end_radius,
                base_center[1] + (u[1] * ca + v[1] * sa) * end_radius,
                base_center[2] + (u[2] * ca + v[2] * sa) * end_radius,
            ];
            self.verts.push(GothicVertex {
                pos,
                normal: [0.0, 1.0, 0.0],
                color: fade_color,
                emissive: 2.0,
                color_b: fade_color,
            });
        }

        for i in 0..SIDES {
            let next = ((i + 1) % SIDES) as u32;
            let curr = i as u32;
            let m0 = mid_base + curr;
            let m1 = mid_base + next;
            let p0 = outer_base + curr;
            let p1 = outer_base + next;

            // Inner core cone slice (apex -> mid ring)
            self.indices.extend_from_slice(&[apex_idx, m0, m1]);
            // Outer feather skirt (mid ring -> zero-alpha outer rim)
            self.indices.extend_from_slice(&[m0, p0, p1, m0, p1, m1]);
        }
    }
}
