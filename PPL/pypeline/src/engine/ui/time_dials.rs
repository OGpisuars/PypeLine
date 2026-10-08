//! Time Dials toolbar: pause/play, step one tick, and 1x/2x/4x.
//!
//! Keys (ignored while typing in a text box): Space = pause/play,
//! period = step one tick (while paused), 1/2/3 = 1x/2x/4x.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::factory::tick::{SPEEDS, SimControl};

/// The dials, drawn inside the top bar.
pub fn time_dials_ui(
    ui: &mut egui::Ui,
    control: &mut SimControl,
    audio: &mut crate::audio::AudioSettings,
) {
    let label = if control.paused {
        "▶ Play"
    } else {
        "⏸ Pause"
    };
    if ui.button(label).on_hover_text("Space").clicked() {
        control.paused = !control.paused;
    }
    let step = ui
        .add_enabled(control.paused, egui::Button::new("Step"))
        .on_hover_text("Run exactly one tick (period key)");
    if step.clicked() {
        control.step_requested = true;
    }
    for (i, speed) in SPEEDS.into_iter().enumerate() {
        let selected = control.speed == speed;
        let button = ui
            .selectable_label(selected, format!("{speed}x"))
            .on_hover_text(format!("key {}", i + 1));
        if button.clicked() && !selected {
            control.speed = speed;
        }
    }
    let speaker = if audio.muted { "🔇" } else { "🔊" };
    if ui
        .button(speaker)
        .on_hover_text("Sound on/off (M)")
        .clicked()
    {
        audio.muted = !audio.muted;
    }
}

pub fn time_dial_keys(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut control: ResMut<SimControl>,
) -> Result {
    if contexts.ctx_mut()?.egui_wants_keyboard_input() {
        return Ok(());
    }
    if keys.just_pressed(KeyCode::Space) {
        control.paused = !control.paused;
    }
    if keys.just_pressed(KeyCode::Period) && control.paused {
        control.step_requested = true;
    }
    for (key, speed) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .zip(SPEEDS)
    {
        if keys.just_pressed(key) && control.speed != speed {
            control.speed = speed;
        }
    }
    Ok(())
}
