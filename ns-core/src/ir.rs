//! The intermediate representation: an abstract piece, renderer-agnostic.
//! All fields are integers so equality (and therefore the golden tests)
//! are exact. Times are in ticks at a fixed resolution.

pub const TICKS_PER_BEAT: u32 = 480;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// The mapping contract this piece was produced under (e.g. "mapping_v1").
    pub mapping_version: &'static str,
    pub bpm: u32,
    pub ticks_per_beat: u32,
    /// Stable order: strings, piano, woodwinds, drums — only tracks that
    /// actually carry notes are present.
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub name: &'static str,
    /// MIDI channel; 9 is the GM percussion channel.
    pub channel: u8,
    /// GM program number; `None` on the percussion channel.
    pub program: Option<u8>,
    /// Sorted by (start, pitch, dur, velocity) — canonical.
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Note {
    pub start: u32,
    pub pitch: u8,
    pub dur: u32,
    pub velocity: u8,
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
