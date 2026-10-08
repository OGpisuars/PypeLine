//! Recording a run line by line, for the line debugger (roadmap Phase 3B).
//!
//! The VM hook (`hooks.rs`) already sees a "line" event every time a new
//! line starts. While a `Recorder` is switched on, each of those events
//! saves where the script is, its variables, and how much it has printed.
//! Runs are deterministic, so replaying the recording step by step (and
//! backwards) shows exactly what the run did, without pausing the VM.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustpython_vm::{
    AsObject, PyObjectRef, PyResult, VirtualMachine,
    builtins::{PyList, PyTuple},
};

use super::console_api::ConsoleSink;

/// Lines recorded per run, at most.
pub const MAX_STEPS: usize = 2_000;
/// Variables shown per step, at most.
const MAX_VARS: usize = 24;
/// Longest value shown, in characters.
const MAX_VALUE_CHARS: usize = 60;

/// One line about to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep {
    /// "main.py" or one of the player's files.
    pub file: String,
    pub line: usize,
    /// "<module>" at the top level, or the function's name.
    pub function: String,
    /// The variables in that function (or the file's top level), as text.
    pub vars: Vec<(String, String)>,
    /// How many lines the script had printed by then.
    pub printed: usize,
    /// Steam used by then.
    pub steam: u64,
}

/// Records line events while switched on.
#[derive(Default)]
pub struct Recorder {
    on: Cell<bool>,
    steps: RefCell<Vec<TraceStep>>,
    truncated: Cell<bool>,
    console: RefCell<Option<Rc<RefCell<ConsoleSink>>>>,
}

impl Recorder {
    pub fn new(console: Rc<RefCell<ConsoleSink>>) -> Self {
        Self {
            console: RefCell::new(Some(console)),
            ..Default::default()
        }
    }

    pub fn start(&self) {
        self.steps.borrow_mut().clear();
        self.truncated.set(false);
        self.on.set(true);
    }

    /// Stop recording and hand over what was recorded, and whether it was
    /// cut short at `MAX_STEPS`.
    pub fn finish(&self) -> (Vec<TraceStep>, bool) {
        self.on.set(false);
        (
            std::mem::take(&mut *self.steps.borrow_mut()),
            self.truncated.get(),
        )
    }

    pub fn is_on(&self) -> bool {
        self.on.get()
    }

    /// Called by the hook on each "line" event.
    pub fn record(&self, frame: &PyObjectRef, steam: u64, vm: &VirtualMachine) {
        if self.steps.borrow().len() >= MAX_STEPS {
            self.truncated.set(true);
            return;
        }
        let step = TraceStep {
            file: code_attr(frame, "co_filename", vm).unwrap_or_default(),
            line: frame
                .get_attr("f_lineno", vm)
                .ok()
                .and_then(|l| l.try_into_value::<usize>(vm).ok())
                .unwrap_or(0),
            function: code_attr(frame, "co_name", vm).unwrap_or_default(),
            vars: variables(frame, vm).unwrap_or_default(),
            printed: self
                .console
                .borrow()
                .as_ref()
                .map_or(0, |c| c.borrow().lines.len()),
            steam,
        };
        self.steps.borrow_mut().push(step);
    }
}

fn code_attr(frame: &PyObjectRef, name: &'static str, vm: &VirtualMachine) -> Option<String> {
    frame
        .get_attr("f_code", vm)
        .and_then(|code| code.get_attr(name, vm))
        .and_then(|value| value.str(vm))
        .ok()
        .map(|s| s.to_string())
}

/// The frame's variables worth showing: not modules, functions or classes,
/// and not names starting with `_`.
fn variables(frame: &PyObjectRef, vm: &VirtualMachine) -> PyResult<Vec<(String, String)>> {
    let locals = frame.get_attr("f_locals", vm)?;
    let items = vm.call_method(&locals, "items", ())?;
    let list = vm.ctx.types.list_type.as_object().call((items,), vm)?;
    let Some(list) = list.downcast_ref::<PyList>() else {
        return Ok(Vec::new());
    };
    let mut vars = Vec::new();
    for item in list.borrow_vec().iter() {
        let Some(pair) = item.downcast_ref::<PyTuple>() else {
            continue;
        };
        let [key, value] = pair.as_slice() else {
            continue;
        };
        let name = key.str(vm)?.to_string();
        let kind = value.class().name().to_string();
        let hidden = matches!(
            kind.as_str(),
            "module" | "function" | "builtin_function_or_method" | "type" | "method"
        );
        if name.starts_with('_') || hidden {
            continue;
        }
        let mut text = value.repr(vm)?.to_string();
        if text.chars().count() > MAX_VALUE_CHARS {
            text = text.chars().take(MAX_VALUE_CHARS - 1).collect::<String>() + "…";
        }
        vars.push((name, text));
        if vars.len() == MAX_VARS {
            break;
        }
    }
    vars.sort();
    Ok(vars)
}
