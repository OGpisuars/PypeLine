//! The bar across the top of the window: Run, files, windows, and the Time
//! Dials. Everything under it is the game area, where the code windows
//! float over the world.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::engine::camera::{GameArea, ViewState};
use crate::factory::tick::SimControl;
use crate::progression::chapters;
use crate::progression::contracts::Progress;
use crate::scripting::files::ScriptStore;
use crate::scripting::{Console, ErrorLine, PendingRun, RunRequests};

use super::editor::Workspace;
use super::help::HelpState;
use super::manual::ManualState;
use super::settings::SettingsWindow;
use super::shop::ShopWindow;
use super::time_dials::time_dials_ui;

/// The windows the top bar opens and closes.
#[derive(SystemParam)]
pub struct Toggles<'w> {
    help: ResMut<'w, HelpState>,
    manual: ResMut<'w, ManualState>,
    settings: ResMut<'w, SettingsWindow>,
    shop: ResMut<'w, ShopWindow>,
    stats: ResMut<'w, super::stats_panel::StatsWindow>,
    view: ResMut<'w, ViewState>,
    debugger: ResMut<'w, super::debugger::Debugger>,
    prefs: ResMut<'w, super::settings::Settings>,
    screen: ResMut<'w, NextState<crate::engine::screens::Screen>>,
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
pub fn top_bar(
    mut contexts: EguiContexts,
    mut workspace: ResMut<Workspace>,
    mut pending: ResMut<PendingRun>,
    mut requests: ResMut<RunRequests>,
    error_line: Res<ErrorLine>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
    mut game_area: ResMut<GameArea>,
    progress: Res<Progress>,
    mut control: ResMut<SimControl>,
    mut audio: ResMut<crate::audio::AudioSettings>,
    mut toggles: Toggles,
    factory: Res<crate::factory::Factory>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let screen = ctx.viewport_rect();
    let mut root = egui::Ui::new(
        ctx.clone(),
        "root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(screen),
    );

    // Ctrl+Enter or F5 runs from anywhere, even while typing.
    let run_key = ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::F5)
    });
    let queued = pending.0.is_some();
    // Some(clean) when Run or Clean Run was pressed.
    let mut run = run_key.then_some(false);

    let bar = egui::Panel::top("top_bar").show(&mut root, |ui| {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            let run_button = ui
                .add_enabled(!queued, egui::Button::new("▶ Run"))
                .on_hover_text("Run main.py (Ctrl+Enter or F5)");
            if run_button.clicked() {
                run = Some(false);
            }
            if ui
                .button("■ Stop")
                .on_hover_text("Halt the belts")
                .clicked()
            {
                requests.stop = true;
            }
            let clean = ui
                .add_enabled(!queued, egui::Button::new("Clean Run"))
                .on_hover_text("Clear the whole factory, then run (coins are kept)");
            if clean.clicked() {
                run = Some(true);
            }
            if ui
                .button("🐞 Debug")
                .on_hover_text(
                    "Record main.py line by line and step through it (F6). \
                     Never changes the factory.",
                )
                .clicked()
            {
                toggles.debugger.requested = true;
            }
            ui.separator();

            files_menu(ui, &mut workspace);
            if ui
                .selectable_label(workspace.console_open, "Console")
                .clicked()
            {
                workspace.console_open = !workspace.console_open;
            }
            snippets_menu(ui, &mut workspace, &progress);
            ui.separator();
            if ui
                .selectable_label(toggles.help.open, "Help (F1)")
                .clicked()
            {
                toggles.help.open = !toggles.help.open;
            }
            if ui
                .selectable_label(toggles.manual.open, "Manual (F2)")
                .clicked()
            {
                toggles.manual.open = !toggles.manual.open;
            }
            if ui
                .selectable_label(toggles.shop.open, "Shop (F3)")
                .clicked()
            {
                toggles.shop.open = !toggles.shop.open;
            }
            if ui
                .selectable_label(toggles.stats.open, "Stats (F4)")
                .clicked()
            {
                toggles.stats.open = !toggles.stats.open;
            }
            if ui
                .selectable_label(toggles.settings.open, "⚙ Settings")
                .clicked()
            {
                toggles.settings.open = !toggles.settings.open;
            }
            ui.menu_button("View", |ui| {
                if ui
                    .button("Center the island (Home)")
                    .on_hover_text("Undo dragging and zooming")
                    .clicked()
                {
                    *toggles.view = ViewState::default();
                    ui.close();
                }
                if ui
                    .button("Reset windows")
                    .on_hover_text("Put the code windows and console back in their places")
                    .clicked()
                {
                    workspace.reset_layout();
                    ui.close();
                }
                let mut bob = toggles.prefs.island_bob;
                if ui
                    .checkbox(&mut bob, "Island bobs up and down")
                    .on_hover_text("Also in Settings")
                    .changed()
                {
                    toggles.prefs.island_bob = bob;
                }
                if ui
                    .button("Title screen")
                    .on_hover_text("Back to the title menu (the factory waits there)")
                    .clicked()
                {
                    toggles.screen.set(crate::engine::screens::Screen::Menu);
                    ui.close();
                }
                ui.separator();
                ui.weak("Drag empty space to move the world.");
                ui.weak("Mouse wheel zooms.");
            });
            ui.separator();
            time_dials_ui(ui, &mut control, &mut audio);
            ui.separator();
            day_clock(ui, factory.ticks);
            if let Some((file, line)) = &error_line.0 {
                ui.separator();
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("problem: {file} line {line}"),
                );
            }
        });
        ui.add_space(2.0);
    });
    if let Some(clean) = run
        && !queued
    {
        requests.clean = clean;
        pending.0 = Some(workspace.program());
        workspace.save_all(store.as_deref(), &mut console);
    }
    let bottom = bar.response.rect.bottom();
    game_area.0 = Some(Rect::new(screen.min.x, bottom, screen.max.x, screen.max.y));
    Ok(())
}

/// The time of day, with a forecast on hover (roadmap: a forecast so the
/// heat is learnable, not punishing).
fn day_clock(ui: &mut egui::Ui, ticks: u64) {
    use crate::factory::daynight;
    let minute = daynight::minute_of_day(ticks);
    let icon = if daynight::is_day(ticks) {
        "☀"
    } else {
        "🌙"
    };
    let next = if daynight::is_day(ticks) {
        "night"
    } else {
        "day"
    };
    ui.label(format!("{icon} {:02}:{:02}", minute / 60, minute % 60))
        .on_hover_text(format!(
            "{}. {next} in {} s. Boilers run hottest around noon (air {} degrees now).",
            daynight::phase(ticks).name(),
            daynight::seconds_until_change(ticks),
            daynight::air_temperature(ticks)
        ));
}

fn files_menu(ui: &mut egui::Ui, workspace: &mut Workspace) {
    ui.menu_button("Files", |ui| {
        for file in &mut workspace.files {
            let label = if file.unsaved() {
                format!("{} ●", file.name)
            } else {
                file.name.clone()
            };
            if ui
                .checkbox(&mut file.open, label)
                .on_hover_text("Show or hide its window")
                .changed()
            {
                ui.close();
            }
        }
        ui.separator();
        if ui.button("＋ New file…").clicked() {
            workspace.open_new_file_dialog();
            ui.close();
        }
        ui.weak("Other files are modules: main.py uses them with import.");
    });
}

/// Starter templates from finished chapters (roadmap: SNIPPETS). A snippet
/// only appears once its chapter is done, so it saves typing but never
/// skips learning.
fn snippets_menu(ui: &mut egui::Ui, workspace: &mut Workspace, progress: &Progress) {
    ui.menu_button("Snippets", |ui| {
        let mut any = false;
        for chapter in chapters::chapters() {
            if !progress.chapter_done(chapter.number) {
                continue;
            }
            for (name, code) in chapter.snippets() {
                any = true;
                if ui.button(format!("{}. {name}", chapter.number)).clicked() {
                    workspace.insert_into_main(code);
                    ui.close();
                }
            }
        }
        if !any {
            ui.label("Finish a chapter to unlock its snippets.");
        }
    });
}
