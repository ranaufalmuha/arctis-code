//! VSCode-style editor tab strip, custom-drawn on the painter.
//!
//! - 35px strip on the tab-group background (`tabstrip_bg`)
//! - active tab takes the editor background with white text
//! - inactive tabs are `tab_inactive_bg` with muted text
//! - close `×` always visible on the active tab, on hover otherwise
//! - dirty tabs show a dot instead of `×` until hovered

use super::TabManager;
use crate::theme::{VscodeTheme, TAB_STRIP_HEIGHT};

pub fn show(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    manager: &mut TabManager,
    mut close_request: impl FnMut(usize) -> bool,
) {
    // Strip background spans the full width.
    let strip = egui::Frame::default()
        .fill(theme.tabstrip_bg)
        .inner_margin(egui::Margin::same(0))
        .outer_margin(egui::Margin::same(0))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), TAB_STRIP_HEIGHT));
            show_row(ui, theme, manager, &mut close_request);
        });

    // Hairline between the strip and the editor below.
    ui.painter().hline(
        strip.response.rect.x_range(),
        strip.response.rect.bottom(),
        egui::Stroke::new(1.0, theme.editor_bg),
    );
}

/// Tab row without its own outer frame: for embedding inside the custom
/// titlebar (`TopBottomPanel::top`), which already paints the strip
/// background. Takes the remaining horizontal width.
pub fn show_row(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    manager: &mut TabManager,
    close_request: &mut impl FnMut(usize) -> bool,
) {
    egui::ScrollArea::horizontal()
        // Explicit salt: sibling Frame/scope children share the
        // default "child" Ui id, so default-salted scroll areas
        // would collide ("Double use of ScrollArea ID").
        .id_salt("tab-strip")
        .max_width(ui.available_width())
        .max_height(TAB_STRIP_HEIGHT)
        // Fill the strip width, but shrink vertically to the 35px
        // tabs — [false, false] here would swallow the whole
        // central panel height and leave the editor with nothing.
        .auto_shrink([false, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                let mut switch_to: Option<usize> = None;
                let mut close_idx: Option<usize> = None;

                for (idx, tab) in manager.tabs.iter().enumerate() {
                    let title = tab.title();
                    if let Some(action) =
                        show_tab(ui, theme, &title, tab.dirty, Some(idx) == manager.active)
                    {
                        match action {
                            TabAction::Switch => switch_to = Some(idx),
                            TabAction::Close => {
                                if close_request(idx) {
                                    close_idx = Some(idx);
                                }
                            }
                        }
                    }
                }

                if let Some(i) = switch_to {
                    manager.active = Some(i);
                }
                if let Some(i) = close_idx {
                    manager.close(i);
                }
            });
        });
}

enum TabAction {
    Switch,
    Close,
}

/// One tab. Returns the action taken this frame, if any.
fn show_tab(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    title: &str,
    dirty: bool,
    active: bool,
) -> Option<TabAction> {
    const PAD_LEFT: f32 = 10.0;
    const CLOSE_BOX: f32 = 26.0;

    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let text_w = ui
        .fonts(|f| f.layout_no_wrap(title.to_owned(), font_id.clone(), theme.text))
        .size()
        .x;
    let tab_w = (PAD_LEFT + text_w + CLOSE_BOX + 4.0).clamp(72.0, 220.0);

    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(tab_w, TAB_STRIP_HEIGHT), egui::Sense::click());
    let hovered = resp.hovered();

    let bg = if active {
        theme.tab_active_bg
    } else if hovered {
        theme.tab_inactive_bg.gamma_multiply(1.2)
    } else {
        theme.tab_inactive_bg
    };
    ui.painter().rect_filled(rect, 0.0, bg);

    // Right-edge divider between inactive tabs.
    if !active {
        ui.painter().vline(
            rect.right(),
            rect.y_range(),
            egui::Stroke::new(1.0, theme.tabstrip_bg),
        );
    }

    // Title.
    let fg = if active || hovered {
        theme.tab_active_fg
    } else {
        theme.tab_inactive_fg
    };
    ui.painter().with_clip_rect(rect).text(
        egui::pos2(rect.min.x + PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        font_id,
        fg,
    );

    // Dirty dot vs close affordance on the right.
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - CLOSE_BOX + 2.0, rect.center().y - 9.0),
        egui::vec2(CLOSE_BOX - 4.0, 18.0),
    );
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let close_hovered = hovered && pointer.map(|pos| close_rect.contains(pos)).unwrap_or(false);

    if dirty && !close_hovered {
        // Modified dot (VSCode `tab.activeModifiedBorder`).
        let dot_color = if active {
            theme.dirty_dot
        } else {
            theme.dirty_dot.gamma_multiply(0.55)
        };
        ui.painter()
            .circle_filled(close_rect.center(), 4.0, dot_color);
    } else if active || hovered {
        if close_hovered {
            ui.painter()
                .rect_filled(close_rect, 3.0, theme.list_hover.gamma_multiply(1.6));
        }
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::TextStyle::Body.resolve(ui.style()),
            fg,
        );
    }

    if resp.clicked() {
        if close_hovered {
            return Some(TabAction::Close);
        }
        return Some(TabAction::Switch);
    }
    None
}
