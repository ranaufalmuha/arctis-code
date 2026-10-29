//! Painter-drawn VSCode-style glyphs (activity bar + explorer actions).
//!
//! egui's bundled fonts don't reliably cover symbol ranges, so the
//! chrome icons are drawn with the painter instead: crisp at any scale
//! and identical on every machine.

use crate::theme::VscodeTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Explorer,
    Search,
    SourceControl,
    RunDebug,
    Extensions,
    Gear,
    Kanban,
    NewFile,
    NewFolder,
    Refresh,
    CollapseAll,
}

/// Draw `glyph` centered in `rect` using strokes only (VSCode outline style).
pub fn draw_glyph(
    painter: &egui::Painter,
    glyph: Glyph,
    rect: egui::Rect,
    color: egui::Color32,
    stroke_w: f32,
) {
    let stroke = egui::Stroke::new(stroke_w, color);
    // 16x16 design box centered in the target rect.
    let s = 16.0_f32.min(rect.width()).min(rect.height());
    let org = egui::pos2(rect.center().x - s / 2.0, rect.center().y - s / 2.0);
    let p = |x: f32, y: f32| egui::pos2(org.x + x, org.y + y);
    let seg = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };

    match glyph {
        Glyph::Explorer => {
            // Document with folded corner + two text lines.
            seg((3.0, 1.5), (9.5, 1.5));
            seg((3.0, 1.5), (3.0, 14.5));
            seg((3.0, 14.5), (13.0, 14.5));
            seg((13.0, 14.5), (13.0, 5.0));
            seg((9.5, 1.5), (9.5, 5.0));
            seg((9.5, 5.0), (13.0, 5.0));
            seg((5.5, 8.0), (10.5, 8.0));
            seg((5.5, 11.0), (10.5, 11.0));
        }
        Glyph::Search => {
            painter.circle_stroke(p(7.0, 7.0), 4.5, stroke);
            seg((10.4, 10.4), (14.0, 14.0));
        }
        Glyph::SourceControl => {
            // Branch: two left nodes joined, merging into the right node.
            painter.circle_stroke(p(4.5, 3.5), 2.0, stroke);
            painter.circle_stroke(p(4.5, 12.5), 2.0, stroke);
            painter.circle_stroke(p(11.5, 8.0), 2.0, stroke);
            seg((4.5, 5.5), (4.5, 10.5));
            seg((6.3, 4.3), (9.7, 7.0));
            seg((6.3, 11.7), (9.7, 9.0));
        }
        Glyph::RunDebug => {
            // Play triangle.
            painter.line_segment([p(5.0, 3.0), p(12.0, 8.0)], stroke);
            painter.line_segment([p(12.0, 8.0), p(5.0, 13.0)], stroke);
            painter.line_segment([p(5.0, 13.0), p(5.0, 3.0)], stroke);
        }
        Glyph::Extensions => {
            // Four squares, bottom-right detached.
            for (x, y) in [(2.5, 2.5), (8.5, 2.5), (2.5, 8.5)] {
                let r = egui::Rect::from_min_size(p(x, y), egui::vec2(5.0, 5.0));
                for (a, b) in [
                    (r.left_top(), r.right_top()),
                    (r.right_top(), r.right_bottom()),
                    (r.right_bottom(), r.left_bottom()),
                    (r.left_bottom(), r.left_top()),
                ] {
                    painter.line_segment([a, b], stroke);
                }
            }
            let r = egui::Rect::from_min_size(p(9.0, 9.0), egui::vec2(5.0, 5.0));
            painter.rect_filled(r, 0.0, color);
        }
        Glyph::Gear => {
            painter.circle_stroke(p(8.0, 8.0), 3.0, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (s, c) = a.sin_cos();
                seg(
                    (8.0 + c * 4.5, 8.0 + s * 4.5),
                    (8.0 + c * 6.5, 8.0 + s * 6.5),
                );
            }
        }
        Glyph::Kanban => {
            // Board outline + two columns with cards.
            seg((2.0, 2.5), (14.0, 2.5));
            seg((2.0, 2.5), (2.0, 13.5));
            seg((2.0, 13.5), (14.0, 13.5));
            seg((14.0, 2.5), (14.0, 13.5));
            seg((6.0, 2.5), (6.0, 13.5));
            seg((10.0, 2.5), (10.0, 13.5));
            seg((3.0, 5.0), (5.0, 5.0));
            seg((3.0, 8.0), (5.0, 8.0));
            seg((7.0, 5.0), (9.0, 5.0));
            seg((11.0, 5.0), (13.0, 5.0));
            seg((11.0, 8.0), (13.0, 8.0));
            seg((11.0, 11.0), (13.0, 11.0));
        }
        Glyph::NewFile => {
            seg((4.0, 1.5), (4.0, 14.5));
            seg((4.0, 14.5), (10.0, 14.5));
            seg((10.0, 5.5), (10.0, 14.5));
            seg((4.0, 1.5), (7.5, 1.5));
            seg((7.5, 1.5), (7.5, 5.5));
            seg((7.5, 5.5), (10.0, 5.5));
            // Plus badge.
            seg((11.0, 9.5), (15.0, 9.5));
            seg((13.0, 7.5), (13.0, 11.5));
        }
        Glyph::NewFolder => {
            seg((1.5, 5.0), (1.5, 13.0));
            seg((1.5, 13.0), (10.5, 13.0));
            seg((10.5, 13.0), (10.5, 5.0));
            seg((10.5, 5.0), (7.0, 5.0));
            seg((7.0, 5.0), (6.0, 3.0));
            seg((6.0, 3.0), (1.5, 3.0));
            seg((1.5, 3.0), (1.5, 5.0));
            // Plus badge.
            seg((11.0, 9.5), (15.0, 9.5));
            seg((13.0, 7.5), (13.0, 11.5));
        }
        Glyph::Refresh => {
            // 270° arc approximated with segments + arrowhead.
            let c = p(8.0, 8.5);
            let r = 5.0;
            let steps = 12;
            let a0 = 0.6_f32;
            let a1 = 5.4_f32;
            let mut prev = egui::pos2(c.x + r * a0.cos(), c.y + r * a0.sin());
            for i in 1..=steps {
                let a = a0 + (a1 - a0) * i as f32 / steps as f32;
                let next = egui::pos2(c.x + r * a.cos(), c.y + r * a.sin());
                painter.line_segment([prev, next], stroke);
                prev = next;
            }
            seg((11.6, 1.6), (13.2, 5.4));
            seg((13.2, 5.4), (9.4, 5.8));
        }
        Glyph::CollapseAll => {
            // Double chevron up.
            seg((3.0, 9.5), (8.0, 5.0));
            seg((8.0, 5.0), (13.0, 9.5));
            seg((3.0, 13.5), (8.0, 9.0));
            seg((8.0, 9.0), (13.0, 13.5));
        }
    }
}

/// Frameless square icon button. Returns the click response.
pub fn icon_button(
    ui: &mut egui::Ui,
    theme: &VscodeTheme,
    glyph: Glyph,
    tooltip: &str,
    size: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.list_hover);
    }
    let fg = if resp.hovered() {
        theme.activity_fg
    } else {
        theme.activity_inactive_fg
    };
    draw_glyph(ui.painter(), glyph, rect.shrink(size * 0.22), fg, 1.4);
    resp.on_hover_text(tooltip)
}

/// Primary action button (VSCode blue idle + lighter blue hover).
/// Global widget chrome is neutral grey (scrollbars, secondary buttons),
// so primaries opt back into blue with a scoped style override.
pub fn primary_button(ui: &mut egui::Ui, theme: &VscodeTheme, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.bg_fill = theme.button_bg;
        visuals.widgets.inactive.weak_bg_fill = theme.button_bg;
        visuals.widgets.hovered.bg_fill = theme.button_hover;
        visuals.widgets.hovered.weak_bg_fill = theme.button_hover;
        visuals.widgets.active.bg_fill = theme.button_hover;
        visuals.widgets.active.weak_bg_fill = theme.button_hover;
        ui.button(label)
    })
    .inner
}

// ---------------------------------------------------------------------------
// Explorer file / folder icons (Seti-style, painter-drawn).
//
// Why painter-drawn instead of `.svg` files: every `.svg` on disk would mean
// file I/O + `resvg` raster + texture upload per tree row, every frame the
// row is visible. That breaks the explorer budgets (lazy tree, <300ms
// project-open, <150MB idle). Stroked vectors cost ~5-10 line segments per
// visible row (~50-100 rows => <0.1ms), need no cache, stay crisp at any
// DPI, and add zero binary weight. The single `public/icons/logo.svg` is
// fine because it rasterizes once; per-file icons must never follow that
// path.
// ---------------------------------------------------------------------------

/// Per-language file identity. Colors are Seti-inspired, tuned for the
/// pitch-black Arctis theme (bright enough on `#000`, muted enough to sit
/// next to grey chrome).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIconKind {
    Rust,
    Cargo,
    Toml,
    Markdown,
    Json,
    Js,
    Jsx,
    Ts,
    Tsx,
    Python,
    Go,
    Java,
    Kotlin,
    Swift,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Lua,
    Dart,
    Vue,
    Svelte,
    Html,
    Css,
    Yaml,
    Shell,
    Sql,
    Docker,
    Git,
    Env,
    Image,
    Archive,
    Text,
    Generic,
}

/// Named-folder identity. `Generic` is the VSCode-style muted-gold folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderKind {
    Src,
    Public,
    Docs,
    Tests,
    Target,
    NodeModules,
    Git,
    Vscode,
    Opencode,
    Dist,
    Assets,
    Scripts,
    Examples,
    Generic,
}

/// Unified tree-row icon. `tree_row` already knows folder vs. file from its
/// `twistie` param (`Some` = folder), so this just carries the kind; the
/// folder open/closed shape is derived from `twistie` at draw time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeIcon {
    File(FileIconKind),
    Folder(FolderKind),
}

/// Map a file path to its icon kind. Zero-alloc: filename + extension
/// probes only, `eq_ignore_ascii_case` chains, no `to_lowercase()`.
/// Exact filenames win over extensions (`Dockerfile` beats `.dockerfile`
/// ext, `Cargo.toml` beats generic `toml`).
pub fn file_icon_for(path: &std::path::Path) -> FileIconKind {
    use FileIconKind as F;
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        // Exact / well-known filenames (case-insensitive).
        if name.eq_ignore_ascii_case("dockerfile")
            || name.eq_ignore_ascii_case("docker-compose.yml")
            || name.eq_ignore_ascii_case("docker-compose.yaml")
            || name.eq_ignore_ascii_case("containerfile")
        {
            return F::Docker;
        }
        if name.eq_ignore_ascii_case("cargo.toml") || name.eq_ignore_ascii_case("cargo.lock") {
            return F::Cargo;
        }
        if name.eq_ignore_ascii_case("makefile")
            || name.eq_ignore_ascii_case("cmakelists.txt")
            || name.eq_ignore_ascii_case("justfile")
        {
            return F::Shell;
        }
        if name.eq_ignore_ascii_case(".gitignore")
            || name.eq_ignore_ascii_case(".gitattributes")
            || name.eq_ignore_ascii_case(".gitmodules")
        {
            return F::Git;
        }
        if name.eq_ignore_ascii_case(".env")
            || name.starts_with(".env.")
            || name.eq_ignore_ascii_case(".env.example")
        {
            return F::Env;
        }
        if name.eq_ignore_ascii_case("package.json")
            || name.eq_ignore_ascii_case("package-lock.json")
            || name.eq_ignore_ascii_case("yarn.lock")
            || name.eq_ignore_ascii_case("pnpm-lock.yaml")
        {
            return F::Js;
        }
        if name.eq_ignore_ascii_case("readme")
            || name.eq_ignore_ascii_case("readme.md")
            || name.eq_ignore_ascii_case("license")
            || name.eq_ignore_ascii_case("licence")
            || name.eq_ignore_ascii_case("changelog.md")
        {
            return F::Markdown;
        }
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    // Multi-dot names (`d.ts`, `test.jsx`) still classify by final ext,
    // which is what the editor's `tabs::classify` does too.
    if ext.eq_ignore_ascii_case("rs") {
        return F::Rust;
    }
    if ext.eq_ignore_ascii_case("toml") {
        return F::Toml;
    }
    if ext.eq_ignore_ascii_case("md")
        || ext.eq_ignore_ascii_case("mdx")
        || ext.eq_ignore_ascii_case("markdown")
    {
        return F::Markdown;
    }
    if ext.eq_ignore_ascii_case("json")
        || ext.eq_ignore_ascii_case("jsonc")
        || ext.eq_ignore_ascii_case("json5")
    {
        return F::Json;
    }
    if ext.eq_ignore_ascii_case("js")
        || ext.eq_ignore_ascii_case("mjs")
        || ext.eq_ignore_ascii_case("cjs")
    {
        return F::Js;
    }
    if ext.eq_ignore_ascii_case("jsx") {
        return F::Jsx;
    }
    if ext.eq_ignore_ascii_case("ts")
        || ext.eq_ignore_ascii_case("mts")
        || ext.eq_ignore_ascii_case("cts")
    {
        return F::Ts;
    }
    if ext.eq_ignore_ascii_case("tsx") {
        return F::Tsx;
    }
    if ext.eq_ignore_ascii_case("vue") {
        return F::Vue;
    }
    if ext.eq_ignore_ascii_case("svelte") {
        return F::Svelte;
    }
    if ext.eq_ignore_ascii_case("py")
        || ext.eq_ignore_ascii_case("pyi")
        || ext.eq_ignore_ascii_case("pyw")
    {
        return F::Python;
    }
    if ext.eq_ignore_ascii_case("go") {
        return F::Go;
    }
    if ext.eq_ignore_ascii_case("java") {
        return F::Java;
    }
    if ext.eq_ignore_ascii_case("kt") || ext.eq_ignore_ascii_case("kts") {
        return F::Kotlin;
    }
    if ext.eq_ignore_ascii_case("swift") {
        return F::Swift;
    }
    if ext.eq_ignore_ascii_case("c") || ext.eq_ignore_ascii_case("h") {
        return F::C;
    }
    if ext.eq_ignore_ascii_case("cpp")
        || ext.eq_ignore_ascii_case("cc")
        || ext.eq_ignore_ascii_case("cxx")
        || ext.eq_ignore_ascii_case("hpp")
        || ext.eq_ignore_ascii_case("hh")
        || ext.eq_ignore_ascii_case("hxx")
    {
        return F::Cpp;
    }
    if ext.eq_ignore_ascii_case("cs") {
        return F::CSharp;
    }
    if ext.eq_ignore_ascii_case("rb") {
        return F::Ruby;
    }
    if ext.eq_ignore_ascii_case("php") {
        return F::Php;
    }
    if ext.eq_ignore_ascii_case("lua") {
        return F::Lua;
    }
    if ext.eq_ignore_ascii_case("dart") {
        return F::Dart;
    }
    if ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm") {
        return F::Html;
    }
    if ext.eq_ignore_ascii_case("css")
        || ext.eq_ignore_ascii_case("scss")
        || ext.eq_ignore_ascii_case("sass")
        || ext.eq_ignore_ascii_case("less")
    {
        return F::Css;
    }
    if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
        return F::Yaml;
    }
    if ext.eq_ignore_ascii_case("sh")
        || ext.eq_ignore_ascii_case("bash")
        || ext.eq_ignore_ascii_case("zsh")
        || ext.eq_ignore_ascii_case("fish")
    {
        return F::Shell;
    }
    if ext.eq_ignore_ascii_case("sql") {
        return F::Sql;
    }
    if ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("gif")
        || ext.eq_ignore_ascii_case("webp")
        || ext.eq_ignore_ascii_case("bmp")
        || ext.eq_ignore_ascii_case("ico")
        || ext.eq_ignore_ascii_case("tiff")
    {
        return F::Image;
    }
    if ext.eq_ignore_ascii_case("svg") {
        return F::Image;
    }
    if ext.eq_ignore_ascii_case("zip")
        || ext.eq_ignore_ascii_case("tar")
        || ext.eq_ignore_ascii_case("gz")
        || ext.eq_ignore_ascii_case("tgz")
        || ext.eq_ignore_ascii_case("bz2")
        || ext.eq_ignore_ascii_case("xz")
        || ext.eq_ignore_ascii_case("7z")
        || ext.eq_ignore_ascii_case("rar")
    {
        return F::Archive;
    }
    if ext.eq_ignore_ascii_case("txt") || ext.eq_ignore_ascii_case("log") {
        return F::Text;
    }
    F::Generic
}

/// Accent color per file kind. `Generic`/`Text` fall back to theme greys at
/// draw time (caller passes `theme`), so this returns `None` for them.
pub fn file_icon_color(kind: FileIconKind) -> Option<egui::Color32> {
    use egui::Color32 as C;
    use FileIconKind as F;
    Some(match kind {
        F::Rust => C::from_rgb(0xDE, 0xA5, 0x84),
        F::Cargo => C::from_rgb(0xFF, 0x6B, 0x33),
        F::Toml => C::from_rgb(0xA1, 0x62, 0x07),
        F::Markdown => C::from_rgb(0x51, 0x9A, 0xBA),
        F::Json => C::from_rgb(0xCB, 0xCB, 0x41),
        F::Js => C::from_rgb(0xE8, 0xD4, 0x4D),
        F::Jsx => C::from_rgb(0x61, 0xDA, 0xFB),
        F::Ts => C::from_rgb(0x31, 0x78, 0xC6),
        F::Tsx => C::from_rgb(0x61, 0xAF, 0xEF),
        F::Vue => C::from_rgb(0x41, 0xB8, 0x83),
        F::Svelte => C::from_rgb(0xFF, 0x3E, 0x00),
        F::Python => C::from_rgb(0x4B, 0x8B, 0xBE),
        F::Go => C::from_rgb(0x00, 0xAD, 0xD8),
        F::Java => C::from_rgb(0xED, 0x8B, 0x00),
        F::Kotlin => C::from_rgb(0x7F, 0x52, 0xFF),
        F::Swift => C::from_rgb(0xF0, 0x51, 0x38),
        F::C => C::from_rgb(0x65, 0x9A, 0xD2),
        F::Cpp => C::from_rgb(0x64, 0x9A, 0xD2),
        F::CSharp => C::from_rgb(0x38, 0x8A, 0x34),
        F::Ruby => C::from_rgb(0xCC, 0x34, 0x2D),
        F::Php => C::from_rgb(0x77, 0x7B, 0xB4),
        F::Lua => C::from_rgb(0x7E, 0x7E, 0xFF),
        F::Dart => C::from_rgb(0x01, 0x75, 0xC2),
        F::Html => C::from_rgb(0xE3, 0x4C, 0x26),
        F::Css => C::from_rgb(0x42, 0xA5, 0xF5),
        F::Yaml => C::from_rgb(0xCB, 0x17, 0x1E),
        F::Shell => C::from_rgb(0x4E, 0xAA, 0x25),
        F::Sql => C::from_rgb(0xDD, 0xAA, 0x00),
        F::Docker => C::from_rgb(0x24, 0x96, 0xED),
        F::Git => C::from_rgb(0xF0, 0x50, 0x32),
        F::Env => C::from_rgb(0xBA, 0xBB, 0x00),
        F::Image => C::from_rgb(0xA0, 0x74, 0xC4),
        F::Archive => C::from_rgb(0x8A, 0x8A, 0x8A),
        F::Text | F::Generic => return None,
    })
}

/// Map a folder name to its kind. Zero-alloc `eq_ignore_ascii_case` chain;
/// dot-folders (`.git`, `.vscode`) match exactly.
pub fn folder_kind_for(dir_name: &str) -> FolderKind {
    use FolderKind as K;
    if dir_name.eq_ignore_ascii_case("src") || dir_name.eq_ignore_ascii_case("source") {
        return K::Src;
    }
    if dir_name.eq_ignore_ascii_case("public")
        || dir_name.eq_ignore_ascii_case("static")
        || dir_name.eq_ignore_ascii_case("www")
    {
        return K::Public;
    }
    if dir_name.eq_ignore_ascii_case("docs")
        || dir_name.eq_ignore_ascii_case("doc")
        || dir_name.eq_ignore_ascii_case("documentation")
    {
        return K::Docs;
    }
    if dir_name.eq_ignore_ascii_case("tests")
        || dir_name.eq_ignore_ascii_case("test")
        || dir_name.eq_ignore_ascii_case("__tests__")
        || dir_name.eq_ignore_ascii_case("spec")
    {
        return K::Tests;
    }
    if dir_name.eq_ignore_ascii_case("target")
        || dir_name.eq_ignore_ascii_case("debug")
        || dir_name.eq_ignore_ascii_case("release")
    {
        return K::Target;
    }
    if dir_name.eq_ignore_ascii_case("node_modules") {
        return K::NodeModules;
    }
    if dir_name.eq_ignore_ascii_case(".git") {
        return K::Git;
    }
    if dir_name.eq_ignore_ascii_case(".vscode") {
        return K::Vscode;
    }
    if dir_name.eq_ignore_ascii_case(".opencode") || dir_name.eq_ignore_ascii_case(".arctis") {
        return K::Opencode;
    }
    if dir_name.eq_ignore_ascii_case("dist")
        || dir_name.eq_ignore_ascii_case("build")
        || dir_name.eq_ignore_ascii_case("out")
        || dir_name.eq_ignore_ascii_case("output")
    {
        return K::Dist;
    }
    if dir_name.eq_ignore_ascii_case("assets")
        || dir_name.eq_ignore_ascii_case("icons")
        || dir_name.eq_ignore_ascii_case("fonts")
        || dir_name.eq_ignore_ascii_case("styles")
        || dir_name.eq_ignore_ascii_case("images")
    {
        return K::Assets;
    }
    if dir_name.eq_ignore_ascii_case("scripts") || dir_name.eq_ignore_ascii_case("tools") {
        return K::Scripts;
    }
    if dir_name.eq_ignore_ascii_case("examples") || dir_name.eq_ignore_ascii_case("example") {
        return K::Examples;
    }
    K::Generic
}

/// Accent color per folder kind. Generic is VSCode-style muted gold.
pub fn folder_color(kind: FolderKind) -> egui::Color32 {
    use egui::Color32 as C;
    use FolderKind as K;
    match kind {
        K::Src => C::from_rgb(0x42, 0xA5, 0xF5),
        K::Public => C::from_rgb(0xE8, 0xD4, 0x4D),
        K::Docs => C::from_rgb(0x51, 0x9A, 0xBA),
        K::Tests => C::from_rgb(0x4C, 0xAF, 0x50),
        K::Target => C::from_rgb(0x8A, 0x8A, 0x8A),
        K::NodeModules => C::from_rgb(0x4E, 0xAA, 0x25),
        K::Git => C::from_rgb(0xF0, 0x50, 0x32),
        K::Vscode => C::from_rgb(0x00, 0x7A, 0xCC),
        K::Opencode => C::from_rgb(0xB3, 0x88, 0xFF),
        K::Dist => C::from_rgb(0x8A, 0x8A, 0x8A),
        K::Assets => C::from_rgb(0xC6, 0x78, 0xDD),
        K::Scripts => C::from_rgb(0xE5, 0xC0, 0x7B),
        K::Examples => C::from_rgb(0x56, 0xB6, 0xC2),
        K::Generic => C::from_rgb(0xC0, 0x95, 0x53),
    }
}

/// Draw a file icon in a 16x16-ish `rect`. Same outline language as
/// `Glyph::NewFile`: document + folded corner in the language accent,
/// dimmed text lines, plus micro-badges for Image / Archive / Docker / Git
/// so those read without relying on color alone.
pub fn draw_file_icon(
    painter: &egui::Painter,
    kind: FileIconKind,
    rect: egui::Rect,
    theme: &VscodeTheme,
) {
    let accent = file_icon_color(kind).unwrap_or(theme.muted);
    let stroke = egui::Stroke::new(1.2, accent);
    let dim = egui::Stroke::new(1.0, accent.gamma_multiply(0.65));
    let s = 16.0_f32.min(rect.width()).min(rect.height());
    let org = egui::pos2(rect.center().x - s / 2.0, rect.center().y - s / 2.0);
    let p = |x: f32, y: f32| egui::pos2(org.x + x, org.y + y);
    let seg = |a: (f32, f32), b: (f32, f32), st: egui::Stroke| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], st);
    };

    match kind {
        FileIconKind::Image => {
            // Frame + sun + mountains.
            for (a, b) in [
                ((2.5, 3.0), (13.5, 3.0)),
                ((2.5, 3.0), (2.5, 13.0)),
                ((2.5, 13.0), (13.5, 13.0)),
                ((13.5, 3.0), (13.5, 13.0)),
            ] {
                seg(a, b, stroke);
            }
            painter.circle_stroke(p(6.0, 6.5), 1.4, dim);
            painter.line_segment([p(3.0, 12.0), p(7.0, 8.0)], dim);
            painter.line_segment([p(7.0, 8.0), p(9.5, 10.5)], dim);
            painter.line_segment([p(9.5, 10.5), p(11.5, 8.5)], dim);
            painter.line_segment([p(11.5, 8.5), p(13.0, 10.0)], dim);
        }
        FileIconKind::Archive => {
            // Box + lid + clasp.
            for (a, b) in [
                ((2.5, 5.0), (13.5, 5.0)),
                ((3.5, 5.0), (3.5, 13.0)),
                ((3.5, 13.0), (12.5, 13.0)),
                ((12.5, 5.0), (12.5, 13.0)),
                ((2.5, 5.0), (2.5, 3.5)),
                ((2.5, 3.5), (13.5, 3.5)),
                ((13.5, 3.5), (13.5, 5.0)),
            ] {
                seg(a, b, stroke);
            }
            seg((7.0, 3.5), (7.0, 5.0), dim);
            seg((9.0, 3.5), (9.0, 5.0), dim);
        }
        FileIconKind::Docker => {
            // Stacked layers.
            for y in [4.0, 7.5, 11.0] {
                seg((3.0, y), (8.0, y), stroke);
                seg((8.0, y), (13.0, y + 1.8), stroke);
                seg((3.0, y), (3.0, y + 0.2), stroke);
            }
        }
        FileIconKind::Git => {
            // Mini branch: two nodes + merge.
            painter.circle_stroke(p(5.0, 4.0), 1.6, stroke);
            painter.circle_stroke(p(5.0, 12.0), 1.6, stroke);
            painter.circle_stroke(p(11.0, 8.0), 1.6, stroke);
            seg((5.0, 5.6), (5.0, 10.4), dim);
            seg((6.4, 4.6), (9.6, 7.2), dim);
            seg((6.4, 11.4), (9.6, 8.8), dim);
        }
        _ => {
            // Document + folded corner + text lines (Seti default).
            seg((3.0, 1.5), (9.5, 1.5), stroke);
            seg((3.0, 1.5), (3.0, 14.5), stroke);
            seg((3.0, 14.5), (13.0, 14.5), stroke);
            seg((13.0, 14.5), (13.0, 5.0), stroke);
            seg((9.5, 1.5), (9.5, 5.0), stroke);
            seg((9.5, 5.0), (13.0, 5.0), stroke);
            seg((5.5, 8.0), (10.5, 8.0), dim);
            seg((5.5, 11.0), (10.5, 11.0), dim);
        }
    }
}

/// Draw a folder icon in a 16x16-ish `rect`. Closed shows tab + body;
/// open adds the inner mouth line so expanded state reads without relying
/// on the chevron alone.
pub fn draw_folder_icon(
    painter: &egui::Painter,
    kind: FolderKind,
    rect: egui::Rect,
    expanded: bool,
) {
    let color = folder_color(kind);
    let stroke = egui::Stroke::new(1.2, color);
    let dim = egui::Stroke::new(1.0, color.gamma_multiply(0.6));
    let s = 16.0_f32.min(rect.width()).min(rect.height());
    let org = egui::pos2(rect.center().x - s / 2.0, rect.center().y - s / 2.0);
    let p = |x: f32, y: f32| egui::pos2(org.x + x, org.y + y);
    let seg = |a: (f32, f32), b: (f32, f32), st: egui::Stroke| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], st);
    };
    // Tab + body.
    seg((1.5, 5.0), (1.5, 13.0), stroke);
    seg((1.5, 13.0), (14.5, 13.0), stroke);
    seg((14.5, 13.0), (14.5, 5.0), stroke);
    seg((14.5, 5.0), (7.5, 5.0), stroke);
    seg((7.5, 5.0), (6.5, 3.0), stroke);
    seg((6.5, 3.0), (1.5, 3.0), stroke);
    seg((1.5, 3.0), (1.5, 5.0), stroke);
    if expanded {
        // Open mouth.
        seg((3.0, 7.5), (13.0, 7.5), dim);
    }
}

/// Draw any tree icon (dispatch helper for `window::tree_row`).
pub fn draw_tree_icon(
    painter: &egui::Painter,
    icon: TreeIcon,
    rect: egui::Rect,
    theme: &VscodeTheme,
    expanded: bool,
) {
    match icon {
        TreeIcon::File(kind) => draw_file_icon(painter, kind, rect, theme),
        TreeIcon::Folder(kind) => draw_folder_icon(painter, kind, rect, expanded),
    }
}
