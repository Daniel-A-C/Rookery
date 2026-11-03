// Instanced Signed Distance Field (SDF) glyph rendering shader.
// Renders overlapping characters with rotation, wind sway, and smooth vector anti-aliasing.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    wind_sway: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

@group(1) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var glyph_sampler: sampler;

struct VertexInput {
    @location(0) quad_pos: vec2<f32>, // [-0.5, -0.5] to [0.5, 0.5]
    @location(1) quad_uv: vec2<f32>,  // [0.0, 0.0] to [1.0, 1.0]
}

struct InstanceInput {
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) rotation: f32,
    @location(5) depth: f32,
    @location(6) uv_bounds: vec4<f32>, // [u0, v0, u1, v1]
    @location(7) color: vec4<f32>,
    @location(8) sway: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Wind sway calculation: sinusoidal wave propagating across space
    let sway_angle = sin(u.time * 1.6 + inst.pos.x * 0.008 + inst.pos.y * 0.004) * u.wind_sway * inst.sway * 0.12;
    let total_rot = inst.rotation + sway_angle;

    // Rotate quad vertex
    let cos_r = cos(total_rot);
    let sin_r = sin(total_rot);
    let rot_x = vertex.quad_pos.x * cos_r - vertex.quad_pos.y * sin_r;
    let rot_y = vertex.quad_pos.x * sin_r + vertex.quad_pos.y * cos_r;

    // Scale and translate to screen pixel coordinates
    let pixel_pos = inst.pos + vec2<f32>(rot_x * inst.size.x, rot_y * inst.size.y);

    // Convert pixel coordinates (top-left [0, 0]) to Normalized Device Coordinates [-1.0, 1.0]
    let ndc_x = (pixel_pos.x / u.resolution.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_pos.y / u.resolution.y) * 2.0;

    // Depth clamped in [0.0, 1.0]
    let depth_val = clamp(inst.depth * 0.001, 0.0, 0.999);
    out.position = vec4<f32>(ndc_x, ndc_y, depth_val, 1.0);

    // Interpolate UV within the glyph's atlas sub-rectangle
    let u_coord = mix(inst.uv_bounds.x, inst.uv_bounds.z, vertex.quad_uv.x);
    let v_coord = mix(inst.uv_bounds.y, inst.uv_bounds.w, vertex.quad_uv.y);
    out.uv = vec2<f32>(u_coord, v_coord);

    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = textureSample(glyph_atlas, glyph_sampler, in.uv).r;

    // Signed distance field edge detection:
    // 0.5 is the exact character boundary.
    // Use fwidth for resolution-independent antialiased edge smoothing.
    let edge = 0.5;
    let width = clamp(fwidth(dist) * 0.75, 0.02, 0.12);
    let alpha = smoothstep(edge - width, edge + width, dist) * in.color.a;

    if (alpha < 0.015) {
        discard;
    }

    return vec4<f32>(in.color.rgb, alpha);
}
