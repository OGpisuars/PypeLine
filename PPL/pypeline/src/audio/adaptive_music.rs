//! Background music: a short, cozy chiptune loop (C - G - Am - F).
//!
//! "Adaptive" layers tied to throughput come in Phase 4; for now it is one
//! gentle loop.

use super::sfx::{Note, Wave, mix, note, render, rest};

const BEAT: f32 = 60.0 / 96.0;
const EIGHTH: f32 = BEAT / 2.0;

/// Lead melody in eighth notes (0 = rest), four bars.
const LEAD: [u8; 32] = [
    72, 76, 79, 76, 74, 72, 74, 76, //
    74, 79, 74, 71, 72, 74, 0, 0, //
    72, 76, 81, 79, 76, 74, 72, 76, //
    77, 76, 74, 72, 71, 74, 72, 0,
];

/// Bass in quarter notes, four bars.
const BASS: [u8; 16] = [
    48, 48, 55, 48, //
    43, 43, 50, 43, //
    45, 45, 52, 45, //
    41, 41, 43, 43,
];

/// One loop of the music, as samples.
pub fn music_loop() -> Vec<f32> {
    let lead: Vec<Note> = LEAD
        .iter()
        .map(|&p| {
            if p == 0 {
                rest(EIGHTH)
            } else {
                note(p, EIGHTH, Wave::Pulse(0.25), 0.07)
            }
        })
        .collect();
    let bass: Vec<Note> = BASS
        .iter()
        .map(|&p| note(p, BEAT, Wave::Triangle, 0.12))
        .collect();
    mix(render(&lead), &render(&bass))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::sfx::SAMPLE_RATE;

    #[test]
    fn music_loops_cleanly() {
        let music = music_loop();
        let seconds = music.len() as f32 / SAMPLE_RATE as f32;
        assert!((seconds - 16.0 * BEAT).abs() < 0.05, "{seconds}");
        assert!(music.iter().all(|s| s.abs() <= 1.0));
    }
}
