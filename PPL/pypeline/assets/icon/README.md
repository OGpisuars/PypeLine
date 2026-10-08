# Game icon

- `PypeLine.jpg`: the original icon art.
- `pypeline.png`: the same picture with the white background outside the
  rounded frame made transparent. This is the one the game uses.

`build.rs` turns `pypeline.png` into the window icon and, on Windows, the
icon built into `PypeLine.exe`. To change the icon, replace `pypeline.png`
(square, 256x256 or larger, transparent outside the frame) and rebuild.
Without the file, the game falls back to the pixel-art icon drawn in code
in `src/engine/icon_art.rs`.
