# ArctisCode

Lightweight native Rust code editor — `egui` + embedded Neovim. No Electron / WebView / Tauri.

## Layout

- `src/` — all Rust code (`main.rs`, `app.rs`, `window.rs`, `theme.rs`, `tasks.rs`, `settings.rs`, `project/`, `explorer/`, `tabs/`, `editor/`, `preview/`, `safety.rs`)
- `public/` — static assets only (`icons/`, `fonts/`, `styles/tokens.json`)
- `tests/` — V1 Definition-of-Done checks
- `docs/` — architecture + perf notes
- `reference-ui/` — gitignored throwaway `ui init` output, never a dependency

## Look & feel

ArctisCode Black: pitch-black background, neutral dark-grey card surfaces,
no blue-tinted chrome (the blue status bar stays). Widget chrome
(scrollbars, secondary buttons) is neutral grey; only primary actions
(Open, Save, Create…) use blue, via an explicit button helper. The window is transparent:
the Explorer sidebar, the activity bar and the status bar are 80% black / 20% see-through
in any state (sharp, no blur; no row boxes — selection reads as white
vs grey text),
while editor and dialogs stay solid. No OS blur: vibrancy
blanks GL compositing on this stack, so transparency stays sharp.
Tokens live in `public/styles/tokens.json`, mapped to egui in
`src/theme.rs` (`src/icons.rs` holds the painter-drawn glyphs).

## Views (activity bar)

- **Explorer** — file tree + editor group (tabs, editor, image preview).
- **Tasks** — Trello-style board (To Do / Doing / Done) per workspace,
  stored in `<workspace>/.arctis.json` (plain JSON, auto-saved on every
  change; global `~/.arctis.json` fallback with no folder open). Cards:
  add/rename/describe/delete, move with `‹ › ↑ ↓`.
- **Settings** — editor + interface font sizes (live), confirm-before-close
  toggle (off auto-saves instead of asking), workspace paths, shortcuts,
  about. Stored in global `~/.arctis.json`.

## Run

```sh
cargo run -- ./some/project
# or open the GUI, then Cmd/Ctrl+O or drop a folder in
```

## Multiple windows (multi-repo)

Press `Shift+Cmd/Ctrl+N` for a new OS window, then open another repo in it.
Each window has its own explorer, tabs, and watcher; the Neovim backend and
image cache are shared process-wide. Opening the same file in two windows
edits one shared buffer, and saving it in both meets the guarded-write
overwrite dialog. Closing a window with unsaved files offers
Save-all / Discard-all. Note: dragging tree rows *between* windows is not
supported — drop files from Finder, or open them in either window.

Requires `nvim` on PATH (falls back to native editing with a status warning if missing).
The embedded backend runs stock (`--noplugin -u NONE`) and never blocks the UI.
Set `ARCTIS_NO_NVIM=1` to force pure native editing.

> For daily use prefer `cargo run --release`: debug builds decode image
> previews ~10× slower (seconds for multi-megapixel photos vs ~100ms).

## Dev hot reload (phase 0)

```sh
cargo build
./target/debug/arctis-host -- ./some/project
# edit code → cargo build → restart host (live swap without restart is phase 1)
```

See `docs/HOTRELOAD.md`. Release `ArctisCode` stays statically linked.

## OpenCode skills

- `graphify` — project-local skill in `.opencode/skills/graphify/`
- `ponytail` — plugin via `opencode.json` (`@dietrichgebert/ponytail@4.10.0`, fetched on session start)

## Perf budgets (PRD V1 §7)

Track: startup, project-open, idle RSS, med-repo RSS, idle CPU, file-open latency. See `docs/PERF.md`.
