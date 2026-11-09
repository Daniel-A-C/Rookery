// Atmospheric & stylistic background shader for The Living ASCII Rookery.
// Supports 8 distinct visual mock-up styles via `u.time_of_day` (repurposed as `style_id` 0..7).

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    style_id: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vid << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vid & 2u) * -2) + 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let d = dot(q, q + 45.32);
    return fract(d);
}

// Style 0: Classic SDF Woodland (Baseline)
fn bg_classic(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let sky_zenith = vec3<f32>(0.020, 0.032, 0.068);
    let sky_mid = vec3<f32>(0.048, 0.068, 0.118);
    let sky_glow = vec3<f32>(0.115, 0.098, 0.142);
    let sky_horizon = vec3<f32>(0.195, 0.142, 0.125);

    var sky = mix(sky_zenith, sky_mid, smoothstep(0.0, 0.48, uv.y));
    sky = mix(sky, sky_glow, smoothstep(0.48, 0.78, uv.y));
    sky = mix(sky, sky_horizon, smoothstep(0.78, 0.90, uv.y));

    if (uv.y < 0.68) {
        let grid_size = 36.0;
        let cell = floor(uv * res / grid_size);
        let star_rnd = hash21(cell);
        if (star_rnd > 0.952) {
            let star_pos = (cell + vec2<f32>(hash21(cell + 1.0), hash21(cell + 2.0))) * grid_size / res;
            let d = length((uv - star_pos) * res);
            let twinkle = 0.55 + 0.45 * sin(u.time * (1.6 + star_rnd * 3.2) + star_rnd * 100.0);
            let star_alpha = smoothstep(1.9, 0.0, d) * twinkle * (1.0 - uv.y / 0.68) * 0.60;
            let star_color = mix(
                vec3<f32>(0.92, 0.95, 1.0),
                vec3<f32>(1.0, 0.88, 0.65),
                fract(star_rnd * 7.0)
            );
            sky += star_color * star_alpha;
        }
    }

    if (uv.y > 0.74 && uv.y < 0.92) {
        let mist_wave1 = sin(uv.x * 4.5 + u.time * 0.18) * 0.5 + 0.5;
        let mist_wave2 = cos(uv.x * 9.2 - u.time * 0.11) * 0.5 + 0.5;
        let mist_factor = mist_wave1 * 0.6 + mist_wave2 * 0.4;
        let mist_envelope = smoothstep(0.74, 0.85, uv.y) * smoothstep(0.92, 0.86, uv.y);
        let mist_color = vec3<f32>(0.085, 0.095, 0.115);
        sky = mix(sky, sky + mist_color * 0.35, mist_envelope * mist_factor);
    }

    let ground_y = 0.898 + 0.014 * sin(uv.x * 3.8 + 0.6) + 0.007 * sin(uv.x * 11.2 - 0.4) + 0.003 * sin(uv.x * 24.0);
    if (uv.y > ground_y) {
        let moss_crest = vec3<f32>(0.065, 0.088, 0.052);
        let soil_top = vec3<f32>(0.045, 0.048, 0.038);
        let soil_deep = vec3<f32>(0.018, 0.020, 0.022);

        let ground_blend = smoothstep(ground_y, ground_y + 0.008, uv.y);
        let moss_to_soil = smoothstep(ground_y + 0.008, ground_y + 0.032, uv.y);
        let soil_to_deep = smoothstep(ground_y + 0.032, ground_y + 0.10, uv.y);

        var ground_col = mix(moss_crest, soil_top, moss_to_soil);
        ground_col = mix(ground_col, soil_deep, soil_to_deep);
        sky = mix(sky, ground_col, ground_blend);
    }

    return sky;
}

// Style 1: Strict ANSI Roguelike Grid
fn bg_roguelike(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let px = uv * res;
    let cell_w = 12.0;
    let cell_h = 20.0;
    let gx = fract(px.x / cell_w);
    let gy = fract(px.y / cell_h);

    var col = vec3<f32>(0.035, 0.047, 0.066); // crisp terminal slate
    // Subtle monospace cell grid lines
    if (gx < (1.0 / cell_w) || gy < (1.0 / cell_h)) {
        col += vec3<f32>(0.014, 0.022, 0.030);
    }

    // Subterranean soil strata below ground row
    if (uv.y > 0.88) {
        col = vec3<f32>(0.045, 0.035, 0.028);
        if (gx < (1.0 / cell_w) || gy < (1.0 / cell_h)) {
            col += vec3<f32>(0.018, 0.012, 0.008);
        }
    }
    return col;
}

// Style 2: Full-Screen ASCII Matrix & Dithering
fn bg_matrix_dither(uv: vec2<f32>) -> vec3<f32> {
    // Deep nocturnal void with soft moonbeam radial pool so the ASCII matrix carries the light
    var col = mix(vec3<f32>(0.008, 0.014, 0.024), vec3<f32>(0.018, 0.032, 0.045), uv.y);
    let moon_pos = vec2<f32>(0.78, 0.20);
    let d_moon = length((uv - moon_pos) * vec2<f32>(1.6, 1.0));
    col += vec3<f32>(0.04, 0.08, 0.11) * exp(-d_moon * 4.5);
    return col;
}

// Style 3: Multi-Line Classic ASCII Art & Parallax
fn bg_multiline_parallax(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    // Quantized retro 16-color text-mode sky bands with Bayer-like dither seam
    let dither = (hash21(floor(uv * res * 0.5)) - 0.5) * 0.035;
    let y_q = floor((uv.y + dither) * 7.0) / 7.0;
    let c_top = vec3<f32>(0.035, 0.030, 0.085);
    let c_mid = vec3<f32>(0.075, 0.065, 0.165);
    let c_low = vec3<f32>(0.155, 0.105, 0.195);
    var col = mix(c_top, c_mid, clamp(y_q * 1.4, 0.0, 1.0));
    col = mix(col, c_low, clamp((y_q - 0.45) * 1.8, 0.0, 1.0));
    if (uv.y > 0.895) {
        col = vec3<f32>(0.032, 0.052, 0.042);
    }
    return col;
}

// Style 4: Retro Phosphor CRT Terminal
fn bg_crt_phosphor(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let centered = uv * 2.0 - 1.0;
    let r2 = dot(centered, centered);
    // Warm amber/green phosphor tube glow
    var col = vec3<f32>(0.032, 0.022, 0.006);
    let tree_glow = exp(-length((uv - vec2<f32>(0.5, 0.55)) * vec2<f32>(1.3, 1.0)) * 3.2);
    col += vec3<f32>(0.065, 0.042, 0.008) * tree_glow;

    // Horizontal CRT scanlines
    let scanline = 0.84 + 0.16 * sin(uv.y * res.y * 1.570796);
    col *= scanline;

    // Subtle oscilloscope / radar graticule ticks
    let px = uv * res;
    let gx = abs(fract(px.x / 96.0) - 0.5) * 96.0;
    let gy = abs(fract(px.y / 96.0) - 0.5) * 96.0;
    if ((gx < 0.8 && gy < 5.0) || (gy < 0.8 && gx < 5.0)) {
        col += vec3<f32>(0.055, 0.038, 0.010);
    }

    // Barrel vignette
    let vig = clamp(1.0 - r2 * 0.38, 0.15, 1.0);
    return col * vig;
}

// Style 5: Sumi-e Ink & Botanical Field Journal (Warm Washi Parchment)
fn bg_sumie_parchment(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let centered = uv * 2.0 - 1.0;
    let r2 = dot(centered, centered);

    // Warm cream Washi paper base with aged sepia edge vignette
    let paper_center = vec3<f32>(0.945, 0.918, 0.858);
    let paper_edge = vec3<f32>(0.835, 0.785, 0.690);
    var col = mix(paper_center, paper_edge, clamp(r2 * 0.55, 0.0, 1.0));

    // Fine procedural paper fiber grain
    let grain = (hash21(floor(uv * res * 0.5)) - 0.5) * 0.022;
    col += vec3<f32>(grain);

    // Faint botanical ledger grid rules
    let px = uv * res;
    let gx = fract(px.x / 64.0);
    let gy = fract(px.y / 64.0);
    if (gx < (1.0 / 64.0) || gy < (1.0 / 64.0)) {
        col -= vec3<f32>(0.032, 0.035, 0.040);
    }

    // Soft ink-wash horizon brush stroke at ground line
    let ground_y = 0.898 + 0.008 * sin(uv.x * 5.0);
    if (uv.y > ground_y) {
        let wash = smoothstep(ground_y, ground_y + 0.04, uv.y);
        col = mix(col, vec3<f32>(0.76, 0.71, 0.62), wash * 0.55);
    }

    return col;
}

// Style 6: Jewel-Box Stained-Glass Mosaic
fn bg_stained_glass(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let center = vec2<f32>(0.5, 0.50);
    let d_vec = (uv - center) * vec2<f32>(res.x / max(res.y, 1.0), 1.0);
    let r = length(d_vec);
    let angle = atan2(d_vec.y, d_vec.x);

    // Cathedral obsidian base with radial stained-glass rose window back-lighting
    var col = vec3<f32>(0.025, 0.018, 0.048);
    let glow = exp(-r * 2.4);
    let hue_shift = 0.5 + 0.5 * sin(angle * 6.0 + r * 8.0);
    let glass_tint = mix(vec3<f32>(0.06, 0.16, 0.28), vec3<f32>(0.22, 0.08, 0.26), hue_shift);
    col += glass_tint * glow;

    // Dark leaded-glass concentric rings & radial ribs
    let ring_dist = abs(fract(r * 5.5) - 0.5);
    let spoke_dist = abs(fract((angle / 6.2831853) * 16.0) - 0.5);
    if (ring_dist < 0.010 || spoke_dist < 0.008) {
        col *= 0.55;
    }

    return col;
}

// Style 7: 3D Volumetric ASCII Tree Visualizer
fn bg_volumetric_3d(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let horizon_y = 0.68;
    var col = mix(vec3<f32>(0.014, 0.024, 0.048), vec3<f32>(0.032, 0.068, 0.118), clamp(uv.y / horizon_y, 0.0, 1.0));

    // Volumetric turntable spotlight glow
    let spot = exp(-length((uv - vec2<f32>(0.5, 0.56)) * vec2<f32>(1.3, 1.5)) * 2.6);
    col += vec3<f32>(0.04, 0.11, 0.10) * spot;

    // Perspective 3D studio floor wireframe grid below horizon
    if (uv.y > horizon_y) {
        let dy = max(uv.y - horizon_y, 0.002);
        let z = 0.22 / dy;
        let x_persp = (uv.x - 0.5) * z * (res.x / max(res.y, 1.0));
        let gz = abs(fract(z + u.time * 0.12) - 0.5);
        let gx = abs(fract(x_persp * 1.4) - 0.5);
        let fade = smoothstep(0.0, 0.06, dy) * exp(-z * 0.22);
        if (gz < 0.018 || gx < 0.018) {
            col += vec3<f32>(0.05, 0.18, 0.24) * fade;
        }
    }

    return col;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;
    let style = i32(round(u.style_id));

    var color = vec3<f32>(0.0);
    switch style {
        case 1: {
            color = bg_roguelike(uv, res);
        }
        case 2: {
            color = bg_matrix_dither(uv);
        }
        case 3: {
            color = bg_multiline_parallax(uv, res);
        }
        case 4: {
            color = bg_crt_phosphor(uv, res);
        }
        case 5: {
            color = bg_sumie_parchment(uv, res);
        }
        case 6: {
            color = bg_stained_glass(uv, res);
        }
        case 7: {
            color = bg_volumetric_3d(uv, res);
        }
        default: {
            color = bg_classic(uv, res);
        }
    }

    return vec4<f32>(color, 1.0);
}
