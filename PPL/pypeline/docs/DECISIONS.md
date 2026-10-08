# Decision Log

The big design decisions (engine, art style, script runtime, sandbox, hot-reload, and so on) are locked in **Part 1 of the roadmap** (`pypeline_roadmap.txt` at the repo root). This file does not repeat them. It records the smaller decisions made while building, so nobody has to guess later why something is the way it is.

Add new entries at the top. Each entry says what was decided, why, and what it affects.

---

## 2026-10-08: Sandbox v1

Layers, in the order a script meets them:
1. **Token check before running** (`scripting/sandbox.rs`): any name or string containing `__` is refused with a line number, except `__name__` and `"__main__"`. Checking tokens instead of walking the AST is simpler and catches every place a dunder can hide: attributes, function names, keyword arguments, and format strings like `"{0.__class__}"`.
2. **Trimmed builtins:** `eval`, `exec`, `compile`, `open`, `input`, `getattr`, `setattr`, `delattr`, `globals`, `locals`, `vars`, `dir`, `breakpoint`, `help`, `exit`, `quit` and `memoryview` are deleted from `builtins`. Without `getattr`, a string built at run time can never become an attribute access; without `eval`/`exec`/`compile`, it can never become code.
3. **Import guard** (from Phase 0): only the game's own modules can be imported.
4. **Step budget, memory cap, watchdog**, all checked in the per-instruction hook:
   - Memory: the global allocator counts live heap bytes per thread. A run that grows its thread's heap by more than 64 MB gets `MemoryError`. Per-thread counting keeps Bevy's render and asset threads (and parallel tests) from counting against a script.
   - Watchdog: a run taking more than 1 second of real time is stopped. This is only a safety net for engine bugs. It never fires in normal play, because the step budget runs out first, and it is the one place where wall-clock time is allowed to affect a run.
   - Recursion limit: 200 nested calls, then a clean `RecursionError`.

**Known gap:** a single, enormous allocation inside one native call (`"a" * 10**10`) happens before the hook can check, and can crash the game. That only hurts the player running the script, so it is acceptable for a single-player game, but it must be solved before shared Workshop scripts (Phase 4/5). Options: patch RustPython's sequence repetition to use fallible allocation, or check sizes in the allocator and return null early.

`dir()` is removed because it lists dunder names. A player-friendly replacement can be added to the Help window if needed.

---

## 2026-10-08: Phase 1, first factory

### The simulation is plain Rust data, not ECS entities
`factory::Factory` is one Bevy resource holding `BTreeMap`s of belts and machines. `Factory::step()` advances it one tick. The renderer reads it and draws sprites; it never writes back.

**Why:** the sim can be stepped and tested with no window or Bevy app (the Phase 1 exit test runs this way), iteration order is fixed so results are deterministic, and later the state hash, saves and replays are just this struct. If large factories get slow, optimize inside `Factory` (lane segments, sleeping machines) without touching scripts or rendering.

### Coordinates
Scripts use plot coordinates: `(0, 0)` is the bottom-left buildable tile, x grows east, y grows north, and the buildable area is 16x10. Directions are `"north"`, `"east"`, `"south"` and `"west"`. Machines output east unless given `dir=`.

### Build plan, then reconcile (command queue + hot-reload)
Python calls (`conveyors.place`, `machines.place`, `power.connect`) record into a `BuildPlan` and are checked immediately, so mistakes (off the plot, two things on one tile, unknown names) raise a `ValueError` on the exact line. Only a run that finishes produces a plan, and the plan is applied on the next tick in `SimSet::Factory`.

Reconcile is **declarative**: the plan describes the whole factory. Belts are matched by tile, machines by name. Unchanged things keep their items, turned belts keep their items, and anything missing from the script is removed, with its items returned to the station inventory. A consequence worth teaching early: a script with no build calls (just `print(1)`) removes the whole factory.

A failed run (error or out of steam) halts the belts and leaves the layout untouched.

### Placeholder art is drawn in code
`engine/sprites.rs` holds 16x16 sprites as character grids (one character per pixel, colors from a small key). Belts have 8 animation frames, chosen by `factory.ticks % 8`, so the chevrons move exactly with the items and freeze when the factory halts.

### Editor font: JetBrains Mono
Chosen by the project owner instead of a pixel font, for the code editor and console. Bundled as the Nerd Font "Mono" build under the SIL Open Font License; see `ASSET_LICENSES.md`. Pixel lettering for in-world text can still come later.

### Phase 1 numbers (tune in playtests)
Belts move 1 px per tick (1.25 tiles/s) with items at least 8 px apart. A miner makes one ore every 2 s and a smelter one plate every 3 s; both hold up to 10 items in and out. Any machine connected to a steam generator is powered; generators have no fuel or capacity limit yet.

---

## 2026-10-07: Phase 0 foundation

### Pinned versions
- **Rust 1.99.0** (`rust-toolchain.toml`).
- **Bevy 0.19.1**, the latest stable release. 0.20 is still a release candidate.
- **bevy_egui 0.42**, the release that matches Bevy 0.19.
- **rustpython-vm 0.6**, with only the `compiler` feature on.

**Why:** pinned versions keep builds the same on every machine and in CI. Upgrade Bevy as a deliberate step, never by accident.

**Watch out: the `rustpython-ruff_*` packages are pinned to 0.16.5 in `Cargo.lock`.** RustPython 0.6 asks for "0.16.5 or newer", but 0.16.10 (released 2026-10-07) contains breaking changes and `rustpython-codegen` fails to compile against it. Do not run a plain `cargo update`, because it would pull 0.16.10 back in. If it happens anyway, pin the five packages again in this order (the parser first, because they require each other's exact version):

```
cargo update -p rustpython-ruff_python_parser --precise 0.16.5
cargo update -p rustpython-ruff_python_ast --precise 0.16.5
cargo update -p rustpython-ruff_python_trivia --precise 0.16.5
cargo update -p rustpython-ruff_source_file --precise 0.16.5
cargo update -p rustpython-ruff_text_size --precise 0.16.5
```

Remove this pin when a RustPython release supports the newer versions. CI uses `--locked`, so it always builds with the pinned versions.

### RustPython runs with no standard library
The interpreter is created with `Interpreter::without_stdlib`, and only the `compiler` feature is enabled. That also turns off `host_env`, which would otherwise bring in ctypes.

**Why:** the safest sandbox is one where `os`, `sys` file access, `socket` and similar modules simply do not exist. Game modules (`auto`, `power`, and so on) will be added one by one as native modules, which also gives us the import allowlist for free.

### The step budget uses `sys.settrace` opcode events, so no VM patch is needed
The roadmap marked this as the biggest technical risk. RustPython 0.6 already supports per-opcode trace events, so `scripting/hooks.rs` installs a native trace function that:
- turns on `f_trace_opcodes` for every frame it sees start, and
- charges one step of steam per bytecode instruction, raising an error once the budget is used up.

The roadmap's plan B (a custom Python-subset interpreter) is not needed for now.

**Details worth knowing:**
- Steam is counted in **bytecode instructions**, not lines. This is fairer than counting lines: one-line infinite loops still cost steam, and a long line costs more than a short one.
- The "out of steam" line number is recorded by the hook itself (from `frame.f_lineno`). RustPython leaves the innermost frame out of the traceback for errors raised by a trace function, so the traceback alone would point at the call site instead of the loop.
- Running out of steam is **sticky**. After the first failure, every later instruction fails too, so a `try/except` inside the loop cannot swallow the error.
- `sys` is built into RustPython even without the stdlib, and `sys.settrace(None)` would switch the hook off. So `builtins.__import__` is replaced with a guard that only allows modules on `ALLOWED_MODULES` (empty for now). This is an early, minimal version of the Phase 1 import allowlist. The Phase 1 AST check should also block dunder tricks that could reach `sys` without an import.
- Performance: one native call per instruction. Fine for the Phase 0 budgets (50,000 deploy steps). Measure again when `tick()` runs every tick.

### Fixed Python hash seed
`Settings.hash_seed = Some(0)`, so set iteration order is the same on every run and every OS (roadmap Part 4 C).

### `print()` goes to the in-game console
`builtins.print` is replaced. Output is sanitized: control characters, escape codes, bidi overrides and zero-width characters are removed, lines are capped at 200 characters, and a run can print at most 200 lines. The console keeps the last 500 lines.

### Scripts run on the main thread, inside the fixed tick
The RustPython interpreter is not thread-safe, so it is a Bevy non-send resource. It runs inside `FixedUpdate` in `SimSet::Scripts`, so scripts only ever run on a tick boundary.

### CI lives at the repo root
GitHub only reads workflows from `.github/workflows/` at the **repository root**, so CI is in `/.github/workflows/ci.yml` and runs inside `PPL/pypeline/`. The empty `PPL/pypeline/.github/` folder is not used by GitHub.

### Provisional palette and default font
- `engine/palette.rs` holds a placeholder palette. Every channel is a multiple of 8, so they are real GBA 15-bit colors. The real palette gets locked in `docs/ART_STYLE.md` and `assets/palettes/master.pal` during the art pass.
- The editor uses egui's default font until a pixel font with a commercial-OK license is chosen and recorded in `ASSET_LICENSES.md`.
