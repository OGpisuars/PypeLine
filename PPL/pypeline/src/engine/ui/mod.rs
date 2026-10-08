//! egui-based UI: the top bar, floating code windows, console, Manual,
//! Help, Shop, Settings, and the debug overlay.
//!
//! egui draws at window resolution, not inside the 480x320 canvas. Its look
//! comes from the theme and font picked in Settings (see `themes.rs` and
//! ASSET_LICENSES.md).

pub mod autocomplete;
pub mod console;
pub mod editor;
pub mod help;
pub mod highlight;
pub mod manual;
pub mod polaroid;
pub mod settings;
pub mod shop;
pub mod stats_panel;
pub mod time_dials;
pub mod top_bar;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::factory::items::ItemKind;
use crate::factory::tick::SimControl;
use crate::factory::{Factory, SimTick};

use super::camera::{HoveredTile, PixelScale};

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
            .init_resource::<stats_panel::StatsWindow>()
            .add_systems(Startup, (editor::load_scripts, settings::load_settings))
            .add_systems(
                Update,
                (
                    help::toggle_help,
                    manual::toggle_manual,
                    shop::toggle_shop,
                    stats_panel::toggle_stats,
                    announce_sales,
                    settings::save_settings,
                ),
            )
            // In Last, so it sees the AppExit sent when the window closes
            // and the cursor bevy_egui picked this frame.
            .add_systems(Last, (editor::autosave, settings::update_cursor))
            // The theme applies during the splash too.
            .add_systems(EguiPrimaryContextPass, settings::apply_settings)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    top_bar::top_bar,
                    editor::code_windows,
                    help::help_window,
                    manual::manual_window,
                    shop::shop_window,
                    stats_panel::stats_window,
                    settings::settings_window,
                    time_dials::time_dial_keys,
                    polaroid::polaroid,
                    debug_overlay,
                )
                    .chain()
                    .after(settings::apply_settings)
                    // The windows and overlays appear once the splash is done.
                    .run_if(super::splash::splash_finished),
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

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn debug_overlay(
    mut contexts: EguiContexts,
    tick: Res<SimTick>,
    scale: Res<PixelScale>,
    factory: Res<Factory>,
    hovered: Res<HoveredTile>,
    progress: Res<crate::progression::contracts::Progress>,
    control: Res<SimControl>,
    area: Res<super::camera::GameArea>,
    diagnostics: Res<DiagnosticsStore>,
    history: Res<crate::factory::ProductionHistory>,
) -> Result {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    egui::Area::new(egui::Id::new("debug_overlay"))
        // Under every window, so it never covers the Manual or Help.
        .order(egui::Order::Background)
        .pivot(egui::Align2::RIGHT_TOP)
        .fixed_pos(area.0.map_or(egui::pos2(8.0, 8.0), |a| {
            egui::pos2(a.max.x - 8.0, a.min.y + 8.0)
        }))
        .interactable(false)
        .show(contexts.ctx_mut()?, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let status = match (factory.halted, control.paused) {
                (true, _) => "  |  HALTED".to_owned(),
                (false, true) => "  |  PAUSED".to_owned(),
                (false, false) if control.speed > 1 => format!("  |  {}x", control.speed),
                _ => String::new(),
            };
            let lines =
                [
                    format!(
                        "tick {}  |  {fps:.0} fps  |  zoom {}x{status}",
                        tick.0, scale.0
                    ),
                    format!(
                        "coins {}  |  ore mined {}  |  plates made {}",
                        factory.coins,
                        factory.produced(ItemKind::IronOre),
                        factory.produced(ItemKind::IronPlate)
                    ),
                    match progress.active.as_ref().and_then(|a| {
                        crate::progression::chapters::contract(&a.id).map(|c| (a, c.1))
                    }) {
                        Some((active, contract)) => {
                            let (have, need) = crate::progression::contracts::goal_progress(
                                &contract.goal,
                                active,
                                &factory,
                                &history.per_minute(&factory),
                            );
                            format!("contract: {}  {have}/{need}", contract.title)
                        }
                        None => "no contract: open the Manual (F2)".to_owned(),
                    },
                    match hovered.0 {
                        Some(pos) => format!("mouse on tile x={}, y={}", pos.x, pos.y),
                        None => "point at the island to see x and y".to_owned(),
                    },
                ];
            for line in lines {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .color(egui::Color32::WHITE)
                        .background_color(egui::Color32::from_black_alpha(140)),
                );
            }
        });
    Ok(())
}
