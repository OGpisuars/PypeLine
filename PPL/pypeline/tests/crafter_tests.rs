//! Crafters: recipes unlocked by achievements, chosen in the script or in
//! the game, and turning their inputs into gears, pipes and engines.

use std::collections::BTreeSet;

use pypeline::factory::items::ItemKind;
use pypeline::factory::recipes::Recipe;
use pypeline::factory::{Factory, Pos};
use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::operate::WorldView;
use pypeline::scripting::reconcile;
use pypeline::scripting::runtime::{RunOutcome, ScriptRuntime};

/// A belt feeding a powered crafter, which sends what it makes along a belt
/// into a station. `recipe` is the crafter's recipe= (or nothing).
fn script(recipe: &str) -> String {
    format!(
        "from auto import conveyors, machines\n\
         import power\n\
         machines.place(\"steam_generator\", name=\"steam\", x=0, y=9)\n\
         conveyors.place(x=1, y=5, dir=\"east\")\n\
         machines.place(\"crafter\", name=\"crafter\", x=2, y=5, dir=\"east\"{recipe})\n\
         conveyors.place(x=3, y=5, dir=\"east\")\n\
         machines.place(\"station\", name=\"station\", x=4, y=5)\n\
         power.connect(generator=\"steam\", to=[\"crafter\"])\n"
    )
}

fn build(factory: &mut Factory, source: &str, recipes: &[Recipe]) -> RunOutcome {
    let runtime = ScriptRuntime::new();
    runtime.set_world(WorldView {
        recipes: recipes.iter().copied().collect::<BTreeSet<_>>(),
        ..WorldView::of(factory, Default::default())
    });
    let report = runtime.run(source, DEPLOY_BUDGET);
    if let Some(plan) = &report.plan {
        reconcile::apply(factory, plan);
    }
    report.outcome
}

#[test]
fn locked_recipes_fail_on_their_line() {
    let mut factory = Factory::default();
    let outcome = build(&mut factory, &script(", recipe=\"iron_gear\""), &[]);
    assert!(
        matches!(&outcome, RunOutcome::Error { line: Some(5), message } if message.contains("locked")),
        "{outcome:?}"
    );
}

#[test]
fn crafters_make_gears_from_plates() {
    let mut factory = Factory::default();
    let outcome = build(
        &mut factory,
        &script(", recipe=\"iron_gear\""),
        &[Recipe::IronGear],
    );
    assert_eq!(outcome, RunOutcome::Finished);
    // Keep the feeding belt full of plates (less than one train visit).
    for _ in 0..550 {
        let belt = factory.conveyors.get_mut(&Pos::new(1, 5)).unwrap();
        if belt.can_accept() {
            belt.push_back(ItemKind::IronPlate);
        }
        factory.step();
    }
    assert!(
        factory.produced(ItemKind::IronGear) >= 3,
        "{:?}",
        factory.produced
    );
    let station = &factory.machines["station"];
    assert!(station.input.contains(&ItemKind::IronGear), "{station:?}");
    assert!(station.input.iter().all(|&item| item == ItemKind::IronGear));
}

#[test]
fn a_recipe_picked_in_the_game_survives_a_run() {
    let mut factory = Factory::default();
    assert_eq!(build(&mut factory, &script(""), &[]), RunOutcome::Finished);
    assert_eq!(factory.machines["crafter"].recipe, None);

    // The player clicks the crafter and picks pipes.
    factory
        .machines
        .get_mut("crafter")
        .unwrap()
        .set_recipe(Some(Recipe::IronPipe));
    assert_eq!(build(&mut factory, &script(""), &[]), RunOutcome::Finished);
    assert_eq!(factory.machines["crafter"].recipe, Some(Recipe::IronPipe));

    // A recipe= in the script wins.
    let outcome = build(
        &mut factory,
        &script(", recipe=\"iron_gear\""),
        &[Recipe::IronGear],
    );
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(factory.machines["crafter"].recipe, Some(Recipe::IronGear));
}
