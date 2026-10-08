pub mod app;
pub mod editor;
pub mod explorer;
pub mod icons;
pub mod preview;
pub mod project;
pub mod safety;
pub mod settings;
pub mod tabs;
pub mod tasks;
pub mod theme;
pub mod v2_stub;
pub mod window;

/// Hot-reload ABI version (dev only). The `arctis-host` binary checks this
/// symbol before running the app. Bump whenever the ABI surface changes.
/// See `docs/HOTRELOAD.md`.
pub const HOT_ABI_VERSION: u64 = 1;

#[no_mangle]
pub extern "C" fn arctis_abi_version() -> u64 {
    HOT_ABI_VERSION
}

/// Construct the app inside the dylib. The double box keeps the pointer
/// thin (`*mut dyn App` would be a fat pointer and must not cross FFI).
///
/// Initial project path is read from `argv[1]`, same as the static binary.
#[no_mangle]
pub extern "C" fn arctis_create_app(cc: &eframe::CreationContext<'_>) -> *mut Box<dyn eframe::App> {
    let initial_path = std::env::args().nth(1);
    let app: Box<dyn eframe::App> = Box::new(app::ArctisCodeApp::new(cc, initial_path));
    Box::into_raw(Box::new(app))
}

/// Destroy an app created by [`arctis_create_app`].
///
/// # Safety
/// `app` must be a non-dangling pointer returned by `arctis_create_app`
/// and must not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn arctis_destroy_app(app: *mut Box<dyn eframe::App>) {
    if !app.is_null() {
        unsafe {
            drop(Box::from_raw(app));
        }
    }
}
