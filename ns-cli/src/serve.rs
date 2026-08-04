//! The HTTP face (axum, loopback-only by enforcement, not convention).
//! Port 7664 — "SONG" on a phone keypad, the house naming joke continued.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::source::GraphSpec;

pub const DEFAULT_BIND: &str = "127.0.0.1:7664";

pub fn run(bind: &str) -> Result<()> {
    let addr: SocketAddr = bind.parse().with_context(|| format!("bad bind address {bind:?}"))?;
    // The garden invariant, enforced: composing a mind stays on this machine.
    if !addr.ip().is_loopback() {
        bail!("refusing to bind {addr} — neuralsymphony serves loopback only");
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let app = Router::new()
            .route("/health", get(health))
            .route("/compose", get(compose))
            .with_state(Arc::new(()));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("binding {addr}"))?;
        println!("neuralsymphony up — http://{addr} (GET /compose?window=7d&format=json|mid|wav|score)");
        axum::serve(listener, app).await?;
        Ok(())
    })
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "neuralsymphony" }))
}

#[derive(serde::Deserialize)]
struct ComposeParams {
    window: Option<String>,
    episode: Option<String>,
    thread: Option<String>,
    #[serde(default)]
    dream: bool,
    #[serde(default)]
    everything: bool,
    agent: Option<String>,
    /// json (default) | mid | wav | score
    format: Option<String>,
}

type ApiError = (StatusCode, String);

async fn compose(
    Query(p): Query<ComposeParams>,
    State(_): State<Arc<()>>,
) -> Result<impl IntoResponse, ApiError> {
    let spec = GraphSpec {
        fixture: None,
        window: p.window,
        episode: p.episode,
        thread: p.thread,
        dream: p.dream,
        everything: p.everything,
        agent: p.agent,
    };
    let format = p.format.unwrap_or_else(|| "json".into());
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let (graph, label) = spec.resolve()?;
        let piece = ns_core::compose(&graph)?;
        Ok((graph, piece, label, spec.mode_label()))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::BAD_GATEWAY, format!("{e:#}")))?;
    let (graph, piece, label, mode_label) = result;

    match format.as_str() {
        "json" => {
            let tracks: Vec<_> = piece
                .tracks
                .iter()
                .map(|t| json!({ "name": t.name, "channel": t.channel, "notes": t.notes.len() }))
                .collect();
            Ok((
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "mapping": piece.mapping_version,
                    "source": label,
                    "memories": graph.memories.len(),
                    "notes": piece.note_count(),
                    "movements": piece.movements.len(),
                    "seconds": piece.len_seconds(),
                    "bpm": piece.bpm,
                    "tracks": tracks,
                }))
                .into_response(),
            ))
        }
        "mid" => {
            let bytes = ns_midi::render(&piece)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(([(header::CONTENT_TYPE, "audio/midi")], bytes.into_response()))
        }
        "score" => {
            let ndjson = ns_score::to_ndjson(&piece, &mode_label);
            Ok(([(header::CONTENT_TYPE, "application/x-ndjson")], ndjson.into_response()))
        }
        "wav" => {
            let tmp = std::env::temp_dir().join(format!("ns-preview-{}.wav", std::process::id()));
            let bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
                ns_synth::render_wav(&piece, &tmp)?;
                let bytes = std::fs::read(&tmp)?;
                let _ = std::fs::remove_file(&tmp);
                Ok(bytes)
            })
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))?;
            Ok(([(header::CONTENT_TYPE, "audio/wav")], bytes.into_response()))
        }
        other => Err((StatusCode::BAD_REQUEST, format!("unknown format: {other}"))),
    }
}
