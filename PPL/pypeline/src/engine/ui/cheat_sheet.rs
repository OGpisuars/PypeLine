//! The cheat sheet (📋 in the top bar): every import and everything inside
//! each module on one page. The Help window (F1) explains each command with
//! an example; this is the quick look.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::autocomplete::cheat_sheet;
use super::editor::Workspace;
use super::help::{IMPORTS, example, table};

#[derive(Resource, Default)]
pub struct CheatSheetWindow {
    pub open: bool,
}

pub fn cheat_sheet_window(
    mut contexts: EguiContexts,
    mut window: ResMut<CheatSheetWindow>,
    mut workspace: ResMut<Workspace>,
) -> Result {
    let mut open = window.open;
    egui::Window::new("Cheat sheet")
        .open(&mut open)
        .default_size(egui::vec2(520.0, 560.0))
        .show(contexts.ctx_mut()?, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label("Put these at the top of main.py to use everything below:");
                example(ui, &mut workspace, IMPORTS);
                for (heading, rows) in cheat_sheet() {
                    ui.label(egui::RichText::new(&heading).monospace().strong());
                    table(ui, &heading, rows.into_iter());
                    ui.add_space(8.0);
                }
                ui.weak("Help (F1) explains each one with an example you can insert.");
            });
        });
    window.open = open;
    Ok(())
}
