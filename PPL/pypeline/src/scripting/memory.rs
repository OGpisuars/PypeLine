//! Memory cap for scripts.
//!
//! The game's global allocator counts live heap bytes PER THREAD, so other
//! threads (rendering, asset loading, parallel tests) never count against a
//! script. When a script starts, the runtime notes its thread's total; the VM
//! hook raises `MemoryError` once the script has grown it by more than
//! `SCRIPT_MEMORY_CAP`. This catches
//! growth over several steps (like doubling a string in a loop).
//!
//! Known gap: one single, enormous allocation (`"a" * 10**10`) happens inside
//! one native call before the hook can look, and can still crash the game.
//! See docs/DECISIONS.md.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// How much a single script run may grow the heap.
pub const SCRIPT_MEMORY_CAP: usize = 64 * 1024 * 1024;

thread_local! {
    // `const` and destructor-free, so using it inside the allocator never
    // allocates. Signed: memory freed on another thread can push it below 0.
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}

fn add(bytes: usize) {
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get().wrapping_add(bytes as isize)));
}

fn sub(bytes: usize) {
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get().wrapping_sub(bytes as isize)));
}

/// The system allocator, plus a running count of live bytes.
pub struct CountingAlloc;

// SAFETY: every call forwards to `System` unchanged; we only update a counter.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        sub(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            add(new_size);
            sub(layout.size());
        }
        new
    }
}

/// Net bytes allocated by the current thread.
pub fn live_bytes() -> isize {
    LIVE_BYTES.try_with(Cell::get).unwrap_or(0)
}
