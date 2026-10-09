//! Hot-reload: apply a build plan to the running factory.
//!
//! Only what changed is touched. Belts keep their items, machines keep their
//! buffers, and anything that disappeared from the script is removed with its
//! items returned to the station inventory (roadmap: HOT-RELOAD ON RUN).

use crate::factory::conveyors::{Conveyor, Split};
use crate::factory::machines::Machine;
use crate::factory::{Dir, Factory};

use super::commands::BuildPlan;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileReport {
    pub created: usize,
    pub changed: usize,
    pub removed: usize,
}

impl ReconcileReport {
    pub fn summary(&self) -> String {
        if self.created + self.changed + self.removed == 0 {
            return "Factory unchanged.".into();
        }
        format!(
            "Factory updated: {} built, {} changed, {} removed.",
            self.created, self.changed, self.removed
        )
    }
}

/// A splitter's outputs (None for a plain belt), to compare with the plan.
fn outputs(belt: &Conveyor) -> Option<[Dir; 2]> {
    belt.split.map(|split| split.outputs)
}

pub fn apply(factory: &mut Factory, plan: &BuildPlan) -> ReconcileReport {
    let mut report = ReconcileReport::default();

    // Belts that are gone from the script.
    let gone: Vec<_> = factory
        .conveyors
        .keys()
        .filter(|pos| !plan.conveyors.contains_key(pos))
        .copied()
        .collect();
    for pos in gone {
        let belt = factory
            .conveyors
            .remove(&pos)
            .expect("key came from the map");
        factory.stash(belt.items.into_iter().map(|i| i.kind));
        report.removed += 1;
    }
    // New, turned and upgraded belts and splitters. A changed belt keeps
    // its items, and a splitter that did not change keeps whose turn it is.
    for (&pos, planned) in &plan.conveyors {
        match factory.conveyors.get_mut(&pos) {
            Some(belt)
                if belt.dir == planned.dir
                    && belt.tier == planned.tier
                    && outputs(belt) == planned.split => {}
            Some(belt) => {
                belt.dir = planned.dir;
                belt.tier = planned.tier;
                if outputs(belt) != planned.split {
                    belt.split = planned.split.map(|[a, b]| Split::new(a, b));
                }
                report.changed += 1;
            }
            None => {
                let mut belt = Conveyor::new(planned.dir);
                belt.tier = planned.tier;
                belt.split = planned.split.map(|[a, b]| Split::new(a, b));
                factory.conveyors.insert(pos, belt);
                report.created += 1;
            }
        }
    }

    // Machines that are gone from the script.
    let gone: Vec<_> = factory
        .machines
        .keys()
        .filter(|name| !plan.machines.contains_key(*name))
        .cloned()
        .collect();
    for name in gone {
        let machine = factory
            .machines
            .remove(&name)
            .expect("key came from the map");
        factory.stash(machine.input.into_iter().chain(machine.output));
        report.removed += 1;
    }
    // New and changed machines.
    for (name, planned) in &plan.machines {
        match factory.machines.get_mut(name) {
            Some(m) if m.kind == planned.kind && m.pos == planned.pos => {
                // An upgraded machine keeps its items and its job.
                if m.dir != planned.dir || m.ore != planned.ore || m.tier != planned.tier {
                    m.dir = planned.dir;
                    m.ore = planned.ore;
                    m.tier = planned.tier;
                    report.changed += 1;
                }
                // A crafter's recipe= wins; without one it keeps the recipe
                // picked in the game. A new recipe returns what it held.
                if planned.recipe.is_some() && m.recipe != planned.recipe {
                    let held = m.set_recipe(planned.recipe);
                    factory.stash(held);
                    report.changed += 1;
                }
            }
            existing => {
                // Moved or a different kind: rebuild it, keeping its items.
                if let Some(old) = existing {
                    let items: Vec<_> = old.input.drain(..).chain(old.output.drain(..)).collect();
                    factory.stash(items);
                    report.changed += 1;
                } else {
                    report.created += 1;
                }
                let mut machine = Machine::new(planned.kind, planned.pos, planned.dir, planned.ore);
                machine.tier = planned.tier;
                machine.recipe = planned.recipe;
                factory.machines.insert(name.clone(), machine);
            }
        }
    }

    if factory.power != plan.power {
        factory.power = plan.power.clone();
        report.changed += 1;
    }

    factory.rebuild_index();
    factory.halted = false;
    if report != ReconcileReport::default() {
        factory.layout_version += 1;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::items::ItemKind;
    use crate::scripting::CANONICAL_SAMPLE;
    use crate::scripting::budget::DEPLOY_BUDGET;
    use crate::scripting::runtime::{RunOutcome, ScriptRuntime};

    fn build(runtime: &ScriptRuntime, factory: &mut Factory, source: &str) -> ReconcileReport {
        let report = runtime.run(source, DEPLOY_BUDGET);
        assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
        apply(factory, &report.plan.expect("a finished run has a plan"))
    }

    /// Phase 1 exit test: the canonical sample produces iron plates, headless.
    #[test]
    fn canonical_sample_makes_iron_plates() {
        let runtime = ScriptRuntime::new();
        let mut factory = Factory::default();
        let report = build(&runtime, &mut factory, CANONICAL_SAMPLE);
        assert_eq!(report.created, 7);

        // 30 seconds of game time.
        for _ in 0..600 {
            factory.step();
        }
        assert!(factory.produced(ItemKind::IronOre) > 0);
        assert!(
            factory.produced(ItemKind::IronPlate) > 0,
            "{:?}",
            factory.produced
        );
    }

    #[test]
    fn the_train_buys_from_stations() {
        let runtime = ScriptRuntime::new();
        let mut factory = Factory::default();
        // The sample, but the smelter feeds a belt into a station.
        let script = CANONICAL_SAMPLE.replace(
            "machines.place(\"smelter\", name=\"smelter_1\", x=5, y=0)",
            "machines.place(\"smelter\", name=\"smelter_1\", x=5, y=0)\n\
             conveyors.place(x=6, y=0, dir=\"east\")\n\
             machines.place(\"station\", name=\"station_1\", x=7, y=0)",
        );
        build(&runtime, &mut factory, &script);
        for _ in 0..1200 {
            factory.step();
        }
        let sale = factory.last_sale.as_ref().expect("the train came");
        assert!(sale.items.contains_key(&ItemKind::IronPlate), "{sale:?}");
        assert!(factory.coins >= 4);
    }

    #[test]
    fn running_twice_changes_nothing() {
        let runtime = ScriptRuntime::new();
        let mut factory = Factory::default();
        build(&runtime, &mut factory, CANONICAL_SAMPLE);
        for _ in 0..200 {
            factory.step();
        }
        let before = factory.clone();
        let report = build(&runtime, &mut factory, CANONICAL_SAMPLE);
        assert_eq!(report, ReconcileReport::default());
        assert_eq!(factory, before, "items and machines must survive a re-run");
    }

    #[test]
    fn removed_belts_return_their_items() {
        let runtime = ScriptRuntime::new();
        let mut factory = Factory::default();
        build(&runtime, &mut factory, CANONICAL_SAMPLE);
        for _ in 0..200 {
            factory.step();
        }
        let on_belts: usize = factory.conveyors.values().map(|b| b.items.len()).sum();
        assert!(on_belts > 0);

        let without_belts = CANONICAL_SAMPLE.replace("conveyors.place", "pass  # ");
        let without_belts = without_belts.replace(
            "from auto import conveyors, machines",
            "from auto import machines",
        );
        let report = build(&runtime, &mut factory, &without_belts);
        assert_eq!(report.removed, 4);
        assert!(factory.conveyors.is_empty());
        assert!(factory.inventory.values().sum::<u64>() >= on_belts as u64);
    }

    #[test]
    fn failed_run_changes_nothing() {
        let runtime = ScriptRuntime::new();
        let mut factory = Factory::default();
        build(&runtime, &mut factory, CANONICAL_SAMPLE);
        let before = factory.clone();

        let broken = format!("{CANONICAL_SAMPLE}\nconveyors.place(x=99, y=0, dir=\"east\")");
        let report = runtime.run(&broken, DEPLOY_BUDGET);
        assert!(
            matches!(report.outcome, RunOutcome::Error { line: Some(13), .. }),
            "{:?}",
            report.outcome
        );
        assert!(report.plan.is_none());
        assert_eq!(factory, before);
    }
}
