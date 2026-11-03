//! Tauri IPC commands exposed to the popover UI.

use rookery_core::interaction::InteractionEvent;
use rookery_core::settings::Settings;
use tauri::{AppHandle, State};

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
pub fn quit(app: AppHandle, state: State<'_, HostState>) {
    state.clean_shutdown();
    app.exit(0);
}

