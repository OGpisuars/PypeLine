//! Per-instruction VM hook.
//!
//! RustPython supports `sys.settrace` with per-opcode events, so no VM patch
//! is needed. We install a native trace function that:
//!   * on every frame "call", turns on opcode tracing for that frame and
//!     returns itself as the frame's local trace function;
//!   * on every "opcode" event, checks the memory cap and the watchdog, charges
//!     one step of steam, and stops the script once the budget is gone;
//!   * on every "line" event, lets the debugger's `Recorder` note the line
//!     (only while it is recording).
//!
//! IMPORTANT: when a trace function raises, RustPython (like CPython) switches
//! tracing OFF. Two rules keep the budget unbreakable anyway:
//!   1. `arm` re-installs the hook at the start of every run.
//!   2. The stop is raised as `Stopped`, which derives from BaseException
//!      (like KeyboardInterrupt), so `except Exception` cannot catch it, and
//!      the sandbox check refuses every other way to catch it (bare `except:`,
//!      `except BaseException`, `finally:`). The script can only end.

use std::rc::Rc;

use rustpython_vm::{
    PyObjectRef, PyResult, VirtualMachine,
    builtins::{PyStrRef, PyTypeRef},
};

use super::budget::Budget;
use super::trace::Recorder;

/// Message carried by the exception raised when steam runs out.
pub const OUT_OF_STEAM: &str = "out of steam: the script used its whole step budget";
pub const TOO_MUCH_MEMORY: &str = "the script is using too much memory";
pub const TOO_SLOW: &str =
    "the script took too long and was stopped (this is a safety net; please report it)";

/// The installed hook, re-armed before every run.
pub struct StepHook {
    hook: PyObjectRef,
}

impl StepHook {
    pub fn install(vm: &VirtualMachine, budget: Rc<Budget>, recorder: Rc<Recorder>) -> Self {
        let stopped: PyTypeRef = vm.ctx.new_exception_type(
            "pypeline",
            "Stopped",
            Some(vec![vm.ctx.exceptions.base_exception_type.to_owned()]),
        );
        let hook = vm.new_function(
            "pypeline_step_hook",
            move |frame: PyObjectRef,
                  event: PyStrRef,
                  _arg: PyObjectRef,
                  vm: &VirtualMachine|
                  -> PyResult {
                match event.to_str() {
                    Some("call") => {
                        frame.set_attr("f_trace_opcodes", vm.ctx.new_bool(true), vm)?;
                        // Returning the trace function keeps tracing this frame.
                        Ok(vm.trace_func.borrow().clone())
                    }
                    Some("opcode") => {
                        let stop = |message: &str| {
                            Err(vm.new_exception_msg(stopped.clone(), message.into()))
                        };
                        if budget.over_memory() {
                            stop(TOO_MUCH_MEMORY)
                        } else if budget.watchdog_fired() {
                            stop(TOO_SLOW)
                        } else if budget.charge() {
                            Ok(vm.ctx.none())
                        } else {
                            // RustPython leaves this frame out of the traceback for
                            // errors raised here, so record the line ourselves.
                            let line = frame
                                .get_attr("f_lineno", vm)
                                .ok()
                                .and_then(|l| l.try_into_value::<usize>(vm).ok());
                            let file = frame
                                .get_attr("f_code", vm)
                                .and_then(|code| code.get_attr("co_filename", vm))
                                .and_then(|name| name.str(vm))
                                .ok()
                                .map(|name| name.to_string());
                            budget.record_stop_line(line, file);
                            stop(OUT_OF_STEAM)
                        }
                    }
                    Some("line") => {
                        if recorder.is_on() {
                            recorder.record(&frame, budget.used(), vm);
                        }
                        Ok(vm.ctx.none())
                    }
                    _ => Ok(vm.ctx.none()),
                }
            },
        );
        Self { hook: hook.into() }
    }

    /// Turn the hook on. Must run before every script run, because a stop
    /// in the previous run switched tracing off.
    pub fn arm(&self, vm: &VirtualMachine) -> PyResult<()> {
        vm.sys_module
            .get_attr("settrace", vm)?
            .call((self.hook.clone(),), vm)?;
        Ok(())
    }
}
