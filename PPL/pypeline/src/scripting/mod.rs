//! Python bridge: runs player scripts inside the simulation tick.

pub mod bindings;
pub mod budget;
pub mod commands;
pub mod concepts;
pub mod console_api;
pub mod errors;
pub mod files;
pub mod hooks;
pub mod memory;
pub mod operate;
pub mod reconcile;
pub mod runtime;
pub mod sandbox;

use bevy::prelude::*;

use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;
use crate::factory::{Factory, PendingBuild, SimSet, SimTick};
use budget::DEPLOY_BUDGET;
use runtime::{HookOutcome, RunOutcome, ScriptRuntime};

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
pub struct PendingRun(pub Option<runtime::Program>);

/// The editor's Stop and Clean Run buttons, applied on the next tick.
#[derive(Resource, Default)]
pub struct RunRequests {
    /// Halt the belts until the next Run.
    pub stop: bool,
    /// Wipe the factory (keeping coins and stats) before building, if the
    /// pending run succeeds.
    pub clean: bool,
}

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
    /// Set by `console.color(...)` in the script.
    pub color: Option<console_api::ConsoleColor>,
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
            color: None,
        });
    }

    /// Apply what a script did to the console: colored lines and clears.
    pub fn apply(&mut self, ops: Vec<console_api::ConsoleOp>) {
        for op in ops {
            match op {
                console_api::ConsoleOp::Clear => self.lines.clear(),
                console_api::ConsoleOp::Line { text, color } => {
                    self.push(ConsoleKind::Output, text);
                    if let Some(line) = self.lines.back_mut() {
                        line.color = color;
                    }
                }
            }
        }
    }
}

/// The last program that ran to the end, which is what built the factory.
/// Contracts check their concept requirements against it.
#[derive(Resource, Default)]
pub struct LastGoodScript {
    pub program: runtime::Program,
    pub steps: u64,
}

impl LastGoodScript {
    /// Every file's source together (for hints that look for names).
    pub fn all_sources(&self) -> String {
        self.program.sources().collect::<Vec<_>>().join("\n")
    }
}

/// Events waiting to be delivered to the script's handlers, in order.
#[derive(Resource, Default)]
pub struct ScriptEvents(pub Vec<(&'static str, Vec<runtime::EventArg>)>);

/// Which optional handlers the current script defines.
#[derive(Resource, Default)]
pub struct DefinedHandlers {
    pub on_contract_complete: bool,
}

/// tick() overruns in a row; three overheat the boiler.
#[derive(Resource, Default)]
struct Overruns(u32);

/// Failed runs in a row, for stuck detection (roadmap Part 4 F).
#[derive(Resource, Default)]
struct FailStreak(u32);

/// Where the last run failed (file name, line), for the editor to
/// highlight. The file is "main.py" or one of the player's other files.
#[derive(Resource, Default)]
pub struct ErrorLine(pub Option<(String, usize)>);

impl ErrorLine {
    fn set(&mut self, file: &Option<String>, line: Option<usize>) {
        self.0 = line.map(|l| (file.clone().unwrap_or_else(|| "main.py".into()), l));
    }
}

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
            .init_resource::<RunRequests>()
            .init_resource::<Console>()
            .init_resource::<ErrorLine>()
            .init_resource::<LastFailure>()
            .init_resource::<LastGoodScript>()
            .init_resource::<ScriptEvents>()
            .init_resource::<DefinedHandlers>()
            .init_resource::<Overruns>()
            .init_resource::<FailStreak>()
            .add_systems(
                FixedUpdate,
                (run_pending_script, run_script_hooks)
                    .chain()
                    .in_set(SimSet::Scripts),
            );
    }
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn run_pending_script(
    runtime: NonSend<ScriptRuntime>,
    mut pending: ResMut<PendingRun>,
    mut console: ResMut<Console>,
    mut error_line: ResMut<ErrorLine>,
    mut build: ResMut<PendingBuild>,
    mut factory: ResMut<Factory>,
    mut failure: ResMut<LastFailure>,
    mut sounds: MessageWriter<SoundCue>,
    mut requests: ResMut<RunRequests>,
    mut last_good: ResMut<LastGoodScript>,
    mut streak: ResMut<FailStreak>,
    mut handlers: ResMut<DefinedHandlers>,
    history: Res<crate::factory::ProductionHistory>,
    tick: Res<SimTick>,
) {
    if std::mem::take(&mut requests.stop) {
        factory.halted = true;
        *failure = LastFailure::None;
        console.push(ConsoleKind::Info, "Stopped. Press Run to start again.");
    }
    let Some(program) = pending.0.take() else {
        return;
    };
    let clean = std::mem::take(&mut requests.clean);
    runtime.set_world(operate::WorldView::of(
        &factory,
        history.per_minute(&factory),
    ));

    console.push(ConsoleKind::Info, format!("> Run (tick {})", tick.0));
    let report = runtime.run_program(&program, DEPLOY_BUDGET);
    console.apply(report.console);

    let file = report.error_file.clone();
    let at = |line: Option<usize>| match (&file, line) {
        (Some(file), Some(l)) => format!(" ({file}, line {l})"),
        (Some(file), None) => format!(" ({file})"),
        (None, Some(l)) => format!(" (line {l})"),
        (None, None) => String::new(),
    };
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
    if clean && report.plan.is_some() {
        factory.clean_reset();
        console.push(
            ConsoleKind::Info,
            "Clean Run: the factory was cleared and rebuilt.",
        );
    }
    build.0 = report.plan;
    handlers.on_contract_complete = runtime.defines("on_contract_complete");
    if report.outcome == RunOutcome::Finished {
        last_good.program = program.clone();
        last_good.steps = report.steps_used;
        streak.0 = 0;
    } else {
        streak.0 += 1;
    }
    match report.outcome {
        RunOutcome::Finished => console.push(
            ConsoleKind::Info,
            format!(
                "Done. Steam used: {}/{}",
                report.steps_used, report.steps_limit
            ),
        ),
        RunOutcome::Error { line, message } => {
            error_line.set(&report.error_file, line);
            console.push(ConsoleKind::Error, format!("Error{}: {message}", at(line)));
            let all: Vec<&str> = program.sources().collect();
            if let Some(hint) = errors::explain(&message, &all.join("\n")) {
                console.push(ConsoleKind::Info, format!("Hint: {hint}"));
            }
            console.push(ConsoleKind::Error, "Belts halted until the next good Run.");
        }
        RunOutcome::OutOfSteam { line } => {
            error_line.set(&report.error_file, line);
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
    if streak.0 == 3 {
        console.push(
            ConsoleKind::Info,
            "Stuck? Every contract in the Manual (F2) has hints, and F1 lists every command.",
        );
    }
}

/// Every tick: deliver events to their handlers, then call tick(). Both read
/// a fresh factory snapshot and queue operations for the next factory step.
#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn run_script_hooks(
    runtime: NonSend<ScriptRuntime>,
    mut factory: ResMut<Factory>,
    history: Res<crate::factory::ProductionHistory>,
    mut events: ResMut<ScriptEvents>,
    mut pending_ops: ResMut<crate::factory::PendingOps>,
    mut console: ResMut<Console>,
    mut overruns: ResMut<Overruns>,
    mut failure: ResMut<LastFailure>,
    mut error_line: ResMut<ErrorLine>,
    last_good: Res<LastGoodScript>,
    mut sounds: MessageWriter<SoundCue>,
) {
    if factory.halted {
        events.0.clear();
        return;
    }
    runtime.set_world(operate::WorldView::of(
        &factory,
        history.per_minute(&factory),
    ));

    let mut reports = Vec::new();
    for (name, args) in std::mem::take(&mut events.0) {
        reports.push((name, runtime.event(name, &args, budget::TICK_BUDGET)));
    }
    reports.push(("tick", runtime.tick(budget::TICK_BUDGET)));

    for (name, report) in reports {
        console.apply(report.console);
        pending_ops.0.extend(report.ops);
        let file = report.error_file.clone();
        let at = |line: Option<usize>| match (&file, line) {
            (Some(file), Some(l)) => format!(" ({file}, line {l})"),
            (Some(file), None) => format!(" ({file})"),
            (None, Some(l)) => format!(" (line {l})"),
            (None, None) => String::new(),
        };
        match report.outcome {
            HookOutcome::NotDefined => {}
            HookOutcome::Finished => {
                if name == "tick" {
                    overruns.0 = 0;
                }
            }
            HookOutcome::Error { line, message } => {
                error_line.set(&report.error_file, line);
                console.push(
                    ConsoleKind::Error,
                    format!("Error in {name}(){}: {message}", at(line)),
                );
                if let Some(hint) = errors::explain(&message, &last_good.all_sources()) {
                    console.push(ConsoleKind::Info, format!("Hint: {hint}"));
                }
                if name == "tick" {
                    console.push(
                        ConsoleKind::Error,
                        "tick() is switched off until the next Run. The factory keeps going.",
                    );
                }
                sounds.write(SoundCue(Sfx::Error));
            }
            HookOutcome::OutOfSteam { line } => {
                if name != "tick" {
                    console.push(
                        ConsoleKind::Error,
                        format!("{name}() ran out of steam{}.", at(line)),
                    );
                    continue;
                }
                overruns.0 += 1;
                if overruns.0 == 1 {
                    console.push(
                        ConsoleKind::Error,
                        format!(
                            "tick() ran out of steam{}: the boiler is heating up!",
                            at(line)
                        ),
                    );
                }
                if overruns.0 >= 3 {
                    factory.halted = true;
                    *failure = LastFailure::OutOfSteam;
                    error_line.set(&report.error_file, line);
                    console.push(
                        ConsoleKind::Error,
                        "The boiler overheated: tick() ran out of steam 3 ticks in a row. Make \
                         tick() do less each tick (or spread work with yield), then Run again.",
                    );
                    sounds.write(SoundCue(Sfx::Overheat));
                    overruns.0 = 0;
                }
            }
        }
    }
}
