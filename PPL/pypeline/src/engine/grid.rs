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
use crate::factory::{Factory, PLOT_WIDTH, PlotSize, Pos};

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

/// How far the island root moves left so a plot of this size, which grows
/// east, stays in the middle of the world. Whole tiles.
pub fn island_shift(size: PlotSize) -> f32 {
    ((size.width - PLOT_WIDTH) / 2 * TILE as i32) as f32
}

/// The island's outline (grass, its edges and the dirt underneath) in
/// canvas pixels from the canvas's top-left corner, y down, before bobbing
/// and before the root's shift. The same tiles `sync_plot` covers.
pub fn island_canvas_rect(size: PlotSize) -> Rect {
    let tile = TILE as f32;
    let (left, right) = (BUILD_ORIGIN.x - 1, BUILD_ORIGIN.x + size.width + 1);
    let (bottom, top) = (BUILD_ORIGIN.y - 2, BUILD_ORIGIN.y + size.height + 1);
    Rect::new(
        left as f32 * tile,
        RES_HEIGHT as f32 - top as f32 * tile,
        right as f32 * tile,
        RES_HEIGHT as f32 - bottom as f32 * tile,
    )
}

/// The plot tile under a point in the island root's space, if it is on
/// a plot of this size.
pub fn world_to_plot(world: Vec2, size: PlotSize) -> Option<Pos> {
    let x = ((world.x + RES_WIDTH as f32 / 2.0) / TILE as f32).floor() as i32;
    let y = ((world.y + RES_HEIGHT as f32 / 2.0) / TILE as f32).floor() as i32;
    let pos = Pos::new(x - BUILD_ORIGIN.x, y - BUILD_ORIGIN.y);
    size.contains(pos).then_some(pos)
}

/// World-space center of a plot tile, in whole pixels.
pub fn plot_tile_center(pos: Pos, z: f32) -> Vec3 {
    canvas_tile_center(BUILD_ORIGIN.x + pos.x, BUILD_ORIGIN.y + pos.y, z)
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_plot);
    }
}

/// One ground tile of the island.
#[derive(Component)]
struct Ground;

/// Lay the island's ground for the plot the factory has (it grows when a
/// bigger island is bought), and move the island root so it stays centered.
fn sync_plot(
    factory: Res<Factory>,
    mut island: ResMut<Island>,
    mut commands: Commands,
    old: Query<Entity, With<Ground>>,
    mut roots: Query<&mut Transform>,
    mut laid: Local<Option<PlotSize>>,
) {
    let size = factory.plot();
    if *laid == Some(size) {
        return;
    }
    *laid = Some(size);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    island.size = size;
    island.shift = island_shift(size);
    if let Ok(mut root) = roots.get_mut(island.root) {
        root.translation.x = -island.shift;
    }

    let tile = Vec2::splat(TILE as f32);
    let root = island.root;
    let mut ground = |x: i32, y: i32, color: Color| {
        commands.spawn((
            Sprite::from_color(color, tile),
            Transform::from_translation(canvas_tile_center(x, y, 0.0)),
            Ground,
            WORLD_LAYER,
            ChildOf(root),
        ));
    };

    // Grass: the buildable area plus a one-tile border on the sides and top.
    let (left, right) = (BUILD_ORIGIN.x - 1, BUILD_ORIGIN.x + size.width);
    let (bottom, top) = (BUILD_ORIGIN.y, BUILD_ORIGIN.y + size.height);
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
    fn island_outline_matches_the_plot() {
        let island = island_canvas_rect(PlotSize::START);
        // 16 + 2 edge tiles wide; 10 + 1 top edge + 2 dirt tiles tall.
        assert_eq!(island.width(), 18.0 * TILE as f32);
        assert_eq!(island.height(), 13.0 * TILE as f32);
        // Plot tile (0, 0)'s center is inside it, near the bottom.
        let center = plot_tile_center(Pos::new(0, 0), 0.0).truncate();
        let from_top_left = Vec2::new(
            center.x + RES_WIDTH as f32 / 2.0,
            RES_HEIGHT as f32 / 2.0 - center.y,
        );
        assert!(island.contains(from_top_left));
        assert!(island.max.y - from_top_left.y < 3.0 * TILE as f32);
    }

    #[test]
    fn plot_tiles_round_trip() {
        let size = PlotSize::START;
        for pos in [Pos::new(0, 0), Pos::new(15, 9), Pos::new(7, 3)] {
            let center = plot_tile_center(pos, 0.0).truncate();
            assert_eq!(world_to_plot(center, size), Some(pos));
            // Any pixel inside the tile maps to it too.
            assert_eq!(
                world_to_plot(center + Vec2::new(-7.9, 7.9), size),
                Some(pos)
            );
        }
        let off_plot = plot_tile_center(Pos::new(-1, 0), 0.0).truncate();
        assert_eq!(world_to_plot(off_plot, size), None);
        let east = plot_tile_center(Pos::new(20, 0), 0.0).truncate();
        assert_eq!(world_to_plot(east, size), None);
        assert_eq!(
            world_to_plot(east, PlotSize::BIGGEST),
            Some(Pos::new(20, 0))
        );
    }

    #[test]
    fn every_island_fits_in_the_world_and_stays_centered() {
        for size in [PlotSize::START, PlotSize::BIGGEST] {
            let outline = island_canvas_rect(size);
            let shift = island_shift(size);
            let (left, right) = (outline.min.x - shift, outline.max.x - shift);
            assert!(left >= 0.0 && right <= RES_WIDTH as f32, "{size:?}");
            assert_eq!(left, RES_WIDTH as f32 - right, "{size:?} is centered");
            assert!(outline.min.y >= 0.0, "{size:?} fits under the top");
        }
    }
}
