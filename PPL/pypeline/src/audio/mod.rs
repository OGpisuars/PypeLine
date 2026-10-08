//! Sound: generated chiptune effects and a music loop.
//!
//! Systems ask for a sound by sending a `SoundCue`. M mutes everything (or the
//! speaker button on the Time Dials bar).

pub mod adaptive_music;
pub mod sfx;

use std::collections::HashMap;

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

use sfx::{Sfx, wav};

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
}

#[derive(Component)]
struct Music;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SoundCue>()
            .init_resource::<AudioSettings>()
            .add_systems(Startup, (build_sounds, start_music).chain())
            .add_systems(Update, play_cues)
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
    commands.insert_resource(Sounds { sfx });
}

fn start_music(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let music = sources.add(source(&adaptive_music::music_loop()));
    commands.spawn((
        AudioPlayer(music),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.6)),
        Music,
    ));
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
