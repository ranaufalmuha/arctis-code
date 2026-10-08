//! Hot-reload ABI smoke test: the cdylib loads and reports the ABI
//! version the host expects. (Phase 0: load-once; no live swap yet.)

fn dylib_path() -> std::path::PathBuf {
    let mut dir = std::env::current_exe().unwrap();
    dir.pop(); // deps/ → debug/
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let file = if cfg!(target_os = "macos") {
        "libarctis_code.dylib"
    } else if cfg!(target_os = "windows") {
        "arctis_code.dll"
    } else {
        "libarctis_code.so"
    };
    dir.join(file)
}

#[test]
fn dylib_exposes_expected_abi() {
    let path = dylib_path();
    assert!(
        path.exists(),
        "cdylib not built at {}. Run `cargo build` first.",
        path.display()
    );
    unsafe {
        let lib = libloading::Library::new(&path).expect("load cdylib");
        let version: libloading::Symbol<unsafe extern "C" fn() -> u64> = lib
            .get(b"arctis_abi_version")
            .expect("symbol arctis_abi_version");
        assert_eq!(version(), arctis_code::HOT_ABI_VERSION);
    }
}
