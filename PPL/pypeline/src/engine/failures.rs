//! Visible, physical failure (roadmap Part 1 ERROR HANDLING):
//! out of steam -> the steam generators blink red and puff steam;
//! a script error -> the belts blink red. Both stop at the next good Run.
//! Render-only, driven by real time so it keeps blinking while halted.

use bevy::prelude::*;

use super::camera::WORLD_LAYER;
use super::floating_plot::Island;
use super::grid::plot_tile_center;
use super::renderer::{BeltSprite, MachineSprite};
use crate::factory::machines::MachineKind;
use crate::factory::{Factory, Pos};
use crate::scripting::LastFailure;

const BLINK_HZ: f32 = 2.0;
const ALARM: Color = Color::srgb(1.0, 0.45, 0.4);
const PUFFS_PER_BOILER: usize = 3;
const PUFF_RISE_PX: f32 = 14.0;

#[derive(Component)]
struct SteamPuff;

pub struct FailurePlugin;

impl Plugin for FailurePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (blink_failures, steam_puffs));
    }
}

fn alarm_on(time: &Time<Real>) -> bool {
    (time.elapsed_secs() * BLINK_HZ) as u32 % 2 == 0
}

fn blink_failures(
    time: Res<Time<Real>>,
    factory: Res<Factory>,
    failure: Res<LastFailure>,
    mut machines: Query<(&mut Sprite, &MachineSprite), Without<BeltSprite>>,
    mut belts: Query<&mut Sprite, With<BeltSprite>>,
) {
    let blink = factory.halted && alarm_on(&time);
    for (mut sprite, machine) in &mut machines {
        let alarmed = blink
            && *failure == LastFailure::OutOfSteam
            && machine.kind == MachineKind::SteamGenerator;
        let wanted = if alarmed { ALARM } else { machine.base };
        if sprite.color != wanted {
            sprite.color = wanted;
        }
    }
    let belt_color = if blink && *failure == LastFailure::Error {
        ALARM
    } else {
        Color::WHITE
    };
    for mut sprite in &mut belts {
        if sprite.color != belt_color {
            sprite.color = belt_color;
        }
    }
}

/// Little steam clouds rising from overheated boilers.
fn steam_puffs(
    island: Res<Island>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    factory: Res<Factory>,
    failure: Res<LastFailure>,
    mut puffs: Query<(&mut Transform, &mut Visibility), With<SteamPuff>>,
) {
    let boilers: Vec<Pos> = if factory.halted && *failure == LastFailure::OutOfSteam {
        factory
            .machines
            .values()
            .filter(|m| m.kind == MachineKind::SteamGenerator)
            .map(|m| m.pos)
            .collect()
    } else {
        Vec::new()
    };
    let now = time.elapsed_secs();
    let wanted: Vec<Vec3> = boilers
        .iter()
        .flat_map(|&pos| {
            let base = plot_tile_center(pos, 5.0) + Vec3::new(0.0, 8.0, 0.0);
            (0..PUFFS_PER_BOILER).map(move |i| {
                let phase = (now * 0.8 + i as f32 / PUFFS_PER_BOILER as f32).fract();
                let drift = if i % 2 == 0 { -2.0 } else { 3.0 };
                (base + Vec3::new(drift * phase, PUFF_RISE_PX * phase, 0.0)).round()
            })
        })
        .collect();

    let mut wanted = wanted.into_iter();
    for (mut transform, mut visibility) in &mut puffs {
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
            Sprite::from_color(Color::srgba(0.97, 0.97, 0.94, 0.85), Vec2::splat(4.0)),
            Transform::from_translation(at),
            SteamPuff,
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    }
}
