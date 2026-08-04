use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
use ns_cerebro::Mode;
use ns_core::MemoryGraph;

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
        /// only — no memory content ever leaves cerebro), so a live moment
        /// becomes replayable and golden-testable
        #[arg(long)]
        save_fixture: Option<PathBuf>,
        /// Where to write the MIDI file
        #[arg(long)]
        out: PathBuf,
    },
}

/// "7d" / "36h" / "2w" / "90m" → seconds.
fn parse_window(s: &str) -> Result<i64> {
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
        } => {
            let (graph, source) = if let Some(path) = fixture {
                let raw = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                let graph: MemoryGraph = serde_json::from_str(&raw)
                    .with_context(|| format!("parsing {}", path.display()))?;
                (graph, format!("fixture {}", path.display()))
            } else {
                let mode = if let Some(w) = &window {
                    let since = chrono::Utc::now().timestamp() - parse_window(w)?;
                    Mode::Window { since_unix: since }
                } else if let Some(id) = episode {
                    Mode::Episode { id }
                } else if let Some(id) = thread {
                    Mode::Thread { id }
                } else if dream {
                    Mode::Dream
                } else {
                    debug_assert!(everything);
                    Mode::All
                };
                let cfg = ns_cerebro::Config::load()?;
                let graph = ns_cerebro::fetch_graph(&cfg.cerebro, &mode, agent.as_deref())?;
                let label = match (&window, &agent) {
                    (Some(w), Some(a)) => format!("live cerebro · {w} · agent {a}"),
                    (Some(w), None) => format!("live cerebro · {w}"),
                    (None, Some(a)) => format!("live cerebro · agent {a}"),
                    (None, None) => "live cerebro".to_string(),
                };
                (graph, label)
            };

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
                source,
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
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_window;

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
