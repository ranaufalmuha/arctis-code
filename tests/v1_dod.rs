//! V1 Definition-of-Done logic checks (PRD V1 §8) — no window needed.

use arctis_code::project::opener::{classify_dropped, DropAction};
use arctis_code::safety::{write_text_guarded, FileSnapshot};
use arctis_code::tabs::{classify, TabKind, TabManager};
use std::path::PathBuf;

#[test]
fn drop_folder_becomes_project() {
    let dir = tempfile::tempdir().unwrap();
    match classify_dropped(std::slice::from_ref(&dir.path().to_path_buf())) {
        DropAction::OpenProject(p) => assert_eq!(p, dir.path()),
        other => panic!("expected OpenProject, got {other:?}"),
    }
}

#[test]
fn drop_files_open_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.rs");
    let b = dir.path().join("b.png");
    std::fs::write(&a, "fn main(){}").unwrap();
    std::fs::write(&b, "fake").unwrap();
    match classify_dropped(&[a.clone(), b.clone()]) {
        DropAction::OpenFiles(files) => assert_eq!(files.len(), 2),
        other => panic!("expected OpenFiles, got {other:?}"),
    }
}

#[test]
fn tabs_open_and_switch() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.rs");
    std::fs::write(&a, "hello").unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&a).unwrap();
    assert_eq!(mgr.tabs.len(), 1);
    // Re-open same file focuses instead of duplicating.
    mgr.open(&a).unwrap();
    assert_eq!(mgr.tabs.len(), 1);
    assert_eq!(mgr.active, Some(0));
}

#[test]
fn guarded_write_blocks_external_edit() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("x.txt");
    std::fs::write(&f, "v1").unwrap();
    let snap = FileSnapshot::capture(&f);
    // External modification.
    std::fs::write(&f, "v2-external").unwrap();
    let err = write_text_guarded(&f, &snap, "v3-mine", false).unwrap_err();
    assert!(err.to_string().contains("changed on disk"));
    // Force overwrites.
    let snap2 = write_text_guarded(&f, &snap, "v3-mine", true).unwrap();
    assert!(!snap2.is_stale());
}

#[test]
fn images_are_not_text() {
    assert_eq!(classify(&PathBuf::from("a.png")), TabKind::Image);
    assert_eq!(classify(&PathBuf::from("a.rs")), TabKind::Text);
}

#[test]
fn file_open_latency_budget() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("big.rs");
    let content = "fn f() {}\n".repeat(1500); // ~50KB
    std::fs::write(&f, &content).unwrap();
    let t = std::time::Instant::now();
    let mut mgr = TabManager::default();
    mgr.open(&f).unwrap();
    assert!(t.elapsed().as_millis() < 100, "open took {:?}", t.elapsed());
}
