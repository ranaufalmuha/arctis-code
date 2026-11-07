# ArctisCode Knowledge Graph Report

## Overview
- Total nodes: 694
- Total edges: 1407
- Total communities: 35
- Hyperedges: 3

## Communities

### Core (Community 15)
- Nodes: 82
- Edge connections: ~228
- Key nodes: NeovimBackend, .render_window(), RepoWindow, .show_tree(), .show_editor(), RepoWindow, .handle_tree_row(), .show_dir()

### Core (Community 35)
- Nodes: 74
- Edge connections: ~88
- Key nodes: Architecture Overview, ArctisCode, PRD v1, Performance Report, Performance, PRD v2, RepoWindow, Image Preview

### Core (Community 18)
- Nodes: 64
- Edge connections: ~90
- Key nodes: FileIconKind, FolderKind, icons.rs, draw_tree_icon(), draw_glyph(), TreeIcon, draw_file_icon(), draw_folder_icon()

### Core (Community 24)
- Nodes: 60
- Edge connections: ~124
- Key nodes: RepoWindow, mod.rs, tree_view.rs, EntryKind, create_entry(), chrome.rs, app.rs, theme.rs

### Core (Community 29)
- Nodes: 53
- Edge connections: ~126
- Key nodes: tasks.rs, Board, show_column(), show_modal(), Urgency, BoardState, show_card(), show_board()

### Core (Community 26)
- Nodes: 52
- Edge connections: ~87
- Key nodes: TabManager, TabKind, Tab, v1_dod.rs, FileSnapshot, open.rs, mod.rs, DropAction

### Core (Community 13)
- Nodes: 38
- Edge connections: ~75
- Key nodes: highlight.rs, highlight_spans(), Lang, Span, HlCache, kinds(), span_color(), language_for()

### Core (Community 25)
- Nodes: 34
- Edge connections: ~86
- Key nodes: import.rs, move_into(), import.rs, copy_into(), setup(), CopyOutcome, plan_operations(), io.rs

### Core (Community 22)
- Nodes: 33
- Edge connections: ~69
- Key nodes: ImageCache, image.rs, fit_contain(), decode_image(), approx(), into_rgba8_fast(), load_raster(), .texture_for()

### Core (Community 28)
- Nodes: 33
- Edge connections: ~91
- Key nodes: VscodeTheme, settings.rs, Settings, show_settings(), parse_settings(), bar.rs, show_row(), load_settings()

### Core (Community 9)
- Nodes: 28
- Edge connections: ~45
- Key nodes: recent.rs, ArctisCodeApp, .update(), .new(), update(), icon(), record(), history_path()

### Core (Community 19)
- Nodes: 19
- Edge connections: ~29
- Key nodes: Glyph, search_field(), .activity_icon(), .show_activity_bar(), Ui, RepoWindow, Response, Explorer

### Core (Community 17)
- Nodes: 18
- Edge connections: ~29
- Key nodes: FileTree, FileNode, tree.rs, .list_dir(), .children_ref(), is_skipped_dir(), .children_of(), mod.rs

### Core (Community 11)
- Nodes: 17
- Edge connections: ~31
- Key nodes: autoedit.rs, plan(), pair_for(), is_closer(), pairs_brackets(), matching_opener(), char_to_byte(), inside_string()

### Core (Community 27)
- Nodes: 11
- Edge connections: ~17
- Key nodes: ProjectWatcher, FsEvent, watcher.rs, Changed, RecommendedWatcher, Receiver, .new(), .watch()

### Core (Community 10)
- Nodes: 10
- Edge connections: ~11
- Key nodes: arctis-host.rs, load_app(), dylib_path(), err(), EXPECTED_ABI, AbiVersionFn, CreateAppFn, CreationContext

### Core (Community 30)
- Nodes: 10
- Edge connections: ~9
- Key nodes: v2_stub.rs, LspClient, AiProvider, TerminalSession, CompletionProvider, .filetypes(), .start_if_needed(), .propose_edit()

### Editor (Community 12)
- Nodes: 8
- Edge connections: ~12
- Key nodes: format.rs, format_content(), formatter_for(), FormatCmd, picks_canonical_tools(), oversize_skips_silently(), rustfmt_roundtrip_when_installed(), MAX_FORMAT_BYTES

### Core (Community 16)
- Nodes: 8
- Edge connections: ~8
- Key nodes: lib.rs, arctis_create_app(), arctis_destroy_app(), .drop(), App, HOT_ABI_VERSION, arctis_abi_version(), CreationContext

### Core (Community 34)
- Nodes: 7
- Edge connections: ~7
- Key nodes: Hot Reload, arctis-host, arctis_abi_version Symbol, arctis-code Library, Hot Reload Phase 0, Hot Reload Phase 1, Hot Reload Phase 2

### Core (Community 32)
- Nodes: 6
- Edge connections: ~7
- Key nodes: nvim.rs, nvim_available(), sync_never_blocks_caller(), sync_then_read_roundtrip(), spawn_returns_promptly(), killswitch_disables_backend()

### Core (Community 5)
- Nodes: 5
- Edge connections: ~4
- Key nodes: ArctisDockActions, dock.m, Runtime, NSObject, -openRecent

### Core (Community 33)
- Nodes: 4
- Edge connections: ~5
- Key nodes: tokens.rs, hex(), Color32, tokens_match_runtime_theme()

### Core (Community 4)
- Nodes: 3
- Edge connections: ~2
- Key nodes: opencode.json, $schema, plugin

### Architecture (Community 31)
- Nodes: 3
- Edge connections: ~3
- Key nodes: hotreload_abi.rs, dylib_path(), dylib_exposes_expected_abi()

### Core (Community 2)
- Nodes: 2
- Edge connections: ~1
- Key nodes: build.rs, main()

### Core (Community 3)
- Nodes: 2
- Edge connections: ~1
- Key nodes: generate_icon.rs, main()

### Core (Community 6)
- Nodes: 2
- Edge connections: ~1
- Key nodes: build-macos.sh, build-macos.sh script

### Core (Community 21)
- Nodes: 2
- Edge connections: ~1
- Key nodes: main.rs, main()

### Core (Community 1)
- Nodes: 1
- Edge connections: ~0
- Key nodes: arctis-code

### Core (Community 7)
- Nodes: 1
- Edge connections: ~0
- Key nodes: Logo

### Core (Community 8)
- Nodes: 1
- Edge connections: ~0
- Key nodes: Cargo.toml

### Core (Community 14)
- Nodes: 1
- Edge connections: ~0
- Key nodes: Build Script

### Architecture (Community 20)
- Nodes: 1
- Edge connections: ~0
- Key nodes: ABI Version

### Core (Community 23)
- Nodes: 1
- Edge connections: ~0
- Key nodes: dnd-drop-zone

## Hyperedges

- **Editor Components**: form (5 nodes)
  - Nodes: editor_nvim, editor_highlight_scanner, editor_autoedit, editor_format, preview_image
  - Confidence: INFERRED (0.85)
- **Window View System**: implement (4 nodes)
  - Nodes: window_editor_view_editor, window_tree_view_explorer, tasks_task_board, settings_global_prefs
  - Confidence: INFERRED (0.85)
- **File Operations Flow**: participate_in (4 nodes)
  - Nodes: project_opener_classify, window_io_file_operations, safety_guarded_write, project_watcher
  - Confidence: INFERRED (0.75)

## Key Relationships

- Most connected nodes:
  - VscodeTheme (`src_theme_vscodetheme`): 44 connections
  - NeovimBackend (`src_editor_nvim_neovimbackend`): 42 connections
  - FileIconKind (`src_icons_fileiconkind`): 40 connections
  - Architecture Overview (`arctis_code_architecture`): 33 connections
  - tasks.rs (`src_tasks`): 29 connections
  - .render_window() (`src_window_mod_repowindow_render_window`): 29 connections
  - highlight.rs (`src_editor_highlight`): 27 connections
  - RepoWindow (`src_window_mod_repowindow`): 27 connections
  - ImageCache (`src_preview_image_imagecache`): 26 connections
  - mod.rs (`src_window_mod`): 26 connections