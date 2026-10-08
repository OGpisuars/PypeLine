//! Which screen the game is on. At startup: the KiloKilo Games logo, then
//! the PypeLine loading screen, then the title menu, and Play starts the
//! game. The factory only ticks while playing.

use bevy::prelude::*;

use crate::factory::SimSet;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Screen {
    /// The animated KiloKilo Games logo (`company_logo.rs`).
    #[default]
    Logo,
    /// The PypeLine loading screen (`splash.rs`).
    Boot,
    /// The title menu: Play, Settings, Quit (`ui/main_menu.rs`).
    Menu,
    Playing,
}

impl Screen {
    /// The screen to start on: `PYPELINE_SCREEN=logo|boot|menu|play`, or
    /// straight into the game when a dev helper wants the game running.
    fn at_start() -> Self {
        match std::env::var("PYPELINE_SCREEN").as_deref() {
            Ok("logo") => return Self::Logo,
            Ok("boot") => return Self::Boot,
            Ok("menu") => return Self::Menu,
            Ok("play") => return Self::Playing,
            _ => {}
        }
        if std::env::var_os("PYPELINE_AUTORUN").is_some()
            || std::env::var_os("PYPELINE_OPEN").is_some()
        {
            Self::Playing
        } else {
            Self::Logo
        }
    }
}

/// Run condition: the title menu or the game, where Settings can open.
pub fn menu_or_playing(screen: Res<State<Screen>>) -> bool {
    matches!(screen.get(), Screen::Menu | Screen::Playing)
}

pub struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(Screen::at_start()).configure_sets(
            FixedUpdate,
            (
                SimSet::Clock,
                SimSet::Scripts,
                SimSet::Factory,
                SimSet::Progress,
            )
                .distributive_run_if(in_state(Screen::Playing)),
        );
    }
}
