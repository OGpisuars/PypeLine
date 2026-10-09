//! The cargo train: every `TRAIN_INTERVAL` factory ticks it stops below the
//! island, buys everything waiting in stations (and in the station inventory
//! of items returned from removed belts and machines), and pays coins.
//!
//! The schedule runs on `Factory::ticks`, so it is deterministic and pauses
//! while the factory is halted. The train's movement on screen is render-only.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::items::ItemKind;

/// Ticks between train visits (30 seconds).
pub const TRAIN_INTERVAL: u64 = 600;

/// Coins the train pays for one item.
pub fn price(item: ItemKind) -> u64 {
    match item {
        ItemKind::IronOre => 1,
        ItemKind::IronPlate => 4,
        // Worth more than what goes into them, so crafting pays.
        ItemKind::IronGear => 12,
        ItemKind::IronPipe => 6,
        ItemKind::Engine => 60,
    }
}

/// What the train bought on one visit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sale {
    pub tick: u64,
    pub items: BTreeMap<ItemKind, u64>,
    pub coins: u64,
}

impl Sale {
    pub fn summary(&self) -> String {
        if self.items.is_empty() {
            return "The cargo train came by, but there was nothing to buy.".into();
        }
        let goods: Vec<String> = self
            .items
            .iter()
            .map(|(item, count)| format!("{count} {}", item.name()))
            .collect();
        format!(
            "The cargo train bought {} for {} coins.",
            goods.join(", "),
            self.coins
        )
    }
}

/// Is `tick` a train visit?
pub fn arrives_at(tick: u64) -> bool {
    tick > 0 && tick.is_multiple_of(TRAIN_INTERVAL)
}
