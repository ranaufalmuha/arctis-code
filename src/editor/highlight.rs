//! Dependency-free syntax highlighting (VSCode colors with TextMate-style
//! regex grammars too — no C parsers, no daemons, nothing to link).
//!
//! A small byte scanner per language family: keywords, strings, comments,
//! numbers, constants. Delimiters are ASCII, so byte slicing never splits
//! UTF-8 (non-ASCII bytes count as identifier characters).
//!
//! Structural scopes (function/type) are deliberately out: without a real
//! parser they would be guesswork. That lives in a future extension.
//!
//! Suggestion engines (LSP et al.) are NOT here either: see
//! `v2_stub::CompletionProvider` — extensions only, never core.

use std::path::{Path, PathBuf};

/// Files larger than this are shown unhighlighted (scan + layout per
/// frame would jank; the editor stays usable).
pub const MAX_HL_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Json,
    Python,
    Toml,
}

/// Grammar pick by extension. Everything known gets *some* coloring;
/// unknown files stay plain.
pub fn language_for(path: &Path) -> Option<Lang> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("rs") => Some(Lang::Rust),
        Some("js" | "jsx" | "mjs" | "cjs") => Some(Lang::JavaScript),
        Some("ts" | "mts" | "cts") => Some(Lang::TypeScript),
        Some("tsx") => Some(Lang::Tsx),
        Some("json" | "jsonc") => Some(Lang::Json),
        Some("py" | "pyi" | "pyw") => Some(Lang::Python),
        Some("toml") => Some(Lang::Toml),
        _ => None,
    }
}

/// Fixed token palette, tuned for the pitch-black theme (code colors are
/// intentionally theme-independent, like VSCode token colors).
/// Brackets rotate 1-2-3 by depth; an unmatched closer is an error.
pub fn span_color(name: &str) -> Option<egui::Color32> {
    use egui::Color32 as C;
    Some(match name {
        "keyword" => C::from_rgb(0x56, 0x9C, 0xD6),
        "string" => C::from_rgb(0xCE, 0x91, 0x78),
        "comment" => C::from_rgb(0x6A, 0x99, 0x55),
        "number" => C::from_rgb(0xB5, 0xCE, 0xA8),
        "constant" => C::from_rgb(0x4F, 0xC1, 0xFF),
        "bracket1" => C::from_rgb(0xFF, 0xD7, 0x00),
        "bracket2" => C::from_rgb(0xDA, 0x70, 0xD6),
        "bracket3" => C::from_rgb(0x17, 0x9F, 0xFF),
        "bracket_error" => C::from_rgb(0xF1, 0x4C, 0x4C),
        _ => return None,
    })
}

/// Bracket depth (1-based) maps onto the rotating palette.
fn bracket_kind(depth: usize) -> &'static str {
    match (depth - 1) % 3 {
        0 => "bracket1",
        1 => "bracket2",
        _ => "bracket3",
    }
}

fn is_opener(b: u8) -> bool {
    matches!(b, b'(' | b'[' | b'{')
}

fn is_closer_for(open: u8, close: u8) -> bool {
    matches!((open, close), (b'(', b')') | (b'[', b']') | (b'{', b'}'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub color: egui::Color32,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn is_keyword(lang: Lang, word: &str) -> bool {
    match lang {
        Lang::Rust => matches!(
            word,
            "fn" | "let"
                | "mut"
                | "pub"
                | "use"
                | "mod"
                | "struct"
                | "enum"
                | "impl"
                | "trait"
                | "if"
                | "else"
                | "match"
                | "for"
                | "while"
                | "loop"
                | "return"
                | "in"
                | "where"
                | "type"
                | "const"
                | "static"
                | "crate"
                | "self"
                | "Self"
                | "super"
                | "as"
                | "ref"
                | "move"
                | "dyn"
                | "async"
                | "await"
                | "try"
                | "union"
                | "unsafe"
                | "extern"
                | "break"
                | "continue"
        ),
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => matches!(
            word,
            "function"
                | "return"
                | "if"
                | "else"
                | "for"
                | "while"
                | "const"
                | "let"
                | "var"
                | "new"
                | "class"
                | "extends"
                | "import"
                | "from"
                | "export"
                | "default"
                | "try"
                | "catch"
                | "throw"
                | "switch"
                | "case"
                | "break"
                | "continue"
                | "typeof"
                | "instanceof"
                | "in"
                | "of"
                | "do"
                | "async"
                | "await"
                | "yield"
                | "static"
                | "this"
                | "super"
                | "delete"
                | "void"
                | "debugger"
                | "interface"
                | "type"
                | "implements"
                | "declare"
                | "namespace"
                | "abstract"
                | "readonly"
                | "keyof"
                | "infer"
                | "satisfies"
        ),
        Lang::Python => matches!(
            word,
            "def"
                | "class"
                | "return"
                | "if"
                | "elif"
                | "else"
                | "for"
                | "while"
                | "import"
                | "from"
                | "as"
                | "try"
                | "except"
                | "finally"
                | "raise"
                | "with"
                | "lambda"
                | "pass"
                | "break"
                | "continue"
                | "global"
                | "nonlocal"
                | "assert"
                | "del"
                | "yield"
                | "async"
                | "await"
                | "in"
                | "is"
                | "not"
                | "and"
                | "or"
        ),
        Lang::Json | Lang::Toml => false,
    }
}

fn is_constant(lang: Lang, word: &str) -> bool {
    match lang {
        Lang::Rust => matches!(word, "true" | "false"),
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => {
            matches!(word, "true" | "false" | "null" | "undefined")
        }
        Lang::Json => matches!(word, "true" | "false" | "null"),
        Lang::Python => matches!(word, "True" | "False" | "None"),
        Lang::Toml => matches!(word, "true" | "false"),
    }
}

/// `true` when a `/` opens a regex: the previous significant byte can only
/// end an expression after these (elsewhere `/` is division).
fn regex_allowed_before(prev: Option<u8>) -> bool {
    matches!(
        prev,
        None | Some(b'=')
            | Some(b'(')
            | Some(b',')
            | Some(b':')
            | Some(b'[')
            | Some(b'!')
            | Some(b'&')
            | Some(b'|')
            | Some(b'?')
            | Some(b'{')
            | Some(b'}')
            | Some(b';')
    )
}

fn push(spans: &mut Vec<Span>, start: usize, end: usize, kind: &str) {
    if start >= end {
        return;
    }
    let Some(color) = span_color(kind) else {
        return;
    };
    if let Some(last) = spans.last_mut() {
        if last.color == color && last.end == start {
            last.end = end;
            return;
        }
    }
    spans.push(Span { start, end, color });
}

/// Byte-range spans for `content`, or `None` when highlighting is
/// unavailable (unknown language, oversize file). Total, never panics.
pub fn highlight_spans(path: &Path, content: &str) -> Option<Vec<Span>> {
    if content.len() > MAX_HL_BYTES {
        return None;
    }
    let lang = language_for(path)?;
    let b = content.as_bytes();
    let n = b.len();
    let mut spans = Vec::new();
    let mut i = 0;
    // Last significant (non-blank) byte before `i`, for the regex heuristic.
    let mut prev_sig: Option<u8> = None;
    // Open brackets outside strings/comments, for depth coloring.
    let mut stack: Vec<u8> = Vec::new();

    // Block-comment nesting (only Rust nests).
    let mut block_depth = 0usize;
    let mut block_start = 0usize;

    let c_like = !matches!(lang, Lang::Python | Lang::Toml);
    let hashes = matches!(lang, Lang::Python | Lang::Toml);

    while i < n {
        let c = b[i];
        // Inside a block comment: only the end (or nesting) matters.
        if block_depth > 0 {
            if c_like && c == b'/' && b.get(i + 1) == Some(&b'*') && lang == Lang::Rust {
                block_depth += 1;
                i += 2;
                continue;
            }
            if c == b'*' && b.get(i + 1) == Some(&b'/') {
                block_depth -= 1;
                i += 2;
                if block_depth == 0 {
                    push(&mut spans, block_start, i, "comment");
                    prev_sig = Some(b'/');
                }
                continue;
            }
            i += 1;
            continue;
        }

        // Line comments.
        if c_like && c == b'/' && b.get(i + 1) == Some(&b'/') {
            let start = i;
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            push(&mut spans, start, i, "comment");
            prev_sig = Some(b'/');
            continue;
        }
        if hashes && c == b'#' {
            let start = i;
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            push(&mut spans, start, i, "comment");
            prev_sig = Some(b'#');
            continue;
        }
        // Block comments.
        if c_like && c == b'/' && b.get(i + 1) == Some(&b'*') {
            block_depth = 1;
            block_start = i;
            i += 2;
            continue;
        }
        // Rust raw strings: r"..." / r#"..."# / r##"..."##.
        if lang == Lang::Rust && c == b'r' && !is_word_byte(b.get(i + 1).copied().unwrap_or(b' ')) {
            let mut j = i + 1;
            while b.get(j) == Some(&b'#') {
                j += 1;
            }
            if b.get(j) == Some(&b'"') {
                let hashes_n = j - i - 1;
                j += 1;
                while j < n {
                    if b[j] == b'"' {
                        let mut k = j + 1;
                        while k < n && k - j - 1 < hashes_n && b[k] == b'#' {
                            k += 1;
                        }
                        if k - j - 1 == hashes_n {
                            j = k;
                            break;
                        }
                    }
                    j += 1;
                }
                push(&mut spans, i, j.min(n), "string");
                prev_sig = Some(b'"');
                i = j.min(n);
                continue;
            }
        }
        // JS/TS regex literal (heuristic, char-class aware).
        if matches!(lang, Lang::JavaScript | Lang::TypeScript | Lang::Tsx)
            && c == b'/'
            && b.get(i + 1) != Some(&b'/')
            && b.get(i + 1) != Some(&b'*')
            && regex_allowed_before(prev_sig)
        {
            let start = i;
            i += 1;
            let mut in_class = false;
            let mut closed = false;
            while i < n {
                let d = b[i];
                if d == b'\\' {
                    i += 2;
                    continue;
                }
                if d == b'[' {
                    in_class = true;
                } else if d == b']' {
                    in_class = false;
                } else if d == b'/' && !in_class {
                    closed = true;
                    i += 1;
                    break;
                } else if d == b'\n' {
                    break;
                }
                i += 1;
            }
            if closed {
                while i < n && b[i].is_ascii_alphabetic() {
                    i += 1; // flags: gimsuy
                }
                push(&mut spans, start, i, "string");
                prev_sig = Some(b'/');
                continue;
            }
            i = start; // not a regex: fall through to division (plain)
        }
        // Strings (and Python triple-quoted).
        if c == b'"' || c == b'\'' || (c == b'`' && c_like && lang != Lang::Rust) {
            // Rust lifetimes ('a) are not strings: only a char-shaped
            // `'x'` / `'\..'` counts as a char literal.
            if lang == Lang::Rust && c == b'\'' {
                let ok = match (b.get(i + 1), b.get(i + 2), b.get(i + 3)) {
                    (Some(b'\\'), _, _) => {
                        // Escape: scan to the closing quote.
                        let mut j = i + 2;
                        if b.get(j) == Some(&b'{') {
                            while j < n && b[j] != b'}' {
                                j += 1;
                            }
                            j += 1;
                        } else {
                            while j < n && b[j] != b'\'' && b[j] != b'\n' {
                                j += 1;
                            }
                        }
                        j < n && b[j] == b'\''
                    }
                    (Some(_), Some(b'\''), _) => b[i + 1] != b'\n',
                    _ => false,
                };
                if !ok {
                    // Lifetime or stray tick: leave plain.
                    prev_sig = Some(c);
                    i += 1;
                    continue;
                }
            }
            let quote = c;
            let triple = b.get(i + 1) == Some(&quote)
                && b.get(i + 2) == Some(&quote)
                && matches!(lang, Lang::Python | Lang::Toml);
            let start = i;
            i += if triple { 3 } else { 1 };
            while i < n {
                let d = b[i];
                if !triple && d == b'\n' {
                    break; // unterminated single-line string
                }
                // Backslash escapes everywhere except TOML literal strings.
                if d == b'\\' && !(lang == Lang::Toml && quote == b'\'') {
                    i += 2;
                    continue;
                }
                if d == quote {
                    if triple {
                        if b.get(i + 1) == Some(&quote) && b.get(i + 2) == Some(&quote) {
                            i += 3;
                            break;
                        }
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            // A string running past EOL/EOF still colors up to the break.
            push(&mut spans, start, i.min(n), "string");
            prev_sig = Some(quote);
            continue;
        }
        // Numbers.
        if c.is_ascii_digit() {
            let start = i;
            while i < n && (is_word_byte(b[i]) || b[i] == b'.') {
                i += 1;
            }
            push(&mut spans, start, i, "number");
            prev_sig = b[start..i].last().copied();
            continue;
        }
        // Words: keywords, constants, else plain.
        if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            let start = i;
            while i < n && is_word_byte(b[i]) {
                i += 1;
            }
            let word = &content[start..i];
            if is_keyword(lang, word) {
                push(&mut spans, start, i, "keyword");
            } else if is_constant(lang, word) {
                push(&mut spans, start, i, "constant");
            }
            // A statement keyword can precede a regex (`return /x/`);
            // anything else ending here makes `/` a division.
            prev_sig = if matches!(
                word,
                "return"
                    | "typeof"
                    | "throw"
                    | "case"
                    | "do"
                    | "else"
                    | "in"
                    | "of"
                    | "delete"
                    | "void"
                    | "yield"
                    | "new"
            ) {
                None
            } else {
                b[start..i].last().copied()
            };
            continue;
        }
        if !c.is_ascii_whitespace() {
            prev_sig = Some(c);
        }
        // Brackets outside strings/comments get depth colors; a closer
        // with no matching opener glows as an error.
        if is_opener(c) {
            stack.push(c);
            push(&mut spans, i, i + 1, bracket_kind(stack.len()));
        } else if matches!(c, b')' | b']' | b'}') {
            match stack.pop() {
                Some(open) if is_closer_for(open, c) => {
                    push(&mut spans, i, i + 1, bracket_kind(stack.len() + 1))
                }
                _ => push(&mut spans, i, i + 1, "bracket_error"),
            }
        }
        i += 1;
    }
    Some(spans)
}

/// Per-tab highlight cache: recompute only when the tab, length, or an
/// edit says so (never re-parse an unchanged buffer every frame).
#[derive(Debug, Default)]
pub struct HlCache {
    path: PathBuf,
    len: usize,
    dirty: bool,
    spans: Option<Vec<Span>>,
}

impl HlCache {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn spans_for(&mut self, path: &Path, content: &str) -> Option<&[Span]> {
        if self.path.as_os_str() != path.as_os_str() || self.len != content.len() || self.dirty {
            self.path = path.to_path_buf();
            self.len = content.len();
            self.dirty = false;
            self.spans = highlight_spans(path, content);
        }
        self.spans.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages() {
        assert_eq!(language_for(Path::new("main.rs")), Some(Lang::Rust));
        assert_eq!(language_for(Path::new("app.tsx")), Some(Lang::Tsx));
        assert_eq!(language_for(Path::new("data.json")), Some(Lang::Json));
        assert_eq!(language_for(Path::new("x.toml")), Some(Lang::Toml));
        assert_eq!(language_for(Path::new("Makefile")), None);
    }

    fn kinds(src: &str, path: &str) -> Vec<String> {
        highlight_spans(Path::new(path), src)
            .unwrap()
            .iter()
            .map(|s| src[s.start..s.end].to_owned())
            .collect()
    }

    #[test]
    fn rust_snippet_highlights() {
        let src = "fn main() {\n  // hi\n  let s = \"x\";\n}\n";
        let k = kinds(src, "a.rs");
        assert!(k.iter().any(|k| k == "fn"), "keyword: {k:?}");
        assert!(k.iter().any(|k| k == "// hi"), "comment: {k:?}");
        assert!(k.iter().any(|k| k == "\"x\""), "string: {k:?}");
    }

    #[test]
    fn rust_char_is_not_lifetime() {
        let k = kinds("let c = 'x';\nfn f<'a>(x: &'a str) {}\n", "a.rs");
        assert!(k.iter().any(|k| k == "'x'"), "char: {k:?}");
        assert!(!k.iter().any(|k| k == "'a"), "lifetime plain: {k:?}");
    }

    #[test]
    fn rust_raw_string_and_nested_comment() {
        let k = kinds("let s = r#\"a\"b\"#;\n/* o /* n */ c */\n", "a.rs");
        assert!(k.iter().any(|k| k == "r#\"a\"b\"#"), "raw: {k:?}");
        assert!(k.iter().any(|k| k == "/* o /* n */ c */"), "nested: {k:?}");
    }

    #[test]
    fn js_regex_not_division() {
        let k = kinds("let re = /ab+c/g;\nlet q = a / b;\n", "a.js");
        assert!(k.iter().any(|k| k == "/ab+c/g"), "regex: {k:?}");
        assert!(
            !k.iter().any(|k| k.contains('/') && k != "/ab+c/g"),
            "div plain: {k:?}"
        );
    }

    #[test]
    fn escapes_do_not_end_strings_early() {
        let k = kinds("let s = \"a\\\"b\";\n", "a.js");
        assert!(k.iter().any(|k| k == "\"a\\\"b\""), "escaped: {k:?}");
    }

    #[test]
    fn brackets_color_by_depth() {
        let src = "a (b (c))";
        let spans = highlight_spans(Path::new("a.rs"), src).unwrap();
        let at = |byte: usize| spans.iter().find(|s| s.start == byte).map(|s| s.color);
        assert_eq!(at(2), span_color("bracket1"));
        assert_eq!(at(5), span_color("bracket2"));
        assert_eq!(at(7), span_color("bracket2"));
        assert_eq!(at(8), span_color("bracket1"));
    }

    #[test]
    fn unmatched_closer_glows_as_error() {
        let spans = highlight_spans(Path::new("a.rs"), "(]").unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[1].color, span_color("bracket_error").unwrap());
    }

    #[test]
    fn brackets_inside_strings_stay_strings() {
        let spans = highlight_spans(Path::new("a.js"), "\"(\"").unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].color, span_color("string").unwrap());
    }

    #[test]
    fn oversize_falls_back_to_plain() {
        let big = "x".repeat(MAX_HL_BYTES + 1);
        assert!(highlight_spans(Path::new("a.rs"), &big).is_none());
    }

    #[test]
    fn cache_recomputes_only_on_change() {
        let mut c = HlCache::default();
        let p = Path::new("a.rs");
        let _ = c.spans_for(p, "fn f() {}").unwrap();
        let ptr = c.spans_for(p, "fn f() {}").unwrap().as_ptr();
        assert_eq!(c.spans_for(p, "fn f() {}").unwrap().as_ptr(), ptr);
        c.mark_dirty();
        let fresh = highlight_spans(p, "fn g() {}").unwrap().len();
        assert_eq!(c.spans_for(p, "fn g() {}").unwrap().len(), fresh);
    }
}
