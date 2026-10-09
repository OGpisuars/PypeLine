//! The recipe picker: click a crafter on the island (press and let go
//! without dragging) to choose what it makes. Locked recipes say which
//! achievement unlocks them.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, egui};

use crate::engine::camera::HoveredTile;
use crate::factory::Factory;
use crate::factory::machines::MachineKind;
use crate::factory::recipes::Recipe;
use crate::progression::achievements;
use crate::progression::contracts::Progress;
use crate::scripting::{Console, ConsoleKind};

/// How far the mouse may move between press and release and still count
/// as a click rather than dragging the world.
const CLICK_SLOP: f32 = 5.0;

/// The crafter whose recipe picker is open, by machine name.
#[derive(Resource, Default)]
pub struct RecipePicker {
    pub machine: Option<String>,
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
pub fn recipe_picker(
    mut contexts: EguiContexts,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    hovered: Res<HoveredTile>,
    mut factory: ResMut<Factory>,
    progress: Res<Progress>,
    mut picker: ResMut<RecipePicker>,
    mut console: ResMut<Console>,
    mut pressed_at: Local<Option<Vec2>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let cursor = window.cursor_position();
    if buttons.just_pressed(MouseButton::Left) {
        *pressed_at = if ctx.is_pointer_over_egui() {
            None
        } else {
            cursor
        };
    }
    if buttons.just_released(MouseButton::Left)
        && let (Some(start), Some(now), Some(tile)) = (pressed_at.take(), cursor, hovered.0)
        && start.distance(now) < CLICK_SLOP
        && let Some(name) = factory.machine_at.get(&tile)
        && factory.machines[name].kind == MachineKind::Crafter
    {
        picker.machine = Some(name.clone());
    }

    let Some(name) = picker.machine.clone() else {
        return Ok(());
    };
    let Some(current) = factory.machines.get(&name).map(|m| m.recipe) else {
        // The crafter was removed by a Run.
        picker.machine = None;
        return Ok(());
    };
    let unlocked = progress.recipes();
    let mut open = true;
    let mut chosen = None;
    egui::Window::new(format!("Crafter: {name}"))
        .id(egui::Id::new("recipe_picker"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_pos(
            window
                .cursor_position()
                .map_or(egui::pos2(300.0, 200.0), |c| egui::pos2(c.x + 24.0, c.y)),
        )
        .show(ctx, |ui| {
            ui.label("What should it make? It takes ingredients from any side and sends");
            ui.label("what it makes out of its brass mark.");
            ui.add_space(6.0);
            for recipe in Recipe::ALL {
                let locked = !unlocked.contains(&recipe);
                let selected = current == Some(recipe);
                let title = if locked {
                    format!("🔒 {}", recipe.output().name())
                } else {
                    recipe.output().name().to_owned()
                };
                let button = ui.add_enabled(
                    !locked,
                    egui::Button::selectable(selected, egui::RichText::new(title).strong()),
                );
                if button.clicked() && !selected {
                    chosen = Some(recipe);
                }
                ui.weak(recipe.describe());
                if locked && let Some(achievement) = achievements::unlocked_by(recipe) {
                    ui.weak(format!(
                        "Unlock: achievement \"{}\" ({})",
                        achievement.title, achievement.goal
                    ));
                }
                ui.add_space(4.0);
            }
            ui.separator();
            ui.weak("Scripts can choose too: recipe=\"iron_gear\" in machines.place.");
            ui.weak("A recipe= in your script wins when you press Run.");
        });
    if let Some(recipe) = chosen
        && let Some(machine) = factory.machines.get_mut(&name)
    {
        let held = machine.set_recipe(Some(recipe));
        factory.stash(held);
        // Redraw it with its new recipe showing.
        factory.layout_version += 1;
        console.push(
            ConsoleKind::Info,
            format!("{name} now makes {}.", recipe.describe()),
        );
    }
    if !open {
        picker.machine = None;
    }
    Ok(())
}
