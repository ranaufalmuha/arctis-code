//! File import: copy dragged files into a project folder (PRD V1 §4.6).
//!
//! Used by both drag sources:
//! - internal tree drag (rearrange/copy within the project),
//! - external Finder/Desktop drop onto a tree folder.
//!
//! Never overwrites: name collisions get `name (2).ext` suffixes.
//! The destination must live inside the project root.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CopyOutcome {
    pub src: PathBuf,
    pub dest: Option<PathBuf>,
    pub renamed: bool,
    pub skipped: bool,
    pub error: Option<String>,
}

impl CopyOutcome {
    fn ok(src: PathBuf, dest: PathBuf, renamed: bool) -> Self {
        Self {
            src,
            dest: Some(dest),
            renamed,
            skipped: false,
            error: None,
        }
    }

    fn skipped(src: PathBuf, reason: String) -> Self {
        Self {
            src,
            dest: None,
            renamed: false,
            skipped: true,
            error: Some(reason),
        }
    }

    fn failed(src: PathBuf, error: String) -> Self {
        Self {
            src,
            dest: None,
            renamed: false,
            skipped: false,
            error: Some(error),
        }
    }

    pub fn succeeded(&self) -> bool {
        self.dest.is_some()
    }
}

fn is_within(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}

/// Pick a non-colliding destination inside `dir` for `file_name`.
fn unique_dest(dir: &Path, file_name: &str) -> (PathBuf, bool) {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return (candidate, false);
    }
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) if i > 0 => (&file_name[..i], Some(&file_name[i..])),
        _ => (file_name, None),
    };
    for n in 2..10000 {
        let alt = match ext {
            Some(e) => format!("{stem} ({n}){e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(&alt);
        if !candidate.exists() {
            return (candidate, true);
        }
    }
    // Practically unreachable; fall back to overwrite-free timestamp name.
    let alt = match ext {
        Some(e) => format!(
            "{stem} (copy {}){e}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ),
        None => format!("{stem} (copy)"),
    };
    (dir.join(alt), true)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(src).follow_links(false) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Copy `sources` into `dest_dir` (which must be inside `project_root`).
///
/// Never overwrites; always continues past per-file errors and reports
/// them in the returned outcomes.
pub fn copy_into(
    sources: &[PathBuf],
    dest_dir: &Path,
    project_root: &Path,
) -> anyhow::Result<Vec<CopyOutcome>> {
    let planned = plan_operations(sources, dest_dir, project_root, "copy")?;
    let mut outcomes = Vec::with_capacity(planned.len());
    for item in planned {
        match item {
            Planned::Done(outcome) => outcomes.push(outcome),
            Planned::Do(op) => {
                let result = if op.is_dir {
                    (|| -> anyhow::Result<()> {
                        std::fs::create_dir_all(&op.target)?;
                        copy_dir_recursive(&op.src_canon, &op.target)
                    })()
                    .map_err(|e| e.to_string())
                } else {
                    std::fs::copy(&op.src, &op.target)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                };
                match result {
                    Ok(()) => outcomes.push(CopyOutcome::ok(op.src, op.target, op.renamed)),
                    Err(e) => outcomes.push(CopyOutcome::failed(op.src, e)),
                }
            }
        }
    }
    Ok(outcomes)
}

/// Move `sources` into `dest_dir` (which must be inside `project_root`).
///
/// Same guards as [`copy_into`], but sources disappear from their old
/// location. Uses atomic `rename`, falling back to copy+delete across
/// filesystems.
pub fn move_into(
    sources: &[PathBuf],
    dest_dir: &Path,
    project_root: &Path,
) -> anyhow::Result<Vec<CopyOutcome>> {
    let planned = plan_operations(sources, dest_dir, project_root, "move")?;
    let mut outcomes = Vec::with_capacity(planned.len());
    for item in planned {
        match item {
            Planned::Done(outcome) => outcomes.push(outcome),
            Planned::Do(op) => match move_one(&op.src_canon, &op.target, op.is_dir) {
                Ok(()) => outcomes.push(CopyOutcome::ok(op.src, op.target, op.renamed)),
                Err(e) => outcomes.push(CopyOutcome::failed(op.src, e)),
            },
        }
    }
    Ok(outcomes)
}

fn move_one(src_canon: &Path, target: &Path, is_dir: bool) -> Result<(), String> {
    match std::fs::rename(src_canon, target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            // Cross-filesystem: copy, then delete the source.
            let copied = if is_dir {
                (|| -> anyhow::Result<()> {
                    std::fs::create_dir_all(target)?;
                    copy_dir_recursive(src_canon, target)
                })()
                .map_err(|e| e.to_string())
            } else {
                std::fs::copy(src_canon, target)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            };
            copied?;
            let removed = if is_dir {
                std::fs::remove_dir_all(src_canon).map_err(|e| e.to_string())
            } else {
                std::fs::remove_file(src_canon).map_err(|e| e.to_string())
            };
            removed?;
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

struct PlannedOp {
    src: PathBuf,
    src_canon: PathBuf,
    target: PathBuf,
    renamed: bool,
    is_dir: bool,
}

enum Planned {
    Do(PlannedOp),
    Done(CopyOutcome),
}

/// Shared validation for copy/move: destination checks, self-containment,
/// already-there skips, and collision-free (raw, non-canonical) targets so
/// reported paths match the tree's own paths.
fn plan_operations(
    sources: &[PathBuf],
    dest_dir: &Path,
    project_root: &Path,
    verb: &str,
) -> anyhow::Result<Vec<Planned>> {
    if !dest_dir.is_dir() {
        anyhow::bail!("drop target is not a folder: {}", dest_dir.display());
    }
    let root_canon = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let dest_canon = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());
    if !is_within(&dest_canon, &root_canon) {
        anyhow::bail!("drop target is outside the project: {}", dest_dir.display());
    }

    let mut planned = Vec::with_capacity(sources.len());
    for src in sources {
        // symlink_metadata covers files, dirs, and (dangling) symlinks.
        if std::fs::symlink_metadata(src).is_err() {
            planned.push(Planned::Done(CopyOutcome::failed(
                src.clone(),
                "source no longer exists".to_owned(),
            )));
            continue;
        }
        // Canonicalize for self-containment checks; fall back to raw path.
        let src_canon = src.canonicalize().unwrap_or_else(|_| src.clone());
        let src_is_dir = src_canon.is_dir();

        // Moving/copying a folder into itself or one of its descendants.
        if src_is_dir && is_within(&dest_canon, &src_canon) {
            planned.push(Planned::Done(CopyOutcome::skipped(
                src.clone(),
                format!("can't {verb} a folder into itself"),
            )));
            continue;
        }
        // Source already lives directly in the destination.
        let parent_canon = src
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .or_else(|| src.parent().map(|p| p.to_path_buf()));
        if parent_canon.map(|p| p == dest_canon).unwrap_or(false) {
            planned.push(Planned::Done(CopyOutcome::skipped(
                src.clone(),
                "already in this folder".to_owned(),
            )));
            continue;
        }

        let file_name = match src.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_owned(),
            None => {
                planned.push(Planned::Done(CopyOutcome::failed(
                    src.clone(),
                    "unsupported file name".to_owned(),
                )));
                continue;
            }
        };
        let (target, renamed) = unique_dest(dest_dir, &file_name);
        planned.push(Planned::Do(PlannedOp {
            src: src.clone(),
            src_canon,
            target,
            renamed,
            is_dir: src_is_dir,
        }));
    }
    Ok(planned)
}
