//! The player's script files, each in its own floating code window, plus
//! the console window.
//!
//! main.py is always there and is what Run starts. Other files are modules
//! main.py can import (`PPL.py` -> `import PPL`). Windows can be moved,
//! resized and closed; the Files menu in the top bar brings them back.

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::scripting::files::ScriptStore;
use crate::scripting::runtime::Program;
use crate::scripting::{Console, ConsoleKind, ErrorLine};

use super::autocomplete::{self, Suggestions};
use super::console::console_ui;
use super::highlight;
use super::settings::Settings;

/// The first script a new player sees: the roadmap's canonical sample.
const STARTER_SCRIPT: &str = crate::scripting::CANONICAL_SAMPLE;

pub const MAIN_FILE: &str = "main.py";

/// Seconds between autosaves while a file has unsaved changes.
const AUTOSAVE_SECS: f32 = 5.0;

/// Names a player file cannot take: the game's own modules (an import would
/// find the game module first) and main itself.
const RESERVED: &[&str] = &[
    "main", "auto", "power", "console", "sensors", "stats", "clock", "pypeline", "shop",
];

/// Python keywords cannot be imported, so they cannot be file names either.
const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

pub struct ScriptFile {
    /// File name with its extension, like "main.py".
    pub name: String,
    pub source: String,
    /// What is on disk, to know when there are unsaved changes.
    pub saved: String,
    /// Its window is showing.
    pub open: bool,
    /// Highlighted autocomplete suggestion.
    pick: usize,
    /// Esc closed the suggestions; they stay closed until the text changes.
    dismissed_for: Option<String>,
}

impl ScriptFile {
    pub fn new(name: &str, source: &str) -> Self {
        Self {
            name: name.to_owned(),
            source: source.to_owned(),
            saved: String::new(),
            open: true,
            pick: 0,
            dismissed_for: None,
        }
    }

    /// The name used in `import`: "PPL.py" -> "PPL".
    pub fn module_name(&self) -> &str {
        self.name.strip_suffix(".py").unwrap_or(&self.name)
    }

    pub fn unsaved(&self) -> bool {
        self.source != self.saved
    }

    /// Add code at the end, on a new line.
    pub fn append(&mut self, code: &str) {
        if !self.source.is_empty() && !self.source.ends_with('\n') {
            self.source.push('\n');
        }
        self.source.push_str(code);
    }
}

#[derive(Default)]
struct NewFileDialog {
    name: String,
    focus: bool,
}

#[derive(Resource)]
pub struct Workspace {
    /// main.py first, then the other files by name.
    pub files: Vec<ScriptFile>,
    pub console_open: bool,
    new_file: Option<NewFileDialog>,
    confirm_delete: Option<String>,
    /// Bumped by "Reset windows": new window ids put every window back in
    /// its starting spot.
    layout: u32,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            files: vec![ScriptFile::new(MAIN_FILE, STARTER_SCRIPT)],
            console_open: true,
            new_file: None,
            confirm_delete: None,
            layout: 0,
        }
    }
}

impl Workspace {
    pub fn main(&mut self) -> &mut ScriptFile {
        &mut self.files[0]
    }

    /// Everything Run needs: main.py and every other file as a module.
    pub fn program(&self) -> Program {
        Program {
            main: self.files[0].source.clone(),
            modules: self.files[1..]
                .iter()
                .map(|f| (f.module_name().to_owned(), f.source.clone()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    /// Add code to the end of main.py and show its window.
    pub fn insert_into_main(&mut self, code: &str) {
        let main = self.main();
        main.append(code);
        main.open = true;
    }

    /// Create a file with this code, or replace the code of the file with
    /// that name, and show its window. It is saved by the next autosave.
    pub fn put_file(&mut self, name: &str, code: &str) {
        match self.files.iter_mut().find(|f| f.name == name) {
            Some(file) => {
                file.source = code.to_owned();
                file.open = true;
            }
            None => self.files.push(ScriptFile::new(name, code)),
        }
    }

    pub fn open_new_file_dialog(&mut self) {
        self.new_file = Some(NewFileDialog {
            focus: true,
            ..default()
        });
    }

    pub fn reset_layout(&mut self) {
        self.layout += 1;
        self.console_open = true;
        self.files[0].open = true;
    }

    /// Check a name typed in the New file box. "PPL" becomes "PPL.py";
    /// "PPL.py" stays as it is.
    pub fn check_new_name(&self, typed: &str) -> Result<String, String> {
        let name = file_name(typed)?;
        let taken = self
            .files
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(&name));
        if taken {
            return Err(format!("there is already a file called {name}"));
        }
        Ok(name)
    }

    /// Save every file with unsaved changes.
    pub fn save_all(&mut self, store: Option<&ScriptStore>, console: &mut Console) {
        let Some(store) = store else { return };
        for file in &mut self.files {
            if !file.unsaved() {
                continue;
            }
            match store.save(&file.name, &file.source) {
                Ok(()) => file.saved = file.source.clone(),
                Err(err) => console.push(
                    ConsoleKind::Error,
                    format!("Could not save {}: {err}", file.name),
                ),
            }
        }
    }
}

/// Turn what the player typed into a file name, or say what is wrong.
pub fn file_name(typed: &str) -> Result<String, String> {
    let typed = typed.trim();
    let stem = typed.strip_suffix(".py").unwrap_or(typed);
    if stem.is_empty() {
        return Err("type a name, like PPL or helpers".into());
    }
    let mut chars = stem.chars();
    let first = chars.next().expect("not empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err("the name must start with a letter".into());
    }
    if let Some(bad) = stem
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_'))
    {
        return Err(format!(
            "'{bad}' cannot be in a file name: use letters, digits and _ (main.py imports it by this name)"
        ));
    }
    if stem.starts_with("__") {
        return Err("the name cannot start with __".into());
    }
    if KEYWORDS.contains(&stem) {
        return Err(format!("{stem} is a Python word, so it cannot be imported"));
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        return Err(format!("{stem} is already used by the game"));
    }
    Ok(format!("{stem}.py"))
}

/// Load the player's files at startup.
pub fn load_scripts(
    mut commands: Commands,
    mut workspace: ResMut<Workspace>,
    mut console: ResMut<Console>,
) {
    let Some(store) = ScriptStore::default_location() else {
        console.push(
            ConsoleKind::Error,
            "No folder to save scripts in; changes will not be kept.",
        );
        return;
    };
    match store.load_all() {
        Ok(files) => {
            for (name, source) in files {
                let file = if name == MAIN_FILE {
                    workspace.main()
                } else if file_name(&name).as_deref() == Ok(name.as_str()) {
                    workspace.files.push(ScriptFile::new(&name, ""));
                    workspace.files.last_mut().expect("just pushed")
                } else {
                    // A file the player made outside the game with a name
                    // that cannot be imported: leave it alone.
                    continue;
                };
                file.saved = source.clone();
                file.source = source;
            }
        }
        Err(err) => console.push(
            ConsoleKind::Error,
            format!("Could not load your scripts: {err}"),
        ),
    }
    console.push(
        ConsoleKind::Info,
        format!("Scripts are saved in {}", store.dir().display()),
    );
    commands.insert_resource(store);
}

/// Save every few seconds while there are unsaved changes, and on exit.
pub fn autosave(
    time: Res<Time>,
    mut since: Local<f32>,
    mut workspace: ResMut<Workspace>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
    mut exit: MessageReader<AppExit>,
) {
    *since += time.delta_secs();
    let exiting = exit.read().count() > 0;
    if exiting || *since >= AUTOSAVE_SECS {
        *since = 0.0;
        workspace.save_all(store.as_deref(), &mut console);
    }
}

/// Default size of a code window, in points.
const CODE_SIZE: egui::Vec2 = egui::vec2(470.0, 440.0);

/// Every open code window, the console, and the New file / Delete dialogs.
#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
pub fn code_windows(
    mut contexts: EguiContexts,
    mut workspace: ResMut<Workspace>,
    error_line: Res<ErrorLine>,
    store: Option<Res<ScriptStore>>,
    mut console: ResMut<Console>,
    settings: Res<Settings>,
    area: Res<crate::engine::camera::GameArea>,
    debugger: Res<super::debugger::Debugger>,
    mut followed: Local<Option<(String, usize)>>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let top = area.0.map_or(40.0, |a| a.min.y) + 8.0;
    let screen = ctx.viewport_rect();
    let layout = workspace.layout;
    let syntax = settings.theme().syntax;
    let theme = settings.theme();
    let here = debugger.current_line();
    // The debugger opens the window of the file it is in, and brings it to
    // the front each time it moves to another line.
    if let Some((file, _)) = &here
        && let Some(f) = workspace.files.iter_mut().find(|f| f.name == *file)
    {
        f.open = true;
        if *followed != here {
            let id = egui::Id::new(("code_window", file, layout));
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
        }
    }
    followed.clone_from(&here);

    let mut delete = None;
    for (index, file) in workspace.files.iter_mut().enumerate() {
        if !file.open {
            continue;
        }
        let mut open = true;
        let title = if file.unsaved() {
            format!("{} ●", file.name)
        } else {
            file.name.clone()
        };
        // Files cascade down and to the right of main.py.
        let offset = 28.0 * index as f32;
        egui::Window::new(title)
            .id(egui::Id::new(("code_window", &file.name, layout)))
            .open(&mut open)
            .default_pos(egui::pos2(8.0 + offset, top + offset))
            .default_size(CODE_SIZE)
            .min_size(egui::vec2(240.0, 140.0))
            .resizable(true)
            .collapsible(true)
            .show(&ctx, |ui| {
                let marked = match &error_line.0 {
                    Some((name, line)) if *name == file.name => Some(*line),
                    _ => None,
                };
                let mut marks = Vec::new();
                if let Some((name, line)) = &here
                    && *name == file.name
                {
                    marks.push((*line, crate::engine::themes::rgb(theme.accent)));
                }
                if let Some(line) = marked {
                    marks.push((line, crate::engine::themes::rgb(syntax.error_line)));
                }
                ui.horizontal(|ui| {
                    if index == 0 {
                        ui.weak("Run starts here");
                    } else {
                        ui.weak(format!("import {}", file.module_name()));
                    }
                    if let Some(line) = marked {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            format!("problem on line {line}"),
                        );
                    }
                    if index > 0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("🗑")
                                .on_hover_text("Delete this file")
                                .clicked()
                            {
                                delete = Some(file.name.clone());
                            }
                        });
                    }
                });
                code_editor(ui, file, &marks, &syntax);
            });
        file.open = open;
    }
    if let Some(name) = delete {
        workspace.confirm_delete = Some(name);
    }

    let mut console_open = workspace.console_open;
    egui::Window::new("Console")
        .id(egui::Id::new(("console_window", layout)))
        .open(&mut console_open)
        .default_pos(egui::pos2(8.0, (screen.max.y - 230.0).max(top + 200.0)))
        .default_size(egui::vec2(CODE_SIZE.x, 200.0))
        .min_size(egui::vec2(200.0, 80.0))
        .resizable(true)
        .show(&ctx, |ui| console_ui(ui, &mut console));
    workspace.console_open = console_open;

    new_file_dialog(&ctx, &mut workspace, store.as_deref(), &mut console);
    delete_dialog(&ctx, &mut workspace, store.as_deref(), &mut console);
    Ok(())
}

/// The text box with highlighting and autocomplete, filling the window.
fn code_editor(
    ui: &mut egui::Ui,
    file: &mut ScriptFile,
    marks: &[(usize, egui::Color32)],
    syntax: &crate::engine::themes::Syntax,
) {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = highlight::layout(buf.as_str(), font.clone(), marks, syntax);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let id = egui::Id::new(("editor", &file.name));
    let suggestions = handle_completion_keys(ui, id, file);
    egui::ScrollArea::vertical()
        .id_salt(("editor_scroll", &file.name))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let output = egui::TextEdit::multiline(&mut file.source)
                .id(id)
                .code_editor()
                .layouter(&mut layouter)
                .desired_width(f32::INFINITY)
                .min_size(ui.available_size())
                .show(ui);
            if let (Some(suggestions), Some(range)) = (suggestions, output.cursor_range) {
                let at = output.galley_pos
                    + output
                        .galley
                        .pos_from_cursor(range.primary)
                        .left_bottom()
                        .to_vec2();
                completion_popup(ui, id, at, &suggestions, file);
            }
        });
}

fn new_file_dialog(
    ctx: &egui::Context,
    workspace: &mut Workspace,
    store: Option<&ScriptStore>,
    console: &mut Console,
) {
    let Some(mut dialog) = workspace.new_file.take() else {
        return;
    };
    let mut keep = true;
    let mut create = None;
    egui::Window::new("New file")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -80.0))
        .show(ctx, |ui| {
            ui.label("Name of the new file:");
            let edit = ui.add(
                egui::TextEdit::singleline(&mut dialog.name)
                    .hint_text("PPL")
                    .desired_width(240.0),
            );
            if std::mem::take(&mut dialog.focus) {
                edit.request_focus();
            }
            let checked = workspace.check_new_name(&dialog.name);
            match &checked {
                Ok(name) => {
                    ui.weak(format!(
                        "Saved as {name}. Use it from main.py with: import {}",
                        name.trim_end_matches(".py")
                    ));
                }
                Err(problem) if !dialog.name.trim().is_empty() => {
                    ui.colored_label(ui.visuals().error_fg_color, problem);
                }
                Err(_) => {
                    ui.weak("\".py\" is added for you if you leave it out.");
                }
            }
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let ok = ui.add_enabled(checked.is_ok(), egui::Button::new("Create"));
                if (ok.clicked() || enter)
                    && let Ok(name) = &checked
                {
                    create = Some(name.clone());
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    keep = false;
                }
            });
        });
    if let Some(name) = create {
        let stem = name.trim_end_matches(".py");
        let mut file = ScriptFile::new(
            &name,
            &format!(
                "# {name}: put functions here and use them from main.py:\n#   import {stem}\n#   {stem}.hello()\n\ndef hello():\n    print(\"Hello from {name}!\")\n"
            ),
        );
        file.saved = String::new();
        workspace.files.push(file);
        workspace.save_all(store, console);
        console.push(ConsoleKind::Info, format!("Created {name}."));
        keep = false;
    }
    if keep {
        workspace.new_file = Some(dialog);
    }
}

fn delete_dialog(
    ctx: &egui::Context,
    workspace: &mut Workspace,
    store: Option<&ScriptStore>,
    console: &mut Console,
) {
    let Some(name) = workspace.confirm_delete.clone() else {
        return;
    };
    egui::Window::new("Delete file?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -80.0))
        .show(ctx, |ui| {
            ui.label(format!("Delete {name}?"));
            ui.weak(format!(
                "A copy is kept in the scripts folder as {name}.bak."
            ));
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let Some(store) = store
                        && let Err(err) = store.delete(&name)
                    {
                        console.push(
                            ConsoleKind::Error,
                            format!("Could not delete {name}: {err}"),
                        );
                        return;
                    }
                    workspace.files.retain(|f| f.name != name);
                    console.push(ConsoleKind::Info, format!("Deleted {name}."));
                    workspace.confirm_delete = None;
                }
                if ui.button("Keep it").clicked() {
                    workspace.confirm_delete = None;
                }
            });
        });
}

/// The cursor position (in characters) of an editor, if it has focus.
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
    file: &mut ScriptFile,
    cursor: usize,
    replace: usize,
    insert: &str,
) {
    let new_cursor = apply_completion(&mut file.source, cursor, replace, insert);
    if let Some(mut edit) = egui::text_edit::TextEditState::load(ctx, id) {
        let at = egui::text::CCursor::new(new_cursor);
        edit.cursor
            .set_char_range(Some(egui::text::CCursorRange::one(at)));
        edit.store(ctx, id);
    }
    ctx.memory_mut(|m| m.request_focus(id));
    file.pick = 0;
}

/// Before the text box sees the keyboard: Up/Down pick a suggestion,
/// Tab/Enter accept it, Esc closes the list. Returns what to show.
fn handle_completion_keys(
    ui: &mut egui::Ui,
    id: egui::Id,
    file: &mut ScriptFile,
) -> Option<Suggestions> {
    let ctx = ui.ctx().clone();
    if file
        .dismissed_for
        .as_ref()
        .is_some_and(|s| *s != file.source)
    {
        file.dismissed_for = None;
    }
    let cursor = editor_cursor(&ctx, id)?;
    if file.dismissed_for.is_some() {
        return None;
    }
    let before: String = file.source.chars().take(cursor).collect();
    let line = before.rsplit('\n').next().unwrap_or("");
    let suggestions = autocomplete::suggest(line)?;
    let count = suggestions.items.len();
    file.pick %= count;
    let (mut down, mut up, mut close, mut take) = (false, false, false, false);
    ui.input_mut(|i| {
        down = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
        up = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
        close = i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        take = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
    });
    if down {
        file.pick = (file.pick + 1) % count;
    }
    if up {
        file.pick = (file.pick + count - 1) % count;
    }
    if close {
        file.dismissed_for = Some(file.source.clone());
        return None;
    }
    if take {
        let insert = suggestions.items[file.pick].insert.clone();
        accept(&ctx, id, file, cursor, suggestions.replace, &insert);
        return None;
    }
    Some(suggestions)
}

fn completion_popup(
    ui: &mut egui::Ui,
    id: egui::Id,
    at: egui::Pos2,
    suggestions: &Suggestions,
    file: &mut ScriptFile,
) {
    let ctx = ui.ctx().clone();
    let mut clicked = None;
    egui::Area::new(id.with("autocomplete"))
        .order(egui::Order::Foreground)
        .fixed_pos(at + egui::vec2(0.0, 2.0))
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (i, item) in suggestions.items.iter().enumerate() {
                    let text = egui::RichText::new(format!("{}   {}", item.insert, item.detail))
                        .monospace();
                    if ui.selectable_label(i == file.pick, text).clicked() {
                        clicked = Some(i);
                    }
                }
                ui.weak("Tab: accept   Esc: close");
            });
        });
    if let (Some(i), Some(cursor)) = (clicked, editor_cursor(&ctx, id).or(Some(0))) {
        let insert = suggestions.items[i].insert.clone();
        accept(&ctx, id, file, cursor, suggestions.replace, &insert);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::{
        budget::DEPLOY_BUDGET,
        runtime::{RunOutcome, ScriptRuntime},
    };

    #[test]
    fn completion_replaces_the_typed_part() {
        let mut source = "# é\npower.co".to_owned();
        let cursor = source.chars().count();
        let new_cursor = apply_completion(&mut source, cursor, 2, "connect(");
        assert_eq!(source, "# é\npower.connect(");
        assert_eq!(new_cursor, source.chars().count());
    }

    #[test]
    fn starter_script_runs() {
        let report = ScriptRuntime::new().run(STARTER_SCRIPT, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
        assert!(report.plan.is_some_and(|plan| !plan.machines.is_empty()));
    }

    #[test]
    fn new_file_names() {
        assert_eq!(file_name("PPL").as_deref(), Ok("PPL.py"));
        assert_eq!(file_name("PPL.py").as_deref(), Ok("PPL.py"));
        assert_eq!(file_name("  helpers ").as_deref(), Ok("helpers.py"));
        for bad in [
            "", ".py", "2fast", "my file", "a-b", "for", "main", "power.py", "__x",
        ] {
            assert!(file_name(bad).is_err(), "{bad:?} should be refused");
        }
        let workspace = Workspace::default();
        assert!(workspace.check_new_name("MAIN").is_err());
    }

    #[test]
    fn other_files_become_modules() {
        let mut workspace = Workspace::default();
        workspace
            .files
            .push(ScriptFile::new("PPL.py", "def two():\n    return 2\n"));
        workspace.main().source = "import PPL\nprint(PPL.two())\n".into();
        let report = ScriptRuntime::new().run_program(&workspace.program(), DEPLOY_BUDGET);
        assert_eq!(report.output, vec!["2"], "{:?}", report.outcome);
    }
}
