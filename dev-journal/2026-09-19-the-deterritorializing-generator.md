# 2026-09-19 — The deterritorializing generator

Jason: *"Build the generator!"* — after: *"I'm seeing a place for LLMs again in the
process, as Deleuze indicates - deterritorialization."*

## Why it was necessary, not decorative

The hardest measured fact in this project is the breeding fixpoint: 11 bred rooms, 3
distinct shapes, periodic by generation 1. `check-fertility`'s argument for why —
**crossover never invents a transition**, so the structure space is finite and the walk
is eventually periodic — **is also a proof of necessity.**

The same holds for the connective operators. `bridge`, `feed`, `replenish` and `vista`
all read the *flow graph* and act on substances already in it. They redistribute. No
sequence of them leaves the space they generate.

And the cost was measured, not theorised: **cannon-grow ran 40 rounds and 49 machines
with zero substance flows.** `check-fertility`'s entire rhizome half was structurally
silent. Not because the world was poor — cannon-fanout's seed names `draft/cold`,
`water/running`, `rubble/fallen`, `light/daylight` and three kinds of sound — but
because every operator was looking at the flow graph, and the flow graph was empty while
seven things sat in the *fact* graph as static attributes of places.

**That is where the boundary of the territory actually is**: not in the operators, in
the proposer's field of view. `cannon feed` builds the machine fine once someone thinks
to ask.

## The move

`latent_objects` reads objects back out of committed facts — excluding the reachability
convention's `mark/yes` and anything standing as a lifecycle state — and proposes
promoting one to matter:

> Nothing in this world is made of anything yet — it is all places and ways, and no
> matter moves through it at all. But the world does already say that way/east **has**
> water/running, and that way/south **has** rubble/fallen. This would make water/running
> a thing that can BECOME rubble/fallen: the first matter here, and the first
> transformation. **Nothing that exists can be rearranged into this — it has to be
> introduced.**

Nothing invented. Every object is a fact somebody wrote down. What is new is treating it
as something that can move.

## Caught on the way back down

The proposal goes through the same interest machinery as everything else, and the moment
the flow graph is non-empty the **rank-scored feeds take over**. Live, in two rounds:

```
r1: connect: transmute-water/running-to-rubble/fallen   (interest 0.52)
r2: connect: feed-rubble/fallen-to-water/running        (interest 0.52)   <- rank-scored
```

Before: *"none. No transition moves a unit, so there is no flow layer at all — this is a
pure reachability world."*

After: **CIRCULATES: 1 strongly connected component, independent circuits 1.**

Deterritorialize, then reterritorialize. Neither half works alone: pure enumeration is
provably periodic, pure generation is unverifiable. The rank measure is what tells a
productive line of flight from noise, and it only exists as of this morning.

## Two honest limits

**Jev did not discriminate the sensible proposal from the nonsensical one.** Round 1
offered `water/running → rubble/fallen` (water erodes, leaves rubble) and
`water/running → light/daylight` (nonsense). They scored **0.52 and 0.56** — the nonsense
one *higher*, within noise. The run took the sensible one **by the sigmoid draw, not by
judgment.** The discriminator has no traction on "is this a plausible transformation" in
a world it has no priors about. That is a real gap and it is not fixed by this commit.

**This is bounded deterritorialization, and the bound is architectural.** It escapes the
flow graph's territory by reading the fact graph. It can promote a thing the world
already names; it cannot name a new one. Unbounded would need a **generative** model, and
this stack has a discriminator — Jev's three primitives (`choice`, `score`, `noul`) all
select, none produce text. That is a limit of the architecture, not an oversight, and
naming it is more useful than pretending the fact graph is infinite.

## Still open

- The plausibility gap above. A `score` question with a rubric ("is this a transformation
  matter could actually undergo?") might do better than a bare `noul`. Untested.
- `bridge` is still proposed by the "two cleared nodes not already related" heuristic —
  the exact thing rank-scoring replaced for feeds.
- The breeding fixpoint itself is untouched. This generator opens the *substance*
  vocabulary; nothing yet opens the *transition-shape* vocabulary, which is where the
  2-cycle actually lives.
