# Architecture

How PypeLine is put together, for anyone reading or changing the code. The big design decisions are in Part 1 of the roadmap (`pypeline_roadmap.txt` at the repo root). The smaller ones, with their reasons, are in [`DECISIONS.md`](DECISIONS.md).

## The short version

PypeLine is a [Bevy](https://bevyengine.org) app. The factory is plain Rust data that steps forward 20 times a second, the same way on every computer. Player scripts run in an embedded [RustPython](https://rustpython.github.io) interpreter with no standard library. A script never touches the factory directly: it returns a **plan** (what should exist) or a list of **operations** (switch this machine off), and the game applies them between ticks.

```text
           Run (Ctrl+Enter)                         every tick (20 Hz)
                 │                                        │
   sandbox check (sandbox.rs)                  Clock ─► Scripts ─► Factory ─► Progress
                 │                                         │          │          │
   main.py runs once (runtime.rs)              tick() and events   step()    contracts,
   build calls fill a BuildPlan                return Ops          belts,    stats
                 │                             (operate.rs)        machines,
   reconcile::apply(factory, plan)                    │            power,
   adds, updates, removes to match                    └─► apply_ops  heat, train
```

## Folders

| Path | What lives there |
|------|------------------|
| `src/main.rs` | Opens the window and adds `PypelinePlugin`. |
| `src/lib.rs` | `PypelinePlugin`: every plugin below, in order. |
| `src/factory/` | The simulation: `Factory`, belts, machines, power, the train, the Shop's tiers, stats, day/night and boiler heat. No rendering, no Python. |
| `src/scripting/` | Python: the interpreter (`runtime.rs`), the game modules (`bindings.rs`, `operate.rs`, `console_api.rs`), the sandbox (`sandbox.rs`, `hooks.rs`, `budget.rs`, `memory.rs`), turning plans into factory changes (`commands.rs`, `reconcile.rs`), friendly errors, the debugger's recorder (`trace.rs`) and a windowless runner (`headless.rs`). |
| `src/progression/` | Manual chapters and contracts (loaded from `assets/data`), contract progress and rewards, and saving. |
| `src/engine/` | Everything you see: the pixel canvas and camera, sprites drawn in code, wires, failure effects, wildlife, the day/night tint, the startup screens, and the egui UI (`engine/ui/`). |
| `src/audio/` | A tiny chiptune synthesizer for every sound effect, plus the background music. |
| `src/bin/headless.rs` | `cargo run --bin headless -- main.py 600`: runs a script without a window and prints what was made and the state hash. |
| `assets/` | Fonts, the icon, music, and the Manual's chapters and contracts (Markdown and RON). All of it is built into the binary. |
| `tests/` | Integration tests: determinism, the sandbox, contracts, modules, tick(), the Shop and the debugger. |

## The simulation (`src/factory`)

- **Deterministic.** `Factory` keeps everything in `BTreeMap`s keyed by name or position, so iteration order never depends on hashing. All math is integers. `Factory::step()` is one tick.
- **Fixed tick.** Bevy's `FixedUpdate` runs at 20 Hz, separate from the frame rate. The work of a tick is split into four ordered sets, `SimSet::Clock → Scripts → Factory → Progress`. They only run while the player is on the `Playing` screen and the Time Dials are not paused.
- **Provably the same everywhere.** `Factory::state_hash()` is an FNV-1a hash of the factory's `Debug` text. `tests/determinism_tests.rs` runs a fixed script for 12,000 ticks and checks the hash on Windows, Linux and macOS in CI. If a change alters the hash on purpose, `DECISIONS.md` records the new value and how it was checked.

## Scripts (`src/scripting`)

**Build: `main.py`, once per Run.**
1. `sandbox::check` refuses dangerous code before anything runs (see [`SANDBOX.md`](SANDBOX.md)).
2. `ScriptRuntime::run` executes the program: `main.py` plus the player's other files, which it can import.
3. Build calls such as `conveyors.place` and `machines.place` do not change the factory. They add to a `BuildPlan`.
4. If the run succeeded, `reconcile::apply` changes the factory to match the plan: new things are added, changed things updated in place, and anything no longer in the script removed. Running the same script twice changes nothing, so pressing Run is always safe.
5. If the run failed, nothing changes, and the console and the code window point at the file and line.

**Operate: `tick()` and events, every tick.**
- The runtime keeps the module from the last good Run. Each tick it calls `tick()` (and handlers like `on_train(coins)`) with a small step budget.
- They read the world through a `WorldView` snapshot (machine states, belt contents, stats, the clock) and can only return `Op`s, such as switching a machine on or off. `operate::apply_ops` applies those in order.
- If `tick()` uses `yield`, the game creates the generator once and advances it one step per tick.

## Progression (`src/progression`)

- Each Manual chapter is a Markdown file plus a RON file of contracts, built in with `include_str!`. See [`CONTENT_GUIDE.md`](CONTENT_GUIDE.md).
- `contracts.rs` tracks the accepted contract's goal, checks that the script uses the concepts it asks for (`scripting::concepts` reads the script's syntax tree), and pays the reward.
- `saves.rs` writes the factory as RON every minute and on quit, with atomic writes and five rolling backups. Scripts and settings are saved separately (`scripting/files.rs`, `engine/ui/settings.rs`). Everything lives in the OS data folder under `PypeLine/`.

## Screens and rendering (`src/engine`)

- **Screens.** `screens::Screen` runs `Logo → Boot → Menu → Playing`: the KiloKilo Games logo (`company_logo.rs`), the loading screen (`splash.rs`), the title menu (`ui/main_menu.rs`) and the game.
- **Pixel canvas.** The world is drawn into a 480x320 image (2x the GBA's screen), then scaled to the window by a whole number so pixels stay sharp (`camera.rs`). The player can drag and zoom it.
- **Art in code.** Sprites are small text grids turned into images (`sprites.rs`), so there are no image files to load yet. `renderer.rs` keeps sprites in step with the `Factory`.
- **UI.** The editor, console, Manual, Help, Shop, Stats, Debugger and Settings are egui windows drawn at full window resolution over the canvas (`engine/ui/`). The theme, font and mouse pointer come from Settings.

## Tests and CI

- `cargo test` runs unit tests in each module and the integration tests in `tests/`.
- Content is tested too: every Manual example must run, every contract's `solution` must beat its contract, and every Help example must work. `docs/API.md` is generated from the in-game Help, and a test fails if it is out of date.
- CI (`.github/workflows/ci.yml`) builds and tests on Windows, Linux and macOS, and runs `cargo fmt --check` and `cargo clippy -D warnings`. `release.yml` builds Windows, macOS and Linux downloads on every push to `main` and publishes them as the [latest build](https://github.com/OGpisuars/PypeLine/releases/tag/latest).
