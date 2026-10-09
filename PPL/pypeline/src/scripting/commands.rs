//! The build plan: what a run of main.py asked to build.
//!
//! Python API calls do not touch the factory. They record into a `BuildPlan`,
//! which is checked as it grows (so mistakes fail on the exact line) and is
//! applied to the factory only if the whole script finishes (roadmap: the
//! command queue + transactional reconcile).

use std::collections::{BTreeMap, BTreeSet};

use crate::factory::items::ItemKind;
use crate::factory::machines::MachineKind;
use crate::factory::recipes::Recipe;
use crate::factory::shop::{self, MAX_TIER, Upgrade};
use crate::factory::{Dir, PlotSize, Pos};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMachine {
    pub kind: MachineKind,
    pub pos: Pos,
    pub dir: Dir,
    pub ore: Option<ItemKind>,
    pub tier: u8,
    /// Crafters: the recipe from `recipe=`. None keeps the one picked in
    /// the game.
    pub recipe: Option<Recipe>,
}

/// Check a `recipe=` against the recipes the player's achievements unlocked.
pub fn recipe(id: &str, unlocked: &BTreeSet<Recipe>) -> Result<Recipe, String> {
    let recipe = Recipe::by_id(id).ok_or_else(|| {
        let known: Vec<String> = Recipe::ALL
            .iter()
            .map(|r| format!("\"{}\"", r.id()))
            .collect();
        format!("unknown recipe \"{id}\" (try {})", known.join(", "))
    })?;
    if !unlocked.contains(&recipe) {
        return Err(format!(
            "the {} recipe is locked: unlock it with an achievement (see Achievements in the top bar)",
            recipe.output().name()
        ));
    }
    Ok(recipe)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedBelt {
    pub dir: Dir,
    pub tier: u8,
    /// A splitter's two outputs (`dir` is the first).
    pub split: Option<[Dir; 2]>,
}

impl PlannedBelt {
    pub fn new(dir: Dir) -> Self {
        Self {
            dir,
            tier: 1,
            split: None,
        }
    }

    /// A splitter that sends items to `first` and `second` in turn.
    pub fn splitter(first: Dir, second: Dir) -> Result<Self, String> {
        if first == second {
            return Err(
                "a splitter needs two different directions, like dir1=\"north\", dir2=\"south\""
                    .into(),
            );
        }
        Ok(Self {
            dir: first,
            tier: 1,
            split: Some([first, second]),
        })
    }
}

/// Check a `tier=` for belts against what the Shop has unlocked.
pub fn belt_tier(tier: i64, unlocked: &BTreeSet<Upgrade>) -> Result<u8, String> {
    let tier = tier_in_range(tier)?;
    match shop::belt_upgrade(tier) {
        Some(upgrade) if !unlocked.contains(&upgrade) => Err(locked(tier, "belts", upgrade)),
        _ => Ok(tier),
    }
}

/// Check a `tier=` for a machine against what the Shop has unlocked.
pub fn machine_tier(
    kind: MachineKind,
    tier: i64,
    unlocked: &BTreeSet<Upgrade>,
) -> Result<u8, String> {
    let tier = tier_in_range(tier)?;
    if tier > 1 && !shop::has_tiers(kind) {
        return Err(format!(
            "a {} has only one tier; leave tier= out",
            kind.name()
        ));
    }
    match shop::machine_upgrade(kind, tier) {
        Some(upgrade) if !unlocked.contains(&upgrade) => {
            Err(locked(tier, &format!("{}s", kind.name()), upgrade))
        }
        _ => Ok(tier),
    }
}

fn tier_in_range(tier: i64) -> Result<u8, String> {
    match u8::try_from(tier) {
        Ok(t) if (1..=MAX_TIER).contains(&t) => Ok(t),
        _ => Err(format!("tier must be 1, 2 or 3, not {tier}")),
    }
}

fn locked(tier: u8, what: &str, upgrade: Upgrade) -> String {
    format!(
        "tier {tier} {what} are locked: buy \"{}\" in the Shop (F3) for {} coins",
        upgrade.title(),
        upgrade.price()
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildPlan {
    pub conveyors: BTreeMap<Pos, PlannedBelt>,
    pub machines: BTreeMap<String, PlannedMachine>,
    /// Machine name -> generator name.
    pub power: BTreeMap<String, String>,
    /// How big the plot is (bigger islands come from the Shop).
    pub size: PlotSize,
    /// What is on each tile, for clear "already taken" messages.
    taken: BTreeMap<Pos, String>,
}

impl BuildPlan {
    pub fn place_conveyor(&mut self, pos: Pos, belt: PlannedBelt) -> Result<(), String> {
        let what = if belt.split.is_some() {
            "a splitter"
        } else {
            "a conveyor"
        };
        self.claim(pos, what.into())?;
        self.conveyors.insert(pos, belt);
        Ok(())
    }

    pub fn place_machine(&mut self, name: &str, machine: PlannedMachine) -> Result<(), String> {
        if name.is_empty() {
            return Err("a machine needs a name, like name=\"miner_1\"".into());
        }
        if self.machines.contains_key(name) {
            return Err(format!("there is already a machine named '{name}'"));
        }
        if machine.kind == MachineKind::Miner && machine.ore.is_none() {
            return Err("a miner needs to know what to dig, like ore=\"iron\"".into());
        }
        if machine.kind != MachineKind::Miner && machine.ore.is_some() {
            return Err(format!(
                "only miners take ore=, not a {}",
                machine.kind.name()
            ));
        }
        if machine.kind != MachineKind::Crafter && machine.recipe.is_some() {
            return Err(format!(
                "only crafters take recipe=, not a {}",
                machine.kind.name()
            ));
        }
        self.claim(machine.pos, format!("machine '{name}'"))?;
        self.machines.insert(name.to_owned(), machine);
        Ok(())
    }

    pub fn connect(&mut self, generator: &str, machines: &[String]) -> Result<(), String> {
        match self.machines.get(generator) {
            None => {
                return Err(format!(
                    "there is no machine named '{generator}' (place it before connecting it)"
                ));
            }
            Some(g) if g.kind != MachineKind::SteamGenerator => {
                return Err(format!(
                    "'{generator}' is a {}, not a steam_generator",
                    g.kind.name()
                ));
            }
            Some(_) => {}
        }
        for name in machines {
            match self.machines.get(name) {
                None => {
                    return Err(format!(
                        "there is no machine named '{name}' (place it before connecting it)"
                    ));
                }
                Some(m) if m.kind == MachineKind::SteamGenerator => {
                    return Err(format!("'{name}' makes power, it does not need any"));
                }
                Some(m) if !m.kind.needs_power() => {
                    return Err(format!("a {} does not need power", m.kind.name()));
                }
                Some(_) => {}
            }
            if let Some(other) = self.power.get(name)
                && other != generator
            {
                return Err(format!("'{name}' is already connected to '{other}'"));
            }
        }
        for name in machines {
            self.power.insert(name.clone(), generator.to_owned());
        }
        Ok(())
    }

    fn claim(&mut self, pos: Pos, what: String) -> Result<(), String> {
        if !self.size.contains(pos) {
            let more = if self.size != PlotSize::BIGGEST {
                " (a bigger island in the Shop gives more room)"
            } else {
                ""
            };
            return Err(format!(
                "({}, {}) is off the plot: x must be 0-{} and y must be 0-{}{more}",
                pos.x,
                pos.y,
                self.size.width - 1,
                self.size.height - 1
            ));
        }
        if let Some(existing) = self.taken.get(&pos) {
            return Err(format!("({}, {}) already has {existing}", pos.x, pos.y));
        }
        self.taken.insert(pos, what);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn miner(x: i32, y: i32) -> PlannedMachine {
        PlannedMachine {
            kind: MachineKind::Miner,
            pos: Pos::new(x, y),
            dir: Dir::East,
            ore: Some(ItemKind::IronOre),
            tier: 1,
            recipe: None,
        }
    }

    #[test]
    fn recipes_must_be_unlocked() {
        let none = BTreeSet::new();
        let err = recipe("iron_gear", &none).unwrap_err();
        assert!(err.contains("locked"), "{err}");
        assert!(
            recipe("cake", &none)
                .unwrap_err()
                .contains("unknown recipe")
        );
        let unlocked = BTreeSet::from([Recipe::IronGear]);
        assert_eq!(recipe("iron_gear", &unlocked), Ok(Recipe::IronGear));
        let mut plan = BuildPlan::default();
        let mut not_a_crafter = miner(0, 0);
        not_a_crafter.recipe = Some(Recipe::IronGear);
        assert!(plan.place_machine("m", not_a_crafter).is_err());
    }

    #[test]
    fn tiers_need_the_shop() {
        let none = BTreeSet::new();
        assert_eq!(belt_tier(1, &none), Ok(1));
        let err = belt_tier(2, &none).unwrap_err();
        assert!(err.contains("Fast belt"), "{err}");
        assert!(belt_tier(4, &none).is_err());
        let bought = BTreeSet::from([Upgrade::FastBelt, Upgrade::MinerMk2]);
        assert_eq!(belt_tier(2, &bought), Ok(2));
        assert_eq!(machine_tier(MachineKind::Miner, 2, &bought), Ok(2));
        assert!(machine_tier(MachineKind::Smelter, 2, &bought).is_err());
        assert!(machine_tier(MachineKind::Station, 2, &bought).is_err());
    }

    #[test]
    fn tiles_cannot_be_used_twice() {
        let mut plan = BuildPlan::default();
        plan.place_conveyor(Pos::new(1, 0), PlannedBelt::new(Dir::East))
            .unwrap();
        let err = plan.place_machine("m", miner(1, 0)).unwrap_err();
        assert!(err.contains("already has a conveyor"), "{err}");
    }

    #[test]
    fn splitters_need_two_directions() {
        assert!(PlannedBelt::splitter(Dir::North, Dir::North).is_err());
        let split = PlannedBelt::splitter(Dir::North, Dir::South).unwrap();
        let mut plan = BuildPlan::default();
        plan.place_conveyor(Pos::new(3, 3), split).unwrap();
        let err = plan
            .place_conveyor(Pos::new(3, 3), PlannedBelt::new(Dir::East))
            .unwrap_err();
        assert!(err.contains("already has a splitter"), "{err}");
    }

    #[test]
    fn placements_must_be_on_the_plot() {
        let mut plan = BuildPlan::default();
        let belt = PlannedBelt::new(Dir::East);
        assert!(plan.place_conveyor(Pos::new(-1, 0), belt).is_err());
        assert!(
            plan.place_conveyor(Pos::new(crate::factory::PLOT_WIDTH, 0), belt)
                .is_err()
        );
    }

    #[test]
    fn connect_needs_placed_machines() {
        let mut plan = BuildPlan::default();
        plan.place_machine("m", miner(0, 0)).unwrap();
        let err = plan.connect("steam_1", &["m".into()]).unwrap_err();
        assert!(err.contains("no machine named 'steam_1'"), "{err}");
    }
}
