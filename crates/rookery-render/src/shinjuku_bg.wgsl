// Atmospheric monospace terminal background shader for Shinjuku Rain v2.0.
// Renders a deep rainy Tokyo night sky with urban neon haze, localized lightning
// cloud illumination, storefront neon bloom halos, moving train/vehicle reflections,
// wet zebra crosswalk stripes, expanding puddle ripples, and subtle terminal grid lines.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    palette_id: f32,
    lightning: f32,
    lightning_x: f32,
    signs_lit: f32,
    wind: f32,
    prev_palette_id: f32,
    palette_blend: f32,
    rain_intensity: f32,
    traffic_red: f32,
    train_light: vec4<f32>,      // (center_x, active_alpha, spark_x, spark_flash)
    store_fx: vec4<f32>,         // (ramen_boost, bar_flicker, vending_pulse, konbini_pulse)
    pointer: vec4<f32>,          // (x_norm, y_norm, active, 0.0)
    vehicles: array<vec4<f32>, 3>, // (x_norm, y_norm, dir_sign, brake_glow)
    ripples: array<vec4<f32>, 4>,  // (x_norm, y_norm, age_norm, strength)
}

struct BgPalette {
    zenith: vec3<f32>,
    horizon: vec3<f32>,
    street_base: vec3<f32>,
    neon_a: vec3<f32>,
    neon_b: vec3<f32>,
    neon_c: vec3<f32>,
    konbini: vec3<f32>,
    grid_glow: vec3<f32>,
}

fn palette_at(idx: i32) -> BgPalette {
    var p: BgPalette;
    // 0: Shinjuku Neon (Kabukicho) — deep indigo night, electric rose/cyan/amber
    p.zenith = vec3<f32>(0.010, 0.010, 0.028);
    p.horizon = vec3<f32>(0.044, 0.022, 0.072);
    p.street_base = vec3<f32>(0.012, 0.014, 0.024);
    p.neon_a = vec3<f32>(0.98, 0.16, 0.52); // Magenta / Rose
    p.neon_b = vec3<f32>(0.10, 0.86, 0.98); // Electric Cyan
    p.neon_c = vec3<f32>(0.98, 0.60, 0.14); // Warm Ramen Amber
    p.konbini = vec3<f32>(0.42, 0.94, 0.84); // Bright Konbini Mint-White
    p.grid_glow = vec3<f32>(0.012, 0.012, 0.024);

    switch idx {
        case 1: {
            // 1: Kyoto Gion — deep ink indigo, vermilion lanterns, warm cedar gold
            p.zenith = vec3<f32>(0.008, 0.012, 0.024);
            p.horizon = vec3<f32>(0.040, 0.026, 0.044);
            p.street_base = vec3<f32>(0.014, 0.016, 0.022);
            p.neon_a = vec3<f32>(0.98, 0.26, 0.16);
            p.neon_b = vec3<f32>(0.24, 0.72, 0.64);
            p.neon_c = vec3<f32>(0.98, 0.74, 0.22);
            p.konbini = vec3<f32>(0.92, 0.84, 0.58);
            p.grid_glow = vec3<f32>(0.010, 0.012, 0.018);
        }
        case 2: {
            // 2: Midnight Downpour — cold rain slate, silver mist, sodium vapor
            p.zenith = vec3<f32>(0.008, 0.014, 0.022);
            p.horizon = vec3<f32>(0.028, 0.042, 0.062);
            p.street_base = vec3<f32>(0.012, 0.016, 0.024);
            p.neon_a = vec3<f32>(0.54, 0.78, 1.00);
            p.neon_b = vec3<f32>(0.80, 0.88, 0.98);
            p.neon_c = vec3<f32>(0.96, 0.72, 0.30);
            p.konbini = vec3<f32>(0.76, 0.90, 1.00);
            p.grid_glow = vec3<f32>(0.010, 0.014, 0.020);
        }
        case 3: {
            // 3: Cyber Terminal — CRT phosphor emerald & amber
            p.zenith = vec3<f32>(0.003, 0.012, 0.006);
            p.horizon = vec3<f32>(0.008, 0.036, 0.016);
            p.street_base = vec3<f32>(0.004, 0.016, 0.008);
            p.neon_a = vec3<f32>(0.18, 0.98, 0.44);
            p.neon_b = vec3<f32>(0.98, 0.72, 0.16);
            p.neon_c = vec3<f32>(0.36, 0.98, 0.58);
            p.konbini = vec3<f32>(0.52, 1.00, 0.68);
            p.grid_glow = vec3<f32>(0.005, 0.022, 0.010);
        }
        default: {}
    }
    return p;
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

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_f = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u_f.x), mix(c, d, u_f.x), u_f.y);
}

fn fbm3(p: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i = i + 1) {
        val += amp * noise2d(pos);
        pos = pos * 2.03 + vec2<f32>(17.1, 31.7);
        amp *= 0.5;
    }
    return val;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;
    let px = uv * res;

    let cell_w = 9.6;
    let cell_h = 16.0;
    let gx = fract(px.x / cell_w);
    let gy = fract(px.y / cell_h);
    let is_grid_line = (gx < (1.0 / cell_w)) || (gy < (1.0 / cell_h));

    let curr_idx = i32(round(u.palette_id)) % 4;
    let prev_idx = i32(round(u.prev_palette_id)) % 4;
    let s = clamp(u.palette_blend, 0.0, 1.0);
    let blend = s * s * (3.0 - 2.0 * s);

    let pa = palette_at(prev_idx);
    let pb = palette_at(curr_idx);

    let zenith = mix(pa.zenith, pb.zenith, blend);
    let horizon = mix(pa.horizon, pb.horizon, blend);
    let street_base = mix(pa.street_base, pb.street_base, blend);
    let neon_a = mix(pa.neon_a, pb.neon_a, blend);
    let neon_b = mix(pa.neon_b, pb.neon_b, blend);
    let neon_c = mix(pa.neon_c, pb.neon_c, blend);
    let konbini = mix(pa.konbini, pb.konbini, blend);
    let grid_glow = mix(pa.grid_glow, pb.grid_glow, blend);

    let sign_strength = mix(0.20, 1.0, u.signs_lit);
    let curb_y = 0.76;
    var col = vec3<f32>(0.0);

    if uv.y < curb_y {
        let sky_t = clamp(uv.y / curb_y, 0.0, 1.0);
        col = mix(zenith, horizon, pow(sky_t, 1.55));

        // Layered drifting rain clouds
        let cloud_uv = vec2<f32>(uv.x * 3.4 + u.time * 0.016 * (1.0 + u.wind), uv.y * 2.5);
        let clouds = fbm3(cloud_uv);
        col += vec3<f32>(0.018, 0.022, 0.036) * smoothstep(0.42, 0.86, clouds) * (1.0 - sky_t * 0.55);

        // Rising urban neon light pollution near the lower skyline
        let haze_band = exp(-pow((uv.y - 0.68) / 0.19, 2.0)) * sign_strength;
        let haze_mix = mix(neon_a, neon_b, smoothstep(0.25, 0.80, uv.x));
        col += haze_mix * haze_band * 0.065;

        // Localized lightning flash illuminating clouds around lightning_x
        let bolt_dist = abs(uv.x - u.lightning_x);
        let bolt_focus = exp(-pow(bolt_dist / 0.32, 2.0));
        let flash = u.lightning * (0.28 + 0.72 * bolt_focus) * (0.45 + 0.55 * clouds) * (1.0 - sky_t * 0.55);
        col += vec3<f32>(0.58, 0.70, 0.98) * flash * 0.55;

        // Overhead commuter train warm window mist & pantograph spark glow
        if u.train_light.y > 0.01 {
            let td = vec2<f32>((uv.x - u.train_light.x) / 0.24, (uv.y - 0.28) / 0.055);
            let t_glow = exp(-dot(td, td)) * u.train_light.y;
            col += neon_c * t_glow * 0.11;
        }
        if u.train_light.w > 0.01 {
            let sd = vec2<f32>((uv.x - u.train_light.z) / 0.055, (uv.y - 0.24) / 0.045);
            let s_glow = exp(-dot(sd, sd)) * u.train_light.w;
            col += vec3<f32>(0.62, 0.92, 1.00) * s_glow * 0.42;
        }

        // Storefront ambient neon bloom halos along the lower street wall
        let ramen_d = vec2<f32>((uv.x - 0.18) / 0.11, (uv.y - 0.67) / 0.09);
        let ramen_boost = 1.0 + u.store_fx.x * 0.45;
        col += neon_c * exp(-dot(ramen_d, ramen_d)) * 0.16 * sign_strength * ramen_boost;

        let alley_d = vec2<f32>((uv.x - 0.33) / 0.055, (uv.y - 0.58) / 0.12);
        col += neon_a * exp(-dot(alley_d, alley_d)) * 0.10 * sign_strength;

        let bar_d = vec2<f32>((uv.x - 0.46) / 0.10, (uv.y - 0.61) / 0.11);
        let bar_flicker = u.store_fx.y;
        col += mix(neon_a, neon_b, 0.35) * exp(-dot(bar_d, bar_d)) * 0.19 * sign_strength * bar_flicker;

        let vend_d = vec2<f32>((uv.x - 0.69) / 0.075, (uv.y - 0.69) / 0.075);
        let vend_boost = 1.0 + u.store_fx.z * 0.75;
        col += neon_b * exp(-dot(vend_d, vend_d)) * 0.18 * sign_strength * vend_boost;

        let konbini_d = vec2<f32>((uv.x - 0.875) / 0.11, (uv.y - 0.67) / 0.085);
        let konbini_boost = 1.0 + u.store_fx.w * 0.55;
        col += konbini * exp(-dot(konbini_d, konbini_d)) * 0.18 * sign_strength * konbini_boost;

        // Crosswalk traffic signal halo (green vs red)
        let sig_d = vec2<f32>((uv.x - 0.585) / 0.032, (uv.y - 0.61) / 0.042);
        let sig_col = mix(vec3<f32>(0.18, 0.96, 0.48), vec3<f32>(1.00, 0.22, 0.24), u.traffic_red);
        col += sig_col * exp(-dot(sig_d, sig_d)) * 0.18;

        // Subtle mist band hovering over the wet sidewalk
        let mist = exp(-pow((uv.y - curb_y) / 0.06, 2.0)) * (0.04 + 0.03 * u.rain_intensity);
        col += mix(neon_b, vec3<f32>(0.45, 0.55, 0.72), 0.75) * mist;
    } else {
        // Wet asphalt street plane
        let street_t = clamp((uv.y - curb_y) / (1.0 - curb_y), 0.0, 1.0);
        col = mix(street_base * 1.45, street_base * 0.65, street_t);

        // Faint wet zebra crosswalk stripes near x ~ 0.57..0.67
        if uv.x > 0.57 && uv.x < 0.67 && street_t > 0.10 && street_t < 0.92 {
            let stripe = smoothstep(0.35, 0.55, sin(street_t * 34.0));
            col += vec3<f32>(0.032, 0.038, 0.052) * stripe * (1.0 - street_t * 0.35);
        }

        // Animated rain-ripple distortion across wet pavement
        var ripple_x = sin(uv.y * 96.0 - u.time * 5.6 + sin(uv.x * 38.0) * 1.9) * (0.008 + 0.004 * u.rain_intensity);
        var ripple_sheen = 0.0;
        for (var i = 0; i < 4; i = i + 1) {
            let rip = u.ripples[i];
            if rip.w > 0.01 {
                let d_vec = vec2<f32>((uv.x - rip.x), (uv.y - rip.y) * 2.4);
                let dist = length(d_vec);
                let radius = 0.012 + rip.z * 0.075;
                let ring = exp(-pow((dist - radius) / 0.010, 2.0)) * (1.0 - rip.z) * rip.w;
                ripple_x += ring * 0.018 * sign(uv.x - rip.x + 0.0001);
                ripple_sheen += ring * 0.14;
            }
        }
        let rx = uv.x + ripple_x;
        let fade = (1.0 - street_t * 0.50) * sign_strength;

        // Vertical neon reflections from all 4 storefronts + traffic light
        let r_ramen = exp(-pow((rx - 0.18) / 0.072, 2.0)) * (1.0 + u.store_fx.x * 0.35);
        col += neon_c * r_ramen * 0.24 * fade;

        let r_alley = exp(-pow((rx - 0.33) / 0.038, 2.0));
        col += neon_a * r_alley * 0.12 * fade;

        let r_bar = exp(-pow((rx - 0.46) / 0.075, 2.0)) * u.store_fx.y;
        col += neon_a * r_bar * 0.25 * fade;

        let sig_col = mix(vec3<f32>(0.18, 0.96, 0.48), vec3<f32>(1.00, 0.22, 0.24), u.traffic_red);
        let r_sig = exp(-pow((rx - 0.585) / 0.028, 2.0));
        col += sig_col * r_sig * 0.16 * (1.0 - street_t * 0.55);

        let r_vending = exp(-pow((rx - 0.69) / 0.060, 2.0)) * (1.0 + u.store_fx.z * 0.55);
        col += neon_b * r_vending * 0.24 * fade;

        let r_konbini = exp(-pow((rx - 0.875) / 0.082, 2.0)) * (1.0 + u.store_fx.w * 0.40);
        col += konbini * r_konbini * 0.24 * fade;

        // Dynamic moving vehicle headlight cones & red taillight streaks on wet asphalt
        for (var v_idx = 0; v_idx < 3; v_idx = v_idx + 1) {
            let veh = u.vehicles[v_idx];
            if veh.x > -0.15 && veh.x < 1.15 && veh.y > curb_y {
                let dir = veh.z;
                // Forward headlight beam cone
                let head_dx = (uv.x - (veh.x + dir * 0.065)) / 0.075;
                let head_dy = (uv.y - (veh.y + 0.012)) / 0.034;
                let ahead = smoothstep(-0.02, 0.03, (uv.x - veh.x) * dir);
                let head_glow = exp(-(head_dx * head_dx + head_dy * head_dy)) * ahead;
                col += vec3<f32>(1.00, 0.94, 0.72) * head_glow * 0.26;

                // Rear red taillight / brake reflection
                let tail_dx = (uv.x - (veh.x - dir * 0.035)) / 0.040;
                let tail_dy = (uv.y - (veh.y + 0.016)) / 0.038;
                let tail_glow = exp(-(tail_dx * tail_dx + tail_dy * tail_dy));
                let brake_boost = 1.0 + veh.w * 1.45;
                col += vec3<f32>(1.00, 0.16, 0.18) * tail_glow * 0.18 * brake_boost;
            }
        }

        // Interactive cursor sheen on wet pavement
        if u.pointer.z > 0.5 && u.pointer.y > curb_y {
            let pd = vec2<f32>((uv.x - u.pointer.x) / 0.055, (uv.y - u.pointer.y) / 0.035);
            col += neon_b * exp(-dot(pd, pd)) * 0.14;
        }

        // Ripple specular highlight
        col += (neon_b * 0.55 + vec3<f32>(0.45)) * ripple_sheen;

        // Crisp wet curb specular highlight line
        let curb_line = exp(-pow((uv.y - curb_y) / 0.0035, 2.0));
        col += (neon_b * 0.18 + vec3<f32>(0.10)) * curb_line;

        // Lightning sheen on wet asphalt
        col += vec3<f32>(0.38, 0.50, 0.78) * u.lightning * 0.22;
    }

    if is_grid_line {
        col += grid_glow * 0.24;
    }

    return vec4<f32>(col, 1.0);
}
