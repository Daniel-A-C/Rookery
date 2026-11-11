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
    var meadow = vec3<f32>(0.016, 0.052, 0.024);
    var grid_glow = vec3<f32>(0.014, 0.020, 0.028);

    switch palette {
        case 1: {
            // Midnight Starlight: obsidian sky, frosted sapphire horizon, luminescent teal glade
            zenith = vec3<f32>(0.008, 0.010, 0.018);
            horizon = vec3<f32>(0.022, 0.034, 0.054);
            meadow = vec3<f32>(0.010, 0.038, 0.042);
            grid_glow = vec3<f32>(0.012, 0.018, 0.028);
        }
        case 2: {
            // Autumn Hearth: warm dusky sky, amber horizon, golden-olive meadow
            zenith = vec3<f32>(0.028, 0.016, 0.022);
            horizon = vec3<f32>(0.068, 0.038, 0.030);
            meadow = vec3<f32>(0.042, 0.038, 0.015);
            grid_glow = vec3<f32>(0.022, 0.015, 0.012);
        }
        case 3: {
            // Verdant Rainforest: deep forest night, vivid emerald floor
            zenith = vec3<f32>(0.010, 0.026, 0.018);
            horizon = vec3<f32>(0.026, 0.062, 0.038);
            meadow = vec3<f32>(0.012, 0.065, 0.028);
            grid_glow = vec3<f32>(0.012, 0.025, 0.016);
        }
        case 4: {
            // Amber Phosphor CRT: warm monochrome amber terminal
            zenith = vec3<f32>(0.020, 0.012, 0.002);
            horizon = vec3<f32>(0.055, 0.034, 0.006);
            meadow = vec3<f32>(0.035, 0.022, 0.005);
            grid_glow = vec3<f32>(0.025, 0.015, 0.003);
        }
        case 5: {
            // Emerald Matrix: hacker green phosphor terminal
            zenith = vec3<f32>(0.004, 0.022, 0.008);
            horizon = vec3<f32>(0.012, 0.055, 0.018);
            meadow = vec3<f32>(0.006, 0.040, 0.014);
            grid_glow = vec3<f32>(0.006, 0.025, 0.010);
        }
        default: {
            // Earth & Sylvan (Caves of Qud): default
        }
    }

    var col = mix(zenith, horizon, smoothstep(0.0, 0.72, uv.y));

    if (is_grid_line) {
        col += grid_glow;
    }

    // Verdant meadow & forest floor ground plane below horizon (0.72)
    if (uv.y > 0.72) {
        let depth_factor = smoothstep(0.72, 1.0, uv.y);
        let deep_meadow = meadow * mix(0.85, 1.25, depth_factor);
        col = mix(col, deep_meadow, smoothstep(0.72, 0.735, uv.y));
        if (is_grid_line) {
            col += grid_glow * 0.8;
        }
    }

    return vec4<f32>(col, 1.0);
}
