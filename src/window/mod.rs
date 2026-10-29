//! One repo window: project + tree + tabs + editor + image preview.
//!
//! The app shell (`app::ArctisCodeApp`) owns N of these — one per OS
//! viewport — plus the services shared across windows (theme, embedded
//! Neovim, image cache). Sharing is safe because the shared services are
//! path-keyed: the same file opened in two windows edits one Neovim
//! buffer, and concurrent saves meet the existing guarded-write dialog.

mod chrome;
mod editor_view;
mod io;
mod tree_view;

use crate::editor::{HlCache, NeovimBackend};
use crate::explorer::FileTree;
use crate::icons;
use crate::preview::ImageCache;
use crate::project::entry::EntryKind;
use crate::project::opener;
use crate::project::watcher::ProjectWatcher;
use crate::project::Project;
use crate::settings::Settings;
use crate::tabs::{self, TabManager};
use crate::tasks::{self, BoardState};
use crate::theme::{VscodeTheme, ACTIVITY_BAR_WIDTH, STATUS_BAR_HEIGHT};
use std::collections::HashMap;
use std::path::PathBuf;

/// Custom titlebar metrics: native traffic lights stay visible (macOS
/// fullsize content view); this left pad keeps our content clear of them.
pub const TITLEBAR_TRAFFIC_PAD: f32 = 78.0;
pub const TITLEBAR_HEIGHT: f32 = 38.0;

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
struct TreeDragPayload(Vec<PathBuf>);

/// Pending new file / folder dialog state.
#[derive(Debug)]
pub(crate) struct RenameDraft {
    path: PathBuf,
    name: String,
    error: Option<String>,
    focus_requested: bool,
}

#[derive(Debug)]
pub(crate) struct NewEntryDialog {
    kind: EntryKind,
    name: String,
    error: Option<String>,
    target: PathBuf,
    focus_requested: bool,
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
    selected_paths: std::collections::HashSet<PathBuf>,
    selection_anchor: Option<PathBuf>,
    visible_paths: Vec<PathBuf>,
    previous_visible_paths: Vec<PathBuf>,
    copied_paths: Vec<PathBuf>,
    rename_draft: Option<RenameDraft>,
    delete_pending: Option<Vec<PathBuf>>,
    pub cursor_pos: Option<(usize, usize)>,
    pub find_query: String,
    /// Titlebar quick filter: substrings match file/folder names in the explorer.
    pub explorer_filter: String,
    /// Memoized (folder, query) -> has-match, cleared when the query or
    /// tree changes; keeps per-frame filter search off disk.
    filter_cache: HashMap<PathBuf, bool>,
    filter_cache_query: String,
    /// Gutter text for the current line count; rebuilt only when count changes.
    gutter_cache: (usize, String),
    /// (query, content_len, hits) for the find row; avoids scanning MBs/frame.
    find_cache: (String, usize, usize),
    /// Last cursor char index the status readout was computed for.
    last_cursor_idx: Option<usize>,
    /// Folder row rects from the last frame, for hover-aware drops.
    pub folder_drop_rects: Vec<(egui::Rect, PathBuf)>,
    /// Activity-bar Explorer toggle.
    pub sidebar_visible: bool,
    sidebar_width: f32,
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
    /// Syntax-highlight cache (re-parses only on edit/switch).
    hl: HlCache,
    /// Buffer content before the last processed keystroke (auto-edit diff).
    edit_prev: String,
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
            selected_paths: Default::default(),
            selection_anchor: None,
            visible_paths: Vec::new(),
            previous_visible_paths: Vec::new(),
            copied_paths: Vec::new(),
            rename_draft: None,
            delete_pending: None,
            cursor_pos: None,
            find_query: String::new(),
            find_cache: (String::new(), 0, 0),
            last_cursor_idx: None,
            gutter_cache: (0, String::new()),
            explorer_filter: String::new(),
            filter_cache: HashMap::new(),
            filter_cache_query: String::new(),
            folder_drop_rects: Vec::new(),
            sidebar_visible: true,
            sidebar_width: 260.0,
            explorer_open: true,
            last_tab_path: None,
            confirm_close_window: false,
            close_after_save: false,
            center_view: CenterView::Editor,
            hl: HlCache::default(),
            edit_prev: String::new(),
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

    /// Tree contents changed: drop fs cache and memoized filter hits.
    fn refresh_tree(&mut self) {
        self.tree.refresh();
        self.filter_cache.clear();
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
        let format = settings.format_on_save;

        // Title + watcher events
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title_text()));
        for evt in self.watcher.drain() {
            match evt {
                crate::project::watcher::FsEvent::Changed(p) => {
                    self.refresh_tree();
                    images.invalidate(&p);
                    self.status = format!("Changed on disk: {}", p.display());
                }
                crate::project::watcher::FsEvent::NeedsRefresh => self.refresh_tree(),
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
            self.save_active(nvim, format, false);
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
                        self.save_active(nvim, format, false);
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
                    self.save_all_text_tabs(nvim, format);
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
        self.show_titlebar(ctx, theme);

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
            .frame(
                egui::Frame::default()
                    .fill(status_bg)
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 10,
                        top: 0,
                        bottom: 0,
                    }),
            )
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
            let panel = egui::SidePanel::left("explorer")
                .resizable(true)
                .default_width(260.0)
                .frame(
                    egui::Frame::default()
                        .fill(sidebar_bg)
                        .inner_margin(egui::Margin::same(0)),
                )
                .show_separator_line(false)
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.show_tree(ui, theme, nvim, images)
                });
            self.sidebar_width = panel.response.rect.width();
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
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.center_view {
                CenterView::Editor => {
                    self.show_tabs(ui, theme, nvim, guards, format);
                    self.show_editor(ui, theme, nvim, images, settings);
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
                            self.save_active(nvim, format, false);
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
                            self.save_all_text_tabs(nvim, format);
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
                            self.save_active(nvim, format, true);
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

        if let Some(paths) = self.delete_pending.clone() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Delete items")
                .collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Permanently delete {} item(s)?", paths.len()));
                    for path in paths.iter().take(5) { ui.label(path.display().to_string()); }
                    if paths.len() > 5 { ui.label(format!("and {} more", paths.len() - 5)); }
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() { confirm = true; }
                        if ui.button("Cancel").clicked() { cancel = true; }
                    });
                });
            if confirm { self.delete_selected_entries(paths, images); self.delete_pending = None; }
            else if cancel { self.delete_pending = None; }
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

    #[test]
    fn filter_matches_without_allocating_lowered_haystack() {
        assert!(RepoWindow::filter_matches("main.rs", "main"));
        assert!(RepoWindow::filter_matches("MAIN.RS", "main"));
        assert!(RepoWindow::filter_matches("lib.rs", "b."));
        assert!(!RepoWindow::filter_matches("lib.rs", "main"));
        assert!(!RepoWindow::filter_matches("a", "abcdef"));
        assert!(RepoWindow::filter_matches("anything", ""));
    }
}
