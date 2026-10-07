//! The build plan: what a run of main.py asked to build.
//!
//! Python API calls do not touch the factory. They record into a `BuildPlan`,
//! which is checked as it grows (so mistakes fail on the exact line) and is
//! applied to the factory only if the whole script finishes (roadmap: the
//! command queue + transactional reconcile).

use std::collections::BTreeMap;

use crate::factory::items::ItemKind;
use crate::factory::machines::MachineKind;
use crate::factory::{Dir, PLOT_HEIGHT, PLOT_WIDTH, Pos};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMachine {
    pub kind: MachineKind,
    pub pos: Pos,
    pub dir: Dir,
    pub ore: Option<ItemKind>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildPlan {
    pub conveyors: BTreeMap<Pos, Dir>,
    pub machines: BTreeMap<String, PlannedMachine>,
    /// Machine name -> generator name.
    pub power: BTreeMap<String, String>,
    /// What is on each tile, for clear "already taken" messages.
    taken: BTreeMap<Pos, String>,
}

impl BuildPlan {
    pub fn place_conveyor(&mut self, pos: Pos, dir: Dir) -> Result<(), String> {
        self.claim(pos, "a conveyor".into())?;
        self.conveyors.insert(pos, dir);
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
                Some(m) if !m.kind.needs_power() => {
                    return Err(format!("'{name}' makes power, it does not need any"));
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
        if !pos.in_plot() {
            return Err(format!(
                "({}, {}) is off the plot: x must be 0-{} and y must be 0-{}",
                pos.x,
                pos.y,
                PLOT_WIDTH - 1,
                PLOT_HEIGHT - 1
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
        }
    }

    #[test]
    fn tiles_cannot_be_used_twice() {
        let mut plan = BuildPlan::default();
        plan.place_conveyor(Pos::new(1, 0), Dir::East).unwrap();
        let err = plan.place_machine("m", miner(1, 0)).unwrap_err();
        assert!(err.contains("already has a conveyor"), "{err}");
    }

    #[test]
    fn placements_must_be_on_the_plot() {
        let mut plan = BuildPlan::default();
        assert!(plan.place_conveyor(Pos::new(-1, 0), Dir::East).is_err());
        assert!(
            plan.place_conveyor(Pos::new(PLOT_WIDTH, 0), Dir::East)
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
