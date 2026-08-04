//! Prompt distillation: the mapping's *structure* rendered as words, so the
//! Dream voice describes the same mind the Echo voice plays. Pure and
//! deterministic — same piece + graph, same prompt, always (sorted tags,
//! fixed phrasing, integer thresholds).

use std::collections::BTreeMap;

use ns_core::{MemoryGraph, Piece};

/// A compact style prompt plus a longer scene description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distilled {
    pub style: String,
    pub description: String,
}

fn mode_word(valence: f64, mixed: bool) -> &'static str {
    if mixed {
        "suspended"
    } else if valence >= 0.5 {
        "lydian-bright"
    } else if valence >= 0.15 {
        "major"
    } else if valence > -0.15 {
        "dorian-cool"
    } else if valence > -0.5 {
        "minor"
    } else {
        "phrygian-dark"
    }
}

pub fn distill(piece: &Piece, graph: &MemoryGraph) -> Distilled {
    let n = graph.memories.len().max(1);

    // Emotional weather from the valence distribution.
    let mut weather: BTreeMap<&str, usize> = BTreeMap::new();
    let mut intensity_sum = 0.0;
    for m in &graph.memories {
        *weather.entry(mode_word(m.emotional_valence, m.valence_mixed)).or_insert(0) += 1;
        intensity_sum += m.emotional_intensity.clamp(0.0, 1.0);
    }
    let mut weather: Vec<(&str, usize)> = weather.into_iter().collect();
    weather.sort_by_key(|&(w, c)| (std::cmp::Reverse(c), w));
    let weather_words: Vec<&str> = weather.iter().take(2).map(|&(w, _)| w).collect();

    let energy = {
        let avg = intensity_sum / n as f64;
        if avg >= 0.6 {
            "high-tension"
        } else if avg >= 0.3 {
            "flowing"
        } else {
            "calm"
        }
    };

    // Instrumentation from note share per family.
    let mut fams: Vec<(&str, usize)> = piece
        .tracks
        .iter()
        .map(|t| (t.name, t.notes.len()))
        .collect();
    fams.sort_by_key(|&(name, c)| (std::cmp::Reverse(c), name));
    let lead = fams.first().map_or("piano", |&(name, _)| name);

    // Recurring themes: top tags by memory count, sorted, stable.
    let mut tag_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for m in &graph.memories {
        for t in &m.tags {
            *tag_counts.entry(t.as_str()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<(&str, usize)> = tag_counts.into_iter().collect();
    tags.sort_by_key(|&(t, c)| (std::cmp::Reverse(c), t));
    let themes: Vec<&str> = tags.iter().take(3).map(|&(t, _)| t).collect();

    let lonely = graph
        .memories
        .iter()
        .filter(|m| {
            m.links.is_empty()
                && !graph.memories.iter().any(|o| o.links.contains(&m.id))
        })
        .count();

    let drums = piece.tracks.iter().any(|t| t.channel == 9);

    let style = format!(
        "instrumental, {energy}, {} mood, {lead}-led chamber electronic, {} bpm, {} movements{}{}",
        weather_words.join(" and "),
        piece.bpm,
        piece.movements.len(),
        if drums { ", sparse percussion" } else { ", no percussion" },
        if lonely > 0 { ", with solitary interludes" } else { "" },
    );

    let description = format!(
        "A {:.0}-second instrumental portrait of a memory graph: {} memories across {} \
         movements, {lead} carrying the narrative{}. Emotional weather: {}. {}Recurring \
         themes echo as leitmotifs{}. Deterministic source: {} — the same mind renders \
         the same piece.",
        piece.len_seconds(),
        n,
        piece.movements.len(),
        if drums { ", procedures as percussion" } else { "" },
        weather_words.join(" shading into "),
        if lonely > 0 {
            format!(
                "{lonely} isolated memor{} alone. ",
                if lonely == 1 { "y plays" } else { "ies play" }
            )
        } else {
            String::new()
        },
        if themes.is_empty() {
            String::new()
        } else {
            format!(" ({})", themes.join(", "))
        },
        piece.mapping_version,
    );

    Distilled { style, description }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Piece, MemoryGraph) {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/week-01.json"
        ))
        .expect("fixture");
        let graph: MemoryGraph = serde_json::from_str(&raw).expect("parse");
        let piece = ns_core::compose(&graph).expect("compose");
        (piece, graph)
    }

    #[test]
    fn distillation_is_deterministic_and_grounded() {
        let (piece, graph) = fixture();
        let a = distill(&piece, &graph);
        let b = distill(&piece, &graph);
        assert_eq!(a, b);
        assert!(a.style.contains("96 bpm"), "style: {}", a.style);
        assert!(a.style.contains("movements"), "style: {}", a.style);
        assert!(a.style.contains("solitary interludes"), "week-01 has iso-3am: {}", a.style);
        assert!(a.description.contains("14 memories"), "desc: {}", a.description);
        assert!(a.style.len() < 400, "style must stay compact: {}", a.style.len());
    }
}
