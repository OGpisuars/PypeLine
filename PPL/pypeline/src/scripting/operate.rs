//! Operating the factory from `tick()` and event handlers.
//!
//! Scripts never touch the factory directly. Each tick they read a snapshot
//! (`WorldView`) and queue commands (`Op`), which the factory applies in a
//! fixed order on the next step. Build calls (place/connect) are only allowed
//! while main.py runs (roadmap: build once, operate every tick).

use std::collections::{BTreeMap, BTreeSet};

use crate::factory::items::ItemKind;
use crate::factory::machines::MachineKind;
use crate::factory::shop::Upgrade;
use crate::factory::stats::{self, MachineState};
use crate::factory::{Factory, Pos};

/// Which calls a script may make right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiMode {
    /// main.py: place and connect things.
    #[default]
    Build,
    /// tick() and event handlers: read sensors and switch machines.
    Operate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineView {
    pub kind: MachineKind,
    pub working: bool,
    pub input: usize,
    pub output: usize,
    pub powered: bool,
    pub enabled: bool,
    pub tier: u8,
    pub state: MachineState,
}

/// What scripts can see of the factory during one tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldView {
    pub ticks: u64,
    pub coins: u64,
    pub produced: BTreeMap<ItemKind, u64>,
    /// Items made over the last minute of game time.
    pub per_minute: BTreeMap<ItemKind, u64>,
    pub machines: BTreeMap<String, MachineView>,
    /// Items on each belt tile.
    pub belts: BTreeMap<Pos, usize>,
    /// Shop upgrades bought, which decide the `tier=` main.py may use.
    pub unlocked: BTreeSet<Upgrade>,
}

impl WorldView {
    pub fn of(factory: &Factory, per_minute: BTreeMap<ItemKind, u64>) -> Self {
        Self {
            ticks: factory.ticks,
            coins: factory.coins,
            produced: factory.produced.clone(),
            per_minute,
            machines: factory
                .machines
                .iter()
                .map(|(name, m)| {
                    let powered = !m.kind.needs_power() || factory.is_powered(name);
                    let view = MachineView {
                        kind: m.kind,
                        working: m.progress > 0,
                        input: m.input.len(),
                        output: m.output.len(),
                        powered,
                        enabled: m.enabled,
                        tier: m.tier,
                        state: stats::machine_state(factory, name).expect("name came from the map"),
                    };
                    (name.clone(), view)
                })
                .collect(),
            belts: factory
                .conveyors
                .iter()
                .map(|(&pos, belt)| (pos, belt.items.len()))
                .collect(),
            unlocked: factory.unlocked.clone(),
        }
    }
}

/// A change a script asked for, applied on the next factory step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    SetEnabled { machine: String, on: bool },
}

/// Apply queued operations in order. Ops for machines that no longer exist
/// are skipped.
pub fn apply_ops(factory: &mut Factory, ops: &[Op]) {
    for op in ops {
        match op {
            Op::SetEnabled { machine, on } => {
                if let Some(m) = factory.machines.get_mut(machine) {
                    m.enabled = *on;
                }
            }
        }
    }
}
