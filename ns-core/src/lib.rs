//! The mapping core: a CerebroCortex-shaped memory graph in, a musical piece
//! out. Pure and deterministic by contract — same graph, same piece, always.
//! Everything time- or randomness-shaped is forbidden here; motif variety
//! comes from stable hashes of the input itself.

pub mod ir;
pub mod map_v1;
pub mod model;

pub use ir::{Note, Piece, Track};
pub use map_v1::compose;
pub use model::{GraphError, Memory, MemoryGraph, MemoryType};
