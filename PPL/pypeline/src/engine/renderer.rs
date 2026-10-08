//! Draws the factory. Read-only: it never changes the simulation.
//!
//! Belts and machines are rebuilt whenever the layout changes; items are a
//! pool of sprites repositioned every frame from their belt progress.

use bevy::prelude::*;

use super::camera::RES_WIDTH;
use super::camera::{HoveredTile, WORLD_LAYER};
use super::floating_plot::Island;
use super::grid::{TILE, canvas_tile_center, plot_tile_center};
use super::palette;
use super::sprites::{self, SpriteSheet};
use crate::factory::conveyors::TILE_PROGRESS;
use crate::factory::machines::MachineKind;
use crate::factory::train::TRAIN_INTERVAL;
use crate::factory::{Dir, Factory};

/// Draw order inside the world.
const Z_BELT: f32 = 1.0;
const Z_ITEM: f32 = 2.0;
const Z_MACHINE: f32 = 3.0;
const Z_HOVER: f32 = 4.0;

/// Tint for machines with no power.
const UNPOWERED: Color = Color::srgb(0.55, 0.55, 0.62);
/// Tint for machines a script switched off.
const SWITCHED_OFF: Color = Color::srgb(0.4, 0.4, 0.48);

#[derive(Component)]
struct LayoutSprite;

/// A belt's direction, how many animation frames it moves per tick (faster
/// tiers scroll faster), and its normal tint.
#[derive(Component)]
pub struct BeltSprite {
    dir: Dir,
    frames_per_tick: u64,
    pub tint: Color,
}

/// Tint for faster belts: warm for tier 2, cool for tier 3.
fn belt_tint(tier: u8) -> Color {
    match tier {
        0 | 1 => Color::WHITE,
        2 => Color::srgb(1.0, 0.78, 0.72),
        _ => Color::srgb(0.72, 0.85, 1.0),
    }
}

/// A machine's sprite and its normal color (dimmed when unpowered).
#[derive(Component)]
pub struct MachineSprite {
    pub kind: MachineKind,
    pub base: Color,
    /// A steam generator that is too hot (it blinks and puffs steam).
    pub overheated: bool,
}

#[derive(Component)]
struct ItemSprite;

#[derive(Component)]
struct HoverHighlight;

/// One side of the outline around a blocked or starved machine.
#[derive(Component)]
struct GlowEdge;

/// Outline colors: blocked machines red, starved ones yellow.
const GLOW_BLOCKED: Color = Color::srgb_u8(248, 64, 48);
const GLOW_STARVED: Color = Color::srgb_u8(248, 208, 64);

/// One piece of the train; 0 is the locomotive at the front.
#[derive(Component)]
struct TrainPart(usize);

/// Canvas row the rail runs along, below the island.
const RAIL_ROW: i32 = 1;
const TRAIN_PARTS: usize = 3;

pub struct FactoryRenderPlugin;

impl Plugin for FactoryRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                sprites::build_sprite_sheet,
                (spawn_hover_highlight, spawn_rail_and_train),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                sync_layout,
                animate_belts,
                sync_items,
                move_hover_highlight,
                glow_bottlenecks,
                move_train,
            )
                .chain(),
        );
    }
}

fn sync_layout(
    island: Res<Island>,
    mut commands: Commands,
    factory: Res<Factory>,
    sheet: Res<SpriteSheet>,
    old: Query<Entity, With<LayoutSprite>>,
    mut drawn_version: Local<Option<u64>>,
) {
    if *drawn_version == Some(factory.layout_version) {
        return;
    }
    *drawn_version = Some(factory.layout_version);
    for entity in &old {
        commands.entity(entity).despawn();
    }

    for (&pos, belt) in &factory.conveyors {
        let mut sprite = Sprite::from_image(sheet.belt(belt.dir, 0));
        sprite.color = belt_tint(belt.tier);
        commands.spawn((
            sprite,
            Transform::from_translation(plot_tile_center(pos, Z_BELT)),
            BeltSprite {
                dir: belt.dir,
                frames_per_tick: u64::from(crate::factory::shop::belt_speed(belt.tier)),
                tint: belt_tint(belt.tier),
            },
            LayoutSprite,
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    }

    for (name, machine) in &factory.machines {
        let center = plot_tile_center(machine.pos, Z_MACHINE);
        let mut sprite = Sprite::from_image(sheet.machine(machine.kind));
        if machine.kind.needs_power() && !factory.is_powered(name) {
            sprite.color = UNPOWERED;
        }
        if !machine.enabled {
            sprite.color = SWITCHED_OFF;
        }
        let base = sprite.color;
        commands.spawn((
            sprite,
            Transform::from_translation(center),
            MachineSprite {
                kind: machine.kind,
                base,
                overheated: machine.overheated,
            },
            LayoutSprite,
            WORLD_LAYER,
            ChildOf(island.root),
        ));

        // Mk2 and Mk3 machines wear one or two copper rivets in a corner.
        for pip in 1..machine.tier {
            let at = Vec3::new(
                TILE as f32 / 2.0 - 2.0 - 3.0 * f32::from(pip - 1),
                TILE as f32 / 2.0 - 2.0,
                0.2,
            );
            commands.spawn((
                Sprite::from_color(palette::COPPER, Vec2::splat(2.0)),
                Transform::from_translation(center + at),
                LayoutSprite,
                WORLD_LAYER,
                ChildOf(island.root),
            ));
        }

        // A small brass port on the side items come out of.
        if machine.kind.has_output() {
            let (dx, dy) = machine.dir.offset();
            let half = TILE as f32 / 2.0 - 1.0;
            let size = if dx != 0 {
                Vec2::new(2.0, 6.0)
            } else {
                Vec2::new(6.0, 2.0)
            };
            commands.spawn((
                Sprite::from_color(palette::BRASS, size),
                Transform::from_translation(
                    center + Vec3::new(dx as f32 * half, dy as f32 * half, 0.1),
                ),
                LayoutSprite,
                WORLD_LAYER,
                ChildOf(island.root),
            ));
        }
    }
}

fn animate_belts(
    factory: Res<Factory>,
    sheet: Res<SpriteSheet>,
    mut belts: Query<(&BeltSprite, &mut Sprite)>,
) {
    // Driven by the factory's own tick, so belts freeze while halted.
    for (belt, mut sprite) in &mut belts {
        let frame = (factory.ticks * belt.frames_per_tick % sprites::BELT_FRAMES as u64) as usize;
        sprite.image = sheet.belt(belt.dir, frame);
    }
}

fn sync_items(
    island: Res<Island>,
    mut commands: Commands,
    factory: Res<Factory>,
    sheet: Res<SpriteSheet>,
    mut pool: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<ItemSprite>>,
) {
    let half_tile = TILE_PROGRESS as f32 / 2.0;
    let wanted: Vec<_> = factory
        .conveyors
        .iter()
        .flat_map(|(&pos, belt)| {
            let (dx, dy) = belt.dir.offset();
            belt.items.iter().map(move |item| {
                // From the back edge (progress 0) to the front edge (16).
                let along = item.progress as f32 - half_tile;
                let at = plot_tile_center(pos, Z_ITEM)
                    + Vec3::new(dx as f32 * along, dy as f32 * along, 0.0);
                (item.kind, at)
            })
        })
        .collect();

    let mut wanted = wanted.into_iter();
    for (mut sprite, mut transform, mut visibility) in &mut pool {
        match wanted.next() {
            Some((kind, at)) => {
                sprite.image = sheet.item(kind);
                transform.translation = at;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // Grow the pool for any items left over.
    for (kind, at) in wanted {
        commands.spawn((
            Sprite::from_image(sheet.item(kind)),
            Transform::from_translation(at),
            ItemSprite,
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    }
}

fn spawn_hover_highlight(island: Res<Island>, mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.3), Vec2::splat(TILE as f32)),
        Transform::default(),
        Visibility::Hidden,
        HoverHighlight,
        WORLD_LAYER,
        ChildOf(island.root),
    ));
}

/// While the Stats window asks for it, blink an outline around machines
/// that are blocked (red) or starved (yellow). Blinking, not fading, keeps
/// it pixel art.
fn glow_bottlenecks(
    island: Res<Island>,
    mut commands: Commands,
    factory: Res<Factory>,
    window: Option<Res<super::ui::stats_panel::StatsWindow>>,
    time: Res<Time<Real>>,
    mut pool: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<GlowEdge>>,
) {
    use crate::factory::stats::{MachineState, machine_state};
    let blink_on = ((time.elapsed_secs() * 2.0) as u64).is_multiple_of(2);
    let show = window.is_some_and(|w| w.glowing()) && blink_on;
    let half = TILE as f32 / 2.0 + 0.5;
    let full = TILE as f32 + 2.0;
    let sides = [
        (Vec2::new(0.0, half), Vec2::new(full, 1.0)),
        (Vec2::new(0.0, -half), Vec2::new(full, 1.0)),
        (Vec2::new(-half, 0.0), Vec2::new(1.0, TILE as f32)),
        (Vec2::new(half, 0.0), Vec2::new(1.0, TILE as f32)),
    ];
    let wanted: Vec<(Vec3, Vec2, Color)> = factory
        .machines
        .iter()
        .filter(|_| show)
        .filter_map(|(name, m)| {
            let color = match machine_state(&factory, name)? {
                MachineState::Blocked => GLOW_BLOCKED,
                MachineState::Starved => GLOW_STARVED,
                _ => return None,
            };
            Some((plot_tile_center(m.pos, Z_HOVER + 0.1), color))
        })
        .flat_map(|(center, color)| {
            sides.map(|(offset, size)| (center + offset.extend(0.0), size, color))
        })
        .collect();

    let mut wanted = wanted.into_iter();
    for (mut sprite, mut transform, mut visibility) in &mut pool {
        match wanted.next() {
            Some((at, size, color)) => {
                sprite.custom_size = Some(size);
                sprite.color = color;
                transform.translation = at;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (at, size, color) in wanted {
        commands.spawn((
            Sprite::from_color(color, size),
            Transform::from_translation(at),
            GlowEdge,
            WORLD_LAYER,
            ChildOf(island.root),
        ));
    }
}

fn move_hover_highlight(
    hovered: Res<HoveredTile>,
    highlight: Single<(&mut Transform, &mut Visibility), With<HoverHighlight>>,
) {
    let (mut transform, mut visibility) = highlight.into_inner();
    match hovered.0 {
        Some(pos) => {
            transform.translation = plot_tile_center(pos, Z_HOVER);
            *visibility = Visibility::Inherited;
        }
        None => *visibility = Visibility::Hidden,
    }
}

fn spawn_rail_and_train(mut commands: Commands, sheet: Res<SpriteSheet>) {
    let row = canvas_tile_center(0, RAIL_ROW, 0.5);
    let width = RES_WIDTH as f32;
    // Two rails and a sleeper every 4 pixels, all on whole pixels.
    for (dy, color) in [(-6.0, palette::OUTLINE), (-4.0, palette::METAL)] {
        commands.spawn((
            Sprite::from_color(color, Vec2::new(width, 1.0)),
            Transform::from_xyz(0.0, row.y + dy + 0.5, row.z),
            WORLD_LAYER,
        ));
    }
    for i in 0..(RES_WIDTH / 4) {
        commands.spawn((
            Sprite::from_color(palette::DIRT_DARK, Vec2::new(2.0, 3.0)),
            Transform::from_xyz(i as f32 * 4.0 - width / 2.0 + 1.0, row.y - 5.5, row.z - 0.1),
            WORLD_LAYER,
        ));
    }
    for part in 0..TRAIN_PARTS {
        let image = if part == 0 {
            sheet.locomotive.clone()
        } else {
            sheet.cargo_car.clone()
        };
        commands.spawn((
            Sprite::from_image(image),
            Transform::from_xyz(0.0, row.y, Z_MACHINE),
            Visibility::Hidden,
            TrainPart(part),
            WORLD_LAYER,
        ));
    }
}

/// Where the train's front is at factory tick `ticks`, if it is on screen.
/// It rolls in over 3 s, waits 2 s while the train buys (on the visit tick),
/// then rolls out. Computed from the sim clock, so it pauses with the factory.
pub fn train_front_x(ticks: u64) -> Option<f32> {
    const ROLL: i64 = 60;
    const WAIT: i64 = 20;
    let interval = TRAIN_INTERVAL as i64;
    let ticks = ticks as i64;
    let visit = ((ticks + interval / 2) / interval) * interval;
    if visit == 0 {
        return None;
    }
    let t = ticks - visit;
    let edge = RES_WIDTH as f32 / 2.0 + 3.0 * TILE as f32;
    let x = match t {
        _ if !(-(ROLL + WAIT)..=ROLL + WAIT).contains(&t) => return None,
        _ if t < -WAIT => -edge * (-(t + WAIT)) as f32 / ROLL as f32,
        _ if t <= WAIT => 0.0,
        _ => edge * (t - WAIT) as f32 / ROLL as f32,
    };
    Some(x.round())
}

fn move_train(
    mut sounds: MessageWriter<crate::audio::SoundCue>,
    mut was_visible: Local<bool>,
    factory: Res<Factory>,
    mut parts: Query<(&TrainPart, &mut Transform, &mut Visibility)>,
) {
    let front = train_front_x(factory.ticks);
    if front.is_some() && !*was_visible {
        sounds.write(crate::audio::SoundCue(crate::audio::sfx::Sfx::TrainWhistle));
    }
    *was_visible = front.is_some();
    for (part, mut transform, mut visibility) in &mut parts {
        match front {
            Some(x) => {
                transform.translation.x = x - part.0 as f32 * TILE as f32;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn train_stops_mid_screen_on_visit_ticks() {
        assert_eq!(train_front_x(0), None);
        assert_eq!(train_front_x(300), None);
        assert_eq!(train_front_x(TRAIN_INTERVAL), Some(0.0));
        // Rolling in from the left, then out to the right.
        assert!(train_front_x(TRAIN_INTERVAL - 50).unwrap() < 0.0);
        assert!(train_front_x(TRAIN_INTERVAL + 50).unwrap() > 0.0);
    }
}
