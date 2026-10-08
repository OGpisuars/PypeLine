//! Same script + same seed = same result, always (roadmap Part 1 SIMULATION
//! MODEL, Part 4 C). CI runs this on Windows, Linux and macOS, so the golden
//! hash below also proves all three simulate identically.

use pypeline::factory::Factory;
use pypeline::scripting::CANONICAL_SAMPLE;
use pypeline::scripting::budget::DEPLOY_BUDGET;
use pypeline::scripting::reconcile;
use pypeline::scripting::runtime::ScriptRuntime;

/// State hash of the canonical sample after 600 ticks. If a deliberate
/// change to the simulation changes it, update it in the same commit and
/// say why in the commit message.
const GOLDEN_HASH: u64 = 0x9742_a2f1_82e7_bc98;

fn run_sample(ticks: u64) -> Factory {
    let report = ScriptRuntime::new().run(CANONICAL_SAMPLE, DEPLOY_BUDGET);
    let mut factory = Factory::default();
    reconcile::apply(&mut factory, &report.plan.expect("the sample runs"));
    for _ in 0..ticks {
        factory.step();
    }
    factory
}

#[test]
fn same_script_same_state() {
    assert_eq!(run_sample(600).state_hash(), run_sample(600).state_hash());
}

#[test]
fn matches_the_golden_hash_on_every_os() {
    let hash = run_sample(600).state_hash();
    assert_eq!(hash, GOLDEN_HASH, "state hash is {hash:#018x}");
}
