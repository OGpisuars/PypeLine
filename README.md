<div align="center">

# 🚂 PyPipeline

**Write real Python. Build mega-factories. Master software engineering.**

A cozy 16-bit, GBA-style industrial automation game where your factory runs on code you write yourself.

![Status](https://img.shields.io/badge/status-pre--alpha-orange)
![Rust](https://img.shields.io/badge/built%20with-Rust-b7410e)
![Engine](https://img.shields.io/badge/engine-Bevy-232326)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![License](https://img.shields.io/badge/license-MIT-green)

<!-- Replace with a real GIF once you have one -->
<!-- ![PyPipeline gameplay](docs/images/gameplay.gif) -->

</div>

---

## 📖 About

PyPipeline blends the code-driven automation of *Factorio* and *The Farmer Was Replaced* with the warm, colorful look of classic handheld RPG overworlds.

You live on a floating industrial island. You don't place every machine by hand. Instead you write Python scripts that deploy equipment, wire terminals together, and manage steam power. Sell your output to the cargo train, unlock new chapters in your Engineering Manual, and use what you learn (loops, functions, imports) to build bigger and smarter factories.

> **Project status:** PyPipeline is in early development. Features below marked **(planned)** are designed but not built yet. See the [Roadmap](#-roadmap).

---

## 🕹️ A Taste of the Code

```python
from auto import conveyors, power

# Lay a 5-tile conveyor line heading east
for i in range(5):
    conveyors.place(x=i, y=0, dir="east")

# Hook up the steam generator
power.connect(generator="steam_1")
```

Hit the **Run** arrow and your factory comes to life. Change the script and run it again: your factory floor keeps its state, and the engine updates only what changed.

---

## ✨ Features

### Core
- **Real Python scripting.** Write actual Python from a central `main.py`. No made-up language.
- **Hot-reload on Run.** Edit and re-run without wiping your factory. Scripts are idempotent, so re-running never duplicates machines.
- **Infrastructure-as-code wires.** Imports like `from auto import conveyors` draw color-coded pixel-art wires between terminals.
- **Visible failures.** Syntax errors halt belts and highlight the exact broken line. An infinite loop overheats the boiler. An unhandled exception makes a conveyor spill items.
- **Code has a cost.** Scripts run on a steam budget, so efficient code beats brute force.
- **Polaroid UI.** Hover over a machine to pull up a vintage snapshot with live stats.
- **Progressive Engineering Manual.** Sell goods to the cargo train to unlock chapters: variables, strings, `for` and `while` loops, conditionals, functions, lists and dicts, imports, events, and generators.
- **Campaign and Sandbox.** Structured contracts that teach one concept at a time, plus an open-ended sandbox.
- **Singleplayer.** A tight, polished solo experience with no netcode.

### Debugging and Insight
- **Time Dials.** Pause, play, speed up, or step the simulation one tick at a time.
- **Line debugger (planned).** Step through your script one line at a time and watch your variables.
- **Factory stats (planned).** A dashboard and a `stats` module for items per minute, steam use, and bottlenecks.
- **Friendly errors (planned).** Python tracebacks translated into helpful hints.
- **Snippets (planned).** Starter templates that unlock as you finish each manual chapter.

### Mid and Late Game (planned)
- **Day/night and thermal management.** Boilers run hotter by day and cooler by night. Read the clock and plan around it in code.
- **Cartridges.** Split your code into modules, shown in-game as physical carts you slot into terminals.
- **Micro-chips.** Place tiny chips on sorters and valves to run fast, local micro-scripts and learn edge vs. central computing.
- **LED matrix panels.** Control 8x8 and 16x16 pixel displays with `display.set_pixel(x, y, "green")` to build dashboards.
- **ASCII dashboards.** Print progress bars and custom success banners to the in-game console.
- **Factory wildlife.** Charming creatures like steam-moths react to your factory's health. A calm island means a healthy factory.
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
| **2. Alpha** | It feels like PyPipeline | Floating plots, polaroid UI, wires, cargo train, save/load, Time Dials, boot splash |
| **3. Beta** | The learning loop works | Manual chapters, contracts, debugger, stats, snippets, day/night, cartridges |
| **4. 1.0** | Shippable | Sandbox mode, blueprints, micro-chips, LED panels, themes, accessibility, polish |
| **5. Post-launch** | Community | Workshop sharing, leaderboards, web demo, more chapters |

Each phase has an exit test that must pass before the next one begins.

---

## 🚀 Getting Started

> The game is not playable yet. These steps are for contributors building from source.

**Requirements**
- [Rust](https://www.rust-lang.org/tools/install) (version pinned in `rust-toolchain.toml`)
- A GPU with Vulkan, Metal, or DirectX 12 support

**Build and run**
```bash
git clone https://github.com/<your-username>/pypipeline.git
cd pypipeline
cargo run --release
```

**Run the tests**
```bash
cargo test
```

---

## 🗂️ Project Structure

```text
pypipeline/
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


PyPipeline is an independent project. It is not affiliated with or endorsed by Nintendo, Game Freak, or the Python Software Foundation. The GBA-inspired look is a style reference only, and all art, music, and names are original. "Python" is a trademark of the Python Software Foundation.
