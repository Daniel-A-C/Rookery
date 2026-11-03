// Rookery demo scene: slow aurora bands over a night-sky gradient, plus additive motes.
// Colors are authored in sRGB and converted to linear (the surface is an sRGB format).

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    gust: f32,       // 0..1 envelope while a gust is blowing
};

@group(0) @binding(0) var<uniform> u: Uniforms;

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

// ---------- Aurora background (full-screen triangle) ----------

struct BgOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> BgOut {
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: BgOut;
    out.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), s.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), s.x),
        s.y
    );
}

fn band(uv: vec2<f32>, t: f32, offset: f32, freq: f32) -> f32 {
    let wave = 0.5 + 0.12 * sin(uv.x * freq + t * 0.05 + offset)
             + 0.06 * sin(uv.x * freq * 2.3 - t * 0.031 + offset * 1.7)
             + 0.08 * (noise(vec2<f32>(uv.x * 3.0 + offset, t * 0.02)) - 0.5);
    let d = abs(uv.y - wave + offset * 0.08);
    return exp(-d * d * 180.0);
}

@fragment
fn fs_main(in: BgOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let aspect = u.resolution.x / max(u.resolution.y, 1.0);
    let t = u.time;

    // Night-sky vertical gradient.
    let top = vec3<f32>(0.020, 0.035, 0.080);
    let bottom = vec3<f32>(0.060, 0.090, 0.150);
    var col = mix(bottom, top, uv.y);

    // Aurora bands.
    let p = vec2<f32>(uv.x * aspect * 0.6, uv.y);
    let a = band(p, t, 0.0, 2.1);
    let b = band(p, t, 1.3, 1.6);
    let c = band(p, t, 2.6, 2.7);
    let shimmer = 0.75 + 0.25 * noise(vec2<f32>(uv.x * 40.0, t * 0.3));
    col += vec3<f32>(0.10, 0.85, 0.55) * a * 0.55 * shimmer;
    col += vec3<f32>(0.25, 0.55, 0.95) * b * 0.40 * shimmer;
    col += vec3<f32>(0.70, 0.35, 0.85) * c * 0.25 * shimmer;

    // Subtle brightening during a gust.
    col *= 1.0 + 0.15 * u.gust;

    // Sparse static stars.
    let star_cell = floor(in.pos.xy / 3.0);
    let star = step(0.9985, hash(star_cell)) * (0.5 + 0.5 * sin(t * 0.7 + hash(star_cell + 7.0) * 6.28));
    col += vec3<f32>(star * 0.6);

    return vec4<f32>(to_linear(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}

// ---------- Motes (instanced quads, additive) ----------

struct MoteIn {
    @location(0) center: vec2<f32>,   // pixels, top-left origin
    @location(1) radius: f32,         // pixels
    @location(2) alpha: f32,
};

struct MoteOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_mote(@builtin(vertex_index) vi: u32, m: MoteIn) -> MoteOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0)
    );
    let local = corners[vi];
    let px = m.center + local * m.radius;
    let ndc = vec2<f32>(px.x / u.resolution.x * 2.0 - 1.0, 1.0 - px.y / u.resolution.y * 2.0);
    var out: MoteOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.local = local;
    out.alpha = m.alpha;
    return out;
}

@fragment
fn fs_mote(in: MoteOut) -> @location(0) vec4<f32> {
    let d = length(in.local);
    let glow = exp(-d * d * 4.0) * in.alpha;
    let tint = to_linear(vec3<f32>(0.85, 0.95, 1.0));
    return vec4<f32>(tint * glow, glow);
}

