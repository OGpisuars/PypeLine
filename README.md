<div align="center">

# 🚂 PypeLine

**Write real Python. Build mega-factories. Master software engineering.**

A cozy 16-bit, GBA-style industrial automation game where your factory runs on code you write yourself.

![Status](https://img.shields.io/badge/status-pre--alpha-orange)
![Rust](https://img.shields.io/badge/built%20with-Rust-b7410e)
![Engine](https://img.shields.io/badge/engine-Bevy-232326)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![License](https://img.shields.io/badge/license-MIT-green)

<!-- Replace with a real GIF once you have one -->
<!-- ![PypeLine gameplay](docs/images/gameplay.gif) -->

</div>

---

## 📖 About

PypeLine blends the code-driven automation of *Factorio* and *The Farmer Was Replaced* with the warm, colorful look of classic handheld RPG overworlds.

You live on a floating industrial island. You don't place every machine by hand. Instead you write Python scripts that deploy equipment, wire terminals together, and manage steam power. Sell your output to the cargo train, unlock new chapters in your Engineering Manual, and use what you learn (loops, functions, imports) to build bigger and smarter factories.

> **Project status:** PypeLine is in early development. Features below marked **(planned)** are designed but not built yet. See the [Roadmap](#-roadmap).

---

## 🕹️ A Taste of the Code

```python
from auto import conveyors, machines
import power

# A steam generator and an iron miner
machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")

# Lay a conveyor line heading east into a smelter
for x in range(1, 5):
    conveyors.place(x=x, y=0, dir="east")
machines.place("smelter", name="smelter_1", x=5, y=0)

# Power the miner and smelter
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

Hit the **Run** arrow and your factory comes to life. Change the script and run it again: your factory floor keeps its state, and the engine updates only what changed.

---

## ⬇️ Try It

**Windows:** download `PypeLine-windows.zip` from the [latest build](https://github.com/OGpisuars/PypeLine/releases/tag/latest), unzip it, and run `PypeLine.exe`. It is rebuilt automatically from every change to `main`.

**Linux and macOS:** build from source (see [Getting Started](#-getting-started)).

### Controls

| Key or button | What it does |
|---------------|--------------|
| **▶ Run** | Run `main.py` and update the factory to match it |
| **F1** or **? Help** | Every command explained, with examples you can insert |
| Mouse over the island | Shows the tile's `x` and `y`, plus a polaroid card for machines and belts |
| **Space** | Pause / play |
| **.** (period) | Step one tick while paused |
| **1 / 2 / 3** | Speed 1x / 2x / 4x |
| **M** | Sound on / off |
| **◀** | Hide the code panel (small windows) |

Your script is saved automatically as you type, and the factory every minute and when you quit.

---

## ✨ Features

### Playable now
- **Real Python scripting.** Write actual Python in `main.py`. No made-up language.
- **Hot-reload on Run.** Edit and re-run without wiping your factory. The script describes the whole factory, so re-running never duplicates machines, and a script with an error changes nothing.
- **Belts, miners, smelters, steam power.** Build with `conveyors.place`, `machines.place` and `power.connect`. Brass wires with moving pulses show what powers what.
- **The cargo train.** Send goods to a station; every 30 seconds the train buys them for coins.
- **Visible failures.** An error halts the belts and highlights the broken line. An infinite loop runs out of steam and the boiler overheats, puffing steam.
- **Code has a cost.** Scripts run on a steam budget, so an infinite loop can never freeze the game.
- **Polaroid UI.** Hover over a machine or belt for a snapshot with live stats.
- **Time Dials.** Pause, play, 1x/2x/4x, or step one tick at a time.
- **Never lose your work.** Autosave, rolling backups and crash-safe writes for both your script and your factory.
- **Floating island, chiptune music and steam-moths.** The island bobs, clouds drift by, and moths circle warm boilers (they scatter when one overheats).

### Coming next
- **Engineering Manual and campaign.** Contracts that teach one concept at a time: variables, strings, `for` and `while` loops, conditionals, functions, lists and dicts, imports, events, and generators.
- **Line debugger.** Step through your script one line at a time and watch your variables.
- **Factory stats.** A dashboard and a `stats` module for items per minute, steam use, and bottlenecks.
- **Friendly errors and snippets.** Python errors translated into helpful hints; starter templates per chapter.

### Mid and late game (planned)
- **Day/night and thermal management.** Boilers run hotter by day and cooler by night. Read the clock and plan around it in code.
- **Cartridges.** Split your code into modules, shown in-game as physical carts you slot into terminals.
- **Micro-chips.** Tiny chips on sorters and valves run fast, local micro-scripts: edge vs. central computing.
- **LED matrix panels.** Control 8x8 and 16x16 pixel displays with `display.set_pixel(x, y, "green")`.
- **ASCII dashboards.** Print progress bars and custom success banners to the in-game console.
- **Terminal themes.** Unlockable border skins for your editor and UI.

---

## 🎨 Art Style

A **16-bit, GBA-inspired look**: top-down 3/4 view, 16x16 tiles, limited palettes, flat shading with clean outlines, and cozy dialogue-box UI. The game renders at **480x320** (exactly 2x the GBA's 240x160) and scales by whole-number factors so every pixel stays razor sharp. All art is original.

---

## 🛠️ Tech Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Language | **Rust** | Fast, safe, great for simulation |
| Engine | **Bevy** (ECS) | Factories are thousands of similar entities |
| Scripting | **RustPython** | Embeddable, cappable, deterministic |
| Code editor UI | **bevy_egui** | Text input and scrolling out of the box, styled to match the game |
| Content | **RON / TOML / Markdown** | Chapters and contracts are data, no recompiling |

**Design principles**
- **Deterministic simulation.** A fixed 20 Hz tick, separate from rendering. Same script plus same seed always gives the same result, which makes saves, replays, and fair scoring possible.
- **Sandboxed scripts.** A static AST check, an import allowlist, a per-tick step budget, and a watchdog. Scripts cannot touch your files or system.
- **Never lose your work.** Autosave, rolling backups, and atomic writes.

---

## 🗺️ Roadmap

| Phase | Goal | Highlights |
|-------|------|------------|
| **0. Foundation** | Window, loop, Python runs | Bevy window, fixed tick, RustPython, egui editor prototype, 3-OS CI |
| **1. MVP** | First automated factory | `auto` API, belts, miner, smelter, generator, editor, error highlighting |
| **2. Alpha** | It feels like PypeLine | Floating plots, polaroid UI, wires, cargo train, save/load, Time Dials, boot splash |
| **3. Beta** | The learning loop works | Manual chapters, contracts, debugger, stats, snippets, day/night, cartridges |
| **4. 1.0** | Shippable | Sandbox mode, blueprints, micro-chips, LED panels, themes, accessibility, polish |
| **5. Post-launch** | Community | Workshop sharing, leaderboards, web demo, more chapters |

Each phase has an exit test that must pass before the next one begins.

---

## 🚀 Getting Started

> Early but playable: Phases 0–2 are built. On Linux, install the ALSA and udev development packages first (on Debian/Ubuntu: `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`).

**Requirements**
- [Rust](https://www.rust-lang.org/tools/install) (the version is pinned in `rust-toolchain.toml` and installed automatically)
- A GPU with Vulkan, Metal, or DirectX 12 support

**Build and run**
```bash
git clone https://github.com/OGpisuars/PypeLine.git
cd PypeLine/PPL/pypeline
cargo run --release
```

**Run the tests**
```bash
cargo test
```

**Run a script without a window** (prints what the factory made and its state hash)
```bash
cargo run --bin headless -- path/to/main.py 600
```

---

## 🗂️ Project Structure

```text
pypeline/
├── Cargo.toml
├── README.md
├── assets/              # Sprites, fonts, palettes, audio, shaders, data
├── python_runtime/      # In-game Python modules (auto, power, ...) and player scripts
├── src/
│   ├── engine/          # Rendering, grid, wires, UI (editor, polaroid, console)
│   ├── factory/         # Conveyors, machines, power, trains, failures
│   ├── scripting/       # RustPython bridge, sandbox, budget, hot-reload
│   ├── progression/     # Economy, contracts, manual unlocks, saves
│   ├── audio/
│   └── settings/
├── tests/               # Sandbox, determinism, and content tests
├── docs/                # API reference, architecture, content guide
└── tools/               # Asset packing and content validation
```

---

## 🤝 Contributing

Contributions are welcome once the foundation is in place. Good ways to help:

- **Playtesting**, especially if you are new to Python.
- **Sandbox testing.** Try to break out of the script sandbox, then report it.
- **Pixel art and music** in the project's style.
- **Manual chapters and contracts**, which are data files.

Please read `CONTRIBUTING.md` before opening a pull request.

---

## 🙏 Inspiration

*Factorio*, *The Farmer Was Replaced*, and the cozy overworlds of classic handheld RPGs.

---

## 📜 License and Credits

- **Code:** MIT. See [`LICENSE`](LICENSE).


PypeLine is an independent project. It is not affiliated with or endorsed by Nintendo, Game Freak, or the Python Software Foundation. The GBA-inspired look is a style reference only, and all art, music, and names are original. "Python" is a trademark of the Python Software Foundation.
