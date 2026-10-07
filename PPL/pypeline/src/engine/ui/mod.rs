//! egui-based UI: code editor, console, and the debug overlay.
//!
//! egui draws at window resolution, not inside the 480x320 canvas. Phase 0
//! styles it with the FireRed-style palette; the pixel font and integer
//! pixels_per_point come once a licensed pixel font is added to assets/fonts.

pub mod console;
pub mod editor;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::factory::SimTick;

use super::camera::PixelScale;
use super::palette;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<editor::EditorState>()
            .add_systems(
                EguiPrimaryContextPass,
                (
                    apply_theme,
                    editor::editor_window,
                    console::console_window,
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
    *applied = true;
    Ok(())
}

fn debug_overlay(
    mut contexts: EguiContexts,
    tick: Res<SimTick>,
    scale: Res<PixelScale>,
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
            ui.label(
                egui::RichText::new(format!("tick {}  |  {fps:.0} fps  |  {}x", tick.0, scale.0))
                    .monospace()
                    .color(egui::Color32::WHITE)
                    .background_color(egui::Color32::from_black_alpha(140)),
            );
        });
    Ok(())
}
