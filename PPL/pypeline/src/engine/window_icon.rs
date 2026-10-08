//! The window's icon (title bar and taskbar). `build.rs` makes the image;
//! on Windows it is also built into the .exe.
//!
//! Wayland ignores window icons: desktops there take the icon from the
//! app's .desktop entry instead, matched by the window's app id "pypeline".

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;

/// 128x128 RGBA, from `build.rs`.
const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.rgba"));
const SIZE: u32 = 128;

pub struct WindowIconPlugin;

impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, set_window_icon);
    }
}

/// Set the icon once winit has made the window (a frame or two in).
fn set_window_icon(
    mut done: Local<bool>,
    window: Query<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    if *done {
        return;
    }
    let Ok(entity) = window.single() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(winit_window) = windows.get_window(entity) else {
            return;
        };
        match winit::window::Icon::from_rgba(ICON.to_vec(), SIZE, SIZE) {
            Ok(icon) => winit_window.set_window_icon(Some(icon)),
            Err(err) => warn!("Could not set the window icon: {err}"),
        }
        *done = true;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_the_right_size() {
        assert_eq!(ICON.len(), (SIZE * SIZE * 4) as usize);
        // Something is drawn in the middle.
        let middle = ((SIZE / 2 * SIZE + SIZE / 2) * 4 + 3) as usize;
        assert_eq!(ICON[middle], 255);
    }
}
