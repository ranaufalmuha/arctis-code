# ArctisCode

Lightweight native Rust code editor — `egui` + embedded Neovim. No Electron / WebView / Tauri.

## Features

- **Native app** — Rust + `egui`, rendered directly through OpenGL; no
  Electron, WebView, or Tauri layer
- **Neovim integration** — embedded backend runs `nvim` headless on a
  background thread, with native editing as the fallback
- **Task board** — Trello-style board (Plan / In Progress / Done) stored per
  workspace in `.arctis.json`
- **Multi-window** — one OS window per repository, sharing the Neovim
  backend and image cache
- **Transparent chrome** — sidebar, activity bar, and status bar are 80%
  black / 20% see-through, with the editor kept opaque

Performance targets (startup, RSS, open latency) are tracked in
[`docs/PERF.md`](docs/PERF.md) — measure them, don't assume them.

## Installation

### Prerequisites

- **Rust** 1.88 or later (edition 2021) — [Install Rust](https://rustup.rs/)
- **Neovim** (optional) — for the embedded editing backend; without it the
  app falls back to native editing with a status-bar warning
- **A working OpenGL driver** — the app renders through `glow` (OpenGL), so
  it needs GPU/GL support on every platform; a headless box with no GL
  driver will not start

```bash
git clone https://github.com/ranaufalmuha/arctis-code.git
cd arctis-code
```

### Build per platform

The binary is named `ArctisCode` (see `[[bin]]` in `Cargo.toml`). The
`arctis-host` binary is a separate development-only entry point.

**macOS is the primary, fully supported platform.** Linux and Windows
type-check but have not been linked or run by the maintainers — treat them
as experimental, and expect to fix things yourself.

#### macOS — fully supported

Requires the Xcode Command Line Tools (`xcode-select --install`): the build
script `build.rs` compiles `packaging/macos/dock.m` with `clang` and links
`AppKit`, and the bundle script uses `sips`, `iconutil`, and `codesign`.

```bash
cargo build --release

# Run it, optionally opening a project
./target/release/ArctisCode
./target/release/ArctisCode ./my-project
```

This is the one platform where the full build has been verified end to end.
For a double-clickable, ad-hoc signed bundle, see
[macOS App Bundle](#macos-app-bundle).

#### Linux — experimental

```bash
cargo build --release
./target/release/ArctisCode ./my-project
```

Verified: `cargo check --target x86_64-unknown-linux-gnu` passes with 0
errors. **Not verified:** linking and running, so the runtime library list
below is the best available reading of the dependency graph rather than a
tested recipe.

- Windowing comes from winit with both backends compiled in (`x11` and
  `wayland`, per its default features). Both are **dlopen'd at runtime**, so
  you need a running X11 or Wayland session plus the usual client libraries;
  a build does not link them, but startup fails without them.
- GL/EGL comes from `glutin`/`khronos-egl`, likewise loaded at runtime — a
  Mesa or vendor driver must be present.
- File dialogs use `rfd`'s `xdg-portal` backend, so a desktop portal service
  must be running (this is not a GTK3 build).
- Nothing here links OpenSSL: there is no `openssl`, `rustls`, or
  `native-tls` in `Cargo.lock`, so the `libssl-dev` line in egui's upstream
  instructions does not apply to this project.

#### Windows — experimental

```bash
cargo build --release
.\target\release\ArctisCode.exe .\my-project
```

Verified: `cargo check --target x86_64-pc-windows-msvc` passes with 0 errors.
**Not verified:** linking and running. Building for the MSVC target needs the
Visual Studio Build Tools (the linker), not just the Rust toolchain.

Two known gaps on Windows:

- The app has no `windows_subsystem` attribute, so a console window opens
  alongside the GUI window.
- The transparent window and custom titlebar in `src/main.rs`
  (`with_transparent`, `with_fullsize_content_view`, `with_titlebar_shown`,
  `with_title_shown`) are macOS-only egui options. They are ignored on other
  platforms, so expect the default system titlebar rather than the
  macOS-style chrome.

### macOS App Bundle

On macOS you can build a native `.app` bundle:

```bash
./scripts/build-macos.sh
# → target/release/ArctisCode.app
```

The script renders `public/icons/logo.svg` into the bundle icon and signs
the bundle **ad hoc** for local use. Public distribution requires an Apple
Developer ID signature and notarization, which the script does not perform.
The build matches the architecture of the machine running it. Drag the
result into `/Applications` to install it.

### Development Build

```bash
# Debug build (faster to compile, slower at runtime)
cargo build
cargo run -- ./my-project
```

## Quick Start

1. **Open a Project**
   - Press `Cmd/Ctrl+O` to open a folder
   - Or drag and drop a folder into the window
   - Or pass a path as the first argument: `ArctisCode ./my-project`
     (there is no flag parsing — the first argument *is* the path; a bare
     `--` would be read as the path itself)

2. **Navigate Files**
   - Use the Explorer sidebar (`Cmd/Ctrl+B` to toggle)
   - Click files to open them in editor tabs

3. **Task Board**
   - Switch to the Tasks view in the activity bar
   - Click **+ Add card**, enter a title, pick a priority, then press Enter
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
# Force pure native editing (skip the embedded Neovim backend)
ARCTIS_NO_NVIM=1 cargo run -- ./my-project
```

`ARCTIS_NO_NVIM` is the only environment variable the app reads. There is no
logging framework wired in, so `RUST_LOG` has no effect.

> **Tip:** For daily use, prefer `cargo run --release` (or the release
> binary): debug builds decode image previews roughly 10× slower — seconds
> for multi-megapixel photos versus about 100 ms.

Requires `nvim` on `PATH` (falls back to native editing with a status warning
if missing). The embedded backend runs stock (`--noplugin -u NONE`) and never
blocks the UI.

## Multiple windows (multi-repo)

Press `Shift+Cmd/Ctrl+N` for a new OS window, then open another repo in it.
Each window has its own explorer, tabs, and watcher; the Neovim backend and
image cache are shared process-wide. Opening the same file in two windows
edits one shared buffer, and saving it in both meets the guarded-write
overwrite dialog. Closing a window with unsaved files offers
Save-all / Discard-all. Note: dragging tree rows *between* windows is not
supported — drop files from Finder, or open them in either window.

## Dev hot reload (phase 0)

```sh
cargo build
./target/debug/arctis-host
# edit code → cargo build → restart host (live swap without restart is phase 1)
```

`arctis-host` loads `libarctis_code.{dylib,so,dll}` from its own directory and
runs the app from it instead of statically linked code. It takes **no command
line arguments** — open a project from inside the app. Phase 0 loads the
library once at startup; live swap without restart is phase 1. See
`docs/HOTRELOAD.md`. Release `ArctisCode` stays statically linked.

## Testing

```bash
# Library unit tests
cargo test --lib

# Unit tests for one module
cargo test --lib tasks::

# Integration tests (tests/ — entry, nvim, multiwindow, tokens, v1 DoD…)
cargo test --test v1_dod
cargo test --tests
```

## Contributing

Branch protection is enabled on `main` and `test`:

- All changes must go through a **Pull Request**
- PRs require **1 approving review** before merge
- Stale reviews are dismissed when new commits are pushed
- Force pushes and branch deletions are blocked

There is currently **no CI workflow** in this repository, so nothing is
checked automatically: run `cargo test --lib` and `cargo build --release`
locally before opening a PR.

### Workflow

1. Create a feature branch:
   ```bash
   git checkout -b feat/your-feature
   ```

2. Make your changes and verify locally:
   ```bash
   cargo test --lib
   cargo build --release
   ```

3. Commit and push:
   ```bash
   git commit -m "feat: describe your changes"
   git push origin feat/your-feature
   ```

4. Open a Pull Request, then wait for review and merge.

## OpenCode skills

- `graphify` — project-local skill in `.opencode/skills/graphify/`
- `ponytail` — plugin via `opencode.json` (`@dietrichgebert/ponytail@4.10.0`, fetched on session start)

## Perf budgets (PRD V1 §7)

Track: startup, project-open, idle RSS, med-repo RSS, idle CPU, file-open latency. See `docs/PERF.md`.

## macOS notes

Building and signing the `.app` bundle is covered under
[macOS App Bundle](#macos-app-bundle) above.

The Dock menu lists up to ten recent project folders; selecting one opens a new
window. History is stored locally in the OS application data directory under
`ArctisCode/recent-projects.json`; missing folders are reported without removing
or replacing the current workspace. Neovim remains an optional external
executable and must be available to the app on PATH.
