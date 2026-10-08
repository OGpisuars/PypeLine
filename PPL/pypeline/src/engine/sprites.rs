//! PLACEHOLDER pixel art, drawn in code.
//!
//! Each sprite is a grid of characters, one per pixel, using the color keys in
//! `color_of`. This keeps Phase 1 free of asset files; real sprites replace
//! these during the art pass (assets/sprites/).

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::factory::Dir;
use crate::factory::items::ItemKind;
use crate::factory::machines::MachineKind;

/// Frames in the belt animation. Belts move 1 px per tick and the chevrons
/// repeat every 8 px, so frame = tick % 8 keeps them in step with the items.
pub const BELT_FRAMES: usize = 8;

const MINER: [&str; 16] = [
    "................",
    "......kkkk......",
    ".....kyyyyk.....",
    "....kywwyyYk....",
    "....kyyyyyYk....",
    "..kkkkkkkkkkkk..",
    "..kllmmmmmmmdk..",
    "..klmmmmmmmmdk..",
    "..kmmmkkkkmmdk..",
    "..kmmkdmmdkmdk..",
    "..kmmkdmmdkmdk..",
    "..kddkkddkkddk..",
    "..kkkk.kk.kkkk..",
    ".......kk.......",
    "......kddk......",
    ".......kk.......",
];

const SMELTER: [&str; 16] = [
    "..........kkk...",
    "..........kRk...",
    "..........kRk...",
    "..kkkkkkkkkRkk..",
    "..krrRrrrRrrrk..",
    "..kRRRRRRRRRRk..",
    "..krrrRrrrrRrk..",
    "..kRRRRRRRRRRk..",
    "..krRkkkkkkRrk..",
    "..kRkooyyookRk..",
    "..krkoyyyyokrk..",
    "..kRkooooookRk..",
    "..kRRRRRRRRRRk..",
    "..kkkkkkkkkkkk..",
    "..kDk......kDk..",
    "..kkk......kkk..",
];

const GENERATOR: [&str; 16] = [
    "......kkkk......",
    ".....kccCCk.....",
    "....kcwcccCk....",
    "...kcwccccCCk...",
    "..kcccccccccCk..",
    "..kkkkkkkkkkkk..",
    "..kyyyyyyyyyYk..",
    "..kcccccccccCk..",
    "..kcwcccccccCk..",
    "..kcwcccccccCk..",
    "..kcccccccccCk..",
    "..kyyyyyyyyyYk..",
    "..kCCCCCCCCCCk..",
    "..kkkkkkkkkkkk..",
    "...kDk....kDk...",
    "...kkk....kkk...",
];

const STATION: [&str; 16] = [
    "................",
    ".....kkkkkk.....",
    "....kRRRRRRk....",
    "...kRrrrrrrRk...",
    "..kRrrrrrrrrRk..",
    ".kkkkkkkkkkkkkk.",
    "..kCccccccccCk..",
    "..kCckkkkkkcCk..",
    "..kCckyyyykcCk..",
    "..kCckyYYykcCk..",
    "..kCckyyyykcCk..",
    "..kCckkkkkkcCk..",
    "..kCccccccccCk..",
    "..kCCCCCCCCCCk..",
    "..kkkkkkkkkkkk..",
    "................",
];

const LOCOMOTIVE: [&str; 16] = [
    "................",
    "................",
    "..kkk...........",
    "..kmk..kkkkkk...",
    ".kkmkkkkcwwck...",
    ".kcccccccwwck...",
    ".kcccccccccck...",
    ".kyyyyyyyyyyk...",
    ".kcccccccccckk..",
    ".kccccccccccCCk.",
    ".kkkkkkkkkkkkkk.",
    "..kDk.kDk.kDk...",
    "..kkk.kkk.kkk...",
    "................",
    "................",
    "................",
];

const CARGO_CAR: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    ".kkkkkkkkkkkkkk.",
    ".kmmmmmmmmmmmmk.",
    ".kmgbgmmpwpmmmk.",
    ".kmggbmmppppmmk.",
    ".kmmmmmmmmmmmmk.",
    ".kddddddddddddk.",
    ".kkkkkkkkkkkkkk.",
    "..kDk......kDk..",
    "..kkk......kkk..",
    "................",
    "................",
    "................",
];

const ORE: [&str; 6] = [".kkkk.", "kgbggk", "kggggk", "kgggbk", "kggggk", ".kkkk."];

const PLATE: [&str; 6] = ["kkkkkk", "kwpppk", "kppppk", "kppppk", "kpppdk", "kkkkkk"];

fn color_of(key: char) -> [u8; 4] {
    let [r, g, b] = match key {
        '.' => return [0, 0, 0, 0],
        'k' => [40, 32, 48],
        'D' => [64, 64, 80],
        'd' => [88, 96, 112],
        'm' => [144, 152, 168],
        'l' => [184, 192, 200],
        'w' => [248, 248, 240],
        'y' => [224, 168, 56],
        'Y' => [168, 112, 32],
        'c' => [200, 112, 64],
        'C' => [136, 72, 40],
        'r' => [168, 64, 48],
        'R' => [112, 40, 40],
        'o' => [248, 152, 40],
        'g' => [112, 104, 120],
        'b' => [176, 96, 56],
        'p' => [200, 208, 216],
        other => panic!("unknown sprite color key {other:?}"),
    };
    [r, g, b, 255]
}

type Grid = Vec<Vec<char>>;

fn grid(rows: &[&str]) -> Grid {
    rows.iter().map(|row| row.chars().collect()).collect()
}

/// An east-facing belt with its chevrons shifted `phase` pixels east.
fn belt_grid(phase: usize) -> Grid {
    (0..16)
        .map(|r: usize| {
            (0..16)
                .map(|c: usize| match r {
                    0 | 15 => 'k',
                    1 => 'm',
                    14 => 'D',
                    3..=12 => {
                        // Distance from the belt's center line: 4,3,2,1,0,0,1,2,3,4.
                        let dy = (2 * r).abs_diff(15) / 2;
                        let along = (c + 8 * 2 - phase - (4 - dy)) % 8;
                        if along < 2 { 'y' } else { 'd' }
                    }
                    _ => 'd',
                })
                .collect()
        })
        .collect()
}

/// Turn an east-facing grid to face `dir`.
fn facing(g: &Grid, dir: Dir) -> Grid {
    let n = g.len();
    (0..n)
        .map(|r| {
            (0..n)
                .map(|c| match dir {
                    Dir::East => g[r][c],
                    Dir::North => g[c][n - 1 - r],
                    Dir::South => g[n - 1 - c][r],
                    Dir::West => g[n - 1 - r][n - 1 - c],
                })
                .collect()
        })
        .collect()
}

fn to_image(g: &Grid) -> Image {
    let height = g.len();
    let width = g[0].len();
    assert!(g.iter().all(|row| row.len() == width), "ragged sprite grid");
    let data = g.iter().flatten().flat_map(|&key| color_of(key)).collect();
    Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Resource)]
pub struct SpriteSheet {
    /// `belts[dir_index(dir)][frame]`.
    belts: Vec<Vec<Handle<Image>>>,
    miner: Handle<Image>,
    smelter: Handle<Image>,
    generator: Handle<Image>,
    station: Handle<Image>,
    pub locomotive: Handle<Image>,
    pub cargo_car: Handle<Image>,
    ore: Handle<Image>,
    plate: Handle<Image>,
}

fn dir_index(dir: Dir) -> usize {
    Dir::ALL
        .iter()
        .position(|&d| d == dir)
        .expect("ALL has every dir")
}

impl SpriteSheet {
    pub fn belt(&self, dir: Dir, frame: usize) -> Handle<Image> {
        self.belts[dir_index(dir)][frame % BELT_FRAMES].clone()
    }

    pub fn machine(&self, kind: MachineKind) -> Handle<Image> {
        match kind {
            MachineKind::Miner => self.miner.clone(),
            MachineKind::Smelter => self.smelter.clone(),
            MachineKind::SteamGenerator => self.generator.clone(),
            MachineKind::Station => self.station.clone(),
        }
    }

    pub fn item(&self, item: ItemKind) -> Handle<Image> {
        match item {
            ItemKind::IronOre => self.ore.clone(),
            ItemKind::IronPlate => self.plate.clone(),
        }
    }
}

pub fn build_sprite_sheet(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let belts = Dir::ALL
        .iter()
        .map(|&dir| {
            (0..BELT_FRAMES)
                .map(|frame| images.add(to_image(&facing(&belt_grid(frame), dir))))
                .collect()
        })
        .collect();
    commands.insert_resource(SpriteSheet {
        belts,
        miner: images.add(to_image(&grid(&MINER))),
        smelter: images.add(to_image(&grid(&SMELTER))),
        generator: images.add(to_image(&grid(&GENERATOR))),
        station: images.add(to_image(&grid(&STATION))),
        locomotive: images.add(to_image(&grid(&LOCOMOTIVE))),
        cargo_car: images.add(to_image(&grid(&CARGO_CAR))),
        ore: images.add(to_image(&grid(&ORE))),
        plate: images.add(to_image(&grid(&PLATE))),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sprites_are_well_formed() {
        for (name, rows) in [
            ("miner", &MINER[..]),
            ("smelter", &SMELTER),
            ("generator", &GENERATOR),
            ("station", &STATION),
            ("locomotive", &LOCOMOTIVE),
            ("cargo car", &CARGO_CAR),
        ] {
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(
                    row.chars().count(),
                    16,
                    "{name} row {i} is not 16 wide: {row:?}"
                );
            }
        }
        for rows in [&ORE[..], &PLATE] {
            assert!(rows.iter().all(|row| row.len() == 6));
        }
        // Every color key is known (color_of panics otherwise).
        for rows in [
            &MINER[..],
            &SMELTER,
            &GENERATOR,
            &STATION,
            &LOCOMOTIVE,
            &CARGO_CAR,
            &ORE,
            &PLATE,
        ] {
            rows.iter().flat_map(|r| r.chars()).for_each(|c| {
                color_of(c);
            });
        }
    }

    #[test]
    fn belt_frames_turn_correctly() {
        let east = belt_grid(0);
        // East-facing: dark rails along the top and bottom rows.
        assert!(east[0].iter().all(|&c| c == 'k'));
        // North-facing: the rails become the left and right columns.
        let north = facing(&east, Dir::North);
        assert!(north.iter().all(|row| row[0] == 'k' && row[15] == 'k'));
    }
}
