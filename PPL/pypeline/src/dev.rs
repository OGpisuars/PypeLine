//! Developer helpers, switched on with environment variables:
//!
//! - `PYPELINE_AUTORUN=1`: press Run on the player's scripts at startup.
//! - `PYPELINE_SCREENSHOT=out.png`: save a screenshot, then quit.
//! - `PYPELINE_SCREENSHOT_AFTER=8`: seconds to wait before the screenshot
//!   (default 5).
//! - `PYPELINE_OPEN=help,shop,settings,manual`: open those windows at startup.
//!
//! Handy for checking visuals in automated runs. Does nothing otherwise.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::engine::ui::editor::Workspace;
use crate::scripting::PendingRun;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("PYPELINE_AUTORUN").is_some() {
            app.add_systems(PostStartup, autorun);
        }
        if let Ok(list) = std::env::var("PYPELINE_OPEN") {
            app.insert_resource(OpenAtStart(list))
                .add_systems(Update, open_windows);
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

fn autorun(workspace: Res<Workspace>, mut pending: ResMut<PendingRun>) {
    pending.0 = Some(workspace.program());
}

#[derive(Resource)]
struct OpenAtStart(String);

/// Open the listed windows once, after the Manual has picked its own state.
fn open_windows(
    list: Res<OpenAtStart>,
    time: Res<Time<Real>>,
    mut done: Local<bool>,
    mut help: ResMut<crate::engine::ui::help::HelpState>,
    mut shop: ResMut<crate::engine::ui::shop::ShopWindow>,
    mut settings: ResMut<crate::engine::ui::settings::SettingsWindow>,
    mut manual: ResMut<crate::engine::ui::manual::ManualState>,
) {
    if *done || time.elapsed_secs() < 1.0 {
        return;
    }
    *done = true;
    for name in list.0.split(',') {
        match name.trim() {
            "help" => help.open = true,
            "shop" => shop.open = true,
            "settings" => settings.open = true,
            "manual" => manual.open = true,
            "-manual" => manual.open = false,
            _ => {}
        }
    }
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
