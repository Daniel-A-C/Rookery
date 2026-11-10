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

    let palette = i32(round(u.palette_id)) % 6;

    var zenith = vec3<f32>(0.016, 0.024, 0.038);
    var horizon = vec3<f32>(0.042, 0.056, 0.078);
    var soil = vec3<f32>(0.045, 0.035, 0.028);
    var grid_glow = vec3<f32>(0.014, 0.020, 0.028);

    switch palette {
        case 1: {
            // Midnight Starlight: obsidian sky, frosted sapphire horizon
            zenith = vec3<f32>(0.008, 0.010, 0.018);
            horizon = vec3<f32>(0.022, 0.034, 0.054);
            soil = vec3<f32>(0.022, 0.028, 0.038);
            grid_glow = vec3<f32>(0.012, 0.018, 0.028);
        }
        case 2: {
            // Autumn Hearth: warm dusky sky, amber horizon
            zenith = vec3<f32>(0.028, 0.016, 0.022);
            horizon = vec3<f32>(0.068, 0.038, 0.030);
            soil = vec3<f32>(0.055, 0.032, 0.020);
            grid_glow = vec3<f32>(0.022, 0.015, 0.012);
        }
        case 3: {
            // Verdant Rainforest: deep forest night
            zenith = vec3<f32>(0.010, 0.026, 0.018);
            horizon = vec3<f32>(0.026, 0.062, 0.038);
            soil = vec3<f32>(0.032, 0.042, 0.024);
            grid_glow = vec3<f32>(0.012, 0.025, 0.016);
        }
        case 4: {
            // Amber Phosphor CRT: warm monochrome amber terminal
            zenith = vec3<f32>(0.020, 0.012, 0.002);
            horizon = vec3<f32>(0.055, 0.034, 0.006);
            soil = vec3<f32>(0.042, 0.024, 0.004);
            grid_glow = vec3<f32>(0.025, 0.015, 0.003);
        }
        case 5: {
            // Emerald Matrix: hacker green phosphor terminal
            zenith = vec3<f32>(0.004, 0.022, 0.008);
            horizon = vec3<f32>(0.012, 0.055, 0.018);
            soil = vec3<f32>(0.008, 0.035, 0.012);
            grid_glow = vec3<f32>(0.006, 0.025, 0.010);
        }
        default: {
            // Earth & Sylvan (Caves of Qud): default
        }
    }

    var col = mix(zenith, horizon, smoothstep(0.0, 0.88, uv.y));

    if (is_grid_line) {
        col += grid_glow;
    }

    // Subterranean soil strata below ground row
    if (uv.y > 0.88) {
        col = mix(col, soil, smoothstep(0.88, 0.89, uv.y));
        if (is_grid_line) {
            col += grid_glow * 0.7;
        }
    }

    return vec4<f32>(col, 1.0);
}
