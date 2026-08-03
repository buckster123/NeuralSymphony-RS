# mapping_v1 — the concrete numbers

The contract behind the goldens. Change any number here and you are writing
mapping_v2: bump the version, bless new goldens, keep this file honest.
The prose rules live in PRD §3; this is their arithmetic.

## Global

| Thing | Value |
|---|---|
| Tempo | 96 bpm, fixed |
| Resolution | 480 ticks/beat, bar = 4 beats = 1920 ticks |
| Canonical memory order | `(created_at, id)` — input record order is meaningless (tested) |
| MIDI shape | SMF type 1; conductor track (name = mapping version, tempo) + one track per used family |

## Families (memory_type → instrument)

| Type | Track | Channel | GM program | Base note |
|---|---|---|---|---|
| episodic | strings | 0 | 48 String Ensemble | C3 (48) |
| semantic | piano | 1 | 0 Acoustic Grand | C4 (60) |
| working | woodwinds | 2 | 73 Flute | C5 (72) |
| procedural | drums | 9 | — (GM percussion) | drum table |

Drum degree table (procedures are rhythm): kick 36, snare 38, closed hat 42,
open hat 46, ride 51, low tom 45, crash 49. Drum hits are 120-tick staccato.

## Valence → mode (per memory)

| Valence | Scale |
|---|---|
| ≥ 0.5 | lydian |
| 0.15 .. 0.5 | major |
| −0.15 .. 0.15 | dorian (neutral) |
| −0.5 .. −0.15 | aeolian |
| ≤ −0.5 | phrygian |

**Mixed** (|valence| < 0.15 *and* intensity > 0.6): suspensions — degrees
restricted to 1-2-4-5 over dorian.

## Motifs (tags → leitmotifs)

Seed = FNV-1a64 of the lexicographically-first tag (memory id if untagged).
Four notes per motif: degree_i = nibble_i of the seed (mod 7; mod 4 into the
sus set when mixed). Rhythm cell = bits 16–17 of the seed, one of four
bar-filling patterns: `[1,1,1,1]`, `[1.5,.5,1,1]`, `[.5,.5,1,2]`,
`[1,.5,.5,2]` beats. Same tag ⇒ same motif contour, forever — recurring tags
are recurring themes across weeks.

## Dynamics

velocity = round(40 + salience·87), per-note spread ±round(intensity·12)
alternating, clamped 1..127. Salience is prominence; intensity is range.

## Structure

- **Movements** = graph components (undirected links; unknown targets
  ignored), ordered by first canonical member; one-bar rest between
  movements. Movement root walks the circle of fourths: root = C + 5·idx
  semitones (mod 12).
- **Timeline mode** (M0's only mode): one bar per memory, sequential within
  its movement.
- **Isolated memories** (degree 0): half-bar of silence before and after —
  a lonely memory audibly alone.
- **Voice-leading-lite**: if a memory is linked to the previous one in
  sequence — or shares a neighbor with it — the previous memory's last
  motif pitch sustains a whole bar underneath (velocity −20, floor 20).
  Drums never sustain, and **same-family neighbors never sustain**: on a
  shared track a same-pitch collision lets the motif's NoteOff truncate the
  sustain in real players, and a voice under itself is mud regardless.
- **Episode cadence**: when a consecutive same-episode run ends, an extra
  bar resolves V (half bar) → I (half bar) on piano at velocity 70, in the
  closing memory's scale.

## Deferred from PRD §3 — loudly, not silently

Rules in the PRD's mapping table that mapping_v1 does **not** implement:

- **Activation curve → note envelope** (a decaying memory audibly fades):
  needs live activation data, which arrives with the M1 cerebro client.
  There is no `activation` field in the fixture model yet. mapping_v2
  material.
- **Episode steps → melodic contour**: only the cadence half of the
  episodes row exists in v1; step-driven contour joins the M1 modes work.
- **Threads → tracks/stems**: v1 tracks are instrument families; thread
  stems wait for a mode that reads threads at all.

## Bounds

The cursor is capped (`GraphError::TooLong`) so a graph can never overflow
the u32 tick space — same input must never panic in debug and wrap in
release. The renderer validates MIDI ranges and errors on deltas beyond
2^28−1 ticks instead of letting them fold silently. Both found by
adversarial review before the first commit; both under test.

## Determinism ledger

No clocks, no randomness, no HashMap iteration: BTree everywhere, canonical
sort first, FNV-1a for all variety, floats only in straight-line arithmetic.
Proofs in `ns-cli/tests/goldens.rs`: byte-identical goldens, double-compose
equality, input-order shuffling.
