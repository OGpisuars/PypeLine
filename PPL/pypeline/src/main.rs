// Release builds on Windows open no extra console window behind the game.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;
use pypeline::{
    PypelinePlugin,
    engine::camera::{RES_HEIGHT, RES_WIDTH},
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "PypeLine".into(),
                        // The app id on Linux (Wayland matches it to a .desktop file's icon).
                        name: Some("pypeline".into()),
                        // Start at 3x the 480x320 canvas.
                        resolution: (RES_WIDTH * 3, RES_HEIGHT * 3).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(PypelinePlugin)
        .run();
}
