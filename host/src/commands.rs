//! Tauri IPC commands exposed to the popover UI.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use rookery_core::interaction::InteractionEvent;
use rookery_core::settings::Settings;
use rookery_core::world::WorldState;
use rookery_render::capture::{write_gif, write_png};
use rookery_render::offscreen::{OFFSCREEN_FORMAT, Offscreen};
use rookery_render::{LivingRookeryScene, Scene, SceneInput};
use tauri::{AppHandle, Manager, State};

use crate::hotkey;
use crate::state::{HostState, HostStatus, SettingsPatch};

#[tauri::command]
pub fn get_status(state: State<'_, HostState>) -> HostStatus {
    state.status()
}

#[tauri::command]
pub fn set_paused(app: AppHandle, state: State<'_, HostState>, paused: bool) -> HostStatus {
    if let Ok(mut inner) = state.inner.lock() {
        inner.user_paused = paused;
    }
    state.recompute_budgets();
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn set_interactive(
    app: AppHandle,
    state: State<'_, HostState>,
    interactive: bool,
) -> HostStatus {
    state.apply_interaction_event(&app, InteractionEvent::SetInteractive(interactive));
    state.status()
}

#[tauri::command]
pub fn get_settings(state: State<'_, HostState>) -> Settings {
    state
        .inner
        .lock()
        .map(|g| g.settings.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, HostState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    if let Some(ref new_hk) = patch.hotkey {
        let trimmed = new_hk.trim();
        if trimmed.is_empty() {
            return Err("Hotkey cannot be empty".into());
        }
        let old_hk = state
            .inner
            .lock()
            .map_err(|_| "state poisoned")?
            .settings
            .hotkey
            .clone();
        hotkey::replace_interaction_hotkey(&app, &old_hk, trimmed).map_err(|e| e.to_string())?;
    }

    let updated = {
        let mut inner = state.inner.lock().map_err(|_| "state poisoned")?;
        if let Some(hk) = patch.hotkey {
            inner.settings.hotkey = hk.trim().to_owned();
        }
        if let Some(v) = patch.passive_fps {
            inner.settings.frame.passive_fps = v;
        }
        if let Some(v) = patch.interactive_fps {
            inner.settings.frame.interactive_fps = v;
        }
        if let Some(v) = patch.low_power_fps {
            inner.settings.frame.low_power_fps = v;
        }
        if let Some(v) = patch.pause_on_battery {
            inner.settings.frame.pause_on_battery = v;
        }
        inner.settings.frame = inner.settings.frame.clamped();
        inner
            .settings
            .save(&state.settings_path)
            .map_err(|e| e.to_string())?;
        inner.settings.clone()
    };

    state.recompute_budgets();
    state.emit_status_if_popover_open(&app);
    Ok(updated)
}

#[tauri::command]
pub fn plant_seed(app: AppHandle, state: State<'_, HostState>, pos: [f32; 2]) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.plant_seed(pos);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn prune_branch(
    app: AppHandle,
    state: State<'_, HostState>,
    pos: [f32; 2],
    radius: Option<f32>,
) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.prune_nearest(pos, radius.unwrap_or(80.0));
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn feed_birds(app: AppHandle, state: State<'_, HostState>, pos: [f32; 2]) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.drop_snack(pos);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn get_world_seed(state: State<'_, HostState>) -> String {
    if let Ok(w) = state.world.lock() {
        w.seed_uri()
    } else {
        "rookery://tree/seed-unknown".into()
    }
}

#[tauri::command]
pub fn get_world_state_string(state: State<'_, HostState>) -> Result<String, String> {
    if let Ok(w) = state.world.lock() {
        w.state_uri().map_err(|e| e.to_string())
    } else {
        Err("Failed to acquire world state lock".into())
    }
}

#[tauri::command]
pub fn import_world_string(
    app: AppHandle,
    state: State<'_, HostState>,
    input: String,
) -> Result<HostStatus, String> {
    let new_world = WorldState::from_share_string(&input)?;
    if let Ok(mut w) = state.world.lock() {
        *w = new_world;
        let _ = w.save_atomic(&state.world_path);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    Ok(status)
}

#[tauri::command]
pub fn capture_snapshot(app: AppHandle, state: State<'_, HostState>) -> Result<String, String> {
    let world_snapshot = {
        let guard = state.world.lock().map_err(|_| "World lock poisoned")?;
        guard.clone()
    };

    let pictures_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("Rookery");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{timestamp}.png");
    let filepath = pictures_dir.join(filename);

    let width = 1280u32;
    let height = 720u32;

    let offscreen = Offscreen::new(&state.gpu, width, height);
    let world_arc = Arc::new(Mutex::new(world_snapshot));
    let mut scene = LivingRookeryScene::new(&state.gpu, OFFSCREEN_FORMAT, world_arc);
    scene.resize(&state.gpu, width, height, 1.0);

    let input = SceneInput::default();
    scene.update(&state.gpu, Duration::from_millis(16), &input);

    let view = offscreen.create_view();
    let mut encoder = state
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("snapshot-encoder"),
        });
    scene.render(&state.gpu, &view, &mut encoder);
    state.gpu.queue.submit(Some(encoder.finish()));

    let rgba = offscreen
        .readback_rgba(&state.gpu)
        .map_err(|e| format!("Failed to readback snapshot frame: {e}"))?;

    write_png(&filepath, width, height, &rgba).map_err(|e| format!("Failed to write PNG: {e}"))?;

    log::info!("Exported snapshot to {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[tauri::command]
pub fn record_loop(app: AppHandle, state: State<'_, HostState>) -> Result<String, String> {
    let world_snapshot = {
        let guard = state.world.lock().map_err(|_| "World lock poisoned")?;
        guard.clone()
    };

    let pictures_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("Rookery");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("loop_{timestamp}.gif");
    let filepath = pictures_dir.join(filename);

    let width = 640u32;
    let height = 360u32;
    let total_frames = 60u32; // ~5 seconds at 12 fps
    let dt = Duration::from_millis(83); // ~12 fps
    let delay_centisecs = 8u16; // 8/100 sec = 12.5 fps

    let offscreen = Offscreen::new(&state.gpu, width, height);
    let world_arc = Arc::new(Mutex::new(world_snapshot));
    let mut scene = LivingRookeryScene::new(&state.gpu, OFFSCREEN_FORMAT, world_arc);
    scene.resize(&state.gpu, width, height, 1.0);

    let input = SceneInput::default();
    let mut frames = Vec::with_capacity(total_frames as usize);

    for _ in 0..total_frames {
        scene.update(&state.gpu, dt, &input);
        let view = offscreen.create_view();
        let mut encoder =
            state
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("loop-frame-encoder"),
                });
        scene.render(&state.gpu, &view, &mut encoder);
        state.gpu.queue.submit(Some(encoder.finish()));

        let rgba = offscreen
            .readback_rgba(&state.gpu)
            .map_err(|e| format!("Failed to readback loop frame: {e}"))?;
        frames.push(rgba);
    }

    write_gif(
        &filepath,
        width as u16,
        height as u16,
        &mut frames,
        delay_centisecs,
    )
    .map_err(|e| format!("Failed to encode GIF: {e}"))?;

    log::info!("Exported loop to {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[tauri::command]
pub fn set_world_seed(app: AppHandle, state: State<'_, HostState>, seed: u64) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        *w = rookery_core::world::WorldState::new(seed);
        let _ = w.save_atomic(&state.world_path);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn quit(app: AppHandle, state: State<'_, HostState>) {
    state.clean_shutdown();
    app.exit(0);
}
