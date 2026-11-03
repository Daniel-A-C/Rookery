// Fullscreen procedural planet & orbital horizon shader for Orbital Haven.
// Atmospheric limb scattering, day/night planet terminator, and cosmic starfield.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    orbit_angle: f32,
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;

    // Deep space black backdrop
    var col = vec3<f32>(0.008, 0.012, 0.022);

    // Stars in space (y < 0.85)
    let star_grid = floor(uv * res / 36.0);
    let star_h = hash(star_grid);
    if (star_h > 0.955 && uv.y < 0.82) {
        let star_pos = (star_grid + vec2<f32>(hash(star_grid + 1.0), hash(star_grid + 2.0))) * 36.0 / res;
        let d = length((uv - star_pos) * res);
        let twinkle = 0.6 + 0.4 * sin(u.time * 2.0 + star_h * 40.0);
        let s_alpha = smoothstep(1.5, 0.0, d) * twinkle * 0.75;
        col += vec3<f32>(0.92, 0.95, 1.0) * s_alpha;
    }

    // Curving planetary horizon sphere (centered far below screen)
    let planet_center = vec2<f32>(0.5, 2.3);
    let planet_radius = 1.62;
    let d_planet = distance(uv, planet_center);

    if (d_planet < planet_radius) {
        // Inside planet body
        let norm_d = d_planet / planet_radius;
        // Day/night terminator angle based on orbit
        let sun_dir = vec2<f32>(cos(u.orbit_angle * 0.5), -sin(u.orbit_angle * 0.5));
        let surface_pos = (uv - planet_center) / planet_radius;
        let day_factor = clamp(dot(surface_pos, sun_dir) * 1.8 + 0.2, 0.05, 1.0);

        // Planet surface bands (ocean blue, cloud swirls, land continents)
        let ocean_col = vec3<f32>(0.03, 0.12, 0.24);
        let land_col = vec3<f32>(0.08, 0.22, 0.14);
        let cloud_col = vec3<f32>(0.75, 0.82, 0.92);

        let band = sin(surface_pos.x * 25.0 + u.orbit_angle * 0.2) * cos(surface_pos.y * 18.0);
        var planet_surface = mix(ocean_col, land_col, smoothstep(-0.2, 0.3, band));
        let clouds = smoothstep(0.4, 0.8, sin(surface_pos.x * 35.0 - u.orbit_angle * 0.4) * sin(surface_pos.y * 30.0));
        planet_surface = mix(planet_surface, cloud_col, clouds * 0.55);

        col = planet_surface * day_factor;

        // City lights on the night side of the planet
        if (day_factor < 0.25) {
            let night_grid = floor(uv * res / 18.0);
            let city_h = hash(night_grid + 50.0);
            if (city_h > 0.96) {
                let city_glow = (1.0 - day_factor / 0.25) * 0.85;
                col += vec3<f32>(0.98, 0.75, 0.35) * city_glow;
            }
        }
    } else {
        // Atmospheric limb glow / Rayleigh scattering just above horizon
        let dist_to_limb = d_planet - planet_radius;
        if (dist_to_limb < 0.12) {
            let limb_alpha = smoothstep(0.12, 0.0, dist_to_limb);
            let atmo_col = vec3<f32>(0.25, 0.55, 0.95); // glowing blue atmospheric limb
            col += atmo_col * limb_alpha * 0.75;
        }
    }

    return vec4<f32>(col, 1.0);
}

