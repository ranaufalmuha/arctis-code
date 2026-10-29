//! Lazy file-tree explorer (PRD V1 §4.3).
//!
//! - Never scans the whole repo eagerly: children load on expand.
//! - Skips heavy dirs (`.git`, `node_modules`, `target`) unless expanded.
//! - Single selection + keyboard navigation handled by the caller.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[allow(dead_code)]
pub(crate) const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".venv", "dist", "build"];

/// True for heavy dirs that filter-search skips (tree still shows them).
pub fn is_skipped_dir(name: &str) -> bool {
    SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    pub depth: usize,
}

impl FileNode {
    fn new(path: PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            path,
            is_dir,
            children: None,
            depth,
        }
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

#[derive(Debug, Default)]
pub struct FileTree {
    pub root: Option<PathBuf>,
    pub expanded: HashSet<PathBuf>,
    pub selected: Option<PathBuf>,
    cache: HashMap<PathBuf, Vec<FileNode>>,
}

impl FileTree {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.expanded.clear();
        self.selected = None;
        self.cache.clear();
    }

    pub fn refresh(&mut self) {
        self.cache.clear();
        // Keep expansion state; children reload lazily on next frame.
    }

    fn list_dir(path: &Path, depth: usize) -> Vec<FileNode> {
        let mut entries: Vec<FileNode> = WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| {
                let p = e.path().to_path_buf();
                // Follow symlinks so a symlinked folder toggles like a
                // folder instead of erroring like a file on click.
                FileNode::new(p.clone(), p.is_dir(), depth + 1)
            })
            .collect();
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
        });
        entries
    }

    pub fn children_of(&mut self, dir: &Path, depth: usize) -> Vec<FileNode> {
        self.children_ref(dir, depth).to_vec()
    }

    /// Borrowed children; callers that recurse must copy out the paths
    /// they need first (the borrow ends before the recursive call).
    pub fn children_ref(&mut self, dir: &Path, depth: usize) -> &[FileNode] {
        if !self.cache.contains_key(dir) {
            let kids = Self::list_dir(dir, depth);
            self.cache.insert(dir.to_path_buf(), kids);
        }
        self.cache.get(dir).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn toggle(&mut self, dir: &Path) {
        if self.expanded.contains(dir) {
            self.expanded.remove(dir);
        } else {
            self.expanded.insert(dir.to_path_buf());
        }
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.expanded.contains(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavy_dirs_skip_filter_search() {
        assert!(is_skipped_dir(".git"));
        assert!(is_skipped_dir("node_modules"));
        assert!(is_skipped_dir("target"));
        assert!(!is_skipped_dir("src"));
    }
}
