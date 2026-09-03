# DMML — Desiring-Machine Markup Language

A portable, substrate-agnostic ontology for declarative, content-
addressed world models: what's true, and rules for how truths change,
expressed as `commit` blocks that `consume` and `produce` facts over a
graph — never edited in place, only ever extended.

Extracted from [written-world](https://github.com/jedelman/written-world)
(a text-adventure engine built on DMML, running on atproto today), once
its ontology and runtime materializer turned out to already be mostly
substrate-blind. This repo is that ontology, standing on its own,
designed to run on either atproto or iroh-docs underneath — see
[`ARCHITECTURE.md`](ARCHITECTURE.md) for the real crate boundary and
what's still open design work.

## Crates

- **`dmml`** — the grammar, parser, interpreter, and validation.
- **`dmml-runtime`** — the materializer (an oxigraph-backed world graph)
  and the `Substrate` trait a concrete backend has to satisfy.
- **`dmml-substrate-kit`** — substrate-specific strategies (today: the
  atproto CID scheme) and shared testing tools.

written-world remains the reference, atproto-backed consumer of this
ontology.

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
