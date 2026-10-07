//! PypeLine: a cozy GBA-style factory game where your factory runs on Python.

pub mod engine;
pub mod factory;
pub mod scripting;

use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin};

/// Everything the game needs on top of Bevy's `DefaultPlugins`.
pub struct PypelinePlugin;

impl Plugin for PypelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            // The screen camera is marked as the egui context explicitly.
            .insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_plugins((
                factory::SimPlugin,
                factory::FactoryPlugin,
                scripting::ScriptingPlugin,
                engine::camera::PixelCameraPlugin,
                engine::grid::GridPlugin,
                engine::renderer::FactoryRenderPlugin,
                engine::ui::UiPlugin,
            ));
    }
}
