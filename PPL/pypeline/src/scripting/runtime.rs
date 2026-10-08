//! RustPython setup and lifecycle.
//!
//! `ScriptRuntime` owns one interpreter with NO standard library (so `import
//! os` and friends simply do not exist), a fixed hash seed for determinism,
//! the step-budget hook, and a `print()` that writes into the game console
//! instead of stdout.

use std::cell::RefCell;
use std::rc::Rc;

use rustpython_vm::{
    AsObject, Interpreter, PyObjectRef, PyResult, Settings, VirtualMachine,
    builtins::PyBaseExceptionRef, compiler::Mode, function::FuncArgs, scope::Scope,
};

use super::bindings::{self, ModuleTable, ScriptContext};
use super::budget::Budget;
use super::commands::BuildPlan;
use super::console_api::{ConsoleOp, ConsoleSink};
use super::hooks;
use super::operate::{ApiMode, Op, WorldView};
use super::sandbox;

/// Deepest call nesting a script may reach.
const RECURSION_LIMIT: usize = 200;

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
    /// Plain text of every printed line.
    pub output: Vec<String>,
    /// Everything done to the console, in order (colored lines, clears).
    pub console: Vec<ConsoleOp>,
    pub outcome: RunOutcome,
    pub steps_used: u64,
    pub steps_limit: u64,
    /// What the script asked to build. Only present if it finished, so a
    /// failed run can never change the factory.
    pub plan: Option<BuildPlan>,
}

/// How a call to `tick()` or an event handler ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutcome {
    /// The script defines no such function (or tick() was switched off).
    NotDefined,
    Finished,
    Error {
        line: Option<usize>,
        message: String,
    },
    OutOfSteam {
        line: Option<usize>,
    },
}

/// Result of one `tick()` or event-handler call.
#[derive(Debug, Clone)]
pub struct HookReport {
    pub outcome: HookOutcome,
    pub output: Vec<String>,
    pub console: Vec<ConsoleOp>,
    /// Operations to apply on the next factory step.
    pub ops: Vec<Op>,
}

/// A value passed to an event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventArg {
    Int(i64),
    Text(String),
}

/// The finished main.py: its globals stay alive so `tick()` and event
/// handlers can use them until the next Run.
struct Session {
    scope: Scope,
    /// A running `tick()` generator (chapter 10), advanced once per tick.
    generator: Option<PyObjectRef>,
    /// tick() raised an error; it is not called again until the next Run.
    tick_broken: bool,
}

pub struct ScriptRuntime {
    interpreter: Interpreter,
    budget: Rc<Budget>,
    ctx: ScriptContext,
    modules: Rc<RefCell<ModuleTable>>,
    hook: hooks::StepHook,
    session: RefCell<Option<Session>>,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let mut settings = Settings::default();
        // Fixed hash seed: set/dict-of-hash order must be identical every run
        // and on every OS (roadmap Part 4 C).
        settings.hash_seed = Some(0);
        let interpreter = Interpreter::without_stdlib(settings);

        let budget = Rc::new(Budget::default());
        let ctx = ScriptContext::default();
        let modules = Rc::new(RefCell::new(ModuleTable::new()));

        interpreter.enter(|vm| {
            install_print(vm, ctx.console.clone()).expect("failed to install print()");
            install_import_guard(vm, modules.clone()).expect("failed to install the import guard");
            sandbox::trim_builtins(vm).expect("failed to trim builtins");
            // Deep recursion gets a clean RecursionError long before the
            // game's own stack is at risk.
            vm.recursion_limit.set(RECURSION_LIMIT);
        });
        let hook = interpreter.enter(|vm| hooks::StepHook::install(vm, budget.clone()));

        Self {
            interpreter,
            budget,
            ctx,
            modules,
            hook,
            session: RefCell::new(None),
        }
    }

    /// The factory snapshot that scripts read (stats, sensors, status).
    pub fn set_world(&self, world: WorldView) {
        *self.ctx.world.borrow_mut() = world;
    }

    /// Does the last good run define this function?
    pub fn defines(&self, name: &str) -> bool {
        let session = self.session.borrow();
        let Some(session) = session.as_ref() else {
            return false;
        };
        self.interpreter.enter(|vm| {
            session
                .scope
                .globals
                .get_item_opt(name, vm)
                .ok()
                .flatten()
                .is_some_and(|f| f.is_callable())
        })
    }

    /// Compile and run `source` as a fresh module with `step_limit` steps.
    pub fn run(&self, source: &str, step_limit: u64) -> RunReport {
        self.ctx.console.borrow_mut().reset();
        self.ctx.ops.borrow_mut().clear();
        self.ctx.mode.set(ApiMode::Build);
        self.budget.reset(step_limit);
        *self.ctx.plan.borrow_mut() = BuildPlan::default();
        // A new Run replaces the old session, even if it fails.
        *self.session.borrow_mut() = None;

        if let Err(rejection) = sandbox::check(source) {
            return RunReport {
                output: Vec::new(),
                console: Vec::new(),
                plan: None,
                outcome: RunOutcome::Error {
                    line: Some(rejection.line),
                    message: rejection.message,
                },
                steps_used: 0,
                steps_limit: step_limit,
            };
        }

        let mut finished_scope = None;
        let outcome = self.interpreter.enter(|vm| {
            if let Err(exc) = self.hook.arm(vm) {
                return error_outcome(vm, &exc);
            }
            match bindings::build_modules(vm, &self.ctx) {
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
            match vm.run_code_obj(code, scope.clone()) {
                Ok(_) => {
                    finished_scope = Some(scope);
                    RunOutcome::Finished
                }
                Err(exc) if self.budget.is_exhausted() => RunOutcome::OutOfSteam {
                    line: self.budget.stop_line().or_else(|| deepest_line(&exc)),
                },
                Err(exc) => error_outcome(vm, &exc),
            }
        });

        if let Some(scope) = finished_scope {
            *self.session.borrow_mut() = Some(Session {
                scope,
                generator: None,
                tick_broken: false,
            });
        }
        let plan = (outcome == RunOutcome::Finished).then(|| self.ctx.plan.take());
        let sink = std::mem::take(&mut *self.ctx.console.borrow_mut());
        RunReport {
            output: sink.lines,
            console: sink.ops,
            plan,
            outcome,
            steps_used: self.budget.used(),
            steps_limit: self.budget.limit(),
        }
    }

    /// Call `tick()` (or advance its generator) with `step_limit` steps.
    pub fn tick(&self, step_limit: u64) -> HookReport {
        let tick_broken = self.session.borrow().as_ref().is_none_or(|s| s.tick_broken);
        if tick_broken {
            return self.not_defined();
        }
        let report = self.invoke(step_limit, |vm, session| {
            if let Some(generator) = session.generator.clone() {
                return advance(vm, session, generator);
            }
            let Some(tick) = function(vm, session, "tick") else {
                return Ok(None);
            };
            let result = tick.call((), vm)?;
            if result.class().is(vm.ctx.types.generator_type) {
                // A generator tick(): start it now and resume it every tick.
                session.generator = Some(result.clone());
                return advance(vm, session, result);
            }
            Ok(Some(()))
        });
        if matches!(report.outcome, HookOutcome::Error { .. })
            && let Some(session) = self.session.borrow_mut().as_mut()
        {
            session.tick_broken = true;
        }
        report
    }

    /// Call an event handler like `on_train(coins)` if the script defines it.
    pub fn event(&self, name: &str, args: &[EventArg], step_limit: u64) -> HookReport {
        if self.session.borrow().is_none() {
            return self.not_defined();
        }
        self.invoke(step_limit, |vm, session| {
            let Some(handler) = function(vm, session, name) else {
                return Ok(None);
            };
            let args: Vec<PyObjectRef> = args
                .iter()
                .map(|arg| match arg {
                    EventArg::Int(n) => vm.ctx.new_int(*n).into(),
                    EventArg::Text(t) => vm.ctx.new_str(t.as_str()).into(),
                })
                .collect();
            handler.call(args, vm)?;
            Ok(Some(()))
        })
    }

    fn not_defined(&self) -> HookReport {
        HookReport {
            outcome: HookOutcome::NotDefined,
            output: Vec::new(),
            console: Vec::new(),
            ops: Vec::new(),
        }
    }

    /// Run `call` in operate mode inside the session. `Ok(None)` means the
    /// function is not defined.
    fn invoke(
        &self,
        step_limit: u64,
        call: impl FnOnce(&VirtualMachine, &mut Session) -> PyResult<Option<()>>,
    ) -> HookReport {
        self.ctx.console.borrow_mut().reset();
        self.ctx.ops.borrow_mut().clear();
        self.ctx.mode.set(ApiMode::Operate);
        self.budget.reset(step_limit);
        let mut session = self.session.borrow_mut();
        let Some(session) = session.as_mut() else {
            return self.not_defined();
        };
        let outcome = self.interpreter.enter(|vm| {
            if let Err(exc) = self.hook.arm(vm) {
                return hook_error(vm, &exc);
            }
            match call(vm, session) {
                Ok(Some(())) => HookOutcome::Finished,
                Ok(None) => HookOutcome::NotDefined,
                Err(exc) if self.budget.is_exhausted() => HookOutcome::OutOfSteam {
                    line: self.budget.stop_line().or_else(|| deepest_line(&exc)),
                },
                Err(exc) => hook_error(vm, &exc),
            }
        });
        let sink = std::mem::take(&mut *self.ctx.console.borrow_mut());
        HookReport {
            outcome,
            output: sink.lines,
            console: sink.ops,
            ops: std::mem::take(&mut *self.ctx.ops.borrow_mut()),
        }
    }
}

/// A callable defined at the top level of main.py, if any.
fn function(vm: &VirtualMachine, session: &Session, name: &str) -> Option<PyObjectRef> {
    session
        .scope
        .globals
        .get_item_opt(name, vm)
        .ok()
        .flatten()
        .filter(|f| f.is_callable())
}

/// Resume a tick() generator once. When it finishes, the next tick starts
/// a fresh one.
fn advance(
    vm: &VirtualMachine,
    session: &mut Session,
    generator: PyObjectRef,
) -> PyResult<Option<()>> {
    match vm.call_method(&generator, "__next__", ()) {
        Ok(_) => Ok(Some(())),
        Err(exc) if exc.fast_isinstance(vm.ctx.exceptions.stop_iteration) => {
            session.generator = None;
            Ok(Some(()))
        }
        Err(exc) => {
            session.generator = None;
            Err(exc)
        }
    }
}

fn hook_error(vm: &VirtualMachine, exc: &PyBaseExceptionRef) -> HookOutcome {
    match error_outcome(vm, exc) {
        RunOutcome::Error { line, message } => HookOutcome::Error { line, message },
        RunOutcome::OutOfSteam { line } => HookOutcome::OutOfSteam { line },
        RunOutcome::Finished => HookOutcome::Finished,
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Replace `builtins.print` with one that appends to the console buffer.
fn install_print(vm: &VirtualMachine, output: Rc<RefCell<ConsoleSink>>) -> PyResult<()> {
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
            output.borrow_mut().print(&parts.join(&sep));
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
    fn except_exception_cannot_swallow_the_budget() {
        // The stop is not an Exception, so this handler never sees it.
        let src = "def spin():\n    while True:\n        pass\nwhile True:\n    try:\n        spin()\n    except Exception:\n        pass";
        let report = run(src);
        assert!(
            matches!(report.outcome, RunOutcome::OutOfSteam { .. }),
            "{:?}",
            report.outcome
        );
    }

    #[test]
    fn budget_still_works_after_a_stop() {
        let rt = ScriptRuntime::new();
        for _ in 0..3 {
            let report = rt.run("while True: pass", 1_000);
            assert!(matches!(report.outcome, RunOutcome::OutOfSteam { .. }));
        }
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
    fn console_module_colors_and_clears() {
        use crate::scripting::console_api::{ConsoleColor, ConsoleOp};
        let report = run("import console\nconsole.color('green')\nprint('ok')\nconsole.clear()");
        assert_eq!(
            report.console,
            vec![
                ConsoleOp::Line {
                    text: "ok".into(),
                    color: Some(ConsoleColor::Green)
                },
                ConsoleOp::Clear,
            ]
        );
        assert!(error_message("import console\nconsole.color('pink')").contains("unknown color"));
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
        assert!(msg.contains("too much memory"), "{msg}");
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
