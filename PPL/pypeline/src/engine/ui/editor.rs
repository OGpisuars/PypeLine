//! Code editor window for main.py with the Run button.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::scripting::{ErrorLine, PendingRun};

use super::help::HelpState;
use super::{palette, rgb};

/// The first script a new player sees: the roadmap's canonical sample.
const STARTER_SCRIPT: &str = crate::scripting::CANONICAL_SAMPLE;

#[derive(Resource)]
pub struct EditorState {
    pub source: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            source: STARTER_SCRIPT.to_owned(),
        }
    }
}

pub fn editor_window(
    mut contexts: EguiContexts,
    mut state: ResMut<EditorState>,
    mut pending: ResMut<PendingRun>,
    error_line: Res<ErrorLine>,
    mut help: ResMut<HelpState>,
) -> Result {
    egui::Window::new("main.py")
        .default_pos(egui::pos2(16.0, 48.0))
        .default_size(egui::vec2(460.0, 420.0))
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal(|ui| {
                let queued = pending.0.is_some();
                let run = ui.add_enabled(!queued, egui::Button::new("▶ Run"));
                if run.clicked() {
                    pending.0 = Some(state.source.clone());
                }
                if ui.button("? Help (F1)").clicked() {
                    help.open = !help.open;
                }
                if let Some(line) = error_line.0 {
                    ui.colored_label(rgb(palette::UI_ERROR), format!("problem on line {line}"));
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.source)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                );
            });
        });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::STARTER_SCRIPT;
    use crate::scripting::{
        budget::DEPLOY_BUDGET,
        runtime::{RunOutcome, ScriptRuntime},
    };

    #[test]
    fn starter_script_runs() {
        let report = ScriptRuntime::new().run(STARTER_SCRIPT, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
        assert!(report.plan.is_some_and(|plan| !plan.machines.is_empty()));
    }
}
