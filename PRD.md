# NeuralSymphony-RS — PRD

*Founded 2026-08-01. The concept and name were produced by Qwen3.5-122B-A10B (UD-Q4_K_M,
MTP, 64k ctx) during its first hour of life on a rented pair of modded 48 GB RTX 4090s —
the ApexRouter garden campaign's R4 box — via an unconstrained "make something cool"
brief through a stock hermes-agent pointed at `127.0.0.1:8888`. Its Python scaffold is
preserved verbatim in `prototype/`. This document is the expanded, human-and-Claude
iterated version of that idea, for implementation in Rust.*

---

## 1. Vision

**The memory graph as a score that drifts with the mind feeding it.**

CerebroCortex already records what a mind (human operator or agent) has lived: memories
with type, salience, emotional valence and intensity, tags, links, episodes, activation
curves, nightly dream clusters. NeuralSymphony renders that structure as music — not as
a visualization gimmick, but as a second way of *knowing* a mind. You don't read the
week's logs; you hear whether the week was consonant.

Long-horizon thesis (the musician case): a person who lives with an ApexOS install
imprints taste, style and preference into their cerebro through ordinary use. Over
months, NeuralSymphony's output stops being generic — the graph becomes an
unreproducible personal synthesizer. Nobody else's memory sounds like yours. The same
holds for agents: each colony member with its own cerebro has its own sound.

And if the thesis under-delivers: it is still a genuinely fun novelty. That floor is
acceptable; the ceiling is an instrument.

## 2. The two voices (both are product, neither is optional)

| Voice | What | Properties |
|---|---|---|
| **Echo** | Deterministic graph → MIDI mapping, rendered locally | Pure function; same memories → same piece, byte-identical. Auditable, hermetic, offline, no external AI. *The music made from AI echoes, not by AIs.* |
| **Dream** | The mapping's structure distilled into a prompt for Suno via **Sonus-RS** | Full produced tracks; many-AIs-as-orchestra. Requires network + credits; always labelled as generated. |

The Echo voice is the identity of the product and the only lane the MVP must perfect.
The Dream voice rides on infrastructure that already exists (`tools/Sonus-RS`).

## 3. The mapping specification (the heart — a pure module)

Every rule below reads only fields cerebro already serves. The mapping is versioned
(`mapping_v1`) and deterministic: goldens in CI render fixed memory fixtures to fixed
MIDI bytes.

| Cerebro signal | Musical meaning |
|---|---|
| `memory_type` | Instrument family: episodic → strings (narrative lines), semantic → piano (chordal statements), procedural → percussion (procedures *are* rhythm), working → woodwinds (transient runs) |
| `salience` | Velocity and prominence in the mix |
| activation curve | Note envelope over time — a decaying memory audibly fades; reinforcement swells it |
| `emotional_valence` | Harmonic color: positive → lydian/major, negative → minor/phrygian, mixed → suspensions and modal interchange, neutral → dorian |
| `emotional_intensity` | Dissonance budget and dynamic range |
| links | Voice-leading: linked memories share and pass motifs; `common_neighbors` → chord membership |
| graph components | Movements. **Isolated memories play unaccompanied** — the fragmentation watchdog made audible; a lonely memory sounds lonely |
| tags / concepts | Leitmotifs: a stable hash of the tag seeds a motif; recurring tags = recurring themes across weeks |
| episodes | Phrases with beginnings and cadences; episode steps → melodic contour |
| threads | Tracks / stems |
| dream clusters (`dream_run`) | The nightly suite: "what did the colony dream" as a morning piece |
| `created_at` vs. activation | Two time axes → two modes: **timeline** (chronological memoir) and **now** (ambient rendering of current activation heat) |

## 4. The taste-imprint loop (opt-in, M3)

Listening feedback — saved, replayed, skipped, exported — is written back to cerebro as
memories and procedure outcomes (`record_procedure_outcome` on motif/mapping choices).
Cerebro's own reinforcement machinery then *is* taste formation: future compositions
weight toward what survived. The instrument learns the player through the same organ
that learns everything else. Write-back is off by default and clearly scoped — the
product must never pollute a cerebro that didn't ask for it.

## 5. Architecture

House pattern (the Occipital shape): **standalone repo, one binary, three faces** —
usable by any human or agent with a cerebro, assimilable by ApexOS-RS later as a
provisioned sibling.

- `neuralsymphony` CLI — `compose --window 7d --mode timeline|now|dream|episode <id>`,
  `render`, `motifs`, `serve`, `mcp`.
- **MCP stdio** — `ns_compose`, `ns_render_full` (Sonus bridge), `ns_motif_of`,
  `ns_now_playing`; agents compose from their own memories.
- **HTTP API** (axum, loopback-default) — `/compose`, `/compositions`, `/ws` (live
  "now playing" driven by the activation heatmap).
- **Reads** cerebro REST (`:8765`), read-only by default. **Renders**: MIDI via `midly`
  (the hermetic core output), instant WAV preview via `fundsp` (pure-Rust synthesis, no
  ffmpeg, no models), full tracks via Sonus-RS. **Viz is a client, not a component**:
  ApexOS kiosk later; and the sleeper pairing — `imaginarium_craft_video` + memory
  imagery + this soundtrack = **the auto-generated music video of your week**.

Rust workspace: `ns-core` (mapping, pure, no I/O), `ns-midi`, `ns-synth` (fundsp),
`ns-cerebro` (client), `ns-sonus` (bridge), `ns-cli` (binary: CLI+MCP+API). House rules
as per the garden: no `sh -c`, no unwraps outside tests, hermetic tests against fixture
graphs, nothing writes into repo dirs at runtime.

## 6. MVP slices

| Slice | Delivers | Proof |
|---|---|---|
| **M0** | `ns-core` mapping_v1 + MIDI out + CLI compose from a JSON fixture graph | Golden MIDI files in CI; a fixture week renders identically forever |
| **M1** | Cerebro client + modes (window / episode / thread / dream) | Compose from a live cerebro; the dream-suite morning track |
| **M2** | fundsp WAV preview + API + MCP | An agent (this one, with its own cerebro) composes and plays back its week |
| **M3** | Sonus bridge (Dream voice) + opt-in taste write-back | A produced track from a memory cluster; feedback visibly re-weights the next composition |
| **M4** | Live mode (`/ws` ambient from activation heatmap) + Imaginarium video pairing | The week-in-review music video, generated end to end in the garden |

## 7. Non-goals (v1)

No DAW ambitions (export MIDI/stems; let real tools be real tools). No streaming
service, no cloud. No music-theory maximalism — mapping_v1 is deliberately simple and
versioned so later mappings can be A/B'd against the same graph. No always-on write-back.
Not an ApexOS plugin *first* — standalone first, assimilation second (the Occipital path).

## 8. Risks, honestly

- **Sonification is easy to do badly.** Mitigation: the mapping is small, versioned and
  golden-tested; taste iteration happens in mapping_v2+, not by patching v1 in place.
- **The imprint thesis is unproven.** Mitigation: the floor (novelty + diagnostics-as-art
  + the dream suite) already justifies the build; the thesis gets months to prove itself.
- **fundsp preview quality vs. expectations.** Mitigation: the preview is a sketch by
  contract; the Dream voice is the produced artifact.
- **Cerebro API drift.** Mitigation: `ns-cerebro` pins against CerebroCortex-RS versions;
  fixture-first testing keeps the core immune.

## 9. Provenance

Idea and name: Qwen3.5-122B-A10B, first hour of operation, R4 campaign box (2× modded
RTX 4090, Guangdong), 2026-08-01 — see `ApexRouter-RS/docs/GARDEN-RUNS.md` §R4.
Prototype preserved in `prototype/` as-built (buggy, unrun, historically significant).
Expanded to this PRD the same night by Andre + Claude (Fable 5).
