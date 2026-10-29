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
}

impl ArctisCodeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<String>) -> Self {
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
        } = self;

        // Settings drive the type scale; theme.apply below picks it up.
        theme.mono_size = settings.editor_font_size;
        theme.ui_size = settings.ui_font_size;

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

        // Keep theme applied (cheap; ensures reload consistency)
        theme.apply(ctx);
    }
}
