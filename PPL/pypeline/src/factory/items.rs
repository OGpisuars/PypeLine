//! Item types that travel on belts and through machines.

/// Every kind of item in the game. Phase 1 hardcodes these; they move to
/// assets/data/recipes.ron once content becomes data-driven.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ItemKind {
    IronOre,
    IronPlate,
    /// Crafted from plates (see `recipes.rs`).
    IronGear,
    IronPipe,
    Engine,
}

impl ItemKind {
    pub const ALL: [Self; 5] = [
        Self::IronOre,
        Self::IronPlate,
        Self::IronGear,
        Self::IronPipe,
        Self::Engine,
    ];

    /// The name used in Python: `stats.produced("iron_plate")`.
    pub fn id(self) -> &'static str {
        match self {
            Self::IronOre => "iron_ore",
            Self::IronPlate => "iron_plate",
            Self::IronGear => "iron_gear",
            Self::IronPipe => "iron_pipe",
            Self::Engine => "engine",
        }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|item| item.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::IronOre => "iron ore",
            Self::IronPlate => "iron plate",
            Self::IronGear => "iron gear",
            Self::IronPipe => "iron pipe",
            Self::Engine => "engine",
        }
    }
}

/// Ores a miner can be set to dig, by the name used in Python (`ore="iron"`).
pub fn ore_by_name(name: &str) -> Option<ItemKind> {
    match name {
        "iron" => Some(ItemKind::IronOre),
        _ => None,
    }
}

/// What a smelter turns an input into.
pub fn smelt(input: ItemKind) -> Option<ItemKind> {
    match input {
        ItemKind::IronOre => Some(ItemKind::IronPlate),
        _ => None,
    }
}
