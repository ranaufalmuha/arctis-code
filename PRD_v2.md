# PRD v2 — Lightweight Native Rust Code Editor

## 1. Overview

V2 extends the lightweight native Rust code editor with developer intelligence while preserving the core philosophy established in V1.

The application remains:

- Native Rust.
- No WebView.
- No Electron.
- No Tauri.
- Lightweight.
- Neovim-powered.
- Focused on code editing rather than becoming a heavyweight IDE.

V2 introduces three major capabilities:

1. Language intelligence and extensions.
2. Intelligent/AI-assisted coding.
3. Integrated terminal.

## 2. V2 Priorities

Priority order:

```text
P0 — Preserve V1 stability and performance

P1 — Language intelligence / extensions

P2 — Intelligent coding / AI

P3 — Integrated terminal
```

New features must not significantly compromise V1's lightweight architecture.

## 3. P1 — Language Intelligence

The editor should provide intelligent development features through established language tooling rather than implementing language intelligence itself.

### 3.1 LSP

Support Language Server Protocol integration.

Examples:

```text
TypeScript → TypeScript language server
Rust       → rust-analyzer
Go         → gopls
Python     → Pyright
CSS        → CSS language server
HTML       → HTML language server
Tailwind   → Tailwind CSS language server
```

The system should allow users to configure language servers.

### 3.2 Core Intelligence

Support:

- Autocomplete.
- Diagnostics.
- Go to definition.
- Go to declaration.
- Find references.
- Hover information.
- Rename symbol.
- Code actions.
- Formatting where supported.
- Signature help.

### 3.3 Tailwind CSS

Tailwind CSS support is a high-priority example.

Expected functionality:

```text
class="flex items-center justify-center"
                 ↓
autocomplete
                 ↓
hover information
                 ↓
diagnostics
```

The editor should rely on the Tailwind language server rather than implementing Tailwind intelligence internally.

### 3.4 Extension Philosophy

V2 should not immediately create a proprietary extension marketplace.

Prefer interoperability with existing standards:

- LSP
- DAP where appropriate
- External language servers
- Existing Neovim capabilities

A future extension system may be introduced only if the existing ecosystem cannot satisfy the product requirements.

## 4. P2 — Intelligent Coding

V2 introduces optional AI-assisted development.

AI must remain optional.

The editor must remain fully usable without an AI service.

### 4.1 AI Actions

Initial functionality:

- Explain selected code.
- Fix selected code.
- Refactor selected code.
- Generate code.
- Generate tests.
- Improve code.
- Explain compiler/LSP errors.

Example:

```text
User selects code
       ↓
AI action
       ↓
"Fix this"
       ↓
AI generates diff
       ↓
User reviews
       ↓
Accept / Reject
```

### 4.2 Diff-Based Editing

AI should not silently overwrite files.

AI-generated modifications should be represented as a reviewable diff.

Example:

```diff
- const user = await db.find(id)
+ const user = await db.findById(id)
```

User controls:

```text
Accept
Reject
```

### 4.3 Context

AI should receive only the context necessary for the requested task.

Potential context sources:

- Current file.
- Selected code.
- Open files.
- Related files.
- Project structure.
- LSP diagnostics.
- Explicitly selected project files.

Avoid indexing every repository by default.

### 4.4 External Agent Integration

The architecture should allow integration with external coding agents.

Possible future integrations:

```text
Editor
   ↓
AI interface
   ↓
OpenCode / other agent
```

The editor should not require a permanently running AI daemon.

## 5. P3 — Integrated Terminal

V2 introduces an optional terminal panel.

The terminal should be implemented natively and remain lightweight.

Example:

```text
┌─────────────────────────────────────────────┐
│ Editor                                      │
│                                             │
│                                             │
├─────────────────────────────────────────────┤
│ Terminal                                    │
│ $ pnpm dev                                  │
│ $ git status                                │
└─────────────────────────────────────────────┘
```

### 5.1 Terminal Requirements

Support:

- Shell process.
- Interactive input.
- Output rendering.
- Multiple terminal sessions.
- Working directory.
- Environment variables.
- Resize handling.
- Copy/paste.
- Basic keyboard shortcuts.

The terminal should default to the project's root directory.

### 5.2 External Terminal Remains Supported

The integrated terminal is additive.

Users should still be able to use:

- Ghostty.
- Other terminal applications.
- External AI agents.

The editor should never require the integrated terminal for normal operation.

## 6. V2 Architecture

```text
┌──────────────────────────────────────────────────┐
│                 Native Rust App                  │
│                                                  │
│  ┌─────────────┐ ┌────────────┐ ┌─────────────┐ │
│  │ File Tree   │ │ Editor     │ │ Tabs        │ │
│  └─────────────┘ └─────┬──────┘ └─────────────┘ │
│                        │                         │
│                   Neovim RPC                    │
│                        │                         │
│  ┌─────────────────────┼─────────────────────┐  │
│  │                     │                     │  │
│  ▼                     ▼                     ▼  │
│ LSP                  AI Layer             Terminal│
│  │                     │                     │  │
│ Language             External               Shell │
│ Servers              Agents                       │
└──────────────────────────────────────────────────┘
```

The system should remain modular.

A user who disables AI should not pay a significant memory or CPU cost for AI infrastructure.

A user who does not use the terminal should not have a persistent terminal process.

## 7. Performance Requirements

V2 must preserve the lightweight behavior of V1.

New functionality must follow these principles:

### Lazy Initialization

Only start services when required.

For example:

```text
Open Rust file
    ↓
Start rust-analyzer if necessary
```

Not:

```text
Application startup
    ↓
Start every available language server
```

### Optional AI

Do not run an AI model locally by default.

AI should communicate with external services or external agents when enabled.

### Optional Terminal

Terminal processes should exist only while terminal sessions are active.

### Repository Indexing

Do not introduce mandatory full-repository indexing.

If indexing is later required for AI functionality, it must be:

- Optional.
- Lazy.
- Interruptible.
- Resource constrained.

## 8. V2 Non-Goals

Still out of scope unless separately justified:

- Full VS Code-compatible extension marketplace.
- Built-in browser.
- Docker management.
- Kubernetes management.
- Database management.
- Cloud IDE functionality.
- Mandatory AI.
- Mandatory repository indexing.
- Built-in Git hosting.
- Heavy project management features.

## 9. V2 Definition of Done

V2 is complete when a developer can:

1. Open a repository through native drag-and-drop.
2. Navigate its filesystem tree.
3. Edit code using Neovim.
4. Use multiple tabs.
5. Drag files, folders, and images into the editor.
6. Receive LSP autocomplete and diagnostics.
7. Use Tailwind CSS intelligence.
8. Navigate definitions and references.
9. Ask AI to modify selected code.
10. Review and accept/reject AI changes.
11. Open an integrated terminal when needed.
12. Continue using external terminal applications when preferred.
13. Use all V1 functionality without AI or terminal enabled.
14. Experience no major performance regression compared with V1.

## 10. Product Philosophy

V2 should not become a feature-heavy IDE.

The core principle remains:

> A fast native Rust application for navigating, understanding, and editing code.

Every feature should justify its CPU, RAM, startup-time, and architectural cost.

The application should prefer:

> Existing standards + native Rust + Neovim

over:

> Reimplementing existing developer infrastructure.

## 11. Future Direction

Potential future capabilities, explicitly not committed to V2:

- Debug Adapter Protocol.
- Git interface.
- Advanced AI agent workflows.
- Codebase-aware AI search.
- Workspace management.
- Plugin system.
- Cross-platform native implementations.
- Project-specific automation.

These should only be added after measuring whether they provide enough value to justify additional complexity and resource usage.
