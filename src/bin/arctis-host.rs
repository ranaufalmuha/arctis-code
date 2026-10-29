//! arctis-host: dev hot-reload shell (phase 0 spike).
//!
//! Loads `libarctis_code.{dylib,so,dll}` and runs the app from it instead of
//! statically linked code. Phase 0 loads once at startup; live swap without
//! restart comes in phase 1. See `docs/HOTRELOAD.md`.

use std::path::PathBuf;

const EXPECTED_ABI: u64 = 1;

type AbiVersionFn = unsafe extern "C" fn() -> u64;
type CreateAppFn = unsafe extern "C" fn(&eframe::CreationContext<'_>) -> *mut Box<dyn eframe::App>;

fn dylib_path() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe path");
    dir.pop();
    let file = if cfg!(target_os = "macos") {
        "libarctis_code.dylib"
    } else if cfg!(target_os = "windows") {
        "arctis_code.dll"
    } else {
        "libarctis_code.so"
    };
    dir.join(file)
}

fn err(msg: String) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(std::io::Error::other(msg))
}

fn load_app(
    cc: &eframe::CreationContext<'_>,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    // Leak the library: app code must stay mapped for the whole process.
    // (Unload/reload support is phase 1.)
    let lib: &'static libloading::Library = Box::leak(Box::new(
        unsafe { libloading::Library::new(dylib_path()) }
            .map_err(|e| err(format!("open {}: {e}", dylib_path().display())))?,
    ));

    let version: libloading::Symbol<AbiVersionFn> = unsafe { lib.get(b"arctis_abi_version") }
        .map_err(|e| err(format!("lookup arctis_abi_version: {e}")))?;
    let found = unsafe { version() };
    if found != EXPECTED_ABI {
        return Err(err(format!(
            "ABI mismatch: dylib={found}, host expects {EXPECTED_ABI}"
        )));
    }

    let create: libloading::Symbol<CreateAppFn> = unsafe { lib.get(b"arctis_create_app") }
        .map_err(|e| err(format!("lookup arctis_create_app: {e}")))?;
    let raw = unsafe { create(cc) };
    if raw.is_null() {
        return Err(err("arctis_create_app returned null".into()));
    }
    Ok(unsafe { *Box::from_raw(raw) })
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("ArctisCode (hot)")
            // Transparent window, sharp (see main.rs).
            .with_transparent(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native("ArctisCode (hot)", options, Box::new(load_app))
}
