// Fullscreen atmospheric water background for Tranquil Koi Pond.
// Procedural caustics, gentle depth gradients, and soft water ripples.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _pad: f32,
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

// Procedural water caustics function
fn caustics(p: vec2<f32>, t: f32) -> f32 {
    var pos = p * 4.5;
    var c = 0.0;
    for (var i = 1; i <= 3; i++) {
        let fi = f32(i);
        pos += vec2<f32>(
            sin(pos.y * 1.5 + t * 0.8 * fi + fi * 1.2),
            cos(pos.x * 1.5 + t * 0.8 * fi + fi * 0.8)
        ) * 0.35;
        c += abs(sin(pos.x + pos.y));
    }
    return pow(c / 3.0, 3.2);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = u.time * 0.6;

    // Deep pond water palette: deep turquoise, jade emerald, and sunlit surface
    let deep_col = vec3<f32>(0.02, 0.08, 0.12);      // deep pond depth
    let mid_col = vec3<f32>(0.04, 0.16, 0.20);       // serene jade green
    let surface_col = vec3<f32>(0.08, 0.26, 0.28);   // sunlit shallows

    var water = mix(deep_col, mid_col, smoothstep(0.0, 0.7, uv.y));
    water = mix(water, surface_col, smoothstep(0.7, 1.0, uv.y) * 0.5);

    // Caustics shimmer
    let c = caustics(uv, t);
    let caustic_color = vec3<f32>(0.15, 0.45, 0.42);
    water += caustic_color * c * 0.45;

    // Soft pond edge vignette
    let d = distance(uv, vec2<f32>(0.5, 0.5));
    let vignette = smoothstep(0.75, 0.2, d);
    water *= (0.75 + 0.25 * vignette);

    return vec4<f32>(water, 1.0);
}

