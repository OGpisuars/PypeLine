# Decision Log

The big design decisions (engine, art style, script runtime, sandbox, hot-reload, and so on) are locked in **Part 1 of the roadmap** (`pypeline_roadmap.txt` at the repo root). This file does not repeat them. It records the smaller decisions made while building, so nobody has to guess later why something is the way it is.

Add new entries at the top. Each entry says what was decided, why, and what it affects.

---

## 2026-10-09: Downloads for every OS, and the web demo moves up

- The release workflow builds Windows, macOS and Linux on every push and publishes all three to the `latest` pre-release together (nothing is published unless all three build). macOS is one universal `PypeLine.app` (Apple Silicon and Intel joined with `lipo`) with an `.icns` made from `assets/icon/pypeline.png`; it is signed ad hoc until notarization (Phase 4), so the first launch needs right-click > Open. Linux is built on Ubuntu 22.04 so it runs on older distros. Everything the game needs is built into the binary, so each download is the program plus license files.
- The web (WASM) demo moves from post-launch to Phase 3D, before 1.0. The audience is beginners, teachers and classrooms; school machines often cannot install anything, and a link removes every barrier. Bevy and RustPython both target wasm32. The demo covers chapters 1-3 first.
- The README leads with a GIF of a script running and the factory snapping into place (`tools/readme_gif.sh`, using the new `PYPELINE_RECORD` and `PYPELINE_AUTORUN_AFTER` dev hooks), because that moment explains the game faster than any screenshot.

## 2026-10-08: Startup: company logo, loading screen, title menu, icon, music

- A `Screen` state (`engine/screens.rs`) runs Logo → Boot → Menu → Playing. The factory's four `SimSet`s only run while Playing, so nothing ticks behind the menu; the game's windows and keys are gated the same way. Settings opens on the menu too. View > Title screen goes back.
- The KiloKilo Games logo is drawn with egui's painter (vector shapes at window resolution), not as pixel sprites: it is the company mark, not game art, and has to stay crisp at any window size. Fades use colors mixed toward the paper instead of alpha, because the K's parts overlap and alpha would darken the overlaps.
- The logo and loading screens count at most 1/20 s per frame, so a slow first frame (window creation, shader compiles) cannot skip the animation.
- Icon: `assets/icon/pypeline.png` (the team's art, with the white outside its rounded frame made transparent). `build.rs` resizes it to `$OUT_DIR/icon.rgba` for the window icon (set through `bevy::winit::WINIT_WINDOWS`, which needs a direct `winit` dependency matching the lockfile) and, when building for Windows, writes a .ico and builds it into the .exe with `embed-resource`. The build script watches the folder, not the file, so a missing file does not rerun it every build; without the file it falls back to the pixel icon drawn in `engine/icon_art.rs`. Wayland ignores window icons; the window's app id is `pypeline` for a future .desktop file.
- Music: Joystick Sunday is an .ogg (converted from the team's mp3) built into the binary with `include_bytes!`, because the release zip ships only the .exe. Vorbis decoding was already compiled in, so no new crates. Settings picks the track (Joystick Sunday, the old chiptune loop, or off) and volume; it fades in over 2 s on the menu, and the logo and loading screens play only their own jingles.
- New jingles: "ki-lo ki-lo!" for the logo and a brighter "pa-ling!" boot chime with an echo. Still original tunes, not any console's startup sound (roadmap BOOT SPLASH).

## 2026-10-08: The line debugger records, then replays

The roadmap warned that pausing the VM mid-line and resuming it is "harder than tick stepping". The debugger avoids pausing altogether:
- A Debug Run turns on a `Recorder` that the existing VM hook feeds from RustPython's "line" trace events. Each step saves the file, line, function, the frame's variables as text (no modules, functions or classes; at most 24, 60 characters each), how much has been printed, and the steam used.
- Runs are deterministic, so stepping through the recording, forwards or backwards, shows exactly what the run did. Stepping back for free is a bonus a pausing debugger would not have.
- Recording costs no steam: the trace function runs with tracing switched off, so `repr()` calls inside it are never charged. A test checks steps_used is identical with and without recording.
- Debug Runs use their own `ScriptRuntime` (a non-send resource), read a snapshot of the real factory, and never apply their build plan, so they cannot disturb the running script's tick() or the factory.
- At most 2000 lines are recorded; the window says when it was cut short.

## 2026-10-08: Stats, day/night and boiler heat

### Stats
- `factory::stats` works out each machine's state (working, starved, blocked, no power, off, idle, overheated) from what can be seen of it, so the Stats window, `machines.status()["state"]` and `stats.bottlenecks()` always agree.
- `StatsHistory` (busy %, uptime, coins earned, tick() steam) is UI-only: recorded after each step, never saved, never hashed.
- Contracts can have a `Rate` goal ("40 plates a minute"), measured over the last minute of game time.

### Day/night and heat
- The day runs on `Factory::ticks` (4 minutes per day, starting at 6:00), so it pauses while halted and replays exactly. The tint is render-only and changes in steps, like a palette swap. The canvas tint and the sky around it are mixed in linear color, because that is how the GPU blends; mixing in sRGB left a visible rectangle.
- Heat is integer math, updated once per game second inside `Factory::step`: a generator closes 1/8 of the gap to (air + 6 per switched-on machine it powers). It overheats at 100 and restarts at 70. Air is 15 at night and 30 at noon, so 11 machines peak at 96 and never overheat; that keeps chapters 1-9 (at most 8 machines per generator) unaffected.
- Contracts can say `keep_cool: true`: an overheat restarts them. Heatwave asks for 500 plates, more than a day's worth, so it always runs through a noon; a test checks that its solution overheats without its tick().
- **Golden hash changed** to `0x1175_a7cb_540a_8474` (machines now carry `heat` and `overheated`). With those fields removed from the state text, the previous hash came out exactly.

## 2026-10-08: Workspace, Shop, Settings, and the prestige plan
Player feedback: the docked editor left no room, one file was not enough, and the world could not be moved.

### Floating windows over a world you can drag
- Every script file and the console are `egui::Window`s. A thin top bar (an `egui::Panel::top` in a background root `Ui`) holds Run, Files, Console, Snippets, Help, Manual, Shop, Settings, View and the Time Dials. `GameArea` is now everything under the top bar.
- `camera::ViewState { zoom, pan }`: zoom is a whole number (None = largest that fits), so pixels stay crisp; pan is a logical-pixel offset, clamped so a corner of the island stays on screen. Left-drag on empty space, or middle/right drag anywhere outside a window, pans; the wheel zooms around the pointer; Home resets. The screen clear color is now the sky, so the world reads as bigger than the 480x320 canvas.
- "Reset windows" bumps a number that is part of every window id, so egui forgets their positions. That is simpler than storing and forcing positions.

### Many files
- `Workspace.files[0]` is always main.py. Run sends a `runtime::Program { main, modules }`; the other files are importable by their stem. The runtime reports which file an error or a stop happened in, and `ErrorLine` holds `(file, line)`.
- File names must be importable: an identifier, not a Python keyword, not a game module (`auto`, `power`, ...), not `main`, not starting with `__`. "PPL" becomes "PPL.py"; "PPL.py" is kept. Comparison ignores case, for Windows.
- Deleting a file keeps `NAME.py.bak`.

### Settings
- `settings.ron` in the data folder: theme, font, text size, mouse pointer, island bobbing. Missing fields load as defaults (`#[serde(default)]`), so old files keep working.
- Themes are Rust constants for now (`engine/themes.rs`), each a full set of egui visuals plus syntax colors plus cursor colors. Every channel is a multiple of 8 (tested). Moving them to data files is still the Phase 4 plan.
- The mouse pointer is a generated pixel arrow (Bevy `CursorIcon::Custom`). bevy_egui only writes the cursor when egui's wanted cursor changes, so a system in `Last` swaps the plain system arrow for ours and leaves egui's text, resize and grab pointers alone. Tested: light themes get a dark arrow, dark themes a light one.
- Fonts: JetBrains Mono (default), Maple Mono, Noto Sans Mono (all OFL, bundled with their license files) and Hack (built into egui). The chosen font goes first in both families; egui's fonts stay as fallbacks for symbols.

### Shop and tiers
- Upgrades live in `Factory.unlocked`, so they are saved with the factory, survive Clean Run, and are part of the deterministic state. Buying is a player input applied outside the tick, like Run.
- An upgrade unlocks a `tier=` argument; it never changes the factory by itself. Scripts see the unlocks through `WorldView.unlocked`, so a locked tier fails on its own line during the run, with the upgrade's name and price.
- Tier 2 is 2x and tier 3 is 4x: belts move 1/2/4 px per tick (spacing stays 8), machines divide their work ticks by 1/2/4. All whole numbers, so determinism is unaffected.
- `Machine.tier` and `Conveyor.tier` default to 1 when loading older saves.
- **Golden hash changed** to `0x26b7_cf8a_40b6_8ae6` because the state's `Debug` text now includes `tier` and `unlocked`. Checked before updating it: with those fields removed from the text, the old hash `0xe2c2_f767_7099_3cee` came out exactly, so the simulation itself did not change.

### Prestige and new languages (planned, not built)
- Each prestige resets the factory, coins and upgrades, keeps Manual progress, grants a permanent bonus, and switches the scripting language, each one harder than the last: Python, Lua, JavaScript, PypeC (our C-like), Belt Assembly, then an esoteric finale.
- Rule for any language: pure Rust, embeddable, sandboxable (step budget, memory cap, watchdog) and deterministic, behind one `Language` trait with the same game API. Details: roadmap Part 1 "PRESTIGE + LANGUAGES" and Phase 4B.

## 2026-10-08: Phase 3A, the teaching loop

### Content is data, built into the game
Chapters are Markdown (`assets/data/manual/chNN_*.md`) and contracts are RON (`assets/data/contracts/chNN_*.ron`). Both are compiled in with `include_str!`, so the Windows `.exe` is still a single file. Loading them at run time (for mods, roadmap Phase 5) can come later without changing the formats.

Manual Markdown is deliberately small: `#` title, `##` headings, paragraphs, `- ` bullets, simple `|` tables, inline `code` and **bold**, and fenced Python blocks. A code block whose first line is `# snippet: Name` also becomes a snippet, unlocked when its chapter is done.

### Contracts
- **Goals:** `Produce(item, count)` counts items made since the contract was accepted; `Earn(coins)` counts coins gained since then.
- **Requirements** (`requires: [ForLoop, ...]`, `max_lines: Some(n)`) are checked against the **last script that ran to the end**, the one that built the factory. If the goal is met but a requirement is not, the player is told once and the contract waits for a good Run.
- **Concepts** are found by walking the script's AST (`scripting/concepts.rs`), not by searching text, so `for` inside a comment or string does not count.
- **Chapter tests:** each chapter has exactly one contract marked `chapter_test`. Passing it completes the chapter, and any chapter test can be taken at any time ("Test out"), which is the roadmap's skip-ahead for experienced coders. A chapter also counts as done when any later chapter is done.
- Contract progress is checked in `SimSet::Progress`, after the factory steps, so it is deterministic. Rewards are added to `factory.coins`.

### CI proves the content works (roadmap Part 4 E)
- `content_is_valid`: chapters in order, unique contract ids, exactly one chapter test per chapter, hints and rewards present.
- `golden_solutions_beat_their_contracts`: every contract's `solution` uses the required concepts, fits the line limit, and reaches the goal within 5 minutes of game time.
- `manual_examples_run`: every code block in the manual runs. Blocks must therefore be self-contained.

### Smelters stop at 10 plates
A machine's output holds 10 items. A line with no belt out of the smelter makes exactly 10 plates and stops, so contracts asking for more than 10 per smelter need a station (or more lines). The chapter 5 hint teaches this.

### Friendly errors and help
- `scripting/errors.rs` adds a one-line hint under common errors (NameError, AttributeError, missing colon or quote, indentation, wrong arguments, IndexError, KeyError and more), with "did you mean ...?" from edit distance over game names, built-ins and names in the player's own script.
- Three failed runs in a row print a pointer to the Manual's hints.
- Hints open one at a time per contract (nudge, bigger hint, nearly the answer), and the count is saved.

### Editor autocomplete
`engine/ui/autocomplete.rs` suggests from the text before the cursor: functions after `conveyors.` / `machines.` / `power.` / `console.`, directions after `dir="`, machine kinds after `machines.place("`, ores, console colors and module names after `import`. Tab or Enter accepts, the arrow keys pick, and Esc closes until the text changes.

### console module
`console.color("green")` and `console.clear()`. Colors come from a fixed set (green, red, yellow, blue, orange, gray). `print` output and console operations go through one `ConsoleSink`, so ordering is exact and the per-run line cap applies to everything.

---

## 2026-10-08: Headless mode and the golden hash

- `Factory::state_hash()` is FNV-1a over the factory's `Debug` text. The state is only integers, enums and strings in `BTreeMap`s, so the text, and therefore the hash, is identical on every OS. std's `HashMap` hasher is randomly seeded, so it is never used for this.
- `tests/determinism_tests.rs` checks that the canonical sample gives the same hash twice, and that it equals `GOLDEN_HASH`. CI runs `cargo test` on Windows, Linux and macOS, so this one constant is the cross-OS determinism check from roadmap Part 4 C. **If a deliberate simulation change moves the hash, update the constant in the same commit and say why.**
- `src/bin/headless.rs` runs any script without a window: `cargo run --bin headless -- main.py 600`. `default-run = "pypeline"` keeps plain `cargo run` starting the game.

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

**The stop must be uncatchable (found 2026-10-08).** When a trace function raises, RustPython switches tracing off, exactly like CPython. Before this fix, every run after the first "out of steam" had no budget at all, and a script could catch the stop in a caller (`try: spin()` / `except: pass` inside `while True`) and loop forever. Now:
- the hook is re-armed at the start of every run (`StepHook::arm`);
- every stop (steam, memory, watchdog) is raised as `pypeline.Stopped`, which derives from `BaseException` like `KeyboardInterrupt`, so `except Exception:` never catches it;
- the token check refuses the remaining ways to catch it or run code after it: bare `except:`, the names `BaseException`, `KeyboardInterrupt`, `SystemExit`, `GeneratorExit`, `BaseExceptionGroup` and `mro`, and `finally:`.

Normal error handling (`except ValueError:`, `except Exception as e:`) still works, which the error-handling chapter needs. `finally:` can come back once the stop no longer relies on unwinding (for example via RustPython's signal/eval-breaker path). `tests/sandbox_tests.rs` runs all known tricks back-to-back on one runtime.

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
