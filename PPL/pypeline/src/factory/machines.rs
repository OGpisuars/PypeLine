//! Machines: miners dig ore, smelters turn ore into plates, steam generators
//! power the machines connected to them.

use super::items::{self, ItemKind};
use super::{Dir, Pos};

/// Ticks a miner needs per ore (2 seconds).
pub const MINE_TICKS: u32 = 40;
/// Ticks a smelter needs per plate (3 seconds).
pub const SMELT_TICKS: u32 = 60;
/// Items a machine can hold in its input and in its output.
pub const BUFFER_CAP: usize = 10;
/// Items a station can hold for the train.
pub const STATION_CAP: usize = 50;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum MachineKind {
    Miner,
    Smelter,
    SteamGenerator,
    /// Holds items for the cargo train to buy.
    Station,
}

impl MachineKind {
    /// The name used in Python: `machines.place("miner", ...)`.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "miner" => Some(Self::Miner),
            "smelter" => Some(Self::Smelter),
            "steam_generator" => Some(Self::SteamGenerator),
            "station" => Some(Self::Station),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Miner => "miner",
            Self::Smelter => "smelter",
            Self::SteamGenerator => "steam_generator",
            Self::Station => "station",
        }
    }

    pub const ALL: [Self; 4] = [
        Self::Miner,
        Self::Smelter,
        Self::SteamGenerator,
        Self::Station,
    ];

    /// Does this machine need steam power to work?
    pub fn needs_power(self) -> bool {
        matches!(self, Self::Miner | Self::Smelter)
    }

    /// Does this machine send items out of its `dir` side?
    pub fn has_output(self) -> bool {
        matches!(self, Self::Miner | Self::Smelter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Machine {
    pub kind: MachineKind,
    pub pos: Pos,
    /// The side items come out of.
    pub dir: Dir,
    /// What a miner digs.
    pub ore: Option<ItemKind>,
    pub input: Vec<ItemKind>,
    pub output: Vec<ItemKind>,
    /// Ticks spent on the current job.
    pub progress: u32,
}

impl Machine {
    pub fn new(kind: MachineKind, pos: Pos, dir: Dir, ore: Option<ItemKind>) -> Self {
        Self {
            kind,
            pos,
            dir,
            ore,
            input: Vec::new(),
            output: Vec::new(),
            progress: 0,
        }
    }

    /// Can this machine take `item` into its input right now?
    pub fn accepts(&self, item: ItemKind) -> bool {
        match self.kind {
            MachineKind::Smelter => items::smelt(item).is_some() && self.input.len() < BUFFER_CAP,
            MachineKind::Station => self.input.len() < STATION_CAP,
            MachineKind::Miner | MachineKind::SteamGenerator => false,
        }
    }

    /// Run one tick of work. Returns the item made this tick, if any.
    pub fn work(&mut self, powered: bool) -> Option<ItemKind> {
        if !powered || self.output.len() >= BUFFER_CAP {
            return None;
        }
        let (ticks, made) = match self.kind {
            MachineKind::Miner => (MINE_TICKS, self.ore?),
            MachineKind::Smelter => (SMELT_TICKS, items::smelt(*self.input.first()?)?),
            MachineKind::SteamGenerator | MachineKind::Station => return None,
        };
        self.progress += 1;
        if self.progress < ticks {
            return None;
        }
        self.progress = 0;
        if self.kind == MachineKind::Smelter {
            self.input.remove(0);
        }
        self.output.push(made);
        Some(made)
    }
}
