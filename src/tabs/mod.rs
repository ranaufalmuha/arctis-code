pub mod bar;

use crate::safety::{read_smart, FileContent, FileSnapshot};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabKind {
    Text,
    Image,
    Unsupported,
}

pub fn classify(path: &Path) -> TabKind {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg") => TabKind::Image,
        Some(
            "rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "md" | "toml" | "yaml" | "yml" | "css"
            | "html" | "py" | "go" | "txt" | "ron" | "lock" | "gitignore",
        )
        | None => TabKind::Text,
        _ => {
            // Unknown extensions: try text, fall back to unsupported in UI.
            TabKind::Text
        }
    }
}

/// VSCode-style language-mode label for the status bar.
pub fn language_label(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("rs") => "Rust",
        Some("ts" | "tsx" | "mts" | "cts") => "TypeScript",
        Some("js" | "jsx" | "mjs" | "cjs") => "JavaScript",
        Some("json") => "JSON",
        Some("md" | "markdown") => "Markdown",
        Some("toml") => "TOML",
        Some("yaml" | "yml") => "YAML",
        Some("css" | "scss" | "less") => "CSS",
        Some("html" | "htm") => "HTML",
        Some("py") => "Python",
        Some("go") => "Go",
        Some("sh" | "bash" | "zsh") => "Shell Script",
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg") => "Image",
        _ => "Plain Text",
    }
}

#[derive(Debug)]
pub struct Tab {
    pub path: PathBuf,
    pub kind: TabKind,
    pub content: String,
    pub snapshot: FileSnapshot,
    pub dirty: bool,
    /// Human-readable note shown above the content (e.g. lossy decode,
    /// or why an Unsupported file can't be edited).
    pub notice: Option<String>,
}

impl Tab {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let kind = classify(path);
        match kind {
            TabKind::Text => {
                let (content, snapshot) = read_smart(path)?;
                match content {
                    FileContent::Text { content, lossy } => Ok(Self {
                        path: path.to_path_buf(),
                        kind,
                        content,
                        snapshot,
                        dirty: false,
                        notice: lossy.then(|| {
                            "Opened with replacement characters (file is not UTF-8). Saving will write back UTF-8."
                                .to_owned()
                        }),
                    }),
                    FileContent::Unsupported { reason } => Ok(Self {
                        path: path.to_path_buf(),
                        kind: TabKind::Unsupported,
                        content: String::new(),
                        snapshot,
                        dirty: false,
                        notice: Some(reason),
                    }),
                }
            }
            TabKind::Image | TabKind::Unsupported => Ok(Self {
                path: path.to_path_buf(),
                kind,
                content: String::new(),
                snapshot: FileSnapshot::capture(path),
                dirty: false,
                notice: None,
            }),
        }
    }

    pub fn title(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    pub fn mark_edited(&mut self) {
        // Cheap dirty check: compare against snapshot length would false-positive;
        // the app sets dirty on TextEdit change and clears on save.
        self.dirty = true;
    }
}

#[derive(Debug, Default)]
pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active: Option<usize>,
}

impl TabManager {
    pub fn open(&mut self, path: &Path) -> anyhow::Result<()> {
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active = Some(idx);
            return Ok(());
        }
        let tab = Tab::open(path)?;
        self.tabs.push(tab);
        self.active = Some(self.tabs.len() - 1);
        Ok(())
    }

    pub fn close(&mut self, idx: usize) -> Option<Tab> {
        if idx >= self.tabs.len() {
            return None;
        }
        let removed = self.tabs.remove(idx);
        self.active = match self.active {
            Some(a) if a == idx => {
                if self.tabs.is_empty() {
                    None
                } else {
                    Some(idx.min(self.tabs.len() - 1))
                }
            }
            Some(a) if a > idx => Some(a - 1),
            other => other,
        };
        Some(removed)
    }

    pub fn active_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }

    pub fn active_ref(&self) -> Option<&Tab> {
        self.active.and_then(|i| self.tabs.get(i))
    }

    /// Point tabs at new locations after a move: exact matches plus anything
    /// nested under a moved folder. Disk already holds the buffer content at
    /// the new path, so snapshots are re-captured while dirty flags stay.
    pub fn retarget_moved(&mut self, old: &Path, new: &Path) {
        for tab in &mut self.tabs {
            let moved = if tab.path == old {
                Some(new.to_path_buf())
            } else {
                tab.path.strip_prefix(old).ok().map(|rel| new.join(rel))
            };
            if let Some(path) = moved {
                tab.path = path.clone();
                tab.snapshot = FileSnapshot::capture(&path);
            }
        }
    }
}
