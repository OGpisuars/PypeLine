//! The line debugger (roadmap Phase 3B, Part 4 F): a Debug Run records
//! main.py line by line, then this window steps through the recording,
//! forwards and backwards, with the current line marked in its code window
//! and a watch list of the variables.
//!
//! A Debug Run uses its own interpreter and never changes the factory, so
//! it is always safe to press. Keys while the window is open and you are
//! not typing: F7 = back one line, F8 = forward one line.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::factory::{Factory, ProductionHistory};
use crate::scripting::budget::DEPLOY_BUDGET;
use crate::scripting::operate::WorldView;
use crate::scripting::runtime::{RunOutcome, ScriptRuntime};
use crate::scripting::trace::{MAX_STEPS, TraceStep};

use super::editor::Workspace;

/// The interpreter Debug Runs use, kept apart from the game's so a Debug Run
/// never replaces the running script's tick() and events.
pub struct DebugRuntime(pub ScriptRuntime);

/// One recorded Debug Run.
pub struct Recording {
    pub steps: Vec<TraceStep>,
    pub truncated: bool,
    pub output: Vec<String>,
    pub outcome: RunOutcome,
    pub error_file: Option<String>,
}

#[derive(Resource, Default)]
pub struct Debugger {
    pub open: bool,
    /// Set by the Debug button; the run happens on the next frame.
    pub requested: bool,
    pub recording: Option<Recording>,
    pub step: usize,
}

impl Debugger {
    /// The file and line the debugger is on, for the code windows.
    pub fn current_line(&self) -> Option<(String, usize)> {
        if !self.open {
            return None;
        }
        let step = self.recording.as_ref()?.steps.get(self.step)?;
        Some((step.file.clone(), step.line))
    }

    fn last(&self) -> usize {
        self.recording
            .as_ref()
            .map_or(0, |r| r.steps.len().saturating_sub(1))
    }
}

pub fn debug_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
    mut debugger: ResMut<Debugger>,
) -> Result {
    if keys.just_pressed(KeyCode::F6) {
        debugger.requested = true;
    }
    if !debugger.open || contexts.ctx_mut()?.egui_wants_keyboard_input() {
        return Ok(());
    }
    if keys.just_pressed(KeyCode::F7) {
        debugger.step = debugger.step.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::F8) {
        debugger.step = (debugger.step + 1).min(debugger.last());
    }
    Ok(())
}

/// Do a requested Debug Run: record the whole program against a snapshot
/// of the factory, so sensors and stats read real values.
pub fn run_debugger(
    mut debugger: ResMut<Debugger>,
    workspace: Res<Workspace>,
    factory: Res<Factory>,
    history: Res<ProductionHistory>,
    runtime: NonSend<DebugRuntime>,
) {
    if !std::mem::take(&mut debugger.requested) {
        return;
    }
    let runtime = &runtime.0;
    runtime.set_world(WorldView::of(&factory, history.per_minute(&factory)));
    let (report, steps, truncated) = runtime.debug_program(&workspace.program(), DEPLOY_BUDGET);
    debugger.recording = Some(Recording {
        steps,
        truncated,
        output: report.output,
        outcome: report.outcome,
        error_file: report.error_file,
    });
    debugger.step = 0;
    debugger.open = true;
}

pub fn debugger_window(mut contexts: EguiContexts, mut debugger: ResMut<Debugger>) -> Result {
    if !debugger.open {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.viewport_rect();
    let mut open = true;
    let mut step = debugger.step;
    let mut rerun = false;
    let last = debugger.last();
    egui::Window::new("Debugger")
        .open(&mut open)
        .default_pos(egui::pos2(500.0, screen.max.y - 420.0))
        .default_size(egui::vec2(400.0, 380.0))
        .show(ctx, |ui| {
            let Some(recording) = &debugger.recording else {
                ui.label("Press Debug (F6) to record a run of main.py.");
                return;
            };
            if recording.steps.is_empty() {
                ui.label("Nothing ran: the script stopped before its first line.");
                outcome(ui, recording);
                return;
            }
            ui.horizontal(|ui| {
                if ui.button("⏮").on_hover_text("First line").clicked() {
                    step = 0;
                }
                if ui.button("◀ Back").on_hover_text("F7").clicked() {
                    step = step.saturating_sub(1);
                }
                if ui.button("Step ▶").on_hover_text("F8").clicked() {
                    step = (step + 1).min(last);
                }
                if ui.button("⏭").on_hover_text("Last line").clicked() {
                    step = last;
                }
                if ui
                    .button("⟳ Record again")
                    .on_hover_text("Debug the current code (F6)")
                    .clicked()
                {
                    rerun = true;
                }
            });
            ui.add(egui::Slider::new(&mut step, 0..=last).show_value(false));
            let here = &recording.steps[step];
            let place = if here.function == "<module>" {
                format!("{} line {}", here.file, here.line)
            } else {
                format!("{} line {}, in {}()", here.file, here.line, here.function)
            };
            ui.label(egui::RichText::new(place).strong());
            ui.weak(format!(
                "step {} of {}{}  |  steam used {}",
                step + 1,
                recording.steps.len(),
                if recording.truncated {
                    format!(" (recorded the first {MAX_STEPS} lines)")
                } else {
                    String::new()
                },
                here.steam
            ));
            ui.horizontal(|ui| {
                let same = |s: &TraceStep| s.file == here.file && s.line == here.line;
                if let Some(back) = recording.steps[..step].iter().rposition(same)
                    && ui.small_button("◀ last time here").clicked()
                {
                    step = back;
                }
                if let Some(ahead) = recording.steps[step + 1..].iter().position(same)
                    && ui.small_button("next time here ▶").clicked()
                {
                    step += 1 + ahead;
                }
            });
            ui.separator();

            ui.strong("Variables (before this line runs)");
            let before = step.checked_sub(1).map(|i| &recording.steps[i]);
            if here.vars.is_empty() {
                ui.weak("none yet");
            }
            egui::ScrollArea::vertical()
                .id_salt("debug_vars")
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("debug_vars_grid")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, value) in &here.vars {
                                let changed = before.is_some_and(|b| {
                                    b.function == here.function
                                        && !b.vars.iter().any(|(n, v)| n == name && v == value)
                                });
                                ui.label(egui::RichText::new(name).monospace());
                                let text = egui::RichText::new(value).monospace();
                                if changed {
                                    ui.label(text.strong().color(ui.visuals().warn_fg_color))
                                        .on_hover_text("changed on the line before");
                                } else {
                                    ui.label(text);
                                }
                                ui.end_row();
                            }
                        });
                });
            ui.separator();

            ui.strong("Printed so far");
            let printed = &recording.output[..here.printed.min(recording.output.len())];
            egui::ScrollArea::vertical()
                .id_salt("debug_output")
                .max_height(90.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if printed.is_empty() {
                        ui.weak("nothing yet");
                    }
                    for line in printed {
                        ui.label(egui::RichText::new(line).monospace());
                    }
                });
            if step == last {
                ui.separator();
                outcome(ui, recording);
            }
        });
    debugger.step = step;
    if rerun {
        debugger.requested = true;
    }
    if !open {
        debugger.open = false;
    }
    Ok(())
}

/// How the recorded run ended.
fn outcome(ui: &mut egui::Ui, recording: &Recording) {
    let file = recording.error_file.as_deref().unwrap_or("main.py");
    match &recording.outcome {
        RunOutcome::Finished => {
            ui.label("The run finished. (A Debug Run never changes the factory.)");
        }
        RunOutcome::Error { line, message } => {
            let at = line.map(|l| format!(", line {l}")).unwrap_or_default();
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Error ({file}{at}): {message}"),
            );
        }
        RunOutcome::OutOfSteam { line } => {
            let at = line.map(|l| format!(", line {l}")).unwrap_or_default();
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Out of steam ({file}{at})"),
            );
        }
    }
}
