//! Python syntax highlighting for the code editor, plus the error-line marker.
//!
//! A small hand-written tokenizer: good enough for coloring, and it never
//! fails (anything it does not recognize is plain text).

use std::ops::Range;

use bevy_egui::egui::{
    Color32, FontId,
    text::{LayoutJob, TextFormat},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Keyword,
    /// Built-in functions and the game modules.
    Builtin,
    Str,
    Number,
    Comment,
}

const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

const BUILTINS: &[&str] = &[
    "abs",
    "all",
    "any",
    "bool",
    "dict",
    "enumerate",
    "float",
    "int",
    "isinstance",
    "len",
    "list",
    "max",
    "min",
    "print",
    "range",
    "reversed",
    "round",
    "set",
    "sorted",
    "str",
    "sum",
    "tuple",
    "zip", // game modules
    "auto",
    "conveyors",
    "machines",
    "power",
];

/// Split Python source into colored pieces covering every byte.
pub fn tokenize(src: &str) -> Vec<(Range<usize>, Kind)> {
    let bytes = src.as_bytes();
    let mut out: Vec<(Range<usize>, Kind)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let c = bytes[i];
        let kind = if c == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            Kind::Comment
        } else if c == b'"' || c == b'\'' {
            i = scan_string(src, i);
            Kind::Str
        } else if c.is_ascii_digit() {
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'.'))
            {
                i += 1;
            }
            Kind::Number
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &src[start..i];
            let is_prefix = word.len() <= 2
                && word.chars().all(|ch| "rRbBfFuU".contains(ch))
                && matches!(bytes.get(i), Some(b'"' | b'\''));
            if is_prefix {
                i = scan_string(src, i);
                Kind::Str
            } else if KEYWORDS.contains(&word) {
                Kind::Keyword
            } else if BUILTINS.contains(&word) {
                Kind::Builtin
            } else {
                Kind::Plain
            }
        } else {
            i += char_len(src, i);
            Kind::Plain
        };
        // Merge runs of plain text into one piece.
        match out.last_mut() {
            Some((range, Kind::Plain)) if kind == Kind::Plain && range.end == start => {
                range.end = i
            }
            _ => out.push((start..i, kind)),
        }
    }
    out
}

/// Scan a string starting at the quote at `i`; returns the index after it.
/// Unterminated single-line strings stop at the end of the line.
fn scan_string(src: &str, mut i: usize) -> usize {
    let bytes = src.as_bytes();
    let quote = bytes[i];
    let triple = bytes.get(i..i + 3) == Some(&[quote, quote, quote][..]);
    i += if triple { 3 } else { 1 };
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                if i < bytes.len() {
                    i += char_len(src, i);
                }
            }
            b if b == quote && !triple => return i + 1,
            b if b == quote && bytes.get(i..i + 3) == Some(&[quote, quote, quote][..]) => {
                return i + 3;
            }
            b'\n' if !triple => return i,
            _ => i += char_len(src, i),
        }
    }
    i
}

fn char_len(src: &str, i: usize) -> usize {
    src[i..].chars().next().map_or(1, char::len_utf8)
}

fn color(kind: Kind) -> Color32 {
    match kind {
        Kind::Plain => Color32::from_rgb(40, 32, 48),
        Kind::Keyword => Color32::from_rgb(152, 48, 120),
        Kind::Builtin => Color32::from_rgb(40, 88, 168),
        Kind::Str => Color32::from_rgb(48, 120, 40),
        Kind::Number => Color32::from_rgb(184, 96, 24),
        Kind::Comment => Color32::from_rgb(128, 120, 104),
    }
}

const ERROR_LINE_BG: Color32 = Color32::from_rgb(255, 208, 200);

/// Byte range of 1-based line `line` (including its newline), if it exists.
fn line_range(src: &str, line: usize) -> Option<Range<usize>> {
    let mut start = 0;
    for (n, text) in src.split_inclusive('\n').enumerate() {
        if n + 1 == line {
            return Some(start..start + text.len());
        }
        start += text.len();
    }
    None
}

/// Build the colored layout for the editor.
pub fn layout(src: &str, font: FontId, error_line: Option<usize>) -> LayoutJob {
    let error = error_line.and_then(|line| line_range(src, line));
    let mut job = LayoutJob::default();
    for (range, kind) in tokenize(src) {
        // Split each piece where the error line starts and ends.
        let mut cuts = vec![range.start, range.end];
        if let Some(err) = &error {
            for at in [err.start, err.end] {
                if range.start < at && at < range.end {
                    cuts.push(at);
                }
            }
        }
        cuts.sort_unstable();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let mut format = TextFormat::simple(font.clone(), color(kind));
            if error
                .as_ref()
                .is_some_and(|err| err.start <= a && b <= err.end)
            {
                format.background = ERROR_LINE_BG;
            }
            job.append(&src[a..b], 0.0, format);
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(&str, Kind)> {
        tokenize(src)
            .into_iter()
            .filter(|(_, k)| *k != Kind::Plain)
            .map(|(r, k)| (&src[r], k))
            .collect()
    }

    #[test]
    fn colors_the_basics() {
        let src = "for x in range(3):  # loop\n    print(f\"hi {x}\", 1.5)\n";
        assert_eq!(
            kinds(src),
            vec![
                ("for", Kind::Keyword),
                ("in", Kind::Keyword),
                ("range", Kind::Builtin),
                ("3", Kind::Number),
                ("# loop", Kind::Comment),
                ("print", Kind::Builtin),
                ("f\"hi {x}\"", Kind::Str),
                ("1.5", Kind::Number),
            ]
        );
    }

    #[test]
    fn covers_every_byte_even_when_broken() {
        for src in [
            "print(\"oops)\nx = 1",
            "'''never ends",
            "s = 'é\\",
            "😀 = 1",
        ] {
            let pieces = tokenize(src);
            let mut at = 0;
            for (range, _) in pieces {
                assert_eq!(range.start, at, "{src:?}");
                at = range.end;
            }
            assert_eq!(at, src.len(), "{src:?}");
        }
    }

    #[test]
    fn error_line_is_marked() {
        let src = "a = 1\nb = oops\nc = 3\n";
        let marked = |line| {
            layout(src, FontId::monospace(12.0), line)
                .sections
                .iter()
                .filter(|s| s.format.background == ERROR_LINE_BG)
                .count()
        };
        assert_eq!(line_range(src, 2), Some(6..15));
        assert!(marked(Some(2)) > 0);
        assert_eq!(marked(None), 0);
    }
}
