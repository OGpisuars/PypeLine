//! Code windows are part of the world, like in The Farmer Was Replaced:
//! each one sits at a spot in world space and is drawn through an egui
//! layer transform that follows the camera. Dragging the view moves them
//! with the island; zooming makes them bigger or smaller with it; and they
//! stay exactly where they were left, even far off screen.
//!
//! Spots are canvas pixels from the canvas's top-left corner (y down), with
//! no limits. Inside its layer a window is laid out at `NATURAL_ZOOM`, so at
//! that zoom it is its normal size. `ui/editor.rs` reads where a dragged
//! window ended up and stores it here. Saved in `terminals.ron` next to the
//! settings, not in the factory save, so Clean Run never moves them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::egui;

use super::camera::CanvasRect;

/// The zoom at which code windows are their normal size; at 4x they are
/// twice as big, at 1x half.
pub const NATURAL_ZOOM: f32 = 2.0;

/// Seconds between saves while windows are being moved.
const SAVE_EVERY: f32 = 1.0;
/// Where main.py's window starts, and how far each next file cascades.
const FIRST_SPOT: Vec2 = Vec2::new(4.0, 4.0);
pub const CASCADE: Vec2 = Vec2::new(14.0, 14.0);

/// A code window's bounds inside its layer, as big as the world. egui
/// clips and limits a window to the screen rectangle by default, but
/// inside the world's transform that rectangle is somewhere else, and
/// windows away from it were cut down to thin strips.
pub const WORLD_BOUNDS: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(-1.0e6, -1.0e6), egui::pos2(1.0e6, 1.0e6));

/// The transform from a code window's layer to the screen: world spots
/// (scaled by `NATURAL_ZOOM`) to where that part of the world is drawn.
pub fn layer_transform(canvas: &CanvasRect) -> egui::emath::TSTransform {
    egui::emath::TSTransform::new(
        egui::vec2(canvas.top_left.x, canvas.top_left.y),
        canvas.zoom / NATURAL_ZOOM,
    )
}

/// A world spot in a code window's layer coordinates, and back.
pub fn to_layer(spot: Vec2) -> egui::Pos2 {
    egui::pos2(spot.x * NATURAL_ZOOM, spot.y * NATURAL_ZOOM)
}

pub fn from_layer(pos: egui::Pos2) -> Vec2 {
    Vec2::new(pos.x, pos.y) / NATURAL_ZOOM
}

/// Where each file's window is in the world, by file name: its top-left
/// corner in canvas pixels.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct TerminalSpots(pub BTreeMap<String, Vec2>);

impl TerminalSpots {
    /// Where main.py's window starts on a fresh profile.
    pub fn first(index: usize) -> Vec2 {
        FIRST_SPOT + CASCADE * index as f32
    }

    /// The new world spot of a window whose top-left corner is now at
    /// `corner` (layer coordinates), or None if it has not moved, so
    /// unchanged windows do not count as changes and are not saved.
    pub fn moved(&self, name: &str, corner: egui::Pos2) -> Option<Vec2> {
        let spot = from_layer(corner);
        let old = self.0.get(name).copied();
        let changed = old.is_none_or(|old| old.distance(spot) > 0.001);
        changed.then_some(spot)
    }

    fn path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("PypeLine").join("terminals.ron"))
    }

    fn load() -> Self {
        let spots: BTreeMap<String, (f32, f32)> = Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default();
        Self(
            spots
                .into_iter()
                .filter(|(_, (x, y))| x.is_finite() && y.is_finite())
                .map(|(name, (x, y))| (name, Vec2::new(x, y)))
                .collect(),
        )
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let spots: BTreeMap<&str, (f32, f32)> = self
            .0
            .iter()
            .map(|(name, spot)| (name.as_str(), (spot.x, spot.y)))
            .collect();
        let text = ron::ser::to_string_pretty(&spots, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        crate::scripting::files::write_atomic(&path, &text)
    }
}

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerminalSpots::load())
            // In Last, so it sees the AppExit sent when the window closes.
            .add_systems(Last, save_spots);
    }
}

/// Save at most once a second while windows move, and on the way out.
fn save_spots(
    spots: Res<TerminalSpots>,
    time: Res<Time<Real>>,
    mut exit: MessageReader<AppExit>,
    mut dirty: Local<bool>,
    mut since: Local<f32>,
    mut console: ResMut<crate::scripting::Console>,
) {
    if spots.is_changed() && !spots.is_added() {
        *dirty = true;
    }
    *since += time.delta_secs();
    let exiting = exit.read().count() > 0;
    if !*dirty || (*since < SAVE_EVERY && !exiting) {
        return;
    }
    *dirty = false;
    *since = 0.0;
    if let Err(err) = spots.save() {
        console.push(
            crate::scripting::ConsoleKind::Error,
            format!("Could not save where the code windows are: {err}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_stay_on_their_spot_of_the_world() {
        let spot = Vec2::new(-120.0, 30.0);
        let corner = to_layer(spot);
        assert_eq!(from_layer(corner), spot);
        let mut spots = TerminalSpots::default();
        spots.0.insert("main.py".into(), spot);
        assert_eq!(spots.moved("main.py", corner), None);
        // Wherever the view is, the window's corner is drawn on the same
        // point of the world as that spot.
        for view in [
            CanvasRect {
                top_left: Vec2::new(100.0, 50.0),
                zoom: 2.0,
            },
            CanvasRect {
                top_left: Vec2::new(-900.0, 400.0),
                zoom: 4.0,
            },
        ] {
            let on_screen = layer_transform(&view) * corner;
            assert_eq!(view.to_canvas(Vec2::new(on_screen.x, on_screen.y)), spot);
        }
    }
}
