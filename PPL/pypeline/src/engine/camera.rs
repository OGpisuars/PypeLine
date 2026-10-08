//! Pixel-perfect camera.
//!
//! The world is rendered into a fixed 480x320 image (2x the GBA's 240x160).
//! A second camera shows that image scaled up by the largest WHOLE number that
//! fits the window, with nearest-neighbor sampling, and letterboxes the rest.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, PrimaryEguiContext};

use super::grid::world_to_plot;
use super::palette;
use crate::factory::Pos;

pub const RES_WIDTH: u32 = 480;
pub const RES_HEIGHT: u32 = 320;

/// Layer for everything drawn at game resolution.
pub const WORLD_LAYER: RenderLayers = RenderLayers::layer(0);
/// Layer for the upscaled canvas (and egui, which draws at window resolution).
pub const SCREEN_LAYER: RenderLayers = RenderLayers::layer(1);

/// Current integer upscale factor (1, 2, 3, ...). The UI reads this too.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PixelScale(pub u32);

/// The plot tile under the mouse (None when off the plot or over a window).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct HoveredTile(pub Option<Pos>);

#[derive(Component)]
struct ScreenCamera;

pub struct PixelCameraPlugin;

impl Plugin for PixelCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PixelScale(1))
            .insert_resource(ClearColor(palette::LETTERBOX))
            .init_resource::<HoveredTile>()
            .add_systems(Startup, setup_cameras)
            .add_systems(Update, fit_canvas)
            // Runs in the egui pass so it can ask whether the mouse is over a window.
            .add_systems(EguiPrimaryContextPass, track_hovered_tile);
    }
}

fn setup_cameras(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: RES_WIDTH,
        height: RES_HEIGHT,
        ..default()
    };
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("world_canvas"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    canvas.resize(size);
    let canvas = images.add(canvas);

    // Renders the world at 480x320.
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(palette::SKY),
            ..default()
        },
        RenderTarget::Image(canvas.clone().into()),
        Msaa::Off,
        WORLD_LAYER,
    ));

    // Shows the canvas on screen; egui draws on this camera too.
    commands.spawn((Sprite::from_image(canvas), SCREEN_LAYER));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        ScreenCamera,
        PrimaryEguiContext,
        SCREEN_LAYER,
    ));
}

fn fit_canvas(
    window: Single<&Window, With<PrimaryWindow>>,
    mut projection: Single<&mut Projection, With<ScreenCamera>>,
    mut scale: ResMut<PixelScale>,
) {
    let fit_w = window.width() / RES_WIDTH as f32;
    let fit_h = window.height() / RES_HEIGHT as f32;
    let factor = (fit_w.min(fit_h).floor() as u32).max(1);
    if let Projection::Orthographic(ortho) = &mut **projection {
        ortho.scale = 1.0 / factor as f32;
    }
    if scale.0 != factor {
        scale.0 = factor;
    }
}

fn track_hovered_tile(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<ScreenCamera>>,
    mut contexts: EguiContexts,
    mut hovered: ResMut<HoveredTile>,
    island: Option<Res<super::floating_plot::Island>>,
) -> Result {
    let over_ui = contexts.ctx_mut()?.is_pointer_over_egui();
    let (camera, transform) = *camera;
    // The canvas sprite is 480x320 world units at the origin, so the screen
    // camera's world space is the same as the game canvas's world space.
    let tile = window
        .cursor_position()
        .filter(|_| !over_ui)
        .and_then(|cursor| camera.viewport_to_world_2d(transform, cursor).ok())
        // The plot bobs with the island.
        .map(|world| world - Vec2::new(0.0, island.as_ref().map_or(0.0, |i| i.bob)))
        .and_then(world_to_plot);
    if hovered.0 != tile {
        hovered.0 = tile;
    }
    Ok(())
}
