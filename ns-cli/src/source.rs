//! Shared graph acquisition for CLI, HTTP, and MCP surfaces: one spec,
//! one resolution path, so every surface means the same thing by
//! "--window 7d".

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use ns_cerebro::Mode;
use ns_core::MemoryGraph;

#[derive(Debug, Default, Clone)]
pub struct GraphSpec {
    pub fixture: Option<PathBuf>,
    pub window: Option<String>,
    pub episode: Option<String>,
    pub thread: Option<String>,
    pub dream: bool,
    pub everything: bool,
    pub agent: Option<String>,
}

/// "7d" / "36h" / "2w" / "90m" → seconds.
pub fn parse_window(s: &str) -> Result<i64> {
    let s = s.trim();
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let n: i64 = num.parse().with_context(|| format!("bad window: {s:?} (try 7d, 36h, 2w)"))?;
    if n <= 0 {
        bail!("window must be positive: {s:?}");
    }
    let secs = match unit {
        "m" => n * 60,
        "h" => n * 3_600,
        "d" => n * 86_400,
        "w" => n * 7 * 86_400,
        _ => bail!("bad window unit in {s:?} (m, h, d, or w)"),
    };
    Ok(secs)
}

impl GraphSpec {
    /// Resolve to a graph plus a human label for the summary line.
    pub fn resolve(&self) -> Result<(MemoryGraph, String)> {
        if let Some(path) = &self.fixture {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            let graph: MemoryGraph = serde_json::from_str(&raw)
                .with_context(|| format!("parsing {}", path.display()))?;
            return Ok((graph, format!("fixture {}", path.display())));
        }
        let mode = if let Some(w) = &self.window {
            let since = chrono::Utc::now().timestamp() - parse_window(w)?;
            Mode::Window { since_unix: since }
        } else if let Some(id) = &self.episode {
            Mode::Episode { id: id.clone() }
        } else if let Some(id) = &self.thread {
            Mode::Thread { id: id.clone() }
        } else if self.dream {
            Mode::Dream
        } else if self.everything {
            Mode::All
        } else {
            bail!("pick a source: --fixture, --window, --episode, --thread, --dream, or --everything");
        };
        let cfg = ns_cerebro::Config::load()?;
        let graph = ns_cerebro::fetch_graph(&cfg.cerebro, &mode, self.agent.as_deref())?;
        let mut label = match &self.window {
            Some(w) => format!("live cerebro · {w}"),
            None => "live cerebro".to_string(),
        };
        if let Some(a) = &self.agent {
            label.push_str(&format!(" · agent {a}"));
        }
        Ok((graph, label))
    }

    pub fn mode_label(&self) -> String {
        if self.fixture.is_some() {
            "fixture".into()
        } else if let Some(w) = &self.window {
            format!("window:{w}")
        } else if let Some(e) = &self.episode {
            format!("episode:{e}")
        } else if let Some(t) = &self.thread {
            format!("thread:{t}")
        } else if self.dream {
            "dream".into()
        } else {
            "everything".into()
        }
    }
}
