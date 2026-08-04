//! The impure shell around the pure mapping: fetch a memory graph from a
//! live CerebroCortex and adapt it into `ns_core::MemoryGraph`.
//!
//! Transport facts (verified against cerebro source, 2026-08-04):
//! - Nodes and episodes come over **MCP stdio** (`cerebro-mcp`), via
//!   `export_memories` / `list_episodes` / `get_episode` — the only bulk
//!   readers that do NOT mutate the store. `recall`/`memory_search` persist
//!   ACT-R/FSRS state and link traversals; a composer must never use them.
//! - **Edges have no API.** The `links` table is read straight from
//!   cerebro's SQLite, read-only, exactly like cerebro's own analysis
//!   tooling does.
//! - Every MCP tool result is double-encoded: JSON inside
//!   `result.content[0].text`.

pub mod adapt;
pub mod config;
pub mod fetch;
pub mod taste;

pub use config::{CerebroConfig, Config, SonusConfig, TasteConfig};
pub use fetch::{fetch_graph, Mode};
pub use taste::{record_feedback, Verdict};

#[derive(Debug)]
pub enum CerebroError {
    Config(String),
    Spawn(String),
    Protocol(String),
    /// The tool ran and answered with an error (or an honest "can't").
    Tool(String),
    Db(String),
}

impl std::fmt::Display for CerebroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CerebroError::Config(e) => write!(f, "config: {e}"),
            CerebroError::Spawn(e) => write!(f, "spawning cerebro-mcp: {e}"),
            CerebroError::Protocol(e) => write!(f, "cerebro-mcp protocol: {e}"),
            CerebroError::Tool(e) => write!(f, "cerebro: {e}"),
            CerebroError::Db(e) => write!(f, "cerebro db (links): {e}"),
        }
    }
}

impl std::error::Error for CerebroError {}

impl From<ns_mcp::McpError> for CerebroError {
    fn from(e: ns_mcp::McpError) -> Self {
        match e {
            ns_mcp::McpError::Spawn(s) => CerebroError::Spawn(s),
            ns_mcp::McpError::Protocol(s) => CerebroError::Protocol(s),
            ns_mcp::McpError::Tool(s) => CerebroError::Tool(s),
        }
    }
}
