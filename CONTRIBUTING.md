# Contributing to PypeLine

Thanks for wanting to help! PypeLine is a GBA-style factory game where players write real Python to automate their factories. This guide explains how to contribute without stepping on landmines.

> **Status:** The project is in early development. Check the [Roadmap](README.md#-roadmap) to see which phase we are in, and open an issue before starting anything big.

---

## 🤝 Code of Conduct

Be kind, be patient, and assume good intent. Many contributors and players are new to programming, so explain things without talking down to anyone. Harassment, discrimination, and personal attacks are not tolerated and will get you removed from the project.

---

## 🧭 Ways to Help

You do not have to write Rust to contribute.

| Contribution | Skills needed |
|--------------|---------------|
| **Playtesting** | None. Beginners are especially valuable. |
| **Sandbox testing** | Python knowledge, curiosity |
| **Bug reports** | Clear writing |
| **Manual chapters and contracts** | Python teaching, data files (RON/Markdown) |
| **Pixel art and animation** | 16-bit GBA-style art (original work only) |
| **Music and sound** | Original compositions or properly licensed audio |
| **Rust code** | Rust, ideally Bevy |
| **Documentation** | Writing |
| **Translation** | Fluency in another language (later phases) |

---

## 🔒 Reporting Security Issues (Sandbox Escapes)

If you find a way for a player script to read files, run system commands, crash the game unrecoverably, or escape the sandbox, **do not open a public issue.**

Contact [YOUR CONTACT EMAIL] with:
- What you did, with a minimal script that reproduces it
- What happened
- Your OS and game version

We will fix it before disclosing it publicly, and we are happy to credit you.

---

## 🐛 Reporting Bugs

Use the bug report issue template and include:

- Game version or commit hash
- Operating system (Windows, Linux, macOS)
- Steps to reproduce
- What you expected and what happened
- Your `main.py` if the bug involves scripting
- A screenshot or short clip if it is visual

For simulation bugs, a **replay file or save** is the most useful thing you can attach.

---

## 💡 Suggesting Features

Open an issue describing the problem you want solved, not just the feature. Check first that:

- It is not already in the roadmap.
- It fits the game: singleplayer, deterministic, teaches real Python.
- It does not require breaking the determinism rules below.

Features that are cosmetic or flavor-only are lower priority and may be queued behind the core learning loop.

---

## 🛠️ Development Setup

**Requirements**
- [Rust](https://www.rust-lang.org/tools/install) (the version is pinned in `rust-toolchain.toml`)
- Git, and Git LFS for assets
- A GPU with Vulkan, Metal, or DirectX 12 support

**Get started**
```bash
git clone https://github.com/<your-username>/pypeline.git
cd pypeline
git lfs pull
cargo run --release
```

**Before every pull request**
```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs on Windows, Linux, and macOS. Your change must pass on all three.

---

## 🧱 Project Architecture

| Folder | Purpose |
|--------|---------|
| `src/engine/` | Rendering, grid, wires, UI (egui editor, console, polaroid) |
| `src/factory/` | Simulation: conveyors, machines, power, thermal, trains |
| `src/scripting/` | RustPython bridge, sandbox, step budget, hot-reload |
| `src/progression/` | Economy, contracts, manual unlocks, saves |
| `python_runtime/stdlib/` | Python modules exposed to players (`auto`, `power`, ...) |
| `assets/data/` | Contracts, manual chapters, recipes (data-driven) |
| `tests/` | Sandbox, determinism, budget, and content tests |

See `docs/ARCHITECTURE.md` for the full picture.

---

## ⚖️ Rules That Must Not Be Broken

These protect the whole game. Pull requests that break them will be sent back.

### 1. Determinism
The simulation must give the same result for the same script and seed on every OS.
- No wall-clock time in simulation logic. Use the tick counter.
- No `HashMap` iteration in simulation code. Use `BTreeMap` or `IndexMap`.
- Use integers or fixed-point math in the simulation. Floats are for rendering.
- All randomness comes from the simulation's seeded RNG.
- Keep system ordering explicit in Bevy.
- **Cosmetic systems (wildlife, visual effects) may read simulation state but must never write to it.**

### 2. Sandbox Safety
- Never expose file, network, process, or OS access to player scripts.
- Every new `auto.*`, `power.*`, or other module function needs bounds checks, a step cost, and a sandbox test.
- Player-visible output must be sanitized (no raw escape codes or control characters).
- Treat all player scripts and shared blueprints as untrusted.

### 3. Never Lose Player Work
- Saves and scripts use atomic writes.
- Any change to the save format needs a version bump and a migration.

### 4. Idempotent Scripting API
Because of hot-reload, API calls must be safe to run twice. Placement functions need stable identities so the reconcile system can create, update, or remove things correctly.

### 5. Teaching First
Changes must not make the game confusing for beginners. New API functions need clear names, helpful error messages, and an entry in `docs/API.md` and the `.pyi` stubs.

---

## 🧪 Tests

Add tests with your change when you can. We especially care about:

- **Sandbox tests** (`tests/sandbox_tests.rs`): forbidden code must be rejected.
- **Determinism tests**: same inputs, same state hash.
- **Budget tests**: infinite loops get capped.
- **Contract tests**: every contract has a golden solution that passes in CI.
- **Manual example tests**: every code sample in the manual must run.

---

## 📚 Contributing Content (No Rust Needed)

Contracts and manual chapters are data files in `assets/data/`.

**Manual chapter** (`assets/data/manual/chNN_name.md`)
- One concept per chapter.
- Short explanations, real examples, tiny exercises.
- Every code block must run in the game's supported Python subset.

**Contract** (`assets/data/contracts/chNN_name.ron`)
- States the concept it teaches.
- Includes a golden solution script in `tests/golden_solutions/`.
- Is beatable but not trivial, and tested by someone new to Python if possible.

See `docs/CONTENT_GUIDE.md` for formats and examples. Run the validator before submitting:

```bash
cargo run --bin validate_content
```

---

## 🎨 Contributing Art and Audio

**Style guide:** see `docs/ART_STYLE.md`.
- 16x16 tiles, top-down 3/4 view
- The master palette in `assets/palettes/master.pal` only
- Max 15 colors plus transparency per sprite
- Flat shading, 1px dark outline, light from the top-left
- No dithering, gradients, or anti-aliasing
- 2 to 4 frame animations

**Legal rules (strict):**
- **Original work only.** The GBA look is a style reference, not a source.
- **Never** copy, trace, recolor, or edit assets from Nintendo, Game Freak, or any other commercial game.
- No creatures, names, or UI that imitate existing franchises.
- AI-generated or third-party assets need a clearly compatible license and must be disclosed in the pull request.
- Every asset must be added to [`ASSET_LICENSES.md`](ASSET_LICENSES.md) in the same pull request.

Assets with unclear origins will be rejected.

---

## 🔀 Pull Request Process

1. **Open or find an issue** and say you are working on it.
2. **Fork** the repo and create a branch: `feature/short-name` or `fix/short-name`.
3. **Keep it focused.** One change per pull request.
4. **Write clear commits.** Short summary line, then details if needed.
5. **Run** `cargo fmt`, `cargo clippy`, and `cargo test`.
6. **Update docs** (`docs/API.md`, stubs, manual) if behavior changed.
7. **Open the pull request** and fill in the template. Include screenshots or clips for visual changes.
8. **Respond to review.** We may ask for changes. This is normal.

Pull requests need passing CI on all three operating systems and at least one maintainer approval before merging into `main`.

---

## 📄 Licensing of Contributions

- **Code:** by submitting a pull request, you agree that your code is licensed under the project's [MIT License](LICENSE).
- **Assets:** by submitting art, music, or other media, you confirm you created it or have the right to share it, and you grant the project the right to use and distribute it as described in [`ASSET_LICENSES.md`](ASSET_LICENSES.md).
- You keep the copyright to your work unless the license says otherwise.

If you are not sure you can legally contribute something, ask first.

---

## ❓ Questions

Open a discussion or an issue labeled `question`. There are no silly questions, especially from people learning Python.

Thank you for helping make learning to code feel like playing a game. 🚂
