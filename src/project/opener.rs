//! Project opening: dialog, CLI arg, and Finder drag-drop routing.
//!
//! PRD V1 §4.2 / §4.6: folder → project root, file → open tab,
//! multi-drop → open supported files.

use std::path::PathBuf;

#[derive(Debug)]
pub enum DropAction {
    OpenProject(PathBuf),
    OpenFiles(Vec<PathBuf>),
    None,
}

/// Classify an OS file-drop event.
pub fn classify_dropped(paths: &[PathBuf]) -> DropAction {
    if paths.is_empty() {
        return DropAction::None;
    }
    if paths.len() == 1 && paths[0].is_dir() {
        return DropAction::OpenProject(paths[0].clone());
    }
    let files: Vec<PathBuf> = paths.iter().filter(|p| p.is_file()).cloned().collect();
    // Multi-drop that includes a folder: first folder wins as project root.
    if files.is_empty() {
        if let Some(dir) = paths.iter().find(|p| p.is_dir()) {
            return DropAction::OpenProject(dir.clone());
        }
        return DropAction::None;
    }
    DropAction::OpenFiles(files)
}

pub fn pick_folder_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

pub fn pick_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_file()
}
