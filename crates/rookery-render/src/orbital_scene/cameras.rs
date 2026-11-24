//! Perspective camera math, multi-feed resolver, and viewport layout rects
//! for *Orbital Haven v3* (Single / Main+PiP / 2x2 CCTV Wall).

use rookery_core::orbital::{
    CameraFeedId, CameraLayout, LandingPadId, OrbitalHavenState, PAD_B_POS, PAD_C_POS,
};

#[derive(Clone, Copy, Debug)]
pub struct ViewportRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub feed: CameraFeedId,
    /// `0` = primary/fullscreen, `1..=2` = PiP mini-monitor, `10..=13` = Grid2x2 quadrant.
    pub slot_tag: u8,
}

#[derive(Clone, Debug)]
pub struct ResolvedCamera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov_deg: f32,
    pub title: String,
    pub subtitle: String,
    pub is_interior: bool,
    pub is_uplink: bool,
}

pub fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt();
    if l > 1e-5 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 0.0, -1.0]
    }
}

/// Build an orthonormal `(right, up, forward)` basis from a forward vector.
pub fn basis_from_forward(forward: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let f = normalize3(forward);
    let world_up = if f[1].abs() > 0.96 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let r = normalize3(cross3(f, world_up));
    let u = normalize3(cross3(r, f));
    (r, u, f)
}

/// Column-major right-handed perspective view-projection matrix for wgpu (`z` in `[0, 1]`).
pub fn build_perspective_view_proj(
    eye: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
    fov_deg: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let f = normalize3(sub3(target, eye));
    let s = normalize3(cross3(f, up));
    let u = cross3(s, f);

    let f_len = 1.0 / (fov_deg.to_radians() * 0.5).tan();
    let asp = aspect.max(0.25);
    let nf = 1.0 / (near - far);

    // View matrix rows:
    // row0 = (s.x, s.y, s.z, -dot(s, eye))
    // row1 = (u.x, u.y, u.z, -dot(u, eye))
    // row2 = (-f.x, -f.y, -f.z, dot(f, eye))
    let tx = -dot3(s, eye);
    let ty = -dot3(u, eye);
    let tz = dot3(f, eye);

    let px = f_len / asp;
    let py = f_len;
    let pz = far * nf;
    let pw = far * near * nf;

    // P * V in column-major layout for WGSL `mat4x4<f32>`:
    [
        [px * s[0], py * u[0], -pz * f[0], f[0]],
        [px * s[1], py * u[1], -pz * f[1], f[1]],
        [px * s[2], py * u[2], -pz * f[2], f[2]],
        [px * tx, py * ty, pz * tz + pw, -tz],
    ]
}

/// Compute the active viewport rectangles for the current layout mode.
pub fn compute_viewports(
    layout: CameraLayout,
    width: f32,
    height: f32,
    primary: CameraFeedId,
    pip_feeds: [CameraFeedId; 2],
    grid_feeds: [CameraFeedId; 4],
) -> Vec<ViewportRect> {
    match layout {
        CameraLayout::Single => vec![ViewportRect {
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
            feed: primary,
            slot_tag: 0,
        }],
        CameraLayout::Pip => {
            let margin = (width.min(height) * 0.022).clamp(10.0, 28.0);
            let pip_w = (width * 0.275).clamp(180.0, 520.0);
            let pip_h = (pip_w * 9.0 / 16.0).min(height * 0.34);
            let x = (width - pip_w - margin).max(0.0);
            let y1 = (height - pip_h - margin).max(0.0);
            let y0 = (y1 - pip_h - margin * 0.75).max(0.0);
            vec![
                ViewportRect {
                    x: 0.0,
                    y: 0.0,
                    w: width,
                    h: height,
                    feed: primary,
                    slot_tag: 0,
                },
                ViewportRect {
                    x,
                    y: y0,
                    w: pip_w,
                    h: pip_h,
                    feed: pip_feeds[0],
                    slot_tag: 1,
                },
                ViewportRect {
                    x,
                    y: y1,
                    w: pip_w,
                    h: pip_h,
                    feed: pip_feeds[1],
                    slot_tag: 2,
                },
            ]
        }
        CameraLayout::Grid2x2 => {
            let hw = width * 0.5;
            let hh = height * 0.5;
            vec![
                ViewportRect {
                    x: 0.0,
                    y: 0.0,
                    w: hw,
                    h: hh,
                    feed: grid_feeds[0],
                    slot_tag: 10,
                },
                ViewportRect {
                    x: hw,
                    y: 0.0,
                    w: width - hw,
                    h: hh,
                    feed: grid_feeds[1],
                    slot_tag: 11,
                },
                ViewportRect {
                    x: 0.0,
                    y: hh,
                    w: hw,
                    h: height - hh,
                    feed: grid_feeds[2],
                    slot_tag: 12,
                },
                ViewportRect {
                    x: hw,
                    y: hh,
                    w: width - hw,
                    h: height - hh,
                    feed: grid_feeds[3],
                    slot_tag: 13,
                },
            ]
        }
    }
}

/// Apply a subtle yaw/pitch pan offset (for manual drag or CCTV servo sweep) to a camera look target.
fn apply_pan(eye: [f32; 3], target: [f32; 3], dyaw: f32, dpitch: f32) -> [f32; 3] {
    if dyaw.abs() < 1e-4 && dpitch.abs() < 1e-4 {
        return target;
    }
    let dir = sub3(target, eye);
    let dist = dot3(dir, dir).sqrt().max(0.1);
    let (r, u, f) = basis_from_forward(dir);
    let cy = dyaw.cos();
    let sy = dyaw.sin();
    let cp = dpitch.cos();
    let sp = dpitch.sin();
    let new_dir = [
        (f[0] * cy + r[0] * sy) * cp + u[0] * sp,
        (f[1] * cy + r[1] * sy) * cp + u[1] * sp,
        (f[2] * cy + r[2] * sy) * cp + u[2] * sp,
    ];
    [
        eye[0] + new_dir[0] * dist,
        eye[1] + new_dir[1] * dist,
        eye[2] + new_dir[2] * dist,
    ]
}

/// Resolve a `CameraFeedId` into a concrete 3D camera pose and HUD metadata.
pub fn resolve_camera(
    state: &OrbitalHavenState,
    feed: CameraFeedId,
    time: f32,
    apply_manual_pan: bool,
) -> ResolvedCamera {
    let pan = if apply_manual_pan {
        state.camera_rig.manual_pan
    } else {
        [0.0, 0.0]
    };
    let servo_sweep = (time * 0.24).sin() * 0.14;

    let (eye, mut target, up, fov_deg, title, subtitle) = match feed {
        CameraFeedId::DroneAlphaWide => {
            let drone = state.drones.first();
            let raw_eye = drone.map(|d| d.pos).unwrap_or([58.0, 26.0, 58.0]);
            let tgt = drone.map(|d| d.look_target).unwrap_or([0.0, 2.0, 8.0]);
            // Offset cleanly ahead of the physical 3D drone optic housing
            let fwd = normalize3(sub3(tgt, raw_eye));
            let eye = [
                raw_eye[0] + fwd[0] * 2.6,
                raw_eye[1] + fwd[1] * 2.6,
                raw_eye[2] + fwd[2] * 2.6,
            ];
            (
                eye,
                tgt,
                [0.0, 1.0, 0.0],
                48.0,
                format!("{} // {}", feed.code(), feed.label()),
                format!("ORBIT ALT 412KM · {}", state.event.name()),
            )
        }
        CameraFeedId::DroneBetaPatrol => {
            let drone = state.drones.get(1);
            let raw_eye = drone.map(|d| d.pos).unwrap_or([-34.0, 8.0, 36.0]);
            let tgt = drone.map(|d| d.look_target).unwrap_or([0.0, 0.0, 14.0]);
            let fwd = normalize3(sub3(tgt, raw_eye));
            let eye = [
                raw_eye[0] + fwd[0] * 2.6,
                raw_eye[1] + fwd[1] * 2.6,
                raw_eye[2] + fwd[2] * 2.6,
            ];
            (
                eye,
                tgt,
                [0.0, 1.0, 0.0],
                54.0,
                format!("{} // {}", feed.code(), feed.label()),
                format!("PERIMETER SWEEP · {} SHIPS", state.ships.len()),
            )
        }
        CameraFeedId::ExtHangarApproach => {
            let eye = [18.5, 11.5, 52.0];
            // Track any ship heading into Hangar Pad A, else frame the hangar mouth
            let pad_a_ship = state
                .ships
                .iter()
                .find(|s| s.assigned_pad == LandingPadId::PadAHangar);
            let tgt = pad_a_ship
                .map(|s| {
                    [
                        s.pos[0] * 0.4,
                        s.pos[1] * 0.5 + 1.0,
                        s.pos[2].clamp(24.0, 72.0),
                    ]
                })
                .unwrap_or([0.0, 1.0, 32.0]);
            (
                eye,
                tgt,
                [0.0, 1.0, 0.0],
                56.0,
                format!("{} // {}", feed.code(), feed.label()),
                "BAY-01 FORCEFIELD · ACTIVE".to_string(),
            )
        }
        CameraFeedId::PadAHangarInterior => {
            let eye = [-10.4, 7.6, 14.2];
            let tgt = [0.8, -0.8, 34.0];
            (
                eye,
                apply_pan(eye, tgt, servo_sweep * 0.65, 0.0),
                [0.0, 1.0, 0.0],
                66.0,
                format!("{} // {}", feed.code(), feed.label()),
                "ATMO FIELD 101.3 KPA · PAD-A".to_string(),
            )
        }
        CameraFeedId::PadBPortDeck => {
            let eye = [-24.5, 11.2, 16.5];
            let tgt = [PAD_B_POS[0] - 1.5, PAD_B_POS[1] + 1.2, PAD_B_POS[2]];
            (
                eye,
                apply_pan(eye, tgt, servo_sweep * 0.5, 0.0),
                [0.0, 1.0, 0.0],
                55.0,
                format!("{} // {}", feed.code(), feed.label()),
                "PORT FREIGHT DECK · PAD-B".to_string(),
            )
        }
        CameraFeedId::PadCStarboardDeck => {
            let eye = [24.5, 11.2, 16.5];
            let tgt = [PAD_C_POS[0] + 1.5, PAD_C_POS[1] + 1.2, PAD_C_POS[2]];
            (
                eye,
                apply_pan(eye, tgt, -servo_sweep * 0.5, 0.0),
                [0.0, 1.0, 0.0],
                55.0,
                format!("{} // {}", feed.code(), feed.label()),
                "STBD UTILITY DECK · PAD-C".to_string(),
            )
        }
        CameraFeedId::IntOpsDeck => {
            // High corner inside Ops Command Spire looking across CRT consoles & holotable out the big +Z viewports!
            let eye = [-7.8, 19.2, -7.2];
            let tgt = [1.5, 14.2, 24.0];
            (
                eye,
                apply_pan(eye, tgt, servo_sweep * 0.9, 0.0),
                [0.0, 1.0, 0.0],
                68.0,
                format!("{} // {}", feed.code(), feed.label()),
                "HELIOS-09 COMMAND · LIVE VIEWPORT".to_string(),
            )
        }
        CameraFeedId::IntConcourseWalk => {
            // Down-corridor security cam along the grated concourse with side windows into space!
            let eye = [-15.2, -7.4, 7.6];
            let tgt = [10.0, -11.8, -1.5];
            (
                eye,
                apply_pan(eye, tgt, servo_sweep * 0.5, 0.0),
                [0.0, 1.0, 0.0],
                68.0,
                format!("{} // {}", feed.code(), feed.label()),
                "DECK-03 CONCOURSE · SEC-CAM".to_string(),
            )
        }
        CameraFeedId::IntReactorCore => {
            // Elevated catwalk angle looking across the pulsing plasma core & exterior hull window
            let eye = [11.5, -7.2, -7.2];
            let tgt = [-2.0, -11.4, 4.0];
            (
                eye,
                apply_pan(eye, tgt, -servo_sweep * 0.6, 0.0),
                [0.0, 1.0, 0.0],
                66.0,
                format!("{} // {}", feed.code(), feed.label()),
                "CORE OUTPUT 480 MW · NOMINAL".to_string(),
            )
        }
        CameraFeedId::ShipNoseUplink => {
            if let Some(ship) = state.uplink_ship() {
                let (r, u, f) = basis_from_forward(ship.forward);
                let nose_fwd = match ship.class {
                    rookery_core::orbital::ShipClass::BulkFreighter => 7.2,
                    _ => 6.4,
                };
                let nose_up = match ship.class {
                    rookery_core::orbital::ShipClass::BulkFreighter => 2.8,
                    _ => 1.3,
                };
                let eye = [
                    ship.pos[0] + f[0] * nose_fwd + u[0] * nose_up,
                    ship.pos[1] + f[1] * nose_fwd + u[1] * nose_up,
                    ship.pos[2] + f[2] * nose_fwd + u[2] * nose_up,
                ];
                let tgt = [
                    eye[0] + f[0] * 32.0 + r[0] * (time * 0.4).sin() * 0.6,
                    eye[1] + f[1] * 32.0 - 2.0,
                    eye[2] + f[2] * 32.0,
                ];
                (
                    eye,
                    tgt,
                    u,
                    66.0,
                    format!("UPLINK-1 // {} NOSE-CAM", ship.callsign),
                    format!(
                        "{} -> {} [{}]",
                        ship.class.name().to_uppercase(),
                        ship.assigned_pad.label(),
                        state.stats.uplink_phase
                    ),
                )
            } else {
                (
                    [18.5, 11.5, 52.0],
                    [0.0, 1.0, 32.0],
                    [0.0, 1.0, 0.0],
                    58.0,
                    "UPLINK-1 // ACQUIRING SIGNAL...".into(),
                    "STANDBY".into(),
                )
            }
        }
        CameraFeedId::ShipDorsalUplink => {
            if let Some(ship) = state.uplink_ship() {
                let (r, u, f) = basis_from_forward(ship.forward);
                let eye = [
                    ship.pos[0] - f[0] * 11.5 + u[0] * 4.2 + r[0] * 2.2,
                    ship.pos[1] - f[1] * 11.5 + u[1] * 4.2 + r[1] * 2.2,
                    ship.pos[2] - f[2] * 11.5 + u[2] * 4.2 + r[2] * 2.2,
                ];
                let tgt = [
                    ship.pos[0] + f[0] * 16.0,
                    ship.pos[1] + f[1] * 16.0,
                    ship.pos[2] + f[2] * 16.0,
                ];
                (
                    eye,
                    tgt,
                    [0.0, 1.0, 0.0],
                    58.0,
                    format!("UPLINK-2 // {} DORSAL-CAM", ship.callsign),
                    format!("LIVE TELEMETRY · {}", state.stats.uplink_phase),
                )
            } else {
                (
                    [-24.5, 11.2, 16.5],
                    PAD_B_POS,
                    [0.0, 1.0, 0.0],
                    55.0,
                    "UPLINK-2 // ACQUIRING SIGNAL...".into(),
                    "STANDBY".into(),
                )
            }
        }
    };

    target = apply_pan(eye, target, pan[0], pan[1]);

    ResolvedCamera {
        eye,
        target,
        up,
        fov_deg,
        title,
        subtitle,
        is_interior: feed.is_interior(),
        is_uplink: feed.is_ship_uplink(),
    }
}
