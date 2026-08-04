//! `neuralsymphony mcp` — MCP stdio server, hand-rolled newline JSON-RPC
//! (the prefrontal pattern; no SDK dependency). Two tools for now: agents
//! compose from their own memories, and ask what a tag sounds like.

use std::io::{BufRead, Write};

use anyhow::Result;
use serde_json::{json, Value};

use crate::source::GraphSpec;

const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(id) = msg.get("id").cloned() else {
            continue; // notification — no reply
        };
        let response = match dispatch(method, &params) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": code, "message": message }
            }),
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}

type RpcError = (i64, String);

fn dispatch(method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or(PROTOCOL_VERSION);
            Ok(json!({
                "protocolVersion": requested,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "neuralsymphony", "version": env!("CARGO_PKG_VERSION") }
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Ok(call_tool(name, &args))
        }
        _ => Err((-32601, format!("method not found: {method}"))),
    }
}

/// Tool outcomes are MCP results with isError — JSON-RPC errors are for
/// protocol breakage only (house rule, same as prefrontal).
fn call_tool(name: &str, args: &Value) -> Value {
    let s = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .filter(|v| !v.is_empty())
    };
    let outcome: Result<String, String> = match name {
        "ns_compose" => tool_compose(
            &GraphSpec {
                fixture: None,
                window: s("window"),
                episode: s("episode"),
                thread: s("thread"),
                dream: args.get("dream").and_then(Value::as_bool).unwrap_or(false),
                everything: args.get("everything").and_then(Value::as_bool).unwrap_or(false),
                agent: s("agent"),
            },
            s("out"),
        ),
        "ns_motif_of" => tool_motif(&s("tag").unwrap_or_default()),
        _ => Err(format!("unknown tool: {name}")),
    };
    match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(msg) => json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
    }
}

fn tool_compose(spec: &GraphSpec, out: Option<String>) -> Result<String, String> {
    let (graph, label) = spec.resolve().map_err(|e| format!("{e:#}"))?;
    let piece = ns_core::compose(&graph).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    if let Some(path) = out {
        let bytes = ns_midi::render(&piece).map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| format!("writing {path}: {e}"))?;
        written.push(path);
    }
    let tracks: Vec<Value> = piece
        .tracks
        .iter()
        .map(|t| json!({ "name": t.name, "notes": t.notes.len() }))
        .collect();
    serde_json::to_string_pretty(&json!({
        "mapping": piece.mapping_version,
        "source": label,
        "memories": graph.memories.len(),
        "notes": piece.note_count(),
        "movements": piece.movements.len(),
        "seconds": piece.len_seconds(),
        "tracks": tracks,
        "written": written,
    }))
    .map_err(|e| e.to_string())
}

fn tool_motif(tag: &str) -> Result<String, String> {
    if tag.is_empty() {
        return Err("tag must not be empty".into());
    }
    let (degrees, rhythm) = ns_core::tag_motif(tag);
    let beats: Vec<f64> = rhythm.iter().map(|&t| f64::from(t) / 480.0).collect();
    serde_json::to_string_pretty(&json!({
        "tag": tag,
        "scale_degrees": degrees,
        "rhythm_beats": beats,
        "note": "same tag, same contour, forever — the mode it lands in is per-memory",
    }))
    .map_err(|e| e.to_string())
}

fn tool_definitions() -> Vec<Value> {
    let obj = |props: Value, required: &[&str]| {
        json!({ "type": "object", "properties": props, "required": required })
    };
    vec![
        json!({
            "name": "ns_compose",
            "description": "Compose music from a live CerebroCortex memory graph (mapping_v1, deterministic). Pick ONE source: window ('7d'/'36h'), episode id, thread id, or everything=true. Optional agent filter and out path for the MIDI file. Never mutates the brain.",
            "inputSchema": obj(json!({
                "window": { "type": "string", "description": "e.g. 7d, 36h, 2w" },
                "episode": { "type": "string" },
                "thread": { "type": "string" },
                "everything": { "type": "boolean" },
                "agent": { "type": "string", "description": "exact agent_id filter" },
                "out": { "type": "string", "description": "write the MIDI here" }
            }), &[]),
        }),
        json!({
            "name": "ns_motif_of",
            "description": "The leitmotif a tag seeds: four scale degrees and a rhythm cell. Same tag = same theme in every composition, forever.",
            "inputSchema": obj(json!({ "tag": { "type": "string" } }), &["tag"]),
        }),
    ]
}
