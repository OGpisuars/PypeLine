//! Code editor window for main.py with the Run button.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::engine::camera::GameArea;
use crate::scripting::files::ScriptStore;
use crate::scripting::{Console, ConsoleKind, ErrorLine, PendingRun, RunRequests};

use super::autocomplete::{self, Suggestions};
use super::console::console_ui;
use super::manual::ManualState;
use crate::progression::chapters;
use crate::progression::contracts::Progress;

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
    /// Highlighted autocomplete suggestion.
    pub pick: usize,
    /// Esc closed the suggestions; they stay closed until the text changes.
    pub dismissed_for: Option<String>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            source: STARTER_SCRIPT.to_owned(),
            saved: String::new(),
            hidden: false,
            pick: 0,
            dismissed_for: None,
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
    mut manual: ResMut<ManualState>,
    progress: Res<Progress>,
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
            ui.horizontal_wrapped(|ui| {
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
                if ui.button("📖 Manual (F2)").clicked() {
                    manual.open = !manual.open;
                }
                snippets_menu(ui, &mut state, &progress);
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
            let id = egui::Id::new(EDITOR_ID);
            let suggestions = handle_completion_keys(ui, id, &mut state);
            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .max_height(editor_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let output = egui::TextEdit::multiline(&mut state.source)
                        .id(id)
                        .code_editor()
                        .layouter(&mut layouter)
                        .desired_width(f32::INFINITY)
                        .min_size(egui::vec2(ui.available_width(), editor_height))
                        .show(ui);
                    if let (Some(suggestions), Some(range)) = (suggestions, output.cursor_range) {
                        let at = output.galley_pos
                            + output
                                .galley
                                .pos_from_cursor(range.primary)
                                .left_bottom()
                                .to_vec2();
                        completion_popup(ui, id, at, &suggestions, &mut state);
                    }
                });
            ui.separator();
            console_ui(ui, &mut console);
        });
    let left = panel.response.rect.right();
    game_area.0 = Some(Rect::new(left, screen.min.y, screen.max.x, screen.max.y));
    Ok(())
}

/// Starter templates from finished chapters (roadmap: SNIPPETS). A snippet
/// only appears once its chapter is done, so it saves typing but never
/// skips learning.
fn snippets_menu(ui: &mut egui::Ui, state: &mut EditorState, progress: &Progress) {
    ui.menu_button("Snippets", |ui| {
        let mut any = false;
        for chapter in chapters::chapters() {
            if !progress.chapter_done(chapter.number) {
                continue;
            }
            for (name, code) in chapter.snippets() {
                any = true;
                if ui.button(format!("{}. {name}", chapter.number)).clicked() {
                    if !state.source.ends_with('\n') {
                        state.source.push('\n');
                    }
                    state.source.push_str(code);
                    ui.close();
                }
            }
        }
        if !any {
            ui.label("Finish a chapter to unlock its snippets.");
        }
    });
}

/// The editor's egui id, so autocomplete can read and move its cursor.
const EDITOR_ID: &str = "main_py_editor";

/// The cursor position (in characters) of the editor, if it has focus.
fn editor_cursor(ctx: &egui::Context, id: egui::Id) -> Option<usize> {
    if !ctx.memory(|m| m.has_focus(id)) {
        return None;
    }
    egui::text_edit::TextEditState::load(ctx, id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.primary.index.0)
}

/// Replace the `replace` characters before `cursor` with `insert`; returns
/// the new cursor position.
fn apply_completion(source: &mut String, cursor: usize, replace: usize, insert: &str) -> usize {
    let start_char = cursor.saturating_sub(replace);
    let byte = |chars: usize| {
        source
            .char_indices()
            .nth(chars)
            .map_or(source.len(), |(i, _)| i)
    };
    let (start, end) = (byte(start_char), byte(cursor));
    source.replace_range(start..end, insert);
    start_char + insert.chars().count()
}

fn accept(
    ctx: &egui::Context,
    id: egui::Id,
    state: &mut EditorState,
    cursor: usize,
    replace: usize,
    insert: &str,
) {
    let new_cursor = apply_completion(&mut state.source, cursor, replace, insert);
    if let Some(mut edit) = egui::text_edit::TextEditState::load(ctx, id) {
        let at = egui::text::CCursor::new(new_cursor);
        edit.cursor
            .set_char_range(Some(egui::text::CCursorRange::one(at)));
        edit.store(ctx, id);
    }
    ctx.memory_mut(|m| m.request_focus(id));
    state.pick = 0;
}

/// Before the text box sees the keyboard: Up/Down pick a suggestion,
/// Tab/Enter accept it, Esc closes the list. Returns what to show.
fn handle_completion_keys(
    ui: &mut egui::Ui,
    id: egui::Id,
    state: &mut EditorState,
) -> Option<Suggestions> {
    let ctx = ui.ctx().clone();
    if state
        .dismissed_for
        .as_ref()
        .is_some_and(|s| *s != state.source)
    {
        state.dismissed_for = None;
    }
    let cursor = editor_cursor(&ctx, id)?;
    if state.dismissed_for.is_some() {
        return None;
    }
    let before: String = state.source.chars().take(cursor).collect();
    let line = before.rsplit('\n').next().unwrap_or("");
    let suggestions = autocomplete::suggest(line)?;
    let count = suggestions.items.len();
    state.pick %= count;
    let (mut down, mut up, mut close, mut take) = (false, false, false, false);
    ui.input_mut(|i| {
        down = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
        up = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
        close = i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        take = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
    });
    if down {
        state.pick = (state.pick + 1) % count;
    }
    if up {
        state.pick = (state.pick + count - 1) % count;
    }
    if close {
        state.dismissed_for = Some(state.source.clone());
        return None;
    }
    if take {
        let insert = suggestions.items[state.pick].insert.clone();
        accept(&ctx, id, state, cursor, suggestions.replace, &insert);
        return None;
    }
    Some(suggestions)
}

fn completion_popup(
    ui: &mut egui::Ui,
    id: egui::Id,
    at: egui::Pos2,
    suggestions: &Suggestions,
    state: &mut EditorState,
) {
    let ctx = ui.ctx().clone();
    let mut clicked = None;
    egui::Area::new(egui::Id::new("autocomplete"))
        .order(egui::Order::Foreground)
        .fixed_pos(at + egui::vec2(0.0, 2.0))
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (i, item) in suggestions.items.iter().enumerate() {
                    let text = egui::RichText::new(format!("{}   {}", item.insert, item.detail))
                        .monospace();
                    if ui.selectable_label(i == state.pick, text).clicked() {
                        clicked = Some(i);
                    }
                }
                ui.weak("Tab: accept   Esc: close");
            });
        });
    if let (Some(i), Some(cursor)) = (clicked, editor_cursor(&ctx, id).or(Some(0))) {
        let insert = suggestions.items[i].insert.clone();
        accept(&ctx, id, state, cursor, suggestions.replace, &insert);
    }
}

#[cfg(test)]
mod tests {
    use super::STARTER_SCRIPT;
    use crate::scripting::{
        budget::DEPLOY_BUDGET,
        runtime::{RunOutcome, ScriptRuntime},
    };

    #[test]
    fn completion_replaces_the_typed_part() {
        let mut source = "# é\npower.co".to_owned();
        let cursor = source.chars().count();
        let new_cursor = super::apply_completion(&mut source, cursor, 2, "connect(");
        assert_eq!(source, "# é\npower.connect(");
        assert_eq!(new_cursor, source.chars().count());
    }

    #[test]
    fn starter_script_runs() {
        let report = ScriptRuntime::new().run(STARTER_SCRIPT, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
        assert!(report.plan.is_some_and(|plan| !plan.machines.is_empty()));
    }
}
