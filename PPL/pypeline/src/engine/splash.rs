//! The loading screen: gold PYPELINE lettering over a starry sky with the
//! boot chime and a filling LOADING bar (roadmap Part 1 BOOT SPLASH). Comes
//! after the KiloKilo Games logo and leads to the title menu. Under 3
//! seconds, skippable with any key or click. Original design: no logo drop,
//! no console startup sound.

use bevy::prelude::*;

use super::camera::{RES_HEIGHT, RES_WIDTH, WORLD_LAYER};
use super::screens::Screen;
use super::sprites::{grid, to_image};
use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;

const DURATION: f32 = 2.6;
const FADE: f32 = 0.6;
const Z_SPLASH: f32 = 50.0;
const TITLE_SCALE: f32 = 4.0;
/// The loading bar's inside, in canvas pixels.
const BAR_WIDTH: f32 = 96.0;
const BAR_HEIGHT: f32 = 4.0;
const BAR_Y: f32 = -44.0;

/// 5x7 pixel letters for the title.
fn letter(c: char) -> [&'static str; 7] {
    match c {
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'I' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
        ],
        'N' => [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
        ],
        _ => ["....."; 7],
    }
}

/// The title as a sprite grid: gold letters with a dark drop shadow.
/// `y` is light gold, `Y` darker gold and `k` the shadow.
pub fn title_grid(text: &str) -> Vec<String> {
    let width = text.len() * 6;
    let mut rows = vec![vec!['.'; width + 1]; 8];
    for (i, c) in text.chars().enumerate() {
        for (y, row) in letter(c).iter().enumerate() {
            for (x, px) in row.chars().enumerate() {
                if px == '#' {
                    let gx = i * 6 + x;
                    rows[y + 1][gx + 1] = 'k';
                    rows[y][gx] = 'y';
                }
            }
        }
    }
    // Shadow pixels drawn after could cover gold; put gold back on top.
    for (i, c) in text.chars().enumerate() {
        for (y, row) in letter(c).iter().enumerate() {
            for (x, px) in row.chars().enumerate() {
                if px == '#' {
                    rows[y][i * 6 + x] = if y < 3 { 'y' } else { 'Y' };
                }
            }
        }
    }
    rows.into_iter().map(|r| r.into_iter().collect()).collect()
}

#[derive(Resource)]
pub struct Splash {
    remaining: f32,
}

#[derive(Component)]
struct SplashPart {
    alpha: f32,
}

/// The part of the loading bar that fills up.
#[derive(Component)]
struct BarFill;

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Boot), show_splash)
            .add_systems(OnExit(Screen::Boot), hide_splash)
            .add_systems(
                Update,
                run_splash.run_if(in_state(Screen::Boot).and_then(resource_exists::<Splash>)),
            );
    }
}

fn show_splash(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sounds: MessageWriter<SoundCue>,
) {
    commands.insert_resource(Splash {
        remaining: DURATION,
    });
    sounds.write(SoundCue(Sfx::Boot));

    let mut part = |sprite: Sprite, at: Vec3| {
        let alpha = sprite.color.alpha();
        commands.spawn((
            sprite,
            Transform::from_translation(at),
            SplashPart { alpha },
            WORLD_LAYER,
        ));
    };
    part(
        Sprite::from_color(
            Color::srgb_u8(24, 24, 56),
            Vec2::new(RES_WIDTH as f32, RES_HEIGHT as f32),
        ),
        Vec3::new(0.0, 0.0, Z_SPLASH),
    );
    // Stars at fixed spots from a tiny generator, so they never change.
    let mut seed: u32 = 7;
    for _ in 0..48 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let x = (seed >> 8) % RES_WIDTH;
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let y = (seed >> 8) % RES_HEIGHT;
        let bright = (seed >> 4).is_multiple_of(3);
        let size = if bright { 2.0 } else { 1.0 };
        let color = if bright {
            Color::srgb_u8(248, 240, 200)
        } else {
            Color::srgb_u8(160, 168, 216)
        };
        let half = size / 2.0;
        part(
            Sprite::from_color(color, Vec2::splat(size)),
            Vec3::new(
                x as f32 - RES_WIDTH as f32 / 2.0 + half,
                y as f32 - RES_HEIGHT as f32 / 2.0 + half,
                Z_SPLASH + 0.1,
            ),
        );
    }
    let rows = title_grid("PYPELINE");
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let image = to_image(&grid(&refs));
    let size = Vec2::new(image.width() as f32, image.height() as f32) * TITLE_SCALE;
    let mut title = Sprite::from_image(images.add(image));
    title.custom_size = Some(size);
    // Centered, with edges on whole pixels.
    let at = Vec3::new(
        if (size.x as u32).is_multiple_of(2) {
            0.0
        } else {
            0.5
        },
        if (size.y as u32).is_multiple_of(2) {
            8.0
        } else {
            8.5
        },
        Z_SPLASH + 0.2,
    );
    part(title, at);

    let rows = title_grid("LOADING");
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let image = to_image(&grid(&refs));
    let size = Vec2::new(image.width() as f32, image.height() as f32);
    let mut label = Sprite::from_image(images.add(image));
    label.custom_size = Some(size);
    part(
        label,
        Vec3::new(
            if (size.x as u32).is_multiple_of(2) {
                0.0
            } else {
                0.5
            },
            -30.0,
            Z_SPLASH + 0.2,
        ),
    );
    // The bar: a dark frame with a gold fill growing from the left.
    let frame = Vec2::new(BAR_WIDTH + 4.0, BAR_HEIGHT + 4.0);
    part(
        Sprite::from_color(Color::srgb_u8(224, 168, 56), frame),
        Vec3::new(0.0, BAR_Y, Z_SPLASH + 0.2),
    );
    part(
        Sprite::from_color(
            Color::srgb_u8(40, 32, 48),
            Vec2::new(BAR_WIDTH + 2.0, BAR_HEIGHT + 2.0),
        ),
        Vec3::new(0.0, BAR_Y, Z_SPLASH + 0.3),
    );
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(248, 216, 96), Vec2::new(0.0, BAR_HEIGHT)),
        Transform::from_xyz(-BAR_WIDTH / 2.0, BAR_Y, Z_SPLASH + 0.4),
        SplashPart { alpha: 1.0 },
        BarFill,
        WORLD_LAYER,
    ));
}

/// How full the loading bar is, 0..1, with `remaining` seconds to go. It
/// fills in whole pixels and is full when the fade starts.
fn bar_width(remaining: f32) -> f32 {
    let done = ((DURATION - remaining) / (DURATION - FADE)).clamp(0.0, 1.0);
    (done * BAR_WIDTH).round()
}

fn hide_splash(mut commands: Commands, parts: Query<Entity, With<SplashPart>>) {
    for entity in &parts {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<Splash>();
}

fn run_splash(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut splash: ResMut<Splash>,
    mut parts: Query<(&SplashPart, &mut Sprite, &mut Transform, Has<BarFill>)>,
    mut next: ResMut<NextState<Screen>>,
) {
    // Capped like the logo's clock, so a slow first frame cannot skip it.
    splash.remaining -= time.delta_secs().min(0.05);
    let skipped =
        keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some();
    if splash.remaining <= 0.0 || skipped {
        next.set(Screen::Menu);
        return;
    }
    let fade = (splash.remaining / FADE).min(1.0);
    let width = bar_width(splash.remaining);
    for (part, mut sprite, mut transform, is_bar) in &mut parts {
        sprite.color.set_alpha(part.alpha * fade);
        if is_bar {
            sprite.custom_size = Some(Vec2::new(width, BAR_HEIGHT));
            transform.translation.x = -BAR_WIDTH / 2.0 + width / 2.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_well_formed() {
        let rows = title_grid("PYPELINE");
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|r| r.len() == rows[0].len()));
        assert!(rows.iter().any(|r| r.contains('y')));
        // Every letter of LOADING is drawn.
        for c in "LOADING".chars() {
            assert!(letter(c).iter().any(|row| row.contains('#')), "{c}");
        }
    }

    #[test]
    fn the_bar_fills_before_the_fade() {
        assert_eq!(bar_width(DURATION), 0.0);
        assert_eq!(bar_width(FADE), BAR_WIDTH);
        assert!(bar_width(DURATION / 2.0) > 0.0);
    }
}
