// Pass 1: Painterly Japanese Garden Koi Pond — Riverbed Pebbles, Refracted Ripples,
// Dual-Octave Caustics, Dappled Canopy Light, and Lantern / Moon Reflections.

struct WaterUniforms {
    resolution_and_time: vec4<f32>,    // xy = resolution, z = time, w = rain_intensity
    water_deep: vec4<f32>,             // rgb + vignette_strength
    water_mid: vec4<f32>,              // rgb + dapple_strength
    water_shallow: vec4<f32>,          // rgb + lantern_glow
    caustic_color: vec4<f32>,          // rgb + ambient_mul
    pebble_tint: vec4<f32>,            // rgb + active_ripples
    pebble_moss: vec4<f32>,            // rgb + active_lanterns
    sun_tint: vec4<f32>,               // rgb + pad
    ripples: array<vec4<f32>, 32>,     // xy = world pos (0..1000, 0..750), z = radius, w = amplitude
    lanterns: array<vec4<f32>, 6>,     // xy = world pos, z = radius, w = flicker_intensity
}

@group(0) @binding(0)
var<uniform> u: WaterUniforms;

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

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q) * 43758.5453);
}

fn hash1(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash1(i);
    let b = hash1(i + vec2<f32>(1.0, 0.0));
    let c = hash1(i + vec2<f32>(0.0, 1.0));
    let d = hash1(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 3; i++) {
        v += a * value_noise(q);
        q = q * 2.03 + vec2<f32>(17.3, 9.2);
        a *= 0.5;
    }
    return v;
}

// Voronoi riverbed pebbles: returns (rounded_height, crevice_mask, stone_id)
fn riverbed_pebbles(p: vec2<f32>) -> vec3<f32> {
    let n = floor(p);
    let f = fract(p);
    var md1 = 8.0;
    var md2 = 8.0;
    var id = 0.0;

    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = hash2(n + g);
            let r = g + o * 0.78 + 0.11 - f;
            let d = dot(r, r);
            if (d < md1) {
                md2 = md1;
                md1 = d;
                id = hash1(n + g);
            } else if (d < md2) {
                md2 = d;
            }
        }
    }
    let edge = sqrt(md2) - sqrt(md1);
    let dome = smoothstep(0.02, 0.28, edge) * (1.0 - smoothstep(0.0, 0.55, md1) * 0.45);
    return vec3<f32>(dome, smoothstep(0.01, 0.14, edge), id);
}

// Soft painterly dual-layer water caustics
fn caustics(p: vec2<f32>, t: f32) -> f32 {
    var pos = p * 3.8;
    var acc = 0.0;
    for (var i = 1; i <= 3; i++) {
        let fi = f32(i);
        pos += vec2<f32>(
            sin(pos.y * 1.45 + t * 0.52 * fi + fi * 1.3),
            cos(pos.x * 1.45 - t * 0.46 * fi + fi * 0.9)
        ) * 0.36;
        let wave = abs(sin(pos.x + pos.y));
        acc += pow(1.0 - wave, 2.2);
    }
    return clamp(acc / 2.6, 0.0, 1.4);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let res = max(u.resolution_and_time.xy, vec2<f32>(1.0, 1.0));
    let t = u.resolution_and_time.z;
    let rain = u.resolution_and_time.w;
    let aspect = res.x / res.y;

    let uv = in.uv;
    let world_pos = vec2<f32>(uv.x * 1000.0, uv.y * 750.0);

    // 1. Compute analytic water ripple refraction offset + specular crest
    var refract_offset = vec2<f32>(0.0, 0.0);
    var ripple_highlight = 0.0;
    let ripple_count = i32(u.pebble_tint.w + 0.5);
    for (var i = 0; i < 32; i++) {
        if (i >= ripple_count) {
            break;
        }
        let rp = u.ripples[i];
        let delta = world_pos - rp.xy;
        let dist = length(delta);
        let ring_d = dist - rp.z;
        if (abs(ring_d) < 38.0 && dist > 0.5) {
            let env = exp(-(ring_d * ring_d) / 180.0) * rp.w;
            let wave = sin(ring_d * 0.34);
            let dir = delta / dist;
            refract_offset += dir * wave * env * 0.0085;
            ripple_highlight += max(0.0, cos(ring_d * 0.34)) * env;
        }
    }

    // Gentle ambient water surface sway
    let gentle_sway = vec2<f32>(
        sin(uv.y * 7.5 + t * 0.75) * 0.0022,
        cos(uv.x * 6.5 - t * 0.65) * 0.0022
    );
    let refracted_uv = uv + refract_offset + gentle_sway;
    let aspect_uv = vec2<f32>(refracted_uv.x * aspect, refracted_uv.y);

    // 2. Pond depth basin: shallow mossy banks near edges, deeper tranquil pool in center
    let center_dist = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.08, 1.0));
    let depth = 1.0 - smoothstep(0.16, 0.72, center_dist);

    // 3. Soft organic riverbed pebbles (veiled by water depth, never harsh tile cracks)
    let big_peb = riverbed_pebbles(aspect_uv * 7.5 + fbm(aspect_uv * 3.0) * 0.45);
    let small_peb = riverbed_pebbles(aspect_uv * 16.5 + vec2<f32>(4.2, 7.8));
    let peb_mix = mix(small_peb.x, big_peb.x, 0.65);
    let peb_crevice = mix(0.76, 1.0, mix(small_peb.y, big_peb.y, 0.6));
    let stone_var = mix(0.84, 1.18, big_peb.z);

    // Warm mineral variation + soft underwater moss patches
    let moss_patch = smoothstep(0.38, 0.76, fbm(aspect_uv * 3.6 + vec2<f32>(1.7, 3.1)));
    var bed_col = u.pebble_tint.rgb * stone_var * (0.64 + 0.36 * peb_mix);
    bed_col = mix(bed_col, u.pebble_moss.rgb * (0.65 + 0.35 * peb_mix), moss_patch * 0.62);
    bed_col *= peb_crevice;

    // 4. Rich water column absorption over riverbed
    var water_col = mix(u.water_shallow.rgb, u.water_mid.rgb, smoothstep(0.0, 0.52, depth));
    water_col = mix(water_col, u.water_deep.rgb, smoothstep(0.42, 1.0, depth));

    // Pebbles show softly in the shallow margins and dissolve into deep jewel water at center
    let bed_visibility = mix(0.40, 0.11, depth) * (1.0 - rain * 0.16);
    var col = mix(water_col, bed_col * (0.58 + 0.42 * u.caustic_color.w), bed_visibility);

    // 5. Domain-warped painterly caustics projected onto the riverbed
    let c1 = caustics(aspect_uv + refract_offset * 4.5, t * 0.78);
    let c2 = caustics(aspect_uv * 1.45 - vec2<f32>(t * 0.03, -t * 0.02), t * 0.55 + 2.1);
    let caustic_net = pow(clamp(c1 * 0.62 + c2 * 0.38, 0.0, 1.4), 1.35) * mix(1.0, 0.52, depth) * (1.0 - rain * 0.30);
    col += u.caustic_color.rgb * caustic_net * 0.24;

    // 6. Dappled overhanging tree-canopy sunlight / moonlight shafts
    let canopy_uv = aspect_uv * 2.2 + vec2<f32>(sin(t * 0.22) * 0.14, cos(t * 0.18) * 0.10);
    let canopy = fbm(canopy_uv);
    let sun_dapple = smoothstep(0.48, 0.82, canopy);
    col += u.sun_tint.rgb * sun_dapple * u.water_mid.w * 0.18;
    // Soft canopy leaf shadow framing corners
    let corner_shade = smoothstep(0.32, 0.74, 1.0 - canopy) * smoothstep(0.22, 0.78, center_dist);
    col *= (1.0 - corner_shade * 0.24);

    // 7. Subtle ripple specular glint on the water surface
    col += u.sun_tint.rgb * clamp(ripple_highlight, 0.0, 1.2) * 0.16;

    // 8. Warm floating tōrō lantern reflections shimmering intimately on the water
    let lantern_glow = u.water_shallow.w;
    let lantern_count = i32(u.pebble_moss.w + 0.5);
    if (lantern_glow > 0.01 && lantern_count > 0) {
        let shimmer = 0.88 + 0.12 * sin(aspect_uv.x * 42.0 + t * 3.0) * cos(aspect_uv.y * 36.0 - t * 2.4);
        for (var i = 0; i < 6; i++) {
            if (i >= lantern_count) {
                break;
            }
            let lt = u.lanterns[i];
            let d = length((world_pos + refract_offset * 420.0) - lt.xy);
            let tight_halo = exp(-(d * d) / (lt.z * lt.z * 0.55)) * lt.w * lantern_glow;
            let soft_halo = exp(-(d * d) / (lt.z * lt.z * 1.45)) * lt.w * lantern_glow;
            col += vec3<f32>(1.0, 0.58, 0.18) * (tight_halo * 0.48 + soft_halo * 0.18) * shimmer;
        }
    }

    // 9. Soft organic pond vignette & subtle painterly paper grain
    let vig = smoothstep(0.86, 0.22, center_dist);
    col *= mix(1.0 - u.water_deep.w, 1.0, vig);

    let grain = (hash1(floor(in.position.xy) + vec2<f32>(0.5, 0.5)) - 0.5) * 0.016;
    col = clamp(col + vec3<f32>(grain), vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(col, 1.0);
}
