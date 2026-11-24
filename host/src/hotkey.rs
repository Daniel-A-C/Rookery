//! Global hotkey registration for Direct Interaction Mode, popover menu, favorite Rookeries,
//! and the `Escape` exit listener.

use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use rookery_core::interaction::InteractionEvent;
use rookery_core::settings::{FAVORITE_HOTKEY_SLOTS, Settings};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::state::HostState;

const ESCAPE_SHORTCUT: &str = "Escape";

/// Validate that every configured Rookery hotkey is non-empty, syntactically valid,
/// and does not collide with another Rookery hotkey.
pub fn validate_hotkeys(settings: &Settings) -> Result<()> {
    let mut seen: Vec<(Shortcut, String)> = Vec::with_capacity(2 + FAVORITE_HOTKEY_SLOTS);

    let mut check = |raw: &str, label: String| -> Result<()> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("{label} hotkey cannot be empty"));
        }
        let parsed = Shortcut::from_str(trimmed)
            .map_err(|e| anyhow!("invalid shortcut '{trimmed}' for {label}: {e}"))?;
        if let Some((_, existing_label)) = seen.iter().find(|(sc, _)| *sc == parsed) {
            return Err(anyhow!(
                "'{trimmed}' is already assigned to {existing_label}"
            ));
        }
        seen.push((parsed, label));
        Ok(())
    };

    check(&settings.hotkey, "Mouse Interaction".to_owned())?;
    check(&settings.popover_hotkey, "Menu at Cursor".to_owned())?;
    for (idx, hk) in settings
        .favorite_hotkeys
        .iter()
        .take(FAVORITE_HOTKEY_SLOTS)
        .enumerate()
    {
        check(hk, format!("Favorite #{}", idx + 1))?;
    }

    Ok(())
}

/// Register the primary Direct Interaction Mode toggle shortcut.
pub fn register_interaction_hotkey(app: &AppHandle, shortcut: &str) -> Result<()> {
    let gs = app.global_shortcut();
    gs.on_shortcut(shortcut, |app, _sc, ev| {
        if ev.state == ShortcutState::Pressed {
            let state = app.state::<HostState>();
            state.apply_interaction_event(app, InteractionEvent::HotkeyPressed);
        }
    })
    .with_context(|| format!("failed to register interaction hotkey '{shortcut}'"))
}

/// Register the global shortcut to summon / toggle the Rookery popover window at the mouse cursor.
pub fn register_popover_hotkey(app: &AppHandle, shortcut: &str) -> Result<()> {
    let gs = app.global_shortcut();
    gs.on_shortcut(shortcut, |app, _sc, ev| {
        if ev.state == ShortcutState::Pressed {
            crate::popover::toggle_at_cursor(app);
        }
    })
    .with_context(|| format!("failed to register menu hotkey '{shortcut}'"))
}

/// Register a global shortcut that jumps directly to favorite Rookery slot `slot_index` (0-based).
pub fn register_favorite_hotkey(app: &AppHandle, slot_index: usize, shortcut: &str) -> Result<()> {
    let gs = app.global_shortcut();
    gs.on_shortcut(shortcut, move |app, _sc, ev| {
        if ev.state == ShortcutState::Pressed {
            let state = app.state::<HostState>();
            let target_app = state
                .inner
                .lock()
                .ok()
                .and_then(|inner| inner.settings.favorite_apps.get(slot_index).cloned());
            if let Some(app_id) = target_app {
                log::info!(
                    "favorite hotkey #{} triggered: switching to '{app_id}'",
                    slot_index + 1
                );
                if let Err(e) = state.switch_app(app, &app_id) {
                    log::warn!("failed to switch to favorite '{app_id}': {e}");
                } else {
                    state.emit_status_if_popover_open(app);
                }
            }
        }
    })
    .with_context(|| {
        format!(
            "failed to register Favorite #{} hotkey '{shortcut}'",
            slot_index + 1
        )
    })
}

/// Temporarily unregister all global shortcuts while the user is recording a new hotkey
/// in the popover UI so existing bindings do not intercept keystrokes.
pub fn suspend_all_hotkeys(app: &AppHandle) {
    let gs = app.global_shortcut();
    if let Err(e) = gs.unregister_all() {
        log::warn!("failed to suspend global shortcuts: {e}");
    }
}

/// Register all active shortcuts from `settings`, logging warnings instead of aborting
/// if an individual OS binding is unavailable (used at startup and when resuming).
pub fn sync_all_hotkeys_best_effort(app: &AppHandle, settings: &Settings, is_interactive: bool) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    if let Err(e) = register_interaction_hotkey(app, settings.hotkey.trim()) {
        log::warn!("{e:#}");
    }
    if let Err(e) = register_popover_hotkey(app, settings.popover_hotkey.trim()) {
        log::warn!("{e:#}");
    }
    let active_favs = settings.favorite_apps.len().min(FAVORITE_HOTKEY_SLOTS);
    for (idx, hk) in settings
        .favorite_hotkeys
        .iter()
        .take(active_favs)
        .enumerate()
    {
        if let Err(e) = register_favorite_hotkey(app, idx, hk.trim()) {
            log::warn!("{e:#}");
        }
    }
    if is_interactive {
        set_escape_listener(app, true);
    }
}

/// Validate and atomically apply a new set of hotkeys, rolling back to `previous` on error.
pub fn apply_hotkeys(
    app: &AppHandle,
    previous: &Settings,
    candidate: &Settings,
    is_interactive: bool,
) -> Result<()> {
    validate_hotkeys(candidate)?;

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let register_result = (|| -> Result<()> {
        register_interaction_hotkey(app, candidate.hotkey.trim())?;
        register_popover_hotkey(app, candidate.popover_hotkey.trim())?;
        let active_favs = candidate.favorite_apps.len().min(FAVORITE_HOTKEY_SLOTS);
        for (idx, hk) in candidate
            .favorite_hotkeys
            .iter()
            .take(active_favs)
            .enumerate()
        {
            register_favorite_hotkey(app, idx, hk.trim())?;
        }
        Ok(())
    })();

    if let Err(err) = register_result {
        sync_all_hotkeys_best_effort(app, previous, is_interactive);
        return Err(err);
    }

    if is_interactive {
        set_escape_listener(app, true);
    }
    Ok(())
}

/// Register `Escape` only while Direct Interaction Mode is active so we never steal `Escape`
/// from other applications during passive wallpaper operation.
pub fn set_escape_listener(app: &AppHandle, enable: bool) {
    let gs = app.global_shortcut();
    if enable {
        let _ = gs.on_shortcut(ESCAPE_SHORTCUT, |app, _sc, ev| {
            if ev.state == ShortcutState::Pressed {
                let state = app.state::<HostState>();
                state.apply_interaction_event(app, InteractionEvent::EscPressed);
            }
        });
    } else {
        let _ = gs.unregister(ESCAPE_SHORTCUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkeys_are_valid_and_unique() {
        let s = Settings::default();
        assert!(validate_hotkeys(&s).is_ok());
    }

    #[test]
    fn duplicate_hotkeys_are_rejected() {
        let mut s = Settings::default();
        s.hotkey = s.popover_hotkey.clone();
        let err = validate_hotkeys(&s).unwrap_err();
        assert!(err.to_string().contains("already assigned"));
    }

    #[test]
    fn favorite_collision_with_interaction_hotkey_is_rejected() {
        let mut s = Settings::default();
        s.favorite_hotkeys[0] = s.hotkey.clone();
        let err = validate_hotkeys(&s).unwrap_err();
        assert!(err.to_string().contains("Mouse Interaction"));
    }
}
