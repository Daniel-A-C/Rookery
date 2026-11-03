//! On-demand tray popover window.
//!
//! To keep idle RAM well below the `< 120 MB` budget, no WebView exists while the popover is
//! closed. Clicking the tray icon builds the `popover` `WebviewWindow`, positions it next to
//! the tray icon via `tauri-plugin-positioner`, and destroys it as soon as it loses focus.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::POPOVER_LABEL;

const POPOVER_WIDTH: f64 = 320.0;
const POPOVER_HEIGHT: f64 = 520.0;

pub fn toggle(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        let _ = existing.destroy();
        return;
    }
    open(app);
}

pub fn open(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        let _ = existing.set_focus();
        return;
    }

    let window =
        match WebviewWindowBuilder::new(app, POPOVER_LABEL, WebviewUrl::App("index.html".into()))
            .title("Rookery")
            .inner_size(POPOVER_WIDTH, POPOVER_HEIGHT)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible_on_all_workspaces(true)
            .focused(true)
            .devtools(true)
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to build popover webview window: {e}");
                return;
            }
        };

    #[cfg(target_os = "macos")]
    let pos = Position::TrayCenter;
    #[cfg(not(target_os = "macos"))]
    let pos = Position::TrayBottomCenter;

    if window.move_window_constrained(pos).is_err() {
        let _ = window.move_window(Position::TopRight);
    }
    let _ = window.set_focus();

    let win_for_ev = window.clone();
    window.on_window_event(move |ev| {
        if let WindowEvent::Focused(false) = ev {
            let _ = win_for_ev.destroy();
        }
    });
}
