# Asset Licenses

This file tracks the license, author, and source of **every** non-code asset and third-party dependency in PyPipeline. If it ships in the game, it must be listed here.

> **Rule:** No asset is added to the repository until it has a row in this file. If you cannot fill in the license column with certainty, the asset does not go in.

---

## 1. License for Original Assets

All original art, music, sound effects, and fonts created for PyPipeline are:

**Copyright (c) 2026 [YOUR NAME OR STUDIO]. All rights reserved.**

The MIT license in [`LICENSE`](LICENSE) covers the **source code only**. It does **not** grant any right to reuse the original game assets.

<!--
OWNER DECISION: "All rights reserved" is the safest default for a commercial
game. If you would rather let people reuse your art, replace the line above
with one of these:
  - CC BY 4.0        (reuse allowed, even commercially, with credit)
  - CC BY-NC 4.0     (reuse allowed, non-commercial only, with credit)
  - CC0              (public domain)
Pick ONE before accepting outside art contributions.
-->

---

## 2. Intellectual Property Rules

PyPipeline is an independent project. To keep it legally clean:

- The GBA-inspired look is a **style reference only**.
- **Never** copy, trace, recolor, or edit sprites, tiles, fonts, UI frames, music, or sound effects from any Nintendo, Game Freak, or other commercial game.
- **Never** use names, logos, or designs that imitate existing franchises (for example Pokemon-like creatures, or Nintendo hardware colorway names and logos).
- Terminal themes use original names and designs (for example Indigo Dusk, Ember, Glasswork, Mint Cart).
- Cartridge items are called "Carts" or "Paks", never by a trademarked console name.
- Do not imitate Nintendo's startup logo animation or jingle.
- "Python" is a trademark of the Python Software Foundation. PyPipeline is not affiliated with or endorsed by it.

---

## 3. Asset Register

### Sprites and Tiles

| File / Folder | Author | License | Source / Link | Notes |
|---------------|--------|---------|---------------|-------|
| `assets/sprites/tiles/` | [name] | All rights reserved | Original | |
| `assets/sprites/machines/` | [name] | All rights reserved | Original | |
| `assets/sprites/conveyors/` | [name] | All rights reserved | Original | |
| `assets/sprites/items/` | [name] | All rights reserved | Original | |
| `assets/sprites/train/` | [name] | All rights reserved | Original | |
| `assets/sprites/wires/` | [name] | All rights reserved | Original | |
| `assets/sprites/ui/` | [name] | All rights reserved | Original | |
| `assets/sprites/fx/` | [name] | All rights reserved | Original | |
| `assets/sprites/creatures/` | [name] | All rights reserved | Original | Must not resemble existing monsters |

### Fonts

| File | Author | License | Source / Link | Commercial use OK? |
|------|--------|---------|---------------|--------------------|
| `assets/fonts/[ui-font]` | [name] | [e.g. SIL OFL 1.1] | [URL] | [Yes / No] |
| `assets/fonts/[mono-font]` | [name] | [e.g. SIL OFL 1.1] | [URL] | [Yes / No] |

The console font must include block and box-drawing glyphs. If you modify a font, check that its license allows modification (SIL OFL does, but may require renaming).

### Music and Sound

| File / Folder | Author | License | Source / Link | Commercial use OK? |
|---------------|--------|---------|---------------|--------------------|
| `assets/audio/music/` | [name] | [license] | [URL or Original] | [Yes / No] |
| `assets/audio/sfx/` | [name] | [license] | [URL or Original] | [Yes / No] |
| `assets/audio/jingles/` | [name] | [license] | [URL or Original] | [Yes / No] |

### Shaders, Palettes, and Themes

| File / Folder | Author | License | Source / Link | Notes |
|---------------|--------|---------|---------------|-------|
| `assets/shaders/` | [name] | MIT (code) | Original | |
| `assets/palettes/` | [name] | All rights reserved | Original | |
| `assets/themes/` | [name] | All rights reserved | Original | Original designs only |

---

## 4. Third-Party Code Dependencies

Cargo dependencies keep their own licenses. Before every release, generate the full list:

```bash
cargo install cargo-license cargo-deny
cargo license
cargo deny check licenses
```

Add a `deny.toml` that only allows compatible licenses (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode). Block GPL or AGPL dependencies unless you intentionally want your whole game under those terms.

| Crate | License | Purpose |
|-------|---------|---------|
| `bevy` | MIT OR Apache-2.0 | Game engine |
| `bevy_egui` | MIT | Editor / UI |
| `rustpython` | MIT | Script runtime |
| `serde` | MIT OR Apache-2.0 | Serialization |
| `ron` | MIT OR Apache-2.0 | Data files |

> Verify each license against the crate's own repository at the exact version you use. Licenses can change between versions.

---

## 5. Contributor Assets

Anyone submitting art, music, fonts, or other media must:

1. Confirm they created it, or have the legal right to submit it.
2. State its license and source in the pull request.
3. Agree that the project may use and distribute it under the terms in Section 1 (or the license the owner chooses).
4. Add a row to the register above.

Assets without clear provenance will be rejected.

---

## 6. Takedown and Corrections

If you believe an asset in this repository infringes your rights or is listed incorrectly, open an issue or contact [YOUR CONTACT EMAIL]. We will review it promptly and remove the asset if needed.

*Last reviewed: [DATE]*
