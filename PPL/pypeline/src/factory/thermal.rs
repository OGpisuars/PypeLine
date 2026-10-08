//! Boiler heat (roadmap: DAY / NIGHT + THERMAL). Integer math only, updated
//! once per game second inside the factory step, so it is deterministic.
//!
//! Every steam generator heads toward a temperature set by the air (hotter
//! by day) plus a share for each switched-on machine it powers. At
//! `OVERHEAT` it stops powering anything until it has cooled to `RESTART`.
//! Only big generators (12+ machines) get there, and only by day, so
//! scripts can plan around it with clock.time_of_day() and
//! sensors.temperature().

use super::machines::MachineKind;
use super::{Factory, daynight};

/// Degrees each switched-on machine adds to its generator.
pub const HEAT_PER_MACHINE: u32 = 6;
/// The generator overheats at this temperature...
pub const OVERHEAT: u32 = 100;
/// ...and starts again once it has cooled to this.
pub const RESTART: u32 = 70;
/// Each second a generator closes this fraction (1/N) of the gap to the
/// temperature it is heading for.
const SETTLE: u32 = 8;

/// The temperature a generator is heading for right now.
pub fn target_temperature(factory: &Factory, generator: &str) -> u32 {
    let load = if factory
        .machines
        .get(generator)
        .is_some_and(|g| g.overheated)
    {
        0
    } else {
        factory
            .power
            .iter()
            .filter(|(machine, g)| {
                *g == generator && factory.machines.get(*machine).is_some_and(|m| m.enabled)
            })
            .count() as u32
    };
    daynight::air_temperature(factory.ticks) + load * HEAT_PER_MACHINE
}

/// One second of heating and cooling for every generator.
pub fn update(factory: &mut Factory) {
    let generators: Vec<String> = factory
        .machines
        .iter()
        .filter(|(_, m)| m.kind == MachineKind::SteamGenerator)
        .map(|(name, _)| name.clone())
        .collect();
    for name in generators {
        let target = target_temperature(factory, &name);
        let g = factory
            .machines
            .get_mut(&name)
            .expect("name came from the map");
        let gap = target.abs_diff(g.heat);
        let step = (gap / SETTLE).max(gap.min(1));
        if target > g.heat {
            g.heat += step;
        } else {
            g.heat -= step;
        }
        if !g.overheated && g.heat >= OVERHEAT {
            g.overheated = true;
            factory.layout_version += 1;
        } else if g.overheated && g.heat <= RESTART {
            g.overheated = false;
            factory.layout_version += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::budget::DEPLOY_BUDGET;
    use crate::scripting::reconcile;
    use crate::scripting::runtime::ScriptRuntime;

    /// A generator powering `machines` miners, run for `seconds`.
    fn generator_with(machines: usize, seconds: u64) -> Factory {
        let script = format!(
            "from auto import machines\nimport power\n\
             machines.place('steam_generator', name='g', x=15, y=9)\n\
             names = []\n\
             for i in range({machines}):\n    \
                 machines.place('miner', name=f'm{{i}}', x=i, y=0, ore='iron')\n    \
                 names.append(f'm{{i}}')\n\
             power.connect(generator='g', to=names)\n"
        );
        let report = ScriptRuntime::new().run(&script, DEPLOY_BUDGET);
        let mut factory = Factory::default();
        reconcile::apply(&mut factory, &report.plan.unwrap());
        for _ in 0..seconds * 20 {
            factory.step();
        }
        factory
    }

    #[test]
    fn small_generators_stay_cool() {
        // Eleven machines peak at 96 degrees at noon.
        let factory = generator_with(11, 120);
        assert!(!factory.machines["g"].overheated);
        assert!(factory.machines["g"].heat < OVERHEAT);
    }

    #[test]
    fn big_generators_overheat_by_day_and_recover() {
        let mut factory = generator_with(14, 0);
        let overheated = (0..60 * 20).any(|_| {
            factory.step();
            factory.machines["g"].overheated
        });
        assert!(overheated, "heat {}", factory.machines["g"].heat);
        assert!(
            !factory.is_powered("m0"),
            "an overheated generator powers nothing"
        );
        // It cools down and starts again by itself.
        let recovered = (0..60 * 20).any(|_| {
            factory.step();
            !factory.machines["g"].overheated
        });
        assert!(recovered);
        assert!(factory.machines["g"].heat <= RESTART + 8);
    }
}
