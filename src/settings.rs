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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor_font_size: 14.0,
            ui_font_size: 13.0,
            confirm_guards: true,
        }
    }
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
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("SETTINGS")
                .size(11.0)
                .color(theme.muted),
        );
    });
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("settings-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    ui.set_min_size(egui::vec2(420.0, 0.0));

                    ui.heading("Appearance");
                    ui.horizontal(|ui| {
                        ui.label("Editor font size");
                        if ui
                            .add(
                                egui::DragValue::new(&mut settings.editor_font_size)
                                    .range(10.0..=24.0)
                                    .speed(0.2),
                            )
                            .changed()
                        {
                            mutated = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Interface font size");
                        if ui
                            .add(
                                egui::DragValue::new(&mut settings.ui_font_size)
                                    .range(10.0..=20.0)
                                    .speed(0.2),
                            )
                            .changed()
                        {
                            mutated = true;
                        }
                    });
                    ui.add_space(12.0);

                    ui.heading("Behavior");
                    if ui
                        .checkbox(
                            &mut settings.confirm_guards,
                            "Confirm before closing unsaved work",
                        )
                        .changed()
                    {
                        mutated = true;
                    }
                    ui.label(
                        egui::RichText::new(
                            "Off: closing saves automatically — never discards silently.",
                        )
                        .size(12.0)
                        .color(theme.muted),
                    );
                    ui.add_space(12.0);

                    ui.heading("Workspace");
                    ui.label(format!(
                        "Folder: {}",
                        root.map(|r| r.display().to_string())
                            .unwrap_or_else(|| "none open".to_owned())
                    ));
                    ui.label(format!("Tasks file: {}", tasks_path.display()));
                    ui.add_space(12.0);

                    ui.heading("Shortcuts");
                    for (keys, what) in [
                        ("⌘O", "Open folder"),
                        ("⌘S", "Save"),
                        ("⌘W", "Close tab"),
                        ("⌘B", "Toggle sidebar"),
                        ("⇧⌘N", "New window"),
                    ] {
                        ui.label(
                            egui::RichText::new(format!("{keys}  {what}"))
                                .size(12.0)
                                .color(theme.muted),
                        );
                    }
                    ui.add_space(12.0);

                    ui.heading("About");
                    ui.label(
                        egui::RichText::new(format!(
                            "ArctisCode {} — native Rust editor (egui + Neovim).",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .size(12.0)
                        .color(theme.muted),
                    );
                });
            });
        });

    if mutated {
        settings.editor_font_size = settings.editor_font_size.clamp(10.0, 24.0);
        settings.ui_font_size = settings.ui_font_size.clamp(10.0, 20.0);
    }
    mutated
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
        };
        let text = serde_json::to_string(&SettingsDoc { settings: s.clone() }).unwrap();
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
        let (s, _) = parse_settings(Some(r#"{"settings":{"editor_font_size":99.0,"ui_font_size":1.0,"confirm_guards":true}}"#));
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
