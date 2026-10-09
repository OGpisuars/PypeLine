//! Crafter recipes: what a crafter turns its inputs into.
//!
//! A crafter takes items from any of its three input sides and sends what it
//! makes out of its `dir` side. Which recipes a player may use is decided by
//! achievements (`progression/achievements.rs`); this file only knows what
//! each recipe needs and makes.

use serde::{Deserialize, Serialize};

use super::items::ItemKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Recipe {
    IronGear,
    IronPipe,
    Engine,
}

impl Recipe {
    pub const ALL: [Self; 3] = [Self::IronGear, Self::IronPipe, Self::Engine];

    /// The name used in Python: `recipe="iron_gear"`.
    pub fn id(self) -> &'static str {
        self.output().id()
    }

    pub fn by_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|recipe| recipe.id() == id)
    }

    pub fn output(self) -> ItemKind {
        match self {
            Self::IronGear => ItemKind::IronGear,
            Self::IronPipe => ItemKind::IronPipe,
            Self::Engine => ItemKind::Engine,
        }
    }

    /// What one craft uses up: (item, how many).
    pub fn inputs(self) -> &'static [(ItemKind, usize)] {
        match self {
            Self::IronGear => &[(ItemKind::IronPlate, 2)],
            Self::IronPipe => &[(ItemKind::IronPlate, 1)],
            Self::Engine => &[
                (ItemKind::IronGear, 1),
                (ItemKind::IronPipe, 2),
                (ItemKind::IronPlate, 1),
            ],
        }
    }

    /// How many of `item` one craft uses (0 if none).
    pub fn needs(self, item: ItemKind) -> usize {
        self.inputs()
            .iter()
            .find(|(input, _)| *input == item)
            .map_or(0, |(_, count)| *count)
    }

    /// Ticks one craft takes.
    pub fn ticks(self) -> u32 {
        match self {
            Self::IronGear => 60,
            Self::IronPipe => 40,
            Self::Engine => 120,
        }
    }

    /// "2 iron plates -> 1 iron gear", for the Help and the recipe picker.
    pub fn describe(self) -> String {
        let inputs: Vec<String> = self
            .inputs()
            .iter()
            .map(|(item, count)| {
                let plural = if *count == 1 { "" } else { "s" };
                format!("{count} {}{plural}", item.name())
            })
            .collect();
        format!(
            "{} -> 1 {} ({} s)",
            inputs.join(" + "),
            self.output().name(),
            self.ticks() / 20
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_have_names_and_inputs() {
        for recipe in Recipe::ALL {
            assert_eq!(Recipe::by_id(recipe.id()), Some(recipe));
            assert!(!recipe.inputs().is_empty());
            assert!(recipe.ticks() > 0);
        }
        assert_eq!(Recipe::Engine.needs(ItemKind::IronPipe), 2);
        assert_eq!(Recipe::IronGear.needs(ItemKind::IronOre), 0);
        assert_eq!(
            Recipe::Engine.describe(),
            "1 iron gear + 2 iron pipes + 1 iron plate -> 1 engine (6 s)"
        );
    }

    #[test]
    fn crafting_pays() {
        use crate::factory::train::price;
        for recipe in Recipe::ALL {
            let cost: u64 = recipe
                .inputs()
                .iter()
                .map(|(item, count)| price(*item) * *count as u64)
                .sum();
            assert!(price(recipe.output()) > cost, "{recipe:?}");
        }
    }
}
