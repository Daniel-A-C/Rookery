# Legacy ASCII Aesthetics Archive

This archive preserves the implementation of the 7 exploratory ASCII aesthetic mock-ups developed during the aesthetic research phase of **The Living ASCII Rookery**.

In accordance with architectural decisions, the platform has standardized strictly on **Option 2: Strict ANSI Roguelike Grid** (Caves of Qud / Dwarf Fortress style). The code below is archived for historical reference and future experimentation.

---

## 1. Summary of Archived Aesthetic Mock-Ups

1. **Style 0: Classic SDF Woodland (Baseline)**:
   - Curved continuous vector branches rasterized with Signed Distance Field (SDF) glyphs. Multi-column striated bark, atmospheric mist, undulating multi-layer ground, and 3-character songbird sprites (`< o \`).
2. **Style 2: Full-Screen ASCII Matrix & Dithering**:
   - Monospace dither grid covering every display pixel. 4-level luminance thresholding (` `, `.`, `:`, `*`, `%`, `#`, `@`), radial moonbeams, and trailing starlight.
3. **Style 3: Multi-Line Classic ASCII Art & Parallax**:
   - Multi-row character art sprites for trees, foliage blocks, and 3×3 bird sprites (`[" _ ", "(v)", "//\\"]`). Text-mode meadow horizon and 16-color Bayer dither sky bands.
4. **Style 4: Retro Phosphor CRT Terminal**:
   - Amber/P3 phosphor glow, horizontal raster scanlines, screen curvature vignette, and phosphor bloom persistence.
5. **Style 5: Sumi-e Ink & Botanical Field Journal**:
   - Aged parchment paper background (`#F4EEDD`), calligraphic brush-stroke bark (`墨`), sepia ink washes, and red vermilion seal stamp signature (`印`).
6. **Style 6: Jewel-Box Stained-Glass Mosaic**:
   - Luminous vibrant stained-glass facets with lead came zinc borders (`#`). Radiant jewel tones (sapphire, emerald, amethyst, ruby).
7. **Style 7: 3D Volumetric ASCII Tree Visualizer**:
   - True 3D perspective projection with real-time camera orbit yaw/pitch, depth-sorted glyph instances with depth-fog attenuation and wind sway.

---

## 2. Archived Shaders (`ascii_bg.wgsl`)

```wgsl
// Style 0: Classic SDF Woodland (Baseline)
fn bg_classic(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let sky_zenith = vec3<f32>(0.020, 0.032, 0.068);
    let sky_mid = vec3<f32>(0.048, 0.068, 0.118);
    let sky_glow = vec3<f32>(0.115, 0.098, 0.142);
    let sky_horizon = vec3<f32>(0.195, 0.142, 0.125);

    var sky = mix(sky_zenith, sky_mid, smoothstep(0.0, 0.48, uv.y));
    sky = mix(sky, sky_glow, smoothstep(0.48, 0.78, uv.y));
    sky = mix(sky, sky_horizon, smoothstep(0.78, 0.90, uv.y));

    if (uv.y < 0.68) {
        let grid_size = 36.0;
        let cell = floor(uv * res / grid_size);
        let star_rnd = hash21(cell);
        if (star_rnd > 0.952) {
            let star_pos = (cell + vec2<f32>(hash21(cell + 1.0), hash21(cell + 2.0))) * grid_size / res;
            let d = length((uv - star_pos) * res);
            let twinkle = 0.55 + 0.45 * sin(u.time * (1.6 + star_rnd * 3.2) + star_rnd * 100.0);
            let star_alpha = smoothstep(1.9, 0.0, d) * twinkle * (1.0 - uv.y / 0.68) * 0.60;
            let star_color = mix(
                vec3<f32>(0.92, 0.95, 1.0),
                vec3<f32>(1.0, 0.88, 0.65),
                fract(star_rnd * 7.0)
            );
            sky += star_color * star_alpha;
        }
    }

    if (uv.y > 0.74 && uv.y < 0.92) {
        let mist_wave1 = sin(uv.x * 4.5 + u.time * 0.18) * 0.5 + 0.5;
        let mist_wave2 = cos(uv.x * 9.2 - u.time * 0.11) * 0.5 + 0.5;
        let mist_factor = mist_wave1 * 0.6 + mist_wave2 * 0.4;
        let mist_envelope = smoothstep(0.74, 0.85, uv.y) * smoothstep(0.92, 0.86, uv.y);
        let mist_color = vec3<f32>(0.085, 0.095, 0.115);
        sky = mix(sky, sky + mist_color * 0.35, mist_envelope * mist_factor);
    }

    let ground_y = 0.898 + 0.014 * sin(uv.x * 3.8 + 0.6) + 0.007 * sin(uv.x * 11.2 - 0.4) + 0.003 * sin(uv.x * 24.0);
    if (uv.y > ground_y) {
        let moss_crest = vec3<f32>(0.065, 0.088, 0.052);
        let soil_top = vec3<f32>(0.045, 0.048, 0.038);
        let soil_deep = vec3<f32>(0.018, 0.020, 0.022);

        let ground_blend = smoothstep(ground_y, ground_y + 0.008, uv.y);
        let moss_to_soil = smoothstep(ground_y + 0.008, ground_y + 0.032, uv.y);
        let soil_to_deep = smoothstep(ground_y + 0.032, ground_y + 0.10, uv.y);

        var ground_col = mix(moss_crest, soil_top, moss_to_soil);
        ground_col = mix(ground_col, soil_deep, soil_to_deep);
        sky = mix(sky, ground_col, ground_blend);
    }

    return sky;
}

// Style 2: Full-Screen ASCII Matrix & Dithering
fn bg_matrix_dither(uv: vec2<f32>) -> vec3<f32> {
    var col = mix(vec3<f32>(0.008, 0.014, 0.024), vec3<f32>(0.018, 0.032, 0.045), uv.y);
    let moon_pos = vec2<f32>(0.78, 0.20);
    let d_moon = length((uv - moon_pos) * vec2<f32>(1.6, 1.0));
    col += vec3<f32>(0.04, 0.08, 0.11) * exp(-d_moon * 4.5);
    return col;
}

// Style 3: Multi-Line Classic ASCII Art & Parallax
fn bg_multiline_parallax(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let dither = (hash21(floor(uv * res * 0.5)) - 0.5) * 0.035;
    let y_q = floor((uv.y + dither) * 7.0) / 7.0;
    let c_top = vec3<f32>(0.035, 0.030, 0.085);
    let c_mid = vec3<f32>(0.075, 0.065, 0.165);
    let c_bot = vec3<f32>(0.155, 0.110, 0.180);
    var col = mix(c_top, c_mid, smoothstep(0.0, 0.55, y_q));
    col = mix(col, c_bot, smoothstep(0.55, 0.90, y_q));
    return col;
}

// Style 4: Retro Phosphor CRT Terminal
fn bg_crt_phosphor(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    var cc = uv - 0.5;
    let dist_sq = dot(cc, cc);
    let cuv = uv + cc * (dist_sq * 0.12);
    if (cuv.x < 0.0 || cuv.x > 1.0 || cuv.y < 0.0 || cuv.y > 1.0) {
        return vec3<f32>(0.002, 0.003, 0.002);
    }
    var col = vec3<f32>(0.012, 0.022, 0.010);
    let scanline = sin(cuv.y * res.y * 3.14159) * 0.5 + 0.5;
    col *= (0.75 + 0.25 * scanline);
    let vignette = smoothstep(0.72, 0.15, length(cc));
    col *= vignette;
    return col;
}

// Style 5: Sumi-e Ink & Botanical Field Journal
fn bg_sumie_journal(uv: vec2<f32>) -> vec3<f32> {
    let paper_base = vec3<f32>(0.965, 0.945, 0.885);
    let paper_grain = (hash21(uv * 180.0) - 0.5) * 0.030;
    let edge_vignette = smoothstep(0.85, 0.20, length(uv - 0.5));
    var col = (paper_base + paper_grain) * (0.92 + 0.08 * edge_vignette);
    return col;
}

// Style 6: Jewel-Box Stained-Glass Mosaic
fn bg_stained_glass(uv: vec2<f32>) -> vec3<f32> {
    let c1 = vec3<f32>(0.065, 0.025, 0.095);
    let c2 = vec3<f32>(0.025, 0.055, 0.095);
    var col = mix(c1, c2, uv.x);
    let d_center = length((uv - vec2<f32>(0.5, 0.45)) * vec2<f32>(1.2, 1.0));
    col += vec3<f32>(0.12, 0.08, 0.16) * exp(-d_center * 3.2);
    return col;
}

// Style 7: 3D Volumetric ASCII Tree Visualizer
fn bg_volumetric_3d(uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let c_zenith = vec3<f32>(0.012, 0.018, 0.035);
    let c_horizon = vec3<f32>(0.045, 0.065, 0.095);
    var col = mix(c_zenith, c_horizon, smoothstep(0.1, 0.88, uv.y));
    let d_depth = length(uv - vec2<f32>(0.5, 0.62));
    col += vec3<f32>(0.025, 0.045, 0.065) * exp(-d_depth * 4.0);
    return col;
}
```

---

## 3. Reference Implementation Notes

The full implementations of the generator methods previously located in `crates/rookery-render/src/ascii_scene.rs` have been safely archived in git history and preserved in the commits prior to this cleanup.

