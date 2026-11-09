//! Multi-monitor wallpaper window reconciler.
//!
//! Creates one bare native `tauri::Window` (no WebView) per connected monitor, attaches it
//! to the OS desktop wallpaper layer, and registers its `wgpu::Surface` with the render thread.
//! Also handles monitor hotplug and resolution/scale changes.

use std::collections::BTreeSet;

use rookery_core::interaction::InteractionEvent;
use rookery_core::{DisplayId, Rect};
use rookery_platform::DisplayInfo;
use rookery_render::SurfaceSize;
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WindowEvent};

use crate::state::{HostState, ManagedDisplay, initial_budget};

fn display_info_from_monitor(index: usize, m: &Monitor) -> DisplayInfo {
    let raw_name = m
        .name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Display {}", index + 1));
    let pos = m.position();
    let size = m.size();
    let scale = m.scale_factor();

    // Stable ID combining sanitized name and origin.
    let slug: String = raw_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let id = DisplayId::new(format!("{slug}-{}-{}", pos.x, pos.y));

    // On macOS, `CGWindowListCopyWindowInfo` and `CGEvent::location` use global logical points,
    // while `Monitor::position()` and `Monitor::size()` are physical pixels. On Windows,
    // `GetWindowRect` and `GetCursorPos` use physical pixels.
    #[cfg(target_os = "macos")]
    let bounds_native = Rect::new(
        pos.x as f64 / scale,
        pos.y as f64 / scale,
        size.width as f64 / scale,
        size.height as f64 / scale,
    );
    #[cfg(not(target_os = "macos"))]
    let bounds_native = Rect::new(
        pos.x as f64,
        pos.y as f64,
        size.width as f64,
        size.height as f64,
    );

    DisplayInfo {
        id,
        name: raw_name,
        bounds_native,
        width_px: size.width.max(1),
        height_px: size.height.max(1),
        scale_factor: scale,
    }
}

fn surface_size(info: &DisplayInfo) -> SurfaceSize {
    SurfaceSize {
        width: info.width_px,
        height: info.height_px,
        scale_factor: info.scale_factor,
        bounds_native: info.bounds_native,
    }
}

/// Reconcile connected monitors with wallpaper windows. Must be called on the main thread.
pub fn reconcile_on_main_thread(app: &AppHandle) {
    let monitors = match app.available_monitors() {
        Ok(m) => m,
        Err(e) => {
            log::warn!("failed to query available monitors: {e}");
            return;
        }
    };

    let desired: Vec<(Monitor, DisplayInfo)> = monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            let info = display_info_from_monitor(i, &m);
            (m, info)
        })
        .collect();
    let desired_ids: BTreeSet<DisplayId> = desired.iter().map(|(_, i)| i.id.clone()).collect();

    let state = app.state::<HostState>();

    // 1. Remove displays that were unplugged.
    let removed_ids: Vec<DisplayId> = {
        let Ok(inner) = state.inner.lock() else {
            return;
        };
        inner
            .displays
            .keys()
            .filter(|id| !desired_ids.contains(*id))
            .cloned()
            .collect()
    };

    let mut topology_changed = !removed_ids.is_empty();
    for id in removed_ids {
        // Drop the wgpu Surface on the render thread BEFORE dropping the native window.
        state.with_renderer(|r| r.remove_surface(id.clone()));
        if let Ok(mut inner) = state.inner.lock()
            && let Some(d) = inner.displays.remove(&id)
        {
            let _ = d.window.destroy();
            log::info!("removed wallpaper surface for unplugged display {id}");
        }
    }

    // 2. Add new displays or resize changed ones.
    for (idx, (monitor, info)) in desired.into_iter().enumerate() {
        let existing_changed = {
            let Ok(mut inner) = state.inner.lock() else {
                continue;
            };
            if let Some(existing) = inner.displays.get_mut(&info.id) {
                if existing.info != info {
                    existing.info = info.clone();
                    let _ = existing.window.set_position(PhysicalPosition::new(
                        monitor.position().x,
                        monitor.position().y,
                    ));
                    let _ = existing
                        .window
                        .set_size(PhysicalSize::new(info.width_px, info.height_px));
                    let _ = state
                        .layer
                        .attach_wallpaper(&existing.window, &existing.info);
                    Some(info.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(updated) = existing_changed {
            state.with_renderer(|r| r.resize(updated.id.clone(), surface_size(&updated)));
            continue;
        }

        let already_exists = state
            .inner
            .lock()
            .map(|g| g.displays.contains_key(&info.id))
            .unwrap_or(false);
        if already_exists {
            continue;
        }

        // Build a bare native window (NO WebView) for this monitor.
        let label = format!("wallpaper-{idx}-{}", monitor.position().x.unsigned_abs());
        let pos = monitor.position();
        let logical_w = info.width_px as f64 / info.scale_factor;
        let logical_h = info.height_px as f64 / info.scale_factor;

        let window = match tauri::window::WindowBuilder::new(app, label)
            .title("Rookery Wallpaper")
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .shadow(false)
            .focused(false)
            .visible(true)
            .position(pos.x as f64, pos.y as f64)
            .inner_size(logical_w, logical_h)
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to create wallpaper window for {}: {e}", info.id);
                continue;
            }
        };

        let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
        let _ = window.set_size(PhysicalSize::new(info.width_px, info.height_px));

        if let Err(e) = state.layer.attach_wallpaper(&window, &info) {
            log::error!("failed to attach wallpaper on {}: {e:#}", info.id);
        }

        // Listen for focus loss so clicking into any normal app exits Direct Interaction Mode.
        let app_for_ev = app.clone();
        window.on_window_event(move |ev| {
            if let WindowEvent::Focused(false) = ev {
                let st = app_for_ev.state::<HostState>();
                st.apply_interaction_event(&app_for_ev, InteractionEvent::FocusLost);
            }
        });

        let surface_res = state.with_renderer(|r| unsafe { r.create_surface(&window) });
        match surface_res {
            Some(Ok(surface)) => {
                state.with_renderer(|r| {
                    r.add_surface(info.id.clone(), surface, surface_size(&info))
                });
            }
            Some(Err(e)) => {
                log::error!("failed to create wgpu surface for {}: {e:#}", info.id);
                let _ = window.destroy();
                continue;
            }
            None => continue,
        }

        if let Ok(mut inner) = state.inner.lock() {
            inner.displays.insert(
                info.id.clone(),
                ManagedDisplay {
                    info,
                    window,
                    occluded: false,
                    uncovered: false,
                    budget: initial_budget(),
                },
            );
        }
        topology_changed = true;
    }

    if topology_changed {
        state.apply_interaction_event(app, InteractionEvent::DisplaysChanged);
        state.recompute_budgets();
    }

    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_else(|_| "living-rookery".into());
    crate::webview_sandbox::update_webview_wallpaper(app, &active_app, state.apps_dir.as_deref());
}
