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
        log::info!("popover toggle: closing existing popover window");
        let _ = existing.destroy();
        return;
    }
    open(app);
}

pub fn open(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        log::info!("popover open: focusing existing popover window");
        let _ = existing.set_focus();
        return;
    }

    log::info!("popover open: building popover webview window");
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
            .devtools(cfg!(debug_assertions))
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
    let _ = rookery_platform::activate_app_and_focus_window(&window);

    let win_for_ev = window.clone();
    let created_at = std::time::Instant::now();
    let has_focused = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let has_focused_clone = has_focused.clone();
    let is_focused = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let is_focused_clone = is_focused.clone();

    window.on_window_event(move |ev| match ev {
        WindowEvent::Focused(true) => {
            log::info!("popover window gained focus");
            has_focused_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            is_focused_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        WindowEvent::Focused(false) => {
            is_focused_clone.store(false, std::sync::atomic::Ordering::SeqCst);
            let elapsed = created_at.elapsed();
            let had_focus = has_focused_clone.load(std::sync::atomic::Ordering::SeqCst);
            // Ignore transient focus loss events during the initial mounting window (macOS status item release)
            if elapsed > std::time::Duration::from_millis(1500) && had_focus {
                let win_clone = win_for_ev.clone();
                let is_f_check = is_focused_clone.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    if is_f_check.load(std::sync::atomic::Ordering::SeqCst) {
                        log::debug!(
                            "popover regained focus after transient loss, keeping window open"
                        );
                        return;
                    }
                    let win_main = win_clone.clone();
                    let _ = win_clone.run_on_main_thread(move || {
                        if let Ok(focused) = win_main.is_focused() {
                            if !focused {
                                log::info!(
                                    "popover window lost focus confirmed after {elapsed:?}: closing window"
                                );
                                let _ = win_main.destroy();
                            } else {
                                log::debug!(
                                    "popover is_focused() returned true, keeping window open"
                                );
                            }
                        }
                    });
                });
            } else {
                log::debug!(
                    "ignoring transient focus loss {elapsed:?} after creation (had_focus={had_focus})"
                );
            }
        }
        _ => {}
    });
}
