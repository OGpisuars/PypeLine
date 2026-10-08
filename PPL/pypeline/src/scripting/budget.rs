//! Steam / step budget.
//!
//! Every Python bytecode instruction costs one step of steam. The VM hook in
//! `hooks.rs` charges the budget; once it runs dry the script is stopped.

use std::cell::Cell;
use std::time::{Duration, Instant};

use super::memory;

/// One-shot budget for running main.py (the "deploy budget" in the roadmap).
pub const DEPLOY_BUDGET: u64 = 50_000;

/// Budget for a single `tick()` call. Not used until Phase 1 adds `tick()`.
pub const TICK_BUDGET: u64 = 2_000;

/// Real time a single run may take before the watchdog stops it. Only a
/// backstop for engine bugs: the step budget should always fire first.
pub const WATCHDOG: Duration = Duration::from_secs(1);

/// Step counter shared between the runtime and the VM hook.
///
/// Uses `Cell` because the interpreter and its hook live on a single thread.
#[derive(Debug, Default)]
pub struct Budget {
    limit: Cell<u64>,
    used: Cell<u64>,
    exhausted: Cell<bool>,
    stop_line: Cell<Option<usize>>,
    memory_baseline: Cell<isize>,
    started: Cell<Option<Instant>>,
}

impl Budget {
    /// Start a fresh run with `limit` steps available.
    pub fn reset(&self, limit: u64) {
        self.limit.set(limit);
        self.used.set(0);
        self.exhausted.set(false);
        self.stop_line.set(None);
        self.memory_baseline.set(memory::live_bytes());
        self.started.set(Some(Instant::now()));
    }

    /// Has this run grown the heap past the script memory cap?
    pub fn over_memory(&self) -> bool {
        memory::live_bytes() - self.memory_baseline.get() > memory::SCRIPT_MEMORY_CAP as isize
    }

    /// Has this run taken longer than the watchdog allows? Checked every
    /// 1024 steps, since reading the clock is slower than counting.
    pub fn watchdog_fired(&self) -> bool {
        self.used.get() % 1024 == 0 && self.started.get().is_some_and(|t| t.elapsed() > WATCHDOG)
    }

    /// Charge one step. Returns `false` once the budget is gone.
    ///
    /// Exhaustion is sticky: after the first failure every later charge fails
    /// too, so a script cannot `try/except` its way past the limit.
    pub fn charge(&self) -> bool {
        if self.exhausted.get() {
            return false;
        }
        let used = self.used.get() + 1;
        self.used.set(used);
        if used > self.limit.get() {
            self.exhausted.set(true);
            return false;
        }
        true
    }

    pub fn used(&self) -> u64 {
        self.used.get().min(self.limit.get())
    }

    pub fn limit(&self) -> u64 {
        self.limit.get()
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted.get()
    }

    /// Remember the script line that was running when steam ran out. Only
    /// the first call counts; later ones come from unwinding.
    pub fn record_stop_line(&self, line: Option<usize>) {
        if self.stop_line.get().is_none() {
            self.stop_line.set(line);
        }
    }

    pub fn stop_line(&self) -> Option<usize> {
        self.stop_line.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustion_is_sticky() {
        let budget = Budget::default();
        budget.reset(2);
        assert!(budget.charge());
        assert!(budget.charge());
        assert!(!budget.charge());
        assert!(!budget.charge());
        assert!(budget.is_exhausted());
        assert_eq!(budget.used(), 2);
    }
}
