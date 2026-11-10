//! First-Party App Catalog & Open App SDK (Milestone 4).
//!
//! Provides declarative manifests, runtime app creation, external manifest loading,
//! and shared simulation state ownership across the flagship catalog.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rookery_core::koi::KoiPondState;
use rookery_core::manifest::{
    AppArchetype, AppBudgets, AppControl, AppManifest, AppMetadata, AppTheme,
};
use rookery_core::nebula::CosmicNebulaState;
use rookery_core::orbital::OrbitalHavenState;
use rookery_core::world::WorldState;

use crate::SceneFactory;
use crate::ascii_scene::LivingRookeryScene;
use crate::audio::AudioEngine;
use crate::koi_scene::KoiPondScene;
use crate::nebula_scene::CosmicNebulaScene;
use crate::orbital_scene::OrbitalHavenScene;

/// Shared `[f32; 4]` uniform parameter slot handle for an external shader app.
pub type SharedShaderParams = Arc<Mutex<[f32; 4]>>;

/// Per-app key-value map (`app_id -> key -> value`).
pub type AppStringMap = BTreeMap<String, BTreeMap<String, String>>;

/// Shared container holding simulation state for all installed catalog apps.
#[derive(Clone)]
pub struct CatalogState {
    pub rookery: Arc<Mutex<WorldState>>,
    pub koi: Arc<Mutex<KoiPondState>>,
    pub nebula: Arc<Mutex<CosmicNebulaState>>,
    pub orbital: Arc<Mutex<OrbitalHavenState>>,
    pub audio: Option<AudioEngine>,
    /// Per-app control values (`app_id -> control_id -> string_value`).
    pub custom_controls: Arc<Mutex<AppStringMap>>,
    /// Per-app shader uniform parameter slots (`app_id -> [f32; 4]`).
    pub custom_params: Arc<Mutex<BTreeMap<String, SharedShaderParams>>>,
    /// Per-app live telemetry stats (`app_id -> stat_id -> formatted_value`).
    pub custom_stats: Arc<Mutex<AppStringMap>>,
}

impl CatalogState {
    pub fn new(seed: u64) -> Self {
        Self {
            rookery: Arc::new(Mutex::new(WorldState::new(seed))),
            koi: Arc::new(Mutex::new(KoiPondState::new(seed))),
            nebula: Arc::new(Mutex::new(CosmicNebulaState::new(seed))),
            orbital: Arc::new(Mutex::new(OrbitalHavenState::new(seed))),
            audio: None,
            custom_controls: Arc::new(Mutex::new(BTreeMap::new())),
            custom_params: Arc::new(Mutex::new(BTreeMap::new())),
            custom_stats: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn with_audio(mut self, audio: AudioEngine) -> Self {
        self.audio = Some(audio);
        self
    }

    /// Pre-populate custom controls from persisted settings.
    pub fn with_controls(self, controls: BTreeMap<String, BTreeMap<String, String>>) -> Self {
        if let Ok(mut map) = self.custom_controls.lock() {
            *map = controls;
        }
        self
    }

    /// Advance offline catchup across all catalog apps.
    pub fn on_wake_all(&self, elapsed_secs: f64) {
        if let Ok(mut w) = self.rookery.lock() {
            w.on_wake(elapsed_secs);
        }
        if let Ok(mut k) = self.koi.lock() {
            k.on_wake(elapsed_secs);
        }
        if let Ok(mut n) = self.nebula.lock() {
            n.on_wake(elapsed_secs);
        }
        if let Ok(mut o) = self.orbital.lock() {
            o.on_wake(elapsed_secs);
        }
    }

    /// Compute the 4-float uniform array for an app based on its manifest controls and current values.
    fn compute_params(
        manifest: Option<&AppManifest>,
        values: &BTreeMap<String, String>,
    ) -> [f32; 4] {
        let mut out = [1.0, 1.0, 0.0, 0.0];
        let Some(m) = manifest else {
            return out;
        };
        let mut slot = 0usize;
        for ctrl in &m.controls {
            if slot >= 4 {
                break;
            }
            match ctrl {
                AppControl::Slider {
                    id, default_val, ..
                } => {
                    let v = values
                        .get(id)
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(*default_val);
                    out[slot] = v;
                    slot += 1;
                }
                AppControl::Select {
                    id,
                    options,
                    default_val,
                    ..
                } => {
                    let current = values.get(id).unwrap_or(default_val);
                    let idx = options.iter().position(|o| o == current).unwrap_or(0);
                    out[slot] = idx as f32;
                    slot += 1;
                }
                _ => {}
            }
        }
        out
    }

    /// Retrieve or initialize the shared `[f32; 4]` uniform parameters handle for `app_id`.
    pub fn params_for_app(
        &self,
        app_id: &str,
        manifest: Option<&AppManifest>,
    ) -> Arc<Mutex<[f32; 4]>> {
        let values = self
            .custom_controls
            .lock()
            .ok()
            .and_then(|map| map.get(app_id).cloned())
            .unwrap_or_default();
        let computed = Self::compute_params(manifest, &values);

        if let Ok(mut params_map) = self.custom_params.lock() {
            let entry = params_map
                .entry(app_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(computed)));
            if let Ok(mut guard) = entry.lock() {
                let preserved_phase = guard[3];
                *guard = computed;
                if computed[3] == 0.0 {
                    guard[3] = preserved_phase;
                }
            }
            entry.clone()
        } else {
            Arc::new(Mutex::new(computed))
        }
    }

    /// Record a control update for `app_id`, refreshing shader parameters and matching stat displays.
    pub fn set_control_value(
        &self,
        app_id: &str,
        control_id: &str,
        value: String,
        manifest: Option<&AppManifest>,
    ) {
        let values = if let Ok(mut map) = self.custom_controls.lock() {
            let app_map = map.entry(app_id.to_string()).or_default();
            app_map.insert(control_id.to_string(), value.clone());
            app_map.clone()
        } else {
            return;
        };

        let computed = Self::compute_params(manifest, &values);
        if let Ok(mut params_map) = self.custom_params.lock() {
            let entry = params_map
                .entry(app_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(computed)));
            if let Ok(mut guard) = entry.lock() {
                let preserved_phase = guard[3];
                *guard = computed;
                if computed[3] == 0.0 {
                    guard[3] = preserved_phase;
                }
            }
        }

        // If the manifest also has a stat or slider display correlated with this control, update custom_stats
        if let Some(m) = manifest {
            for ctrl in &m.controls {
                if let AppControl::Slider { id, unit, .. } = ctrl
                    && id == control_id
                {
                    let formatted = match unit {
                        Some(u) => format!("{value}{u}"),
                        None => value.clone(),
                    };
                    self.set_stat(app_id, control_id, formatted);
                } else if let AppControl::Select { id, .. } = ctrl
                    && id == control_id
                {
                    self.set_stat(app_id, control_id, value.clone());
                }
            }
        }
    }

    /// Update a single live telemetry stat for `app_id`.
    pub fn set_stat(&self, app_id: &str, stat_id: &str, val: impl Into<String>) {
        if let Ok(mut map) = self.custom_stats.lock() {
            map.entry(app_id.to_string())
                .or_default()
                .insert(stat_id.to_string(), val.into());
        }
    }

    /// Merge multiple live telemetry stats for `app_id`.
    pub fn set_stats(&self, app_id: &str, stats: BTreeMap<String, String>) {
        if let Ok(mut map) = self.custom_stats.lock() {
            let entry = map.entry(app_id.to_string()).or_default();
            for (k, v) in stats {
                entry.insert(k, v);
            }
        }
    }

    /// Snapshot all custom control values across apps.
    pub fn all_controls(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.custom_controls
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    /// Snapshot all custom telemetry stats across apps.
    pub fn all_stats(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.custom_stats
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }
}

/// Registry and manifest catalog for Rookery Apps.
pub struct AppCatalog;

impl AppCatalog {
    /// Return declarative manifests for the built-in flagship catalog.
    pub fn built_in_manifests() -> Vec<AppManifest> {
        vec![
            AppManifest {
                metadata: AppMetadata {
                    id: "living-rookery".into(),
                    name: "The Living ASCII Rookery".into(),
                    description: "An algorithmic layered ASCII tree that grows continuously over days, inhabited by procedural birds.".into(),
                    author: "Rookery Team".into(),
                    version: "1.0.0".into(),
                    archetype: AppArchetype::CozyEcosystem,
                    tags: vec!["ascii".into(), "tree".into(), "birds".into(), "forest".into(), "nature".into()],
                },
                theme: AppTheme {
                    tagline: Some("Woodland Sanctuary · ASCII Realm".into()),
                    banner_title: Some("Sylvan Sanctuary Chronicle".into()),
                    icon: Some("tree-evergreen".into()),
                    ornament: Some("leaf".into()),
                    bg: Some("#0c1912".into()),
                    card: Some("rgba(19, 38, 27, 0.92)".into()),
                    border_outer: Some("#366847".into()),
                    accent: Some("#34d399".into()),
                    accent_secondary: Some("#eab308".into()),
                    crest_gradient: Some(["#15803d".into(), "#ca8a04".into()]),
                },
                controls: vec![
                    AppControl::Stat {
                        id: "age".into(),
                        label: "Tree Age".into(),
                        default_val: "0h 00m".into(),
                        icon: Some("hourglass".into()),
                    },
                    AppControl::Stat {
                        id: "branches".into(),
                        label: "Boughs".into(),
                        default_val: "0".into(),
                        icon: Some("branch".into()),
                    },
                    AppControl::Stat {
                        id: "foliage".into(),
                        label: "Canopy Buds".into(),
                        default_val: "0".into(),
                        icon: Some("leaf".into()),
                    },
                    AppControl::Stat {
                        id: "birds".into(),
                        label: "Sky Flock".into(),
                        default_val: "4 birds".into(),
                        icon: Some("bird".into()),
                    },
                    AppControl::Button {
                        id: "feed".into(),
                        label: "Feed Birds".into(),
                        icon: Some("grains".into()),
                        tooltip: Some("Scatter snacks on branches to entice birds".into()),
                    },
                    AppControl::Button {
                        id: "prune".into(),
                        label: "Prune".into(),
                        icon: Some("scissors".into()),
                        tooltip: Some("Gently prune outer branches to shape the canopy".into()),
                    },
                    AppControl::Button {
                        id: "plant".into(),
                        label: "Plant Sapling".into(),
                        icon: Some("plant".into()),
                        tooltip: Some("Drop a seed to sprout a new sapling".into()),
                    },
                    AppControl::Button {
                        id: "pan_left".into(),
                        label: "Pan Left".into(),
                        icon: Some("caret-left".into()),
                        tooltip: Some("Explore the forest to the left".into()),
                    },
                    AppControl::Button {
                        id: "pan_right".into(),
                        label: "Pan Right".into(),
                        icon: Some("caret-right".into()),
                        tooltip: Some("Explore the forest to the right".into()),
                    },
                    AppControl::Select {
                        id: "color_palette".into(),
                        label: "Color Palette".into(),
                        options: rookery_core::world::ROOKERY_PALETTE_NAMES
                            .iter()
                            .enumerate()
                            .map(|(i, n)| format!("{i}: {n}"))
                            .collect(),
                        default_val: "0: Earth & Sylvan (Caves of Qud)".into(),
                    },
                    AppControl::Button {
                        id: "cycle_palette".into(),
                        label: "Next Palette".into(),
                        icon: Some("palette".into()),
                        tooltip: Some("Cycle through the 6 curated ANSI roguelike color palettes".into()),
                    },
                ],
                budgets: AppBudgets {
                    target_fps: 30,
                    max_cpu_ms: 2.0,
                    max_rss_mb: 120.0,
                },
            },
            AppManifest {
                metadata: AppMetadata {
                    id: "koi-pond".into(),
                    name: "Tranquil Koi Pond".into(),
                    description: "Serene water caustics with undulating articulated koi fish, blooming lotus pads, and feeding ripples.".into(),
                    author: "Rookery Team".into(),
                    version: "1.0.0".into(),
                    archetype: AppArchetype::CozyEcosystem,
                    tags: vec!["aquatic".into(), "koi".into(), "zen".into(), "lotus".into(), "water".into()],
                },
                theme: AppTheme {
                    tagline: Some("Imperial Lotus Sanctuary".into()),
                    banner_title: Some("Imperial Pond Scroll".into()),
                    icon: Some("koi".into()),
                    ornament: Some("flower-lotus".into()),
                    bg: Some("#071b24".into()),
                    card: Some("rgba(12, 39, 51, 0.92)".into()),
                    border_outer: Some("#1e6076".into()),
                    accent: Some("#fb7185".into()),
                    accent_secondary: Some("#2dd4bf".into()),
                    crest_gradient: Some(["#e11d48".into(), "#f97316".into()]),
                },
                controls: vec![
                    AppControl::Stat {
                        id: "koi_count".into(),
                        label: "Koi School".into(),
                        default_val: "12".into(),
                        icon: Some("koi".into()),
                    },
                    AppControl::Stat {
                        id: "lilies".into(),
                        label: "Lily Pads".into(),
                        default_val: "9".into(),
                        icon: Some("lily-pad".into()),
                    },
                    AppControl::Stat {
                        id: "lotuses".into(),
                        label: "Lotus Blooms".into(),
                        default_val: "3".into(),
                        icon: Some("flower-lotus".into()),
                    },
                    AppControl::Stat {
                        id: "ripples".into(),
                        label: "Water Ripples".into(),
                        default_val: "0".into(),
                        icon: Some("ripple".into()),
                    },
                    AppControl::Button {
                        id: "feed_koi".into(),
                        label: "Feed Koi".into(),
                        icon: Some("pellets".into()),
                        tooltip: Some("Scatter floating food pellets for the koi fish".into()),
                    },
                    AppControl::Button {
                        id: "tap_water".into(),
                        label: "Tap Surface".into(),
                        icon: Some("ripple".into()),
                        tooltip: Some("Create gentle water ripples across the pond".into()),
                    },
                ],
                budgets: AppBudgets {
                    target_fps: 30,
                    max_cpu_ms: 1.5,
                    max_rss_mb: 120.0,
                },
            },
            AppManifest {
                metadata: AppMetadata {
                    id: "cosmic-nebula".into(),
                    name: "Cosmic Nebula".into(),
                    description: "Procedural deep space gas cloud with celestial particle boids swirling through gravitational vortexes.".into(),
                    author: "Rookery Team".into(),
                    version: "1.0.0".into(),
                    archetype: AppArchetype::GenerativePhysics,
                    tags: vec!["space".into(), "nebula".into(), "stars".into(), "gravity".into(), "shader".into()],
                },
                theme: AppTheme {
                    tagline: Some("Astral Singularity Forge".into()),
                    banner_title: Some("Astral Star-Chart Deck".into()),
                    icon: Some("nebula".into()),
                    ornament: Some("star-four".into()),
                    bg: Some("#0d071e".into()),
                    card: Some("rgba(24, 13, 52, 0.92)".into()),
                    border_outer: Some("#6d28d9".into()),
                    accent: Some("#c084fc".into()),
                    accent_secondary: Some("#38bdf8".into()),
                    crest_gradient: Some(["#7c3aed".into(), "#ec4899".into()]),
                },
                controls: vec![
                    AppControl::Stat {
                        id: "particles".into(),
                        label: "Stellar Dust".into(),
                        default_val: "1500".into(),
                        icon: Some("sparkle".into()),
                    },
                    AppControl::Stat {
                        id: "wells".into(),
                        label: "Singularities".into(),
                        default_val: "2".into(),
                        icon: Some("spiral".into()),
                    },
                    AppControl::Stat {
                        id: "palette".into(),
                        label: "Spectrum".into(),
                        default_val: "Pillars of Creation".into(),
                        icon: Some("palette".into()),
                    },
                    AppControl::Stat {
                        id: "turbulence".into(),
                        label: "Flux Index".into(),
                        default_val: "1.0x".into(),
                        icon: Some("lightning".into()),
                    },
                    AppControl::Button {
                        id: "add_attractor".into(),
                        label: "Gravity Well".into(),
                        icon: Some("spiral".into()),
                        tooltip: Some("Create an attractive gravitational vortex".into()),
                    },
                    AppControl::Button {
                        id: "add_repulsor".into(),
                        label: "Repel Cloud".into(),
                        icon: Some("nova".into()),
                        tooltip: Some("Create an explosive gravitational repulsor".into()),
                    },
                    AppControl::Button {
                        id: "cycle_palette".into(),
                        label: "Cycle Colors".into(),
                        icon: Some("palette".into()),
                        tooltip: Some("Switch between stellar gold, amethyst, solar, and aurora palettes".into()),
                    },
                ],
                budgets: AppBudgets {
                    target_fps: 30,
                    max_cpu_ms: 2.0,
                    max_rss_mb: 120.0,
                },
            },
            AppManifest {
                metadata: AppMetadata {
                    id: "orbital-haven".into(),
                    name: "Orbital Haven".into(),
                    description: "A deep-space megastructure citadel assembling counter-rotating rings, solar wings, and biodomes with autonomous robotics and a 6-class spacecraft fleet.".into(),
                    author: "Rookery Team".into(),
                    version: "2.0.0".into(),
                    archetype: AppArchetype::SlowBurnIdle,
                    tags: vec!["scifi".into(), "station".into(), "orbit".into(), "idle".into(), "space".into(), "robots".into(), "ships".into()],
                },
                theme: AppTheme {
                    tagline: Some("Aegis-VII Orbital Citadel".into()),
                    banner_title: Some("Aegis-VII Command & Telemetry".into()),
                    icon: Some("satellite".into()),
                    ornament: Some("hexagon".into()),
                    bg: Some("#08121c".into()),
                    card: Some("rgba(14, 30, 46, 0.94)".into()),
                    border_outer: Some("#0284c7".into()),
                    accent: Some("#38bdf8".into()),
                    accent_secondary: Some("#f59e0b".into()),
                    crest_gradient: Some(["#0284c7".into(), "#f59e0b".into()]),
                },
                controls: vec![
                    AppControl::Stat {
                        id: "modules".into(),
                        label: "Megastructure".into(),
                        default_val: "13/66 · Core".into(),
                        icon: Some("hexagon".into()),
                    },
                    AppControl::Stat {
                        id: "solar".into(),
                        label: "Grid & Crew".into(),
                        default_val: "+195 MW · 92".into(),
                        icon: Some("solar-panel".into()),
                    },
                    AppControl::Stat {
                        id: "rings".into(),
                        label: "Directive".into(),
                        default_val: "Auto (35%)".into(),
                        icon: Some("hab-ring".into()),
                    },
                    AppControl::Stat {
                        id: "shuttles".into(),
                        label: "Fleet & Bots".into(),
                        default_val: "3 Ships · 8 Bots".into(),
                        icon: Some("rocket".into()),
                    },
                    AppControl::Button {
                        id: "overdrive_drones".into(),
                        label: "Overdrive Drones".into(),
                        icon: Some("lightning".into()),
                        tooltip: Some("Boost autonomous welder drones and accelerate active module construction".into()),
                    },
                    AppControl::Button {
                        id: "launch_shuttle".into(),
                        label: "Launch Vessel".into(),
                        icon: Some("rocket-launch".into()),
                        tooltip: Some("Dispatch a freighter, miner, liner, corvette, or interceptor".into()),
                    },
                    AppControl::Button {
                        id: "cycle_focus".into(),
                        label: "Cycle Directive".into(),
                        icon: Some("hab-ring".into()),
                        tooltip: Some("Switch construction priority across Auto, Solar, Rings, Biodomes, and Comms".into()),
                    },
                    AppControl::Button {
                        id: "cycle_planet".into(),
                        label: "Cycle Planet".into(),
                        icon: Some("compass".into()),
                        tooltip: Some("Switch planetary horizon between Terran, Europa, Ares, and Ecumenopolis".into()),
                    },
                    AppControl::Button {
                        id: "scan_pulse".into(),
                        label: "Subspace Scan".into(),
                        icon: Some("satellite".into()),
                        tooltip: Some("Emit a radial subspace sensor pulse and toggle diegetic HUD telemetry".into()),
                    },
                ],
                budgets: AppBudgets {
                    target_fps: 30,
                    max_cpu_ms: 1.5,
                    max_rss_mb: 120.0,
                },
            },
        ]
    }

    /// Discover external custom apps from a directory containing `rookery.toml` files.
    pub fn discover_external_apps(dir: &Path) -> Vec<AppManifest> {
        let mut manifests = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                let toml_path = path.join("rookery.toml");
                if toml_path.is_file()
                    && let Ok(m) = AppManifest::load_from_toml_file(&toml_path)
                {
                    manifests.push(m);
                }
            }
        }
        manifests
    }

    /// Return all manifests: built-in plus any discovered external apps.
    pub fn all_manifests(external_dir: Option<&Path>) -> Vec<AppManifest> {
        let mut all = Self::built_in_manifests();
        if let Some(dir) = external_dir {
            for ext in Self::discover_external_apps(dir) {
                if !all.iter().any(|m| m.metadata.id == ext.metadata.id) {
                    all.push(ext);
                }
            }
        }
        all
    }

    /// Find a single manifest by `app_id`.
    pub fn find_manifest(app_id: &str, external_dir: Option<&Path>) -> Option<AppManifest> {
        Self::all_manifests(external_dir)
            .into_iter()
            .find(|m| m.metadata.id == app_id)
    }

    /// Construct a [`SceneFactory`] for the given app id, supporting built-in and external WGSL shader apps.
    pub fn create_scene_factory_with_dir(
        app_id: &str,
        state: CatalogState,
        external_dir: Option<&Path>,
    ) -> SceneFactory {
        let id_string = app_id.to_string();
        let external_path = external_dir.map(|p| p.join(&id_string));
        let manifest = Self::find_manifest(app_id, external_dir);

        Box::new(move |gpu, format| match id_string.as_str() {
            "koi-pond" => Box::new(
                KoiPondScene::new(gpu, format, state.koi.clone()).with_audio(state.audio.clone()),
            ),
            "cosmic-nebula" => Box::new(
                CosmicNebulaScene::new(gpu, format, state.nebula.clone())
                    .with_audio(state.audio.clone()),
            ),
            "orbital-haven" => Box::new(
                OrbitalHavenScene::new(gpu, format, state.orbital.clone())
                    .with_audio(state.audio.clone()),
            ),
            "living-rookery" => Box::new(
                LivingRookeryScene::new(gpu, format, state.rookery.clone())
                    .with_audio(state.audio.clone()),
            ),
            _ => {
                if let Some(ref dir) = external_path {
                    let shader_path = dir.join("scene.wgsl");
                    if shader_path.is_file()
                        && let Ok(source) = std::fs::read_to_string(&shader_path)
                    {
                        let params = state.params_for_app(&id_string, manifest.as_ref());
                        if let Ok(custom) = crate::custom_shader::CustomShaderScene::new_with_params(
                            gpu, format, &source, params,
                        ) {
                            return Box::new(custom);
                        }
                    }
                    let html_path = dir.join("index.html");
                    if html_path.is_file() {
                        return Box::new(crate::scene::BlankScene);
                    }
                }
                Box::new(
                    LivingRookeryScene::new(gpu, format, state.rookery.clone())
                        .with_audio(state.audio.clone()),
                )
            }
        })
    }

    /// Construct a [`SceneFactory`] for the given app id.
    pub fn create_scene_factory(app_id: &str, state: CatalogState) -> SceneFactory {
        Self::create_scene_factory_with_dir(app_id, state, None)
    }

    /// Return the sRGB border/theme color matching the popover menu for `app_id`, reading
    /// from its `rookery.toml` `[theme]` section if available.
    pub fn theme_color_with_dir(app_id: &str, external_dir: Option<&Path>) -> [f32; 3] {
        if let Some(m) = Self::find_manifest(app_id, external_dir)
            && let Some(rgb) = m.theme.border_rgb_f32()
        {
            return rgb;
        }
        Self::theme_color(app_id)
    }

    /// Return the sRGB border/theme color matching the popover menu for `app_id`.
    pub fn theme_color(app_id: &str) -> [f32; 3] {
        match app_id {
            // #1e6076 (Tranquil Koi Pond --border-outer)
            "koi-pond" => [0.118, 0.376, 0.463],
            // #6d28d9 (Cosmic Nebula --border-outer)
            "cosmic-nebula" => [0.427, 0.157, 0.851],
            // #0284c7 (Orbital Haven --border-outer)
            "orbital-haven" => [0.008, 0.518, 0.780],
            // #0d9488 (Aurora Waves --border-outer)
            "aurora-waves" => [0.051, 0.580, 0.533],
            // #65a30d (Twilight Fireflies --border-outer)
            "fireflies" => [0.396, 0.639, 0.051],
            // #366847 (The Living ASCII Rookery --border-outer)
            _ => [0.212, 0.408, 0.278],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_external_apps_in_apps_dir() {
        let manifest_list = AppCatalog::discover_external_apps(Path::new("../../apps"));
        // Should find fireflies and aurora-waves
        let has_aurora = manifest_list
            .iter()
            .any(|m| m.metadata.id == "aurora-waves");
        let has_fireflies = manifest_list.iter().any(|m| m.metadata.id == "fireflies");
        assert!(has_aurora, "aurora-waves should be discovered");
        assert!(has_fireflies, "fireflies should be discovered");

        let aurora = manifest_list
            .into_iter()
            .find(|m| m.metadata.id == "aurora-waves")
            .unwrap();
        assert_eq!(aurora.metadata.archetype, AppArchetype::WebCanvas);
        assert!(aurora.controls.len() >= 3);
        assert!(aurora.theme.border_rgb_f32().is_some());
    }

    #[test]
    fn catalog_state_with_controls_populates_values() {
        let mut controls = BTreeMap::new();
        let mut aurora_controls = BTreeMap::new();
        aurora_controls.insert("wave_speed".to_string(), "2.5".to_string());
        aurora_controls.insert("color_speed".to_string(), "1.2".to_string());
        controls.insert("aurora-waves".to_string(), aurora_controls);

        let state = CatalogState::new(42).with_controls(controls);
        let all = state.all_controls();
        let aurora = all.get("aurora-waves").unwrap();
        assert_eq!(aurora.get("wave_speed").unwrap(), "2.5");
        assert_eq!(aurora.get("color_speed").unwrap(), "1.2");
    }
}
