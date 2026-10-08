//! What scripts write to the in-game console: `print()` lines plus the
//! `console` module (`console.clear()`, `console.color("green")`).
//!
//! Colors come from a fixed palette (roadmap: ASCII DASHBOARDS), never raw
//! escape codes, and all text is sanitized on the way in.

use std::cell::RefCell;
use std::rc::Rc;

use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine, builtins::PyStrRef};

/// Max characters kept from a single `print()` call.
const MAX_LINE_CHARS: usize = 200;
/// Max console lines one run may produce.
pub const MAX_LINES_PER_RUN: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleColor {
    Green,
    Red,
    Yellow,
    Blue,
    Orange,
    Gray,
}

impl ConsoleColor {
    pub const NAMES: [(&'static str, Self); 6] = [
        ("green", Self::Green),
        ("red", Self::Red),
        ("yellow", Self::Yellow),
        ("blue", Self::Blue),
        ("orange", Self::Orange),
        ("gray", Self::Gray),
    ];

    pub fn by_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, c)| c)
    }
}

/// One thing a script did to the console, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleOp {
    Line {
        text: String,
        color: Option<ConsoleColor>,
    },
    Clear,
}

/// Collects one run's console output.
#[derive(Debug, Default)]
pub struct ConsoleSink {
    /// Plain text of every printed line (for tests and the headless runner).
    pub lines: Vec<String>,
    pub ops: Vec<ConsoleOp>,
    pub color: Option<ConsoleColor>,
}

impl ConsoleSink {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn print(&mut self, text: &str) {
        for line in text.split('\n') {
            if self.lines.len() >= MAX_LINES_PER_RUN {
                return;
            }
            let line = sanitize(line);
            self.lines.push(line.clone());
            self.ops.push(ConsoleOp::Line {
                text: line,
                color: self.color,
            });
        }
    }
}

/// Keep console output safe: no control characters or escape codes, no
/// invisible or direction-changing characters, and a capped line length.
pub fn sanitize(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_control() && !is_invisible(*c))
        .take(MAX_LINE_CHARS)
        .collect()
}

/// Bidi overrides and zero-width characters that could disguise text.
fn is_invisible(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// Build the `console` module for one run.
pub fn console_module(
    vm: &VirtualMachine,
    sink: &Rc<RefCell<ConsoleSink>>,
) -> PyResult<Vec<(&'static str, PyObjectRef)>> {
    let clear = {
        let sink = sink.clone();
        vm.new_function("clear", move || {
            sink.borrow_mut().ops.push(ConsoleOp::Clear);
        })
    };
    let color = {
        let sink = sink.clone();
        vm.new_function(
            "color",
            move |name: Option<PyStrRef>, vm: &VirtualMachine| -> PyResult<()> {
                let color = match name.as_ref().and_then(|n| n.to_str()) {
                    None | Some("default") => None,
                    Some(name) => Some(ConsoleColor::by_name(name).ok_or_else(|| {
                        let names: Vec<&str> =
                            ConsoleColor::NAMES.iter().map(|(n, _)| *n).collect();
                        vm.new_value_error(format!(
                            "unknown color \"{name}\" (try {} or \"default\")",
                            names.join(", ")
                        ))
                    })?),
                };
                sink.borrow_mut().color = color;
                Ok(())
            },
        )
    };
    Ok(vec![("clear", clear.into()), ("color", color.into())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_are_capped_and_colored() {
        let mut sink = ConsoleSink {
            color: Some(ConsoleColor::Green),
            ..Default::default()
        };
        sink.print("a\nb");
        assert_eq!(sink.lines, vec!["a", "b"]);
        assert_eq!(
            sink.ops[0],
            ConsoleOp::Line {
                text: "a".into(),
                color: Some(ConsoleColor::Green)
            }
        );
        for _ in 0..500 {
            sink.print("x");
        }
        assert_eq!(sink.lines.len(), MAX_LINES_PER_RUN);
    }
}
