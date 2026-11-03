//! System Tray / macOS Menu Bar icon and context menu.

use anyhow::Result;
use rookery_core::interaction::InteractionEvent;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::popover;
use crate::state::HostState;

const MENU_OPEN: &str = "open_popover";
const MENU_INTERACTIVE: &str = "toggle_interactive";
const MENU_PAUSE: &str = "toggle_pause";
const MENU_QUIT: &str = "quit";

pub fn setup_tray(app: &AppHandle) -> Result<()> {
    let open_item = MenuItemBuilder::with_id(MENU_OPEN, "Open Rookery Controls…").build(app)?;
    let interactive_item =
        MenuItemBuilder::with_id(MENU_INTERACTIVE, "Toggle Direct Interaction Mode").build(app)?;
    let pause_item =
        MenuItemBuilder::with_id(MENU_PAUSE, "Pause / Resume Simulation").build(app)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, "Quit Rookery").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_item)
        .item(&interactive_item)
        .item(&pause_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let icon = build_tray_icon();

    TrayIconBuilder::with_id("rookery-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Rookery — Living Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            MENU_OPEN => popover::open(app),
            MENU_INTERACTIVE => {
                let state = app.state::<HostState>();
                state.apply_interaction_event(app, InteractionEvent::HotkeyPressed);
            }
            MENU_PAUSE => {
                let state = app.state::<HostState>();
                if let Ok(mut inner) = state.inner.lock() {
                    inner.user_paused = !inner.user_paused;
                }
                state.recompute_budgets();
                state.emit_status_if_popover_open(app);
            }
            MENU_QUIT => {
                let state = app.state::<HostState>();
                state.clean_shutdown();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &ev);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                popover::toggle(app);
            }
        })
        .build(app)?;

    Ok(())
}

/// Generates a crisp 44×44 monochrome bird-wing template icon (RGBA8).
fn build_tray_icon() -> Image<'static> {
    const SIZE: u32 = 44;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        let v = (y as f32 + 0.5) / SIZE as f32;
        for x in 0..SIZE {
            let u = (x as f32 + 0.5) / SIZE as f32;
            let sx = (u - 0.5) * 2.0;
            let sy = (v - 0.52) * 2.0;
            let alpha: f32 = if sx.abs() > 0.78 {
                0.0
            } else {
                let wing_y = -0.25 * sx.abs() + 0.22 * sx * sx - 0.05;
                let thickness = 0.22 * (1.0 - 0.65 * sx.abs());
                let d = (sy - wing_y).abs() - thickness;
                ((0.04 - d) / 0.08).clamp(0.0, 1.0)
            };
            rgba.extend_from_slice(&[0, 0, 0, (alpha * 255.0).round() as u8]);
        }
    }
    Image::new_owned(rgba, SIZE, SIZE)
}
