//! Draws the factory. Read-only: it never changes the simulation.
//!
//! Belts and machines are rebuilt whenever the layout changes; items are a
//! pool of sprites repositioned every frame from their belt progress.

use bevy::prelude::*;

use super::camera::{HoveredTile, WORLD_LAYER};
use super::grid::{TILE, plot_tile_center};
use super::palette;
use super::sprites::{self, SpriteSheet};
use crate::factory::conveyors::TILE_PROGRESS;
use crate::factory::{Dir, Factory};

/// Draw order inside the world.
const Z_BELT: f32 = 1.0;
const Z_ITEM: f32 = 2.0;
const Z_MACHINE: f32 = 3.0;
const Z_HOVER: f32 = 4.0;

/// Tint for machines with no power.
const UNPOWERED: Color = Color::srgb(0.55, 0.55, 0.62);

#[derive(Component)]
struct LayoutSprite;

#[derive(Component)]
struct BeltSprite(Dir);

#[derive(Component)]
struct ItemSprite;

#[derive(Component)]
struct HoverHighlight;

pub struct FactoryRenderPlugin;

impl Plugin for FactoryRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (sprites::build_sprite_sheet, spawn_hover_highlight),
        )
        .add_systems(
            Update,
            (sync_layout, animate_belts, sync_items, move_hover_highlight).chain(),
        );
    }
}

fn sync_layout(
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
        commands.spawn((
            Sprite::from_image(sheet.belt(belt.dir, 0)),
            Transform::from_translation(plot_tile_center(pos, Z_BELT)),
            BeltSprite(belt.dir),
            LayoutSprite,
            WORLD_LAYER,
        ));
    }

    for (name, machine) in &factory.machines {
        let center = plot_tile_center(machine.pos, Z_MACHINE);
        let mut sprite = Sprite::from_image(sheet.machine(machine.kind));
        if machine.kind.needs_power() && !factory.is_powered(name) {
            sprite.color = UNPOWERED;
        }
        commands.spawn((
            sprite,
            Transform::from_translation(center),
            LayoutSprite,
            WORLD_LAYER,
        ));

        // A small brass port on the side items come out of.
        if machine.kind.needs_power() {
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
    let frame = (factory.ticks % sprites::BELT_FRAMES as u64) as usize;
    for (belt, mut sprite) in &mut belts {
        sprite.image = sheet.belt(belt.0, frame);
    }
}

fn sync_items(
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
        ));
    }
}

fn spawn_hover_highlight(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.3), Vec2::splat(TILE as f32)),
        Transform::default(),
        Visibility::Hidden,
        HoverHighlight,
        WORLD_LAYER,
    ));
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
