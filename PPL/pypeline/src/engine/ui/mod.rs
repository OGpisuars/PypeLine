//! egui-based UI: code editor, console, and the debug overlay.
//!
//! egui draws at window resolution, not inside the 480x320 canvas. It is
//! styled with the FireRed-style palette and uses JetBrains Mono for all text
//! (see ASSET_LICENSES.md).

pub mod console;
pub mod editor;
pub mod help;
pub mod highlight;
pub mod manual;
pub mod polaroid;
pub mod time_dials;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::factory::items::ItemKind;
use crate::factory::tick::SimControl;
use crate::factory::{Factory, SimTick};

use super::camera::{HoveredTile, PixelScale};
use super::palette;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<editor::EditorState>()
            .init_resource::<help::HelpState>()
            .init_resource::<manual::ManualState>()
            .add_systems(Startup, editor::load_script)
            .add_systems(
                Update,
                (help::toggle_help, manual::toggle_manual, announce_sales),
            )
            // In Last, so it sees the AppExit sent when the window closes.
            .add_systems(Last, editor::autosave)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    apply_theme,
                    editor::code_panel,
                    help::help_window,
                    manual::manual_window,
                    time_dials::time_dials,
                    time_dials::time_dial_keys,
                    polaroid::polaroid,
                    debug_overlay,
                )
                    .chain()
                    // The code panel and overlays appear once the splash is done.
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

pub(crate) fn rgb([r, g, b]: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}

/// FireRed-style dialogue boxes: cream fill, dark ink, rounded blue border.
fn apply_theme(mut contexts: EguiContexts, mut applied: Local<bool>) -> Result {
    if *applied {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = rgb(palette::UI_CREAM);
    visuals.panel_fill = rgb(palette::UI_CREAM);
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 252, 240);
    visuals.code_bg_color = egui::Color32::from_rgb(255, 252, 240);
    visuals.override_text_color = Some(rgb(palette::UI_INK));
    visuals.window_stroke = egui::Stroke::new(3.0, rgb(palette::UI_BORDER));
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.window_shadow = egui::Shadow::NONE;
    ctx.set_visuals(visuals);

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf"
        ))),
    );
    for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "jetbrains_mono".to_owned());
    }
    ctx.set_fonts(fonts);
    *applied = true;
    Ok(())
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
) -> Result {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    egui::Area::new(egui::Id::new("debug_overlay"))
        // Under every window, so it never covers the Manual or Help.
        .order(egui::Order::Background)
        .fixed_pos(area.0.map_or(egui::pos2(8.0, 8.0), |a| {
            egui::pos2(a.min.x + 8.0, a.min.y + 52.0)
        }))
        .interactable(false)
        .show(contexts.ctx_mut()?, |ui| {
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
