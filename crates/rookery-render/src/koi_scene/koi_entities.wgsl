// Pass 2: Painterly 2D Entity Shader for Tranquil Koi Pond.
// Renders shadows, sculpted Nishikigoi with procedural organic markings, translucent fins,
// veined lily pads, botanical lotus petals & maple leaves, concentric ripple crests, and Zen props.

struct EntityUniforms {
    resolution_and_time: vec4<f32>, // xy = resolution, z = time, w = ambient_mul
    water_absorb: vec4<f32>,        // rgb = depth absorption water tint, w = lantern_glow
    sun_tint: vec4<f32>,            // rgb = sun/moon highlight tint, w = pad
}

@group(0) @binding(0)
var<uniform> u: EntityUniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,    // screen pixels (top-left origin)
    @location(1) uv: vec2<f32>,     // local UV (e.g. koi u=head..tail, v=-1..1; disc u,v in -1..1)
    @location(2) color: vec4<f32>,  // primary color
    @location(3) accent: vec4<f32>, // secondary / pattern / vein color
    @location(4) params: vec4<f32>, // x = material_id, y = variety/subkind, z = seed/depth, w = extra
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) accent: vec4<f32>,
    @location(3) params: vec4<f32>,
}

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let res = max(u.resolution_and_time.xy, vec2<f32>(1.0, 1.0));
    let ndc_x = (v.pos.x / res.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (v.pos.y / res.y) * 2.0;
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    out.accent = v.accent;
    out.params = v.params;
    return out;
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

fn fbm2(p: vec2<f32>) -> f32 {
    return value_noise(p) * 0.65 + value_noise(p * 2.15 + vec2<f32>(7.1, 3.4)) * 0.35;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let mat = i32(in.params.x + 0.5);
    let uv = in.uv;
    let amb = u.resolution_and_time.w;

    // -------------------------------------------------------------------------
    // Material 0: Soft Gaussian Shadow / Glow Disc
    // -------------------------------------------------------------------------
    if (mat == 0) {
        let r2 = dot(uv, uv);
        if (r2 > 1.0) {
            discard;
        }
        let softness = max(in.params.y, 0.25);
        let alpha = exp(-r2 * (2.6 / softness)) * smoothstep(1.0, 0.15, r2) * in.color.a;
        return vec4<f32>(in.color.rgb, alpha);
    }

    // -------------------------------------------------------------------------
    // Material 1: Sculpted Nishikigoi Body (7 authentic varieties)
    // uv.x = 0.0 (snout) .. 1.0 (tail peduncle), uv.y = -1.0 .. 1.0 (left..right flank)
    // params.y = variety_idx (0..6), params.z = pattern_seed, params.w = swim_depth
    // -------------------------------------------------------------------------
    if (mat == 1) {
        let edge_d = abs(uv.y);
        if (edge_d > 1.0) {
            discard;
        }
        let variety = i32(in.params.y + 0.5);
        let pseed = in.params.z;
        let depth = in.params.w;

        // Soft anti-aliased body silhouette edge
        let alpha_edge = smoothstep(1.0, 0.84, edge_d) * smoothstep(0.0, 0.035, uv.x) * smoothstep(1.0, 0.96, uv.x);

        // Organic dorsal pattern coordinates
        let p_uv = vec2<f32>(uv.x * 4.2 + pseed * 1.7, uv.y * 1.9 + pseed * 0.9);
        let blotch = fbm2(p_uv);
        let fleck = fbm2(p_uv * 2.2 + vec2<f32>(11.3, 5.7));

        // Base colors
        let white_porcelain = vec3<f32>(0.97, 0.95, 0.90);
        let hi_vermilion = vec3<f32>(0.92, 0.23, 0.11);
        let sumi_ink = vec3<f32>(0.11, 0.11, 0.14);
        let gold_luster = vec3<f32>(0.96, 0.77, 0.20);
        let platinum = vec3<f32>(0.93, 0.95, 0.98);
        let asagi_blue = vec3<f32>(0.24, 0.37, 0.54);
        let asagi_coral = vec3<f32>(0.92, 0.39, 0.22);

        var body_col = white_porcelain;

        if (variety == 0) {
            // Kohaku: crisp vermilion-red dorsal saddles on snow-white
            let mask = smoothstep(0.48, 0.57, blotch) * smoothstep(0.95, 0.25, edge_d);
            body_col = mix(white_porcelain, hi_vermilion, mask);
        } else if (variety == 1) {
            // Taisho Sanke: ivory white + vermilion saddles + sumi ink spots
            let red_m = smoothstep(0.49, 0.58, blotch);
            let ink_m = smoothstep(0.67, 0.74, fleck) * smoothstep(0.12, 0.28, uv.x);
            body_col = mix(white_porcelain, hi_vermilion, red_m);
            body_col = mix(body_col, sumi_ink, ink_m * 0.92);
        } else if (variety == 2) {
            // Showa: deep lacquer black base with wrapping crimson & cream bands
            let red_m = smoothstep(0.50, 0.59, blotch);
            let wht_m = smoothstep(0.58, 0.66, fleck);
            body_col = mix(sumi_ink, hi_vermilion, red_m);
            body_col = mix(body_col, white_porcelain, wht_m * 0.85);
        } else if (variety == 3) {
            // Yamabuki Ogon: luminous metallic gold with subtle scale sheen
            let scales = 0.92 + 0.08 * sin(uv.x * 58.0) * cos(uv.y * 16.0);
            let sheen = smoothstep(0.55, 0.0, edge_d) * 0.16;
            body_col = gold_luster * scales + vec3<f32>(0.18, 0.14, 0.04) * sheen;
        } else if (variety == 4) {
            // Tancho: pure white body with solitary crimson crown circle on forehead
            let crown_d = length((uv - vec2<f32>(0.135, 0.0)) * vec2<f32>(1.0, 0.34));
            let crown = smoothstep(0.072, 0.048, crown_d);
            body_col = mix(white_porcelain, vec3<f32>(0.90, 0.14, 0.12), crown);
        } else if (variety == 5) {
            // Platinum Ogon: pearlescent silver-white with iridescent dorsal sheen
            let pearl = 0.94 + 0.06 * sin(uv.x * 46.0 + uv.y * 12.0);
            body_col = platinum * pearl + vec3<f32>(0.06, 0.08, 0.11) * (1.0 - edge_d);
        } else {
            // Asagi: reticulated indigo-blue dorsal scales with coral-orange flanks & cheeks
            let scale_net = sin(uv.x * 54.0) * sin(uv.y * 18.0);
            let dorsal_indigo = mix(asagi_blue * 0.78, asagi_blue * 1.18, smoothstep(-0.3, 0.5, scale_net));
            let flank_mask = smoothstep(0.38, 0.78, edge_d);
            body_col = mix(dorsal_indigo, asagi_coral, flank_mask * 0.88);
            // Soft silver-blue head cap
            let head_mask = smoothstep(0.18, 0.06, uv.x);
            body_col = mix(body_col, vec3<f32>(0.84, 0.88, 0.94), head_mask * 0.72);
        }

        // Cylindrical 3D dorsal shading (brighter along spine ridge, softer along flanks)
        let dorsal_curve = sqrt(max(0.0, 1.0 - edge_d * edge_d));
        let shade = mix(0.68, 1.08, dorsal_curve);
        body_col *= shade;

        // Subtle sunlit wet dorsal highlight
        let spec = pow(max(0.0, dorsal_curve), 5.0) * smoothstep(0.85, 0.15, uv.x);
        body_col += u.sun_tint.rgb * spec * 0.16;

        // Delicate dark eyes near the snout
        let eye_l = length((uv - vec2<f32>(0.095, 0.54)) * vec2<f32>(1.0, 0.42));
        let eye_r = length((uv - vec2<f32>(0.095, -0.54)) * vec2<f32>(1.0, 0.42));
        let eye_m = smoothstep(0.032, 0.018, min(eye_l, eye_r));
        body_col = mix(body_col, vec3<f32>(0.05, 0.05, 0.07), eye_m);

        // Underwater depth absorption: deeper koi blend softly into pond water color
        let absorb = clamp(depth * 0.54, 0.0, 0.58);
        body_col = mix(body_col * amb, u.water_absorb.rgb * 1.25, absorb);

        return vec4<f32>(body_col, alpha_edge * in.color.a);
    }

    // -------------------------------------------------------------------------
    // Material 2: Translucent Flowing Fins (pectoral, pelvic, dorsal, caudal)
    // uv.x = 0 (root) .. 1 (tip), uv.y = -1 .. 1 (span)
    // -------------------------------------------------------------------------
    if (mat == 2) {
        let depth = in.params.w;
        // Organic teardrop / fan fin mask
        let root_fade = smoothstep(0.0, 0.12, uv.x);
        let tip_fade = smoothstep(1.0, 0.45, uv.x);
        let width_at_x = sin(clamp(uv.x, 0.0, 1.0) * 3.14159) * 0.92 + 0.12;
        let span_mask = smoothstep(width_at_x, width_at_x * 0.55, abs(uv.y));
        let rays = 0.82 + 0.18 * sin(uv.y * 26.0 + uv.x * 4.0);

        let alpha = root_fade * tip_fade * span_mask * in.color.a * rays;
        if (alpha < 0.01) {
            discard;
        }
        let col = mix(in.color.rgb * amb, u.water_absorb.rgb * 1.2, clamp(depth * 0.50, 0.0, 0.52));
        return vec4<f32>(col, alpha);
    }

    // -------------------------------------------------------------------------
    // Material 3: Veined Botanical Lily Pad (uv in -1..1)
    // -------------------------------------------------------------------------
    if (mat == 3) {
        let r = length(uv);
        let ang = atan2(uv.y, uv.x);
        // Subtle organic wavy margin
        let margin = 0.94 + 0.04 * sin(ang * 7.0 + in.params.z) + 0.02 * cos(ang * 13.0);
        if (r > margin) {
            discard;
        }
        // Classic lily pad V-notch along +X axis
        let notch_half = 0.24 * smoothstep(0.0, 0.45, r);
        if (abs(ang) < notch_half && r > 0.06) {
            discard;
        }
        let edge_aa = smoothstep(margin, margin - 0.045, r);
        let notch_aa = smoothstep(notch_half, notch_half + 0.07, abs(ang) + smoothstep(0.14, 0.0, r));

        // Radial botanical veins + subtle upturned rim shadow
        let vein = pow(abs(sin(ang * 11.0 + sin(r * 5.0) * 0.3)), 10.0) * smoothstep(0.08, 0.78, r);
        let center_glow = smoothstep(0.55, 0.0, r) * 0.14;
        let rim = smoothstep(0.76, margin, r);

        var col = mix(in.color.rgb, in.accent.rgb, vein * 0.55 + center_glow);
        col = mix(col, in.color.rgb * 0.62, rim * 0.55);
        // Subtle dew highlight
        col += u.sun_tint.rgb * smoothstep(0.45, 0.0, length(uv - vec2<f32>(-0.22, -0.22))) * 0.09;

        return vec4<f32>(col, edge_aa * notch_aa * in.color.a);
    }

    // -------------------------------------------------------------------------
    // Material 4: Botanical Petal / Lotus Petal / Maple Leaf (uv in -1..1)
    // params.y: 0 = pointed lotus/sakura petal, 1 = notched sakura petal, 2 = 5-lobed maple leaf
    // -------------------------------------------------------------------------
    if (mat == 4) {
        let kind = i32(in.params.y + 0.5);
        var mask = 0.0;
        var grad = 0.5;

        if (kind == 2) {
            // 5-lobed autumn Japanese maple leaf
            let r = length(uv);
            let ang = atan2(uv.y, uv.x);
            let lobes = 0.48 + 0.34 * pow(max(0.0, cos(ang * 2.5)), 1.6) + 0.14 * cos(ang * 5.0);
            mask = smoothstep(lobes, lobes - 0.08, r);
            grad = clamp(r / max(lobes, 0.1), 0.0, 1.0);
        } else {
            // Pointed lotus petal (uv.x from -1 base to +1 tip, uv.y = -1..1)
            let t = uv.x * 0.5 + 0.5; // 0..1
            let half_w = sin(clamp(t, 0.0, 1.0) * 3.14159) * (1.0 - 0.18 * t);
            let d = abs(uv.y);
            mask = smoothstep(half_w, half_w - 0.12, d) * smoothstep(0.0, 0.08, t) * smoothstep(1.0, 0.92, t);
            if (kind == 1 && t > 0.78) {
                // Delicate sakura tip cleft
                let cleft = smoothstep(0.0, 0.22, abs(uv.y) + (0.94 - t) * 1.4);
                mask *= cleft;
            }
            grad = t;
        }

        if (mask < 0.02) {
            discard;
        }
        var col = mix(in.accent.rgb, in.color.rgb, smoothstep(0.08, 0.85, grad));
        // Soft central petal ridge highlight
        col += vec3<f32>(0.08) * smoothstep(0.35, 0.0, abs(uv.y)) * (1.0 - grad * 0.5);
        return vec4<f32>(col, mask * in.color.a);
    }

    // -------------------------------------------------------------------------
    // Material 5: Concentric Water Ripple / Wake Crest Ring (uv in -1..1)
    // -------------------------------------------------------------------------
    if (mat == 5) {
        let r = length(uv);
        if (r > 1.0) {
            discard;
        }
        let ring_count = max(in.params.y, 1.0);
        var ring_alpha = 0.0;

        // Primary outer crest
        let d0 = abs(r - 0.92);
        ring_alpha += exp(-(d0 * d0) / 0.0022);

        // Secondary trailing crests for pebble / gulp splashes
        if (ring_count >= 2.0) {
            let d1 = abs(r - 0.72);
            ring_alpha += exp(-(d1 * d1) / 0.0028) * 0.58;
        }
        if (ring_count >= 3.0) {
            let d2 = abs(r - 0.52);
            ring_alpha += exp(-(d2 * d2) / 0.0034) * 0.32;
        }

        let alpha = clamp(ring_alpha, 0.0, 1.0) * in.color.a;
        if (alpha < 0.01) {
            discard;
        }
        return vec4<f32>(in.color.rgb, alpha);
    }

    // -------------------------------------------------------------------------
    // Material 6: Solid Painterly Prop (mossy rocks, bamboo, lanterns, frogs, turtles, dragonflies)
    // params.y: 0 = mossy organic rock, 1 = smooth rounded capsule/prop, 2 = backlit lantern paper
    // -------------------------------------------------------------------------
    let sub = i32(in.params.y + 0.5);
    if (sub == 0) {
        let r = length(uv);
        let ang = atan2(uv.y, uv.x);
        let rock_edge = 0.88 + 0.08 * sin(ang * 3.0 + in.params.z) + 0.05 * cos(ang * 5.0 - in.params.z);
        if (r > rock_edge) {
            discard;
        }
        let aa = smoothstep(rock_edge, rock_edge - 0.06, r);
        let n = fbm2(uv * 3.5 + vec2<f32>(in.params.z, in.params.z * 0.6));
        let light = clamp(0.65 - uv.x * 0.25 - uv.y * 0.30, 0.35, 1.15);
        var col = in.color.rgb * light * (0.85 + 0.3 * n);
        // Soft painterly moss cap on upper-left face
        let moss_mask = smoothstep(0.42, 0.72, n - uv.y * 0.35) * in.params.w;
        col = mix(col, in.accent.rgb * (0.82 + 0.35 * light), moss_mask * 0.82);
        return vec4<f32>(col, aa * in.color.a);
    } else if (sub == 2) {
        // Warm backlit washi paper lantern shade
        let box_d = max(abs(uv.x), abs(uv.y));
        if (box_d > 1.0) {
            discard;
        }
        let aa = smoothstep(1.0, 0.90, box_d);
        let core_glow = exp(-dot(uv, uv) * 1.4);
        let frame = smoothstep(0.76, 0.92, box_d);
        var col = mix(in.color.rgb, in.accent.rgb, core_glow * 0.78);
        col = mix(col, vec3<f32>(0.24, 0.14, 0.08), frame * 0.82);
        return vec4<f32>(col, aa * in.color.a);
    } else {
        // Smooth rounded painterly ellipse / capsule (bamboo, turtle shell, frog, dragonfly, pellet)
        let r2 = dot(uv, uv);
        if (r2 > 1.0) {
            discard;
        }
        let aa = smoothstep(1.0, 0.76, r2);
        let dome = sqrt(max(0.0, 1.0 - r2));
        let col = mix(in.accent.rgb, in.color.rgb, 0.45 + 0.55 * dome);
        return vec4<f32>(col, aa * in.color.a);
    }
}
