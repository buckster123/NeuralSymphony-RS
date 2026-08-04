# score_v0 — the semantic score stream

The contract between the composer and any renderer (first customer:
Enthea-RS — see `docs/ideas/enthea-fusion.md`). NDJSON: one JSON object per
line, `type`-tagged. The `.score` file form doubles as the fixture format:
golden `.score` files pin this contract in CI exactly like the MIDI bytes
(`ns-cli/tests/goldens/week-01.score`). Changes mean `score_v1`.

`t` is always milliseconds from piece start (exact integer math from ticks).
Line one is always `score_meta`. Ties order: section < phrase < note_off <
note_on.

| Event | Fields | Meaning |
|---|---|---|
| `score_meta` | `version, mapping, mode, bpm, ticks_per_beat` | stream header |
| `section` | `t, index, root_offset, isolated` | a movement (graph component) begins; `isolated` = a lone memory playing unaccompanied |
| `phrase` | `t, episode_id, phase: begin\|cadence` | an episode run opens / resolves (V→I) |
| `note_on` | `t, track, pitch, velocity, memory_id, memory_type, salience, valence, mixed, isolated, role: motif\|sustain\|cadence` | a memory sounds — **this is the semantics PCM can't carry** |
| `note_off` | `t, track, pitch` | it stops |

What a renderer can do with this that FFT inference never could:

- palette from `valence`/`mixed` (the mapping already chose the musical
  mode from it — the visuals can agree instead of guessing)
- prominence from `salience`, not just loudness
- mode/family selection from `memory_type`
- fire drop effects on `phrase: cadence` — *told*, not detected
- render `isolated` memories as lone particles outside the lattice
- `memory_id` links a visual entity to the exact memory across the piece —
  the same memory pulses every time its motif returns

Consumption: read the `.score` file, or `GET /compose?...&format=score`
from the loopback API, or `--emit-score` alongside any compose. MIT-licensed
crate `ns-score` holds the serde types; consuming it or hand-rolling the
five shapes are both fine (that's why the spec is this short).

Deliberately absent in v0 (candidates for v1): `heat` events (live
activation, arrives with M4's now-mode), absolute wall-clock sync marks,
per-note motif/tag identity.
