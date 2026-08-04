//! Central config, one file: `~/.config/neuralsymphony/config.toml`.
//! Everything defaults so a bare install still points somewhere sensible —
//! but note the two-brains trap below.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::CerebroError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cerebro: CerebroConfig,
}

/// How to reach the cerebro. **The two-brains trap:** cerebro-mcp resolves
/// its data dir from `CEREBRO_DATA_DIR`, defaulting to `~/.cerebro-cortex` —
/// but a machine can easily carry a second, live brain elsewhere (this one
/// does: the MCP registration points at `.../CerebroCortex/data`). Set
/// `env.CEREBRO_DATA_DIR` here to compose from the brain you mean.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CerebroConfig {
    /// The cerebro-mcp binary (PATH-resolved or absolute).
    pub command: String,
    pub args: Vec<String>,
    /// Environment for the child — CEREBRO_DATA_DIR is the one that matters.
    pub env: HashMap<String, String>,
    /// cerebro.db for the links table. Default: `$env.CEREBRO_DATA_DIR/
    /// cerebro.db`, else `~/.cerebro-cortex/cerebro.db`.
    pub db_path: Option<String>,
    /// Passed to export_memories; the whole store must fit (no pagination
    /// exists on cerebro's side).
    pub export_limit: u64,
}

impl Default for CerebroConfig {
    fn default() -> Self {
        Self {
            command: "cerebro-mcp".into(),
            args: Vec::new(),
            env: HashMap::new(),
            db_path: None,
            export_limit: 100_000,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, CerebroError> {
        let Some(path) = Self::path() else {
            return Ok(Self::default());
        };
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| CerebroError::Config(format!("reading {}: {e}", path.display())))?;
        toml::from_str(&raw)
            .map_err(|e| CerebroError::Config(format!("parsing {}: {e}", path.display())))
    }

    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("neuralsymphony").join("config.toml"))
    }
}

impl CerebroConfig {
    /// Where the links table lives, honoring db_path > env > default.
    pub fn resolve_db_path(&self) -> PathBuf {
        if let Some(p) = &self.db_path {
            return PathBuf::from(p);
        }
        if let Some(dir) = self.env.get("CEREBRO_DATA_DIR") {
            return PathBuf::from(dir).join("cerebro.db");
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cerebro-cortex")
            .join("cerebro.db")
    }
}
