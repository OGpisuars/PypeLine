//! Sound: generated chiptune effects and the background music.
//!
//! Systems ask for a sound by sending a `SoundCue`. M mutes everything (or the
//! speaker button on the Time Dials bar). The music track and its volume are
//! picked in Settings; it fades in on the title menu and keeps playing in
//! the game. The logo and loading screens play only their own jingles.

pub mod adaptive_music;
pub mod sfx;

use std::collections::HashMap;

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::engine::screens::Screen;
use crate::engine::ui::settings::Settings;
use sfx::{Sfx, wav};

/// The background music to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MusicChoice {
    #[default]
    JoystickSunday,
    /// The little generated chiptune loop (`adaptive_music.rs`).
    Chiptune,
    Off,
}

impl MusicChoice {
    pub const ALL: [Self; 3] = [Self::JoystickSunday, Self::Chiptune, Self::Off];

    pub fn name(self) -> &'static str {
        match self {
            Self::JoystickSunday => "Joystick Sunday",
            Self::Chiptune => "Chiptune loop",
            Self::Off => "Off",
        }
    }
}

/// Seconds the music takes to fade in.
const MUSIC_FADE_IN: f32 = 2.0;

/// Ask for a sound effect to play.
#[derive(Message, Debug, Clone, Copy)]
pub struct SoundCue(pub Sfx);

#[derive(Resource, Debug, Default)]
pub struct AudioSettings {
    pub muted: bool,
}

#[derive(Resource)]
struct Sounds {
    sfx: HashMap<Sfx, Handle<AudioSource>>,
    music: HashMap<MusicChoice, Handle<AudioSource>>,
}

/// The music playing, and how far it has faded in (0..1).
#[derive(Component)]
struct Music {
    track: MusicChoice,
    fade: f32,
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SoundCue>()
            .init_resource::<AudioSettings>()
            .add_systems(Startup, build_sounds)
            .add_systems(Update, (play_cues, play_music))
            // In the egui pass, so it can tell whether you are typing.
            .add_systems(bevy_egui::EguiPrimaryContextPass, apply_mute);
    }
}

fn source(samples: &[f32]) -> AudioSource {
    AudioSource {
        bytes: wav(samples).into(),
    }
}

fn build_sounds(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let sfx = Sfx::ALL
        .into_iter()
        .map(|s| (s, sources.add(source(&s.samples()))))
        .collect();
    let music = [
        (
            MusicChoice::JoystickSunday,
            sources.add(AudioSource {
                bytes: include_bytes!("../../assets/audio/music/joystick_sunday.ogg")
                    .as_slice()
                    .into(),
            }),
        ),
        (
            MusicChoice::Chiptune,
            sources.add(source(&adaptive_music::music_loop())),
        ),
    ]
    .into_iter()
    .collect();
    commands.insert_resource(Sounds { sfx, music });
}

/// Start, switch, fade in and set the volume of the background music.
fn play_music(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    screen: Res<State<Screen>>,
    mut playing: Query<(Entity, &mut Music, Option<&mut AudioSink>)>,
) {
    let (Some(sounds), Some(settings)) = (sounds, settings) else {
        return;
    };
    let wanted = match screen.get() {
        Screen::Logo | Screen::Boot => MusicChoice::Off,
        Screen::Menu | Screen::Playing => settings.music,
    };
    for (entity, mut music, sink) in &mut playing {
        if music.track != wanted {
            commands.entity(entity).despawn();
            continue;
        }
        music.fade = (music.fade + time.delta_secs() / MUSIC_FADE_IN).min(1.0);
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(settings.music_volume * music.fade));
        }
    }
    let already = playing.iter().any(|(_, music, _)| music.track == wanted);
    if !already && let Some(handle) = sounds.music.get(&wanted) {
        commands.spawn((
            AudioPlayer(handle.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
            Music {
                track: wanted,
                fade: 0.0,
            },
        ));
    }
}

fn play_cues(
    mut commands: Commands,
    mut cues: MessageReader<SoundCue>,
    sounds: Option<Res<Sounds>>,
    settings: Res<AudioSettings>,
) {
    let Some(sounds) = sounds else { return };
    for SoundCue(sfx) in cues.read() {
        if settings.muted {
            continue;
        }
        if let Some(handle) = sounds.sfx.get(sfx) {
            commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

fn apply_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut contexts: bevy_egui::EguiContexts,
    mut settings: ResMut<AudioSettings>,
    mut music: Query<&mut AudioSink, With<Music>>,
) {
    let typing = contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.egui_wants_keyboard_input());
    if keys.just_pressed(KeyCode::KeyM) && !typing {
        settings.muted = !settings.muted;
    }
    // Checked every frame: the music's sink only appears once it starts.
    for mut sink in &mut music {
        if sink.is_muted() != settings.muted {
            if settings.muted {
                sink.mute();
            } else {
                sink.unmute();
            }
        }
    }
}
