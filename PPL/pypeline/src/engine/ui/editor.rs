//! Code editor window for main.py with the Run button.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::scripting::files::ScriptStore;
use crate::scripting::{Console, ConsoleKind, ErrorLine, PendingRun};

use super::help::HelpState;
use super::highlight;
use super::{palette, rgb};

/// The first script a new player sees: the roadmap's canonical sample.
const STARTER_SCRIPT: &str = crate::scripting::CANONICAL_SAMPLE;

/// Seconds between autosaves while the script has unsaved changes.
const AUTOSAVE_SECS: f32 = 5.0;

#[derive(Resource)]
pub struct EditorState {
    pub source: String,
    /// What is on disk, to know when there are unsaved changes.
    pub saved: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            source: STARTER_SCRIPT.to_owned(),
            saved: String::new(),
        }
    }
}

/// Load main.py from the player's script folder at startup.
pub fn load_script(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    mut console: ResMut<Console>,
) {
    let Some(store) = ScriptStore::default_location() else {
        console.push(
            ConsoleKind::Error,
            "No folder to save scripts in; changes will not be kept.",
        );
        return;
    };
    match store.load_main() {
        Ok(Some(source)) => {
            state.saved = source.clone();
            state.source = source;
        }
        Ok(None) => {}
        Err(err) => console.push(ConsoleKind::Error, format!("Could not load main.py: {err}")),
    }
    console.push(
        ConsoleKind::Info,
        format!("Scripts are saved in {}", store.main_path().display()),
    );
    commands.insert_resource(store);
}

fn save(state: &mut EditorState, store: Option<&ScriptStore>, console: &mut Console) {
    let Some(store) = store else { return };
    if state.source == state.saved {
        return;
    }
    match store.save_main(&state.source) {
        Ok(()) => state.saved = state.source.clone(),
        Err(err) => console.push(ConsoleKind::Error, format!("Could not save main.py: {err}")),
    }
}

/// Save every few seconds while there are unsaved changes, and on exit.
pub fn autosave(
    time: Res<Time>,
    mut since: Local<f32>,
    mut state: ResMut<EditorState>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
    mut exit: MessageReader<AppExit>,
) {
    *since += time.delta_secs();
    let exiting = exit.read().count() > 0;
    if exiting || *since >= AUTOSAVE_SECS {
        *since = 0.0;
        save(&mut state, store.as_deref(), &mut console);
    }
}

pub fn editor_window(
    mut contexts: EguiContexts,
    mut state: ResMut<EditorState>,
    mut pending: ResMut<PendingRun>,
    error_line: Res<ErrorLine>,
    mut help: ResMut<HelpState>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
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
                    save(&mut state, store.as_deref(), &mut console);
                }
                if ui.button("? Help (F1)").clicked() {
                    help.open = !help.open;
                }
                if let Some(line) = error_line.0 {
                    ui.colored_label(rgb(palette::UI_ERROR), format!("problem on line {line}"));
                }
                if state.source != state.saved {
                    ui.weak("● unsaved");
                }
            });
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            let marked = error_line.0;
            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let mut job = highlight::layout(buf.as_str(), font.clone(), marked);
                job.wrap.max_width = wrap_width;
                ui.fonts_mut(|f| f.layout_job(job))
            };
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.source)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .layouter(&mut layouter),
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
