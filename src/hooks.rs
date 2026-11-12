//! Native detours for all 8 HighFleet 1.163 ship-pool sites.
//!
//! Each detour is installed **only** when its corresponding `escadra.json`
//! section has edits, after verifying the exact 1.163 instruction bytes at the
//! hook site. Every FFI callback catches panics (`catch_unwind`) so Rust never
//! unwinds across a native frame.
//!
//! Hook ownership & byte proofs:
//! 1. `0x140203b80` (`somethingSpawnShips` entry, 7B): garrisons (pool 0,
//!    campaign vs endgame), mercenary cities (pool 2), endgame free ships
//!    (pool 4), and Tactical Air/Missile packages (`chanceA` / `chanceB`).
//! 2. `0x1402077c0` (`Endgame_BuildEscadra` entry, 7B): roaming & endgame
//!    Strike Groups (`role = 5`).
//! 3. `0x140262a19` (`spawnEndgameShips?` phase-3 rider, 15B → `0x140262a28`):
//!    outer endgame Strike Group missile-carrier rider.
//! 4. `0x14028f695` (`restOfEventsPrologueAntIsaiah` Tarkhan switch, 7B →
//!    `0x14028f783`): named Tarkhan personal ships (`NPC.m_ship_first`).
//!    **Bugfix (v0.3.0):** `patchy-rs` `preserved_call` only saves `XMM0..XMM5`
//!    while Win64 ABI treats `XMM6..XMM15` as callee-saved (`restOfEventsPrologueAntIsaiah`
//!    holds `0.0f` in `XMM15`, `1500.0f` in `XMM6`, abs-mask in `XMM7` across the
//!    loop!). Calling `detour_assign_campaign_tarkhan_ship` as a normal Rust
//!    `extern "C"` function guarantees the Rust compiler saves/restores every
//!    `XMM6..XMM15` register it touches, preventing `XMM` loop-invariant corruption!
//! 5. `0x140238e70` (`endgameTarkhanShipNames?` entry, 5B): Khiva finale
//!    Tarkhan flagship groups `1..4` (`sret` ABI: initializes raw `[RCX]` via
//!    `0x140015390` and returns `RCX` in `RAX`).
//! 6. `0x14021b96f` (`addReinforceingShips` tier dispatch, 8B → `0x14021bd59`):
//!    Tarkhan call-in reinforcement ships (`tier 0` and `tier 1`).
//! 7. `0x140208660` (`spawnTradeConvoy?` entry, 7B): prize trade ships +
//!    fuel-endurance convoy escorts (`role = 1`).
//! 8. `0x140206950` (`Fleet_BuildShopOffer` entry, 7B): Shipworks `TEST`
//!    combat opponent selector (`SMALL` / `MID` / `LARGE` / `CARRIER`).

use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use patchy::{PatchSession, ProcessModule, ReturnType, Trampoline};

use crate::native::{
    assign_existing_escadra_string, clear_game_string_vec, create_escadra_from_ships,
    destroy_game_string_vec, hge_rand_float, hge_rand_int, init_uninitialized_escadra_string_cstr,
    is_endwar, is_far_from_ur_or_hard, is_hard_mode, lookup_design_convoy_stats,
    lookup_design_shipworks_stats, lookup_design_strength, mark_first_matching_ship_as_prize,
    push_game_string, RawEscadraString, RawStringVec, Vec2, NPC_SHIP_FIRST_OFFSET,
};
use crate::pool_math::{
    classify_shipworks_bucket, convoy_can_support_escort, roll_spawn_ships_indices,
    roll_strikegroup_indices, PoolEntry, ResolvedPools, ShipworksBucket,
};

static RESOLVED_POOLS: OnceLock<ResolvedPools> = OnceLock::new();
static LOGGED_HOOK_PANIC: AtomicBool = AtomicBool::new(false);

const SPAWN_SHIPS_RVA: usize = 0x203b80;
const SPAWN_SHIPS_STOLEN: usize = 7;
const SPAWN_SHIPS_GUARD: [u8; 16] = [
    0x48, 0x8b, 0xc4, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18, 0x48, 0x89, 0x78, 0x20, 0x55,
];

const STRIKEGROUPS_RVA: usize = 0x2077c0;
const STRIKEGROUPS_STOLEN: usize = 7;
const STRIKEGROUPS_GUARD: [u8; 16] = [
    0x48, 0x8b, 0xc4, 0x48, 0x89, 0x58, 0x08, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56,
];

const SG_RIDER_RVA: usize = 0x262a19;
const SG_RIDER_RESUME_RVA: usize = 0x262a28;
const SG_RIDER_STOLEN: usize = 15;
const SG_RIDER_GUARD: [u8; 15] = [
    0x48, 0x8d, 0x0d, 0x40, 0xae, 0x10, 0x00, 0x66, 0x0f, 0x2f, 0xf1, 0x48, 0x0f, 0x47, 0xcf,
];

const TARKHAN_SWITCH_RVA: usize = 0x28f695;
const TARKHAN_SWITCH_RESUME_RVA: usize = 0x28f783;
const TARKHAN_SWITCH_STOLEN: usize = 7;
const TARKHAN_SWITCH_GUARD: [u8; 25] = [
    0x48, 0x8b, 0x0d, 0x44, 0x23, 0x78, 0x03, 0x48, 0x8b, 0x01, 0xba, 0x01, 0x00, 0x00, 0x00, 0x44,
    0x8d, 0x42, 0x0a, 0xff, 0x90, 0x08, 0x01, 0x00, 0x00,
];

const TARKHAN_KHIVA_RVA: usize = 0x238e70;
const TARKHAN_KHIVA_STOLEN: usize = 5;
const TARKHAN_KHIVA_GUARD: [u8; 16] = [
    0x48, 0x89, 0x5c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48, 0x89, 0x7c, 0x24, 0x20, 0x55,
];

const REINFORCE_TIER_RVA: usize = 0x21b96f;
const REINFORCE_TIER_RESUME_RVA: usize = 0x21bd59;
const REINFORCE_TIER_STOLEN: usize = 8;
const REINFORCE_TIER_GUARD: [u8; 16] = [
    0x85, 0xdb, 0x0f, 0x84, 0x80, 0x08, 0x00, 0x00, 0x83, 0xeb, 0x01, 0x0f, 0x84, 0x50, 0x02, 0x00,
];

const TRADE_CONVOY_RVA: usize = 0x208660;
const TRADE_CONVOY_STOLEN: usize = 7;
const TRADE_CONVOY_GUARD: [u8; 16] = [
    0x48, 0x8b, 0xc4, 0x48, 0x89, 0x58, 0x08, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56,
];

const SHIPWORKS_TEST_RVA: usize = 0x206950;
const SHIPWORKS_TEST_STOLEN: usize = 7;
const SHIPWORKS_TEST_GUARD: [u8; 16] = [
    0x48, 0x8b, 0xc4, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18, 0x48, 0x89, 0x78, 0x20, 0x55,
];

fn report_hook_panic(site: &str) {
    if !LOGGED_HOOK_PANIC.swap(true, Ordering::Relaxed) {
        log::error!("hf_hee: caught panic in {site} detour; falling back safely");
    }
}

unsafe fn verify_site_bytes(
    module: ProcessModule,
    rva: usize,
    expected: &[u8],
    label: &str,
) -> Result<usize, String> {
    let address = module.resolve_rva(rva).map_err(|e| e.to_string())?;
    // SAFETY: `module` is the verified Steam 1.163 executable in `.text`.
    let found = std::slice::from_raw_parts(address as *const u8, expected.len());
    if found != expected {
        return Err(format!(
            "{label} @ {address:#x} has unexpected bytes {found:02x?}"
        ));
    }
    Ok(address)
}

/// Installs detours for every configured pool section into one atomic `PatchSession`.
pub unsafe fn install(pools: ResolvedPools) -> Result<usize, String> {
    if !pools.has_any_edits() {
        log::info!("hf_hee: all pools are vanilla; no detours needed");
        return Ok(0);
    }

    let module = ProcessModule::main().map_err(|e| e.to_string())?;
    let mut session = PatchSession::new();
    let mut installed_count = 0usize;

    if pools.spawn_ships_edited {
        let addr = verify_site_bytes(
            module,
            SPAWN_SHIPS_RVA,
            &SPAWN_SHIPS_GUARD,
            "somethingSpawnShips",
        )?;
        let mut tramp = Trampoline::new();
        tramp.absolute_jump(detour_spawn_ships as *const () as usize);
        session
            .detour_trampoline(addr, SPAWN_SHIPS_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.sg_edited {
        let addr = verify_site_bytes(
            module,
            STRIKEGROUPS_RVA,
            &STRIKEGROUPS_GUARD,
            "Endgame_BuildEscadra",
        )?;
        let mut tramp = Trampoline::new();
        tramp.absolute_jump(detour_build_strikegroup as *const () as usize);
        session
            .detour_trampoline(addr, STRIKEGROUPS_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.sg_rider_edited {
        let addr = verify_site_bytes(
            module,
            SG_RIDER_RVA,
            &SG_RIDER_GUARD,
            "spawnEndgameShips phase-3 rider",
        )?;
        let resume = module
            .resolve_rva(SG_RIDER_RESUME_RVA)
            .map_err(|e| e.to_string())?;
        let mut tramp = Trampoline::new();
        // `XMM0` holds `RandFloat(0.0, 1.0)` from `0x140262a0c`.
        // Pass `XMM0` as `arg1` (`XMM0` on Win64) and take `RAX` -> `RCX`.
        tramp.preserved_call(detour_pick_sg_rider as *const (), &[], &[], ReturnType::Rax);
        tramp.bytes(&[0x48, 0x89, 0xc1]); // MOV RCX, RAX
        tramp.absolute_jump(resume);
        session
            .detour_trampoline(addr, SG_RIDER_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.tarkhan_campaign_edited {
        let addr = verify_site_bytes(
            module,
            TARKHAN_SWITCH_RVA,
            &TARKHAN_SWITCH_GUARD,
            "Tarkhan campaign ship switch",
        )?;
        let resume = module
            .resolve_rva(TARKHAN_SWITCH_RESUME_RVA)
            .map_err(|e| e.to_string())?;
        let mut tramp = Trampoline::new();
        // At `0x14028f695` (right after `CALL 0x1400056e0` at `0x14028f690`):
        // - `RSP` is 16-byte aligned (`14028de70` pushed 5 GPRs + `SUB RSP, 0x180`,
        //   plus 8B return address => `RSP % 16 == 0` at every call site in the body!).
        // - Every volatile register (`RAX, RCX, RDX, R8..R11, XMM0..XMM5`) is dead
        //   between `0x14028f695` and `0x14028f783` (`MOV R14, [RSP+0x60]; MOV RCX, [RSP+0x48]`)!
        // - Non-volatile registers (`R13 = NPC*`, `RDI`, `R12`, `RBP`, `XMM6`, `XMM7`, `XMM15`)
        //   are automatically preserved by `extern "C" fn detour_assign_campaign_tarkhan_ship`
        //   and by `0x1400056e0` (`CopyString`), using the caller's existing 32-byte shadow space
        //   without shifting `RSP` into `[RSP+0x48..0x60]` locals!
        tramp.bytes(&[
            0x4c, 0x89, 0xe9, // MOV RCX, R13 (`0x4c` = REX.WR so reg=R13, r/m=RCX!)
            0x48, 0x83, 0xec,
            0x20, // SUB RSP, 0x20 (dedicated Win64 shadow space, 16B aligned)
        ]);
        tramp.absolute_call(detour_assign_campaign_tarkhan_ship as *const ());
        tramp.bytes(&[
            0x48, 0x83, 0xc4, 0x20, // ADD RSP, 0x20
        ]);
        tramp.absolute_jump(resume);
        session
            .detour_trampoline(addr, TARKHAN_SWITCH_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.tarkhan_khiva_edited {
        let addr = verify_site_bytes(
            module,
            TARKHAN_KHIVA_RVA,
            &TARKHAN_KHIVA_GUARD,
            "endgameTarkhanShipNames",
        )?;
        let mut tramp = Trampoline::new();
        tramp.absolute_jump(detour_endgame_tarkhan_ship as *const () as usize);
        session
            .detour_trampoline(addr, TARKHAN_KHIVA_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.reinforcements_edited {
        let addr = verify_site_bytes(
            module,
            REINFORCE_TIER_RVA,
            &REINFORCE_TIER_GUARD,
            "addReinforceingShips tier dispatch",
        )?;
        let resume = module
            .resolve_rva(REINFORCE_TIER_RESUME_RVA)
            .map_err(|e| e.to_string())?;
        let mut tramp = Trampoline::new();
        // At `0x14021b96f`:
        // - `EBX` = `tier` (`0`, `1`, `2`)
        // - `[RSP + 0x60]` = `local_178` (`CVec<EscadraString>`)
        // - All volatile registers are dead; `XMM6..XMM8` are reloaded right after `0x14021bd59`.
        // Using a direct `SUB RSP, 0x20` + `absolute_call` keeps stack offsets crystal clear
        // (`[RSP + 0x80]` = caller's `[RSP + 0x60]`) and preserves Win64 non-volatiles!
        tramp.bytes(&[
            0x48, 0x8d, 0x4c, 0x24, 0x60, // LEA RCX, [RSP + 0x60]
            0x89, 0xda, // MOV EDX, EBX
            0x48, 0x83, 0xec, 0x20, // SUB RSP, 0x20
        ]);
        tramp.absolute_call(detour_populate_reinforcements as *const ());
        tramp.bytes(&[
            0x48, 0x83, 0xc4, 0x20, // ADD RSP, 0x20
            0x4c, 0x8b, 0x74, 0x24, 0x68, // MOV R14, [RSP + 0x68] (`local_178.end`)
            0x48, 0x8b, 0x7c, 0x24, 0x70, // MOV RDI, [RSP + 0x70] (`local_178.capacity_end`)
        ]);
        tramp.absolute_jump(resume);
        session
            .detour_trampoline(addr, REINFORCE_TIER_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.traders_edited {
        let addr = verify_site_bytes(
            module,
            TRADE_CONVOY_RVA,
            &TRADE_CONVOY_GUARD,
            "spawnTradeConvoy",
        )?;
        let mut tramp = Trampoline::new();
        tramp.absolute_jump(detour_spawn_trade_convoy as *const () as usize);
        session
            .detour_trampoline(addr, TRADE_CONVOY_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    if pools.shipworks_edited {
        let addr = verify_site_bytes(
            module,
            SHIPWORKS_TEST_RVA,
            &SHIPWORKS_TEST_GUARD,
            "Fleet_BuildShopOffer",
        )?;
        let mut tramp = Trampoline::new();
        tramp.absolute_jump(detour_shipworks_test_offer as *const () as usize);
        session
            .detour_trampoline(addr, SHIPWORKS_TEST_STOLEN, tramp)
            .map_err(|e| e.to_string())?;
        installed_count += 1;
    }

    let _ = RESOLVED_POOLS.set(pools);
    session.install_permanently().map_err(|e| e.to_string())?;
    Ok(installed_count)
}

// ---------------------------------------------------------------------------
// 1. `somethingSpawnShips` (`0x140203b80`)
//    ABI: `(RCX = out_vec, XMM1 = budget, R8D = pool_id, XMM3 = chance_a,
//           [RSP+0x28] = chance_b, [RSP+0x30] = any_size)`
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_spawn_ships(
    out_vec: *mut RawStringVec,
    budget: f32,
    pool_id: i32,
    chance_a: f32,
    chance_b: f32,
    any_size: u8,
) {
    if catch_unwind(AssertUnwindSafe(|| {
        run_spawn_ships(out_vec, budget, pool_id, chance_a, chance_b, any_size != 0);
    }))
    .is_err()
    {
        report_hook_panic("somethingSpawnShips");
    }
}

unsafe fn run_spawn_ships(
    out_vec: *mut RawStringVec,
    budget: f32,
    pool_id: i32,
    chance_a: f32,
    chance_b: f32,
    any_size: bool,
) {
    if out_vec.is_null() {
        return;
    }
    clear_game_string_vec(out_vec);

    let Some(pools) = RESOLVED_POOLS.get() else {
        return;
    };

    let entries: &[PoolEntry] = match pool_id {
        0 => {
            if is_endwar() {
                &pools.garrisons_endgame
            } else {
                &pools.garrisons_campaign
            }
        }
        1 => &pools.pool_1_dead,
        2 => &pools.mercs_cities_campaign,
        3 => &pools.pool_3_dead,
        4 => &pools.mercs_cities_endgame,
        _ => &[],
    };

    let prices: Vec<f32> = entries
        .iter()
        .map(|entry| lookup_design_strength(entry.c_name.as_ptr()))
        .collect();

    let picked = roll_spawn_ships_indices(&prices, budget, any_size, false, |len| {
        hge_rand_int(0, (len - 1) as i32) as usize
    });

    for idx in picked {
        push_game_string(out_vec, entries[idx].c_name.as_ptr());
    }

    // Post-roll Aircraft Tactical package (`chanceA`, `0x140205925`):
    if hge_rand_float(0.0, 1.0) < chance_a && !pools.tactical_air.is_empty() {
        let pick = hge_rand_int(0, (pools.tactical_air.len() - 1) as i32) as usize;
        push_game_string(out_vec, pools.tactical_air[pick].c_name.as_ptr());
    }

    // Post-roll Missile Tactical package (`chanceB`, `0x14020596d`):
    if hge_rand_float(0.0, 1.0) < chance_b {
        append_tactical_missile_package(out_vec, pools);
    }
}

unsafe fn append_tactical_missile_package(out_vec: *mut RawStringVec, pools: &ResolvedPools) {
    if !pools.tactical_missile_edited {
        // Exact vanilla `chanceB` sequence (`0x140205976`..`0x140205a4c`):
        push_game_string(out_vec, c"Tarantul".as_ptr());
        push_game_string(out_vec, c"Hound".as_ptr());
        if hge_rand_float(0.0, 1.0) > 0.5 {
            push_game_string(out_vec, c"Eagle".as_ptr());
        }
        if hge_rand_float(0.0, 1.0) > 0.5 {
            push_game_string(out_vec, c"Tarantul MRL".as_ptr());
        }
        return;
    }

    let ships = &pools.tactical_missile_ships;
    if !ships.is_empty() {
        let first = hge_rand_int(0, (ships.len() - 1) as i32) as usize;
        push_game_string(out_vec, ships[first].c_name.as_ptr());

        if ships.len() >= 2 && hge_rand_float(0.0, 1.0) > 0.5 {
            let extra_a = hge_rand_int(0, (ships.len() - 1) as i32) as usize;
            push_game_string(out_vec, ships[extra_a].c_name.as_ptr());
        }
        if ships.len() >= 3 && hge_rand_float(0.0, 1.0) > 0.5 {
            let extra_b = hge_rand_int(0, (ships.len() - 1) as i32) as usize;
            push_game_string(out_vec, ships[extra_b].c_name.as_ptr());
        }
    }

    let escorts = &pools.tactical_missile_escorts;
    if !escorts.is_empty() {
        let esc = hge_rand_int(0, (escorts.len() - 1) as i32) as usize;
        push_game_string(out_vec, escorts[esc].c_name.as_ptr());
    }
}

// ---------------------------------------------------------------------------
// 2. `Endgame_BuildEscadra` (`0x1402077c0`)
//    ABI: `(XMM0 = budget, XMM1 = variant, R8 = spawn_pos, R9 = escadra_name,
//           [RSP+0x28] = typhon_flag)` -> `RAX = Escadra*`
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_build_strikegroup(
    budget: f32,
    variant: f32,
    spawn_pos: *const Vec2,
    escadra_name: *const c_char,
    typhon_flag: u8,
) -> usize {
    match catch_unwind(AssertUnwindSafe(|| {
        run_build_strikegroup(budget, variant, spawn_pos, escadra_name, typhon_flag != 0)
    })) {
        Ok(escadra) => escadra,
        Err(_) => {
            report_hook_panic("Endgame_BuildEscadra");
            0
        }
    }
}

unsafe fn run_build_strikegroup(
    budget: f32,
    variant: f32,
    spawn_pos: *const Vec2,
    escadra_name: *const c_char,
    typhon_flag: bool,
) -> usize {
    let Some(pools) = RESOLVED_POOLS.get() else {
        return 0;
    };

    let hard_mode = is_hard_mode();
    let far_or_hard = is_far_from_ur_or_hard(spawn_pos);

    let flagships = if far_or_hard {
        &pools.sg_flagships_hard
    } else {
        &pools.sg_flagships_normal
    };
    let fill_ships = if far_or_hard {
        &pools.sg_ships_hard
    } else {
        &pools.sg_ships_normal
    };

    let flagship_prices: Vec<f32> = flagships
        .iter()
        .map(|e| lookup_design_strength(e.c_name.as_ptr()))
        .collect();
    let fill_prices: Vec<f32> = fill_ships
        .iter()
        .map(|e| lookup_design_strength(e.c_name.as_ptr()))
        .collect();

    let (chosen_flagship, chosen_fill) =
        roll_strikegroup_indices(&flagship_prices, &fill_prices, budget, |len| {
            hge_rand_int(0, (len - 1) as i32) as usize
        });

    let mut names_vec = RawStringVec::zeroed();
    if let Some(flag_idx) = chosen_flagship {
        push_game_string(&mut names_vec, flagships[flag_idx].c_name.as_ptr());
    }
    for fill_idx in chosen_fill {
        push_game_string(&mut names_vec, fill_ships[fill_idx].c_name.as_ptr());
    }

    let extra_prob = if hard_mode { 0.50 } else { 0.25 };

    if far_or_hard && !pools.sg_extra_missile.is_empty() && hge_rand_float(0.0, 1.0) < extra_prob {
        let idx = hge_rand_int(0, (pools.sg_extra_missile.len() - 1) as i32) as usize;
        push_game_string(&mut names_vec, pools.sg_extra_missile[idx].c_name.as_ptr());
    }

    if !pools.sg_extra_carriers.is_empty() && hge_rand_float(0.0, 1.0) < extra_prob {
        let idx = hge_rand_int(0, (pools.sg_extra_carriers.len() - 1) as i32) as usize;
        push_game_string(&mut names_vec, pools.sg_extra_carriers[idx].c_name.as_ptr());
    }

    if typhon_flag && !pools.sg_extra_khiva.is_empty() {
        let idx = hge_rand_int(0, (pools.sg_extra_khiva.len() - 1) as i32) as usize;
        push_game_string(&mut names_vec, pools.sg_extra_khiva[idx].c_name.as_ptr());
    }

    let escadra = create_escadra_from_ships(&mut names_vec, variant, 5, spawn_pos, escadra_name, 0);
    destroy_game_string_vec(&mut names_vec);
    escadra
}

// ---------------------------------------------------------------------------
// 3. `spawnEndgameShips?` phase-3 rider (`0x140262a19` -> `0x140262a28`)
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_pick_sg_rider(roll_0_to_1: f32) -> *const c_char {
    match catch_unwind(AssertUnwindSafe(|| pick_sg_rider_inner(roll_0_to_1))) {
        Ok(ptr) => ptr,
        Err(_) => {
            report_hook_panic("sg_endgame_rider");
            c"Tarantul".as_ptr()
        }
    }
}

unsafe fn pick_sg_rider_inner(roll_0_to_1: f32) -> *const c_char {
    let Some(pools) = RESOLVED_POOLS.get() else {
        return if (roll_0_to_1 as f64) < 0.66 {
            c"Tarantul".as_ptr()
        } else {
            c"Tarantul ARM".as_ptr()
        };
    };
    let list = &pools.sg_endgame_rider;
    if list.is_empty() {
        return c"".as_ptr();
    }
    let count = list.len();
    let idx = ((roll_0_to_1.clamp(0.0, 0.999_999) * (count as f32)) as usize).min(count - 1);
    list[idx].c_name.as_ptr()
}

// ---------------------------------------------------------------------------
// 4. Campaign Tarkhan ship switch (`0x14028f695` -> `0x14028f783`)
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_assign_campaign_tarkhan_ship(npc_ptr: usize) {
    if catch_unwind(AssertUnwindSafe(|| {
        run_assign_campaign_tarkhan_ship(npc_ptr);
    }))
    .is_err()
    {
        report_hook_panic("tarkhans.campaign");
    }
}

unsafe fn run_assign_campaign_tarkhan_ship(npc_ptr: usize) {
    if npc_ptr == 0 {
        return;
    }
    let Some(pools) = RESOLVED_POOLS.get() else {
        return;
    };
    let list = &pools.tarkhans_campaign;
    if list.is_empty() {
        return;
    }
    let idx = hge_rand_int(0, (list.len() - 1) as i32) as usize;
    let ship_first_slot = (npc_ptr + NPC_SHIP_FIRST_OFFSET) as *mut RawEscadraString;
    assign_existing_escadra_string(ship_first_slot, &list[idx].name);
}

// ---------------------------------------------------------------------------
// 5. `endgameTarkhanShipNames?` (`0x140238e70`)
//    C++ `sret` ABI (`0x140239992` / `0x1402399d2`):
//      `(RCX = uninitialized out_name: *mut RawEscadraString, EDX = group: i32) -> RAX = out_name`
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_endgame_tarkhan_ship(
    out_name: *mut RawEscadraString,
    group: i32,
) -> *mut RawEscadraString {
    match catch_unwind(AssertUnwindSafe(|| {
        run_endgame_tarkhan_ship(out_name, group)
    })) {
        Ok(ret) => ret,
        Err(_) => {
            report_hook_panic("tarkhans.endgame_khiva");
            init_uninitialized_escadra_string_cstr(out_name, c"".as_ptr())
        }
    }
}

unsafe fn run_endgame_tarkhan_ship(
    out_name: *mut RawEscadraString,
    group: i32,
) -> *mut RawEscadraString {
    if out_name.is_null() {
        return out_name;
    }
    let Some(pools) = RESOLVED_POOLS.get() else {
        return init_uninitialized_escadra_string_cstr(out_name, c"".as_ptr());
    };
    let Some(list) = (1..=4)
        .contains(&group)
        .then(|| &pools.tarkhans_khiva_groups[(group - 1) as usize])
    else {
        return init_uninitialized_escadra_string_cstr(out_name, c"".as_ptr());
    };
    if list.is_empty() {
        return init_uninitialized_escadra_string_cstr(out_name, c"".as_ptr());
    }
    let idx = hge_rand_int(0, (list.len() - 1) as i32) as usize;
    init_uninitialized_escadra_string_cstr(out_name, list[idx].c_name.as_ptr())
}

// ---------------------------------------------------------------------------
// 6. `addReinforceingShips` tier dispatch (`0x14021b96f` -> `0x14021bd59`)
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_populate_reinforcements(vec_ptr: *mut RawStringVec, tier: i32) {
    if catch_unwind(AssertUnwindSafe(|| {
        run_populate_reinforcements(vec_ptr, tier);
    }))
    .is_err()
    {
        report_hook_panic("mercs.reinforcements");
    }
}

unsafe fn run_populate_reinforcements(vec_ptr: *mut RawStringVec, tier: i32) {
    if vec_ptr.is_null() {
        return;
    }
    let Some(pools) = RESOLVED_POOLS.get() else {
        return;
    };

    match tier {
        0 => {
            let list = &pools.reinforcements_tier0;
            if list.is_empty() {
                return;
            }
            let idx = hge_rand_int(0, (list.len() - 1) as i32) as usize;
            push_game_string(vec_ptr, list[idx].c_name.as_ptr());
        }
        1 | 2 => {
            let list = &pools.reinforcements_tier1;
            if list.is_empty() {
                return;
            }
            let idx = hge_rand_int(0, (list.len() - 1) as i32) as usize;
            let chosen = &list[idx];
            push_game_string(vec_ptr, chosen.c_name.as_ptr());
            if matches!(chosen.name.as_str(), "Navarin" | "Intrepid" | "Jaguar") {
                push_game_string(vec_ptr, chosen.c_name.as_ptr());
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// 7. `spawnTradeConvoy?` (`0x140208660`)
//    Verified caller ABI (`0x1402900c3` & `0x1400a9617`):
//      `XMM0` = unused (`30.0f`), `XMM1` = `variant` (`-1.0f`),
//      `R8`   = `spawn_pos: *const Vec2`, `R9` = `escadra_name: *const c_char`
//      -> `RAX = Escadra*`
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_spawn_trade_convoy(
    _unused_xmm0: f32,
    variant: f32,
    spawn_pos: *const Vec2,
    escadra_name: *const c_char,
) -> usize {
    match catch_unwind(AssertUnwindSafe(|| {
        run_spawn_trade_convoy(variant, spawn_pos, escadra_name)
    })) {
        Ok(escadra) => escadra,
        Err(_) => {
            report_hook_panic("spawnTradeConvoy");
            0
        }
    }
}

unsafe fn run_spawn_trade_convoy(
    variant: f32,
    spawn_pos: *const Vec2,
    escadra_name: *const c_char,
) -> usize {
    let Some(pools) = RESOLVED_POOLS.get() else {
        return 0;
    };
    if pools.traders_ships.is_empty() {
        return 0;
    }

    // 1. Pick 1 prize trade ship from `traders_ships`:
    let trader_idx = hge_rand_int(0, (pools.traders_ships.len() - 1) as i32) as usize;
    let chosen_trader = &pools.traders_ships[trader_idx];
    let Some(trader_stats) = lookup_design_convoy_stats(chosen_trader.c_name.as_ptr()) else {
        return 0;
    };

    let mut names_vec = RawStringVec::zeroed();
    let mut convoy_stats = Vec::with_capacity(6);
    push_game_string(&mut names_vec, chosen_trader.c_name.as_ptr());
    convoy_stats.push(trader_stats);

    let hard_mode = is_hard_mode();

    // 2. Hardmode heavy escort check (`0x14020886a`..`0x1402089ff`):
    //    In vanilla, `iVar8 > 1` (`75%` — i.e., trader is not Pegasus/Odissey)
    //    and `RandFloat(0, 1) > 0.75` (`25%`).
    let is_light_trader = matches!(chosen_trader.name.as_str(), "Pegasus" | "Odissey");
    if hard_mode
        && !is_light_trader
        && !pools.traders_escorts_hard.is_empty()
        && hge_rand_float(0.0, 1.0) > 0.75
    {
        let h_idx = hge_rand_int(0, (pools.traders_escorts_hard.len() - 1) as i32) as usize;
        let heavy = &pools.traders_escorts_hard[h_idx];
        if let Some(h_stats) = lookup_design_convoy_stats(heavy.c_name.as_ptr()) {
            push_game_string(&mut names_vec, heavy.c_name.as_ptr());
            convoy_stats.push(h_stats);
        }
    }

    // 3. Iterative fuel-endurance escort drafting (`0x140208fe2`..`0x140209300`):
    //    Cap is 2 escorts on Normal, 4 escorts on Hard; up to 100 attempts
    //    (clamped after first accepted escort to `1..=4` like vanilla's `local_104`
    //    so convoys stay balanced!).
    let mut escorts_left: i32 = if hard_mode { 4 } else { 2 };
    let clamp_tries: i32 = if is_light_trader {
        1
    } else {
        hge_rand_int(2, 4)
    };
    let mut tries_left: i32 = 100;

    if !pools.traders_escorts.is_empty() {
        while escorts_left > 0 && tries_left > 0 {
            let e_idx = hge_rand_int(0, (pools.traders_escorts.len() - 1) as i32) as usize;
            let cand_entry = &pools.traders_escorts[e_idx];
            if let Some(cand_stats) = lookup_design_convoy_stats(cand_entry.c_name.as_ptr()) {
                if convoy_can_support_escort(&convoy_stats, cand_stats) {
                    push_game_string(&mut names_vec, cand_entry.c_name.as_ptr());
                    convoy_stats.push(cand_stats);
                    escorts_left -= 1;
                    if tries_left > clamp_tries {
                        tries_left = clamp_tries;
                    }
                }
            }
            tries_left -= 1;
        }
    }

    // 4. Instantiate `Escadra` with `role = 1` (Trade Convoy) and ensure custom
    //    trade hulls get `m_prize = 1`, `m_playable = 0` (`+0xc19`, `+0xad0`)!
    let escadra = create_escadra_from_ships(&mut names_vec, variant, 1, spawn_pos, escadra_name, 0);
    destroy_game_string_vec(&mut names_vec);

    if escadra != 0 {
        mark_first_matching_ship_as_prize(escadra, &chosen_trader.name);
    }
    escadra
}

// ---------------------------------------------------------------------------
// 8. `Fleet_BuildShopOffer` — Shipworks `TEST` combat (`0x140206950`)
//    Verified caller ABI (`EDC_TEST_FIGHT` at `0x14013df2e`):
//      `RCX` = `out_vec: *mut RawStringVec`,
//      `XMM1` = `budget: f32`,
//      `R8B` = `allow_small: u8`,
//      `R9B` = `allow_mid: u8`,
//      `[RSP+0x20]` = `allow_large: u8`,
//      `[RSP+0x28]` = `allow_carrier: u8`
// ---------------------------------------------------------------------------

unsafe extern "C" fn detour_shipworks_test_offer(
    out_vec: *mut RawStringVec,
    budget: f32,
    allow_small: u8,
    allow_mid: u8,
    allow_large: u8,
    allow_carrier: u8,
) {
    if catch_unwind(AssertUnwindSafe(|| {
        run_shipworks_test_offer(
            out_vec,
            budget,
            allow_small != 0,
            allow_mid != 0,
            allow_large != 0,
            allow_carrier != 0,
        );
    }))
    .is_err()
    {
        report_hook_panic("Fleet_BuildShopOffer");
    }
}

unsafe fn run_shipworks_test_offer(
    out_vec: *mut RawStringVec,
    budget: f32,
    allow_small: bool,
    allow_mid: bool,
    allow_large: bool,
    allow_carrier: bool,
) {
    if out_vec.is_null() {
        return;
    }
    clear_game_string_vec(out_vec);

    let Some(pools) = RESOLVED_POOLS.get() else {
        return;
    };

    let mut candidates: Vec<&PoolEntry> = Vec::new();
    let mut prices: Vec<f32> = Vec::new();

    if allow_small {
        for entry in &pools.shipworks_small {
            candidates.push(entry);
            prices.push(lookup_design_strength(entry.c_name.as_ptr()));
        }
    }
    if allow_mid {
        for entry in &pools.shipworks_mid {
            candidates.push(entry);
            prices.push(lookup_design_strength(entry.c_name.as_ptr()));
        }
    }
    if allow_large {
        for entry in &pools.shipworks_large {
            candidates.push(entry);
            prices.push(lookup_design_strength(entry.c_name.as_ptr()));
        }
    }
    if allow_carrier {
        for entry in &pools.shipworks_carrier {
            candidates.push(entry);
            prices.push(lookup_design_strength(entry.c_name.as_ptr()));
        }
    }

    // Auto-classify flat `shipworks.add` (and optional `garrisons.include_in_shipworks`)
    // entries into the 4 checkboxes using live `LIB_SHIPS` stats!
    for entry in &pools.shipworks_auto_adds {
        let Some(stats) = lookup_design_shipworks_stats(entry.c_name.as_ptr()) else {
            continue;
        };
        let bucket = classify_shipworks_bucket(stats.combat_value, stats.has_planes);
        let enabled = match bucket {
            ShipworksBucket::Small => allow_small,
            ShipworksBucket::Mid => allow_mid,
            ShipworksBucket::Large => allow_large,
            ShipworksBucket::Carrier => allow_carrier,
        };
        if enabled {
            candidates.push(entry);
            prices.push(stats.combat_value);
        }
    }

    // Exact `Fleet_SelectShipsForBudget` (`0x140206060`) roll, including its
    // `0x140206892` fallback (`fallback_random_if_empty = true`) when the tested
    // player ship's budget is smaller than every candidate hull in the checked boxes!
    let picked = roll_spawn_ships_indices(&prices, budget, false, true, |len| {
        hge_rand_int(0, (len - 1) as i32) as usize
    });

    for idx in picked {
        push_game_string(out_vec, candidates[idx].c_name.as_ptr());
    }
}
