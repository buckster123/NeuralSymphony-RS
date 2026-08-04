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
    /// Dream voice: a produced track via Sonus/Suno — SPENDS CREDITS
    #[command(group(ArgGroup::new("source").required(true)
        .args(["fixture", "window", "episode", "thread", "everything"])))]
    Produce {
        #[arg(long)]
        fixture: Option<PathBuf>,
        #[arg(long)]
        window: Option<String>,
        #[arg(long)]
        episode: Option<String>,
        #[arg(long)]
        thread: Option<String>,
        #[arg(long)]
        everything: bool,
        #[arg(long)]
        agent: Option<String>,
        /// Track title (default: derived from the source)
        #[arg(long)]
        title: Option<String>,
        /// 0-100: how hard the distilled style text steers Suno
        #[arg(long, default_value_t = 65)]
        style_pct: u64,
        /// 0-100: Suno's weirdness dial
        #[arg(long, default_value_t = 50)]
        weirdness_pct: u64,
        /// Where the produced tracks land (default: sonus's download dir)
        #[arg(long)]
        out_dir: Option<String>,
        /// Show the distilled prompt and exit WITHOUT spending credits
        #[arg(long)]
        dry_run: bool,
    },
    /// Taste loop: record a verdict on a composition (needs taste.write_back)
    Feedback {
        /// loved | kept | skipped
        #[arg(long)]
        verdict: String,
        /// The composition's saved fixture (for themes + label)
        #[arg(long)]
        fixture: PathBuf,
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
        Command::Produce {
            fixture,
            window,
            episode,
            thread,
            everything,
            agent,
            title,
            style_pct,
            weirdness_pct,
            out_dir,
            dry_run,
        } => {
            let spec = GraphSpec {
                fixture,
                window,
                episode,
                thread,
                dream: false,
                everything,
                agent,
            };
            let (graph, label) = spec.resolve()?;
            let piece = ns_core::compose(&graph)?;
            let distilled = ns_sonus::distill(&piece, &graph);
            println!("style: {}", distilled.style);
            println!("scene: {}", distilled.description);
            if dry_run {
                println!("(dry run — no credits spent)");
                return Ok(());
            }
            let cfg = ns_cerebro::Config::load()?;
            let opts = ns_sonus::SonusOptions {
                command: cfg.sonus.command.clone(),
                args: cfg.sonus.args.clone(),
                env: cfg.sonus.env.clone(),
                model: cfg.sonus.model.clone(),
                timeout_secs: cfg.sonus.timeout_secs,
                style_pct: style_pct.min(100),
                weirdness_pct: weirdness_pct.min(100),
                title: title.unwrap_or_else(|| format!("NeuralSymphony · {label}")),
                download_dir: out_dir,
            };
            let produced = ns_sonus::produce(&opts, &distilled.style)?;
            if let (Some(b), Some(a)) = (produced.credits_before, produced.credits_after) {
                println!("credits: {b} → {a} (spent {})", b - a);
            }
            println!("task {} · {}", produced.task_id, produced.status);
            for f in &produced.files {
                println!("  produced → {f}");
            }
        }
        Command::Feedback { verdict, fixture } => {
            let cfg = ns_cerebro::Config::load()?;
            if !cfg.taste.write_back {
                anyhow::bail!(
                    "taste write-back is OFF (the default — composing must never pollute a \
                     cerebro that didn't ask). Enable with [taste] write_back = true in \
                     ~/.config/neuralsymphony/config.toml"
                );
            }
            let v = ns_cerebro::Verdict::parse(&verdict)
                .ok_or_else(|| anyhow::anyhow!("verdict must be loved | kept | skipped"))?;
            let raw = std::fs::read_to_string(&fixture)
                .with_context(|| format!("reading {}", fixture.display()))?;
            let graph: ns_core::MemoryGraph = serde_json::from_str(&raw)?;
            let piece = ns_core::compose(&graph)?;
            let mut tag_counts: std::collections::BTreeMap<&str, usize> = Default::default();
            for m in &graph.memories {
                for t in &m.tags {
                    *tag_counts.entry(t.as_str()).or_insert(0) += 1;
                }
            }
            let mut themes: Vec<(&str, usize)> = tag_counts.into_iter().collect();
            themes.sort_by_key(|&(t, c)| (std::cmp::Reverse(c), t));
            let themes: Vec<String> =
                themes.into_iter().take(3).map(|(t, _)| t.to_string()).collect();
            let label = format!("{} memories from {}", graph.memories.len(), fixture.display());
            let report = ns_cerebro::record_feedback(
                &cfg.cerebro,
                piece.mapping_version,
                v,
                &label,
                &themes,
            )?;
            println!(
                "verdict recorded · memory {} · procedure {} · salience {:?} · outcomes {}",
                report.memory_id,
                report.procedure_id,
                report.new_procedure_salience,
                report.outcomes.unwrap_or_default(),
            );
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
