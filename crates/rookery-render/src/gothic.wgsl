// Gothic Metropolis 3D Isometric Shader (1990s Batman / Anton Furst Aesthetic)

struct Uniforms {
    // Column-major 4x4 orthographic/isometric view-projection matrix
    view_proj: mat4x4<f32>,
    // xyz = key light direction (normalized), w = time
    light_and_time: vec4<f32>,
    // xyz = canyon warm bounce color, w = searchlight intensity
    bounce_and_beam: vec4<f32>,
    // x = palette blend factor [0..1] between color and color_b
    palette_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: f32,
    @location(4) color_b: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_pos = in.pos;
    out.normal = normalize(in.normal);
    out.color = mix(in.color, in.color_b, u.palette_params.x);
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);

    // 1. Translucent volumetric searchlight cones, headlight beams, and steam plumes (emissive >= 1.5)
    if (in.emissive >= 1.5) {
        return in.color;
    }

    // 2. Emissive windows, clock faces, train carriages, headlights, taillights, and beacons
    if (in.emissive >= 0.5) {
        let spatial = in.world_pos.x * 1.7 + in.world_pos.y * 2.3 + in.world_pos.z * 1.3;
        let pulse = 0.92 + 0.08 * sin(u.light_and_time.w * 2.1 + spatial);
        let boost = mix(1.0, 1.24, clamp(in.emissive - 0.5, 0.0, 1.0)) * pulse;
        return vec4<f32>( clamp(in.color.rgb * boost, vec3<f32>(0.0), vec3<f32>(1.0)), in.color.a );
    }

    // 3. Opaque 3D Gothic Architecture, Viaducts, Walkways, Vehicles & Airship Hulls
    let key_dir = normalize(u.light_and_time.xyz);
    let rim_dir = normalize(vec3<f32>(-key_dir.x, 0.45, -key_dir.z));

    let ndotl = max(dot(n, key_dir), 0.0);
    let ndotrim = max(dot(n, rim_dir), 0.0);
    let up_factor = max(n.y, 0.0);

    // Deep canyon ambient occlusion: lower street chasms are dramatic and shadowed,
    // while upper setbacks, buttresses, and spires catch crisp moonlight/rim light.
    let height_ao = smoothstep(-0.8, 12.0, in.world_pos.y);
    let canyon_ao = mix(0.35, 1.10, height_ao);

    // Warm upward bounce light from street avenues (y ~ 0.2..3.2) onto vertical canyon facades
    let street_proximity = (1.0 - smoothstep(0.0, 3.2, abs(in.world_pos.y - 1.35)))
        * smoothstep(0.05, 0.35, in.world_pos.y);
    let side_wall = 1.0 - abs(n.y) * 0.75;
    let warm_bounce = u.bounce_and_beam.xyz * street_proximity * side_wall * 0.18;

    // Subtle horizontal masonry course shading so towering facades have gothic scale
    let masonry_band = 0.96 + 0.04 * sin(in.world_pos.y * 14.0);

    // Combine ambient + key directional + rim + rooftop zenith catch
    let diffuse = (0.29 + 0.57 * ndotl + 0.21 * ndotrim + 0.11 * up_factor) * canyon_ao * masonry_band;
    var lit_rgb = in.color.rgb * diffuse + warm_bounce;

    // Rain-slicked wet asphalt puddle sheen on street-level roadbeds (y < 0.06, upward-facing)
    if (n.y > 0.92 && in.world_pos.y > -0.02 && in.world_pos.y < 0.055) {
        let puddle = sin(in.world_pos.x * 1.9 + in.world_pos.z * 1.2)
            * cos(in.world_pos.x * 1.3 - in.world_pos.z * 2.1);
        let wetness = smoothstep(-0.15, 0.65, puddle);
        let wet_sheen = u.bounce_and_beam.xyz * (0.07 + 0.11 * wetness)
            + vec3<f32>(0.05, 0.06, 0.08) * wetness;
        lit_rgb = lit_rgb + wet_sheen;
    }

    // Plinth abyss fade: smoothly fade the bottom of the foundation plinth into the black void
    let abyss_fade = smoothstep(-1.75, -0.15, in.world_pos.y);
    lit_rgb = lit_rgb * abyss_fade;

    return vec4<f32>(clamp(lit_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), in.color.a);
}
