//! Sandbox layer 1: checks run BEFORE a script executes, plus the trimmed
//! set of built-in functions (roadmap Part 1 SANDBOX STRATEGY, Part 4 B).
//!
//! The static check works on the token stream: any name or string containing
//! a double underscore is refused. Classic Python sandbox escapes all go
//! through dunder attributes (`().__class__.__base__.__subclasses__()` and
//! friends), and checking strings too stops `"{0.__class__}".format(x)`.
//! Beginners' `if __name__ == "__main__":` is still allowed.

use ruff_text_size::Ranged;
use rustpython_vm::{
    AsObject, PyResult, VirtualMachine,
    compiler::{ast::token::TokenKind, parser},
};

/// Dunder spellings scripts may use.
const ALLOWED_DUNDERS: &[&str] = &["__name__", "\"__main__\"", "'__main__'"];

/// Built-in functions removed from every script's reach. Without `getattr`
/// a string can never become an attribute access, and without `eval`/`exec`/
/// `compile` a string can never become code.
const REMOVED_BUILTINS: &[&str] = &[
    "eval",
    "exec",
    "compile",
    "open",
    "input",
    "breakpoint",
    "globals",
    "locals",
    "vars",
    "dir",
    "getattr",
    "setattr",
    "delattr",
    "help",
    "exit",
    "quit",
    "memoryview",
    "copyright",
    "credits",
    "license",
];

/// A problem found before running: the 1-based line and a friendly message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub line: usize,
    pub message: String,
}

/// Check `source` before it runs. Code that does not parse is let through so
/// the compiler can report the syntax error the usual way.
pub fn check(source: &str) -> Result<(), Rejection> {
    let Ok(parsed) = parser::parse_module(source) else {
        return Ok(());
    };
    for token in parsed.tokens() {
        if token.kind() == TokenKind::Comment {
            continue;
        }
        let range = token.range();
        let text = &source[usize::from(range.start())..usize::from(range.end())];
        if text.contains("__") && !ALLOWED_DUNDERS.contains(&text) {
            let line = source[..usize::from(range.start())].matches('\n').count() + 1;
            return Err(Rejection {
                line,
                message: format!(
                    "names with double underscores (like {}) are not allowed in PypeLine scripts",
                    shorten(text)
                ),
            });
        }
    }
    Ok(())
}

fn shorten(text: &str) -> String {
    let mut short: String = text.chars().take(30).collect();
    if short.len() < text.len() {
        short.push('…');
    }
    short
}

/// Take the dangerous built-in functions away from scripts.
pub fn trim_builtins(vm: &VirtualMachine) -> PyResult<()> {
    for name in REMOVED_BUILTINS {
        // Some of these do not exist without the stdlib; that is fine.
        let _ = vm.builtins.as_object().del_attr(*name, vm);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dunder_tricks_are_refused_with_their_line() {
        for (src, line) in [
            ("x = 1\ny = x.__class__", 2),
            ("print(().__class__.__base__.__subclasses__())", 1),
            ("s = '{0.__class__}'.format(1)", 1),
            ("t = f\"{x.__dict__}\"", 1),
            ("def __init__(self):\n    pass", 1),
            ("x = __builtins__", 1),
        ] {
            let err = check(src).expect_err(src);
            assert_eq!(err.line, line, "{src}");
        }
    }

    #[test]
    fn ordinary_code_passes() {
        for src in [
            "if __name__ == \"__main__\":\n    print(1)",
            "# a comment with __dunder__ is fine\nx = 1_000",
            "name_with_one_underscore = 2",
            "def broken(:",
        ] {
            assert_eq!(check(src), Ok(()), "{src}");
        }
    }
}
