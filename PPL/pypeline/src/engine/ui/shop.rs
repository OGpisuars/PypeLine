//! The Shop window (F3): spend train coins on faster belts and machines.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::factory::Factory;
use crate::factory::shop::{self, Upgrade};
use crate::scripting::{Console, ConsoleKind};

use super::editor::Workspace;

#[derive(Resource, Default)]
pub struct ShopWindow {
    pub open: bool,
}

pub fn toggle_shop(keys: Res<ButtonInput<KeyCode>>, mut shop: ResMut<ShopWindow>) {
    if keys.just_pressed(KeyCode::F3) {
        shop.open = !shop.open;
    }
}

pub fn shop_window(
    mut contexts: EguiContexts,
    mut window: ResMut<ShopWindow>,
    mut factory: ResMut<Factory>,
    mut workspace: ResMut<Workspace>,
    mut console: ResMut<Console>,
    mut sounds: MessageWriter<crate::audio::SoundCue>,
) -> Result {
    let mut open = window.open;
    let mut bought = None;
    egui::Window::new("Shop")
        .open(&mut open)
        .default_pos(egui::pos2(520.0, 80.0))
        .default_size(egui::vec2(430.0, 520.0))
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading(format!("🪙 {} coins", factory.coins));
            ui.label(
                "The cargo train pays for what reaches your stations. Upgrades unlock a \
                 higher tier= in your script; nothing changes until you use it.",
            );
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for upgrade in Upgrade::ALL {
                    offer(ui, &factory, upgrade, &mut bought, &mut workspace);
                    ui.add_space(6.0);
                }
                ui.separator();
                ui.label(egui::RichText::new("Coming later: Prestige").strong());
                ui.weak(
                    "Start a fresh factory with a permanent bonus. Each prestige \
                     swaps Python for a new language, and each one is fussier than \
                     the last.",
                );
            });
        });
    window.open = open;
    if let Some(upgrade) = bought {
        match factory.buy(upgrade) {
            Ok(()) => {
                console.push(
                    ConsoleKind::Info,
                    format!("Bought {}! Try: {}", upgrade.title(), upgrade.example()),
                );
                sounds.write(crate::audio::SoundCue(crate::audio::sfx::Sfx::Sale));
            }
            Err(reason) => console.push(
                ConsoleKind::Error,
                format!("Could not buy {}: {reason}", upgrade.title()),
            ),
        }
    }
    Ok(())
}

fn offer(
    ui: &mut egui::Ui,
    factory: &Factory,
    upgrade: Upgrade,
    bought: &mut Option<Upgrade>,
    workspace: &mut Workspace,
) {
    let owned = factory.unlocked.contains(&upgrade);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(upgrade.title()).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if owned {
                    ui.label("✔ owned");
                    return;
                }
                let problem = shop::cannot_buy(factory, upgrade);
                let button = ui
                    .add_enabled(
                        problem.is_none(),
                        egui::Button::new(format!("Buy for {}", upgrade.price())),
                    )
                    .on_disabled_hover_text(problem.unwrap_or_default());
                if button.clicked() {
                    *bought = Some(upgrade);
                }
            });
        });
        ui.label(upgrade.about());
        if let Some(first) = upgrade.requires()
            && !factory.unlocked.contains(&first)
        {
            ui.weak(format!("Needs {} first.", first.title()));
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(upgrade.example()).monospace().small());
            if owned
                && ui
                    .small_button("Insert")
                    .on_hover_text("Add this line to main.py")
                    .clicked()
            {
                workspace.insert_into_main(&format!("{}\n", upgrade.example()));
            }
        });
    });
}
