//! Instant WAV preview: the Piece rendered by simple pure-Rust voices.
//! A sketch by contract (PRD §8) — it exists so hearing a composition
//! needs zero external tools, not to compete with a produced track.
//! Float DSP is not golden-tested; determinism lives in the MIDI bytes.

// fundsp's graph DSL leans on `gain * unit >> filter` chains where `*`
// binding tighter than `>>` is exactly the idiom — parenthesizing every
// chain buries the signal path in noise.
#![allow(clippy::precedence)]

use std::path::Path;

use fundsp::hacker::*;
use ns_core::Piece;

const SAMPLE_RATE: f64 = 44_100.0;
const TAIL_SECONDS: f64 = 1.5;
const PEAK_TARGET: f32 = 0.89;

#[derive(Debug)]
pub enum SynthError {
    Io(String),
    /// The piece renders to nothing (no notes).
    Silent,
}

impl std::fmt::Display for SynthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SynthError::Io(e) => write!(f, "writing wav: {e}"),
            SynthError::Silent => write!(f, "nothing to render — the piece has no notes"),
        }
    }
}

impl std::error::Error for SynthError {}

fn midi_hz(pitch: u8) -> f32 {
    (440.0 * 2f64.powf((f64::from(pitch) - 69.0) / 12.0)) as f32
}

fn gain_for(velocity: u8) -> f32 {
    let v = f32::from(velocity) / 127.0;
    v * v
}

/// (unit, fade_in, fade_out) per melodic family voice.
fn melodic_voice(track: &str, pitch: u8, velocity: u8) -> (Box<dyn AudioUnit>, f64, f64) {
    let f = midi_hz(pitch);
    let g = gain_for(velocity);
    match track {
        "strings" => {
            // Detuned saw pair under a gentle lowpass: a soft pad.
            let unit = (saw_hz(f) * 0.5 + saw_hz(f * 1.004) * 0.5)
                >> lowpass_hz(1400.0, 0.8);
            (Box::new(unit * (0.16 * g) >> pan(-0.3)), 0.09, 0.35)
        }
        "woodwinds" => {
            let unit = sine_hz(f) * 0.85 + triangle_hz(f) * 0.15;
            (Box::new(unit * (0.20 * g) >> pan(0.3)), 0.06, 0.22)
        }
        _ => {
            // Piano-ish: three partials with an exponential decay baked in.
            let unit = (sine_hz(f) * 0.55 + sine_hz(f * 2.0) * 0.28 + sine_hz(f * 3.0) * 0.12)
                * envelope(|t| (-2.2 * t).exp());
            (Box::new(unit * (0.24 * g) >> pan(0.0)), 0.004, 0.25)
        }
    }
}

/// (unit, fade_in, fade_out, forced_duration) per drum hit.
fn drum_voice(pitch: u8, velocity: u8) -> (Box<dyn AudioUnit>, f64, f64, f64) {
    let g = gain_for(velocity);
    match pitch {
        36 => (
            Box::new(sine_hz(55.0) * envelope(|t| (-22.0 * t).exp()) * (0.9 * g) >> pan(0.0)),
            0.001,
            0.05,
            0.30,
        ),
        38 => (
            Box::new(
                ((noise() >> bandpass_hz(1900.0, 1.0)) * envelope(|t| (-16.0 * t).exp()) * 0.7
                    + sine_hz(185.0) * envelope(|t| (-28.0 * t).exp()) * 0.35)
                    * (0.8 * g)
                    >> pan(-0.08),
            ),
            0.001,
            0.06,
            0.30,
        ),
        46 => (
            Box::new(
                (noise() >> highpass_hz(6200.0, 0.7)) * envelope(|t| (-7.0 * t).exp())
                    * (0.35 * g)
                    >> pan(0.12),
            ),
            0.001,
            0.10,
            0.45,
        ),
        51 => (
            Box::new(
                (noise() >> highpass_hz(8000.0, 0.6)) * envelope(|t| (-4.5 * t).exp())
                    * (0.28 * g)
                    >> pan(0.18),
            ),
            0.001,
            0.15,
            0.70,
        ),
        45 => (
            Box::new(sine_hz(110.0) * envelope(|t| (-11.0 * t).exp()) * (0.7 * g) >> pan(-0.15)),
            0.001,
            0.06,
            0.35,
        ),
        49 => (
            Box::new(
                (noise() >> highpass_hz(4200.0, 0.5)) * envelope(|t| (-2.8 * t).exp())
                    * (0.30 * g)
                    >> pan(0.2),
            ),
            0.001,
            0.30,
            1.10,
        ),
        _ => (
            // 42 closed hat and anything unexpected.
            Box::new(
                (noise() >> highpass_hz(6000.0, 0.7)) * envelope(|t| (-32.0 * t).exp())
                    * (0.32 * g)
                    >> pan(0.10),
            ),
            0.001,
            0.04,
            0.15,
        ),
    }
}

pub fn render_wav(piece: &Piece, path: &Path) -> Result<(), SynthError> {
    if piece.note_count() == 0 {
        return Err(SynthError::Silent);
    }
    let tick_secs = 60.0 / (f64::from(piece.bpm) * f64::from(piece.ticks_per_beat));

    let mut seq = Sequencer::new(false, 2);
    for track in &piece.tracks {
        let drums = track.channel == 9;
        for n in &track.notes {
            let start = f64::from(n.start) * tick_secs;
            if drums {
                let (unit, fi, fo, dur) = drum_voice(n.pitch, n.velocity);
                seq.push_duration(start, dur, Fade::Smooth, fi, fo, unit);
            } else {
                let (unit, fi, fo) = melodic_voice(track.name, n.pitch, n.velocity);
                let dur = (f64::from(n.dur) * tick_secs).max(0.05);
                seq.push_duration(start, dur + fo, Fade::Smooth, fi, fo, unit);
            }
        }
    }

    let duration = piece.len_seconds() + TAIL_SECONDS;
    let mut wave = Wave::render(SAMPLE_RATE, duration, &mut seq);

    // Normalize to a healthy peak so quiet graphs aren't inaudible and
    // dense ones don't clip.
    let peak = wave.amplitude();
    if peak > 0.0 {
        let scale = PEAK_TARGET / peak;
        for ch in 0..wave.channels() {
            for i in 0..wave.len() {
                wave.set(ch, i, wave.at(ch, i) * scale);
            }
        }
    }
    wave.save_wav16(path).map_err(|e| SynthError::Io(e.to_string()))
}
