// Atmospheric background shader for The Living ASCII Rookery.
// Soft dusk/twilight sky gradient, twinkling stars, and gentle horizon ground.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    time_of_day: f32,
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
    // Fullscreen triangle
    let x = f32(i32(vid << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vid & 2u) * -2) + 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

// Pseudo-random hash for procedural stars
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let d = dot(q, q + 45.32);
    return fract(d);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;

    // Sky colors: deep midnight twilight fading to warm horizon glow
    let sky_top = vec3<f32>(0.035, 0.045, 0.08);     // deep twilight navy
    let sky_mid = vec3<f32>(0.07, 0.09, 0.14);       // twilight slate
    let sky_horizon = vec3<f32>(0.12, 0.13, 0.16);   // soft horizon dusk

    var sky = mix(sky_top, sky_mid, smoothstep(0.0, 0.65, uv.y));
    sky = mix(sky, sky_horizon, smoothstep(0.65, 0.95, uv.y));

    // Subtle stars in the upper half of the sky
    if (uv.y < 0.6) {
        let grid_size = 40.0;
        let cell = floor(uv * res / grid_size);
        let star_rnd = hash21(cell);
        if (star_rnd > 0.96) {
            let star_pos = (cell + vec2<f32>(hash21(cell + 1.0), hash21(cell + 2.0))) * grid_size / res;
            let d = length((uv - star_pos) * res);
            let twinkle = 0.6 + 0.4 * sin(u.time * (1.5 + star_rnd * 3.0) + star_rnd * 100.0);
            let star_alpha = smoothstep(1.5, 0.0, d) * twinkle * (1.0 - uv.y / 0.6) * 0.45;
            sky += vec3<f32>(0.85, 0.9, 1.0) * star_alpha;
        }
    }

    // Gentle ground silhouette at the bottom (y > 0.90)
    let ground_y = 0.90 + 0.015 * sin(uv.x * 4.0 + 1.0) + 0.008 * sin(uv.x * 12.0);
    if (uv.y > ground_y) {
        let ground_col = vec3<f32>(0.025, 0.03, 0.035);
        let ground_blend = smoothstep(ground_y, ground_y + 0.02, uv.y);
        sky = mix(sky, ground_col, ground_blend * 0.85);
    }

    return vec4<f32>(sky, 1.0);
}
