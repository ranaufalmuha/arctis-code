//! New file / folder creation (explorer header buttons).
//!
//! Names are validated before touching disk; collisions get `name (2)`
//! suffixes so creation never overwrites (PRD V1 §4.8).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Folder,
}

/// Validate a user-typed entry name. Returns the cleaned name or a message
/// suitable for inline display in the dialog.
pub fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim().replace('\\', "/");
    let trimmed = trimmed.trim_matches('/').trim().to_owned();
    if trimmed.is_empty() {
        return Err("Enter a name.".to_owned());
    }
    if trimmed.contains('/') {
        return Err("Nested paths aren't supported yet — pick a plain name.".to_owned());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("That name isn't allowed.".to_owned());
    }
    if trimmed.len() > 255 {
        return Err("That name is too long.".to_owned());
    }
    Ok(trimmed)
}

fn unique_sibling(dir: &Path, name: &str, is_dir: bool) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = if !is_dir {
        match name.rfind('.') {
            Some(i) if i > 0 => (name[..i].to_owned(), Some(name[i..].to_owned())),
            _ => (name.to_owned(), None),
        }
    } else {
        (name.to_owned(), None)
    };
    for n in 2..10000 {
        let alt = match &ext {
            Some(e) => format!("{stem} ({n}){e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(&alt);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem} (copy)"))
}

/// Create a file or folder inside `dir` (must exist and be a dir).
/// Returns the created path.
pub fn create_entry(dir: &Path, kind: EntryKind, name: &str) -> anyhow::Result<PathBuf> {
    if !dir.is_dir() {
        anyhow::bail!("not a folder: {}", dir.display());
    }
    let clean = validate_name(name).map_err(|e| anyhow::anyhow!(e))?;
    let target = unique_sibling(dir, &clean, matches!(kind, EntryKind::Folder));
    match kind {
        EntryKind::File => {
            std::fs::write(&target, b"")?;
        }
        EntryKind::Folder => {
            std::fs::create_dir(&target)?;
        }
    }
    Ok(target)
}
