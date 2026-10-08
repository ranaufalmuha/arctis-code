//! Neovim backend regression tests: spawn fast, never block, degrade cleanly.

use arctis_code::editor::NeovimBackend;

fn nvim_available() -> bool {
    std::process::Command::new("nvim")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn spawn_returns_promptly() {
    let t = std::time::Instant::now();
    let backend = NeovimBackend::spawn();
    assert!(
        t.elapsed() < std::time::Duration::from_secs(15),
        "spawn took {:?}",
        t.elapsed()
    );
    // Either way it must report honestly, never hang.
    let _ = backend.status();
}

#[test]
fn sync_never_blocks_caller() {
    if !nvim_available() {
        return;
    }
    let backend = NeovimBackend::spawn();
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("tiny.txt");
    std::fs::write(&f, "asdasda").unwrap(); // no trailing newline, like the freeze report
    let t = std::time::Instant::now();
    for _ in 0..50 {
        backend.sync_buffer(&f, "asdasda");
    }
    assert!(
        t.elapsed() < std::time::Duration::from_secs(2),
        "sync_buffer blocked the caller for {:?}",
        t.elapsed()
    );
}

#[test]
fn sync_then_read_roundtrip() {
    if !nvim_available() {
        return;
    }
    let backend = NeovimBackend::spawn();
    if !matches!(backend.status(), arctis_code::editor::NvimStatus::Connected) {
        return; // degraded environment; spawn test above covers reporting
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("roundtrip.txt");
    std::fs::write(&f, "hello").unwrap();
    backend.sync_buffer(&f, "hello\nedited");
    // Give the fire-and-forget task a moment, then read back.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let back = backend
        .read_buffer(&f)
        .expect("read_buffer works")
        .expect("buffer present");
    assert!(back.contains("edited"), "unexpected buffer: {back:?}");
}

#[test]
fn killswitch_disables_backend() {
    std::env::set_var("ARCTIS_NO_NVIM", "1");
    let backend = NeovimBackend::spawn();
    std::env::remove_var("ARCTIS_NO_NVIM");
    assert!(matches!(
        backend.status(),
        arctis_code::editor::NvimStatus::Unavailable(_)
    ));
    // Must be a silent no-op, never panic.
    backend.sync_buffer(std::path::Path::new("/tmp/x"), "hi");
}
