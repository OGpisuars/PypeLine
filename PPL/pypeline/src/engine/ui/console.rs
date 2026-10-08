//! Output console window: shows print() output and run results.

use bevy_egui::egui;

use crate::scripting::console_api::ConsoleColor;
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
                    ConsoleKind::Output => match line.color {
                        Some(color) => text.color(script_color(color)),
                        None => text,
                    },
                    ConsoleKind::Info => text.color(rgb(palette::UI_INFO)),
                    ConsoleKind::Error => text.color(rgb(palette::UI_ERROR)),
                };
                ui.label(text);
            }
        });
}

/// The palette colors scripts can pick with console.color(...).
fn script_color(color: ConsoleColor) -> egui::Color32 {
    let [r, g, b] = match color {
        ConsoleColor::Green => [48, 136, 56],
        ConsoleColor::Red => palette::UI_ERROR,
        ConsoleColor::Yellow => [176, 128, 0],
        ConsoleColor::Blue => palette::UI_INFO,
        ConsoleColor::Orange => [200, 104, 40],
        ConsoleColor::Gray => [128, 120, 104],
    };
    egui::Color32::from_rgb(r, g, b)
}
