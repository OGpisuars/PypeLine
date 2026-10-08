//! Day/night shading (roadmap: DAY / NIGHT). Render-only: it reads the
//! factory's tick counter and never changes the simulation.
//!
//! In GBA style the light changes in steps (dawn, day, dusk, night), with a
//! flat tint over the 480x320 canvas and the same tint mixed into the sky
//! around it, so the world and the space around it always match.

use bevy::prelude::*;

use super::camera::{RES_HEIGHT, RES_WIDTH, WORLD_LAYER};
use super::palette;
use crate::factory::Factory;
use crate::factory::daynight::{self, Phase};

/// Above everything else in the world.
const Z_SHADE: f32 = 50.0;

#[derive(Component)]
struct Shade;

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shade)
            .add_systems(Update, shade_world);
    }
}

fn spawn_shade(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::NONE, Vec2::new(RES_WIDTH as f32, RES_HEIGHT as f32)),
        Transform::from_xyz(0.0, 0.0, Z_SHADE),
        Visibility::Hidden,
        Shade,
        WORLD_LAYER,
    ));
}

/// The tint for each part of the day: color and strength.
fn tint(phase: Phase) -> Option<(Srgba, f32)> {
    match phase {
        Phase::Day => None,
        Phase::Dawn => Some((Srgba::rgb_u8(248, 160, 96), 0.18)),
        Phase::Dusk => Some((Srgba::rgb_u8(200, 96, 72), 0.25)),
        Phase::Night => Some((Srgba::rgb_u8(24, 24, 72), 0.45)),
    }
}

fn shade_world(
    factory: Res<Factory>,
    settings: Option<Res<super::ui::settings::Settings>>,
    mut clear: ResMut<ClearColor>,
    shade: Single<(&mut Sprite, &mut Visibility), With<Shade>>,
    mut shown: Local<Option<Option<Phase>>>,
) {
    let enabled = settings.is_none_or(|s| s.day_night);
    let phase = enabled.then(|| daynight::phase(factory.ticks));
    if *shown == Some(phase) {
        return;
    }
    *shown = Some(phase);
    let (mut sprite, mut visibility) = shade.into_inner();
    match phase.and_then(tint) {
        Some((color, alpha)) => {
            sprite.color = color.with_alpha(alpha).into();
            *visibility = Visibility::Inherited;
            // The GPU blends the tint over the canvas in linear color, so
            // mix the sky around it the same way or the two would not match.
            let sky = palette::SKY.to_linear();
            let tint = LinearRgba::from(color);
            clear.0 = LinearRgba::rgb(
                sky.red + (tint.red - sky.red) * alpha,
                sky.green + (tint.green - sky.green) * alpha,
                sky.blue + (tint.blue - sky.blue) * alpha,
            )
            .into();
        }
        None => {
            *visibility = Visibility::Hidden;
            clear.0 = palette::SKY;
        }
    }
}
