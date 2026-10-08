//! Drag-to-copy / drag-to-move engine checks (never overwrite).

use arctis_code::project::import::{copy_into, move_into};

fn setup() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn copies_file_into_folder() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src = root.join("a.txt");
    let dest = root.join("docs");
    std::fs::write(&src, "hello").unwrap();
    std::fs::create_dir(&dest).unwrap();

    let out = copy_into(std::slice::from_ref(&src), &dest, &root).unwrap();
    assert_eq!(out.len(), 1);
    assert!(out[0].succeeded());
    let got = out[0].dest.as_ref().unwrap();
    assert_eq!(got.file_name().unwrap(), "a.txt");
    // macOS symlinks /var → /private/var, so compare canonicalized paths.
    assert_eq!(
        got.canonicalize().unwrap(),
        dest.join("a.txt").canonicalize().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn collision_renames_instead_of_overwriting() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src = root.join("a.txt");
    let dest = root.join("docs");
    std::fs::create_dir(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), "original").unwrap();
    std::fs::write(&src, "incoming").unwrap();

    let out = copy_into(&[src], &dest, &root).unwrap();
    assert!(out[0].succeeded());
    assert!(out[0].renamed);
    // Original untouched.
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).unwrap(),
        "original"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("a (2).txt")).unwrap(),
        "incoming"
    );
}

#[test]
fn copies_directory_recursively() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src_dir = root.join("assets");
    std::fs::create_dir_all(src_dir.join("sub")).unwrap();
    std::fs::write(src_dir.join("sub").join("x.txt"), "x").unwrap();
    let dest = root.join("docs");
    std::fs::create_dir(&dest).unwrap();

    let out = copy_into(&[src_dir], &dest, &root).unwrap();
    assert!(out[0].succeeded());
    assert_eq!(
        std::fs::read_to_string(dest.join("assets").join("sub").join("x.txt")).unwrap(),
        "x"
    );
}

#[test]
fn rejects_folder_into_itself() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let sub = root.join("sub");
    std::fs::create_dir(&sub).unwrap();

    let out = copy_into(std::slice::from_ref(&root), &sub, &root).unwrap();
    assert!(out[0].skipped);
}

#[test]
fn rejects_target_outside_project() {
    let tmp = setup();
    let root = tmp.path().join("proj");
    std::fs::create_dir(&root).unwrap();
    let outside = tmp.path().join("elsewhere");
    std::fs::create_dir(&outside).unwrap();
    let src = root.join("a.txt");
    std::fs::write(&src, "hi").unwrap();

    let err = copy_into(&[src], &outside, &root).unwrap_err();
    assert!(err.to_string().contains("outside the project"));
}

#[test]
fn skips_file_already_in_folder() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src = root.join("a.txt");
    std::fs::write(&src, "hi").unwrap();

    let out = copy_into(&[src], &root, &root).unwrap();
    assert!(out[0].skipped);
}

#[test]
fn moves_file_out_of_old_location() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src = root.join("a.txt");
    let dest = root.join("docs");
    std::fs::write(&src, "hello").unwrap();
    std::fs::create_dir(&dest).unwrap();

    let out = move_into(std::slice::from_ref(&src), &dest, &root).unwrap();
    assert!(out[0].succeeded());
    assert!(!out[0].renamed);
    assert!(!src.exists(), "source must be gone after move");
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn move_collision_renames_without_overwriting() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src = root.join("a.txt");
    let dest = root.join("docs");
    std::fs::create_dir(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), "original").unwrap();
    std::fs::write(&src, "incoming").unwrap();

    let out = move_into(std::slice::from_ref(&src), &dest, &root).unwrap();
    assert!(out[0].succeeded());
    assert!(out[0].renamed);
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).unwrap(),
        "original"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("a (2).txt")).unwrap(),
        "incoming"
    );
    assert!(!src.exists());
}

#[test]
fn moved_folder_retargets_nested_tabs() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let src_dir = root.join("old");
    std::fs::create_dir(&src_dir).unwrap();
    let inner = src_dir.join("code.rs");
    std::fs::write(&inner, "fn f(){}").unwrap();
    let dest = root.join("docs");
    std::fs::create_dir(&dest).unwrap();

    let mut mgr = arctis_code::tabs::TabManager::default();
    mgr.open(&inner).unwrap();

    let out = move_into(std::slice::from_ref(&src_dir), &dest, &root).unwrap();
    assert!(out[0].succeeded());
    let new_dir = out[0].dest.clone().unwrap();
    mgr.retarget_moved(&src_dir, &new_dir);
    assert_eq!(mgr.tabs[0].path, new_dir.join("code.rs"));
    assert!(mgr.tabs[0].path.exists());
}

#[test]
fn move_rejects_folder_into_itself() {
    let tmp = setup();
    let root = tmp.path().to_path_buf();
    let sub = root.join("sub");
    std::fs::create_dir(&sub).unwrap();

    let out = move_into(std::slice::from_ref(&root), &sub, &root).unwrap();
    assert!(out[0].skipped);
    assert!(root.join("sub").exists(), "nothing must have moved");
}
