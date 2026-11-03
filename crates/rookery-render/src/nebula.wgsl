// Procedural Deep Space Nebula & Gravitational Field Shader.
// Multi-octave cosmic dust, ionization fronts, and gravitational vortex glow.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    palette_idx: f32,
    vortex_pos: vec2<f32>,
    turbulence: f32,
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

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    return fract(dot(q, q + 45.32));
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        val += amp * noise(pos);
        pos = pos * 2.1 + vec2<f32>(1.7, 9.2);
        amp *= 0.5;
    }
    return val;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = u.time * 0.08;

    // Palette colors: 0=StellarGold, 1=DeepAmethyst, 2=SolarFlare, 3=AuroraVeil
    var col_deep = vec3<f32>(0.015, 0.02, 0.04);
    var col_mid = vec3<f32>(0.08, 0.22, 0.38);
    var col_highlight = vec3<f32>(0.95, 0.82, 0.35);

    let pal = i32(u.palette_idx + 0.5) % 4;
    if (pal == 1) {
        // Deep Amethyst
        col_deep = vec3<f32>(0.02, 0.01, 0.04);
        col_mid = vec3<f32>(0.28, 0.08, 0.42);
        col_highlight = vec3<f32>(0.92, 0.45, 0.88);
    } else if (pal == 2) {
        // Solar Flare
        col_deep = vec3<f32>(0.04, 0.015, 0.01);
        col_mid = vec3<f32>(0.45, 0.12, 0.06);
        col_highlight = vec3<f32>(0.98, 0.75, 0.22);
    } else if (pal == 3) {
        // Aurora Veil
        col_deep = vec3<f32>(0.01, 0.03, 0.035);
        col_mid = vec3<f32>(0.08, 0.36, 0.32);
        col_highlight = vec3<f32>(0.45, 0.95, 0.75);
    }

    // Swirling cosmic nebular dust
    let p = uv * 3.2 * u.turbulence + vec2<f32>(cos(t), sin(t)) * 0.4;
    let q = vec2<f32>(fbm(p + t * 0.5), fbm(p + vec2<f32>(5.2, 1.3)));
    let r = vec2<f32>(fbm(p + 4.0 * q + vec2<f32>(1.7, 9.2)), fbm(p + 4.0 * q + vec2<f32>(8.3, 2.8)));
    let neb = fbm(p + 4.0 * r);

    var color = mix(col_deep, col_mid, smoothstep(0.1, 0.65, neb));
    color = mix(color, col_highlight, pow(neb, 3.0) * 0.85);

    // Gravitational vortex center glow
    let v_dist = distance(uv * u.resolution, u.vortex_pos);
    let vortex_glow = smoothstep(180.0, 0.0, v_dist);
    color += col_highlight * vortex_glow * 0.35;

    // Distant background star points
    let star_grid = floor(uv * u.resolution / 32.0);
    let star_h = hash(star_grid);
    if (star_h > 0.96) {
        let star_pos = (star_grid + vec2<f32>(hash(star_grid + 1.0), hash(star_grid + 2.0))) * 32.0 / u.resolution;
        let d = length((uv - star_pos) * u.resolution);
        let twinkle = 0.5 + 0.5 * sin(u.time * 2.5 + star_h * 50.0);
        let s_alpha = smoothstep(1.5, 0.0, d) * twinkle * 0.8;
        color += vec3<f32>(0.95, 0.98, 1.0) * s_alpha;
    }

    return vec4<f32>(color, 1.0);
}
