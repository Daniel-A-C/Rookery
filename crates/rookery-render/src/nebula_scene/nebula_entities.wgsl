// Cosmic Nebula v2.0 — Pass 2: Painterly Space Entity Mesh Shader.
//
// Uses premultiplied alpha blending (src = One, dst = OneMinusSrcAlpha):
// - Luminous additive entities (stars, protostars, dust motes, comets, meteors) output alpha = 0.0
//   for pure HDR-like additive glow without dark fringes.
// - Astral Cartography lines output subtle alpha > 0.0 for crisp readability over bright clouds.

struct EntityUniforms {
    resolution_and_time: vec4<f32>, // (w, h, time, _pad)
}

@group(0) @binding(0)
var<uniform> u: EntityUniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) kind: u32,
    @location(4) param: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) kind: u32,
    @location(3) param: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x = (in.pos.x / u.resolution_and_time.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.pos.y / u.resolution_and_time.y) * 2.0;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    out.kind = in.kind;
    out.param = in.param;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let col = in.color.rgb;
    let intensity = in.color.a;
    let time = u.resolution_and_time.z;

    // Kind 0: JWST 6-point hexagonal diffraction-spiked star
    if (in.kind == 0u) {
        let r = length(uv);
        if (r > 1.0) {
            discard;
        }
        let core = exp(-r * r * 68.0) * 1.45;
        let corona = exp(-r * 6.5) * 0.52;

        // 3 axes separated by 60 degrees (classic JWST hexagonal mirror spikes)
        let d0 = abs(uv.y);
        let d1 = abs(uv.x * 0.8660254 - uv.y * 0.5);
        let d2 = abs(uv.x * 0.8660254 + uv.y * 0.5);
        let spike_falloff = exp(-r * 3.4) * smoothstep(1.0, 0.15, r);
        let s0 = exp(-d0 * 52.0);
        let s1 = exp(-d1 * 52.0);
        let s2 = exp(-d2 * 52.0);
        let spikes = (s0 + s1 + s2) * spike_falloff * (0.34 + 0.18 * clamp(in.param, 0.0, 2.0));

        let hot_core = vec3<f32>(1.0, 0.99, 0.97) * core;
        let glow = col * (corona + spikes);
        return vec4<f32>((hot_core + glow) * intensity, 0.0);
    }

    // Kind 1: Collapsing Protostar with swirling accretion ring
    if (in.kind == 1u) {
        let r = length(uv);
        if (r > 1.0) {
            discard;
        }
        let collapse = clamp(in.param, 0.0, 1.0);
        let angle = atan2(uv.y, uv.x);
        let ring_r = mix(0.58, 0.16, collapse);
        let ring_band = exp(-pow((r - ring_r) * 10.5, 2.0));
        let spiral = 0.55 + 0.45 * sin(angle * 3.0 - time * 4.5 + r * 9.0);
        let pinch_core = exp(-r * r * mix(22.0, 75.0, collapse)) * (0.65 + 0.9 * collapse);
        let halo = exp(-r * 4.2) * 0.35;

        let rgb = vec3<f32>(1.0, 0.96, 0.86) * pinch_core + col * (ring_band * spiral * 0.85 + halo);
        return vec4<f32>(rgb * intensity, 0.0);
    }

    // Kind 2: Soft luminous stardust mote
    if (in.kind == 2u) {
        let r2 = dot(uv, uv);
        if (r2 > 1.0) {
            discard;
        }
        let g = exp(-r2 * 5.2) * (1.0 - r2);
        return vec4<f32>(col * g * intensity, 0.0);
    }

    // Kind 3: Comet tail / Meteor streak / Supernova filament ribbon
    if (in.kind == 3u) {
        let along = clamp(uv.x, 0.0, 1.0);
        let across = abs(uv.y);
        // param > 0.5: closed continuous shockwave filament (no longitudinal taper)
        if (in.param > 0.5) {
            let trans = exp(-across * across * 6.5) * smoothstep(1.0, 0.1, across);
            let shimmer = 0.78 + 0.22 * sin(along * 31.4159 - time * 3.5);
            return vec4<f32>(col * trans * shimmer * intensity, 0.0);
        }
        let width_taper = mix(1.0, 0.25, along);
        let trans = exp(-pow(across / max(width_taper, 0.08), 2.0) * 5.5);
        let longi = pow(1.0 - along, 1.35) * smoothstep(0.0, 0.06, along + 0.04);
        let shimmer = 0.85 + 0.15 * sin(along * 14.0 - time * 5.0);
        return vec4<f32>(col * trans * longi * shimmer * intensity, 0.0);
    }

    // Kind 4: Astral Cartography constellation line / astrolabe reticle
    let across = abs(uv.y);
    let line_alpha = exp(-across * across * 6.5) * smoothstep(1.0, 0.1, across);
    var dash = 1.0;
    if (in.param > 0.5) {
        dash = smoothstep(0.25, 0.45, fract(uv.x * 2.0));
    }
    let a = clamp(line_alpha * dash * intensity, 0.0, 1.0);
    return vec4<f32>(col * a * 1.15, a * 0.35);
}
