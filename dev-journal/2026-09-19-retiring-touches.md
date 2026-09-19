# 2026-09-19 — Retiring `touches`

Jason: *"Let's retire touches then?"*

The last regex in the conflict path. `depends_on` had just stopped guessing what a firing
*reads*; `touches` was still guessing what it *writes*, by scraping the rendered commit.

## Why this one was the worse shape, even though it was working

A commit does not say what it changed in one place. An assert is a fact line:

```
  crew/digger `took` rock/1
```

A retract is not a line at all. It appears further down, in a different block, as a
provenance citation:

```
  consumes
    fact local:world.dmml#fnv1a64:3e9c43f1df96644c
      rock/1 . in = quarry/north
```

Two regexes, two indent levels, two grammars, one `declare `-line exception — to recover
something `DMML.Fire` had already computed exactly. A reader watching only the asserted
lines would miss every retraction, and a firing that only *spends* invalidates people just
as thoroughly as one that only adds.

So `scan-candidates` now reports `writes` alongside `reads`, straight off
`ResolvedEffect`: `ResolvedAssert` and `ResolvedRetract` each contribute their resolved
`(subject, predicate)`. `ResolvedSpawn` contributes nothing — a spawned machine asserts and
retracts no facts; it becomes a machine *file*, and a machine file is not part of the fact
world anyone's guards walk.

Unlike a guard, an effect has no wildcard case: it either resolved at fire time or the
firing failed. Every subject here is concrete.

## This one was not a bug fix, and it is worth saying so

The regex and the parser were cross-checked before the regex was deleted — both computed,
both compared, on every firing of every round of all four scenarios:

```
quarry-delve    compared 140   mismatch 0
cannon-grow     compared 208   mismatch 0
cannon-fanout   compared 256   mismatch 0
kiln-delve      compared  80   mismatch 0
```

**684 firings, zero disagreements.** The guard reader replaced last round *was* wrong in
ways that mattered (it could not see the implicit state guard, and it widened bound params
to wildcards). This one was right. It was a second parser waiting to be wrong — which is a
reason to retire it, but a different reason, and the difference is worth keeping straight.

The probe and prune counts are byte-identical to the previous commit on all four scenarios,
as they should be when the answer did not change.

## Conservatism runs the other way here

`cand.reads is None` means "reads everything" — an unknown reader hears every write.
`cand.writes is None` means "writes everything" — an unknown writer reaches every reader.
Opposite defaults, same principle: the unknown side must be the one that costs a probe
rather than the one that skips it. `may_conflict` and `ReadIndex.readers_of` both
short-circuit on the write-side sentinel now.

In practice it only comes up on the per-candidate fallback path, where no scanner ran.

## Regression

Every Haskell binary that links `DMML.Fire` rebuilt (the change exports `predText`;
nothing else moved) and its selftest re-run: 14 green. `cascade-demo` green, so the
`fire-transition` CLI contract is intact. `check-prose-coverage` 8/8;
`minting-params-selftest.py`; `index-scaling-bench.py`; `conflict-pruning-selftest.py` on
all four scenarios; transcripts identical against the previous commit *and* against the
per-candidate fallback path.

## Still open

- `app/ListCandidates.hs` has a redundant `MachineStmt` import that fails a local
  `-Werror` build. Pre-existing, untouched here, and invisible to CI, which builds with
  `-Wall` alone. Noted rather than fixed, because it is nobody's business in this diff.
- `render-prose` and `list-candidates` are still one subprocess per round each.
- With both halves now structured, the conflict analysis no longer reads rendered text at
  all. The driver still does, elsewhere — `parse_machine_text` survives for the growth
  machinery, which is a bigger and much less mechanical job.
