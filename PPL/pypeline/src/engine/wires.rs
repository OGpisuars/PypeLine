//! Power wires: a brass line from each steam generator to every machine it
//! powers, with pulses running along it. Makes `power.connect` visible.
//!
//! Wires follow an L-shaped path (across, then up or down) between tile
//! centers. Pulses move one pixel per factory tick, so they stop when the
//! factory halts. Render-only.

use bevy::prelude::*;

use super::camera::WORLD_LAYER;
use super::grid::plot_tile_center;
use super::palette;
use crate::factory::{Factory, Pos};

/// Between the ground and the belts.
const Z_WIRE: f32 = 0.8;
const Z_PULSE: f32 = 0.9;
/// Pixels between pulses on a wire.
const PULSE_GAP: usize = 12;

#[derive(Component)]
struct WireSprite;

#[derive(Component)]
struct PulseSprite;

/// Pixel paths of the current wires, for the pulses to follow.
#[derive(Resource, Default)]
struct WirePaths(Vec<Vec<IVec2>>);

pub struct WirePlugin;

impl Plugin for WirePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WirePaths>()
            .add_systems(Update, (sync_wires, move_pulses).chain());
    }
}

fn center(pos: Pos) -> IVec2 {
    plot_tile_center(pos, 0.0).truncate().as_ivec2()
}

/// Every pixel corner along an L-shaped path from `from` to `to`.
pub fn wire_path(from: Pos, to: Pos) -> Vec<IVec2> {
    let (a, b) = (center(from), center(to));
    let mut path = Vec::new();
    let step_x = (b.x - a.x).signum();
    let mut x = a.x;
    while x != b.x {
        path.push(IVec2::new(x, a.y));
        x += step_x;
    }
    let step_y = (b.y - a.y).signum();
    let mut y = a.y;
    while y != b.y {
        path.push(IVec2::new(b.x, y));
        y += step_y;
    }
    path.push(b);
    path
}

fn sync_wires(
    mut commands: Commands,
    factory: Res<Factory>,
    old: Query<Entity, With<WireSprite>>,
    mut paths: ResMut<WirePaths>,
    mut drawn_version: Local<Option<u64>>,
) {
    if *drawn_version == Some(factory.layout_version) {
        return;
    }
    *drawn_version = Some(factory.layout_version);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    paths.0.clear();

    for (machine, generator) in &factory.power {
        let (Some(m), Some(g)) = (
            factory.machines.get(machine),
            factory.machines.get(generator),
        ) else {
            continue;
        };
        let path = wire_path(g.pos, m.pos);
        // One 1x1 sprite per pixel keeps every pixel exactly on the grid.
        for p in &path {
            commands.spawn((
                Sprite::from_color(palette::BRASS, Vec2::ONE),
                Transform::from_xyz(p.x as f32 + 0.5, p.y as f32 + 0.5, Z_WIRE),
                WireSprite,
                WORLD_LAYER,
            ));
        }
        paths.0.push(path);
    }
}

fn move_pulses(
    mut commands: Commands,
    factory: Res<Factory>,
    paths: Res<WirePaths>,
    mut pool: Query<(&mut Transform, &mut Visibility), With<PulseSprite>>,
) {
    let offset = factory.ticks as usize % PULSE_GAP;
    let wanted: Vec<Vec3> = paths
        .0
        .iter()
        .flat_map(|path| {
            path.iter()
                .skip(offset)
                .step_by(PULSE_GAP)
                .map(|p| Vec3::new(p.x as f32 + 0.5, p.y as f32 + 0.5, Z_PULSE))
        })
        .collect();

    let mut wanted = wanted.into_iter();
    for (mut transform, mut visibility) in &mut pool {
        match wanted.next() {
            Some(at) => {
                transform.translation = at;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for at in wanted {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(248, 240, 168), Vec2::splat(3.0)),
            Transform::from_translation(at),
            PulseSprite,
            WORLD_LAYER,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_paths_connect_both_ends() {
        let (from, to) = (Pos::new(0, 2), Pos::new(5, 0));
        let path = wire_path(from, to);
        assert_eq!(path.first(), Some(&center(from)));
        assert_eq!(path.last(), Some(&center(to)));
        // Every step moves exactly one pixel.
        for pair in path.windows(2) {
            assert_eq!((pair[1] - pair[0]).abs().element_sum(), 1);
        }
    }
}
