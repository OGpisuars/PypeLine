//! Friendly errors (roadmap Part 4 F): turn Python's error messages into a
//! plain-language hint for beginners. The original message is still shown;
//! this adds one line under it.

/// Names a script can use without defining them.
const KNOWN_NAMES: &[&str] = &[
    // Game modules and their functions.
    "auto",
    "conveyors",
    "machines",
    "power",
    "place",
    "connect",
    // Built-ins beginners use.
    "print",
    "range",
    "len",
    "int",
    "str",
    "float",
    "list",
    "dict",
    "set",
    "tuple",
    "bool",
    "abs",
    "min",
    "max",
    "sum",
    "round",
    "sorted",
    "reversed",
    "enumerate",
    "zip",
    "isinstance",
    "True",
    "False",
    "None",
];

/// A one-line hint for `message` (like "NameError: name 'x' is not defined"),
/// or None if there is nothing useful to add.
pub fn explain(message: &str, source: &str) -> Option<String> {
    let (kind, detail) = message.split_once(": ").unwrap_or((message, ""));
    let hint = match kind {
        "NameError" => {
            let name = quoted(detail)?;
            match closest(&name, known_names(source)) {
                Some(guess) => format!("Did you mean '{guess}'? Python names must match exactly."),
                None => format!(
                    "Python does not know '{name}' yet. Check the spelling, or give it a value \
                     first ({name} = ...)."
                ),
            }
        }
        "AttributeError" => {
            let name = detail.rsplit('\'').nth(1)?;
            match closest(name, KNOWN_NAMES.iter().map(|s| s.to_string()).collect()) {
                Some(guess) => format!("Did you mean '{guess}'?"),
                None => format!("'{name}' does not exist there. Press F1 to see every command."),
            }
        }
        "SyntaxError" if detail.starts_with("expected ':'") => {
            "Lines starting with for, while, if, elif, else and def need a colon : at the end."
                .into()
        }
        "SyntaxError" if detail.starts_with("unterminated string") => {
            "A string is missing its closing quote. Start and end it with the same kind: \
             \"...\" or '...'."
                .into()
        }
        "SyntaxError"
            if detail.contains("'(' was never closed") || detail.contains("never closed") =>
        {
            "A bracket was opened but never closed. Count your ( and ) on that line.".into()
        }
        "IndentationError" if detail.starts_with("expected an indented block") => {
            "The lines inside a for, while, if or def must be indented by 4 more spaces than \
             the line with the colon."
                .into()
        }
        "IndentationError" => {
            "This line is indented but should not be. Lines in the same block must start at \
             the same column."
                .into()
        }
        "TypeError" if detail.contains("missing required argument") => {
            let arg = detail.split('\'').nth(1).unwrap_or("an argument");
            format!("The call is missing {arg}=. Press F1 to see what each command needs.")
        }
        "TypeError" if detail.contains("unexpected keyword argument") => {
            let arg = detail.split('\'').nth(1).unwrap_or("that argument");
            format!("This command has no {arg}= option. Press F1 to see what it takes.")
        }
        "TypeError" if detail.starts_with("can only concatenate str") => {
            "You can't + text and a number. Use an f-string: f\"plates: {count}\", or str(count)."
                .into()
        }
        "IndexError" => {
            "That position is past the end of the list. Lists start at 0, so the last item of \
             a 3-item list is [2]."
                .into()
        }
        "KeyError" => "That key is not in the dict. Check the spelling, or add it first.".into(),
        "ZeroDivisionError" => "You divided by zero somewhere on this line.".into(),
        "RecursionError" => {
            "A function keeps calling itself and never stops. Make sure it has a way to end.".into()
        }
        _ => return None,
    };
    Some(hint)
}

/// The first 'quoted' word in `text`.
fn quoted(text: &str) -> Option<String> {
    text.split('\'').nth(1).map(str::to_owned)
}

/// Known names plus every name written in the script.
fn known_names(source: &str) -> Vec<String> {
    let mut names: Vec<String> = KNOWN_NAMES.iter().map(|s| s.to_string()).collect();
    let mut word = String::new();
    for c in source.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else if !word.is_empty() {
            if !word.starts_with(|c: char| c.is_ascii_digit()) && !names.contains(&word) {
                names.push(word.clone());
            }
            word.clear();
        }
    }
    names
}

/// The known name closest to `name`, if it is a likely typo.
fn closest(name: &str, candidates: Vec<String>) -> Option<String> {
    let limit = if name.len() <= 4 { 1 } else { 2 };
    candidates
        .into_iter()
        .filter(|c| c != name)
        .map(|c| (edit_distance(name, &c), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Levenshtein distance between two words.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_fixes_for_typos() {
        let hint = explain("NameError: name 'conveyer' is not defined", "").unwrap();
        assert!(hint.contains("'conveyors'"), "{hint}");
        let hint = explain(
            "AttributeError: module 'auto.conveyors' has no attribute 'plce'",
            "",
        )
        .unwrap();
        assert!(hint.contains("'place'"), "{hint}");
        // Names from the player's own script count too.
        let hint = explain("NameError: name 'smelterx' is not defined", "smelter_x = 6").unwrap();
        assert!(hint.contains("'smelter_x'"), "{hint}");
    }

    #[test]
    fn explains_common_syntax_mistakes() {
        for message in [
            "SyntaxError: expected ':' (main.py, line 1)",
            "SyntaxError: unterminated string literal (detected at line 1) (main.py, line 1)",
            "IndentationError: expected an indented block after 'if' statement on line 1",
            "IndentationError: unexpected indentation (main.py, line 2)",
            "TypeError: place() missing required argument 'dir' (pos 3)",
            "TypeError: can only concatenate str (not \"int\") to str",
        ] {
            assert!(explain(message, "").is_some(), "{message}");
        }
    }

    #[test]
    fn unknown_names_get_a_general_hint() {
        let hint = explain("NameError: name 'zzzzzz' is not defined", "").unwrap();
        assert!(hint.contains("does not know"), "{hint}");
    }

    #[test]
    fn edit_distance_works() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("same", "same"), 0);
    }
}
