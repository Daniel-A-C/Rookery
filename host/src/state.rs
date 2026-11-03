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
use rookery_core::settings::Settings;
use rookery_core::world::WorldState;
use rookery_platform::{DesktopLayer, DisplayInfo};
use rookery_render::{Gpu, Renderer};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Window};

use crate::hotkey;

pub const POPOVER_LABEL: &str = "popover";
pub const STATUS_EVENT: &str = "rookery://status";

pub struct ManagedDisplay {
    pub info: DisplayInfo,
    pub window: Window,
    pub occluded: bool,
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
    pub world: Arc<Mutex<WorldState>>,
    pub layer: Box<dyn DesktopLayer>,
    pub renderer: Mutex<Option<Renderer>>,
    pub gpu: Arc<Gpu>,
    pub inner: Mutex<HostInner>,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct HostStatus {
    pub displays: Vec<DisplayStatus>,
    pub power: PowerState,
    pub interactive: bool,
    pub user_paused: bool,
    pub rookery: Option<RookeryStatus>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub hotkey: Option<String>,
    pub passive_fps: Option<u32>,
    pub interactive_fps: Option<u32>,
    pub low_power_fps: Option<u32>,
    pub pause_on_battery: Option<bool>,
}

impl HostState {
    pub fn with_renderer<R>(&self, f: impl FnOnce(&Renderer) -> R) -> Option<R> {
        let guard = self.renderer.lock().ok()?;
        guard.as_ref().map(f)
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

        for (id, budget) in budgets {
            if let Some(d) = inner.displays.get_mut(&id) {
                d.budget = budget;
            }
            self.with_renderer(|r| r.set_budget(id, budget));
        }
    }

    /// Snapshot current status for the popover UI.
    pub fn status(&self) -> HostStatus {
        let stats = self.with_renderer(|r| r.stats()).unwrap_or_default();
        let rookery = self.world.lock().ok().map(|w| RookeryStatus {
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
            nests_built: w.stats.nests_built,
        });

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
                }
            })
            .collect();
        HostStatus {
            displays,
            power: inner.power,
            interactive: inner.interaction.is_interactive(),
            user_paused: inner.user_paused,
            rookery,
        }
    }

    /// Emit `rookery://status` if and only if the popover window is currently open.
    pub fn emit_status_if_popover_open(&self, app: &AppHandle) {
        if app.get_webview_window(POPOVER_LABEL).is_some() {
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
            inner.interaction.handle(event)
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
                                if let Err(e) = state.layer.promote_interactive(&d.window, &d.info)
                                {
                                    log::error!("promote failed on {}: {e:#}", d.info.id);
                                }
                            }
                        }
                    }
                    Effect::Demote => {
                        if let Ok(inner) = state.inner.lock() {
                            for d in inner.displays.values() {
                                if let Err(e) = state.layer.demote_wallpaper(&d.window, &d.info) {
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
