//! One repo window: editor + image preview center view.
use super::RepoWindow;
use crate::editor::NeovimBackend;
use crate::icons;
use crate::preview::ImageCache;
use crate::project::opener;
use crate::settings::Settings;
use crate::tabs::TabKind;
use crate::theme::VscodeTheme;

impl RepoWindow {
    pub(super) fn show_editor(
        &mut self,
        ui: &mut egui::Ui,
        theme: &VscodeTheme,
        nvim: &mut NeovimBackend,
        images: &mut ImageCache,
        settings: &Settings,
    ) {
        // Keep the cursor readout honest when the active tab changes.
        let active_path = self.tabs.active_ref().map(|t| t.path.clone());
        if active_path != self.last_tab_path {
            self.cursor_pos = None;
            self.last_cursor_idx = None;
            self.last_tab_path = active_path;
            self.edit_prev = self
                .tabs
                .active_ref()
                .map(|t| t.content.clone())
                .unwrap_or_default();
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

        // Image preview branch: decode happens on a background thread so
        // the UI never freezes; shrink-to-fit, never larger than the
        // window, small images keep their native size.
        if self.tabs.tabs[idx].kind == TabKind::Image {
            self.cursor_pos = None;
            let path = self.tabs.tabs[idx].path.clone();
            match images.texture_for(ui.ctx(), &path) {
                Ok(Some(tex)) => {
                    egui::ScrollArea::both()
                        .id_salt("image-preview")
                        .show(ui, |ui| {
                            ui.heading(path.display().to_string());
                            ui.vertical_centered(|ui| {
                                let size = crate::preview::fit_contain(
                                    tex.size_vec2(),
                                    ui.available_size(),
                                );
                                ui.add(egui::Image::new(&tex).fit_to_exact_size(size));
                            });
                        });
                }
                Err(e) => {
                    ui.label(format!("preview failed: {e:#}"));
                }
                Ok(None) => {
                    // Still decoding: spinner keeps the frame alive.
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label("Loading preview…");
                    });
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
        // Full-height editor: minimum height is the remaining panel space,
        // so short files still fill the view; long files grow and scroll.
        let editor_h = (ui.available_height() - 4.0).max(120.0);
        // Gutter + code share one scroll area so they scroll in lockstep.
        // No-wrap layout keeps visual rows 1:1 with line numbers.
        let mono = egui::TextStyle::Monospace;
        let mono_id = mono.resolve(ui.style());
        let count = crate::editor::line_count(&self.tabs.tabs[idx].content);
        // Rebuild the gutter string only when the line count changes.
        if self.gutter_cache.0 != count {
            self.gutter_cache = (count, crate::editor::gutter_text(count).0);
        }
        let gutter_string = self.gutter_cache.1.clone();
        let gutter_w = ui.fonts(|f| f.glyph_width(&mono_id, '0')) * 6.0 + 16.0;
        let editor_id = egui::Id::new(("code-editor", self.tabs.tabs[idx].path.clone()));

        // Previous frame's cursor line gets the brighter VSCode number.
        let active_line = self.cursor_pos.map(|(line, _)| line);
        let gutter_mono = mono_id.clone();
        let gutter_fg = theme.gutter_fg;
        let gutter_active = theme.gutter_active_fg;
        let mut gutter_layouter = move |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
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
                let number = line.trim_start();
                let number_width = ui.fonts(|f| f.layout_no_wrap(number.to_owned(), gutter_mono.clone(), color)).size().x;
                job.append(
                    number,
                    ((gutter_w - 8.0 - number_width) * 0.5).max(0.0),
                    egui::text::TextFormat::simple(gutter_mono.clone(), color),
                );
            }
            job.wrap.max_width = f32::INFINITY;
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
                        // Highlight spans for this buffer; re-parsed only on
                        // edit/switch (HlCache), never every frame.
                        let spans = {
                            let t = &self.tabs.tabs[idx];
                            self.hl.spans_for(&t.path, &t.content).map(|s| s.to_vec())
                        };
                        let tab = &mut self.tabs.tabs[idx];
                        let text_color = ui.visuals().text_color();
                        let font_id = mono_id.clone();
                        let mut layouter = |ui: &egui::Ui,
                                            text: &dyn egui::TextBuffer,
                                            _wrap: f32| {
                            let body = text.as_str();
                            let mut job = egui::text::LayoutJob::default();
                            let mut pos = 0usize;
                            // Stale spans can't panic: every slice is
                            // bounds-checked, worst case one plain frame.
                            if let Some(spans) = spans.as_ref() {
                                for s in spans {
                                    if let Some(gap) = body.get(pos..s.start) {
                                        if !gap.is_empty() {
                                            job.append(
                                                gap,
                                                0.0,
                                                egui::text::TextFormat::simple(
                                                    font_id.clone(),
                                                    text_color,
                                                ),
                                            );
                                        }
                                    }
                                    if let Some(tok) = body.get(s.start..s.end) {
                                        job.append(
                                            tok,
                                            0.0,
                                            egui::text::TextFormat::simple(
                                                font_id.clone(),
                                                s.color,
                                            ),
                                        );
                                    }
                                    pos = pos.max(s.end);
                                }
                            }
                            if let Some(rest) = body.get(pos..) {
                                if !rest.is_empty() {
                                    job.append(
                                        rest,
                                        0.0,
                                        egui::text::TextFormat::simple(font_id.clone(), text_color),
                                    );
                                }
                            }
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
                            self.find_cache.0.clear();
                            let cursor = egui::TextEdit::load_state(ui.ctx(), editor_id)
                                .and_then(|s| s.cursor.char_range())
                                .map(|r| r.primary.index);
                            let (auto, smart) =
                                (settings.auto_close_brackets, settings.smart_enter);
                            self.apply_auto_edit(ui.ctx(), editor_id, idx, cursor, auto, smart);
                            let live = &self.tabs.tabs[idx];
                            nvim.sync_buffer(&live.path.clone(), &live.content.clone());
                            self.hl.mark_dirty();
                        }
                        // Cursor readout for the status bar; line_col scans
                        // text, so recompute only when the cursor moved.
                        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                            if let Some(range) = state.cursor.char_range() {
                                let ci = range.primary.index;
                                if self.last_cursor_idx != Some(ci) {
                                    self.last_cursor_idx = Some(ci);
                                    let (line, col) =
                                        crate::editor::line_col(&self.tabs.tabs[idx].content, ci);
                                    self.cursor_pos = Some((line, col));
                                }
                            }
                        }
                    });
                });
        });
    }

    /// Bracket pairing + smart Enter after a user edit. Reads the real
    /// widget cursor, applies `autoedit::plan`, writes the cursor back.
    fn apply_auto_edit(
        &mut self,
        ctx: &egui::Context,
        editor_id: egui::Id,
        idx: usize,
        cursor: Option<usize>,
        auto_close: bool,
        smart_enter: bool,
    ) {
        let cur = self.tabs.tabs[idx].content.clone();
        let Some(cc) = cursor else {
            self.edit_prev = cur;
            return;
        };
        if let Some((next, at)) =
            crate::editor::autoedit::plan(&self.edit_prev, &cur, cc, auto_close, smart_enter)
        {
            self.tabs.tabs[idx].content = next.clone();
            if let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(at),
                )));
                egui::TextEdit::store_state(ctx, editor_id, st);
            }
            self.edit_prev = next;
            // Cursor moved programmatically: force the readout to catch up.
            self.last_cursor_idx = None;
        } else {
            self.edit_prev = cur;
        }
    }
}
