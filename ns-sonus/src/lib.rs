//! The Dream voice: the mapping's structure distilled into a produced track
//! via Sonus-RS. Costs real credits — nothing here ever auto-fires; every
//! generation is an explicit human or agent decision, and produced tracks
//! are always labelled as generated.

pub mod bridge;
pub mod prompt;

pub use bridge::{produce, Produced, SonusError, SonusOptions};
pub use prompt::{distill, Distilled};
