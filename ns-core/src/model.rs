//! Input model: the fixture-graph shape, mirroring only fields cerebro
//! already serves (PRD §3). The M1 cerebro client adapts live data into
//! these types; the mapping never learns where a graph came from.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Working,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    /// 0.0..=1.0 — out-of-range values are clamped, non-finite rejected.
    pub salience: f64,
    /// -1.0..=1.0
    #[serde(default)]
    pub emotional_valence: f64,
    /// 0.0..=1.0
    #[serde(default)]
    pub emotional_intensity: f64,
    /// Unix seconds. With `id`, the canonical sort key.
    pub created_at: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Ids of linked memories, treated as undirected. Unknown ids are ignored.
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraph {
    pub memories: Vec<Memory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    Empty,
    DuplicateId(String),
    /// NaN/inf in salience, valence, or intensity — can't be mapped honestly.
    NonFinite(String),
    /// The piece would outgrow the IR's u32 tick space — same graph must
    /// never panic in one build profile and wrap in another.
    TooLong,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Empty => write!(f, "graph has no memories"),
            GraphError::DuplicateId(id) => write!(f, "duplicate memory id: {id}"),
            GraphError::NonFinite(id) => {
                write!(f, "non-finite salience/valence/intensity on memory: {id}")
            }
            GraphError::TooLong => {
                write!(f, "piece exceeds the mapping's tick capacity — compose a smaller window")
            }
        }
    }
}

impl std::error::Error for GraphError {}
