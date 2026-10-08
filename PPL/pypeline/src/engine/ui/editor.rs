//! Code editor window for main.py with the Run button.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::engine::camera::GameArea;
use crate::scripting::files::ScriptStore;
use crate::scripting::{Console, ConsoleKind, ErrorLine, PendingRun, RunRequests};

use super::console::console_ui;

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
    /// Code panel hidden, so the game gets the whole window.
    pub hidden: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            source: STARTER_SCRIPT.to_owned(),
            saved: String::new(),
            hidden: false,
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

/// Height of the console under the editor, in points.
const CONSOLE_HEIGHT: f32 = 190.0;

/// The code panel docked on the left: main.py on top, the console below.
/// Whatever space is left becomes the game area.
#[allow(clippy::too_many_arguments)]
pub fn code_panel(
    mut contexts: EguiContexts,
    mut state: ResMut<EditorState>,
    mut pending: ResMut<PendingRun>,
    error_line: Res<ErrorLine>,
    mut help: ResMut<HelpState>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
    mut game_area: ResMut<GameArea>,
    mut requests: ResMut<RunRequests>,
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
    if state.hidden {
        // Just a small button to bring the panel back.
        egui::Area::new(egui::Id::new("show_code"))
            .fixed_pos(screen.min + egui::vec2(8.0, 8.0))
            .show(&ctx, |ui| {
                if ui
                    .button("▶ Code")
                    .on_hover_text("Show the code panel")
                    .clicked()
                {
                    state.hidden = false;
                }
            });
        game_area.0 = Some(Rect::new(
            screen.min.x,
            screen.min.y,
            screen.max.x,
            screen.max.y,
        ));
        return Ok(());
    }
    let default_width = (screen.width() * 0.4).clamp(320.0, 640.0);
    let panel = egui::Panel::left("code_panel")
        .resizable(true)
        .default_size(default_width)
        .min_size(280.0)
        .show(&mut root, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button("◀")
                    .on_hover_text("Hide the code panel")
                    .clicked()
                {
                    state.hidden = true;
                }
                ui.strong("main.py");
                let queued = pending.0.is_some();
                let run = ui.add_enabled(!queued, egui::Button::new("▶ Run"));
                if run.clicked() {
                    pending.0 = Some(state.source.clone());
                    save(&mut state, store.as_deref(), &mut console);
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
                    requests.clean = true;
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
            let editor_height = (ui.available_height() - CONSOLE_HEIGHT).max(120.0);
            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .max_height(editor_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), editor_height],
                        egui::TextEdit::multiline(&mut state.source)
                            .code_editor()
                            .layouter(&mut layouter),
                    );
                });
            ui.separator();
            console_ui(ui, &mut console);
        });
    let left = panel.response.rect.right();
    game_area.0 = Some(Rect::new(left, screen.min.y, screen.max.x, screen.max.y));
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
