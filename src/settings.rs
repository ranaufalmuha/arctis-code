//! App settings: model, global `~/.arctis.json` storage, settings page.
//!
//! Settings are global (fonts aren't per-repo). Workspace task boards live
//! in `<root>/.arctis.json` — same filename, different scope. Every change
//! persists immediately; a corrupt file warns and falls back to defaults.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub editor_font_size: f32,
    pub ui_font_size: f32,
    /// Off = closing unsaved work auto-saves instead of asking (never
    /// discards silently).
    pub confirm_guards: bool,
    /// Run the external formatter (rustfmt/prettier/gofmt) on save.
    /// Defaults to true for settings files written before this flag existed.
    #[serde(default = "default_true")]
    pub format_on_save: bool,
    /// Type `(` and get `()`; Enter keeps indentation (and splits `{}`).
    #[serde(default = "default_true")]
    pub auto_close_brackets: bool,
    /// Enter copies the line indent (+1 level after `{`/`[`/`(`/`:`).
    #[serde(default = "default_true")]
    pub smart_enter: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor_font_size: 14.0,
            ui_font_size: 13.0,
            confirm_guards: true,
            format_on_save: true,
            auto_close_brackets: true,
            smart_enter: true,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Settings {
    /// Clamp hand-edited values into the ranges the page UI offers.
    pub fn clamped(mut self) -> Self {
        self.editor_font_size = self.editor_font_size.clamp(10.0, 24.0);
        self.ui_font_size = self.ui_font_size.clamp(10.0, 20.0);
        self
    }
}

/// On-disk global shape: `{ "settings": Settings }`.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SettingsDoc {
    #[serde(default)]
    pub settings: Settings,
}

pub fn settings_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".arctis.json"))
        .unwrap_or_else(|| PathBuf::from(".arctis.json"))
}

/// Parse global file content: corrupt input warns (for the status line)
/// and yields defaults; a missing file is silent (normal first run).
pub fn parse_settings(content: Option<&str>) -> (Settings, Option<String>) {
    match content {
        None => (Settings::default(), None),
        Some(text) => match serde_json::from_str::<SettingsDoc>(text) {
            Ok(doc) => (doc.settings.clamped(), None),
            Err(e) => (
                Settings::default(),
                Some(format!("settings unreadable ({e}); using defaults")),
            ),
        },
    }
}

pub fn load_settings() -> (Settings, Option<String>) {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_settings(Some(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => parse_settings(None),
        Err(e) => (
            Settings::default(),
            Some(format!("settings unreadable ({e}); using defaults")),
        ),
    }
}

pub fn save_settings(settings: &Settings) -> anyhow::Result<()> {
    let doc = SettingsDoc {
        settings: settings.clone(),
    };
    std::fs::write(settings_path(), serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

/// Render the settings page. Returns true when something changed (the
/// caller persists + the shell re-applies font sizes every frame).
pub fn show_settings(
    ui: &mut egui::Ui,
    theme: &crate::theme::VscodeTheme,
    settings: &mut Settings,
    root: Option<&Path>,
    tasks_path: &Path,
) -> bool {
    let mut mutated = false;
    egui::ScrollArea::vertical()
        .id_salt("settings-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 20)).show(ui, |ui| {
                ui.set_max_width(680.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Settings").size(20.0).strong().color(theme.text));
                    ui.label(egui::RichText::new("Customize your editor and workspace.").size(12.0).color(theme.muted));
                    ui.add_space(20.0);

                    settings_section(ui, theme, "Appearance");
                    setting_number(ui, theme, "Editor font size", "Size of text in the editor.", &mut settings.editor_font_size, 10.0..=24.0, &mut mutated);
                    setting_number(ui, theme, "Interface font size", "Size of text in the interface.", &mut settings.ui_font_size, 10.0..=20.0, &mut mutated);

                    ui.add_space(18.0);
                    settings_section(ui, theme, "Behavior");
                    setting_toggle(ui, theme, "Confirm before closing unsaved work", "When disabled, unsaved work is saved automatically.", &mut settings.confirm_guards, &mut mutated);
                    setting_toggle(ui, theme, "Format on save", "Format supported files when saving.", &mut settings.format_on_save, &mut mutated);
                    setting_toggle(ui, theme, "Auto-close brackets", "Automatically insert closing brackets.", &mut settings.auto_close_brackets, &mut mutated);
                    setting_toggle(ui, theme, "Smart Enter", "Keep indentation and indent after opening brackets.", &mut settings.smart_enter, &mut mutated);

                    ui.add_space(18.0);
                    settings_section(ui, theme, "Workspace");
                    info_row(ui, theme, "Folder", &root.map(|r| r.display().to_string()).unwrap_or_else(|| "No folder open".to_owned()));
                    info_row(ui, theme, "Tasks file", &tasks_path.display().to_string());

                    ui.add_space(18.0);
                    settings_section(ui, theme, "Shortcuts");
                    for (keys, what) in [("⌘O", "Open folder"), ("⌘S", "Save"), ("⌘W", "Close tab"), ("⌘B", "Toggle sidebar"), ("⇧⌘N", "New window")] {
                        info_row(ui, theme, what, keys);
                    }
                    ui.add_space(18.0);
                    settings_section(ui, theme, "About");
                    info_row(ui, theme, "ArctisCode", &format!("Version {}", env!("CARGO_PKG_VERSION")));
                });
            });
        });

    if mutated {
        settings.editor_font_size = settings.editor_font_size.clamp(10.0, 24.0);
        settings.ui_font_size = settings.ui_font_size.clamp(10.0, 20.0);
    }
    mutated
}

fn settings_section(ui: &mut egui::Ui, theme: &crate::theme::VscodeTheme, title: &str) {
    ui.label(egui::RichText::new(title).size(14.0).strong().color(theme.text));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

fn setting_number(ui: &mut egui::Ui, theme: &crate::theme::VscodeTheme, title: &str, description: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, changed: &mut bool) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).strong().color(theme.text));
    ui.label(egui::RichText::new(description).size(12.0).color(theme.muted));
    *changed |= ui.add(egui::DragValue::new(value).range(range).speed(0.2).suffix(" px")).changed();
    ui.add_space(6.0);
}

fn setting_toggle(ui: &mut egui::Ui, theme: &crate::theme::VscodeTheme, title: &str, description: &str, value: &mut bool, changed: &mut bool) {
    ui.add_space(6.0);
    *changed |= ui.checkbox(value, egui::RichText::new(title).strong().color(theme.text)).changed();
    ui.indent(("setting-description", title), |ui| {
        ui.label(egui::RichText::new(description).size(12.0).color(theme.muted));
    });
    ui.add_space(6.0);
}

fn info_row(ui: &mut egui::Ui, theme: &crate::theme::VscodeTheme, title: &str, value: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title).color(theme.text));
    ui.label(egui::RichText::new(value).size(12.0).color(theme.muted));
    ui.add_space(4.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_theme() {
        let s = Settings::default();
        assert_eq!(s.editor_font_size, 14.0);
        assert_eq!(s.ui_font_size, 13.0);
        assert!(s.confirm_guards);
    }

    #[test]
    fn serde_round_trip() {
        let s = Settings {
            editor_font_size: 16.0,
            ui_font_size: 12.0,
            confirm_guards: false,
            format_on_save: false,
            auto_close_brackets: false,
            smart_enter: false,
        };
        let text = serde_json::to_string(&SettingsDoc {
            settings: s.clone(),
        })
        .unwrap();
        let (back, warning) = parse_settings(Some(&text));
        assert!(warning.is_none());
        assert_eq!(back, s);
    }

    #[test]
    fn corrupt_starts_default_with_warning() {
        let (s, warning) = parse_settings(Some("[oops"));
        assert!(warning.is_some());
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn out_of_range_values_clamp() {
        let (s, _) = parse_settings(Some(
            r#"{"settings":{"editor_font_size":99.0,"ui_font_size":1.0,"confirm_guards":true}}"#,
        ));
        assert_eq!(s.editor_font_size, 24.0);
        assert_eq!(s.ui_font_size, 10.0);
    }

    #[test]
    fn settings_path_is_plain_json() {
        assert_eq!(
            settings_path().extension().and_then(|e| e.to_str()),
            Some("json")
        );
    }
}
