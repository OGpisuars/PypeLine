//! Simulation: everything that runs on the fixed tick.
//!
//! The factory is plain Rust data (no ECS) so it can be stepped and tested
//! without a window, and so iteration order is fixed: every collection is a
//! `BTreeMap` (roadmap Part 4 C). Rendering reads it and never writes to it.

pub mod conveyors;
pub mod items;
pub mod machines;
pub mod tick;
pub mod train;

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub use tick::{SimPlugin, SimSet, SimTick};

use conveyors::Conveyor;
use items::ItemKind;
use machines::{Machine, MachineKind};

/// Buildable area of the plot, in tiles.
pub const PLOT_WIDTH: i32 = 16;
pub const PLOT_HEIGHT: i32 = 10;

/// A tile on the plot. (0, 0) is the bottom-left buildable tile; x grows to
/// the east, y grows to the north.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn step(self, dir: Dir) -> Self {
        let (dx, dy) = dir.offset();
        Self::new(self.x + dx, self.y + dy)
    }

    pub fn in_plot(self) -> bool {
        (0..PLOT_WIDTH).contains(&self.x) && (0..PLOT_HEIGHT).contains(&self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Dir {
    North,
    East,
    South,
    West,
}

impl Dir {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    pub fn offset(self) -> (i32, i32) {
        match self {
            Self::North => (0, 1),
            Self::East => (1, 0),
            Self::South => (0, -1),
            Self::West => (-1, 0),
        }
    }

    /// The name used in Python: `dir="east"`.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            _ => None,
        }
    }
}

/// The whole factory on the plot.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
// Fields added later load as their defaults from older saves.
#[serde(default)]
pub struct Factory {
    pub conveyors: BTreeMap<Pos, Conveyor>,
    /// Machines by their stable name.
    pub machines: BTreeMap<String, Machine>,
    /// Which tile holds which machine (rebuilt whenever the layout changes).
    pub machine_at: BTreeMap<Pos, String>,
    /// Machine name -> the generator powering it.
    pub power: BTreeMap<String, String>,
    /// Items returned from removed belts and machines (the station).
    pub inventory: BTreeMap<ItemKind, u64>,
    /// Total items made since the game started.
    pub produced: BTreeMap<ItemKind, u64>,
    /// Money earned from the cargo train.
    pub coins: u64,
    /// The train's most recent visit.
    pub last_sale: Option<train::Sale>,
    /// Ticks the factory has actually run (it stops while halted).
    pub ticks: u64,
    /// Set when the last Run failed: belts stop until the next good Run.
    pub halted: bool,
    /// Bumped on every layout change so the renderer knows to rebuild.
    pub layout_version: u64,
}

impl Factory {
    pub fn is_powered(&self, machine: &str) -> bool {
        self.power
            .get(machine)
            .and_then(|generator| self.machines.get(generator))
            .is_some_and(|g| g.kind == MachineKind::SteamGenerator)
    }

    pub fn produced(&self, item: ItemKind) -> u64 {
        self.produced.get(&item).copied().unwrap_or(0)
    }

    pub fn rebuild_index(&mut self) {
        self.machine_at = self
            .machines
            .iter()
            .map(|(name, m)| (m.pos, name.clone()))
            .collect();
    }

    /// Advance the factory by one tick: machines work, machines hand items
    /// to whatever they face, then belts move. All in a fixed order.
    pub fn step(&mut self) {
        if self.halted {
            return;
        }
        self.ticks += 1;

        let names: Vec<String> = self.machines.keys().cloned().collect();
        for name in &names {
            let powered = self.is_powered(name);
            let machine = self.machines.get_mut(name).expect("name came from the map");
            let needs_power = machine.kind.needs_power();
            if let Some(made) = machine.work(powered || !needs_power) {
                *self.produced.entry(made).or_default() += 1;
            }
        }
        for name in &names {
            self.push_machine_output(name);
        }

        let belts: Vec<Pos> = self.conveyors.keys().copied().collect();
        for pos in belts {
            self.move_belt(pos);
        }

        if train::arrives_at(self.ticks) {
            self.sell_to_train();
        }
    }

    /// The cargo train buys everything in stations and the station inventory.
    fn sell_to_train(&mut self) {
        let mut sale = train::Sale {
            tick: self.ticks,
            ..Default::default()
        };
        let mut goods: Vec<ItemKind> = Vec::new();
        for machine in self.machines.values_mut() {
            if machine.kind == MachineKind::Station {
                goods.append(&mut machine.input);
            }
        }
        for (item, count) in std::mem::take(&mut self.inventory) {
            goods.extend(std::iter::repeat_n(item, count as usize));
        }
        for item in goods {
            *sale.items.entry(item).or_default() += 1;
            sale.coins += train::price(item);
        }
        self.coins += sale.coins;
        self.last_sale = Some(sale);
    }

    fn push_machine_output(&mut self, name: &str) {
        let machine = &self.machines[name];
        if !machine.kind.has_output() {
            return;
        }
        let Some(&item) = machine.output.first() else {
            return;
        };
        let target = machine.pos.step(machine.dir);
        if self.give(target, item, machine.pos) {
            self.machines
                .get_mut(name)
                .expect("name came from the map")
                .output
                .remove(0);
        }
    }

    fn move_belt(&mut self, pos: Pos) {
        let belt = self.conveyors.get_mut(&pos).expect("pos came from the map");
        if !belt.advance() {
            return;
        }
        let item = belt.items[0].kind;
        let target = pos.step(belt.dir);
        if self.give(target, item, pos) {
            self.conveyors
                .get_mut(&pos)
                .expect("pos came from the map")
                .items
                .remove(0);
        }
    }

    /// Try to hand `item` to whatever is on tile `target`. `from` is the tile
    /// it comes from, so a belt cannot feed itself.
    fn give(&mut self, target: Pos, item: ItemKind, from: Pos) -> bool {
        if target == from {
            return false;
        }
        if let Some(belt) = self.conveyors.get_mut(&target) {
            if belt.can_accept() {
                belt.push_back(item);
                return true;
            }
            return false;
        }
        if let Some(name) = self.machine_at.get(&target)
            && let Some(machine) = self.machines.get_mut(name)
            && machine.accepts(item)
        {
            machine.input.push(item);
            return true;
        }
        false
    }

    /// A fingerprint of the whole factory state. Equal states give equal
    /// hashes on every OS: the state is all integers in BTreeMaps, its
    /// `Debug` text is fully determined by it, and FNV-1a is a fixed,
    /// platform-independent hash (unlike std's randomly seeded hasher).
    pub fn state_hash(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        format!("{self:?}").bytes().fold(OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(PRIME)
        })
    }

    /// Clear the plot for a Clean Run. Coins, totals and the clock are kept:
    /// they are the player's progress, not the layout.
    pub fn clean_reset(&mut self) {
        *self = Self {
            coins: self.coins,
            produced: std::mem::take(&mut self.produced),
            last_sale: self.last_sale.take(),
            ticks: self.ticks,
            layout_version: self.layout_version + 1,
            ..Self::default()
        };
    }

    /// Move a removed belt's or machine's items into the station inventory.
    pub fn stash(&mut self, items: impl IntoIterator<Item = ItemKind>) {
        for item in items {
            *self.inventory.entry(item).or_default() += 1;
        }
    }
}

/// A successful Run's build plan, waiting to be applied on the next tick.
#[derive(Resource, Default)]
pub struct PendingBuild(pub Option<crate::scripting::commands::BuildPlan>);

pub struct FactoryPlugin;

impl Plugin for FactoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Factory>()
            .init_resource::<PendingBuild>()
            .add_systems(
                FixedUpdate,
                (apply_pending_build, step_factory)
                    .chain()
                    .in_set(SimSet::Factory),
            );
    }
}

fn apply_pending_build(
    mut pending: ResMut<PendingBuild>,
    mut factory: ResMut<Factory>,
    mut console: ResMut<crate::scripting::Console>,
) {
    let Some(plan) = pending.0.take() else {
        return;
    };
    let report = crate::scripting::reconcile::apply(&mut factory, &plan);
    console.push(crate::scripting::ConsoleKind::Info, report.summary());
}

fn step_factory(mut factory: ResMut<Factory>) {
    factory.step();
}
