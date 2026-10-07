//! Python bridge: runs player scripts inside the simulation tick.

pub mod budget;
pub mod hooks;
pub mod runtime;

use bevy::prelude::*;

use crate::factory::{SimSet, SimTick};
use budget::DEPLOY_BUDGET;
use runtime::{RunOutcome, ScriptRuntime};

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

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        // The interpreter is not thread-safe, so it lives on the main thread.
        app.insert_non_send(ScriptRuntime::new())
            .init_resource::<PendingRun>()
            .init_resource::<Console>()
            .init_resource::<ErrorLine>()
            .add_systems(FixedUpdate, run_pending_script.in_set(SimSet::Scripts));
    }
}

fn run_pending_script(
    runtime: NonSend<ScriptRuntime>,
    mut pending: ResMut<PendingRun>,
    mut console: ResMut<Console>,
    mut error_line: ResMut<ErrorLine>,
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
