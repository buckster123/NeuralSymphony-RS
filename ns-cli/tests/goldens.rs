//! The M0 proof (PRD): a fixture week renders identically forever.
//!
//! Golden bytes live in tests/goldens/. To bless new goldens after an
//! *intentional* mapping change: UPDATE_GOLDENS=1 cargo test -p ns-cli
//! (and bump the mapping version — goldens never change under a version).

use std::path::PathBuf;

use ns_core::MemoryGraph;

fn fixture(name: &str) -> MemoryGraph {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn render(graph: &MemoryGraph) -> Vec<u8> {
    let piece = ns_core::compose(graph).expect("compose");
    ns_midi::render(&piece).expect("render")
}

fn check_golden(fixture_name: &str, golden_name: &str) {
    let graph = fixture(fixture_name);
    let bytes = render(&graph);
    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(golden_name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(golden_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&golden_path, &bytes).expect("bless golden");
        return;
    }
    let golden = std::fs::read(&golden_path).unwrap_or_else(|e| {
        panic!(
            "reading golden {} ({e}) — bless with UPDATE_GOLDENS=1",
            golden_path.display()
        )
    });
    assert_eq!(
        bytes,
        golden,
        "{golden_name}: rendered bytes diverged from the golden — either the \
         mapping changed (bump the version, bless new goldens) or determinism broke"
    );
}

#[test]
fn golden_week_01() {
    check_golden("week-01.json", "week-01.mid");
}

#[test]
fn golden_minimal() {
    check_golden("minimal.json", "minimal.mid");
}

#[test]
fn same_graph_same_bytes() {
    let graph = fixture("week-01.json");
    assert_eq!(render(&graph), render(&graph));
}

#[test]
fn input_order_is_irrelevant() {
    // The canonical sort inside compose must make record order meaningless.
    let graph = fixture("week-01.json");
    let mut reversed = graph.clone();
    reversed.memories.reverse();
    assert_eq!(render(&graph), render(&reversed));

    let mut rotated = graph.clone();
    rotated.memories.rotate_left(5);
    assert_eq!(render(&graph), render(&rotated));
}

#[test]
fn structure_is_sane() {
    let piece = ns_core::compose(&fixture("week-01.json")).expect("compose");
    assert_eq!(piece.mapping_version, "mapping_v1");
    assert!(piece.note_count() > 14, "each memory contributes notes");
    assert!(piece.tracks.len() == 4, "week-01 exercises all four families");
    assert!(piece.len_seconds() > 20.0, "a week should not be a jingle");
    for t in &piece.tracks {
        for n in &t.notes {
            assert!(n.velocity >= 1 && n.pitch <= 127, "MIDI ranges hold");
        }
    }
}

#[test]
fn empty_and_duplicate_graphs_error() {
    let empty = MemoryGraph { memories: vec![] };
    assert!(ns_core::compose(&empty).is_err());

    let mut dup = fixture("minimal.json");
    let clone = dup.memories[0].clone();
    dup.memories.push(clone);
    assert!(ns_core::compose(&dup).is_err());
}
