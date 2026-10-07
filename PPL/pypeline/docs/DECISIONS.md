# Decision Log

The big design decisions (engine, art style, script runtime, sandbox, hot-reload, and so on) are locked in **Part 1 of the roadmap** (`pypeline_roadmap.txt` at the repo root). This file does not repeat them. It records the smaller decisions made while building, so nobody has to guess later why something is the way it is.

Add new entries at the top. Each entry says what was decided, why, and what it affects.

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
