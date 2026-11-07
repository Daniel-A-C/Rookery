// Fullscreen 3D planetary horizon, atmospheric scattering, and orbital backdrop shader
// for Orbital Haven v2.0.
//
// Features:
// - 3-layer parallax starfield with blackbody color temperatures & galactic dust band
// - True 3D spherical planet projection (N = (x, y, z)) with latitude/longitude FBM terrain
// - 4 selectable planetary biomes: Terran Cradle, Oceanic Europa, Ares Forge, Cyber-Ecumenopolis
// - Rotating FBM cloud deck with surface shadow casting & terminator sunset tinting
// - Specular ocean sun-glint, coastal city night-light networks, and polar auroral curtains
// - Rayleigh/Mie atmospheric limb glow, orbital sunrise flare, and subspace scan pulse wave

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    orbit_angle: f32,
    camera_offset: vec2<f32>,
    planet_biome: f32,
    hud_intensity: f32,
    scan_center: vec2<f32>,
    scan_progress: f32,
    station_tier: f32,
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
    var q = fract(p * vec2<f32>(127.1, 311.7));
    q += dot(q, q + 34.53);
    return fract(q.x * q.y);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q) * 43758.5453);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_f = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u_f.x), mix(c, d, u_f.x), u_f.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var p = p_in;
    let rot = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    for (var i = 0; i < 4; i++) {
        v += a * noise2(p);
        p = rot * p * 2.04 + vec2<f32>(11.3, 7.7);
        a *= 0.5;
    }
    return v;
}

fn star_layer(uv: vec2<f32>, res: vec2<f32>, cell_px: f32, threshold: f32, time: f32, seed_off: f32) -> vec3<f32> {
    let grid = floor(uv * res / cell_px);
    let h = hash21(grid + seed_off);
    if (h <= threshold) {
        return vec3<f32>(0.0);
    }
    let rand_pt = hash22(grid + seed_off + 17.0);
    let star_pos = (grid + 0.15 + rand_pt * 0.7) * cell_px / res;
    let d = length((uv - star_pos) * res);
    let twinkle = 0.65 + 0.35 * sin(time * (1.8 + rand_pt.x * 2.5) + h * 50.0);
    let core = smoothstep(1.85, 0.0, d) * twinkle;

    // Stellar blackbody tint (blue-white, warm solar, coral giant)
    var tint = vec3<f32>(0.90, 0.95, 1.0);
    if (rand_pt.y > 0.75) {
        tint = vec3<f32>(1.0, 0.86, 0.68);
    } else if (rand_pt.y < 0.25) {
        tint = vec3<f32>(0.72, 0.88, 1.0);
    }
    return tint * core;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = max(u.resolution, vec2<f32>(1.0, 1.0));
    let aspect = res.x / res.y;
    let cam = u.camera_offset / vec2<f32>(1000.0, 750.0);

    // 1. Deep space backdrop with subtle galactic nebula band
    var col = vec3<f32>(0.005, 0.010, 0.020);
    let sky_uv = uv + cam * 0.12;
    let band_coord = vec2<f32>(sky_uv.x * 2.2 - sky_uv.y * 1.1, sky_uv.y * 2.5);
    let dust = fbm(band_coord * 1.8 + vec2<f32>(u.orbit_angle * 0.05, 0.0));
    let band_mask = exp(-pow((sky_uv.y - 0.28 + (sky_uv.x - 0.5) * 0.25) * 3.8, 2.0));
    col += vec3<f32>(0.025, 0.048, 0.095) * dust * band_mask * 0.85;
    col += vec3<f32>(0.055, 0.025, 0.075) * smoothstep(0.45, 0.85, dust) * band_mask * 0.45;

    // 2. Multi-layer parallax starfield
    let s1 = star_layer(uv + cam * 0.08, res, 48.0, 0.945, u.time, 1.0);
    let s2 = star_layer(uv + cam * 0.16, res, 30.0, 0.962, u.time, 43.0) * 0.75;
    let s3 = star_layer(uv + cam * 0.24, res, 72.0, 0.920, u.time, 91.0) * 1.25;
    var stars = s1 + s2 + s3;

    // 3. True 3D spherical planet geometry (aspect-corrected)
    let planet_center = vec2<f32>(0.5 - cam.x * 0.45, 1.88 - cam.y * 0.45);
    let planet_radius = 1.24;
    let delta = vec2<f32>((uv.x - planet_center.x) * aspect, uv.y - planet_center.y);
    let d_planet = length(delta);

    // Dynamic 3D sun direction orbiting gracefully
    let sun_phase = u.orbit_angle * 0.65 + 0.4;
    let sun_dir = normalize(vec3<f32>(
        cos(sun_phase) * 0.85,
        -0.38 - 0.22 * sin(sun_phase * 0.7),
        0.35 + 0.50 * sin(sun_phase)
    ));

    // Biome palette selection (0=Terran, 1=Europa, 2=Ares, 3=Ecumenopolis)
    let biome = round(u.planet_biome);
    var atmo_col = vec3<f32>(0.22, 0.58, 0.98);
    var sunset_col = vec3<f32>(0.98, 0.52, 0.22);
    if (biome == 1.0) {
        atmo_col = vec3<f32>(0.18, 0.78, 0.95);
        sunset_col = vec3<f32>(0.35, 0.85, 1.0);
    } else if (biome == 2.0) {
        atmo_col = vec3<f32>(0.92, 0.44, 0.24);
        sunset_col = vec3<f32>(1.0, 0.68, 0.28);
    } else if (biome == 3.0) {
        atmo_col = vec3<f32>(0.58, 0.32, 0.96);
        sunset_col = vec3<f32>(0.95, 0.35, 0.78);
    }

    if (d_planet < planet_radius) {
        // Inside planet body: compute 3D normal on unit sphere
        let p2 = delta / planet_radius;
        let r2 = clamp(dot(p2, p2), 0.0, 1.0);
        let nz = sqrt(max(0.001, 1.0 - r2));
        let normal = normalize(vec3<f32>(p2.x, p2.y, nz));
        let view_dir = vec3<f32>(0.0, 0.0, 1.0);

        // Spherical coordinates for continent & weather FBM
        let lon = atan2(normal.x, normal.z) + u.orbit_angle * 0.35;
        let lat = asin(clamp(normal.y, -1.0, 1.0));
        let sphere_uv = vec2<f32>(lon * 2.4, lat * 3.2);

        let n_elev = fbm(sphere_uv * 1.45 + vec2<f32>(2.1, 4.7));
        let n_detail = fbm(sphere_uv * 3.8 - vec2<f32>(1.4, 3.2));
        let height_val = n_elev * 0.72 + n_detail * 0.28;

        // Illumination terms
        let ndotl = dot(normal, sun_dir);
        let day_factor = smoothstep(-0.14, 0.28, ndotl);
        let terminator = smoothstep(-0.18, 0.06, ndotl) * (1.0 - smoothstep(0.04, 0.32, ndotl));
        let fresnel = pow(1.0 - nz, 3.0);

        var surface_col = vec3<f32>(0.0);
        var is_ocean = false;
        var is_land = false;

        if (biome == 0.0) {
            // Terran Cradle: deep ocean, turquoise shelf, emerald lowlands, highland peaks
            is_ocean = height_val < 0.50;
            is_land = !is_ocean;
            let deep_ocean = vec3<f32>(0.02, 0.09, 0.22);
            let shelf_ocean = vec3<f32>(0.04, 0.26, 0.38);
            let lowland = vec3<f32>(0.08, 0.27, 0.16);
            let highland = vec3<f32>(0.26, 0.24, 0.18);
            let snow_peak = vec3<f32>(0.82, 0.88, 0.94);

            if (is_ocean) {
                let shelf = smoothstep(0.38, 0.50, height_val);
                surface_col = mix(deep_ocean, shelf_ocean, shelf);
            } else {
                let mtn = smoothstep(0.50, 0.72, height_val);
                let peaks = smoothstep(0.72, 0.86, height_val);
                surface_col = mix(lowland, highland, mtn);
                surface_col = mix(surface_col, snow_peak, peaks);
            }
        } else if (biome == 1.0) {
            // Oceanic Europa: glacial crust, cobalt fissures, subsurface glow
            is_ocean = true;
            is_land = height_val > 0.52;
            let ice_base = vec3<f32>(0.55, 0.76, 0.90);
            let ice_ridge = vec3<f32>(0.80, 0.92, 0.98);
            let fissure = vec3<f32>(0.06, 0.24, 0.44);
            let crack = abs(n_detail - 0.5);
            surface_col = mix(fissure, mix(ice_base, ice_ridge, height_val), smoothstep(0.02, 0.12, crack));
        } else if (biome == 2.0) {
            // Ares Forge: rust canyons, basalt plains, ochre highlands
            is_ocean = false;
            is_land = true;
            let basalt = vec3<f32>(0.22, 0.09, 0.06);
            let rust_plain = vec3<f32>(0.54, 0.21, 0.11);
            let ochre_ridge = vec3<f32>(0.76, 0.42, 0.22);
            surface_col = mix(basalt, rust_plain, smoothstep(0.30, 0.55, height_val));
            surface_col = mix(surface_col, ochre_ridge, smoothstep(0.58, 0.82, height_val));
        } else {
            // Cyber-Ecumenopolis: obsidian alloy plates, arcology sectors
            is_ocean = height_val < 0.35;
            is_land = !is_ocean;
            let alloy_dark = vec3<f32>(0.05, 0.06, 0.10);
            let sector_plate = vec3<f32>(0.12, 0.15, 0.22);
            surface_col = mix(alloy_dark, sector_plate, step(0.48, n_detail));
        }

        // Cloud layer with shadow offset
        let cloud_uv = vec2<f32>(lon * 2.8 - u.time * 0.012, lat * 3.4);
        let cloud_n = fbm(cloud_uv * 1.9 + vec2<f32>(0.0, fbm(cloud_uv * 2.5) * 0.45));
        let cloud_shadow_n = fbm((cloud_uv - sun_dir.xy * 0.025) * 1.9);
        let cloud_density = smoothstep(0.46, 0.78, cloud_n) * select(0.78, 0.35, biome == 3.0);
        let cloud_shadow = smoothstep(0.46, 0.78, cloud_shadow_n) * 0.45;

        // Apply cloud shadow to surface
        surface_col *= (1.0 - cloud_shadow * day_factor);

        // Specular ocean glint
        if (is_ocean) {
            let half_vec = normalize(sun_dir + view_dir);
            let spec = pow(max(dot(normal, half_vec), 0.0), 28.0) * day_factor * (1.0 - cloud_density * 0.8);
            surface_col += mix(vec3<f32>(0.45, 0.75, 1.0), sunset_col, terminator) * spec * 0.55;
        }

        // Blend clouds (tinted warm amber at the terminator)
        let cloud_tint = mix(vec3<f32>(0.90, 0.95, 1.0), sunset_col, terminator * 0.75);
        let lit_surface = mix(surface_col, cloud_tint, cloud_density);

        // Combine day illumination + ambient night earthshine
        let ambient_night = surface_col * 0.045 + atmo_col * 0.018;
        col = mix(ambient_night, lit_surface, day_factor);
        col += sunset_col * terminator * (0.18 + 0.25 * cloud_density);

        // Night-side city lights / bioluminescent rifts / arcology grids
        let night_mask = (1.0 - smoothstep(-0.18, 0.12, ndotl)) * (1.0 - cloud_density * 0.65);
        if (night_mask > 0.01 && is_land) {
            let city_cell = floor(sphere_uv * 18.0);
            let city_h = hash21(city_cell);
            let coast_factor = smoothstep(0.75, 0.50, height_val);
            let threshold = select(0.74 - coast_factor * 0.14, 0.45, biome == 3.0);
            if (city_h > threshold) {
                let sub = fract(sphere_uv * 18.0);
                let hub_d = length(sub - vec2<f32>(0.5, 0.5));
                let hub_glow = smoothstep(0.42, 0.05, hub_d);
                let filament = smoothstep(0.08, 0.0, abs(sub.x - 0.5)) + smoothstep(0.08, 0.0, abs(sub.y - 0.5));
                var light_col = vec3<f32>(1.0, 0.76, 0.36);
                if (biome == 1.0) {
                    light_col = vec3<f32>(0.20, 0.92, 0.98);
                } else if (biome == 3.0) {
                    light_col = mix(vec3<f32>(0.98, 0.72, 0.28), vec3<f32>(0.32, 0.85, 1.0), step(0.7, city_h));
                }
                col += light_col * (hub_glow * 0.85 + filament * 0.22) * night_mask;
            }
        }

        // Polar auroral curtains near upper limb
        let aurora_band = smoothstep(0.55, 0.88, fresnel) * smoothstep(-0.4, 0.3, normal.x * sin(u.time * 0.3 + lon * 4.0));
        let aurora_curtain = sin(lon * 24.0 + u.time * 1.1) * 0.5 + 0.5;
        let aurora_col = select(vec3<f32>(0.18, 0.95, 0.68), vec3<f32>(0.95, 0.45, 0.25), biome == 2.0);
        col += aurora_col * aurora_band * aurora_curtain * (1.0 - day_factor * 0.65) * 0.32;

        // Inner atmospheric Rayleigh rim scattering
        let rim_light = clamp(dot(normal, sun_dir) * 0.6 + 0.5, 0.15, 1.0);
        col = mix(col, mix(atmo_col, sunset_col, terminator * 0.6) * rim_light, fresnel * 0.68);
    } else {
        // Outside planet body: stars + outer Rayleigh/Mie limb halo + orbital sunrise flare
        let dist_to_limb = d_planet - planet_radius;
        let star_fade = smoothstep(0.0, 0.05, dist_to_limb);
        col += stars * star_fade;

        if (dist_to_limb < 0.18) {
            let limb_norm = normalize(vec3<f32>(delta.x / planet_radius, delta.y / planet_radius, 0.15));
            let sun_align = clamp(dot(limb_norm, sun_dir) * 0.65 + 0.45, 0.12, 1.0);
            let sunset_grazing = pow(clamp(1.0 - abs(dot(limb_norm, sun_dir)), 0.0, 1.0), 3.0);

            let halo_tight = exp(-dist_to_limb * 38.0);
            let halo_wide = exp(-dist_to_limb * 13.5);
            let scatter_col = mix(atmo_col, sunset_col, sunset_grazing * 0.65);

            col += scatter_col * (halo_tight * 0.82 + halo_wide * 0.28) * sun_align;

            // Sunrise / sunset solar corona grazing the horizon limb
            let sun_screen = planet_center + vec2<f32>(sun_dir.x / aspect, sun_dir.y) * planet_radius * 0.96;
            let d_sun = length(vec2<f32>((uv.x - sun_screen.x) * aspect, uv.y - sun_screen.y));
            let flare = exp(-d_sun * 7.5) * halo_wide * clamp(sun_dir.z + 0.3, 0.0, 1.0);
            col += vec3<f32>(1.0, 0.82, 0.48) * flare * 0.55;
        }
    }

    // 4. Subtle station core volumetric backlight & orbital trajectory arc
    let station_uv = vec2<f32>(0.5 - cam.x, 0.473 - cam.y);
    let d_station = length(vec2<f32>((uv.x - station_uv.x) * aspect, uv.y - station_uv.y));
    let tier_boost = 0.7 + 0.12 * u.station_tier;
    col += vec3<f32>(0.08, 0.24, 0.42) * exp(-d_station * 5.2) * 0.18 * tier_boost;

    // 5. Subspace Sensor Scan Wave (when triggered)
    if (u.scan_progress > 0.001 && u.scan_progress < 0.999) {
        let scan_uv = u.scan_center / vec2<f32>(1000.0, 750.0) - cam;
        let d_scan = length(vec2<f32>((uv.x - scan_uv.x) * aspect, uv.y - scan_uv.y));
        let wave_r = u.scan_progress * 0.85;
        let ring = exp(-pow((d_scan - wave_r) * 42.0, 2.0));
        let fade = 1.0 - u.scan_progress;
        col += vec3<f32>(0.25, 0.88, 1.0) * ring * fade * 0.75;
    }

    return vec4<f32>(col, 1.0);
}
