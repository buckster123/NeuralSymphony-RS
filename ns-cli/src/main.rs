use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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
    /// Compose a piece from a memory-graph fixture (M0: timeline mode)
    Compose {
        /// Path to a memory-graph JSON fixture
        #[arg(long)]
        fixture: PathBuf,
        /// Where to write the MIDI file
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compose { fixture, out } => {
            let raw = std::fs::read_to_string(&fixture)
                .with_context(|| format!("reading {}", fixture.display()))?;
            let graph: MemoryGraph = serde_json::from_str(&raw)
                .with_context(|| format!("parsing {}", fixture.display()))?;
            let piece = ns_core::compose(&graph)?;
            let bytes = ns_midi::render(&piece)?;
            std::fs::write(&out, &bytes)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "{} · {} memories → {} notes on {} tracks · {:.1}s @ {} bpm → {}",
                piece.mapping_version,
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
        }
    }
    Ok(())
}
