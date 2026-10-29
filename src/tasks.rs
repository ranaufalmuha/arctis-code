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

/// Fixed columns: Plan -> In Progress -> Done. Nothing else.
pub const TASK_COLUMNS: &[(&str, &str)] = &[
    ("plan", "Plan"),
    ("progress", "In Progress"),
    ("done", "Done"),
];

/// How urgent a card is. Shown as a colored chip; one click cycles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    #[default]
    Medium,
    High,
}

impl Urgency {
    pub fn label(self) -> &'static str {
        match self {
            Urgency::Low => "Low",
            Urgency::Medium => "Medium",
            Urgency::High => "High",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Urgency::Low => Urgency::Medium,
            Urgency::Medium => Urgency::High,
            Urgency::High => Urgency::Low,
        }
    }

    pub fn color(self) -> egui::Color32 {
        match self {
            Urgency::Low => egui::Color32::from_rgb(0x4C, 0xAF, 0x50),
            Urgency::Medium => egui::Color32::from_rgb(0xE5, 0xC0, 0x7B),
            Urgency::High => egui::Color32::from_rgb(0xF1, 0x4C, 0x4C),
        }
    }
}

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
    /// Missing in files written before urgency existed → Medium.
    #[serde(default)]
    pub urgency: Urgency,
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

    /// Migrate pre-Trello boards: `todo`/`doing` columns become
    /// `plan`/`progress`, cards follow them, urgency defaults to Medium
    /// (see `#[serde(default)]`). Columns end up in canonical order.
    pub fn migrate_legacy(&mut self) {
        const RENAME: &[(&str, &str)] = &[("todo", "plan"), ("doing", "progress")];
        for (old, new) in RENAME {
            for c in &mut self.columns {
                if c.id == *old {
                    c.id = new.to_string();
                }
            }
            for card in &mut self.cards {
                if card.column == *old {
                    card.column = new.to_string();
                }
            }
        }
        let mut ordered = Vec::with_capacity(self.columns.len());
        for (id, title) in TASK_COLUMNS {
            if let Some(pos) = self.columns.iter().position(|c| c.id == *id) {
                let mut col = self.columns.remove(pos);
                col.title = title.to_string();
                ordered.push(col);
            } else {
                ordered.push(BoardColumn {
                    id: id.to_string(),
                    title: title.to_string(),
                });
            }
        }
        // Unknown extra columns (if any) survive at the end.
        ordered.append(&mut self.columns);
        self.columns = ordered;
    }

    /// Cards of one column, top-to-bottom.
    pub fn column_cards(&self, column: &str) -> Vec<&TaskCard> {
        let mut cards: Vec<&TaskCard> = self.cards.iter().filter(|c| c.column == column).collect();
        cards.sort_by_key(|c| c.order);
        cards
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
            urgency: Urgency::default(),
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

    pub fn set_urgency(&mut self, id: &str, urgency: Urgency) -> bool {
        match self.cards.iter_mut().find(|c| c.id == id) {
            Some(card) => {
                card.urgency = urgency;
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

    /// Drop target: put `id` into `column` at `index` (clamped), then
    /// renumber that column 0..n. Unknown card or column → false.
    pub fn move_card_to(&mut self, id: &str, column: &str, index: usize) -> bool {
        if !self.columns.iter().any(|c| c.id == column) {
            return false;
        }
        let mut ids: Vec<String> = self
            .column_cards(column)
            .iter()
            .map(|c| c.id.clone())
            .collect();
        let same_column = self
            .cards
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.column.as_str())
            == Some(column);
        if same_column {
            ids.retain(|c| c != id);
        }
        let at = index.min(ids.len());
        ids.insert(at, id.to_owned());
        let Some(card) = self.cards.iter_mut().find(|c| c.id == id) else {
            return false;
        };
        card.column = column.to_owned();
        for (order, cid) in ids.iter().enumerate() {
            if let Some(c) = self.cards.iter_mut().find(|c| &c.id == cid) {
                c.order = order;
            }
        }
        true
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
                doc.tasks.migrate_legacy();
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
/// add-card inputs, plus the card currently open in the modal editor.
#[derive(Debug, Default)]
pub struct BoardState {
    pub board: Board,
    pub new_text: HashMap<String, String>,
    pub open: Option<OpenCard>,
}

/// Draft being edited in the modal. Applied to the board on Save only.
#[derive(Debug, Clone)]
pub struct OpenCard {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub urgency: Urgency,
}

impl BoardState {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            new_text: HashMap::new(),
            open: None,
        }
    }
}

/// Drag payload: the card being moved.
#[derive(Debug, Clone)]
struct CardDrag(String);

/// Render the board. Returns true when the model mutated (caller saves).
pub fn show_board(ui: &mut egui::Ui, theme: &VscodeTheme, state: &mut BoardState) -> bool {
    let mut mutated = false;
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Task board").size(16.0).color(theme.text));
        ui.add_space(4.0);
        let total = state.board.cards.len();
        ui.label(egui::RichText::new(format!("{total} tasks")).size(12.0).color(theme.muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("Drag cards to move · click to edit").size(11.0).color(theme.faint));
        });
    });
    ui.add_space(10.0);

    // Rails stretch the full viewport height; each rail scrolls its own
    // cards (Trello layout). Measured before the scroll area (finite).
    let board_h = (ui.available_height() - 8.0).max(300.0);
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
                    mutated |= show_column(ui, theme, state, col_id, col_title, board_h);
                }
            });
        });
    mutated |= show_modal(ui, theme, state);
    mutated
}

fn show_column(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    state: &mut BoardState,
    col_id: &str,
    col_title: &str,
    board_h: f32,
) -> bool {
    let mut mutated = false;
    let rail_accent = match col_id {
        "progress" => theme.accent,
        "done" => egui::Color32::from_rgb(0x4C, 0xAF, 0x50),
        _ => theme.muted,
    };
    let mut card_rects = Vec::new();
    let rail = egui::Frame::default()
        .fill(egui::Color32::from_rgb(0x12, 0x12, 0x12))
        .stroke(egui::Stroke::new(1.0, theme.input_border))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
          ui.vertical(|ui| {
            ui.set_width(264.0);
            ui.set_min_height(board_h - 14.0);
            ui.horizontal(|ui| {
                ui.painter().circle_filled(ui.cursor().min + egui::vec2(4.0, 9.0), 3.0, rail_accent);
                ui.add_space(8.0);
                ui.label(egui::RichText::new(col_title).size(13.0).strong().color(theme.text));
                let n = state.board.column_cards(col_id).len();
                egui::Frame::new().fill(theme.input_bg).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("{n}")).size(11.0).color(theme.muted));
                });
            });
            ui.add_space(8.0);

            // Cards scroll inside the rail; the add-row stays pinned below.
            let scroll_h = (board_h - 120.0).max(200.0);
            egui::ScrollArea::vertical()
                .id_salt(("tasks-col", col_id.to_owned()))
                .auto_shrink([false, false])
                .max_height(scroll_h)
                .show(ui, |ui| {
                    let ids: Vec<String> = state
                        .board
                        .column_cards(col_id)
                        .iter()
                        .map(|c| c.id.clone())
                        .collect();
                    for id in &ids {
                        let rect = show_card(ui, theme, state, id);
                        card_rects.push((id.clone(), rect));
                    }
                    if ids.is_empty() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("No tasks yet").size(12.0).color(theme.muted));
                    }

                });

            // Add-card row: button or Enter commits non-empty drafts.
            ui.horizontal(|ui| {
                let draft = state.new_text.entry(col_id.to_string()).or_default();
                let resp = ui.add_sized(
                    egui::vec2((ui.available_width() - 48.0).max(80.0), 28.0),
                    egui::TextEdit::singleline(draft)
                        .hint_text("Add a card")
                        .frame(true),
                );
                let enter = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (icons::primary_button(ui, theme, "+").clicked() || enter)
                    && !draft.trim().is_empty()
                {
                    state.board.add_card(col_id, draft.trim().to_owned());
                    state.new_text.insert(col_id.to_string(), String::new());
                    mutated = true;
                }
            });
          });
        });
    if let (Some(payload), Some(pointer)) = (
        egui::DragAndDrop::payload::<CardDrag>(ui.ctx()),
        ui.ctx().pointer_interact_pos(),
    ) {
        let rect = rail.response.rect.intersect(ui.clip_rect());
        if rect.contains(pointer) {
            let visible_cards = card_rects.iter().filter(|(id, _)| id != &payload.0);
            let index = visible_cards.filter(|(_, card)| pointer.y > card.center().y).count();
            let y = card_rects.iter().filter(|(id, _)| id != &payload.0)
                .find(|(_, card)| pointer.y <= card.center().y)
                .map(|(_, card)| card.top() - 2.0)
                .or_else(|| card_rects.iter().filter(|(id, _)| id != &payload.0).last().map(|(_, card)| card.bottom() + 2.0))
                .unwrap_or(rect.top() + 42.0)
                .clamp(rect.top() + 2.0, rect.bottom() - 2.0);
            ui.painter().line_segment([egui::pos2(rect.left() + 10.0, y), egui::pos2(rect.right() - 10.0, y)], egui::Stroke::new(2.0, theme.accent));
            if ui.input(|i| i.pointer.any_released()) {
                mutated |= state.board.move_card_to(&payload.0, col_id, index);
                egui::DragAndDrop::take_payload::<CardDrag>(ui.ctx());
            }
        }
    }
    mutated
}

/// Card description preview: first lines, clamped. Pure, unit-tested.
pub fn preview_text(desc: &str) -> String {
    const MAX_LINES: usize = 3;
    const MAX_CHARS: usize = 140;
    let mut out = String::new();
    for (i, line) in desc.lines().take(MAX_LINES + 1).enumerate() {
        if i == MAX_LINES {
            out.push('…');
            break;
        }
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
    }
    if out.chars().count() > MAX_CHARS {
        out = out.chars().take(MAX_CHARS).collect::<String>() + "…";
    }
    out
}

fn show_card(ui: &mut egui::Ui, theme: &VscodeTheme, state: &mut BoardState, id: &str) -> egui::Rect {
    let Some(card) = state.board.cards.iter().find(|c| c.id == id).cloned() else {
        return egui::Rect::NOTHING;
    };
    let width = ui.available_width();
    let frame = egui::Frame::new()
            .fill(theme.input_bg)
            .stroke(egui::Stroke::new(1.0, theme.input_border))
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_min_width((width - 22.0).max(0.0));
                ui.set_max_width((width - 22.0).max(0.0));
                ui.label(egui::RichText::new(if card.title.is_empty() { "(untitled)" } else { &card.title }).size(13.0).color(theme.text));
                if !card.desc.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(preview_text(&card.desc)).size(12.0).color(theme.muted));
                }
                ui.add_space(8.0);
                egui::Frame::new().fill(card.urgency.color().linear_multiply(0.12))
                    .corner_radius(egui::CornerRadius::same(3))
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| { ui.label(egui::RichText::new(card.urgency.label()).size(11.0).color(card.urgency.color())); });
            });
    let response = ui.interact(frame.response.rect, egui::Id::new(("task-drag", id)), egui::Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::Grab)
        .on_hover_text("Drag to move · click to edit");
    if response.dragged() {
        egui::DragAndDrop::set_payload(ui.ctx(), CardDrag(id.to_owned()));
    }
    if response.clicked() {
        state.open = Some(OpenCard { id: card.id, title: card.title, desc: card.desc, urgency: card.urgency });
    }
    response.rect
}

/// Card editor modal (the only place a card changes). Draft lives in
/// `state.open`; the board updates on Save, never while typing.
fn show_modal(ui: &mut egui::Ui, theme: &VscodeTheme, state: &mut BoardState) -> bool {
    if state.open.is_none() {
        return false;
    }
    // Dim backdrop so the modal reads as modal.
    ui.ctx()
        .layer_painter(egui::LayerId::background())
        .rect_filled(
            ui.ctx().screen_rect(),
            0.0,
            egui::Color32::from_black_alpha(140),
        );
    let column_title = state.open.as_ref()
        .and_then(|draft| state.board.cards.iter().find(|card| card.id == draft.id))
        .and_then(|card| state.board.columns.iter().find(|column| column.id == card.column))
        .map(|column| column.title.clone()).unwrap_or_default();
    let mut action: Option<&str> = None;
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        action = Some("cancel");
    }
    egui::Window::new("Card details")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .min_size(egui::vec2(420.0, 0.0))
        .show(ui.ctx(), |ui| {
            let Some(draft) = state.open.as_mut() else {
                return;
            };
            ui.label(egui::RichText::new(format!("In {column_title}")).size(12.0).color(theme.muted));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Title").size(12.0).color(theme.muted));
            ui.add_sized(
                egui::vec2(ui.available_width(), 24.0),
                egui::TextEdit::singleline(&mut draft.title).hint_text("Card title"),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Description")
                    .size(11.0)
                    .color(theme.muted),
            );
            ui.add_sized(
                egui::vec2(ui.available_width(), 120.0),
                egui::TextEdit::multiline(&mut draft.desc)
                    .hint_text("Details…")
                    .desired_rows(6),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Priority")
                    .size(11.0)
                    .color(theme.muted),
            );
            ui.horizontal(|ui| {
                for level in [Urgency::Low, Urgency::Medium, Urgency::High] {
                    ui.selectable_value(
                        &mut draft.urgency,
                        level,
                        egui::RichText::new(format!("● {}", level.label()))
                            .size(12.0)
                            .color(level.color()),
                    );
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("Delete").color(theme.danger))
                    .clicked()
                {
                    action = Some("delete");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::primary_button(ui, theme, "Save").clicked() {
                        action = Some("save");
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some("cancel");
                    }
                });
            });
        });
    match action {
        Some("save") => {
            if let Some(d) = state.open.clone() {
                state.board.rename_card(&d.id, d.title);
                state.board.set_desc(&d.id, d.desc);
                state.board.set_urgency(&d.id, d.urgency);
            }
            state.open = None;
            true
        }
        Some("delete") => {
            if let Some(d) = state.open.clone() {
                state.board.delete_card(&d.id);
            }
            state.open = None;
            true
        }
        _ => {
            if action.is_some() {
                state.open = None;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> Board {
        let mut b = Board::empty();
        b.add_card("plan", "first".into());
        b.add_card("plan", "second".into());
        b.add_card("progress", "wip".into());
        b
    }

    #[test]
    fn crud_and_reorder() {
        let mut b = seeded();
        let ids: Vec<String> = b
            .column_cards("plan")
            .iter()
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(ids.len(), 2);

        // Drop second card above the first.
        assert!(b.move_card_to(&ids[1], "plan", 0));
        let top: Vec<String> = b
            .column_cards("plan")
            .iter()
            .map(|c| c.title.clone())
            .collect();
        assert_eq!(top, vec!["second".to_string(), "first".to_string()]);

        // Unknown card or column refuses.
        assert!(!b.move_card_to("nope", "plan", 0));
        assert!(!b.move_card_to(&ids[1], "nope", 0));

        // Across columns, lands at the requested index.
        assert!(b.move_card_to(&ids[1], "progress", 0));
        let prog: Vec<String> = b
            .column_cards("progress")
            .iter()
            .map(|c| c.title.clone())
            .collect();
        assert_eq!(prog, vec!["second".to_string(), "wip".to_string()]);
        assert_eq!(b.column_cards("plan").len(), 1);

        // Rename + desc + urgency + delete.
        assert!(b.rename_card(&ids[0], "renamed".into()));
        assert!(b.set_desc(&ids[0], "details".into()));
        assert!(b.set_urgency(&ids[0], Urgency::High));
        assert!(!b.set_urgency("nope", Urgency::High));
        let card = b.cards.iter().find(|c| c.id == ids[0]).unwrap();
        assert_eq!(card.urgency, Urgency::High);
        assert!(b.delete_card(&ids[0]));
        assert!(!b.delete_card("nope"));
        assert_eq!(b.column_cards("plan").len(), 0);
    }

    #[test]
    fn legacy_boards_migrate_to_plan_progress_done() {
        let legacy = serde_json::json!({
            "tasks": {
                "columns": [
                    {"id": "todo", "title": "To Do"},
                    {"id": "doing", "title": "Doing"},
                    {"id": "done", "title": "Done"}
                ],
                "cards": [
                    {"id": "c1", "column": "todo", "title": "a", "desc": "", "order": 0},
                    {"id": "c2", "column": "doing", "title": "b", "desc": "", "order": 0}
                ],
                "next_id": 3
            }
        });
        let (board, warning) = parse_tasks(Some(&legacy.to_string()));
        assert!(warning.is_none());
        let ids: Vec<&str> = board.columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["plan", "progress", "done"]);
        let titles: Vec<&str> = board.columns.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, vec!["Plan", "In Progress", "Done"]);
        assert_eq!(board.column_cards("plan").len(), 1);
        assert_eq!(board.column_cards("progress").len(), 1);
        // Urgency absent in the old file → Medium.
        assert!(board.cards.iter().all(|c| c.urgency == Urgency::Medium));
    }

    #[test]
    fn preview_clamps_lines_and_length() {
        assert_eq!(preview_text("one\ntwo"), "one\ntwo");
        assert_eq!(preview_text("a\nb\nc\nd"), "a\nb\nc…");
        let long = "x".repeat(200);
        assert!(preview_text(&long).chars().count() <= 141);
        assert_eq!(preview_text(""), "");
    }

    #[test]
    fn urgency_cycles_low_medium_high() {
        assert_eq!(Urgency::Low.next(), Urgency::Medium);
        assert_eq!(Urgency::Medium.next(), Urgency::High);
        assert_eq!(Urgency::High.next(), Urgency::Low);
        assert_eq!(Urgency::default(), Urgency::Medium);
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
