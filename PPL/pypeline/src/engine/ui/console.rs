//! Output console window: shows print() output and run results.

use bevy_egui::egui;

use crate::scripting::console_api::ConsoleColor;
use crate::scripting::{Console, ConsoleKind};

/// The inside of the console window.
pub fn console_ui(ui: &mut egui::Ui, console: &mut Console) {
    let dark = ui.visuals().dark_mode;
    let info = ui.visuals().hyperlink_color;
    let error = ui.visuals().error_fg_color;
    if ui
        .small_button("Clear")
        .on_hover_text("Empty the console")
        .clicked()
    {
        console.lines.clear();
    }
    egui::ScrollArea::vertical()
        .id_salt("console_scroll")
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in &console.lines {
                let text = egui::RichText::new(&line.text).monospace();
                let text = match line.kind {
                    ConsoleKind::Output => match line.color {
                        Some(color) => text.color(script_color(color, dark)),
                        None => text,
                    },
                    ConsoleKind::Info => text.color(info),
                    ConsoleKind::Error => text.color(error),
                };
                ui.label(text);
            }
        });
}

/// The colors scripts can pick with console.color(...), brighter on dark
/// themes so they stay readable.
fn script_color(color: ConsoleColor, dark: bool) -> egui::Color32 {
    let ([r, g, b], [dr, dg, db]) = match color {
        ConsoleColor::Green => ([48, 136, 56], [120, 216, 120]),
        ConsoleColor::Red => ([200, 48, 48], [248, 112, 112]),
        ConsoleColor::Yellow => ([176, 128, 0], [248, 216, 88]),
        ConsoleColor::Blue => ([72, 96, 152], [128, 176, 248]),
        ConsoleColor::Orange => ([200, 104, 40], [248, 160, 88]),
        ConsoleColor::Gray => ([128, 120, 104], [168, 168, 160]),
    };
    if dark {
        egui::Color32::from_rgb(dr, dg, db)
    } else {
        egui::Color32::from_rgb(r, g, b)
    }
}
