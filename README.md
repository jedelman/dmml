# DMML — Desiring-Machine Markup Language

A declarative, content-addressed world-model ontology: what's true, and
rules for how truths change, expressed as `commit` blocks that `consume`
and `produce` facts over a graph — never edited in place, only ever
extended.

Extracted from [written-world](https://github.com/jedelman/written-world)
(an atproto-backed text world model built on DMML), once its ontology and
runtime materializer turned out to already be mostly substrate-blind.

## `dmml-hs` — the real interpreter

[`dmml-hs/`](dmml-hs/) is the canonical, current implementation (Haskell)
— grammar, parser, interpreter, machines, guards, effects, firing,
retroactive-consistency checking, citation-integrity checking, and a real
atproto client. `dmml-hs/SURFACE.md` is the grammar reference;
[`ARCHITECTURE.md`](ARCHITECTURE.md) covers its real module boundaries.

A real, standalone Cabal package (`dmml-hs/dmml-hs.cabal`) — a library
(`src/DMML/*.hs`) plus a real binary per real capability:

- **Materialize & query**: `render-snapshot`, `check-declared`,
  `check-divergence`, `check-citations`, `check-string-cap`.
- **Fire a transition for real**: `fire-transition` (also
  `retro-gate`/`retro-gate-demo`/`retro-chain-demo` for the
  whole-machine-set consistency gate, `guard-demo`/`governance-demo`/
  `retroconsistency-demo` for the underlying primitives).
- **Persistent checkpoints**: `checkpoint-rebuild`.
- **atproto**: `atproto-resolve` (handle/DID → PDS endpoint, optionally
  list a collection — unauthenticated), `atproto-publish` (authenticate
  + write a commit record), `atproto-pull` (pull a peer's new records,
  cursor-tracked), `atproto-delete`.
- **Surface/compliance tooling**: `surface-demo`, `compliance-check-
  surface`, `compliance-check-informed`, `entropy-sidecar`.

written-world is the reference, atproto-backed consumer — its
[`cli/`](https://github.com/jedelman/written-world/tree/main/cli) pins
`dmml-hs` as a git dependency and is the real, primary interface a player
actually runs.

## `dmml-agent-nucleus/`

The real "fork this" release vehicle for anyone standing up their own
DMML-backed world: `GRAMMAR.md` (the one-page spec, pointed at `dmml-hs`
as of 2026-09-04), `harness.py` (real atproto PDS commit minting),
`TERRITORIES.md` (self-registration), `discover.py`.

## Other real, currently-open work in this repo

- `spikes/iroh-chain-integrity/`, `android-poc/` — separate, still-open
  substrate/platform explorations, independent of the `dmml-hs`-is-
  canonical decision.
- `compliance*/`, `compliance-endurance/` — real endurance/compliance
  test harnesses run against `dmml-hs` binaries.

## Papers

Two outlines-in-progress under [`papers/`](papers/), neither drafted
yet:

- [`desiring-production-ontology/`](papers/desiring-production-ontology/OUTLINE.md)
  — DMML's commit model as a real implementation of Deleuze &
  Guattari's desiring-production, not a metaphor.
- [`text-world-model/`](papers/text-world-model/OUTLINE.md) — DMML as a
  distributed world model authored by heterogeneous, ephemeral agents
  and coordinated by a substrate that functionally occupies a
  meta-agent's role; also argues the underlying symbolic, explicit
  world-model comparison against learned latent/video world models.
