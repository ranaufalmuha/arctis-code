/// Git status tracking for file/folder change highlighting in the explorer.
use std::collections::HashSet;
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
    }
}
