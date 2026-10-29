//! Bracket pairing + smart Enter (local, no deps, no processes).
//!
//! - Auto-close inserts the mate right after the cursor; typing a closer
//!   over an existing one skips instead of doubling.
//! - `'` never auto-closes (Rust lifetimes like `&'a` are not strings).
//! - Typing an opener over a selection wraps it.
//! - Enter copies the current line's indent, adds one level after `{`,
//!   `[`, `(`, `:` and splits `{}` onto three lines.
//! - Everything is gated by settings; all persistence is the existing
//!   `~/.arctis.json` (no new storage).

/// Skip auto-edit for huge buffers (diff + clones per keystroke).
pub const MAX_EDIT_BYTES: usize = 512 * 1024;

/// Auto-inserted mate, if any. `'` deliberately absent (lifetimes).
pub fn pair_for(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '`' => Some('`'),
        _ => None,
    }
}

/// Characters that skip over an identical follower instead of doubling.
pub fn is_closer(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}' | '"' | '`')
}

/// The opener a `skip-over` requires immediately before the cursor.
/// Brackets demand their own opener (`f(|)` skips, `f()|` inserts —
/// never block a literal `)`); quotes skip on a plain match so strings
/// still close with a single keystroke.
fn matching_opener(ch: char) -> Option<char> {
    match ch {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        '"' => Some('"'),
        '`' => Some('`'),
        _ => None,
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// `true` when the text before the keystroke holds an unterminated
/// string of `quote` on the current line (so the keystroke closes it).
fn inside_string(before: &str, quote: char) -> bool {
    let line = before.rsplit('\n').next().unwrap_or(before);
    let mut open = false;
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == quote {
            open = !open;
        }
    }
    open
}

/// Byte range the edit replaced: `(prefix, inserted, deleted)`.
/// `None` only when the contents are identical.
fn diff_replacement<'a>(old: &'a str, new: &'a str) -> Option<(usize, &'a str, &'a str)> {
    if old == new {
        return None;
    }
    let (ob, nb) = (old.as_bytes(), new.as_bytes());
    let mut pre = 0;
    while pre < ob.len() && pre < nb.len() && ob[pre] == nb[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < ob.len() - pre
        && suf < nb.len() - pre
        && ob[ob.len() - 1 - suf] == nb[nb.len() - 1 - suf]
    {
        suf += 1;
    }
    Some((
        pre,
        new.get(pre..nb.len() - suf)?,
        old.get(pre..ob.len() - suf)?,
    ))
}

/// Decide the auto-edit: `Some((content, cursor_char_index))`.
/// `cursor_char` is the post-keystroke cursor from the widget state —
/// diffs alone misplace identical adjacent characters (`))`).
/// `None` means "plain edit, nothing to add".
pub fn plan(
    old: &str,
    new: &str,
    cursor_char: usize,
    auto_close: bool,
    smart_enter: bool,
) -> Option<(String, usize)> {
    if old == new || old.len() > MAX_EDIT_BYTES || new.len() > MAX_EDIT_BYTES {
        return None;
    }
    let (pre, inserted, deleted) = diff_replacement(old, new)?;
    let mut chars = inserted.chars();
    let (ch, single) = match (chars.next(), chars.next()) {
        (Some(c), None) => (c, true),
        _ => return None,
    };
    let _ = single;

    // Typing an opener over a selection wraps it: `(sel)` — never for `'`.
    if auto_close && !deleted.is_empty() {
        if let Some(close) = pair_for(ch) {
            let tail = pre + deleted.len();
            let mut out = String::with_capacity(new.len() + 2);
            out.push_str(&old[..pre]);
            out.push(ch);
            out.push_str(deleted);
            out.push(close);
            out.push_str(&old[tail..]);
            let cursor = old[..pre].chars().count() + 1 + deleted.chars().count();
            return Some((out, cursor));
        }
        return None;
    }
    if !deleted.is_empty() {
        return None;
    }

    // Pin the typed character to the real cursor, not the diff guess.
    let cursor_byte = char_to_byte(new, cursor_char);
    let start_byte = cursor_byte.checked_sub(inserted.len())?;
    if new.get(start_byte..cursor_byte) != Some(inserted) {
        return None;
    }

    // Skip over an identical closer instead of doubling it — but only
    // when it still belongs to us (its opener is right before the
    // keystroke). Otherwise the keystroke stays: `:()` + `)` is `:())`.
    if auto_close && is_closer(ch) && new[cursor_byte..].starts_with(ch) {
        let ours = match matching_opener(ch) {
            Some(open) if open == '"' || open == '`' => true,
            Some(open) => new[..start_byte].ends_with(open),
            None => false,
        };
        if ours {
            let mut out = String::with_capacity(new.len() - 1);
            out.push_str(&new[..start_byte]);
            out.push_str(&new[cursor_byte..]);
            return Some((out, cursor_char - 1));
        }
        // Not ours: keep the typed character (fall through to None).
        return None;
    }
    // Auto-close the opener right after the cursor — except quotes
    // mid-word (`don't` must stay single) and quotes that close an
    // open string (`"ab|` + `"` is a closer, not a new pair).
    if auto_close {
        if let Some(close) = pair_for(ch) {
            let quoted = ch == '"' || ch == '`';
            let next_word = new[cursor_byte..]
                .chars()
                .next()
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);
            let pair_it = if !quoted {
                true
            } else if next_word {
                false
            } else {
                !inside_string(&new[..start_byte], ch)
            };
            if pair_it {
                let mut out = String::with_capacity(new.len() + 1);
                out.push_str(&new[..cursor_byte]);
                out.push(close);
                out.push_str(&new[cursor_byte..]);
                return Some((out, cursor_char));
            }
        }
    }
    // Smart Enter: copy the line indent, plus one level after an opener.
    if smart_enter && ch == '\n' {
        let line_start = new[..start_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let head = &new[line_start..start_byte];
        let base_len = head.len() - head.trim_start_matches([' ', '\t']).len();
        let base = &head[..base_len];
        let opener = head.trim_end().ends_with(['{', '[', '(']) || head.trim_end().ends_with(':');
        let unit = if base.contains('\t') { "\t" } else { "    " };
        let nest_split = opener && new[cursor_byte..].starts_with(['}', ']', ')']);
        let (indent, cursor) = if nest_split {
            let extra = format!("{base}{unit}");
            let at = cursor_char + extra.chars().count();
            (format!("{extra}\n{base}"), at)
        } else if opener {
            let extra = format!("{base}{unit}");
            let at = cursor_char + extra.chars().count();
            (extra, at)
        } else {
            let at = cursor_char + base.chars().count();
            (base.to_owned(), at)
        };
        let mut out = String::with_capacity(new.len() + indent.len());
        out.push_str(&new[..cursor_byte]);
        out.push_str(&indent);
        out.push_str(&new[cursor_byte..]);
        return Some((out, cursor));
    }
    // `'` intentionally falls through: no auto behavior at all.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_brackets() {
        assert_eq!(pair_for('('), Some(')'));
        assert_eq!(pair_for('{'), Some('}'));
        assert_eq!(pair_for('\''), None);
        assert!(is_closer(')'));
        assert!(!is_closer('('));
    }

    #[test]
    fn typing_opener_inserts_mate() {
        assert_eq!(plan("ab", "ab(", 3, true, true), Some(("ab()".into(), 3)));
        // Quotes mid-word stay single.
        assert_eq!(plan("ab", "a\"b", 2, true, true), None);
        // Quotes at end of word still pair.
        assert_eq!(
            plan("ab", "ab\"", 3, true, true),
            Some(("ab\"\"".into(), 3))
        );
        // A cursor that disagrees with the diff means "not a keystroke".
        assert_eq!(plan("ab", "ab(", 0, true, true), None);
    }

    #[test]
    fn typing_closer_skips_over_only_its_own_pair() {
        // `f(|)` + `)` skips to the end.
        assert_eq!(plan("f()", "f())", 3, true, true), Some(("f()".into(), 2)));
        // `:()` + `)` at the end inserts literally: `:())` is reachable.
        assert_eq!(plan(":()", ":())", 4, true, true), None);
        // Quotes close strings with one keystroke, then stay.
        assert_eq!(plan("\"ab", "\"ab\"", 4, true, true), None);
        assert_eq!(
            plan("say ", "say \"", 5, true, true),
            Some(("say \"\"".into(), 5))
        );
    }

    #[test]
    fn typing_opener_over_selection_wraps_it() {
        assert_eq!(plan("ab", "a(", 2, true, true), Some(("a(b)".into(), 3)));
    }

    #[test]
    fn enter_copies_indent_and_opens_level() {
        assert_eq!(
            plan("fn f() {", "fn f() {\n", 9, true, true),
            Some(("fn f() {\n    ".into(), 13))
        );
        assert_eq!(
            plan("def f():", "def f():\n", 9, true, true),
            Some(("def f():\n    ".into(), 13))
        );
    }

    #[test]
    fn enter_between_braces_splits_three_lines() {
        assert_eq!(
            plan("{}", "{\n}", 2, true, true),
            Some(("{\n    \n}".into(), 6))
        );
    }

    #[test]
    fn toggles_off_mean_plain_edits() {
        assert_eq!(plan("ab", "ab(", 3, false, false), None);
        assert_eq!(plan("fn f() {", "fn f() {\n", 9, true, false), None);
    }

    #[test]
    fn deletions_and_pastes_are_plain() {
        assert_eq!(plan("abc", "ac", 1, true, true), None);
        assert_eq!(plan("a", "abcdef", 6, true, true), None);
    }
}
