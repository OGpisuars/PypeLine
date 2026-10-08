//! Makes the game icon: `assets/icon/pypeline.png` if it exists, or else the
//! pixel art drawn in `src/engine/icon_art.rs`.
//!
//! - `$OUT_DIR/icon.rgba`: 128x128 RGBA for the window icon (see
//!   `src/engine/window_icon.rs`).
//! - On Windows, `PypeLine.exe` gets the icon built in, for Explorer and the
//!   taskbar.

use std::path::{Path, PathBuf};

#[allow(dead_code)]
mod icon_art {
    include!("src/engine/icon_art.rs");
}

/// The window icon's size, in pixels.
const WINDOW_ICON: usize = 128;
/// The sizes inside the Windows .ico.
const ICO_SIZES: [usize; 5] = [256, 64, 48, 32, 16];

fn main() {
    // Watch the folder, not the file: a missing file would rerun this every build.
    println!("cargo::rerun-if-changed=assets/icon");
    println!("cargo::rerun-if-changed=src/engine/icon_art.rs");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let png = Path::new("assets/icon/pypeline.png");

    let sized = |size: usize| -> Vec<u8> {
        if png.exists() {
            from_png(png, size)
        } else if size.is_multiple_of(icon_art::ICON_SIZE)
            || icon_art::ICON_SIZE.is_multiple_of(size)
        {
            icon_art::icon_rgba_sized(size)
        } else {
            // 48 px: draw it at 192, then average each 4x4 square.
            icon_art::shrink(&icon_art::icon_rgba_sized(size * 4), size * 4, 4)
        }
    };
    std::fs::write(out.join("icon.rgba"), sized(WINDOW_ICON)).expect("write icon.rgba");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let images: Vec<(usize, Vec<u8>)> = ICO_SIZES.iter().map(|&s| (s, sized(s))).collect();
        let ico = out.join("pypeline.ico");
        std::fs::write(&ico, icon_art::ico(&images)).expect("write pypeline.ico");
        let rc = out.join("pypeline.rc");
        let ico_path = ico.display().to_string().replace('\\', "/");
        std::fs::write(&rc, format!("1 ICON \"{ico_path}\"\n")).expect("write pypeline.rc");
        embed_resource::compile(&rc, embed_resource::NONE)
            .manifest_optional()
            .expect("could not build the icon into the .exe");
    }
}

/// The PNG resized to `size` square.
fn from_png(path: &Path, size: usize) -> Vec<u8> {
    let image = image::open(path)
        .unwrap_or_else(|err| panic!("could not read {}: {err}", path.display()))
        .into_rgba8();
    image::imageops::resize(
        &image,
        size as u32,
        size as u32,
        image::imageops::FilterType::Lanczos3,
    )
    .into_raw()
}
