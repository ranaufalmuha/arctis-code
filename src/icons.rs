//! Painter-drawn VSCode-style glyphs (activity bar + explorer actions).
//!
//! egui's bundled fonts don't reliably cover symbol ranges, so the
//! chrome icons are drawn with the painter instead: crisp at any scale
//! and identical on every machine.

use crate::theme::VscodeTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Explorer,
    Search,
    SourceControl,
    RunDebug,
    Extensions,
    Gear,
    Kanban,
    NewFile,
    NewFolder,
    Refresh,
    CollapseAll,
}

/// Draw `glyph` centered in `rect` using strokes only (VSCode outline style).
pub fn draw_glyph(
    painter: &egui::Painter,
    glyph: Glyph,
    rect: egui::Rect,
    color: egui::Color32,
    stroke_w: f32,
) {
    let stroke = egui::Stroke::new(stroke_w, color);
    // 16x16 design box centered in the target rect.
    let s = 16.0_f32.min(rect.width()).min(rect.height());
    let org = egui::pos2(rect.center().x - s / 2.0, rect.center().y - s / 2.0);
    let p = |x: f32, y: f32| egui::pos2(org.x + x, org.y + y);
    let seg = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };

    match glyph {
        Glyph::Explorer => {
            // Document with folded corner + two text lines.
            seg((3.0, 1.5), (9.5, 1.5));
            seg((3.0, 1.5), (3.0, 14.5));
            seg((3.0, 14.5), (13.0, 14.5));
            seg((13.0, 14.5), (13.0, 5.0));
            seg((9.5, 1.5), (9.5, 5.0));
            seg((9.5, 5.0), (13.0, 5.0));
            seg((5.5, 8.0), (10.5, 8.0));
            seg((5.5, 11.0), (10.5, 11.0));
        }
        Glyph::Search => {
            painter.circle_stroke(p(7.0, 7.0), 4.5, stroke);
            seg((10.4, 10.4), (14.0, 14.0));
        }
        Glyph::SourceControl => {
            // Branch: two left nodes joined, merging into the right node.
            painter.circle_stroke(p(4.5, 3.5), 2.0, stroke);
            painter.circle_stroke(p(4.5, 12.5), 2.0, stroke);
            painter.circle_stroke(p(11.5, 8.0), 2.0, stroke);
            seg((4.5, 5.5), (4.5, 10.5));
            seg((6.3, 4.3), (9.7, 7.0));
            seg((6.3, 11.7), (9.7, 9.0));
        }
        Glyph::RunDebug => {
            // Play triangle.
            painter.line_segment([p(5.0, 3.0), p(12.0, 8.0)], stroke);
            painter.line_segment([p(12.0, 8.0), p(5.0, 13.0)], stroke);
            painter.line_segment([p(5.0, 13.0), p(5.0, 3.0)], stroke);
        }
        Glyph::Extensions => {
            // Four squares, bottom-right detached.
            for (x, y) in [(2.5, 2.5), (8.5, 2.5), (2.5, 8.5)] {
                let r = egui::Rect::from_min_size(p(x, y), egui::vec2(5.0, 5.0));
                for (a, b) in [
                    (r.left_top(), r.right_top()),
                    (r.right_top(), r.right_bottom()),
                    (r.right_bottom(), r.left_bottom()),
                    (r.left_bottom(), r.left_top()),
                ] {
                    painter.line_segment([a, b], stroke);
                }
            }
            let r = egui::Rect::from_min_size(p(9.0, 9.0), egui::vec2(5.0, 5.0));
            painter.rect_filled(r, 0.0, color);
        }
        Glyph::Gear => {
            painter.circle_stroke(p(8.0, 8.0), 3.0, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (s, c) = a.sin_cos();
                seg((8.0 + c * 4.5, 8.0 + s * 4.5), (8.0 + c * 6.5, 8.0 + s * 6.5));
            }
        }
        Glyph::Kanban => {
            // Board outline + two columns with cards.
            seg((2.0, 2.5), (14.0, 2.5));
            seg((2.0, 2.5), (2.0, 13.5));
            seg((2.0, 13.5), (14.0, 13.5));
            seg((14.0, 2.5), (14.0, 13.5));
            seg((6.0, 2.5), (6.0, 13.5));
            seg((10.0, 2.5), (10.0, 13.5));
            seg((3.0, 5.0), (5.0, 5.0));
            seg((3.0, 8.0), (5.0, 8.0));
            seg((7.0, 5.0), (9.0, 5.0));
            seg((11.0, 5.0), (13.0, 5.0));
            seg((11.0, 8.0), (13.0, 8.0));
            seg((11.0, 11.0), (13.0, 11.0));
        }
        Glyph::NewFile => {
            seg((4.0, 1.5), (4.0, 14.5));
            seg((4.0, 14.5), (10.0, 14.5));
            seg((10.0, 5.5), (10.0, 14.5));
            seg((4.0, 1.5), (7.5, 1.5));
            seg((7.5, 1.5), (7.5, 5.5));
            seg((7.5, 5.5), (10.0, 5.5));
            // Plus badge.
            seg((11.0, 9.5), (15.0, 9.5));
            seg((13.0, 7.5), (13.0, 11.5));
        }
        Glyph::NewFolder => {
            seg((1.5, 5.0), (1.5, 13.0));
            seg((1.5, 13.0), (10.5, 13.0));
            seg((10.5, 13.0), (10.5, 5.0));
            seg((10.5, 5.0), (7.0, 5.0));
            seg((7.0, 5.0), (6.0, 3.0));
            seg((6.0, 3.0), (1.5, 3.0));
            seg((1.5, 3.0), (1.5, 5.0));
            // Plus badge.
            seg((11.0, 9.5), (15.0, 9.5));
            seg((13.0, 7.5), (13.0, 11.5));
        }
        Glyph::Refresh => {
            // 270° arc approximated with segments + arrowhead.
            let c = p(8.0, 8.5);
            let r = 5.0;
            let steps = 12;
            let a0 = 0.6_f32;
            let a1 = 5.4_f32;
            let mut prev = egui::pos2(c.x + r * a0.cos(), c.y + r * a0.sin());
            for i in 1..=steps {
                let a = a0 + (a1 - a0) * i as f32 / steps as f32;
                let next = egui::pos2(c.x + r * a.cos(), c.y + r * a.sin());
                painter.line_segment([prev, next], stroke);
                prev = next;
            }
            seg((11.6, 1.6), (13.2, 5.4));
            seg((13.2, 5.4), (9.4, 5.8));
        }
        Glyph::CollapseAll => {
            // Double chevron up.
            seg((3.0, 9.5), (8.0, 5.0));
            seg((8.0, 5.0), (13.0, 9.5));
            seg((3.0, 13.5), (8.0, 9.0));
            seg((8.0, 9.0), (13.0, 13.5));
        }
    }
}

/// Frameless square icon button. Returns the click response.
pub fn icon_button(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    glyph: Glyph,
    tooltip: &str,
    size: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.list_hover);
    }
    let fg = if resp.hovered() {
        theme.activity_fg
    } else {
        theme.activity_inactive_fg
    };
    draw_glyph(ui.painter(), glyph, rect.shrink(size * 0.22), fg, 1.4);
    resp.on_hover_text(tooltip)
}

/// Primary action button (VSCode blue idle + lighter blue hover).
/// Global widget chrome is neutral grey (scrollbars, secondary buttons),
// so primaries opt back into blue with a scoped style override.
pub fn primary_button(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    label: &str,
) -> egui::Response {
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.bg_fill = theme.button_bg;
        visuals.widgets.inactive.weak_bg_fill = theme.button_bg;
        visuals.widgets.hovered.bg_fill = theme.button_hover;
        visuals.widgets.hovered.weak_bg_fill = theme.button_hover;
        visuals.widgets.active.bg_fill = theme.button_hover;
        visuals.widgets.active.weak_bg_fill = theme.button_hover;
        ui.button(label)
    })
    .inner
}
