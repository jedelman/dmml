# 2026-09-19 — Pruning the probe

Jason: *"Oh hell yeah buddy. Let's do that pruning before it gets lost."*

The sequel to [not-the-reads](2026-09-19-not-the-reads.md), which ended on a promise:
batching made each conflict probe cheap, but the probe set was still quadratic, and the
next move was to stop running the ones that cannot matter.

## The question the probe was asking badly

`group_into_generations` asks, for every ordered pair of legal candidates, *"if A fires,
is B still legal and unchanged?"* — and it answered by simulating. Simulation is the
right answer when the two firings might touch each other. It is a very expensive way to
learn that a kiln and a bond on the far side of the world have nothing to say to one
another.

And nearly all of them don't. Firing A can only change B's verdict if something A
**writes or spends** is something B **reads, cites or writes**. That is a set
intersection, and both sides were already sitting in memory.

## Three sets, and the one that makes it sound

`touches(output)` — every `(subject, predicate)` a firing **changes**. Both halves of the
commit count: the asserted facts, and the `consumes` block. A retract doesn't print as an
assertion; it lowers the fact it spent into provenance. A firing that only consumes is
exactly as capable of invalidating someone as one that only asserts.

`depends_on(cand, output)` — every `(subject, predicate)` a firing **reads**, with `None`
meaning *any subject*. Three sources, and dropping any one makes the pruning unsound
rather than merely weak:

- its **guards**, which decide legality and appear nowhere in the output — read back off
  the machine file. A guard over a `?binder` or `$param` subject matches anything, so it
  records as a wildcard: a write to *any* subject with that predicate could change which
  witness it finds.
- what it **consumes**, because the provenance citation names those facts exactly, and a
  change there changes the output even when the firing stays legal.
- what it **asserts**, because two firings writing the same `(subject, predicate)`
  interact whatever their guards say.

`may_conflict` is the intersection, conservative in every doubtful case: a wildcard read
matches any subject, and a guard this reader cannot parse is recorded as reading
everything. A missed conflict corrupts a round. A missed pruning costs a probe.

## What it bought

Probes actually run, versus pairs proven unable to interact:

| scenario | probes run | pruned | wall clock before → after |
|---|---|---|---|
| grouping 23 candidates | 5 | **248** | 0.258 s → 0.086 s |
| `quarry-delve` full run | 90 | 290 | 13.7 s → **7.5 s** |
| `cannon-grow` full run | 35 | 188 | 44.3 s → **38.6 s** |
| `cannon-fanout` full run | 15 | **1372** | 52.8 s → **11.6 s** |
| `kiln-delve` full run | 15 | 92 | 5.5 s → **3.8 s** |

`cannon-fanout` prunes 99% of its pairs, and that is the shape of the win: the denser the
round, the larger the fraction of pairs that are strangers to each other.

## The check that makes it a fix and not a hope

An optimization that changes the answer is a bug with a stopwatch attached. So both paths
run, on all four scenarios, and the transcripts are compared line for line — the pruned
driver against the same driver with `may_conflict` monkey-patched to always return `True`,
which is the pre-pruning behaviour exactly.

**Identical on all four.** Every binding, every interest score, every generation boundary,
every minted machine. The dry-run chooser is deterministic, so this is a real equivalence
check over the whole run and not just over the partitions.

## Regression

14 Haskell selftests and demos green; `check-prose-coverage` still 8/8 on the closed
`cannon-fanout` world; `minting-params-selftest.py` green; four driver dry-runs green.

## Still open

- Still **O(n²) in the intersection**, which is now the cheap part — but at a few thousand
  candidates the pair loop itself becomes the cost, and the answer there is an index from
  predicate to the candidates that read it, not a faster loop.
- The guard reader is a regex over the machine text. It is conservative when it fails, so
  it cannot be wrong in the dangerous direction, but it is a second parser for a language
  that already has one. `scan-candidates` could report a transition's read set directly.
- `render-prose` and `list-candidates` are still one subprocess per round each.
