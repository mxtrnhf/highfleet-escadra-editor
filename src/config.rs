//! The `escadra.json` schema: one file, every pool.
//!
//! Every group config runs the same four fields, applied in this order:
//! `replace` (the pool's contents become exactly this list), `remove` (drop
//! matching names), then `add` (append names). `extras_*` apply to the same
//! pool's post-roll special ships (the `'Longbow'` / `'Eagle'` / `'Tarantul
//! MRL'` chance branches), not the main candidate list.
//!
//! Built names are resolved through the game's design/property library at
//! spawn time, so a name must be a design that exists there (vanilla designs
//! or a mod design registered in `Data/Ships`). Unknown names resolve to
//! zero strength; their downstream behavior has not been verified.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Master switch. When false the mod loads, logs, and changes nothing.
    pub enabled: bool,
    /// Strike groups: the roaming big fleets.
    pub strikegroups: PoolConfig,
    /// Tactical groups, air specialization. Shares one base pool with
    /// `tactical_missile` (the game draws both specializations from the same
    /// candidate list; adds merge, removes union).
    pub tactical_air: PoolConfig,
    /// Tactical groups, missile specialization. The `Longbow` / `'Tarantul
    /// MRL'` extras land here.
    pub tactical_missile: PoolConfig,
    /// City garrisons. Two code paths, both editable:
    /// `campaign` (fleet present from campaign start) and `endgame` (the
    /// garrison ships spawned by the endgame collapse).
    pub garrisons: GarrisonsConfig,
    /// General enemies: the price-bucketed 46-name catalogue behind the
    /// scenario / escalating response fleets.
    pub general_enemies: PoolConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            strikegroups: PoolConfig::default(),
            tactical_air: PoolConfig::default(),
            tactical_missile: PoolConfig::default(),
            garrisons: GarrisonsConfig::default(),
            general_enemies: PoolConfig::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct GarrisonsConfig {
    /// Campaign garrisons and scripted event escadras
    /// (`somethingSpawnShips` pool 0).
    pub campaign: PoolConfig,
    /// Endgame city garrisons (`somethingSpawnShips` pool 4).
    pub endgame: PoolConfig,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct PoolConfig {
    /// Replace the whole pool with exactly these names.
    pub replace: Option<Vec<String>>,
    /// Remove these names from the pool.
    pub remove: Vec<String>,
    /// Append these names to the pool.
    pub add: Vec<String>,
    /// Replace the pool's post-roll special ships with exactly these names.
    pub extras_replace: Option<Vec<String>>,
    /// Append names to the pool's post-roll special ships.
    pub extras_add: Vec<String>,
    /// Remove names from the pool's post-roll special ships.
    pub extras_remove: Vec<String>,
}

impl PoolConfig {
    /// True when the config would leave the pool exactly as vanilla.
    pub fn is_empty(&self) -> bool {
        self.replace.is_none()
            && self.add.is_empty()
            && self.remove.is_empty()
            && self.extras_replace.is_none()
            && self.extras_add.is_empty()
            && self.extras_remove.is_empty()
    }

    /// One line for the log: what this config does to its pool.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "vanilla".to_string();
        }
        let mut parts: Vec<String> = Vec::new();
        if let Some(names) = &self.replace {
            parts.push(format!("replace:{}", names.len()));
        }
        if !self.remove.is_empty() {
            parts.push(format!("remove:{}", self.remove.len()));
        }
        if !self.add.is_empty() {
            parts.push(format!("add:{}", self.add.len()));
        }
        if let Some(names) = &self.extras_replace {
            parts.push(format!("extras replace:{}", names.len()));
        }
        if !self.extras_remove.is_empty() {
            parts.push(format!("extras remove:{}", self.extras_remove.len()));
        }
        if !self.extras_add.is_empty() {
            parts.push(format!("extras add:{}", self.extras_add.len()));
        }
        parts.join(", ")
    }
}

/// Read `escadra.json`, or write it with defaults when missing or broken.
/// Missing keys take defaults (serde `#[serde(default)]`, unknown keys are
/// ignored), so a hand-edited file with only the categories being changed is
/// a valid file.
pub fn load_or_create(path: &str) -> Config {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
            log::error!("hf_hee: bad {path}: {error}; using defaults");
            Config::default()
        }),
        Err(_) => {
            let config = Config::default();
            match serde_json::to_string_pretty(&config) {
                Ok(text) => {
                    if let Err(error) = std::fs::write(path, text) {
                        log::error!("hf_hee: cannot write {path}: {error}");
                    }
                }
                Err(error) => log::error!("hf_hee: cannot serialize defaults: {error}"),
            }
            config
        }
    }
}
