//! Fetch a mode-selected memory graph from a live cerebro.
//!
//! Deterministic where it can be: node order comes from cerebro but the
//! mapping canonicalizes anyway; episode assignment is earliest-episode-wins
//! with a stable tie-break; links are read from SQLite and attached only
//! between nodes both present in the selection.

use std::collections::{BTreeMap, BTreeSet};

use ns_core::MemoryGraph;
use rusqlite::OpenFlags;
use serde_json::{json, Value};

use ns_mcp::McpClient;

use crate::adapt::{adapt, RawNode};
use crate::{CerebroConfig, CerebroError};

#[derive(Debug, Clone)]
pub enum Mode {
    /// Everything the store will export.
    All,
    /// Memories created at/after this unix second.
    Window { since_unix: i64 },
    /// One episode's memories.
    Episode { id: String },
    /// One thread's memories.
    Thread { id: String },
    /// The nightly dream suite — honest error until cerebro exposes
    /// cluster membership (its dream reports carry counts, not memory ids).
    Dream,
}

pub fn fetch_graph(
    cfg: &CerebroConfig,
    mode: &Mode,
    agent: Option<&str>,
) -> Result<MemoryGraph, CerebroError> {
    if let Mode::Dream = mode {
        // Verified against cerebro source 2026-08-04: dream reports expose
        // phase counts only — no cluster membership, nothing to compose.
        return Err(CerebroError::Tool(
            "dream mode needs cerebro to expose dream-cluster membership; its reports \
             currently carry only counts. Compose --window over the night instead."
                .into(),
        ));
    }

    let mut client = McpClient::spawn(&cfg.command, &cfg.args, &cfg.env)?;

    // Nodes: export_memories is the one bulk reader that doesn't mutate the
    // brain. Scope note: passing agent_id returns shared ∪ that agent's
    // private, so an exact --agent ask is post-filtered below.
    let mut args = json!({ "limit": cfg.export_limit });
    if let Some(a) = agent {
        args["agent_id"] = json!(a);
    }
    let exported = client.call_tool("export_memories", args)?;
    let raw: Vec<RawNode> = serde_json::from_value(exported)
        .map_err(|e| CerebroError::Protocol(format!("export_memories shape: {e}")))?;

    let mut nodes: BTreeMap<String, ns_core::Memory> = BTreeMap::new();
    let mut thread_of: BTreeMap<String, String> = BTreeMap::new();
    for r in &raw {
        // Anti-Larsen guard: the taste loop's own write-backs are tagged
        // ns-internal and never re-enter the composer's input — a memory
        // feedback loop screeches exactly like an audio one.
        if r.tags.iter().any(|t| t == "ns-internal") {
            continue;
        }
        if let (Some(want), Some(have)) = (agent, r.agent_id.as_deref()) {
            if want != have {
                continue;
            }
        } else if agent.is_some() && r.agent_id.is_none() {
            continue;
        }
        if let Some(t) = &r.thread_id {
            thread_of.insert(r.id.clone(), t.clone());
        }
        nodes.insert(r.id.clone(), adapt(r));
    }

    // Episodes: list, then fetch each for memory_ids + steps. Earliest
    // (started_at, id) episode claims a memory that appears in several.
    let episodes = client.call_tool("list_episodes", json!({ "limit": 1000 }))?;
    let episode_rows: Vec<Value> = serde_json::from_value(episodes)
        .map_err(|e| CerebroError::Protocol(format!("list_episodes shape: {e}")))?;
    let mut episode_order: Vec<(String, String)> = episode_rows
        .iter()
        .filter_map(|e| {
            let id = e.get("id").and_then(Value::as_str)?.to_string();
            let started = e
                .get("started_at")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            Some((started, id))
        })
        .collect();
    episode_order.sort();

    let mut episode_members: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, ep_id) in &episode_order {
        let ep = client.call_tool("get_episode", json!({ "episode_id": ep_id }))?;
        let mut members = BTreeSet::new();
        if let Some(ids) = ep.get("memory_ids").and_then(Value::as_array) {
            for id in ids.iter().filter_map(Value::as_str) {
                members.insert(id.to_string());
            }
        }
        if let Some(steps) = ep.get("steps").and_then(Value::as_array) {
            for id in steps.iter().filter_map(|s| s.get("memory_id").and_then(Value::as_str)) {
                members.insert(id.to_string());
            }
        }
        for m in &members {
            if let Some(node) = nodes.get_mut(m) {
                if node.episode_id.is_none() {
                    node.episode_id = Some(ep_id.clone());
                }
            }
        }
        episode_members.insert(ep_id.clone(), members);
    }

    // Edges: no API exists — read the links table straight from the file,
    // read-only, like cerebro's own analysis tooling. Only pairs where both
    // ends made it into the selection get attached.
    let db_path = cfg.resolve_db_path();
    let db = rusqlite::Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| CerebroError::Db(format!("{}: {e}", db_path.display())))?;
    let mut stmt = db
        .prepare("SELECT source_id, target_id FROM links")
        .map_err(|e| CerebroError::Db(e.to_string()))?;
    let pairs = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| CerebroError::Db(e.to_string()))?;
    for pair in pairs {
        let (s, t) = pair.map_err(|e| CerebroError::Db(e.to_string()))?;
        if nodes.contains_key(&t) {
            if let Some(node) = nodes.get_mut(&s) {
                if !node.links.contains(&t) {
                    node.links.push(t);
                }
            }
        }
    }

    // Mode selection.
    let selected: Vec<ns_core::Memory> = match mode {
        Mode::All => nodes.into_values().collect(),
        Mode::Window { since_unix } => nodes
            .into_values()
            .filter(|m| m.created_at >= *since_unix)
            .collect(),
        Mode::Episode { id } => {
            let members = episode_members.get(id).ok_or_else(|| {
                CerebroError::Tool(format!("unknown episode: {id} (try list_episodes)"))
            })?;
            nodes
                .into_values()
                .filter(|m| members.contains(&m.id))
                .collect()
        }
        Mode::Thread { id } => nodes
            .into_values()
            .filter(|m| thread_of.get(&m.id) == Some(id))
            .collect(),
        Mode::Dream => unreachable!("handled above"),
    };

    Ok(MemoryGraph { memories: selected })
}
