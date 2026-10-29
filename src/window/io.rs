//! One repo window: filesystem + tab I/O (open/save/move/drop).
use super::RepoWindow;
use crate::editor::NeovimBackend;
use crate::preview::ImageCache;
use crate::project::entry::{create_entry, EntryKind};
use crate::project::import::{copy_into, move_into};
use crate::project::opener::{self, DropAction};
use crate::safety::write_text_guarded;
use crate::tabs::TabKind;
use std::path::PathBuf;

impl RepoWindow {
    pub(super) fn open_project(&mut self, root: PathBuf) {
        self.filter_cache.clear();
        self.selected_paths.clear();
        self.selection_anchor = None;
        self.visible_paths.clear();
        self.previous_visible_paths.clear();
        self.copied_paths.clear();
        self.rename_draft = None;
        self.delete_pending = None;
        self.project.root = Some(root.clone());
        self.tree.set_root(root.clone());
        if let Err(e) = self.watcher.watch(&root) {
            self.status = format!("watch failed: {e:#}");
        } else {
            self.status = format!("Project: {}", root.display());
        }
        self.refresh_tasks();
    }
    pub(super) fn open_file(&mut self, path: PathBuf, nvim: &mut NeovimBackend) {
        if crate::preview::is_image(&path) || matches!(crate::tabs::classify(&path), TabKind::Image)
        {
            match self.tabs.open(&path) {
                Ok(()) => self.status = format!("Preview: {}", path.display()),
                Err(e) => self.report_open_error(&path, e),
            }
            return;
        }
        match self.tabs.open(&path) {
            Ok(()) => {
                // Sync buffer into embedded Neovim so edits stay Neovim-backed.
                if let Some(tab) = self.tabs.active_ref() {
                    nvim.sync_buffer(&tab.path, &tab.content);
                }
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.report_open_error(&path, e),
        }
    }
    pub(super) fn report_open_error(&mut self, path: &std::path::Path, e: anyhow::Error) {
        let msg = format!("Couldn't open {}:\n{e:#}", path.display());
        eprintln!("ArctisCode: {msg}");
        self.status = format!("open failed: {e:#}");
        self.open_error = Some(msg);
    }
    pub(super) fn save_active(
        &mut self,
        nvim: &mut NeovimBackend,
        format_on_save: bool,
        force: bool,
    ) {
        let Some(tab) = self.tabs.active_mut() else {
            return;
        };
        if tab.kind != TabKind::Text {
            return;
        }
        let mut formatted = false;
        let mut fmt_issue: Option<String> = None;
        if format_on_save {
            match crate::editor::format::format_content(&tab.path, &tab.content) {
                Some(Ok(out)) => {
                    if out != tab.content {
                        tab.content = out;
                        formatted = true;
                    }
                }
                Some(Err(e)) => fmt_issue = Some(e),
                None => {}
            }
        }
        match write_text_guarded(&tab.path, &tab.snapshot, &tab.content, force) {
            Ok(snap) => {
                tab.snapshot = snap;
                tab.dirty = false;
                self.status = match (formatted, fmt_issue) {
                    (true, _) => format!("Saved {} (formatted)", tab.path.display()),
                    (false, Some(e)) => format!("Saved {} (formatter: {e})", tab.path.display()),
                    _ => format!("Saved {}", tab.path.display()),
                };
                // Keep the embedded backend honest after a reformat.
                nvim.sync_buffer(&tab.path.clone(), &tab.content.clone());
            }
            Err(e) => {
                self.status = format!("save blocked: {e:#}");
                self.confirm_overwrite = Some(tab.path.clone());
            }
        }
    }
    /// Save every dirty text tab. Files blocked by external modification
    /// surface the overwrite guard and stay dirty (caller re-checks).
    pub(super) fn save_all_text_tabs(&mut self, nvim: &mut NeovimBackend, format_on_save: bool) {
        let dirty_paths: Vec<PathBuf> = self
            .tabs
            .tabs
            .iter()
            .filter(|t| t.dirty && t.kind == TabKind::Text)
            .map(|t| t.path.clone())
            .collect();
        for path in dirty_paths {
            if let Some(i) = self.tabs.tabs.iter().position(|t| t.path == path) {
                self.tabs.active = Some(i);
                self.save_active(nvim, format_on_save, false);
            }
        }
    }
    pub(super) fn discard_all_tabs(&mut self) {
        self.tabs.tabs.clear();
        self.tabs.active = None;
    }
    /// Copy `sources` into project folder `dest_dir`, refresh the tree,
    /// and summarize the result in the status bar.
    pub(super) fn import_dropped_onto(
        &mut self,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        nvim: &mut NeovimBackend,
    ) {
        let Some(root) = self.project.root.clone() else {
            self.status = "Open a project folder first.".to_owned();
            return;
        };
        match copy_into(&sources, &dest_dir, &root) {
            Ok(outcomes) => self.finish_transfer(&outcomes, &dest_dir, "Copied", nvim),
            Err(e) => {
                self.status = format!("copy failed: {e:#}");
            }
        }
    }
    /// Move internal tree drags into `dest_dir`: same guards as copy, but
    /// sources disappear and open tabs follow them to the new location.
    pub(super) fn move_dropped_onto(
        &mut self,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        nvim: &mut NeovimBackend,
    ) {
        let Some(root) = self.project.root.clone() else {
            self.status = "Open a project folder first.".to_owned();
            return;
        };
        match move_into(&sources, &dest_dir, &root) {
            Ok(outcomes) => {
                for o in outcomes.iter().filter(|o| o.succeeded()) {
                    if let Some(new) = o.dest.clone() {
                        self.tabs.retarget_moved(&o.src, &new);
                        // A moved file keeps its buffer; re-sync nvim.
                        // (Image-cache invalidation for the moved source
                        // happens at the call sites, which own the cache.)
                        if let Some(tab) = self.tabs.tabs.iter().find(|t| t.path == new) {
                            nvim.sync_buffer(&new, &tab.content.clone());
                        }
                        if self.tree.expanded.remove(&o.src) {
                            self.tree.expanded.insert(new.clone());
                        }
                        if self.tree.selected.as_ref() == Some(&o.src) {
                            self.tree.selected = Some(new.clone());
                        } else if let Some(sel) = self.tree.selected.clone() {
                            if let Ok(rel) = sel.strip_prefix(&o.src) {
                                self.tree.selected = Some(new.join(rel));
                            }
                        }
                    }
                }
                self.finish_transfer(&outcomes, &dest_dir, "Moved", nvim);
            }
            Err(e) => {
                self.status = format!("move failed: {e:#}");
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_transfer(
        &mut self,
        outcomes: &[crate::project::import::CopyOutcome],
        dest_dir: &std::path::Path,
        verb_past: &str,
        nvim: &mut NeovimBackend,
    ) {
        let ok: Vec<_> = outcomes.iter().filter(|o| o.succeeded()).collect();
        let skipped = outcomes.iter().filter(|o| o.skipped).count();
        let failed = outcomes
            .iter()
            .filter(|o| !o.succeeded() && !o.skipped)
            .count();
        self.refresh_tree();
        self.tree.expanded.insert(dest_dir.to_path_buf());
        self.selected_paths.clear();
        self.selection_anchor = Some(dest_dir.to_path_buf());
        self.tree.selected = Some(dest_dir.to_path_buf());

        let dest_name = dest_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| dest_dir.display().to_string());
        if ok.len() == 1 && skipped == 0 && failed == 0 {
            let dest = ok[0].dest.clone().unwrap_or_else(|| dest_dir.to_path_buf());
            let name = dest
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            // Auto-open a single transferred text file.
            if crate::tabs::classify(&dest) == TabKind::Text {
                self.open_file(dest, nvim);
            }
            self.status = format!("{verb_past} {name} → {dest_name}/");
        } else {
            let renamed = ok.iter().filter(|o| o.renamed).count();
            let mut parts = vec![format!("{verb_past} {} → {dest_name}/", ok.len())];
            if renamed > 0 {
                parts.push(format!("{renamed} renamed"));
            }
            if skipped > 0 {
                let reasons: Vec<String> = outcomes
                    .iter()
                    .filter(|o| o.skipped)
                    .filter_map(|o| o.error.clone())
                    .take(1)
                    .collect();
                parts.push(format!("{skipped} skipped ({})", reasons.join("; ")));
            }
            if failed > 0 {
                parts.push(format!("{failed} failed"));
            }
            self.status = parts.join(", ");
        }
    }
    /// Folder a new file/folder lands in: selected folder, parent of the
    /// selected file, else the project root.
    pub(super) fn new_entry_target(&self) -> Option<PathBuf> {
        let root = self.project.root.clone()?;
        match &self.tree.selected {
            Some(p) if p.is_dir() => Some(p.clone()),
            Some(p) => Some(p.parent().map(|par| par.to_path_buf()).unwrap_or(root)),
            None => Some(self.tabs.active_ref()
                .filter(|tab| tab.path.starts_with(&root))
                .and_then(|tab| tab.path.parent())
                .map(|parent| parent.to_path_buf())
                .unwrap_or(root)),
        }
    }
    pub(super) fn commit_new_entry(&mut self, nvim: &mut NeovimBackend) {
        let (kind, name) = match &self.new_entry {
            Some(d) => (d.kind, d.name.clone()),
            None => return,
        };
        let Some(target) = self.new_entry.as_ref().map(|draft| draft.target.clone()) else {
            return;
        };
        match create_entry(&target, kind, &name) {
            Ok(path) => {
                self.refresh_tree();
                self.tree.expanded.insert(target.clone());
                self.selected_paths.clear();
                self.selected_paths.insert(path.clone());
                self.selection_anchor = Some(path.clone());
                self.tree.selected = Some(path.clone());
                self.new_entry = None;
                let what = if matches!(kind, EntryKind::File) {
                    "File"
                } else {
                    "Folder"
                };
                self.status = format!("{} created: {}", what, path.display());
                if matches!(kind, EntryKind::File) {
                    self.open_file(path, nvim);
                }
            }
            Err(e) => {
                if let Some(d) = self.new_entry.as_mut() {
                    d.error = Some(format!("{e:#}"));
                    d.focus_requested = false;
                }
            }
        }
    }
    pub(super) fn paste_explorer_items(&mut self, dest: PathBuf, nvim: &mut NeovimBackend) {
        let Some(root) = self.project.root.clone() else { return; };
        match crate::project::import::copy_explorer_items(&self.copied_paths, &dest, &root) {
            Ok(outcomes) => self.finish_transfer(&outcomes, &dest, "Copied", nvim),
            Err(e) => self.status = format!("copy failed: {e:#}"),
        }
    }

    pub(super) fn commit_rename(&mut self, nvim: &mut NeovimBackend, images: &mut ImageCache) {
        let Some(draft) = self.rename_draft.as_ref() else { return; };
        let old = draft.path.clone();
        let result = (|| -> anyhow::Result<PathBuf> {
            let name = crate::project::entry::validate_name(&draft.name).map_err(anyhow::Error::msg)?;
            let parent = old.parent().ok_or_else(|| anyhow::anyhow!("No parent folder"))?;
            let target = parent.join(name);
            if target == old { return Ok(target); }
            if std::fs::symlink_metadata(&target).is_ok() { anyhow::bail!("An item with that name already exists."); }
            std::fs::rename(&old, &target)?;
            Ok(target)
        })();
        match result {
            Ok(target) => {
                if target != old {
                    self.tabs.retarget_moved(&old, &target);
                    for tab in &self.tabs.tabs {
                        if tab.path.starts_with(&target) && tab.kind == TabKind::Text { nvim.sync_buffer(&tab.path, &tab.content); }
                    }
                    images.invalidate(&old);
                    let expanded = std::mem::take(&mut self.tree.expanded);
                    self.tree.expanded = expanded.into_iter().map(|p| p.strip_prefix(&old).map(|r| target.join(r)).unwrap_or(p.clone())).collect();
                    self.refresh_tree();
                    self.selected_paths.clear();
                    self.selected_paths.insert(target.clone());
                    self.selection_anchor = Some(target.clone());
                    self.tree.selected = Some(target.clone());
                    self.status = format!("Renamed to {}", target.display());
                }
                self.rename_draft = None;
            }
            Err(e) => {
                self.status = format!("Rename failed: {e:#}");
                if let Some(draft) = self.rename_draft.as_mut() { draft.error = Some(format!("{e:#}")); draft.focus_requested = false; }
            }
        }
    }

    pub(super) fn delete_selected_entries(&mut self, paths: Vec<PathBuf>, images: &mut ImageCache) {
        if self.tabs.tabs.iter().any(|tab| tab.dirty && paths.iter().any(|path| tab.path.starts_with(path))) {
            self.status = "Save or close unsaved files before deleting them.".to_owned();
            return;
        }
        let mut errors = Vec::new();
        for path in &paths {
            let result = std::fs::symlink_metadata(path).and_then(|meta| {
                if meta.file_type().is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) }
            });
            match result {
                Ok(()) => {
                    images.invalidate(path);
                    for i in (0..self.tabs.tabs.len()).rev() {
                        if self.tabs.tabs[i].path.starts_with(path) { self.tabs.close(i); }
                    }
                    self.tree.expanded.retain(|p| !p.starts_with(path));
                    self.selected_paths.retain(|p| !p.starts_with(path));
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        self.tree.selected = self.selected_paths.iter().next().cloned();
        self.selection_anchor = self.tree.selected.clone();
        self.refresh_tree();
        self.status = if errors.is_empty() { format!("Deleted {} item(s)", paths.len()) } else { format!("Delete failed: {}", errors.join("; ")) };
    }

    pub(super) fn drop_target_under_pointer(&self, ctx: &egui::Context) -> Option<PathBuf> {
        let pos = ctx.input(|i| i.pointer.hover_pos())?;
        // Deepest (smallest) folder rect wins for nested folders.
        let mut best: Option<&(egui::Rect, PathBuf)> = None;
        for entry in &self.folder_drop_rects {
            if entry.0.contains(pos) {
                let smaller = match &best {
                    None => true,
                    Some(b) => entry.0.area() < b.0.area(),
                };
                if smaller {
                    best = Some(entry);
                }
            }
        }
        best.map(|b| b.1.clone())
    }
    pub(super) fn handle_drops(
        &mut self,
        ctx: &egui::Context,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
    ) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if dropped.is_empty() {
            return;
        }
        // External files dropped onto a tree folder → copy them there.
        if let Some(target) = self.drop_target_under_pointer(ctx) {
            // Moved-away images must not linger in the shared cache.
            for src in &dropped {
                images.invalidate(src);
            }
            self.import_dropped_onto(dropped, target, nvim);
            return;
        }
        match opener::classify_dropped(&dropped) {
            DropAction::OpenProject(root) => self.open_project(root),
            DropAction::OpenFiles(files) => {
                for f in files {
                    self.open_file(f, nvim);
                }
            }
            DropAction::None => {}
        }
    }
}
