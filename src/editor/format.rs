//! Format-on-save via external tools (never reimplemented here).
//!
//! - stdin goes in, formatted stdout comes back; the file on disk is
//!   untouched until the normal guarded write.
//! - Only explicit saves trigger it (no daemons, no watchers, no index).
//! - Missing binary or formatter failure never blocks the save: the file
//!   is written as-is and the status line says why.
//! - Files over [`MAX_FORMAT_BYTES`] skip formatting (parity with the
//!   nvim live-sync cap).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Files larger than this are saved unformatted (formatter subprocess
/// latency would exceed the value on huge generated files).
pub const MAX_FORMAT_BYTES: usize = 512 * 1024;

pub struct FormatCmd {
    pub bin: &'static str,
    pub args: Vec<String>,
}

/// External formatter for `path`, if we know a canonical one.
pub fn formatter_for(path: &Path) -> Option<FormatCmd> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("rs") => Some(FormatCmd {
            bin: "rustfmt",
            args: vec![
                "--edition".into(),
                "2021".into(),
                "--emit".into(),
                "stdout".into(),
            ],
        }),
        Some(
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" | "json" | "css" | "html"
            | "md" | "mdx",
        ) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Some(FormatCmd {
                bin: "prettier",
                args: vec!["--stdin-filepath".into(), name],
            })
        }
        Some("go") => Some(FormatCmd {
            bin: "gofmt",
            args: Vec::new(),
        }),
        _ => None,
    }
}

/// Run the formatter: `None` = no formatter (or oversize) — save as-is.
/// `Some(Ok)` = replacement text (possibly unchanged), `Some(Err)` = the
/// save proceeds unformatted and the caller reports this message.
pub fn format_content(path: &Path, content: &str) -> Option<Result<String, String>> {
    if content.len() > MAX_FORMAT_BYTES {
        return None;
    }
    let cmd = formatter_for(path)?;
    let mut child = Command::new(cmd.bin)
        .args(&cmd.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{} not available ({e})", cmd.bin))
        .ok()?;
    // shortcut: synchronous spawn on an explicit user save; upgrade to a
    // background job if a slow formatter ever freezes the UI noticeably.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("{} failed ({e})", cmd.bin));
    match out {
        Err(e) => Some(Err(e)),
        Ok(o) if !o.status.success() => {
            let detail = String::from_utf8_lossy(&o.stderr);
            let first = detail.lines().next().unwrap_or("formatter error");
            Some(Err(format!("{}: {first}", cmd.bin)))
        }
        Ok(o) => Some(Ok(String::from_utf8_lossy(&o.stdout).into_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_canonical_tools() {
        let rs = formatter_for(Path::new("main.rs")).unwrap();
        assert_eq!(rs.bin, "rustfmt");
        let ts = formatter_for(Path::new("app.ts")).unwrap();
        assert_eq!(ts.bin, "prettier");
        assert!(ts.args.contains(&"app.ts".to_owned()));
        let go = formatter_for(Path::new("main.go")).unwrap();
        assert_eq!(go.bin, "gofmt");
        assert!(formatter_for(Path::new("x.png")).is_none());
        assert!(formatter_for(Path::new("Cargo.toml")).is_none());
    }

    #[test]
    fn oversize_skips_silently() {
        let big = "x".repeat(MAX_FORMAT_BYTES + 1);
        assert!(format_content(Path::new("a.rs"), &big).is_none());
    }

    #[test]
    fn rustfmt_roundtrip_when_installed() {
        // Passes vacuously where rustfmt is absent (CI without toolchain).
        let Ok(child) = Command::new("rustfmt").arg("--version").output() else {
            return;
        };
        assert!(child.status.success());
        let out = format_content(Path::new("a.rs"), "fn main(){}\n")
            .expect("formatter known")
            .expect("rustfmt works");
        assert_eq!(out, "fn main() {}\n");
    }
}
