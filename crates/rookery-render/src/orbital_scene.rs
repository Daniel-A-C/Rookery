//! Orbital Haven v2.0 Scene (Slow-Burn Sci-Fi Megastructure & Autonomous Ecosystem).
//!
//! Hybrid 3D planetary horizon shader + depth-sorted multi-layered SDF glyph megastructure
//! featuring counter-rotating habitat rings, tracking solar wings, thermal radiators,
//! hydroponic biodomes, orbital drydocks, 4 autonomous robot classes, 6 spacecraft classes,
//! asteroid mining lasers, and toggleable diegetic telemetry overlays.

use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use rookery_core::Activity;
use rookery_core::orbital::{
    ModuleKind, OrbitalHavenState, RobotKind, RobotTask, STATION_CENTER, ShipClass, ShuttleState,
};

use crate::atlas::GlyphPass;
use crate::fullscreen;
use crate::{Gpu, Pointer, PointerAction, Scene, SceneInput};

const MAX_INSTANCES: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OrbitalUniforms {
    resolution: [f32; 2],
    time: f32,
    orbit_angle: f32,
    camera_offset: [f32; 2],
    planet_biome: f32,
    hud_intensity: f32,
    scan_center: [f32; 2],
    scan_progress: f32,
    station_tier: f32,
}

pub struct OrbitalHavenScene {
    pub station: Arc<Mutex<OrbitalHavenState>>,
    pub audio: Option<crate::audio::AudioEngine>,
    width: f32,
    height: f32,
    scale_factor: f32,
    time: f64,

    bg_pipeline: wgpu::RenderPipeline,
    bg_uniforms_buf: wgpu::Buffer,
    bg_bind_group: wgpu::BindGroup,

    glyphs: GlyphPass,

    last_pointer: Option<Pointer>,
    is_dragging: bool,
    drag_accum: f32,
    interactive_active: bool,
}

impl OrbitalHavenScene {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        station: Arc<Mutex<OrbitalHavenState>>,
    ) -> Self {
        let device = &gpu.device;

        let bg_source = include_str!("orbital.wgsl");
        let bg_layout = fullscreen::uniform_layout(device, "orbital-bg-layout");
        let bg_uniforms_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("orbital-bg-uniforms"),
            size: std::mem::size_of::<OrbitalUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("orbital-bg-bind-group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bg_uniforms_buf.as_entire_binding(),
            }],
        });
        let bg_pipeline = fullscreen::pipeline(
            device,
            "orbital-bg-pipeline",
            bg_source,
            format,
            Some(&bg_layout),
            None,
        );

        let glyphs = GlyphPass::new(gpu, format, MAX_INSTANCES);

        Self {
            station,
            audio: None,
            width: 1280.0,
            height: 720.0,
            scale_factor: 1.0,
            time: 0.0,
            bg_pipeline,
            bg_uniforms_buf,
            bg_bind_group,
            glyphs,
            last_pointer: None,
            is_dragging: false,
            drag_accum: 0.0,
            interactive_active: false,
        }
    }

    pub fn with_audio(mut self, audio: Option<crate::audio::AudioEngine>) -> Self {
        self.audio = audio;
        self
    }

    fn world_to_screen(&self, pt: [f32; 2], cam: [f32; 2]) -> [f32; 2] {
        [
            ((pt[0] - cam[0]) / 1000.0) * self.width,
            ((pt[1] - cam[1]) / 750.0) * self.height,
        ]
    }

    fn screen_to_world(&self, pt: Pointer, cam: [f32; 2]) -> [f32; 2] {
        [
            (pt.x / self.width) * 1000.0 + cam[0],
            (pt.y / self.height) * 750.0 + cam[1],
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn push_glyph(
        &mut self,
        c: char,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        depth: f32,
        color: [f32; 4],
        sway: f32,
    ) {
        self.glyphs.push(c, pos, size, rotation, depth, color, sway);
    }

    fn push_text(
        &mut self,
        text: &str,
        start_pos: [f32; 2],
        char_size: [f32; 2],
        depth: f32,
        color: [f32; 4],
    ) {
        let step_x = char_size[0] * 0.68;
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = start_pos[0] + (i as f32) * step_x;
            self.push_glyph(ch, [x, start_pos[1]], char_size, 0.0, depth, color, 0.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_dotted_beam(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        steps: usize,
        glyph: char,
        size: [f32; 2],
        depth: f32,
        color: [f32; 4],
    ) {
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let rot = dy.atan2(dx);
        for i in 1..steps {
            let t = i as f32 / steps as f32;
            let p = [from[0] + dx * t, from[1] + dy * t];
            self.push_glyph(glyph, p, size, rot, depth, color, 0.0);
        }
    }

    fn generate_instances(&mut self) {
        self.glyphs.clear();

        let st_arc = self.station.clone();
        let st = match st_arc.lock() {
            Ok(s) => s,
            Err(_) => return,
        };

        let cam = st.camera_offset;
        let scale = (self.height / 750.0).clamp(0.55, 2.2);
        let t = self.time as f32;
        let center_scr = self.world_to_screen(STATION_CENTER, cam);

        // Color Palettes
        let titanium_bright = [0.88, 0.92, 0.98, 0.98];
        let titanium_mid = [0.58, 0.65, 0.76, 0.95];
        let truss_steel = [0.34, 0.42, 0.52, 0.92];
        let blueprint_cyan = [0.22, 0.82, 0.98, 0.55];
        let solar_azure = [0.14, 0.48, 0.92, 0.96];
        let solar_gold_trim = [0.92, 0.72, 0.28, 0.92];
        let window_warm = [1.0, 0.86, 0.42, 0.96];
        let cyan_core = [0.28, 0.92, 1.0, 0.96];
        let radiator_glow = [0.96, 0.34, 0.18, 0.92];
        let biodome_flora = [0.24, 0.88, 0.54, 0.95];

        // =====================================================================
        // LAYER 1: ORBITAL ASTEROID BELT & DISTANT RELAYS (depth 0.03 .. 0.07)
        // =====================================================================
        for ast in &st.asteroids {
            let apos = self.world_to_screen(ast.pos, cam);
            let r_scr = ast.radius * scale;
            let rock_col = match ast.richness {
                2 => [0.58, 0.52, 0.44, 0.94],
                1 => [0.46, 0.50, 0.58, 0.94],
                _ => [0.44, 0.42, 0.40, 0.92],
            };
            let vein_col = match ast.richness {
                2 => [0.98, 0.82, 0.32, 0.95], // Gold/Helium-3
                1 => [0.38, 0.88, 0.98, 0.95], // Crystalline Titanium
                _ => [0.85, 0.52, 0.28, 0.88], // Iron/Copper
            };

            // Core boulder silhouette
            self.push_glyph(
                '@',
                apos,
                [r_scr * 2.1, r_scr * 1.7],
                ast.rotation,
                0.04,
                rock_col,
                0.0,
            );
            self.push_glyph(
                '#',
                [apos[0] - r_scr * 0.35, apos[1] + r_scr * 0.15],
                [r_scr * 1.4, r_scr * 1.3],
                -ast.rotation * 0.7,
                0.045,
                [rock_col[0] * 0.8, rock_col[1] * 0.8, rock_col[2] * 0.8, 0.9],
                0.0,
            );
            // Glinting ore veins
            if ast.ore_remaining > 15.0 {
                let glint = 0.65 + 0.35 * (t * 2.5 + ast.id as f32).sin();
                self.push_glyph(
                    '*',
                    [
                        apos[0] + ast.rotation.cos() * r_scr * 0.35,
                        apos[1] + ast.rotation.sin() * r_scr * 0.25,
                    ],
                    [10.0 * scale, 10.0 * scale],
                    t,
                    0.05,
                    [vein_col[0], vein_col[1], vein_col[2], glint],
                    0.0,
                );
            }
        }

        // =====================================================================
        // LAYER 2: HORIZONTAL MAIN TRUSS & SOLAR WINGS (depth 0.11 .. 0.15)
        // =====================================================================
        // Continuous horizontal cross-beam spanning port to starboard
        for step in -9..=9i32 {
            if step == 0 {
                continue;
            }
            let bx = STATION_CENTER[0] + step as f32 * 26.0;
            let b_scr = self.world_to_screen([bx, STATION_CENTER[1]], cam);
            self.push_glyph(
                '=',
                b_scr,
                [26.0 * scale, 13.0 * scale],
                0.0,
                0.12,
                truss_steel,
                0.0,
            );
            if step % 2 == 0 {
                self.push_glyph(
                    '+',
                    b_scr,
                    [15.0 * scale, 15.0 * scale],
                    0.0,
                    0.125,
                    titanium_mid,
                    0.0,
                );
            }
        }

        // Continuous elliptical structural ring tracks (inner habitat wheel & outer gravity wheel)
        for (is_outer, rx, base_rot) in [
            (false, 94.0f32, st.ring_rotation),
            (true, 148.0f32, st.outer_ring_rotation),
        ] {
            if is_outer && st.outer_ring_sectors == 0 && st.epoch.tier_number() < 3 {
                continue;
            }
            let ry = rx * 0.43;
            let track_steps = if is_outer { 48 } else { 36 };
            for step in 0..track_steps {
                let angle = base_rot + (step as f32 / track_steps as f32) * TAU;
                let world_pt = [
                    STATION_CENTER[0] + angle.cos() * rx,
                    STATION_CENTER[1] + angle.sin() * ry,
                ];
                let t_scr = self.world_to_screen(world_pt, cam);
                let is_front = angle.sin() >= 0.0;
                let track_depth = if is_front {
                    if is_outer { 0.275 } else { 0.255 }
                } else if is_outer {
                    0.082
                } else {
                    0.092
                };
                let shade = if is_front { 0.95 } else { 0.68 };
                let tan_dx = -angle.sin() * rx;
                let tan_dy = angle.cos() * ry;
                let tangent_rot = tan_dy.atan2(tan_dx);
                self.push_glyph(
                    '-',
                    t_scr,
                    [16.0 * scale, 10.0 * scale],
                    tangent_rot,
                    track_depth,
                    [
                        titanium_mid[0] * shade,
                        titanium_mid[1] * shade,
                        titanium_mid[2] * shade,
                        0.88,
                    ],
                    0.0,
                );
            }
        }

        // Render Solar Array Wings, Radiators, Spine, Biodomes, Drydocks, Comms, and Rings
        for m in &st.modules {
            let complete = m.is_complete();
            let prog = m.build_progress;

            match m.kind {
                ModuleKind::SolarArrayWing => {
                    if !complete && prog < 0.01 {
                        continue;
                    }
                    let w_scr = self.world_to_screen(m.anchor_pos, cam);
                    let shimmer = 0.82 + 0.18 * (t * 1.6 + m.slot_index as f32 * 0.7).sin();

                    // Vertical collector mast
                    self.push_glyph(
                        '|',
                        w_scr,
                        [12.0 * scale, 96.0 * scale * prog.max(0.35)],
                        0.0,
                        0.13,
                        solar_gold_trim,
                        0.0,
                    );

                    // Upper and lower 3-row photovoltaic blanket panels
                    for row in 1..=3 {
                        let row_f = row as f32;
                        let dy = (14.0 + row_f * 15.0) * scale;
                        let row_built = prog >= (row_f - 0.5) / 3.0;
                        let col = if complete || row_built {
                            [
                                solar_azure[0] * shimmer,
                                solar_azure[1] * shimmer,
                                solar_azure[2],
                                0.95,
                            ]
                        } else {
                            blueprint_cyan
                        };
                        let ch = if complete || row_built { '#' } else { ':' };

                        for sign in [-1.0f32, 1.0f32] {
                            let py = w_scr[1] + sign * dy;
                            self.push_glyph(
                                ch,
                                [w_scr[0] - 6.0 * scale, py],
                                [16.0 * scale, 15.0 * scale],
                                0.0,
                                0.135,
                                col,
                                0.0,
                            );
                            self.push_glyph(
                                ch,
                                [w_scr[0] + 6.0 * scale, py],
                                [16.0 * scale, 15.0 * scale],
                                0.0,
                                0.135,
                                col,
                                0.0,
                            );
                        }
                    }
                }

                ModuleKind::ThermalRadiator => {
                    if !complete && prog < 0.01 {
                        continue;
                    }
                    let r_scr = self.world_to_screen(m.anchor_pos, cam);
                    let pulse = 0.75 + 0.25 * (t * 2.2 + m.slot_index as f32).sin();
                    let col = if complete {
                        [
                            radiator_glow[0],
                            radiator_glow[1] * pulse,
                            radiator_glow[2],
                            0.90,
                        ]
                    } else {
                        blueprint_cyan
                    };
                    let side = if m.anchor_pos[0] >= STATION_CENTER[0] {
                        1.0
                    } else {
                        -1.0
                    };
                    for fin in 0..3 {
                        let fx = r_scr[0] + side * (fin as f32 * 13.0 * scale);
                        self.push_glyph(
                            '=',
                            [fx, r_scr[1]],
                            [16.0 * scale, 12.0 * scale],
                            0.0,
                            0.14,
                            col,
                            0.0,
                        );
                    }
                }

                ModuleKind::SpineTruss => {
                    if !complete && prog < 0.01 && m.slot_index > st.spine_segments + 1 {
                        continue;
                    }
                    let s_scr = self.world_to_screen(m.anchor_pos, cam);
                    let col = if complete {
                        titanium_mid
                    } else {
                        blueprint_cyan
                    };
                    self.push_glyph(
                        'H',
                        s_scr,
                        [24.0 * scale, 30.0 * scale],
                        0.0,
                        0.18,
                        col,
                        0.0,
                    );
                    self.push_glyph(
                        '|',
                        [s_scr[0] - 9.0 * scale, s_scr[1]],
                        [11.0 * scale, 30.0 * scale],
                        0.0,
                        0.175,
                        truss_steel,
                        0.0,
                    );
                    self.push_glyph(
                        '|',
                        [s_scr[0] + 9.0 * scale, s_scr[1]],
                        [11.0 * scale, 30.0 * scale],
                        0.0,
                        0.175,
                        truss_steel,
                        0.0,
                    );
                    if complete {
                        // Internal power conduit glow
                        self.push_glyph(
                            ':',
                            s_scr,
                            [11.0 * scale, 20.0 * scale],
                            0.0,
                            0.185,
                            cyan_core,
                            0.0,
                        );
                    }
                }

                ModuleKind::HydroponicBiodome => {
                    if !complete && prog < 0.01 {
                        continue;
                    }
                    let d_scr = self.world_to_screen(m.anchor_pos, cam);
                    // Strut connecting to hub
                    self.push_dotted_beam(
                        center_scr,
                        d_scr,
                        5,
                        '-',
                        [14.0 * scale, 10.0 * scale],
                        0.16,
                        truss_steel,
                    );
                    let ring_col = if complete {
                        [0.65, 0.88, 0.98, 0.92]
                    } else {
                        blueprint_cyan
                    };
                    self.push_glyph(
                        'O',
                        d_scr,
                        [30.0 * scale, 26.0 * scale],
                        0.0,
                        0.21,
                        ring_col,
                        0.0,
                    );
                    if complete {
                        self.push_glyph(
                            '☘',
                            d_scr,
                            [18.0 * scale, 18.0 * scale],
                            0.12 * (t * 0.8).sin(),
                            0.22,
                            biodome_flora,
                            0.0,
                        );
                        self.push_glyph(
                            '*',
                            [d_scr[0], d_scr[1] - 7.0 * scale],
                            [9.0 * scale, 9.0 * scale],
                            0.0,
                            0.225,
                            window_warm,
                            0.0,
                        );
                    }
                }

                ModuleKind::OrbitalDrydock => {
                    if !complete && prog < 0.01 {
                        continue;
                    }
                    let k_scr = self.world_to_screen(m.anchor_pos, cam);
                    let col = if complete {
                        titanium_bright
                    } else {
                        blueprint_cyan
                    };
                    self.push_glyph(
                        '[',
                        [k_scr[0] - 16.0 * scale, k_scr[1]],
                        [16.0 * scale, 26.0 * scale],
                        0.0,
                        0.20,
                        col,
                        0.0,
                    );
                    self.push_glyph(
                        '=',
                        k_scr,
                        [28.0 * scale, 14.0 * scale],
                        0.0,
                        0.195,
                        truss_steel,
                        0.0,
                    );
                    self.push_glyph(
                        ']',
                        [k_scr[0] + 16.0 * scale, k_scr[1]],
                        [16.0 * scale, 26.0 * scale],
                        0.0,
                        0.20,
                        col,
                        0.0,
                    );
                    if complete {
                        let strobe = ((t * 3.0 + m.slot_index as f32).sin() > 0.2) as i32 as f32;
                        self.push_glyph(
                            '.',
                            [k_scr[0], k_scr[1] - 9.0 * scale],
                            [10.0 * scale, 10.0 * scale],
                            0.0,
                            0.21,
                            [1.0, 0.65, 0.18, 0.4 + 0.6 * strobe],
                            0.0,
                        );
                    }
                }

                ModuleKind::CommsSensorArray => {
                    if !complete && prog < 0.01 {
                        continue;
                    }
                    let c_scr = self.world_to_screen(m.anchor_pos, cam);
                    let col = if complete {
                        titanium_bright
                    } else {
                        blueprint_cyan
                    };
                    self.push_glyph(
                        '|',
                        [c_scr[0], c_scr[1] + 10.0 * scale],
                        [10.0 * scale, 20.0 * scale],
                        0.0,
                        0.20,
                        truss_steel,
                        0.0,
                    );
                    self.push_glyph(
                        ')',
                        c_scr,
                        [18.0 * scale, 18.0 * scale],
                        FRAC_PI_2,
                        0.21,
                        col,
                        0.0,
                    );
                    if complete {
                        let wave_y = c_scr[1]
                            - (6.0 + ((t * 14.0 + m.slot_index as f32 * 5.0) % 18.0)) * scale;
                        let wave_alpha =
                            1.0 - ((c_scr[1] - wave_y) / (24.0 * scale)).clamp(0.0, 1.0);
                        self.push_glyph(
                            '~',
                            [c_scr[0], wave_y],
                            [12.0 * scale, 8.0 * scale],
                            0.0,
                            0.215,
                            [0.35, 0.92, 1.0, wave_alpha * 0.85],
                            0.0,
                        );
                    }
                }

                ModuleKind::HabitatRingSector | ModuleKind::OuterRingSector => {
                    let is_outer = m.kind == ModuleKind::OuterRingSector;
                    if is_outer && !complete && prog < 0.01 && st.epoch.tier_number() < 3 {
                        continue;
                    }
                    let total_slots = if is_outer { 16.0 } else { 12.0 };
                    let base_rot = if is_outer {
                        st.outer_ring_rotation
                    } else {
                        st.ring_rotation
                    };
                    let rx = if is_outer { 148.0 } else { 94.0 };
                    let ry = rx * 0.43;

                    let angle = base_rot + (m.slot_index as f32 / total_slots) * TAU;
                    let world_pt = [
                        STATION_CENTER[0] + angle.cos() * rx,
                        STATION_CENTER[1] + angle.sin() * ry,
                    ];
                    let r_scr = self.world_to_screen(world_pt, cam);

                    // True 3D back-to-front depth sorting:
                    // sin(angle) < 0 is the upper/far arc BEHIND the axial spine (depth ~0.09)
                    // sin(angle) >= 0 is the lower/near arc IN FRONT OF the axial spine (depth ~0.27)
                    let is_front = angle.sin() >= 0.0;
                    let ring_depth = if is_front {
                        if is_outer { 0.28 } else { 0.26 }
                    } else if is_outer {
                        0.085
                    } else {
                        0.095
                    };
                    let shade = if is_front { 1.0 } else { 0.72 };

                    // Radial tension spokes every 3rd sector (inner) or 4th sector (outer)
                    let spoke_div = if is_outer { 4 } else { 3 };
                    if m.slot_index % spoke_div == 0 && (complete || prog > 0.05 || !is_outer) {
                        self.push_dotted_beam(
                            center_scr,
                            r_scr,
                            if is_outer { 6 } else { 5 },
                            '-',
                            [16.0 * scale, 10.0 * scale],
                            ring_depth - 0.005,
                            [
                                truss_steel[0] * shade,
                                truss_steel[1] * shade,
                                truss_steel[2] * shade,
                                0.88,
                            ],
                        );
                    }

                    // Tangent angle along ellipse
                    let tan_dx = -angle.sin() * rx;
                    let tan_dy = angle.cos() * ry;
                    let tangent_rot = tan_dy.atan2(tan_dx);

                    if complete {
                        let hull_c = [
                            titanium_bright[0] * shade,
                            titanium_bright[1] * shade,
                            titanium_bright[2] * shade,
                            0.98,
                        ];
                        self.push_glyph(
                            '=',
                            r_scr,
                            [28.0 * scale, 18.0 * scale],
                            tangent_rot,
                            ring_depth,
                            hull_c,
                            0.0,
                        );
                        self.push_glyph(
                            'O',
                            r_scr,
                            [21.0 * scale, 18.0 * scale],
                            tangent_rot,
                            ring_depth + 0.002,
                            [
                                titanium_mid[0] * shade,
                                titanium_mid[1] * shade,
                                titanium_mid[2] * shade,
                                0.96,
                            ],
                            0.0,
                        );
                        // Lit crew viewport windows
                        self.push_glyph(
                            '.',
                            r_scr,
                            [11.0 * scale, 11.0 * scale],
                            0.0,
                            ring_depth + 0.004,
                            window_warm,
                            0.0,
                        );
                    } else {
                        // Holographic wireframe sector
                        let wire_col = if prog > 0.01 {
                            [0.28, 0.88, 1.0, 0.82]
                        } else {
                            [0.38, 0.56, 0.72, 0.52]
                        };
                        self.push_glyph(
                            ':',
                            r_scr,
                            [16.0 * scale, 15.0 * scale],
                            tangent_rot,
                            ring_depth,
                            wire_col,
                            0.0,
                        );
                    }
                }
            }
        }

        // =====================================================================
        // LAYER 3: CENTRAL AEGIS COMMAND CORE HUB (depth 0.19 .. 0.23)
        // =====================================================================
        self.push_glyph(
            'O',
            center_scr,
            [44.0 * scale, 44.0 * scale],
            t * 0.12,
            0.19,
            titanium_bright,
            0.0,
        );
        self.push_glyph(
            '@',
            center_scr,
            [32.0 * scale, 32.0 * scale],
            -t * 0.25,
            0.195,
            titanium_mid,
            0.0,
        );
        self.push_glyph(
            '+',
            center_scr,
            [24.0 * scale, 24.0 * scale],
            t * 0.4,
            0.20,
            cyan_core,
            0.0,
        );
        let core_pulse = 0.75 + 0.25 * (t * 3.5).sin();
        self.push_glyph(
            '*',
            center_scr,
            [14.0 * scale * core_pulse, 14.0 * scale * core_pulse],
            0.0,
            0.205,
            [0.95, 0.98, 1.0, 0.98],
            0.0,
        );

        // Dorsal & ventral navigation beacon strobes at ends of spine
        let strobe_on = (t * 2.8).sin() > 0.55;
        let spine_reach = (st.spine_segments.max(2) as f32 * 0.5 + 0.8) * 34.0;
        for sign in [-1.0f32, 1.0f32] {
            let tip = self.world_to_screen(
                [STATION_CENTER[0], STATION_CENTER[1] + sign * spine_reach],
                cam,
            );
            self.push_glyph(
                '^',
                tip,
                [14.0 * scale, 14.0 * scale],
                if sign < 0.0 { 0.0 } else { PI },
                0.20,
                titanium_bright,
                0.0,
            );
            if strobe_on {
                self.push_glyph(
                    '*',
                    [tip[0], tip[1] + sign * 8.0 * scale],
                    [13.0 * scale, 13.0 * scale],
                    0.0,
                    0.22,
                    [1.0, 0.28, 0.22, 0.96],
                    0.0,
                );
            }
        }

        // =====================================================================
        // LAYER 4: 6-CLASS SPACECRAFT FLEET & MINING LASERS (depth 0.32 .. 0.42)
        // =====================================================================
        for shuttle in &st.shuttles {
            let s_pos = self.world_to_screen(shuttle.pos, cam);
            let speed_sq = shuttle.vel[0] * shuttle.vel[0] + shuttle.vel[1] * shuttle.vel[1];
            let rot = if speed_sq > 0.5 {
                shuttle.vel[1].atan2(shuttle.vel[0])
            } else {
                0.0
            };
            let dir = [rot.cos(), rot.sin()];

            // If MiningSkiff is actively laser-mining an asteroid, draw extraction beam
            if let ShuttleState::MiningAsteroid { asteroid_id, .. } = shuttle.state
                && let Some(ast) = st.asteroids.iter().find(|a| a.id == asteroid_id)
            {
                let a_pos = self.world_to_screen(ast.pos, cam);
                let dist = ((a_pos[0] - s_pos[0]).powi(2) + (a_pos[1] - s_pos[1]).powi(2)).sqrt();
                if dist < 95.0 * scale {
                    let beam_pulse = 0.65 + 0.35 * (t * 18.0).sin();
                    self.push_dotted_beam(
                        s_pos,
                        a_pos,
                        6,
                        ':',
                        [12.0 * scale, 12.0 * scale],
                        0.33,
                        [0.25, 0.95, 1.0, beam_pulse],
                    );
                }
            }

            // If approaching and HUD enabled, draw subtle docking approach guide vector
            if st.hud_enabled && matches!(shuttle.state, ShuttleState::Approaching) {
                let dock_scr = self.world_to_screen(shuttle.target_dock, cam);
                self.push_dotted_beam(
                    s_pos,
                    dock_scr,
                    5,
                    '.',
                    [9.0 * scale, 9.0 * scale],
                    0.31,
                    [0.25, 0.85, 0.98, 0.32],
                );
            }

            // Thruster plume when moving
            if speed_sq > 2.0 {
                let plume_pos = [
                    s_pos[0] - dir[0] * 14.0 * scale,
                    s_pos[1] - dir[1] * 14.0 * scale,
                ];
                let tail_pos = [
                    s_pos[0] - dir[0] * 22.0 * scale,
                    s_pos[1] - dir[1] * 22.0 * scale,
                ];
                let flicker = 0.8 + 0.2 * (t * 24.0 + shuttle.id as f32).sin();
                self.push_glyph(
                    '*',
                    plume_pos,
                    [13.0 * scale * flicker, 13.0 * scale * flicker],
                    rot,
                    0.34,
                    [0.32, 0.85, 1.0, 0.90],
                    0.0,
                );
                self.push_glyph(
                    '.',
                    tail_pos,
                    [9.0 * scale, 9.0 * scale],
                    0.0,
                    0.335,
                    [0.22, 0.62, 0.98, 0.65],
                    0.0,
                );
            }

            // Multi-glyph articulated spacecraft hull by class
            match shuttle.class {
                ShipClass::BulkFreighter => {
                    let rear = [
                        s_pos[0] - dir[0] * 9.0 * scale,
                        s_pos[1] - dir[1] * 9.0 * scale,
                    ];
                    let front = [
                        s_pos[0] + dir[0] * 8.0 * scale,
                        s_pos[1] + dir[1] * 8.0 * scale,
                    ];
                    self.push_glyph(
                        '#',
                        rear,
                        [15.0 * scale, 13.0 * scale],
                        rot,
                        0.36,
                        solar_gold_trim,
                        0.0,
                    );
                    self.push_glyph(
                        '=',
                        s_pos,
                        [18.0 * scale, 14.0 * scale],
                        rot,
                        0.365,
                        titanium_mid,
                        0.0,
                    );
                    self.push_glyph(
                        '>',
                        front,
                        [15.0 * scale, 15.0 * scale],
                        rot,
                        0.37,
                        titanium_bright,
                        0.0,
                    );
                }
                ShipClass::HabLiner => {
                    self.push_glyph(
                        'O',
                        s_pos,
                        [18.0 * scale, 14.0 * scale],
                        rot,
                        0.36,
                        [0.96, 0.97, 1.0, 0.98],
                        0.0,
                    );
                    self.push_glyph(
                        '>',
                        [
                            s_pos[0] + dir[0] * 7.0 * scale,
                            s_pos[1] + dir[1] * 7.0 * scale,
                        ],
                        [15.0 * scale, 15.0 * scale],
                        rot,
                        0.365,
                        solar_gold_trim,
                        0.0,
                    );
                }
                ShipClass::MiningSkiff => {
                    self.push_glyph(
                        '#',
                        s_pos,
                        [16.0 * scale, 15.0 * scale],
                        rot,
                        0.36,
                        [0.92, 0.64, 0.22, 0.98],
                        0.0,
                    );
                    self.push_glyph(
                        ']',
                        [
                            s_pos[0] + dir[0] * 6.0 * scale,
                            s_pos[1] + dir[1] * 6.0 * scale,
                        ],
                        [14.0 * scale, 14.0 * scale],
                        rot,
                        0.365,
                        titanium_bright,
                        0.0,
                    );
                }
                ShipClass::ScienceCorvette => {
                    self.push_glyph(
                        'o',
                        s_pos,
                        [15.0 * scale, 15.0 * scale],
                        rot,
                        0.36,
                        cyan_core,
                        0.0,
                    );
                    self.push_glyph(
                        '>',
                        [
                            s_pos[0] + dir[0] * 6.0 * scale,
                            s_pos[1] + dir[1] * 6.0 * scale,
                        ],
                        [15.0 * scale, 14.0 * scale],
                        rot,
                        0.365,
                        titanium_bright,
                        0.0,
                    );
                }
                ShipClass::SolarClipper => {
                    self.push_glyph(
                        ')',
                        [
                            s_pos[0] + dir[0] * 4.0 * scale,
                            s_pos[1] + dir[1] * 4.0 * scale,
                        ],
                        [22.0 * scale, 22.0 * scale],
                        rot,
                        0.36,
                        [0.98, 0.88, 0.52, 0.92],
                        0.0,
                    );
                    self.push_glyph(
                        '>',
                        s_pos,
                        [14.0 * scale, 14.0 * scale],
                        rot,
                        0.365,
                        titanium_bright,
                        0.0,
                    );
                }
                ShipClass::PatrolInterceptor => {
                    self.push_glyph(
                        '>',
                        s_pos,
                        [15.0 * scale, 13.0 * scale],
                        rot,
                        0.36,
                        [0.75, 0.90, 1.0, 0.98],
                        0.0,
                    );
                }
            }
        }

        // =====================================================================
        // LAYER 5: AUTONOMOUS ROBOTICS & SPARK PARTICLES (depth 0.43 .. 0.52)
        // =====================================================================
        for robot in &st.robots {
            let r_pos = self.world_to_screen(robot.pos, cam);
            let boost_glow = if robot.boost_timer > 0.0 { 1.25 } else { 1.0 };

            match robot.kind {
                RobotKind::WelderDrone => {
                    self.push_glyph(
                        'o',
                        r_pos,
                        [11.0 * scale * boost_glow, 11.0 * scale * boost_glow],
                        robot.phase,
                        0.44,
                        cyan_core,
                        0.0,
                    );
                    self.push_glyph(
                        '+',
                        r_pos,
                        [8.0 * scale, 8.0 * scale],
                        -robot.phase,
                        0.445,
                        [1.0, 0.92, 0.55, 0.95],
                        0.0,
                    );
                }
                RobotKind::SpiderCrawler => {
                    self.push_glyph(
                        'x',
                        r_pos,
                        [12.0 * scale, 12.0 * scale],
                        (robot.phase * 2.0).sin() * 0.35,
                        0.44,
                        solar_gold_trim,
                        0.0,
                    );
                }
                RobotKind::CargoTug => {
                    self.push_glyph(
                        '#',
                        r_pos,
                        [12.0 * scale, 12.0 * scale],
                        0.0,
                        0.44,
                        [0.95, 0.72, 0.25, 0.95],
                        0.0,
                    );
                    if let RobotTask::HaulingCargo { to, .. } = robot.task {
                        let to_scr = self.world_to_screen(to, cam);
                        self.push_dotted_beam(
                            r_pos,
                            to_scr,
                            3,
                            '.',
                            [7.0 * scale, 7.0 * scale],
                            0.43,
                            [0.95, 0.72, 0.25, 0.35],
                        );
                    }
                }
                RobotKind::DefenseMicroSat => {
                    self.push_glyph(
                        '*',
                        r_pos,
                        [12.0 * scale, 12.0 * scale],
                        robot.phase,
                        0.44,
                        [0.48, 0.88, 1.0, 0.92],
                        0.0,
                    );
                }
            }
        }

        // Plasma welding sparks & mining ore embers
        for sp in &st.sparks {
            let p_scr = self.world_to_screen(sp.pos, cam);
            let alpha = (1.0 - sp.life / sp.max_life).clamp(0.0, 1.0);
            let col = match sp.color_kind {
                1 => [0.30, 0.92, 1.0, alpha],
                2 => [1.0, 0.65, 0.22, alpha],
                _ => [1.0, 0.90, 0.38, alpha],
            };
            self.push_glyph(
                '*',
                p_scr,
                [9.0 * scale * alpha.max(0.4), 9.0 * scale * alpha.max(0.4)],
                sp.life * 6.0,
                0.48,
                col,
                0.0,
            );
        }

        // =====================================================================
        // LAYER 6: DIEGETIC ORBITAL TELEMETRY HUD OVERLAY (depth 0.55 .. 0.65)
        // =====================================================================
        if st.hud_enabled {
            let hud_col = [0.35, 0.86, 0.98, 0.58];
            let hud_accent = [0.98, 0.78, 0.32, 0.68];
            let font_sz = [10.5 * scale, 14.0 * scale];

            // Top-left orbital command telemetry readout
            let header_left = format!(
                "[ AEGIS-VII // {} // {} ]",
                st.epoch.short_name().to_uppercase(),
                st.planet_biome.name().to_uppercase()
            );
            self.push_text(
                &header_left,
                [28.0 * scale, 28.0 * scale],
                font_sz,
                0.56,
                hud_col,
            );

            let sub_left = format!(
                "  MOD: {:02}/66  PWR: +{}MW  CREW: {}  DIR: {}",
                st.stats.total_modules,
                st.stats.power_mw,
                st.stats.crew_population,
                st.focus.name().to_uppercase()
            );
            self.push_text(
                &sub_left,
                [28.0 * scale, 46.0 * scale],
                [9.5 * scale, 12.5 * scale],
                0.56,
                hud_accent,
            );

            // Active module construction progress bar
            if let Some(m) = st
                .modules
                .iter()
                .find(|m| m.build_progress > 0.001 && !m.is_complete())
            {
                let m_pos = self.world_to_screen(st.module_world_pos(m), cam);
                let filled = ((m.build_progress * 8.0).round() as usize).min(8);
                let mut bar = String::with_capacity(12);
                bar.push('[');
                for i in 0..8 {
                    bar.push(if i < filled { '|' } else { '.' });
                }
                bar.push(']');
                self.push_text(
                    &bar,
                    [m_pos[0] - 28.0 * scale, m_pos[1] - 22.0 * scale],
                    [9.0 * scale, 12.0 * scale],
                    0.58,
                    [0.32, 0.95, 0.78, 0.82],
                );
            }
        }

        // Interactive cursor targeting reticle when in Direct Interaction Mode
        if self.interactive_active
            && let Some(ptr) = self.last_pointer
        {
            let r_sz = [16.0 * scale, 16.0 * scale];
            let ret_col = [0.38, 0.95, 1.0, 0.85];
            self.push_glyph(
                '[',
                [ptr.x - 14.0 * scale, ptr.y],
                r_sz,
                0.0,
                0.62,
                ret_col,
                0.0,
            );
            self.push_glyph(
                ']',
                [ptr.x + 14.0 * scale, ptr.y],
                r_sz,
                0.0,
                0.62,
                ret_col,
                0.0,
            );
            self.push_glyph(
                '+',
                [ptr.x, ptr.y],
                [10.0 * scale, 10.0 * scale],
                t * 1.5,
                0.62,
                [1.0, 0.82, 0.35, 0.85],
                0.0,
            );
        }

        // Sort instances back-to-front by depth so 3D ring occlusion is exact
        self.glyphs
            .instances
            .sort_by(|a, b| a.depth.total_cmp(&b.depth));
    }
}

impl Scene for OrbitalHavenScene {
    fn resize(&mut self, _gpu: &Gpu, width: u32, height: u32, scale_factor: f64) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.scale_factor = scale_factor.max(0.5) as f32;
    }

    fn update(&mut self, gpu: &Gpu, dt: Duration, input: &SceneInput) -> Activity {
        let dt_secs = dt.as_secs_f32().min(0.2);
        self.time += dt_secs as f64;
        self.interactive_active = input.interactive;

        let (
            activity,
            orbit_angle,
            cam_offset,
            planet_biome,
            hud_intensity,
            scan_center,
            scan_progress,
            station_tier,
        ) = {
            let mut guard = match self.station.lock() {
                Ok(g) => g,
                Err(_) => return Activity::Idle,
            };

            if input.interactive {
                if let Some(cur) = input.pointer {
                    if input.left_held {
                        if let Some(prev) = self.last_pointer {
                            let dx_scr = cur.x - prev.x;
                            let dy_scr = cur.y - prev.y;
                            let dist = (dx_scr * dx_scr + dy_scr * dy_scr).sqrt();
                            self.drag_accum += dist;
                            if self.drag_accum > 6.0 {
                                self.is_dragging = true;
                                let wx = -(dx_scr / self.width) * 1000.0;
                                let wy = -(dy_scr / self.height) * 750.0;
                                guard.pan_camera(wx, wy);
                            }
                        }
                    } else {
                        self.is_dragging = false;
                        self.drag_accum = 0.0;
                    }
                    self.last_pointer = Some(cur);
                }

                let world_pt = input
                    .pointer
                    .map(|p| self.screen_to_world(p, guard.camera_offset))
                    .unwrap_or(STATION_CENTER);

                match input.action {
                    PointerAction::LeftClick => {
                        if !self.is_dragging {
                            guard.interact_at(world_pt);
                            if let Some(ref a) = self.audio {
                                a.play_sfx(crate::audio::SoundEffect::ShuttleLaunch);
                            }
                        }
                    }
                    PointerAction::RightClick => {
                        guard.trigger_scan_pulse(world_pt);
                        if let Some(ref a) = self.audio {
                            a.play_sfx(crate::audio::SoundEffect::OrbitalScan);
                        }
                    }
                    PointerAction::None => {}
                }
            } else {
                self.is_dragging = false;
                self.drag_accum = 0.0;
                self.last_pointer = input.pointer;
            }

            let act = guard.update(dt_secs, input.interactive, None);
            (
                act,
                guard.orbit_angle,
                guard.camera_offset,
                guard.planet_biome.shader_index(),
                if guard.hud_enabled { 1.0 } else { 0.0 },
                guard.scan_pulse.center,
                if guard.scan_pulse.active {
                    guard.scan_pulse.progress
                } else {
                    0.0
                },
                guard.epoch.tier_number() as f32,
            )
        };

        let orb_uniforms = OrbitalUniforms {
            resolution: [self.width, self.height],
            time: self.time as f32,
            orbit_angle,
            camera_offset: cam_offset,
            planet_biome,
            hud_intensity,
            scan_center,
            scan_progress,
            station_tier,
        };
        gpu.queue
            .write_buffer(&self.bg_uniforms_buf, 0, bytemuck::bytes_of(&orb_uniforms));

        self.generate_instances();
        self.glyphs
            .upload(gpu, [self.width, self.height], self.time as f32, 0.0);

        activity
    }

    fn render(&mut self, _gpu: &Gpu, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        // Pass 1: 3D Planetary horizon & atmospheric scattering shader
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("orbital-bg-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.005,
                            g: 0.010,
                            b: 0.020,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.bg_pipeline);
            rpass.set_bind_group(0, &self.bg_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Pass 2: Depth-sorted SDF Glyph Megastructure, Fleet, Robotics & HUD
        self.glyphs.render(view, encoder);
    }
}
