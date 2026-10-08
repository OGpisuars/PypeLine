//! A tiny chiptune synthesizer: square, triangle and noise voices rendered to
//! in-memory WAV files. Every sound in the game is generated here, so there
//! are no audio files to ship or license.

/// Samples per second. Plenty for GBA-style sound.
pub const SAMPLE_RATE: u32 = 22_050;

#[derive(Debug, Clone, Copy)]
pub enum Wave {
    /// Pulse wave with the given duty cycle (0.5 = square).
    Pulse(f32),
    Triangle,
    Noise,
}

/// One note: MIDI pitch (None = rest), length in seconds, wave, volume.
#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub pitch: Option<u8>,
    pub secs: f32,
    pub wave: Wave,
    pub volume: f32,
}

pub fn note(pitch: u8, secs: f32, wave: Wave, volume: f32) -> Note {
    Note {
        pitch: Some(pitch),
        secs,
        wave,
        volume,
    }
}

pub fn rest(secs: f32) -> Note {
    Note {
        pitch: None,
        secs,
        wave: Wave::Triangle,
        volume: 0.0,
    }
}

fn frequency(pitch: u8) -> f32 {
    440.0 * 2f32.powf((pitch as f32 - 69.0) / 12.0)
}

/// Render notes one after another into samples in -1..1.
pub fn render(notes: &[Note]) -> Vec<f32> {
    let mut out = Vec::new();
    // Small deterministic noise generator (an LCG), so sounds never change.
    let mut seed: u32 = 0x1234_5678;
    for n in notes {
        let len = (n.secs * SAMPLE_RATE as f32) as usize;
        let Some(pitch) = n.pitch else {
            out.extend(std::iter::repeat_n(0.0, len));
            continue;
        };
        let freq = frequency(pitch);
        for i in 0..len {
            let t = i as f32 / SAMPLE_RATE as f32;
            let phase = (t * freq).fract();
            let raw = match n.wave {
                Wave::Pulse(duty) => {
                    if phase < duty {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Noise => {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 16) as f32 / 32_768.0 - 1.0
                }
            };
            // Quick attack, gentle release, so notes do not click.
            let attack = (i as f32 / (0.005 * SAMPLE_RATE as f32)).min(1.0);
            let release = ((len - i) as f32 / (0.03 * SAMPLE_RATE as f32)).min(1.0);
            out.push(raw * n.volume * attack * release);
        }
    }
    out
}

/// Add `b` into `a`, sample by sample (for chords and multi-voice music).
pub fn mix(mut a: Vec<f32>, b: &[f32]) -> Vec<f32> {
    if a.len() < b.len() {
        a.resize(b.len(), 0.0);
    }
    for (x, y) in a.iter_mut().zip(b) {
        *x += y;
    }
    a
}

/// Encode samples as a 16-bit mono WAV file.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

const SQ: Wave = Wave::Pulse(0.5);
const THIN: Wave = Wave::Pulse(0.125);

/// The game's sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    /// Original boot chime (not any console's startup sound).
    Boot,
    Run,
    Error,
    Overheat,
    Sale,
    TrainWhistle,
}

impl Sfx {
    pub const ALL: [Self; 6] = [
        Self::Boot,
        Self::Run,
        Self::Error,
        Self::Overheat,
        Self::Sale,
        Self::TrainWhistle,
    ];

    pub fn samples(self) -> Vec<f32> {
        match self {
            Self::Boot => {
                let lead = render(&[
                    note(72, 0.10, THIN, 0.22),
                    note(76, 0.10, THIN, 0.22),
                    note(79, 0.10, THIN, 0.22),
                    note(84, 0.45, THIN, 0.22),
                ]);
                let bass = render(&[rest(0.3), note(48, 0.45, Wave::Triangle, 0.3)]);
                mix(lead, &bass)
            }
            Self::Run => render(&[note(84, 0.03, SQ, 0.15), note(91, 0.04, SQ, 0.12)]),
            Self::Error => render(&[note(43, 0.12, SQ, 0.2), note(38, 0.22, SQ, 0.2)]),
            Self::Overheat => {
                let hiss = render(&[note(60, 0.6, Wave::Noise, 0.12)]);
                let whistle = render(&[
                    note(88, 0.15, Wave::Triangle, 0.25),
                    note(86, 0.15, Wave::Triangle, 0.25),
                    note(84, 0.3, Wave::Triangle, 0.25),
                ]);
                mix(hiss, &whistle)
            }
            Self::Sale => render(&[note(88, 0.06, THIN, 0.2), note(95, 0.18, THIN, 0.2)]),
            Self::TrainWhistle => {
                let a = render(&[
                    note(69, 0.35, Wave::Triangle, 0.22),
                    rest(0.05),
                    note(69, 0.5, Wave::Triangle, 0.22),
                ]);
                let b = render(&[
                    note(73, 0.35, Wave::Triangle, 0.18),
                    rest(0.05),
                    note(73, 0.5, Wave::Triangle, 0.18),
                ]);
                mix(a, &b)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_valid() {
        let bytes = wav(&render(&[note(69, 0.1, SQ, 0.5)]));
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 44 + data_len);
    }

    #[test]
    fn every_sound_is_short_and_never_clips() {
        for sfx in Sfx::ALL {
            let samples = sfx.samples();
            assert!(!samples.is_empty(), "{sfx:?}");
            assert!(
                samples.len() < SAMPLE_RATE as usize * 2,
                "{sfx:?} is too long"
            );
            assert!(samples.iter().all(|s| s.abs() <= 1.0), "{sfx:?} clips");
        }
    }
}
