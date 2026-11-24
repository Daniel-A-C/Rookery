// Cosmic Nebula v2.0 — Pass 1: Volumetric JWST Stellar Nursery Backdrop Shader.
//
// Renders:
// - Deep-field twinkling background starfield + subtle distant spiral galaxies
// - 3-layer parallax domain-warped volumetric gas clouds & backlit molecular dust pillars
// - Real-time volumetric scattering from up to 16 live NurseryStars
// - Expanding luminous supernova shockwave rings (up to 6) with chromatic ionization fronts
// - Interactive cursor stir wakes (up to 12) warping local gas & leaving faint ionized trails
// - Twin rotating pulsar synchrotron lighthouse beams

struct BackdropUniforms {
    resolution_and_time: vec4<f32>, // (w, h, time, star_count)
    counts_and_flags: vec4<f32>,    // (shockwave_count, wake_count, pulsar_intensity, _pad)
    pulsar_data: vec4<f32>,         // (norm_x, norm_y, angle, pulse_phase)
    deep_space: vec4<f32>,
    gas_primary: vec4<f32>,
    gas_secondary: vec4<f32>,
    dust_dark: vec4<f32>,
    rim_glow: vec4<f32>,
    star_warm: vec4<f32>,
    star_cool: vec4<f32>,
    // (norm_x, norm_y, luminosity, temp_t)
    stars: array<vec4<f32>, 16>,
    // (norm_x, norm_y, radius_norm, fade)
    shockwaves: array<vec4<f32>, 6>,
    // (norm_x, norm_y, dir_x_packed, dir_y_packed) — strength*fade encoded in length of (z, w)
    wakes: array<vec4<f32>, 12>,
}

@group(0) @binding(0)
var<uniform> u: BackdropUniforms;

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

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    let r = q + dot(q, q + 34.53);
    return fract(r.x * r.y);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(n) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn fbm4(p_in: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var p = p_in;
    let rot = mat2x2<f32>(0.80, -0.60, 0.60, 0.80);
    for (var i = 0; i < 4; i++) {
        v += amp * vnoise(p);
        p = rot * p * 2.04 + vec2<f32>(4.1, 1.7);
        amp *= 0.5;
    }
    return v;
}

fn fbm3(p_in: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.52;
    var p = p_in;
    let rot = mat2x2<f32>(0.80, -0.60, 0.60, 0.80);
    for (var i = 0; i < 3; i++) {
        v += amp * vnoise(p);
        p = rot * p * 2.08 + vec2<f32>(2.3, 5.9);
        amp *= 0.5;
    }
    return v;
}

fn deep_star_layer(uv: vec2<f32>, aspect: f32, grid_scale: f32, threshold: f32, time: f32) -> vec3<f32> {
    let p = vec2<f32>(uv.x * aspect, uv.y) * grid_scale;
    let cell = floor(p);
    let h = hash21(cell);
    if (h < threshold) {
        return vec3<f32>(0.0);
    }
    let rnd = hash22(cell + 13.7);
    let star_pos = cell + 0.18 + rnd * 0.64;
    let d = length(p - star_pos);
    let twinkle = 0.62 + 0.38 * sin(time * (1.8 + rnd.x * 2.4) + h * 60.0);
    let core = exp(-d * d * 55.0) * twinkle;
    let col = mix(u.star_warm.rgb, u.star_cool.rgb, rnd.y);
    return mix(col, vec3<f32>(1.0), 0.35) * core * (0.45 + 0.55 * rnd.x);
}

fn distant_galaxy(p: vec2<f32>, center: vec2<f32>, angle: f32, scale: vec2<f32>, tint: vec3<f32>) -> vec3<f32> {
    let d = p - center;
    let ca = cos(angle);
    let sa = sin(angle);
    let q = vec2<f32>(d.x * ca - d.y * sa, d.x * sa + d.y * ca) / scale;
    let r2 = dot(q, q);
    if (r2 > 2.4) {
        return vec3<f32>(0.0);
    }
    let r = sqrt(r2);
    let theta = atan2(q.y, q.x);
    let spiral = 0.55 + 0.45 * sin(theta * 2.0 - r * 7.5);
    let disk = exp(-r2 * 3.8) * spiral * 0.45;
    let core = exp(-r2 * 28.0) * 0.9;
    return tint * (disk + core) * 0.55;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution_and_time.xy;
    let time = u.resolution_and_time.z;
    let aspect = res.x / max(res.y, 1.0);

    // Aspect-corrected centered coordinates
    let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);

    // 1. Cursor Sculpting / Stirring Wakes (deform local gas & add subtle plasma glow)
    var stir_warp = vec2<f32>(0.0);
    var stir_glow = 0.0;
    let wake_count = min(i32(u.counts_and_flags.y + 0.5), 12);
    for (var i = 0; i < wake_count; i++) {
        let w = u.wakes[i];
        let wp = vec2<f32>((w.x - 0.5) * aspect, w.y - 0.5);
        let d = p - wp;
        let d2 = dot(d, d);
        if (d2 < 0.08) {
            let falloff = exp(-d2 * 42.0);
            let strength = length(w.zw);
            stir_warp -= w.zw * falloff * 0.16;
            stir_glow += falloff * strength * 0.42;
        }
    }

    // 2. Deep-field background (void + subtle galaxies + 2-scale crisp starfield)
    var color = u.deep_space.rgb;
    color += distant_galaxy(p, vec2<f32>(-0.42 * aspect * 0.5, -0.26), 0.48, vec2<f32>(0.085, 0.032), u.star_warm.rgb);
    color += distant_galaxy(p, vec2<f32>(0.36 * aspect * 0.5, -0.30), -0.62, vec2<f32>(0.068, 0.028), u.star_cool.rgb);
    color += deep_star_layer(uv + vec2<f32>(time * 0.0005, 0.0), aspect, 28.0, 0.90, time) * 0.85;
    color += deep_star_layer(uv + vec2<f32>(time * 0.0011, 0.0003), aspect, 56.0, 0.93, time * 1.15) * 0.55;

    // 3. Multi-layer volumetric nebula clouds (domain-warped FBM)
    let drift = vec2<f32>(time * 0.014, -time * 0.008);
    let np = p * 2.15 + drift + stir_warp;

    let q = vec2<f32>(
        fbm3(np + vec2<f32>(0.0, 0.0)),
        fbm3(np + vec2<f32>(5.2, 1.3))
    );
    let r = vec2<f32>(
        fbm4(np + 2.4 * q + vec2<f32>(1.7, 9.2) + time * 0.012),
        fbm4(np + 2.4 * q + vec2<f32>(8.3, 2.8) - time * 0.009)
    );
    let veil = fbm4(np + 2.2 * r);
    let wisps = pow(clamp(fbm3(np * 1.85 + r * 1.6), 0.0, 1.0), 1.8);

    // Rich emission gas layering (calibrated in linear space for sRGB swapchain richness)
    let gas_col = mix(u.gas_primary.rgb, u.gas_secondary.rgb, smoothstep(0.32, 0.76, q.x));
    let gas_density = pow(smoothstep(0.30, 0.86, veil), 1.45);
    color += gas_col * gas_density * 0.24;
    color += mix(u.gas_secondary.rgb, u.rim_glow.rgb, 0.45) * wisps * gas_density * 0.18;

    // 4. JWST Cosmic Cliffs / Molecular Dust Pillars along lower & side horizons
    let cliff_wave = 0.22 * sin(p.x * 2.4 + 0.7) + 0.10 * cos(p.x * 5.2 - 0.4);
    let cliff_mask = smoothstep(-0.04 + cliff_wave, 0.48 + cliff_wave, p.y);
    let pillar_noise = fbm4(p * 3.1 + stir_warp * 1.5 + vec2<f32>(3.1, -time * 0.006) + q * 0.95);
    let dust_field = cliff_mask * smoothstep(0.36, 0.76, pillar_noise);

    // Finite-difference backlit ionization ridge along the upper edge of the dust cliffs!
    let pillar_up = fbm4((p + vec2<f32>(0.0, -0.022)) * 3.1 + stir_warp * 1.5 + vec2<f32>(3.1, -time * 0.006) + q * 0.95);
    let dust_up = smoothstep(-0.04 + cliff_wave, 0.48 + cliff_wave, p.y - 0.022) * smoothstep(0.36, 0.76, pillar_up);
    let rim_edge = clamp((dust_field - dust_up) * 4.8, 0.0, 1.0);

    // Attenuate background behind dense dark molecular dust, then paint luminous backlit rim
    color = mix(color, u.dust_dark.rgb * 0.35 + u.gas_secondary.rgb * 0.015, dust_field * 0.88);
    color += u.rim_glow.rgb * rim_edge * (0.28 + 0.30 * wisps);

    // 5. Real-time volumetric light scattering from live Nursery Stars
    let star_count = min(i32(u.resolution_and_time.w + 0.5), 16);
    for (var i = 0; i < star_count; i++) {
        let s = u.stars[i];
        let sp = vec2<f32>((s.x - 0.5) * aspect, s.y - 0.5);
        let d2 = dot(p - sp, p - sp);
        if (d2 < 0.24) {
            let scatter = exp(-d2 * 28.0) * s.z * (0.20 + 0.80 * gas_density);
            let s_col = mix(u.star_warm.rgb, u.star_cool.rgb, s.w);
            color += s_col * scatter * 0.14;
            // Extra rim illumination on nearby dust cliffs
            color += s_col * rim_edge * exp(-d2 * 16.0) * s.z * 0.22;
        }
    }

    // 6. Expanding Supernova Shockwaves (dual chromatic ionization front)
    let sw_count = min(i32(u.counts_and_flags.x + 0.5), 6);
    for (var i = 0; i < sw_count; i++) {
        let sw = u.shockwaves[i];
        let swp = vec2<f32>((sw.x - 0.5) * aspect, sw.y - 0.5);
        let d = length(p - swp);
        let ring_d = abs(d - sw.z);
        let inner_ring = exp(-ring_d * ring_d * 2600.0);
        let outer_halo = exp(-ring_d * ring_d * 300.0);
        let fade = sw.w;
        let front_col = mix(u.star_cool.rgb, u.rim_glow.rgb, smoothstep(-0.02, 0.03, d - sw.z));
        color += front_col * (inner_ring * 0.55 + outer_halo * 0.18 * (0.35 + gas_density)) * fade;
        // Soft flash near center early in detonation
        color += u.rim_glow.rgb * exp(-d * d * 32.0) * pow(fade, 3.0) * 0.35;
    }

    // 7. Pulsar Twin Synchrotron Lighthouse Beams
    let pulsar_intensity = u.counts_and_flags.z;
    if (pulsar_intensity > 0.01) {
        let pp = vec2<f32>((u.pulsar_data.x - 0.5) * aspect, u.pulsar_data.y - 0.5);
        let dp = p - pp;
        let dist = length(dp);
        if (dist > 0.002 && dist < 0.85) {
            let dir = dp / dist;
            let beam_dir = vec2<f32>(cos(u.pulsar_data.z), sin(u.pulsar_data.z));
            let align = abs(dot(dir, beam_dir));
            let cone = pow(clamp(align, 0.0, 1.0), 48.0);
            let atten = exp(-dist * 3.0);
            let pulse = 0.78 + 0.22 * sin(u.pulsar_data.w);
            color += u.star_cool.rgb * cone * atten * pulsar_intensity * pulse * (0.16 + 0.35 * gas_density);
        }
    }

    // 8. Interactive cursor stir ionized plasma shimmer
    color += mix(u.rim_glow.rgb, u.star_cool.rgb, 0.5) * stir_glow * (0.22 + 0.30 * wisps);

    // 9. Soft filmic tone mapping & subtle photographic vignette
    color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);
    let vig = 1.0 - smoothstep(0.50, 1.15, length(p));
    color *= mix(0.62, 1.0, vig);

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
