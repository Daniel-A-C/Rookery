//! Global hotkey registration for Direct Interaction Mode and `Escape` exit listener.

use anyhow::{Context, Result};
use rookery_core::interaction::InteractionEvent;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::state::HostState;

const ESCAPE_SHORTCUT: &str = "Escape";

/// Register the primary Direct Interaction Mode toggle shortcut.
pub fn register_interaction_hotkey(app: &AppHandle, shortcut: &str) -> Result<()> {
    let gs = app.global_shortcut();
    gs.on_shortcut(shortcut, |app, _sc, ev| {
        if ev.state == ShortcutState::Pressed {
            let state = app.state::<HostState>();
            state.apply_interaction_event(app, InteractionEvent::HotkeyPressed);
        }
    })
    .with_context(|| format!("failed to register hotkey '{shortcut}'"))
}

/// Swap the interaction hotkey, rolling back to `old_shortcut` if `new_shortcut` fails.
pub fn replace_interaction_hotkey(
    app: &AppHandle,
    old_shortcut: &str,
    new_shortcut: &str,
) -> Result<()> {
    if old_shortcut.eq_ignore_ascii_case(new_shortcut) {
        return Ok(());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister(old_shortcut);
    if let Err(e) = register_interaction_hotkey(app, new_shortcut) {
        let _ = register_interaction_hotkey(app, old_shortcut);
        return Err(e);
    }
    Ok(())
}

/// Register the global shortcut to summon / toggle the Rookery popover window.
pub fn register_popover_hotkey(app: &AppHandle, shortcut: &str) -> Result<()> {
    let gs = app.global_shortcut();
    gs.on_shortcut(shortcut, |app, _sc, ev| {
        if ev.state == ShortcutState::Pressed {
            crate::popover::toggle(app);
        }
    })
    .with_context(|| format!("failed to register popover hotkey '{shortcut}'"))
}

/// Swap the popover hotkey, rolling back to `old_shortcut` if `new_shortcut` fails.
pub fn replace_popover_hotkey(
    app: &AppHandle,
    old_shortcut: &str,
    new_shortcut: &str,
) -> Result<()> {
    if old_shortcut.eq_ignore_ascii_case(new_shortcut) {
        return Ok(());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister(old_shortcut);
    if let Err(e) = register_popover_hotkey(app, new_shortcut) {
        let _ = register_popover_hotkey(app, old_shortcut);
        return Err(e);
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
