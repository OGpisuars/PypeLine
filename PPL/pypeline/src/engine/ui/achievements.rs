//! The Achievements window: every achievement, which are earned, and the
//! crafter recipes they unlock.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::progression::achievements::ALL;
use crate::progression::contracts::Progress;

#[derive(Resource, Default)]
pub struct AchievementsWindow {
    pub open: bool,
}

pub fn achievements_window(
    mut contexts: EguiContexts,
    mut window: ResMut<AchievementsWindow>,
    progress: Res<Progress>,
) -> Result {
    let mut open = window.open;
    egui::Window::new("Achievements")
        .open(&mut open)
        .default_size(egui::vec2(420.0, 460.0))
        .show(contexts.ctx_mut()?, |ui| {
            let earned = ALL
                .iter()
                .filter(|a| progress.achievements.contains(a.id))
                .count();
            ui.label(format!(
                "{earned} of {} earned. Some unlock new crafter recipes.",
                ALL.len()
            ));
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for achievement in ALL {
                    let done = progress.achievements.contains(achievement.id);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(if done { "🏆" } else { "🔒" });
                            ui.label(egui::RichText::new(achievement.title).strong());
                            if done {
                                ui.colored_label(super::done_green(ui), "✓ earned");
                            }
                        });
                        ui.label(achievement.goal);
                        if let Some(recipe) = achievement.unlocks {
                            ui.weak(format!("Unlocks the crafter recipe: {}", recipe.describe()));
                        }
                    });
                    ui.add_space(4.0);
                }
            });
        });
    window.open = open;
    Ok(())
}
