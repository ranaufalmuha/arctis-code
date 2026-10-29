//! One repo window: project + tree + tabs + editor + image preview.
//!
//! The app shell (`app::ArctisCodeApp`) owns N of these — one per OS
//! viewport — plus the services shared across windows (theme, embedded
//! Neovim, image cache). Sharing is safe because the shared services are
//! path-keyed: the same file opened in two windows edits one Neovim
//! buffer, and concurrent saves meet the existing guarded-write dialog.

use crate::editor::NeovimBackend;
use crate::explorer::FileTree;
use crate::icons::{self, Glyph};
use crate::preview::ImageCache;
use crate::project::entry::{create_entry, EntryKind};
use crate::project::import::{copy_into, move_into};
use crate::project::opener::{self, DropAction};
use crate::project::watcher::ProjectWatcher;
use crate::project::Project;
use crate::safety::write_text_guarded;
use crate::settings::Settings;
use crate::tabs::{self, TabKind, TabManager};
use crate::tasks::{self, BoardState};
use crate::theme::{
    VscodeTheme, ACTIVITY_BAR_WIDTH, STATUS_BAR_HEIGHT, TREE_INDENT, TREE_ROW_HEIGHT,
};
use std::path::PathBuf;

/// Custom titlebar metrics: native traffic lights stay visible (macOS
/// fullsize content view); this left pad keeps our content clear of them.
pub const TITLEBAR_TRAFFIC_PAD: f32 = 78.0;
pub const TITLEBAR_HEIGHT: f32 = 38.0;
pub const TITLEBAR_SEARCH_W: f32 = 220.0;

/// What the window's center area shows. Explorer picks the editor (and
/// re-shows the sidebar); Tasks and Settings are full center pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CenterView {
    #[default]
    Editor,
    Tasks,
    Settings,
}

/// Internal tree drag payload: the path being dragged onto a folder.
#[derive(Debug, Clone)]
struct TreeDragPayload(PathBuf);

/// Pending new file / folder dialog state.
#[derive(Debug)]
pub(crate) struct NewEntryDialog {
    kind: EntryKind,
    name: String,
    error: Option<String>,
}

/// Requests a window wants the app shell to carry out (spawn / destroy OS
/// viewports). Rendered windows can't do that themselves mid-pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    NewWindow,
    CloseWindow(egui::ViewportId),
}

/// Deterministic viewport ids: `counter` is owned by the shell and bumps
/// once per spawned window, so ids are unique for the process lifetime.
pub fn fresh_viewport_id(counter: u64) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("arctis-window", counter))
}

pub struct RepoWindow {
    pub viewport: egui::ViewportId,
    pub project: Project,
    pub tree: FileTree,
    pub tabs: TabManager,
    pub watcher: ProjectWatcher,
    pub status: String,
    pub confirm_close: Option<usize>,
    pub confirm_overwrite: Option<PathBuf>,
    pub open_error: Option<String>,
    pub(crate) new_entry: Option<NewEntryDialog>,
    pub cursor_pos: Option<(usize, usize)>,
    pub find_query: String,
    /// Titlebar quick filter: substrings match file/folder names in the explorer.
    pub explorer_filter: String,
    /// Folder row rects from the last frame, for hover-aware drops.
    pub folder_drop_rects: Vec<(egui::Rect, PathBuf)>,
    /// Activity-bar Explorer toggle.
    pub sidebar_visible: bool,
    /// Explorer section (project folder) collapsed or expanded.
    pub explorer_open: bool,
    /// Active tab path seen last frame; resets the cursor readout on switch.
    pub last_tab_path: Option<PathBuf>,
    /// Window-level unsaved-changes guard (viewport close with dirty tabs).
    pub confirm_close_window: bool,
    /// Set by Save-all / Discard-all; closes once no dirty tab remains.
    pub close_after_save: bool,
    /// Center content: editor group, task board, or settings page.
    pub center_view: CenterView,
    /// Workspace task board + the file it persists to.
    pub tasks: BoardState,
    pub tasks_path: PathBuf,
}

impl RepoWindow {
    pub fn blank(viewport: egui::ViewportId) -> Self {
        Self {
            viewport,
            project: Project::default(),
            tree: FileTree::default(),
            tabs: TabManager::default(),
            watcher: ProjectWatcher::new(),
            status: String::from("Ready — open a folder (Cmd/Ctrl+O) or drop it here."),
            confirm_close: None,
            confirm_overwrite: None,
            open_error: None,
            new_entry: None,
            cursor_pos: None,
            find_query: String::new(),
            explorer_filter: String::new(),
            folder_drop_rects: Vec::new(),
            sidebar_visible: true,
            explorer_open: true,
            last_tab_path: None,
            confirm_close_window: false,
            close_after_save: false,
            center_view: CenterView::Editor,
            tasks: BoardState::new(tasks::load_tasks(&tasks::global_tasks_path()).0),
            tasks_path: tasks::global_tasks_path(),
        }
    }

    pub fn with_initial(viewport: egui::ViewportId, initial_path: Option<String>) -> Self {
        let mut window = Self::blank(viewport);
        window.project = Project::new(initial_path);
        if let Some(root) = &window.project.root {
            window.tree.set_root(root.clone());
            let _ = window.watcher.watch(root);
        }
        window.refresh_tasks();
        window
    }

    /// (Re)load the task board for the current project root (workspace
    /// file, or the global fallback with no folder open).
    fn refresh_tasks(&mut self) {
        self.tasks_path = tasks::tasks_path(self.project.root.as_deref());
        let (board, warning) = tasks::load_tasks(&self.tasks_path);
        self.tasks = BoardState::new(board);
        if let Some(w) = warning {
            self.status = w;
        }
    }

    /// Persist the board; failures land on the status line, never a dialog.
    fn save_tasks(&mut self) {
        if let Err(e) = tasks::save_tasks(&self.tasks_path, &self.tasks.board) {
            self.status = format!("tasks save failed: {e:#}");
        }
    }

    pub fn has_dirty_tabs(&self) -> bool {
        self.tabs.tabs.iter().any(|t| t.dirty)
    }

    fn open_project(&mut self, root: PathBuf) {
        self.project.root = Some(root.clone());
        self.tree.set_root(root.clone());
        if let Err(e) = self.watcher.watch(&root) {
            self.status = format!("watch failed: {e:#}");
        } else {
            self.status = format!("Project: {}", root.display());
        }
        self.refresh_tasks();
    }

    fn open_file(&mut self, path: PathBuf, nvim: &mut NeovimBackend) {
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

    fn report_open_error(&mut self, path: &std::path::Path, e: anyhow::Error) {
        let msg = format!("Couldn't open {}:\n{e:#}", path.display());
        eprintln!("ArctisCode: {msg}");
        self.status = format!("open failed: {e:#}");
        self.open_error = Some(msg);
    }

    fn save_active(&mut self, force: bool) {
        let Some(tab) = self.tabs.active_mut() else {
            return;
        };
        if tab.kind != TabKind::Text {
            return;
        }
        match write_text_guarded(&tab.path, &tab.snapshot, &tab.content, force) {
            Ok(snap) => {
                tab.snapshot = snap;
                tab.dirty = false;
                self.status = format!("Saved {}", tab.path.display());
            }
            Err(e) => {
                self.status = format!("save blocked: {e:#}");
                self.confirm_overwrite = Some(tab.path.clone());
            }
        }
    }

    /// Save every dirty text tab. Files blocked by external modification
    /// surface the overwrite guard and stay dirty (caller re-checks).
    fn save_all_text_tabs(&mut self) {
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
                self.save_active(false);
            }
        }
    }

    fn discard_all_tabs(&mut self) {
        self.tabs.tabs.clear();
        self.tabs.active = None;
    }

    /// Copy `sources` into project folder `dest_dir`, refresh the tree,
    /// and summarize the result in the status bar.
    fn import_dropped_onto(
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
    fn move_dropped_onto(
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
    fn finish_transfer(
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
        self.tree.refresh();
        self.tree.expanded.insert(dest_dir.to_path_buf());
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
    fn new_entry_target(&self) -> Option<PathBuf> {
        let root = self.project.root.clone()?;
        match &self.tree.selected {
            Some(p) if p.is_dir() => Some(p.clone()),
            Some(p) => Some(p.parent().map(|par| par.to_path_buf()).unwrap_or(root)),
            None => Some(root),
        }
    }

    fn commit_new_entry(&mut self, nvim: &mut NeovimBackend) {
        let (kind, name) = match &self.new_entry {
            Some(d) => (d.kind, d.name.clone()),
            None => return,
        };
        let Some(target) = self.new_entry_target() else {
            return;
        };
        match create_entry(&target, kind, &name) {
            Ok(path) => {
                self.tree.refresh();
                self.tree.expanded.insert(target.clone());
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
                }
            }
        }
    }

    fn drop_target_under_pointer(&self, ctx: &egui::Context) -> Option<PathBuf> {
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

    fn handle_drops(
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

    /// VSCode title-bar text: `file — folder — ArctisCode`.
    pub fn title_text(&self) -> String {
        let file = self.tabs.active_ref().map(|t| t.title());
        let project = self
            .project
            .root
            .as_ref()
            .and_then(|r| r.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        match (file, project) {
            (Some(f), Some(p)) => format!("{f} — {p} — ArctisCode"),
            (None, Some(p)) => format!("{p} — ArctisCode"),
            _ => "ArctisCode".to_owned(),
        }
    }

    fn show_activity_bar(&mut self, ui: &mut egui::Ui, theme: &VscodeTheme) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if Self::activity_icon(
                ui,
                theme,
                Glyph::Explorer,
                "Explorer (⌘B)",
                self.center_view == CenterView::Editor,
                true,
            )
            .clicked()
            {
                self.center_view = CenterView::Editor;
                self.sidebar_visible = true;
            }
            if Self::activity_icon(
                ui,
                theme,
                Glyph::Kanban,
                "Tasks",
                self.center_view == CenterView::Tasks,
                true,
            )
            .clicked()
            {
                self.center_view = CenterView::Tasks;
            }
            if Self::activity_icon(
                ui,
                theme,
                Glyph::Gear,
                "Settings",
                self.center_view == CenterView::Settings,
                true,
            )
            .clicked()
            {
                self.center_view = CenterView::Settings;
            }
        });
    }

    fn activity_icon(
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        glyph: Glyph,
        tooltip: &str,
        active: bool,
        clickable: bool,
    ) -> egui::Response {
        let sense = if clickable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ACTIVITY_BAR_WIDTH, ACTIVITY_BAR_WIDTH), sense);
        if active {
            // White accent bar at the left edge, VSCode style.
            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.min.y + 10.0),
                egui::vec2(2.0, ACTIVITY_BAR_WIDTH - 20.0),
            );
            ui.painter().rect_filled(bar, 0.0, theme.activity_fg);
        }
        let fg = if active {
            theme.activity_fg
        } else {
            theme.activity_inactive_fg
        };
        icons::draw_glyph(ui.painter(), glyph, rect.shrink(13.0), fg, 1.5);
        resp.on_hover_text(tooltip)
    }

    /// Section twistie (`▸`/`▾`). No hover fill by design: the tree
    /// column paints no boxes, only text and guide lines.
    fn twistie(ui: &mut egui::Ui, theme: &VscodeTheme, expanded: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(20.0, TREE_ROW_HEIGHT),
            egui::Sense::click(),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if expanded { "▾" } else { "▸" },
            egui::TextStyle::Body.resolve(ui.style()),
            theme.text,
        );
        resp
    }

    /// Drop zone that paints nothing.
    ///
    /// egui's `dnd_drop_zone` overwrites its frame fill from the widget
    /// visuals (`widgets.inactive/hovered/active.bg_fill`) and always
    /// paints it — stock zones spray opaque boxes behind the whole tree
    /// area and every folder row. Scoping those fills to transparent keeps
    /// stock DnD semantics (payloads, clicks, drag-starts, drops) with
    /// zero background. No drop-hover highlight either: absolute
    /// transparency as designed (reversible if missed).
    fn clear_drop_zone<R>(
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> (
        egui::InnerResponse<R>,
        Option<std::sync::Arc<TreeDragPayload>>,
    ) {
        ui.scope(|ui| {
            let visuals = ui.visuals_mut();
            visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
            visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
            ui.dnd_drop_zone::<TreeDragPayload, _>(egui::Frame::default(), add_contents)
        })
        .inner
    }

    fn show_titlebar(
        &mut self,
        ctx: &egui::Context,
        theme: &VscodeTheme,
        guards: bool,
    ) {
        let titlebar_bg = theme.tabstrip_bg;
        egui::TopBottomPanel::top("titlebar")
            .exact_height(TITLEBAR_HEIGHT)
            .frame(
                egui::Frame::default()
                    .fill(titlebar_bg)
                    .inner_margin(egui::Margin {
                        left: 0,
                        right: 8,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    // Spacer for the native traffic lights (macOS
                    // fullsize content view draws our content behind them).
                    ui.add_space(TITLEBAR_TRAFFIC_PAD);
                    // Explorer quick filter.
                    let search_resp = ui.add_sized(
                        egui::vec2(TITLEBAR_SEARCH_W, 24.0),
                        egui::TextEdit::singleline(&mut self.explorer_filter)
                            .hint_text("Search files…"),
                    );
                    if !self.explorer_filter.is_empty()
                        && ui.small_button("✕").on_hover_text("Clear filter").clicked()
                    {
                        self.explorer_filter.clear();
                    }
                    // Esc clears the filter when the search box is focused.
                    if search_resp.has_focus()
                        && ctx.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        self.explorer_filter.clear();
                    }
                    // Tabs fill the rest of the row (Editor view only).
                    if self.center_view == CenterView::Editor {
                        let dirty: Vec<bool> =
                            self.tabs.tabs.iter().map(|t| t.dirty).collect();
                        let mut pending: Option<usize> = None;
                        crate::tabs::bar::show_row(ui, theme, &mut self.tabs, &mut |idx| {
                            // Ask caller to confirm if dirty; return false to defer close.
                            if dirty.get(idx).copied().unwrap_or(false) {
                                pending = Some(idx);
                                false
                            } else {
                                true
                            }
                        });
                        if let Some(idx) = pending {
                            if guards {
                                self.confirm_close = Some(idx);
                            } else {
                                self.tabs.active = Some(idx);
                                self.save_active(false);
                                self.tabs.close(idx);
                            }
                        }
                    } else {
                        let label = match self.center_view {
                            CenterView::Tasks => "Tasks",
                            CenterView::Settings => "Settings",
                            CenterView::Editor => "",
                        };
                        ui.label(
                            egui::RichText::new(label)
                                .size(12.0)
                                .color(theme.muted),
                        );
                    }
                });
            });
    }

    /// Case-insensitive substring match for the explorer filter.
    fn filter_matches(name: &str, query_lower: &str) -> bool {
        name.to_lowercase().contains(query_lower)
    }

    /// True if any descendant of `dir` matches the filter (bounded walk so
    /// huge repos don't stall the frame; cache makes repeats cheap).
    fn dir_has_match(&mut self, dir: &std::path::Path, query_lower: &str, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        let kids = self.tree.children_of(dir, depth);
        let mut checked = 0;
        for kid in kids {
            checked += 1;
            if checked > 500 {
                break;
            }
            if Self::filter_matches(&kid.name(), query_lower) {
                return true;
            }
            if kid.is_dir && self.dir_has_match(&kid.path, query_lower, depth + 1) {
                return true;
            }
        }
        false
    }

    fn show_tree(
        &mut self,
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
    ) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("EXPLORER")
                    .size(11.0)
                    .color(theme.muted),
            );
        });
        ui.add_space(4.0);

        let Some(root) = self.project.root.clone() else {
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("You have not yet opened a folder.")
                        .size(12.0)
                        .color(theme.muted),
                );
                ui.add_space(10.0);
                if icons::primary_button(ui, theme, "Open Folder").clicked() {
                    if let Some(dir) = opener::pick_folder_dialog() {
                        self.open_project(dir);
                    }
                }
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("⌘O  Open folder")
                        .size(12.0)
                        .color(theme.faint),
                );
            });
            return;
        };

        // Project section header: twistie + UPPERCASE name + view actions.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if Self::twistie(ui, theme, self.explorer_open).clicked() {
                self.explorer_open = !self.explorer_open;
            }
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_uppercase())
                .unwrap_or_else(|| root.display().to_string());
            ui.label(egui::RichText::new(name).size(11.0).color(theme.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, theme, Glyph::CollapseAll, "Collapse Folders in View", 24.0)
                    .clicked()
                {
                    self.tree.expanded.clear();
                }
                if icons::icon_button(ui, theme, Glyph::Refresh, "Refresh Explorer", 24.0)
                    .clicked()
                {
                    self.tree.refresh();
                }
                if icons::icon_button(ui, theme, Glyph::NewFolder, "New Folder", 24.0)
                    .clicked()
                {
                    self.new_entry = Some(NewEntryDialog {
                        kind: EntryKind::Folder,
                        name: String::new(),
                        error: None,
                    });
                }
                if icons::icon_button(ui, theme, Glyph::NewFile, "New File", 24.0).clicked()
                {
                    self.new_entry = Some(NewEntryDialog {
                        kind: EntryKind::File,
                        name: String::new(),
                        error: None,
                    });
                }
            });
        });

        if !self.explorer_open {
            return;
        }

        // Fresh drop-target rects every frame (handle_drops reads last frame's).
        self.folder_drop_rects.clear();
        let root_target = root.clone();
        let (zone, dropped) = Self::clear_drop_zone(ui, |ui| {
            // Restore neutral widget fills inside: the scrollbar reads them
            // and must stay visible (the zone frame itself stays clear).
            ui.scope(|ui| {
                let visuals = ui.visuals_mut();
                visuals.widgets.inactive.bg_fill = theme.button_secondary_bg;
                visuals.widgets.hovered.bg_fill = theme.button_secondary_hover;
                visuals.widgets.active.bg_fill = theme.button_secondary_hover;
                egui::ScrollArea::vertical()
                    .id_salt("explorer")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.show_dir(ui, &root_target, 0, theme, nvim, images);
                    })
            })
            .inner
        });
        self.folder_drop_rects
            .push((zone.response.rect, root.clone()));
        if let Some(payload) = dropped {
            // Moved-away images must not linger in the shared cache.
            images.invalidate(&payload.0);
            self.move_dropped_onto(vec![payload.0.clone()], root, nvim);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn show_dir(
        &mut self,
        ui: &mut egui::Ui,
        dir: &std::path::Path,
        depth: usize,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
    ) {
        let kids = self.tree.children_of(dir, depth);
        let query_lower = self.explorer_filter.trim().to_lowercase();
        let filtering = !query_lower.is_empty();
        let mut any_shown = false;
        for kid in kids {
            let name = kid.name();
            if kid.is_dir {
                // While filtering: show the folder if its own name matches
                // or any descendant matches; auto-expand to reveal hits.
                let expanded = if filtering {
                    if !Self::filter_matches(&name, &query_lower)
                        && !self.dir_has_match(&kid.path, &query_lower, depth + 1)
                    {
                        continue;
                    }
                    true
                } else {
                    self.tree.is_expanded(&kid.path)
                };
                let target = kid.path.clone();
                let selected = self.tree.selected.as_ref() == Some(&target);
                let zone_target = target.clone();
                let folder_kind = icons::folder_kind_for(&name);
                let (zone, dropped) = Self::clear_drop_zone(ui, |ui| {
                    Self::tree_row(
                        ui,
                        theme,
                        depth,
                        Some(expanded),
                        &name,
                        selected,
                        icons::TreeIcon::Folder(folder_kind),
                    )
                });
                self.folder_drop_rects
                    .push((zone.response.rect, zone_target.clone()));
                if let Some(payload) = dropped {
                    // Internal drags move; external Finder drops copy
                    // (see handle_drops → import_dropped_onto).
                    images.invalidate(&payload.0);
                    self.move_dropped_onto(vec![payload.0.clone()], zone_target, nvim);
                }
                // Payload is set only once a real drag begins, so plain
                // clicks keep their native behavior.
                if zone.inner.drag_started() {
                    zone.inner
                        .dnd_set_drag_payload(TreeDragPayload(target.clone()));
                }
                if zone.inner.clicked() {
                    self.tree.selected = Some(target.clone());
                    // While filtering all folders render expanded; only
                    // toggle real expansion when no filter is active.
                    if !filtering {
                        self.tree.toggle(&target);
                    }
                }
                if expanded {
                    self.show_dir(ui, &target, depth + 1, theme, nvim, images);
                }
                any_shown = true;
            } else {
                if filtering && !Self::filter_matches(&name, &query_lower) {
                    continue;
                }
                let selected = self.tree.selected.as_ref() == Some(&kid.path);
                let file_kind = icons::file_icon_for(&kid.path);
                let resp = Self::tree_row(
                    ui,
                    theme,
                    depth,
                    None,
                    &name,
                    selected,
                    icons::TreeIcon::File(file_kind),
                );
                if resp.drag_started() {
                    resp.dnd_set_drag_payload(TreeDragPayload(kid.path.clone()));
                }
                if resp.clicked() {
                    self.tree.selected = Some(kid.path.clone());
                    self.open_file(kid.path.clone(), nvim);
                }
                any_shown = true;
            }
        }
        if filtering && depth == 0 && !any_shown {
            ui.label(
                egui::RichText::new("No matches")
                    .size(12.0)
                    .color(theme.faint),
            );
        }
    }

    /// Full-sidebar-width tree row (VSCode-style): selected paints
    /// `list_active_sel`, hover paints `list_hover`. `twistie`:
    /// `None` for files, `Some(expanded)` for folders. `icon` is the
    /// Seti-style painter glyph (no assets, no textures — see `icons.rs`).
    fn tree_row(
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        depth: usize,
        twistie: Option<bool>,
        label: &str,
        selected: bool,
        icon: icons::TreeIcon,
    ) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), TREE_ROW_HEIGHT),
            egui::Sense::click_and_drag(),
        );
        if selected {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, theme.list_active_sel);
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, theme.list_hover);
        }
        let guide = theme.tree_guide.gamma_multiply(0.6);
        for level in 0..depth {
            let x = rect.min.x + 8.0 + level as f32 * TREE_INDENT;
            ui.painter()
                .vline(x, rect.y_range(), egui::Stroke::new(1.0, guide));
        }
        let fg = if selected {
            theme.list_sel_fg
        } else {
            theme.text
        };
        let chevron_fg = if selected {
            theme.list_sel_fg
        } else {
            theme.muted
        };
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let base_x = rect.min.x + depth as f32 * TREE_INDENT + 4.0;
        // Chevron gutter is reserved for files too, so file labels align
        // with folder labels exactly like VSCode.
        const CHEVRON_W: f32 = 16.0;
        const ICON_W: f32 = 18.0;
        if let Some(expanded) = twistie {
            ui.painter().text(
                egui::pos2(base_x + CHEVRON_W / 2.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                if expanded { "▾" } else { "▸" },
                font_id.clone(),
                chevron_fg,
            );
        }
        // Seti-style icon in its own slot (16px glyph in an 18px gutter).
        // Folder open/closed follows the chevron state.
        let expanded = twistie.unwrap_or(false);
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(base_x + CHEVRON_W + 1.0, rect.center().y - 8.0),
            egui::vec2(16.0, 16.0),
        );
        icons::draw_tree_icon(ui.painter(), icon, icon_rect, theme, expanded);
        ui.painter().text(
            egui::pos2(base_x + CHEVRON_W + ICON_W, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            fg,
        );
        resp
    }

    fn show_editor(
        &mut self,
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
    ) {
        // Keep the cursor readout honest when the active tab changes.
        let active_path = self.tabs.active_ref().map(|t| t.path.clone());
        if active_path != self.last_tab_path {
            self.cursor_pos = None;
            self.last_tab_path = active_path;
        }

        let Some(idx) = self.tabs.active else {
            self.cursor_pos = None;
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0);
                    ui.label(
                        egui::RichText::new("ArctisCode")
                            .size(24.0)
                            .color(theme.editor_fg),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Open a folder to start editing")
                            .size(13.0)
                            .color(theme.muted),
                    );
                    ui.add_space(16.0);
                    if icons::primary_button(ui, theme, "Open Folder…").clicked() {
                        if let Some(dir) = opener::pick_folder_dialog() {
                            self.open_project(dir);
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("⌘O  Open folder      ⌘S  Save      ⌘W  Close tab      ⌘B  Toggle sidebar")
                            .size(12.0)
                            .color(theme.faint),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("⇧⌘N  New window      Drop a folder or file to begin. Terminal stays in Ghostty.")
                            .size(12.0)
                            .color(theme.faint),
                    );
                });
            });
            return;
        };
        if idx >= self.tabs.tabs.len() {
            return;
        }

        // Image preview branch: shrink-to-fit, never larger than the
        // window, small images keep their native size.
        if self.tabs.tabs[idx].kind == TabKind::Image {
            self.cursor_pos = None;
            let path = self.tabs.tabs[idx].path.clone();
            match images.texture_for(ui.ctx(), &path) {
                Ok(tex) => {
                    egui::ScrollArea::both()
                        .id_salt("image-preview")
                        .show(ui, |ui| {
                            ui.heading(path.display().to_string());
                            ui.vertical_centered(|ui| {
                                let size = crate::preview::fit_contain(
                                    tex.size_vec2(),
                                    ui.available_size(),
                                );
                                ui.add(
                                    egui::Image::new(&tex).fit_to_exact_size(size),
                                );
                            });
                        });
                }
                Err(e) => {
                    ui.label(format!("preview failed: {e:#}"));
                }
            }
            return;
        }

        // Unsupported/binary branch: explain, don't error.
        if self.tabs.tabs[idx].kind == TabKind::Unsupported {
            self.cursor_pos = None;
            let path = self.tabs.tabs[idx].path.clone();
            let notice = self.tabs.tabs[idx].notice.clone();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            ui.vertical_centered(|ui| {
                ui.heading(path.display().to_string());
                ui.label(notice.unwrap_or_else(|| "This file can't be edited as text.".to_owned()));
                ui.label(format!("Size: {size} bytes"));
                ui.horizontal(|ui| {
                    if icons::primary_button(ui, theme, "Reveal in Finder").clicked() {
                        if let Err(e) = open::that(&path) {
                            self.status = format!("reveal failed: {e:#}");
                        }
                    }
                    if ui.button("Close tab").clicked() {
                        self.tabs.close(idx);
                    }
                });
            });
            return;
        }

        // Text branch with a compact VSCode-style find row.
        if let Some(notice) = self.tabs.tabs[idx].notice.clone() {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!("⚠ {notice}"))
                        .size(12.0)
                        .color(theme.warning),
                );
            });
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                ui.add_sized(
                    egui::vec2(220.0, 24.0),
                    egui::TextEdit::singleline(&mut self.find_query)
                        .hint_text("Find")
                        .frame(true),
                );
                if !self.find_query.is_empty() {
                    let hits = self.tabs.tabs[idx]
                        .content
                        .matches(self.find_query.as_str())
                        .count();
                    ui.label(
                        egui::RichText::new(format!("{hits} found"))
                            .size(12.0)
                            .color(theme.muted),
                    );
                }
            });
        });
        ui.add_space(4.0);

        // Full-height editor: minimum height is the remaining panel space,
        // so short files still fill the view; long files grow and scroll.
        let editor_h = (ui.available_height() - 4.0).max(120.0);
        // Gutter + code share one scroll area so they scroll in lockstep.
        // No-wrap layout keeps visual rows 1:1 with line numbers.
        let mono = egui::TextStyle::Monospace;
        let mono_id = mono.resolve(ui.style());
        let count = crate::editor::line_count(&self.tabs.tabs[idx].content);
        let (gutter_string, digits) = crate::editor::gutter_text(count);
        let gutter_w = ui.fonts(|f| f.glyph_width(&mono_id, '0')) * digits as f32 + 16.0;
        let editor_id = egui::Id::new(("code-editor", self.tabs.tabs[idx].path.clone()));

        // Previous frame's cursor line gets the brighter VSCode number.
        let active_line = self.cursor_pos.map(|(line, _)| line);
        let gutter_mono = mono_id.clone();
        let gutter_fg = theme.gutter_fg;
        let gutter_active = theme.gutter_active_fg;
        let mut gutter_layouter = move |ui: &egui::Ui,
                                        text: &dyn egui::TextBuffer,
                                        _wrap: f32| {
            let mut job = egui::text::LayoutJob::default();
            for (i, line) in text.as_str().split('\n').enumerate() {
                if i > 0 {
                    job.append(
                        "\n",
                        0.0,
                        egui::text::TextFormat::simple(gutter_mono.clone(), gutter_fg),
                    );
                }
                let color = if Some(i + 1) == active_line {
                    gutter_active
                } else {
                    gutter_fg
                };
                job.append(
                    line,
                    0.0,
                    egui::text::TextFormat::simple(gutter_mono.clone(), color),
                );
            }
            ui.fonts(|f| f.layout_job(job))
        };

        let editor_bg = theme.editor_bg;
        let editor_fg = theme.editor_fg;
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(editor_fg);
            egui::ScrollArea::both()
                .id_salt("code-editor")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let mut gutter = gutter_string;
                        ui.add(
                            egui::TextEdit::multiline(&mut gutter)
                                .font(mono.clone())
                                .frame(false)
                                .interactive(false)
                                .background_color(editor_bg)
                                .layouter(&mut gutter_layouter)
                                .desired_width(gutter_w)
                                .min_size(egui::vec2(gutter_w, editor_h)),
                        );
                        let tab = &mut self.tabs.tabs[idx];
                        let text_color = ui.visuals().text_color();
                        let font_id = mono_id.clone();
                        let mut layouter =
                            |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                                let mut job = egui::text::LayoutJob::simple(
                                    text.as_str().to_owned(),
                                    font_id.clone(),
                                    text_color,
                                    f32::INFINITY,
                                );
                                job.wrap.max_width = f32::INFINITY;
                                ui.fonts(|f| f.layout_job(job))
                            };
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut tab.content)
                                .font(mono)
                                .code_editor()
                                .id(editor_id)
                                .frame(false)
                                .background_color(editor_bg)
                                .layouter(&mut layouter)
                                .desired_width(f32::INFINITY)
                                .min_size(egui::vec2(0.0, editor_h)),
                        );
                        if resp.changed() {
                            tab.mark_edited();
                            // Best-effort live sync to Neovim (throttled by frame rate).
                            nvim.sync_buffer(&tab.path.clone(), &tab.content.clone());
                        }
                        // Cursor readout for the status bar.
                        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                            if let Some(range) = state.cursor.char_range() {
                                let (line, col) =
                                    crate::editor::line_col(&tab.content, range.primary.index);
                                self.cursor_pos = Some((line, col));
                            }
                        }
                    });
                });
        });
    }

    /// Render this window's full chrome in the current viewport pass.
    /// Returns shell-level requests (spawn / destroy OS viewports).
    pub fn render_window(
        &mut self,
        ctx: &egui::Context,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
        settings: &mut Settings,
    ) -> Vec<WindowAction> {
        let mut actions = Vec::new();
        // Off = closing saves automatically instead of asking (never
        // discards silently).
        let guards = settings.confirm_guards;

        // Title + watcher events
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title_text()));
        for evt in self.watcher.drain() {
            match evt {
                crate::project::watcher::FsEvent::Changed(p) => {
                    self.tree.refresh();
                    images.invalidate(&p);
                    self.status = format!("Changed on disk: {}", p.display());
                }
                crate::project::watcher::FsEvent::NeedsRefresh => self.tree.refresh(),
            }
        }
        self.handle_drops(ctx, nvim, images);

        // Keyboard: Cmd/Ctrl+O open, Cmd/Ctrl+S save, Cmd/Ctrl+W close tab,
        // Cmd/Ctrl+B toggle sidebar, Shift+Cmd/Ctrl+N new window.
        let open_pressed = ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::O,
            ))
        });
        if open_pressed {
            if let Some(dir) = opener::pick_folder_dialog() {
                self.open_project(dir);
            }
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::S,
            ))
        }) {
            self.save_active(false);
        }
        let close_pressed = ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::W,
            ))
        });
        if close_pressed {
            if let Some(idx) = self.tabs.active {
                let dirty = self.tabs.tabs.get(idx).map(|t| t.dirty).unwrap_or(false);
                if dirty {
                    if guards {
                        self.confirm_close = Some(idx);
                    } else {
                        self.tabs.active = Some(idx);
                        self.save_active(false);
                        self.tabs.close(idx);
                    }
                } else {
                    self.tabs.close(idx);
                }
            }
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::B,
            ))
        }) {
            self.sidebar_visible = !self.sidebar_visible;
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::N,
            ))
        }) {
            actions.push(WindowAction::NewWindow);
        }

        // Viewport close (red traffic light): dirty tabs cancel the close
        // and raise the window-level guard instead — unless guards are
        // off, in which case everything saves automatically.
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.has_dirty_tabs() {
                if guards {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.confirm_close_window = true;
                } else {
                    self.save_all_text_tabs();
                    if self.has_dirty_tabs() {
                        // Overwrite-blocked files keep the window open.
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.confirm_close_window = true;
                    } else {
                        actions.push(WindowAction::CloseWindow(self.viewport));
                    }
                }
            } else {
                actions.push(WindowAction::CloseWindow(self.viewport));
            }
        }

        // Custom titlebar: native traffic lights + search + tabs in one row.
        // Must come before side/central panels so it spans the full width.
        self.show_titlebar(ctx, theme, guards);

        // Status bar: same 80/20 treatment as the sidebar in every state,
        // so opening a folder never shifts the chrome.
        let status_bg = theme.statusbar_bg;
        let status_fg = theme.statusbar_fg;
        let status_msg = self.status.clone();
        let cursor = self.cursor_pos;
        let language = self
            .tabs
            .active_ref()
            .map(|t| tabs::language_label(&t.path));
        let nvim_status = nvim.status();
        egui::TopBottomPanel::bottom("status")
            .exact_height(STATUS_BAR_HEIGHT)
            .frame(egui::Frame::default().fill(status_bg).inner_margin(
                egui::Margin {
                    left: 10,
                    right: 10,
                    top: 0,
                    bottom: 0,
                },
            ))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(14.0, 0.0);
                    match nvim_status {
                        crate::editor::NvimStatus::Connected => {
                            ui.label(egui::RichText::new("nvim").size(12.0).color(status_fg));
                        }
                        crate::editor::NvimStatus::Unavailable(reason) => {
                            ui.label(
                                egui::RichText::new("nvim: unavailable")
                                    .size(12.0)
                                    .color(status_fg),
                            )
                            .on_hover_text(reason);
                        }
                    }
                    ui.label(egui::RichText::new(status_msg).size(12.0).color(status_fg));
                    // Right-aligned VSCode items (added right-to-left):
                    // language, encoding, cursor position.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(lang) = language {
                            ui.label(egui::RichText::new(lang).size(12.0).color(status_fg));
                        }
                        ui.label(egui::RichText::new("UTF-8").size(12.0).color(status_fg));
                        if let Some((line, col)) = cursor {
                            ui.label(
                                egui::RichText::new(format!("Ln {line}, Col {col}"))
                                    .size(12.0)
                                    .color(status_fg),
                            );
                        }
                    });
                });
            });

        // Activity bar.
        let activity_bg = theme.activity_bg;
        egui::SidePanel::left("activity")
            .exact_width(ACTIVITY_BAR_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::default()
                    .fill(activity_bg)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| self.show_activity_bar(ui, theme));

        // Explorer sidebar.
        if self.sidebar_visible {
            let sidebar_bg = theme.sidebar_bg;
            egui::SidePanel::left("explorer")
                .resizable(true)
                .default_width(260.0)
                .frame(
                    egui::Frame::default()
                        .fill(sidebar_bg)
                        .inner_margin(egui::Margin::same(0)),
                )
                .show_separator_line(false)
                .show(ctx, |ui| self.show_tree(ui, theme, nvim, images));
        }

        // Center content follows the activity-bar view.
        let editor_bg = theme.editor_bg;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(editor_bg)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| {
                match self.center_view {
                    CenterView::Editor => {
                        self.show_editor(ui, theme, nvim, images);
                    }
                    CenterView::Tasks => {
                        if tasks::show_board(ui, theme, &mut self.tasks) {
                            self.save_tasks();
                        }
                    }
                    CenterView::Settings => {
                        if crate::settings::show_settings(
                            ui,
                            theme,
                            settings,
                            self.project.root.as_deref(),
                            &self.tasks_path.clone(),
                        ) {
                            if let Err(e) = crate::settings::save_settings(settings) {
                                self.status = format!("settings save failed: {e:#}");
                            }
                        }
                    }
                }
            });

        // Confirm-close dialog (unsaved changes guard, PRD §4.5/§4.8)
        if let Some(idx) = self.confirm_close {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Close without saving?");
                    ui.horizontal(|ui| {
                        if icons::primary_button(ui, theme, "Save & close").clicked() {
                            self.tabs.active = Some(idx);
                            self.save_active(false);
                            self.tabs.close(idx);
                            self.confirm_close = None;
                        }
                        if ui.button("Discard").clicked() {
                            self.tabs.close(idx);
                            self.confirm_close = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_close = None;
                        }
                    });
                });
        }

        // Window-level guard: viewport close with dirty tabs.
        if self.confirm_close_window {
            egui::Window::new("Unsaved changes in this window")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("This window has unsaved files. Close it anyway?");
                    ui.horizontal(|ui| {
                        if icons::primary_button(ui, theme, "Save all & close").clicked() {
                            // Files blocked by external modification surface
                            // the overwrite guard and keep the window open
                            // for another pass.
                            self.save_all_text_tabs();
                            self.confirm_close_window = false;
                            self.close_after_save = true;
                        }
                        if ui.button("Discard & close").clicked() {
                            self.discard_all_tabs();
                            self.confirm_close_window = false;
                            self.close_after_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_close_window = false;
                        }
                    });
                });
        }

        // Overwrite guard (external modification, PRD §4.8)
        if let Some(path) = self.confirm_overwrite.clone() {
            egui::Window::new("File changed on disk")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("{} changed externally. Overwrite?", path.display()));
                    ui.horizontal(|ui| {
                        if icons::primary_button(ui, theme, "Overwrite").clicked() {
                            self.save_active(true);
                            self.confirm_overwrite = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_overwrite = None;
                        }
                    });
                });
        }

        // Open failure dialog (full error chain + stderr already logged)
        if let Some(msg) = self.open_error.clone() {
            egui::Window::new("Couldn't open file")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label(&msg);
                    if ui.button("Dismiss").clicked() {
                        self.open_error = None;
                    }
                });
        }

        // New file / folder dialog
        if self.new_entry.is_some() {
            let title = match self.new_entry.as_ref().map(|d| d.kind) {
                Some(EntryKind::Folder) => "New folder",
                _ => "New file",
            };
            let target = self.new_entry_target();
            let mut commit = false;
            let mut cancel = false;
            let danger = theme.danger;
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if let Some(t) = target {
                        ui.label(format!("Location: {}", t.display()));
                    }
                    if let Some(d) = self.new_entry.as_mut() {
                        let first_frame = d.name.is_empty() && d.error.is_none();
                        let resp = ui.text_edit_singleline(&mut d.name);
                        if first_frame {
                            resp.request_focus();
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            commit = true;
                        }
                        if let Some(err) = d.error.clone() {
                            ui.colored_label(danger, err);
                        }
                    }
                    ui.horizontal(|ui| {
                        if icons::primary_button(ui, theme, "Create").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if commit {
                self.commit_new_entry(nvim);
            } else if cancel {
                self.new_entry = None;
            }
        }

        // Save-all / Discard-all resolved and nothing dirty left → close.
        if self.close_after_save {
            self.close_after_save = false;
            if !self.has_dirty_tabs() {
                actions.push(WindowAction::CloseWindow(self.viewport));
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_window_has_no_project_or_tabs() {
        let w = RepoWindow::blank(egui::ViewportId::ROOT);
        assert!(w.project.root.is_none());
        assert!(w.tabs.tabs.is_empty());
        assert!(w.tabs.active.is_none());
        assert!(!w.has_dirty_tabs());
    }

    #[test]
    fn viewport_ids_are_unique_per_counter() {
        let a = fresh_viewport_id(1);
        let b = fresh_viewport_id(2);
        let c = fresh_viewport_id(1);
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn with_initial_picks_up_folder() {
        let dir = tempfile::tempdir().unwrap();
        let w = RepoWindow::with_initial(
            egui::ViewportId::ROOT,
            Some(dir.path().to_string_lossy().into_owned()),
        );
        assert_eq!(w.project.root.as_deref(), Some(dir.path()));
    }
}
