//! score_v0 — the semantic score stream (docs/score_v0.md is the spec).
//!
//! A visualizer fed PCM has to *infer* structure after the fact. A
//! visualizer fed this stream is *told*: which memory is sounding, its
//! valence and salience, when the episode cadences, where movements begin.
//! The `.score` file form doubles as the fixture format, so a renderer can
//! be built and golden-tested against canned streams before the composer
//! even runs. Versioned like the mapping: byte-deterministic per version.

use ns_core::{MemoryType, NoteRole, Piece};
use serde::{Deserialize, Serialize};

pub const SCORE_VERSION: &str = "score_v0";

/// One line of the NDJSON stream. `t` is milliseconds from piece start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScoreEvent {
    ScoreMeta {
        version: String,
        mapping: String,
        mode: String,
        bpm: u32,
        ticks_per_beat: u32,
    },
    /// A movement (graph component) begins.
    Section {
        t: u64,
        index: u32,
        root_offset: u8,
        isolated: bool,
    },
    /// An episode run begins or resolves.
    Phrase {
        t: u64,
        episode_id: String,
        phase: PhrasePhase,
    },
    NoteOn {
        t: u64,
        track: String,
        pitch: u8,
        velocity: u8,
        memory_id: String,
        memory_type: MemoryType,
        salience: f64,
        valence: f64,
        mixed: bool,
        isolated: bool,
        role: Role,
    },
    NoteOff {
        t: u64,
        track: String,
        pitch: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhrasePhase {
    Begin,
    Cadence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Motif,
    Sustain,
    Cadence,
}

impl From<NoteRole> for Role {
    fn from(r: NoteRole) -> Self {
        match r {
            NoteRole::Motif => Role::Motif,
            NoteRole::Sustain => Role::Sustain,
            NoteRole::Cadence => Role::Cadence,
        }
    }
}

fn ticks_to_ms(ticks: u32, bpm: u32, tpb: u32) -> u64 {
    // Exact integer math: ms = ticks * 60_000 / (bpm * tpb).
    u64::from(ticks) * 60_000 / (u64::from(bpm) * u64::from(tpb))
}

/// Emit the full event list for a piece, time-ordered, deterministic.
pub fn events(piece: &Piece, mode: &str) -> Vec<ScoreEvent> {
    let ms = |ticks: u32| ticks_to_ms(ticks, piece.bpm, piece.ticks_per_beat);

    // (t, rank, seq, event): rank orders ties — sections 0, phrase 1,
    // offs 2, ons 3; seq keeps equal entries in build order.
    let mut timed: Vec<(u64, u8, usize, ScoreEvent)> = Vec::new();
    let mut seq = 0usize;
    let mut push = |timed: &mut Vec<(u64, u8, usize, ScoreEvent)>, t, rank, ev| {
        timed.push((t, rank, seq, ev));
        seq += 1;
    };

    for (i, mv) in piece.movements.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let index = i as u32;
        push(
            &mut timed,
            ms(mv.start),
            0,
            ScoreEvent::Section {
                t: ms(mv.start),
                index,
                root_offset: mv.root_offset,
                isolated: mv.isolated,
            },
        );
    }

    // Phrase events derive from provenance: the first motif note of each
    // episode's run opens it; cadence notes resolve it. (One phrase per
    // episode id per contiguous run — runs are contiguous by construction.)
    let mut seen_begin: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut cadenced: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();

    let mut all_notes: Vec<(&ns_core::Track, &ns_core::Note)> = piece
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter().map(move |n| (t, n)))
        .collect();
    all_notes.sort_by_key(|(_, n)| (n.start, n.pitch, n.source));

    for (track, n) in &all_notes {
        let src = &piece.sources[n.source as usize];
        if let Some(ep) = &src.episode_id {
            match src.role {
                NoteRole::Motif if !seen_begin.contains(ep.as_str()) => {
                    seen_begin.insert(ep.as_str());
                    push(
                        &mut timed,
                        ms(n.start),
                        1,
                        ScoreEvent::Phrase {
                            t: ms(n.start),
                            episode_id: ep.clone(),
                            phase: PhrasePhase::Begin,
                        },
                    );
                }
                NoteRole::Cadence if !cadenced.contains(ep.as_str()) => {
                    cadenced.insert(ep.as_str());
                    push(
                        &mut timed,
                        ms(n.start),
                        1,
                        ScoreEvent::Phrase {
                            t: ms(n.start),
                            episode_id: ep.clone(),
                            phase: PhrasePhase::Cadence,
                        },
                    );
                }
                _ => {}
            }
        }
        push(
            &mut timed,
            ms(n.start),
            3,
            ScoreEvent::NoteOn {
                t: ms(n.start),
                track: track.name.to_string(),
                pitch: n.pitch,
                velocity: n.velocity,
                memory_id: src.memory_id.clone(),
                memory_type: src.memory_type,
                salience: src.salience,
                valence: src.valence,
                mixed: src.mixed,
                isolated: src.isolated,
                role: src.role.into(),
            },
        );
        push(
            &mut timed,
            ms(n.start + n.dur),
            2,
            ScoreEvent::NoteOff {
                t: ms(n.start + n.dur),
                track: track.name.to_string(),
                pitch: n.pitch,
            },
        );
    }

    timed.sort_by_key(|e| (e.0, e.1, e.2));

    let mut out = Vec::with_capacity(timed.len() + 1);
    out.push(ScoreEvent::ScoreMeta {
        version: SCORE_VERSION.to_string(),
        mapping: piece.mapping_version.to_string(),
        mode: mode.to_string(),
        bpm: piece.bpm,
        ticks_per_beat: piece.ticks_per_beat,
    });
    out.extend(timed.into_iter().map(|(_, _, _, ev)| ev));
    out
}

/// The `.score` file form: one JSON object per line.
pub fn to_ndjson(piece: &Piece, mode: &str) -> String {
    let mut s = String::new();
    for ev in events(piece, mode) {
        // serde_json::to_string on these enums cannot fail
        if let Ok(line) = serde_json::to_string(&ev) {
            s.push_str(&line);
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_to_ms_is_exact_at_96bpm() {
        // one beat at 96 bpm = 625 ms
        assert_eq!(ticks_to_ms(480, 96, 480), 625);
        assert_eq!(ticks_to_ms(1920, 96, 480), 2500); // one bar
        assert_eq!(ticks_to_ms(0, 96, 480), 0);
    }
}
