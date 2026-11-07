/// Git status tracking for file/folder change highlighting in the explorer.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git status for a path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
    Conflict,
    Clean,
}

/// Represents a git commit with metadata
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub files_changed: Vec<String>,
}

/// Options for fetching git history
#[derive(Debug, Default, Clone)]
pub struct GitHistoryOptions {
    pub limit: usize,
    pub skip: usize,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub path_filter: Option<String>,
}

/// Line-level change info for a single file
#[derive(Debug, Default, Clone)]
pub struct FileDiff {
    /// Set of line numbers that were added (1-indexed)
    pub added_lines: HashSet<usize>,
    /// Set of line numbers that were modified (1-indexed)
    pub modified_lines: HashSet<usize>,
}

impl FileDiff {
    pub fn is_empty(&self) -> bool {
        self.added_lines.is_empty() && self.modified_lines.is_empty()
    }
}

impl GitCommit {
    /// Fetch commits from a repository
    pub fn fetch(repo_path: &Path, opts: &GitHistoryOptions) -> anyhow::Result<Vec<GitCommit>> {
        let mut args = vec!["log".to_string(), "--format=%H|%an|%ct|%s".to_string()];

        if let Some(since) = opts.since {
            args.push(format!("--since={}", since.timestamp()));
        }
        if let Some(until) = opts.until {
            args.push(format!("--until={}", until.timestamp()));
        }
        if let Some(ref path) = opts.path_filter {
            args.push("--".to_string());
            args.push(path.clone());
        }

        args.push(format!("-n{}", opts.limit));
        if opts.skip > 0 {
            args.push(format!("--skip={}", opts.skip));
        }

        let output = std::process::Command::new("git")
            .args(["-C"])
            .arg(repo_path)
            .args(&args)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("git log failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                continue;
            }

            let hash = parts[0].to_string();
            let author = parts[1].to_string();
            let timestamp = parts[2].parse::<i64>()?;
            let message = parts[3..].join("|");

            // Get files changed
            let files_output = std::process::Command::new("git")
                .args(["-C"])
                .arg(repo_path)
                .args(["show", "--name-only", "--format=", &hash])
                .output()?;

            let files_changed = if files_output.status.success() {
                String::from_utf8_lossy(&files_output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|s| s.replace('\\', "/"))
                    .collect()
            } else {
                Vec::new()
            };

            commits.push(GitCommit {
                short_hash: hash.chars().take(7).collect(),
                hash,
                author,
                date: chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or(chrono::Utc::now()),
                message,
                files_changed,
            });
        }

        Ok(commits)
    }
}

/// Tracks git-changed paths for the current repository.
#[derive(Debug, Default, Clone)]
pub struct GitChangeTracker {
    /// Paths with changes (relative to repo root, normalized with /)
    changed: HashSet<String>,
    /// Repo root this tracker is for
    repo_root: Option<PathBuf>,
    /// Line-level diffs for changed files (relative paths, normalized with /)
    line_changes: HashMap<String, FileDiff>,
}

impl GitChangeTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Refresh git status from disk. Returns true if status changed.
    pub fn refresh(&mut self, repo_root: &Path) -> bool {
        let old = self.changed.clone();
        self.changed.clear();
        self.repo_root = Some(repo_root.to_path_buf());

        // Get modified/staged/untracked files
        let output = Command::new("git")
            .args(["-C"])
            .arg(repo_root)
            .args(["status", "--porcelain=v1", "-unormal"])
            .output();

        let Ok(output) = output else {
            return old != self.changed;
        };
        if !output.status.success() {
            return old != self.changed;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }
            // porcelain format: XY <path> or XY <orig> -> <path>
            let path_part = &line[3..];
            let path = if let Some(arrow) = path_part.find(" -> ") {
                &path_part[arrow + 4..]
            } else {
                path_part
            };
            let path = path.trim().trim_matches('"');
            if !path.is_empty() {
                // Normalize to forward slashes for consistent matching
                let normalized = path.replace('\\', "/");
                self.changed.insert(normalized.clone());
                // Also mark all parent directories as changed
                let mut parent = Path::new(&normalized);
                while let Some(p) = parent.parent() {
                    let ps = p.to_string_lossy().replace('\\', "/");
                    if ps.is_empty() || ps == "." {
                        break;
                    }
                    self.changed.insert(ps);
                    parent = p;
                }
            }
        }

        old != self.changed
    }

    /// Refresh line-level changes for all changed files
    pub fn refresh_all_line_changes(&mut self, repo_path: &Path) {
        let mut paths_to_update: Vec<PathBuf> = vec![];

        if let Some(root) = &self.repo_root {
            for changed_path in &self.changed {
                let full_path = root.join(changed_path.replace('/', "\\"));
                if full_path.exists() && full_path.is_file() {
                    paths_to_update.push(full_path);
                }
            }
        }

        for path in paths_to_update {
            self.update_line_changes(repo_path, &path);
        }
    }

    /// Update line-level changes for a specific file
    fn update_line_changes(&mut self, repo_path: &Path, file_path: &Path) {
        // Get relative path first
        let rel_str = if let Some(root) = &self.repo_root {
            match file_path.strip_prefix(root) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => return,
            }
        } else {
            return;
        };

        // Parse git diff to find added and modified lines
        // - Lines with + prefix and no corresponding - in the same hunk = added
        // - Lines where - and + appear together for the same position = modified
        // - Deleted-only lines are ignored (per user request)
        let output = Command::new("git")
            .args(["-C"])
            .arg(repo_path)
            .args(["diff", "--unified=0", "--", &file_path.to_string_lossy()])
            .output();

        let Ok(output) = output else {
            return;
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut added_lines: HashSet<usize> = HashSet::new();
        let mut modified_lines: HashSet<usize> = HashSet::new();

        let mut in_hunk = false;
        let mut new_line = 0usize;
        let mut hunk_has_deletions = false;
        let mut hunk_new_lines: Vec<usize> = Vec::new();

        for line in stdout.lines() {
            if line.starts_with("@@") {
                // Flush previous hunk
                if !hunk_new_lines.is_empty() {
                    if hunk_has_deletions {
                        // Hunk had both deletions and additions -> treat additions as modifications
                        for ln in hunk_new_lines.drain(..) {
                            modified_lines.insert(ln);
                        }
                    } else {
                        // Only additions -> purely added lines
                        for ln in hunk_new_lines.drain(..) {
                            added_lines.insert(ln);
                        }
                    }
                }
                hunk_has_deletions = false;

                // Parse: @@ -old_start[,old_count] +new_start[,new_count] @@
                in_hunk = true;
                new_line = 0;
                if let Some(plus_pos) = line.find('+') {
                    let rest = &line[plus_pos + 1..];
                    let end = rest.find(|c: char| c == ',' || c == ' ' || c == '@').unwrap_or(rest.len());
                    if let Ok(n) = rest[..end].parse::<usize>() {
                        new_line = n;
                    }
                }
                continue;
            }

            if !in_hunk {
                continue;
            }

            if line.starts_with('+') && !line.starts_with("+++") {
                if new_line > 0 {
                    hunk_new_lines.push(new_line);
                }
                new_line += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                hunk_has_deletions = true;
                // Don't increment new_line for deleted lines
            } else if line.starts_with('\\') {
                // "\ No newline at end of file" - skip
            }
            // Context lines (shouldn't appear with --unified=0, but ignore safely)
        }

        // Flush last hunk
        if !hunk_new_lines.is_empty() {
            if hunk_has_deletions {
                for ln in hunk_new_lines.drain(..) {
                    modified_lines.insert(ln);
                }
            } else {
                for ln in hunk_new_lines.drain(..) {
                    added_lines.insert(ln);
                }
            }
        }

        self.line_changes.insert(
            rel_str,
            FileDiff {
                added_lines,
                modified_lines,
            },
        );
    }

    /// Get line-level diff for a file
    pub fn get_line_diff(&self, path: &Path) -> Option<&FileDiff> {
        let rel_str = path
            .strip_prefix(self.repo_root.as_ref()?)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        self.line_changes.get(&rel_str)
    }

    /// Check if a path (absolute) has git changes.
    pub fn is_changed(&self, path: &Path) -> bool {
        let Some(root) = &self.repo_root else {
            return false;
        };
        let Ok(rel) = path.strip_prefix(root) else {
            return false;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if rel_str.is_empty() {
            return false;
        }
        self.changed.contains(&rel_str)
    }

    /// Check if any child under this directory path has changes.
    pub fn has_changed_children(&self, dir_path: &Path) -> bool {
        let Some(root) = &self.repo_root else {
            return false;
        };
        let Ok(rel) = dir_path.strip_prefix(root) else {
            return false;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if rel_str.is_empty() {
            return !self.changed.is_empty();
        }
        let rel_str_trimmed = rel_str.trim_end_matches('/');
        let prefix = format!("{}/", rel_str_trimmed);
        self.changed.iter().any(|p| p == rel_str_trimmed || p.starts_with(&prefix))
    }

    pub fn clear(&mut self) {
        self.changed.clear();
        self.repo_root = None;
        self.line_changes.clear();
    }
}
