//! Commit history viewer — shows what changed per commit.
//!
//! Renders as a sidebar panel in the repo window. Uses local `git log`
//! (no network, no rate limits) so it stays fast and works offline.

use crate::git::{GitCommit, GitHistoryOptions};
use egui::{Context, Frame, RichText, ScrollArea};
use std::path::PathBuf;

/// Commit history panel state
#[derive(Debug, Default, Clone)]
pub struct CommitHistoryPanel {
    pub show: bool,
    pub repo_path: Option<PathBuf>,
    pub commits: Vec<GitCommit>,
    pub selected_commit: Option<usize>,
    pub search_query: String,
    pub filter_by_author: Option<String>,
    pub file_filter: String,
    pub loading: bool,
    pub error: Option<String>,
}

impl CommitHistoryPanel {
    pub fn toggle(&mut self, repo_path: Option<PathBuf>) {
        if self.show {
            self.show = false;
        } else {
            self.show = true;
            self.repo_path = repo_path;
            if self.show {
                self.load_commits();
            }
        }
    }

    pub fn load_commits(&mut self) {
        let Some(repo_path) = &self.repo_path else {
            return;
        };
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;

        let opts = GitHistoryOptions {
            limit: 50,
            ..Default::default()
        };

        match GitCommit::fetch(repo_path, &opts) {
            Ok(commits) => {
                self.commits = commits;
                self.loading = false;
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.loading = false;
            }
        }
    }

    /// Filter commits by search query (message contains query)
    pub fn filtered_commits(&self) -> Vec<&GitCommit> {
        let query = self.search_query.to_lowercase();
        let author_filter = self
            .filter_by_author
            .as_ref()
            .map(|a| a.to_lowercase());

        self.commits
            .iter()
            .filter(|c| {
                let msg_match = query.is_empty()
                    || c.message.to_lowercase().contains(&query);
                let author_match = author_filter
                    .as_ref()
                    .map(|a| c.author.to_lowercase().contains(a.as_str()))
                    .unwrap_or(true);
                msg_match && author_match
            })
            .collect()
    }

    /// Render the commit history panel
    pub fn show_panel(&mut self, ctx: &egui::Context) {
        if !self.show {
            return;
        }

        egui::SidePanel::right("commit_history")
            .min_width(300.0)
            .max_width(450.0)
            .frame(Frame::default().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                ui.heading("Commit History");
                ui.add_space(8.0);

                // Search bar
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                });

                // Filter by file
                ui.horizontal(|ui| {
                    ui.label("File:");
                    ui.text_edit_singleline(&mut self.file_filter);
                });

                // Filter by author
                ui.horizontal(|ui| {
                    ui.label("Author:");
                    let mut author_text = self.filter_by_author.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut author_text).changed() {
                        self.filter_by_author = if author_text.is_empty() {
                            None
                        } else {
                            Some(author_text)
                        };
                    }
                });

                ui.add_space(8.0);

                // Refresh button
                if ui.button("Refresh").clicked() {
                    self.load_commits();
                }

                ui.add_space(8.0);

                // Loading/error states
                if self.loading {
                    ui.spinner();
                    ui.label("Loading commits...");
                    return;
                }

                if let Some(err) = &self.error {
                    ui.label(RichText::new(err).color(egui::Color32::RED));
                    return;
                }

                if self.commits.is_empty() {
                    ui.label("No commits found.");
                    return;
                }

                // Commit list
                ScrollArea::vertical().show(ui, |ui| {
                    let filtered: Vec<_> = self.filtered_commits().into_iter().cloned().collect();
                    for (i, commit) in filtered.iter().enumerate() {
                        let is_selected = self.selected_commit == Some(i);
                        let fill = if is_selected {
                            ui.style().visuals.selection.bg_fill
                        } else {
                            ui.style().visuals.extreme_bg_color
                        };
                        let frame_resp = Frame::group(ui.style())
                            .fill(fill)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&commit.short_hash).monospace().strong(),
                                    );
                                    ui.label(RichText::new(&commit.author).italics());
                                });
                                ui.label(&commit.message);
                                if !commit.files_changed.is_empty() {
                                    ui.label(
                                        RichText::new(format!(
                                            "Files: {}",
                                            commit.files_changed.join(", ")
                                        ))
                                        .size(11.0)
                                        .color(ui.style().visuals.text_color().gamma_multiply(0.7)),
                                    );
                                }
                            });
                        let resp = frame_resp
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_text(format!(
                                "Hash: {}\nDate: {}\nFiles: {}",
                                commit.hash,
                                commit.date.format("%Y-%m-%d %H:%M"),
                                commit.files_changed.len(),
                            ));
                        if resp.clicked() {
                            self.selected_commit = Some(i);
                        }
                    }
                });
            });
    }
}

/// Render commit history panel in the repo window
pub fn render_commit_history(
    panel: &mut CommitHistoryPanel,
    repo_path: Option<&PathBuf>,
    ctx: &Context,
    _theme: &crate::theme::VscodeTheme,
) {
    if !panel.show {
        return;
    }

    // Ensure repo path is set
    if panel.repo_path.is_none() {
        panel.repo_path = repo_path.cloned();
    }

    panel.show_panel(ctx);
}