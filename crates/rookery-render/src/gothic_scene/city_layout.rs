//! Static city layout geometry: plinth, sidewalks, avenues, elevated highways, portals, walkways, and viaduct.

use rookery_core::gothic::{
    CityBlockPlot, CityBlueprint, ElevatedHighwaySpan, GaragePortal, StreetAvenue,
    UndergroundPortal, WalkwaySpan, WalkwayStyle,
};

use super::buildings::build_archetype_mesh;
use super::landmarks::build_bespoke_landmarks_mesh;
use super::mesh_builder::{GothicVertex, MeshBuilder};
use super::theme::GothicThemeColors;

/// Build the complete static 3D city mesh from [`CityBlueprint`] and [`GothicThemeColors`].
pub(super) fn build_static_city_mesh(
    bp: &CityBlueprint,
    theme: &GothicThemeColors,
) -> (Vec<GothicVertex>, Vec<u32>) {
    let mut mb = MeshBuilder::with_capacity(44_000, 66_000);

    // 1. Stepped Citadel Foundation Plinth floating in the black void
    let ext = bp.plinth_half_extent;
    let plinth_cutouts = collect_underground_plinth_cutouts(bp);
    // Lower dark obsidian/slate deck slab (y = -0.26 .. 0.0), carved open at sunken tunnel ramps
    for r in subtract_cutouts([-ext, -ext, ext, ext], &plinth_cutouts) {
        mb.push_box(
            (r[0] + r[2]) * 0.5,
            -0.26,
            (r[1] + r[3]) * 0.5,
            r[2] - r[0],
            0.26,
            r[3] - r[1],
            theme.plinth_dark,
            0.0,
        );
    }
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
        build_garage_portal_mesh(&mut mb, bp, garage, theme);
    }

    // 5b. 2nd-Level Outer Elevated Highway Overpasses & Straddling Arch Piers
    for span in &bp.elevated_highways {
        build_elevated_highway_mesh(&mut mb, bp, span, theme);
    }

    // 5c. Sunken Roadside Underground Tunnel Portals
    for portal in &bp.underground_portals {
        build_underground_portal_mesh(&mut mb, bp, portal, theme);
    }

    // 6. Multi-Level Elevated Walkways / Skybridges
    for span in &bp.walkways {
        build_walkway_mesh(&mut mb, span, theme);
    }

    // 7. Elevated Train Viaduct Loop & Support Arches
    build_viaduct_mesh(&mut mb, bp, theme);

    // 8. Eight Bespoke One-of-a-Kind City Landmarks
    build_bespoke_landmarks_mesh(&mut mb, theme);

    mb.into_parts()
}

/// Determine the half-width of the road segment containing `pos` from `bp.avenues`.
fn road_half_width_at(bp: &CityBlueprint, pos: [f32; 2]) -> f32 {
    bp.avenues
        .iter()
        .filter(|a| point_to_avenue_dist(pos[0], pos[1], a) < 0.25)
        .map(|a| a.width * 0.5)
        .fold(0.0f32, f32::max)
        .max(0.55)
}

/// Compute an axis-aligned 2D bounding rectangle `[x0, z0, x1, z1]` spanning from `start`
/// to `end` along `fwd` with total lateral width `width`.
fn oriented_span_rect(start: [f32; 2], end: [f32; 2], fwd: [f32; 2], width: f32) -> [f32; 4] {
    let right = [-fwd[1], fwd[0]];
    let hw = width * 0.5;
    let corners = [
        [start[0] - right[0] * hw, start[1] - right[1] * hw],
        [start[0] + right[0] * hw, start[1] + right[1] * hw],
        [end[0] - right[0] * hw, end[1] - right[1] * hw],
        [end[0] + right[0] * hw, end[1] + right[1] * hw],
    ];
    let mut x0 = f32::INFINITY;
    let mut z0 = f32::INFINITY;
    let mut x1 = f32::NEG_INFINITY;
    let mut z1 = f32::NEG_INFINITY;
    for [x, z] in corners {
        x0 = x0.min(x);
        z0 = z0.min(z);
        x1 = x1.max(x);
        z1 = z1.max(z);
    }
    [x0, z0, x1, z1]
}

fn rect_overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[2] > b[0] && a[0] < b[2] && a[3] > b[1] && a[1] < b[3]
}

fn subtract_rect_from_rects(rects: Vec<[f32; 4]>, cut: [f32; 4]) -> Vec<[f32; 4]> {
    let mut out = Vec::with_capacity(rects.len());
    for r in rects {
        if !rect_overlaps(r, cut) {
            out.push(r);
            continue;
        }
        let ix0 = r[0].max(cut[0]);
        let ix1 = r[2].min(cut[2]);
        if cut[0] > r[0] {
            out.push([r[0], r[1], cut[0], r[3]]);
        }
        if r[2] > cut[2] {
            out.push([cut[2], r[1], r[2], r[3]]);
        }
        if ix1 > ix0 {
            if cut[1] > r[1] {
                out.push([ix0, r[1], ix1, cut[1]]);
            }
            if r[3] > cut[3] {
                out.push([ix0, cut[3], ix1, r[3]]);
            }
        }
    }
    out
}

fn subtract_cutouts(initial: [f32; 4], cutouts: &[[f32; 4]]) -> Vec<[f32; 4]> {
    let mut rects = vec![initial];
    for &cut in cutouts {
        rects = subtract_rect_from_rects(rects, cut);
    }
    rects
}

/// Plinth cutouts for sunken underground tunnel ramps so the driving channel opens below `y = 0`.
fn collect_underground_plinth_cutouts(bp: &CityBlueprint) -> Vec<[f32; 4]> {
    bp.underground_portals
        .iter()
        .map(|p| {
            let fwd = p.inward_dir;
            let road_half_w = road_half_width_at(bp, p.entry_pos);
            let road_edge = [
                p.entry_pos[0] + fwd[0] * road_half_w,
                p.entry_pos[1] + fwd[1] * road_half_w,
            ];
            let pit_end = [
                p.portal_pos[0] + fwd[0] * 0.25,
                p.portal_pos[1] + fwd[1] * 0.25,
            ];
            oriented_span_rect(road_edge, pit_end, fwd, p.width + 0.04)
        })
        .collect()
}

/// Sidewalk/curb/paver cutouts for both sunken underground tunnel portals and garage driveways.
fn collect_sidewalk_cutouts(bp: &CityBlueprint) -> Vec<[f32; 4]> {
    let mut cutouts = Vec::with_capacity(bp.underground_portals.len() + bp.garages.len());
    for p in &bp.underground_portals {
        let fwd = p.inward_dir;
        let road_half_w = road_half_width_at(bp, p.entry_pos);
        let road_edge = [
            p.entry_pos[0] + fwd[0] * road_half_w,
            p.entry_pos[1] + fwd[1] * road_half_w,
        ];
        let pit_end = [
            p.portal_pos[0] + fwd[0] * 0.45,
            p.portal_pos[1] + fwd[1] * 0.45,
        ];
        cutouts.push(oriented_span_rect(road_edge, pit_end, fwd, p.width + 0.24));
    }
    for g in &bp.garages {
        let fwd = g.inward_dir;
        let road_half_w = road_half_width_at(bp, g.street_entry);
        let road_edge = [
            g.street_entry[0] + fwd[0] * road_half_w,
            g.street_entry[1] + fwd[1] * road_half_w,
        ];
        let end = [
            g.portal_pos[0] + fwd[0] * 0.12,
            g.portal_pos[1] + fwd[1] * 0.12,
        ];
        cutouts.push(oriented_span_rect(road_edge, end, fwd, g.width + 0.18));
    }
    cutouts
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
    let sw_cutouts = collect_sidewalk_cutouts(bp);

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

    let push_cutout_box =
        |mb: &mut MeshBuilder, rect: [f32; 4], y0: f32, h: f32, color: [f32; 4]| {
            for r in subtract_cutouts(rect, &sw_cutouts) {
                mb.push_box(
                    (r[0] + r[2]) * 0.5,
                    y0,
                    (r[1] + r[3]) * 0.5,
                    r[2] - r[0],
                    h,
                    r[3] - r[1],
                    color,
                    0.0,
                );
            }
        };

    for block in blocks {
        let (x0, z0, x1, z1) = (block.x0, block.z0, block.x1, block.z1);
        let bw = x1 - x0;
        let bd = z1 - z0;

        // Main raised block sidewalk & plaza platform
        push_cutout_box(mb, [x0, z0, x1, z1], 0.0, sw_h, theme.sidewalk);

        // Inner slightly recessed dark stone building plinth pad so the surrounding
        // 0.42m sidewalk walkway reads as a crisp perimeter pedestrian belt!
        if bw > walk_band * 2.2 && bd > walk_band * 2.2 {
            push_cutout_box(
                mb,
                [
                    x0 + walk_band,
                    z0 + walk_band,
                    x1 - walk_band,
                    z1 - walk_band,
                ],
                sw_h,
                0.015,
                theme.stone_dark,
            );
        }

        // Raised stone curbs along the 4 street-facing edges of the city block
        push_cutout_box(
            mb,
            [x0, z0, x1, z0 + curb_w],
            0.0,
            curb_h,
            theme.sidewalk_curb,
        );
        push_cutout_box(
            mb,
            [x0, z1 - curb_w, x1, z1],
            0.0,
            curb_h,
            theme.sidewalk_curb,
        );
        push_cutout_box(
            mb,
            [x0, z0, x0 + curb_w, z1],
            0.0,
            curb_h,
            theme.sidewalk_curb,
        );
        push_cutout_box(
            mb,
            [x1 - curb_w, z0, x1, z1],
            0.0,
            curb_h,
            theme.sidewalk_curb,
        );

        // Textured flagstone paving slabs along North & South block sidewalks
        let x_pavers = ((bw - 0.4) / 0.54).floor().max(2.0) as usize;
        let x_step = (bw - 0.36) / x_pavers as f32;
        for i in 0..x_pavers {
            let px = x0 + 0.18 + (i as f32 + 0.5) * x_step;
            let paver_w = (x_step * 0.76).min(0.42);
            let paver_d = walk_band * 0.68;
            for pz in [z0 + walk_band * 0.52, z1 - walk_band * 0.52] {
                let r = [
                    px - paver_w * 0.5,
                    pz - paver_d * 0.5,
                    px + paver_w * 0.5,
                    pz + paver_d * 0.5,
                ];
                if !sw_cutouts.iter().any(|&c| rect_overlaps(r, c)) {
                    mb.push_box(
                        px,
                        sw_h,
                        pz,
                        paver_w,
                        0.012,
                        paver_d,
                        theme.sidewalk_paver,
                        0.0,
                    );
                }
            }
        }

        // Textured flagstone paving slabs along West & East block sidewalks
        let z_pavers = ((bd - 0.4) / 0.54).floor().max(2.0) as usize;
        let z_step = (bd - 0.36) / z_pavers as f32;
        for i in 0..z_pavers {
            let pz = z0 + 0.18 + (i as f32 + 0.5) * z_step;
            let paver_d = (z_step * 0.76).min(0.42);
            let paver_w = walk_band * 0.68;
            for px in [x0 + walk_band * 0.52, x1 - walk_band * 0.52] {
                let r = [
                    px - paver_w * 0.5,
                    pz - paver_d * 0.5,
                    px + paver_w * 0.5,
                    pz + paver_d * 0.5,
                ];
                if !sw_cutouts.iter().any(|&c| rect_overlaps(r, c)) {
                    mb.push_box(
                        px,
                        sw_h,
                        pz,
                        paver_w,
                        0.012,
                        paver_d,
                        theme.sidewalk_paver,
                        0.0,
                    );
                }
            }
        }

        // Gothic corner curb bollards at all 4 corners of the block
        for bx in [x0 + 0.14, x1 - 0.14] {
            for bz in [z0 + 0.14, z1 - 0.14] {
                let r = [bx - 0.06, bz - 0.06, bx + 0.06, bz + 0.06];
                if sw_cutouts.iter().any(|&c| rect_overlaps(r, c)) {
                    continue;
                }
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

        // Cast-iron fire hydrants at alternating diagonal block corners + wrought-iron benches on mid-edges
        if block.id % 2 == 1 {
            let hx = x0 + 0.30;
            let hz = z1 - 0.18;
            let r = [hx - 0.06, hz - 0.06, hx + 0.06, hz + 0.06];
            if !sw_cutouts.iter().any(|&c| rect_overlaps(r, c)) {
                mb.push_box(hx, sw_h, hz, 0.07, 0.12, 0.07, theme.beacon_red, 0.0);
                mb.push_box(
                    hx,
                    sw_h + 0.12,
                    hz,
                    0.09,
                    0.03,
                    0.09,
                    theme.accent_trim,
                    0.0,
                );
            }
        }
        // Wrought-iron sidewalk bench centered on the south/north sidewalk edge
        let bench_x = (x0 + x1) * 0.5;
        let bench_z = if block.id % 2 == 0 {
            z1 - 0.18
        } else {
            z0 + 0.18
        };
        let br = [
            bench_x - 0.20,
            bench_z - 0.08,
            bench_x + 0.20,
            bench_z + 0.08,
        ];
        if !sw_cutouts.iter().any(|&c| rect_overlaps(br, c)) {
            mb.push_box(
                bench_x,
                sw_h,
                bench_z,
                0.32,
                0.05,
                0.10,
                theme.plinth_dark,
                0.0,
            );
            mb.push_box(
                bench_x,
                sw_h + 0.05,
                bench_z,
                0.30,
                0.025,
                0.09,
                theme.accent_trim,
                0.0,
            );
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
            // Also skip garage and underground portal turnoff points
            let near_garage = bp.garages.iter().any(|g| {
                (mx - g.street_entry[0]).abs() < 0.55 && (mz - g.street_entry[1]).abs() < 0.55
            });
            let near_portal = bp
                .underground_portals
                .iter()
                .any(|p| (mx - p.entry_pos[0]).abs() < 0.55 && (mz - p.entry_pos[1]).abs() < 0.55);
            if near_garage || near_portal {
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
                    // Skip if a garage driveway apron or underground portal occupies this sidewalk spot
                    let blocks_garage = bp.garages.iter().any(|g| {
                        (px - g.portal_pos[0]).abs() < 0.75 && (pz - g.portal_pos[1]).abs() < 0.75
                    });
                    let blocks_portal = bp.underground_portals.iter().any(|p| {
                        (px - p.portal_pos[0]).abs() < 1.35 && (pz - p.portal_pos[1]).abs() < 1.35
                    });
                    if blocks_garage || blocks_portal {
                        continue;
                    }
                    mb.push_box(px, 0.075, pz, 0.06, 0.34, 0.06, theme.stone_dark, 0.0);
                    mb.push_box(px, 0.415, pz, 0.11, 0.10, 0.11, theme.window_warm, 1.0);
                    // Wrought-iron lantern cap + subtle warm sidewalk glow pool
                    mb.push_pyramid(px, 0.515, pz, 0.15, 0.06, 0.15, theme.plinth_dark, 0.0);
                    mb.push_box(px, 0.077, pz, 0.34, 0.008, 0.34, theme.window_dim, 0.55);
                }
            }
        }

        // Cast-iron utility manhole covers & gutter storm grates along inner-city avenues
        if !ave.is_highway && len > 2.0 {
            let right = [-fwd[1], fwd[0]];
            let grate_steps = (len / 2.6).floor().max(1.0) as usize;
            for i in 0..grate_steps {
                let t = (i as f32 + 0.5) / grate_steps as f32;
                let gx = ave.start[0] + dx * t;
                let gz = ave.start[1] + dz * t;
                let near_junction = bp.avenues.iter().enumerate().any(|(other_idx, other)| {
                    other_idx != ave_idx
                        && point_to_avenue_dist(gx, gz, other) < other.width * 0.5 + 0.45
                });
                if near_junction {
                    continue;
                }
                // Alternating manhole cover near center lane & storm drain along curb gutter
                if i % 2 == 0 {
                    mb.push_box(
                        gx + right[0] * 0.18,
                        0.026,
                        gz + right[1] * 0.18,
                        0.15,
                        0.008,
                        0.15,
                        theme.plinth_trim,
                        0.0,
                    );
                } else {
                    let side = if i % 4 == 1 { 1.0 } else { -1.0 };
                    let edge = ave.width * 0.44 * side;
                    mb.push_oriented_box(
                        [gx + right[0] * edge, 0.026, gz + right[1] * edge],
                        fwd,
                        0.22,
                        0.008,
                        0.12,
                        theme.stone_dark,
                        0.0,
                    );
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
    bp: &CityBlueprint,
    garage: &GaragePortal,
    theme: &GothicThemeColors,
) {
    let fwd = garage.inward_dir;
    let right = [-fwd[1], fwd[0]];
    let w = garage.width;

    let road_half_w = road_half_width_at(bp, garage.street_entry);
    let road_edge = [
        garage.street_entry[0] + fwd[0] * road_half_w,
        garage.street_entry[1] + fwd[1] * road_half_w,
    ];
    let dx = garage.portal_pos[0] - road_edge[0];
    let dz = garage.portal_pos[1] - road_edge[1];
    let apron_len = (dx * dx + dz * dz).sqrt().max(0.25);

    // 1. Asphalt driveway apron cutting across the raised sidewalk from road edge to building portal
    let apron_cx = (road_edge[0] + garage.portal_pos[0]) * 0.5;
    let apron_cz = (road_edge[1] + garage.portal_pos[1]) * 0.5;
    mb.push_oriented_box(
        [apron_cx, 0.005, apron_cz],
        fwd,
        apron_len + 0.06,
        0.022,
        w * 0.96,
        theme.asphalt,
        0.0,
    );
    // Amber center guide stripe leading into the garage portal (strictly inside the apron)
    mb.push_oriented_box(
        [apron_cx, 0.026, apron_cz],
        fwd,
        (apron_len - 0.08).max(0.15),
        0.008,
        0.06,
        theme.accent_trim,
        0.0,
    );

    // 2. Flanking stone curb wing-walls bordering the garage driveway across the sidewalk
    let cheek_w = 0.09f32;
    let wall_len = (apron_len - 0.04).max(0.20);
    for side in [-1.0f32, 1.0f32] {
        let wx = apron_cx + right[0] * (w * 0.48 + cheek_w * 0.5) * side;
        let wz = apron_cz + right[1] * (w * 0.48 + cheek_w * 0.5) * side;
        mb.push_oriented_box(
            [wx, 0.0, wz],
            fwd,
            wall_len,
            0.14,
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
    bp: &CityBlueprint,
    portal: &UndergroundPortal,
    theme: &GothicThemeColors,
) {
    let fwd = portal.inward_dir;
    let right = [-fwd[1], fwd[0]];
    let w = portal.width;

    let road_half_w = road_half_width_at(bp, portal.entry_pos);
    let road_edge = [
        portal.entry_pos[0] + fwd[0] * road_half_w,
        portal.entry_pos[1] + fwd[1] * road_half_w,
    ];

    // 1. Sloped asphalt ramp descending from the road edge into the sunken tunnel mouth
    mb.push_sloped_box(
        [road_edge[0], -0.015, road_edge[1]],
        [portal.portal_pos[0], -0.24, portal.portal_pos[1]],
        0.04,
        w * 0.94,
        theme.asphalt,
        0.0,
    );
    // Amber center guide stripe descending into the tunnel (inset inside the road edge)
    let guide_start = [road_edge[0] + fwd[0] * 0.04, road_edge[1] + fwd[1] * 0.04];
    mb.push_sloped_box(
        [guide_start[0], 0.018, guide_start[1]],
        [portal.portal_pos[0], -0.195, portal.portal_pos[1]],
        0.01,
        0.06,
        theme.accent_trim,
        0.0,
    );

    // 2. Flanking stone retaining walls bordering the sunken ramp (strictly inside road edge)
    let dx = portal.portal_pos[0] - road_edge[0];
    let dz = portal.portal_pos[1] - road_edge[1];
    let ramp_len = (dx * dx + dz * dz).sqrt().max(0.4);
    let mid_x = (road_edge[0] + portal.portal_pos[0]) * 0.5;
    let mid_z = (road_edge[1] + portal.portal_pos[1]) * 0.5;
    let wall_len = (ramp_len - 0.04).max(0.32);
    for side in [-1.0f32, 1.0f32] {
        let wx = mid_x + right[0] * (w * 0.50 + 0.06) * side;
        let wz = mid_z + right[1] * (w * 0.50 + 0.06) * side;
        mb.push_oriented_box(
            [wx, -0.26, wz],
            fwd,
            wall_len,
            0.48,
            0.12,
            theme.stone_secondary,
            0.0,
        );
        mb.push_oriented_box(
            [wx, 0.22, wz],
            fwd,
            wall_len,
            0.04,
            0.14,
            theme.accent_trim,
            0.0,
        );
    }

    // 3. Heavy gothic stone tunnel portal piers, overhead lintel & overhanging cornice
    let portal_h = 0.78f32;
    let lintel_y = portal_h - 0.22;
    let arch_cx = portal.portal_pos[0] + fwd[0] * 0.18;
    let arch_cz = portal.portal_pos[1] + fwd[1] * 0.18;
    let pier_w = 0.18f32;
    for side in [-1.0f32, 1.0f32] {
        let px = arch_cx + right[0] * (w * 0.50 + pier_w * 0.5) * side;
        let pz = arch_cz + right[1] * (w * 0.50 + pier_w * 0.5) * side;
        mb.push_oriented_box(
            [px, -0.26, pz],
            fwd,
            0.52,
            portal_h + 0.26,
            pier_w,
            theme.stone_secondary,
            0.0,
        );
    }
    // Overhead stone lintel above the open tunnel mouth
    mb.push_oriented_box(
        [arch_cx, lintel_y, arch_cz],
        fwd,
        0.52,
        portal_h - lintel_y,
        w + pier_w * 2.0,
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

    // 4. Pitch-black subterranean tunnel mouth void recessed behind the open portal piers
    let void_color = [0.01, 0.01, 0.018, 1.0];
    mb.push_oriented_box(
        [
            portal.portal_pos[0] + fwd[0] * 0.18,
            -0.26,
            portal.portal_pos[1] + fwd[1] * 0.18,
        ],
        fwd,
        0.48,
        lintel_y + 0.26,
        w,
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
    let pts = bp.viaduct_points();
    let n = pts.len();
    if n < 2 {
        return;
    }
    let y = bp.viaduct_elevation;
    let deck_w = 0.72;
    let total_len = bp.viaduct_total_length();

    // 1. Continuous curved girder deck, under-truss keel, running rails, and art-deco guard parapets
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let dx = b[0] - a[0];
        let dz = b[1] - a[1];
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-4 {
            continue;
        }
        let cx = (a[0] + b[0]) * 0.5;
        let cz = (a[1] + b[1]) * 0.5;
        let fwd = [dx / len, dz / len];
        let right = [-fwd[1], fwd[0]];
        // Slight overlap seals outer miter gaps on curved segments without protruding past corners
        let overlap = 0.08;

        // Main steel viaduct girder deck
        mb.push_oriented_box(
            [cx, y - 0.14, cz],
            fwd,
            len + overlap,
            0.14,
            deck_w,
            theme.steel_viaduct,
            0.0,
        );

        // Stepped dark under-belly structural keel
        mb.push_oriented_box(
            [cx, y - 0.23, cz],
            fwd,
            len + overlap * 0.6,
            0.09,
            deck_w * 0.54,
            theme.stone_dark,
            0.0,
        );

        // Outer art-deco parapet guard girders + inner twin steel running rails
        for side in [-1.0f32, 1.0f32] {
            let px = cx + right[0] * (deck_w * 0.47) * side;
            let pz = cz + right[1] * (deck_w * 0.47) * side;
            mb.push_oriented_box(
                [px, y - 0.01, pz],
                fwd,
                len + overlap * 0.9,
                0.10,
                0.06,
                theme.stone_secondary,
                0.0,
            );

            // Inner steel running rail
            let rx = cx + right[0] * (deck_w * 0.25) * side;
            let rz = cz + right[1] * (deck_w * 0.25) * side;
            mb.push_oriented_box(
                [rx, y + 0.01, rz],
                fwd,
                len + overlap * 0.5,
                0.045,
                0.04,
                theme.accent_trim,
                0.0,
            );
        }
    }

    // 2. Uniformly spaced railway sleepers (cross-ties) along the smoothed loop
    let sleeper_spacing = 0.36f32;
    let sleeper_count = (total_len / sleeper_spacing).round().max(1.0) as usize;
    let actual_sleeper_step = total_len / sleeper_count as f32;
    for s in 0..sleeper_count {
        let dist = (s as f32 + 0.5) * actual_sleeper_step;
        let (pos, fwd) = bp.sample_viaduct(dist);
        mb.push_oriented_box(
            [pos[0], y, pos[2]],
            fwd,
            0.10,
            0.025,
            deck_w * 0.72,
            theme.plinth_dark,
            0.0,
        );
    }

    // 3. Support piers with stepped plinth bases and gothic cross-arch headers (skipping inside buildings)
    let pier_spacing = 2.65f32;
    let pier_count = (total_len / pier_spacing).round().max(1.0) as usize;
    let actual_pier_step = total_len / pier_count as f32;
    for p in 0..pier_count {
        let dist = (p as f32 + 0.5) * actual_pier_step;
        let (pos, fwd) = bp.sample_viaduct(dist);
        let px = pos[0];
        let pz = pos[2];
        let right = [-fwd[1], fwd[0]];

        let inside_building = bp.buildings.iter().any(|bld| {
            (px - bld.x).abs() < bld.width * 0.5 + 0.35
                && (pz - bld.z).abs() < bld.depth * 0.5 + 0.35
        });
        if inside_building {
            continue;
        }

        for side in [-1.0f32, 1.0f32] {
            let col_x = px + right[0] * (deck_w * 0.34) * side;
            let col_z = pz + right[1] * (deck_w * 0.34) * side;
            // Stepped stone plinth base
            mb.push_oriented_box(
                [col_x, 0.0, col_z],
                fwd,
                0.24,
                0.18,
                0.24,
                theme.stone_secondary,
                0.0,
            );
            // Main pier shaft
            mb.push_oriented_box(
                [col_x, 0.18, col_z],
                fwd,
                0.15,
                y - 0.34,
                0.15,
                theme.stone_dark,
                0.0,
            );
        }
        // Gothic cross-arch header & corbel cap under the viaduct deck
        mb.push_oriented_box(
            [px, y - 0.38, pz],
            fwd,
            0.22,
            0.22,
            deck_w * 0.92,
            theme.stone_secondary,
            0.0,
        );
        mb.push_oriented_box(
            [px, y - 0.28, pz],
            fwd,
            0.28,
            0.12,
            deck_w * 1.04,
            theme.steel_viaduct,
            0.0,
        );
    }
}
