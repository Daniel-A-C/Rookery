//! First-Party App Catalog & Open App SDK (Milestone 4).
//!
//! Provides declarative manifests, runtime app creation, external manifest loading,
//! and shared simulation state ownership across the flagship catalog.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rookery_core::koi::KoiPondState;
use rookery_core::manifest::{AppArchetype, AppBudgets, AppControl, AppManifest, AppMetadata};
use rookery_core::nebula::CosmicNebulaState;
use rookery_core::orbital::OrbitalHavenState;
use rookery_core::world::WorldState;

use crate::SceneFactory;
use crate::ascii_scene::LivingRookeryScene;
use crate::koi_scene::KoiPondScene;
use crate::nebula_scene::CosmicNebulaScene;
use crate::orbital_scene::OrbitalHavenScene;

/// Shared container holding simulation state for all installed catalog apps.
#[derive(Clone)]
pub struct CatalogState {
    pub rookery: Arc<Mutex<WorldState>>,
    pub koi: Arc<Mutex<KoiPondState>>,
    pub nebula: Arc<Mutex<CosmicNebulaState>>,
    pub orbital: Arc<Mutex<OrbitalHavenState>>,
}

impl CatalogState {
    pub fn new(seed: u64) -> Self {
        Self {
            rookery: Arc::new(Mutex::new(WorldState::new(seed))),
            koi: Arc::new(Mutex::new(KoiPondState::new(seed))),
            nebula: Arc::new(Mutex::new(CosmicNebulaState::new(seed))),
            orbital: Arc::new(Mutex::new(OrbitalHavenState::new(seed))),
        }
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
                },
                controls: vec![
                    AppControl::Button {
                        id: "feed".into(),
                        label: "Feed Birds".into(),
                        icon: Some("🌾".into()),
                        tooltip: Some("Scatter snacks on branches to entice birds".into()),
                    },
                    AppControl::Button {
                        id: "prune".into(),
                        label: "Prune".into(),
                        icon: Some("✂️".into()),
                        tooltip: Some("Gently prune outer branches to shape the canopy".into()),
                    },
                    AppControl::Button {
                        id: "plant".into(),
                        label: "Plant Sapling".into(),
                        icon: Some("🌱".into()),
                        tooltip: Some("Drop a seed to sprout a new sapling".into()),
                    },
                    AppControl::Button {
                        id: "pan_left".into(),
                        label: "Pan Left".into(),
                        icon: Some("◀".into()),
                        tooltip: Some("Explore the forest to the left".into()),
                    },
                    AppControl::Button {
                        id: "pan_right".into(),
                        label: "Pan Right".into(),
                        icon: Some("▶".into()),
                        tooltip: Some("Explore the forest to the right".into()),
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
                },
                controls: vec![
                    AppControl::Button {
                        id: "feed_koi".into(),
                        label: "Feed Koi".into(),
                        icon: Some("🦐".into()),
                        tooltip: Some("Scatter floating food pellets for the koi fish".into()),
                    },
                    AppControl::Button {
                        id: "tap_water".into(),
                        label: "Tap Surface".into(),
                        icon: Some("💧".into()),
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
                },
                controls: vec![
                    AppControl::Button {
                        id: "add_attractor".into(),
                        label: "Gravity Well".into(),
                        icon: Some("🌀".into()),
                        tooltip: Some("Create an attractive gravitational vortex".into()),
                    },
                    AppControl::Button {
                        id: "add_repulsor".into(),
                        label: "Repel Cloud".into(),
                        icon: Some("💫".into()),
                        tooltip: Some("Create an explosive gravitational repulsor".into()),
                    },
                    AppControl::Button {
                        id: "cycle_palette".into(),
                        label: "Cycle Colors".into(),
                        icon: Some("🎨".into()),
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
                    description: "A slow-burn idle space station assembling modules and habitat rings over time, serviced by cargo shuttles.".into(),
                    author: "Rookery Team".into(),
                    version: "1.0.0".into(),
                    archetype: AppArchetype::SlowBurnIdle,
                },
                controls: vec![
                    AppControl::Button {
                        id: "focus_solar".into(),
                        label: "Build Solar".into(),
                        icon: Some("☀️".into()),
                        tooltip: Some("Prioritize assembling solar collector wings".into()),
                    },
                    AppControl::Button {
                        id: "focus_habitat".into(),
                        label: "Build Habitat".into(),
                        icon: Some("🛸".into()),
                        tooltip: Some("Prioritize assembling rotating centrifugal habitat rings".into()),
                    },
                    AppControl::Button {
                        id: "launch_shuttle".into(),
                        label: "Launch Shuttle".into(),
                        icon: Some("🚀".into()),
                        tooltip: Some("Deploy an auxiliary transport shuttle to the station".into()),
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
            for entry in entries.flatten() {
                let toml_path = entry.path().join("rookery.toml");
                if toml_path.is_file()
                    && let Ok(m) = AppManifest::load_from_toml_file(&toml_path)
                {
                    manifests.push(m);
                }
            }
        }
        manifests
    }

    /// Construct a [`SceneFactory`] for the given app id.
    pub fn create_scene_factory(app_id: &str, state: CatalogState) -> SceneFactory {
        let id_string = app_id.to_string();
        Box::new(move |gpu, format| match id_string.as_str() {
            "koi-pond" => Box::new(KoiPondScene::new(gpu, format, state.koi.clone())),
            "cosmic-nebula" => Box::new(CosmicNebulaScene::new(gpu, format, state.nebula.clone())),
            "orbital-haven" => Box::new(OrbitalHavenScene::new(gpu, format, state.orbital.clone())),
            _ => Box::new(LivingRookeryScene::new(gpu, format, state.rookery.clone())),
        })
    }
}
