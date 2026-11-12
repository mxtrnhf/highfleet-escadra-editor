//! Steam 1.163 native layout bindings and game-owned memory helpers.
//!
//! Uses `highfleet-rs` (`05c6207`) for `Profile`, `Creature`, `CVec`, `EscadraString`,
//! and `SentinelList` layout contracts, while strictly delegating string/vector
//! allocation and destruction to HighFleet's own CRT functions so game-owned
//! memory is never allocated or freed by Rust's allocator.
//!
//! Supporting RE findings:
//! - `hf_notes/docs/findings/strategy/enemy-ship-pools.md`
//! - `hf_notes/docs/findings/strategy/mercenary-ships.md`
//! - `hf_notes/docs/structures.md`

use std::ffi::{c_char, c_void};
use std::mem::{offset_of, size_of};
use windows_sys::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION};

pub use highfleet::general::{CVec, EscadraString, Node, SentinelList};
pub use highfleet::v1_163::{Creature, Profile};

/// Compile-time assertions pinning the Steam 1.163 native layout contract.
const _: () = {
    assert!(size_of::<EscadraString>() == 0x20);
    assert!(size_of::<CVec<usize>>() == 0x18);
    assert!(size_of::<SentinelList<usize>>() == 0x10);
    assert!(size_of::<Profile>() == 0x5e8);
    assert!(offset_of!(Profile, hardmode) == 0x165);
    assert!(offset_of!(Profile, endwar) == 0x2e9);
    assert!(size_of::<Creature>() == 0x1a50);
    assert!(offset_of!(Creature, planes) == 0x14c8);
    assert!(offset_of!(Creature, fuel) == 0x1638);
    assert!(offset_of!(Creature, fuel_capacity) == 0x1640);
};

/// `Profile::hardmode` byte (`+0x165`).
pub const PROFILE_HARDMODE_OFFSET: usize = offset_of!(Profile, hardmode);
/// `Profile::endwar` byte (`+0x2e9`, set to `1` when Nuclear War / Khiva defense begins).
pub const PROFILE_ENDWAR_OFFSET: usize = offset_of!(Profile, endwar);

/// `Node::children` (`SentinelList` head pointer at `+0x100`) and count (`+0x108`).
const NODE_CHILDREN_OFFSET: usize = offset_of!(Node, children);
/// `Link::target` (`+0x8` inside `Link`, or `owner` link on child `Body` at `+0xb8`).
const CHILD_DATA_OWNER_CREATURE_OFFSET: usize = 0xb8;

/// `Creature` fields uncovered by `highfleet-rs` (verified in `Ship_LoadSeriaAndMarkPrize`
/// `0x1401b0950..0x1401b0a24` and `spawnTradeConvoy?` `0x1402087de..0x140208803`):
pub const CREATURE_SHIP_NAME_OFFSET: usize = 0xa90;
pub const CREATURE_PLAYABLE_OFFSET: usize = 0xad0;
pub const CREATURE_PRIZE_OFFSET: usize = 0xc19;
pub const CREATURE_PLANES_VEC_OFFSET: usize = offset_of!(Creature, planes);
pub const CREATURE_COMBAT_VALUE_OFFSET: usize = 0x15f8;
pub const CREATURE_AIRSPEED_OFFSET: usize = 0x1614;
pub const CREATURE_FUEL_TOTAL_OFFSET: usize = offset_of!(Creature, fuel);
pub const CREATURE_FUEL_CAPACITY_OFFSET: usize = offset_of!(Creature, fuel_capacity);
pub const CREATURE_FUEL_NEED_CR_OFFSET: usize = 0x1650;

/// `Escadra` child-property list head (`Node::children` at `+0x100`).
const ESCADRA_CHILDREN_HEAD_OFFSET: usize = 0x100;

/// `Location` position (`Vec2` at `+0x150`, uncovered by `highfleet-rs`; see `structures.md`).
pub const LOCATION_POS_OFFSET: usize = 0x150;

/// `NPC::m_ship_first` (`EscadraString` at `+0xd0`, uncovered by `highfleet-rs`;
/// verified from `Npc_RegisterProperties` `0x14023fbd0`).
pub const NPC_SHIP_FIRST_OFFSET: usize = 0xd0;

/// Global `HGE` singleton pointer (`0x143a119e0`).
const HGE_GLOBAL: usize = 0x1_43a1_19e0;
/// Global `LIB_SHIPS` library `Node*` (`0x143a13a70`, loaded from `Objects/Designs/*.seria`).
const LIB_SHIPS_GLOBAL: usize = 0x1_43a1_3a70;
/// Virtual slot `+0xb8` on `Node`: `FindChildByName(this, cstr) -> Node*`.
const NODE_VTBL_FIND_CHILD_BY_NAME: usize = 0xb8;

/// `CopyString(dst, cstr, len)` (`0x1400056e0`): assigns `len` bytes into an
/// **already-initialized** `EscadraString` using the game's allocator.
const FN_COPY_STRING: usize = 0x1_4000_56e0;
/// `String_Tidy(str)` (`0x140006d70`): frees any heap buffer on an
/// `EscadraString` with the game's `free` and resets it to empty SSO.
const FN_STRING_TIDY: usize = 0x1_4000_6d70;
/// `EscadraName_AssignFromCStr(dst, cstr)` (`0x140015390`): initializes an
/// **uninitialized** 0x20-byte `EscadraString` slot (`len=0, cap=0xf`) and copies `cstr`.
const FN_STRING_INIT_FROM_CSTR: usize = 0x1_4001_5390;
/// `VectorString_MovePushBack(vec, str)` (`0x1400b8d30`): moves `str` into a
/// game-owned `CVec<EscadraString>`, growing via the game's allocator.
const FN_VEC_STRING_MOVE_PUSH: usize = 0x1_400b_8d30;
/// `VectorString_Clear(vec)` (`0x14015cd20`): destroys all elements in a
/// game-owned `CVec<EscadraString>` and sets `end = begin`.
const FN_VEC_STRING_CLEAR: usize = 0x1_4015_cd20;
/// `VectorString_Destroy(vec)` (`0x14005e580`): destroys all elements and frees
/// the backing buffer of a game-owned `CVec<EscadraString>`.
const FN_VEC_STRING_DESTROY: usize = 0x1_4005_e580;
/// `retCreature154c_(cstr)` (`0x140219980`): looks up `cstr` in `LIB_SHIPS`
/// (`0x143a13a70`) and returns `design+0x15f8` (`f32` price/strength scalar).
const FN_DESIGN_STRENGTH: usize = 0x1_4021_9980;
/// `GetPropertyLinkedObject(prop)` (`0x14021b070`): resolves an `Escadra` child
/// property node to its linked ship `Creature*`.
const FN_GET_PROPERTY_LINKED_OBJECT: usize = 0x1_4021_b070;
/// `Get_Profile_()` (`0x140292a90`): returns the active `Profile*`.
const FN_GET_PROFILE: usize = 0x1_4029_2a90;
/// `Profile_FindCreatureByName(profile, name)` (`0x140292ce0`): searches
/// `Profile::locations` (`+0x210`) by `EscadraString` name.
const FN_FIND_LOCATION_BY_NAME: usize = 0x1_4029_2ce0;
/// `validateFloatArray_(a, b)` (`0x14020f9a0`): returns Euclidean `SQRTSS`
/// distance between two `Vec2` points (`0x14020fb47`).
const FN_VEC2_DISTANCE: usize = 0x1_4020_f9a0;
/// `Fleet_CreateEscadraFromShips` (`0x1401c7060`).
const FN_CREATE_ESCADRA_FROM_SHIPS: usize = 0x1_401c_7060;
/// `HGE::RandFloat(this, min, max)` (`0x1401742d0`, vtable `+0x110`).
const FN_HGE_RAND_FLOAT: usize = 0x1_4017_42d0;
/// `HGE::RandInt(this, min, max)` (`0x140174320`, vtable `+0x108`, inclusive).
const FN_HGE_RAND_INT: usize = 0x1_4017_4320;

/// Raw 0x20-byte stack scratch buffer matching `EscadraString` without Rust `Drop`.
#[repr(C, align(8))]
pub struct RawEscadraString {
    bytes: [u8; size_of::<EscadraString>()],
}

/// Raw 0x18-byte stack scratch header matching `CVec<EscadraString>` without Rust `Drop`.
#[repr(C, align(8))]
pub struct RawStringVec {
    pub begin: usize,
    pub end: usize,
    pub capacity_end: usize,
}

impl RawStringVec {
    pub const fn zeroed() -> Self {
        Self {
            begin: 0,
            end: 0,
            capacity_end: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// Fuel & speed telemetry read from a `LIB_SHIPS` design `Creature`, matching
/// `spawnTradeConvoy?` (`0x1402087de..0x14020918d`).
#[derive(Clone, Copy, Debug)]
pub struct DesignConvoyStats {
    /// `m_tele_airspeed` (`+0x1614`, `f32`).
    pub airspeed: f32,
    /// `m_tele_fuel_total` (`+0x1638`, `f64` -> `f32`, used for candidate check).
    pub fuel_total: f32,
    /// `m_tele_fuel_capacity` (`+0x1640`, `f64` -> `f32`, stored into convoy record).
    pub fuel_capacity: f32,
    /// `m_tele_fuel_need_cr` (`+0x1650`, `f64` -> `f32`).
    pub fuel_rate: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct DesignShipworksStats {
    pub combat_value: f32,
    pub has_planes: bool,
}

type CopyStringFn =
    unsafe extern "C" fn(*mut RawEscadraString, *const c_char, usize) -> *mut RawEscadraString;
type StringTidyFn = unsafe extern "C" fn(*mut RawEscadraString);
type StringInitFromCStrFn =
    unsafe extern "C" fn(*mut RawEscadraString, *const c_char) -> *mut RawEscadraString;
type VecStringMovePushFn = unsafe extern "C" fn(*mut RawStringVec, *mut RawEscadraString);
type VecStringClearFn = unsafe extern "C" fn(*mut RawStringVec);
type VecStringDestroyFn = unsafe extern "C" fn(*mut RawStringVec);
type DesignStrengthFn = unsafe extern "C" fn(*const c_char) -> f32;
type NodeFindChildByNameFn = unsafe extern "C" fn(usize, *const c_char) -> usize;
type GetPropertyLinkedObjectFn = unsafe extern "C" fn(usize) -> usize;
type GetProfileFn = unsafe extern "C" fn() -> usize;
type FindLocationByNameFn = unsafe extern "C" fn(usize, *const RawEscadraString) -> usize;
type Vec2DistanceFn = unsafe extern "C" fn(*const Vec2, *const Vec2) -> f32;
type CreateEscadraFromShipsFn =
    unsafe extern "C" fn(*mut RawStringVec, f32, i32, *const Vec2, *const c_char, i32) -> usize;
type HgeRandFloatFn = unsafe extern "C" fn(usize, f32, f32) -> f32;
type HgeRandIntFn = unsafe extern "C" fn(usize, i32, i32) -> i32;

/// Page-checked unaligned scalar read from game memory.
pub unsafe fn read_checked<T: Copy>(address: usize) -> Option<T> {
    let end = address.checked_add(size_of::<T>())?;
    if address < 0x1_0000 {
        return None;
    }
    if !is_readable_page(address) {
        return None;
    }
    if (end - 1) / 4096 != address / 4096 && !is_readable_page(end - 1) {
        return None;
    }
    // SAFETY: `VirtualQuery` confirmed all spanned pages are committed and readable;
    // `read_unaligned` avoids alignment assumptions on raw game addresses.
    Some((address as *const T).read_unaligned())
}

unsafe fn is_readable_page(address: usize) -> bool {
    let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
    if VirtualQuery(
        address as *const c_void,
        &mut info,
        size_of::<MEMORY_BASIC_INFORMATION>(),
    ) == 0
    {
        return false;
    }
    info.State == 0x1000 && info.Protect & 0x101 == 0 && info.Protect != 0
}

/// Assigns `text` into an **existing, initialized** game-owned `EscadraString` at `dst`
/// (e.g. `NPC.m_ship_first`, which was constructed by `NPC_ConstructRecord`).
pub unsafe fn assign_existing_escadra_string(dst: *mut RawEscadraString, text: &str) {
    if dst.is_null() {
        return;
    }
    // SAFETY: `FN_COPY_STRING` (`0x1400056e0`) takes `(dst, ptr, count)` on an
    // already-constructed `EscadraString` and copies `count` bytes via the game CRT.
    let copy_fn: CopyStringFn = std::mem::transmute(FN_COPY_STRING);
    copy_fn(dst, text.as_ptr().cast::<c_char>(), text.len());
}

/// Initializes an **uninitialized** out-parameter `EscadraString` (`sret` ABI, such as
/// `endgameTarkhanShipNames?` at `0x140238e70` where the caller passes raw stack space
/// `[RBP + 0xa00]` with garbage `[dst + 0x18]`) and returns `dst` in `RAX`.
pub unsafe fn init_uninitialized_escadra_string_cstr(
    dst: *mut RawEscadraString,
    cstr: *const c_char,
) -> *mut RawEscadraString {
    if dst.is_null() {
        return dst;
    }
    let safe_cstr = if cstr.is_null() { c"".as_ptr() } else { cstr };
    // SAFETY: `FN_STRING_INIT_FROM_CSTR` (`0x140015390`) writes `[dst+0x10]=0`,
    // `[dst+0x18]=0xf`, `[dst]=0` BEFORE calling `CopyString`, matching the exact
    // C++ return-value construction contract at `0x140239992` / `0x1402399c3`.
    let init_fn: StringInitFromCStrFn = std::mem::transmute(FN_STRING_INIT_FROM_CSTR);
    init_fn(dst, safe_cstr)
}

/// Clears a game-owned `CVec<EscadraString>` (`end = begin` after tidying elements).
pub unsafe fn clear_game_string_vec(vec: *mut RawStringVec) {
    if vec.is_null() {
        return;
    }
    // SAFETY: `FN_VEC_STRING_CLEAR` (`0x14015cd20`) destroys initialized strings
    // in `[begin, end)` using the game's CRT allocator and sets `end = begin`.
    let clear_fn: VecStringClearFn = std::mem::transmute(FN_VEC_STRING_CLEAR);
    clear_fn(vec);
}

/// Destroys and frees a game-owned `CVec<EscadraString>`.
pub unsafe fn destroy_game_string_vec(vec: *mut RawStringVec) {
    if vec.is_null() {
        return;
    }
    // SAFETY: `FN_VEC_STRING_DESTROY` (`0x14005e580`) frees all elements and the
    // vector's backing allocation via the game's CRT `free`.
    let destroy_fn: VecStringDestroyFn = std::mem::transmute(FN_VEC_STRING_DESTROY);
    destroy_fn(vec);
}

/// Appends a ship name to a game-owned `CVec<EscadraString>` using native CRT routines.
pub unsafe fn push_game_string(vec: *mut RawStringVec, name_cstr: *const c_char) {
    if vec.is_null() || name_cstr.is_null() {
        return;
    }
    let mut temp = RawEscadraString { bytes: [0; 0x20] };
    // SAFETY: `temp` is a valid 0x20-byte stack buffer; `FN_STRING_INIT_FROM_CSTR`
    // initializes it, `FN_VEC_STRING_MOVE_PUSH` moves it into `vec`, and
    // `FN_STRING_TIDY` cleans up any remaining heap state.
    let init_fn: StringInitFromCStrFn = std::mem::transmute(FN_STRING_INIT_FROM_CSTR);
    let push_fn: VecStringMovePushFn = std::mem::transmute(FN_VEC_STRING_MOVE_PUSH);
    let tidy_fn: StringTidyFn = std::mem::transmute(FN_STRING_TIDY);
    init_fn(&mut temp, name_cstr);
    push_fn(vec, &mut temp);
    tidy_fn(&mut temp);
}

/// Resolves `name_cstr` in `LIB_SHIPS` (`0x143a13a70`) to its root `Creature*`,
/// following the exact chain in `retCreature154c_` (`0x140219980..0x1402199c8`).
pub unsafe fn lookup_design_creature(name_cstr: *const c_char) -> Option<usize> {
    if name_cstr.is_null() {
        return None;
    }
    let lib_ships = read_checked::<usize>(LIB_SHIPS_GLOBAL)?;
    if lib_ships == 0 {
        return None;
    }
    let vtbl = read_checked::<usize>(lib_ships)?;
    let find_fn_addr = read_checked::<usize>(vtbl + NODE_VTBL_FIND_CHILD_BY_NAME)?;
    if find_fn_addr == 0 {
        return None;
    }
    // SAFETY: `lib_ships` is the live `LIB_SHIPS` `Node*`; vtable `+0xb8` is
    // `Node::FindChildByName(this, const char*)`.
    let find_fn: NodeFindChildByNameFn = std::mem::transmute(find_fn_addr);
    let design_node = find_fn(lib_ships, name_cstr);
    if design_node == 0 {
        return None;
    }
    let children_count = read_checked::<usize>(design_node + NODE_CHILDREN_OFFSET + 8)?;
    if children_count == 0 {
        return None;
    }
    let children_head = read_checked::<usize>(design_node + NODE_CHILDREN_OFFSET)?;
    let first_dll_node = read_checked::<usize>(children_head)?;
    let first_child_body = read_checked::<usize>(first_dll_node + 0x10)?;
    if first_child_body == 0 {
        return None;
    }
    let creature = read_checked::<usize>(first_child_body + CHILD_DATA_OWNER_CREATURE_OFFSET)?;
    (creature != 0).then_some(creature)
}

/// Looks up `name_cstr` in `LIB_SHIPS` (`0x143a13a70`) and returns `design+0x15f8`.
pub unsafe fn lookup_design_strength(name_cstr: *const c_char) -> f32 {
    if name_cstr.is_null() {
        return 0.0;
    }
    // SAFETY: `FN_DESIGN_STRENGTH` (`0x140219980`) checks `newNode_ != null` and
    // `param_1 != null`, returning `0.0` when a design is absent.
    let strength_fn: DesignStrengthFn = std::mem::transmute(FN_DESIGN_STRENGTH);
    strength_fn(name_cstr)
}

/// Reads fuel/speed stats for `spawnTradeConvoy?` from a `LIB_SHIPS` design.
pub unsafe fn lookup_design_convoy_stats(name_cstr: *const c_char) -> Option<DesignConvoyStats> {
    let creature = lookup_design_creature(name_cstr)?;
    let airspeed = read_checked::<f32>(creature + CREATURE_AIRSPEED_OFFSET)?;
    let fuel_total = read_checked::<f64>(creature + CREATURE_FUEL_TOTAL_OFFSET)? as f32;
    let fuel_capacity = read_checked::<f64>(creature + CREATURE_FUEL_CAPACITY_OFFSET)? as f32;
    let fuel_rate = read_checked::<f64>(creature + CREATURE_FUEL_NEED_CR_OFFSET)? as f32;
    Some(DesignConvoyStats {
        airspeed,
        fuel_total,
        fuel_capacity,
        fuel_rate,
    })
}

/// Reads combat value and carrier capability for Shipworks `TEST` auto-classification.
pub unsafe fn lookup_design_shipworks_stats(
    name_cstr: *const c_char,
) -> Option<DesignShipworksStats> {
    let creature = lookup_design_creature(name_cstr)?;
    let combat_value = read_checked::<f32>(creature + CREATURE_COMBAT_VALUE_OFFSET).unwrap_or(0.0);
    let planes_begin = read_checked::<usize>(creature + CREATURE_PLANES_VEC_OFFSET).unwrap_or(0);
    let planes_end = read_checked::<usize>(creature + CREATURE_PLANES_VEC_OFFSET + 8).unwrap_or(0);
    let has_planes = planes_end > planes_begin;
    Some(DesignShipworksStats {
        combat_value,
        has_planes,
    })
}

/// Reads a bounded UTF-8 view of a game-owned `EscadraString` at `str_addr` without taking ownership.
unsafe fn read_escadra_string_lossy(str_addr: usize) -> Option<String> {
    let len = read_checked::<usize>(str_addr + 0x10)?;
    let cap = read_checked::<usize>(str_addr + 0x18)?;
    if len > 256 || len > cap {
        return None;
    }
    let buf_addr = if cap >= 0x10 {
        read_checked::<usize>(str_addr)?
    } else {
        str_addr
    };
    if buf_addr == 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(len);
    for i in 0..len {
        bytes.push(read_checked::<u8>(buf_addr + i)?);
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Marks the trade ship inside a newly created Trade Convoy `Escadra` with
/// `m_prize = 1` (`+0xc19`) and `m_playable = 0` (`+0xad0`), replicating
/// `Ship_LoadSeriaAndMarkPrize` (`0x1401b0a1d..0x1401b0a24`) for custom trader designs!
pub unsafe fn mark_first_matching_ship_as_prize(escadra: usize, trader_name: &str) {
    if escadra == 0 || trader_name.is_empty() {
        return;
    }
    let Some(head) = read_checked::<usize>(escadra + ESCADRA_CHILDREN_HEAD_OFFSET) else {
        return;
    };
    if head == 0 {
        return;
    }
    let get_linked: GetPropertyLinkedObjectFn = std::mem::transmute(FN_GET_PROPERTY_LINKED_OBJECT);
    let mut cur = read_checked::<usize>(head).unwrap_or(0);
    let mut steps = 0usize;
    while cur != 0 && cur != head && steps < 64 {
        steps += 1;
        if let Some(prop) = read_checked::<usize>(cur + 0x10) {
            if prop != 0 {
                // SAFETY: `prop` is a live `Property*` child node of `escadra`.
                let ship = get_linked(prop);
                if ship != 0 {
                    let matches = read_escadra_string_lossy(ship + CREATURE_SHIP_NAME_OFFSET)
                        .is_some_and(|s| s == trader_name);
                    if matches {
                        // SAFETY: `ship` is the freshly cloned `Creature*` in `escadra`;
                        // `+0xc19` (`m_prize`) and `+0xad0` (`m_playable`) are single-byte flags.
                        *((ship + CREATURE_PRIZE_OFFSET) as *mut u8) = 1;
                        *((ship + CREATURE_PLAYABLE_OFFSET) as *mut u8) = 0;
                        return;
                    }
                }
            }
        }
        cur = read_checked::<usize>(cur).unwrap_or(0);
    }
}

/// Reads `Profile::hardmode != 0` from the active profile (`0x140292a90`).
pub unsafe fn is_hard_mode() -> bool {
    // SAFETY: `FN_GET_PROFILE` returns the active `Profile*` (or 0 if uninitialized).
    let get_profile: GetProfileFn = std::mem::transmute(FN_GET_PROFILE);
    let profile = get_profile();
    if profile == 0 {
        return false;
    }
    read_checked::<u8>(profile + PROFILE_HARDMODE_OFFSET).unwrap_or(0) != 0
}

/// Reads `Profile::endwar != 0` (`+0x2e9`, true once Khiva defense / Nuclear War begins).
pub unsafe fn is_endwar() -> bool {
    // SAFETY: `FN_GET_PROFILE` returns the active `Profile*` (or 0 if uninitialized).
    let get_profile: GetProfileFn = std::mem::transmute(FN_GET_PROFILE);
    let profile = get_profile();
    if profile == 0 {
        return false;
    }
    read_checked::<u8>(profile + PROFILE_ENDWAR_OFFSET).unwrap_or(0) != 0
}

/// Replicates `Endgame_BuildEscadra`'s `"UR"` distance gate (`0x140207828..0x1402078e8`):
/// `true` on Hard mode, or when `"UR"` is missing, or when `dist("UR", pos) >= 5000.0`.
pub unsafe fn is_far_from_ur_or_hard(spawn_pos: *const Vec2) -> bool {
    if is_hard_mode() || spawn_pos.is_null() {
        return true;
    }
    let get_profile: GetProfileFn = std::mem::transmute(FN_GET_PROFILE);
    let profile = get_profile();
    if profile == 0 {
        return true;
    }

    let mut ur_name = RawEscadraString { bytes: [0; 0x20] };
    let init_fn: StringInitFromCStrFn = std::mem::transmute(FN_STRING_INIT_FROM_CSTR);
    let find_fn: FindLocationByNameFn = std::mem::transmute(FN_FIND_LOCATION_BY_NAME);
    let tidy_fn: StringTidyFn = std::mem::transmute(FN_STRING_TIDY);
    let dist_fn: Vec2DistanceFn = std::mem::transmute(FN_VEC2_DISTANCE);

    init_fn(&mut ur_name, c"UR".as_ptr());
    let ur_loc = find_fn(profile, &ur_name);
    tidy_fn(&mut ur_name);

    if ur_loc == 0 {
        return true;
    }
    let ur_pos = (ur_loc + LOCATION_POS_OFFSET) as *const Vec2;
    dist_fn(ur_pos, spawn_pos) >= 5000.0
}

/// Calls `Fleet_CreateEscadraFromShips` (`0x1401c7060`).
pub unsafe fn create_escadra_from_ships(
    names: *mut RawStringVec,
    variant: f32,
    role: i32,
    pos: *const Vec2,
    escadra_name: *const c_char,
    field_1a20: i32,
) -> usize {
    // SAFETY: Forwards exact native arguments to `0x1401c7060`.
    let create_fn: CreateEscadraFromShipsFn = std::mem::transmute(FN_CREATE_ESCADRA_FROM_SHIPS);
    create_fn(names, variant, role, pos, escadra_name, field_1a20)
}

/// Calls `HGE::RandFloat(min, max)` (`0x1401742d0`).
pub unsafe fn hge_rand_float(min: f32, max: f32) -> f32 {
    let hge = read_checked::<usize>(HGE_GLOBAL).unwrap_or(0);
    // SAFETY: `0x1401742d0` steps the global LCG at `0x1439e8688` (ignoring `this` in `RCX`).
    let rand_fn: HgeRandFloatFn = std::mem::transmute(FN_HGE_RAND_FLOAT);
    rand_fn(hge, min, max)
}

/// Calls `HGE::RandInt(min, max)` (`0x140174320`, inclusive `[min, max]`).
pub unsafe fn hge_rand_int(min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    let hge = read_checked::<usize>(HGE_GLOBAL).unwrap_or(0);
    // SAFETY: `0x140174320` steps the global LCG and requires `max - min + 1 != 0`
    // (`DIV R8D`); `max > min` guarantees `R8D >= 2`.
    let rand_fn: HgeRandIntFn = std::mem::transmute(FN_HGE_RAND_INT);
    rand_fn(hge, min, max)
}
