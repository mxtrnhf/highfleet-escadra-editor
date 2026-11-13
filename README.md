# hf_hee — Highfleet Escadra Editor

HEE makes every ship spawn pool in HighFleet (Steam 1.163 only) editable from
one config file, `Modloader/config/escadra.json`. All pools are hard-coded inline
in `.text` (no data files), so HEE installs byte-verified native detours only
for the pools you edit (leaving untouched pools completely unhooked).

Reverse-engineering evidence and exact call-site proofs:
* `hf_notes/docs/findings/strategy/enemy-ship-pools.md`
* `hf_notes/docs/findings/strategy/mercenary-ships.md`

## How to use `Modloader/config/escadra.json`

1. Put any custom `.seria` ship designs into **`Objects/Designs/`** (e.g.
   `Objects/Designs/MyCruiser.seria` -> `"MyCruiser"`).
2. Edit whichever of the **8 top-level pools** you want in `Modloader/config/escadra.json`:
   * `"add": ["MyCruiser"]` — adds to the vanilla pool (listing a ship twice
     doubles its draw weight).
   * `"remove": ["Slogger"]` — removes matching ships from the vanilla pool.
   * `"replace": ["MyCruiser", "Gladiator"]` *(optional)* — replaces the vanilla
     pool outright with your exact list.

```json
{
  "version": 6,
  "enabled": true,
  "strikegroups": {
    "remove": [],
    "add": []
  },
  "tactical_air": {
    "remove": [],
    "add": []
  },
  "tactical_missile": {
    "remove": [],
    "add": []
  },
  "garrisons": {
    "remove": [],
    "add": []
  },
  "tarkhans": {
    "remove": [],
    "add": []
  },
  "mercs": {
    "remove": [],
    "add": []
  },
  "traders": {
    "remove": [],
    "add": []
  },
  "shipworks": {
    "remove": [],
    "add": []
  }
}
```

| Pool | What it controls |
|---|---|
| `strikegroups` | **Roaming & Endgame Strike Groups** (`Endgame_BuildEscadra` `0x2077c0`): heavy cruisers, flagships, SG escorts, Khiva `Typhon`, and outer Nuclear War riders. |
| `tactical_air` | **Aircraft Tactical Groups**: carriers spawned in city tactical groups (`0x205925`, `Longbow`) and SG carrier escorts (`Wasp`, `Longbow`). |
| `tactical_missile` | **Missile Tactical Groups**: strategic-missile cruisers spawned in city tactical groups (`0x20596d`: `Tarantul`, `Eagle`, `Tarantul MRL`, escorted by `Hound`) plus SG missile riders (`Tarantul`, `Tarantul ARM`). |
| `garrisons` | **City Garrisons**: 13-ship base pool defending Khiva, the 6 HQ ring cities, and generated towns (`somethingSpawnShips` `0x203b80`, pool `0`). |
| `tarkhans` | **Tarkhan Ships**: personal ships of named Tarkhans in cities (`0x28f695`) and Khiva defense `ENDWAR_CARD` Tarkhan flagships (`0x238e70` groups `1..4` + `tier0` fallback). |
| `mercs` | **Mercenaries & Reinforcements**: paid `FOR_HIRE` pier ships in Mercenary cities (pool `2`), free `FOR_JOIN` pier ships during Nuclear War (pool `4`), and Tarkhan `ASK_REINFORCE` support ships (`0x21b96f`). |
| `traders` | **Trade Convoys** (`spawnTradeConvoy?` `0x208660`): convoy escorts (fuel-range drafted) and prize trade ships (`Pegasus`, `Odissey`, `Kastor`, `Pollux`, `Kadesh Mk1`). |
| `shipworks` | **Shipworks `TEST` Combat** (`Fleet_BuildShopOffer` `0x206950`): opponent pools for the `SMALL`, `MID`, `LARGE`, and `CARRIER` checkboxes in the Shipworks editor. |

---

## Optional extra fields (nuanced per-subpool control)

You can keep `escadra.json` completely flat (`add`/`remove`/`replace` on the 8
categories above), **or** add optional sub-pool objects inside any category when
you want finer control. When a sub-field is omitted, the category's top-level
`add`/`remove`/`replace` applies; when specified, that sub-field overrides it:

* **`strikegroups`**:
  * `flagships_normal` (`Borey`), `flagships_hard` (`Nomad Mk2, Nomad, Gryphon`)
  * `fill_normal` (`Kormoran, Negev, Borey`), `fill_hard` (`Gryphon, Kormoran, Negev, Borey`)
  * `missile_escorts` (`Tarantul ARM` — missile frigate/cruiser escort with SGs)
  * `carrier_escorts` (`Wasp, Longbow`), `khiva_extra` (`Typhon`), `endgame_rider` (`Tarantul, Tarantul ARM`)
* **`tactical_missile`**:
  * `ships` (`Tarantul, Eagle, Tarantul MRL`), `escorts` (`Hound`), `sg_missile` (`Tarantul ARM`), `endgame_rider` (`Tarantul, Tarantul ARM`)
* **`garrisons`**:
  * `campaign` — usual campaign city garrisons (`Profile.endwar == 0`)
  * `endgame` — late-game city spawns after Khiva defense / Nuclear War begins (`Profile.endwar != 0`; also used as fallback for `mercs.endgame` pool `4` unless `mercs.endgame` is set)
  * `include_in_shipworks` *(boolean, default `false`)* — automatically mirror `garrisons` edits into Shipworks `TEST` combat
* **`tarkhans`**:
  * `campaign` — named Tarkhan personal ships in cities (`FOR_TARKHAN`)
  * `tier0` — Khiva defense escort fallback (`Navarin, Scarab, Meteor Mk3, Fatman, Lightning Mk2`)
  * `tier1`..`tier4` — Khiva defense `ENDWAR_CARD` flagship loyalty tiers (`tier1` = 14 light/medium hulls, `tier2` = 6 heavy cruisers, `tier3` = `Triumphant, Longbow, Kormoran, Negev`, `tier4` = `Vega, Nomad, Nomad Mk2`)
* **`mercs`**:
  * `hire` (Mercenary city `FOR_HIRE` pool `2`), `endgame` (Nuclear War city `FOR_JOIN` pool `4`), `reinforcements` (`ASK_REINFORCE` tier `1`), `reinforcements_tier0`
* **`traders`**:
  * *(Top-level `traders.add` / `remove` edits convoy escorts safely so combat ships aren't accidentally turned into unarmed prizes.)*
  * `ships` — prize trade ships (`Pegasus`, `Odissey`, `Kastor` x2, `Pollux` x2, `Kadesh Mk1` x2). Custom designs added here are automatically marked `m_prize = 1, m_playable = 0` so they can be captured intact!
  * `escorts` — standard convoy escorts (`Intrepid, Scarab, Ballistic, Talos, Slogger, Lightning, Meteor Mk3, Meteor Mk2, Courageous`)
  * `escorts_hard` — Hard-mode heavy convoy escorts (`Kormoran, Negev`)
* **`shipworks`**:
  * *(Top-level `shipworks.add` auto-classifies custom ships into `SMALL`/`MID`/`LARGE`/`CARRIER` from their live `LIB_SHIPS` aircraft & combat-value stats!)*
  * `small`, `mid`, `large`, `carrier` — explicit per-checkbox overrides

### Example with nuanced sub-fields

```json
{
  "garrisons": {
    "campaign": { "remove": ["Courageous"], "add": ["MyCorvette"] },
    "endgame": { "add": ["MyHeavyCruiser"] },
    "include_in_shipworks": true
  },
  "strikegroups": {
    "add": ["MyBattlecruiser"],
    "missile_escorts": { "replace": ["MyMissileFrigate"] }
  },
  "tarkhans": {
    "campaign": { "add": ["MyTarkhanCruiser"] },
    "tier4": { "add": ["MySuperFlagship"] }
  },
  "traders": {
    "ships": { "add": ["MyMerchantman"] },
    "escorts": { "add": ["MyEscortFrigate"] }
  }
}
```

## Optional `count` field (how many groups exist in total)

Four categories take an extra optional string field, `count`, that adjusts **how
many groups of that type are spawned in total**:

| Category | Vanilla count | What `count` controls |
|---|---|---|
| `traders` | `30` convoys at campaign start (`MOV EDX, 0x1e` `0x14028fd63`) | The convoy spawn-loop argument itself |
| `strikegroups` | `6` roaming at campaign start (`0x14028fd4d`), up to `4` Khiva assault (`0x1402623de`), `3` outer Nuclear War (`0x1402629b1`) | Each strike-group spawn wave's loop bound |
| `tactical_air` | per-city `chanceA` (`0.15` normal / `0.20` hard, `0` near `"UR"`) | The chance is scaled, which controls how many air groups appear in total |
| `tactical_missile` | per-city `chanceB` (same scaling) | Same, for missile groups |

Format (operand `N` must be an integer):

* `"N"` — set: exactly `N` groups (e.g. `"10"` → 10 convoys)
* `"*N"` — multiply the vanilla value by `N` (e.g. `"*2"` → 60 convoys / double the tactical chance)
* `"-N"` — subtract `N`, clamped so the value can't go below `0` (e.g. `"-5"` → 25 convoys; on a `0.15` chance `"-1"` → `0`, i.e. no tactical groups)
* `"+N"` — add `N` (e.g. `"+3"` → 33 convoys; on a chance, `"1"` → every city gets one)

```json
{
  "strikegroups": { "add": ["MyBattlecruiser"], "count": "*2", "sprinkle": true },
  "traders": { "count": "-10" },
  "tactical_air": { "count": "0" },
  "tactical_missile": { "count": "+1" }
}
```

Notes:

* For `tactical_air` / `tactical_missile` the op applies to the per-city spawn
  chance inside `somethingSpawnShips` (pool `0` garrisons only — mercenary pools
  pass `0.0` and are never affected). `"0"` disables that tactical group type
  entirely; `"1"` makes every garrison roll it.
* `strikegroups.count` applies the same op to each strike-group wave with that
  wave's own vanilla count (`6` / `4` / `3`), so `"+2"` gives 8 roaming + up to 6
  Khiva + 5 outer groups.
* Invalid strings (non-integer `N`, empty) are logged and ignored (vanilla count).

### Sprinkling strike groups across cities (`strikegroups.sprinkle`)

Vanilla's campaign Strike Group loop picks its spawn city by scanning the city
list **in order** and taking the first eligible one (with a Y-spacing rule) —
with the vanilla count of 6 that yields ~6 different cities, but with a large
`count` (e.g. `"*50"`) nearly all groups land on the same first-eligible city
(observed in a real save: 295 of 300 SGs at city `ISA`).

`strikegroups.sprinkle` **defaults to `true`**: the pick point (`0x14028fc27`)
is hooked and each group chooses a **uniformly random eligible city** (same
eligibility filters as vanilla: `[loc+0x2e8] >= 2`, `[loc+0x248] != 0`), so
groups spread across the map like trade convoys do. Set `"sprinkle": false`
for vanilla behavior. Trade convoys and tactical groups never clustered in the
first place (convoys pick a uniform random city each; tactical groups attach to
their own garrison city), so no sprinkle flag exists for them.

## Changelog

* **v0.5.1**:
  * `strikegroups.sprinkle` now **defaults to `true`** (config schema `6`); set
    `"sprinkle": false` to restore vanilla's first-eligible-city stacking.
* **v0.5.0**:
  * Added `strikegroups.sprinkle` (bool, default `false`): hooks the campaign SG
    city pick at `0x14028fc27` and replaces the first-eligible city with a
    uniformly random eligible city per group, so large `strikegroups.count`
    values spread across the map instead of stacking on one city. Config
    schema version `5`.
* **v0.4.0**:
  * Added optional `count` string field to `traders`, `strikegroups`,
    `tactical_air` and `tactical_missile` (`"N"` set / `"*N"` multiply /
    `"-N"` subtract clamped at 0 / `"+N"` add). New byte-guarded detours at the
    convoy-count argument `0x14028fd63` (`MOV EDX, 0x1e`), the campaign SG loop
    bound `0x14028fd4d` (`CMP ESI, 0x6; JGE`), the Khiva assault wave entry
    `0x1402623de` (clamps the walked city vector's end), and the outer Nuclear
    War SG bound `0x1402629b1` (`CMP EAX, 0x3; JL`); tactical counts adjust the
    `chanceA`/`chanceB` arguments in the `somethingSpawnShips` detour (pool `0`
    only). Config schema version bumped to `4`.
* **v0.3.1**:
  * Fixed the REX byte in the campaign Tarkhan trampoline at `0x14028f695` (`0x49 0x89 0xe9` encoded `MOV R9, RBP` instead of `0x4c 0x89 0xe9` `MOV RCX, R13`, leaving `RCX` pointing into the `"ASSASSIN"` city-type string from `0x14028f690` when `CopyString` ran).
* **v0.3.0**:
  * Fixed campaign-start crash when editing `tarkhans`: `patchy-rs` `preserved_call` at `0x14028f695` clobbered Win64 non-volatile `XMM6`/`XMM7`/`XMM15` loop invariants in `restOfEventsPrologueAntIsaiah`, and `endgameTarkhanShipNames?` (`0x140238e70`) required C++ `sret` initialization (`0x140015390`) on raw stack memory.
  * Added `shipworks` category (`Fleet_BuildShopOffer` `0x140206950` for Shipworks `TEST` combat checkboxes, with automatic `SMALL`/`MID`/`LARGE`/`CARRIER` classification and optional `garrisons.include_in_shipworks`).
  * Added `traders` category (`spawnTradeConvoy?` `0x140208660` for convoy escorts and prize trade ships, including automatic `m_prize` capture tagging on custom trader hulls).
  * Added optional nuanced sub-fields across all categories (`tarkhans.tier0..tier4`, `strikegroups.missile_escorts`, `garrisons.campaign`/`endgame`, `mercs.hire`/`endgame`/`reinforcements`, `traders.ships`/`escorts`/`escorts_hard`, `shipworks.small`/`mid`/`large`/`carrier`) while keeping the flat JSON structure fully supported.

## Validation

* **v0.4.0 counts — verified in a real save** (`Saves/Profile_1/profile.seria`,
  Steam 1.163, saved 2026-11-13 with `"count": "*50"` on `traders` and
  `strikegroups`): the save contains exactly **1500 role-1 trade convoys**
  (vanilla 30 × 50) and **300 role-5 strike groups** (vanilla 6 × 50).
  Scan a save with [`../hf_dev_tools/save_scan.py`](../hf_dev_tools/README.md).
* **v0.4.0 clustering — verified in the same save**: without sprinkle, 295 of
  the 300 SGs sat at one city (`ISA`); convoys already spread (853 distinct
  positions over 1500).
* **v0.5.1 sprinkle — validated in game** (Steam 1.163, hf_hee v0.5.1,
  `strikegroups.count` `*10`, sprinkle default-on): strike groups spawn spread
  across cities instead of stacked.
* Not yet covered: the endgame Khiva-assault (`role 6`) and outer Nuclear War
  wave count hooks fire only when the endgame triggers.

## Build and install

* Build: `cargo xwin build --release --target x86_64-pc-windows-msvc`
* Install: copy `target/x86_64-pc-windows-msvc/release/hf_hee.dll` to
  `<game dir>/Modloader/mods/hf_hee_v<version>.dll` (remove older `hf_hee_v*.dll`).
  Latest: `hf_hee_v0.5.1.dll`.
* Config: `Modloader/config/escadra.json` is auto-created with defaults on
  first run.

## Credits

* [logdot's HighFleet Modloader](https://github.com/logdot/Highfleet-Modloader) (`highfleet-mod-api`) and [`patchy-rs`](https://github.com/logdot/patchy-rs) for mod entry points, logging, and executable detours.
* [logdot's `highfleet-rs`](https://github.com/logdot/highfleet-rs) for Steam 1.163 native layout definitions.
