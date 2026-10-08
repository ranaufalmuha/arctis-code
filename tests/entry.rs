//! New file / folder creation checks.

use arctis_code::project::entry::{create_entry, validate_name, EntryKind};

#[test]
fn rejects_bad_names() {
    assert!(validate_name("").is_err());
    assert!(validate_name("   ").is_err());
    assert!(validate_name("a/b").is_err());
    assert!(validate_name("..").is_err());
    assert!(validate_name(".").is_err());
    assert_eq!(validate_name("  notes.md ").unwrap(), "notes.md");
}

#[test]
fn creates_file_and_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let f = create_entry(root, EntryKind::File, "hello.rs").unwrap();
    assert!(f.is_file());
    let d = create_entry(root, EntryKind::Folder, "src").unwrap();
    assert!(d.is_dir());
}

#[test]
fn collisions_rename_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("a.txt"), "original").unwrap();
    let created = create_entry(root, EntryKind::File, "a.txt").unwrap();
    assert_eq!(created.file_name().unwrap(), "a (2).txt");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        "original"
    );
}

#[test]
fn rejects_non_folder_target() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("f.txt");
    std::fs::write(&f, "x").unwrap();
    assert!(create_entry(&f, EntryKind::File, "y.txt").is_err());
}
