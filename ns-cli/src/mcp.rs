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
        "ns_render_full" => tool_render_full(
            &GraphSpec {
                fixture: None,
                window: s("window"),
                episode: s("episode"),
                thread: s("thread"),
                dream: false,
                everything: args.get("everything").and_then(Value::as_bool).unwrap_or(false),
                agent: s("agent"),
            },
            s("title"),
            args.get("style_pct").and_then(Value::as_u64).unwrap_or(65),
            args.get("weirdness_pct").and_then(Value::as_u64).unwrap_or(50),
            args.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
        ),
        "ns_feedback" => tool_feedback(
            &s("verdict").unwrap_or_default(),
            &s("fixture").unwrap_or_default(),
        ),
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

fn tool_render_full(
    spec: &GraphSpec,
    title: Option<String>,
    style_pct: u64,
    weirdness_pct: u64,
    dry_run: bool,
) -> Result<String, String> {
    let (graph, label) = spec.resolve().map_err(|e| format!("{e:#}"))?;
    let piece = ns_core::compose(&graph).map_err(|e| e.to_string())?;
    let distilled = ns_sonus::distill(&piece, &graph);
    if dry_run {
        return serde_json::to_string_pretty(&json!({
            "dry_run": true,
            "style": distilled.style,
            "description": distilled.description,
            "note": "no credits spent",
        }))
        .map_err(|e| e.to_string());
    }
    let cfg = ns_cerebro::Config::load().map_err(|e| e.to_string())?;
    let opts = ns_sonus::SonusOptions {
        command: cfg.sonus.command.clone(),
        args: cfg.sonus.args.clone(),
        env: cfg.sonus.env.clone(),
        model: cfg.sonus.model.clone(),
        timeout_secs: cfg.sonus.timeout_secs,
        style_pct: style_pct.min(100),
        weirdness_pct: weirdness_pct.min(100),
        title: title.unwrap_or_else(|| format!("NeuralSymphony · {label}")),
        download_dir: None,
    };
    let produced = ns_sonus::produce(&opts, &distilled.style).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&json!({
        "task_id": produced.task_id,
        "status": produced.status,
        "files": produced.files,
        "credits_before": produced.credits_before,
        "credits_after": produced.credits_after,
        "style": distilled.style,
    }))
    .map_err(|e| e.to_string())
}

fn tool_feedback(verdict: &str, fixture: &str) -> Result<String, String> {
    let cfg = ns_cerebro::Config::load().map_err(|e| e.to_string())?;
    if !cfg.taste.write_back {
        return Err("taste write-back is OFF (default). The operator must set [taste] \
                    write_back = true in ~/.config/neuralsymphony/config.toml first."
            .into());
    }
    let v = ns_cerebro::Verdict::parse(verdict)
        .ok_or("verdict must be loved | kept | skipped")?;
    if fixture.is_empty() {
        return Err("fixture path required (from compose --save-fixture)".into());
    }
    let raw = std::fs::read_to_string(fixture).map_err(|e| format!("reading {fixture}: {e}"))?;
    let graph: ns_core::MemoryGraph = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let piece = ns_core::compose(&graph).map_err(|e| e.to_string())?;
    let mut tag_counts: std::collections::BTreeMap<&str, usize> = Default::default();
    for m in &graph.memories {
        for t in &m.tags {
            *tag_counts.entry(t.as_str()).or_insert(0) += 1;
        }
    }
    let mut themes: Vec<(&str, usize)> = tag_counts.into_iter().collect();
    themes.sort_by_key(|&(t, c)| (std::cmp::Reverse(c), t));
    let themes: Vec<String> = themes.into_iter().take(3).map(|(t, _)| t.to_string()).collect();
    let label = format!("{} memories from {fixture}", graph.memories.len());
    let report =
        ns_cerebro::record_feedback(&cfg.cerebro, piece.mapping_version, v, &label, &themes)
            .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&json!({
        "memory_id": report.memory_id,
        "procedure_id": report.procedure_id,
        "new_procedure_salience": report.new_procedure_salience,
        "outcomes": report.outcomes,
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
        json!({
            "name": "ns_render_full",
            "description": "Dream voice: distill a composition into a style prompt and produce a full track via Sonus/Suno. SPENDS CREDITS unless dry_run=true (dry_run shows the distilled prompt free — use it first). Pick one source: window/episode/thread/everything.",
            "inputSchema": obj(json!({
                "window": { "type": "string" },
                "episode": { "type": "string" },
                "thread": { "type": "string" },
                "everything": { "type": "boolean" },
                "agent": { "type": "string" },
                "title": { "type": "string" },
                "style_pct": { "type": "integer", "description": "0-100 style adherence, default 65" },
                "weirdness_pct": { "type": "integer", "description": "0-100, default 50" },
                "dry_run": { "type": "boolean", "description": "true = show prompt, spend nothing" }
            }), &[]),
        }),
        json!({
            "name": "ns_feedback",
            "description": "Taste loop (opt-in; requires taste.write_back in the operator's config): record loved/kept/skipped on a composition's saved fixture. Writes scoped, ns-internal-tagged records to cerebro — the graph drifts, the mapping stays deterministic.",
            "inputSchema": obj(json!({
                "verdict": { "type": "string", "description": "loved | kept | skipped" },
                "fixture": { "type": "string", "description": "path from compose --save-fixture" }
            }), &["verdict", "fixture"]),
        }),
    ]
}
