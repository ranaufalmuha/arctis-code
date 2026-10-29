//! Recent projects, stored separately from settings and workspace data.
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

pub fn record(items: &mut Vec<PathBuf>, path: &Path) -> bool {
    if !path.is_dir() { return false; }
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if items.first() == Some(&path) { return false; }
    items.retain(|p| p != &path);
    items.insert(0, path);
    items.truncate(MAX_RECENT);
    true
}

fn history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("ArctisCode").join("recent-projects.json"))
}

pub fn load() -> Vec<PathBuf> {
    let Some(path) = history_path() else { return Vec::new() };
    let mut items: Vec<PathBuf> = std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    items.truncate(MAX_RECENT);
    items
}

pub fn save(items: &[PathBuf]) -> anyhow::Result<()> {
    let path = history_path().ok_or_else(|| anyhow::anyhow!("No local data directory"))?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(items)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
mod native {
    use std::ffi::{CStr, CString};
    use std::path::PathBuf;
    static CONTEXT: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();
    pub fn install_context(ctx: &egui::Context) { let _ = CONTEXT.set(ctx.clone()); }
    #[no_mangle]
    pub extern "C" fn arctis_dock_wake() { if let Some(ctx) = CONTEXT.get() { ctx.request_repaint(); } }
    unsafe extern "C" {
        fn arctis_dock_init();
        fn arctis_dock_set_history(paths: *const *const std::ffi::c_char, count: usize);
        fn arctis_dock_take_selection() -> *const std::ffi::c_char;
    }
    pub fn update(items: &[PathBuf]) {
        let paths: Vec<CString> = items.iter().filter_map(|p| CString::new(p.to_string_lossy().as_bytes()).ok()).collect();
        let pointers: Vec<_> = paths.iter().map(|p| p.as_ptr()).collect();
        unsafe { arctis_dock_init(); arctis_dock_set_history(pointers.as_ptr(), pointers.len()); }
    }
    pub fn take() -> Option<PathBuf> {
        unsafe { let p = arctis_dock_take_selection(); if p.is_null() { None } else { Some(PathBuf::from(CStr::from_ptr(p).to_string_lossy().into_owned())) } }
    }
}
#[cfg(target_os = "macos")]
pub use native::{install_context, take, update};
#[cfg(not(target_os = "macos"))]
pub fn install_context(_: &egui::Context) {}
#[cfg(not(target_os = "macos"))]
pub fn update(_: &[PathBuf]) {}
#[cfg(not(target_os = "macos"))]
pub fn take() -> Option<PathBuf> { None }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_is_deduplicated_and_capped() {
        let dir = tempfile::tempdir().unwrap();
        let mut items = Vec::new();
        for i in 0..12 { let p = dir.path().join(i.to_string()); std::fs::create_dir(&p).unwrap(); record(&mut items, &p); }
        assert_eq!(items.len(), 10);
        let p = dir.path().join("4"); record(&mut items, &p);
        assert_eq!(items[0], p.canonicalize().unwrap());
        assert_eq!(items.len(), 10);
        assert!(!record(&mut items, &p));
    }
}
