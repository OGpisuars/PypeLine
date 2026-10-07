//! 16x16 tile grid and a placeholder floating plot.
//!
//! Tile (0, 0) is the bottom-left tile of the 480x320 canvas, which is 30x20
//! tiles. Positions are whole pixels so nothing ever lands between pixels.

use bevy::prelude::*;

use super::camera::{RES_HEIGHT, RES_WIDTH, WORLD_LAYER};
use super::palette;

pub const TILE: u32 = 16;
pub const GRID_WIDTH: i32 = (RES_WIDTH / TILE) as i32;
pub const GRID_HEIGHT: i32 = (RES_HEIGHT / TILE) as i32;

/// A position on the tile grid.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    /// World-space center of the tile, in whole pixels.
    pub fn to_world(self, z: f32) -> Vec3 {
        let half = TILE as f32 / 2.0;
        Vec3::new(
            (self.x * TILE as i32) as f32 - RES_WIDTH as f32 / 2.0 + half,
            (self.y * TILE as i32) as f32 - RES_HEIGHT as f32 / 2.0 + half,
            z,
        )
    }
}

/// The placeholder island: a rectangle of tiles, in grid coordinates.
const PLOT_MIN: IVec2 = IVec2::new(6, 4);
const PLOT_MAX: IVec2 = IVec2::new(23, 14);

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_placeholder_plot);
    }
}

fn spawn_placeholder_plot(mut commands: Commands) {
    let tile = Vec2::splat(TILE as f32);

    for y in PLOT_MIN.y..=PLOT_MAX.y {
        for x in PLOT_MIN.x..=PLOT_MAX.x {
            let pos = TilePos { x, y };
            let color = if x == PLOT_MIN.x || x == PLOT_MAX.x || y == PLOT_MAX.y {
                palette::GRASS_DARK
            } else if (x + y) % 2 == 0 {
                palette::GRASS
            } else {
                palette::GRASS_LIGHT
            };
            commands.spawn((
                Sprite::from_color(color, tile),
                Transform::from_translation(pos.to_world(0.0)),
                pos,
                WORLD_LAYER,
            ));
        }
    }
    // Dirt underside so the plot reads as a floating island.
    for x in PLOT_MIN.x..=PLOT_MAX.x {
        for (dy, color) in [(1, palette::DIRT), (2, palette::DIRT_DARK)] {
            let pos = TilePos {
                x,
                y: PLOT_MIN.y - dy,
            };
            commands.spawn((
                Sprite::from_color(color, tile),
                Transform::from_translation(pos.to_world(0.0)),
                pos,
                WORLD_LAYER,
            ));
        }
    }

    // A metal pad with a placeholder terminal and steam generator.
    for x in 8..=11 {
        for y in 10..=12 {
            let pos = TilePos { x, y };
            let color = if (x + y) % 2 == 0 {
                palette::METAL
            } else {
                palette::METAL_LIGHT
            };
            commands.spawn((
                Sprite::from_color(color, tile),
                Transform::from_translation(pos.to_world(1.0)),
                pos,
                WORLD_LAYER,
            ));
        }
    }
    spawn_block(
        &mut commands,
        TilePos { x: 9, y: 11 },
        palette::OUTLINE,
        palette::BRASS,
    );
    spawn_block(
        &mut commands,
        TilePos { x: 10, y: 11 },
        palette::OUTLINE,
        palette::COPPER,
    );
}

/// A 1x1 placeholder machine: a 1px dark outline around a flat fill.
fn spawn_block(commands: &mut Commands, pos: TilePos, outline: Color, fill: Color) {
    let world = pos.to_world(2.0);
    commands.spawn((
        Sprite::from_color(outline, Vec2::splat(TILE as f32)),
        Transform::from_translation(world),
        pos,
        WORLD_LAYER,
    ));
    commands.spawn((
        Sprite::from_color(fill, Vec2::splat(TILE as f32 - 2.0)),
        Transform::from_translation(world + Vec3::Z * 0.1),
        WORLD_LAYER,
    ));
}
