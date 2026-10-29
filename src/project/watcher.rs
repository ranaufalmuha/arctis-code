//! Filesystem watcher: refresh + external-change detection.
//!
//! PRD V1 §4.3: refresh directory, detect changes where practical.
//! Uses `notify` with a debounced channel; the UI polls non-blockingly.

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

#[derive(Debug)]
pub enum FsEvent {
    Changed(PathBuf),
    NeedsRefresh,
}

pub struct ProjectWatcher {
    _watcher: Option<RecommendedWatcher>,
    rx: Receiver<FsEvent>,
}

impl ProjectWatcher {
    pub fn new() -> Self {
        let (_, rx) = channel();
        Self { _watcher: None, rx }
    }

    pub fn watch(&mut self, root: &std::path::Path) -> Result<()> {
        let (tx_raw, rx_raw) = channel();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(evt) = res {
                    let _ = tx_raw.send(evt);
                }
            })?;
        watcher.watch(root, RecursiveMode::Recursive)?;

        // Bridge notify events into FsEvent channel on a background thread
        // with a trailing-edge debounce: rapid saves coalesce, but the
        // last one always lands (leading-edge `continue` could drop it).
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut pending: Option<PathBuf> = None;
            loop {
                match rx_raw.recv_timeout(Duration::from_millis(150)) {
                    Ok(evt) => {
                        pending = Some(evt.paths.first().cloned().unwrap_or_default());
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if let Some(p) = pending.take() {
                            let _ = tx.send(FsEvent::Changed(p));
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        if let Some(p) = pending.take() {
                            let _ = tx.send(FsEvent::Changed(p));
                        }
                        break;
                    }
                }
            }
        });

        self._watcher = Some(watcher);
        self.rx = rx;
        Ok(())
    }

    pub fn drain(&self) -> Vec<FsEvent> {
        let mut out = Vec::new();
        while let Ok(evt) = self.rx.try_recv() {
            out.push(evt);
        }
        out
    }
}

impl Default for ProjectWatcher {
    fn default() -> Self {
        Self::new()
    }
}
