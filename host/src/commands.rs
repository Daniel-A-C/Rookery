//! Tauri IPC commands exposed to the popover UI.

use std::time::Duration;

use chrono::Local;
use rookery_core::interaction::InteractionEvent;
use rookery_core::manifest::AppManifest;
use rookery_core::orbital::{ConstructionFocus, OrbitalHavenState, STATION_CENTER};
use rookery_core::settings::Settings;
use rookery_core::world::WorldState;
use rookery_render::capture::{encode_mp4_stream, write_gif, write_png};
use rookery_render::offscreen::{OFFSCREEN_FORMAT, Offscreen};
use rookery_render::{AppCatalog, SceneInput, SoundEffect};
use tauri::{AppHandle, Manager, State};

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

    if let Some(ref new_p_hk) = patch.popover_hotkey {
        let trimmed = new_p_hk.trim();
        if trimmed.is_empty() {
            return Err("Popover hotkey cannot be empty".into());
        }
        let old_hk = state
            .inner
            .lock()
            .map_err(|_| "state poisoned")?
            .settings
            .popover_hotkey
            .clone();
        hotkey::replace_popover_hotkey(&app, &old_hk, trimmed).map_err(|e| e.to_string())?;
    }

    let updated = {
        let mut inner = state.inner.lock().map_err(|_| "state poisoned")?;
        if let Some(hk) = patch.hotkey {
            inner.settings.hotkey = hk.trim().to_owned();
        }
        if let Some(hk) = patch.popover_hotkey {
            inner.settings.popover_hotkey = hk.trim().to_owned();
        }
        if let Some(opacity) = patch.popover_opacity {
            inner.settings.popover_opacity = opacity.clamp(
                rookery_core::settings::MIN_POPOVER_OPACITY,
                rookery_core::settings::MAX_POPOVER_OPACITY,
            );
        }
        if let Some(blur) = patch.popover_blur {
            inner.settings.popover_blur = blur.min(rookery_core::settings::MAX_POPOVER_BLUR);
        }
        if let Some(favs) = patch.favorite_apps {
            inner.settings.favorite_apps = favs;
        }
        if let Some(mode) = patch.schedule_mode {
            inner.settings.schedule_mode = mode;
        }
        if let Some(mins) = patch.schedule_interval_mins {
            inner.settings.schedule_interval_mins = mins;
        }
        if let Some(mapping) = patch.day_night_mapping {
            inner.settings.day_night_mapping = mapping;
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
        if let Some(v) = patch.audio_enabled {
            inner.settings.audio.enabled = v;
        }
        if let Some(v) = patch.audio_volume {
            inner.settings.audio.volume = v;
        }
        if let Some(v) = patch.audio_direct_mode_only {
            inner.settings.audio.direct_mode_only = v;
        }
        if let Some(v) = patch.update_auto_check {
            inner.settings.update.auto_check = v;
        }
        if let Some(v) = patch.update_auto_download {
            inner.settings.update.auto_download = v;
        }
        if let Some(dbg) = patch.debug_mode {
            inner.settings.debug_mode = dbg;
        }
        inner.settings.frame = inner.settings.frame.clamped();
        inner.settings.audio = inner.settings.audio.clamped();
        state.audio.update_settings(inner.settings.audio);
        inner
            .settings
            .save(&state.settings_path)
            .map_err(|e| e.to_string())?;
        inner.settings.clone()
    };

    if let Some(dbg) = patch.debug_mode
        && let Ok(mut w) = state.world.lock()
    {
        w.debug_mode = dbg;
    }

    if let Some(blur) = patch.popover_blur {
        apply_popover_blur_on_main_thread(&app, blur);
    }

    if let Some(ref app_id) = patch.active_app {
        state.switch_app(&app, app_id)?;
    }

    state.recompute_budgets();
    state.emit_status_if_popover_open(&app);
    Ok(updated)
}

fn apply_popover_blur_on_main_thread(app: &AppHandle, blur: u32) {
    let clamped = blur.min(rookery_core::settings::MAX_POPOVER_BLUR);
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app_clone.get_webview_window(crate::state::POPOVER_LABEL) {
            let _ = rookery_platform::set_popover_blur(&win, clamped);
        }
    });
}

#[tauri::command]
pub fn set_popover_blur(app: AppHandle, blur: u32) {
    apply_popover_blur_on_main_thread(&app, blur);
}

#[tauri::command]
pub fn start_drag(app: AppHandle) {
    if let Some(win) = app.get_webview_window(crate::state::POPOVER_LABEL) {
        #[cfg(target_os = "macos")]
        {
            let win_clone = win.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = rookery_platform::start_window_drag(&win_clone);
            });
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = win.start_dragging();
        }
    }
}

#[tauri::command]
pub fn plant_seed(app: AppHandle, state: State<'_, HostState>, pos: [f32; 2]) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.plant_seed(pos);
    }
    state.audio.play_sfx(SoundEffect::PlantSeed);
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn prune_branch(
    app: AppHandle,
    state: State<'_, HostState>,
    pos: [f32; 2],
    radius: Option<f32>,
) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.prune_nearest(pos, radius.unwrap_or(80.0));
    }
    state.audio.play_sfx(SoundEffect::Prune);
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn feed_birds(app: AppHandle, state: State<'_, HostState>, pos: [f32; 2]) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.drop_snack(pos);
    }
    state.audio.play_sfx(SoundEffect::BirdChirp);
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn get_world_seed(state: State<'_, HostState>) -> String {
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_default();
    if active_app == "orbital-haven"
        && let Ok(o) = state.catalog_state.orbital.lock()
    {
        return o.seed_uri();
    }
    if let Ok(w) = state.world.lock() {
        w.seed_uri()
    } else {
        "rookery://tree/seed-unknown".into()
    }
}

#[tauri::command]
pub fn get_world_state_string(state: State<'_, HostState>) -> Result<String, String> {
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_default();
    if active_app == "orbital-haven"
        && let Ok(o) = state.catalog_state.orbital.lock()
    {
        return o.state_uri();
    }
    if let Ok(w) = state.world.lock() {
        w.state_uri().map_err(|e| e.to_string())
    } else {
        Err("Failed to acquire world state lock".into())
    }
}

#[tauri::command]
pub fn import_world_string(
    app: AppHandle,
    state: State<'_, HostState>,
    input: String,
) -> Result<HostStatus, String> {
    let trimmed = input.trim();
    if trimmed.starts_with("rookery://orbital/") || trimmed.starts_with("orbital-seed-") {
        let new_orb = OrbitalHavenState::from_share_string(trimmed)?;
        if let Ok(mut o) = state.catalog_state.orbital.lock() {
            *o = new_orb;
            let _ = o.save_atomic(&state.orbital_path);
        }
        let _ = state.switch_app(&app, "orbital-haven");
        let status = state.status();
        state.emit_status_if_popover_open(&app);
        return Ok(status);
    }

    let new_world = WorldState::from_share_string(trimmed)?;
    if let Ok(mut w) = state.world.lock() {
        *w = new_world;
        let _ = w.save_atomic(&state.world_path);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    Ok(status)
}

#[tauri::command]
pub fn get_catalog(state: State<'_, HostState>) -> Vec<AppManifest> {
    AppCatalog::all_manifests(state.apps_dir.as_deref())
}

#[tauri::command]
pub fn reload_catalog(
    app: AppHandle,
    state: State<'_, HostState>,
) -> Result<Vec<AppManifest>, String> {
    let manifests = AppCatalog::all_manifests(state.apps_dir.as_deref());
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_else(|_| "living-rookery".into());
    if manifests.iter().any(|m| m.metadata.id == active_app) {
        crate::webview_sandbox::clear_loaded_webview_cache();
        let _ = state.switch_app(&app, &active_app);
    }
    state.emit_status_if_popover_open(&app);
    Ok(manifests)
}

#[tauri::command]
pub fn switch_app(
    app: AppHandle,
    state: State<'_, HostState>,
    app_id: String,
) -> Result<HostStatus, String> {
    log::info!("IPC switch_app: requested switch to '{app_id}'");
    state.switch_app(&app, &app_id)?;
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    Ok(status)
}

#[tauri::command]
pub fn pan_forest(app: AppHandle, state: State<'_, HostState>, delta_x: f32) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        w.pan_forest(delta_x);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn execute_app_action(
    app: AppHandle,
    state: State<'_, HostState>,
    app_id: String,
    action: String,
    payload: Option<serde_json::Value>,
) -> Result<HostStatus, String> {
    match app_id.as_str() {
        "living-rookery" => {
            let mut w = state.world.lock().map_err(|_| "lock poisoned")?;
            match action.as_str() {
                "feed" => {
                    w.drop_snack([500.0, 450.0]);
                    state.audio.play_sfx(SoundEffect::BirdChirp);
                }
                "prune" => {
                    w.prune_nearest([500.0, 400.0], 120.0);
                    state.audio.play_sfx(SoundEffect::Prune);
                }
                "plant" => {
                    let rx = 350.0 + (w.rng.f32() * 300.0);
                    w.plant_seed([rx, 750.0]);
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                "pan_left" => {
                    w.pan_forest(-150.0);
                }
                "pan_right" => {
                    w.pan_forest(150.0);
                }
                "cycle_style" | "cycle_palette" => {
                    w.cycle_palette();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "prev_style" | "prev_palette" => {
                    w.prev_palette();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "cycle_bird_style" => {
                    w.cycle_bird_style();
                    state.audio.play_sfx(SoundEffect::BirdChirp);
                }
                "sprout_now" => {
                    w.execute_growth_step();
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                "set_style" | "set_palette" | "aesthetic_style" | "color_palette" => {
                    if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                        let parsed_idx = match val {
                            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u8),
                            serde_json::Value::String(s) => s
                                .split(':')
                                .next()
                                .and_then(|prefix| prefix.trim().parse::<u8>().ok()),
                            _ => None,
                        };
                        if let Some(idx) = parsed_idx {
                            w.set_palette(rookery_core::world::RookeryPalette::from_u8(idx));
                            state.audio.play_sfx(SoundEffect::PaletteCycle);
                        }
                    } else {
                        w.cycle_palette();
                        state.audio.play_sfx(SoundEffect::PaletteCycle);
                    }
                }
                "set_debug_mode" => {
                    let enabled = payload
                        .as_ref()
                        .and_then(|p| p.get("enabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    w.debug_mode = enabled;
                    if let Ok(mut inner) = state.inner.lock() {
                        inner.settings.debug_mode = enabled;
                    }
                }
                "view_mature_tree" => {
                    let species_str = payload
                        .as_ref()
                        .and_then(|p| p.get("species"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("AncientOak");
                    let species = rookery_core::world::TreeSpecies::from_id(species_str)
                        .unwrap_or(rookery_core::world::TreeSpecies::AncientOak);
                    let seed = payload
                        .as_ref()
                        .and_then(|p| p.get("seed"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or_else(|| w.rng.next_u64());
                    w.view_mature_tree(species, seed);
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                "showcase_all_mature" => {
                    let seed = payload
                        .as_ref()
                        .and_then(|p| p.get("seed"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(42);
                    w.generate_mature_showcase(seed);
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "variety_grove" => {
                    let species_str = payload
                        .as_ref()
                        .and_then(|p| p.get("species"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("AncientOak");
                    let species = rookery_core::world::TreeSpecies::from_id(species_str)
                        .unwrap_or(rookery_core::world::TreeSpecies::AncientOak);
                    let seed = payload
                        .as_ref()
                        .and_then(|p| p.get("seed"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or_else(|| w.rng.next_u64());
                    w.generate_species_variety_grove(species, seed);
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "mature_active_tree" => {
                    let active_id = w.active_tree_id;
                    w.mature_tree_by_id(active_id, false);
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "growth_step" => {
                    let count = payload
                        .as_ref()
                        .and_then(|p| p.get("count"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u32;
                    let active_id = w.active_tree_id;
                    let active_idx = w.trees.iter().position(|t| t.id == active_id);
                    if let Some(idx) = active_idx {
                        for _ in 0..count {
                            w.execute_growth_step_for(idx, false);
                        }
                    } else {
                        w.execute_growth_step();
                    }
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                "rewind_sapling" => {
                    let active_id = w.active_tree_id;
                    w.rewind_tree_to_sapling(active_id);
                    state.audio.play_sfx(SoundEffect::Prune);
                }
                "focus_tree" => {
                    if let Some(tid) = payload
                        .as_ref()
                        .and_then(|p| p.get("tree_id"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                    {
                        w.focus_tree(tid);
                    }
                }
                "set_sim_speed" => {
                    if let Some(spd) = payload
                        .as_ref()
                        .and_then(|p| p.get("speed"))
                        .and_then(|v| v.as_f64())
                    {
                        w.sim_speed = (spd as f32).clamp(0.0, 100.0);
                    }
                }
                "set_bird_style" => {
                    if let Some(style_str) = payload
                        .as_ref()
                        .and_then(|p| p.get("style"))
                        .and_then(|v| v.as_str())
                    {
                        w.bird_style = match style_str.to_lowercase().as_str() {
                            "streamlined" | "2cell" => {
                                rookery_core::world::BirdStyle::Streamlined2Cell
                            }
                            "ornate" | "flock" => rookery_core::world::BirdStyle::OrnateFlock,
                            _ => rookery_core::world::BirdStyle::Classic3Cell,
                        };
                        state.audio.play_sfx(SoundEffect::BirdChirp);
                    }
                }
                "spawn_bird" => {
                    let sp_str = payload
                        .as_ref()
                        .and_then(|p| p.get("species"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("SnowyOwl");
                    let species = match sp_str.to_lowercase().replace(['-', '_', ' '], "").as_str()
                    {
                        "celestialraven" | "raven" => {
                            rookery_core::world::BirdSpecies::CelestialRaven
                        }
                        "rubytanager" | "tanager" => rookery_core::world::BirdSpecies::RubyTanager,
                        "snowyowl" | "owl" => rookery_core::world::BirdSpecies::SnowyOwl,
                        "azurejay" | "jay" => rookery_core::world::BirdSpecies::AzureJay,
                        "goldfinch" | "finch" => rookery_core::world::BirdSpecies::Goldfinch,
                        "robin" => rookery_core::world::BirdSpecies::Robin,
                        "emeralddove" | "dove" => rookery_core::world::BirdSpecies::EmeraldDove,
                        _ => rookery_core::world::BirdSpecies::Rook,
                    };
                    w.spawn_bird_of_species(species);
                    state.audio.play_sfx(SoundEffect::BirdChirp);
                }
                "spawn_fauna" => {
                    let sp_str = payload
                        .as_ref()
                        .and_then(|p| p.get("species"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Fox");
                    let species = match sp_str.to_lowercase().replace(['-', '_', ' '], "").as_str()
                    {
                        "squirrel" => rookery_core::world::FaunaSpecies::Squirrel,
                        "badger" => rookery_core::world::FaunaSpecies::Badger,
                        "hedgehog" => rookery_core::world::FaunaSpecies::Hedgehog,
                        "rabbit" | "cottontail" => rookery_core::world::FaunaSpecies::Rabbit,
                        _ => rookery_core::world::FaunaSpecies::Fox,
                    };
                    w.spawn_fauna_of_species(species);
                    state.audio.play_sfx(SoundEffect::Prune);
                }
                "scatter_snacks" => {
                    let trunk_base_x = w
                        .trees
                        .iter()
                        .find(|t| t.id == w.active_tree_id)
                        .map(|t| t.base_pos[0])
                        .unwrap_or(500.0);
                    let rx = w.rng.range(trunk_base_x - 180.0, trunk_base_x + 180.0);
                    let ry = w.rng.range(755.0, 825.0);
                    for f in &mut w.fauna {
                        if matches!(
                            f.state,
                            rookery_core::world::FaunaState::Trotting { .. }
                                | rookery_core::world::FaunaState::Sniffing { .. }
                        ) {
                            f.state = rookery_core::world::FaunaState::Foraging {
                                remaining_secs: 5.5,
                                target_snack: Some([rx, ry]),
                            };
                        }
                    }
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                "reset_world" => {
                    let seed = payload
                        .as_ref()
                        .and_then(|p| p.get("seed"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(42);
                    *w = rookery_core::world::WorldState::new(seed);
                    state.audio.play_sfx(SoundEffect::PlantSeed);
                }
                other => return Err(format!("Unknown rookery action '{other}'")),
            }
            drop(w);
            state.with_renderer(|r| r.trigger_action());
        }
        "koi-pond" => {
            let mut k = state
                .catalog_state
                .koi
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action.as_str() {
                "feed_koi" | "feed" => {
                    k.feed_koi([500.0, 375.0]);
                    state.audio.play_sfx(SoundEffect::FeedKoi);
                }
                "tap_water" | "pebble" => {
                    k.add_ripple([500.0, 375.0], 0.7);
                    state.audio.play_sfx(SoundEffect::WaterRipple);
                }
                other => return Err(format!("Unknown koi action '{other}'")),
            }
        }
        "cosmic-nebula" => {
            let mut n = state
                .catalog_state
                .nebula
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action.as_str() {
                "add_attractor" => {
                    n.add_gravity_well([500.0, 375.0], false);
                    state.audio.play_sfx(SoundEffect::NebulaPulse);
                }
                "add_repulsor" | "repulsor_pulse" => {
                    n.add_gravity_well([500.0, 375.0], true);
                    state.audio.play_sfx(SoundEffect::NebulaPulse);
                }
                "cycle_palette" => {
                    n.cycle_palette();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                other => return Err(format!("Unknown nebula action '{other}'")),
            }
        }
        "orbital-haven" => {
            let mut o = state
                .catalog_state
                .orbital
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action.as_str() {
                "overdrive_drones" | "add_module" => {
                    o.overdrive_drones();
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "launch_shuttle" => {
                    o.launch_shuttle();
                    state.audio.play_sfx(SoundEffect::ShuttleLaunch);
                }
                "cycle_focus" => {
                    o.cycle_focus();
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "focus_auto" => {
                    o.set_focus(ConstructionFocus::AutoBalance);
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "focus_solar" => {
                    o.set_focus(ConstructionFocus::SolarPower);
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "focus_habitat" => {
                    o.set_focus(ConstructionFocus::HabitatRing);
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "focus_shipyard" => {
                    o.set_focus(ConstructionFocus::ShipyardBiodome);
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "focus_comms" => {
                    o.set_focus(ConstructionFocus::CommsGrid);
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "cycle_planet" => {
                    o.cycle_planet_biome();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "scan_pulse" => {
                    o.trigger_scan_pulse(STATION_CENTER);
                    state.audio.play_sfx(SoundEffect::OrbitalScan);
                }
                "toggle_hud" => {
                    o.toggle_hud();
                    state.audio.play_sfx(SoundEffect::OrbitalScan);
                }
                other => return Err(format!("Unknown orbital action '{other}'")),
            }
            let _ = o.save_atomic(&state.orbital_path);
        }
        other => {
            let all = AppCatalog::all_manifests(state.apps_dir.as_deref());
            if let Some(manifest) = all.into_iter().find(|m| m.metadata.id == other) {
                if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    state.catalog_state.set_control_value(
                        other,
                        &action,
                        val_str.clone(),
                        Some(&manifest),
                    );
                    if let Ok(mut inner) = state.inner.lock() {
                        inner
                            .settings
                            .app_controls
                            .entry(other.to_string())
                            .or_default()
                            .insert(action.clone(), val_str);
                        if let Err(e) = inner.settings.save(&state.settings_path) {
                            log::warn!("failed to save control setting: {e}");
                        }
                    }
                    crate::webview_sandbox::dispatch_control(&app, &action, val);
                } else {
                    crate::webview_sandbox::dispatch_action(&app, &action);
                    state.with_renderer(|r| r.trigger_action());
                }
                log::info!("executed action '{action}' on external app '{other}'");
            } else {
                return Err(format!("Unknown app id '{other}'"));
            }
        }
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    Ok(status)
}

#[tauri::command]
pub fn capture_snapshot(app: AppHandle, state: State<'_, HostState>) -> Result<String, String> {
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_else(|_| "living-rookery".into());

    let pictures_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("Rookery");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{active_app}_{timestamp}.png");
    let filepath = pictures_dir.join(filename);

    let width = 1280u32;
    let height = 720u32;

    let offscreen = Offscreen::new(&state.gpu, width, height);
    let factory = AppCatalog::create_scene_factory_with_dir(
        &active_app,
        state.catalog_state.clone(),
        state.apps_dir.as_deref(),
    );
    let mut scene = factory(&state.gpu, OFFSCREEN_FORMAT);
    scene.resize(&state.gpu, width, height, 1.0);

    let input = SceneInput::default();
    scene.update(&state.gpu, Duration::from_millis(16), &input);

    let view = offscreen.create_view();
    let mut encoder = state
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("snapshot-encoder"),
        });
    scene.render(&state.gpu, &view, &mut encoder);
    state.gpu.queue.submit(Some(encoder.finish()));

    let rgba = offscreen
        .readback_rgba(&state.gpu)
        .map_err(|e| format!("Failed to readback snapshot frame: {e}"))?;

    write_png(&filepath, width, height, &rgba).map_err(|e| format!("Failed to write PNG: {e}"))?;

    log::info!("Exported snapshot to {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[tauri::command]
pub fn record_loop(app: AppHandle, state: State<'_, HostState>) -> Result<String, String> {
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_else(|_| "living-rookery".into());

    let pictures_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("Rookery");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("loop_{active_app}_{timestamp}.gif");
    let filepath = pictures_dir.join(filename);

    let width = 640u32;
    let height = 360u32;
    let total_frames = 60u32;
    let dt = Duration::from_millis(83);
    let delay_centisecs = 8u16;

    let offscreen = Offscreen::new(&state.gpu, width, height);
    let factory = AppCatalog::create_scene_factory_with_dir(
        &active_app,
        state.catalog_state.clone(),
        state.apps_dir.as_deref(),
    );
    let mut scene = factory(&state.gpu, OFFSCREEN_FORMAT);
    scene.resize(&state.gpu, width, height, 1.0);

    let input = SceneInput::default();
    let mut frames = Vec::with_capacity(total_frames as usize);

    for _ in 0..total_frames {
        scene.update(&state.gpu, dt, &input);
        let view = offscreen.create_view();
        let mut encoder =
            state
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("loop-frame-encoder"),
                });
        scene.render(&state.gpu, &view, &mut encoder);
        state.gpu.queue.submit(Some(encoder.finish()));

        let rgba = offscreen
            .readback_rgba(&state.gpu)
            .map_err(|e| format!("Failed to readback loop frame: {e}"))?;
        frames.push(rgba);
    }

    write_gif(
        &filepath,
        width as u16,
        height as u16,
        &mut frames,
        delay_centisecs,
    )
    .map_err(|e| format!("Failed to encode GIF: {e}"))?;

    log::info!("Exported loop to {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[tauri::command]
pub fn record_video(app: AppHandle, state: State<'_, HostState>) -> Result<String, String> {
    let active_app = state
        .inner
        .lock()
        .map(|i| i.settings.active_app.clone())
        .unwrap_or_else(|_| "living-rookery".into());

    let pictures_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("Rookery");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("clip_{active_app}_{timestamp}.mp4");
    let filepath = pictures_dir.join(filename);

    let width = 1280u32;
    let height = 720u32;
    let fps = 60u32;
    let total_frames = 300u32;
    let dt = Duration::from_micros(16_667);

    let offscreen = Offscreen::new(&state.gpu, width, height);
    let factory = AppCatalog::create_scene_factory_with_dir(
        &active_app,
        state.catalog_state.clone(),
        state.apps_dir.as_deref(),
    );
    let mut scene = factory(&state.gpu, OFFSCREEN_FORMAT);
    scene.resize(&state.gpu, width, height, 1.0);

    let input = SceneInput::default();

    let frame_iter = (0..total_frames).map(|_| {
        scene.update(&state.gpu, dt, &input);
        let view = offscreen.create_view();
        let mut encoder =
            state
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("video-frame-encoder"),
                });
        scene.render(&state.gpu, &view, &mut encoder);
        state.gpu.queue.submit(Some(encoder.finish()));

        offscreen.readback_rgba(&state.gpu)
    });

    encode_mp4_stream(&filepath, width, height, fps, frame_iter)
        .map_err(|e| format!("Failed to encode MP4 video: {e}"))?;

    log::info!("Exported MP4 video to {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[tauri::command]
pub fn set_world_seed(app: AppHandle, state: State<'_, HostState>, seed: u64) -> HostStatus {
    if let Ok(mut w) = state.world.lock() {
        *w = rookery_core::world::WorldState::new(seed);
        let _ = w.save_atomic(&state.world_path);
    }
    let status = state.status();
    state.emit_status_if_popover_open(&app);
    status
}

#[tauri::command]
pub fn relaunch_app(app: AppHandle, state: State<'_, HostState>) {
    state.clean_shutdown();
    app.restart();
}

#[tauri::command]
pub fn quit(app: AppHandle, state: State<'_, HostState>) {
    state.clean_shutdown();
    app.exit(0);
}
