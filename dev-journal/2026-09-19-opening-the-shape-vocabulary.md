# 2026-09-19 — Opening the transition-shape vocabulary

Jason: *"Pull that one too — open the transition-shape vocabulary."*

One level below the substance generator. That one opens what the world is *made of*;
this opens what a machine can *be*.

## The measurable came first, for once

`check-fertility`'s lineage walk halts at the first repeated shape. **So walk length is a
direct test of whether the shape space opened** — no proxy needed. That decided the
design before any code.

## `cannon imply` — a shape no crossover can reach

Guard one predicate, assert a **different** one about the same unit, **consume nothing**.

```
transition implies()
  guard ?unit `has` water/running
  assert ?unit `yields` sound/echo
```

No existing operator expresses this: `feed` crosses objects *within* one predicate,
`vista` relates two places and touches no unit at all. And it is an articulation, not a
transformation — the unit keeps what it was and is now also something else. Water that
runs is thereby water that sounds.

## The control, which changed the claim

First numbers looked great and were confounded — adding *any* third seed lengthened the
walk, because the breeding policy cycles parent B over the seed list. So: add duplicate
shapes instead.

| pool | generations |
|---|---|
| 2 seeds, baseline | 2 |
| 4 seeds, **two duplicate shapes** | **2** |
| 4 seeds, rain + imply | **14** |

**Adding seeds does not lengthen the walk. Adding shapes does.** Seven times the
reachable structure space from one new atom in the pool.

## A correctness fix `imply` exposed

A non-consuming assert registered as a **source**, so every `imply` would have made its
world trivially "sustained". It is not a source: rain *mints* a unit that did not exist
(`$unit`), an articulation adds a property to a unit already present. Same
minting-vs-binding distinction as yesterday's rain fix, one layer up. `flowOf` now
returns `(consumed, minted, articulated)` and only minted productions count as sources.

Also fixed: the report printed `sinks (consumed, never produced)`, which is the opposite
of both what it computes and what the code's own haddock says a terminal is. Matter
*piles up* at a sink.

## Live: Jev refused it twice, then didn't

First live run: **0.19–0.36 across the board**, consistently below the transmutes and
below the run baseline. **Nothing was ever built.** Two real defects behind it —
`vista`'s `overlooks watch/keeper` fact was offering *"anything cold is thereby also the
keeper"*, which is not a proposition about the world; and the description said what the
shape *is* rather than what it *does*.

Reframed to name the consequence — *"right now `yields` is a dead word here… this brings
it alive, so everything the world has ever said with that word starts to matter"* —
scores went **0.30 → 0.47 mean, and two implies got built.**

## And then it cascaded, which is the actual result

```
r2  works/i4_1     imply-water/running-sound/wind      <- first machine anywhere
                                                          asserting ?unit `yields` …
r3  room/g8, g9    bred offspring                      <- crossover spreads the atom
r4  decay/d12_0    feed-sound/none-to-sound/wind       <- rank-scored; only proposable
r5  decay/d16_0    feed-sound/wind-to-sound/none          because `yields` is now a
                                                          substance axis at all
```

Final: **CIRCULATES, 2 strongly connected components, independent circuits 2** — one on
`has`, one on `yields`. The second axis did not exist before the new shape was
introduced, and no non-imply machine introduces `?unit yields` anywhere in the run.

Deterritorialize (a shape from outside the operator set) → the territory absorbs it
(crossover propagates the atom into the lineage) → reterritorialize (the rank measure
closes it into a circuit).

## What I got wrong, twice

**The "without the implies" control was invalid.** Deleting the `works_i*` files leaves
the walk at 11 either way — but the atom is *already in the bred offspring* by then, so
the control removes the parent and not the gene. The timeline above is the valid
evidence, not that comparison.

**And the 7× does not reproduce live.** It was measured on a 2-seed pool. In a
20-machine pool the marginal contribution of one more shape is invisible to walk length.
What *is* demonstrated live is the flow-axis cascade — rank 1 → 2 on an axis that could
not previously exist.

## Still open

- **Interest is a local signal; shape-opening has non-local value.** A reader does not
  want a new shape — it makes no round better. The world needs one, because it enlarges
  the option space for every later round. That mismatch is why the first framing scored
  0.3, and reframing only papers over it. The structurally honest answer is probably an
  explicit allocation, the way `unbidden_every` buys weather.
- `imply` adds no flow edge by design, so `rank_delta` cannot score it. Its value is in
  the shape space and nothing prices that at proposal time.
- The 7× number needs re-measuring on a pool large enough to matter.
