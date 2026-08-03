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

## Status: M0 shipped — the Echo voice exists

```sh
cargo build --release
./target/release/neuralsymphony compose --fixture fixtures/week-01.json --out week.mid
# mapping_v1 · 14 memories → 67 notes on 4 tracks · 45.0s @ 96 bpm
```

- `ns-core` — mapping_v1, pure and deterministic (docs/mapping_v1.md has the numbers)
- `ns-midi` — byte-deterministic SMF rendering (midly)
- `ns-cli` — binary `neuralsymphony`, composes from JSON fixture graphs
- Goldens in CI: the fixture week renders identically forever; input order is meaningless

Next: **M1** — the live cerebro client (window / episode / thread / dream modes).
