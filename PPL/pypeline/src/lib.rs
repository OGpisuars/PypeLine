//! PypeLine: a cozy GBA-style factory game where your factory runs on Python.

pub mod audio;
pub mod dev;
pub mod engine;
pub mod factory;
pub mod progression;
pub mod scripting;

use bevy::prelude::*;

/// Counts heap bytes so scripts can be held to a memory cap.
#[global_allocator]
static ALLOCATOR: scripting::memory::CountingAlloc = scripting::memory::CountingAlloc;
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
            // Simulation, scripting and progression.
            .add_plugins((
                factory::SimPlugin,
                factory::FactoryPlugin,
                scripting::ScriptingPlugin,
                progression::saves::SavePlugin,
                progression::contracts::ContractPlugin,
                audio::GameAudioPlugin,
                dev::DevPlugin,
            ))
            // Rendering and UI.
            .add_plugins((
                engine::camera::PixelCameraPlugin,
                engine::floating_plot::FloatingPlotPlugin,
                engine::grid::GridPlugin,
                engine::renderer::FactoryRenderPlugin,
                engine::wires::WirePlugin,
                engine::failures::FailurePlugin,
                engine::wildlife::WildlifePlugin,
                engine::splash::SplashPlugin,
                engine::ui::UiPlugin,
            ));
    }
}
