//! Run a PypeLine script without a window (roadmap Part 4 E).
//!
//! Usage: `cargo run --bin headless -- path/to/main.py [ticks]`
//! Every other `.py` file next to main.py can be imported, like in the
//! game. tick() and events run too. Prints the console output, what the
//! factory made, and the state hash after `ticks` ticks (default 600, i.e.
//! 30 seconds).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use pypeline::factory::Factory;
use pypeline::factory::items::ItemKind;
use pypeline::scripting::headless::Headless;
use pypeline::scripting::runtime::Program;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: headless <main.py> [ticks]");
        return ExitCode::FAILURE;
    };
    let ticks: u64 = match args.next().map(|t| t.parse()) {
        None => 600,
        Some(Ok(t)) => t,
        Some(Err(_)) => {
            eprintln!("ticks must be a whole number");
            return ExitCode::FAILURE;
        }
    };
    let program = match load(Path::new(&path)) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("could not read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut game = match Headless::start(&program, Factory::default()) {
        Ok(game) => game,
        Err(report) => {
            for line in &report.output {
                println!("{line}");
            }
            eprintln!("run failed: {:?}", report.outcome);
            return ExitCode::FAILURE;
        }
    };
    for _ in 0..ticks {
        game.step();
    }
    for line in &game.output {
        println!("{line}");
    }
    println!(
        "after {ticks} ticks: {} iron ore mined, {} iron plates made, {} coins",
        game.factory.produced(ItemKind::IronOre),
        game.factory.produced(ItemKind::IronPlate),
        game.factory.coins
    );
    println!("state hash: {:016x}", game.factory.state_hash());
    ExitCode::SUCCESS
}

/// main.py plus every other .py file in its folder, as modules.
fn load(main: &Path) -> std::io::Result<Program> {
    let source = std::fs::read_to_string(main)?;
    let mut modules = BTreeMap::new();
    let folder = main.parent().filter(|p| !p.as_os_str().is_empty());
    for entry in std::fs::read_dir(folder.unwrap_or(Path::new(".")))? {
        let file = entry?.path();
        if file.file_name() == main.file_name() || file.extension().is_none_or(|e| e != "py") {
            continue;
        }
        if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
            modules.insert(stem.to_owned(), std::fs::read_to_string(&file)?);
        }
    }
    Ok(Program {
        main: source,
        modules,
    })
}
