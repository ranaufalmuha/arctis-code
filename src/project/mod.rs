pub mod entry;
pub mod import;
pub mod opener;
pub mod watcher;

use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Project {
    /// Current project root (folder opened via dialog / CLI / drop).
    pub root: Option<PathBuf>,
}

impl Project {
    pub fn new(initial: Option<String>) -> Self {
        let root = initial.and_then(|p| {
            let pb = PathBuf::from(&p);
            if pb.is_dir() {
                Some(pb)
            } else {
                pb.parent().map(|par| par.to_path_buf())
            }
        });
        Self { root }
    }

    pub fn title(&self) -> String {
        match &self.root {
            Some(r) => format!(
                "ArctisCode — {}",
                r.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| r.display().to_string())
            ),
            None => "ArctisCode".to_owned(),
        }
    }
}
