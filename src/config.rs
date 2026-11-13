//! The `escadra.json` schema: eight flat top-level pools with optional nuanced sub-fields.
//!
//! Every pool uses the same base fields (`replace` -> `remove` -> `add`), applied in
//! deterministic order:
//! 1. `replace` (optional — when set, replaces the vanilla pool outright)
//! 2. `remove` (drops matching ship names)
//! 3. `add` (appends ship names)
//!
//! When an optional sub-pool field is omitted, the category's base `PoolConfig`
//! applies to all of its sub-tables (preserving the simple flat workflow). When an
//! optional sub-pool is explicitly specified, that sub-pool overrides the base edit
//! for that specific sub-table.

use serde::{Deserialize, Serialize};

const CURRENT_CONFIG_VERSION: u32 = 6;

/// `strikegroups.sprinkle` defaults to `true`: campaign Strike Groups pick a
/// random eligible city per group instead of stacking on one city.
fn default_sprinkle() -> bool {
    true
}

/// Optional group-count adjustment, parsed from a string in `count` fields.
///
/// Format:
/// - `"N"`   — set: exactly `N` groups are allowed to exist (vanilla count replaced)
/// - `"*N"`  — multiply the vanilla count by `N` (`N` must be an integer)
/// - `"-N"`  — subtract `N` from the vanilla count, clamped so the value can't go below `0`
/// - `"+N"`  — add `N` to the vanilla count
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountOp {
    Set(i64),
    Mul(i64),
    Sub(i64),
    Add(i64),
}

/// Largest count we ever bake into a 32-bit immediate (loop bounds / `MOV EDX, imm32`).
const MAX_COUNT: i64 = 1_000_000;

impl CountOp {
    /// Parses a `count` string. Returns `None` (and the caller logs) on anything
    /// that is not an optional sign-prefix (`*`, `-`, `+`) plus an integer.
    pub fn parse(spec: &str) -> Option<CountOp> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return None;
        }
        // `i64` parse accepts only integers: `"*2.5"` or `"*2x"` are rejected.
        match trimmed.as_bytes()[0] {
            b'*' => Some(CountOp::Mul(trimmed[1..].trim().parse().ok()?)),
            b'-' => Some(CountOp::Sub(trimmed[1..].trim().parse().ok()?)),
            b'+' => Some(CountOp::Add(trimmed[1..].trim().parse().ok()?)),
            _ => Some(CountOp::Set(trimmed.parse().ok()?)),
        }
    }

    /// Applies the op to an integer vanilla count (loop bounds), clamping the
    /// result to `[0, MAX_COUNT]`.
    pub fn apply_int(self, base: i64) -> i32 {
        let value = match self {
            CountOp::Set(n) => n,
            CountOp::Mul(n) => base.saturating_mul(n),
            CountOp::Sub(n) => base.saturating_sub(n),
            CountOp::Add(n) => base.saturating_add(n),
        };
        value.clamp(0, MAX_COUNT) as i32
    }

    /// Applies the op to a float vanilla value (tactical-group spawn chance),
    /// clamping the result so it can't go below `0.0`.
    pub fn apply_float(self, base: f32) -> f32 {
        let n = match self {
            CountOp::Set(n) | CountOp::Mul(n) | CountOp::Sub(n) | CountOp::Add(n) => n as f32,
        };
        let value = match self {
            CountOp::Set(_) => n,
            CountOp::Mul(_) => base * n,
            CountOp::Sub(_) => base - n,
            CountOp::Add(_) => base + n,
        };
        if value.is_nan() {
            0.0
        } else {
            value.max(0.0)
        }
    }

    /// Short human-readable form for startup logging, e.g. `"*2"`, `"+3"`, `"5"`.
    pub fn describe(self) -> String {
        match self {
            CountOp::Set(n) => format!("{n}"),
            CountOp::Mul(n) => format!("*{n}"),
            CountOp::Sub(n) => format!("-{n}"),
            CountOp::Add(n) => format!("+{n}"),
        }
    }
}

/// Parses an optional `count` string field, logging invalid values (which are
/// then ignored, leaving the vanilla count).
pub fn parse_count_field(category: &str, field: &Option<String>) -> Option<CountOp> {
    let spec = field.as_ref()?;
    match CountOp::parse(spec) {
        Some(op) => Some(op),
        None => {
            log::error!(
                "hf_hee: bad {category}.count = {spec:?}; expected \"N\", \"*N\", \"-N\" or \"+N\" with integer N"
            );
            None
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Schema version.
    pub version: u32,
    /// Master switch. When false the mod loads, logs, and changes nothing.
    pub enabled: bool,
    /// Roaming & Endgame Strike Groups (`Endgame_BuildEscadra` `0x2077c0`).
    pub strikegroups: StrikeGroupsConfig,
    /// Aircraft Tactical Groups (`somethingSpawnShips` `chanceA` `0x205925`).
    pub tactical_air: TacticalAirConfig,
    /// Missile Tactical Groups (`somethingSpawnShips` `chanceB` `0x20596d` and
    /// endgame phase-3 rider `0x262a19`).
    pub tactical_missile: TacticalMissileConfig,
    /// City Garrisons (`somethingSpawnShips` `0x203b80`, pool `0`).
    pub garrisons: GarrisonsConfig,
    /// Tarkhan personal ships (`0x28f695`), Khiva finale `ENDWAR_CARD` flagships
    /// (`0x238e70` groups `1..4`), and `tier0` Khiva escort fallback (`0x21b96f`).
    pub tarkhans: TarkhansConfig,
    /// Mercenaries & Reinforcements: Mercenary cities (`pool 2`), Endgame free
    /// cities (`pool 4`), and Tarkhan call-in reinforcements (`0x21b96f`).
    pub mercs: MercsConfig,
    /// Trade Convoys (`spawnTradeConvoy?` `0x208660`): prize trade ships and
    /// fuel-endurance convoy escorts.
    pub traders: TradersConfig,
    /// Shipworks `TEST` combat opponent selector (`Fleet_BuildShopOffer` `0x206950`).
    pub shipworks: ShipworksConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            enabled: true,
            strikegroups: StrikeGroupsConfig {
                sprinkle: default_sprinkle(),
                ..StrikeGroupsConfig::default()
            },
            tactical_air: TacticalAirConfig::default(),
            tactical_missile: TacticalMissileConfig::default(),
            garrisons: GarrisonsConfig::default(),
            tarkhans: TarkhansConfig::default(),
            mercs: MercsConfig::default(),
            traders: TradersConfig::default(),
            shipworks: ShipworksConfig::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct PoolConfig {
    /// Replace the whole pool with exactly these names (when `Some`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<Vec<String>>,
    /// Remove matching names from the pool.
    pub remove: Vec<String>,
    /// Append these names to the pool.
    pub add: Vec<String>,
}

impl PoolConfig {
    /// True when the config leaves this pool untouched (vanilla).
    pub fn is_empty(&self) -> bool {
        self.replace.is_none() && self.remove.is_empty() && self.add.is_empty()
    }

    /// Concise summary for startup logging.
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
        parts.join(", ")
    }
}

/// Aircraft Tactical Groups: flat pool edits plus the optional group-count field.
#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct TacticalAirConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Optional group-count adjustment for Air Tactical Groups. Applies to the
    /// per-city `chanceA` spawn probability (vanilla `0.15` normal / `0.20` hard,
    /// `0` near `"UR"` on normal), which is what controls how many air groups
    /// exist in total: `"0"` disables them, `"1"` gives every city one, `"*2"`
    /// doubles the vanilla chance, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<String>,
}

impl TacticalAirConfig {
    pub fn summary(&self) -> String {
        let mut s = self.base.summary();
        if let Some(count) = &self.count {
            if s != "vanilla" {
                s.push_str("; ");
            }
            s.push_str(&format!("count:{count}"));
        }
        s
    }
}

fn append_sub_summary(parts: &mut Vec<String>, name: &str, sub: &Option<PoolConfig>) {
    if let Some(cfg) = sub {
        if !cfg.is_empty() {
            parts.push(format!("{name}({})", cfg.summary()));
        } else {
            parts.push(format!("{name}(vanilla)"));
        }
    }
}

fn summarize_with_subs(base: &PoolConfig, subs: &[(&str, &Option<PoolConfig>)]) -> String {
    let any_sub = subs.iter().any(|(_, s)| s.is_some());
    if base.is_empty() && !any_sub {
        return "vanilla".to_string();
    }
    let mut parts = Vec::new();
    if !base.is_empty() {
        parts.push(base.summary());
    }
    for (name, sub) in subs {
        append_sub_summary(&mut parts, name, sub);
    }
    parts.join("; ")
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct StrikeGroupsConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Normal-mode close-to-Ur flagship pool (`[Borey]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagships_normal: Option<PoolConfig>,
    /// Hard-mode / far-from-Ur flagship pool (`[Nomad Mk2, Nomad, Gryphon]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagships_hard: Option<PoolConfig>,
    /// Normal-mode close-to-Ur budget fill pool (`[Kormoran, Negev, Borey]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_normal: Option<PoolConfig>,
    /// Hard-mode / far-from-Ur budget fill pool (`[Gryphon, Kormoran, Negev, Borey]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_hard: Option<PoolConfig>,
    /// Missile frigate/cruiser escort rolled on Strike Groups (`[Tarantul ARM]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missile_escorts: Option<PoolConfig>,
    /// Carrier escort rolled on Strike Groups (`[Wasp, Longbow]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier_escorts: Option<PoolConfig>,
    /// Superheavy flagship appended to Khiva assault Strike Groups (`[Typhon]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub khiva_extra: Option<PoolConfig>,
    /// Outer Nuclear War Strike Group phase-3 missile rider (`[Tarantul, Tarantul ARM]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endgame_rider: Option<PoolConfig>,
    /// Optional group-count adjustment for Strike Groups. Applied to every
    /// strike-group spawn wave's vanilla count: campaign roaming groups
    /// (vanilla `6`), Khiva assault wave (vanilla up to `4`), and outer
    /// Nuclear War wave (vanilla `3`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<String>,
    /// When `true` (the default), each campaign Strike Group picks a
    /// **uniformly random eligible city** as its spawn point instead of
    /// vanilla's first-eligible city in vector order (which stacks high
    /// `count` values onto one city). Set to `false` for vanilla behavior.
    #[serde(default = "default_sprinkle")]
    pub sprinkle: bool,
}

impl StrikeGroupsConfig {
    pub fn summary(&self) -> String {
        let mut s = summarize_with_subs(
            &self.base,
            &[
                ("flagships_normal", &self.flagships_normal),
                ("flagships_hard", &self.flagships_hard),
                ("fill_normal", &self.fill_normal),
                ("fill_hard", &self.fill_hard),
                ("missile_escorts", &self.missile_escorts),
                ("carrier_escorts", &self.carrier_escorts),
                ("khiva_extra", &self.khiva_extra),
                ("endgame_rider", &self.endgame_rider),
            ],
        );
        if let Some(count) = &self.count {
            if s != "vanilla" {
                s.push_str("; ");
            }
            s.push_str(&format!("count:{count}"));
        }
        if self.sprinkle {
            if s != "vanilla" {
                s.push_str("; ");
            }
            s.push_str("sprinkle");
        }
        s
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct TacticalMissileConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// City Missile Tactical Group cruisers (`[Tarantul, Eagle, Tarantul MRL]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ships: Option<PoolConfig>,
    /// City Missile Tactical Group escort (`[Hound]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escorts: Option<PoolConfig>,
    /// Strike Group missile frigate/cruiser escort (`[Tarantul ARM]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sg_missile: Option<PoolConfig>,
    /// Nuclear War outer Strike Group phase-3 rider (`[Tarantul, Tarantul ARM]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endgame_rider: Option<PoolConfig>,
    /// Optional group-count adjustment for Missile Tactical Groups. Applies to
    /// the per-city `chanceB` spawn probability (same vanilla scaling as
    /// `chanceA`), which controls how many missile groups exist in total.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<String>,
}

impl TacticalMissileConfig {
    pub fn summary(&self) -> String {
        let mut s = summarize_with_subs(
            &self.base,
            &[
                ("ships", &self.ships),
                ("escorts", &self.escorts),
                ("sg_missile", &self.sg_missile),
                ("endgame_rider", &self.endgame_rider),
            ],
        );
        if let Some(count) = &self.count {
            if s != "vanilla" {
                s.push_str("; ");
            }
            s.push_str(&format!("count:{count}"));
        }
        s
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct GarrisonsConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Usual campaign city garrisons (`Profile.endwar == 0`, pool `0`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<PoolConfig>,
    /// Late-game / Nuclear War (`Profile.endwar != 0`) garrison & city pool:
    /// controls `somethingSpawnShips` pool `0` if rolled after Khiva defense begins,
    /// and also acts as a fallback for `mercs.endgame` (pool `4` city pier ships in
    /// Nuclear War) unless `mercs.endgame` is explicitly set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endgame: Option<PoolConfig>,
    /// Optional auto-sync into Shipworks `TEST` combat (`Fleet_BuildShopOffer`).
    /// Defaults to `false` because top-level `"shipworks"` controls `TEST` directly.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub include_in_shipworks: bool,
}

impl GarrisonsConfig {
    pub fn summary(&self) -> String {
        let mut s = summarize_with_subs(
            &self.base,
            &[("campaign", &self.campaign), ("endgame", &self.endgame)],
        );
        if self.include_in_shipworks {
            s.push_str("; include_in_shipworks");
        }
        s
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct TarkhansConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Named Tarkhan personal ships in cities (`FOR_TARKHAN`, `0x28f695`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<PoolConfig>,
    /// Khiva defense tier 0 (`addReinforceingShips` tier `0` escort fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier0: Option<PoolConfig>,
    /// Khiva defense `ENDWAR_CARD` group 1 (1-star loyalty Tarkhans, 14 vanilla ships).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier1: Option<PoolConfig>,
    /// Khiva defense `ENDWAR_CARD` group 2 (2-star loyalty Tarkhans, 6 vanilla ships).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier2: Option<PoolConfig>,
    /// Khiva defense `ENDWAR_CARD` group 3 (3-star loyalty Tarkhans, 4 vanilla ships).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier3: Option<PoolConfig>,
    /// Khiva defense `ENDWAR_CARD` group 4 (max loyalty Tarkhans: `Vega, Nomad, Nomad Mk2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier4: Option<PoolConfig>,
}

impl TarkhansConfig {
    pub fn summary(&self) -> String {
        summarize_with_subs(
            &self.base,
            &[
                ("campaign", &self.campaign),
                ("tier0", &self.tier0),
                ("tier1", &self.tier1),
                ("tier2", &self.tier2),
                ("tier3", &self.tier3),
                ("tier4", &self.tier4),
            ],
        )
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MercsConfig {
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Paid `FOR_HIRE` pier ships in Mercenary cities (`somethingSpawnShips` pool `2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hire: Option<PoolConfig>,
    /// Free `FOR_JOIN` pier ships in non-Khiva cities after Nuclear War begins (`pool 4`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endgame: Option<PoolConfig>,
    /// Tarkhan `ASK_REINFORCE` support call-in ships (`addReinforceingShips` tier `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reinforcements: Option<PoolConfig>,
    /// `addReinforceingShips` tier `0` escort pool (can also be set via `tarkhans.tier0`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reinforcements_tier0: Option<PoolConfig>,
}

impl MercsConfig {
    pub fn summary(&self) -> String {
        summarize_with_subs(
            &self.base,
            &[
                ("hire", &self.hire),
                ("endgame", &self.endgame),
                ("reinforcements", &self.reinforcements),
                ("reinforcements_tier0", &self.reinforcements_tier0),
            ],
        )
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct TradersConfig {
    /// Base edits applied to **convoy escorts** (keeping `"traders": { "add": [...] }`
    /// safe for combat hulls; prize trade ships use `ships` so escorts never become
    /// unarmed prize targets by accident unless `ships` is edited).
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Prize trade ships (`[Pegasus, Odissey, Kastor, Pollux, Kadesh Mk1, Kastor, Pollux, Kadesh Mk1]`).
    /// Any custom design added here is automatically marked `m_prize = 1`, `m_playable = 0`
    /// on the spawned convoy hull so it can be captured intact!
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ships: Option<PoolConfig>,
    /// Standard convoy escorts (`9` hulls, fuel-range gated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escorts: Option<PoolConfig>,
    /// Hard-mode heavy convoy escort (`[Kormoran, Negev]`, 25% roll when `hardmode != 0`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escorts_hard: Option<PoolConfig>,
    /// Optional group-count adjustment for Trade Convoys: how many convoys are
    /// spawned at campaign start (vanilla `30`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<String>,
}

impl TradersConfig {
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
            && self.ships.is_none()
            && self.escorts.is_none()
            && self.escorts_hard.is_none()
    }

    pub fn summary(&self) -> String {
        let mut s = summarize_with_subs(
            &self.base,
            &[
                ("ships", &self.ships),
                ("escorts", &self.escorts),
                ("escorts_hard", &self.escorts_hard),
            ],
        );
        if let Some(count) = &self.count {
            if s != "vanilla" {
                s.push_str("; ");
            }
            s.push_str(&format!("count:{count}"));
        }
        s
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ShipworksConfig {
    /// Flat base edits: removed ships are dropped from all 4 checkboxes; added
    /// ships are auto-classified into `SMALL` / `MID` / `LARGE` / `CARRIER` by
    /// their live `LIB_SHIPS` stats (`planes` -> carrier, else combat value `<= 20` / `<= 55` / `> 55`).
    #[serde(flatten)]
    pub base: PoolConfig,
    /// Explicit `SMALL` checkbox (`EDC_TEST_SMALL`) override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small: Option<PoolConfig>,
    /// Explicit `MID` checkbox (`EDC_TEST_MID`) override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid: Option<PoolConfig>,
    /// Explicit `LARGE` checkbox (`EDC_TEST_LARGE`) override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large: Option<PoolConfig>,
    /// Explicit `CARRIER` checkbox (`EDC_TEST_CARRIER`) override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier: Option<PoolConfig>,
}

impl ShipworksConfig {
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
            && self.small.is_none()
            && self.mid.is_none()
            && self.large.is_none()
            && self.carrier.is_none()
    }

    pub fn summary(&self) -> String {
        summarize_with_subs(
            &self.base,
            &[
                ("small", &self.small),
                ("mid", &self.mid),
                ("large", &self.large),
                ("carrier", &self.carrier),
            ],
        )
    }
}

/// Read `escadra.json`, or write it with defaults when missing.
/// Missing keys take defaults (`#[serde(default)]`, unknown keys are ignored).
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
