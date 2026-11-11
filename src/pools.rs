//! The map from `escadra.json` categories to the game's pool code
//! (Steam 1.163, image base `0x140000000`; addresses are RVAs).
//!
//! Findings and evidence: `hf_notes/docs/findings/strategy/enemy-ship-pools.md`.
//! Every pool is hard-coded inline in `.text`; entries are resolved by name
//! through the design library at spawn time (`retCreature154c?` `0x219980`,
//! strength at design `+0x15f8`). That library is the `LIB_SHIPS` node
//! (global `0x143a13a70`), built at startup by `PreLoadLibraries_`
//! (`0x233710`) from the seria files in `Objects/Designs/`. So a name in
//! `escadra.json` is valid exactly when `Objects/Designs/<Name>.seria`
//! exists; dropping a new design there makes it eligible.

// The vanilla pool contents below are reference data: the v0.2 pool detours
// check the pools the game builds against them, and the README mirrors them
// for config authors. Until the detours exist nothing reads them.
#![expect(dead_code)]

use crate::config::Config;

/// `Endgame_BuildEscadra` (`0x2077c0`): the Strike Group composer.
pub const STRIKEGROUPS_FN: usize = 0x2077c0;
/// `somethingSpawnShips` (`0x203b80`): strength-table pools; the pool is its
/// `R8D` argument. Pool 0 backs the campaign-start settlement garrisons,
/// the Khiva quest fleet and the tarkhan/event escadras linked into the
/// event records (`record+0x2b8`, strength `+0x248`); pool 2 backs the
/// group-6 tarkhan/NPC fleets; pool 4 backs the endgame garrisons.
pub const SPAWN_SHIPS_FN: usize = 0x203b80;
/// `Fleet_GenerateShipNames` (`0x205ae0`): the 46-name price-bucket
/// catalogue for the skirmish escadras.
pub const GENERATE_SHIP_NAMES_FN: usize = 0x205ae0;
/// `Fleet_CreateRandomEscadraForSlot` (`0x1c2ba0`): the level presets of the
/// skirmish ladder; the slot id comes from `Profile+0x5dc`
/// (`nextSkirmishEscadraLevel`, serialized), initialized to 1 and stepped
/// once per skirmish escadra.
pub const SKIRMISH_FOR_SLOT_FN: usize = 0x1c2ba0;
/// `addReinforceingShips` (`0x21b8c0`): the tarkhan for-hire / event
/// reinforcement tables. Callers: `Dialog_ExecuteChoiceActions`
/// (`ASK_REINFORCE`, tier 1) and `creatureLogic` (event/script path, tier 0).
pub const ADD_REINFORCING_SHIPS_FN: usize = 0x21b8c0;

/// `Prologue_SpawnEventGroups` (`0x2a7f60`): group-6 tarkhan/NPC fleets use
/// `somethingSpawnShips` pool 2. Both tactical specializations share it.
pub const TACTICAL_POOL_ID: u32 = 2;
/// Pool 0: campaign-start settlement garrisons, the Khiva quest fleet and
/// the tarkhan/event escadras.
pub const GARRISON_CAMPAIGN_POOL_ID: u32 = 0;
/// Pool 4: endgame city garrisons.
pub const GARRISON_ENDGAME_POOL_ID: u32 = 4;

/// Vanilla pool 0 (campaign garrisons / event escadras).
pub const VANILLA_GARRISONS_CAMPAIGN: &[&str] = &[
    "Courageous",
    "Slogger",
    "Navarin",
    "Sarma",
    "Ballistic",
    "Archangel",
    "Fearsome",
    "Flower",
    "Gladiator",
    "Gladiator MRL",
    "Intrepid",
    "Nimrod",
    "Paladin Mk2",
];

/// Vanilla pool 4 (endgame garrisons).
pub const VANILLA_GARRISONS_ENDGAME: &[&str] = &[
    "Archangel",
    "Fatman",
    "Fearsome",
    "Fenek",
    "Gepard",
    "Gladiator",
    "Intrepid",
    "Kormoran",
    "Negev",
    "Paladin Mk2",
    "Scarab",
    "Tanker",
    "Voskhod",
    "Wasp",
];

/// Vanilla pool 2 (tactical groups; both air and missile specializations).
pub const VANILLA_TACTICAL: &[&str] = &[
    "Yars",
    "Yars Mk2",
    "Mockingbird",
    "Skylark",
    "Tanker",
    "Rook",
    "Voskhod",
    "Negev",
    "Navarin",
    "Intrepid",
    "Fearsome",
    "Paladin Mk2",
    "Gladiator",
    "Gepard",
    "Kormoran",
    "Longbow",
    "Lightning",
    "Jaguar",
    "Fatman",
    "Scarab",
    "Wasp",
    "Flower",
    "Fenek",
    "Egida Mk2",
    "Archangel",
    "Triumphant",
    "Meteor Mk3",
];

/// Air-specialized extras (`'Eagle'` branch of the tactical post-rolls).
pub const VANILLA_TACTICAL_AIR_EXTRAS: &[&str] = &["Eagle"];
/// Missile-specialized extras (`'Longbow'` and `'Tarantul MRL'` post-rolls;
/// `'Tarantul'` and `'Hound'` are added before the air/missile coin flips,
/// so they are counted in both specializations).
pub const VANILLA_TACTICAL_MISSILE_EXTRAS: &[&str] =
    &["Longbow", "Tarantul", "Hound", "Tarantul MRL"];

/// Vanilla Strike Group catalogue (`Endgame_BuildEscadra`). `Borey` is
/// seeded twice (a second copy is appended); the `Gryphon`/`Nomad` names are
/// added only when profile flag `+0x165` is set.
pub const VANILLA_STRIKEGROUPS: &[&str] = &["Borey", "Kormoran", "Negev"];
pub const VANILLA_STRIKEGROUPS_FLAGGED: &[&str] = &["Nomad Mk2", "Nomad", "Gryphon", "Gryphon"];
/// Strike Group post-roll specials.
pub const VANILLA_STRIKEGROUPS_EXTRAS: &[&str] = &["Tarantul ARM", "Wasp", "Longbow", "Typhon"];

/// The skirmish catalogue: 46 pushes (Fearsome appears twice in the code;
/// de-duplicated here) fed through `Fleet_AddNamedShipToPool` and filtered
/// into five price buckets (<=15 / <=25 / <=40 / <=75 / >75). The level
/// presets that draw from it live in `Fleet_CreateRandomEscadraForSlot`.
pub const VANILLA_SKIRMISH: &[&str] = &[
    "Courageous",
    "Meteor Mk2",
    "Rook",
    "Meteor Mk3",
    "Slogger",
    "Lightning",
    "Talos",
    "Navarin Mk2",
    "Flower",
    "Navarin",
    "Fenek",
    "Ballistic",
    "Lightning Mk2",
    "Navarin Mk3",
    "Intrepid",
    "Fatman",
    "Fury",
    "Scarab",
    "Lancer",
    "Sarma",
    "Jaguar RSZO",
    "Jaguar",
    "Fearsome",
    "Intrepid Mk2",
    "Jaguar Mk2",
    "Egida",
    "Eagle",
    "Gladiator",
    "Gladiator MRL",
    "Gepard",
    "Intrepid Mk3",
    "Paladin Mk2",
    "Paladin",
    "Gladiator LANCE",
    "Longbow",
    "Kormoran",
    "Archangel",
    "Negev",
    "Borey",
    "Novorossiysk",
    "Nomad",
    "Nomad Mk2",
    "Sevastopol",
];

/// Tarkhan event reinforcements (`addReinforceingShips` tier 0, rolled by
/// `HGE_RandInt(0,4)`).
pub const VANILLA_TARKHANS_TIER0: &[&str] =
    &["Navarin", "Scarab", "Meteor Mk3", "Fatman", "Lightning Mk2"];
/// Tarkhan ships for hire at a city (`addReinforceingShips` tier 1, rolled by
/// `HGE_RandInt(0,7)`).
pub const VANILLA_TARKHANS_TIER1: &[&str] = &[
    "Navarin",
    "Intrepid",
    "Jaguar",
    "Voskhod X2",
    "Triumphant",
    "Longbow",
    "Vega",
    "Kormoran",
];
/// Tier 2 uses the tier-1 list twice; no live caller found in 1.163.
pub const VANILLA_TARKHANS_TIER2: &[&str] = VANILLA_TARKHANS_TIER1;

/// Log one line per config category: what `escadra.json` asks for and which
/// code path it will be applied to. Until the hooks exist this is the only
/// effect of the config.
pub fn log_summary(config: &Config) {
    log::info!(
        "HEE strikegroups   @ {STRIKEGROUPS_FN:#x}: {}",
        config.strikegroups.summary()
    );
    log::info!(
        "HEE tactical air    @ {SPAWN_SHIPS_FN:#x} pool {TACTICAL_POOL_ID}: {} (base pool shared with missile)",
        config.tactical_air.summary()
    );
    log::info!(
        "HEE tactical air    extras: {} | missile extras: {}",
        config.tactical_air.summary(),
        config.tactical_missile.summary()
    );
    log::info!(
        "HEE garrisons camp. @ {SPAWN_SHIPS_FN:#x} pool {GARRISON_CAMPAIGN_POOL_ID}: {}",
        config.garrisons.campaign.summary()
    );
    log::info!(
        "HEE garrisons endg. @ {SPAWN_SHIPS_FN:#x} pool {GARRISON_ENDGAME_POOL_ID}: {}",
        config.garrisons.endgame.summary()
    );
    log::info!(
        "HEE skirmish         @ {GENERATE_SHIP_NAMES_FN:#x} / {SKIRMISH_FOR_SLOT_FN:#x}: {}",
        config.skirmish.summary()
    );
    log::info!(
        "HEE tarkhan tier0/1/2 @ {ADD_REINFORCING_SHIPS_FN:#x}: {} / {} / {}",
        config.tarkhans.tier0.summary(),
        config.tarkhans.tier1.summary(),
        config.tarkhans.tier2.summary()
    );
}
