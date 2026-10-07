//! Output console window: shows print() output and run results.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::scripting::{Console, ConsoleKind};

use super::{palette, rgb};

pub fn console_window(mut contexts: EguiContexts, mut console: ResMut<Console>) -> Result {
    egui::Window::new("Console")
        .default_pos(egui::pos2(16.0, 500.0))
        .default_size(egui::vec2(460.0, 200.0))
        .show(contexts.ctx_mut()?, |ui| {
            if ui.button("Clear").clicked() {
                console.lines.clear();
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line in &console.lines {
                        let text = egui::RichText::new(&line.text).monospace();
                        let text = match line.kind {
                            ConsoleKind::Output => text,
                            ConsoleKind::Info => text.color(rgb(palette::UI_INFO)),
                            ConsoleKind::Error => text.color(rgb(palette::UI_ERROR)),
                        };
                        ui.label(text);
                    }
                });
        });
    Ok(())
}
