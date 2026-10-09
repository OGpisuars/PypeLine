//! Help window (F1): how the game works, every name and command, with
//! examples that can be inserted into main.py.
//!
//! The lists of names (machines, items, directions, colors, upgrades) are
//! built from the game's own tables, so they never fall out of date.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::editor::Workspace;
use crate::factory::Dir;
use crate::factory::items::{self, ItemKind};
use crate::factory::machines::{MINE_TICKS, MachineKind, SMELT_TICKS};
use crate::factory::recipes::Recipe;
use crate::factory::shop::{self, Upgrade};
use crate::factory::train;
use crate::progression::achievements;
use crate::scripting::console_api::ConsoleColor;

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
     your next good Run. The console shows the file and line with the problem.",
];

const MOVING: &[&str] = &[
    "Drag empty space with the left mouse button to move the world. The middle and \
     right buttons drag from anywhere outside a window. You can go as far as you like; \
     Home comes back.",
    "The mouse wheel zooms in and out around the pointer. Home (or View > Center the \
     island) puts the island back in the middle.",
    "Code windows are part of the world: they move with the island when you drag, grow and \
     shrink when you zoom, and stay where you left them when you look away, even off \
     screen. Drag a window by its title to park it anywhere. Resize any window from its edges and corners, and close it \
     with its x; the top bar brings it back (Files, Console, Help...).",
    "View > Reset windows puts the code windows and console back where they started.",
    "Untick View > Island bobs up and down (also in Settings) to keep the island still.",
];

const KEYS: &[(&str, &str)] = &[
    ("Ctrl+Enter or F5", "Run"),
    ("F6", "Debug: record main.py line by line"),
    ("F7 / F8", "debugger: back / forward one line"),
    ("F1 / F2 / F3 / F4", "Help / Manual / Shop / Stats"),
    ("Esc", "pause menu: resume, settings, title screen, quit"),
    ("Space", "pause or play (when not typing)"),
    (". (period)", "step one tick while paused"),
    ("1 / 2 / 3", "speed 1x / 2x / 4x"),
    ("M", "sound on or off"),
    ("Home", "center the island"),
    ("Tab or Enter", "accept a suggestion while typing"),
];

const GRID: &[&str] = &[
    "The plot starts 16 tiles wide and 10 tiles tall. Bigger islands from the Shop (F3) add \
     room to the east and north, up to 28 by 15.",
    "x goes from 0 (left) to 15 (right) and y from 0 (bottom) to 9 (top); a bigger island \
     lets them go higher. (0, 0) is always the bottom-left tile, so scripts keep working.",
    "Point at the island with the mouse to see a tile's x and y in the top-right corner.",
    "Directions: \"north\" is up, \"east\" is right, \"south\" is down, \"west\" is left.",
];

pub(super) const IMPORTS: &str = "from auto import conveyors, machines, splitters\n\
                       import power\n\
                       import console\n\
                       import sensors\n\
                       import stats\n\
                       import clock\n";

const MODULES: &[(&str, &str)] = &[
    ("auto.conveyors", "place belts"),
    ("auto.splitters", "split one belt into two"),
    (
        "auto.machines",
        "place machines; switch them on and off; read their status",
    ),
    ("power", "connect machines to steam generators"),
    ("console", "colors and clearing for print()"),
    ("sensors", "how many items are on a belt tile"),
    ("stats", "items made, items per minute, coins"),
    ("clock", "the current tick and second"),
];

/// Build calls: they go in main.py (or a file it imports), not in tick().
const BUILD: &[Command] = &[
    Command {
        signature: "conveyors.place(x, y, dir, tier=1)",
        about: &[
            "Puts a conveyor belt on tile (x, y).",
            "Items on it move toward dir and are handed to whatever is on the next tile: \
             another belt or a machine.",
            "tier=2 and tier=3 are faster belts from the Shop.",
        ],
        example: "for x in range(1, 5):\n    conveyors.place(x=x, y=0, dir=\"east\")\n",
    },
    Command {
        signature: "splitters.place(x, y, dir1, dir2, tier=1)",
        about: &[
            "Puts a splitter on tile (x, y). Items can come in from any side.",
            "It hands them out in turn: one toward dir1, the next toward dir2, so each side \
             gets half.",
            "If one side is full, the other side gets the items until there is room again, so \
             the line never jams.",
            "tier=2 and tier=3 are faster, like belts.",
        ],
        example: "conveyors.place(x=3, y=5, dir=\"east\")\n\
                  splitters.place(x=4, y=5, dir1=\"north\", dir2=\"south\")\n\
                  conveyors.place(x=4, y=6, dir=\"north\")\n\
                  conveyors.place(x=4, y=4, dir=\"south\")\n",
    },
    Command {
        signature: "machines.place(kind, name, x, y, dir=\"east\", ore=..., tier=1, recipe=...)",
        about: &[
            "Builds a machine on tile (x, y). See \"Names you can use\" for the kinds.",
            "name must be different for every machine. It is how power.connect and \
             machines.enable find it.",
            "dir is the side items come out of (the small brass mark). Leave it out for east.",
            "Miners need ore=\"iron\". tier=2 and tier=3 are faster machines from the Shop.",
            "Crafters take recipe=\"iron_gear\" (see \"Names you can use\"), or click one on \
             the island to pick its recipe. Recipes are unlocked by achievements.",
        ],
        example: "machines.place(\"miner\", name=\"miner_1\", x=0, y=0, ore=\"iron\")\n\
                  machines.place(\"smelter\", name=\"smelter_1\", x=5, y=0)\n\
                  conveyors.place(x=6, y=0, dir=\"east\")\n\
                  machines.place(\"station\", name=\"station_1\", x=7, y=0)\n",
    },
    Command {
        signature: "power.connect(generator, to)",
        about: &[
            "Powers machines from a steam generator. to is a list of machine names.",
            "Place the generator and the machines first, then connect them.",
            "A machine without power looks gray and does nothing.",
        ],
        example: "machines.place(\"steam_generator\", name=\"steam_1\", x=0, y=2)\n\
                  power.connect(generator=\"steam_1\", to=[\"miner_1\", \"smelter_1\"])\n",
    },
    Command {
        signature: "console.color(name) / console.clear()",
        about: &[
            "console.color(\"green\") colors the lines you print after it; \"default\" goes back.",
            "console.clear() wipes the console, handy for dashboards that redraw.",
        ],
        example: "console.color(\"green\")\nprint(\"[\" + \"#\" * 8 + \"..]\")\nconsole.color(\"default\")\n",
    },
    Command {
        signature: "print(...)",
        about: &["Writes to the console. Handy for checking what a variable holds."],
        example: "print(\"Hello, factory!\")\n",
    },
];

/// Operate calls: they read the running factory, inside tick() or an event.
const OPERATE: &[Command] = &[
    Command {
        signature: "def tick():",
        about: &[
            "If main.py defines tick(), the game calls it 20 times a second while the \
             factory runs. Variables made outside it keep their values between ticks.",
            "Each tick has a small step budget. A tick() that runs out of steam 3 ticks in \
             a row overheats the boiler. Use yield inside tick() to spread work over \
             several ticks.",
            "Build calls (place, connect) are not allowed inside tick().",
        ],
        example: "def tick():\n    if stats.per_minute(\"iron_plate\") < 10:\n        \
                  machines.enable(\"miner_1\")\n    else:\n        machines.disable(\"miner_1\")\n",
    },
    Command {
        signature: "machines.enable(name) / machines.disable(name)",
        about: &[
            "Switch a machine on or off. An off machine keeps its items but does no work.",
            "Changes happen on the next factory step.",
        ],
        example: "def on_contract_complete(title):\n    machines.enable(\"smelter_1\")\n",
    },
    Command {
        signature: "machines.status(name)",
        about: &["A dict about the machine: kind, working, input, output, powered, on, tier."],
        example: "def show_smelter():\n    s = machines.status(\"smelter_1\")\n    \
                  print(s[\"kind\"], \"has\", s[\"input\"], \"ore waiting\")\n",
    },
    Command {
        signature: "sensors.count(x, y) / sensors.temperature(generator)",
        about: &[
            "How many items are on the belt at (x, y) right now (0 if there is no belt).",
            "A steam generator's temperature. At 100 it overheats and powers nothing \
             until it cools to 70. Each switched-on machine it powers adds 6 degrees, \
             and the air is up to 15 degrees warmer at noon.",
        ],
        example: "def belt_is_busy():\n    return sensors.count(2, 0) >= 2\n\n\
                  def boiler_is_hot():\n    return sensors.temperature(\"steam_1\") >= 90\n",
    },
    Command {
        signature: "stats.produced(item) / stats.per_minute(item) / stats.coins()",
        about: &[
            "Items made since the start, items made in the last minute, and your coins.",
            "item is an item id like \"iron_plate\" (see \"Names you can use\").",
        ],
        example: "def report():\n    print(stats.produced(\"iron_ore\"), \"ore,\", stats.coins(), \"coins\")\n",
    },
    Command {
        signature: "stats.bottlenecks() / stats.steam() / stats.steam_limit()",
        about: &[
            "stats.bottlenecks() lists the machines whose output is full: whatever comes \
             after them is too slow or missing. The Stats window (F4) shows them glowing red.",
            "stats.steam() is how much steam this Run or tick() has used so far, and \
             stats.steam_limit() how much it has in total.",
            "machines.status(name)[\"state\"] is \"working\", \"starved\", \"blocked\", \
             \"no power\", \"off\" or \"idle\".",
        ],
        example: "def check():\n    for name in stats.bottlenecks():\n        \
                  print(name, \"is blocked\")\n    \
                  print(stats.steam(), \"of\", stats.steam_limit(), \"steam used\")\n",
    },
    Command {
        signature: "clock.tick() / clock.seconds() / clock.time_of_day() / clock.is_day()",
        about: &[
            "Ticks and whole seconds the factory has run (20 ticks a second).",
            "The hour (0-23) and whether it is day (6:00 to 18:00). A whole day lasts \
             four minutes; the clock in the top bar shows it.",
        ],
        example: "def every_ten_seconds():\n    return clock.tick() % 200 == 0\n\n\
                  def hot_hours():\n    return clock.is_day() and 10 <= clock.time_of_day() <= 14\n",
    },
    Command {
        signature: "def on_train(coins): / def on_contract_complete(title):",
        about: &[
            "Events: define them and the game calls them when the cargo train pays you, \
             or when you finish a contract.",
        ],
        example: "def on_train(coins):\n    print(\"The train paid\", coins, \"coins\")\n",
    },
];

const DEBUGGING: &[&str] = &[
    "Press Debug (F6) to record a run of main.py line by line. The Debugger window \
     then steps through it: forward, backward, or straight to the next time a line runs.",
    "The line about to run is marked in its code window, and the window lists your \
     variables as they were just before it. Values that just changed are highlighted.",
    "A Debug Run uses its own copy of Python and never changes the factory, so press \
     it as often as you like. It records up to 2000 lines.",
];

const FILES: &[&str] = &[
    "Files > + New file makes another file next to main.py. Type a name like PPL and \
     it is saved as PPL.py (if you type PPL.py it stays PPL.py).",
    "A name must start with a letter and use only letters, digits and _, because main.py \
     uses it in an import.",
    "Run always starts main.py. Other files are modules: write import PPL in main.py, \
     then call PPL.some_function(). Files can import each other too.",
    "Each file has its own code window in the world (+ Window makes one where you are \
     looking). A brass cable links each file to the files it imports. Errors name the file \
     and the line.",
];

const TROUBLESHOOTING: &[(&str, &str)] = &[
    (
        "Error (file, line N)",
        "Python could not run that line. Nothing in the factory changed. Fix the line and Run again.",
    ),
    (
        "Out of steam",
        "The script used too many steps, usually a loop that never ends.",
    ),
    (
        "\"except:\" or \"finally:\" is not allowed",
        "Name the error you expect, like except ValueError: (or except Exception: for any \
         ordinary error). This keeps the game's safety stop from being caught.",
    ),
    (
        "\"tier 2 belts are locked\"",
        "Buy the upgrade in the Shop (F3) first, or leave tier= out.",
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
    mut workspace: ResMut<Workspace>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut open = help.open;
    let screen = ctx.viewport_rect();
    egui::Window::new("Help")
        .open(&mut open)
        .default_pos(egui::pos2(screen.max.x - 470.0, 56.0))
        .default_size(egui::vec2(440.0, 620.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, "How it works", true, |ui| paragraphs(ui, BASICS));
                section(ui, "Moving around and keys", false, |ui| {
                    paragraphs(ui, MOVING);
                    ui.add_space(4.0);
                    table(
                        ui,
                        "keys",
                        KEYS.iter().map(|(k, v)| (k.to_string(), v.to_string())),
                    );
                });
                section(ui, "The grid", false, |ui| paragraphs(ui, GRID));
                section(ui, "Names you can use", true, names);
                section(ui, "Imports", false, |ui| {
                    ui.label("Put these at the top of main.py to use every command:");
                    table(
                        ui,
                        "modules",
                        MODULES.iter().map(|(m, v)| (m.to_string(), v.to_string())),
                    );
                    example(ui, &mut workspace, IMPORTS);
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Building (in main.py)").strong());
                for command in BUILD {
                    command_section(ui, command, &mut workspace);
                }
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Running (in tick() and events)").strong());
                for command in OPERATE {
                    command_section(ui, command, &mut workspace);
                }
                ui.add_space(4.0);
                section(ui, "Your own files", false, |ui| paragraphs(ui, FILES));
                section(ui, "The debugger", false, |ui| paragraphs(ui, DEBUGGING));
                section(ui, "The Shop and tiers", false, shop_help);
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

/// Every name the API takes, straight from the game's own tables.
/// Machine kinds and what they do.
fn machine_rows() -> Vec<(String, String)> {
    MachineKind::ALL
        .iter()
        .map(|&kind| {
            let what = match kind {
                MachineKind::Miner => format!(
                    "digs one ore every {} s; needs ore=\"iron\" and power",
                    MINE_TICKS / 20
                ),
                MachineKind::Smelter => format!(
                    "turns ore into a plate every {} s; needs power",
                    SMELT_TICKS / 20
                ),
                MachineKind::Crafter => "makes gears, pipes and engines from up to three inputs \
                                         by a recipe; needs power"
                    .to_owned(),
                MachineKind::SteamGenerator => "powers machines you connect to it".to_owned(),
                MachineKind::Station => format!(
                    "holds items; the train buys them every {} s",
                    train::TRAIN_INTERVAL / 20
                ),
            };
            (format!("\"{}\"", kind.name()), what)
        })
        .collect()
}

fn item_rows() -> Vec<(String, String)> {
    ItemKind::ALL
        .iter()
        .map(|&item| {
            let made = match item {
                ItemKind::IronOre => "dug by miners",
                ItemKind::IronPlate => "made by smelters from iron ore",
                ItemKind::IronGear => "made by crafters (recipe \"iron_gear\")",
                ItemKind::IronPipe => "made by crafters (recipe \"iron_pipe\")",
                ItemKind::Engine => "made by crafters (recipe \"engine\")",
            };
            (
                format!("\"{}\"", item.id()),
                format!(
                    "{}: {made}; the train pays {}",
                    item.name(),
                    train::price(item)
                ),
            )
        })
        .collect()
}

fn recipe_rows() -> Vec<(String, String)> {
    Recipe::ALL
        .iter()
        .map(|&recipe| {
            let unlock = achievements::unlocked_by(recipe)
                .map(|a| format!("; unlocked by the achievement \"{}\"", a.title))
                .unwrap_or_default();
            (
                format!("\"{}\"", recipe.id()),
                format!("{}{unlock}", recipe.describe()),
            )
        })
        .collect()
}

fn ore_rows() -> Vec<(String, String)> {
    ["iron"]
        .into_iter()
        .filter_map(|name| items::ore_by_name(name).map(|item| (name, item)))
        .map(|(name, item)| (format!("\"{name}\""), format!("digs {}", item.name())))
        .collect()
}

fn dir_rows() -> Vec<(String, String)> {
    Dir::ALL
        .iter()
        .map(|&d| {
            let name = format!("{d:?}").to_lowercase();
            let way = match d {
                Dir::North => "up (y + 1)",
                Dir::East => "right (x + 1)",
                Dir::South => "down (y - 1)",
                Dir::West => "left (x - 1)",
            };
            (format!("\"{name}\""), way.to_owned())
        })
        .collect()
}

fn color_names() -> Vec<String> {
    ConsoleColor::NAMES
        .iter()
        .map(|(n, _)| format!("\"{n}\""))
        .chain(["\"default\"".to_owned()])
        .collect()
}

fn upgrade_rows() -> Vec<(String, String)> {
    Upgrade::ALL
        .iter()
        .map(|u| (format!("{} ({} coins)", u.title(), u.price()), u.about()))
        .collect()
}

const SHOP_INTRO: &str = "The cargo train pays coins for everything in your stations. Spend \
     them in the Shop (F3) on faster parts and bigger islands. A faster part does nothing until \
     your script asks for it with tier=, so you choose where the fast parts go. Quick splitters \
     and a bigger island work straight away.";
const PRESTIGE: &str = "Start over with a fresh factory and keep a permanent bonus. Every \
     prestige switches your scripts to a different programming language, and the higher you \
     go, the trickier and fussier the language gets.";

fn names(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Machine kinds: machines.place(\"...\")").strong());
    table(ui, "machine_names", machine_rows().into_iter());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Items: stats.produced(\"...\")").strong());
    table(ui, "item_names", item_rows().into_iter());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Ores: ore=\"...\"").strong());
    table(ui, "ore_names", ore_rows().into_iter());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Recipes: recipe=\"...\"").strong());
    table(ui, "recipe_names", recipe_rows().into_iter());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Directions: dir=\"...\"").strong());
    table(ui, "dir_names", dir_rows().into_iter());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Console colors: console.color(\"...\")").strong());
    ui.label(egui::RichText::new(color_names().join("  ")).monospace());
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Tiers: tier=1, 2 or 3").strong());
    ui.label("1 is the normal part. 2 and 3 are bought in the Shop (F3).");
}

fn shop_help(ui: &mut egui::Ui) {
    ui.label(SHOP_INTRO);
    table(ui, "upgrades", upgrade_rows().into_iter());
    ui.label(format!(
        "Changing a tier keeps the machine's items. Tier {} is the highest for now.",
        shop::MAX_TIER
    ));
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Coming later: Prestige").strong());
    ui.label(PRESTIGE);
}

/// The Help window's content as Markdown: `docs/API.md` is this, and a test
/// keeps the file up to date.
pub fn api_markdown() -> String {
    use std::fmt::Write;
    let mut md = String::new();
    let table = |md: &mut String, head: (&str, &str), rows: &[(String, String)]| {
        let _ = writeln!(md, "| {} | {} |\n|---|---|", head.0, head.1);
        for (a, b) in rows {
            let _ = writeln!(md, "| `{}` | {} |", a.replace('|', "\\|"), b);
        }
        md.push('\n');
    };
    let paragraphs = |md: &mut String, lines: &[&str]| {
        for line in lines {
            let _ = writeln!(md, "{line}\n");
        }
    };
    let code = |md: &mut String, code: &str| {
        let _ = writeln!(md, "```python\n{}\n```\n", code.trim_end());
    };
    md.push_str(
        "# PypeLine scripting API\n\n\
         <!-- Generated from src/engine/ui/help.rs, the in-game Help (F1). Do not edit by \
         hand: run `PYPELINE_WRITE_DOCS=1 cargo test api_doc` to update it. -->\n\n\
         Everything a PypeLine script can use. The same text is in the game's Help window \
         (F1), where each example can be inserted into `main.py`. Every example here runs: \
         the test suite runs them all.\n\n",
    );
    md.push_str("## How it works\n\n");
    paragraphs(&mut md, BASICS);
    md.push_str("## The grid\n\n");
    paragraphs(&mut md, GRID);
    md.push_str("## Imports\n\nPut these at the top of `main.py` to use every command:\n\n");
    code(&mut md, IMPORTS);
    let modules: Vec<(String, String)> = MODULES
        .iter()
        .map(|(m, v)| (m.to_string(), v.to_string()))
        .collect();
    table(&mut md, ("Module", "What it is for"), &modules);
    md.push_str("## Cheat sheet\n\nEverything inside each module, at a glance.\n\n");
    for (heading, rows) in super::autocomplete::cheat_sheet() {
        let _ = writeln!(md, "### `{heading}`\n");
        table(&mut md, ("Name", "What it is"), &rows);
    }
    md.push_str("## Names you can use\n\n### Machine kinds: `machines.place(\"...\")`\n\n");
    table(&mut md, ("Name", "What it does"), &machine_rows());
    md.push_str("### Items: `stats.produced(\"...\")`\n\n");
    table(&mut md, ("Name", "What it is"), &item_rows());
    md.push_str("### Ores: `ore=\"...\"`\n\n");
    table(&mut md, ("Name", "What it does"), &ore_rows());
    md.push_str("### Recipes: `recipe=\"...\"`\n\n");
    table(&mut md, ("Name", "What it makes"), &recipe_rows());
    md.push_str("### Directions: `dir=\"...\"`\n\n");
    table(&mut md, ("Name", "Which way"), &dir_rows());
    let _ = writeln!(
        md,
        "### Console colors: `console.color(\"...\")`\n\n{}\n",
        color_names()
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    md.push_str(
        "### Tiers: `tier=1`, `2` or `3`\n\n1 is the normal part. 2 and 3 are bought in the \
         Shop (F3).\n\n",
    );
    for (title, commands) in [
        ("Building (in main.py)", BUILD),
        ("Running (in tick() and events)", OPERATE),
    ] {
        let _ = writeln!(md, "## {title}\n");
        for command in commands {
            let _ = writeln!(md, "### `{}`\n", command.signature);
            paragraphs(&mut md, command.about);
            code(&mut md, command.example);
        }
    }
    md.push_str("## Your own files\n\n");
    paragraphs(&mut md, FILES);
    md.push_str("## The debugger\n\n");
    paragraphs(&mut md, DEBUGGING);
    let _ = writeln!(md, "## The Shop and tiers\n\n{SHOP_INTRO}\n");
    table(&mut md, ("Upgrade", "What it does"), &upgrade_rows());
    let _ = writeln!(
        md,
        "Changing a tier keeps the machine's items. Tier {} is the highest for now.\n\n\
         **Coming later: Prestige.** {PRESTIGE}\n",
        shop::MAX_TIER
    );
    md.push_str("## When something goes wrong\n\n");
    let fixes: Vec<(String, String)> = TROUBLESHOOTING
        .iter()
        .map(|(p, f)| (p.to_string(), f.to_string()))
        .collect();
    table(&mut md, ("You see", "What to do"), &fixes);
    md.trim_end().to_owned() + "\n"
}

fn command_section(ui: &mut egui::Ui, command: &Command, workspace: &mut Workspace) {
    section(ui, command.signature, false, |ui| {
        paragraphs(ui, command.about);
        example(ui, workspace, command.example);
    });
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

pub(super) fn table(ui: &mut egui::Ui, id: &str, rows: impl Iterator<Item = (String, String)>) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for (name, what) in rows {
                ui.label(egui::RichText::new(name).monospace());
                ui.label(what);
                ui.end_row();
            }
        });
}

/// Shows example code with a button that appends it to main.py.
pub(super) fn example(ui: &mut egui::Ui, workspace: &mut Workspace, code: &str) {
    egui::Frame::new()
        .fill(ui.visuals().code_bg_color)
        .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(code.trim_end()).monospace());
        });
    if ui.button("Insert into main.py").clicked() {
        workspace.insert_into_main(code);
    }
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::{BUILD, IMPORTS, OPERATE, api_markdown};
    use crate::factory::Factory;
    use crate::factory::shop::Upgrade;
    use crate::scripting::budget::{DEPLOY_BUDGET, TICK_BUDGET};
    use crate::scripting::operate::WorldView;
    use crate::scripting::reconcile;
    use crate::scripting::runtime::{EventArg, HookOutcome, RunOutcome, ScriptRuntime};

    /// Everything on the cheat sheet really is in the game's modules.
    #[test]
    fn cheat_sheet_names_exist() {
        let mut script = IMPORTS.to_owned();
        for (_, rows) in super::super::autocomplete::cheat_sheet() {
            for (name, _) in rows {
                script.push_str(&format!("_ = {}\n", name.trim_end_matches("()")));
            }
        }
        let report = ScriptRuntime::new().run(&script, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{script}");
    }

    /// Every example in the Help window must run (roadmap Part 4 E). Inserted
    /// in order after the imports, they form one working script, and its
    /// tick() and event handlers work on the factory it builds.
    #[test]
    fn help_examples_run() {
        let script: String = std::iter::once(IMPORTS)
            .chain(BUILD.iter().map(|c| c.example))
            .chain(OPERATE.iter().map(|c| c.example))
            .chain([
                "first_tick = tick\ndef tick():\n    first_tick()\n    show_smelter()\n    \
                 report()\n    belt_is_busy()\n    every_ten_seconds()\n    check()\n    \
                 boiler_is_hot()\n    hot_hours()\n",
            ])
            .collect();
        let runtime = ScriptRuntime::new();
        let report = runtime.run(&script, DEPLOY_BUDGET);
        assert_eq!(
            report.outcome,
            RunOutcome::Finished,
            "{script}\n{:?}",
            report.output
        );

        // tick() reads the factory the script built.
        let mut factory = Factory::default();
        reconcile::apply(&mut factory, &report.plan.unwrap());
        factory.step();
        runtime.set_world(WorldView::of(&factory, Default::default()));
        assert_eq!(runtime.tick(TICK_BUDGET).outcome, HookOutcome::Finished);
        for (name, args) in [
            ("on_train", vec![EventArg::Int(12)]),
            ("on_contract_complete", vec![EventArg::Text("Hello".into())]),
        ] {
            let outcome = runtime.event(name, &args, TICK_BUDGET).outcome;
            assert_eq!(outcome, HookOutcome::Finished, "{name}");
        }
    }

    /// The Shop's examples run once their upgrade is bought.
    #[test]
    fn shop_examples_run() {
        let factory = Factory {
            unlocked: Upgrade::ALL.into_iter().collect(),
            ..Default::default()
        };
        for upgrade in Upgrade::ALL {
            let runtime = ScriptRuntime::new();
            runtime.set_world(WorldView::of(&factory, Default::default()));
            let script = format!("{IMPORTS}{}\n", upgrade.example());
            let report = runtime.run(&script, DEPLOY_BUDGET);
            assert_eq!(report.outcome, RunOutcome::Finished, "{script}");
        }
    }

    /// docs/API.md matches the in-game Help. Set PYPELINE_WRITE_DOCS=1 to
    /// write it after changing the Help.
    #[test]
    fn api_doc_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/API.md");
        let generated = api_markdown();
        if std::env::var_os("PYPELINE_WRITE_DOCS").is_some() {
            std::fs::write(path, &generated).expect("write docs/API.md");
        }
        // Git on Windows may check the file out with CRLF line endings.
        let on_disk = std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            on_disk == generated,
            "docs/API.md is out of date: run PYPELINE_WRITE_DOCS=1 cargo test api_doc"
        );
    }
}
