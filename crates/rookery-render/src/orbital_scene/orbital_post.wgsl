// Orbital Haven v3 — CCTV Multi-Feed Compositor & Y2K Post-Processing Shader
//
// Applies subtle 4x4 ordered Bayer dithering, faint scanlines, dark gunmetal monitor
// bezels between PiP / 2x2 Grid feeds, and a brief static/glitch tear on camera cuts.

struct PostUniforms {
    // xy = resolution in pixels, z = time, w = glitch_intensity [0..1]
    res_time_glitch: vec4<f32>,
    // x = layout_mode (0 = Single, 1 = Pip, 2 = Grid2x2), yzw = accent RGB
    layout_and_accent: vec4<f32>,
    // PiP rect 0 in normalized [0..1] UV space: (x, y, w, h)
    pip0: vec4<f32>,
    // PiP rect 1 in normalized [0..1] UV space: (x, y, w, h)
    pip1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: PostUniforms;

@group(0) @binding(1)
var scene_tex: texture_2d<f32>;

@group(0) @binding(2)
var scene_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((idx << 1u) & 2u);
    let y = f32(idx & 2u);
    out.clip_pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let d = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * d);
}

fn bayer4(px: vec2<i32>) -> f32 {
    let x = px.x & 3;
    let y = px.y & 3;
    let idx = y * 4 + x;
    var m: f32 = 0.0;
    switch idx {
        case 0:  { m = 0.0; }
        case 1:  { m = 8.0; }
        case 2:  { m = 2.0; }
        case 3:  { m = 10.0; }
        case 4:  { m = 12.0; }
        case 5:  { m = 4.0; }
        case 6:  { m = 14.0; }
        case 7:  { m = 6.0; }
        case 8:  { m = 3.0; }
        case 9:  { m = 11.0; }
        case 10: { m = 1.0; }
        case 11: { m = 9.0; }
        case 12: { m = 15.0; }
        case 13: { m = 7.0; }
        case 14: { m = 13.0; }
        default: { m = 5.0; }
    }
    return (m / 16.0) - 0.5;
}

fn bezel_factor(uv: vec2<f32>, rect: vec4<f32>, border_px: vec2<f32>) -> vec2<f32> {
    // Returns (is_outer_bezel, is_accent_hairline)
    let inside = uv.x >= rect.x && uv.x <= rect.x + rect.z && uv.y >= rect.y && uv.y <= rect.y + rect.w;
    if (!inside) {
        return vec2<f32>(0.0, 0.0);
    }
    let dx = min(uv.x - rect.x, rect.x + rect.z - uv.x);
    let dy = min(uv.y - rect.y, rect.y + rect.w - uv.y);
    let outer = (dx < border_px.x * 3.0) || (dy < border_px.y * 3.0);
    let accent = (dx >= border_px.x * 2.0 && dx < border_px.x * 3.5)
        || (dy >= border_px.y * 2.0 && dy < border_px.y * 3.5);
    return vec2<f32>(select(0.0, 1.0, outer), select(0.0, 1.0, accent));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let res = max(u.res_time_glitch.xy, vec2<f32>(1.0, 1.0));
    let t = u.res_time_glitch.z;
    let glitch = clamp(u.res_time_glitch.w, 0.0, 1.0);
    let px = in.uv * res;
    let px_i = vec2<i32>(px);

    // Camera-switch static horizontal tear & chromatic offset
    var sample_uv = in.uv;
    if (glitch > 0.01) {
        let band = floor(in.uv.y * 42.0);
        let tear = (hash12(vec2<f32>(band, floor(t * 60.0))) - 0.5) * 0.045 * glitch;
        sample_uv.x = clamp(sample_uv.x + tear, 0.0, 1.0);
    }

    let chroma = 0.0008 + 0.004 * glitch;
    let r = textureSample(scene_tex, scene_sampler, clamp(sample_uv + vec2<f32>(chroma, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).r;
    let g = textureSample(scene_tex, scene_sampler, sample_uv).g;
    let b = textureSample(scene_tex, scene_sampler, clamp(sample_uv - vec2<f32>(chroma, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).b;
    var rgb = vec3<f32>(r, g, b);

    // Subtle 4x4 ordered Bayer dithering + gentle 24-level retro color quantization feel
    let dither = bayer4(px_i) * (1.0 / 38.0);
    rgb = clamp(rgb + vec3<f32>(dither), vec3<f32>(0.0), vec3<f32>(1.0));

    // Faint CCTV horizontal scanlines
    let scan = 0.965 + 0.035 * sin(px.y * 3.14159);
    rgb = rgb * scan;

    // Static snow burst on camera switch
    if (glitch > 0.01) {
        let snow = hash12(px * 0.35 + vec2<f32>(t * 173.0, t * 91.0));
        rgb = mix(rgb, vec3<f32>(snow * 0.85), glitch * 0.45);
    }

    // Subtle per-screen vignette
    let centered = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let vig = 1.0 - dot(centered, centered) * 0.16;
    rgb = rgb * clamp(vig, 0.65, 1.0);

    // Monitor Bezels & Accent Hairlines for PiP and 2x2 Grid modes
    let layout_mode = i32(u.layout_and_accent.x + 0.5);
    let accent = u.layout_and_accent.yzw;
    let px_step = 1.0 / res;

    if (layout_mode == 1) {
        // PiP monitor frames
        let b0 = bezel_factor(in.uv, u.pip0, px_step);
        let b1 = bezel_factor(in.uv, u.pip1, px_step);
        if (b0.x > 0.5 || b1.x > 0.5) {
            rgb = vec3<f32>(0.04, 0.05, 0.07);
        }
        if (b0.y > 0.5 || b1.y > 0.5) {
            rgb = mix(rgb, accent, 0.72);
        }
    } else if (layout_mode == 2) {
        // 2x2 CCTV Wall divider crosshair & outer edge
        let mid_x = abs(in.uv.x - 0.5) < px_step.x * 2.5;
        let mid_y = abs(in.uv.y - 0.5) < px_step.y * 2.5;
        let edge = in.uv.x < px_step.x * 2.0 || in.uv.x > 1.0 - px_step.x * 2.0
            || in.uv.y < px_step.y * 2.0 || in.uv.y > 1.0 - px_step.y * 2.0;
        if (mid_x || mid_y || edge) {
            rgb = vec3<f32>(0.03, 0.04, 0.06);
        }
        let accent_x = abs(in.uv.x - 0.5) >= px_step.x * 2.5 && abs(in.uv.x - 0.5) < px_step.x * 3.5;
        let accent_y = abs(in.uv.y - 0.5) >= px_step.y * 2.5 && abs(in.uv.y - 0.5) < px_step.y * 3.5;
        if (accent_x || accent_y) {
            rgb = mix(rgb, accent, 0.45);
        }
    }

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
