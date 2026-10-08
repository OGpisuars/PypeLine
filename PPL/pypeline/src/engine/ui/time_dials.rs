//! Time Dials toolbar: pause/play, step one tick, and 1x/2x/4x.
//!
//! Keys (ignored while typing in a text box): Space = pause/play,
//! period = step one tick (while paused), 1/2/3 = 1x/2x/4x.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::factory::tick::{SPEEDS, SimControl};

pub fn time_dials(
    mut contexts: EguiContexts,
    mut control: ResMut<SimControl>,
    mut audio: ResMut<crate::audio::AudioSettings>,
    area: Res<crate::engine::camera::GameArea>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("time_dials"))
        .pivot(egui::Align2::CENTER_TOP)
        .fixed_pos(area.0.map_or(egui::pos2(400.0, 8.0), |a| {
            egui::pos2(a.center().x, a.min.y + 8.0)
        }))
        .show(ctx, |ui| {
            egui::Frame::window(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
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
                    ui.separator();
                    for (i, speed) in SPEEDS.into_iter().enumerate() {
                        let selected = control.speed == speed;
                        let button = ui
                            .selectable_label(selected, format!("{speed}x"))
                            .on_hover_text(format!("key {}", i + 1));
                        if button.clicked() && !selected {
                            control.speed = speed;
                        }
                    }
                    ui.separator();
                    let speaker = if audio.muted { "🔇" } else { "🔊" };
                    if ui
                        .button(speaker)
                        .on_hover_text("Sound on/off (M)")
                        .clicked()
                    {
                        audio.muted = !audio.muted;
                    }
                });
            });
        });
    Ok(())
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
