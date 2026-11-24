//! Shared host state and IPC status payloads.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rookery_core::DisplayId;
use rookery_core::governor::{
    DisplayInputs, FrameBudget, GovernorInputs, PauseReason, PowerState, compute,
};
use rookery_core::interaction::{Effect, Interaction, InteractionEvent};
use rookery_core::manifest::{AppManifest, DayNightMapping, ScheduleMode};
use rookery_core::settings::{AudioSettings, Settings};
use rookery_core::world::WorldState;
use rookery_platform::{DesktopLayer, DisplayInfo};
use rookery_render::{AppCatalog, AudioEngine, CatalogState, Gpu, Renderer};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Window};

use crate::hotkey;

pub const POPOVER_LABEL: &str = "popover";
pub const STATUS_EVENT: &str = "rookery://status";

pub struct ManagedDisplay {
    pub info: DisplayInfo,
    pub window: Window,
    pub occluded: bool,
    pub uncovered: bool,
    pub budget: FrameBudget,
}

pub struct HostInner {
    pub settings: Settings,
    pub interaction: Interaction,
    pub user_paused: bool,
    pub power: PowerState,
    pub screens_asleep: bool,
    pub displays: BTreeMap<DisplayId, ManagedDisplay>,
}

pub struct HostState {
    pub settings_path: PathBuf,
    pub world_path: PathBuf,
    pub orbital_path: PathBuf,
    pub apps_dir: Option<PathBuf>,
    pub cached_manifests: Mutex<Vec<AppManifest>>,
    pub world: Arc<Mutex<WorldState>>,
    pub catalog_state: CatalogState,
    pub audio: AudioEngine,
    pub layer: Box<dyn DesktopLayer>,
    pub renderer: Mutex<Option<Renderer>>,
    pub gpu: Arc<Gpu>,
    pub inner: Mutex<HostInner>,
    pub popover_visible: AtomicBool,
    pub hotkey_recording: AtomicBool,
    pub settings_dirty: AtomicBool,
    pub world_dirty: AtomicBool,
    pub orbital_dirty: AtomicBool,
    pub shutting_down: AtomicBool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DisplayStatus {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub budget: FrameBudget,
    pub actual_fps: f32,
    pub frame_cpu_ms: f32,
    pub occluded: bool,
    pub uncovered: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RookeryStatus {
    pub seed: u64,
    pub total_age_secs: f64,
    pub total_offline_caught_up_secs: f64,
    pub total_branches: u32,
    pub total_foliage: u32,
    pub birds_active: u32,
    pub birds_perched: u32,
    pub pruned_branches: u32,
    pub seeds_planted: u32,
    pub snacks_eaten: u32,
    pub nests_built: u32,
    pub aesthetic_style: u8,
    pub aesthetic_style_name: String,
    pub bird_style: String,
    pub active_species: String,
    pub forest_trees_count: u32,
    pub active_tree_growth_steps: u32,
    pub active_tree_target_steps: u32,
    pub debug_mode: bool,
    pub sim_speed: f32,
    pub active_tree_id: u32,
    pub active_tree_is_mature: bool,
    pub active_tree_seed: u64,
    pub viewport_offset_x: f32,
    pub trees: Vec<rookery_core::world::TreeSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct KoiStatus {
    pub seed: u64,
    pub total_koi: u32,
    pub total_fish: u32,
    pub total_lilies: u32,
    pub blooming_lotuses: u32,
    pub ripples_active: u32,
    pub pellets_active: u32,
    pub pellets_eaten: u32,
    pub ripples_created: u32,
    pub theme: u8,
    pub theme_name: String,
    pub rain_enabled: bool,
    pub lanterns_enabled: bool,
    pub dragonflies: u32,
    pub color_shift_rate: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct NebulaStatus {
    pub seed: u64,
    pub stars_active: u32,
    pub protostars: u32,
    pub supernovae_total: u32,
    pub active_shockwaves: u32,
    pub active_comets: u32,
    pub active_meteors: u32,
    pub star_chart_enabled: bool,
    pub palette_idx: u8,
    pub palette_name: String,
    pub palette: String,
    pub color_shift_rate: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrbitalStatus {
    pub seed: u64,
    pub station_age_hours: f64,
    pub active_feed_code: String,
    pub active_feed_label: String,
    pub layout: String,
    pub auto_director: bool,
    pub palette: String,
    pub station_event: String,
    pub active_ships: u32,
    pub docked_ships: u32,
    pub active_drones: u32,
    pub active_agents: u32,
    pub uplink_callsign: String,
    pub uplink_phase: String,
    pub hud_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct GothicStatus {
    pub palette: u8,
    pub palette_blend: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct HostStatus {
    pub displays: Vec<DisplayStatus>,
    pub power: PowerState,
    pub interactive: bool,
    pub user_paused: bool,
    pub active_app: String,
    pub schedule_mode: ScheduleMode,
    pub audio: AudioSettings,
    pub rookery: Option<RookeryStatus>,
    pub koi: Option<KoiStatus>,
    pub nebula: Option<NebulaStatus>,
    pub orbital: Option<OrbitalStatus>,
    pub gothic: Option<GothicStatus>,
    pub app_stats: BTreeMap<String, BTreeMap<String, String>>,
    pub app_controls: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub hotkey: Option<String>,
    pub popover_hotkey: Option<String>,
    pub popover_opacity: Option<f32>,
    pub popover_blur: Option<u32>,
    pub active_app: Option<String>,
    pub favorite_apps: Option<Vec<String>>,
    pub favorite_hotkeys: Option<Vec<String>>,
    pub schedule_mode: Option<ScheduleMode>,
    pub schedule_interval_mins: Option<u32>,
    pub day_night_mapping: Option<DayNightMapping>,
    pub passive_fps: Option<u32>,
    pub interactive_fps: Option<u32>,
    pub low_power_fps: Option<u32>,
    pub pause_on_battery: Option<bool>,
    pub audio_enabled: Option<bool>,
    pub audio_volume: Option<f32>,
    pub audio_direct_mode_only: Option<bool>,
    pub update_auto_check: Option<bool>,
    pub update_auto_download: Option<bool>,
    pub debug_mode: Option<bool>,
}

impl HostState {
    pub fn with_renderer<R>(&self, f: impl FnOnce(&Renderer) -> R) -> Option<R> {
        let guard = self.renderer.lock().ok()?;
        guard.as_ref().map(f)
    }

    /// Return cached app manifests without hitting the filesystem.
    pub fn manifests(&self) -> Vec<AppManifest> {
        if let Ok(guard) = self.cached_manifests.lock()
            && !guard.is_empty()
        {
            return guard.clone();
        }
        self.refresh_manifests()
    }

    /// Rescan app manifests from disk and update the in-memory cache.
    pub fn refresh_manifests(&self) -> Vec<AppManifest> {
        let manifests = AppCatalog::all_manifests(self.apps_dir.as_deref());
        if let Ok(mut guard) = self.cached_manifests.lock() {
            *guard = manifests.clone();
        }
        manifests
    }

    pub fn mark_settings_dirty(&self) {
        self.settings_dirty.store(true, Ordering::Release);
    }

    pub fn mark_world_dirty(&self) {
        self.world_dirty.store(true, Ordering::Release);
    }

    pub fn mark_orbital_dirty(&self) {
        self.orbital_dirty.store(true, Ordering::Release);
    }

    /// Flush any pending dirty settings or simulation state to disk on the background thread.
    pub fn flush_dirty_state(&self) {
        if self.settings_dirty.swap(false, Ordering::AcqRel) {
            let settings_snapshot = self.inner.lock().ok().map(|i| i.settings.clone());
            if let Some(settings) = settings_snapshot
                && let Err(e) = settings.save(&self.settings_path)
            {
                log::warn!("deferred settings save failed: {e}");
            }
        }
        if self.world_dirty.swap(false, Ordering::AcqRel)
            && let Ok(mut w) = self.world.lock()
            && let Err(e) = w.save_atomic(&self.world_path)
        {
            log::warn!("deferred world save failed: {e}");
        }
        if self.orbital_dirty.swap(false, Ordering::AcqRel)
            && let Ok(o) = self.catalog_state.orbital.lock()
            && let Err(e) = o.save_atomic(&self.orbital_path)
        {
            log::warn!("deferred orbital save failed: {e}");
        }
    }

    /// Recompute governor budgets from current state and push them to the render thread.
    pub fn recompute_budgets(&self) {
        let stats = self.with_renderer(|r| r.stats()).unwrap_or_default();
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let display_inputs: Vec<DisplayInputs> = inner
            .displays
            .values()
            .map(|d| DisplayInputs {
                id: d.info.id.clone(),
                occluded: d.occluded,
                activity: stats
                    .get(&d.info.id)
                    .map(|s| s.activity)
                    .unwrap_or_default(),
                uncovered: d.uncovered,
            })
            .collect();

        let budgets = compute(&GovernorInputs {
            displays: &display_inputs,
            power: inner.power,
            interactive: inner.interaction.is_interactive(),
            user_paused: inner.user_paused,
            screens_asleep: inner.screens_asleep,
            settings: &inner.settings.frame,
        });

        let audio_paused = inner.user_paused || inner.screens_asleep;
        self.audio.set_paused(audio_paused);

        for (id, budget) in budgets {
            if let Some(d) = inner.displays.get_mut(&id) {
                d.budget = budget;
            }
            self.with_renderer(|r| r.set_budget(id, budget));
        }
    }

    /// Snapshot current status for the popover UI, locking only the active app's simulation.
    pub fn status(&self) -> HostStatus {
        let stats = self.with_renderer(|r| r.stats()).unwrap_or_default();
        let (active_app, schedule_mode, power, interactive, user_paused, audio, displays) = {
            let inner = self.inner.lock().expect("host state poisoned");
            let displays = inner
                .displays
                .values()
                .map(|d| {
                    let st = stats.get(&d.info.id);
                    DisplayStatus {
                        id: d.info.id.0.clone(),
                        name: d.info.name.clone(),
                        width: d.info.width_px,
                        height: d.info.height_px,
                        budget: d.budget,
                        actual_fps: st.map(|s| s.actual_fps).unwrap_or(0.0),
                        frame_cpu_ms: st.map(|s| s.frame_cpu_ms).unwrap_or(0.0),
                        occluded: d.occluded,
                        uncovered: d.uncovered,
                    }
                })
                .collect();
            (
                inner.settings.active_app.clone(),
                inner.settings.schedule_mode,
                inner.power,
                inner.interaction.is_interactive(),
                inner.user_paused,
                inner.settings.audio,
                displays,
            )
        };

        let mut rookery = None;
        let mut koi = None;
        let mut nebula = None;
        let mut orbital = None;
        let mut gothic = None;

        match active_app.as_str() {
            "living-rookery" => {
                rookery = self.world.lock().ok().map(|w| {
                    let active_tree = w.trees.iter().find(|t| t.id == w.active_tree_id);
                    let active_species = active_tree
                        .map(|t| t.species.display_name().to_string())
                        .unwrap_or_else(|| "Ancient Oak".to_string());
                    let active_growth = active_tree.map(|t| t.growth_steps_completed).unwrap_or(0);
                    let active_target = active_tree.map(|t| t.target_growth_steps).unwrap_or(42);
                    let active_is_mature = active_tree.map(|t| t.is_mature).unwrap_or(false);
                    let active_seed = active_tree.map(|t| t.seed).unwrap_or(w.seed);
                    let tree_summaries = if w.debug_mode {
                        w.tree_summary_list()
                    } else {
                        Vec::new()
                    };

                    RookeryStatus {
                        seed: w.seed,
                        total_age_secs: w.total_age_secs,
                        total_offline_caught_up_secs: w.total_offline_caught_up_secs,
                        total_branches: w.stats.total_branches,
                        total_foliage: w.stats.total_foliage,
                        birds_active: w.stats.birds_active,
                        birds_perched: w.stats.birds_perched,
                        pruned_branches: w.stats.pruned_branches,
                        seeds_planted: w.stats.seeds_planted,
                        snacks_eaten: w.stats.snacks_eaten,
                        nests_built: w.stats.nests_built + w.stats.houses_built,
                        aesthetic_style: w.aesthetic_style,
                        aesthetic_style_name: w.aesthetic_style_name().to_string(),
                        bird_style: w.bird_style.id().to_string(),
                        active_species,
                        forest_trees_count: w.trees.len() as u32,
                        active_tree_growth_steps: active_growth,
                        active_tree_target_steps: active_target,
                        debug_mode: w.debug_mode,
                        sim_speed: w.sim_speed,
                        active_tree_id: w.active_tree_id,
                        active_tree_is_mature: active_is_mature,
                        active_tree_seed: active_seed,
                        viewport_offset_x: w.viewport_offset_x,
                        trees: tree_summaries,
                    }
                });
            }
            "koi-pond" => {
                koi = self.catalog_state.koi.lock().ok().map(|k| {
                    let rate_str = format_rate_clean(k.color_shift_rate);
                    self.catalog_state
                        .set_control_value("koi-pond", "color_speed", rate_str, None);
                    KoiStatus {
                        seed: k.seed,
                        total_koi: k.stats.total_fish,
                        total_fish: k.stats.total_fish,
                        total_lilies: k.stats.total_lilies,
                        blooming_lotuses: k.lilies.iter().filter(|l| l.has_flower).count() as u32,
                        ripples_active: k.ripples.len() as u32,
                        pellets_active: k.pellets.len() as u32,
                        pellets_eaten: k.stats.pellets_eaten,
                        ripples_created: k.stats.ripples_created,
                        theme: k.theme.index(),
                        theme_name: k.theme.name().to_string(),
                        rain_enabled: k.rain_enabled,
                        lanterns_enabled: k.lanterns_enabled,
                        dragonflies: k.dragonflies.len() as u32,
                        color_shift_rate: k.color_shift_rate,
                    }
                });
            }
            "cosmic-nebula" => {
                nebula = self.catalog_state.nebula.lock().ok().map(|n| {
                    let palette_name = n.palette.name().to_string();
                    let rate_str = format_rate_clean(n.color_shift_rate);
                    self.catalog_state.set_control_value(
                        "cosmic-nebula",
                        "color_speed",
                        rate_str,
                        None,
                    );
                    NebulaStatus {
                        seed: n.seed,
                        stars_active: n.stats.stars_active,
                        protostars: n.stats.protostars,
                        supernovae_total: n.stats.supernovae_total,
                        active_shockwaves: n.stats.active_shockwaves,
                        active_comets: n.stats.active_comets,
                        active_meteors: n.stats.active_meteors,
                        star_chart_enabled: n.star_chart_enabled,
                        palette_idx: n.palette.index(),
                        palette_name: palette_name.clone(),
                        palette: palette_name,
                        color_shift_rate: n.color_shift_rate,
                    }
                });
            }
            "orbital-haven" => {
                orbital = self
                    .catalog_state
                    .orbital
                    .lock()
                    .ok()
                    .map(|o| OrbitalStatus {
                        seed: o.seed,
                        station_age_hours: o.stats.station_age_hours,
                        active_feed_code: o.camera_rig.primary_feed.code().to_string(),
                        active_feed_label: o.camera_rig.primary_feed.short_label().to_string(),
                        layout: o.camera_rig.layout.short_name().to_string(),
                        auto_director: o.camera_rig.auto_director,
                        palette: o.palette.name().to_string(),
                        station_event: o.event.name().to_string(),
                        active_ships: o.stats.active_ships,
                        docked_ships: o.stats.docked_ships,
                        active_drones: o.stats.active_drones,
                        active_agents: o.stats.active_agents,
                        uplink_callsign: o.stats.uplink_callsign.clone(),
                        uplink_phase: o.stats.uplink_phase.clone(),
                        hud_enabled: o.hud_enabled,
                    });
            }
            "shinjuku-rain" => {
                if let Ok(s) = self.catalog_state.shinjuku.lock() {
                    let st = s.stats();
                    let mut map = std::collections::BTreeMap::new();
                    map.insert(
                        "rain_rate".to_string(),
                        format!("{} mm/hr", st.rain_rate_mm_hr),
                    );
                    map.insert("pedestrians".to_string(), st.pedestrian_count.to_string());
                    map.insert("trains".to_string(), st.trains_passed.to_string());
                    map.insert("palette".to_string(), st.palette_name.clone());
                    self.catalog_state.set_stats("shinjuku-rain", map);
                    self.catalog_state.set_control_value(
                        "shinjuku-rain",
                        "color_speed",
                        format_rate_clean(s.color_shift_rate),
                        None,
                    );
                    self.catalog_state.set_control_value(
                        "shinjuku-rain",
                        "color_palette",
                        format!("{}: {}", s.palette as u8, st.palette_name),
                        None,
                    );
                }
            }
            "gothic-metropolis" => {
                if let Ok(g) = self.catalog_state.gothic.lock() {
                    let st = g.stats();
                    let mut map = std::collections::BTreeMap::new();
                    map.insert(
                        "trains".to_string(),
                        format!("{} Active · #{}", st.active_trains, st.trains_dispatched),
                    );
                    map.insert(
                        "airships".to_string(),
                        format!(
                            "{} Blimps · {} Cars",
                            st.active_airships, st.street_vehicles
                        ),
                    );
                    map.insert(
                        "walkways".to_string(),
                        format!("{} Walkers", st.walkway_pedestrians),
                    );
                    map.insert("palette".to_string(), st.palette_name.clone());
                    self.catalog_state.set_stats("gothic-metropolis", map);
                    self.catalog_state.set_control_value(
                        "gothic-metropolis",
                        "toggle_palette_cycle",
                        g.palette_cycle_enabled.to_string(),
                        None,
                    );
                    self.catalog_state.set_control_value(
                        "gothic-metropolis",
                        "palette_cycle_speed",
                        format!("{:.1}", g.palette_cycle_speed),
                        None,
                    );
                    self.catalog_state.set_control_value(
                        "gothic-metropolis",
                        "color_palette",
                        format!("{}: {}", g.palette as u8, st.palette_name),
                        None,
                    );
                    gothic = Some(GothicStatus {
                        palette: g.palette as u8,
                        palette_blend: g.palette_blend,
                    });
                }
            }
            _ => {}
        }

        HostStatus {
            displays,
            power,
            interactive,
            user_paused,
            active_app,
            schedule_mode,
            audio,
            rookery,
            koi,
            nebula,
            orbital,
            gothic,
            app_stats: self.catalog_state.all_stats(),
            app_controls: self.catalog_state.all_controls(),
        }
    }

    /// Hot-switch running scene to a new catalog app.
    pub fn switch_app(&self, app: &AppHandle, app_id: &str) -> Result<(), String> {
        let all = self.manifests();
        let Some(manifest) = all.iter().find(|m| m.metadata.id == app_id) else {
            log::warn!("switch_app: unknown app id '{app_id}'");
            return Err(format!("Unknown app id '{app_id}'"));
        };
        if let Ok(mut inner) = self.inner.lock() {
            inner.settings.active_app = app_id.to_string();
        }
        self.mark_settings_dirty();

        let factory = AppCatalog::create_scene_factory_with_dir(
            app_id,
            self.catalog_state.clone(),
            self.apps_dir.as_deref(),
        );
        self.audio.switch_app(app_id);
        let theme_color = manifest
            .theme
            .border_rgb_f32()
            .unwrap_or_else(|| AppCatalog::theme_color(app_id));
        self.with_renderer(|r| {
            r.set_theme_color(theme_color);
            r.switch_scene(factory);
        });

        let app_clone = app.clone();
        let app_id_str = app_id.to_string();
        let apps_dir = self.apps_dir.clone();
        let popover_open = self.popover_visible.load(Ordering::Relaxed);
        let _ = app.run_on_main_thread(move || {
            crate::webview_sandbox::update_webview_wallpaper(
                &app_clone,
                &app_id_str,
                apps_dir.as_deref(),
            );
            // Ensure popover window retains focus only when currently open/visible
            if popover_open && let Some(p) = app_clone.get_webview_window(POPOVER_LABEL) {
                let _ = p.set_focus();
                let _ = rookery_platform::activate_app_and_focus_window(&p);
            }
        });

        Ok(())
    }

    /// Emit `rookery://status` if and only if the popover window is currently visible.
    pub fn emit_status_if_popover_open(&self, app: &AppHandle) {
        if self.popover_visible.load(Ordering::Relaxed)
            && app.get_webview_window(POPOVER_LABEL).is_some()
        {
            let status = self.status();
            let _ = app.emit_to(POPOVER_LABEL, STATUS_EVENT, status);
        }
    }

    /// Feed an [`InteractionEvent`] into the state machine and execute the resulting effects
    /// on the main thread.
    pub fn apply_interaction_event(&self, app: &AppHandle, event: InteractionEvent) {
        let effects = {
            let mut inner = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let eff = inner.interaction.handle(event);
            self.audio
                .set_interactive(inner.interaction.is_interactive());
            eff
        };
        if effects.is_empty() {
            return;
        }

        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            let state = app_clone.state::<HostState>();
            for effect in effects {
                match effect {
                    Effect::Promote => {
                        if let Ok(inner) = state.inner.lock() {
                            for d in inner.displays.values() {
                                if !crate::webview_sandbox::set_interactive_webviews(
                                    &app_clone, true, &d.info,
                                ) && let Err(e) =
                                    state.layer.promote_interactive(&d.window, &d.info)
                                {
                                    log::error!("promote failed on {}: {e:#}", d.info.id);
                                }
                            }
                        }
                    }
                    Effect::Demote => {
                        if let Ok(inner) = state.inner.lock() {
                            for d in inner.displays.values() {
                                if !crate::webview_sandbox::set_interactive_webviews(
                                    &app_clone, false, &d.info,
                                ) && let Err(e) =
                                    state.layer.demote_wallpaper(&d.window, &d.info)
                                {
                                    log::error!("demote failed on {}: {e:#}", d.info.id);
                                }
                            }
                        }
                    }
                    Effect::ShowIndicator(on) => {
                        state.with_renderer(|r| r.set_interactive(on));
                    }
                    Effect::ListenForEscape(on) => {
                        hotkey::set_escape_listener(&app_clone, on);
                    }
                }
            }
            state.recompute_budgets();
            state.emit_status_if_popover_open(&app_clone);
        });
    }

    /// Persist clean shutdown and stop the render thread.
    pub fn clean_shutdown(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        if let Ok(mut inner) = self.inner.lock() {
            inner.settings.mark_clean_shutdown(Utc::now());
            if let Err(e) = inner.settings.save(&self.settings_path) {
                log::warn!("failed to save settings on shutdown: {e}");
            }
        }
        if let Ok(mut w) = self.world.lock()
            && let Err(e) = w.save_atomic(&self.world_path)
        {
            log::warn!("failed to save world state on shutdown: {e}");
        }
        if let Ok(o) = self.catalog_state.orbital.lock()
            && let Err(e) = o.save_atomic(&self.orbital_path)
        {
            log::warn!("failed to save orbital state on shutdown: {e}");
        }
        if let Ok(mut r) = self.renderer.lock()
            && let Some(mut renderer) = r.take()
        {
            renderer.shutdown();
        }
    }
}

pub fn initial_budget() -> FrameBudget {
    FrameBudget::Paused(PauseReason::Occluded)
}

fn format_rate_clean(v: f32) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    let s = format!("{rounded:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
