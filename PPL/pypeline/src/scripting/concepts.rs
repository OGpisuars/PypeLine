//! Which Python concepts a script uses, for contracts like "make 5 plates
//! using a for loop, in under 12 lines" (roadmap Phase 3A).

use std::collections::BTreeSet;

use rustpython_vm::compiler::ast::visitor::{Visitor, walk_expr, walk_stmt};
use rustpython_vm::compiler::ast::{Expr, Stmt};
use rustpython_vm::compiler::parser;
use serde::{Deserialize, Serialize};

/// A Python idea a contract can ask for. One per manual chapter, roughly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Concept {
    /// Assigning a value to a name: `x = 3`.
    Variables,
    /// An f-string: `f"made {count} plates"`.
    FStrings,
    ForLoop,
    /// `if`, `elif` or `else`.
    Conditionals,
    WhileLoop,
    /// Defining a function with `def`.
    Functions,
    /// A list `[...]` or list comprehension.
    Lists,
    /// A dict `{key: value}`.
    Dicts,
}

impl Concept {
    /// How the concept is named to the player.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Variables => "a variable (name = value)",
            Self::FStrings => "an f-string (f\"...\")",
            Self::ForLoop => "a for loop",
            Self::Conditionals => "an if statement",
            Self::WhileLoop => "a while loop",
            Self::Functions => "a function (def)",
            Self::Lists => "a list [...]",
            Self::Dicts => "a dict {key: value}",
        }
    }
}

/// What a script uses, and how long it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptShape {
    pub concepts: BTreeSet<Concept>,
    /// Lines with code on them (blank lines and comments do not count).
    pub code_lines: usize,
}

/// Look at a script. Returns None if it does not parse.
pub fn analyze(source: &str) -> Option<ScriptShape> {
    let parsed = parser::parse_module(source).ok()?;
    let mut finder = Finder::default();
    for stmt in &parsed.syntax().body {
        finder.visit_stmt(stmt);
    }
    let code_lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count();
    Some(ScriptShape {
        concepts: finder.found,
        code_lines,
    })
}

/// Concepts used anywhere in a program, and its total code lines. Files
/// that do not parse are skipped.
pub fn analyze_program<'a>(sources: impl Iterator<Item = &'a str>) -> ScriptShape {
    let mut total = ScriptShape::default();
    for shape in sources.filter_map(analyze) {
        total.concepts.extend(shape.concepts);
        total.code_lines += shape.code_lines;
    }
    total
}

#[derive(Default)]
struct Finder {
    found: BTreeSet<Concept>,
}

impl<'a> Visitor<'a> for Finder {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let concept = match stmt {
            Stmt::Assign(_) | Stmt::AugAssign(_) | Stmt::AnnAssign(_) => Some(Concept::Variables),
            Stmt::For(_) => Some(Concept::ForLoop),
            Stmt::While(_) => Some(Concept::WhileLoop),
            Stmt::If(_) => Some(Concept::Conditionals),
            Stmt::FunctionDef(_) => Some(Concept::Functions),
            _ => None,
        };
        self.found.extend(concept);
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let concept = match expr {
            Expr::FString(_) => Some(Concept::FStrings),
            Expr::List(_) | Expr::ListComp(_) => Some(Concept::Lists),
            Expr::Dict(_) | Expr::DictComp(_) => Some(Concept::Dicts),
            Expr::If(_) => Some(Concept::Conditionals),
            _ => None,
        };
        self.found.extend(concept);
        walk_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concepts(src: &str) -> Vec<Concept> {
        analyze(src).unwrap().concepts.into_iter().collect()
    }

    #[test]
    fn finds_each_concept() {
        assert_eq!(concepts("x = 1"), vec![Concept::Variables]);
        assert_eq!(concepts("print(f'{1}')"), vec![Concept::FStrings]);
        assert_eq!(
            concepts("for i in range(3):\n    pass"),
            vec![Concept::ForLoop]
        );
        assert_eq!(concepts("if True:\n    pass"), vec![Concept::Conditionals]);
        assert_eq!(concepts("while False:\n    pass"), vec![Concept::WhileLoop]);
        assert_eq!(concepts("def f():\n    pass"), vec![Concept::Functions]);
        assert_eq!(concepts("print([1, 2])"), vec![Concept::Lists]);
        assert_eq!(concepts("print({'a': 1})"), vec![Concept::Dicts]);
    }

    #[test]
    fn looks_inside_nested_code() {
        let src = "def build():\n    for x in range(3):\n        if x > 1:\n            print(f'{x}')\nbuild()";
        assert_eq!(
            concepts(src),
            vec![
                Concept::FStrings,
                Concept::ForLoop,
                Concept::Conditionals,
                Concept::Functions
            ]
        );
    }

    #[test]
    fn counts_only_code_lines() {
        let shape = analyze("# a comment\n\nx = 1\n   \nprint(x)  # inline\n").unwrap();
        assert_eq!(shape.code_lines, 2);
    }

    #[test]
    fn broken_scripts_give_none() {
        assert_eq!(analyze("for x in"), None);
    }
}
