//! Low-poly indexed 3D mesh builder with procedural atlas sub-tile UV support.

use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};

use super::textures::Tile;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OrbitalVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// `[u_min, v_min, u_size, v_size]` within the procedural 8x8 pixel texture atlas.
    pub tile_rect: [f32; 4],
    /// Per-vertex tint and baked ambient occlusion (`rgb` tint, `a` opacity).
    pub color: [f32; 4],
    /// `0.0` = lit surface, `>= 0.5` = emissive CRT/beacon/plasma, `>= 1.5` = additive/translucent FX.
    pub emissive: f32,
}

impl OrbitalVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 32,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 48,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 64,
                shader_location: 5,
                format: wgpu::VertexFormat::Float32,
            },
        ],
    };
}

pub struct MeshBuilder {
    pub verts: Vec<OrbitalVertex>,
    pub indices: Vec<u32>,
}

impl MeshBuilder {
    pub fn with_capacity(vcap: usize, icap: usize) -> Self {
        Self {
            verts: Vec::with_capacity(vcap),
            indices: Vec::with_capacity(icap),
        }
    }

    pub fn clear(&mut self) {
        self.verts.clear();
        self.indices.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_quad(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        d: [f32; 3],
        normal: [f32; 3],
        uv_scale: [f32; 2],
        tile: Tile,
        color: [f32; 4],
        emissive: f32,
    ) {
        let base = self.verts.len() as u32;
        let rect = tile.rect();
        let uvs = [
            [0.0, 0.0],
            [uv_scale[0], 0.0],
            [uv_scale[0], uv_scale[1]],
            [0.0, uv_scale[1]],
        ];
        for (pos, uv) in [a, b, c, d].into_iter().zip(uvs) {
            self.verts.push(OrbitalVertex {
                pos,
                normal,
                uv,
                tile_rect: rect,
                color,
                emissive,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_tri(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        normal: [f32; 3],
        tile: Tile,
        color: [f32; 4],
        emissive: f32,
    ) {
        let base = self.verts.len() as u32;
        let rect = tile.rect();
        for (pos, uv) in [(a, [0.0, 0.0]), (b, [1.0, 0.0]), (c, [0.5, 1.0])] {
            self.verts.push(OrbitalVertex {
                pos,
                normal,
                uv,
                tile_rect: rect,
                color,
                emissive,
            });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Push an axis-aligned box centered at `center` with full dimensions `size = [sx, sy, sz]`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_box(
        &mut self,
        center: [f32; 3],
        size: [f32; 3],
        tile: Tile,
        tile_world_unit: f32,
        color: [f32; 4],
        emissive: f32,
    ) {
        let hx = size[0] * 0.5;
        let hy = size[1] * 0.5;
        let hz = size[2] * 0.5;
        let [cx, cy, cz] = center;
        let inv = 1.0 / tile_world_unit.max(0.25);

        let ux = (size[0] * inv).max(0.25);
        let uy = (size[1] * inv).max(0.25);
        let uz = (size[2] * inv).max(0.25);

        // +Z front
        self.push_quad(
            [cx - hx, cy - hy, cz + hz],
            [cx + hx, cy - hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
            [0.0, 0.0, 1.0],
            [ux, uy],
            tile,
            color,
            emissive,
        );
        // -Z back
        self.push_quad(
            [cx + hx, cy - hy, cz - hz],
            [cx - hx, cy - hy, cz - hz],
            [cx - hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
            [0.0, 0.0, -1.0],
            [ux, uy],
            tile,
            color,
            emissive,
        );
        // +X right
        self.push_quad(
            [cx + hx, cy - hy, cz + hz],
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz + hz],
            [1.0, 0.0, 0.0],
            [uz, uy],
            tile,
            color,
            emissive,
        );
        // -X left
        self.push_quad(
            [cx - hx, cy - hy, cz - hz],
            [cx - hx, cy - hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz - hz],
            [-1.0, 0.0, 0.0],
            [uz, uy],
            tile,
            color,
            emissive,
        );
        // +Y top
        self.push_quad(
            [cx - hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz - hz],
            [cx - hx, cy + hy, cz - hz],
            [0.0, 1.0, 0.0],
            [ux, uz],
            tile,
            color,
            emissive,
        );
        // -Y bottom
        self.push_quad(
            [cx - hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz + hz],
            [cx - hx, cy - hy, cz + hz],
            [0.0, -1.0, 0.0],
            [ux, uz],
            tile,
            color,
            emissive,
        );
    }

    /// Push a vertical cylinder (faceted Y2K low-poly prism) centered at `center`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_cylinder(
        &mut self,
        center: [f32; 3],
        radius_bottom: f32,
        radius_top: f32,
        height: f32,
        segments: u32,
        tile: Tile,
        color: [f32; 4],
        emissive: f32,
    ) {
        let [cx, cy, cz] = center;
        let y0 = cy - height * 0.5;
        let y1 = cy + height * 0.5;
        let segs = segments.max(5);

        for i in 0..segs {
            let a0 = (i as f32 / segs as f32) * TAU;
            let a1 = ((i + 1) as f32 / segs as f32) * TAU;
            let (c0, s0) = (a0.cos(), a0.sin());
            let (c1, s1) = (a1.cos(), a1.sin());

            let p00 = [cx + c0 * radius_bottom, y0, cz + s0 * radius_bottom];
            let p10 = [cx + c1 * radius_bottom, y0, cz + s1 * radius_bottom];
            let p11 = [cx + c1 * radius_top, y1, cz + s1 * radius_top];
            let p01 = [cx + c0 * radius_top, y1, cz + s0 * radius_top];

            let am = (a0 + a1) * 0.5;
            let n = [am.cos(), 0.0, am.sin()];
            // Outward CCW winding when viewed from outside the cylinder
            self.push_quad(
                p10,
                p00,
                p01,
                p11,
                n,
                [1.0, (height / 4.0).max(0.5)],
                tile,
                color,
                emissive,
            );

            if radius_top > 0.01 {
                self.push_tri(
                    [cx, y1, cz],
                    p11,
                    p01,
                    [0.0, 1.0, 0.0],
                    tile,
                    color,
                    emissive,
                );
            }
            if radius_bottom > 0.01 {
                self.push_tri(
                    [cx, y0, cz],
                    p00,
                    p10,
                    [0.0, -1.0, 0.0],
                    tile,
                    color,
                    emissive,
                );
            }
        }
    }

    /// Push a cylinder oriented along the Z axis (for rotating ring bearing hubs & collars).
    #[allow(clippy::too_many_arguments)]
    pub fn push_cylinder_z(
        &mut self,
        center: [f32; 3],
        radius: f32,
        depth: f32,
        segments: u32,
        angle_offset: f32,
        tile: Tile,
        color: [f32; 4],
        emissive: f32,
    ) {
        let [cx, cy, cz] = center;
        let z0 = cz - depth * 0.5;
        let z1 = cz + depth * 0.5;
        let segs = segments.max(6);

        for i in 0..segs {
            let a0 = angle_offset + (i as f32 / segs as f32) * TAU;
            let a1 = angle_offset + ((i + 1) as f32 / segs as f32) * TAU;
            let (c0, s0) = (a0.cos(), a0.sin());
            let (c1, s1) = (a1.cos(), a1.sin());

            let p00 = [cx + c0 * radius, cy + s0 * radius, z0];
            let p10 = [cx + c1 * radius, cy + s1 * radius, z0];
            let p11 = [cx + c1 * radius, cy + s1 * radius, z1];
            let p01 = [cx + c0 * radius, cy + s0 * radius, z1];

            let am = (a0 + a1) * 0.5;
            let n = [am.cos(), am.sin(), 0.0];
            // Outward CCW winding when viewed from outside the Z-axis cylinder
            self.push_quad(
                p00,
                p10,
                p11,
                p01,
                n,
                [1.0, (depth / 4.0).max(0.5)],
                tile,
                color,
                emissive,
            );
            // +Z front cap (CCW looking from +Z: center -> p01 -> p11)
            self.push_tri(
                [cx, cy, z1],
                p01,
                p11,
                [0.0, 0.0, 1.0],
                tile,
                color,
                emissive,
            );
            // -Z back cap (CCW looking from -Z: center -> p10 -> p00)
            self.push_tri(
                [cx, cy, z0],
                p10,
                p00,
                [0.0, 0.0, -1.0],
                tile,
                color,
                emissive,
            );
        }
    }

    /// Push a low-poly oriented box aligned to a spacecraft/entity basis `(right, up, forward)`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_oriented_box(
        &mut self,
        origin: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        forward: [f32; 3],
        local_center: [f32; 3],
        size: [f32; 3],
        tile: Tile,
        color: [f32; 4],
        emissive: f32,
    ) {
        // Ensure local basis is strictly right-handed so triangle winding is always outward CCW
        let cx_u = [
            right[1] * up[2] - right[2] * up[1],
            right[2] * up[0] - right[0] * up[2],
            right[0] * up[1] - right[1] * up[0],
        ];
        let det = cx_u[0] * forward[0] + cx_u[1] * forward[1] + cx_u[2] * forward[2];
        let right = if det < 0.0 {
            [-right[0], -right[1], -right[2]]
        } else {
            right
        };

        let xform = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
            let x = local_center[0] + lx;
            let y = local_center[1] + ly;
            let z = local_center[2] + lz;
            [
                origin[0] + right[0] * x + up[0] * y + forward[0] * z,
                origin[1] + right[1] * x + up[1] * y + forward[1] * z,
                origin[2] + right[2] * x + up[2] * y + forward[2] * z,
            ]
        };
        let neg = |v: [f32; 3]| [-v[0], -v[1], -v[2]];
        let hx = size[0] * 0.5;
        let hy = size[1] * 0.5;
        let hz = size[2] * 0.5;

        // +Z forward
        self.push_quad(
            xform(-hx, -hy, hz),
            xform(hx, -hy, hz),
            xform(hx, hy, hz),
            xform(-hx, hy, hz),
            forward,
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
        // -Z aft
        self.push_quad(
            xform(hx, -hy, -hz),
            xform(-hx, -hy, -hz),
            xform(-hx, hy, -hz),
            xform(hx, hy, -hz),
            neg(forward),
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
        // +X right
        self.push_quad(
            xform(hx, -hy, hz),
            xform(hx, -hy, -hz),
            xform(hx, hy, -hz),
            xform(hx, hy, hz),
            right,
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
        // -X left
        self.push_quad(
            xform(-hx, -hy, -hz),
            xform(-hx, -hy, hz),
            xform(-hx, hy, hz),
            xform(-hx, hy, -hz),
            neg(right),
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
        // +Y top
        self.push_quad(
            xform(-hx, hy, hz),
            xform(hx, hy, hz),
            xform(hx, hy, -hz),
            xform(-hx, hy, -hz),
            up,
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
        // -Y bottom
        self.push_quad(
            xform(-hx, -hy, -hz),
            xform(hx, -hy, -hz),
            xform(hx, -hy, hz),
            xform(-hx, -hy, hz),
            neg(up),
            [1.0, 1.0],
            tile,
            color,
            emissive,
        );
    }

    /// Push a faceted low-poly sphere (for the stylized Y2K planet backdrop).
    #[allow(clippy::too_many_arguments)]
    pub fn push_sphere(
        &mut self,
        center: [f32; 3],
        radius: f32,
        lat_segs: u32,
        lon_segs: u32,
        tile: Tile,
        color_north: [f32; 4],
        color_equator: [f32; 4],
        emissive: f32,
    ) {
        let [cx, cy, cz] = center;
        for iy in 0..lat_segs {
            let v0 = iy as f32 / lat_segs as f32;
            let v1 = (iy + 1) as f32 / lat_segs as f32;
            let phi0 = (v0 - 0.5) * std::f32::consts::PI;
            let phi1 = (v1 - 0.5) * std::f32::consts::PI;
            let band_t = ((v0 + v1) * 0.5 - 0.5).abs() * 2.0;
            let col = [
                color_equator[0] * (1.0 - band_t) + color_north[0] * band_t,
                color_equator[1] * (1.0 - band_t) + color_north[1] * band_t,
                color_equator[2] * (1.0 - band_t) + color_north[2] * band_t,
                1.0,
            ];

            for ix in 0..lon_segs {
                let u0 = ix as f32 / lon_segs as f32;
                let u1 = (ix + 1) as f32 / lon_segs as f32;
                let th0 = u0 * TAU;
                let th1 = u1 * TAU;

                let pt = |phi: f32, th: f32| -> [f32; 3] {
                    let cp = phi.cos();
                    [
                        cx + radius * cp * th.cos(),
                        cy + radius * phi.sin(),
                        cz + radius * cp * th.sin(),
                    ]
                };
                let p00 = pt(phi0, th0);
                let p10 = pt(phi0, th1);
                let p11 = pt(phi1, th1);
                let p01 = pt(phi1, th0);

                let pm = ((phi0 + phi1) * 0.5).cos();
                let tm = (th0 + th1) * 0.5;
                let n = [pm * tm.cos(), ((phi0 + phi1) * 0.5).sin(), pm * tm.sin()];

                // Map global spherical UVs smoothly across the whole planet (avoiding per-quad tiling!)
                let base = self.verts.len() as u32;
                let rect = tile.rect();
                let verts_uv = [
                    (p10, [u1 * 2.0, v0]),
                    (p00, [u0 * 2.0, v0]),
                    (p01, [u0 * 2.0, v1]),
                    (p11, [u1 * 2.0, v1]),
                ];
                for (pos, uv) in verts_uv {
                    self.verts.push(OrbitalVertex {
                        pos,
                        normal: n,
                        uv,
                        tile_rect: rect,
                        color: col,
                        emissive,
                    });
                }
                self.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
            }
        }
    }
}
