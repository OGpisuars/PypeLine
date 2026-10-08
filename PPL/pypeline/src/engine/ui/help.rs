//! Help window: what every command does, with examples that can be inserted
//! into main.py. Opened with the editor's Help button or F1.
//!
//! This is a quick reference until the Engineering Manual (Phase 3A) exists.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::editor::EditorState;
use super::{palette, rgb};

#[derive(Resource, Default)]
pub struct HelpState {
    pub open: bool,
}

struct Command {
    signature: &'static str,
    about: &'static [&'static str],
    example: &'static str,
}

const BASICS: &[&str] = &[
    "Your script describes the WHOLE factory. Each time you press Run, the game builds \
     exactly what the script says: new things are added, changed things are updated, and \
     anything you took out of the script is removed (its items go back to the station).",
    "Running the same script again changes nothing, so it is safe to press Run often.",
    "If the script has an error, nothing in the factory changes and the belts stop until \
     your next good Run. The console shows the line with the problem.",
];

const GRID: &[&str] = &[
    "The plot is 16 tiles wide and 10 tiles tall.",
    "x goes from 0 (left) to 15 (right). y goes from 0 (bottom) to 9 (top).",
    "Point at the island with the mouse to see a tile's x and y in the top-left corner.",
    "Directions: \"north\" is up, \"east\" is right, \"south\" is down, \"west\" is left.",
];

const IMPORTS: &str = "from auto import conveyors, machines\nimport power\n";

const COMMANDS: &[Command] = &[
    Command {
        signature: "conveyors.place(x, y, dir)",
        about: &[
            "Puts a conveyor belt on tile (x, y).",
            "Items on it move toward dir and are handed to whatever is on the next tile: \
             another belt or a machine.",
        ],
        example: "for x in range(1, 5):\n    conveyors.place(x=x, y=0, dir=\"east\")\n",
    },
    Command {
        signature: "machines.place(kind, name, x, y, dir=\"east\", ore=...)",
        about: &[
            "Builds a machine on tile (x, y).",
            "name must be different for every machine. It is how power.connect finds it.",
            "dir is the side items come out of (the small brass mark). Leave it out for east.",
            "Kinds of machine:",
            "  \"miner\": digs one ore every 2 seconds. Needs ore=\"iron\" and power.",
            "  \"smelter\": turns iron ore into an iron plate every 3 seconds. Needs power. \
             It takes ore from belts that point into it.",
            "  \"steam_generator\": powers the machines you connect to it.",
            "  \"station\": collects items from belts that point into it. Every 30 \
             seconds the cargo train buys everything in your stations: 1 coin per ore, \
             4 coins per plate. No power needed.",
        ],
        example: "machines.place(\"miner\", name=\"miner_1\", x=0, y=0, ore=\"iron\")\n",
    },
    Command {
        signature: "power.connect(generator, to)",
        about: &[
            "Powers machines from a steam generator. to is a list of machine names.",
            "Place the generator and the machines first, then connect them.",
            "A machine without power looks gray and does nothing.",
        ],
        example: "machines.place(\"steam_generator\", name=\"steam_1\", x=0, y=2)\n\
                  power.connect(generator=\"steam_1\", to=[\"miner_1\"])\n",
    },
    Command {
        signature: "print(...)",
        about: &["Writes to the console. Handy for checking what a variable holds."],
        example: "print(\"Hello, factory!\")\n",
    },
];

const TROUBLESHOOTING: &[(&str, &str)] = &[
    (
        "Error (line N)",
        "Python could not run that line. Nothing in the factory changed. Fix the line and Run again.",
    ),
    (
        "Out of steam",
        "The script used too many steps, usually a loop that never ends.",
    ),
    (
        "A machine is gray",
        "It has no power. Connect it with power.connect.",
    ),
    (
        "Items stop moving",
        "The next tile cannot take them: nothing is there, a belt points the wrong way, \
         or the machine is full.",
    ),
    (
        "HALTED in the corner",
        "The last Run failed. The belts restart after your next good Run.",
    ),
];

pub fn toggle_help(keys: Res<ButtonInput<KeyCode>>, mut help: ResMut<HelpState>) {
    if keys.just_pressed(KeyCode::F1) {
        help.open = !help.open;
    }
}

pub fn help_window(
    mut contexts: EguiContexts,
    mut help: ResMut<HelpState>,
    mut editor: ResMut<EditorState>,
) -> Result {
    let mut open = help.open;
    egui::Window::new("Help")
        .open(&mut open)
        .default_pos(egui::pos2(1000.0, 48.0))
        .default_size(egui::vec2(420.0, 600.0))
        .show(contexts.ctx_mut()?, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, "How it works", true, |ui| paragraphs(ui, BASICS));
                section(ui, "The grid", true, |ui| paragraphs(ui, GRID));
                section(ui, "Imports", false, |ui| {
                    ui.label("Put these lines at the top of main.py to use the commands:");
                    example(ui, &mut editor, IMPORTS);
                });
                for command in COMMANDS {
                    section(ui, command.signature, false, |ui| {
                        paragraphs(ui, command.about);
                        example(ui, &mut editor, command.example);
                    });
                }
                section(ui, "When something goes wrong", false, |ui| {
                    for (problem, fix) in TROUBLESHOOTING {
                        ui.label(egui::RichText::new(*problem).strong());
                        ui.label(*fix);
                        ui.add_space(4.0);
                    }
                });
            });
        });
    help.open = open;
    Ok(())
}

fn section(ui: &mut egui::Ui, title: &str, open: bool, body: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(egui::RichText::new(title).strong().monospace())
        .default_open(open)
        .show(ui, body);
}

fn paragraphs(ui: &mut egui::Ui, lines: &[&str]) {
    for line in lines {
        ui.label(*line);
    }
}

/// Shows example code with a button that appends it to main.py.
fn example(ui: &mut egui::Ui, editor: &mut EditorState, code: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(255, 252, 240))
        .stroke(egui::Stroke::new(1.0, rgb(palette::UI_BORDER)))
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(code.trim_end()).monospace());
        });
    if ui.button("Insert into main.py").clicked() {
        if !editor.source.ends_with('\n') {
            editor.source.push('\n');
        }
        editor.source.push_str(code);
    }
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, IMPORTS};
    use crate::scripting::budget::DEPLOY_BUDGET;
    use crate::scripting::runtime::{RunOutcome, ScriptRuntime};

    /// Every example in the Help window must run (roadmap Part 4 E). Inserted
    /// in order after the imports, they form one working script.
    #[test]
    fn help_examples_run() {
        let script: String = std::iter::once(IMPORTS)
            .chain(COMMANDS.iter().map(|c| c.example))
            .collect();
        let report = ScriptRuntime::new().run(&script, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{script}");
    }
}
