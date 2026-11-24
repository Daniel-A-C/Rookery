// Atmospheric monospace terminal background shader for Shinjuku Rain.
// Renders rainy night sky gradients, distant storm clouds & lightning flashes,
// wet asphalt street reflections with rain-ripple shimmer, and crisp ASCII cell grid lines.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    palette_id: f32,
    lightning: f32,
    signs_lit: f32,
    wind: f32,
    prev_palette_id: f32,
    palette_blend: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct BgPalette {
    zenith: vec3<f32>,
    horizon: vec3<f32>,
    street_base: vec3<f32>,
    neon_a: vec3<f32>,
    neon_b: vec3<f32>,
    neon_c: vec3<f32>,
    grid_glow: vec3<f32>,
}

fn palette_at(idx: i32) -> BgPalette {
    var p: BgPalette;
    p.zenith = vec3<f32>(0.016, 0.014, 0.042);
    p.horizon = vec3<f32>(0.055, 0.032, 0.095);
    p.street_base = vec3<f32>(0.018, 0.020, 0.034);
    p.neon_a = vec3<f32>(0.95, 0.18, 0.52); // Magenta / Rose
    p.neon_b = vec3<f32>(0.12, 0.82, 0.96); // Electric Cyan
    p.neon_c = vec3<f32>(0.98, 0.62, 0.16); // Warm Ramen Amber
    p.grid_glow = vec3<f32>(0.016, 0.014, 0.030);

    switch idx {
        case 1: {
            // Kyoto Gion: deep indigo rain, vermilion lanterns, warm stone
            p.zenith = vec3<f32>(0.012, 0.016, 0.034);
            p.horizon = vec3<f32>(0.045, 0.036, 0.058);
            p.street_base = vec3<f32>(0.020, 0.022, 0.030);
            p.neon_a = vec3<f32>(0.96, 0.28, 0.18);
            p.neon_b = vec3<f32>(0.24, 0.68, 0.62);
            p.neon_c = vec3<f32>(0.98, 0.74, 0.24);
            p.grid_glow = vec3<f32>(0.014, 0.016, 0.024);
        }
        case 2: {
            // Midnight Downpour: cool steel slate, silver rain, sodium vapor
            p.zenith = vec3<f32>(0.012, 0.018, 0.028);
            p.horizon = vec3<f32>(0.036, 0.050, 0.072);
            p.street_base = vec3<f32>(0.016, 0.022, 0.032);
            p.neon_a = vec3<f32>(0.55, 0.76, 0.98);
            p.neon_b = vec3<f32>(0.78, 0.86, 0.96);
            p.neon_c = vec3<f32>(0.94, 0.72, 0.34);
            p.grid_glow = vec3<f32>(0.012, 0.016, 0.024);
        }
        case 3: {
            // Cyber Terminal: CRT phosphor emerald & amber
            p.zenith = vec3<f32>(0.004, 0.016, 0.008);
            p.horizon = vec3<f32>(0.012, 0.042, 0.020);
            p.street_base = vec3<f32>(0.006, 0.020, 0.010);
            p.neon_a = vec3<f32>(0.18, 0.96, 0.42);
            p.neon_b = vec3<f32>(0.96, 0.72, 0.16);
            p.neon_c = vec3<f32>(0.36, 0.98, 0.58);
            p.grid_glow = vec3<f32>(0.006, 0.024, 0.012);
        }
        default: {}
    }
    return p;
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
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_f = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u_f.x), mix(c, d, u_f.x), u_f.y);
}

fn fbm3(p: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i = i + 1) {
        val += amp * noise2d(pos);
        pos = pos * 2.03 + vec2<f32>(17.1, 31.7);
        amp *= 0.5;
    }
    return val;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;
    let px = uv * res;

    let cell_w = 9.6;
    let cell_h = 16.0;
    let gx = fract(px.x / cell_w);
    let gy = fract(px.y / cell_h);
    let is_grid_line = (gx < (1.0 / cell_w)) || (gy < (1.0 / cell_h));

    let curr_idx = i32(round(u.palette_id)) % 4;
    let prev_idx = i32(round(u.prev_palette_id)) % 4;
    let s = clamp(u.palette_blend, 0.0, 1.0);
    let blend = s * s * (3.0 - 2.0 * s);

    let pa = palette_at(prev_idx);
    let pb = palette_at(curr_idx);

    let zenith = mix(pa.zenith, pb.zenith, blend);
    let horizon = mix(pa.horizon, pb.horizon, blend);
    let street_base = mix(pa.street_base, pb.street_base, blend);
    let neon_a = mix(pa.neon_a, pb.neon_a, blend);
    let neon_b = mix(pa.neon_b, pb.neon_b, blend);
    let neon_c = mix(pa.neon_c, pb.neon_c, blend);
    let grid_glow = mix(pa.grid_glow, pb.grid_glow, blend);

    let curb_y = 0.76;
    var col = vec3<f32>(0.0);

    if uv.y < curb_y {
        let sky_t = clamp(uv.y / curb_y, 0.0, 1.0);
        col = mix(zenith, horizon, pow(sky_t, 1.4));

        // Drifting storm clouds
        let cloud_uv = vec2<f32>(uv.x * 3.2 + u.time * 0.015 * (1.0 + u.wind), uv.y * 2.4);
        let clouds = fbm3(cloud_uv);
        col += vec3<f32>(0.02, 0.025, 0.04) * smoothstep(0.42, 0.85, clouds) * (1.0 - sky_t * 0.6);

        // Distant lightning flash illuminating upper clouds
        let flash = u.lightning * (0.35 + 0.65 * clouds) * (1.0 - sky_t * 0.7);
        col += vec3<f32>(0.55, 0.65, 0.95) * flash * 0.45;
    } else {
        // Wet asphalt street & sidewalk plane
        let street_t = clamp((uv.y - curb_y) / (1.0 - curb_y), 0.0, 1.0);
        col = mix(street_base * 1.35, street_base * 0.75, street_t);

        // Animated rain-ripple shimmer across the wet pavement
        let ripple_x = sin(uv.y * 90.0 - u.time * 5.2 + sin(uv.x * 35.0) * 1.8) * 0.012;
        let rx = uv.x + ripple_x;

        // Vertical neon reflections from storefronts above
        let sign_strength = mix(0.22, 1.0, u.signs_lit);
        let fade = (1.0 - street_t * 0.55) * sign_strength;

        // Ramen shop warm amber reflection (x ~ 0.18..0.28)
        let r_ramen = exp(-pow((rx - 0.22) / 0.065, 2.0));
        col += neon_c * r_ramen * 0.20 * fade;

        // Arcade / Bar magenta reflection (x ~ 0.44..0.54)
        let r_bar = exp(-pow((rx - 0.48) / 0.075, 2.0));
        col += neon_a * r_bar * 0.22 * fade;

        // Vending machine & Konbini cyan reflection (x ~ 0.68..0.82)
        let r_vending = exp(-pow((rx - 0.74) / 0.070, 2.0));
        col += neon_b * r_vending * 0.20 * fade;

        // Curb highlight line
        let curb_line = exp(-pow((uv.y - curb_y) / 0.004, 2.0));
        col += (neon_b * 0.14 + vec3<f32>(0.08)) * curb_line;

        // Lightning sheen on wet asphalt
        col += vec3<f32>(0.35, 0.45, 0.70) * u.lightning * 0.18;
    }

    if is_grid_line {
        col += grid_glow * 0.55;
    }

    return vec4<f32>(col, 1.0);
}
