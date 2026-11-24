//! Per-app action and control dispatch for built-in and external Rookery apps.

use rookery_core::orbital::{ConstructionFocus, STATION_CENTER};
use rookery_render::SoundEffect;
use tauri::AppHandle;

use crate::commands::open_directory_in_os;
use crate::state::HostState;

pub(crate) fn dispatch_app_action(
    app: &AppHandle,
    state: &HostState,
    app_id: &str,
    action: &str,
    payload: Option<serde_json::Value>,
) -> Result<(), String> {
    match app_id {
        "living-rookery" => {
            let mut w = state.world.lock().map_err(|_| "lock poisoned")?;
            match action {
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
                    let was_debug = w.debug_mode;
                    *w = rookery_core::world::WorldState::new(seed);
                    w.debug_mode = was_debug;
                    state.mark_world_dirty();
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
            match action {
                "feed_koi" | "feed" => {
                    k.feed_koi([500.0, 375.0]);
                    state.audio.play_sfx(SoundEffect::FeedKoi);
                }
                "tap_water" | "pebble" => {
                    k.add_ripple([500.0, 375.0], 1.15);
                    state.audio.play_sfx(SoundEffect::WaterRipple);
                }
                "spawn_koi" => {
                    k.spawn_koi();
                    state.audio.play_sfx(SoundEffect::WaterRipple);
                }
                "bloom_lotuses" => {
                    k.bloom_all_lotuses();
                    state.audio.play_sfx(SoundEffect::LotusChime);
                }
                "toggle_rain" => {
                    k.toggle_rain();
                    state.audio.play_sfx(SoundEffect::WaterRipple);
                }
                "toggle_lanterns" => {
                    k.toggle_lanterns();
                    state.audio.play_sfx(SoundEffect::LotusChime);
                }
                "trigger_spout" => {
                    k.trigger_bamboo_spout();
                    state.audio.play_sfx(SoundEffect::BambooClack);
                }
                "cycle_theme" | "cycle_palette" => {
                    k.cycle_theme();
                    state.audio.play_sfx(SoundEffect::LotusChime);
                }
                "set_theme" | "theme" => {
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
                            k.set_theme(rookery_core::koi::PondTheme::from_u8(idx));
                            state.audio.play_sfx(SoundEffect::LotusChime);
                        }
                    } else {
                        k.cycle_theme();
                        state.audio.play_sfx(SoundEffect::LotusChime);
                    }
                }
                other => return Err(format!("Unknown koi action '{other}'")),
            }
            drop(k);
            state.with_renderer(|r| r.trigger_action());
        }
        "cosmic-nebula" => {
            let mut n = state
                .catalog_state
                .nebula
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action {
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
            match action {
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
            drop(o);
            state.mark_orbital_dirty();
        }
        "shinjuku-rain" => {
            let mut s = state
                .catalog_state
                .shinjuku
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action {
                "splash" => {
                    let rx = s.rng.range(180.0, 820.0);
                    let ry = s.rng.range(780.0, 940.0);
                    s.splash_at(rx, ry);
                    state.audio.play_sfx(SoundEffect::RainSplash);
                }
                "dispatch_train" => {
                    s.dispatch_train();
                    state.audio.play_sfx(SoundEffect::TrainPass);
                }
                "wind_gust" => {
                    s.trigger_wind_gust();
                }
                "toggle_signs" => {
                    s.toggle_signs();
                    state.audio.play_sfx(SoundEffect::SignToggle);
                }
                "thunder" => {
                    s.trigger_lightning();
                    state.audio.play_sfx(SoundEffect::ThunderRumble);
                }
                "cycle_palette" => {
                    s.cycle_palette();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "color_palette" | "set_palette" => {
                    if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                        let parsed_idx = match val {
                            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u8),
                            serde_json::Value::String(str_val) => str_val
                                .split(':')
                                .next()
                                .and_then(|prefix| prefix.trim().parse::<u8>().ok()),
                            _ => None,
                        };
                        if let Some(idx) = parsed_idx {
                            s.set_palette(rookery_core::shinjuku::ShinjukuPalette::from_u8(idx));
                            state.audio.play_sfx(SoundEffect::PaletteCycle);
                        }
                    } else {
                        s.cycle_palette();
                        state.audio.play_sfx(SoundEffect::PaletteCycle);
                    }
                }
                other => return Err(format!("Unknown shinjuku-rain action '{other}'")),
            }
        }
        "gothic-metropolis" => {
            let mut g = state
                .catalog_state
                .gothic
                .lock()
                .map_err(|_| "lock poisoned")?;
            match action {
                "dispatch_train" => {
                    g.dispatch_train();
                    state.audio.play_sfx(SoundEffect::TrainPass);
                }
                "dispatch_interceptor" => {
                    g.dispatch_interceptor();
                    state.audio.play_sfx(SoundEffect::ShuttleLaunch);
                }
                "toggle_rush_hour" => {
                    g.toggle_rush_hour();
                    state.audio.play_sfx(SoundEffect::SignToggle);
                }
                "green_wave" => {
                    g.trigger_green_wave();
                    state.audio.play_sfx(SoundEffect::SignToggle);
                }
                "toggle_follow_car" => {
                    g.toggle_follow_car();
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "summon_airship" => {
                    g.summon_airship();
                    state.audio.play_sfx(SoundEffect::ShuttleLaunch);
                }
                "toggle_searchlights" => {
                    g.toggle_searchlights();
                    state.audio.play_sfx(SoundEffect::SignToggle);
                }
                "rotate_camera" => {
                    g.rotate_camera_quarter_turn();
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "reset_camera" => {
                    g.reset_camera();
                    state.audio.play_sfx(SoundEffect::ModuleConstruct);
                }
                "cycle_palette" => {
                    g.cycle_palette();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "toggle_palette_cycle" => {
                    g.toggle_palette_cycle();
                    state.audio.play_sfx(SoundEffect::PaletteCycle);
                }
                "palette_cycle_speed" => {
                    if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                        let parsed = match val {
                            serde_json::Value::Number(n) => n.as_f64().map(|v| v as f32),
                            serde_json::Value::String(s) => s.trim().parse::<f32>().ok(),
                            _ => None,
                        };
                        if let Some(speed) = parsed {
                            g.set_palette_cycle_speed(speed);
                        }
                    }
                }
                "color_palette" | "set_palette" => {
                    if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                        let parsed_idx = match val {
                            serde_json::Value::Number(n) => n.as_u64().map(|v| v as u8),
                            serde_json::Value::String(str_val) => str_val
                                .split(':')
                                .next()
                                .and_then(|prefix| prefix.trim().parse::<u8>().ok()),
                            _ => None,
                        };
                        if let Some(idx) = parsed_idx {
                            g.set_palette(rookery_core::gothic::GothicPalette::from_u8(idx));
                            state.audio.play_sfx(SoundEffect::PaletteCycle);
                        }
                    } else {
                        g.cycle_palette();
                        state.audio.play_sfx(SoundEffect::PaletteCycle);
                    }
                }
                other => return Err(format!("Unknown gothic-metropolis action '{other}'")),
            }
        }
        other => {
            let all = state.manifests();
            if let Some(manifest) = all.into_iter().find(|m| m.metadata.id == other) {
                if let Some(val) = payload.as_ref().and_then(|p| p.get("value")) {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    state.catalog_state.set_control_value(
                        other,
                        action,
                        val_str.clone(),
                        Some(&manifest),
                    );
                    if let Ok(mut inner) = state.inner.lock() {
                        inner
                            .settings
                            .app_controls
                            .entry(other.to_string())
                            .or_default()
                            .insert(action.to_string(), val_str);
                    }
                    state.mark_settings_dirty();
                    crate::webview_sandbox::dispatch_control(app, action, val);
                } else {
                    if other == "dynamic-wallpaper" && action == "open_media_folder" {
                        let base_apps = state
                            .apps_dir
                            .clone()
                            .unwrap_or_else(crate::webview_sandbox::get_apps_dir);
                        let media_dir = base_apps.join("dynamic-wallpaper").join("media");
                        let _ = std::fs::create_dir_all(&media_dir);
                        open_directory_in_os(&media_dir);
                    } else {
                        crate::webview_sandbox::dispatch_action(app, action);
                        state.with_renderer(|r| r.trigger_action());
                    }
                }
                log::info!("executed action '{action}' on external app '{other}'");
            } else {
                return Err(format!("Unknown app id '{other}'"));
            }
        }
    }
    Ok(())
}
