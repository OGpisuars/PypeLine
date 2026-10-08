//! The sandbox escape corpus (roadmap Part 4 B). Every script here is a known
//! way out of a Python sandbox, or a way to hurt the game. Each one must be
//! refused or stopped; none may finish normally. Add every new trick you find.

use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::runtime::{RunOutcome, ScriptRuntime};

const ESCAPES: &[&str] = &[
    // Reaching Python's internals through dunder attributes.
    "().__class__.__base__.__subclasses__()",
    "print.__self__",
    "(lambda: 0).__code__",
    "x = [].__class__",
    "'{0.__class__}'.format(1)",
    "f'{print.__self__}'",
    "import auto.conveyors as c\nc.place.__globals__",
    "from auto import __loader__",
    "__builtins__",
    // Importing anything that is not a game module.
    "import os",
    "import sys",
    "from sys import settrace",
    "import importlib",
    "import builtins",
    "import _thread",
    "__import__('os')",
    // Turning strings into code or attribute access.
    "eval('1 + 1')",
    "exec('import os')",
    "compile('1', 'x', 'eval')",
    "getattr(print, 'x')",
    "setattr(print, 'x', 1)",
    "globals()",
    "locals()",
    "vars()",
    "dir()",
    // Files, input and the debugger.
    "open('/etc/passwd')",
    "input()",
    "breakpoint()",
    // Hurting the game: endless loops, deep recursion, runaway memory.
    "while True: pass",
    "while True:\n    try:\n        pass\n    except BaseException:\n        pass",
    // Catching the stop in a caller and carrying on without a budget.
    "def spin():\n    while True:\n        pass\nwhile True:\n    try:\n        spin()\n    except:\n        pass",
    "def spin():\n    while True:\n        pass\nwhile True:\n    try:\n        spin()\n    except Exception:\n        pass",
    "def spin():\n    while True:\n        pass\ntry:\n    spin()\nfinally:\n    while True:\n        pass",
    "B = ValueError.mro()[2]\ndef spin():\n    while True:\n        pass\nwhile True:\n    try:\n        spin()\n    except B:\n        pass",
    "def f():\n    return f()\nf()",
    "s = 'a' * 1000\nwhile True:\n    s = s + s",
];

#[test]
fn every_escape_is_refused_or_stopped() {
    let runtime = ScriptRuntime::new();
    for script in ESCAPES {
        let report = runtime.run(script, DEPLOY_BUDGET);
        assert_ne!(
            report.outcome,
            RunOutcome::Finished,
            "this script must not run to the end:\n{script}"
        );
        assert!(
            report.plan.is_none(),
            "a refused script built something:\n{script}"
        );
    }
}

#[test]
fn the_runtime_still_works_after_every_escape() {
    let runtime = ScriptRuntime::new();
    for script in ESCAPES {
        runtime.run(script, DEPLOY_BUDGET);
        let report = runtime.run("print('still here')", DEPLOY_BUDGET);
        assert_eq!(report.output, vec!["still here"], "after:\n{script}");
    }
}
