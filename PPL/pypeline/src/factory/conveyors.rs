//! Conveyor belts and splitters.
//!
//! Each belt tile carries items front-to-back. An item's `progress` is how far
//! it has travelled across the tile, in whole pixels (0 = back edge, 16 = front
//! edge), so rendering never lands between pixels.
//!
//! A splitter is a belt tile with two outputs. It takes items from any side
//! and hands them out in turn: one to each output, so both get half. If the
//! output whose turn it is cannot take the item, the other one gets it, so
//! a full line never jams the splitter.

use super::Dir;
use super::items::ItemKind;

/// Length of one tile in progress units (= pixels).
pub const TILE_PROGRESS: u8 = 16;
/// Minimum gap between two items on a belt.
pub const SPACING: u8 = 8;
/// Pixels an item moves per tick on a tier 1 belt (20 px/s at 20 ticks/s).
/// Faster tiers from the Shop: see `shop::belt_speed`.
pub const SPEED: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BeltItem {
    pub kind: ItemKind,
    pub progress: u8,
}

/// A splitter's two outputs, and which one gets the next item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Split {
    pub outputs: [Dir; 2],
    /// Index into `outputs` of the output whose turn it is.
    pub next: u8,
}

impl Split {
    pub fn new(first: Dir, second: Dir) -> Self {
        Self {
            outputs: [first, second],
            next: 0,
        }
    }

    /// The output whose turn it is.
    pub fn current(&self) -> Dir {
        self.outputs[usize::from(self.next & 1)]
    }

    /// The other output.
    pub fn other(&self) -> Dir {
        self.outputs[usize::from(!self.next & 1)]
    }

    /// The other output's turn next.
    pub fn take_turn(&mut self) {
        self.next ^= 1;
    }
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Conveyor {
    pub dir: Dir,
    /// Front item first (highest progress).
    pub items: Vec<BeltItem>,
    /// 1 = plain belt, 2 = fast, 3 = express.
    #[serde(default = "first_tier")]
    pub tier: u8,
    /// Set for a splitter. `dir` is then its first output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<Split>,
}

// Written by hand so a plain belt prints exactly as it did before splitters
// existed: the factory's state hash is taken from this text, and the
// determinism tests compare it with a fixed value.
impl std::fmt::Debug for Conveyor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("Conveyor");
        out.field("dir", &self.dir)
            .field("items", &self.items)
            .field("tier", &self.tier);
        if let Some(split) = &self.split {
            out.field("split", split);
        }
        out.finish()
    }
}

pub(crate) fn first_tier() -> u8 {
    1
}

impl Conveyor {
    pub fn new(dir: Dir) -> Self {
        Self {
            dir,
            items: Vec::new(),
            tier: 1,
            split: None,
        }
    }

    /// The way items are heading: a splitter's output whose turn it is.
    pub fn heading(&self) -> Dir {
        self.split.map_or(self.dir, |split| split.current())
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
        let speed = super::shop::belt_speed(self.tier);
        let mut limit = TILE_PROGRESS;
        for item in &mut self.items {
            item.progress = (item.progress + speed).min(limit);
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

    #[test]
    fn plain_belts_print_as_before_splitters() {
        let text = format!("{:?}", Conveyor::new(Dir::East));
        assert_eq!(text, "Conveyor { dir: East, items: [], tier: 1 }");
    }

    #[test]
    fn splitters_take_turns() {
        let mut split = Split::new(Dir::North, Dir::South);
        assert_eq!((split.current(), split.other()), (Dir::North, Dir::South));
        split.take_turn();
        assert_eq!((split.current(), split.other()), (Dir::South, Dir::North));
        split.take_turn();
        assert_eq!(split.current(), Dir::North);
    }

    #[test]
    fn express_belts_are_four_times_as_fast() {
        let ticks_to_cross = |tier| {
            let mut belt = Conveyor::new(Dir::East);
            belt.tier = tier;
            belt.push_back(ItemKind::IronOre);
            (1..).find(|_| belt.advance()).unwrap()
        };
        assert_eq!(ticks_to_cross(1), 16);
        assert_eq!(ticks_to_cross(2), 8);
        assert_eq!(ticks_to_cross(3), 4);
    }
}
