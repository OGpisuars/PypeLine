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

/// Names that would let a script catch the game's own stop (out of steam,
/// memory, watchdog), which derives from BaseException. `mro` is here because
/// `ValueError.mro()` lists BaseException without naming it.
const UNCATCHABLE: &[&str] = &[
    "BaseException",
    "KeyboardInterrupt",
    "SystemExit",
    "GeneratorExit",
    "BaseExceptionGroup",
    "mro",
];

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
    let tokens: Vec<_> = parsed
        .tokens()
        .iter()
        .filter(|t| t.kind() != TokenKind::Comment)
        .collect();
    let line_of = |offset: usize| source[..offset].matches('\n').count() + 1;
    for (i, token) in tokens.iter().enumerate() {
        let range = token.range();
        let start = usize::from(range.start());
        let text = &source[start..usize::from(range.end())];
        let refuse = |message: String| {
            Err(Rejection {
                line: line_of(start),
                message,
            })
        };
        match token.kind() {
            TokenKind::Except
                if tokens
                    .get(i + 1)
                    .is_some_and(|t| t.kind() == TokenKind::Colon) =>
            {
                return refuse(
                    "a bare \"except:\" is not allowed; name the error you expect, \
                     like \"except ValueError:\""
                        .into(),
                );
            }
            TokenKind::Finally => {
                return refuse("\"finally:\" is not available in PypeLine scripts yet".into());
            }
            TokenKind::Name if UNCATCHABLE.contains(&text) => {
                return refuse(format!(
                    "\"{text}\" is not available in PypeLine scripts; catch a specific error \
                     like ValueError, or Exception for any ordinary error"
                ));
            }
            _ => {}
        }
        if text.contains("__") && !ALLOWED_DUNDERS.contains(&text) {
            return refuse(format!(
                "names with double underscores (like {}) are not allowed in PypeLine scripts",
                shorten(text)
            ));
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
            ("try:\n    pass\nexcept:\n    pass", 3),
            ("try:\n    pass\nexcept BaseException:\n    pass", 3),
            ("try:\n    pass\nfinally:\n    pass", 3),
            ("b = ValueError.mro()", 1),
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
            "try:\n    x = int('a')\nexcept ValueError:\n    print('not a number')",
            "try:\n    x = 1 / 0\nexcept Exception as e:\n    print(e)",
        ] {
            assert_eq!(check(src), Ok(()), "{src}");
        }
    }
}
