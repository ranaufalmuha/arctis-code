//! Multi-window coexist model: two repo windows hold independent tab
//! buffers for the same file, so edits in one never leak into the other.
//! (Shared Neovim sync is path-keyed at runtime; the buffers themselves
//! stay per-window, VSCode-style.)

use arctis_code::window::{fresh_viewport_id, RepoWindow};

#[test]
fn two_windows_hold_independent_buffers_for_same_file() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("shared.rs");
    std::fs::write(&shared, "fn a() {}\n").unwrap();

    let mut wa = RepoWindow::blank(fresh_viewport_id(1));
    let mut wb = RepoWindow::blank(fresh_viewport_id(2));
    assert_ne!(wa.viewport, wb.viewport);

    wa.tabs.open(&shared).unwrap();
    wb.tabs.open(&shared).unwrap();
    assert_eq!(wa.tabs.tabs.len(), 1);
    assert_eq!(wb.tabs.tabs.len(), 1);

    // Edit in window A only.
    wa.tabs.tabs[0].content.push_str("// edited in A\n");
    wa.tabs.tabs[0].mark_edited();

    assert!(wa.tabs.tabs[0].dirty);
    assert!(!wb.tabs.tabs[0].dirty);
    assert_eq!(wb.tabs.tabs[0].content, "fn a() {}\n");
}

#[test]
fn blank_windows_start_with_no_project() {
    let w = RepoWindow::blank(fresh_viewport_id(7));
    assert!(w.project.root.is_none());
    assert!(w.tabs.tabs.is_empty());
}
