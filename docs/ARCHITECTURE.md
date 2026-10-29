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

`window/` is split by responsibility, all sharing one `RepoWindow`:
- `window/mod.rs` — state, shell (`render_window`), dialogs, title text.
- `window/io.rs` — open/save/move/drop, guarded writes.
- `window/tree_view.rs` — explorer tree, filter search (memoized, skips
  heavy dirs like `.git`/`node_modules`).
- `window/editor_view.rs` — editor + image preview (gutter/find/cursor
  results cached per frame).
- `window/chrome.rs` — activity bar, titlebar.

- `project/` — root state, `opener::classify_dropped` (folder→project, file→tab), `watcher::ProjectWatcher` (notify, debounced).
- `explorer/tree.rs` — lazy `walkdir(max_depth=1)` per expanded dir, dirs-first sort, skips preloading `.git/node_modules/target`. Rows render through `window::clear_drop_zone` (stock `dnd_drop_zone` forces an opaque frame fill, so the fills are scoped transparent; scrollbar fills restored inside).
- `tabs/` — `TabManager` (open/close/switch, dirty dot), `bar::show` (custom-drawn tab strip).
- `tasks.rs` — task board model, `.arctis.json` workspace storage, board UI.
- `settings.rs` — global settings model, `~/.arctis.json` storage, settings page.
- `icons.rs` — painter-drawn VSCode-style glyphs (activity bar, explorer actions).
- `editor/nvim.rs` — spawns `nvim --embed --headless` via `nvim-rs` + tokio; `sync_buffer` / `read_buffer`. egui `TextEdit` is the surface in V1; grid rendering plugs in here.
- `editor/highlight.rs` — dependency-free scanner (keywords, strings,
  comments, numbers, constants + depth-colored brackets, red mismatch
  for rust/js/ts/tsx/json/python/toml),
  per-tab cache, 512KB cap with plain fallback. Tree-sitter was tried
  and removed: its C parsers make Apple's linker emit unloadable
  binaries on this toolchain, and cost ~12MB against the tiny-binary goal.
- `editor/autoedit.rs` — bracket pairing (skip-over only its own pair,
  `'` never pairs, selection wrap) + smart Enter, all settings-gated.
- `editor/format.rs` — format-on-save via external tools only (rustfmt/prettier/gofmt, stdin→stdout, 512KB cap, never blocks the save).
- `preview/image.rs` — `image` crate + `resvg`/`tiny-skia` for SVG, `thumbnail(2048)` cap, `ImageCache` textures, `fit_contain` shrink-to-fit preview sizing. Decode runs on a background thread (UI shows a spinner meanwhile); GPU upload stays on the UI thread.
- `safety.rs` — mtime+len snapshots, guarded write, overwrite dialog.
- `theme.rs` — ArctisCode Black tokens from `public/styles/tokens.json` → `egui::Visuals`. Explorer, activity bar and status bar are 80% opaque / 20% see-through (opaque `card_bg` covers task columns and weak fills); sharp transparency, no OS blur.
- `icons.rs` — painter-drawn glyphs, frameless icon buttons, and the blue
  `primary_button` helper (global widget chrome stays neutral grey).
- `v2_stub.rs` — `LspClient` / `AiProvider` / `TerminalSession` seams, no cost in V1.

rust-ui (`ui-cli`) is used only to generate `reference-ui/` for token reference. Runtime has zero web deps.
