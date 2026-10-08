//! The player's own files: main.py imports them like modules.

use std::collections::BTreeMap;

use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::runtime::{Program, RunOutcome, RunReport, ScriptRuntime};

fn run(main: &str, modules: &[(&str, &str)]) -> RunReport {
    let program = Program {
        main: main.to_owned(),
        modules: modules
            .iter()
            .map(|(n, s)| (n.to_string(), s.to_string()))
            .collect::<BTreeMap<_, _>>(),
    };
    ScriptRuntime::new().run_program(&program, DEPLOY_BUDGET)
}

const HELPER: &str = "def double(x):\n    return x * 2\n";

#[test]
fn main_imports_a_player_file() {
    let report = run(
        "import helper\nprint(helper.double(21))",
        &[("helper", HELPER)],
    );
    assert_eq!(report.output, vec!["42"], "{:?}", report.outcome);
    let report = run(
        "from helper import double\nprint(double(4))",
        &[("helper", HELPER)],
    );
    assert_eq!(report.output, vec!["8"], "{:?}", report.outcome);
}

#[test]
fn files_can_build_factory_parts() {
    let lines = "from auto import conveyors\ndef belt(y):\n    for x in range(3):\n        conveyors.place(x=x, y=y, dir='east')\n";
    let report = run(
        "import lines\nlines.belt(0)\nlines.belt(2)",
        &[("lines", lines)],
    );
    assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
    assert_eq!(report.plan.unwrap().conveyors.len(), 6);
}

#[test]
fn errors_name_the_file_they_are_in() {
    let report = run("import broken", &[("broken", "x = 1\ny = 1 / 0")]);
    assert!(
        matches!(report.outcome, RunOutcome::Error { line: Some(2), .. }),
        "{:?}",
        report.outcome
    );
    assert_eq!(report.error_file.as_deref(), Some("broken.py"));

    let report = run("import typo", &[("typo", "for x in range(3)\n    pass")]);
    assert!(
        matches!(report.outcome, RunOutcome::Error { line: Some(1), .. }),
        "{:?}",
        report.outcome
    );
    assert_eq!(report.error_file.as_deref(), Some("typo.py"));

    let report = run("import sneaky", &[("sneaky", "x = 1\ny = ().__class__")]);
    assert!(matches!(
        report.outcome,
        RunOutcome::Error { line: Some(2), .. }
    ));
    assert_eq!(report.error_file.as_deref(), Some("sneaky.py"));

    // Errors in main.py name no file.
    let report = run("x = 1 / 0", &[]);
    assert_eq!(report.error_file, None);
}

#[test]
fn running_out_of_steam_in_a_file_names_it() {
    let report = run(
        "import spin\nspin.forever()",
        &[("spin", "def forever():\n    while True:\n        pass\n")],
    );
    assert!(
        matches!(report.outcome, RunOutcome::OutOfSteam { .. }),
        "{:?}",
        report.outcome
    );
    assert_eq!(report.error_file.as_deref(), Some("spin.py"));
}

#[test]
fn circular_imports_are_an_error_not_a_hang() {
    let report = run("import a", &[("a", "import b"), ("b", "import a")]);
    match report.outcome {
        RunOutcome::Error { message, .. } => assert!(message.contains("circle"), "{message}"),
        other => panic!("{other:?}"),
    }
}
