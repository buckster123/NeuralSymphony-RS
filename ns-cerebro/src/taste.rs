//! The taste-imprint loop (PRD §4) — opt-in, scoped, anti-pollution.
//!
//! Verified sharp edges honored here (cerebro source, 2026-08-04):
//! - Every write passes `agent_id: "neuralsymphony"` — a write WITHOUT
//!   agent_id lands ownerless and SHARED, polluting every agent's recall
//!   (the `visibility` param on remember is advertised but ignored).
//! - Taste memories carry the `ns-internal` tag; the composer's fetch
//!   filters them out, so the loop can never feed on its own output.
//! - `record_procedure_outcome` is a strict bool with asymmetric
//!   reinforcement (failure bites harder by design).
//!
//! How feedback "re-weights the next composition": never by patching the
//! deterministic mapping — through cerebro itself. Verdict memories and
//! procedure outcomes enter the same reinforcement machinery as everything
//! else the mind lives; the GRAPH drifts, and the same pure mapping over a
//! drifted graph is a different piece.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use ns_mcp::McpClient;

use crate::{CerebroConfig, CerebroError};

pub const TASTE_AGENT: &str = "neuralsymphony";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Loved,
    Kept,
    Skipped,
}

impl Verdict {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "loved" => Some(Verdict::Loved),
            "kept" => Some(Verdict::Kept),
            "skipped" => Some(Verdict::Skipped),
            _ => None,
        }
    }
    fn success(self) -> bool {
        !matches!(self, Verdict::Skipped)
    }
    fn salience(self) -> f64 {
        match self {
            Verdict::Loved => 0.9,
            Verdict::Kept => 0.7,
            Verdict::Skipped => 0.4,
        }
    }
    fn word(self) -> &'static str {
        match self {
            Verdict::Loved => "loved",
            Verdict::Kept => "kept",
            Verdict::Skipped => "skipped",
        }
    }
}

/// Local state: the procedure id cerebro assigned to each mapping version
/// (procedures are identified by UUID only — there is no name lookup).
#[derive(Debug, Default, Serialize, Deserialize)]
struct TasteState {
    #[serde(default)]
    procedures: std::collections::BTreeMap<String, String>,
}

fn state_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("neuralsymphony").join("taste.json"))
}

fn load_state() -> TasteState {
    state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_state(state: &TasteState) -> Result<(), CerebroError> {
    let Some(path) = state_path() else {
        return Err(CerebroError::Config("no data dir for taste state".into()));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CerebroError::Config(format!("creating {}: {e}", parent.display())))?;
    }
    let raw = serde_json::to_string_pretty(state)
        .map_err(|e| CerebroError::Config(e.to_string()))?;
    std::fs::write(&path, raw)
        .map_err(|e| CerebroError::Config(format!("writing {}: {e}", path.display())))
}

pub struct FeedbackReport {
    pub memory_id: String,
    pub procedure_id: String,
    pub new_procedure_salience: Option<f64>,
    pub outcomes: Option<Value>,
}

/// Record a verdict on a composition. `themes` are its top tags, `label`
/// describes the source (e.g. "window:7d · 129 memories").
pub fn record_feedback(
    cfg: &CerebroConfig,
    mapping_version: &str,
    verdict: Verdict,
    label: &str,
    themes: &[String],
) -> Result<FeedbackReport, CerebroError> {
    let mut client = McpClient::spawn(&cfg.command, &cfg.args, &cfg.env)?;

    // 1. The verdict as an affective memory — private to the instrument.
    let mut tags: Vec<String> = vec![
        "ns-internal".into(),
        "taste".into(),
        format!("verdict:{}", verdict.word()),
        format!("mapping:{mapping_version}"),
    ];
    tags.extend(themes.iter().take(3).cloned());
    let content = format!(
        "Taste verdict '{}' on a {mapping_version} composition ({label}). Recurring \
         themes: {}. The listener's reaction, remembered so future pieces can lean \
         toward what survived.",
        verdict.word(),
        if themes.is_empty() { "none".to_string() } else { themes.join(", ") },
    );
    let node = client.call_tool(
        "remember",
        json!({
            "content": content,
            "memory_type": "affective",
            "tags": tags,
            "salience": verdict.salience(),
            "agent_id": TASTE_AGENT,
        }),
    )?;
    let memory_id = node
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    // 2. The mapping as a procedure, graded. Stored once per version; the
    //    UUID lives in local state (cerebro has no name lookup).
    let mut state = load_state();
    let procedure_id = match state.procedures.get(mapping_version) {
        Some(id) => id.clone(),
        None => {
            let stored = client.call_tool(
                "store_procedure",
                json!({
                    "content": format!(
                        "NeuralSymphony {mapping_version}: compose music from a memory \
                         graph — types as instruments, salience as dynamics, valence \
                         as harmony, links as voice-leading. Graded by listener taste \
                         verdicts; outcomes steer future mapping-version choices."
                    ),
                    "tags": ["ns-internal", "neuralsymphony", "mapping"],
                    "salience": 0.8,
                    "agent_id": TASTE_AGENT,
                }),
            )?;
            let id = stored
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| CerebroError::Protocol("store_procedure: no id".into()))?
                .to_string();
            state.procedures.insert(mapping_version.to_string(), id.clone());
            save_state(&state)?;
            id
        }
    };

    let graded = client.call_tool(
        "record_procedure_outcome",
        json!({
            "procedure_id": procedure_id,
            "success": verdict.success(),
            "agent_id": TASTE_AGENT,
        }),
    )?;

    Ok(FeedbackReport {
        memory_id,
        procedure_id,
        new_procedure_salience: graded.get("new_salience").and_then(Value::as_f64),
        outcomes: graded.get("outcomes").cloned(),
    })
}
