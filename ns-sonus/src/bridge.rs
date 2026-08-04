//! The Sonus bridge: generate → poll → download, with credits printed
//! before and after so nobody spends blind. Reference audio is NOT
//! possible yet — Sonus-RS has no upload/cover transport (verified against
//! source 2026-08-04; parked in its backlog as "upload_audio"). Until that
//! lands, the Dream voice is conditioned by the distilled style text plus
//! Suno's style/weirdness dials only.

use std::collections::HashMap;

use ns_mcp::McpClient;
use serde_json::{json, Value};

pub struct SonusOptions {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub model: String,
    pub timeout_secs: u64,
    /// 0–100 → Suno styleWeight: how hard the style text steers.
    pub style_pct: u64,
    /// 0–100 → Suno weirdnessConstraint.
    pub weirdness_pct: u64,
    pub title: String,
    pub download_dir: Option<String>,
}

#[derive(Debug)]
pub struct Produced {
    pub task_id: String,
    pub status: String,
    pub files: Vec<String>,
    pub credits_before: Option<i64>,
    pub credits_after: Option<i64>,
}

#[derive(Debug)]
pub enum SonusError {
    Mcp(ns_mcp::McpError),
    /// Generation ran but ended in a terminal failure / timeout.
    Generation(String),
}

impl std::fmt::Display for SonusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SonusError::Mcp(e) => write!(f, "sonus: {e}"),
            SonusError::Generation(e) => write!(f, "generation: {e}"),
        }
    }
}

impl std::error::Error for SonusError {}

impl From<ns_mcp::McpError> for SonusError {
    fn from(e: ns_mcp::McpError) -> Self {
        SonusError::Mcp(e)
    }
}

fn credits(client: &mut McpClient) -> Option<i64> {
    client
        .call_tool("check_credits", json!({}))
        .ok()
        .and_then(|v| v.get("credits_remaining").and_then(Value::as_i64))
}

/// Fire one produced track. SPENDS CREDITS the moment generate_song lands;
/// a timeout is resumable upstream (the task keeps cooking), reported here
/// as an error carrying the task id.
pub fn produce(opts: &SonusOptions, style: &str) -> Result<Produced, SonusError> {
    let mut client = McpClient::spawn(&opts.command, &opts.args, &opts.env)?;
    let credits_before = credits(&mut client);

    let generated = client.call_tool(
        "generate_song",
        json!({
            "styles": style,
            "title": opts.title,
            "instrumental": true,
            "model": opts.model,
            "style_pct": opts.style_pct,
            "weirdness_pct": opts.weirdness_pct,
        }),
    )?;
    let task_id = generated
        .get("task_id")
        .and_then(Value::as_str)
        .ok_or_else(|| SonusError::Generation(format!("no task_id in {generated}")))?
        .to_string();

    let done = client.call_tool(
        "check_status_until_done",
        json!({ "task_id": task_id, "timeout_seconds": opts.timeout_secs }),
    )?;
    let status = done
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    if done.get("is_failed").and_then(Value::as_bool).unwrap_or(false) {
        let msg = done
            .get("error_message")
            .and_then(Value::as_str)
            .unwrap_or("upstream failure");
        return Err(SonusError::Generation(format!("task {task_id}: {status}: {msg}")));
    }
    if !done.get("is_complete").and_then(Value::as_bool).unwrap_or(false) {
        return Err(SonusError::Generation(format!(
            "task {task_id} not complete after {}s (status {status}) — it keeps cooking \
             upstream; resume with sonus check_status_until_done / download_track",
            opts.timeout_secs
        )));
    }

    let mut dl_args = json!({ "task_id": task_id });
    if let Some(dir) = &opts.download_dir {
        dl_args["download_dir"] = json!(dir);
    }
    let downloaded = client.call_tool("download_track", dl_args)?;
    let files = downloaded
        .get("files")
        .and_then(Value::as_array)
        .map(|fs| {
            fs.iter()
                .filter_map(|f| f.get("path").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let credits_after = credits(&mut client);
    Ok(Produced { task_id, status, files, credits_before, credits_after })
}
