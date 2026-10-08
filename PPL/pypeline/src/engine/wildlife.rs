//! Wildlife v0: steam-moths flutter around warm boilers.
//!
//! HARD RULE (roadmap Part 1 WILDLIFE): cosmetic only. Moths read the factory
//! but never write to it, use no random numbers from the simulation (their
//! motion comes from real time and their index), and are not part of the
//! state hash or saves. They scatter when a boiler overheats and settle again
//! after a good Run. Shape and motion, not only color, show their mood.

use bevy::prelude::*;

use super::camera::WORLD_LAYER;
use super::floating_plot::Island;
use super::grid::plot_tile_center;
use super::sprites::{grid, to_image};
use crate::factory::Factory;
use crate::factory::machines::MachineKind;
use crate::scripting::LastFailure;

const MOTHS_PER_BOILER: usize = 2;
/// Never more than this many on screen.
const MAX_MOTHS: usize = 8;
const Z_MOTH: f32 = 4.5;

const WINGS_OPEN: [&str; 3] = ["w.k.w", "wwkww", "..k.."];
const WINGS_SHUT: [&str; 3] = ["..k..", ".wkw.", "..k.."];

#[derive(Resource)]
struct MothFrames([Handle<Image>; 2]);

#[derive(Component)]
struct Moth(usize);

pub struct WildlifePlugin;

impl Plugin for WildlifePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_moth_frames)
            .add_systems(Update, flutter);
    }
}

fn load_moth_frames(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(MothFrames([
        images.add(to_image(&grid(&WINGS_OPEN))),
        images.add(to_image(&grid(&WINGS_SHUT))),
    ]));
}

/// Where moth `i` circling a boiler at `center` is at time `t` (seconds).
/// `scatter` goes from 0 (calm) to 1 (fled).
pub fn moth_position(center: Vec2, i: usize, t: f32, scatter: f32) -> Vec2 {
    let speed = 1.3 + 0.4 * (i % 3) as f32;
    let angle = t * speed + i as f32 * 2.4;
    let radius = 9.0 + 3.0 * (i % 2) as f32 + 50.0 * scatter;
    let wobble = (t * 5.0 + i as f32).sin() * 1.5;
    // A flattened orbit when calm, opening into a wide circle when scared.
    let squash = 0.6 + 0.4 * scatter;
    center
        + Vec2::new(
            angle.cos() * radius,
            angle.sin() * radius * squash + 10.0 + wobble,
        )
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn flutter(
    mut commands: Commands,
    time: Res<Time<Real>>,
    factory: Res<Factory>,
    failure: Res<LastFailure>,
    frames: Option<Res<MothFrames>>,
    island: Res<Island>,
    mut moths: Query<(&Moth, &mut Transform, &mut Sprite, &mut Visibility)>,
    mut scatter: Local<f32>,
) {
    let Some(frames) = frames else { return };
    let boilers: Vec<Vec2> = factory
        .machines
        .values()
        .filter(|m| m.kind == MachineKind::SteamGenerator)
        .map(|m| plot_tile_center(m.pos, 0.0).truncate())
        .collect();
    let wanted = (boilers.len() * MOTHS_PER_BOILER).min(MAX_MOTHS);

    // Flee quickly when a boiler overheats, drift back slowly when calm.
    let alarmed = factory.halted && *failure == LastFailure::OutOfSteam;
    let dt = time.delta_secs();
    *scatter = if alarmed {
        (*scatter + dt * 1.5).min(1.0)
    } else {
        (*scatter - dt * 0.4).max(0.0)
    };

    let have = moths.iter().count();
    for i in have..wanted {
        commands.spawn((
            Sprite::from_image(frames.0[0].clone()),
            Transform::default(),
            Visibility::Hidden,
            Moth(i),
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    }

    let t = time.elapsed_secs();
    for (moth, mut transform, mut sprite, mut visibility) in &mut moths {
        if moth.0 >= wanted {
            *visibility = Visibility::Hidden;
            continue;
        }
        let center = boilers[moth.0 / MOTHS_PER_BOILER];
        let at = moth_position(center, moth.0, t, *scatter);
        // 5x3 sprites sit on pixel centers.
        transform.translation = (at.round() + Vec2::splat(0.5)).extend(Z_MOTH);
        // Frightened moths beat their wings twice as fast.
        let flap_hz = if alarmed { 16.0 } else { 8.0 };
        let frame = (t * flap_hz) as usize % 2;
        sprite.image = frames.0[frame].clone();
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calm_moths_stay_near_their_boiler_and_scared_ones_flee() {
        let boiler = Vec2::ZERO;
        for t in [0.0, 1.7, 9.3] {
            for i in 0..4 {
                assert!(moth_position(boiler, i, t, 0.0).distance(boiler) < 26.0);
                assert!(moth_position(boiler, i, t, 1.0).distance(boiler) > 30.0);
            }
        }
    }
}
