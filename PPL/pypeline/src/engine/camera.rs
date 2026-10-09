//! Pixel-perfect camera.
//!
//! The world is rendered into a fixed 480x320 image (2x the GBA's 240x160).
//! A second camera shows that image scaled up by a WHOLE number, with
//! nearest-neighbor sampling. By default that is the largest scale that fits
//! the window; the player can drag the world around and zoom with the mouse
//! wheel to make room for their code windows (`ViewState`).

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, PrimaryEguiContext, egui};

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

/// The part of the window under the top bar, in logical pixels with y
/// growing downward (egui's coordinates). None until the UI has run.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct GameArea(pub Option<Rect>);

/// Where the 480x320 canvas is on screen: its top-left corner in logical
/// pixels (y down, egui's coordinates) and the zoom. Things pinned to the
/// world, like the code terminals, use it to follow the view.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CanvasRect {
    pub top_left: Vec2,
    pub zoom: f32,
}

impl Default for CanvasRect {
    fn default() -> Self {
        Self {
            top_left: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl CanvasRect {
    /// Screen position of a canvas pixel, counted from the canvas's
    /// top-left corner.
    pub fn to_screen(&self, canvas_px: Vec2) -> Vec2 {
        self.top_left + canvas_px * self.zoom
    }

    /// The canvas pixel (from the top-left corner) at a screen position.
    pub fn to_canvas(&self, screen: Vec2) -> Vec2 {
        (screen - self.top_left) / self.zoom
    }
}

/// How the player moved the view.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct ViewState {
    /// Whole-number zoom, or None for the largest that fits the game area.
    pub zoom: Option<u32>,
    /// Offset of the canvas center from the game area's center, in logical
    /// pixels.
    pub pan: Vec2,
}

/// Highest zoom the mouse wheel reaches.
pub const MAX_ZOOM: u32 = 8;
/// Wheel distance (in pixels, for touchpads) that counts as one notch.
const PIXELS_PER_NOTCH: f32 = 60.0;

/// A drag in progress: the cursor position last frame.
#[derive(Default)]
struct Drag {
    last: Option<Vec2>,
    wheel: f32,
}

/// Marks the sprite that shows the 480x320 canvas on screen.
#[derive(Component)]
struct CanvasSprite;

/// The plot tile under the mouse (None when off the plot or over a window).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct HoveredTile(pub Option<Pos>);

#[derive(Component)]
struct ScreenCamera;

pub struct PixelCameraPlugin;

impl Plugin for PixelCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PixelScale(1))
            .insert_resource(ClearColor(palette::SKY))
            .init_resource::<HoveredTile>()
            .init_resource::<GameArea>()
            .init_resource::<ViewState>()
            .init_resource::<CanvasRect>()
            .add_systems(Startup, setup_cameras)
            .add_systems(Update, fit_canvas)
            // These run in the egui pass so they can ask whether the mouse is
            // over a window.
            .add_systems(
                EguiPrimaryContextPass,
                (move_view, track_hovered_tile).chain(),
            );
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
    commands.spawn((Sprite::from_image(canvas), CanvasSprite, SCREEN_LAYER));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        ScreenCamera,
        PrimaryEguiContext,
        SCREEN_LAYER,
    ));
}

/// The largest whole-number scale at which the canvas fits `area`.
fn fitting_zoom(area: Rect) -> u32 {
    let fit_w = area.width() / RES_WIDTH as f32;
    let fit_h = area.height() / RES_HEIGHT as f32;
    (fit_w.min(fit_h).floor() as u32).max(1)
}

/// Scale the canvas by the chosen whole number (or the largest that fits),
/// place it where the player dragged it, with its edges on whole pixels.
fn fit_canvas(
    window: Single<&Window, With<PrimaryWindow>>,
    area: Res<GameArea>,
    view: Res<ViewState>,
    mut projection: Single<&mut Projection, With<ScreenCamera>>,
    mut canvas: Single<&mut Transform, With<CanvasSprite>>,
    mut scale: ResMut<PixelScale>,
    mut placed: ResMut<CanvasRect>,
) {
    let (w, h) = (window.width(), window.height());
    let area = area.0.unwrap_or(Rect::new(0.0, 0.0, w, h));
    let factor = view.zoom.unwrap_or_else(|| fitting_zoom(area));
    let f = factor as f32;
    if let Projection::Orthographic(ortho) = &mut **projection
        && ortho.scale != 1.0 / f
    {
        ortho.scale = 1.0 / f;
    }
    // The view is not held near the island: code windows can be parked
    // anywhere in the world, and Home comes back.
    let middle = area.center() + view.pan;
    // Top-left corner of the canvas on screen, on a whole logical pixel.
    let left = (middle.x - RES_WIDTH as f32 * f / 2.0).round();
    let top = (middle.y - RES_HEIGHT as f32 * f / 2.0).round();
    // Its center relative to the window center, in canvas pixels (world
    // units of the screen camera), with y pointing up.
    let center = Vec2::new(
        left + RES_WIDTH as f32 * f / 2.0 - w / 2.0,
        -(top + RES_HEIGHT as f32 * f / 2.0 - h / 2.0),
    ) / f;
    let wanted = center.extend(0.0);
    if canvas.translation != wanted {
        canvas.translation = wanted;
    }
    if scale.0 != factor {
        scale.0 = factor;
    }
    let now = CanvasRect {
        top_left: Vec2::new(left, top),
        zoom: f,
    };
    if *placed != now {
        *placed = now;
    }
}

/// Drag the world with the mouse (left button on empty space, or the middle
/// or right button anywhere outside a window), zoom with the wheel around the
/// cursor, and press Home to put everything back.
#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn move_view(
    window: Single<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut contexts: EguiContexts,
    area: Res<GameArea>,
    scale: Res<PixelScale>,
    mut view: ResMut<ViewState>,
    mut drag: Local<Drag>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let over_ui = ctx.is_pointer_over_egui() || ctx.egui_is_using_pointer();
    let cursor = window.cursor_position();
    let area_rect = area
        .0
        .unwrap_or(Rect::new(0.0, 0.0, window.width(), window.height()));

    if keys.just_pressed(KeyCode::Home) && !ctx.egui_wants_keyboard_input() {
        *view = ViewState::default();
    }

    let buttons_down = [MouseButton::Left, MouseButton::Middle, MouseButton::Right];
    if drag.last.is_none() && !over_ui && buttons.any_just_pressed(buttons_down) {
        drag.last = cursor;
    }
    if !buttons.any_pressed(buttons_down) {
        drag.last = None;
    }
    if let (Some(last), Some(now)) = (drag.last, cursor) {
        if now != last {
            let pan = view.pan + (now - last);
            if view.pan != pan {
                view.pan = pan;
            }
        }
        drag.last = Some(now);
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    let mut notches = 0.0;
    for event in wheel.read() {
        if over_ui {
            continue;
        }
        notches += match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => event.y,
            bevy::input::mouse::MouseScrollUnit::Pixel => event.y / PIXELS_PER_NOTCH,
        };
    }
    drag.wheel += notches;
    let steps = drag.wheel.trunc();
    if steps != 0.0 {
        drag.wheel -= steps;
        let old = scale.0;
        let new = (old as i32 + steps as i32).clamp(1, MAX_ZOOM.max(old) as i32) as u32;
        if new != old {
            // Keep the spot under the cursor where it is.
            let middle = area_rect.center() + view.pan;
            let anchor = cursor.unwrap_or(middle);
            let spot = (anchor - middle) / old as f32;
            let new_middle = anchor - spot * new as f32;
            view.zoom = Some(new);
            view.pan = new_middle - area_rect.center();
        }
    }
    Ok(())
}

fn track_hovered_tile(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<ScreenCamera>>,
    canvas: Single<&Transform, With<CanvasSprite>>,
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
        // From screen-camera space to canvas space; the plot also bobs.
        .map(|world| world - canvas.translation.truncate())
        .map(|world| world - Vec2::new(0.0, island.as_ref().map_or(0.0, |i| i.bob)))
        .and_then(world_to_plot);
    if hovered.0 != tile {
        hovered.0 = tile;
    }
    Ok(())
}
