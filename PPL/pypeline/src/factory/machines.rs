//! Machines: miners dig ore, smelters turn ore into plates, crafters make
//! gears, pipes and engines, steam generators power the machines connected
//! to them, and stations hold items for the train.

use super::items::{self, ItemKind};
use super::recipes::Recipe;
use super::{Dir, Pos};

/// Ticks a tier 1 miner needs per ore (2 seconds).
pub const MINE_TICKS: u32 = 40;
/// Ticks a tier 1 smelter needs per plate (3 seconds).
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
    /// Makes things from up to three inputs, by a recipe (`recipes.rs`).
    Crafter,
}

impl MachineKind {
    /// The name used in Python: `machines.place("miner", ...)`.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "miner" => Some(Self::Miner),
            "smelter" => Some(Self::Smelter),
            "steam_generator" => Some(Self::SteamGenerator),
            "station" => Some(Self::Station),
            "crafter" => Some(Self::Crafter),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Miner => "miner",
            Self::Smelter => "smelter",
            Self::SteamGenerator => "steam_generator",
            Self::Station => "station",
            Self::Crafter => "crafter",
        }
    }

    pub const ALL: [Self; 5] = [
        Self::Miner,
        Self::Smelter,
        Self::Crafter,
        Self::SteamGenerator,
        Self::Station,
    ];

    /// Does this machine need steam power to work?
    pub fn needs_power(self) -> bool {
        matches!(self, Self::Miner | Self::Smelter | Self::Crafter)
    }

    /// Does this machine send items out of its `dir` side?
    pub fn has_output(self) -> bool {
        matches!(self, Self::Miner | Self::Smelter | Self::Crafter)
    }
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Switched off by a script (machines.disable). Off machines do no work.
    #[serde(default = "switched_on")]
    pub enabled: bool,
    /// 1, or 2-3 for the faster Mk2/Mk3 bought in the Shop.
    #[serde(default = "super::conveyors::first_tier")]
    pub tier: u8,
    /// Steam generators only: temperature in degrees (see thermal.rs).
    #[serde(default = "room_temperature")]
    pub heat: u32,
    /// Steam generators only: too hot to power anything until it cools.
    #[serde(default)]
    pub overheated: bool,
    /// Crafters only: what it makes. None until one is picked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<Recipe>,
}

// Written by hand so machines without a recipe print exactly as they did
// before crafters existed: the factory's state hash is taken from this
// text, and the determinism tests compare it with a fixed value.
impl std::fmt::Debug for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("Machine");
        out.field("kind", &self.kind)
            .field("pos", &self.pos)
            .field("dir", &self.dir)
            .field("ore", &self.ore)
            .field("input", &self.input)
            .field("output", &self.output)
            .field("progress", &self.progress)
            .field("enabled", &self.enabled)
            .field("tier", &self.tier)
            .field("heat", &self.heat)
            .field("overheated", &self.overheated);
        if let Some(recipe) = &self.recipe {
            out.field("recipe", recipe);
        }
        out.finish()
    }
}

fn room_temperature() -> u32 {
    15
}

fn switched_on() -> bool {
    true
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
            enabled: true,
            tier: 1,
            heat: room_temperature(),
            overheated: false,
            recipe: None,
        }
    }

    /// Switch a crafter to another recipe. What it was holding for the old
    /// one is returned so it can go to the station inventory.
    pub fn set_recipe(&mut self, recipe: Option<Recipe>) -> Vec<ItemKind> {
        if self.recipe == recipe {
            return Vec::new();
        }
        self.recipe = recipe;
        self.progress = 0;
        std::mem::take(&mut self.input)
    }

    /// Does a crafter hold everything its recipe needs for one craft?
    fn has_ingredients(&self, recipe: Recipe) -> bool {
        recipe
            .inputs()
            .iter()
            .all(|&(item, count)| self.input.iter().filter(|&&i| i == item).count() >= count)
    }

    /// Ticks one job takes (one ore mined, one plate smelted).
    pub fn work_ticks(&self) -> u32 {
        let base = match self.kind {
            MachineKind::Miner => MINE_TICKS,
            MachineKind::Smelter => SMELT_TICKS,
            MachineKind::Crafter => return self.recipe.map_or(0, Recipe::ticks),
            MachineKind::SteamGenerator | MachineKind::Station => return 0,
        };
        base / super::shop::speedup(self.tier)
    }

    /// Can this machine take `item` into its input right now?
    pub fn accepts(&self, item: ItemKind) -> bool {
        match self.kind {
            MachineKind::Smelter => items::smelt(item).is_some() && self.input.len() < BUFFER_CAP,
            MachineKind::Station => self.input.len() < STATION_CAP,
            // Up to two crafts' worth of each ingredient, so one input line
            // cannot fill it up while another ingredient is missing.
            MachineKind::Crafter => self.recipe.is_some_and(|recipe| {
                let held = self.input.iter().filter(|&&i| i == item).count();
                held < 2 * recipe.needs(item)
            }),
            MachineKind::Miner | MachineKind::SteamGenerator => false,
        }
    }

    /// Run one tick of work. Returns the item made this tick, if any.
    pub fn work(&mut self, powered: bool) -> Option<ItemKind> {
        if !powered || !self.enabled || self.output.len() >= BUFFER_CAP {
            return None;
        }
        let ticks = self.work_ticks();
        let made = match self.kind {
            MachineKind::Miner => self.ore?,
            MachineKind::Smelter => items::smelt(*self.input.first()?)?,
            MachineKind::Crafter => {
                let recipe = self.recipe?;
                if !self.has_ingredients(recipe) {
                    return None;
                }
                recipe.output()
            }
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
        if let (MachineKind::Crafter, Some(recipe)) = (self.kind, self.recipe) {
            for &(item, count) in recipe.inputs() {
                for _ in 0..count {
                    let at = self
                        .input
                        .iter()
                        .position(|&i| i == item)
                        .expect("checked by has_ingredients");
                    self.input.remove(at);
                }
            }
        }
        self.output.push(made);
        Some(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crafters_follow_their_recipe() {
        let mut crafter = Machine::new(MachineKind::Crafter, Pos::new(0, 0), Dir::East, None);
        assert!(!crafter.accepts(ItemKind::IronPlate), "no recipe yet");
        crafter.set_recipe(Some(Recipe::IronGear));
        assert!(crafter.accepts(ItemKind::IronPlate));
        assert!(!crafter.accepts(ItemKind::IronOre));

        // One plate is not enough for a gear.
        crafter.input.push(ItemKind::IronPlate);
        assert!((0..200).all(|_| crafter.work(true).is_none()));
        crafter.input.push(ItemKind::IronPlate);
        let made: Vec<ItemKind> = (0..Recipe::IronGear.ticks())
            .filter_map(|_| crafter.work(true))
            .collect();
        assert_eq!(made, [ItemKind::IronGear]);
        assert!(crafter.input.is_empty());

        // It holds at most two crafts' worth of an ingredient.
        crafter.input.extend([ItemKind::IronPlate; 4]);
        assert!(!crafter.accepts(ItemKind::IronPlate));
        // Switching recipes hands back what it was holding.
        assert_eq!(crafter.set_recipe(Some(Recipe::IronPipe)).len(), 4);
    }

    #[test]
    fn machines_without_a_recipe_print_as_before() {
        let miner = Machine::new(MachineKind::Miner, Pos::new(0, 0), Dir::East, None);
        let text = format!("{miner:?}");
        assert!(!text.contains("recipe"), "{text}");
        assert!(text.ends_with("heat: 15, overheated: false }"), "{text}");
    }
}
