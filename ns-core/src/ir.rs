//! The intermediate representation: an abstract piece, renderer-agnostic.
//! All fields are integers so equality (and therefore the golden tests)
//! are exact. Times are in ticks at a fixed resolution.

pub const TICKS_PER_BEAT: u32 = 480;

use crate::model::MemoryType;

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// The mapping contract this piece was produced under (e.g. "mapping_v1").
    pub mapping_version: &'static str,
    pub bpm: u32,
    pub ticks_per_beat: u32,
    /// Stable order: strings, piano, woodwinds, drums — only tracks that
    /// actually carry notes are present.
    pub tracks: Vec<Track>,
    /// Semantic provenance, indexed by `Note::source`. MIDI rendering
    /// ignores this entirely (goldens are annotation-blind); the score
    /// stream is made of it.
    pub sources: Vec<SourceInfo>,
    /// Movement boundaries in playback order.
    pub movements: Vec<MovementInfo>,
}

/// Why a note exists: which memory, and in what role.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    pub memory_id: String,
    pub memory_type: MemoryType,
    pub salience: f64,
    pub valence: f64,
    pub mixed: bool,
    /// The memory plays unaccompanied (degree-0 in the graph).
    pub isolated: bool,
    pub episode_id: Option<String>,
    pub role: NoteRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteRole {
    /// The memory's own four-note motif.
    Motif,
    /// A neighbor's pitch sustained underneath (voice-leading).
    Sustain,
    /// The V→I resolution closing an episode run.
    Cadence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementInfo {
    pub start: u32,
    pub root_offset: u8,
    pub isolated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub name: &'static str,
    /// MIDI channel; 9 is the GM percussion channel.
    pub channel: u8,
    /// GM program number; `None` on the percussion channel.
    pub program: Option<u8>,
    /// Sorted by (start, pitch, dur, velocity) — canonical. `source` is the
    /// final tie-break so equal-sounding notes can't reorder the bytes.
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Note {
    pub start: u32,
    pub pitch: u8,
    pub dur: u32,
    pub velocity: u8,
    /// Index into `Piece::sources`.
    pub source: u32,
}

impl Piece {
    /// Total length in ticks (end of the last note).
    pub fn len_ticks(&self) -> u32 {
        self.tracks
            .iter()
            .flat_map(|t| t.notes.iter())
            .map(|n| n.start + n.dur)
            .max()
            .unwrap_or(0)
    }

    pub fn note_count(&self) -> usize {
        self.tracks.iter().map(|t| t.notes.len()).sum()
    }

    pub fn len_seconds(&self) -> f64 {
        let beats = f64::from(self.len_ticks()) / f64::from(self.ticks_per_beat);
        beats * 60.0 / f64::from(self.bpm)
    }
}
