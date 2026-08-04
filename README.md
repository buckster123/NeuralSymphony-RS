# NeuralSymphony-RS

**Your memory graph, as music.** CerebroCortex memories rendered as living compositions —
deterministic MIDI from the graph itself (the Echo voice) and full produced tracks via
Sonus/Suno (the Dream voice). Types are instruments, salience is dynamics, valence is
harmony, links are voice-leading — and an isolated memory sounds lonely.

Conceived and named by a Qwen3.5-122B running its first hour on rented modded silicon;
its original prototype lives in `prototype/`. See **PRD.md** for the real plan, and
`docs/ideas/enthea-fusion.md` for where this is headed: a semantic score stream driving
[Enthea-RS](https://github.com/buckster123/enthea-rs) — the visualizer that stops
guessing, because the same mind wrote the song.

## Status: M3 shipped — the Dream voice, and an instrument that learns

```sh
# Dream voice (SPENDS Suno credits; --dry-run shows the prompt free):
neuralsymphony produce --window 7d --dry-run
# → style: instrumental, flowing, lydian-bright and major mood, strings-led…
neuralsymphony produce --window 7d --style-pct 65   # the real thing, via Sonus-RS

# Taste loop (opt-in: [taste] write_back = true, hard-gated by default):
neuralsymphony feedback --verdict loved --fixture my-week.json
# → scoped, ns-internal-tagged records in cerebro; the graph drifts,
#   the mapping stays deterministic. Compose filters its own write-backs
#   out (anti-Larsen) — the instrument never plays songs about its reviews.
```

Reference audio (seeding Suno with the Echo rendering itself) is pinned in
the PRD and blocked upstream — Sonus-RS has no upload transport yet; the
demand note is filed in its repo.

## Previously: M2 — it plays, it serves, and it can explain itself

```sh
# one command, three artifacts: MIDI + instant WAV (pure Rust, no external
# synth) + the score_v0 semantic stream:
neuralsymphony compose --window 7d --out week.mid --wav week.wav --emit-score week.score
# loopback API (port 7664 — "SONG" on a phone keypad):
neuralsymphony serve   # GET /compose?window=7d&format=json|mid|wav|score
# agents: claude mcp add neuralsymphony -- neuralsymphony mcp
#   → ns_compose, ns_motif_of
```

`docs/score_v0.md` is the stream contract — the fusion channel where
Enthea stops inferring and starts *knowing* (palette from valence, drops on
told cadences, lone particles for isolated memories). Golden-pinned in CI
like everything else.

## Previously: M1 — it composes from a living brain

```sh
cargo build --release
# hermetic (no cerebro needed):
./target/release/neuralsymphony compose --fixture fixtures/week-01.json --out week.mid
# live (config: ~/.config/neuralsymphony/config.toml → your cerebro-mcp):
./target/release/neuralsymphony compose --window 7d --out my-week.mid
# mapping_v1 · live cerebro · 7d · 129 memories → 521 notes on 4 tracks · 543.8s @ 96 bpm
```

- `ns-core` — mapping_v1, pure and deterministic (docs/mapping_v1.md has the numbers)
- `ns-midi` — byte-deterministic SMF rendering (midly)
- `ns-cerebro` — the live client: MCP stdio for nodes/episodes, read-only
  SQLite for the links table (cerebro exposes no edge API), and **never
  `recall`** — reading a brain must not rewrite its activation state
- `ns-cli` — binary `neuralsymphony`; modes: `--fixture`, `--window 7d`,
  `--episode <id>`, `--thread <id>`, `--everything`, `--agent <id>`, plus
  `--save-fixture` (structure only, no memory content — a live moment
  becomes a replayable, shareable fixture)
- Goldens in CI: the fixture week renders identically forever; input order is meaningless

Next: **M2** — fundsp WAV preview, HTTP API + MCP, and the `score_v0`
stream that lets [Enthea-RS](https://github.com/buckster123/enthea-rs)
stop guessing.
