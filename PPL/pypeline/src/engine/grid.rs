//! 16x16 tile grid and the floating plot the factory is built on.
//!
//! Canvas tile (0, 0) is the bottom-left tile of the 480x320 canvas (30x20
//! tiles). The plot's buildable area starts at `BUILD_ORIGIN`, so script
//! position (0, 0) maps to that canvas tile. Positions are whole pixels so
//! nothing ever lands between pixels.

use bevy::prelude::*;

use super::camera::{RES_HEIGHT, RES_WIDTH, WORLD_LAYER};
use super::floating_plot::Island;
use super::palette;
use crate::factory::{PLOT_HEIGHT, PLOT_WIDTH, Pos};

pub const TILE: u32 = 16;

/// Canvas tile of script position (0, 0).
pub const BUILD_ORIGIN: IVec2 = IVec2::new(7, 4);

/// World-space center of canvas tile (x, y), in whole pixels.
pub fn canvas_tile_center(x: i32, y: i32, z: f32) -> Vec3 {
    let half = TILE as f32 / 2.0;
    Vec3::new(
        (x * TILE as i32) as f32 - RES_WIDTH as f32 / 2.0 + half,
        (y * TILE as i32) as f32 - RES_HEIGHT as f32 / 2.0 + half,
        z,
    )
}

/// The plot tile under a world-space point, if it is on the plot.
pub fn world_to_plot(world: Vec2) -> Option<Pos> {
    let x = ((world.x + RES_WIDTH as f32 / 2.0) / TILE as f32).floor() as i32;
    let y = ((world.y + RES_HEIGHT as f32 / 2.0) / TILE as f32).floor() as i32;
    let pos = Pos::new(x - BUILD_ORIGIN.x, y - BUILD_ORIGIN.y);
    pos.in_plot().then_some(pos)
}

/// World-space center of a plot tile, in whole pixels.
pub fn plot_tile_center(pos: Pos, z: f32) -> Vec3 {
    canvas_tile_center(BUILD_ORIGIN.x + pos.x, BUILD_ORIGIN.y + pos.y, z)
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_plot);
    }
}

fn spawn_plot(island: Res<Island>, mut commands: Commands) {
    let tile = Vec2::splat(TILE as f32);
    let mut ground = |x: i32, y: i32, color: Color| {
        commands.spawn((
            Sprite::from_color(color, tile),
            Transform::from_translation(canvas_tile_center(x, y, 0.0)),
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    };

    // Grass: the buildable area plus a one-tile border on the sides and top.
    let (left, right) = (BUILD_ORIGIN.x - 1, BUILD_ORIGIN.x + PLOT_WIDTH);
    let (bottom, top) = (BUILD_ORIGIN.y, BUILD_ORIGIN.y + PLOT_HEIGHT);
    for y in bottom..=top {
        for x in left..=right {
            let color = if x == left || x == right || y == top {
                palette::GRASS_DARK
            } else if (x + y) % 2 == 0 {
                palette::GRASS
            } else {
                palette::GRASS_LIGHT
            };
            ground(x, y, color);
        }
    }
    // Dirt underside so the plot reads as a floating island.
    for x in left..=right {
        ground(x, bottom - 1, palette::DIRT);
        ground(x, bottom - 2, palette::DIRT_DARK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_tiles_round_trip() {
        for pos in [Pos::new(0, 0), Pos::new(15, 9), Pos::new(7, 3)] {
            let center = plot_tile_center(pos, 0.0).truncate();
            assert_eq!(world_to_plot(center), Some(pos));
            // Any pixel inside the tile maps to it too.
            assert_eq!(world_to_plot(center + Vec2::new(-7.9, 7.9)), Some(pos));
        }
        let off_plot = plot_tile_center(Pos::new(-1, 0), 0.0).truncate();
        assert_eq!(world_to_plot(off_plot), None);
    }
}
