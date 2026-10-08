//! The floating island: everything on the plot hangs off one root entity that
//! bobs up and down by a whole pixel, and two layers of clouds drift behind it
//! at different speeds (parallax). Render-only.

use bevy::prelude::*;

use super::camera::{RES_WIDTH, WORLD_LAYER};

/// Seconds for one full bob up and down.
const BOB_PERIOD: f32 = 4.0;
/// Behind everything else in the world.
const Z_CLOUDS: f32 = -5.0;

/// The island root. Spawn plot sprites with `ChildOf(island.root)`.
#[derive(Resource, Debug, Clone, Copy)]
pub struct Island {
    pub root: Entity,
    /// Current bob offset in whole pixels (-1, 0 or 1).
    pub bob: f32,
}

#[derive(Component)]
struct Cloud {
    /// Pixels per second to the left; far clouds move slower.
    speed: f32,
    x: f32,
}

pub struct FloatingPlotPlugin;

impl Plugin for FloatingPlotPlugin {
    fn build(&self, app: &mut App) {
        // PreStartup, so the root exists before anything spawns onto it.
        app.add_systems(PreStartup, spawn_island_root)
            .add_systems(Startup, spawn_clouds)
            .add_systems(Update, (bob_island, drift_clouds));
    }
}

fn spawn_island_root(mut commands: Commands) {
    let root = commands
        .spawn((Transform::default(), Visibility::default(), WORLD_LAYER))
        .id();
    commands.insert_resource(Island { root, bob: 0.0 });
}

fn bob_island(time: Res<Time<Real>>, mut island: ResMut<Island>, mut roots: Query<&mut Transform>) {
    let phase = time.elapsed_secs() / BOB_PERIOD * std::f32::consts::TAU;
    let bob = phase.sin().round();
    if island.bob != bob {
        island.bob = bob;
        if let Ok(mut transform) = roots.get_mut(island.root) {
            transform.translation.y = bob;
        }
    }
}

/// A cloud made of a few overlapping white blocks, so it reads as pixel art.
fn spawn_clouds(mut commands: Commands) {
    // (x, y, size, speed): two far, slow layers and a near, faster one.
    let clouds = [
        (-180.0, 120.0, 1.0, 3.0),
        (40.0, 140.0, 1.0, 3.0),
        (200.0, 100.0, 1.0, 3.0),
        (-60.0, -110.0, 2.0, 6.0),
        (160.0, -130.0, 2.0, 6.0),
    ];
    for (x, y, size, speed) in clouds {
        let tint = if speed < 5.0 {
            Color::srgba(1.0, 1.0, 1.0, 0.55)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.8)
        };
        commands
            .spawn((
                Transform::from_xyz(x, y, Z_CLOUDS),
                Visibility::default(),
                Cloud { speed, x },
                WORLD_LAYER,
            ))
            .with_children(|cloud| {
                for (dx, dy, w, h) in [
                    (0.0, 0.0, 24.0, 6.0),
                    (-6.0, 4.0, 12.0, 4.0),
                    (4.0, 5.0, 10.0, 6.0),
                    (10.0, -2.0, 8.0, 4.0),
                ] {
                    cloud.spawn((
                        Sprite::from_color(tint, Vec2::new(w * size, h * size).round()),
                        Transform::from_xyz((dx * size).round(), (dy * size).round(), 0.0),
                        WORLD_LAYER,
                    ));
                }
            });
    }
}

fn drift_clouds(time: Res<Time<Real>>, mut clouds: Query<(&mut Cloud, &mut Transform)>) {
    let wrap = RES_WIDTH as f32 / 2.0 + 40.0;
    for (mut cloud, mut transform) in &mut clouds {
        cloud.x -= cloud.speed * time.delta_secs();
        if cloud.x < -wrap {
            cloud.x += 2.0 * wrap;
        }
        // Whole pixels only.
        transform.translation.x = cloud.x.round();
    }
}
