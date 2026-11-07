//! On-demand tray popover window.
//!
//! To keep idle RAM well below the `< 120 MB` budget, no WebView exists while the popover is
//! closed. Clicking the tray icon builds the `popover` `WebviewWindow`, positions it next to
//! the tray icon via `tauri-plugin-positioner`, and destroys it as soon as it loses focus.

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::POPOVER_LABEL;

const POPOVER_WIDTH: f64 = 384.0;
const POPOVER_HEIGHT: f64 = 388.0;
const POPOVER_CORNER_RADIUS: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverAnchor {
    Tray,
    Cursor,
}

pub fn toggle(app: &AppHandle) {
    toggle_with_anchor(app, PopoverAnchor::Tray);
}

pub fn toggle_at_cursor(app: &AppHandle) {
    toggle_with_anchor(app, PopoverAnchor::Cursor);
}

fn toggle_with_anchor(app: &AppHandle, anchor: PopoverAnchor) {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        log::info!("popover toggle: closing existing popover window");
        let _ = existing.destroy();
        return;
    }
    open_with_anchor(app, anchor);
}

pub fn open(app: &AppHandle) {
    open_with_anchor(app, PopoverAnchor::Tray);
}

pub fn open_at_cursor(app: &AppHandle) {
    open_with_anchor(app, PopoverAnchor::Cursor);
}

fn open_with_anchor(app: &AppHandle, anchor: PopoverAnchor) {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        log::info!("popover open: focusing existing popover window");
        if anchor == PopoverAnchor::Cursor {
            position_window_at_cursor(app, &existing);
        }
        let _ = existing.set_focus();
        return;
    }

    log::info!("popover open ({anchor:?}): building popover webview window");
    let window =
        match WebviewWindowBuilder::new(app, POPOVER_LABEL, WebviewUrl::App("index.html".into()))
            .title("Rookery")
            .inner_size(POPOVER_WIDTH, POPOVER_HEIGHT)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(true)
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

    let _ = rookery_platform::style_popover_window(&window, POPOVER_CORNER_RADIUS);
    if let Some(state) = app.try_state::<crate::state::HostState>()
        && let Ok(inner) = state.inner.lock()
    {
        let _ = rookery_platform::set_popover_blur(&window, inner.settings.popover_blur);
    }

    match anchor {
        PopoverAnchor::Cursor => {
            position_window_at_cursor(app, &window);
        }
        PopoverAnchor::Tray => {
            #[cfg(target_os = "macos")]
            let pos = Position::TrayCenter;
            #[cfg(not(target_os = "macos"))]
            let pos = Position::TrayBottomCenter;

            if window.move_window_constrained(pos).is_err() {
                let _ = window.move_window(Position::TopRight);
            }
        }
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
                    if rookery_platform::mouse_buttons().left {
                        log::debug!("left mouse button held down, keeping popover window open");
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

fn position_window_at_cursor(app: &AppHandle, window: &WebviewWindow) {
    let Some((cx, cy)) = rookery_platform::cursor_position() else {
        let _ = window.move_window(Position::Center);
        return;
    };

    let monitors = app.available_monitors().unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        // On macOS, `cursor_position()` returns global logical points while `Monitor::position()`
        // and `Monitor::size()` are physical pixels.
        let target_monitor = monitors
            .iter()
            .find(|m| {
                let scale = m.scale_factor();
                let mx = m.position().x as f64 / scale;
                let my = m.position().y as f64 / scale;
                let mw = m.size().width as f64 / scale;
                let mh = m.size().height as f64 / scale;
                cx >= mx && cx < mx + mw && cy >= my && cy < my + mh
            })
            .or_else(|| monitors.first());

        if let Some(m) = target_monitor {
            let scale = m.scale_factor();
            let mx = m.position().x as f64 / scale;
            let my = m.position().y as f64 / scale;
            let mw = m.size().width as f64 / scale;
            let mh = m.size().height as f64 / scale;

            let margin = 12.0;
            let top_bar_pad = 30.0;
            let min_x = mx + margin;
            let max_x = (mx + mw - POPOVER_WIDTH - margin).max(min_x);
            let min_y = my + top_bar_pad;
            let max_y = (my + mh - POPOVER_HEIGHT - margin).max(min_y);

            let lx = (cx - POPOVER_WIDTH * 0.5).clamp(min_x, max_x);
            let ly = (cy - POPOVER_HEIGHT * 0.35).clamp(min_y, max_y);

            let phys_x = (lx * scale).round() as i32;
            let phys_y = (ly * scale).round() as i32;
            let _ = window.set_position(PhysicalPosition::new(phys_x, phys_y));
            return;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Windows/Linux, `cursor_position()` and `Monitor` bounds are in physical pixels.
        let target_monitor = monitors
            .iter()
            .find(|m| {
                let mx = m.position().x as f64;
                let my = m.position().y as f64;
                let mw = m.size().width as f64;
                let mh = m.size().height as f64;
                cx >= mx && cx < mx + mw && cy >= my && cy < my + mh
            })
            .or_else(|| monitors.first());

        if let Some(m) = target_monitor {
            let scale = m.scale_factor();
            let mx = m.position().x as f64;
            let my = m.position().y as f64;
            let mw = m.size().width as f64;
            let mh = m.size().height as f64;
            let win_w = POPOVER_WIDTH * scale;
            let win_h = POPOVER_HEIGHT * scale;

            let margin = 12.0 * scale;
            let min_x = mx + margin;
            let max_x = (mx + mw - win_w - margin).max(min_x);
            let min_y = my + margin;
            let max_y = (my + mh - win_h - margin).max(min_y);

            let px = (cx - win_w * 0.5).clamp(min_x, max_x).round() as i32;
            let py = (cy - win_h * 0.35).clamp(min_y, max_y).round() as i32;
            let _ = window.set_position(PhysicalPosition::new(px, py));
            return;
        }
    }

    let _ = window.move_window(Position::Center);
}
