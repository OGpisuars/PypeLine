//! Python bridge: runs player scripts inside the simulation tick.

pub mod bindings;
pub mod budget;
pub mod commands;
pub mod files;
pub mod hooks;
pub mod memory;
pub mod reconcile;
pub mod runtime;
pub mod sandbox;

use bevy::prelude::*;

use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;
use crate::factory::{Factory, PendingBuild, SimSet, SimTick};
use budget::DEPLOY_BUDGET;
use runtime::{RunOutcome, ScriptRuntime};

/// The roadmap's CANONICAL SAMPLE (Part 1): the README sample and the Phase 1
/// exit test. Keep all three in sync.
pub const CANONICAL_SAMPLE: &str = r#"from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
for x in range(1, 5):
    conveyors.place(x=x, y=0, dir="east")
machines.place("smelter", name="smelter_1", x=5, y=0)

power.connect(generator="steam_1",
              to=["miner_1", "smelter_1"])
"#;

/// Script source waiting to run on the next tick. The editor's Run button
/// fills this; scripts only ever run on a tick boundary.
#[derive(Resource, Default)]
pub struct PendingRun(pub Option<String>);

/// Max lines kept in the console scrollback (ring buffer).
const CONSOLE_CAPACITY: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleKind {
    Output,
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub struct ConsoleLine {
    pub kind: ConsoleKind,
    pub text: String,
}

/// In-game console scrollback.
#[derive(Resource, Default)]
pub struct Console {
    pub lines: std::collections::VecDeque<ConsoleLine>,
}

impl Console {
    pub fn push(&mut self, kind: ConsoleKind, text: impl Into<String>) {
        if self.lines.len() == CONSOLE_CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(ConsoleLine {
            kind,
            text: text.into(),
        });
    }
}

/// The line the last run failed on, for the editor to highlight.
#[derive(Resource, Default)]
pub struct ErrorLine(pub Option<usize>);

/// Why the last run failed, for the failure visuals. Not simulation state:
/// the sim only knows it is halted.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LastFailure {
    #[default]
    None,
    /// An exception or syntax error: belts halt.
    Error,
    /// The step budget ran out: the boiler overheats.
    OutOfSteam,
}

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        // The interpreter is not thread-safe, so it lives on the main thread.
        app.insert_non_send(ScriptRuntime::new())
            .init_resource::<PendingRun>()
            .init_resource::<Console>()
            .init_resource::<ErrorLine>()
            .init_resource::<LastFailure>()
            .add_systems(FixedUpdate, run_pending_script.in_set(SimSet::Scripts));
    }
}

fn run_pending_script(
    runtime: NonSend<ScriptRuntime>,
    mut pending: ResMut<PendingRun>,
    mut console: ResMut<Console>,
    mut error_line: ResMut<ErrorLine>,
    mut build: ResMut<PendingBuild>,
    mut factory: ResMut<Factory>,
    mut failure: ResMut<LastFailure>,
    mut sounds: MessageWriter<SoundCue>,
    tick: Res<SimTick>,
) {
    let Some(source) = pending.0.take() else {
        return;
    };

    console.push(ConsoleKind::Info, format!("> Run (tick {})", tick.0));
    let report = runtime.run(&source, DEPLOY_BUDGET);
    for line in report.output {
        console.push(ConsoleKind::Output, line);
    }

    let at = |line: Option<usize>| line.map(|l| format!(" (line {l})")).unwrap_or_default();
    error_line.0 = None;
    // Any failed run halts the belts until the next good Run (roadmap:
    // ERROR HANDLING). The factory layout itself is left untouched.
    factory.halted = report.outcome != RunOutcome::Finished;
    *failure = match report.outcome {
        RunOutcome::Finished => LastFailure::None,
        RunOutcome::Error { .. } => LastFailure::Error,
        RunOutcome::OutOfSteam { .. } => LastFailure::OutOfSteam,
    };
    sounds.write(SoundCue(match *failure {
        LastFailure::None => Sfx::Run,
        LastFailure::Error => Sfx::Error,
        LastFailure::OutOfSteam => Sfx::Overheat,
    }));
    build.0 = report.plan;
    match report.outcome {
        RunOutcome::Finished => console.push(
            ConsoleKind::Info,
            format!(
                "Done. Steam used: {}/{}",
                report.steps_used, report.steps_limit
            ),
        ),
        RunOutcome::Error { line, message } => {
            error_line.0 = line;
            console.push(ConsoleKind::Error, format!("Error{}: {message}", at(line)));
            console.push(ConsoleKind::Error, "Belts halted until the next good Run.");
        }
        RunOutcome::OutOfSteam { line } => {
            error_line.0 = line;
            console.push(
                ConsoleKind::Error,
                format!(
                    "Out of steam{}! The boiler overheats. Used all {} steps.",
                    at(line),
                    report.steps_limit
                ),
            );
        }
    }
}
