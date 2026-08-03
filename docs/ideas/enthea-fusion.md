# Idea: the Enthea fusion — the score stream

Filed 2026-08-04 (Andre + Fable, the session after the colony panel). Status:
agreed direction, waiting on NS M0–M2. Companion note lives in
`Enthea-RS/docs/score-input-idea.md`.

## The insight

Enthea — like Milkdrop and every visualizer since — **infers** structure from
PCM after the fact: FFT, spectral flux, "was that a drop?", a 10-second build
heuristic. NeuralSymphony doesn't have to infer anything: **it wrote the
score.** It knows the exact note onsets, which memory is sounding, its valence
and salience, when the episode cadences, what the activation heat is right now.

So the fusion is not an app. It is a **semantic score stream**: NS emits
"now playing" events; Enthea grows a second input alongside its FFT and stops
guessing. Mode selection from memory type, palette from valence (Enthea
already thinks in OKLab), field-PDE parameters from activation heat, the drop
arsenal firing on episode cadences because it was *told*, not because it
detected. An isolated memory renders as a lone particle outside the lattice.

The end state Andre named: **an organic, dynamic visual portrait of the state
of an apex at any given moment** — the colony's dream suite on the morning
kiosk, the week's memories dancing while they play. Milkdrop never had this
and never could: the visualizer knows what the song means because the same
mind made both.

## Decisions already made (2026-08-04)

- **No third app.** The fusion is a protocol, not a codebase. NS stays the
  composer; Enthea stays the renderer; a player shell can be a thin thing
  much later, after both engines work.
- **Not a Sonus fold-in.** License seals what taste already suggested:
  Enthea is AGPL-3.0 (inherited from upstream ENTHEA, and its shaders are a
  structural translation — derivative forever). Folding it into any MIT repo
  is off the table.
- **The process boundary is the license firewall.** AGPL propagates through
  linking, not across sockets. NS keeps its permissive license, Enthea keeps
  AGPL, ApexOS-RS keeps whatever it wants; they meet over the stream. The
  protocol *spec* lives here (permissive), so anyone can implement either end.
- **Land NS v1 first.** Everything waits on the Echo voice existing (PRD
  M0–M2). This idea slots into the existing PRD milestones; it does not
  reshuffle them.

## The score stream — protocol sketch (score_v0, to be pinned at M2)

- **Transport:** newline-delimited JSON. Live: Unix socket (loopback-only by
  construction). Offline: a `.score` file — identical NDJSON with millisecond
  offsets — which doubles as the **fixture format**: golden `.score` files in
  both repos, so Enthea can build and test its score input against canned
  streams before NS M1 even exists. Parallel development, integration as a
  formality.
- **Event vocabulary (draft):**
  - `score_meta` — mapping version, key, bpm, mode (timeline/now/dream/episode)
  - `note_on` / `note_off` — memory_id, memory_type, salience, valence,
    intensity, isolated (bool), pitch, velocity
  - `phrase` — episode_id, phase: `begin` | `cadence`
  - `section` — graph-component / movement id
  - `heat` — activation-heatmap summary (N bucketed values), for `now` mode
- **Timing:** live events are emitted at play time (Enthea applies with small
  smoothing; it still hears the audio via its existing loopback capture, so
  sync tolerance is visual-perceptual, ~tens of ms). Offline renders share a
  clock between `.score` and WAV.
- **Versioned like `mapping_v1`:** `score_v0` is a contract; changes get a
  new version, goldens pin both.
- **Where the types live:** a tiny `score-protocol` crate (or spec-only +
  hand-rolled NDJSON on each side) so Enthea does not depend on NS crates —
  keeps the license boundary and the dependency graph clean. House religion
  applies: both ends deserialize the same shapes, nobody string-matches.

## Work split

**NS side (this repo):** the emitter — `ns-cli play` streams events in real
time; `compose --emit-score out.score` writes the offline form. Slots into
**M2** (where the API/MCP surface already lands). `now` mode heat events are
the M4 live story — M4's "viz client" is hereby *named*: it's Enthea.

**Enthea side:** a score-input module behind `--score <socket|file>`; semantic
state ring buffer; new uniforms (valence, salience-weighted intensity, episode
phase, per-type activity, isolation); a mapping policy (type → mode family,
valence → palette, heat → field params, cadence → drop trigger). The FFT path
stays untouched — with no `--score`, Enthea remains exactly the faithful
music visualizer it is today. The score input is additive divergence from the
upstream port; note it honestly in Enthea's README when it lands.

## v1 — the demo that counts

`neuralsymphony compose --window 7d` plays the week back while Enthea, fed
`--score` plus its normal loopback audio, renders it — palettes shifting with
valence, a drop landing on the episode cadence, one lonely memory drifting
outside the lattice. If that demo gives goosebumps, the thesis holds.

## Open questions

1. Socket vs stdin-pipe for live transport (socket favored: reconnect, kiosk).
2. Heat in-band vs separate channel (in-band favored until proven noisy).
3. Sync tolerance target — measure, don't guess; fix at M2 integration.
4. When NS gets its HTTP port (M2): pick one nobody uses (8765 is taken
   twice over, ask the prototype), and add NS + score socket to Prefrontal's
   colony roster in the same commit.
