//! ArctisCode app shell: N repo windows (one per OS viewport) plus the
//! services shared across windows (theme, embedded Neovim, image cache).
//!
//! - `windows[0]` renders in the root viewport.
//! - `windows[1..]` each get an immediate viewport (`Shift+Cmd/Ctrl+N`).
//! - Sharing is path-keyed, so the same file in two windows edits one
//!   Neovim buffer and concurrent saves meet the guarded-write dialog.

use crate::editor::NeovimBackend;
use crate::preview::ImageCache;
use crate::settings::{self, Settings};
use crate::theme::VscodeTheme;
use crate::window::{fresh_viewport_id, RepoWindow, WindowAction};

pub struct ArctisCodeApp {
    theme: VscodeTheme,
    nvim: NeovimBackend,
    images: ImageCache,
    settings: Settings,
    windows: Vec<RepoWindow>,
    next_viewport: u64,
    applied_mono: f32,
    applied_ui: f32,
    recent: Vec<std::path::PathBuf>,
    observed_roots: std::collections::HashMap<egui::ViewportId, std::path::PathBuf>,
    dock_initialized: bool,
}

impl ArctisCodeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<String>) -> Self {
        crate::recent::install_context(&cc.egui_ctx);
        let theme = VscodeTheme::default();
        theme.apply(&cc.egui_ctx);

        let (settings, settings_warning) = settings::load_settings();
        if let Some(w) = settings_warning {
            eprintln!("ArctisCode: {w}");
        }

        Self {
            theme,
            nvim: NeovimBackend::spawn(),
            images: ImageCache::default(),
            settings,
            windows: vec![RepoWindow::with_initial(
                egui::ViewportId::ROOT,
                initial_path,
            )],
            next_viewport: 1,
            applied_mono: 0.0,
            applied_ui: 0.0,
            recent: crate::recent::load(),
            observed_roots: Default::default(),
            dock_initialized: false,
        }
    }
}

impl eframe::App for ArctisCodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            theme,
            nvim,
            images,
            settings,
            windows,
            next_viewport,
            applied_mono,
            applied_ui,
            recent,
            observed_roots,
            dock_initialized,
        } = self;

        // Settings drive the type scale; re-apply only when sizes changed
        // (apply() rebuilds fonts, so every-frame calls waste CPU).
        if settings.editor_font_size != *applied_mono || settings.ui_font_size != *applied_ui {
            theme.mono_size = settings.editor_font_size;
            theme.ui_size = settings.ui_font_size;
            theme.apply(ctx);
            *applied_mono = settings.editor_font_size;
            *applied_ui = settings.ui_font_size;
        }

        if !*dock_initialized {
            crate::recent::update(recent);
            *dock_initialized = true;
        }
        if let Some(path) = crate::recent::take() {
            if path.is_dir() {
                let id = fresh_viewport_id(*next_viewport);
                *next_viewport += 1;
                windows.push(RepoWindow::with_initial(id, Some(path.to_string_lossy().into_owned())));
            } else if let Some(first) = windows.first_mut() {
                first.status = format!("Project not found: {}", path.display());
            }
        }
        let mut actions: Vec<WindowAction> = Vec::new();

        // Root viewport always shows the first window.
        if let Some(first) = windows.first_mut() {
            actions.extend(first.render_window(ctx, theme, nvim, images, settings));
        }

        // One immediate OS viewport per extra window.
        let mut idx = 1;
        while idx < windows.len() {
            let title = windows[idx].title_text();
            let id = windows[idx].viewport;
            let more: Vec<WindowAction> = ctx.show_viewport_immediate(
                id,
                egui::ViewportBuilder::default()
                    .with_icon(crate::app_icon::icon().unwrap_or_default())
                    .with_inner_size([1280.0, 800.0])
                    .with_min_inner_size([800.0, 500.0])
                    .with_title(title)
                    // Transparent window, sharp (see main.rs).
                    .with_transparent(true)
                    .with_drag_and_drop(true)
                    .with_fullsize_content_view(true)
                    .with_titlebar_shown(false)
                    .with_title_shown(false),
                |vctx, _| windows[idx].render_window(vctx, theme, nvim, images, settings),
            );
            actions.extend(more);
            idx += 1;
        }

        let mut changed = false;
        for window in windows.iter() {
            if let Some(root) = &window.project.root {
                if observed_roots.get(&window.viewport) != Some(root) {
                    observed_roots.insert(window.viewport, root.clone());
                    changed |= crate::recent::record(recent, root);
                }
            } else { observed_roots.remove(&window.viewport); }
        }
        observed_roots.retain(|id, _| windows.iter().any(|w| w.viewport == *id));
        if changed {
            if let Err(e) = crate::recent::save(recent) { eprintln!("ArctisCode: recent projects: {e:#}"); }
            crate::recent::update(recent);
        }

        // Apply shell-level requests.
        for action in actions {
            match action {
                WindowAction::NewWindow => {
                    let id = fresh_viewport_id(*next_viewport);
                    *next_viewport += 1;
                    windows.push(RepoWindow::blank(id));
                }
                WindowAction::CloseWindow(id) => {
                    if id == egui::ViewportId::ROOT {
                        // Root viewport closing quits the app; the window
                        // already passed its dirty guard to get here.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        windows.retain(|w| w.viewport != id);
                    }
                }
            }
        }
    }
}
