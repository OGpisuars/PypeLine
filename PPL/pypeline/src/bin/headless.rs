//! Run a PypeLine script without a window (roadmap Part 4 E).
//!
//! Usage: `cargo run --bin headless -- path/to/main.py [ticks]`
//! Prints the console output, how the run ended, what the factory made, and
//! the state hash after `ticks` ticks (default 600, i.e. 30 seconds).

use std::process::ExitCode;

use pypeline::factory::Factory;
use pypeline::factory::items::ItemKind;
use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::reconcile;
use pypeline::scripting::runtime::{RunOutcome, ScriptRuntime};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: headless <script.py> [ticks]");
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
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("could not read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let report = ScriptRuntime::new().run(&source, DEPLOY_BUDGET);
    for line in &report.output {
        println!("{line}");
    }
    let Some(plan) = report.plan else {
        eprintln!("run failed: {:?}", report.outcome);
        return ExitCode::FAILURE;
    };
    debug_assert_eq!(report.outcome, RunOutcome::Finished);

    let mut factory = Factory::default();
    println!("{}", reconcile::apply(&mut factory, &plan).summary());
    for _ in 0..ticks {
        factory.step();
    }
    println!(
        "after {ticks} ticks: {} iron ore mined, {} iron plates made",
        factory.produced(ItemKind::IronOre),
        factory.produced(ItemKind::IronPlate)
    );
    println!("state hash: {:016x}", factory.state_hash());
    ExitCode::SUCCESS
}
