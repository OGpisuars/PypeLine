//! Saving and loading the factory (roadmap Part 4 A: never lose player work).
//!
//! The save is human-readable RON with a version number from the very first
//! format, so later versions can migrate old saves. Writes are atomic, and the
//! last few saves are kept as rolling backups (`factory.1.ron` is the newest).

use std::fs;
use std::io;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::contracts::Progress;
use crate::factory::{Factory, SimTick};
use crate::scripting::files::write_atomic;
use crate::scripting::runtime::Program;
use crate::scripting::{Console, ConsoleKind, LastGoodScript};

/// Bump this when the save format changes, and add a migration in `load`.
pub const SAVE_VERSION: u32 = 1;
/// Older saves kept next to the current one.
const BACKUPS: u32 = 5;
/// Real seconds between autosaves.
const AUTOSAVE_SECS: f32 = 60.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub tick: u64,
    pub factory: Factory,
    /// Manual progress. Older saves have none, which loads as a fresh start.
    #[serde(default)]
    pub progress: Progress,
    /// The script that built this factory, so contracts can check it after
    /// a load without the player pressing Run first. Older saves have none.
    #[serde(default)]
    pub script: Program,
}

/// Set to save at the end of this frame instead of waiting for the next
/// autosave: after a contract completes, or when leaving to the title.
#[derive(Resource, Default)]
pub struct SaveNow(pub bool);

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Format(String),
    TooNew(u32),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Format(err) => write!(f, "the save file is damaged ({err})"),
            Self::TooNew(v) => write!(
                f,
                "the save is from a newer PypeLine (format {v}, this game reads up to {SAVE_VERSION})"
            ),
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct SaveStore {
    dir: PathBuf,
}

impl SaveStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn default_location() -> Option<Self> {
        dirs::data_dir().map(|data| Self::new(data.join("PypeLine").join("saves")))
    }

    fn path(&self, backup: u32) -> PathBuf {
        if backup == 0 {
            self.dir.join("factory.ron")
        } else {
            self.dir.join(format!("factory.{backup}.ron"))
        }
    }

    pub fn save(&self, save: &SaveFile) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let text = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
            .map_err(io::Error::other)?;
        // Rotate backups: factory.4 -> factory.5, ..., factory -> factory.1.
        for n in (0..BACKUPS).rev() {
            let from = self.path(n);
            if from.exists() {
                fs::rename(&from, self.path(n + 1))?;
            }
        }
        write_atomic(&self.path(0), &text)
    }

    /// Move a save that failed to load out of the way, so autosaves never
    /// overwrite it. The player (or a future fix) can still recover it.
    pub fn quarantine(&self) -> io::Result<PathBuf> {
        let to = self.dir.join("factory.damaged.ron");
        fs::rename(self.path(0), &to)?;
        Ok(to)
    }

    /// The saved game, or None if there is none yet.
    pub fn load(&self) -> Result<Option<SaveFile>, LoadError> {
        let text = match fs::read_to_string(self.path(0)) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(LoadError::Io(err)),
        };
        parse(&text).map(Some)
    }
}

fn parse(text: &str) -> Result<SaveFile, LoadError> {
    #[derive(Deserialize)]
    struct VersionOnly {
        version: u32,
    }
    let version = ron::from_str::<VersionOnly>(text)
        .map_err(|err| LoadError::Format(err.to_string()))?
        .version;
    if version > SAVE_VERSION {
        return Err(LoadError::TooNew(version));
    }
    // Migrations from older formats go here, one `if version == N` per step.
    let mut save: SaveFile =
        ron::from_str(text).map_err(|err| LoadError::Format(err.to_string()))?;
    // Make sure the renderer draws the loaded layout.
    save.factory.layout_version += 1;
    Ok(save)
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveNow>()
            .add_systems(Startup, load_game)
            .add_systems(Last, autosave_game);
    }
}

fn load_game(
    mut commands: Commands,
    mut factory: ResMut<Factory>,
    mut tick: ResMut<SimTick>,
    mut progress: ResMut<Progress>,
    mut last_good: ResMut<LastGoodScript>,
    mut console: ResMut<Console>,
) {
    let Some(store) = SaveStore::default_location() else {
        return;
    };
    match store.load() {
        Ok(Some(save)) => {
            *factory = save.factory;
            tick.0 = save.tick;
            *progress = save.progress;
            last_good.program = save.script;
            console.push(ConsoleKind::Info, "Factory loaded from your last session.");
        }
        Ok(None) => {}
        Err(err) => {
            let kept = match store.quarantine() {
                Ok(path) => format!("It was kept as {}.", path.display()),
                Err(_) => String::new(),
            };
            console.push(
                ConsoleKind::Error,
                format!("Could not load the factory: {err}. Starting fresh. {kept}"),
            );
        }
    }
    commands.insert_resource(store);
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn autosave_game(
    time: Res<Time<Real>>,
    mut since: Local<f32>,
    store: Option<Res<SaveStore>>,
    factory: Res<Factory>,
    tick: Res<SimTick>,
    progress: Res<Progress>,
    last_good: Res<LastGoodScript>,
    mut now: ResMut<SaveNow>,
    mut console: ResMut<Console>,
    mut exit: MessageReader<AppExit>,
) {
    let Some(store) = store else { return };
    *since += time.delta_secs();
    let exiting = exit.read().count() > 0;
    if !exiting && !now.0 && *since < AUTOSAVE_SECS {
        return;
    }
    *since = 0.0;
    now.0 = false;
    let save = SaveFile {
        version: SAVE_VERSION,
        tick: tick.0,
        factory: factory.clone(),
        progress: progress.clone(),
        script: last_good.program.clone(),
    };
    if let Err(err) = store.save(&save) {
        console.push(
            ConsoleKind::Error,
            format!("Could not save the factory: {err}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::CANONICAL_SAMPLE;
    use crate::scripting::budget::DEPLOY_BUDGET;
    use crate::scripting::reconcile;
    use crate::scripting::runtime::ScriptRuntime;

    fn sample_factory() -> Factory {
        let report = ScriptRuntime::new().run(CANONICAL_SAMPLE, DEPLOY_BUDGET);
        let mut factory = Factory::default();
        reconcile::apply(&mut factory, &report.plan.unwrap());
        for _ in 0..300 {
            factory.step();
        }
        factory
    }

    #[test]
    fn save_and_load_round_trip_with_backups() {
        let dir = std::env::temp_dir().join(format!("pypeline-saves-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        let factory = sample_factory();
        for tick in 0..3 {
            store
                .save(&SaveFile {
                    version: SAVE_VERSION,
                    tick,
                    factory: factory.clone(),
                    progress: Progress::default(),
                    script: Program::main_only(CANONICAL_SAMPLE),
                })
                .unwrap();
        }
        let mut loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.tick, 2);
        // Same state, only bumped so it gets drawn.
        loaded.factory.layout_version -= 1;
        assert_eq!(loaded.factory, factory);
        assert_eq!(loaded.script.main, CANONICAL_SAMPLE);
        assert!(store.path(2).exists() && !store.path(3).exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn newer_saves_are_refused_not_overwritten() {
        let text = "(version: 99, tick: 0, factory: ())";
        assert!(matches!(parse(text), Err(LoadError::TooNew(99))));
    }

    #[test]
    fn damaged_saves_are_reported() {
        assert!(matches!(parse("not ron at all"), Err(LoadError::Format(_))));
    }
}
