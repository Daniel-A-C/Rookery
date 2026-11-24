//! State persistence, v2->v3 migration, and shareable URI encoding for *Orbital Haven v3*.

use std::fs;
use std::path::Path;

use base64::Engine as _;
use serde::Deserialize;

use super::OrbitalHavenState;
use super::types::ORBITAL_HAVEN_STATE_VERSION;

#[derive(Deserialize)]
struct LegacySeedProbe {
    seed: u64,
}

impl OrbitalHavenState {
    /// Ensure deserialized v3 state has valid drones, ships, and agents.
    pub fn ensure_migrated(&mut self) {
        if self.version < ORBITAL_HAVEN_STATE_VERSION
            || self.drones.is_empty()
            || self.ships.is_empty()
            || self.agents.len() < 14
        {
            *self = Self::new(self.seed);
        }
        self.recompute_telemetry();
    }

    /// Load state from disk or initialize a fresh v3 station with `default_seed`.
    /// Legacy v1/v2 `orbital_haven.json` files gracefully migrate while preserving `seed`.
    pub fn load_or_init(path: &Path, default_seed: u64) -> Self {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(mut state) = serde_json::from_slice::<Self>(&bytes) {
                state.ensure_migrated();
                return state;
            }
            if let Ok(legacy) = serde_json::from_slice::<LegacySeedProbe>(&bytes) {
                return Self::new(legacy.seed);
            }
        }
        Self::new(default_seed)
    }

    /// Atomically persist Orbital Haven state to disk.
    pub fn save_atomic(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Compact shareable seed URI (`rookery://orbital/seed-<hex>`).
    pub fn seed_uri(&self) -> String {
        format!("rookery://orbital/seed-{:016x}", self.seed)
    }

    /// Full base64-encoded state URI (`rookery://orbital/state-<base64>`).
    pub fn state_uri(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        Ok(format!("rookery://orbital/state-{encoded}"))
    }

    /// Parse a `rookery://orbital/...` seed or state string.
    pub fn from_share_string(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if let Some(rest) = trimmed.strip_prefix("rookery://orbital/state-") {
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(rest.trim())
                .map_err(|e| format!("Invalid base64 orbital state: {e}"))?;
            if let Ok(mut state) = serde_json::from_slice::<Self>(&bytes) {
                state.ensure_migrated();
                return Ok(state);
            }
            if let Ok(legacy) = serde_json::from_slice::<LegacySeedProbe>(&bytes) {
                return Ok(Self::new(legacy.seed));
            }
            return Err("Invalid orbital JSON payload".into());
        }

        let seed_hex = if let Some(rest) = trimmed.strip_prefix("rookery://orbital/seed-") {
            Some(rest.trim())
        } else {
            trimmed.strip_prefix("orbital-seed-").map(|s| s.trim())
        };

        if let Some(hex_str) = seed_hex {
            let seed = u64::from_str_radix(hex_str, 16)
                .map_err(|e| format!("Invalid hex seed '{hex_str}': {e}"))?;
            return Ok(Self::new(seed));
        }

        Err("Unrecognized orbital share string. Expected 'rookery://orbital/seed-...' or 'rookery://orbital/state-...'".into())
    }
}
