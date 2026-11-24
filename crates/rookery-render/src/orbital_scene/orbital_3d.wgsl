// Orbital Haven v3 — Y2K Low-Poly Textured 3D Shader (Deus Ex / System Shock 2 Aesthetic)

struct PointLight {
    // xyz = world position, w = radius
    pos_radius: vec4<f32>,
    // xyz = linear RGB color, w = intensity
    color_intensity: vec4<f32>,
};

struct Orbital3dUniforms {
    view_proj: mat4x4<f32>,
    // xyz = camera eye pos, w = time
    eye_and_time: vec4<f32>,
    // xyz = key sun direction (normalized), w = sun_intensity (dims during SolarEclipse)
    sun_dir_and_power: vec4<f32>,
    // xyz = ambient sky/space tint, w = fog_density
    ambient_and_fog: vec4<f32>,
    // xyz = fog color, w = palette accent pulse
    fog_color_and_pulse: vec4<f32>,
    // 8 dynamic colored point lights (docking floods, corridor fluorescents, reactor, beacons)
    lights: array<PointLight, 8>,
};

@group(0) @binding(0)
var<uniform> u: Orbital3dUniforms;

@group(0) @binding(1)
var atlas_tex: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tile_rect: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) emissive: f32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tile_rect: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) emissive: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_pos = in.pos;
    out.normal = normalize(in.normal);
    out.uv = in.uv;
    out.tile_rect = in.tile_rect;
    out.color = in.color;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Wrap local UV cleanly inside the procedural 64x64 atlas tile
    let tiled_uv = fract(in.uv) * in.tile_rect.zw + in.tile_rect.xy;
    let texel = textureSample(atlas_tex, atlas_sampler, tiled_uv);
    let base_rgb = texel.rgb * in.color.rgb;
    let alpha = texel.a * in.color.a;

    // 1. Translucent forcefields, thruster plasma plumes, and holotable projections
    if (in.emissive >= 1.5) {
        let shimmer = 0.85 + 0.15 * sin(u.eye_and_time.w * 4.5 + in.world_pos.y * 1.4 + in.world_pos.x * 0.9);
        return vec4<f32>(clamp(base_rgb * shimmer * 1.25, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
    }

    // 2. Self-lit CRT monitors, navigation beacons, reactor plasma, strip lights
    if (in.emissive >= 0.5) {
        let flicker = 0.94 + 0.06 * sin(u.eye_and_time.w * 7.0 + in.world_pos.x * 2.1 + in.world_pos.z * 1.7);
        let boost = mix(1.0, 1.35, clamp(in.emissive - 0.5, 0.0, 1.0)) * flicker;
        return vec4<f32>(clamp(base_rgb * boost, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
    }

    // 3. Y2K Forward Lit Geometry (directional sun + rim + 8 colored point lights)
    let n = normalize(in.normal);
    let sun_dir = normalize(u.sun_dir_and_power.xyz);
    let sun_power = u.sun_dir_and_power.w;
    let rim_dir = normalize(vec3<f32>(-sun_dir.x, 0.35, -sun_dir.z));

    let ndotl = max(dot(n, sun_dir), 0.0);
    let ndotrim = max(dot(n, rim_dir), 0.0);

    var light_accum = u.ambient_and_fog.xyz
        + vec3<f32>(0.92, 0.95, 1.0) * (ndotl * 0.72 * sun_power)
        + vec3<f32>(0.24, 0.46, 0.68) * (ndotrim * 0.28);

    // 8 dynamic station point lights (hangar sodium floods, corridor tubes, reactor, alert strobes)
    for (var i = 0u; i < 8u; i = i + 1u) {
        let pl = u.lights[i];
        let to_light = pl.pos_radius.xyz - in.world_pos;
        let dist = length(to_light);
        let radius = pl.pos_radius.w;
        if (dist < radius && radius > 0.01) {
            let atten = 1.0 - (dist / radius);
            let falloff = atten * atten;
            let ldir = to_light / max(dist, 0.001);
            // Soft wrap diffuse so angular Y2K bulkheads catch rich colored fill light
            let wrap = clamp((dot(n, ldir) + 0.25) / 1.25, 0.0, 1.0);
            light_accum = light_accum + pl.color_intensity.rgb * (pl.color_intensity.w * falloff * wrap);
        }
    }

    var lit_rgb = base_rgb * light_accum;

    // Subtle distance haze (stronger inside moody corridors, gentle in deep space)
    let cam_dist = length(in.world_pos - u.eye_and_time.xyz);
    let fog_t = 1.0 - exp(-cam_dist * u.ambient_and_fog.w);
    lit_rgb = mix(lit_rgb, u.fog_color_and_pulse.rgb, clamp(fog_t, 0.0, 0.62));

    return vec4<f32>(clamp(lit_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}
