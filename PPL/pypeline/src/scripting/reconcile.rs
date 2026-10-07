//! Hot-reload: apply a build plan to the running factory.
//!
//! Only what changed is touched. Belts keep their items, machines keep their
//! buffers, and anything that disappeared from the script is removed with its
//! items returned to the station inventory (roadmap: HOT-RELOAD ON RUN).

use crate::factory::Factory;
use crate::factory::conveyors::Conveyor;
use crate::factory::machines::Machine;

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
    // New and turned belts. A turned belt keeps its items.
    for (&pos, &dir) in &plan.conveyors {
        match factory.conveyors.get_mut(&pos) {
            Some(belt) if belt.dir == dir => {}
            Some(belt) => {
                belt.dir = dir;
                report.changed += 1;
            }
            None => {
                factory.conveyors.insert(pos, Conveyor::new(dir));
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
                if m.dir != planned.dir || m.ore != planned.ore {
                    m.dir = planned.dir;
                    m.ore = planned.ore;
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
                factory.machines.insert(
                    name.clone(),
                    Machine::new(planned.kind, planned.pos, planned.dir, planned.ore),
                );
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
