//! Workspace task board (Trello-style): model, `.arctis.json` storage,
//! and the board UI.
//!
//! Boards persist per workspace (`<root>/.arctis.json`, `{ tasks: Board }`)
//! with a global fallback (`~/.arctis.json`) when no folder is open.
//! Every mutation auto-saves; a corrupt file warns on the status line and
//! starts fresh instead of crashing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::icons;
use crate::theme::VscodeTheme;

/// Fixed v1 columns: id + display title, in board order.
pub const TASK_COLUMNS: &[(&str, &str)] =
    &[("todo", "To Do"), ("doing", "Doing"), ("done", "Done")];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BoardColumn {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskCard {
    pub id: String,
    pub column: String,
    pub title: String,
    pub desc: String,
    pub order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Board {
    pub columns: Vec<BoardColumn>,
    pub cards: Vec<TaskCard>,
    pub next_id: u64,
}

impl Board {
    pub fn empty() -> Self {
        let mut board = Self {
            columns: Vec::new(),
            cards: Vec::new(),
            next_id: 1,
        };
        board.ensure_default_columns();
        board
    }

    /// Re-insert any missing default column (e.g. files predating a column).
    /// Custom extra columns are preserved in place.
    pub fn ensure_default_columns(&mut self) {
        for (id, title) in TASK_COLUMNS {
            if !self.columns.iter().any(|c| c.id == *id) {
                self.columns.push(BoardColumn {
                    id: id.to_string(),
                    title: title.to_string(),
                });
            }
        }
    }

    /// Cards of one column, top-to-bottom.
    pub fn column_cards(&self, column: &str) -> Vec<&TaskCard> {
        let mut cards: Vec<&TaskCard> =
            self.cards.iter().filter(|c| c.column == column).collect();
        cards.sort_by_key(|c| c.order);
        cards
    }

    /// Column index left of `column`, if any.
    fn sibling(&self, column: &str, delta: isize) -> Option<String> {
        let pos = self.columns.iter().position(|c| c.id == column)?;
        let next = pos as isize + delta;
        if next < 0 {
            return None;
        }
        self.columns.get(next as usize).map(|c| c.id.clone())
    }

    fn normalize(&mut self) {
        for col in self.columns.clone() {
            let mut ids: Vec<(usize, String)> = self
                .cards
                .iter()
                .filter(|c| c.column == col.id)
                .map(|c| (c.order, c.id.clone()))
                .collect();
            ids.sort();
            for (order, id) in ids.iter().enumerate() {
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id.1) {
                    card.order = order;
                }
            }
        }
    }

    pub fn add_card(&mut self, column: &str, title: String) -> String {
        let order = self
            .cards
            .iter()
            .filter(|c| c.column == column)
            .map(|c| c.order)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        let id = format!("c{}", self.next_id);
        self.next_id += 1;
        self.cards.push(TaskCard {
            id: id.clone(),
            column: column.to_string(),
            title,
            desc: String::new(),
            order,
        });
        self.normalize();
        id
    }

    pub fn rename_card(&mut self, id: &str, title: String) -> bool {
        match self.cards.iter_mut().find(|c| c.id == id) {
            Some(card) => {
                card.title = title;
                true
            }
            None => false,
        }
    }

    pub fn set_desc(&mut self, id: &str, desc: String) -> bool {
        match self.cards.iter_mut().find(|c| c.id == id) {
            Some(card) => {
                card.desc = desc;
                true
            }
            None => false,
        }
    }

    pub fn delete_card(&mut self, id: &str) -> bool {
        let before = self.cards.len();
        self.cards.retain(|c| c.id != id);
        let removed = self.cards.len() != before;
        if removed {
            self.normalize();
        }
        removed
    }

    /// Move one column left/right. Returns false at the edges.
    pub fn move_column(&mut self, id: &str, delta: isize) -> bool {
        let current = match self.cards.iter().find(|c| c.id == id) {
            Some(card) => card.column.clone(),
            None => return false,
        };
        match self.sibling(&current, delta) {
            Some(next) => {
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                    card.column = next;
                }
                self.normalize();
                // Land at the bottom of the new column.
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                    card.order = usize::MAX;
                }
                self.normalize();
                true
            }
            None => false,
        }
    }

    /// Move up/down within the column. Returns false at the edges.
    pub fn move_within(&mut self, id: &str, delta: isize) -> bool {
        let (column, order) = match self.cards.iter().find(|c| c.id == id) {
            Some(card) => (card.column.clone(), card.order as isize),
            None => return false,
        };
        let target = order + delta;
        if target < 0 {
            return false;
        }
        let neighbor = self
            .cards
            .iter()
            .find(|c| c.column == column && c.order as isize == target)
            .map(|c| c.id.clone());
        match neighbor {
            Some(other) => {
                for card in self.cards.iter_mut() {
                    if card.id == id {
                        card.order = target as usize;
                    } else if card.id == other {
                        card.order = order as usize;
                    }
                }
                true
            }
            None => false,
        }
    }
}

/// On-disk workspace shape: `{ "tasks": Board }`. Unknown extra keys are
/// ignored so the file stays forward-compatible.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceDoc {
    #[serde(default)]
    pub tasks: Board,
}

impl Default for Board {
    fn default() -> Self {
        Self::empty()
    }
}

/// Workspace tasks file, or the global fallback when no folder is open.
pub fn tasks_path(root: Option<&Path>) -> PathBuf {
    match root {
        Some(r) => r.join(".arctis.json"),
        None => global_tasks_path(),
    }
}

pub fn global_tasks_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".arctis.json"))
        .unwrap_or_else(|| PathBuf::from(".arctis.json"))
}

/// Parse file content: corrupt input warns (for the status line) and yields
/// a fresh board; a missing file is silent (normal first run).
pub fn parse_tasks(content: Option<&str>) -> (Board, Option<String>) {
    match content {
        None => (Board::empty(), None),
        Some(text) => match serde_json::from_str::<WorkspaceDoc>(text) {
            Ok(mut doc) => {
                doc.tasks.ensure_default_columns();
                (doc.tasks, None)
            }
            Err(e) => (
                Board::empty(),
                Some(format!("tasks file unreadable ({e}); started fresh")),
            ),
        },
    }
}

pub fn load_tasks(path: &Path) -> (Board, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(text) => parse_tasks(Some(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => parse_tasks(None),
        Err(e) => (
            Board::empty(),
            Some(format!("tasks file unreadable ({e}); started fresh")),
        ),
    }
}

pub fn save_tasks(path: &Path, board: &Board) -> anyhow::Result<()> {
    let doc = WorkspaceDoc {
        tasks: board.clone(),
    };
    let text = serde_json::to_string_pretty(&doc)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Ephemeral UI state (never serialized): per-column draft text for the
/// add-card inputs.
#[derive(Debug, Default)]
pub struct BoardState {
    pub board: Board,
    pub new_text: HashMap<String, String>,
}

impl BoardState {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            new_text: HashMap::new(),
        }
    }
}

/// Render the board. Returns true when the model mutated (caller saves).
pub fn show_board(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    state: &mut BoardState,
) -> bool {
    let mut mutated = false;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("TASKS")
                .size(11.0)
                .color(theme.muted),
        );
    });
    ui.add_space(4.0);

    // Snapshot column order; all mutations go through Board methods.
    let columns: Vec<(String, String)> = state
        .board
        .columns
        .iter()
        .map(|c| (c.id.clone(), c.title.clone()))
        .collect();
    egui::ScrollArea::both()
        .id_salt("tasks-board")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (col_id, col_title) in &columns {
                    mutated |= show_column(ui, theme, state, col_id, col_title);
                }
            });
        });
    mutated
}

fn show_column(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    state: &mut BoardState,
    col_id: &str,
    col_title: &str,
) -> bool {
    let mut mutated = false;
    egui::Frame::default()
        .fill(theme.card_bg)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(264.0, 0.0));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(col_title)
                        .size(12.0)
                        .color(theme.text),
                );
                let n = state.board.column_cards(col_id).len();
                ui.label(
                    egui::RichText::new(format!("{n}"))
                        .size(11.0)
                        .color(theme.muted),
                );
            });
            ui.add_space(4.0);

            let ids: Vec<String> = state
                .board
                .column_cards(col_id)
                .iter()
                .map(|c| c.id.clone())
                .collect();
            for id in &ids {
                mutated |= show_card(ui, theme, state, id);
            }
            if ids.is_empty() {
                ui.label(
                    egui::RichText::new("No cards yet.")
                        .size(12.0)
                        .color(theme.faint),
                );
            }
            ui.add_space(4.0);

            // Add-card row: button or Enter commits non-empty drafts.
            ui.horizontal(|ui| {
                let draft = state.new_text.entry(col_id.to_string()).or_default();
                let resp = ui.add_sized(
                    egui::vec2((ui.available_width() - 56.0).max(80.0), 24.0),
                    egui::TextEdit::singleline(draft)
                        .hint_text("Add a card")
                        .frame(true),
                );
                let enter = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (icons::primary_button(ui, theme, "Add").clicked() || enter) && !draft.trim().is_empty() {
                    state.board.add_card(col_id, draft.trim().to_owned());
                    state.new_text.insert(col_id.to_string(), String::new());
                    mutated = true;
                }
            });
        });
    mutated
}

fn show_card(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    state: &mut BoardState,
    id: &str,
) -> bool {
    let mut mutated = false;
    // Stable id scope per card so cursor/focus survive reorders.
    ui.push_id(("task-card", id.to_string()), |ui| {
        egui::Frame::default()
            .fill(theme.input_bg)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(ui.available_width(), 0.0));
                let (mut title, mut desc) =
                    match state.board.cards.iter().find(|c| c.id == id) {
                        Some(c) => (c.title.clone(), c.desc.clone()),
                        None => return,
                    };
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut title)
                            .hint_text("Card title")
                            .frame(false),
                    )
                    .changed()
                {
                    mutated |= state.board.rename_card(id, title);
                }
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut desc)
                            .hint_text("Details…")
                            .frame(false)
                            .desired_rows(1),
                    )
                    .changed()
                {
                    mutated |= state.board.set_desc(id, desc);
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    if ui.small_button("‹").clicked() {
                        mutated |= state.board.move_column(id, -1);
                    }
                    if ui.small_button("›").clicked() {
                        mutated |= state.board.move_column(id, 1);
                    }
                    if ui.small_button("↑").clicked() {
                        mutated |= state.board.move_within(id, -1);
                    }
                    if ui.small_button("↓").clicked() {
                        mutated |= state.board.move_within(id, 1);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            mutated |= state.board.delete_card(id);
                        }
                    });
                });
            });
    });
    mutated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> Board {
        let mut b = Board::empty();
        b.add_card("todo", "first".into());
        b.add_card("todo", "second".into());
        b.add_card("doing", "wip".into());
        b
    }

    #[test]
    fn crud_and_order() {
        let mut b = seeded();
        let ids: Vec<String> = b
            .column_cards("todo")
            .iter()
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(ids.len(), 2);

        // Move second card up → it becomes first.
        assert!(b.move_within(&ids[1], -1));
        let top: Vec<String> = b
            .column_cards("todo")
            .iter()
            .map(|c| c.title.clone())
            .collect();
        assert_eq!(top, vec!["second".to_string(), "first".to_string()]);

        // Edges refuse.
        assert!(!b.move_within(&ids[1], -1));
        assert!(!b.move_column(&ids[1], -1));

        // Across columns, lands at the bottom.
        assert!(b.move_column(&ids[1], 1));
        assert_eq!(b.column_cards("doing").len(), 2);
        assert_eq!(b.column_cards("todo").len(), 1);

        // Rename + desc + delete.
        assert!(b.rename_card(&ids[0], "renamed".into()));
        assert!(b.set_desc(&ids[0], "details".into()));
        assert!(b.delete_card(&ids[0]));
        assert!(!b.delete_card("nope"));
        assert_eq!(b.column_cards("todo").len(), 0);
    }

    #[test]
    fn serde_round_trip() {
        let b = seeded();
        let text = serde_json::to_string(&WorkspaceDoc { tasks: b.clone() }).unwrap();
        let (back, warning) = parse_tasks(Some(&text));
        assert!(warning.is_none());
        assert_eq!(back, b);
    }

    #[test]
    fn corrupt_and_missing_start_fresh() {
        let (board, warning) = parse_tasks(Some("{oops"));
        assert!(warning.is_some());
        assert_eq!(board.columns.len(), TASK_COLUMNS.len());

        let (board, warning) = parse_tasks(None);
        assert!(warning.is_none());
        assert!(board.cards.is_empty());
    }

    #[test]
    fn save_and_reload_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".arctis.json");
        let b = seeded();
        save_tasks(&path, &b).unwrap();
        let (back, warning) = load_tasks(&path);
        assert!(warning.is_none());
        assert_eq!(back, b);
    }

    #[test]
    fn workspace_path_resolution() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            tasks_path(Some(dir.path())),
            dir.path().join(".arctis.json")
        );
        // Global fallback is a plain .json file under home (or cwd).
        assert_eq!(
            global_tasks_path().extension().and_then(|e| e.to_str()),
            Some("json")
        );
    }
}
