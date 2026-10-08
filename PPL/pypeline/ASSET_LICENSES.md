# Asset Licenses

Every third-party file shipped with PypeLine (fonts, sounds, images) and every Rust crate with an unusual license goes in this table. Only licenses that allow commercial use are accepted (roadmap Part 4 G).

Original PypeLine art, sprites and music are not listed here; they are covered by the project's own asset license.

## Fonts

| File | What it is | Author | License | License file | Source |
|------|------------|--------|---------|--------------|--------|
| `assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf` | Code editor and console font (JetBrains Mono, Nerd Font "Mono" build) | The JetBrains Mono Project Authors; Nerd Fonts patch by Ryan L McIntyre | SIL Open Font License 1.1 (no Reserved Font Name) | `assets/fonts/JetBrainsMono-OFL.txt` | https://github.com/JetBrains/JetBrainsMono, https://github.com/ryanoasis/nerd-fonts |
| `assets/fonts/MapleMono-Regular.ttf` | Optional font (Settings) | subframe7536 | SIL Open Font License 1.1 | `assets/fonts/MapleMono-OFL.txt` | https://github.com/subframe7536/maple-font |
| `assets/fonts/NotoSansMono-Regular.ttf` | Optional font (Settings) | The Noto Project Authors | SIL Open Font License 1.1 | `assets/fonts/NotoSansMono-OFL.txt` | https://github.com/notofonts/latin-greek-cyrillic |
| Hack (built into egui) | Optional font (Settings) | Source Foundry | MIT / Bitstream Vera license | shipped inside the `epaint_default_fonts` crate | https://github.com/source-foundry/Hack |

**OFL notes:** the font may be bundled with the game, including a commercial release, as long as the license file ships with it. The font must not be sold on its own.
