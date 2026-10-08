# The script sandbox

PypeLine runs Python that players write, and players share scripts. A script must never be able to read your files, run programs, use the network, or freeze the game. This page explains how that is enforced, and what is known not to be covered yet.

Found a way out? Please **do not open a public issue**. See [`SECURITY.md`](../../../SECURITY.md).

## Layer 0: there is nothing to escape into

The interpreter is RustPython created with `Interpreter::without_stdlib`, with only the `compiler` feature enabled. The modules that make sandboxes hard (`os`, `io`, `socket`, `subprocess`, `ctypes`) are simply not there. `import` only finds the game's own modules (`auto`, `power`, `console`, `sensors`, `stats`, `clock`) and the player's own files. Code: `scripting/runtime.rs`.

## Layer 1: checks before anything runs

`scripting/sandbox.rs` reads the script's tokens before it executes and refuses:

- **Any name or string containing a double underscore**, except `__name__` and `"__main__"`. Classic Python escapes all go through dunder attributes (`().__class__.__base__.__subclasses__()`), and checking strings too stops `"{0.__class__}".format(x)`.
- **Ways to catch the game's stop**: a bare `except:`, `finally:`, and the names `BaseException`, `SystemExit`, `KeyboardInterrupt`, `GeneratorExit`, `BaseExceptionGroup` and `mro`.

It also removes built-ins that turn strings into code or attribute access, or that touch the outside world: `eval`, `exec`, `compile`, `open`, `input`, `getattr`, `setattr`, `delattr`, `globals`, `locals`, `vars`, `dir`, `breakpoint`, `help`, `exit`, `quit`, `memoryview`, and a few more.

## Layer 2: limits while it runs

A native trace function (`scripting/hooks.rs`, through RustPython's `sys.settrace` opcode events) runs before every bytecode instruction and enforces three limits:

| Limit | Value | What happens |
|-------|-------|--------------|
| **Steam** (step budget) | 50,000 instructions per Run of `main.py`; 2,000 per `tick()` or event | The script stops with "out of steam". The factory keeps its last good state. |
| **Memory** | 64 MB of growth per run | `MemoryError`. The allocator counts bytes per thread, so rendering never counts against a script. |
| **Watchdog** | 1 second of real time per run | The script stops, even if it is stuck in something cheap in steam. |

The stop is raised as an exception derived from `BaseException`, so `except Exception:` cannot catch it, and layer 1 refuses every other way to catch it. The hook is installed again at the start of every run, so a script cannot switch it off. Recording a run for the debugger is free: the recorder runs with tracing switched off, and a test checks steam is identical with and without it.

## Layer 3: scripts cannot touch the world directly

Build calls only add to a plan, and `tick()` and events only return operations. The game checks every value (names, positions, directions, tiers, unlocked upgrades) when the plan is applied (`scripting/commands.rs`, `scripting/reconcile.rs`). A failed run changes nothing.

## Known gaps

- **One enormous allocation** such as `"a" * 10**10` happens inside a single native call, before the hook can look, and can still crash the game. Fixing it needs a size check in RustPython's allocation paths. This is tracked in `DECISIONS.md`.
- **Determinism, not secrecy.** Scripts can see the whole factory through the game modules. That is intended.

## Tests

`tests/sandbox_tests.rs` tries the known escapes (dunder chains, format-string attribute access, importing anything that is not a game module, removed built-ins, catching the stop, endless loops, deep recursion and runaway memory) and checks each one is refused or stopped. New escape attempts should be added there along with their fix.
