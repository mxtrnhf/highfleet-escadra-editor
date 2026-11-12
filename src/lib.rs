//! Highfleet Escadra Editor (HEE). Steam 1.163 only.
//!
//! Every ship spawn pool in HighFleet is hard-coded inline in `.text` (no data
//! files): Strike Groups, Aircraft and Missile Tactical Groups, City Garrisons,
//! Tarkhan personal/finale ships, Mercenary city pier offers, Tarkhan call-in
//! reinforcements, Trade Convoys, and Shipworks `TEST` combat. HEE exposes them
//! in one config file, `Modloader/config/escadra.json`, and installs byte-guarded
//! native detours only for the pools modified in the config.
//!
//! Supporting reverse-engineering notes:
//! - `hf_notes/docs/findings/strategy/enemy-ship-pools.md`
//! - `hf_notes/docs/findings/strategy/mercenary-ships.md`

highfleet_mod_api::export_logger!();

mod config;
mod hooks;
mod native;
mod pool_math;
mod pools;

use std::ffi::c_char;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;

use crate::pool_math::ResolvedPools;

const IMAGE_BASE: usize = 0x1_4000_0000;
const CONFIG_PATH: &str = "Modloader/config/escadra.json";

static GAME_IS_STEAM_1_163: AtomicBool = AtomicBool::new(false);

#[no_mangle]
unsafe extern "C" fn version(version: *const c_char) -> bool {
    let supported = CStr::from_ptr(version).to_bytes() == b"Steam 1.163";
    GAME_IS_STEAM_1_163.store(supported, Ordering::Relaxed);
    supported
}

#[no_mangle]
unsafe extern "C" fn init() -> bool {
    log::info!("hf_hee v{}", env!("CARGO_PKG_VERSION"));
    if !GAME_IS_STEAM_1_163.load(Ordering::Relaxed) {
        log::error!("hf_hee: needs Steam 1.163, not starting");
        return false;
    }
    if GetModuleHandleA(std::ptr::null()) as usize != IMAGE_BASE {
        log::error!("hf_hee: unexpected image base, not starting");
        return false;
    }

    let config = config::load_or_create(CONFIG_PATH);
    if !config.enabled {
        log::info!("hf_hee: disabled in {CONFIG_PATH}");
        return true;
    }

    pools::log_summary(&config);
    let resolved = ResolvedPools::from_config(&config);

    match hooks::install(resolved) {
        Ok(0) => true,
        Ok(count) => {
            log::info!("hf_hee: active ({count} pool detour(s) installed)");
            true
        }
        Err(error) => {
            log::error!("hf_hee: failed to install detours: {error}");
            false
        }
    }
}
