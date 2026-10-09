# Changelog

All notable changes to PypeLine. The game is pre-alpha: every push to `main` builds new [downloads for Windows, macOS and Linux](https://github.com/OGpisuars/PypeLine/releases/tag/latest), and there are no numbered releases yet.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Startup
- Animated KiloKilo Games logo, drawn in code, with its own jingle.
- PypeLine loading screen with a LOADING bar and a retro startup chime. It always plays in full (keys and clicks no longer skip it), and its starry sky fills the whole window at any size.
- Title menu with Play, Settings and Quit. The factory waits until Play; View > Title screen goes back.
- Game icon on the window and built into `PypeLine.exe`.
- Background music, "Joystick Sunday", with track and volume in Settings.

### Downloads
- macOS (one app for Apple Silicon and Intel) and Linux downloads next to the Windows one, built on every push.

### Playing
- **Bigger islands** in the Shop: 20 x 12 (1500 coins), 24 x 14 (4000) and 28 x 15 (9000), each after the last. The plot grows east and north, so (0, 0) stays the bottom-left tile and every script keeps working; the island stays centered in the sky.
- **Cheat sheet** (📋 in the top bar): every import, like `from auto import conveyors, machines, splitters`, and everything inside each module on one page, with a button to insert the imports into `main.py`.
- **Crafters:** a machine with three input sides and one output that turns plates into iron gears and pipes, and gears, pipes and plates into engines. Click a crafter on the island to pick its recipe, or use `recipe="iron_gear"` in `machines.place`. Gears, pipes and engines sell for more than what goes into them.
- **Achievements** (🏆 in the top bar): goals like "Make 25 iron plates" or "Build a splitter". Three of them unlock the crafter recipes.
- Dragging a code window keeps up with the mouse at every zoom (it used to lag behind when zoomed out).
- **Splitters:** `splitters.place(x, y, dir1, dir2)` takes items from any side and sends them out in turn, half toward `dir1` and half toward `dir2`. If one side is full, the other gets everything until there is room again.
- Code windows are part of the world, like in The Farmer Was Replaced: they pan and zoom with the island and stay where you park them, even far off screen. The view can be dragged as far as you like (Home comes back).
- **+ Window** in the top bar makes a new file in its own code window, where you are looking. Brass cables link each file to the files it imports.
- Pause menu (Esc, or ☰ Menu in the top bar): Resume, Settings, Title screen and Quit. The factory pauses while it is open.
- A new HUD card: coins with the last train's pay, ore and plates made, the contract's progress bar, and a PAUSED / HALTED / speed badge.
- Shop prices are about four times higher, so upgrades are goals to save up for.

### Fixed
- The title no longer draws over the Settings window.
- A finished contract can never be taken or paid again. The game saves the moment a contract completes, and saves now keep the script, so a contract reached right after loading completes without pressing Run again.

### Phase 3B: tools for bigger factories
- **Line debugger (F6):** record a run of `main.py`, then step through it forwards and backwards with your variables shown.
- **Stats (F4):** items and coins per minute, steam use, uptime, and every machine's state. Bottlenecks blink on the island; `stats.bottlenecks()` gives scripts the same list.
- **Day and night:** a four-minute day. Boilers run hotter by day and a big one overheats around noon unless `tick()` eases off.
- Chapters 8–10: imports and modules, events and sensors, `tick()` and generators, with rate and keep-cool contracts.
- `tick()`, `on_train(coins)`, `on_contract_complete(title)`, and the `sensors`, `stats` and `clock` modules.

### Phase 3C: room to code
- Floating, resizable windows for every file and the console, over a world you can drag and zoom.
- Many files: split code into modules and `import` them from `main.py`. Errors name the file and line.
- The Shop: spend train coins on Fast/Express belts and Mk2/Mk3 miners and smelters, used with `tier=`.
- Settings: six themes with a matching mouse pointer, four fonts, text size, island bobbing.
- Editor autocomplete.

### Phase 3A: the teaching loop
- Engineering Manual (F2) with chapters 1–7, contracts that pay coins and unlock the next chapter, and chapter tests for skipping ahead.
- Friendly error hints, hints that open one at a time, and snippets that unlock as you learn.
- `console` module: colored output and clearing for ASCII dashboards.
- Help window (F1) with every command and name.

### Phase 2: it feels like PypeLine
- Floating island with drifting clouds, steam-moths, power wires with moving pulses.
- Cargo train, stations and coins. Polaroid hover cards with live stats.
- Time Dials: pause, step one tick, 1x/2x/4x. Boot splash.
- Saving and loading with autosave, rolling backups and crash-safe writes.
- Generated chiptune sound effects and music.
- Visible failures: errors halt the belts and highlight the line; boilers overheat and puff steam.

### Phases 0–1: foundation and first factory
- Bevy window with a fixed 20 Hz simulation, RustPython with a step budget ("steam").
- `auto` API: conveyors, miners, smelters, steam generators and `power.connect`. Re-running a script updates the factory to match it.
- Code editor with syntax highlighting and error-line marking.
- Sandbox: refused dunders and built-ins, an import allowlist, a memory cap and a watchdog. A headless runner and a cross-OS determinism check in CI.
- Windows build on every push to `main`.
