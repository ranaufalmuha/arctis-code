//! ArctisCode Black visual tokens: pitch-black background, neutral
//! dark-grey card surfaces, no blue-tinted chrome.
//!
//! This module maps those tokens to `egui::Visuals` + `egui::Style`.
//! No WebView / CSS dependency at runtime.

use egui::{Color32, CornerRadius, Stroke, Visuals};

/// VSCode layout metrics (px) used across the shell.
pub const ACTIVITY_BAR_WIDTH: f32 = 48.0;
pub const TAB_STRIP_HEIGHT: f32 = 35.0;
pub const STATUS_BAR_HEIGHT: f32 = 22.0;
pub const TREE_ROW_HEIGHT: f32 = 22.0;
pub const TREE_INDENT: f32 = 8.0;

#[derive(Debug, Clone)]
pub struct VscodeTheme {
    // Surfaces
    pub editor_bg: Color32,
    pub editor_fg: Color32,
    pub sidebar_bg: Color32,
    pub card_bg: Color32,
    pub activity_bg: Color32,
    pub tabstrip_bg: Color32,
    pub tab_active_bg: Color32,
    pub tab_inactive_bg: Color32,
    // Text
    pub text: Color32,
    pub tab_active_fg: Color32,
    pub tab_inactive_fg: Color32,
    pub muted: Color32,
    pub faint: Color32,
    // Gutter
    pub gutter_fg: Color32,
    pub gutter_active_fg: Color32,
    pub tree_guide: Color32,
    // Accent / selection
    pub accent: Color32,
    pub focus_border: Color32,
    pub selection: Color32,
    pub list_hover: Color32,
    pub list_active_sel: Color32,
    pub list_sel_fg: Color32,
    pub dirty_dot: Color32,
    // Inputs / buttons / widgets
    pub input_bg: Color32,
    pub input_border: Color32,
    pub button_bg: Color32,
    pub button_hover: Color32,
    pub button_fg: Color32,
    pub button_secondary_bg: Color32,
    pub button_secondary_hover: Color32,
    pub widget_bg: Color32,
    pub widget_border: Color32,
    // Status bar
    pub statusbar_fg: Color32,
    // Activity bar icons
    pub activity_fg: Color32,
    pub activity_inactive_fg: Color32,
    // Semantic
    pub link: Color32,
    pub danger: Color32,
    pub warning: Color32,
    // Type
    pub mono_size: f32,
    pub ui_size: f32,
    pub small_size: f32,
}

impl Default for VscodeTheme {
    fn default() -> Self {
        Self {
            editor_bg: Color32::from_rgb(0x00, 0x00, 0x00),
            editor_fg: Color32::from_rgb(0xD4, 0xD4, 0xD4),
            // Sidebar paints are translucent: the transparent window shows
            // the desktop behind them (sharp, no OS blur).
            sidebar_bg: Color32::from_rgba_unmultiplied(0x10, 0x10, 0x10, 0),
            // Opaque card surface for in-window cards (task columns, weak
            // fills) that must stay readable over any background.
            card_bg: Color32::from_rgb(0x10, 0x10, 0x10),
            activity_bg: Color32::from_rgb(0x00, 0x00, 0x00),
            tabstrip_bg: Color32::from_rgb(0x10, 0x10, 0x10),
            tab_active_bg: Color32::from_rgb(0x00, 0x00, 0x00),
            tab_inactive_bg: Color32::from_rgb(0x16, 0x16, 0x16),

            text: Color32::from_rgb(0xCC, 0xCC, 0xCC),
            tab_active_fg: Color32::WHITE,
            tab_inactive_fg: Color32::from_rgb(0x96, 0x96, 0x96),
            muted: Color32::from_rgb(0x96, 0x96, 0x96),
            faint: Color32::from_rgb(0x6E, 0x6E, 0x6E),

            gutter_fg: Color32::from_rgb(0x85, 0x85, 0x85),
            gutter_active_fg: Color32::from_rgb(0xC6, 0xC6, 0xC6),
            tree_guide: Color32::from_rgb(0x33, 0x33, 0x33),

            accent: Color32::from_rgb(0x00, 0x7A, 0xCC),
            focus_border: Color32::from_rgb(0x00, 0x7F, 0xD4),
            selection: Color32::from_rgb(0x26, 0x4F, 0x78),
            list_hover: Color32::from_rgba_unmultiplied(0x1A, 0x1A, 0x1A, 204),
            list_active_sel: Color32::from_rgba_unmultiplied(0x2A, 0x2A, 0x2A, 230),
            list_sel_fg: Color32::WHITE,
            dirty_dot: Color32::from_rgb(0x33, 0x99, 0xCC),

            input_bg: Color32::from_rgb(0x16, 0x16, 0x16),
            input_border: Color32::from_rgb(0x2A, 0x2A, 0x2A),
            button_bg: Color32::from_rgb(0x0E, 0x63, 0x9C),
            button_hover: Color32::from_rgb(0x11, 0x77, 0xBB),
            button_fg: Color32::WHITE,
            button_secondary_bg: Color32::from_rgb(0x2D, 0x2D, 0x2D),
            button_secondary_hover: Color32::from_rgb(0x38, 0x38, 0x38),
            widget_bg: Color32::from_rgb(0x10, 0x10, 0x10),
            widget_border: Color32::from_rgb(0x2A, 0x2A, 0x2A),

            statusbar_fg: Color32::WHITE,

            activity_fg: Color32::WHITE,
            activity_inactive_fg: Color32::from_rgb(0x86, 0x86, 0x86),

            link: Color32::from_rgb(0x37, 0x94, 0xFF),
            danger: Color32::from_rgb(0xF1, 0x4C, 0x4C),
            warning: Color32::from_rgb(0xCC, 0xA7, 0x00),

            mono_size: 14.0,
            ui_size: 13.0,
            small_size: 12.0,
        }
    }
}

// Keep the old name working for any external users of the crate.
pub type ArctisTheme = VscodeTheme;

impl VscodeTheme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.dark_mode = true;
        visuals.override_text_color = Some(self.text);
        visuals.hyperlink_color = self.link;
        // Single-line inputs (find box, dialogs) get the VSCode input fill.
        // The code editor + gutter override this per-widget.
        visuals.text_edit_bg_color = Some(self.input_bg);

        // Dialogs / popups on the card surface.
        visuals.window_fill = self.widget_bg;
        visuals.window_stroke = Stroke::new(1.0, self.widget_border);
        visuals.window_corner_radius = CornerRadius::ZERO;
        visuals.menu_corner_radius = CornerRadius::ZERO;

        // Panels default to the editor surface; side strips paint their own
        // frames explicitly, so this mostly affects the central area.
        visuals.panel_fill = self.editor_bg;
        visuals.extreme_bg_color = self.editor_bg;
        visuals.faint_bg_color = self.card_bg;
        visuals.code_bg_color = self.editor_bg;

        // Text selection inside the editor.
        visuals.selection.bg_fill = self.selection;
        visuals.selection.stroke = Stroke::new(1.0, self.selection);

        // VSCode is (almost) square: sharp panels, 2px widgets.
        let widget_rounding = CornerRadius::same(2);
        visuals.widgets.noninteractive.bg_fill = self.card_bg;
        visuals.widgets.noninteractive.weak_bg_fill = self.card_bg;
        visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.noninteractive.corner_radius = widget_rounding;

        // Neutral widget chrome: scrollbars, checkboxes and secondary
        // buttons share these. Primary actions opt back into blue via
        // the `primary_button` helper (icons.rs), which scopes its own
        // fills instead of relying on the global widgets style.
        visuals.widgets.inactive.bg_fill = self.button_secondary_bg;
        visuals.widgets.inactive.weak_bg_fill = self.button_secondary_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.button_fg);
        visuals.widgets.inactive.corner_radius = widget_rounding;

        visuals.widgets.hovered.bg_fill = self.button_secondary_hover;
        visuals.widgets.hovered.weak_bg_fill = self.button_secondary_hover;
        visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.button_fg);
        visuals.widgets.hovered.corner_radius = widget_rounding;

        visuals.widgets.active.bg_fill = self.button_secondary_hover;
        visuals.widgets.active.weak_bg_fill = self.button_secondary_hover;
        visuals.widgets.active.bg_stroke = Stroke::NONE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.button_fg);
        visuals.widgets.active.corner_radius = widget_rounding;

        visuals.widgets.open.bg_fill = self.input_bg;
        visuals.widgets.open.weak_bg_fill = self.input_bg;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, self.focus_border);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.open.corner_radius = widget_rounding;

        ctx.set_visuals(visuals);

        ctx.style_mut(|style| {
            use egui::{FontFamily, FontId, TextStyle};
            style.text_styles.insert(
                TextStyle::Body,
                FontId::new(self.ui_size, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Button,
                FontId::new(self.ui_size, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Small,
                FontId::new(self.small_size, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Heading,
                FontId::new(self.ui_size, FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(self.mono_size, FontFamily::Monospace),
            );
            // Tight VSCode-like density.
            style.spacing.item_spacing = egui::vec2(4.0, 4.0);
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            style.spacing.indent = 12.0;
        });
    }
}
