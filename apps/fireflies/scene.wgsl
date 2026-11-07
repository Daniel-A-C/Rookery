struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    interactive: f32,
    pointer: vec2<f32>,
    action: f32,
    left_held: f32,
    params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(sin(vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)))) * 43758.5453);
    return fract(q.x + q.y);
}

fn rotate_hue(c: vec3<f32>, angle: f32) -> vec3<f32> {
    let k = vec3<f32>(0.57735026, 0.57735026, 0.57735026);
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let rotated = c * cos_a + cross(k, c) * sin_a + k * dot(k, c) * (1.0 - cos_a);
    return clamp(rotated, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let aspect = u.resolution.x / max(u.resolution.y, 1.0);
    let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
    let glow_mult = max(u.params.y, 0.1);
    let hue_angle = u.params.w * 0.35;

    // Deep dusk gradient background
    var col = mix(vec3<f32>(0.01, 0.02, 0.05), vec3<f32>(0.05, 0.03, 0.08), uv.y);

    // Pointer interactive glow
    if (u.interactive > 0.5) {
        let ptr_uv = vec2<f32>(u.pointer.x / u.resolution.x, u.pointer.y / u.resolution.y);
        let ptr_p = vec2<f32>((ptr_uv.x - 0.5) * aspect, ptr_uv.y - 0.5);
        let dist = length(p - ptr_p);
        let glow = 0.015 / (dist + 0.02);
        col += rotate_hue(vec3<f32>(0.1, 0.3, 0.25), hue_angle) * glow;
    }

    // Expanding light wave when action / pulse is triggered
    let excite = max(u.action, u.left_held * 0.6);
    if (excite > 0.01) {
        let wave_r = (1.0 - u.action) * 0.85;
        let ring = exp(-abs(length(p) - wave_r) * 18.0) * u.action;
        col += rotate_hue(vec3<f32>(0.25, 0.55, 0.30), hue_angle) * ring * 0.35;
    }

    // Swarm of procedural fireflies
    for (var i = 0; i < 28; i++) {
        let fi = f32(i);
        let speed = 0.3 + hash21(vec2<f32>(fi, 1.0)) * 0.4;
        let t = u.time * speed + fi * 2.39;

        let fx = sin(t * 0.7 + fi) * 0.6 * aspect + cos(t * 0.3) * 0.1;
        let fy = cos(t * 0.8 + fi * 1.5) * 0.4 + sin(t * 0.5) * 0.1;
        let fly_pos = vec2<f32>(fx, fy);

        let d = length(p - fly_pos);
        let pulse = sin(u.time * 2.5 + fi * 3.7) * 0.5 + 0.5;
        let boost = (1.0 + excite * 1.6) * glow_mult;
        let intensity = (0.0018 / (d * d + 0.0003)) * (pulse * 0.7 + 0.3) * boost;

        let base_col = mix(vec3<f32>(0.4, 0.9, 0.2), vec3<f32>(0.9, 0.8, 0.2), pulse);
        let firefly_col = rotate_hue(base_col, hue_angle);
        col += firefly_col * intensity;
    }

    return vec4<f32>(col, 1.0);
}

