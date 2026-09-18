# 2026-09-18 — The orphans, and a chooser that drifts

Jason, after reading the fertility numbers: *"fix the orphans - and start
using Brownian motion or Perlin noise for the dry run chooser to cancel bias."*

## Where the numbers pointed

Counting what actually fired, rather than what existed, found two opposite
pathologies in the two demo worlds:

| | `cannon-grow` | `quarry-delve` |
|---|---|---|
| machines that ever fired | 48 / 49 | 3 / 12 |
| transitions that ever fired | 49 | 8 / 36 |
| firings | 49 | 30 |
| max firings of any one transition | **1** | 8 (its cap) |
| substance flows (`?unit` as effect subject) | **0** | 18 |

`cannon-grow` is a **ratchet** — broad, and nothing ever happens twice. With
zero substance flows in 49 machines, the rhizome half of `check-fertility` is
structurally silent there; it can only report on reachability.
`quarry-delve` is the inverse: narrow, but what moves, cycles. Architecture
without substance; substance without architecture.

## The orphans

Every one of the 11 bred rooms carried the same corpse — a `fall(unit)` whose
effects named `?rock` with no guard to bind it, and whose formal `unit` was
used nowhere. Firing both exits of one settled that it was dead under any
chooser:

```
fall → refused -- an effect's asserted value term did not resolve: TermBind "rock"
rest → refused -- transition's guards do not currently hold
```

`rest` goes `spent -> gathering`; the machine is seeded in `gathering`. No way
out of its initial state and a broken forward edge.

**Injected by `Chimera`** (A's guards with B's world effects — B's binders
don't come along), then **inherited forever**, because parent A of each
generation is the last offspring. One bad cross in generation 0 rode every
later cross.

### The repair, and the call it required

A `?binder` is **a variable, not a condition.** The surface writes both as
`guard`, but `guard room/x \`cleared\` mark/yes` is a test the machine must
pass, while `guard ?rock \`in\` quarry/north` is a *query* handing a witness to
whatever comes after. Chimera's contract is A's conditions with B's
consequences — and the query supplying a witness to B's consequence **is part
of that consequence**. So it travels with the effect that needs it.

The alternative was to drop any effect A's guards can't satisfy. That also
yields a fireable offspring, and yields one that does nothing: crossing any
binder-driven machine as B would reduce chimera to lifecycle, deleting the
mode's whole point. Dropping stays as the fallback in `pruneOrphans`, for a
binder no donor guard supplies. A vestigial param goes the same way — the Jev
driver reads params to decide what kind of decision it faces, so a param
standing for nothing is a question about nothing.

`pruneOrphans` runs last over every transition whatever mode made it, which is
what stops the inheritance.

**`room/g5`, before and after** — the same machine, repaired:

```
  transition fall(unit)            transition fall()
    gathering -> spent               gathering -> spent
                                     guard ?rock `in` quarry/north
    assert self `took` ?rock         assert self `took` ?rock
    retract ?rock `in` quarry/north  retract ?rock `in` quarry/north
```

## The chooser

`--dry-run` took `group[0]` — the first legal candidate, the first unmapped
edge, the first binding option. That is not "no chooser", it is a chooser with
a very strong opinion, and the opinion is list order. Several bred rooms never
fired all run, not because they couldn't but because a sibling was listed ahead
of them every round. Reading that as "those machines are dead" is reading the
sort order as biology.

**Perlin, and deliberately not a hash.** A hash would de-bias it — uncorrelated,
uniform, fine — but a hash is memoryless and a real chooser is not. Jev has
intent that persists: a crew that has been digging keeps digging a while, then
its attention moves. Perlin is smooth, so sampling it along a time axis gives
exactly that. A dry run then rehearses the shape of a live run rather than the
shape of a shuffle.

Measured against a plain hash over 200 rounds: mean run-length **1.77 vs 1.18**.
It does stick.

Each option's **lane** comes from its name, never its index — that is what
kills positional bias. Lanes also slide sideways a little each round
(`DRIFT_LANE_SCALE`), because without it each option sits on one curve forever
and some curves simply run higher: not positional bias, but bias. Swept
0.0–0.3 over 120 key-sets rather than one draw — on a single draw 0.1 looked
twice as good as 0.05, and averaged it is not. Mean spread 0.88 → 0.60,
run-length 1.88 → 1.84; flat from 0.02 to 0.15, so the constant is not
load-bearing. The residual 0.60 does not go away at any setting: argmax over a
smooth field is lumpier than uniform, and that is the price of coherence.

Still fully deterministic — fixed permutation table, hand-rolled LCG shuffle,
no `random` module anywhere, so nothing else seeding the global RNG can perturb
a rehearsal.

## What the two changes did, measured

`quarry-delve`, 20 rounds, same config:

| | orphans + `group[0]` | repaired + `group[0]` | repaired + drift |
|---|---|---|---|
| transitions ever fired | 8 | 16 | 17 |
| firings | 30 | 33 | **38** |
| machines dead *in that run* | 9 | 5 | 5 |

The five still dead are **a different five each time** — and across the three
runs, **11 of 12 machines fire under at least one chooser.** They are losing a
race for one rock at a time, which is the world working, not machines being
stillborn. `cannon-grow` is unchanged (49 machines, no binders anywhere, so
nothing to repair) — the fix is targeted, not broad.

## The real remaining limit, and it is neither of those

11 bred rooms, **3 distinct shapes.** The lineage reaches a fixpoint at
generation 1 and mints copies of it forever.

`check-fertility` already said so: its walk halts the moment a shape repeats,
and on these seeds it halts at g1. This is the periodicity that module's
haddock argues for — crossover never invents a transition, so the structure
space is finite and the walk is eventually periodic. It predicted this before
it was measured.

So the fertilizer that remains is not a bug. It is the **breeding policy**:
parent A = newest offspring, B cycles the seeds, modes cycle — a 2-cycle by
construction. Three knobs, all still mine.

## Also fixed

`check-fertility --generations N` had never worked: `parseArgs` paired each arg
with the one *after* it and then ignored the pair, so the flag was dropped and
its value was not — `--generations 8` fed `8` through as a seed path and died
on `openFile`. Found by using it.

## Still open

- **The breeding policy is a 2-cycle.** The lineage is the limit now, not the
  crossover. Parent selection and mode selection are deterministic knobs.
- **`cannon-grow` has no substance at all.** Every flow finding so far is from
  quarry-delve's 12 machines, not the 49-machine dungeon. A seed with both is
  what would let the two halves of the fertility analysis speak about one world.
- **No live Jev run** of the connective loop, the rain, or the drift chooser.
