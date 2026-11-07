# ArctisCode

Lightweight native Rust code editor — `egui` + embedded Neovim. No Electron / WebView / Tauri.

## Features

- **Native Performance** — Built with Rust and egui for minimal memory footprint
- **Neovim Integration** — Embedded Neovim backend with fallback to native editing
- **Task Board** — Trello-style task management per workspace
- **Multi-window Support** — Work with multiple repositories simultaneously
- **Transparent UI** — Modern transparent sidebar with VSCode-like design

## Installation

### Prerequisites

- **Rust** 1.70 or later — [Install Rust](https://rustup.rs/)
- **Neovim** (optional) — For enhanced editing experience
- **macOS** (primary platform) — Linux/Windows support experimental

### Build from Source

```bash
# Clone the repository
git clone https://github.com/ranaufalmuha/arctis-code.git
cd arctis-code

# Build in release mode (recommended)
cargo build --release

# Run the editor
./target/release/arctis-code

# Or open a specific project
./target/release/arctis-code -- ./my-project
```

### macOS App Bundle

For macOS users, you can build a native `.app` bundle:

```bash
# Build the app bundle
./scripts/build-macos.sh

# The app will be available at:
# target/release/ArctisCode.app
```

You can then drag it to your Applications folder.

### Development Build

```bash
# Build in debug mode (faster compile, slower runtime)
cargo build

# Run with debug logging
RUST_LOG=debug cargo run -- ./my-project
```

## Quick Start

1. **Open a Project**
   - Press `Cmd/Ctrl+O` to open a folder
   - Or drag and drop a folder into the window
   - Or run `arctis-code -- /path/to/project`

2. **Navigate Files**
   - Use the Explorer sidebar (Cmd/Ctrl+B to toggle)
   - Click files to open in editor tabs

3. **Task Board**
   - Switch to Tasks view in the activity bar
   - Create cards with title and priority
   - Drag cards between columns (Plan → In Progress → Done)

4. **Settings**
   - Adjust font sizes, enable confirm-before-close
   - Settings auto-save to `~/.arctis.json`

## Layout

- `src/` — all Rust code (`main.rs`, `app.rs`, `window/` split by view/io/chrome, `theme.rs`, `tasks.rs`, `settings.rs`, `project/`, `explorer/`, `tabs/`, `editor/`, `preview/`, `safety.rs`)
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
- **Tasks** — Trello-style board (Plan / In Progress / Done) per workspace,
  stored in `<workspace>/.arctis.json` (plain JSON, auto-saved on every
  change; global `~/.arctis.json` fallback with no folder open). Cards
  show title, description preview, and priority chip — drag the whole
  card between rails and positions; click a card to edit it in a modal
  (title, description, priority, delete).
- **Settings** — editor + interface font sizes (live), confirm-before-close
  toggle (off auto-saves instead of asking), workspace paths, shortcuts,
  about. Stored in global `~/.arctis.json`.

## Run

```sh
cargo run -- ./some/project
# or open the GUI, then Cmd/Ctrl+O or drop a folder in
```

### Environment Variables

```bash
# Disable Neovim integration (use native editor only)
ARCTIS_NO_NVIM=1 cargo run -- ./my-project

# Enable debug logging
RUST_LOG=debug cargo run

# Build release binary with optimizations
cargo build --release
./target/release/arctis-code -- ./my-project
```

> **Tip:** For daily use, prefer `cargo run --release`: debug builds decode image
> previews ~10× slower (seconds for multi-megapixel photos vs ~100ms).

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

## Testing

```bash
# Run all tests
cargo test --lib

# Run tests for specific module
cargo test --lib tasks::

# Run tests with logging
RUST_LOG=debug cargo test --lib -- --nocapture
```

## Contributing

This repository uses branch protection on `main` and `test` branches:

- All changes require a **Pull Request**
- PRs must be **approved** before merging
- Tests must pass before merge
- Force push and deletions are disabled

### Workflow

1. Create a feature branch from `dev`:
   ```bash
   git checkout -b feat/your-feature
   ```

2. Make your changes and test:
   ```bash
   cargo test --lib
   cargo build --release
   ```

3. Commit and push:
   ```bash
   git commit -m "feat: describe your changes"
   git push origin feat/your-feature
   ```

4. Open a Pull Request on GitHub
5. Wait for approval and merge

## OpenCode skills

- `graphify` — project-local skill in `.opencode/skills/graphify/`
- `ponytail` — plugin via `opencode.json` (`@dietrichgebert/ponytail@4.10.0`, fetched on session start)

## Perf budgets (PRD V1 §7)

Track: startup, project-open, idle RSS, med-repo RSS, idle CPU, file-open latency. See `docs/PERF.md`.

## macOS release bundle

On macOS, run `scripts/build-macos.sh` to build `target/release/ArctisCode.app`.
The script renders `public/icons/logo.svg` into the bundle icon and signs the
bundle ad hoc for local use. Public distribution requires an Apple Developer ID
signature and notarization; the script does not perform either step. The build
matches the architecture of the machine running it. Neovim remains an optional
external executable and must be available to the app on PATH.

The Dock menu lists up to ten recent project folders. Selecting one opens a new
window. History is stored locally in the OS application data directory under
`ArctisCode/recent-projects.json`; missing folders are reported without removing
or replacing the current workspace.
