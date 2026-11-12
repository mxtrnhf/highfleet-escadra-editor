//! Map from `escadra.json` categories to HighFleet 1.163 pool code
//! (image base `0x140000000`; constants below are RVAs).
//!
//! Full reverse-engineering evidence:
//! - `hf_notes/docs/findings/strategy/enemy-ship-pools.md`
//! - `hf_notes/docs/findings/strategy/mercenary-ships.md`
//!
//! Every ship name is resolved through the global `LIB_SHIPS` library node
//! (`0x143a13a70`), populated at startup by `PreLoadLibraries_` (`0x233710`)
//! from `Objects/Designs/<Name>.seria`. Budget-rolled pools read each design's
//! price/strength scalar at `design+0x15f8`.

use crate::config::Config;

/// `Endgame_BuildEscadra` (`0x2077c0`): Strike Group composer (`role = 5`, or
/// `6` for the Endgame Khiva quadrant wave).
pub const STRIKEGROUPS_FN: usize = 0x2077c0;
/// `spawnEndgameShips?` phase-3 rider (`0x262a19`): appends one missile carrier
/// (`66% Tarantul`, `34% Tarantul ARM`) to each phase-2 outer Strike Group.
pub const STRIKEGROUPS_ENDGAME_RIDER_RVA: usize = 0x262a19;

/// `somethingSpawnShips` (`0x203b80`): strength-budget pool roller + post-roll
/// Tactical Group packages.
pub const SPAWN_SHIPS_FN: usize = 0x203b80;
/// `somethingSpawnShips` `chanceA` branch (`0x205925`): Aircraft Tactical Group.
pub const TACTICAL_AIR_BRANCH_RVA: usize = 0x205925;
/// `somethingSpawnShips` `chanceB` branch (`0x20596d`): Missile Tactical Group.
pub const TACTICAL_MISSILE_BRANCH_RVA: usize = 0x20596d;

/// `somethingSpawnShips` pool `0`: Campaign City Garrisons.
pub const GARRISON_POOL_ID: u32 = 0;
/// `somethingSpawnShips` pool `2`: Paid `FOR_HIRE` Mercenary City stock.
pub const MERC_CITIES_CAMPAIGN_POOL_ID: u32 = 2;
/// `somethingSpawnShips` pool `4`: Free `FOR_JOIN` Endgame City stock.
pub const MERC_CITIES_ENDGAME_POOL_ID: u32 = 4;

/// `restOfEventsPrologueAntIsaiah` Tarkhan ship switch (`0x28f695`).
pub const TARKHAN_CAMPAIGN_SWITCH_RVA: usize = 0x28f695;
/// `endgameTarkhanShipNames?` (`0x238e70`): Khiva finale Tarkhan flagships.
pub const TARKHAN_ENDGAME_KHIVA_FN: usize = 0x238e70;

/// `addReinforceingShips` tier dispatch (`0x21b96f`).
pub const ADD_REINFORCING_SHIPS_RVA: usize = 0x21b96f;

/// `spawnTradeConvoy?` (`0x208660`): Trade Convoy composer (`role = 1`).
pub const TRADE_CONVOY_FN: usize = 0x208660;

/// `Fleet_BuildShopOffer` (`0x206950`): Shipworks `TEST` combat opponent selector
/// (`EDC_TEST_FIGHT` at `0x13df2e`, checkboxes `SMALL`/`MID`/`LARGE`/`CARRIER`).
pub const SHIPWORKS_TEST_FN: usize = 0x206950;

// --- Strike Groups (`Endgame_BuildEscadra` `0x2077c0`) ---

pub const VANILLA_SG_FLAGSHIPS_NORMAL: &[&str] = &["Borey"];
pub const VANILLA_SG_FLAGSHIPS_HARD: &[&str] = &["Nomad Mk2", "Nomad", "Gryphon"];
pub const VANILLA_SG_SHIPS_NORMAL: &[&str] = &["Kormoran", "Negev", "Borey"];
pub const VANILLA_SG_SHIPS_HARD: &[&str] = &["Gryphon", "Kormoran", "Negev", "Borey"];
pub const VANILLA_SG_EXTRA_MISSILE: &[&str] = &["Tarantul ARM"];
pub const VANILLA_SG_EXTRA_CARRIERS: &[&str] = &["Wasp", "Longbow"];
pub const VANILLA_SG_EXTRA_KHIVA: &[&str] = &["Typhon"];
/// Vanilla `spawnEndgameShips?` phase-3 rider (`66% Tarantul`, `34% Tarantul ARM` —
/// modelled as 2:1 `["Tarantul", "Tarantul", "Tarantul ARM"]` so `"remove": ["Tarantul"]`
/// drops both copies while preserving ~67%/33% weights by default).
pub const VANILLA_SG_ENDGAME_RIDER: &[&str] = &["Tarantul", "Tarantul", "Tarantul ARM"];

// --- Tactical Groups (`somethingSpawnShips` `chanceA` / `chanceB`) ---

pub const VANILLA_TACTICAL_AIR: &[&str] = &["Longbow"];
pub const VANILLA_TACTICAL_MISSILE_SHIPS: &[&str] = &["Tarantul", "Eagle", "Tarantul MRL"];
pub const VANILLA_TACTICAL_MISSILE_ESCORTS: &[&str] = &["Hound"];

// --- City Garrisons (`somethingSpawnShips` pool `0`) ---

pub const VANILLA_GARRISONS: &[&str] = &[
    "Courageous",
    "Slogger",
    "Navarin",
    "Ballistic",
    "Sarma",
    "Nimrod",
    "Flower",
    "Intrepid",
    "Paladin Mk2",
    "Gladiator",
    "Gladiator MRL",
    "Fearsome",
    "Archangel",
];

pub const VANILLA_DEAD_POOL_1: &[&str] = &["Kormoran", "Negev", "Nomad", "Nomad Mk2"];
pub const VANILLA_DEAD_POOL_3: &[&str] = &[
    "Mockingbird",
    "Locust",
    "Skylark",
    "Rook",
    "Yars",
    "Meteor Mk3",
    "Lightning",
    "Lightning Mk3",
    "Fenek",
    "Fenek Mk2",
    "Fatman",
    "Flower",
    "Navarin",
    "Scarab",
    "Intrepid",
    "Paladin Mk2",
    "Gladiator",
    "Fearsome",
    "Gepard",
    "Voskhod",
    "Voskhod X2",
    "Wasp",
    "Wanderer",
];

// --- Tarkhan Ships (`0x28f695` & `0x238e70`) ---

pub const VANILLA_TARKHANS_CAMPAIGN: &[&str] = &[
    "Intrepid Mk2",
    "Jaguar Mk2",
    "Paladin Mk2",
    "Gladiator",
    "Gepard",
    "Gepard Mk2",
    "Archangel",
    "Wasp",
    "Longbow",
    "Kormoran",
    "Negev",
];

pub const VANILLA_TARKHANS_KHIVA_GROUP1: &[&str] = &[
    "Skylark",
    "Tanker",
    "Fatman",
    "Intrepid Mk2",
    "Intrepid Mk3",
    "Wanderer Mk2",
    "Jaguar Mk2",
    "Paladin Mk2",
    "Gladiator",
    "Fearsome",
    "Gepard Mk2",
    "Slingshot",
    "Wasp",
    "Voskhod",
];
pub const VANILLA_TARKHANS_KHIVA_GROUP2: &[&str] = &[
    "Archangel",
    "Egida Mk2",
    "Triumphant",
    "Longbow",
    "Kormoran",
    "Negev",
];
pub const VANILLA_TARKHANS_KHIVA_GROUP3: &[&str] = &["Triumphant", "Longbow", "Kormoran", "Negev"];
pub const VANILLA_TARKHANS_KHIVA_GROUP4: &[&str] = &["Vega", "Nomad", "Nomad Mk2"];

// --- Mercenary / Hireable City Ships & Reinforcements ---

/// Vanilla `somethingSpawnShips` pool `2` (28 pushes; `"Fearsome"` twice).
pub const VANILLA_MERCS_CITIES_CAMPAIGN: &[&str] = &[
    "Yars",
    "Yars Mk2",
    "Mockingbird",
    "Skylark",
    "Tanker",
    "Rook",
    "Lightning",
    "Fenek",
    "Navarin",
    "Scarab",
    "Meteor Mk3",
    "Fatman",
    "Flower",
    "Intrepid",
    "Fearsome",
    "Jaguar",
    "Paladin Mk2",
    "Gladiator",
    "Fearsome",
    "Gepard",
    "Archangel",
    "Kormoran",
    "Negev",
    "Voskhod",
    "Wasp",
    "Longbow",
    "Egida Mk2",
    "Triumphant",
];

/// Vanilla `somethingSpawnShips` pool `4` (14 designs; endgame `FOR_JOIN`).
pub const VANILLA_MERCS_CITIES_ENDGAME: &[&str] = &[
    "Tanker",
    "Fenek",
    "Scarab",
    "Fatman",
    "Intrepid",
    "Paladin Mk2",
    "Gladiator",
    "Fearsome",
    "Gepard",
    "Archangel",
    "Kormoran",
    "Negev",
    "Voskhod",
    "Wasp",
];

/// Vanilla `addReinforceingShips` tier `1` (`ASK_REINFORCE` / `DIPDIALOG_WIN`).
pub const VANILLA_MERCS_REINFORCEMENTS_TIER1: &[&str] = &[
    "Navarin",
    "Intrepid",
    "Jaguar",
    "Voskhod X2",
    "Triumphant",
    "Longbow",
    "Vega",
    "Kormoran",
];

/// Vanilla `addReinforceingShips` tier `0` (Khiva finale escort fallback).
pub const VANILLA_MERCS_REINFORCEMENTS_TIER0: &[&str] =
    &["Navarin", "Scarab", "Meteor Mk3", "Fatman", "Lightning Mk2"];

// --- Trade Convoys (`spawnTradeConvoy?` `0x208660`) ---

/// Vanilla prize trade ships with exact 12.5% / 12.5% / 25% / 25% / 25% weights
/// (`Pegasus` x1, `Odissey` x1, `Kastor` x2, `Pollux` x2, `Kadesh Mk1` x2).
/// Removing e.g. `"Kastor"` drops all matching entries cleanly.
pub const VANILLA_TRADERS_SHIPS: &[&str] = &[
    "Pegasus",
    "Odissey",
    "Kastor",
    "Kastor",
    "Pollux",
    "Pollux",
    "Kadesh Mk1",
    "Kadesh Mk1",
];

/// Vanilla Trade Convoy standard escorts (9 designs, gated by convoy fuel endurance).
pub const VANILLA_TRADERS_ESCORTS: &[&str] = &[
    "Intrepid",
    "Scarab",
    "Ballistic",
    "Talos",
    "Slogger",
    "Lightning",
    "Meteor Mk3",
    "Meteor Mk2",
    "Courageous",
];

/// Vanilla Trade Convoy hardmode heavy escorts (`25%` roll when `hardmode != 0`).
pub const VANILLA_TRADERS_ESCORTS_HARD: &[&str] = &["Kormoran", "Negev"];

// --- Shipworks TEST Combat (`Fleet_BuildShopOffer` `0x206950`) ---

pub const VANILLA_SHIPWORKS_SMALL: &[&str] = &[
    "Courageous",
    "Slogger",
    "Navarin",
    "Ballistic",
    "Sarma",
    "Flower",
];

pub const VANILLA_SHIPWORKS_MID: &[&str] = &[
    "Intrepid",
    "Nimrod",
    "Paladin Mk2",
    "Gladiator",
    "Gladiator MRL",
    "Fearsome",
    "Archangel",
];

pub const VANILLA_SHIPWORKS_LARGE: &[&str] = &[
    "Archangel",
    "Gryphon",
    "Borey",
    "Kormoran",
    "Negev",
    "Nomad",
    "Nomad Mk2",
    "Varyag",
];

pub const VANILLA_SHIPWORKS_CARRIER: &[&str] = &["Wasp", "Longbow"];

/// Log one summary line per top-level pool at startup.
pub fn log_summary(config: &Config) {
    log::info!(
        "HEE strikegroups     @ {STRIKEGROUPS_FN:#x}: {}",
        config.strikegroups.summary(),
    );
    log::info!(
        "HEE tactical_air     @ {TACTICAL_AIR_BRANCH_RVA:#x}: {}",
        config.tactical_air.summary(),
    );
    log::info!(
        "HEE tactical_missile @ {TACTICAL_MISSILE_BRANCH_RVA:#x} / {STRIKEGROUPS_ENDGAME_RIDER_RVA:#x}: {}",
        config.tactical_missile.summary(),
    );
    log::info!(
        "HEE garrisons        @ {SPAWN_SHIPS_FN:#x} pool {GARRISON_POOL_ID}: {}",
        config.garrisons.summary(),
    );
    log::info!(
        "HEE tarkhans         @ {TARKHAN_CAMPAIGN_SWITCH_RVA:#x} / {TARKHAN_ENDGAME_KHIVA_FN:#x}: {}",
        config.tarkhans.summary(),
    );
    log::info!(
        "HEE mercs            @ {SPAWN_SHIPS_FN:#x} pools {MERC_CITIES_CAMPAIGN_POOL_ID}/{MERC_CITIES_ENDGAME_POOL_ID} & {ADD_REINFORCING_SHIPS_RVA:#x}: {}",
        config.mercs.summary(),
    );
    log::info!(
        "HEE traders          @ {TRADE_CONVOY_FN:#x}: {}",
        config.traders.summary(),
    );
    log::info!(
        "HEE shipworks        @ {SHIPWORKS_TEST_FN:#x}: {}",
        config.shipworks.summary(),
    );
}
