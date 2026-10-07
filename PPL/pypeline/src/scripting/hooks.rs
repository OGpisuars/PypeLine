//! Per-instruction VM hook.
//!
//! RustPython supports `sys.settrace` with per-opcode events, so no VM patch
//! is needed. We install a native trace function that:
//!   * on every frame "call", turns on opcode tracing for that frame and
//!     returns itself as the frame's local trace function;
//!   * on every "opcode" event, charges one step of steam and raises once the
//!     budget is gone.
//!
//! The same hook is where the Phase 1 watchdog flag and the Phase 3 line
//! debugger will plug in.

use std::rc::Rc;

use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine, builtins::PyStrRef};

use super::budget::Budget;

/// Message carried by the exception raised when steam runs out.
pub const OUT_OF_STEAM: &str = "out of steam: the script used its whole step budget";

pub fn install(vm: &VirtualMachine, budget: Rc<Budget>) -> PyResult<()> {
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
                    if budget.charge() {
                        Ok(vm.ctx.none())
                    } else {
                        // RustPython leaves this frame out of the traceback for
                        // errors raised here, so record the line ourselves.
                        let line = frame
                            .get_attr("f_lineno", vm)
                            .ok()
                            .and_then(|l| l.try_into_value::<usize>(vm).ok());
                        budget.record_stop_line(line);
                        Err(vm.new_runtime_error(OUT_OF_STEAM))
                    }
                }
                _ => Ok(vm.ctx.none()),
            }
        },
    );
    let hook: PyObjectRef = hook.into();
    vm.sys_module.get_attr("settrace", vm)?.call((hook,), vm)?;
    Ok(())
}
