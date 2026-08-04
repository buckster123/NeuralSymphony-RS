mod mcp;
mod serve;
mod source;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
use source::GraphSpec;

#[derive(Parser)]
#[command(
    name = "neuralsymphony",
    about = "Your memory graph, as music",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compose a piece — from a fixture file, or straight from a live cerebro
    #[command(group(ArgGroup::new("source").required(true)
        .args(["fixture", "window", "episode", "thread", "dream", "everything"])))]
    Compose {
        /// Path to a memory-graph JSON fixture (hermetic; no cerebro needed)
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Live: memories from the last DURATION — e.g. 7d, 36h, 2w
        #[arg(long)]
        window: Option<String>,
        /// Live: one episode's memories (id from cerebro's list_episodes)
        #[arg(long)]
        episode: Option<String>,
        /// Live: one thread's memories
        #[arg(long)]
        thread: Option<String>,
        /// Live: the nightly dream suite (waits on cerebro-side support)
        #[arg(long)]
        dream: bool,
        /// Live: the whole store
        #[arg(long)]
        everything: bool,
        /// Live: keep only this agent's own records (exact match)
        #[arg(long)]
        agent: Option<String>,
        /// Also write the fetched graph as a fixture (structural fields
        /// only — no memory content ever leaves cerebro)
        #[arg(long)]
        save_fixture: Option<PathBuf>,
        /// Where to write the MIDI file
        #[arg(long)]
        out: PathBuf,
        /// Also render an instant WAV preview (pure Rust, no fluidsynth)
        #[arg(long)]
        wav: Option<PathBuf>,
        /// Also emit the score_v0 semantic stream (.score NDJSON — what
        /// Enthea listens to)
        #[arg(long)]
        emit_score: Option<PathBuf>,
    },
    /// Serve the HTTP API (loopback only) — GET /compose?window=7d&format=json|mid|wav|score
    Serve {
        /// Bind address; must be loopback
        #[arg(long, default_value = serve::DEFAULT_BIND)]
        bind: String,
    },
    /// Serve the MCP stdio server (for agents: claude mcp add neuralsymphony -- neuralsymphony mcp)
    Mcp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compose {
            fixture,
            window,
            episode,
            thread,
            dream,
            everything,
            agent,
            save_fixture,
            out,
            wav,
            emit_score,
        } => {
            let spec = GraphSpec { fixture, window, episode, thread, dream, everything, agent };
            let (graph, label) = spec.resolve()?;

            if let Some(path) = &save_fixture {
                std::fs::write(path, serde_json::to_string_pretty(&graph)?)
                    .with_context(|| format!("writing {}", path.display()))?;
            }

            let piece = ns_core::compose(&graph)?;
            let bytes = ns_midi::render(&piece)?;
            std::fs::write(&out, &bytes)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "{} · {} · {} memories → {} notes on {} tracks · {:.1}s @ {} bpm → {}",
                piece.mapping_version,
                label,
                graph.memories.len(),
                piece.note_count(),
                piece.tracks.len(),
                piece.len_seconds(),
                piece.bpm,
                out.display(),
            );
            for t in &piece.tracks {
                println!("  {:<10} ch{:<2} {:>4} notes", t.name, t.channel, t.notes.len());
            }
            if let Some(path) = save_fixture {
                println!("  fixture saved (structure only, no content) → {}", path.display());
            }
            if let Some(path) = emit_score {
                std::fs::write(&path, ns_score::to_ndjson(&piece, &spec.mode_label()))
                    .with_context(|| format!("writing {}", path.display()))?;
                println!("  score_v0 stream → {}", path.display());
            }
            if let Some(path) = wav {
                ns_synth::render_wav(&piece, &path)?;
                println!("  wav preview → {}", path.display());
            }
        }
        Command::Serve { bind } => serve::run(&bind)?,
        Command::Mcp => mcp::run()?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::source::parse_window;

    #[test]
    fn windows_parse() {
        assert_eq!(parse_window("7d").unwrap(), 7 * 86_400);
        assert_eq!(parse_window("36h").unwrap(), 36 * 3_600);
        assert_eq!(parse_window("2w").unwrap(), 14 * 86_400);
        assert_eq!(parse_window("90m").unwrap(), 5_400);
        assert!(parse_window("7x").is_err());
        assert!(parse_window("-3d").is_err());
        assert!(parse_window("d").is_err());
    }
}
