//! RustPython setup and lifecycle.
//!
//! `ScriptRuntime` owns one interpreter with NO standard library (so `import
//! os` and friends simply do not exist), a fixed hash seed for determinism,
//! the step-budget hook, and a `print()` that writes into the game console
//! instead of stdout.

use std::cell::RefCell;
use std::rc::Rc;

use rustpython_vm::{
    AsObject, Interpreter, PyResult, Settings, VirtualMachine, builtins::PyBaseExceptionRef,
    compiler::Mode, function::FuncArgs,
};

use super::bindings::{self, ModuleTable};
use super::budget::Budget;
use super::commands::BuildPlan;
use super::hooks;
use super::sandbox;

/// Deepest call nesting a script may reach.
const RECURSION_LIMIT: usize = 200;
/// Max characters kept from a single `print()` call.
const MAX_LINE_CHARS: usize = 200;
/// Max console lines one run may produce.
const MAX_LINES_PER_RUN: usize = 200;

/// How a script run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The script finished normally.
    Finished,
    /// The script raised an error or failed to compile.
    Error {
        line: Option<usize>,
        message: String,
    },
    /// The step budget ran out (slow code or an infinite loop).
    OutOfSteam { line: Option<usize> },
}

/// Result of one script run: console output plus how it ended.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub output: Vec<String>,
    pub outcome: RunOutcome,
    pub steps_used: u64,
    pub steps_limit: u64,
    /// What the script asked to build. Only present if it finished, so a
    /// failed run can never change the factory.
    pub plan: Option<BuildPlan>,
}

pub struct ScriptRuntime {
    interpreter: Interpreter,
    budget: Rc<Budget>,
    output: Rc<RefCell<Vec<String>>>,
    plan: Rc<RefCell<BuildPlan>>,
    modules: Rc<RefCell<ModuleTable>>,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let mut settings = Settings::default();
        // Fixed hash seed: set/dict-of-hash order must be identical every run
        // and on every OS (roadmap Part 4 C).
        settings.hash_seed = Some(0);
        let interpreter = Interpreter::without_stdlib(settings);

        let budget = Rc::new(Budget::default());
        let output = Rc::new(RefCell::new(Vec::new()));
        let modules = Rc::new(RefCell::new(ModuleTable::new()));

        interpreter.enter(|vm| {
            install_print(vm, output.clone()).expect("failed to install print()");
            install_import_guard(vm, modules.clone()).expect("failed to install the import guard");
            sandbox::trim_builtins(vm).expect("failed to trim builtins");
            // Deep recursion gets a clean RecursionError long before the
            // game's own stack is at risk.
            vm.recursion_limit.set(RECURSION_LIMIT);
            hooks::install(vm, budget.clone()).expect("failed to install the step hook");
        });

        Self {
            interpreter,
            budget,
            output,
            plan: Rc::new(RefCell::new(BuildPlan::default())),
            modules,
        }
    }

    /// Compile and run `source` as a fresh module with `step_limit` steps.
    pub fn run(&self, source: &str, step_limit: u64) -> RunReport {
        self.output.borrow_mut().clear();
        self.budget.reset(step_limit);
        *self.plan.borrow_mut() = BuildPlan::default();

        if let Err(rejection) = sandbox::check(source) {
            return RunReport {
                output: Vec::new(),
                plan: None,
                outcome: RunOutcome::Error {
                    line: Some(rejection.line),
                    message: rejection.message,
                },
                steps_used: 0,
                steps_limit: step_limit,
            };
        }

        let outcome = self.interpreter.enter(|vm| {
            match bindings::build_modules(vm, &self.plan) {
                Ok(table) => *self.modules.borrow_mut() = table,
                Err(exc) => return error_outcome(vm, &exc),
            }
            let code = match vm.compile(source, Mode::Exec, "main.py") {
                Ok(code) => code,
                Err(err) => {
                    let exc = err.into_pyexception(vm, Some(source));
                    return error_outcome(vm, &exc);
                }
            };
            let scope = vm.new_scope_with_builtins();
            if let Err(exc) =
                scope
                    .globals
                    .set_item("__name__", vm.ctx.new_str("__main__").into(), vm)
            {
                return error_outcome(vm, &exc);
            }
            match vm.run_code_obj(code, scope) {
                Ok(_) => RunOutcome::Finished,
                Err(exc) if self.budget.is_exhausted() => RunOutcome::OutOfSteam {
                    line: self.budget.stop_line().or_else(|| deepest_line(&exc)),
                },
                Err(exc) => error_outcome(vm, &exc),
            }
        });

        let plan = (outcome == RunOutcome::Finished).then(|| self.plan.take());
        RunReport {
            output: std::mem::take(&mut *self.output.borrow_mut()),
            plan,
            outcome,
            steps_used: self.budget.used(),
            steps_limit: self.budget.limit(),
        }
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Replace `builtins.print` with one that appends to the console buffer.
fn install_print(vm: &VirtualMachine, output: Rc<RefCell<Vec<String>>>) -> PyResult<()> {
    let print = vm.new_function(
        "print",
        move |args: FuncArgs, vm: &VirtualMachine| -> PyResult<()> {
            let sep = match args.kwargs.get("sep") {
                Some(sep) if !vm.is_none(sep) => sep.str(vm)?.to_string(),
                _ => " ".to_owned(),
            };
            let mut parts = Vec::with_capacity(args.args.len());
            for arg in &args.args {
                parts.push(arg.str(vm)?.to_string());
            }
            let mut out = output.borrow_mut();
            for line in parts.join(&sep).split('\n') {
                if out.len() >= MAX_LINES_PER_RUN {
                    break;
                }
                out.push(sanitize(line));
            }
            Ok(())
        },
    );
    vm.builtins.set_attr("print", print, vm)
}

/// Replace `builtins.__import__` so scripts can only import the game modules.
///
/// Even without the stdlib, RustPython has built-in modules like `sys`, and
/// `sys.settrace(None)` would switch off the step budget. Every `import`
/// statement goes through `builtins.__import__`, so this closes that door.
fn install_import_guard(vm: &VirtualMachine, modules: Rc<RefCell<ModuleTable>>) -> PyResult<()> {
    let guard = vm.new_function(
        "__import__",
        move |args: FuncArgs, vm: &VirtualMachine| -> PyResult {
            let Some(name) = args.args.first() else {
                return Err(vm.new_type_error("__import__() missing module name"));
            };
            let name = name.str(vm)?;
            // Names that are not valid UTF-8 fall through as "" and are refused.
            let full = name.to_str().unwrap_or_default().to_owned();
            let top_level = full.split('.').next().unwrap_or_default().to_owned();
            // `from a.b import c` wants the module a.b itself; `import a.b`
            // binds the top-level module a.
            let fromlist = args
                .args
                .get(3)
                .or_else(|| args.kwargs.get("fromlist"))
                .map(|list| list.clone().try_to_bool(vm))
                .transpose()?
                .unwrap_or(false);
            let modules = modules.borrow();
            match (modules.get(&full), modules.get(&top_level)) {
                (Some(module), _) if fromlist => Ok(module.clone()),
                (Some(_), Some(top)) => Ok(top.clone()),
                _ => Err(vm.new_import_error(
                    format!("module '{full}' is not available in PypeLine"),
                    name,
                )),
            }
        },
    );
    vm.builtins.set_attr("__import__", guard, vm)
}

/// Keep console output safe: no control characters or escape codes, and a
/// capped line length (roadmap: ASCII DASHBOARDS / sandbox output cap).
fn sanitize(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_control() && !is_invisible(*c))
        .take(MAX_LINE_CHARS)
        .collect()
}

/// Bidi overrides and zero-width characters that could disguise text.
fn is_invisible(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

fn error_outcome(vm: &VirtualMachine, exc: &PyBaseExceptionRef) -> RunOutcome {
    let kind = exc.class().name().to_string();
    let detail = exc
        .as_object()
        .str(vm)
        .map(|s| s.to_string())
        .unwrap_or_default();
    let message = if detail.is_empty() {
        kind
    } else {
        format!("{kind}: {detail}")
    };
    // Syntax errors carry their line as an attribute, not a traceback.
    let line = deepest_line(exc).or_else(|| {
        exc.as_object()
            .get_attr("lineno", vm)
            .ok()
            .and_then(|l| l.try_into_value::<usize>(vm).ok())
    });
    RunOutcome::Error { line, message }
}

/// Line number of the innermost traceback entry (where the error happened).
fn deepest_line(exc: &PyBaseExceptionRef) -> Option<usize> {
    let mut tb = exc.traceback()?;
    loop {
        let next = tb.next.lock().clone();
        match next {
            Some(n) => tb = n,
            None => return Some(tb.lineno.get()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::budget::DEPLOY_BUDGET;

    fn run(source: &str) -> RunReport {
        ScriptRuntime::new().run(source, DEPLOY_BUDGET)
    }

    #[test]
    fn print_goes_to_console() {
        let report = run("print('hello')\nprint(1, 2, sep='-')");
        assert_eq!(report.outcome, RunOutcome::Finished);
        assert_eq!(report.output, vec!["hello", "1-2"]);
    }

    #[test]
    fn infinite_loop_runs_out_of_steam() {
        let report = run("x = 0\nwhile True:\n    x += 1");
        assert!(matches!(report.outcome, RunOutcome::OutOfSteam { .. }));
        assert_eq!(report.steps_used, DEPLOY_BUDGET);
    }

    #[test]
    fn one_line_infinite_loop_runs_out_of_steam() {
        let report = run("while True: pass");
        assert!(matches!(report.outcome, RunOutcome::OutOfSteam { .. }));
    }

    #[test]
    fn try_except_cannot_swallow_the_budget() {
        let src = "while True:\n    try:\n        pass\n    except BaseException:\n        pass";
        let report = run(src);
        assert!(matches!(report.outcome, RunOutcome::OutOfSteam { .. }));
    }

    #[test]
    fn functions_are_charged_too() {
        let src = "def spin():\n    while True:\n        pass\nspin()";
        let report = run(src);
        // The loop's lines (2-3), not the `spin()` call site on line 4.
        assert!(
            matches!(report.outcome, RunOutcome::OutOfSteam { line: Some(2 | 3) }),
            "{:?}",
            report.outcome
        );
    }

    #[test]
    fn runtime_error_reports_line() {
        let report = run("a = 1\nb = a / 0");
        match report.outcome {
            RunOutcome::Error { line, message } => {
                assert_eq!(line, Some(2));
                assert!(message.starts_with("ZeroDivisionError"), "{message}");
            }
            other => panic!("unexpected outcome {other:?}"),
        }
    }

    #[test]
    fn syntax_error_reports_line() {
        let report = run("x = 1\nfor i in range(3)\n    print(i)");
        match report.outcome {
            RunOutcome::Error { line, message } => {
                assert_eq!(line, Some(2));
                assert!(message.starts_with("SyntaxError"), "{message}");
            }
            other => panic!("unexpected outcome {other:?}"),
        }
    }

    #[test]
    fn no_stdlib_to_escape_into() {
        let report = run("import os");
        assert!(matches!(report.outcome, RunOutcome::Error { .. }));
    }

    fn error_message(src: &str) -> String {
        match run(src).outcome {
            RunOutcome::Error { message, .. } => message,
            other => panic!("expected an error for {src:?}, got {other:?}"),
        }
    }

    #[test]
    fn dangerous_builtins_are_gone() {
        for src in [
            "eval('1')",
            "exec('x = 1')",
            "getattr(1, 'real')",
            "globals()",
            "open('x')",
        ] {
            assert!(error_message(src).starts_with("NameError"), "{src}");
        }
    }

    #[test]
    fn dunder_code_is_refused_before_running() {
        let report = run("print('ran')\nx = ().__class__");
        assert!(report.output.is_empty(), "nothing may run");
        assert!(matches!(
            report.outcome,
            RunOutcome::Error { line: Some(2), .. }
        ));
    }

    #[test]
    fn main_guard_works() {
        let report = run("if __name__ == '__main__':\n    print('main')");
        assert_eq!(report.output, vec!["main"]);
    }

    #[test]
    fn deep_recursion_is_a_clean_error() {
        assert!(
            error_message("def f(n):\n    return f(n + 1)\nf(0)").starts_with("RecursionError")
        );
    }

    #[test]
    fn memory_growth_is_capped() {
        let msg = error_message("s = 'a' * 1000\nwhile True:\n    s = s + s");
        assert!(msg.starts_with("MemoryError"), "{msg}");
    }

    #[test]
    fn sys_cannot_switch_off_the_budget() {
        for src in [
            "import sys\nsys.settrace(None)\nwhile True: pass",
            "from sys import settrace\nsettrace(None)\nwhile True: pass",
        ] {
            match run(src).outcome {
                RunOutcome::Error { line, message } => {
                    assert_eq!(line, Some(1));
                    assert!(message.starts_with("ImportError"), "{message}");
                }
                other => panic!("unexpected outcome {other:?} for {src:?}"),
            }
        }
    }

    #[test]
    fn output_is_sanitized() {
        let report = run("print('\\x1b[31mred\\x1b[0m')");
        assert_eq!(report.output, vec!["[31mred[0m"]);
    }

    #[test]
    fn runtime_is_reusable_between_runs() {
        let rt = ScriptRuntime::new();
        assert!(matches!(
            rt.run("while True: pass", 1_000).outcome,
            RunOutcome::OutOfSteam { .. }
        ));
        let report = rt.run("print('again')", 1_000);
        assert_eq!(report.outcome, RunOutcome::Finished);
        assert_eq!(report.output, vec!["again"]);
    }
}
