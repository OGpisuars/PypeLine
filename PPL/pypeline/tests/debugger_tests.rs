//! The line debugger's recording (scripting/trace.rs).

use std::collections::BTreeMap;

use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::runtime::{Program, RunOutcome, ScriptRuntime};

const SCRIPT: &str =
    "total = 0\nfor i in range(3):\n    total = total + i\n    print(total)\nname = 'done'\n";

#[test]
fn records_each_line_with_its_variables() {
    let rt = ScriptRuntime::new();
    let (report, steps, truncated) = rt.debug_program(&Program::main_only(SCRIPT), DEPLOY_BUDGET);
    assert_eq!(report.outcome, RunOutcome::Finished);
    assert!(!truncated);
    let lines: Vec<usize> = steps.iter().map(|s| s.line).collect();
    assert_eq!(lines, vec![1, 2, 3, 4, 2, 3, 4, 2, 3, 4, 2, 5]);
    assert!(
        steps
            .iter()
            .all(|s| s.file == "main.py" && s.function == "<module>")
    );
    // Before line 3 runs the second time, total is still 0 and i is 1.
    let vars = &steps[5].vars;
    assert!(vars.contains(&("i".into(), "1".into())), "{vars:?}");
    assert!(vars.contains(&("total".into(), "0".into())), "{vars:?}");
    // Output so far is counted, so the debugger can show it step by step.
    assert_eq!(steps[4].printed, 1);
    assert_eq!(steps.last().unwrap().printed, 3);
}

#[test]
fn recording_costs_no_steam_and_switches_off() {
    let rt = ScriptRuntime::new();
    let plain = rt.run(SCRIPT, DEPLOY_BUDGET).steps_used;
    let (report, _, _) = rt.debug_program(&Program::main_only(SCRIPT), DEPLOY_BUDGET);
    assert_eq!(report.steps_used, plain);
    // Ordinary runs after a debug run still work the same.
    assert_eq!(rt.run(SCRIPT, DEPLOY_BUDGET).steps_used, plain);
}

#[test]
fn follows_calls_into_functions_and_files() {
    let program = Program {
        main: "import helper\nx = helper.double(4)\n".into(),
        modules: BTreeMap::from([(
            "helper".into(),
            "def double(n):\n    result = n * 2\n    return result\n".into(),
        )]),
    };
    let rt = ScriptRuntime::new();
    let (_, steps, _) = rt.debug_program(&program, DEPLOY_BUDGET);
    let inside: Vec<_> = steps.iter().filter(|s| s.function == "double").collect();
    assert_eq!(inside.len(), 2, "{steps:?}");
    assert_eq!(inside[0].file, "helper.py");
    assert!(inside[1].vars.contains(&("result".into(), "8".into())));
    // Functions and modules are not shown as variables.
    let last = steps.last().unwrap();
    assert!(
        last.vars.iter().all(|(name, _)| name != "helper"),
        "{:?}",
        last.vars
    );
}

#[test]
fn long_runs_are_cut_short() {
    let rt = ScriptRuntime::new();
    let (report, steps, truncated) = rt.debug_program(
        &Program::main_only("n = 0\nwhile n < 1200:\n    n = n + 1\n"),
        DEPLOY_BUDGET,
    );
    assert_eq!(report.outcome, RunOutcome::Finished);
    assert!(truncated);
    assert_eq!(steps.len(), pypeline::scripting::trace::MAX_STEPS);
}
