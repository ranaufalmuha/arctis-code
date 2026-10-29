//! One repo window: activity bar, titlebar, status-adjacent chrome.
use super::{CenterView, RepoWindow, TITLEBAR_HEIGHT, TITLEBAR_TRAFFIC_PAD};
use crate::editor::NeovimBackend;
use crate::icons::{self, Glyph};
use crate::theme::{VscodeTheme, ACTIVITY_BAR_WIDTH};

/// Shared, compact search field for the explorer and editor.
fn search_field(ui: &mut egui::Ui, rect: egui::Rect, value: &mut String, hint: &str, theme: &VscodeTheme) -> egui::Response {
    let response = ui.put(rect, egui::TextEdit::singleline(value)
        .font(egui::TextStyle::Body)
        .margin(egui::Margin { left: 26, right: 6, top: 4, bottom: 4 })
        .hint_text(hint));
    let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 14.0, rect.center().y), egui::vec2(14.0, 14.0));
    icons::draw_glyph(ui.painter(), Glyph::Search, icon, theme.muted, 1.3);
    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) { value.clear(); }
    response
}

impl RepoWindow {
    pub(super) fn show_activity_bar(&mut self, ui: &mut egui::Ui, theme: &VscodeTheme) {
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
    pub(super) fn activity_icon(
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
    pub(super) fn show_titlebar(&mut self, ctx: &egui::Context, theme: &VscodeTheme) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(TITLEBAR_HEIGHT)
            .frame(egui::Frame::new().fill(theme.tabstrip_bg))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(TITLEBAR_TRAFFIC_PAD);
                    let left_width = if self.sidebar_visible {
                        (ACTIVITY_BAR_WIDTH + self.sidebar_width - TITLEBAR_TRAFFIC_PAD).max(0.0)
                    } else {
                        (ACTIVITY_BAR_WIDTH - TITLEBAR_TRAFFIC_PAD).max(0.0)
                    };
                    if self.sidebar_visible {
                        ui.allocate_ui_with_layout(egui::vec2(left_width, TITLEBAR_HEIGHT), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if icons::icon_button(ui, theme, Glyph::Explorer, "Open folder", 24.0).clicked() {
                                if let Some(dir) = crate::project::opener::pick_folder_dialog() {
                                    self.open_project(dir);
                                }
                            }
                            let width = (ui.available_width() - 8.0).max(0.0);
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
                            search_field(ui, rect, &mut self.explorer_filter, "Search files…", theme);
                        });
                    }
                    ui.add_space(12.0);
                    let name = self.project.root.as_ref().and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "ArctisCode".to_owned());
                    ui.add(egui::Label::new(egui::RichText::new(&name).size(13.0).strong().color(theme.text)).truncate()).on_hover_text(name);
                });
            });
    }

    pub(super) fn show_tabs(&mut self, ui: &mut egui::Ui, theme: &VscodeTheme, nvim: &mut NeovimBackend, guards: bool, format: bool) {
        let dirty: Vec<bool> = self.tabs.tabs.iter().map(|t| t.dirty).collect();
        let previous_tab = self.tabs.active_ref().map(|tab| tab.path.clone());
        let mut pending = None;
        let width = ui.available_width();
        let height = crate::theme::TAB_STRIP_HEIGHT;
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter().rect_filled(row, 0.0, theme.tabstrip_bg);
        let find_width = 220.0_f32.min((width * 0.45).max(0.0));
        let tabs_rect = egui::Rect::from_min_max(row.min, egui::pos2(row.right() - find_width, row.bottom()));
        let find_rect = egui::Rect::from_center_size(
            egui::pos2(row.right() - find_width / 2.0, row.center().y),
            egui::vec2((find_width - 12.0).max(0.0), 26.0),
        );
        let mut tab_ui = ui.new_child(egui::UiBuilder::new().id_salt("tabs-row").max_rect(tabs_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        tab_ui.set_clip_rect(tabs_rect.intersect(ui.clip_rect()));
        let scroll = &mut tab_ui.spacing_mut().scroll;
        *scroll = egui::style::ScrollStyle::floating();
        scroll.bar_width = 4.0;
        scroll.floating_width = 2.0;
        crate::tabs::bar::show_row(&mut tab_ui, theme, &mut self.tabs, &mut |idx| {
            if dirty.get(idx).copied().unwrap_or(false) { pending = Some(idx); false } else { true }
        });
        let response = search_field(ui, find_rect, &mut self.find_query, "Find", theme);
        if response.changed() { self.find_cache.0.clear(); }
        if !self.find_query.is_empty() {
            if let Some(tab) = self.tabs.active_ref() {
                let (query, len, hits) = &mut self.find_cache;
                if query != &self.find_query || *len != tab.content.len() {
                    *hits = tab.content.matches(self.find_query.as_str()).count();
                    *query = self.find_query.clone();
                    *len = tab.content.len();
                }
                response.on_hover_text(format!("{} found", *hits));
            }
        }
        if previous_tab != self.tabs.active_ref().map(|tab| tab.path.clone()) {
            self.find_cache.0.clear();
        }
        if let Some(idx) = pending {
            if guards {
                self.confirm_close = Some(idx);
            } else {
                self.tabs.active = Some(idx);
                self.save_active(nvim, format, false);
                self.tabs.close(idx);
            }
        }
    }
}
