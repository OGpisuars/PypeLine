//! egui-based UI: the top bar, floating code windows, console, Manual,
//! Help, Shop, Settings, the pause menu and the HUD.
//!
//! egui draws at window resolution, not inside the 480x320 canvas. Its look
//! comes from the theme and font picked in Settings (see `themes.rs` and
//! ASSET_LICENSES.md).

pub mod achievements;
pub mod autocomplete;
pub mod cheat_sheet;
pub mod console;
pub mod crafter;
pub mod debugger;
pub mod editor;
pub mod help;
pub mod highlight;
pub mod hud;
pub mod main_menu;
pub mod manual;
pub mod pause_menu;
pub mod polaroid;
pub mod settings;
pub mod shop;
pub mod stats_panel;
pub mod time_dials;
pub mod top_bar;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::EguiPrimaryContextPass;

use crate::factory::Factory;

use super::screens::{Screen, menu_or_playing};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<editor::Workspace>()
            .init_resource::<help::HelpState>()
            .init_resource::<manual::ManualState>()
            .init_resource::<settings::Settings>()
            .init_resource::<settings::SettingsWindow>()
            .init_resource::<shop::ShopWindow>()
            .init_resource::<pause_menu::PauseMenu>()
            .init_resource::<achievements::AchievementsWindow>()
            .init_resource::<crafter::RecipePicker>()
            .init_resource::<cheat_sheet::CheatSheetWindow>()
            .init_resource::<stats_panel::StatsWindow>()
            .init_resource::<debugger::Debugger>()
            .insert_non_send(debugger::DebugRuntime(
                crate::scripting::runtime::ScriptRuntime::new(),
            ))
            .add_systems(Startup, (editor::load_scripts, settings::load_settings))
            .add_systems(
                Update,
                (
                    help::toggle_help,
                    manual::toggle_manual,
                    shop::toggle_shop,
                    stats_panel::toggle_stats,
                    announce_sales,
                    debugger::run_debugger,
                )
                    .run_if(in_state(Screen::Playing)),
            )
            .add_systems(Update, settings::save_settings)
            // In Last, so it sees the AppExit sent when the window closes
            // and the cursor bevy_egui picked this frame.
            .add_systems(Last, (editor::autosave, settings::update_cursor))
            // The theme applies on every screen.
            .add_systems(EguiPrimaryContextPass, settings::apply_settings)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    main_menu::main_menu.run_if(in_state(Screen::Menu)),
                    settings::settings_window.run_if(menu_or_playing),
                )
                    .after(settings::apply_settings),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    top_bar::top_bar,
                    editor::code_windows,
                    help::help_window,
                    manual::manual_window,
                    shop::shop_window,
                    stats_panel::stats_window,
                    debugger::debugger_window,
                    debugger::debug_keys,
                    time_dials::time_dial_keys,
                    polaroid::polaroid,
                    hud::hud,
                    crafter::recipe_picker,
                    achievements::achievements_window,
                    cheat_sheet::cheat_sheet_window,
                    pause_menu::pause_menu,
                )
                    .chain()
                    .after(settings::apply_settings)
                    // The game's windows and overlays only show while playing.
                    .run_if(in_state(Screen::Playing)),
            );
    }
}

/// Tell the player in the console each time the train buys something.
fn announce_sales(
    factory: Res<Factory>,
    mut console: ResMut<crate::scripting::Console>,
    mut sounds: MessageWriter<crate::audio::SoundCue>,
    mut announced: Local<Option<u64>>,
) {
    let Some(sale) = &factory.last_sale else {
        return;
    };
    // The first time, just note the sale already in a loaded save.
    if announced.is_none() {
        *announced = Some(sale.tick);
        return;
    }
    if *announced != Some(sale.tick) {
        *announced = Some(sale.tick);
        console.push(crate::scripting::ConsoleKind::Info, sale.summary());
        if !sale.items.is_empty() {
            sounds.write(crate::audio::SoundCue(crate::audio::sfx::Sfx::Sale));
        }
    }
}
