//! The Shop: spend coins from the cargo train on faster belts and machines.
//!
//! Buying an upgrade unlocks a higher `tier=` in the build API, like
//! `conveyors.place(x, y, dir="east", tier=2)`. Nothing changes until the
//! script asks for it, so the player decides where the fast parts go.
//! Unlocks live in the `Factory`, so they are saved with it and survive a
//! Clean Run.

use serde::{Deserialize, Serialize};

use super::Factory;
use super::machines::MachineKind;

/// Highest tier of anything.
pub const MAX_TIER: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Upgrade {
    FastBelt,
    ExpressBelt,
    MinerMk2,
    MinerMk3,
    SmelterMk2,
    SmelterMk3,
}

/// What an upgrade unlocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unlocks {
    Belt { tier: u8 },
    Machine { kind: MachineKind, tier: u8 },
}

impl Upgrade {
    pub const ALL: [Self; 6] = [
        Self::FastBelt,
        Self::ExpressBelt,
        Self::MinerMk2,
        Self::MinerMk3,
        Self::SmelterMk2,
        Self::SmelterMk3,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::FastBelt => "Fast belt",
            Self::ExpressBelt => "Express belt",
            Self::MinerMk2 => "Miner Mk2",
            Self::MinerMk3 => "Miner Mk3",
            Self::SmelterMk2 => "Smelter Mk2",
            Self::SmelterMk3 => "Smelter Mk3",
        }
    }

    pub fn price(self) -> u64 {
        match self {
            Self::FastBelt => 600,
            Self::ExpressBelt => 2500,
            Self::MinerMk2 => 800,
            Self::MinerMk3 => 3500,
            Self::SmelterMk2 => 1200,
            Self::SmelterMk3 => 5000,
        }
    }

    pub fn unlocks(self) -> Unlocks {
        match self {
            Self::FastBelt => Unlocks::Belt { tier: 2 },
            Self::ExpressBelt => Unlocks::Belt { tier: 3 },
            Self::MinerMk2 => Unlocks::Machine {
                kind: MachineKind::Miner,
                tier: 2,
            },
            Self::MinerMk3 => Unlocks::Machine {
                kind: MachineKind::Miner,
                tier: 3,
            },
            Self::SmelterMk2 => Unlocks::Machine {
                kind: MachineKind::Smelter,
                tier: 2,
            },
            Self::SmelterMk3 => Unlocks::Machine {
                kind: MachineKind::Smelter,
                tier: 3,
            },
        }
    }

    /// The upgrade that must be bought first.
    pub fn requires(self) -> Option<Self> {
        match self {
            Self::ExpressBelt => Some(Self::FastBelt),
            Self::MinerMk3 => Some(Self::MinerMk2),
            Self::SmelterMk3 => Some(Self::SmelterMk2),
            _ => None,
        }
    }

    /// One line on what it does.
    pub fn about(self) -> String {
        match self.unlocks() {
            Unlocks::Belt { tier } => format!(
                "Belts {}x as fast. Use tier={tier} in conveyors.place.",
                belt_speed(tier)
            ),
            Unlocks::Machine { kind, tier } => format!(
                "{}s work {}x as fast. Use tier={tier} in machines.place(\"{}\", ...).",
                capitalize(kind.name()),
                speedup(tier),
                kind.name()
            ),
        }
    }

    /// The code that uses it, for the Shop window and the Help.
    pub fn example(self) -> &'static str {
        match self {
            Self::FastBelt => "conveyors.place(x=1, y=0, dir=\"east\", tier=2)",
            Self::ExpressBelt => "conveyors.place(x=1, y=0, dir=\"east\", tier=3)",
            Self::MinerMk2 => {
                "machines.place(\"miner\", name=\"miner_1\", x=0, y=0, ore=\"iron\", tier=2)"
            }
            Self::MinerMk3 => {
                "machines.place(\"miner\", name=\"miner_1\", x=0, y=0, ore=\"iron\", tier=3)"
            }
            Self::SmelterMk2 => "machines.place(\"smelter\", name=\"smelter_1\", x=5, y=0, tier=2)",
            Self::SmelterMk3 => "machines.place(\"smelter\", name=\"smelter_1\", x=5, y=0, tier=3)",
        }
    }
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Pixels a belt item moves per tick at each tier.
pub fn belt_speed(tier: u8) -> u8 {
    match tier {
        0 | 1 => 1,
        2 => 2,
        _ => 4,
    }
}

/// How many times faster a machine of this tier works.
pub fn speedup(tier: u8) -> u32 {
    match tier {
        0 | 1 => 1,
        2 => 2,
        _ => 4,
    }
}

/// The upgrade that unlocks this belt tier (None for tier 1).
pub fn belt_upgrade(tier: u8) -> Option<Upgrade> {
    Upgrade::ALL
        .into_iter()
        .find(|u| u.unlocks() == Unlocks::Belt { tier })
}

/// The upgrade that unlocks this machine tier (None for tier 1, or for
/// machines that have no tiers).
pub fn machine_upgrade(kind: MachineKind, tier: u8) -> Option<Upgrade> {
    Upgrade::ALL
        .into_iter()
        .find(|u| u.unlocks() == Unlocks::Machine { kind, tier })
}

/// Does this kind of machine come in faster tiers?
pub fn has_tiers(kind: MachineKind) -> bool {
    matches!(kind, MachineKind::Miner | MachineKind::Smelter)
}

/// Why the shop will not sell this right now, if it will not.
pub fn cannot_buy(factory: &Factory, upgrade: Upgrade) -> Option<String> {
    if factory.unlocked.contains(&upgrade) {
        return Some("already bought".into());
    }
    if let Some(first) = upgrade.requires()
        && !factory.unlocked.contains(&first)
    {
        return Some(format!("buy {} first", first.title()));
    }
    if factory.coins < upgrade.price() {
        return Some(format!(
            "needs {} more coins",
            upgrade.price() - factory.coins
        ));
    }
    None
}

impl Factory {
    /// Buy an upgrade with coins.
    pub fn buy(&mut self, upgrade: Upgrade) -> Result<(), String> {
        if let Some(reason) = cannot_buy(self, upgrade) {
            return Err(reason);
        }
        self.coins -= upgrade.price();
        self.unlocked.insert(upgrade);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buying_costs_coins_and_needs_the_tier_below() {
        let mut factory = Factory {
            coins: 4000,
            ..Default::default()
        };
        assert!(factory.buy(Upgrade::ExpressBelt).is_err());
        factory.buy(Upgrade::FastBelt).unwrap();
        assert_eq!(factory.coins, 3400);
        assert!(factory.buy(Upgrade::FastBelt).is_err());
        factory.buy(Upgrade::ExpressBelt).unwrap();
        assert_eq!(factory.coins, 900);
        assert_eq!(
            cannot_buy(&factory, Upgrade::SmelterMk2).as_deref(),
            Some("needs 300 more coins")
        );
    }

    #[test]
    fn every_tier_has_one_upgrade() {
        for tier in 2..=MAX_TIER {
            assert!(belt_upgrade(tier).is_some());
            assert!(machine_upgrade(MachineKind::Miner, tier).is_some());
            assert!(machine_upgrade(MachineKind::Smelter, tier).is_some());
        }
        assert_eq!(belt_upgrade(1), None);
        assert_eq!(machine_upgrade(MachineKind::Station, 2), None);
    }
}
