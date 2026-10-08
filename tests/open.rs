//! Click-to-open edge cases: encodings, symlinks, size, missing files.

use arctis_code::tabs::{classify, TabKind, TabManager};

#[test]
fn opens_utf16_file_lossy_instead_of_error() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("utf16.txt");
    // "hi" in UTF-16LE with BOM — not valid UTF-8, but plain text to a user.
    std::fs::write(&f, b"\xff\xfeh\x00i\x00").unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&f)
        .expect("UTF-16 text must open (lossy), not error");
    assert_eq!(mgr.tabs.len(), 1);
}

#[test]
fn opens_latin1_file_lossy_instead_of_error() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("latin1.txt");
    std::fs::write(&f, b"caf\xe9").unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&f)
        .expect("Latin-1 text must open (lossy), not error");
}

#[test]
fn binary_file_opens_as_unsupported_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("blob.bin");
    std::fs::write(&f, [0x00, 0x01, 0x02, b'h', b'i', 0x00]).unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&f)
        .expect("binary must open as Unsupported, not error");
    assert_eq!(mgr.tabs[0].kind, TabKind::Unsupported);
}

#[test]
fn directory_opens_as_unsupported_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&sub)
        .expect("directory must open as Unsupported, not error");
    assert_eq!(mgr.tabs[0].kind, TabKind::Unsupported);
}

#[test]
fn symlinked_dir_classifies_for_tree_toggle() {
    #[cfg(unix)]
    {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        // Tree must treat this as a dir (toggle), so Tab::open must not
        // be asked to read it as a file. Defensive: still no error.
        assert!(link.is_dir());
        let mut mgr = TabManager::default();
        mgr.open(&link).expect("symlinked dir must not error");
        assert_eq!(mgr.tabs[0].kind, TabKind::Unsupported);
    }
}

#[test]
fn missing_file_reports_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("gone.txt");
    let mut mgr = TabManager::default();
    let err = mgr.open(&f).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("No such file") || msg.contains("not found") || msg.contains("failed"),
        "unexpected error: {msg}"
    );
}

#[test]
fn empty_and_extensionless_files_open() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.txt");
    std::fs::write(&empty, b"").unwrap();
    let makefile = dir.path().join("Makefile");
    std::fs::write(&makefile, b"all:\n\techo hi\n").unwrap();
    let mut mgr = TabManager::default();
    mgr.open(&empty).expect("empty file must open");
    mgr.open(&makefile).expect("extensionless file must open");
    assert_eq!(mgr.tabs.len(), 2);
}

#[test]
fn dotfiles_classify_as_text() {
    assert_eq!(
        classify(&std::path::PathBuf::from(".gitignore")),
        TabKind::Text
    );
}
