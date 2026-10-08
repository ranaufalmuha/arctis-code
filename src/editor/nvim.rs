//! Neovim integration (PRD V1 §4.4).
//!
//! V1 strategy — buffer-synced editing:
//! - The Rust GUI owns the native window, tree, tabs, drag-drop, preview.
//! - A single embedded `nvim --embed` process owns buffer state via msgpack-RPC.
//! - egui provides the immediate editing surface; opens/edits sync the
//!   buffer through Neovim (`nvim_buf_set_lines` / `nvim_buf_get_lines`).
//!
//! Hard rule: **never block the UI thread on Neovim.** All syncs are
//! fire-and-forget runtime tasks with timeouts, so a slow or dead nvim
//! degrades to native editing instead of freezing the app.
//!
//! The embedded nvim runs stock (`--noplugin -u NONE`) so user plugins can
//! never wedge startup. `ARCTIS_NO_NVIM=1` disables the backend entirely.

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

type NvimWriter = nvim_rs::compat::tokio::Compat<tokio::process::ChildStdin>;
type Nvim = nvim_rs::Neovim<NvimWriter>;
type IoTask = tokio::task::JoinHandle<Result<(), Box<nvim_rs::error::LoopError>>>;

/// RPC calls slower than this are abandoned (backend marked unhealthy).
const RPC_TIMEOUT: Duration = Duration::from_secs(2);
/// Embedded nvim must answer its health check within this time.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct NopHandler;

impl nvim_rs::Handler for NopHandler {
    type Writer = NvimWriter;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NvimStatus {
    Connected,
    Unavailable(String),
}

pub struct NeovimBackend {
    nvim: Option<Arc<Mutex<Nvim>>>,
    status: NvimStatus,
    rt: Option<tokio::runtime::Runtime>,
    healthy: Arc<AtomicBool>,
    // Owned so the child process + RPC io loop live as long as the backend.
    _child: Option<tokio::process::Child>,
    _io: Option<IoTask>,
}

impl Drop for NeovimBackend {
    fn drop(&mut self) {
        if let Some(mut child) = self._child.take() {
            let _ = child.start_kill();
        }
    }
}

impl NeovimBackend {
    pub fn spawn() -> Self {
        if std::env::var("ARCTIS_NO_NVIM").is_ok() {
            return Self {
                nvim: None,
                status: NvimStatus::Unavailable("disabled via ARCTIS_NO_NVIM".into()),
                rt: None,
                healthy: Arc::new(AtomicBool::new(false)),
                _child: None,
                _io: None,
            };
        }

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .ok();
        let mut backend = Self {
            nvim: None,
            status: NvimStatus::Unavailable("runtime init failed".into()),
            rt,
            healthy: Arc::new(AtomicBool::new(false)),
            _child: None,
            _io: None,
        };
        let Some(rt) = backend.rt.as_ref() else {
            return backend;
        };

        let result: Result<(Nvim, IoTask, tokio::process::Child)> = rt.block_on(async {
            let mut cmd = tokio::process::Command::new("nvim");
            cmd.arg("--embed")
                .arg("--headless")
                .arg("--noplugin")
                .arg("-u")
                .arg("NONE");
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::null());
            let (nvim, io_handler, child) =
                nvim_rs::create::tokio::new_child_cmd(&mut cmd, NopHandler)
                    .await
                    .map_err(|e| anyhow::anyhow!("spawn nvim --embed: {e}"))?;
            // Health check: stock nvim must answer promptly, or we refuse
            // to depend on it (caller keeps native editing).
            tokio::time::timeout(SPAWN_TIMEOUT, nvim.get_api_info())
                .await
                .map_err(|_| anyhow::anyhow!("nvim health check timed out"))?
                .map_err(|e| anyhow::anyhow!("nvim health check failed: {e:?}"))?;
            Ok((nvim, io_handler, child))
        });

        match result {
            Ok((nvim, io_handler, child)) => {
                backend.nvim = Some(Arc::new(Mutex::new(nvim)));
                backend._io = Some(io_handler);
                backend._child = Some(child);
                backend.healthy.store(true, Ordering::SeqCst);
                backend.status = NvimStatus::Connected;
            }
            Err(e) => {
                backend.status = NvimStatus::Unavailable(e.to_string());
            }
        }
        backend
    }

    pub fn status(&self) -> NvimStatus {
        if self.nvim.is_some() && !self.healthy.load(Ordering::SeqCst) {
            return NvimStatus::Unavailable("nvim stopped responding".into());
        }
        self.status.clone()
    }

    /// Push current GUI text into the Neovim buffer for `path`.
    ///
    /// Fire-and-forget: returns immediately, never blocks the UI thread.
    pub fn sync_buffer(&self, path: &Path, content: &str) {
        let Some(nvim) = self.nvim.clone() else {
            return;
        };
        let Some(rt) = self.rt.as_ref() else { return };
        if !self.healthy.load(Ordering::SeqCst) {
            return;
        }
        let path = path.to_path_buf();
        let lines: Vec<String> = content.lines().map(|l| l.to_owned()).collect();
        let healthy = self.healthy.clone();
        rt.spawn(async move {
            let res = tokio::time::timeout(RPC_TIMEOUT, async {
                let nvim = nvim.lock().await;
                let escaped = path.display().to_string().replace('\'', "''");
                let _ = nvim.command(&format!("silent! edit \'{escaped}\'")).await;
                if let Ok(buf) = nvim.get_current_buf().await {
                    let _ = buf.set_lines(0, -1, true, lines).await;
                }
            })
            .await;
            if res.is_err() {
                healthy.store(false, Ordering::SeqCst);
            }
        });
    }

    /// Pull buffer text back from Neovim (e.g. after `:w` externally).
    ///
    /// Blocking with a timeout; call off the UI thread (tests, background
    /// tasks). Currently unused by the GUI, which owns buffer state.
    pub fn read_buffer(&self, path: &Path) -> Result<Option<String>> {
        let Some(nvim) = self.nvim.clone() else {
            return Ok(None);
        };
        let Some(rt) = self.rt.as_ref() else {
            return Ok(None);
        };
        let path = path.to_path_buf();
        let text = rt.block_on(async {
            tokio::time::timeout(RPC_TIMEOUT, async {
                let nvim = nvim.lock().await;
                let escaped = path.display().to_string().replace('\'', "''");
                nvim.command(&format!("silent! edit \'{escaped}\'"))
                    .await
                    .context("nvim edit")?;
                let buf = nvim.get_current_buf().await.context("current buf")?;
                let lines = buf.get_lines(0, -1, true).await.context("get_lines")?;
                Ok::<_, anyhow::Error>(lines.join("\n"))
            })
            .await
            .map_err(|_| anyhow::anyhow!("nvim read timed out"))?
        })?;
        Ok(Some(text))
    }
}
