//! Raw cerebro records → `ns_core` model. Structural fields only: the
//! adapter never reads `content`, so a composed fixture can be shared
//! without leaking a single memory's text — the music is the shape of the
//! mind, not its words.

use chrono::{DateTime, Utc};
use ns_core::{Memory, MemoryType};
use serde::Deserialize;

/// The subset of cerebro's raw `MemoryNode` the mapping needs. Unknown
/// fields (including `content`) are ignored by serde.
#[derive(Debug, Clone, Deserialize)]
pub struct RawNode {
    pub id: String,
    pub memory_type: String,
    #[serde(default)]
    pub layer: Option<String>,
    pub salience: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub emotional_valence: Option<String>,
    #[serde(default)]
    pub emotional_intensity: f64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub thread_id: Option<String>,
}

fn memory_type(raw_type: &str, _layer: Option<&str>) -> MemoryType {
    // Cerebro's `layer` is consolidation state, NOT kind: an un-dreamed
    // store is 100% working-layer, and mapping layer→instrument flattened
    // the first live compose to a single flute (130/130 memories). Kind
    // comes from memory_type alone; `working` remains for fixtures and any
    // future cerebro that grows a working *type*.
    match raw_type {
        "working" => MemoryType::Working,
        "episodic" => MemoryType::Episodic,
        "procedural" => MemoryType::Procedural,
        "affective" => MemoryType::Affective,
        "prospective" => MemoryType::Prospective,
        "schematic" => MemoryType::Schematic,
        // "semantic" and anything a future cerebro invents: the chordal
        // middle is the least-wrong default, and it's deterministic.
        _ => MemoryType::Semantic,
    }
}

/// Cerebro's valence is categorical; the mapping wants a signed float plus
/// the explicit mixed flag. ±0.7 lands in lydian/phrygian territory —
/// decisive but not maximal.
fn valence(v: Option<&str>) -> (f64, bool) {
    match v {
        Some("positive") => (0.7, false),
        Some("negative") => (-0.7, false),
        Some("mixed") => (0.0, true),
        _ => (0.0, false),
    }
}

pub fn adapt(raw: &RawNode) -> Memory {
    let (emotional_valence, valence_mixed) = valence(raw.emotional_valence.as_deref());
    Memory {
        id: raw.id.clone(),
        memory_type: memory_type(&raw.memory_type, raw.layer.as_deref()),
        salience: raw.salience.clamp(0.0, 1.0),
        emotional_valence,
        emotional_intensity: raw.emotional_intensity.clamp(0.0, 1.0),
        valence_mixed,
        created_at: raw.created_at.timestamp(),
        tags: raw.tags.clone(),
        links: Vec::new(),      // filled from the SQLite links table
        episode_id: None,       // filled from episode inversion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_a_real_shaped_node() {
        // Field names and formats from cerebro source + a live sample.
        let raw: RawNode = serde_json::from_str(
            r#"{
                "access_count": 2,
                "access_times": ["2026-06-10T03:38:00Z"],
                "agent_id": "FORGE",
                "content": "the words of the memory, never read by the adapter",
                "created_at": "2026-06-10T05:38:00Z",
                "emotional_intensity": 0.5,
                "emotional_valence": "positive",
                "id": "mem_0cc5b55ff31b",
                "layer": "working",
                "memory_type": "semantic",
                "metadata": {},
                "salience": 1.0,
                "strength": {"difficulty": 4.77, "last_review": null, "stability": 1.0},
                "tags": ["colony", "music"],
                "updated_at": "2026-06-10T05:38:00Z",
                "visibility": "shared"
            }"#,
        )
        .expect("raw node parses");
        let m = adapt(&raw);
        assert_eq!(m.id, "mem_0cc5b55ff31b");
        // layer is consolidation state and must NOT change the instrument —
        // this record is working-LAYER but semantic-TYPE, so: piano
        assert_eq!(m.memory_type, MemoryType::Semantic);
        assert!((m.emotional_valence - 0.7).abs() < f64::EPSILON);
        assert!(!m.valence_mixed);
        assert_eq!(m.created_at, 1781069880); // 2026-06-10T05:38:00Z
        assert_eq!(m.tags, vec!["colony", "music"]);
    }

    #[test]
    fn mixed_valence_sets_the_flag() {
        let (v, mixed) = valence(Some("mixed"));
        assert_eq!(v, 0.0);
        assert!(mixed);
        assert_eq!(valence(None), (0.0, false));
        assert_eq!(valence(Some("negative")), (-0.7, false));
    }

    #[test]
    fn types_map_and_layer_never_interferes() {
        assert_eq!(memory_type("episodic", Some("long_term")), MemoryType::Episodic);
        assert_eq!(memory_type("episodic", Some("working")), MemoryType::Episodic);
        assert_eq!(memory_type("episodic", Some("sensory")), MemoryType::Episodic);
        assert_eq!(memory_type("affective", None), MemoryType::Affective);
        assert_eq!(memory_type("prospective", Some("cortex")), MemoryType::Prospective);
        assert_eq!(memory_type("schematic", None), MemoryType::Schematic);
        assert_eq!(memory_type("working", None), MemoryType::Working);
        assert_eq!(memory_type("unheard_of", None), MemoryType::Semantic);
    }
}
