// Atmospheric & stylistic monospace terminal background shader for The Living ASCII Rookery.
// Renders subtle sky gradients, subterranean soil strata, and crisp cell grid lines per palette.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    palette_id: f32,
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
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
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
    let aspect = res.x / max(res.y, 1.0);

    let cell_w = 9.6;
    let cell_h = 16.0;
    let gx = fract(px.x / cell_w);
    let gy = fract(px.y / cell_h);
    let is_grid_line = (gx < (1.0 / cell_w)) || (gy < (1.0 / cell_h));

    let palette = i32(round(u.palette_id)) % 6;

    var zenith = vec3<f32>(0.016, 0.024, 0.038);
    var horizon = vec3<f32>(0.042, 0.056, 0.078);
    var meadow_deep = vec3<f32>(0.036, 0.088, 0.048);
    var meadow_front = vec3<f32>(0.048, 0.112, 0.062);
    var grid_glow = vec3<f32>(0.014, 0.020, 0.028);
    var moon_tint = vec3<f32>(0.78, 0.86, 1.00);

    switch palette {
        case 1: {
            // Midnight Starlight: obsidian sky, frosted sapphire horizon, luminescent teal glade
            zenith = vec3<f32>(0.008, 0.010, 0.018);
            horizon = vec3<f32>(0.022, 0.034, 0.054);
            meadow_deep = vec3<f32>(0.024, 0.068, 0.086);
            meadow_front = vec3<f32>(0.032, 0.088, 0.112);
            grid_glow = vec3<f32>(0.012, 0.018, 0.028);
            moon_tint = vec3<f32>(0.75, 0.88, 1.00);
        }
        case 2: {
            // Autumn Hearth: warm dusky sky, amber horizon, golden-olive meadow
            zenith = vec3<f32>(0.028, 0.016, 0.022);
            horizon = vec3<f32>(0.068, 0.038, 0.030);
            meadow_deep = vec3<f32>(0.068, 0.058, 0.024);
            meadow_front = vec3<f32>(0.088, 0.074, 0.032);
            grid_glow = vec3<f32>(0.022, 0.015, 0.012);
            moon_tint = vec3<f32>(1.00, 0.86, 0.68);
        }
        case 3: {
            // Verdant Rainforest: deep forest night, vivid emerald floor
            zenith = vec3<f32>(0.010, 0.026, 0.018);
            horizon = vec3<f32>(0.026, 0.062, 0.038);
            meadow_deep = vec3<f32>(0.028, 0.098, 0.048);
            meadow_front = vec3<f32>(0.038, 0.125, 0.062);
            grid_glow = vec3<f32>(0.012, 0.025, 0.016);
            moon_tint = vec3<f32>(0.80, 0.98, 0.86);
        }
        case 4: {
            // Amber Phosphor CRT: warm monochrome amber terminal
            zenith = vec3<f32>(0.020, 0.012, 0.002);
            horizon = vec3<f32>(0.055, 0.034, 0.006);
            meadow_deep = vec3<f32>(0.060, 0.038, 0.009);
            meadow_front = vec3<f32>(0.078, 0.050, 0.012);
            grid_glow = vec3<f32>(0.025, 0.015, 0.003);
            moon_tint = vec3<f32>(1.00, 0.82, 0.30);
        }
        case 5: {
            // Emerald Matrix: hacker green phosphor terminal
            zenith = vec3<f32>(0.004, 0.022, 0.008);
            horizon = vec3<f32>(0.012, 0.055, 0.018);
            meadow_deep = vec3<f32>(0.016, 0.070, 0.024);
            meadow_front = vec3<f32>(0.022, 0.092, 0.032);
            grid_glow = vec3<f32>(0.006, 0.025, 0.010);
            moon_tint = vec3<f32>(0.45, 0.98, 0.58);
        }
        default: {
            // Earth & Sylvan (Caves of Qud): default
        }
    }

    var col = mix(zenith, horizon, smoothstep(0.0, 0.80, uv.y));

    // Subtle slow-moving clouds / haze in the upper sky
    if (uv.y < 0.65) {
        let sky_band = smoothstep(0.01, 0.14, uv.y) * (1.0 - smoothstep(0.28, 0.62, uv.y));
        let cloud_uv = vec2<f32>(uv.x * aspect * 2.2 + u.time * 0.007, uv.y * 6.5 + u.time * 0.0012);
        let cloud_n = fbm3(cloud_uv);
        let cloud_wisp = smoothstep(0.46, 0.82, cloud_n) * sky_band;
        let haze_col = mix(horizon * 1.45, moon_tint * 0.14, 0.25);
        col += haze_col * cloud_wisp * 0.28;
    }

    // Soft upper-right atmospheric moonlight bloom (smoothly fades to horizon with no hard cutoff)
    if (uv.y < 0.82) {
        let moon_uv = vec2<f32>(0.92, 0.08);
        let delta = vec2<f32>((uv.x - moon_uv.x) * aspect, uv.y - moon_uv.y);
        let moon_dist = length(delta);
        let sky_fade = 1.0 - smoothstep(0.45, 0.80, uv.y);
        let moon_glow = (exp(-moon_dist * 4.2) * 0.055 + exp(-moon_dist * 1.6) * 0.020) * sky_fade;
        col += moon_tint * moon_glow;
    }

    if (is_grid_line) {
        col += grid_glow;
    }

    // Verdant meadow & forest floor ground plane below horizon (0.80)
    if (uv.y > 0.80) {
        let depth_factor = smoothstep(0.80, 1.0, uv.y) * 0.68;
        let deep_meadow = mix(meadow_deep, meadow_front, depth_factor);
        col = mix(col, deep_meadow, smoothstep(0.80, 0.815, uv.y));
        if (is_grid_line) {
            col += grid_glow * 0.8;
        }
    }

    return vec4<f32>(col, 1.0);
}
