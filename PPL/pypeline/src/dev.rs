//! Developer helpers, switched on with environment variables:
//!
//! - `PYPELINE_AUTORUN=1`: press Run on the script in the editor at startup.
//! - `PYPELINE_SCREENSHOT=out.png`: save a screenshot, then quit.
//! - `PYPELINE_SCREENSHOT_AFTER=8`: seconds to wait before the screenshot
//!   (default 5).
//!
//! Handy for checking visuals in automated runs. Does nothing otherwise.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::engine::ui::editor::EditorState;
use crate::scripting::PendingRun;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("PYPELINE_AUTORUN").is_some() {
            app.add_systems(PostStartup, autorun);
        }
        if let Some(path) = std::env::var_os("PYPELINE_SCREENSHOT") {
            let after = std::env::var("PYPELINE_SCREENSHOT_AFTER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5.0);
            app.insert_resource(ScreenshotPlan {
                path: path.into(),
                after,
                taken_at: None,
            })
            .add_systems(Update, screenshot_then_quit);
        }
    }
}

#[derive(Resource)]
struct ScreenshotPlan {
    path: std::path::PathBuf,
    after: f32,
    taken_at: Option<f32>,
}

fn autorun(editor: Res<EditorState>, mut pending: ResMut<PendingRun>) {
    pending.0 = Some(editor.source.clone());
}

fn screenshot_then_quit(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut plan: ResMut<ScreenshotPlan>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    match plan.taken_at {
        None if now >= plan.after => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(plan.path.clone()));
            plan.taken_at = Some(now);
        }
        // Give the screenshot a moment to be written, then quit.
        Some(at) if now >= at + 1.0 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
