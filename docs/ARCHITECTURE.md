# Architecture

`ArctisCode` binary (`src/main.rs` → `app::ArctisCodeApp`).

Multi-window model: the shell (`app.rs`) owns one `window::RepoWindow` per
OS viewport (`windows[0]` in the root viewport, the rest in immediate
viewports) plus the shared services (`VscodeTheme`, `NeovimBackend`,
`ImageCache`, global `Settings`). Spawning is `Shift+Cmd/Ctrl+N`; closing
a dirty window cancels the viewport close and raises a Save-all /
Discard-all guard. Cross-window state stays consistent because sharing is
path-keyed.

Each window shows one center view (`window::CenterView`): `Editor` (tab
strip + editor), `Tasks` (Trello-style board from `tasks.rs`, persisted
per workspace in `<root>/.arctis.json`), or `Settings` (global prefs from
`settings.rs`: font sizes, confirm guards). The activity bar switches
views; there is no custom title bar (native OS chrome only).

- `project/` — root state, `opener::classify_dropped` (folder→project, file→tab), `watcher::ProjectWatcher` (notify, debounced).
- `explorer/tree.rs` — lazy `walkdir(max_depth=1)` per expanded dir, dirs-first sort, skips preloading `.git/node_modules/target`. Rows render through `window::clear_drop_zone` (stock `dnd_drop_zone` forces an opaque frame fill, so the fills are scoped transparent; scrollbar fills restored inside).
- `tabs/` — `TabManager` (open/close/switch, dirty dot), `bar::show` (custom-drawn tab strip).
- `tasks.rs` — task board model, `.arctis.json` workspace storage, board UI.
- `settings.rs` — global settings model, `~/.arctis.json` storage, settings page.
- `icons.rs` — painter-drawn VSCode-style glyphs (activity bar, explorer actions).
- `editor/nvim.rs` — spawns `nvim --embed --headless` via `nvim-rs` + tokio; `sync_buffer` / `read_buffer`. egui `TextEdit` is the surface in V1; grid rendering plugs in here.
- `preview/image.rs` — `image` crate + `resvg`/`tiny-skia` for SVG, `thumbnail(2048)` cap, `ImageCache` textures, `fit_contain` shrink-to-fit preview sizing.
- `safety.rs` — mtime+len snapshots, guarded write, overwrite dialog.
- `theme.rs` — ArctisCode Black tokens from `public/styles/tokens.json` → `egui::Visuals`. Explorer, activity bar and status bar are 80% opaque / 20% see-through (opaque `card_bg` covers task columns and weak fills); sharp transparency, no OS blur.
- `icons.rs` — painter-drawn glyphs, frameless icon buttons, and the blue
  `primary_button` helper (global widget chrome stays neutral grey).
- `v2_stub.rs` — `LspClient` / `AiProvider` / `TerminalSession` seams, no cost in V1.

rust-ui (`ui-cli`) is used only to generate `reference-ui/` for token reference. Runtime has zero web deps.
