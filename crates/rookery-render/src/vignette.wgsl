// Direct Interaction Mode indicator: soft edge vignette + pulsing corner badge.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    scale: f32,
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
    // Edge vignette (warm tint, so it's clearly "different" from the passive look).
    let edge = min(min(in.uv.x, 1.0 - in.uv.x), min(in.uv.y, 1.0 - in.uv.y));
    let vignette = 1.0 - smoothstep(0.0, 0.06, edge);
    let tint = pow(vec3<f32>(1.0, 0.78, 0.45), vec3<f32>(2.2));

    // Corner badge: pulsing dot in the top-right corner.
    let badge_center = vec2<f32>(u.resolution.x - 28.0 * u.scale, 28.0 * u.scale);
    let d = length(in.pos.xy - badge_center) / u.scale;
    let pulse = 0.75 + 0.25 * sin(u.time * 3.0);
    let dot = (1.0 - smoothstep(6.0, 8.0, d)) * pulse;
    let ring = (1.0 - smoothstep(0.0, 1.5, abs(d - 12.0))) * 0.5 * pulse;

    let alpha = clamp(vignette * 0.35 + dot + ring, 0.0, 1.0);
    return vec4<f32>(tint * alpha, alpha);
}

