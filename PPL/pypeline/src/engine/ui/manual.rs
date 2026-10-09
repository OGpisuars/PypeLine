//! The Engineering Manual window (F2): chapters, examples and contracts.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::editor::Workspace;
use crate::factory::Factory;
use crate::progression::chapters::{self, Block, Chapter, Contract};
use crate::progression::contracts::{self, Progress};

#[derive(Resource)]
pub struct ManualState {
    pub open: bool,
    pub chapter: u32,
}

impl Default for ManualState {
    fn default() -> Self {
        // Open on first launch, so new players find it.
        Self {
            open: true,
            chapter: 1,
        }
    }
}

pub fn toggle_manual(
    keys: Res<ButtonInput<KeyCode>>,
    mut manual: ResMut<ManualState>,
    progress: Res<Progress>,
    mut opened_on_load: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F2) {
        manual.open = !manual.open;
    }
    // Once a save is loaded, start on the newest unlocked chapter.
    if !*opened_on_load && progress.is_changed() {
        *opened_on_load = true;
        manual.chapter = chapters::chapters()
            .iter()
            .map(|ch| ch.number)
            .filter(|&n| progress.chapter_unlocked(n))
            .max()
            .unwrap_or(1);
        manual.open = progress.completed.is_empty();
    }
}

pub fn manual_window(
    mut contexts: EguiContexts,
    mut manual: ResMut<ManualState>,
    mut progress: ResMut<Progress>,
    mut workspace: ResMut<Workspace>,
    factory: Res<Factory>,
    history: Res<crate::factory::ProductionHistory>,
) -> Result {
    let rates = history.per_minute(&factory);
    let ctx = contexts.ctx_mut()?;
    let mut open = manual.open;
    let screen = ctx.viewport_rect();
    egui::Window::new("Engineering Manual")
        .open(&mut open)
        .default_pos(egui::pos2(screen.max.x - 580.0, 60.0))
        .default_size(egui::vec2(560.0, 640.0))
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for chapter in chapters::chapters() {
                    let mark = if progress.chapter_done(chapter.number) {
                        "✓"
                    } else if progress.chapter_unlocked(chapter.number) {
                        ""
                    } else {
                        "🔒"
                    };
                    let label = format!("{} {mark}", chapter.number);
                    let selected = manual.chapter == chapter.number;
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(&chapter.title)
                        .clicked()
                    {
                        manual.chapter = chapter.number;
                    }
                }
            });
            ui.separator();
            let Some(chapter) = chapters::chapters()
                .iter()
                .find(|ch| ch.number == manual.chapter)
            else {
                return;
            };
            egui::ScrollArea::vertical()
                .id_salt("manual_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading(&chapter.title);
                    if progress.chapter_unlocked(chapter.number) {
                        for block in &chapter.blocks {
                            show_block(ui, block, &mut workspace);
                        }
                    } else {
                        ui.label(format!(
                            "🔒 Finish chapter {} to unlock this one. Already know this? \
                             Pass its chapter test below to skip ahead.",
                            chapter.number - 1
                        ));
                    }
                    ui.add_space(12.0);
                    ui.heading("Contracts");
                    for contract in &chapter.contracts {
                        show_contract(ui, chapter, contract, &mut progress, &factory, &rates);
                    }
                });
        });
    manual.open = open;
    Ok(())
}

fn show_block(ui: &mut egui::Ui, block: &Block, workspace: &mut Workspace) {
    match block {
        Block::Heading(text) => {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(text).strong().size(17.0));
        }
        Block::Text(text) => {
            for line in text.lines() {
                if line.starts_with('|') {
                    // Tables are shown as aligned monospace text.
                    if !line.contains("---") {
                        ui.label(egui::RichText::new(line.replace('`', "")).monospace());
                    }
                } else if let Some(item) = line.strip_prefix("- ") {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("•");
                        inline(ui, item);
                    });
                } else {
                    ui.horizontal_wrapped(|ui| inline(ui, line));
                }
            }
            ui.add_space(4.0);
        }
        Block::Code {
            code,
            snippet,
            file,
        } => {
            egui::Frame::new()
                .fill(ui.visuals().code_bg_color)
                .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(code.trim_end()).monospace());
                });
            ui.horizontal(|ui| {
                // A whole file of its own, which main.py imports.
                if let Some(name) = file {
                    let exists = workspace.files.iter().any(|f| f.name == *name);
                    let label = if exists {
                        format!("Replace {name}")
                    } else {
                        format!("Create {name}")
                    };
                    if ui.small_button(label).clicked() {
                        workspace.put_file(name, code);
                    }
                    return;
                }
                if ui.small_button("Insert into main.py").clicked() {
                    workspace.insert_into_main(code);
                }
                // Whole examples can replace the script outright.
                if code.contains("import") && ui.small_button("Use as main.py").clicked() {
                    let main = workspace.main();
                    main.source = code.clone();
                    main.open = true;
                }
                if let Some(name) = snippet {
                    ui.weak(format!("snippet: {name}"));
                }
            });
            ui.add_space(6.0);
        }
    }
}

/// A line of manual text: `code` in monospace and **bold** in bold. Code
/// sits on the theme's button color, which every theme keeps readable
/// under its text color.
fn inline(ui: &mut egui::Ui, line: &str) {
    ui.spacing_mut().item_spacing.x = 0.0;
    let chip = ui.visuals().widgets.inactive.bg_fill;
    for (i, part) in line.split('`').enumerate() {
        if i % 2 == 1 {
            ui.label(egui::RichText::new(part).monospace().background_color(chip));
        } else {
            for (j, piece) in part.split("**").enumerate() {
                let text = egui::RichText::new(piece);
                ui.label(if j % 2 == 1 { text.strong() } else { text });
            }
        }
    }
}

fn show_contract(
    ui: &mut egui::Ui,
    chapter: &Chapter,
    contract: &Contract,
    progress: &mut Progress,
    factory: &Factory,
    rates: &std::collections::BTreeMap<crate::factory::items::ItemKind, u64>,
) {
    let done = progress.completed.contains(&contract.id);
    let active = progress
        .active
        .as_ref()
        .filter(|a| a.id == contract.id)
        .cloned();
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&contract.title).strong());
            if contract.chapter_test {
                ui.weak("★ chapter test");
            }
            if done {
                ui.colored_label(super::done_green(ui), "✓ done");
            }
        });
        ui.label(&contract.brief);
        ui.label(format!("Goal: {}", contract.goal.describe()));
        if contract.keep_cool {
            ui.label("Keep cool: if a boiler overheats, the contract starts over.");
        }
        if !contract.requires.is_empty() {
            let needs: Vec<&str> = contract.requires.iter().map(|c| c.describe()).collect();
            ui.label(format!("Must use: {}", needs.join(", ")));
        }
        if let Some(max) = contract.max_lines {
            ui.label(format!("At most {max} lines of code"));
        }
        ui.label(format!("Reward: {} coins", contract.reward));
        if let Some(best) = progress.best.get(&contract.id) {
            ui.weak(format!(
                "Best: {} lines, {} steam, {} s",
                best.lines, best.steam, best.seconds
            ));
        }

        ui.horizontal(|ui| {
            if let Some(active) = &active {
                let (have, need) = contracts::goal_progress(&contract.goal, active, factory, rates);
                ui.add(
                    egui::ProgressBar::new(have as f32 / need as f32)
                        .desired_width(200.0)
                        .text(format!("{have} / {need}")),
                );
                if ui.small_button("Abandon").clicked() {
                    progress.active = None;
                }
            } else if progress.can_accept(chapter, contract) {
                let label = if progress.chapter_unlocked(chapter.number) {
                    "Accept"
                } else {
                    "Test out"
                };
                if ui.button(label).clicked() {
                    progress.accept(contract, factory);
                }
            }
        });

        // Hints open one at a time: nudge, bigger hint, nearly the answer.
        let seen = progress.hints_seen.get(&contract.id).copied().unwrap_or(0);
        for hint in contract.hints.iter().take(seen) {
            ui.label(egui::RichText::new(format!("💡 {hint}")).italics());
        }
        if !done && seen < contract.hints.len() {
            let label = format!("Show hint ({}/{})", seen + 1, contract.hints.len());
            if ui.small_button(label).clicked() {
                progress.hints_seen.insert(contract.id.clone(), seen + 1);
            }
        }
    });
    ui.add_space(6.0);
}
