//! Pure pool-resolution and budget/convoy-roll math (no game-memory access).
//!
//! Maps the eight top-level `escadra.json` categories (plus optional sub-pools)
//! onto HighFleet's internal sub-tables using deterministic `replace` -> `remove` -> `add` rules.

use std::collections::{BTreeMap, HashSet};
use std::ffi::CString;
use std::path::Path;

use crate::config::{parse_count_field, Config, CountOp, PoolConfig, ShipworksConfig};
use crate::native::DesignConvoyStats;
use crate::pools;

/// Case-insensitive wildcard matcher supporting `*` and `?`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().map(|c| c.to_ascii_lowercase()).collect();
    let t: Vec<char> = text.chars().map(|c| c.to_ascii_lowercase()).collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star_pi: Option<usize> = None;
    let mut star_ti: usize = 0;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn has_wildcard(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

fn scan_seria_stems(dir: &str, pattern: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut stems = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_seria = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("seria"));
        if !is_seria {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if glob_match(pattern, stem) {
            stems.push(stem.to_string());
        }
    }
    stems.sort();
    stems
}

fn strip_seria_suffix(s: &str) -> &str {
    if s.len() > 6 && s[s.len() - 6..].eq_ignore_ascii_case(".seria") {
        &s[..s.len() - 6]
    } else {
        s
    }
}

/// Expands `@group` references and `*` / `?` wildcards in `escadra.json` lists,
/// and records any explicit subdirectories referenced in path-qualified tokens.
struct DesignExpansionContext<'a> {
    groups: &'a BTreeMap<String, Vec<String>>,
    ship_dirs: Vec<String>,
}

impl<'a> DesignExpansionContext<'a> {
    fn new(config: &'a Config) -> Self {
        let mut ship_dirs = Vec::new();
        for dir in &config.ship_dirs {
            let norm = dir
                .trim()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string();
            if !norm.is_empty() && !ship_dirs.iter().any(|d| d == &norm) {
                ship_dirs.push(norm);
            }
        }
        Self {
            groups: &config.groups,
            ship_dirs,
        }
    }

    fn register_dir(&mut self, dir: &str) {
        let norm = dir
            .trim()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string();
        if !norm.is_empty() && !self.ship_dirs.iter().any(|d| d == &norm) {
            self.ship_dirs.push(norm);
        }
    }

    fn expand_groups(
        &self,
        raw_items: &[String],
        visited: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        for raw in raw_items {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(group_name) = trimmed.strip_prefix('@') {
                let group_key = group_name.trim();
                if !visited.insert(group_key.to_string()) {
                    log::error!("hf_hee: cyclic group reference \"@{group_key}\"");
                    continue;
                }
                if let Some(members) = self.groups.get(group_key) {
                    self.expand_groups(members, visited, out);
                } else {
                    log::warn!("hf_hee: unknown group \"@{group_key}\" in escadra.json");
                }
                visited.remove(group_key);
            } else {
                out.push(trimmed.to_string());
            }
        }
    }

    fn expand_add_list(&mut self, raw_items: &[String]) -> Vec<String> {
        let mut tokens = Vec::new();
        self.expand_groups(raw_items, &mut HashSet::new(), &mut tokens);

        let mut out = Vec::new();
        for token in tokens {
            let norm = token.replace('\\', "/");
            if let Some((raw_dir, raw_file)) = norm.rsplit_once('/') {
                let dir = raw_dir.trim_end_matches('/');
                let pat = strip_seria_suffix(raw_file.trim());
                let pat = if pat.is_empty() { "*" } else { pat };
                if !dir.is_empty() {
                    self.register_dir(dir);
                }
                if has_wildcard(pat) {
                    let matched = scan_seria_stems(dir, pat);
                    if matched.is_empty() {
                        log::warn!("hf_hee: wildcard \"{token}\" matched 0 .seria files");
                    } else {
                        log::info!("hf_hee: wildcard \"{token}\" -> {} ship(s)", matched.len());
                    }
                    out.extend(matched);
                } else {
                    out.push(pat.to_string());
                }
                continue;
            }

            let pat = strip_seria_suffix(&norm);
            if has_wildcard(pat) {
                let mut matched = Vec::new();
                let mut seen = HashSet::new();
                for dir in &self.ship_dirs {
                    for stem in scan_seria_stems(dir, pat) {
                        if seen.insert(stem.clone()) {
                            matched.push(stem);
                        }
                    }
                }
                for stem in scan_seria_stems("Objects/Designs", pat) {
                    if seen.insert(stem.clone()) {
                        matched.push(stem);
                    }
                }
                matched.sort();
                if matched.is_empty() {
                    log::warn!("hf_hee: wildcard \"{token}\" matched 0 .seria files");
                } else {
                    log::info!("hf_hee: wildcard \"{token}\" -> {} ship(s)", matched.len());
                }
                out.extend(matched);
            } else {
                out.push(pat.to_string());
            }
        }
        out
    }

    fn expand_remove_patterns(&self, raw_items: &[String]) -> Vec<String> {
        let mut tokens = Vec::new();
        self.expand_groups(raw_items, &mut HashSet::new(), &mut tokens);
        tokens
            .into_iter()
            .map(|t| {
                let norm = t.replace('\\', "/");
                let leaf = Path::new(&norm)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&norm);
                strip_seria_suffix(leaf).to_string()
            })
            .collect()
    }
}

/// Pre-sanitized ship design entry with both its UTF-8 name and a null-terminated
/// `CString` ready for zero-allocation native `LIB_SHIPS` lookups at spawn time.
#[derive(Clone, Debug)]
pub struct PoolEntry {
    pub name: String,
    pub c_name: CString,
}

impl PoolEntry {
    pub fn new(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let c_name = CString::new(trimmed).ok()?;
        Some(Self {
            name: trimmed.to_string(),
            c_name,
        })
    }
}

/// Shipworks `TEST` checkbox category (`SMALL` / `MID` / `LARGE` / `CARRIER`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipworksBucket {
    Small,
    Mid,
    Large,
    Carrier,
}

/// Classifies a custom ship for flat `"shipworks": { "add": [...] }` based on its
/// live `LIB_SHIPS` stats, matching vanilla's 4 checkboxes:
/// - `has_planes` -> `CARRIER` (`Wasp` CV 15, `Longbow` CV 27)
/// - `combat_value <= 20.0` -> `SMALL` (vanilla `Courageous`..`Flower` max 18.1)
/// - `combat_value <= 55.0` -> `MID` (vanilla `Intrepid`..`Fearsome`; `Archangel` bridges `MID`/`LARGE` at 61.3)
/// - `combat_value > 55.0` -> `LARGE` (vanilla `Archangel`, `Gryphon`..`Varyag`)
pub fn classify_shipworks_bucket(combat_value: f32, has_planes: bool) -> ShipworksBucket {
    if has_planes {
        ShipworksBucket::Carrier
    } else if combat_value <= 20.0 {
        ShipworksBucket::Small
    } else if combat_value <= 55.0 {
        ShipworksBucket::Mid
    } else {
        ShipworksBucket::Large
    }
}

/// Resolved Strike Group counts for each spawn wave, derived from
/// `strikegroups.count` applied to that wave's vanilla count.
#[derive(Clone, Copy, Debug)]
pub struct SgWaveCounts {
    /// Campaign-start roaming Strike Groups (vanilla `6`, `CMP ESI, 0x6` `0x28fd4d`).
    pub campaign: i32,
    /// Endgame Khiva assault wave (vanilla up to `4`, one per quadrant list).
    pub khiva: i32,
    /// Endgame outer Nuclear War wave (vanilla `3`, `CMP EAX, 0x3` `0x2629b1`).
    pub outer: i32,
}

/// Resolved ship lists for every native hook site, derived from `escadra.json`.
pub struct ResolvedPools {
    /// Ordered list of custom ship directories (`config.ship_dirs` plus any
    /// explicit directories from path-qualified wildcards/entries).
    pub ship_dirs: Vec<String>,

    pub sg_edited: bool,
    pub sg_flagships_normal: Vec<PoolEntry>,
    pub sg_flagships_hard: Vec<PoolEntry>,
    pub sg_ships_normal: Vec<PoolEntry>,
    pub sg_ships_hard: Vec<PoolEntry>,
    pub sg_extra_missile: Vec<PoolEntry>,
    pub sg_extra_carriers: Vec<PoolEntry>,
    pub sg_extra_khiva: Vec<PoolEntry>,

    pub sg_rider_edited: bool,
    pub sg_endgame_rider: Vec<PoolEntry>,

    pub spawn_ships_edited: bool,
    pub garrisons_campaign: Vec<PoolEntry>,
    pub garrisons_endgame: Vec<PoolEntry>,
    pub mercs_cities_campaign: Vec<PoolEntry>,
    pub mercs_cities_endgame: Vec<PoolEntry>,
    pub pool_1_dead: Vec<PoolEntry>,
    pub pool_3_dead: Vec<PoolEntry>,

    pub tactical_air: Vec<PoolEntry>,

    pub tactical_missile_edited: bool,
    pub tactical_missile_ships: Vec<PoolEntry>,
    pub tactical_missile_escorts: Vec<PoolEntry>,

    pub tarkhan_campaign_edited: bool,
    pub tarkhans_campaign: Vec<PoolEntry>,

    pub tarkhan_khiva_edited: bool,
    pub tarkhans_khiva_groups: [Vec<PoolEntry>; 4],

    pub reinforcements_edited: bool,
    pub reinforcements_tier0: Vec<PoolEntry>,
    pub reinforcements_tier1: Vec<PoolEntry>,

    pub traders_edited: bool,
    pub traders_ships: Vec<PoolEntry>,
    pub traders_escorts: Vec<PoolEntry>,
    pub traders_escorts_hard: Vec<PoolEntry>,
    /// Resolved Trade Convoy count (`traders.count` vs vanilla `30`).
    pub traders_count: Option<i32>,

    /// Resolved Strike Group wave counts (`strikegroups.count` vs vanilla `6`/`4`/`3`).
    pub sg_wave_counts: Option<SgWaveCounts>,
    /// `strikegroups.sprinkle`: pick a random eligible city per campaign SG
    /// instead of the first eligible city in vector order.
    pub sg_sprinkle: bool,

    /// Group-count ops applied at spawn time to the per-city tactical-group
    /// chances (`chanceA` / `chanceB`, pool `0` garrisons only).
    pub tactical_air_count_op: Option<CountOp>,
    pub tactical_missile_count_op: Option<CountOp>,

    pub shipworks_edited: bool,
    pub shipworks_small: Vec<PoolEntry>,
    pub shipworks_mid: Vec<PoolEntry>,
    pub shipworks_large: Vec<PoolEntry>,
    pub shipworks_carrier: Vec<PoolEntry>,
    /// Flat additions that get auto-classified by live `LIB_SHIPS` stats at `TEST` click time.
    pub shipworks_auto_adds: Vec<PoolEntry>,
}

/// Applies `replace` -> `remove` -> `add` to a vanilla `&[&str]` slice,
/// expanding `@group` references and `*`/`?` wildcards.
///
/// Removing a name (or wildcard like `"Gladiator*"`) drops **all** matching occurrences.
fn resolve_pool_with_ctx(
    vanilla: &[&str],
    cfg: &PoolConfig,
    ctx: &mut DesignExpansionContext<'_>,
) -> Vec<PoolEntry> {
    let base: Vec<String> = match &cfg.replace {
        Some(replacement) => ctx.expand_add_list(replacement),
        None => vanilla.iter().map(|s| (*s).to_string()).collect(),
    };
    let remove_patterns = ctx.expand_remove_patterns(&cfg.remove);
    let added_names = ctx.expand_add_list(&cfg.add);

    let mut items: Vec<PoolEntry> = Vec::with_capacity(base.len() + added_names.len());
    for candidate in base {
        let trimmed = candidate.trim();
        let is_removed = remove_patterns.iter().any(|pat| {
            if has_wildcard(pat) {
                glob_match(pat, trimmed)
            } else {
                pat == trimmed
            }
        });
        if is_removed {
            continue;
        }
        if let Some(entry) = PoolEntry::new(trimmed) {
            items.push(entry);
        }
    }

    for added in added_names {
        if let Some(entry) = PoolEntry::new(&added) {
            items.push(entry);
        }
    }

    items
}

fn select_sub<'a>(sub: &'a Option<PoolConfig>, fallback: &'a PoolConfig) -> &'a PoolConfig {
    sub.as_ref().unwrap_or(fallback)
}

struct ShipworksPools {
    small: Vec<PoolEntry>,
    mid: Vec<PoolEntry>,
    large: Vec<PoolEntry>,
    carrier: Vec<PoolEntry>,
    auto_adds: Vec<PoolEntry>,
}

fn resolve_shipworks(
    sw: &ShipworksConfig,
    garrison_sync: Option<&PoolConfig>,
    ctx: &mut DesignExpansionContext<'_>,
) -> ShipworksPools {
    let mut flat_remove = ctx.expand_remove_patterns(&sw.base.remove);
    let mut auto_add_names = ctx.expand_add_list(&sw.base.add);

    if let Some(gs) = garrison_sync {
        flat_remove.extend(ctx.expand_remove_patterns(&gs.remove));
        auto_add_names.extend(ctx.expand_add_list(&gs.add));
    }

    // When `sw.base.replace` is set and a bucket has no explicit override, replace
    // all 3 hull buckets via auto-classification (`small/mid/large/carrier` start empty
    // and receive auto-classified entries from `sw.base.replace` + `add`).
    let base_replace_active = sw.base.replace.is_some();
    if let Some(rep) = &sw.base.replace {
        let expanded_rep = ctx.expand_add_list(rep);
        let mut combined: Vec<String> = expanded_rep
            .iter()
            .filter(|cand| {
                !flat_remove.iter().any(|pat| {
                    if has_wildcard(pat) {
                        glob_match(pat, cand.trim())
                    } else {
                        pat == cand.trim()
                    }
                })
            })
            .cloned()
            .collect();
        combined.extend(auto_add_names);
        auto_add_names = combined;
    }

    let empty_replace = PoolConfig {
        replace: Some(Vec::new()),
        remove: Vec::new(),
        add: Vec::new(),
    };
    let remove_only = PoolConfig {
        replace: None,
        remove: flat_remove,
        add: Vec::new(),
    };
    let default_bucket_cfg = if base_replace_active {
        &empty_replace
    } else {
        &remove_only
    };

    let small = match &sw.small {
        Some(cfg) => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_SMALL, cfg, ctx),
        None => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_SMALL, default_bucket_cfg, ctx),
    };
    let mid = match &sw.mid {
        Some(cfg) => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_MID, cfg, ctx),
        None => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_MID, default_bucket_cfg, ctx),
    };
    let large = match &sw.large {
        Some(cfg) => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_LARGE, cfg, ctx),
        None => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_LARGE, default_bucket_cfg, ctx),
    };
    let carrier = match &sw.carrier {
        Some(cfg) => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_CARRIER, cfg, ctx),
        None => resolve_pool_with_ctx(pools::VANILLA_SHIPWORKS_CARRIER, default_bucket_cfg, ctx),
    };

    let auto_adds: Vec<PoolEntry> = auto_add_names
        .iter()
        .filter_map(|s| PoolEntry::new(s))
        .collect();

    ShipworksPools {
        small,
        mid,
        large,
        carrier,
        auto_adds,
    }
}

impl ResolvedPools {
    pub fn from_config(config: &Config) -> Self {
        let mut ctx = DesignExpansionContext::new(config);
        let empty = PoolConfig::default();
        let sg = &config.strikegroups;
        let tm = &config.tactical_missile;

        // Strike Group carrier escorts: explicit `strikegroups.carrier_escorts` wins,
        // else syncs with `tactical_air` when edited.
        let sg_carrier_cfg = sg
            .carrier_escorts
            .as_ref()
            .unwrap_or(&config.tactical_air.base);

        // Strike Group missile frigate/cruiser escort (`Tarantul ARM` in vanilla):
        // explicit `strikegroups.missile_escorts` wins, else `tactical_missile.sg_missile`,
        // else `tactical_missile.base`.
        let sg_missile_cfg = sg
            .missile_escorts
            .as_ref()
            .or(tm.sg_missile.as_ref())
            .unwrap_or(&tm.base);

        let sg_khiva_cfg = sg.khiva_extra.as_ref().unwrap_or(&empty);

        let sg_edited = !sg.base.is_empty()
            || sg.flagships_normal.is_some()
            || sg.flagships_hard.is_some()
            || sg.fill_normal.is_some()
            || sg.fill_hard.is_some()
            || !sg_carrier_cfg.is_empty()
            || !sg_missile_cfg.is_empty()
            || !sg_khiva_cfg.is_empty();

        // Endgame outer SG phase-3 rider (`66% Tarantul`, `34% Tarantul ARM`):
        // explicit `strikegroups.endgame_rider` wins, else `tactical_missile.endgame_rider`,
        // else `tactical_missile.base`.
        let sg_rider_cfg = sg
            .endgame_rider
            .as_ref()
            .or(tm.endgame_rider.as_ref())
            .unwrap_or(&tm.base);
        let sg_rider_edited = !sg_rider_cfg.is_empty();

        // Garrisons: usual campaign (`campaign` or `base`) vs late-game Nuclear War (`endgame` or `base`).
        let garrisons_campaign_cfg = select_sub(&config.garrisons.campaign, &config.garrisons.base);
        let garrisons_endgame_cfg = select_sub(&config.garrisons.endgame, &config.garrisons.base);

        // Mercs: `hire` (pool 2), `endgame` (pool 4 — also falls back to `garrisons.endgame` if
        // the user put late-game Khiva-defense city ships under `garrisons.endgame` without setting `mercs.endgame`!),
        // and `reinforcements` (`ASK_REINFORCE` tier 1).
        let mercs_hire_cfg = select_sub(&config.mercs.hire, &config.mercs.base);
        let mercs_endgame_cfg = config
            .mercs
            .endgame
            .as_ref()
            .or_else(|| {
                if config.mercs.base.is_empty() {
                    config.garrisons.endgame.as_ref()
                } else {
                    None
                }
            })
            .unwrap_or(&config.mercs.base);

        let tm_ships_cfg = select_sub(&tm.ships, &tm.base);
        let tm_escorts_cfg = tm.escorts.as_ref().unwrap_or(&empty);
        let tactical_missile_edited = !tm_ships_cfg.is_empty() || !tm_escorts_cfg.is_empty();

        // Group-count fields: `traders.count` / `strikegroups.count` resolve against
        // the vanilla spawn-loop counts; `tactical_air.count` / `tactical_missile.count`
        // keep the op to apply to the per-city `chanceA` / `chanceB` at spawn time
        // (pool `0` garrisons only — merc pools pass `0.0` and must stay unaffected).
        let traders_count = parse_count_field("traders", &config.traders.count)
            .map(|op| op.apply_int(pools::VANILLA_TRADER_CONVOY_COUNT as i64));
        let sg_wave_counts = parse_count_field("strikegroups", &sg.count).map(|op| SgWaveCounts {
            campaign: op.apply_int(pools::VANILLA_SG_CAMPAIGN_COUNT as i64),
            khiva: op.apply_int(pools::VANILLA_SG_KHIVA_COUNT as i64),
            outer: op.apply_int(pools::VANILLA_SG_OUTER_COUNT as i64),
        });
        let tactical_air_count_op = parse_count_field("tactical_air", &config.tactical_air.count);
        let tactical_missile_count_op = parse_count_field("tactical_missile", &tm.count);
        if let Some(op) = tactical_air_count_op {
            log::info!(
                "hf_hee: tactical_air.count {} applies to chanceA (pool 0)",
                op.describe()
            );
        }
        if let Some(op) = tactical_missile_count_op {
            log::info!(
                "hf_hee: tactical_missile.count {} applies to chanceB (pool 0)",
                op.describe()
            );
        }
        if let Some(count) = traders_count {
            log::info!(
                "hf_hee: traders.count -> {count} convoys (vanilla {})",
                pools::VANILLA_TRADER_CONVOY_COUNT
            );
        }
        if let Some(counts) = sg_wave_counts {
            log::info!(
                "hf_hee: strikegroups.count -> campaign {} (vanilla {}) / khiva {} (vanilla {}) / outer {} (vanilla {})",
                counts.campaign, pools::VANILLA_SG_CAMPAIGN_COUNT,
                counts.khiva, pools::VANILLA_SG_KHIVA_COUNT,
                counts.outer, pools::VANILLA_SG_OUTER_COUNT,
            );
        }

        let spawn_ships_edited = !garrisons_campaign_cfg.is_empty()
            || !garrisons_endgame_cfg.is_empty()
            || !mercs_hire_cfg.is_empty()
            || !mercs_endgame_cfg.is_empty()
            || !config.tactical_air.base.is_empty()
            || tactical_missile_edited
            || tactical_air_count_op.is_some()
            || tactical_missile_count_op.is_some();

        // Tarkhans: `campaign` (city `FOR_TARKHAN`) and `tier1..tier4` (`ENDWAR_CARD` Khiva groups 1..4).
        let tarkhan_campaign_cfg = select_sub(&config.tarkhans.campaign, &config.tarkhans.base);
        let tarkhan_campaign_edited = !tarkhan_campaign_cfg.is_empty();

        let tk_g1_cfg = select_sub(&config.tarkhans.tier1, &config.tarkhans.base);
        let tk_g2_cfg = select_sub(&config.tarkhans.tier2, &config.tarkhans.base);
        let tk_g3_cfg = select_sub(&config.tarkhans.tier3, &config.tarkhans.base);
        let tk_g4_cfg = select_sub(&config.tarkhans.tier4, &config.tarkhans.base);
        let tarkhan_khiva_edited = !tk_g1_cfg.is_empty()
            || !tk_g2_cfg.is_empty()
            || !tk_g3_cfg.is_empty()
            || !tk_g4_cfg.is_empty();

        // Reinforcements (`addReinforceingShips` `0x21b96f`):
        // - `tier0` (Khiva escort fallback): `tarkhans.tier0` wins, else `mercs.reinforcements_tier0`, else `mercs.base`.
        // - `tier1` (`ASK_REINFORCE` Tarkhan support): `mercs.reinforcements` wins, else `mercs.base`.
        let reinf_t0_cfg = config
            .tarkhans
            .tier0
            .as_ref()
            .or(config.mercs.reinforcements_tier0.as_ref())
            .unwrap_or(&config.mercs.base);
        let reinf_t1_cfg = select_sub(&config.mercs.reinforcements, &config.mercs.base);
        let reinforcements_edited = !reinf_t0_cfg.is_empty() || !reinf_t1_cfg.is_empty();

        // Traders (`spawnTradeConvoy?` `0x208660`):
        // - `ships` = prize trade ships (only changed when `traders.ships` is set, so flat `traders.add`
        //   safely edits escorts without turning combat ships into unarmed prizes by mistake!)
        // - `escorts` = `traders.escorts` or `traders.base`
        // - `escorts_hard` = `traders.escorts_hard` or `empty` (unless `traders.base.remove` removes `Kormoran`/`Negev`)
        let traders_ships_cfg = config.traders.ships.as_ref().unwrap_or(&empty);
        let traders_escorts_cfg = select_sub(&config.traders.escorts, &config.traders.base);
        let default_hard_escorts_cfg = PoolConfig {
            replace: None,
            remove: config.traders.base.remove.clone(),
            add: Vec::new(),
        };
        let traders_escorts_hard_cfg = config
            .traders
            .escorts_hard
            .as_ref()
            .unwrap_or(&default_hard_escorts_cfg);
        let traders_edited = !config.traders.is_empty();

        // Shipworks `TEST` combat (`Fleet_BuildShopOffer` `0x206950`):
        let garrison_sync = config
            .garrisons
            .include_in_shipworks
            .then_some(garrisons_campaign_cfg);
        let shipworks_edited =
            !config.shipworks.is_empty() || garrison_sync.is_some_and(|gs| !gs.is_empty());
        let shipworks = resolve_shipworks(&config.shipworks, garrison_sync, &mut ctx);

        let sg_flagships_normal = resolve_pool_with_ctx(
            pools::VANILLA_SG_FLAGSHIPS_NORMAL,
            select_sub(&sg.flagships_normal, &sg.base),
            &mut ctx,
        );
        let sg_flagships_hard = resolve_pool_with_ctx(
            pools::VANILLA_SG_FLAGSHIPS_HARD,
            select_sub(&sg.flagships_hard, &sg.base),
            &mut ctx,
        );
        let sg_ships_normal = resolve_pool_with_ctx(
            pools::VANILLA_SG_SHIPS_NORMAL,
            select_sub(&sg.fill_normal, &sg.base),
            &mut ctx,
        );
        let sg_ships_hard = resolve_pool_with_ctx(
            pools::VANILLA_SG_SHIPS_HARD,
            select_sub(&sg.fill_hard, &sg.base),
            &mut ctx,
        );
        let sg_extra_missile =
            resolve_pool_with_ctx(pools::VANILLA_SG_EXTRA_MISSILE, sg_missile_cfg, &mut ctx);
        let sg_extra_carriers =
            resolve_pool_with_ctx(pools::VANILLA_SG_EXTRA_CARRIERS, sg_carrier_cfg, &mut ctx);
        let sg_extra_khiva =
            resolve_pool_with_ctx(pools::VANILLA_SG_EXTRA_KHIVA, sg_khiva_cfg, &mut ctx);
        let sg_endgame_rider =
            resolve_pool_with_ctx(pools::VANILLA_SG_ENDGAME_RIDER, sg_rider_cfg, &mut ctx);
        let garrisons_campaign =
            resolve_pool_with_ctx(pools::VANILLA_GARRISONS, garrisons_campaign_cfg, &mut ctx);
        let garrisons_endgame =
            resolve_pool_with_ctx(pools::VANILLA_GARRISONS, garrisons_endgame_cfg, &mut ctx);
        let mercs_cities_campaign = resolve_pool_with_ctx(
            pools::VANILLA_MERCS_CITIES_CAMPAIGN,
            mercs_hire_cfg,
            &mut ctx,
        );
        let mercs_cities_endgame = resolve_pool_with_ctx(
            pools::VANILLA_MERCS_CITIES_ENDGAME,
            mercs_endgame_cfg,
            &mut ctx,
        );
        let pool_1_dead = resolve_pool_with_ctx(pools::VANILLA_DEAD_POOL_1, &empty, &mut ctx);
        let pool_3_dead = resolve_pool_with_ctx(pools::VANILLA_DEAD_POOL_3, &empty, &mut ctx);
        let tactical_air = resolve_pool_with_ctx(
            pools::VANILLA_TACTICAL_AIR,
            &config.tactical_air.base,
            &mut ctx,
        );
        let tactical_missile_ships = resolve_pool_with_ctx(
            pools::VANILLA_TACTICAL_MISSILE_SHIPS,
            tm_ships_cfg,
            &mut ctx,
        );
        let tactical_missile_escorts = resolve_pool_with_ctx(
            pools::VANILLA_TACTICAL_MISSILE_ESCORTS,
            tm_escorts_cfg,
            &mut ctx,
        );
        let tarkhans_campaign = resolve_pool_with_ctx(
            pools::VANILLA_TARKHANS_CAMPAIGN,
            tarkhan_campaign_cfg,
            &mut ctx,
        );
        let tarkhans_khiva_groups = [
            resolve_pool_with_ctx(pools::VANILLA_TARKHANS_KHIVA_GROUP1, tk_g1_cfg, &mut ctx),
            resolve_pool_with_ctx(pools::VANILLA_TARKHANS_KHIVA_GROUP2, tk_g2_cfg, &mut ctx),
            resolve_pool_with_ctx(pools::VANILLA_TARKHANS_KHIVA_GROUP3, tk_g3_cfg, &mut ctx),
            resolve_pool_with_ctx(pools::VANILLA_TARKHANS_KHIVA_GROUP4, tk_g4_cfg, &mut ctx),
        ];
        let reinforcements_tier0 = resolve_pool_with_ctx(
            pools::VANILLA_MERCS_REINFORCEMENTS_TIER0,
            reinf_t0_cfg,
            &mut ctx,
        );
        let reinforcements_tier1 = resolve_pool_with_ctx(
            pools::VANILLA_MERCS_REINFORCEMENTS_TIER1,
            reinf_t1_cfg,
            &mut ctx,
        );
        let traders_ships =
            resolve_pool_with_ctx(pools::VANILLA_TRADERS_SHIPS, traders_ships_cfg, &mut ctx);
        let traders_escorts = resolve_pool_with_ctx(
            pools::VANILLA_TRADERS_ESCORTS,
            traders_escorts_cfg,
            &mut ctx,
        );
        let traders_escorts_hard = resolve_pool_with_ctx(
            pools::VANILLA_TRADERS_ESCORTS_HARD,
            traders_escorts_hard_cfg,
            &mut ctx,
        );

        Self {
            ship_dirs: ctx.ship_dirs,
            sg_edited,
            sg_flagships_normal,
            sg_flagships_hard,
            sg_ships_normal,
            sg_ships_hard,
            sg_extra_missile,
            sg_extra_carriers,
            sg_extra_khiva,

            sg_rider_edited,
            sg_endgame_rider,

            spawn_ships_edited,
            garrisons_campaign,
            garrisons_endgame,
            mercs_cities_campaign,
            mercs_cities_endgame,
            pool_1_dead,
            pool_3_dead,

            tactical_air,

            tactical_missile_edited,
            tactical_missile_ships,
            tactical_missile_escorts,

            tarkhan_campaign_edited,
            tarkhans_campaign,

            tarkhan_khiva_edited,
            tarkhans_khiva_groups,

            reinforcements_edited,
            reinforcements_tier0,
            reinforcements_tier1,

            traders_edited,
            traders_ships,
            traders_escorts,
            traders_escorts_hard,
            traders_count,
            sg_wave_counts,
            sg_sprinkle: sg.sprinkle,
            tactical_air_count_op,
            tactical_missile_count_op,

            shipworks_edited,
            shipworks_small: shipworks.small,
            shipworks_mid: shipworks.mid,
            shipworks_large: shipworks.large,
            shipworks_carrier: shipworks.carrier,
            shipworks_auto_adds: shipworks.auto_adds,
        }
    }

    /// True when at least one pool's ship roster is edited.
    pub fn has_any_ship_edits(&self) -> bool {
        self.sg_edited
            || self.sg_rider_edited
            || self.spawn_ships_edited
            || self.tarkhan_campaign_edited
            || self.tarkhan_khiva_edited
            || self.reinforcements_edited
            || self.traders_edited
            || self.shipworks_edited
    }

    /// True when at least one pool in `escadra.json` differs from vanilla.
    pub fn has_any_edits(&self) -> bool {
        self.has_any_ship_edits()
            || self.traders_count.is_some()
            || self.sg_wave_counts.is_some()
            || self.sg_sprinkle
            || self.tactical_air_count_op.is_some()
            || self.tactical_missile_count_op.is_some()
    }
}

/// Exact replica of `somethingSpawnShips`'s (`0x1402056f1`..`0x1402058f9`) and
/// `Fleet_SelectShipsForBudget`'s (`0x140206060`..`0x1402068f0`) 10-attempt budget roller.
pub fn roll_spawn_ships_indices<R>(
    prices: &[f32],
    budget: f32,
    any_size: bool,
    fallback_random_if_empty: bool,
    mut rand_index: R,
) -> Vec<usize>
where
    R: FnMut(usize) -> usize,
{
    if prices.is_empty() {
        return Vec::new();
    }
    if budget <= 0.0 {
        if fallback_random_if_empty {
            return vec![rand_index(prices.len())];
        }
        return Vec::new();
    }

    let slack = budget * 0.2;
    let max_allowed = budget + slack;

    let mut best_any: Vec<usize> = Vec::new();
    let mut best_any_total: f32 = 0.0;
    let mut best_ge2: Vec<usize> = Vec::new();
    let mut best_ge2_total: f32 = 0.0;
    let mut best_ge3: Vec<usize> = Vec::new();
    let mut best_ge3_total: f32 = 0.0;

    for _attempt in 0..10 {
        let mut candidate: Vec<usize> = Vec::new();
        let mut rejects_left: u32 = 10;
        let mut total: f32 = 0.0;

        while total < budget && rejects_left > 0 {
            let idx = rand_index(prices.len());
            let next_total = total + prices[idx];
            if next_total > max_allowed {
                rejects_left -= 1;
            } else {
                total = next_total;
                candidate.push(idx);
                rejects_left = 10;
            }
        }

        if total > best_any_total {
            best_any_total = total;
            best_any.clone_from(&candidate);
        }
        if candidate.len() >= 2 && total > best_ge2_total {
            best_ge2_total = total;
            best_ge2.clone_from(&candidate);
        }
        if candidate.len() >= 3 && total > best_ge3_total {
            best_ge3_total = total;
            best_ge3.clone_from(&candidate);

            let delta = total - budget;
            if delta > -0.5 * slack && delta < slack {
                break;
            }
        }
    }

    let chosen = if any_size {
        best_any
    } else if !best_ge3.is_empty() {
        best_ge3
    } else if !best_ge2.is_empty() {
        best_ge2
    } else {
        best_any
    };

    if chosen.is_empty() && fallback_random_if_empty {
        vec![rand_index(prices.len())]
    } else {
        chosen
    }
}

/// Exact replica of `Endgame_BuildEscadra`'s flagship + fill budget roller
/// (`0x1402080ba`..`0x140208243`), returning `(Option<flagship_idx>, Vec<fill_idx>)`.
pub fn roll_strikegroup_indices<R>(
    flagship_prices: &[f32],
    fill_prices: &[f32],
    budget: f32,
    mut rand_index: R,
) -> (Option<usize>, Vec<usize>)
where
    R: FnMut(usize) -> usize,
{
    let mut total: f32 = 0.0;
    let mut chosen_flagship: Option<usize> = None;

    if !flagship_prices.is_empty() {
        let half_budget = budget * 0.5;
        for (idx, &price) in flagship_prices.iter().enumerate() {
            if price <= half_budget {
                chosen_flagship = Some(idx);
                total += price;
                break;
            }
        }
        if chosen_flagship.is_none() {
            let last_idx = flagship_prices.len() - 1;
            chosen_flagship = Some(last_idx);
            total += flagship_prices[last_idx];
        }
    }

    let mut chosen_fill: Vec<usize> = Vec::new();
    if fill_prices.is_empty() {
        return (chosen_flagship, chosen_fill);
    }

    let max_allowed = budget + budget * 0.2;
    let mut retries_left: u32 = 10;

    while total < budget && retries_left > 0 {
        let idx = rand_index(fill_prices.len());
        let next_total = total + fill_prices[idx];
        if next_total >= max_allowed {
            retries_left -= 1;
        } else {
            chosen_fill.push(idx);
            total = next_total;
            retries_left = 10;
        }
    }

    (chosen_flagship, chosen_fill)
}

/// Exact replica of `spawnTradeConvoy?`'s convoy fuel-range eligibility check
/// (`0x140209096`..`0x140209191`, threshold `3_000_000.0` at `0x1403565fc`).
///
/// Each already-accepted ship in `convoy` carries `(airspeed, fuel_capacity, fuel_rate)`,
/// while the candidate's initial fuel contribution uses `candidate.fuel_total` (then stores
/// `candidate.fuel_capacity` once accepted — matching `0x14020911a` vs `0x140209221`).
pub fn convoy_can_support_escort(
    convoy: &[DesignConvoyStats],
    candidate: DesignConvoyStats,
) -> bool {
    if candidate.airspeed <= 0.0 {
        return false;
    }
    let mut min_speed = candidate.airspeed;
    for member in convoy {
        if member.airspeed > 0.0 && member.airspeed < min_speed {
            min_speed = member.airspeed;
        }
    }

    let mut total_rate = (min_speed / candidate.airspeed) * candidate.fuel_rate;
    let mut total_fuel = candidate.fuel_total;

    for member in convoy {
        if member.airspeed > 0.0 {
            total_rate += (min_speed / member.airspeed) * member.fuel_rate;
        }
        total_fuel += member.fuel_capacity;
    }

    if total_rate <= 0.0 {
        return true;
    }
    (total_fuel / total_rate) * min_speed > 3_000_000.0
}
