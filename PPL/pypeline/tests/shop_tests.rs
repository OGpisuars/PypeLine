//! The Shop: bought upgrades unlock faster tiers in the build API.

use pypeline::factory::Factory;
use pypeline::factory::items::ItemKind;
use pypeline::factory::shop::Upgrade;
use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::operate::WorldView;
use pypeline::scripting::reconcile;
use pypeline::scripting::runtime::{RunOutcome, ScriptRuntime};

/// The canonical sample with every part at `tier`, and a station so the
/// smelter never fills up.
fn sample(tier: u8) -> String {
    pypeline::scripting::CANONICAL_SAMPLE
        .replace("ore=\"iron\")", &format!("ore=\"iron\", tier={tier})"))
        .replace("dir=\"east\")", &format!("dir=\"east\", tier={tier})"))
        .replace(
            "x=5, y=0)",
            &format!(
                "x=5, y=0, tier={tier})\nconveyors.place(x=6, y=0, dir=\"east\")\n\
                 machines.place(\"station\", name=\"station_1\", x=7, y=0)"
            ),
        )
}

fn build(factory: &mut Factory, script: &str) -> Result<(), String> {
    let runtime = ScriptRuntime::new();
    runtime.set_world(WorldView::of(factory, Default::default()));
    let report = runtime.run(script, DEPLOY_BUDGET);
    match report.outcome {
        RunOutcome::Finished => {
            reconcile::apply(factory, &report.plan.unwrap());
            Ok(())
        }
        other => Err(format!("{other:?}")),
    }
}

#[test]
fn locked_tiers_fail_on_their_line() {
    let mut factory = Factory::default();
    let err = build(&mut factory, &sample(2)).unwrap_err();
    assert!(err.contains("Shop"), "{err}");
    assert!(factory.machines.is_empty(), "a failed run builds nothing");
}

#[test]
fn bought_tiers_make_more() {
    let plates_after_a_minute = |tier: u8| {
        let mut factory = Factory {
            coins: 10_000,
            ..Default::default()
        };
        for upgrade in Upgrade::ALL {
            factory.buy(upgrade).unwrap();
        }
        build(&mut factory, &sample(tier)).unwrap();
        for _ in 0..1200 {
            factory.step();
        }
        factory.produced(ItemKind::IronPlate)
    };
    let (slow, fast) = (plates_after_a_minute(1), plates_after_a_minute(3));
    assert!(fast >= slow * 2, "tier 1 made {slow}, tier 3 made {fast}");
}

#[test]
fn upgrading_keeps_items_and_upgrades_survive_a_clean_run() {
    let mut factory = Factory {
        coins: 500,
        ..Default::default()
    };
    factory.buy(Upgrade::FastBelt).unwrap();
    build(&mut factory, pypeline::scripting::CANONICAL_SAMPLE).unwrap();
    for _ in 0..300 {
        factory.step();
    }
    let on_belts = |f: &Factory| f.conveyors.values().map(|b| b.items.len()).sum::<usize>();
    let before = on_belts(&factory);
    assert!(before > 0);
    let fast_belts =
        pypeline::scripting::CANONICAL_SAMPLE.replace("dir=\"east\")", "dir=\"east\", tier=2)");
    build(&mut factory, &fast_belts).unwrap();
    assert_eq!(on_belts(&factory), before);
    assert!(factory.conveyors.values().all(|b| b.tier == 2));

    factory.clean_reset();
    assert!(factory.unlocked.contains(&Upgrade::FastBelt));
    assert_eq!(factory.coins, 350);
}
