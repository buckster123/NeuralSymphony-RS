//! Piece IR → Standard MIDI File bytes. Byte-deterministic: event order is
//! totally defined — (tick, offs-before-ons, pitch) — so the same IR always
//! serializes to the same bytes, which is what the golden tests pin.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use ns_core::Piece;

/// Largest delta a MIDI variable-length quantity can carry.
const MAX_DELTA: u32 = (1 << 28) - 1;

#[derive(Debug)]
pub enum RenderError {
    /// The piece violates MIDI ranges (render is a public API; it validates
    /// instead of letting midly's masking constructors silently corrupt
    /// bytes — adversarial-review finding).
    InvalidPiece(String),
    /// An inter-event gap exceeds what a MIDI delta can express (2^28−1
    /// ticks). midly's u28::new would silently fold the note ~97 hours
    /// early; erroring is the honest outcome.
    DeltaTooLarge(u32),
    /// midly refused the write — unreachable for pieces that pass validation.
    Write(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InvalidPiece(what) => write!(f, "invalid piece: {what}"),
            RenderError::DeltaTooLarge(d) => {
                write!(f, "event gap of {d} ticks exceeds MIDI's delta range (2^28-1)")
            }
            RenderError::Write(e) => write!(f, "midi write failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

fn validate(piece: &Piece) -> Result<(), RenderError> {
    let bad = |what: String| Err(RenderError::InvalidPiece(what));
    if piece.bpm == 0 {
        return bad("bpm must be >= 1".into());
    }
    if 60_000_000 / piece.bpm > 0x00FF_FFFF {
        return bad(format!("bpm {} too slow for SMF's 24-bit tempo", piece.bpm));
    }
    if piece.ticks_per_beat == 0 || piece.ticks_per_beat > 0x7FFF {
        return bad(format!("ticks_per_beat {} outside 1..=32767", piece.ticks_per_beat));
    }
    for t in &piece.tracks {
        if t.channel > 15 {
            return bad(format!("track {}: channel {} > 15", t.name, t.channel));
        }
        if let Some(p) = t.program {
            if p > 127 {
                return bad(format!("track {}: program {p} > 127", t.name));
            }
        }
        for n in &t.notes {
            if n.pitch > 127 || n.velocity > 127 {
                return bad(format!("track {}: pitch/velocity out of MIDI range", t.name));
            }
            if n.start.checked_add(n.dur).is_none() {
                return bad(format!("track {}: note end overflows tick space", t.name));
            }
        }
    }
    Ok(())
}

/// Absolute-time event, ordered so simultaneous offs precede ons (no
/// accidental retriggers) and ties break on pitch then velocity.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct AbsEvent {
    tick: u32,
    /// 0 = note off, 1 = note on.
    rank: u8,
    pitch: u8,
    velocity: u8,
}

pub fn render(piece: &Piece) -> Result<Vec<u8>, RenderError> {
    validate(piece)?;
    #[allow(clippy::cast_possible_truncation)]
    let tpb = piece.ticks_per_beat as u16; // validated <= 0x7FFF
    let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(u15::new(tpb))));

    // Track 0: conductor — mapping version as name, tempo, end.
    let micros_per_beat = 60_000_000 / piece.bpm;
    let mut conductor = vec![
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(
                piece.mapping_version.as_bytes(),
            )),
        },
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros_per_beat))),
        },
    ];
    conductor.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    smf.tracks.push(conductor);

    for track in &piece.tracks {
        let mut events: Vec<AbsEvent> = Vec::with_capacity(track.notes.len() * 2);
        for n in &track.notes {
            events.push(AbsEvent { tick: n.start, rank: 1, pitch: n.pitch, velocity: n.velocity });
            events.push(AbsEvent { tick: n.start + n.dur, rank: 0, pitch: n.pitch, velocity: 0 });
        }
        events.sort_unstable();

        let channel = u4::new(track.channel);
        let mut out: Vec<TrackEvent> = vec![TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
        }];
        if let Some(program) = track.program {
            out.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::ProgramChange { program: u7::new(program) },
                },
            });
        }

        let mut last_tick = 0u32;
        for ev in events {
            let delta = ev.tick - last_tick;
            if delta > MAX_DELTA {
                return Err(RenderError::DeltaTooLarge(delta));
            }
            last_tick = ev.tick;
            let message = if ev.rank == 1 {
                MidiMessage::NoteOn { key: u7::new(ev.pitch), vel: u7::new(ev.velocity) }
            } else {
                MidiMessage::NoteOff { key: u7::new(ev.pitch), vel: u7::new(0) }
            };
            out.push(TrackEvent {
                delta: u28::new(delta),
                kind: TrackEventKind::Midi { channel, message },
            });
        }
        out.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        smf.tracks.push(out);
    }

    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .map_err(|e| RenderError::Write(e.to_string()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ns_core::{Note, Track};

    fn piece_with(notes: Vec<Note>, bpm: u32) -> Piece {
        Piece {
            mapping_version: "test",
            bpm,
            ticks_per_beat: 480,
            tracks: vec![Track { name: "piano", channel: 1, program: Some(0), notes }],
            sources: Vec::new(),
            movements: Vec::new(),
        }
    }

    fn note(start: u32, pitch: u8, dur: u32, velocity: u8) -> Note {
        Note { start, pitch, dur, velocity, source: 0 }
    }

    #[test]
    fn zero_bpm_is_an_error_not_a_panic() {
        let p = piece_with(vec![note(0, 60, 480, 80)], 0);
        assert!(matches!(render(&p), Err(RenderError::InvalidPiece(_))));
    }

    #[test]
    fn out_of_range_pitch_is_an_error_not_a_masked_note() {
        let p = piece_with(vec![note(0, 200, 480, 80)], 96);
        assert!(matches!(render(&p), Err(RenderError::InvalidPiece(_))));
    }

    #[test]
    fn oversized_delta_is_an_error_not_a_folded_note() {
        let p = piece_with(
            vec![note(0, 60, 480, 80), note(MAX_DELTA + 1000, 62, 480, 80)],
            96,
        );
        assert!(matches!(render(&p), Err(RenderError::DeltaTooLarge(_))));
    }
}
