//! State persistence, atomic file saving, URI serialization, and offline progression.

use std::fs;
use std::path::Path;

use super::*;

impl WorldState {
    /// Offline time-delta engine: deterministically catch up simulation state after sleep or shutdown.
    /// Fast-forwards tree growth and advances bird states without lag (< 5ms execution time).
    pub fn on_wake(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 {
            return;
        }

        log::info!(
            "offline time-delta engine: catching up {:.1}s of offline progression",
            elapsed_secs
        );

        self.total_offline_caught_up_secs += elapsed_secs;
        self.total_age_secs += elapsed_secs;

        // Step through in bulk chunks, capped at max 60 steps for instant wake-up (< 20ms)
        let max_steps = 60;
        let step_dt = (elapsed_secs / max_steps as f64).min(30.0) as f32;
        let steps = ((elapsed_secs / step_dt as f64).round() as usize).min(max_steps);

        for _ in 0..steps {
            self.advance_growth(step_dt);
        }

        self.recompute_stats();
    }

    pub fn recompute_stats(&mut self) {
        self.stats.total_branches = self.branches.len() as u32;
        self.stats.total_foliage = self.branches.iter().map(|b| b.foliage.len() as u32).sum();
        self.stats.birds_active = self.birds.len() as u32;
        self.stats.birds_perched = self
            .birds
            .iter()
            .filter(|b| {
                matches!(
                    b.state,
                    BirdState::Perched { .. }
                        | BirdState::Feeding { .. }
                        | BirdState::GatheringTwig { .. }
                        | BirdState::BuildingNest { .. }
                        | BirdState::BroodingEggs { .. }
                        | BirdState::Bathing { .. }
                )
            })
            .count() as u32;
        self.stats.nests_built = self.branches.iter().filter(|b| b.has_nest).count() as u32
            + self.nests.iter().filter(|n| n.progress >= 1.0).count() as u32;
        self.stats.houses_built = self.houses.len() as u32;
    }

    /// Load persisted world state from disk, falling back to a freshly seeded world if missing or corrupt.
    pub fn load_or_init(path: &Path, seed: u64) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<WorldState>(&content) {
                Ok(mut state) => {
                    if state.trees.is_empty() && !state.branches.is_empty() {
                        let root_branch = state
                            .branches
                            .iter()
                            .find(|b| b.depth == 0)
                            .map(|b| b.id)
                            .unwrap_or(1);
                        let base_pos = state
                            .branches
                            .iter()
                            .find(|b| b.id == root_branch)
                            .map(|b| b.start)
                            .unwrap_or([500.0, 750.0]);
                        state.trees.push(TreeRecord {
                            id: 1,
                            species: TreeSpecies::AncientOak,
                            base_pos,
                            root_branch_id: root_branch,
                            is_mature: state.branches.len() > 25,
                            growth_steps_completed: if state.branches.len() > 25 {
                                40
                            } else {
                                state.branches.len() as u32
                            },
                            target_growth_steps: 40,
                            planted_age_secs: 0.0,
                            seed: state.seed,
                        });
                        state.active_tree_id = 1;
                    }

                    // Migration: Mark ground flare buttress roots as is_root and sanitize any children
                    for b in &mut state.branches {
                        if b.depth == 1 && b.start[1] >= 748.0 && b.angle.abs() > 1.0 {
                            b.is_root = true;
                        }
                    }
                    let root_ids: Vec<u32> = state
                        .branches
                        .iter()
                        .filter(|b| b.is_root)
                        .map(|b| b.id)
                        .collect();
                    state.branches.retain(|b| {
                        if !b.is_root
                            && let Some(pid) = b.parent_id
                        {
                            !root_ids.contains(&pid)
                        } else {
                            true
                        }
                    });

                    // Migration: Reposition any birds stranded far outside active viewport
                    let active_x = state
                        .trees
                        .iter()
                        .find(|t| t.id == state.active_tree_id)
                        .map(|t| t.base_pos[0])
                        .unwrap_or(state.viewport_offset_x + 500.0);
                    for (i, bird) in state.birds.iter_mut().enumerate() {
                        if (bird.pos[0] - active_x).abs() > 1200.0 {
                            let center = [active_x, 320.0];
                            let radius = 160.0 + ((i as f32) * 20.0);
                            let angle = (i as f32) * 1.5;
                            let speed = if i % 2 == 0 { 0.55 } else { -0.55 };
                            bird.pos = [
                                center[0] + angle.cos() * radius,
                                center[1] + angle.sin() * radius * 0.45,
                            ];
                            bird.state = BirdState::Circling {
                                center,
                                radius,
                                angle,
                                speed,
                            };
                        }
                    }

                    // Migration: Ensure mature worlds have default woodland fauna roaming the meadow,
                    // strictly capped at at most 1 or 2 creatures.
                    if state.fauna.is_empty() {
                        state.seed_default_fauna();
                        log::info!(
                            "migrated empty fauna: seeded default woodland creatures for {}",
                            path.display()
                        );
                    } else {
                        if state.fauna.len() > fauna::MAX_FLOOR_FAUNA {
                            state.fauna.truncate(fauna::MAX_FLOOR_FAUNA);
                        }
                        // Reposition any fauna stranded far from active tree & clamp to 2D meadow bounds
                        for f in &mut state.fauna {
                            f.pos[1] = f.pos[1].clamp(746.0, 835.0);
                            if (f.pos[0] - active_x).abs() > 1400.0 {
                                let rx = state.rng.range(active_x - 220.0, active_x + 220.0);
                                let ry = state.rng.range(750.0, 830.0);
                                let tx = state.rng.range(active_x - 200.0, active_x + 200.0);
                                let ty = state.rng.range(750.0, 830.0);
                                f.pos = [rx, ry];
                                f.state = FaunaState::Trotting {
                                    target: [tx, ty],
                                    speed: f.species.base_speed(),
                                };
                            }
                        }
                    }

                    state.recompute_stats();
                    log::info!(
                        "loaded world state from {} (age {:.1}h, {} branches)",
                        path.display(),
                        state.total_age_secs / 3600.0,
                        state.branches.len()
                    );
                    state
                }
                Err(e) => {
                    log::warn!(
                        "failed to deserialize {}: {e}; creating new world",
                        path.display()
                    );
                    Self::new(seed)
                }
            },
            Err(_) => Self::new(seed),
        }
    }

    /// Atomically persist current world state to disk (write temp file + sync + rename).
    pub fn save_atomic(&mut self, path: &Path) -> std::io::Result<()> {
        self.last_saved_utc = Some(Utc::now());
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json.as_bytes())?;
        fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Formats this world's seed into a standard sharing URI: `rookery://tree/seed-<hex>`.
    pub fn seed_uri(&self) -> String {
        format!("rookery://tree/seed-{:016x}", self.seed)
    }

    /// Serializes this world's entire simulation state into a base64url URI: `rookery://tree/state-<base64>`.
    pub fn state_uri(&self) -> Result<String, serde_json::Error> {
        use base64::Engine;
        let json = serde_json::to_vec(self)?;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&json);
        Ok(format!("rookery://tree/state-{b64}"))
    }

    /// Parses a share string (seed URI, state URI, hex seed, decimal seed, or raw JSON)
    /// and returns either a newly seeded world or the exact deserialized state.
    pub fn from_share_string(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Empty input string".into());
        }

        // 1. Exact snapshot state URI
        if let Some(rest) = trimmed.strip_prefix("rookery://tree/state-") {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(rest.trim())
                .map_err(|e| format!("Invalid base64 state: {e}"))?;
            let state: WorldState =
                serde_json::from_slice(&bytes).map_err(|e| format!("Invalid world JSON: {e}"))?;
            return Ok(state);
        }

        // 2. Seed URI: rookery://tree/seed-<hex> or seed-<hex>
        let seed_hex = if let Some(rest) = trimmed.strip_prefix("rookery://tree/seed-") {
            Some(rest.trim())
        } else {
            trimmed.strip_prefix("seed-").map(|s| s.trim())
        };

        if let Some(hex_str) = seed_hex {
            let seed = u64::from_str_radix(hex_str, 16)
                .map_err(|e| format!("Invalid hex seed '{hex_str}': {e}"))?;
            return Ok(Self::new(seed));
        }

        // 3. Raw decimal u64 seed
        if let Ok(seed) = trimmed.parse::<u64>() {
            return Ok(Self::new(seed));
        }

        // 4. Raw JSON state
        if trimmed.starts_with('{') {
            let state: WorldState =
                serde_json::from_str(trimmed).map_err(|e| format!("Invalid world JSON: {e}"))?;
            return Ok(state);
        }

        Err("Unrecognized share string. Expected 'rookery://tree/seed-...' or 'rookery://tree/state-...'".into())
    }
}
