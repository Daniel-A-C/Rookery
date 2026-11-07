// Direct Interaction Mode indicator: subtle theme-colored screen outline.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    scale: f32,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Distance from the nearest display edge in logical points.
    let edge_px = min(
        min(in.pos.x, u.resolution.x - in.pos.x),
        min(in.pos.y, u.resolution.y - in.pos.y),
    ) / max(u.scale, 1.0);

    // Slim 2px border matching the popover menu outline, with a tight 8px inner fade.
    let border = 1.0 - smoothstep(1.5, 2.5, edge_px);
    let glow = 1.0 - smoothstep(0.0, 8.0, edge_px);
    let tint = pow(u.color.rgb, vec3<f32>(2.2));

    let alpha = clamp(border * 0.8 + glow * 0.2, 0.0, 1.0);
    return vec4<f32>(tint * alpha, alpha);
}

