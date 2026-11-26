# hf_hee — Highfleet Escadra Editor (HEE)

HEE lets you edit every hard-coded ship spawn pool in HighFleet (**Steam 1.163 only**) from one file: `Modloader/config/escadra.json`.

HEE installs the hooks it needs at startup. A hook shared by several pools can still run when another pool is edited.

---

## Using custom ships (without touching `Objects/Designs/`)

**Do not move custom `.seria` files into `Objects/Designs/`.** HighFleet treats `Objects/Designs/` as read-only "Original Designs" and hides or locks them in Shipworks until unlocked in a campaign save.

Instead, leave your ships in **`Ships/`** (where Shipworks saves them) or put them in **`Modloader/ships/`**:

1. Vanilla ships in `Objects/Designs/` always work by bare name in `add`, `remove`, and `replace` (`"remove": ["Courageous"]` removes a vanilla ship from that pool—`ship_dirs` is only for custom ships).
2. HEE searches `"ship_dirs"` in order (`["Modloader/ships", "Ships"]` by default) for any ship name not already in vanilla `Objects/Designs/`.
3. Custom designs are loaded into a **private HEE cache**—never into the game's `LIB_SHIPS` list—so Shipworks continues to treat them as normal, editable player designs.
4. HEE checks file modification times before spawning, so if you edit a ship in Shipworks and immediately click `TEST`, HEE reloads the updated `.seria` without restarting the game.

---

## Quick start (`Modloader/config/escadra.json`)

Every pool supports three operations, applied in order:

1. **`"replace"`** *(optional)* — replaces the **entire** pool with your list (it is *not* an old-for-new pair list).
2. **`"remove"`** — drops matching ship names from the pool.
3. **`"add"`** — appends ship names to the pool (listing a ship twice doubles its draw weight).

```json
{
  "enabled": true,
  "ship_dirs": ["Modloader/ships", "Ships"],
  "groups": {
    "my_escorts": ["MyCorvette", "MyFrigate"]
  },
  "garrisons": {
    "remove": ["Courageous"],
    "add": ["@my_escorts"]
  },
  "shipworks": {
    "add": ["Modloader/ships/test_targets/*"]
  }
}
```

---

## Wildcards & reusable `@groups`

To avoid copying long ship lists across multiple pools:

* **Named groups (`"groups"`):** define `"my_fleet": ["ShipA", "ShipB"]` at the top level and reference `"@my_fleet"` inside any `add`, `remove`, or `replace` array (nested `@groups` are supported).
* **Folder wildcards (`*`, `?`):** e.g. `"Modloader/ships/garrisons/*"` or `"Ships/enemy_*"` loads all matching `.seria` files from that folder (sorted alphabetically) and automatically registers that folder for lookup.
* **Name wildcards:**
  * In `add` / `replace`: `"MyCorvette_*"` matches `.seria` files across `ship_dirs` and `Objects/Designs/`.
  * In `remove`: `"Gladiator*"` drops every matching ship from the current pool (`Gladiator`, `Gladiator MRL`, `Gladiator LANCE`).

---

## The 8 spawn pools

| Pool | What it controls | Optional sub-fields |
|---|---|---|
| `strikegroups` | Roaming & endgame Strike Groups | `flagships_normal`, `flagships_hard`, `fill_normal`, `fill_hard`, `missile_escorts`, `carrier_escorts`, `khiva_extra`, `endgame_rider`, `count`, `budget`, `sprinkle`, `extra_escort` |
| `tactical_air` | City aircraft tactical groups & SG carrier escorts | `count`, `ship_count`, `extra_escort` |
| `tactical_missile` | City missile tactical groups & SG missile riders | `ships`, `escorts`, `sg_missile`, `endgame_rider`, `count`, `ship_count`, `extra_escort` |
| `garrisons` | City garrisons (campaign & Nuclear War) | `campaign`, `endgame`, `city_budget_multiplier`, `include_in_shipworks` *(bool, mirrors roster edits into `shipworks`)* |
| `tarkhans` | Named Tarkhan personal ships & Khiva finale flagships | `campaign`, `tier0`, `tier1`, `tier2`, `tier3`, `tier4` |
| `mercs` | Mercenary `FOR_HIRE`, Nuclear War `FOR_JOIN`, `ASK_REINFORCE` | `hire`, `city_budget_multiplier` (paid stock only), `endgame`, `reinforcements`, `reinforcements_tier0`, `extra_escort` (paid stock only) |
| `traders` | Trade Convoys *(flat `add`/`remove` edits escorts safely)* | `ships` *(prize hulls, auto-marked capturable `m_prize = 1`)*, `escorts`, `escorts_hard`, `count` |
| `shipworks` | Shipworks `TEST` combat opponents *(flat `add` auto-classifies by stats)* | `small`, `mid`, `large`, `carrier` |

Where a roster sub-pool supports top-level inheritance, its own `replace`/`remove`/`add` overrides the category's edits. `extra_escort` is always a separate pool, not a roster override.

---

## Extra escorts (`extra_escort`)

Use `extra_escort` to add hulls **on top of** a normal roster, without putting them into its draft pool or charging them against its budget. For example, this adds up to three Hounds to a city Air Tactical package without putting Hounds into the carrier pool:

```json
{
  "tactical_air": {
    "extra_escort": { "add": ["Hound"], "count": 3, "chance": 0.1 }
  }
}
```

* `add`: a list of ship designs. Bare names, custom designs, `@groups`, and wildcards work as in other `add` lists. Missing designs are skipped. Repeating a name increases its draw weight.
* `count`: a JSON integer from `0` to `32`, **not a string**. This is the number of attempts per eligible spawn, not a group count.
* `chance`: a JSON number from `0` to `1` **per attempt**. In this example the game rolls three times at 10% each, for zero to three escorts. Each success draws one ship from the same list, so ships can repeat. Set `chance` to `1` to guarantee each attempt if at least one design is available.

Omit the field, use zero attempts or chance, or leave `add` empty to disable it. A malformed entry is logged and ignored without resetting the rest of the config.

Put `extra_escort` directly under one of these sections:

| Parent section | When it adds ships |
|---|---|
| `tactical_air` | Only when a city Air Tactical package adds its base ship(s). Does not change the carrier pool, `ship_count`, or Strike Group carrier escorts. |
| `tactical_missile` | Only when a city Missile Tactical package adds its base ship(s). Does not change `ship_count` or Strike Group missile riders. |
| `strikegroups` | Once for each spawned roaming, Khiva assault, or outer Nuclear War Strike Group. Vanilla escorts and the late missile rider are separate. |
| `mercs` | Once per paid Mercenary city stock roll. Keeps the normal zero to five offers, then adds the successful picks as extra `FOR_HIRE` stock, even if the normal budget is zero. Put it under `mercs`, not under `hire`. Free `FOR_JOIN` stock and Tarkhan reinforcements are unchanged. |

These settings only affect future generation. They do not add ships to rosters already stored in a save.

---

## Group counts (`count`), spawn math & `strikegroups.sprinkle`

`traders`, `strikegroups`, `tactical_air`, and `tactical_missile` accept an optional `"count"` string. `N` must be an **integer** (`"*2.5"` is rejected and logged):

* `"N"` — set exact value (`"10"` -> 10, `"0"` -> disable)
* `"*N"` — multiply the vanilla value by `N` (`"*2"` -> double)
* `"-N"` — subtract `N` (clamped at `0`)
* `"+N"` — add `N`

### How each category's `count` & spawn math works

* **`traders` (vanilla `30` campaign convoys):** sets the exact convoy-loop count at campaign start (`0` skips the loop; quest convoy `CROW50` is separate). Each convoy picks 1 prize ship (`traders.ships`) plus fuel-gated escorts (`2` normal / `4` hard max).
* **`strikegroups` (vanilla `6` campaign / up to `4` Khiva assault / `3` outer Nuclear War):** applies the same op to **all three** Strike Group waves using each wave's own baseline (e.g. `"+2"` -> `8` roaming, up to `6` Khiva capped by the 4 quadrant vectors, `5` outer Nuclear War). `"0"` skips all three waves.
  * *Strike Group composition:* picks the **first** flagship in `flagships_*` with `combat_value <= budget * 0.5` (falling back to the last entry), then fills up to `1.2 * budget` (10 consecutive-reject cap), then rolls `25%` normal / `50%` hard for a carrier escort and (when hard mode or `>= 5000` from `"UR"`) a missile escort.
  * **`strikegroups.sprinkle`** *(default `true`)* spreads campaign Strike Groups across uniformly random eligible cities instead of vanilla's first-eligible-city scan (which piles high `count` values onto a single city). Set `"sprinkle": false` for vanilla behavior.
* **`tactical_air` & `tactical_missile` (per-city spawn probability, not an integer count):** tactical groups are not separate fleets at campaign start—they are appended onto Pool `0` city garrisons when `RandFloat(0, 1) < chance` (vanilla `0.15` normal / `0.20` hard, and `0.0` within `5000` of `"UR"` on normal).
  * `"count"` applies to that per-city probability (Pool `0` garrisons only; mercenary pools pass `0.0` and are never affected).
  * `"*2"` doubles each city's chance (`0.15` -> `0.30`); `"0"` sets it to `0.0` (disables that tactical group type); `"1"` or `"+1"` raises it to `>= 1.0` so **every** eligible garrison spawns one.
  * *Why `"-1"` disables them:* `0.15 - 1 = -0.85`, clamped to `0.0`. Because `N` is an integer, `"-N"` on a `0.15`/`0.20` chance always clamps to `0.0`.
  * *Near `"UR"` on normal:* vanilla forces `chance = 0.0` within `5000` of `"UR"`, so `"*2"` still yields `0.0` near `"UR"`, whereas `"1"` or `"+1"` overrides it to `1.0` everywhere.
* **Garrisons & Mercs budget roll:** rolls ships up to `1.2 * budget` (10 attempts, 10 consecutive-reject limit, preferring `>= 3` ships within `[0.9 * budget, 1.2 * budget]`); tactical packages are appended *after* the budget roll.

All changes affect **new campaigns and future spawns**—ships already saved in an existing `profile.seria` are not rewritten.

---

## Tactical package size (`ship_count`)

`tactical_air` and `tactical_missile` accept `"ship_count": [min, max]`. The roll includes both endpoints; equal bounds such as `[4, 4]` give a fixed size. Both values must be positive 32-bit JSON integers, with `min` no greater than `max`. Malformed ranges are logged and ignored.

```json
{
  "tactical_air": { "ship_count": [1, 2] },
  "tactical_missile": { "ship_count": [2, 4] }
}
```

This controls the initial city package only, not the base garrison, Strike Group escorts, or which ships detach later. `count` still controls whether the package rolls at all. Designs can repeat. A missile package of at least two ships reserves one escort slot when both the missile and escort pools have usable designs.

### Vanilla group sizes

These are totals for one generated group or one mercenary city's stock, not the number of groups on the map. Tactical numbers apply only if a package actually spawns.

| Group | Vanilla size |
|---|---|
| Strike Groups | **No fixed ship-count cap.** One flagship, plus as many fill ships as the budget roll allows. Up to one carrier and one missile escort can be added. Khiva adds a Typhon; an outer Nuclear War group gets one late missile rider. |
| Air tactical package | **1** Longbow. |
| Missile tactical package | **2–4** ships: a Tarantul and a Hound, with independent 50% rolls for an Eagle and a Tarantul MRL. |
| City garrison | **No fixed ship-count cap.** The city budget determines the base roster. The draft prefers at least three ships, but can return two, one, or none. Tactical packages add ships on top. |
| Paid mercenary stock (`FOR_HIRE`) | **0–5** offers per mercenary city. The budget determines the draft; the game keeps at most five offers. |

### Combat-value targets (`budget`)

`strikegroups` accepts `"budget": N` to replace its roster draft target in all three waves. `N` is a JSON number from 0 to 10,000 in **combat value**, not credits or ships. The draft can finish about 20% over its target. A higher budget can yield more ships, stronger ships, or both; it doesn't guarantee a ship count. Omit `budget` to keep the vanilla targets: **150–350** for campaign roaming groups, **100** for Khiva, and **200 normal / 300 hard** for outer Nuclear War groups.

The flagship is chosen even if its cost is above the target. Escorts, the Khiva hull, and the late missile rider aren't charged against the draft budget. `budget: 0` stops the fill draft but doesn't disable the fleet; use `count: "0"` to disable the waves.

### City budget scaling (`city_budget_multiplier`)

Use a nonnegative JSON number to multiply each city's own vanilla draft budget. This preserves differences in city size and the stronger budgets toward Khiva instead of setting all cities to the same target. Omit the multiplier or set it to `1` for vanilla. For example, `1.5` gives a city with a vanilla budget of 40 a budget of 60; a city with a budget of 80 gets 120. When scaling is enabled, HEE caps the result at 10,000 combat value before the draft's usual allowance. Invalid values are logged and ignored.

```json
{
  "garrisons": { "city_budget_multiplier": 1.5 },
  "mercs": { "hire": { "city_budget_multiplier": 1.25 } }
}
```

* **Garrisons:** scales only the base roster. A multiplier of `0` stops that draft but leaves the tactical package chances unchanged.
* **Paid mercenaries (`FOR_HIRE`):** scales only the normal paid stock, capped at **five standard offers** per mercenary city. A multiplier of `0` skips the normal draft but still allows `mercs.extra_escort` offers. Free `FOR_JOIN` ships and reinforcements are unaffected. `mercs.city_budget_multiplier` is shorthand; a valid `mercs.hire.city_budget_multiplier` takes precedence (`for_hire` is an alias for `hire`).

When upgrading from v0.8.0, replace any flat `budget` under `garrisons` or `mercs`/`mercs.hire`; those keys are now ignored. There is no direct conversion to a multiplier because the vanilla budget varies by city. Large multipliers can slow down generation and gameplay. See the [Steam 1.163 spawn-pool audit](https://github.com/mxtrnhf/hf_notes/blob/master/docs/findings/strategy/enemy-ship-pools.md) for the native draft and spawn rules.

---

## Example: Basic edited `escadra.json`

A minimal config using flat lists only—ideal when you want to tweak a few
garrisons, swap out a vanilla ship, and add your own `Ships/` designs to
Shipworks `TEST` without touching the rest of the game:

> **Note:** The examples below use `//` comments for readability. Real `escadra.json` files must be strict JSON (strip `//` comments when copying).

```jsonc
{
  "enabled": true,
  "ship_dirs": ["Modloader/ships", "Ships"],
  "strikegroups": {
    "remove": [],
    // Keep vanilla Strike Groups and add one custom cruiser to every SG pool.
    "add": ["MyCustomCruiser"]
  },
  // Leaving a pool's arrays empty keeps it 100% vanilla (unhooked).
  "tactical_air": {
    "remove": [],
    "add": []
  },
  "tactical_missile": {
    "remove": [],
    "add": []
  },
  "garrisons": {
    // Drop a vanilla ship you dislike, then append two custom designs from Ships/.
    "remove": ["Courageous"],
    "add": ["MyCustomCorvette", "MyCustomFrigate"]
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
    // Flat "add" auto-classifies each ship into SMALL/MID/LARGE/CARRIER by its stats.
    "add": ["MyCustomCorvette", "MyCustomFrigate", "MyCustomCruiser"]
  }
}
```

**What this does:**

* **`strikegroups`:** keeps all vanilla Strike Group ships and adds `MyCustomCruiser` (loaded from `Ships/MyCustomCruiser.seria`) to every Strike Group pool.
* **`garrisons`:** removes `Courageous` from city garrisons and adds `MyCustomCorvette` and `MyCustomFrigate`.
* **`shipworks`:** adds your three custom ships to Shipworks `TEST` combat, auto-classifying each into `SMALL` / `MID` / `LARGE` / `CARRIER` from its stats.
* **Empty pools (`tactical_air`, `tactical_missile`, `tarkhans`, `mercs`, `traders`):** completely unhooked and untouched.

---

## Example: Heavily edited `escadra.json` (total fleet overhaul)

A full `"this game is mine and I will play with MY ships"` overhaul using
`@groups`, folder & name wildcards, sub-pools, `replace`, `count`, and
`sprinkle`:

```jsonc
{
  "enabled": true,
  // Custom design folders searched in priority order before falling back to vanilla Objects/Designs/.
  "ship_dirs": [
    "Modloader/ships/my_faction",
    "Modloader/ships",
    "Ships"
  ],
  // Reusable named groups referenced as "@name" anywhere below.
  "groups": {
    "corvettes": ["Ships/my_corvettes/*"],                   // Folder glob: loads all .seria in Ships/my_corvettes/
    "frigates": ["Modloader/ships/my_faction/frigate_*"],   // Prefix glob inside a custom folder
    "cruisers": ["MyCruiser_A", "MyCruiser_B"],
    "carriers": ["MyCarrier_Light", "MyCarrier_Heavy"],
    "missile_ships": ["MyMissileBoat", "MyMissileCruiser"],
    "garrison_mix": ["@corvettes", "@frigates", "@cruisers"] // Nested @group composition
  },
  "strikegroups": {
    // Sub-pool overrides replace every vanilla Strike Group role with your own roster.
    "flagships_normal": { "replace": ["@cruisers"] },
    "flagships_hard": { "replace": ["MyCruiser_B", "MyDreadnought"] },
    "fill_normal": { "replace": ["@frigates", "@cruisers"] },
    "fill_hard": { "replace": ["@frigates", "@cruisers", "MyDreadnought"] },
    "missile_escorts": { "replace": ["@missile_ships"] },
    "carrier_escorts": { "replace": ["@carriers"] },
    "khiva_extra": { "replace": ["MySuperheavyFlagship"] },
    "endgame_rider": { "replace": ["@missile_ships"] },
    "count": "+2",     // +2 groups per wave (8 roaming, up to 4 Khiva, 5 outer Nuclear War)
    "sprinkle": true   // Scatter roaming SGs across random eligible cities
  },
  "tactical_air": {
    "replace": ["@carriers"],
    "count": "*2"      // Double per-city air TG spawn chance (0.15 -> 0.30 normal, 0.20 -> 0.40 hard)
  },
  "tactical_missile": {
    "ships": { "replace": ["@missile_ships"] },
    "escorts": { "replace": ["@corvettes"] },
    "sg_missile": { "replace": ["@missile_ships"] },
    "endgame_rider": { "replace": ["@missile_ships"] },
    "count": "*2"      // Double per-city missile TG spawn chance
  },
  "garrisons": {
    "campaign": {
      "replace": ["@garrison_mix"],
      "remove": ["*_proto"] // Wildcard remove: drop any prototype hulls from the expanded group
    },
    "endgame": {
      "replace": ["@frigates", "@cruisers", "MyDreadnought"]
    },
    "include_in_shipworks": false
  },
  "tarkhans": {
    "campaign": { "replace": ["@cruisers", "MyTarkhanFlagship"] },
    "tier0": { "replace": ["@corvettes"] },
    "tier1": { "replace": ["@frigates"] },
    "tier2": { "replace": ["@cruisers"] },
    "tier3": { "replace": ["@cruisers", "MyDreadnought"] },
    "tier4": { "replace": ["MyDreadnought", "MySuperheavyFlagship"] }
  },
  "mercs": {
    "hire": { "replace": ["@corvettes", "@frigates"] },           // FOR_HIRE city piers
    "endgame": { "replace": ["@cruisers", "@carriers"] },         // Nuclear War FOR_JOIN piers
    "reinforcements": { "replace": ["@frigates", "@cruisers"] },  // Tarkhan ASK_REINFORCE
    "reinforcements_tier0": { "replace": ["@corvettes"] }         // Tier 0 call-in escorts
  },
  "traders": {
    "ships": { "replace": ["MyFreighter_Small", "MyFreighter_Large"] }, // Prize hulls (m_prize = 1)
    "escorts": { "replace": ["@corvettes", "@frigates"] },
    "escorts_hard": { "replace": ["@cruisers"] },
    "count": "25"      // Spawn 25 campaign convoys instead of vanilla 30
  },
  "shipworks": {
    // Explicit TEST checkbox buckets instead of auto-classification.
    "small": { "replace": ["@corvettes"] },
    "mid": { "replace": ["@frigates"] },
    "large": { "replace": ["@cruisers", "MyDreadnought"] },
    "carrier": { "replace": ["@carriers"] }
  }
}
```

**What this does:**

* **`ship_dirs` & `groups`:** searches your custom faction folder first, expands folder wildcards (`Ships/my_corvettes/*`) and prefix wildcards (`frigate_*`), and composes `@garrison_mix` from nested `@groups`.
* **`strikegroups`:** replaces every vanilla Strike Group role (`flagships_*`, `fill_*`, escorts, Khiva superheavy, endgame rider) with your own roster, adds `+2` groups per wave (`8` roaming, up to `4` Khiva, `5` outer Nuclear War), and scatters roaming groups across random eligible cities (`"sprinkle": true`).
* **`tactical_air` & `tactical_missile`:** replaces vanilla `Longbow`/`Tarantul`/`Hound` tactical packages with your own carriers and missile boats, and doubles (`"*2"`) their per-city spawn chance.
* **`garrisons`:** replaces campaign and Nuclear War city garrisons with your fleet while filtering out any prototype designs matching `*_proto`.
* **`tarkhans` & `mercs`:** replaces every Tarkhan personal ship, Khiva finale tier (`tier1`–`tier4`), mercenary `FOR_HIRE` pier ship, Nuclear War `FOR_JOIN` ship, and `ASK_REINFORCE` call-in.
* **`traders`:** replaces unarmed prize merchant hulls (`ships`, auto-marked capturable `m_prize = 1`) and convoy escorts (`escorts` / `escorts_hard`), and sets total campaign convoys to `25`.
* **`shipworks`:** explicitly assigns each `TEST` checkbox (`SMALL` / `MID` / `LARGE` / `CARRIER`) to your own ships so you can test against your exact faction roster.

---

## Build & install

* Build: `cargo xwin build --release --target x86_64-pc-windows-msvc`
* Install: copy `target/x86_64-pc-windows-msvc/release/hf_hee.dll` to `<game dir>/Modloader/mods/hf_hee_v<version>.dll` (remove older `hf_hee_v*.dll`). Latest: `hf_hee_v0.10.0.dll`.
* Release packs: include `README.md` and `HEE_Manual.pdf` alongside the DLL so users have the `escadra.json` reference.
* Config: `Modloader/config/escadra.json` is auto-created on first run if missing. Existing configs load without changes.

---

## Changelog

* **v0.10.0**: Added opt-in `extra_escort` pools for city Air and Missile Tactical packages, Strike Groups, and paid Mercenary stock. Extras roll independently of the normal roster budget and slots.
* **v0.9.0**: Replaced flat city budgets with separate `city_budget_multiplier` settings for garrisons and paid `FOR_HIRE` stock, preserving vanilla city progression.
* **v0.8.0**: Replaced `ship_count` with `budget` for Strike Groups, garrisons, and paid mercenary stock. Tactical packages still use `ship_count`. Refactored roster drafting and documented vanilla sizes.
* **v0.7.0**: Added optional `ship_count` ranges and fixed sizes for Strike Groups, air and missile tactical packages, city garrisons, and paid mercenary stock.
* **v0.6.0**:
  * Custom `.seria` ships now load directly from `"ship_dirs"` (default `["Modloader/ships", "Ships"]`) into a private HEE cache without touching `LIB_SHIPS` (`0x143a13a70`), so Shipworks never hides or locks custom designs and edits reload automatically on `mtime` change.
  * Added `"groups"` (`"@group_name"`) and `*` / `?` wildcard expansion (both folder globs like `"Ships/my_pack/*"` and name patterns like `"Gladiator*"`).
  * Removed the unused `"version"` field from `escadra.json` (existing configs with `"version"` still load fine).
* **v0.5.2**: Fixed `strikegroups.count: "0"` still spawning 1 campaign Strike Group and fixed outer Nuclear War SG top-of-loop bound `0x1402626b9`.
* **v0.5.1**: `strikegroups.sprinkle` now defaults to `true`.
* **v0.5.0**: Added `strikegroups.sprinkle` (`0x14028fc27`).
* **v0.4.0**: Added `count` support to `traders`, `strikegroups`, `tactical_air`, and `tactical_missile`.
* **v0.3.1**: Fixed REX byte in campaign Tarkhan trampoline (`0x14028f695`).
* **v0.3.0**: Added `shipworks` and `traders` categories, nuanced sub-pools, and fixed campaign Tarkhan `XMM` preservation.

---

## Credits

* [logdot's HighFleet Modloader](https://github.com/logdot/Highfleet-Modloader) (`highfleet-mod-api`) and [`patchy-rs`](https://github.com/logdot/patchy-rs)
* [logdot's `highfleet-rs`](https://github.com/logdot/highfleet-rs)
