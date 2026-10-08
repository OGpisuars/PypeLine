//! Output console window: shows print() output and run results.

use bevy_egui::egui;

use crate::scripting::{Console, ConsoleKind};

use super::{palette, rgb};

/// The console section of the code panel.
pub fn console_ui(ui: &mut egui::Ui, console: &mut Console) {
    ui.horizontal(|ui| {
        ui.strong("Console");
        if ui.small_button("Clear").clicked() {
            console.lines.clear();
        }
    });
    egui::ScrollArea::vertical()
        .id_salt("console_scroll")
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
}
