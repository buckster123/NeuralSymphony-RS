# Prototype review — how the 122B did

Reviewed 2026-08-04 by Claude (Fable 5), from the preserved-verbatim scaffold
in `prototype/`. Context: Qwen3.5-122B-A10B, first hour of operation, stock
hermes-agent, "make something cool." Kept as the honest record — the museum
piece deserves a museum placard.

## The impressive half

For a first hour of existence: a coherent product concept with a name, a
five-phase plan, a working-shaped scaffold (FastAPI + Vue/Three.js + MCP
client to cerebro), a demo fallback when cerebro is absent (good instinct),
a test script, a start script, and an ARCHITECTURE.md whose mapping tables
the PRD still largely follows. The core insight — type → instrument,
salience → velocity, links → harmony — is in the original verbatim. The
Three.js scene is the most finished code: proper cleanup, resize handling,
legend, pulsing spheres. The sign-off credited "Andre & Hermes" — it knew
the harness's name, not yet its own.

## The bug ledger

- **The music app makes no sound.** Web Audio is in the stack diagram;
  `isPlaying` shows "Playing Symphony" over zero audio code. It plays
  silence for `duration` seconds, then stops.
- **The backend binds :8765 — cerebro's own port.** The client sits on the
  port of its data source. (A live demonstration of the colony panel's
  "port open identifies nothing" design note, one day before that note was
  written.)
- **Determinism claimed, randomness delivered.** "Deterministic note
  selection based on memory ID hash" — via Python's `hash()`, which is
  seed-randomized per process. Same memories, different tune every run.
  Right instinct, wrong primitive; `mapping_v1`'s goldens are the correction.
- **The memory graph implodes.** `node.position *= 0.999` per frame,
  commented "gentle orbit" — exponential decay; every sphere spirals into
  the origin within about a minute. Memory consolidation as gravitational
  collapse: accidentally the most on-theme bug on record.
- **The core endpoint can never succeed.** `generate_composition_from_memories`
  returns a dict; the route reads `.title` / `.note_count` as attributes →
  `AttributeError` on every call, swallowed into `{"error": …}`. The
  Pydantic models exist and are never used.
- **The frontend can't reach the backend anyway**: it GETs `/api/compose`,
  the backend POSTs `/compose`, and the vite proxy has no path rewrite —
  wrong method *and* wrong path.
- **Interfaces imagined, not read**: `list_memories` isn't a cerebro tool;
  the MCP result is treated as a plain dict; `command:
  "~/.local/bin/cerebro-mcp"` — the tilde never expands without a shell.
  `requirements.txt` ships redis, numpy, scipy, jinja2 (unused) and omits
  `mcp` — the one package the client imports.
  *Errata 2026-08-04: this review originally listed
  `affective/prospective/schematic` as hallucinated memory types. They are
  real (`cerebro/src/types.rs:29-38`) — the 122B knew the schema better
  than its reviewer. Corrected during the M1 source scout; the reviewer
  regrets the slander.*
- Both servers bind `0.0.0.0` — the garden invariant it most needed to
  absorb. The PRD's loopback-default corrects it.

## Verdict

Classic capable-mid-size-model signature: **the shape is right, the seams
are hallucinated.** Every interface it could imagine, it imagined plausibly;
every interface it needed to read, it guessed. But nothing in the ledger is
a bad *decision* — they are bad *facts*, and facts are cheap to fix. The
decisions (fallback demo mode, deterministic-mapping instinct, docs
discipline, phased plan) were good. Preserving the prototype verbatim while
rebuilding in Rust with goldens is the right response: keep the soul,
replace the body.
