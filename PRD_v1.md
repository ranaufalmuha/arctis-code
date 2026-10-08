# PRD v1 — Lightweight Native Rust Code Editor

## 1. Overview

Build a lightweight, native desktop code editor written entirely in Rust.

The primary goal is not to replace VS Code or become a full IDE. The goal is to provide a fast, low-memory application for developers who mainly need to:

- Open and inspect repositories.
- Navigate project folders.
- Read and edit source code.
- Drag and drop files/folders/images from the native file manager.
- Keep heavy workflows such as terminals, AI agents, servers, and development commands in external terminal applications.

The editor should feel like a native desktop application rather than a web application.

## 2. Core Principles

### Native First

The application must be implemented using Rust and a native desktop GUI approach.

Do not use:

- Electron
- Chromium
- WebView
- Tauri
- Embedded browser runtimes
- JavaScript/TypeScript as the application UI layer

### Lightweight

Memory usage is a first-class requirement.

The application should avoid unnecessary background processes, indexing services, databases, telemetry systems, and other persistent services.

### Neovim-powered Editing

Neovim should be used as the underlying editing engine where practical.

The native Rust application is responsible for the graphical desktop experience, filesystem interaction, project navigation, and drag-and-drop functionality.

## 3. Target Workflow

The intended workflow is:

```text
Native Code Editor
├── Repository / Folder Tree
├── File Tabs
├── Code Editing
├── File Preview
└── Drag & Drop

External Terminal
├── Ghostty
├── OpenCode / Hermes
├── Development servers
├── Git
└── Other CLI tools
```

The editor does not need to provide a terminal in V1.

## 4. V1 Scope

### 4.1 Application Window

The application must provide:

- Native desktop window.
- Application title.
- Standard close/minimize/maximize behavior where supported.
- Resizable window.
- Native keyboard input.
- Native clipboard integration.

### 4.2 Project Opening

Users must be able to:

- Open a folder as a project.
- Open a single file.
- Drag a folder from the native file manager into the application.
- Drag a file from the native file manager into the application.

When a folder is opened, it becomes the current project root.

Example:

```text
~/Projects/live2dev
```

The application should display:

```text
live2dev/
├── apps/
├── packages/
├── api/
├── public/
├── package.json
└── turbo.json
```

### 4.3 File Tree Explorer

The editor must provide a filesystem tree.

Required functionality:

- Expand folder.
- Collapse folder.
- Open file.
- Single selection.
- Basic keyboard navigation.
- Refresh directory.
- Detect filesystem changes where practical.
- Display common file/folder icons or indicators.
- Support nested directories.

The tree should not load the entire repository into memory unnecessarily.

Large directories should be loaded lazily.

### 4.4 Code Editing

The editor must support:

- Opening text files.
- Editing text files.
- Saving files.
- Save As where practical.
- Undo.
- Redo.
- Cursor movement.
- Text selection.
- Copy/paste.
- Find.
- Basic syntax highlighting.
- Multiple open files/tabs.

Neovim should provide the editing functionality where practical.

The Rust GUI should communicate with Neovim through an appropriate RPC mechanism.

### 4.5 Tabs

Users should be able to open multiple files.

Example:

```text
[ users.ts ] [ auth.ts ] [ index.ts ]
```

Required:

- Open tab.
- Close tab.
- Switch tab.
- Modified-file indicator.
- Prevent accidental loss of unsaved changes.
- Basic tab keyboard navigation.

### 4.6 Drag & Drop

Native drag-and-drop is a high-priority V1 feature.

Supported sources:

- Native file manager.
- Desktop.
- Multiple selected files.
- Multiple selected folders.

Supported objects:

- Files.
- Folders.
- Images.
- Text files.
- Source files.
- Configuration files.
- Other filesystem objects where appropriate.

Expected behavior:

#### Dropping a file

```text
Finder
  ↓
foo.ts
  ↓
Editor
  ↓
Open foo.ts
```

#### Dropping a folder

```text
Finder
  ↓
my-project/
  ↓
Editor
  ↓
Open as project
```

#### Dropping multiple files

```text
foo.ts
bar.ts
image.png
   ↓
Editor
   ↓
Open supported files appropriately
```

### 4.7 Image Handling

Images must not be treated as text files.

Supported behavior:

- Open image file.
- Display native image preview.
- Support common image formats.
- Avoid loading unnecessarily large images into memory where possible.

At minimum, support:

- PNG
- JPEG/JPG
- GIF
- WebP
- SVG where practical

### 4.8 File Safety

The application must not silently overwrite data.

Requirements:

- Detect external file modifications.
- Warn before overwriting externally modified files.
- Warn before closing unsaved files.
- Handle deleted/moved files gracefully.
- Never modify project files merely by opening a project.

## 5. Non-Goals

The following are explicitly out of scope for V1:

- Integrated terminal.
- AI assistant.
- AI agent.
- Git GUI.
- Debugger.
- Extension marketplace.
- Built-in package manager.
- Database tools.
- Docker UI.
- Kubernetes UI.
- Project indexing service.
- Built-in browser.
- Cloud synchronization.
- User accounts.
- Telemetry.

## 6. Architecture

Conceptually:

```text
┌──────────────────────────────────────┐
│          Native Rust Application     │
│                                      │
│  Window                              │
│  ├── File Explorer                   │
│  ├── Tabs                            │
│  ├── Drag & Drop                     │
│  ├── Image Preview                   │
│  └── Editor Container                │
│                 │                    │
│                 ▼                    │
│          Neovim Process              │
│                 │                    │
│                 ▼                    │
│          Project Filesystem          │
└──────────────────────────────────────┘
```

The architecture should keep GUI responsibilities separate from editing-engine responsibilities.

## 7. Performance Requirements

Performance is a primary product requirement.

The application should:

- Start quickly.
- Open small repositories quickly.
- Avoid scanning the entire repository unnecessarily.
- Lazy-load directories.
- Avoid unnecessary indexing.
- Avoid background CPU-heavy tasks.
- Avoid embedded browser runtimes.
- Avoid persistent helper processes except the required Neovim process.

Performance should be measured rather than assumed.

Track at minimum:

- Startup time.
- Project opening time.
- Memory usage at idle.
- Memory usage with a medium repository.
- CPU usage at idle.
- File opening latency.

## 8. V1 Definition of Done

V1 is complete when a developer can:

1. Launch the application.
2. Drag a project folder from the native file manager.
3. See the project folder tree.
4. Navigate directories.
5. Open source files.
6. Edit files using Neovim-powered editing.
7. Open multiple files in tabs.
8. Save changes.
9. Drag individual files into the editor.
10. Drag images into the editor and view them.
11. Close the project without losing unsaved work.
12. Continue using Ghostty independently for terminal/AI workflows.

The application should feel significantly lighter and simpler than a traditional full IDE.

## 9. Product Philosophy

V1 should resist feature creep.

The question for every proposed feature is:

> Does this improve lightweight repository navigation and code editing?

If not, defer it.
