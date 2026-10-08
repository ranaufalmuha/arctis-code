//! File-safety guards (PRD V1 §4.8).
//!
//! Never silently overwrite data: track mtime + size, warn on external
//! modification, handle deleted/moved files gracefully.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub mtime: Option<SystemTime>,
    pub len: u64,
}

impl FileSnapshot {
    pub fn capture(path: &Path) -> Self {
        let (mtime, len) = fs::metadata(path)
            .map(|m| (m.modified().ok(), m.len()))
            .unwrap_or((None, 0));
        Self {
            path: path.to_path_buf(),
            mtime,
            len,
        }
    }

    /// True if the file changed on disk since this snapshot.
    pub fn is_stale(&self) -> bool {
        let current = Self::capture(&self.path);
        current.mtime != self.mtime || current.len != self.len
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Files larger than this are not loaded into the editor.
pub const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum FileContent {
    Text { content: String, lossy: bool },
    Unsupported { reason: String },
}

/// Read a file the way click-to-open needs: never fails for content reasons.
/// Directories, binaries, and oversized files come back as `Unsupported`
/// with a human-readable reason; only real I/O errors (missing file,
/// permissions) are `Err`.
pub fn read_smart(path: &Path) -> Result<(FileContent, FileSnapshot)> {
    let snap = FileSnapshot::capture(path);
    let meta = fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
    if meta.file_type().is_dir() {
        return Ok((
            FileContent::Unsupported {
                reason: "This is a folder, not a file.".to_owned(),
            },
            snap,
        ));
    }
    if meta.len() > MAX_TEXT_BYTES {
        return Ok((
            FileContent::Unsupported {
                reason: format!(
                    "File is too large to preview ({:.1} MB, limit {:.0} MB).",
                    meta.len() as f64 / 1_048_576.0,
                    MAX_TEXT_BYTES as f64 / 1_048_576.0
                ),
            },
            snap,
        ));
    }
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    // NUL byte in the probe window => binary.
    if bytes.iter().take(8192).any(|b| *b == 0) {
        return Ok((
            FileContent::Unsupported {
                reason: "This looks like a binary file, so it can't be edited as text.".to_owned(),
            },
            snap,
        ));
    }
    match String::from_utf8(bytes) {
        Ok(content) => Ok((
            FileContent::Text {
                content,
                lossy: false,
            },
            snap,
        )),
        Err(e) => Ok((
            FileContent::Text {
                content: String::from_utf8_lossy(e.as_bytes()).into_owned(),
                lossy: true,
            },
            snap,
        )),
    }
}

/// Write only if `expected` still matches disk, unless `force` is set.
/// Returns an error describing the conflict so the UI can show a dialog.
pub fn write_text_guarded(
    path: &Path,
    expected: &FileSnapshot,
    content: &str,
    force: bool,
) -> Result<FileSnapshot> {
    if !force {
        let current = FileSnapshot::capture(path);
        if current.mtime != expected.mtime || current.len != expected.len {
            anyhow::bail!(
                "file changed on disk since opened ({}). Review before overwriting.",
                path.display()
            );
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(FileSnapshot::capture(path))
}
