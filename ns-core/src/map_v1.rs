//! mapping_v1 — the versioned contract from graph to music (PRD §3).
//!
//! Deliberately simple; taste iteration happens in mapping_v2+, never by
//! patching this in place. Every choice below is deterministic: canonical
//! memory order is (created_at, id), collections are BTree, and all variety
//! is seeded by FNV-1a hashes of the input itself. The concrete numbers are
//! documented in docs/mapping_v1.md; the golden tests pin them forever.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{Note, Piece, Track, TICKS_PER_BEAT};
use crate::model::{GraphError, Memory, MemoryGraph, MemoryType};

pub const MAPPING_VERSION: &str = "mapping_v1";
pub const BPM: u32 = 96;

const BAR: u32 = 4 * TICKS_PER_BEAT;
const HALF_BAR: u32 = BAR / 2;

/// Hard ceiling on the timeline cursor, with headroom so `start + dur` can
/// never overflow u32 downstream. Found by adversarial review: unchecked
/// cursor math panicked in debug and wrapped in release at ~559k memories —
/// a build-profile-dependent outcome, the exact thing this crate forbids.
const MAX_TICKS: u32 = u32::MAX - 4 * BAR;

fn advance(cursor: u32, delta: u32) -> Result<u32, GraphError> {
    match cursor.checked_add(delta) {
        Some(next) if next <= MAX_TICKS => Ok(next),
        _ => Err(GraphError::TooLong),
    }
}

/// Modes, chosen per memory from emotional valence (PRD: positive → lydian /
/// major, negative → minor / phrygian, neutral → dorian, mixed → suspensions).
const LYDIAN: [u8; 7] = [0, 2, 4, 6, 7, 9, 11];
const MAJOR: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const DORIAN: [u8; 7] = [0, 2, 3, 5, 7, 9, 10];
const AEOLIAN: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];
const PHRYGIAN: [u8; 7] = [0, 1, 3, 5, 7, 8, 10];
/// "Mixed" renders as suspensions: degrees restricted to 1-2-4-5 over dorian.
const SUS_DEGREES: [usize; 4] = [0, 1, 3, 4];

/// Four rhythm cells, each summing to one bar; picked by seed.
const RHYTHMS: [[u32; 4]; 4] = [
    [480, 480, 480, 480],
    [720, 240, 480, 480],
    [240, 240, 480, 960],
    [480, 240, 240, 960],
];

/// GM drum notes for procedural motifs (procedures ARE rhythm):
/// kick, snare, closed hat, open hat, ride, low tom, crash.
const DRUM_NOTES: [u8; 7] = [36, 38, 42, 46, 51, 45, 49];
const DRUM_HIT: u32 = 120;

/// Fixed track slots in stable order. Cadences always land on piano.
const STRINGS: usize = 0;
const PIANO: usize = 1;
const WOODWINDS: usize = 2;
const DRUMS: usize = 3;

struct Family {
    slot: usize,
    base_note: u8,
}

fn family(t: MemoryType) -> Family {
    // PRD §3: episodic → strings (narrative lines), semantic → piano
    // (chordal statements), procedural → percussion, working → woodwinds.
    match t {
        MemoryType::Episodic => Family { slot: STRINGS, base_note: 48 },
        MemoryType::Semantic => Family { slot: PIANO, base_note: 60 },
        MemoryType::Working => Family { slot: WOODWINDS, base_note: 72 },
        MemoryType::Procedural => Family { slot: DRUMS, base_note: 0 },
    }
}

fn track_shell(slot: usize) -> Track {
    let (name, channel, program) = match slot {
        STRINGS => ("strings", 0, Some(48)),
        PIANO => ("piano", 1, Some(0)),
        WOODWINDS => ("woodwinds", 2, Some(73)),
        _ => ("drums", 9, None),
    };
    Track { name, channel, program, notes: Vec::new() }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn scale_for(valence: f64, intensity: f64) -> (&'static [u8; 7], bool) {
    let sus = valence.abs() < 0.15 && intensity > 0.6;
    if sus {
        return (&DORIAN, true);
    }
    let scale = if valence >= 0.5 {
        &LYDIAN
    } else if valence >= 0.15 {
        &MAJOR
    } else if valence > -0.15 {
        &DORIAN
    } else if valence > -0.5 {
        &AEOLIAN
    } else {
        &PHRYGIAN
    };
    (scale, false)
}

/// One rendered motif: up to four (onset, dur, pitch, velocity) notes plus
/// the last melodic pitch (for voice-leading sustains).
struct Motif {
    notes: [(u32, u32, u8, u8); 4],
    last_pitch: u8,
}

fn motif_for(mem: &Memory, root_offset: u8) -> Motif {
    // Leitmotif seed (PRD: a stable hash of the tag seeds a motif). Sorted
    // first tag so tag order in the record never matters; id as fallback.
    let mut tags = mem.tags.clone();
    tags.sort();
    let seed = match tags.first() {
        Some(t) => fnv1a64(t.as_bytes()),
        None => fnv1a64(mem.id.as_bytes()),
    };

    let (scale, sus) = scale_for(mem.emotional_valence, mem.emotional_intensity);
    let rhythm = &RHYTHMS[((seed >> 16) & 0x3) as usize];
    let fam = family(mem.memory_type);
    let drums = fam.slot == DRUMS;

    let salience = mem.salience.clamp(0.0, 1.0);
    let intensity = mem.emotional_intensity.clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let base_vel = (40.0 + salience * 87.0).round() as i16;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let spread = (intensity * 12.0).round() as i16;

    let mut notes = [(0u32, 0u32, 0u8, 0u8); 4];
    let mut onset = 0u32;
    let mut last_pitch = 0u8;
    for (i, item) in notes.iter_mut().enumerate() {
        let nibble = ((seed >> (4 * i)) & 0xF) as usize;
        let degree = if sus { SUS_DEGREES[nibble % 4] } else { nibble % 7 };
        let pitch = if drums {
            DRUM_NOTES[degree % 7]
        } else {
            fam.base_note + root_offset + scale[degree]
        };
        let vel = (base_vel + if i % 2 == 0 { spread } else { -spread }).clamp(1, 127);
        let dur = if drums { DRUM_HIT } else { rhythm[i] };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let vel8 = vel as u8;
        *item = (onset, dur, pitch, vel8);
        if !drums {
            last_pitch = pitch;
        }
        onset += rhythm[i];
    }
    Motif { notes, last_pitch }
}

/// Compose a piece from a memory graph under mapping_v1 (timeline mode).
///
/// Pure and total for valid graphs: same graph in, byte-identical IR out.
pub fn compose(graph: &MemoryGraph) -> Result<Piece, GraphError> {
    if graph.memories.is_empty() {
        return Err(GraphError::Empty);
    }
    let mut seen = BTreeSet::new();
    for m in &graph.memories {
        if !seen.insert(m.id.as_str()) {
            return Err(GraphError::DuplicateId(m.id.clone()));
        }
        if !(m.salience.is_finite()
            && m.emotional_valence.is_finite()
            && m.emotional_intensity.is_finite())
        {
            return Err(GraphError::NonFinite(m.id.clone()));
        }
    }

    // Canonical order: (created_at, id). Everything downstream keys off it.
    let mut order: Vec<&Memory> = graph.memories.iter().collect();
    order.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));

    let index_of: BTreeMap<&str, usize> =
        order.iter().enumerate().map(|(i, m)| (m.id.as_str(), i)).collect();

    // Undirected adjacency; unknown link targets ignored.
    let mut adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); order.len()];
    for (i, m) in order.iter().enumerate() {
        for target in &m.links {
            if let Some(&j) = index_of.get(target.as_str()) {
                if j != i {
                    adj[i].insert(j);
                    adj[j].insert(i);
                }
            }
        }
    }

    // Graph components → movements (PRD: isolated memories play alone).
    // Discovery over canonical order keeps movement order canonical too.
    let mut component = vec![usize::MAX; order.len()];
    let mut movements: Vec<Vec<usize>> = Vec::new();
    for start in 0..order.len() {
        if component[start] != usize::MAX {
            continue;
        }
        let id = movements.len();
        let mut members = Vec::new();
        let mut queue = vec![start];
        component[start] = id;
        while let Some(i) = queue.pop() {
            members.push(i);
            for &j in &adj[i] {
                if component[j] == usize::MAX {
                    component[j] = id;
                    queue.push(j);
                }
            }
        }
        members.sort_unstable();
        movements.push(members);
    }

    let mut tracks = [
        track_shell(STRINGS),
        track_shell(PIANO),
        track_shell(WOODWINDS),
        track_shell(DRUMS),
    ];

    let mut cursor = 0u32;
    for (mv_idx, members) in movements.iter().enumerate() {
        // Movement root walks the circle of fourths — deterministic variety.
        #[allow(clippy::cast_possible_truncation)]
        let root_offset = ((mv_idx * 5) % 12) as u8;
        let isolated = members.len() == 1 && adj[members[0]].is_empty();

        for (pos, &i) in members.iter().enumerate() {
            let mem = order[i];
            if isolated {
                cursor = advance(cursor, HALF_BAR)?; // a lonely memory gets space around it
            }

            let motif = motif_for(mem, root_offset);
            let slot = family(mem.memory_type).slot;
            for &(onset, dur, pitch, vel) in &motif.notes {
                tracks[slot].notes.push(Note {
                    start: cursor + onset,
                    pitch,
                    dur,
                    velocity: vel,
                });
            }

            // Voice-leading-lite (PRD: linked memories share and pass motifs;
            // common neighbors → chord membership): if this memory is linked
            // to — or shares a neighbor with — the previous one in sequence,
            // the previous memory's last pitch sustains under this bar.
            // Same-family neighbors never sustain: on a shared track the
            // sustain can collide with a same-pitch motif note, and the
            // motif's early NoteOff truncates it in real players (review
            // finding) — and a voice sustaining under itself is mud anyway.
            if pos > 0 {
                let prev = members[pos - 1];
                let related = adj[i].contains(&prev)
                    || !adj[i].is_disjoint(&adj[prev]);
                let prev_mem = order[prev];
                if related
                    && family(prev_mem.memory_type).slot != DRUMS
                    && family(prev_mem.memory_type).slot != slot
                {
                    let prev_motif = motif_for(prev_mem, root_offset);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let sustain_vel = ((40.0
                        + prev_mem.salience.clamp(0.0, 1.0) * 87.0)
                        .round() as i16
                        - 20)
                        .clamp(20, 127) as u8;
                    tracks[family(prev_mem.memory_type).slot].notes.push(Note {
                        start: cursor,
                        pitch: prev_motif.last_pitch,
                        dur: BAR,
                        velocity: sustain_vel,
                    });
                }
            }

            cursor = advance(cursor, BAR)?;
            if isolated {
                cursor = advance(cursor, HALF_BAR)?;
            }

            // Episode cadence (PRD: phrases with beginnings and cadences):
            // when a consecutive same-episode run ends, resolve V → I on
            // piano in an extra bar.
            let run_ends = match (&mem.episode_id, members.get(pos + 1)) {
                (Some(ep), Some(&next)) => order[next].episode_id.as_ref() != Some(ep),
                (Some(_), None) => true,
                (None, _) => false,
            };
            if run_ends {
                let (scale, _) = scale_for(mem.emotional_valence, mem.emotional_intensity);
                let dominant = 60 + root_offset + scale[4];
                let tonic = 60 + root_offset;
                tracks[PIANO].notes.push(Note {
                    start: cursor,
                    pitch: dominant,
                    dur: HALF_BAR,
                    velocity: 70,
                });
                tracks[PIANO].notes.push(Note {
                    start: cursor + HALF_BAR,
                    pitch: tonic,
                    dur: HALF_BAR,
                    velocity: 70,
                });
                cursor = advance(cursor, BAR)?;
            }
        }

        if mv_idx + 1 < movements.len() {
            cursor = advance(cursor, BAR)?; // breath between movements
        }
    }

    let mut out: Vec<Track> = tracks
        .into_iter()
        .filter(|t| !t.notes.is_empty())
        .collect();
    for t in &mut out {
        t.notes.sort_unstable();
    }

    Ok(Piece {
        mapping_version: MAPPING_VERSION,
        bpm: BPM,
        ticks_per_beat: TICKS_PER_BEAT,
        tracks: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Memory;

    #[test]
    fn advance_errors_instead_of_overflowing() {
        assert_eq!(advance(0, BAR).unwrap(), BAR);
        assert_eq!(advance(MAX_TICKS, 1), Err(GraphError::TooLong));
        assert_eq!(advance(u32::MAX, BAR), Err(GraphError::TooLong));
    }

    #[test]
    fn same_family_neighbors_do_not_sustain() {
        // Two linked semantic memories: without the same-family guard the
        // piano would carry 8 motif notes + 1 sustain; with it, exactly 8.
        let mem = |id: &str, t, links: &[&str]| Memory {
            id: id.into(),
            memory_type: MemoryType::Semantic,
            salience: 0.5,
            emotional_valence: 0.0,
            emotional_intensity: 0.0,
            created_at: t,
            tags: vec![],
            links: links.iter().map(|s| (*s).into()).collect(),
            episode_id: None,
        };
        let graph = MemoryGraph {
            memories: vec![mem("x", 10, &["y"]), mem("y", 20, &["x"])],
        };
        let piece = compose(&graph).expect("compose");
        assert_eq!(piece.tracks.len(), 1);
        assert_eq!(piece.tracks[0].name, "piano");
        assert_eq!(piece.tracks[0].notes.len(), 8);
    }
}
