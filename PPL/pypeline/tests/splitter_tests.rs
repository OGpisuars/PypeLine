//! Splitters: one belt into two, half each way, and never jammed by a full
//! side.

use pypeline::factory::conveyors::{Conveyor, Split};
use pypeline::factory::items::ItemKind;
use pypeline::factory::machines::{Machine, MachineKind};
use pypeline::factory::{Dir, Factory, Pos};
use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::reconcile;
use pypeline::scripting::runtime::{RunOutcome, ScriptRuntime};

const FEED: Pos = Pos::new(1, 5);
const SPLITTER: Pos = Pos::new(2, 5);
const NORTH: Pos = Pos::new(2, 6);
const SOUTH: Pos = Pos::new(2, 4);

/// A belt running east into a north/south splitter, with a station on each
/// of the tiles in `stations`.
fn splitter_factory(stations: &[Pos]) -> Factory {
    let mut factory = Factory::default();
    factory.conveyors.insert(FEED, Conveyor::new(Dir::East));
    let mut splitter = Conveyor::new(Dir::North);
    splitter.split = Some(Split::new(Dir::North, Dir::South));
    factory.conveyors.insert(SPLITTER, splitter);
    for (i, &pos) in stations.iter().enumerate() {
        factory.machines.insert(
            format!("station_{i}"),
            Machine::new(MachineKind::Station, pos, Dir::East, None),
        );
    }
    factory.rebuild_index();
    factory
}

/// Keep the feeding belt full for `ticks` ticks (less than one train visit,
/// so the stations are not emptied).
fn feed(factory: &mut Factory, ticks: u32) {
    for _ in 0..ticks {
        let belt = factory.conveyors.get_mut(&FEED).unwrap();
        if belt.can_accept() {
            belt.push_back(ItemKind::IronOre);
        }
        factory.step();
    }
}

fn held(factory: &Factory, station: &str) -> usize {
    factory.machines[station].input.len()
}

#[test]
fn splitters_send_half_each_way() {
    let mut factory = splitter_factory(&[NORTH, SOUTH]);
    feed(&mut factory, 400);
    let (north, south) = (held(&factory, "station_0"), held(&factory, "station_1"));
    assert!(north + south >= 20, "only {north} + {south} items arrived");
    assert!(north.abs_diff(south) <= 1, "{north} north, {south} south");
}

#[test]
fn a_blocked_side_does_not_jam_the_splitter() {
    // Nothing south of the splitter: every item goes north.
    let mut factory = splitter_factory(&[NORTH]);
    feed(&mut factory, 400);
    assert!(held(&factory, "station_0") >= 20);
    assert!(factory.conveyors[&SPLITTER].items.len() <= 2);
}

#[test]
fn splitters_are_built_from_python() {
    let runtime = ScriptRuntime::new();
    let report = runtime.run(
        "from auto import conveyors, splitters\n\
         conveyors.place(x=1, y=5, dir=\"east\")\n\
         splitters.place(x=2, y=5, dir1=\"north\", dir2=\"south\")\n",
        DEPLOY_BUDGET,
    );
    assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
    let mut factory = Factory::default();
    reconcile::apply(&mut factory, &report.plan.unwrap());
    assert_eq!(
        factory.conveyors[&SPLITTER].split,
        Some(Split::new(Dir::North, Dir::South))
    );

    // Running the same script again keeps whose turn it is.
    factory.conveyors.get_mut(&SPLITTER).unwrap().split = Some(Split {
        outputs: [Dir::North, Dir::South],
        next: 1,
    });
    let again = runtime.run(
        "from auto import conveyors, splitters\n\
         conveyors.place(x=1, y=5, dir=\"east\")\n\
         splitters.place(x=2, y=5, dir1=\"north\", dir2=\"south\")\n",
        DEPLOY_BUDGET,
    );
    let changes = reconcile::apply(&mut factory, &again.plan.unwrap());
    assert_eq!(changes, reconcile::ReconcileReport::default());
    assert_eq!(factory.conveyors[&SPLITTER].split.unwrap().next, 1);
}

#[test]
fn a_splitter_needs_two_different_directions() {
    let report = ScriptRuntime::new().run(
        "from auto import splitters\nsplitters.place(x=2, y=5, dir1=\"north\", dir2=\"north\")\n",
        DEPLOY_BUDGET,
    );
    assert!(
        matches!(report.outcome, RunOutcome::Error { line: Some(2), .. }),
        "{:?}",
        report.outcome
    );
}
