//! Highfleet Escadra Editor (HEE). Steam 1.163 only.
//!
//! Every enemy ship pool in the game is hard-coded inline in the executable
//! (no data files): strike groups, tactical groups, city garrisons and the
//! general-enemy catalogue. HEE makes them editable from one config file,
//! `Modloader/config/escadra.json`, so ships can be added to or removed from
//! a pool without rebuilding anything. Evidence and addresses:
//! `hf_notes/docs/findings/strategy/enemy-ship-pools.md`.
//!
//! v0.1.0 is a scaffold: it loads/creates the config, logs one summary line
//! per pool and changes nothing in game. The detours that apply the edits
//! land in the next version.

highfleet_mod_api::export_logger!();

mod config;
mod pools;

use std::ffi::c_char;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;

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
    log::info!("hf_hee: v0.1.0 is scaffold-only; no pools are modified yet");
    true
}
