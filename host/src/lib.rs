//! Rookery Core Desktop Host (Milestone 1).

pub mod commands;
pub mod hotkey;
pub mod monitors;
pub mod popover;
pub mod state;
pub mod tray;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use rookery_core::governor::PowerState;
use rookery_core::interaction::Interaction;
use rookery_core::settings::{SETTINGS_FILE_NAME, Settings};
use rookery_platform::{DisplayInfo, desktop_layer, occlusion_probe, power_probe, screens_asleep};
use rookery_render::demo::DemoScene;
use rookery_render::{Gpu, Renderer};
use state::{HostInner, HostState};
use tauri::{AppHandle, Manager, RunEvent};

pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            popover::open(app);
        }))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::set_paused,
            commands::set_interactive,
            commands::get_settings,
            commands::update_settings,
            commands::quit,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("rookery"));
            let settings_path = config_dir.join(SETTINGS_FILE_NAME);
            let (mut settings, outcome) = Settings::load(&settings_path);
            log::info!(
                "settings loaded from {} ({outcome:?})",
                settings_path.display()
            );
            settings.touch_running(Utc::now());
            if let Err(e) = settings.save(&settings_path) {
                log::warn!("failed to write initial settings heartbeat: {e}");
            }

            let gpu = Gpu::new_blocking()?;
            let seed_counter = Arc::new(std::sync::atomic::AtomicU64::new(1));
            let renderer = Renderer::spawn(
                gpu,
                Box::new(move |gpu, format| {
                    let s = seed_counter.fetch_add(1, Ordering::Relaxed);
                    Box::new(DemoScene::new(gpu, format, s * 0x9E37_79B9))
                }),
                Arc::new(rookery_platform::cursor_position),
            )?;

            let initial_hotkey = settings.hotkey.clone();
            let host_state = HostState {
                settings_path,
                layer: desktop_layer(),
                renderer: Mutex::new(Some(renderer)),
                inner: Mutex::new(HostInner {
                    settings,
                    interaction: Interaction::new(),
                    user_paused: false,
                    power: PowerState::Ac,
                    screens_asleep: false,
                    displays: BTreeMap::new(),
                }),
                shutting_down: AtomicBool::new(false),
            };
            app.manage(host_state);

            let handle = app.handle();
            monitors::reconcile_on_main_thread(handle);
            tray::setup_tray(handle)?;

            if let Err(e) = hotkey::register_interaction_hotkey(handle, &initial_hotkey) {
                log::warn!("{e:#}");
            }

            spawn_governor_thread(handle.clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Rookery Tauri host");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let state = app_handle.state::<HostState>();
            if !state.shutting_down.load(Ordering::SeqCst) {
                // Closing the on-demand popover window must not exit the background host.
                api.prevent_exit();
            }
        }
    });
}

fn spawn_governor_thread(app: AppHandle) {
    std::thread::Builder::new()
        .name("rookery-governor".into())
        .spawn(move || {
            let mut occ_probe = occlusion_probe();
            let mut pwr_probe = power_probe();
            let mut ticks: u64 = 0;

            loop {
                std::thread::sleep(Duration::from_secs(1));
                ticks = ticks.wrapping_add(1);

                let state = app.state::<HostState>();
                if state.shutting_down.load(Ordering::SeqCst) {
                    break;
                }

                // Periodically reconcile monitors on the main thread (hotplug / resolution).
                if ticks.is_multiple_of(2) {
                    let app_for_reconcile = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        monitors::reconcile_on_main_thread(&app_for_reconcile);
                    });
                }

                let display_infos: Vec<DisplayInfo> = {
                    let Ok(inner) = state.inner.lock() else {
                        continue;
                    };
                    inner.displays.values().map(|d| d.info.clone()).collect()
                };

                let occluded = occ_probe.query(&display_infos);
                let power = pwr_probe.query();
                let asleep = screens_asleep();

                if let Ok(mut inner) = state.inner.lock() {
                    inner.power = power;
                    inner.screens_asleep = asleep;
                    for (info, is_occ) in display_infos.iter().zip(occluded) {
                        if let Some(d) = inner.displays.get_mut(&info.id) {
                            d.occluded = is_occ;
                        }
                    }

                    // 5-minute settings heartbeat for Milestone 2's offline time-delta engine.
                    if ticks.is_multiple_of(300) {
                        inner.settings.touch_running(Utc::now());
                        if let Err(e) = inner.settings.save(&state.settings_path) {
                            log::debug!("heartbeat save failed: {e}");
                        }
                    }
                }

                state.recompute_budgets();
                state.emit_status_if_popover_open(&app);
            }
        })
        .expect("failed to spawn governor thread");
}
