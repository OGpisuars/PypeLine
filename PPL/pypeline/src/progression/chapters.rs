//! Engineering Manual content: chapters and their contracts.
//!
//! Chapters are Markdown files in assets/data/manual and contracts are RON
//! files in assets/data/contracts. They are built into the game (so the
//! Windows .exe is a single file); editing a file and rebuilding is all it
//! takes to change content. Loading content at run time comes later.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::factory::items::ItemKind;
use crate::scripting::concepts::Concept;

/// What a contract asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Goal {
    /// Make this many items (counted from when the contract was accepted).
    Produce { item: ItemKind, count: u64 },
    /// Earn this many coins from the train.
    Earn { coins: u64 },
    /// Make this many items per minute (over the last minute of game time).
    Rate { item: ItemKind, per_minute: u64 },
}

impl Goal {
    pub fn describe(&self) -> String {
        match self {
            Self::Produce { item, count } => format!("Make {count} {}s", item.name()),
            Self::Earn { coins } => format!("Earn {coins} coins from the train"),
            Self::Rate { item, per_minute } => {
                format!(
                    "Make {per_minute} {}s a minute (see Stats, F4)",
                    item.name()
                )
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Contract {
    pub id: String,
    pub title: String,
    pub brief: String,
    pub goal: Goal,
    /// Python ideas the script must use.
    #[serde(default)]
    pub requires: Vec<Concept>,
    /// Most lines of code allowed (blank lines and comments are free).
    #[serde(default)]
    pub max_lines: Option<usize>,
    pub reward: u64,
    /// From a gentle nudge to nearly the answer.
    #[serde(default)]
    pub hints: Vec<String>,
    /// A script that beats this contract. CI runs it (roadmap Part 4 E).
    pub solution: String,
    /// Other files the solution imports, by module name ("lines" for
    /// lines.py).
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Passing this contract completes the whole chapter, so experienced
    /// coders can skip ahead by passing it directly.
    #[serde(default)]
    pub chapter_test: bool,
}

/// One piece of a chapter's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(String),
    Text(String),
    /// A Python example. `snippet` is its name if it starts with
    /// `# snippet: Name`, which also puts it in the Snippets menu once the
    /// chapter is done. `file` is set if it starts with `# file: NAME.py`:
    /// it is a whole file of its own, which later examples can import.
    Code {
        code: String,
        snippet: Option<String>,
        file: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub number: u32,
    pub title: String,
    pub blocks: Vec<Block>,
    pub contracts: Vec<Contract>,
}

impl Chapter {
    pub fn snippets(&self) -> impl Iterator<Item = (&str, &str)> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Code {
                code,
                snippet: Some(name),
                ..
            } => Some((name.as_str(), code.as_str())),
            _ => None,
        })
    }
}

const SOURCES: [(u32, &str, &str); 10] = [
    (
        1,
        include_str!("../../assets/data/manual/ch01_variables.md"),
        include_str!("../../assets/data/contracts/ch01_variables.ron"),
    ),
    (
        2,
        include_str!("../../assets/data/manual/ch02_strings.md"),
        include_str!("../../assets/data/contracts/ch02_strings.ron"),
    ),
    (
        3,
        include_str!("../../assets/data/manual/ch03_for_loops.md"),
        include_str!("../../assets/data/contracts/ch03_for_loops.ron"),
    ),
    (
        4,
        include_str!("../../assets/data/manual/ch04_conditionals.md"),
        include_str!("../../assets/data/contracts/ch04_conditionals.ron"),
    ),
    (
        5,
        include_str!("../../assets/data/manual/ch05_while_loops.md"),
        include_str!("../../assets/data/contracts/ch05_while_loops.ron"),
    ),
    (
        6,
        include_str!("../../assets/data/manual/ch06_functions.md"),
        include_str!("../../assets/data/contracts/ch06_functions.ron"),
    ),
    (
        7,
        include_str!("../../assets/data/manual/ch07_lists_dicts.md"),
        include_str!("../../assets/data/contracts/ch07_lists_dicts.ron"),
    ),
    (
        8,
        include_str!("../../assets/data/manual/ch08_modules.md"),
        include_str!("../../assets/data/contracts/ch08_modules.ron"),
    ),
    (
        9,
        include_str!("../../assets/data/manual/ch09_events.md"),
        include_str!("../../assets/data/contracts/ch09_events.ron"),
    ),
    (
        10,
        include_str!("../../assets/data/manual/ch10_tick.md"),
        include_str!("../../assets/data/contracts/ch10_tick.ron"),
    ),
];

/// All chapters, parsed once. Content errors are caught by the tests, so a
/// bad file can never reach players.
pub fn chapters() -> &'static [Chapter] {
    static CHAPTERS: OnceLock<Vec<Chapter>> = OnceLock::new();
    CHAPTERS.get_or_init(|| {
        SOURCES
            .iter()
            .map(|&(number, manual, contracts)| {
                let (title, blocks) = parse_manual(manual);
                let contracts = ron::from_str(contracts).unwrap_or_else(|err| {
                    panic!("contracts for chapter {number} are broken: {err}")
                });
                Chapter {
                    number,
                    title,
                    blocks,
                    contracts,
                }
            })
            .collect()
    })
}

pub fn contract(id: &str) -> Option<(&'static Chapter, &'static Contract)> {
    chapters()
        .iter()
        .find_map(|ch| ch.contracts.iter().find(|c| c.id == id).map(|c| (ch, c)))
}

/// Split a chapter's Markdown into its title and blocks. Supports `#`/`##`
/// headings, paragraphs, `- ` bullets, simple tables, and ```python blocks.
pub fn parse_manual(text: &str) -> (String, Vec<Block>) {
    let mut title = String::new();
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<Vec<&str>> = None;

    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Text(paragraph.join("\n")));
            paragraph.clear();
        }
    };

    for line in text.lines() {
        if let Some(lines) = code.as_mut() {
            if line.trim_start().starts_with("```") {
                let code_text = lines.join("\n") + "\n";
                let marker = |prefix: &str| {
                    lines
                        .first()
                        .and_then(|first| first.strip_prefix(prefix))
                        .map(|name| name.trim().to_owned())
                };
                blocks.push(Block::Code {
                    code: code_text,
                    snippet: marker("# snippet:"),
                    file: marker("# file:"),
                });
                code = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        if line.trim_start().starts_with("```") {
            flush(&mut paragraph, &mut blocks);
            code = Some(Vec::new());
        } else if let Some(heading) = line.strip_prefix("# ") {
            flush(&mut paragraph, &mut blocks);
            title = heading.trim().to_owned();
        } else if let Some(heading) = line.strip_prefix("## ") {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading(heading.trim().to_owned()));
        } else if line.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else {
            paragraph.push(line);
        }
    }
    flush(&mut paragraph, &mut blocks);
    (title, blocks)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::factory::Factory;
    use crate::scripting::budget::DEPLOY_BUDGET;
    use crate::scripting::concepts;
    use crate::scripting::headless::Headless;
    use crate::scripting::runtime::{Program, RunOutcome, ScriptRuntime};

    /// Content validator (roadmap Part 4 E).
    #[test]
    fn content_is_valid() {
        let mut ids = BTreeSet::new();
        for (i, chapter) in chapters().iter().enumerate() {
            assert_eq!(chapter.number as usize, i + 1, "chapters must be in order");
            assert!(
                !chapter.title.is_empty(),
                "chapter {} has no title",
                chapter.number
            );
            assert!(
                !chapter.contracts.is_empty(),
                "chapter {} has no contracts",
                chapter.number
            );
            assert_eq!(
                chapter.contracts.iter().filter(|c| c.chapter_test).count(),
                1,
                "chapter {} needs exactly one chapter test",
                chapter.number
            );
            for contract in &chapter.contracts {
                assert!(
                    ids.insert(contract.id.clone()),
                    "duplicate contract id {}",
                    contract.id
                );
                assert!(!contract.hints.is_empty(), "{} has no hints", contract.id);
                assert!(contract.reward > 0, "{} pays nothing", contract.id);
            }
        }
    }

    /// Every contract ships with a solution that beats it (roadmap Part 4 E).
    #[test]
    fn golden_solutions_beat_their_contracts() {
        for chapter in chapters() {
            for contract in &chapter.contracts {
                let id = &contract.id;
                let program = Program {
                    main: contract.solution.clone(),
                    modules: contract.files.clone(),
                };
                for source in program.sources() {
                    assert!(
                        concepts::analyze(source).is_some(),
                        "{id}: a file does not parse"
                    );
                }
                let shape = concepts::analyze_program(program.sources());
                for concept in &contract.requires {
                    assert!(
                        shape.concepts.contains(concept),
                        "{id}: solution lacks {concept:?}"
                    );
                }
                if let Some(max) = contract.max_lines {
                    assert!(
                        shape.code_lines <= max,
                        "{id}: solution is {} lines",
                        shape.code_lines
                    );
                }
                let mut game = match Headless::start(&program, Factory::default()) {
                    Ok(game) => game,
                    Err(report) => panic!("{id}: {:?} {:?}", report.outcome, report.output),
                };
                // Five minutes of game time is plenty for every contract.
                let met = (0..6000).any(|_| {
                    game.step();
                    match contract.goal {
                        Goal::Produce { item, count } => game.factory.produced(item) >= count,
                        Goal::Earn { coins } => game.factory.coins >= coins,
                        Goal::Rate { item, per_minute } => {
                            game.per_minute().get(&item).copied().unwrap_or(0) >= per_minute
                        }
                    }
                });
                assert!(!game.had_errors(), "{id}: {:?}", game.output);
                assert!(
                    met,
                    "{id}: goal not reached in 5 minutes ({:?})",
                    game.factory.produced
                );
            }
        }
    }

    /// Every code example in the manual runs (roadmap Part 4 E).
    #[test]
    fn manual_examples_run() {
        let runtime = ScriptRuntime::new();
        for chapter in chapters() {
            // `# file:` examples become modules the later examples can import.
            let mut files = BTreeMap::new();
            for block in &chapter.blocks {
                if let Block::Code { code, file, .. } = block {
                    if let Some(name) = file {
                        let module = name.strip_suffix(".py").expect("file names end in .py");
                        files.insert(module.to_owned(), code.clone());
                    }
                    let program = Program {
                        main: code.clone(),
                        modules: files.clone(),
                    };
                    let report = runtime.run_program(&program, DEPLOY_BUDGET);
                    assert_eq!(
                        report.outcome,
                        RunOutcome::Finished,
                        "chapter {} example failed:\n{code}",
                        chapter.number
                    );
                }
            }
        }
    }

    #[test]
    fn parses_markdown_blocks() {
        let (title, blocks) = parse_manual(
            "# Title\n\nSome text\nmore\n\n## Part\n\n```python\n# snippet: Demo\nx = 1\n```\n",
        );
        assert_eq!(title, "Title");
        assert_eq!(
            blocks,
            vec![
                Block::Text("Some text\nmore".into()),
                Block::Heading("Part".into()),
                Block::Code {
                    code: "# snippet: Demo\nx = 1\n".into(),
                    snippet: Some("Demo".into()),
                    file: None,
                },
            ]
        );
    }
}
