pub mod nvim;

pub use nvim::{NeovimBackend, NvimStatus};

/// Number of visual rows in `content` (no-wrap editor: one per `\n`).
pub fn line_count(content: &str) -> usize {
    content.split('\n').count()
}

/// Right-aligned line numbers, VSCode style. Returns the text and the
/// digit width used (minimum 2, so the gutter doesn't jitter).
pub fn gutter_text(line_count: usize) -> (String, usize) {
    let digits = line_count.to_string().len().max(2);
    let text = (1..=line_count)
        .map(|n| format!("{n:>width$}", width = digits))
        .collect::<Vec<_>>()
        .join("\n");
    (text, digits)
}

/// 1-based (line, column) for a char index into `content`.
pub fn line_col(content: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in content.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_lines() {
        assert_eq!(line_count(""), 1);
        assert_eq!(line_count("a"), 1);
        assert_eq!(line_count("a\n"), 2);
        assert_eq!(line_count("a\nb\nc"), 3);
    }

    #[test]
    fn gutter_aligns() {
        let (text, digits) = gutter_text(12);
        assert_eq!(digits, 2);
        let rows: Vec<_> = text.split('\n').collect();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0], " 1");
        assert_eq!(rows[11], "12");
    }

    #[test]
    fn cursor_position() {
        assert_eq!(line_col("ab\ncd", 0), (1, 1));
        assert_eq!(line_col("ab\ncd", 2), (1, 3));
        assert_eq!(line_col("ab\ncd", 3), (2, 1));
        assert_eq!(line_col("ab\ncd", 5), (2, 3));
    }
}
