//! Conveyor belts.
//!
//! Each belt tile carries items front-to-back. An item's `progress` is how far
//! it has travelled across the tile, in whole pixels (0 = back edge, 16 = front
//! edge), so rendering never lands between pixels.

use super::Dir;
use super::items::ItemKind;

/// Length of one tile in progress units (= pixels).
pub const TILE_PROGRESS: u8 = 16;
/// Minimum gap between two items on a belt.
pub const SPACING: u8 = 8;
/// Pixels an item moves per tick (20 px/s at 20 ticks/s).
pub const SPEED: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BeltItem {
    pub kind: ItemKind,
    pub progress: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Conveyor {
    pub dir: Dir,
    /// Front item first (highest progress).
    pub items: Vec<BeltItem>,
}

impl Conveyor {
    pub fn new(dir: Dir) -> Self {
        Self {
            dir,
            items: Vec::new(),
        }
    }

    /// True if there is room for a new item at the back of the belt.
    pub fn can_accept(&self) -> bool {
        self.items
            .last()
            .is_none_or(|back| back.progress >= SPACING)
    }

    /// Put an item on the back of the belt. Call `can_accept` first.
    pub fn push_back(&mut self, kind: ItemKind) {
        self.items.push(BeltItem { kind, progress: 0 });
    }

    /// Move items forward, keeping them spaced out and stopping at the front
    /// edge. Returns true if the front item is waiting to leave the tile.
    pub fn advance(&mut self) -> bool {
        let mut limit = TILE_PROGRESS;
        for item in &mut self.items {
            item.progress = (item.progress + SPEED).min(limit);
            limit = item.progress.saturating_sub(SPACING);
        }
        self.items
            .first()
            .is_some_and(|front| front.progress == TILE_PROGRESS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_keep_their_spacing() {
        let mut belt = Conveyor::new(Dir::East);
        belt.push_back(ItemKind::IronOre);
        for _ in 0..SPACING {
            belt.advance();
        }
        assert!(belt.can_accept());
        belt.push_back(ItemKind::IronOre);
        for _ in 0..100 {
            belt.advance();
        }
        // Front item waits at the edge; the second stops one gap behind it.
        assert_eq!(belt.items[0].progress, TILE_PROGRESS);
        assert_eq!(belt.items[1].progress, TILE_PROGRESS - SPACING);
    }
}
