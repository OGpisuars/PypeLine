//! tick(), generators, events and the operate API (roadmap Phase 3B).

use std::collections::BTreeMap;

use pypeline::factory::Pos;
use pypeline::factory::items::ItemKind;
use pypeline::factory::machines::MachineKind;
use pypeline::scripting::budget::{DEPLOY_BUDGET, TICK_BUDGET};
use pypeline::scripting::operate::{MachineView, Op, WorldView};
use pypeline::scripting::runtime::{EventArg, HookOutcome, RunOutcome, ScriptRuntime};

fn start(source: &str) -> ScriptRuntime {
    let runtime = ScriptRuntime::new();
    let report = runtime.run(source, DEPLOY_BUDGET);
    assert_eq!(report.outcome, RunOutcome::Finished, "{:?}", report.output);
    runtime
}

#[test]
fn tick_keeps_its_variables() {
    let rt = start("count = 0\ndef tick():\n    global count\n    count += 1\n    print(count)");
    let outputs: Vec<String> = (0..3).flat_map(|_| rt.tick(TICK_BUDGET).output).collect();
    assert_eq!(outputs, vec!["1", "2", "3"]);
}

#[test]
fn generator_tick_resumes_each_tick() {
    let rt = start("def tick():\n    for i in range(2):\n        print(i)\n        yield");
    let outputs: Vec<String> = (0..5).flat_map(|_| rt.tick(TICK_BUDGET).output).collect();
    // 0, 1, then the generator ends and a fresh one starts.
    assert_eq!(outputs, vec!["0", "1", "0", "1"]);
}

#[test]
fn no_tick_means_nothing_happens() {
    let rt = start("x = 1");
    assert_eq!(rt.tick(TICK_BUDGET).outcome, HookOutcome::NotDefined);
}

#[test]
fn build_calls_are_refused_in_tick() {
    let rt =
        start("from auto import conveyors\ndef tick():\n    conveyors.place(x=1, y=0, dir='east')");
    match rt.tick(TICK_BUDGET).outcome {
        HookOutcome::Error { line, message } => {
            assert_eq!(line, Some(3));
            assert!(message.contains("main.py"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn switching_machines_is_queued_and_refused_while_building() {
    let rt = ScriptRuntime::new();
    let mut world = WorldView::default();
    world.machines.insert(
        "m".into(),
        MachineView {
            kind: MachineKind::Miner,
            working: false,
            input: 0,
            output: 0,
            powered: true,
            enabled: true,
            tier: 1,
            state: pypeline::factory::stats::MachineState::Working,
            heat: 15,
            overheated: false,
        },
    );
    rt.set_world(world);
    let report = rt.run(
        "from auto import machines\nmachines.disable('m')",
        DEPLOY_BUDGET,
    );
    assert!(matches!(report.outcome, RunOutcome::Error { .. }));

    rt.run(
        "from auto import machines\ndef tick():\n    machines.disable('m')",
        DEPLOY_BUDGET,
    );
    let report = rt.tick(TICK_BUDGET);
    assert_eq!(report.outcome, HookOutcome::Finished);
    assert_eq!(
        report.ops,
        vec![Op::SetEnabled {
            machine: "m".into(),
            on: false
        }]
    );
}

#[test]
fn an_endless_tick_runs_out_of_steam_every_time() {
    let rt = start("def tick():\n    while True:\n        pass");
    for _ in 0..3 {
        assert!(matches!(
            rt.tick(TICK_BUDGET).outcome,
            HookOutcome::OutOfSteam { .. }
        ));
    }
}

#[test]
fn a_broken_tick_stops_until_the_next_run() {
    let rt = start("def tick():\n    print(1 / 0)");
    assert!(matches!(
        rt.tick(TICK_BUDGET).outcome,
        HookOutcome::Error { .. }
    ));
    assert_eq!(rt.tick(TICK_BUDGET).outcome, HookOutcome::NotDefined);
}

#[test]
fn events_get_their_arguments() {
    let rt = start("def on_train(coins):\n    print(f'sold for {coins}')");
    assert!(rt.defines("on_train"));
    assert!(!rt.defines("on_contract_complete"));
    let report = rt.event("on_train", &[EventArg::Int(12)], TICK_BUDGET);
    assert_eq!(report.output, vec!["sold for 12"]);
}

#[test]
fn stats_and_sensors_read_the_snapshot() {
    let rt = ScriptRuntime::new();
    rt.set_world(WorldView {
        ticks: 400,
        coins: 17,
        produced: BTreeMap::from([(ItemKind::IronPlate, 5)]),
        per_minute: BTreeMap::from([(ItemKind::IronPlate, 20)]),
        machines: BTreeMap::new(),
        belts: BTreeMap::from([(Pos::new(2, 0), 3)]),
        ..Default::default()
    });
    let report = rt.run(
        "import stats, sensors, clock\nprint(stats.produced('iron_plate'), stats.per_minute('iron_plate'), stats.coins())\nprint(sensors.count(2, 0), sensors.count(9, 9), clock.seconds())",
        DEPLOY_BUDGET,
    );
    assert_eq!(
        report.output,
        vec!["5 20 17", "3 0 20"],
        "{:?}",
        report.outcome
    );
}

#[test]
fn stats_report_bottlenecks_and_steam() {
    let rt = ScriptRuntime::new();
    let mut world = WorldView::default();
    for (name, state) in [
        ("fine", pypeline::factory::stats::MachineState::Working),
        ("stuck", pypeline::factory::stats::MachineState::Blocked),
    ] {
        world.machines.insert(
            name.into(),
            MachineView {
                kind: MachineKind::Miner,
                working: true,
                input: 0,
                output: 0,
                powered: true,
                enabled: true,
                tier: 1,
                state,
                heat: 15,
                overheated: false,
            },
        );
    }
    rt.set_world(world);
    let report = rt.run(
        "import stats\nfrom auto import machines\nprint(stats.bottlenecks(), machines.status('stuck')['state'])\nprint(stats.steam() > 0, stats.steam_limit())",
        DEPLOY_BUDGET,
    );
    assert_eq!(
        report.output,
        vec!["['stuck'] blocked", &format!("True {DEPLOY_BUDGET}")],
        "{:?}",
        report.outcome
    );
}

#[test]
fn clock_and_temperature_read_the_snapshot() {
    use pypeline::factory::{Factory, daynight::DAY_TICKS};
    use pypeline::scripting::reconcile;
    let rt = ScriptRuntime::new();
    let report = rt.run(pypeline::scripting::CANONICAL_SAMPLE, DEPLOY_BUDGET);
    let mut factory = Factory::default();
    reconcile::apply(&mut factory, &report.plan.unwrap());
    factory.ticks = DAY_TICKS / 2;
    rt.set_world(WorldView::of(&factory, BTreeMap::new()));
    let report = rt.run(
        "import clock, sensors\nprint(clock.time_of_day(), clock.is_day(), sensors.temperature('steam_1'))\nsensors.temperature('miner_1')",
        DEPLOY_BUDGET,
    );
    assert_eq!(report.output, vec!["18 False 15"]);
    assert!(
        matches!(&report.outcome, RunOutcome::Error { message, .. } if message.contains("only steam generators")),
        "{:?}",
        report.outcome
    );
}
