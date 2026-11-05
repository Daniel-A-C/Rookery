// Atmospheric background shader for The Living ASCII Rookery.
// Soft dusk/twilight sky gradient, twinkling stars, and gentle horizon ground.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    time_of_day: f32,
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
    // Fullscreen triangle
    let x = f32(i32(vid << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vid & 2u) * -2) + 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

// Pseudo-random hash for procedural stars
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let d = dot(q, q + 45.32);
    return fract(d);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let res = u.resolution;

    // Refined atmospheric sky: deep twilight sapphire into warm amber horizon dusk
    let sky_zenith = vec3<f32>(0.020, 0.032, 0.068);  // deep twilight indigo
    let sky_mid = vec3<f32>(0.048, 0.068, 0.118);     // tranquil dusk slate
    let sky_glow = vec3<f32>(0.115, 0.098, 0.142);    // soft dusk violet
    let sky_horizon = vec3<f32>(0.195, 0.142, 0.125);  // warm ember dusk glow

    var sky = mix(sky_zenith, sky_mid, smoothstep(0.0, 0.48, uv.y));
    sky = mix(sky, sky_glow, smoothstep(0.48, 0.78, uv.y));
    sky = mix(sky, sky_horizon, smoothstep(0.78, 0.90, uv.y));

    // Subtle stars and chromatic motes in the upper sky
    if (uv.y < 0.68) {
        let grid_size = 36.0;
        let cell = floor(uv * res / grid_size);
        let star_rnd = hash21(cell);
        if (star_rnd > 0.952) {
            let star_pos = (cell + vec2<f32>(hash21(cell + 1.0), hash21(cell + 2.0))) * grid_size / res;
            let d = length((uv - star_pos) * res);
            let twinkle = 0.55 + 0.45 * sin(u.time * (1.6 + star_rnd * 3.2) + star_rnd * 100.0);
            let star_alpha = smoothstep(1.9, 0.0, d) * twinkle * (1.0 - uv.y / 0.68) * 0.60;
            // Chromatic star tinting (silver, amber gold, sapphire)
            let star_color = mix(
                vec3<f32>(0.92, 0.95, 1.0),
                vec3<f32>(1.0, 0.88, 0.65),
                fract(star_rnd * 7.0)
            );
            sky += star_color * star_alpha;
        }
    }

    // Soft atmospheric twilight ground mist rolling across the lower horizon
    if (uv.y > 0.74 && uv.y < 0.92) {
        let mist_wave1 = sin(uv.x * 4.5 + u.time * 0.18) * 0.5 + 0.5;
        let mist_wave2 = cos(uv.x * 9.2 - u.time * 0.11) * 0.5 + 0.5;
        let mist_factor = mist_wave1 * 0.6 + mist_wave2 * 0.4;
        let mist_envelope = smoothstep(0.74, 0.85, uv.y) * smoothstep(0.92, 0.86, uv.y);
        let mist_color = vec3<f32>(0.085, 0.095, 0.115);
        sky = mix(sky, sky + mist_color * 0.35, mist_envelope * mist_factor);
    }

    // Rich rolling ground floor with layered mossy soil tones and undulating terrain
    let ground_y = 0.898 + 0.014 * sin(uv.x * 3.8 + 0.6) + 0.007 * sin(uv.x * 11.2 - 0.4) + 0.003 * sin(uv.x * 24.0);
    if (uv.y > ground_y) {
        let moss_crest = vec3<f32>(0.065, 0.088, 0.052); // velvety moss crest
        let soil_top = vec3<f32>(0.045, 0.048, 0.038);   // deep earthy forest loam
        let soil_deep = vec3<f32>(0.018, 0.020, 0.022);  // deep subterranean bed

        let ground_blend = smoothstep(ground_y, ground_y + 0.008, uv.y);
        let moss_to_soil = smoothstep(ground_y + 0.008, ground_y + 0.032, uv.y);
        let soil_to_deep = smoothstep(ground_y + 0.032, ground_y + 0.10, uv.y);

        var ground_col = mix(moss_crest, soil_top, moss_to_soil);
        ground_col = mix(ground_col, soil_deep, soil_to_deep);

        sky = mix(sky, ground_col, ground_blend);
    }

    return vec4<f32>(sky, 1.0);
}

