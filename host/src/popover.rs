//! Persistent, pre-warmed tray popover window.
//!
//! Keeping a single hidden `popover` `WebviewWindow` alive across toggles avoids the 300–600ms
//! WebView cold-start and synchronous teardown on every tray click, making menu open and close
//! instantaneous (< 5ms).

use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::{HostState, POPOVER_LABEL};

const POPOVER_WIDTH: f64 = 384.0;
const POPOVER_HEIGHT: f64 = 388.0;
const POPOVER_CORNER_RADIUS: f64 = 20.0;

static PROCESS_START: LazyLock<Instant> = LazyLock::new(Instant::now);
static LAST_OPENED_MS: AtomicU64 = AtomicU64::new(0);
static IS_FOCUSED: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    PROCESS_START.elapsed().as_millis() as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverAnchor {
    Tray,
    Cursor,
}

/// Pre-build the popover WebView hidden in the background during startup so the first open is instant.
pub fn prewarm(app: &AppHandle) {
    let _ = ensure_popover_window(app);
}

pub fn toggle(app: &AppHandle) {
    toggle_with_anchor(app, PopoverAnchor::Tray);
}

pub fn toggle_at_cursor(app: &AppHandle) {
    toggle_with_anchor(app, PopoverAnchor::Cursor);
}

pub fn hide(app: &AppHandle) {
    if let Some(state) = app.try_state::<HostState>() {
        state.popover_visible.store(false, Ordering::SeqCst);
        if state.hotkey_recording.swap(false, Ordering::SeqCst)
            && let Ok(inner) = state.inner.lock()
        {
            let settings = inner.settings.clone();
            let is_interactive = inner.interaction.is_interactive();
            drop(inner);
            crate::hotkey::sync_all_hotkeys_best_effort(app, &settings, is_interactive);
        }
    }
    IS_FOCUSED.store(false, Ordering::SeqCst);
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        let _ = existing.hide();
    }
}

fn toggle_with_anchor(app: &AppHandle, anchor: PopoverAnchor) {
    let is_visible = app
        .try_state::<HostState>()
        .map(|s| s.popover_visible.load(Ordering::SeqCst))
        .unwrap_or_else(|| {
            app.get_webview_window(POPOVER_LABEL)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false)
        });

    if is_visible {
        log::info!("popover toggle: hiding popover window");
        hide(app);
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

fn ensure_popover_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(existing) = app.get_webview_window(POPOVER_LABEL) {
        return Some(existing);
    }

    log::info!("building persistent popover webview window");
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
            .visible(false)
            .focused(false)
            .devtools(cfg!(debug_assertions))
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to build popover webview window: {e}");
                return None;
            }
        };

    let _ = rookery_platform::style_popover_window(&window, POPOVER_CORNER_RADIUS);
    if let Some(state) = app.try_state::<HostState>()
        && let Ok(inner) = state.inner.lock()
    {
        let _ = rookery_platform::set_popover_blur(&window, inner.settings.popover_blur);
    }

    let app_for_ev = app.clone();
    window.on_window_event(move |ev| match ev {
        WindowEvent::Focused(true) => {
            IS_FOCUSED.store(true, Ordering::SeqCst);
        }
        WindowEvent::Focused(false) => {
            IS_FOCUSED.store(false, Ordering::SeqCst);
            let is_vis = app_for_ev
                .try_state::<HostState>()
                .map(|s| s.popover_visible.load(Ordering::SeqCst))
                .unwrap_or(false);
            if !is_vis {
                return;
            }

            let opened_at = LAST_OPENED_MS.load(Ordering::SeqCst);
            let elapsed_ms = now_ms().saturating_sub(opened_at);
            // Ignore transient focus bounce during tray icon mouse release (< 160ms)
            if elapsed_ms < 160 {
                return;
            }

            let app_check = app_for_ev.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(45));
                if IS_FOCUSED.load(Ordering::SeqCst) {
                    return;
                }
                if rookery_platform::mouse_buttons().left {
                    return;
                }
                let app_main = app_check.clone();
                let _ = app_check.run_on_main_thread(move || {
                    if let Some(win) = app_main.get_webview_window(POPOVER_LABEL)
                        && !win.is_focused().unwrap_or(false)
                    {
                        hide(&app_main);
                    }
                });
            });
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&app_for_ev);
        }
        _ => {}
    });

    Some(window)
}

fn open_with_anchor(app: &AppHandle, anchor: PopoverAnchor) {
    let Some(window) = ensure_popover_window(app) else {
        return;
    };

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

    LAST_OPENED_MS.store(now_ms(), Ordering::SeqCst);
    IS_FOCUSED.store(true, Ordering::SeqCst);
    if let Some(state) = app.try_state::<HostState>() {
        state.popover_visible.store(true, Ordering::SeqCst);
    }

    let _ = window.show();
    let _ = window.set_focus();
    let _ = rookery_platform::activate_app_and_focus_window(&window);

    if let Some(state) = app.try_state::<HostState>() {
        state.emit_status_if_popover_open(app);
    }
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
