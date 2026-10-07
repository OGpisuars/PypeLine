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
