//! egui-based UI: code editor, console, and the debug overlay.
//!
//! egui draws at window resolution, not inside the 480x320 canvas. It is
//! styled with the FireRed-style palette and uses JetBrains Mono for all text
//! (see ASSET_LICENSES.md).

pub mod console;
pub mod editor;
pub mod help;
pub mod highlight;
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
            .add_systems(Startup, editor::load_script)
            .add_systems(Update, (help::toggle_help, editor::autosave))
            .add_systems(
                EguiPrimaryContextPass,
                (
                    apply_theme,
                    editor::editor_window,
                    console::console_window,
                    help::help_window,
                    time_dials::time_dials,
                    time_dials::time_dial_keys,
                    debug_overlay,
                )
                    .chain(),
            );
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

fn debug_overlay(
    mut contexts: EguiContexts,
    tick: Res<SimTick>,
    scale: Res<PixelScale>,
    factory: Res<Factory>,
    hovered: Res<HoveredTile>,
    control: Res<SimControl>,
    diagnostics: Res<DiagnosticsStore>,
) -> Result {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    egui::Area::new(egui::Id::new("debug_overlay"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
        .interactable(false)
        .show(contexts.ctx_mut()?, |ui| {
            let status = match (factory.halted, control.paused) {
                (true, _) => "  |  HALTED".to_owned(),
                (false, true) => "  |  PAUSED".to_owned(),
                (false, false) if control.speed > 1 => format!("  |  {}x", control.speed),
                _ => String::new(),
            };
            let lines = [
                format!(
                    "tick {}  |  {fps:.0} fps  |  zoom {}x{status}",
                    tick.0, scale.0
                ),
                format!(
                    "ore mined {}  |  plates made {}",
                    factory.produced(ItemKind::IronOre),
                    factory.produced(ItemKind::IronPlate)
                ),
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
