# hf_hee — Highfleet Escadra Editor

HEE makes every enemy ship pool in HighFleet (Steam 1.163 only) editable from
one config file, `Modloader/config/escadra.json`. The pools are hard-coded
inline in the executable — no data files — so this mod hooks the pool code.
Reverse-engineering evidence and addresses:
`hf_notes/docs/findings/strategy/enemy-ship-pools.md` (workspace notes repo).

**v0.1.0 is a scaffold**: it loads/creates the config and logs one summary
line per pool. No game behavior changes yet; the detours land in the next
version.

## Where the ships come from

Every name used in `escadra.json` (and every name the vanilla pools use) is
resolved at spawn time through the game's design library, the `LIB_SHIPS`
node (global `0x143a13a70`, created by `PreLoadLibraries_`, the seria loader
at `0x140233710`). That library is built from the files in
**`Objects/Designs/`**: one `<Name>.seria` per design. So:

* a name in `escadra.json` is valid exactly when
  `Objects/Designs/<Name>.seria` exists (vanilla designs ship there; a new
  design copied there becomes eligible immediately);
* the pool strength/price of a design is read from its record (`+0x15f8`)
  at spawn time — changing the design changes how the pool rolls it;
* `Ships/` is NOT the source: it holds the game-managed working copies
  (rewritten on version bumps, backed up to `Ships_backup/`). `HEE` never
  reads or writes ship files itself; it only passes names to the game.

## Config: `Modloader/config/escadra.json`

One sub-object per pool. Each has the same fields, applied in this order:
`replace` (optionally exact contents), `remove`, `add`, then the `extras_*`
trio for the pool's post-roll special ships. Missing keys are defaults;
unknown keys are ignored. With `seria` handling, an empty file is vanilla
behavior.

| Category | Game code (RVA, image base `0x140000000`) |
|---|---|
| `strikegroups` | `Endgame_BuildEscadra` `0x2077c0` — the roaming big fleets (campaign-start ring + endgame waves). Extras: `Tarantul ARM`, `Wasp`/`Longbow`, `Typhon`. |
| `tactical_air` / `tactical_missile` | `somethingSpawnShips` `0x203b80`, pool id 2 (from `Prologue_SpawnEventGroups` `0x2a7f60`, group 6): the **tarkhan/NPC fleets** — the escadra is created together with its tarkhan NPC record. Both specializations share one base pool: `add` lists merge, `remove` lists union. `Eagle` is the air extra; `Longbow`/`Tarantul MRL` the missile extras. |
| `garrisons.campaign` | pool id 0: the campaign-start fleets — settlement garrisons of the six generated towns, the Khiva quest fleet, and the tarkhan/event escadras linked into the event records (`+0x2b8`, strength `+0x248`). |
| `garrisons.endgame` | pool id 4 (city garrisons spawned by the endgame collapse). |
| `skirmish` | `Fleet_GenerateShipNames` `0x205ae0` — the 46-name, five-price-bucket catalogue behind the **skirmish escadras**: the escalating random fleets driven by the profile field `nextSkirmishEscadraLevel` (level presets in `Fleet_CreateRandomEscadraForSlot` `0x1c2ba0`, first fleet at level 1 at campaign start, level +1 per spawn). |
| `tarkhans.tier0/1/2` | `addReinforceingShips` `0x21b8c0` — the tables the game rolls one ship name from per call: **tier 0** the event/script path (`creatureLogic`), **tier 1** the city ASK_REINFORCE dialog (`Dialog_ExecuteChoiceActions`), **tier 2** an unused copy of the tier-1 list. The chosen ship is instantiated as a `NEW_SHIP` appended to the player's escadra, so these are the tarkhan/mercenary ships that come to your side. `extras_*` do not apply here — per-tier `add`/`remove` only. |

Terminology check (2026-11-12): the engine itself names the bucket-pool fleets
"skirmish escadras" (`nextSkirmishEscadraLevel`, serialized in the profile) —
they are the escalating hunting forces, *not* city garrisons.

Notes: pools 1 and 3 of `somethingSpawnShips` have no live call site in
1.163 and are not configurable. Trade convoys are a separate path
(`spawnTradeConvoy?` `0x209454`) with their own hard-coded values, not pools;
not covered.

## Vanilla pool contents (for building `remove`/`replace` lists)

* **garrisons.campaign**: Courageous, Slogger, Navarin, Sarma, Ballistic,
  Archangel, Fearsome, Flower, Gladiator, Gladiator MRL, Intrepid, Nimrod,
  Paladin Mk2
* **garrisons.endgame**: Archangel, Fatman, Fearsome, Fenek, Gepard,
  Gladiator, Intrepid, Kormoran, Negev, Paladin Mk2, Scarab, Tanker, Voskhod,
  Wasp
* **tactical (air and missile share)**: Yars, Yars Mk2, Mockingbird, Skylark,
  Tanker, Rook, Voskhod, Negev, Navarin, Intrepid, Fearsome, Paladin Mk2,
  Gladiator, Gepard, Kormoran, Longbow, Lightning, Jaguar, Fatman, Scarab,
  Wasp, Flower, Fenek, Egida Mk2, Archangel, Triumphant, Meteor Mk3
* **strikegroups**: Borey (seeded twice), Kormoran, Negev; flagged designs
  Nomad Mk2, Nomad, Gryphon (twice); extras Tarantul ARM, Wasp, Longbow,
  Typhon
* **skirmish** (46 entries, two duplicate Fearsome pushes
  de-duplicated): Courageous, Meteor Mk2, Rook, Meteor Mk3, Slogger,
  Lightning, Talos, Navarin Mk2, Flower, Navarin, Fenek, Ballistic,
  Lightning Mk2, Navarin Mk3, Intrepid, Fatman, Fury, Scarab, Lancer, Sarma,
  Jaguar RSZO, Jaguar, Fearsome, Intrepid Mk2, Jaguar Mk2, Egida, Eagle,
  Gladiator, Gladiator MRL, Gepard, Intrepid Mk3, Paladin Mk2, Paladin,
  Gladiator LANCE, Longbow, Kormoran, Archangel, Negev, Borey, Novorossiysk,
  Nomad, Nomad Mk2, Sevastopol
* **tarkhans.tier0** (event path): Navarin, Scarab, Meteor Mk3, Fatman,
  Lightning Mk2
* **tarkhans.tier1/2** (city dialog; tier 2 is the same list twice): Navarin,
  Intrepid, Jaguar, Voskhod X2, Triumphant, Longbow, Vega, Kormoran

## Build and install

* Build: `cargo xwin build --release` (the msvc target is set by
  `.cargo/config.toml`).
* Install: copy `target/x86_64-pc-windows-msvc/release/hf_hee.dll` to
  `<game dir>/Modloader/mods/hf_hee_v<version>.dll`. Remove any older
  `hf_hee_v*.dll` first.
* Config: `Modloader/config/escadra.json` is created with defaults on first
  run; while any `add`/`remove` list is empty and there is no `replace`,
  that pool stays vanilla.

## Credits

Built on [logdot's HighFleet Modloader](https://github.com/logdot/Highfleet-Modloader)
(`highfleet-mod-api`) for the mod entry points and logging.
