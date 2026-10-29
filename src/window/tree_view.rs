//! One repo window: explorer tree view.
use super::{NewEntryDialog, RenameDraft, RepoWindow, TreeDragPayload};
use crate::editor::NeovimBackend;
use crate::icons::{self, Glyph};
use crate::preview::ImageCache;
use crate::project::entry::EntryKind;
use crate::project::opener;
use crate::theme::{VscodeTheme, TREE_INDENT, TREE_ROW_HEIGHT};
use std::path::PathBuf;

fn deepest_drop_target(targets: &[(egui::Rect, PathBuf)], pointer: egui::Pos2) -> Option<(egui::Rect, PathBuf)> {
    targets.iter().filter(|(rect, _)| rect.contains(pointer))
        .min_by(|a, b| a.0.area().total_cmp(&b.0.area())).cloned()
}

impl RepoWindow {
    /// Section twistie (`▸`/`▾`). No hover fill by design: the tree
    /// column paints no boxes, only text and guide lines.
    pub(super) fn twistie(
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        expanded: bool,
    ) -> egui::Response {
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(20.0, TREE_ROW_HEIGHT), egui::Sense::click());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if expanded { "▾" } else { "▸" },
            egui::TextStyle::Body.resolve(ui.style()),
            theme.text,
        );
        resp
    }
    /// Case-insensitive substring match without allocating a lowered
    /// haystack per row (the query is lowered once per frame).
    pub(super) fn filter_matches(name: &str, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        let n = name.as_bytes();
        let q = query_lower.as_bytes();
        if q.len() > n.len() {
            return false;
        }
        (0..=n.len() - q.len()).any(|i| {
            n[i..i + q.len()]
                .iter()
                .zip(q.iter())
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        })
    }
    /// True if any descendant of `dir` matches the filter (bounded walk so
    /// huge repos don't stall the frame; results memoized per query, cache
    /// cleared when the query or tree changes). Skips heavy dirs.
    pub(super) fn dir_has_match(
        &mut self,
        dir: &std::path::Path,
        query_lower: &str,
        depth: usize,
    ) -> bool {
        if depth > 8 || query_lower.is_empty() {
            return false;
        }
        if self.filter_cache_query != query_lower {
            self.filter_cache_query = query_lower.to_owned();
            self.filter_cache.clear();
        }
        let key = dir.to_path_buf();
        if let Some(&hit) = self.filter_cache.get(&key) {
            return hit;
        }
        // Copy out what recursion needs so the cache borrow ends first.
        let kids: Vec<(PathBuf, bool, String)> = self
            .tree
            .children_ref(dir, depth)
            .iter()
            .take(501)
            .map(|k| (k.path.clone(), k.is_dir, k.name()))
            .collect();
        let mut checked = 0;
        let mut hit = false;
        for (path, is_dir, name) in &kids {
            checked += 1;
            if checked > 500 {
                break;
            }
            if *is_dir && crate::explorer::tree::is_skipped_dir(name) {
                continue;
            }
            if Self::filter_matches(name, query_lower) {
                hit = true;
                break;
            }
            if *is_dir && self.dir_has_match(path, query_lower, depth + 1) {
                hit = true;
                break;
            }
        }
        self.filter_cache.insert(key, hit);
        hit
    }
    pub(super) fn show_tree(
        &mut self,
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
    ) {
        let Some(root) = self.project.root.clone() else {
            ui.add_space(crate::theme::TAB_STRIP_HEIGHT);
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

        // The action row shares the tab strip height; the tree starts below it.
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), crate::theme::TAB_STRIP_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if Self::twistie(ui, theme, self.explorer_open).on_hover_text("Toggle folders").clicked() {
                self.explorer_open = !self.explorer_open;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(
                    ui,
                    theme,
                    Glyph::CollapseAll,
                    "Collapse Folders in View",
                    24.0,
                )
                .clicked()
                {
                    self.tree.expanded.clear();
                }
                if icons::icon_button(ui, theme, Glyph::Refresh, "Refresh Explorer", 24.0).clicked()
                {
                    self.refresh_tree();
                }
                if icons::icon_button(ui, theme, Glyph::NewFolder, "New Folder", 24.0).clicked() {
                    self.start_new_entry(EntryKind::Folder, nvim);
                }
                if icons::icon_button(ui, theme, Glyph::NewFile, "New File", 24.0).clicked() {
                    self.start_new_entry(EntryKind::File, nvim);
                }
            });
        });

        if !self.explorer_open {
            return;
        }

        self.previous_visible_paths = std::mem::take(&mut self.visible_paths);
        // Fresh drop-target rects every frame (handle_drops reads last frame's).
        self.folder_drop_rects.clear();
        let root_target = root.clone();
        let zone = ui.scope(|ui| {
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
        zone.response.context_menu(|ui| {
            if !self.copied_paths.is_empty() && ui.button("Paste into workspace").clicked() {
                self.paste_explorer_items(root.clone(), nvim);
                ui.close();
            }
        });
        self.folder_drop_rects
            .push((zone.response.rect, root.clone()));
        self.show_tree_drag_feedback(ui, theme, nvim, images);

    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn show_dir(
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
        let mut draft_shown = false;
        for kid in kids {
            if !kid.is_dir && !draft_shown {
                self.show_new_entry_row(ui, dir, depth, theme, nvim);
                draft_shown = true;
            }
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
                self.visible_paths.push(target.clone());
                let selected = self.selected_paths.contains(&target) || (self.selected_paths.is_empty() && self.tree.selected.as_ref() == Some(&target));
                let zone_target = target.clone();
                let folder_kind = icons::folder_kind_for(&name);
                let row = Self::tree_row(
                    ui, theme, depth, Some(expanded), &name, selected,
                    icons::TreeIcon::Folder(folder_kind),
                );
                self.handle_tree_row(ui, &row, &target, true, depth, theme, nvim, images);
                if row.clicked() && !ui.input(|i| i.modifiers.command || i.modifiers.ctrl || i.modifiers.shift) && !filtering {
                    self.tree.toggle(&target);
                }
                if expanded {
                    self.show_dir(ui, &target, depth + 1, theme, nvim, images);
                }
                let bottom = ui.min_rect().bottom().max(row.rect.bottom());
                let area = egui::Rect::from_min_max(row.rect.min, egui::pos2(row.rect.right(), bottom)).intersect(ui.clip_rect());
                if area.is_positive() { self.folder_drop_rects.push((area, zone_target)); }
                any_shown = true;
            } else {
                if filtering && !Self::filter_matches(&name, &query_lower) {
                    continue;
                }
                self.visible_paths.push(kid.path.clone());
                let selected = self.selected_paths.contains(&kid.path) || (self.selected_paths.is_empty() && self.tree.selected.as_ref() == Some(&kid.path));
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
                self.handle_tree_row(ui, &resp, &kid.path, false, depth, theme, nvim, images);
                any_shown = true;
            }
        }
        if !draft_shown {
            self.show_new_entry_row(ui, dir, depth, theme, nvim);
        }
        if filtering && depth == 0 && !any_shown {
            ui.label(
                egui::RichText::new("No matches")
                    .size(12.0)
                    .color(theme.faint),
            );
        }
    }
    fn show_tree_drag_feedback(&mut self, ui: &mut egui::Ui, theme: &VscodeTheme, nvim: &mut NeovimBackend, images: &mut ImageCache) {
        let Some(payload) = egui::DragAndDrop::payload::<TreeDragPayload>(ui.ctx()) else { return; };
        let Some(pointer) = ui.ctx().pointer_interact_pos() else { return; };
        let target = deepest_drop_target(&self.folder_drop_rects, pointer);
        let valid = target.as_ref().is_some_and(|(_, dest)| {
            !payload.0.iter().any(|src| dest == src || dest.starts_with(src))
                && payload.0.iter().any(|src| src.parent() != Some(dest.as_path()))
        });
        if let Some((rect, _)) = &target {
            let color = if valid { theme.accent } else { theme.danger };
            ui.painter().rect_filled(*rect, 0.0, color.linear_multiply(0.16));
            ui.painter().rect_stroke(*rect, 0.0, egui::Stroke::new(1.0, color), egui::StrokeKind::Inside);
        }
        let name = if payload.0.len() == 1 {
            payload.0[0].file_name().unwrap_or_default().to_string_lossy().into_owned()
        } else { format!("{} items", payload.0.len()) };
        let destination = target.as_ref().map(|(_, p)| p.file_name().unwrap_or_default().to_string_lossy().into_owned()).unwrap_or_default();
        egui::Area::new(egui::Id::new("explorer-drag-preview"))
            .order(egui::Order::Tooltip).fixed_pos(pointer + egui::vec2(16.0, 18.0))
            .interactable(false).show(ui.ctx(), |ui| {
                egui::Frame::new().fill(theme.widget_bg).stroke(egui::Stroke::new(1.0, theme.widget_border))
                    .corner_radius(egui::CornerRadius::same(4)).inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(name).color(theme.text));
                        if !destination.is_empty() {
                            ui.label(egui::RichText::new(if valid { format!("Move to {destination}") } else { "Cannot move here".to_owned() }).size(11.0).color(if valid { theme.muted } else { theme.danger }));
                        }
                    });
            });
        if ui.input(|i| i.pointer.any_released()) {
            if let Some((_, dest)) = target.filter(|_| valid) {
                for path in &payload.0 { images.invalidate(path); }
                self.move_dropped_onto(payload.0.clone(), dest, nvim);
            }
            egui::DragAndDrop::take_payload::<TreeDragPayload>(ui.ctx());
        }
    }

    fn select_tree_path(&mut self, path: &std::path::Path, modifiers: egui::Modifiers) {
        if modifiers.shift {
            let anchor = self.selection_anchor.as_ref().unwrap_or(&path.to_path_buf()).clone();
            let order = &self.previous_visible_paths;
            if let (Some(a), Some(b)) = (order.iter().position(|p| p == &anchor), order.iter().position(|p| p == path)) {
                if !modifiers.command && !modifiers.ctrl { self.selected_paths.clear(); }
                self.selected_paths.extend(order[a.min(b)..=a.max(b)].iter().cloned());
            } else {
                self.selected_paths.clear();
                self.selected_paths.insert(path.to_path_buf());
            }
        } else if modifiers.command || modifiers.ctrl {
            if !self.selected_paths.remove(path) { self.selected_paths.insert(path.to_path_buf()); }
            self.selection_anchor = Some(path.to_path_buf());
        } else {
            self.selected_paths.clear();
            self.selected_paths.insert(path.to_path_buf());
            self.selection_anchor = Some(path.to_path_buf());
        }
        self.tree.selected = self.selected_paths.contains(path).then(|| path.to_path_buf());
    }

    fn selected_roots(&self) -> Vec<PathBuf> {
        let mut paths: Vec<_> = self.selected_paths.iter().cloned().collect();
        paths.sort();
        paths.iter().filter(|p| !paths.iter().any(|parent| parent != *p && p.starts_with(parent))).cloned().collect()
    }

    fn handle_tree_row(&mut self, ui: &mut egui::Ui, response: &egui::Response, path: &std::path::Path, is_dir: bool, depth: usize, theme: &VscodeTheme, nvim: &mut NeovimBackend, images: &mut ImageCache) {
        if response.secondary_clicked() && !self.selected_paths.contains(path) {
            self.select_tree_path(path, egui::Modifiers::NONE);
        }
        if response.drag_started() {
            if !self.selected_paths.contains(path) { self.select_tree_path(path, egui::Modifiers::NONE); }
            response.dnd_set_drag_payload(TreeDragPayload(self.selected_roots()));
        }
        if response.double_clicked() && !is_dir && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| pos.x >= response.rect.left() + depth as f32 * TREE_INDENT + 38.0) {
            self.select_tree_path(path, egui::Modifiers::NONE);
            self.begin_rename(path);
        } else if response.clicked() {
            let modifiers = ui.input(|i| i.modifiers);
            self.select_tree_path(path, modifiers);
            if !is_dir && !modifiers.command && !modifiers.ctrl && !modifiers.shift { self.open_file(path.to_path_buf(), nvim); }
        }
        response.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                self.copied_paths = self.selected_roots();
                ui.ctx().copy_text(self.copied_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"));
                ui.close();
            }
            if is_dir && !self.copied_paths.is_empty() && ui.button("Paste").clicked() {
                self.paste_explorer_items(path.to_path_buf(), nvim);
                ui.close();
            }
            if self.selected_paths.len() == 1 && ui.button("Rename").clicked() {
                self.begin_rename(path);
                ui.close();
            }
            if ui.button(egui::RichText::new("Delete").color(theme.danger)).clicked() {
                self.delete_pending = Some(self.selected_roots());
                ui.close();
            }
        });
        self.show_rename_row(ui, response.rect, path, depth, theme, nvim, images);
    }

    fn begin_rename(&mut self, path: &std::path::Path) {
        self.rename_draft = Some(RenameDraft { path: path.to_path_buf(), name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(), error: None, focus_requested: false });
    }

    fn show_rename_row(&mut self, ui: &mut egui::Ui, row: egui::Rect, path: &std::path::Path, depth: usize, theme: &VscodeTheme, nvim: &mut NeovimBackend, images: &mut ImageCache) {
        let Some(draft) = self.rename_draft.as_mut().filter(|d| d.path == path) else { return; };
        let x = (row.left() + depth as f32 * TREE_INDENT + 38.0).min(row.right());
        let rect = egui::Rect::from_min_max(egui::pos2(x, row.top()), row.right_bottom());
        ui.painter().rect_filled(rect, 0.0, theme.input_bg);
        let response = ui.put(rect, egui::TextEdit::singleline(&mut draft.name).id(egui::Id::new(("rename-entry", path))));
        if !draft.focus_requested { response.request_focus(); draft.focus_requested = true; }
        let cancel = (response.has_focus() || response.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape));
        let commit = !cancel && (response.lost_focus() || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))));
        if let Some(error) = &draft.error { response.on_hover_text(error); }
        if cancel { self.rename_draft = None; }
        else if commit { self.commit_rename(nvim, images); }
    }

    fn start_new_entry(&mut self, kind: EntryKind, nvim: &mut NeovimBackend) {
        if self.new_entry.as_ref().is_some_and(|draft| !draft.name.trim().is_empty()) {
            self.commit_new_entry(nvim);
            if self.new_entry.is_some() { return; }
        }
        if let Some(target) = self.new_entry_target() {
            self.tree.expanded.insert(target.clone());
            self.explorer_open = true;
            self.explorer_filter.clear();
            self.new_entry = Some(NewEntryDialog { kind, name: String::new(), error: None, target, focus_requested: false });
        }
    }

    fn show_new_entry_row(&mut self, ui: &mut egui::Ui, dir: &std::path::Path, depth: usize, theme: &VscodeTheme, nvim: &mut NeovimBackend) {
        if !self.new_entry.as_ref().is_some_and(|draft| draft.target == dir) { return; }
        let mut commit = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * TREE_INDENT + 20.0);
            let draft = self.new_entry.as_mut().unwrap();
            let icon = match draft.kind {
                EntryKind::File => icons::TreeIcon::File(icons::file_icon_for(std::path::Path::new(&draft.name))),
                EntryKind::Folder => icons::TreeIcon::Folder(icons::folder_kind_for(&draft.name)),
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            icons::draw_tree_icon(ui.painter(), icon, rect, theme, false);
            let response = ui.add_sized(egui::vec2(ui.available_width().max(20.0), TREE_ROW_HEIGHT), egui::TextEdit::singleline(&mut draft.name).hint_text("Name"));
            if !draft.focus_requested { response.request_focus(); response.scroll_to_me(Some(egui::Align::Center)); draft.focus_requested = true; }
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if escape && (response.has_focus() || response.lost_focus()) {
                cancel = true;
            } else if response.lost_focus() {
                if draft.name.trim().is_empty() { cancel = true; } else { commit = true; }
            } else if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                commit = true;
            }
        });
        if let Some(error) = self.new_entry.as_ref().and_then(|draft| draft.error.as_ref()) {
            ui.colored_label(theme.danger, error);
        }
        if cancel { self.new_entry = None; }
        else if commit { self.commit_new_entry(nvim); }
    }

    /// Full-sidebar-width tree row (VSCode-style): selected paints
    /// `list_active_sel`, hover paints `list_hover`. `twistie`:
    /// `None` for files, `Some(expanded)` for folders. `icon` is the
    /// Seti-style painter glyph (no assets, no textures — see `icons.rs`).
    pub(super) fn tree_row(
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
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height())),
                0.0,
                theme.accent,
            );
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
        ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())).text(
            egui::pos2(base_x + CHEVRON_W + ICON_W, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            fg,
        );
        resp.on_hover_text(label)
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn drop_target_picks_deepest_visible_folder_or_root() {
        let rect = |y1, y2| egui::Rect::from_min_max(egui::pos2(0.0, y1), egui::pos2(200.0, y2));
        let targets = vec![(rect(20.0, 100.0), PathBuf::from("root/folder")), (rect(40.0, 70.0), PathBuf::from("root/folder/nested")), (rect(0.0, 300.0), PathBuf::from("root"))];
        assert_eq!(deepest_drop_target(&targets, egui::pos2(50.0, 50.0)).unwrap().1, PathBuf::from("root/folder/nested"));
        assert_eq!(deepest_drop_target(&targets, egui::pos2(50.0, 90.0)).unwrap().1, PathBuf::from("root/folder"));
        assert_eq!(deepest_drop_target(&targets, egui::pos2(50.0, 200.0)).unwrap().1, PathBuf::from("root"));
        assert!(deepest_drop_target(&targets, egui::pos2(220.0, 50.0)).is_none());
    }

    #[test]
    fn modifiers_select_visible_range_and_toggle() {
        let mut window = RepoWindow::blank(egui::ViewportId::ROOT);
        let paths: Vec<PathBuf> = ["a", "b", "c", "d"].iter().map(PathBuf::from).collect();
        window.previous_visible_paths = paths.clone();
        window.select_tree_path(&paths[0], egui::Modifiers::NONE);
        window.select_tree_path(&paths[2], egui::Modifiers { shift: true, ..egui::Modifiers::NONE });
        assert_eq!(window.selected_paths.len(), 3);
        assert!(window.selected_paths.contains(&paths[1]));
        window.select_tree_path(&paths[1], egui::Modifiers { command: true, ..egui::Modifiers::NONE });
        assert!(!window.selected_paths.contains(&paths[1]));
        assert_eq!(window.selected_paths.len(), 2);
    }

    #[test]
    fn selected_roots_exclude_children_of_selected_folders() {
        let mut window = RepoWindow::blank(egui::ViewportId::ROOT);
        window.selected_paths.extend([PathBuf::from("dir"), PathBuf::from("dir/file"), PathBuf::from("other")]);
        assert_eq!(window.selected_roots(), vec![PathBuf::from("dir"), PathBuf::from("other")]);
    }
}
