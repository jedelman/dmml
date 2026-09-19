# 2026-09-19 — The maximal run, and Jev measured

Jason: *"measure those two, then fire the maximal run. For the maximal run — **measure
Jev's performance**."*

## The two measurements, first

**Per-round cost at scale.** 30 rounds, 22 seed machines growing to 149, **215s wall =
7.2s per round** with no Jev in the loop. Candidates grew 8 → 23 per round. Not a
bottleneck at this size; it is the `scan-candidates` path doing what it was built for.

**`max_questions` bit for the first time ever.** Rounds 29 and 30, at the cap of 60, with
1 and then 6 options going unasked. Two rounds of thirty, at the very end, as the world
got dense — exactly where it was predicted to bite and nowhere earlier. It reports the
truncation rather than swallowing it, which is what makes this a measurement and not a
surprise.

## The seed

A pool searched rather than collected: 600 samples, **22 machines, walk 131, fertile** —
mills, gates, herds, barges, shrines, a kiln, rain. `build-maximal-seed.py` then makes
them *able to act*: lifecycle states, satisfying facts for every literal guard, and **88
raw substance units** across the vocabulary the pool consumes. 141 facts, 27 predicates,
28 zero-parameter transitions handed over as candidates.

Two grammar landmines on the way: a commit **label** cannot contain a hyphen
(`commit maximal-seed` is a parse error at `-s`), and a pool handed over with no
candidates is inert, because the driver only registers candidates for machines *it* mints.

## The world

```
22 seeds -> 112 minted -> 134 machines
204 firings, 124 distinct transitions fired
built: bridge 81, feed 9, vista 9, imply 8, breed 4, replenish 1
257 relations offered; every round grew something
```

```
CIRCULATES: 3 strongly connected component(s), independent circuits 6
  {state grain/dry, state grain/milled}                       rank 1
  {refinedInto ingot/batch2, refinedInto ingot/tempered}      rank 1
  {yields relic/asSpur, yields sigil/g0, yields sigil/wVault}  rank 4
This world SUSTAINS, and sustains ITSELF.
```

**Six independent circuits across three components**, against a previous best of two.
Grain goes dry → milled → dry. Ingots temper and return. And a three-substance relic/sigil
component with **four** independent ways round it — cut any three edges and it still
circulates.

## Jev, measured — 30 live calls

```
questions/call : min 27   median 46   max 59
latency (s)    : min 0.49 median 0.61 max 0.80
input tokens   : min 6337 median 12992 max 16969
TOTAL          : 386,346 in, 53,014 out, 18.8s of Jev time
```

**The whole world cost under twenty seconds of model time.**

### The vendor claim is true, and true on the wrong axis

> *"All questions are evaluated in parallel, so adding more questions to a call typically
> doesn't add any latency."*

Tested against a real spread of 27 to 59 questions:

| | median latency |
|---|---|
| calls with ≤30 questions | 0.66 s |
| calls with ≥50 questions | **0.65 s** |

`correlation(questions, latency) = +0.29` — weak, and the medians say it is noise.
**The parallelism is real**, which is the assumption the entire fan-out design rests on,
and it had never been checked.

But:

```
correlation(questions, input tokens) = +0.99
```

**Questions are free in latency and perfectly linear in tokens.** For anyone paying, the
docs' claim is about the axis that does not cost anything. This project has been saying
since the beginning that the real budget is tokens and attention rather than wall clock —
that is now a measured fact about the API and not a stance.

### Jev leans; it does not commit

```
noul decisiveness (0 = shrug, 1 = certain): median 0.211, range 0.14-0.26
noul spread within a call:                  median 0.210, max 0.34
choice confidence:                          median 0.649
served model: jev-1.13.0
```

A typical `noul` answer is near 0.4 or 0.6, not 0.05 or 0.95 — **a weak but consistent
discriminator on questions of this kind**, and consistently weak rather than erratically
so: decisiveness never left the band 0.14–0.26 across 30 calls and over a thousand
questions.

That retroactively justifies a decision made earlier today on much thinner evidence. The
ranked sigmoid against a running baseline exists because 122 scores lived in 0.21–0.59;
with decisiveness measured at 0.21, **no absolute threshold was ever going to work**, and
the floor-and-band version would have been just as wrong, only later.

## Still open

- **`max_questions` now genuinely binds.** At the density this run reached, the cap is
  live and options go unasked. It needs either a bigger budget or a principled way to
  choose which options are worth asking about — the second is the interesting one, and is
  a ranking problem over a signal we can only get by asking.
- **Bridges dominate: 81 of 112.** The proposal mix is still lopsided even with
  `proposals_per_kind` raised — bridges are offered wherever two cleared nodes are
  unrelated, which is N-squared, while feeds and implies are gated on much scarcer
  structure.
- Round cost grows with world size and was measured only to 149 machines.
