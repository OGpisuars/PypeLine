//! Play a program without a window: deploy it, then run its events and
//! tick() around every factory step in the same order as the game does
//! (roadmap Part 4 E). Used by the golden-solution tests and the
//! `headless` binary.

use crate::factory::{Factory, ProductionHistory};
use crate::scripting::budget::{DEPLOY_BUDGET, TICK_BUDGET};
use crate::scripting::operate::{self, WorldView};
use crate::scripting::reconcile;
use crate::scripting::runtime::{EventArg, HookOutcome, Program, RunReport, ScriptRuntime};

/// tick() running out of steam this many ticks in a row overheats the boiler.
const MAX_OVERRUNS: u32 = 3;

pub struct Headless {
    pub runtime: ScriptRuntime,
    pub factory: Factory,
    history: ProductionHistory,
    /// Events waiting for the next tick, like the game's `ScriptEvents`.
    events: Vec<(&'static str, Vec<EventArg>)>,
    overruns: u32,
    last_sale: Option<u64>,
    /// Everything printed and every hook error, in order.
    pub output: Vec<String>,
}

impl Headless {
    /// Run `program` against `factory`. A failed run returns its report.
    pub fn start(program: &Program, factory: Factory) -> Result<Self, Box<RunReport>> {
        let runtime = ScriptRuntime::new();
        let history = ProductionHistory::default();
        runtime.set_world(WorldView::of(&factory, history.per_minute(&factory)));
        let report = runtime.run_program(program, DEPLOY_BUDGET);
        let Some(plan) = report.plan.as_ref() else {
            return Err(Box::new(report));
        };
        let mut factory = factory;
        reconcile::apply(&mut factory, plan);
        let last_sale = factory.last_sale.as_ref().map(|s| s.tick);
        Ok(Self {
            runtime,
            factory,
            history,
            events: Vec::new(),
            overruns: 0,
            last_sale,
            output: report.output,
        })
    }

    /// One game tick: events and tick(), then the factory step.
    pub fn step(&mut self) {
        if !self.factory.halted {
            self.run_hooks();
        }
        self.factory.step();
        if let Some(sale) = &self.factory.last_sale
            && self.last_sale != Some(sale.tick)
        {
            self.last_sale = Some(sale.tick);
            self.events
                .push(("on_train", vec![EventArg::Int(sale.coins as i64)]));
        }
        if self.factory.ticks.is_multiple_of(20) {
            self.history.sample(&self.factory);
        }
    }

    /// Items made over the last minute of game time.
    pub fn per_minute(&self) -> std::collections::BTreeMap<crate::factory::items::ItemKind, u64> {
        self.history.per_minute(&self.factory)
    }

    /// Queue an event for the next tick, like finishing a contract does.
    pub fn send(&mut self, name: &'static str, args: Vec<EventArg>) {
        self.events.push((name, args));
    }

    fn run_hooks(&mut self) {
        let world = WorldView::of(&self.factory, self.history.per_minute(&self.factory));
        self.runtime.set_world(world);
        let mut reports = Vec::new();
        for (name, args) in std::mem::take(&mut self.events) {
            reports.push((name, self.runtime.event(name, &args, TICK_BUDGET)));
        }
        reports.push(("tick", self.runtime.tick(TICK_BUDGET)));
        for (name, report) in reports {
            self.output.extend(report.output);
            operate::apply_ops(&mut self.factory, &report.ops);
            match report.outcome {
                HookOutcome::NotDefined => {}
                HookOutcome::Finished => {
                    if name == "tick" {
                        self.overruns = 0;
                    }
                }
                HookOutcome::Error { line, message } => self
                    .output
                    .push(format!("Error in {name}() (line {line:?}): {message}")),
                HookOutcome::OutOfSteam { .. } if name == "tick" => {
                    self.overruns += 1;
                    if self.overruns >= MAX_OVERRUNS {
                        self.factory.halted = true;
                        self.output.push("tick() overheated the boiler".into());
                    }
                }
                HookOutcome::OutOfSteam { .. } => {
                    self.output.push(format!("{name}() ran out of steam"));
                }
            }
        }
    }

    /// Did any event or tick() fail so far?
    pub fn had_errors(&self) -> bool {
        self.output
            .iter()
            .any(|line| line.starts_with("Error in ") || line.contains("overheated"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::items::ItemKind;

    #[test]
    fn tick_and_events_run_between_steps() {
        let main = format!(
            "{}\nconveyors.place(x=6, y=0, dir=\"east\")\n\
             machines.place(\"station\", name=\"station_1\", x=7, y=0)\n\
             trains = 0\n\
             def on_train(coins):\n    global trains\n    trains += 1\n    print('paid', coins)\n\
             def tick():\n    if clock.tick() == 100:\n        machines.disable('miner_1')\n",
            crate::scripting::CANONICAL_SAMPLE.replace("import power", "import power, clock")
        );
        let mut game = Headless::start(&Program::main_only(&main), Factory::default()).unwrap();
        for _ in 0..1300 {
            game.step();
        }
        assert!(!game.had_errors(), "{:?}", game.output);
        assert!(!game.factory.machines["miner_1"].enabled);
        // The miner stopped after 5 seconds, so only a couple of ore exist.
        assert!(game.factory.produced(ItemKind::IronOre) <= 3);
        assert_eq!(
            game.output.iter().filter(|l| l.starts_with("paid")).count(),
            2
        );
    }
}
