//! Factory analytics (roadmap: FACTORY ANALYTICS): what each machine is
//! doing, and a minute of history for the Stats window.
//!
//! None of this is simulation state. The history is sampled after each
//! factory step and is never saved, so it cannot change the state hash.

use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;

use super::Factory;
use super::machines::{BUFFER_CAP, MachineKind, STATION_CAP};

/// What a machine is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MachineState {
    Working,
    /// Ready to work but nothing to work on (a smelter with no ore).
    Starved,
    /// Its output (or a station's store) is full: whatever is after it is
    /// too slow or missing. This is a bottleneck.
    Blocked,
    NoPower,
    /// Switched off by the script.
    Off,
    /// A generator that powers nothing.
    Idle,
}

impl MachineState {
    /// The name used in Python and in the Stats window.
    pub fn name(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Starved => "starved",
            Self::Blocked => "blocked",
            Self::NoPower => "no power",
            Self::Off => "off",
            Self::Idle => "idle",
        }
    }

    pub fn advice(self) -> &'static str {
        match self {
            Self::Working => "busy, all good",
            Self::Starved => "waiting for input: feed it more",
            Self::Blocked => "output is full: nothing takes its items",
            Self::NoPower => "connect it to a steam generator",
            Self::Off => "switched off by your script",
            Self::Idle => "powers nothing yet",
        }
    }
}

/// The state of a machine from what can be seen of it.
pub fn state_of(
    kind: MachineKind,
    powered: bool,
    enabled: bool,
    input: usize,
    output: usize,
    powering: usize,
) -> MachineState {
    match kind {
        MachineKind::SteamGenerator if powering == 0 => MachineState::Idle,
        MachineKind::SteamGenerator => MachineState::Working,
        MachineKind::Station if input >= STATION_CAP => MachineState::Blocked,
        MachineKind::Station => MachineState::Working,
        MachineKind::Miner | MachineKind::Smelter => {
            if !enabled {
                MachineState::Off
            } else if !powered {
                MachineState::NoPower
            } else if output >= BUFFER_CAP {
                MachineState::Blocked
            } else if kind == MachineKind::Smelter && input == 0 {
                MachineState::Starved
            } else {
                MachineState::Working
            }
        }
    }
}

/// The state of the machine called `name`.
pub fn machine_state(factory: &Factory, name: &str) -> Option<MachineState> {
    let m = factory.machines.get(name)?;
    let powered = !m.kind.needs_power() || factory.is_powered(name);
    let powering = factory.power.values().filter(|g| *g == name).count();
    Some(state_of(
        m.kind,
        powered,
        m.enabled,
        m.input.len(),
        m.output.len(),
        powering,
    ))
}

/// One second of history.
#[derive(Debug, Default, Clone)]
struct Second {
    ticks: u32,
    /// Ticks the factory was running (not halted).
    running: u32,
    /// Ticks each machine spent working.
    busy: BTreeMap<String, u32>,
    /// Coins the train paid (spending in the Shop does not count).
    earned: u64,
    /// Steam tick() used, and how many times it ran.
    tick_steam: u64,
    tick_calls: u32,
}

/// The last minute of factory history, for the Stats window.
#[derive(Resource, Debug, Default)]
pub struct StatsHistory {
    seconds: VecDeque<Second>,
    current: Second,
    last_coins: Option<u64>,
}

impl StatsHistory {
    const SECONDS: usize = 60;
    const TICKS_PER_SECOND: u32 = 20;

    /// Record one game tick, after the factory stepped.
    pub fn record(&mut self, factory: &Factory) {
        let now = &mut self.current;
        now.ticks += 1;
        if !factory.halted {
            now.running += 1;
            for name in factory.machines.keys() {
                if machine_state(factory, name) == Some(MachineState::Working) {
                    *now.busy.entry(name.clone()).or_default() += 1;
                }
            }
        }
        if let Some(before) = self.last_coins {
            now.earned += factory.coins.saturating_sub(before);
        }
        self.last_coins = Some(factory.coins);
        if now.ticks == Self::TICKS_PER_SECOND {
            self.seconds.push_back(std::mem::take(now));
            if self.seconds.len() > Self::SECONDS {
                self.seconds.pop_front();
            }
        }
    }

    /// Record what one tick() call cost.
    pub fn record_tick_steam(&mut self, steps: u64) {
        self.current.tick_steam += steps;
        self.current.tick_calls += 1;
    }

    fn window(&self) -> impl Iterator<Item = &Second> {
        self.seconds.iter().chain(std::iter::once(&self.current))
    }

    fn ticks(&self) -> u32 {
        self.window().map(|s| s.ticks).sum()
    }

    /// Percent of the last minute the factory was running.
    pub fn uptime(&self) -> Option<u32> {
        let ticks = self.ticks();
        (ticks > 0).then(|| self.window().map(|s| s.running).sum::<u32>() * 100 / ticks)
    }

    /// Percent of the last minute `machine` spent working.
    pub fn busy(&self, machine: &str) -> Option<u32> {
        let ticks = self.ticks();
        (ticks > 0).then(|| {
            let busy: u32 = self
                .window()
                .map(|s| s.busy.get(machine).copied().unwrap_or(0))
                .sum();
            busy * 100 / ticks
        })
    }

    /// Coins the train paid over the last minute.
    pub fn earned_last_minute(&self) -> u64 {
        self.window().map(|s| s.earned).sum()
    }

    /// Average steam per tick() call over the last minute, if it ran.
    pub fn tick_steam(&self) -> Option<u64> {
        let calls: u32 = self.window().map(|s| s.tick_calls).sum();
        (calls > 0).then(|| self.window().map(|s| s.tick_steam).sum::<u64>() / u64::from(calls))
    }

    /// How many seconds the numbers cover (up to a minute).
    pub fn seconds_covered(&self) -> u32 {
        self.ticks() / Self::TICKS_PER_SECOND
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_explain_what_is_wrong() {
        use MachineKind::*;
        assert_eq!(
            state_of(Smelter, true, true, 0, 0, 0),
            MachineState::Starved
        );
        assert_eq!(
            state_of(Smelter, true, true, 3, 0, 0),
            MachineState::Working
        );
        assert_eq!(
            state_of(Miner, true, true, 0, BUFFER_CAP, 0),
            MachineState::Blocked
        );
        assert_eq!(state_of(Miner, false, true, 0, 0, 0), MachineState::NoPower);
        assert_eq!(state_of(Miner, false, false, 0, 0, 0), MachineState::Off);
        assert_eq!(
            state_of(SteamGenerator, true, true, 0, 0, 0),
            MachineState::Idle
        );
        assert_eq!(
            state_of(Station, true, true, STATION_CAP, 0, 0),
            MachineState::Blocked
        );
    }

    #[test]
    fn history_finds_the_idle_miner() {
        // A miner with no belt in front fills up and stops: it is busy at
        // first, then blocked.
        let runtime = crate::scripting::runtime::ScriptRuntime::new();
        let report = runtime.run(
            "from auto import machines\nimport power\n\
             machines.place('steam_generator', name='g', x=0, y=2)\n\
             machines.place('miner', name='m', x=0, y=0, ore='iron')\n\
             power.connect(generator='g', to=['m'])",
            crate::scripting::budget::DEPLOY_BUDGET,
        );
        let mut factory = Factory::default();
        crate::scripting::reconcile::apply(&mut factory, &report.plan.unwrap());
        let mut history = StatsHistory::default();
        for _ in 0..1200 {
            factory.step();
            history.record(&factory);
        }
        assert_eq!(machine_state(&factory, "m"), Some(MachineState::Blocked));
        let busy = history.busy("m").unwrap();
        assert!((20..=50).contains(&busy), "busy {busy}%");
        assert_eq!(history.uptime(), Some(100));
        assert_eq!(history.seconds_covered(), 60);
    }
}
