//! Editor autocomplete for the game's API (roadmap Phase 3A).
//!
//! `suggest` looks at the text before the cursor and offers what can come
//! next: functions after `conveyors.`, directions after `dir="`, machine
//! kinds after `machines.place("`, and so on. Pure and testable; the editor
//! draws the popup and handles the keys.

use crate::factory::Dir;
use crate::factory::machines::MachineKind;
use crate::scripting::console_api::ConsoleColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Text that replaces what has been typed of the word.
    pub insert: String,
    /// Shown next to it, like a signature.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestions {
    /// How many characters before the cursor the suggestion replaces.
    pub replace: usize,
    pub items: Vec<Suggestion>,
}

fn item(insert: &str, detail: &str) -> Suggestion {
    Suggestion {
        insert: insert.to_owned(),
        detail: detail.to_owned(),
    }
}

/// What can follow `module.`.
fn members(module: &str) -> Vec<Suggestion> {
    match module {
        "auto" => vec![
            item("conveyors", "belts"),
            item("machines", "miners, smelters..."),
        ],
        "conveyors" => vec![item("place(", "x, y, dir, tier=1")],
        "machines" => vec![
            item("place(", "kind, name, x, y, dir=\"east\", ore=..., tier=1"),
            item("enable(", "name  (in tick)"),
            item("disable(", "name  (in tick)"),
            item("status(", "name -> dict"),
        ],
        "power" => vec![item("connect(", "generator, to=[names]")],
        "console" => vec![
            item("color(", "\"green\", \"red\", ... or \"default\""),
            item("clear()", "wipe the console"),
        ],
        "sensors" => vec![item("count(", "x, y -> items on that belt")],
        "stats" => vec![
            item("produced(", "\"iron_plate\" -> total made"),
            item("per_minute(", "\"iron_plate\" -> made in the last minute"),
            item("coins()", "your coins"),
        ],
        "clock" => vec![
            item("tick()", "ticks so far"),
            item("seconds()", "seconds so far"),
        ],
        _ => Vec::new(),
    }
}

fn words(list: impl IntoIterator<Item = String>, detail: &str) -> Vec<Suggestion> {
    list.into_iter().map(|w| item(&w, detail)).collect()
}

/// Suggestions for the text before the cursor on the current line, if any.
pub fn suggest(line_before_cursor: &str) -> Option<Suggestions> {
    let line = line_before_cursor;
    // The word being typed (letters, digits, _), and what comes before it.
    let word_start = line
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map_or(line.len(), |(i, _)| i);
    let (before, partial) = line.split_at(word_start);

    let options: Vec<Suggestion> = if let Some(prefix) = before.strip_suffix('.') {
        let module = prefix
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("");
        members(module)
    } else if before.ends_with("dir=\"") || before.ends_with("dir='") {
        let names = Dir::ALL.iter().map(|d| format!("{d:?}").to_lowercase());
        words(names, "direction")
    } else if before.ends_with("machines.place(\"") || before.ends_with("machines.place('") {
        words(
            MachineKind::ALL.iter().map(|k| k.name().to_owned()),
            "machine",
        )
    } else if before.ends_with("ore=\"") || before.ends_with("ore='") {
        words(["iron".to_owned()], "ore")
    } else if before.ends_with("console.color(\"") || before.ends_with("console.color('") {
        words(
            ConsoleColor::NAMES
                .iter()
                .map(|(n, _)| n.to_string())
                .chain(["default".to_owned()]),
            "color",
        )
    } else if ["produced(\"", "per_minute(\"", "produced('", "per_minute('"]
        .iter()
        .any(|end| before.ends_with(end))
    {
        words(
            crate::factory::items::ItemKind::ALL
                .iter()
                .map(|i| i.id().to_owned()),
            "item",
        )
    } else if before.trim_start() == "from auto import " {
        members("auto")
    } else if before.trim_start() == "import " {
        words(
            ["auto", "power", "console", "sensors", "stats", "clock"].map(String::from),
            "game module",
        )
    } else {
        return None;
    };

    let items: Vec<Suggestion> = options
        .into_iter()
        .filter(|s| s.insert.starts_with(partial) && s.insert != partial)
        .collect();
    (!items.is_empty()).then(|| Suggestions {
        replace: partial.chars().count(),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inserts(line: &str) -> Vec<String> {
        suggest(line)
            .map(|s| s.items.into_iter().map(|i| i.insert).collect())
            .unwrap_or_default()
    }

    #[test]
    fn completes_module_members() {
        assert_eq!(inserts("conveyors."), vec!["place("]);
        assert_eq!(inserts("    power.co"), vec!["connect("]);
        assert_eq!(inserts("auto.m"), vec!["machines"]);
        assert_eq!(suggest("power.co").unwrap().replace, 2);
    }

    #[test]
    fn completes_string_arguments() {
        assert_eq!(inserts("conveyors.place(x=1, y=0, dir=\"e"), vec!["east"]);
        assert_eq!(
            inserts("machines.place(\"s"),
            vec!["smelter", "steam_generator", "station"]
        );
        assert_eq!(
            inserts("machines.place(\"miner\", name=\"m\", x=0, y=0, ore=\""),
            vec!["iron"]
        );
        assert!(inserts("console.color(\"").contains(&"green".to_owned()));
        assert_eq!(inserts("stats.produced(\"iron_p"), vec!["iron_plate"]);
    }

    #[test]
    fn completes_imports() {
        assert_eq!(inserts("from auto import "), vec!["conveyors", "machines"]);
        assert_eq!(inserts("import p"), vec!["power"]);
        assert_eq!(inserts("import st"), vec!["stats"]);
    }

    #[test]
    fn stays_quiet_otherwise() {
        assert_eq!(suggest("x = 1"), None);
        assert_eq!(suggest("conveyors.place("), None);
        // A finished word needs no suggestion.
        assert_eq!(suggest("import power"), None);
    }
}
