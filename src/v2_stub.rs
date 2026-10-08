//! V2 extension points (stubs only — zero runtime cost in V1).
//!
//! PRD V2: LSP (P1), AI diff (P2), integrated terminal (P3). These traits
//! define the seam so V2 adds files instead of restructuring V1.

use std::path::PathBuf;

/// Lazily-spawned language server per filetype. V1: not implemented.
pub trait LspClient: Send {
    fn filetypes(&self) -> &[&str];
    fn start_if_needed(&mut self, _filetype: &str) {}
}

/// Optional AI edit: context in, reviewable diff out. Never auto-applies.
pub trait AiProvider: Send {
    fn propose_edit(&self, _file: &PathBuf, _selection: &str) -> anyhow::Result<String> {
        anyhow::bail!("AI disabled in V1")
    }
}

/// Optional terminal session. V1 has no integrated terminal by design.
pub trait TerminalSession: Send {
    fn spawn(_cwd: PathBuf) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        anyhow::bail!("terminal not available in V1 — use Ghostty")
    }
}
