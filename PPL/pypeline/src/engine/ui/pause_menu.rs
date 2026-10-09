//! The pause menu (Esc, or ☰ Menu in the top bar): Resume, Settings, back
//! to the title, or quit. The factory pauses while it is open and goes back
//! to how it was on Resume.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::main_menu::{menu_box, menu_button};
use super::settings::SettingsWindow;
use crate::engine::screens::Screen;
use crate::factory::tick::SimControl;
use crate::progression::saves::SaveNow;

#[derive(Resource, Default)]
pub struct PauseMenu {
    pub open: bool,
    /// Whether the menu has paused the factory (it is shown).
    shown: bool,
    /// Was the factory already paused before the menu opened?
    was_paused: bool,
    /// Settings was opened from this menu, so closing it comes back here.
    in_settings: bool,
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
pub fn pause_menu(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<PauseMenu>,
    mut control: ResMut<SimControl>,
    mut settings: ResMut<SettingsWindow>,
    mut screen: ResMut<NextState<Screen>>,
    mut save_now: ResMut<SaveNow>,
    mut exit: MessageWriter<AppExit>,
    mut was_typing: Local<bool>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    // Esc that only leaves a text box (or closes autocomplete) is not for us.
    let typing = ctx.egui_wants_keyboard_input();
    let esc = keys.just_pressed(KeyCode::Escape) && !typing && !*was_typing;
    *was_typing = typing;

    if menu.in_settings {
        if esc {
            settings.open = false;
        }
        if settings.open {
            return Ok(());
        }
        // Settings closed: back to the menu.
        menu.in_settings = false;
    } else if esc && settings.open && !menu.open {
        // Esc closes Settings opened from the top bar first.
        settings.open = false;
    } else if esc {
        menu.open = !menu.open;
    }

    if menu.open && !menu.shown {
        menu.was_paused = control.paused;
        control.paused = true;
        menu.shown = true;
    }
    if !menu.open {
        if menu.shown {
            control.paused = menu.was_paused;
            menu.shown = false;
        }
        return Ok(());
    }

    let screen_rect = ctx.viewport_rect();
    let mut choice = None;
    egui::Area::new(egui::Id::new("pause_menu"))
        // Above the top bar, which is in the foreground itself.
        .order(egui::Order::Tooltip)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            // Dim the game, and keep clicks from reaching it.
            ui.interact(
                screen_rect,
                egui::Id::new("pause_backdrop"),
                egui::Sense::click(),
            );
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(150));
            let column = egui::Rect::from_min_max(
                egui::pos2(screen_rect.min.x, screen_rect.center().y - 190.0),
                screen_rect.max,
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(column)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
                |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    menu_box(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("PAUSED").size(26.0).strong());
                            ui.add_space(12.0);
                            if menu_button(ui, "▶  Resume").on_hover_text("Esc").clicked() {
                                choice = Some(Choice::Resume);
                            }
                            ui.add_space(10.0);
                            if menu_button(ui, "⚙  Settings").clicked() {
                                choice = Some(Choice::Settings);
                            }
                            ui.add_space(10.0);
                            if menu_button(ui, "🏠  Title screen")
                                .on_hover_text("Saves first")
                                .clicked()
                            {
                                choice = Some(Choice::Title);
                            }
                            ui.add_space(10.0);
                            if menu_button(ui, "✖  Quit game")
                                .on_hover_text("Saves first")
                                .clicked()
                            {
                                choice = Some(Choice::Quit);
                            }
                        });
                    });
                },
            );
        });

    match choice {
        Some(Choice::Resume) => menu.open = false,
        Some(Choice::Settings) => {
            menu.in_settings = true;
            settings.open = true;
        }
        Some(Choice::Title) => {
            menu.open = false;
            save_now.0 = true;
            screen.set(Screen::Menu);
        }
        // The autosave sees AppExit and saves on the way out.
        Some(Choice::Quit) => {
            exit.write(AppExit::Success);
        }
        None => {}
    }
    Ok(())
}

enum Choice {
    Resume,
    Settings,
    Title,
    Quit,
}
