//! Developer helpers, switched on with environment variables:
//!
//! - `PYPELINE_AUTORUN=1`: press Run on the player's scripts at startup.
//! - `PYPELINE_AUTORUN_AFTER=1.5`: press Run after that many seconds instead.
//! - `PYPELINE_SCREENSHOT=out.png`: save a screenshot, then quit.
//! - `PYPELINE_SCREENSHOT_AFTER=8`: seconds to wait before the screenshot
//!   (default 5).
//! - `PYPELINE_OPEN=help,shop,settings,manual,stats,debug`: open those windows at startup.
//! - `PYPELINE_RECORD=dir`: save numbered frames into `dir` (for the README
//!   GIF, see `tools/readme_gif.sh`), then quit. `PYPELINE_RECORD_FPS`
//!   (default 12) and `PYPELINE_RECORD_SECONDS` (default 8) set how many.
//!
//! Handy for checking visuals in automated runs. Does nothing otherwise.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::engine::ui::editor::Workspace;
use crate::scripting::PendingRun;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if let Some(after) = env_secs("PYPELINE_AUTORUN_AFTER") {
            app.insert_resource(AutorunAt(after))
                .add_systems(Update, autorun_later);
        } else if std::env::var_os("PYPELINE_AUTORUN").is_some() {
            app.add_systems(PostStartup, autorun);
        }
        if let Some(dir) = std::env::var_os("PYPELINE_RECORD") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).expect("create the PYPELINE_RECORD folder");
            app.insert_resource(Recording {
                dir,
                fps: env_secs("PYPELINE_RECORD_FPS").unwrap_or(12.0),
                seconds: env_secs("PYPELINE_RECORD_SECONDS").unwrap_or(8.0),
                frames: 0,
                started: None,
            })
            .add_systems(Update, record_then_quit);
        }
        if let Ok(list) = std::env::var("PYPELINE_OPEN") {
            app.insert_resource(OpenAtStart(list))
                .add_systems(Update, open_windows);
        }
        if let Some(path) = std::env::var_os("PYPELINE_SCREENSHOT") {
            let after = env_secs("PYPELINE_SCREENSHOT_AFTER").unwrap_or(5.0);
            app.insert_resource(ScreenshotPlan {
                path: path.into(),
                after,
                taken_at: None,
            })
            .add_systems(Update, screenshot_then_quit);
        }
    }
}

/// A number from an environment variable, like `PYPELINE_RECORD_FPS=12`.
fn env_secs(name: &str) -> Option<f32> {
    std::env::var(name).ok()?.parse().ok()
}

#[derive(Resource)]
struct AutorunAt(f32);

fn autorun_later(
    at: Res<AutorunAt>,
    time: Res<Time<Real>>,
    workspace: Res<Workspace>,
    mut pending: ResMut<PendingRun>,
    mut done: Local<bool>,
) {
    if !*done && time.elapsed_secs() >= at.0 {
        *done = true;
        pending.0 = Some(workspace.program());
    }
}

#[derive(Resource)]
struct Recording {
    dir: std::path::PathBuf,
    fps: f32,
    seconds: f32,
    frames: u32,
    /// When the first frame was taken.
    started: Option<f32>,
}

/// Save frames at a steady rate from one second in, then quit.
fn record_then_quit(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut rec: ResMut<Recording>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if now < 1.0 {
        return;
    }
    let started = *rec.started.get_or_insert(now);
    let total = (rec.fps * rec.seconds).round() as u32;
    if rec.frames >= total {
        // Give the last frames a moment to be written, then quit.
        if now >= started + rec.seconds + 1.5 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if now >= started + rec.frames as f32 / rec.fps {
        let path = rec.dir.join(format!("frame_{:05}.png", rec.frames));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        rec.frames += 1;
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
#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn open_windows(
    list: Res<OpenAtStart>,
    time: Res<Time<Real>>,
    mut done: Local<bool>,
    mut help: ResMut<crate::engine::ui::help::HelpState>,
    mut shop: ResMut<crate::engine::ui::shop::ShopWindow>,
    mut settings: ResMut<crate::engine::ui::settings::SettingsWindow>,
    mut manual: ResMut<crate::engine::ui::manual::ManualState>,
    mut stats: ResMut<crate::engine::ui::stats_panel::StatsWindow>,
    mut debugger: ResMut<crate::engine::ui::debugger::Debugger>,
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
            "stats" => stats.open = true,
            "debug" => debugger.requested = true,
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
