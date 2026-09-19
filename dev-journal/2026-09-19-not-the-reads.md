# 2026-09-19 — It wasn't the reads

Jason: *"Can we go back to using a db instead of files if reads are choking us?"*

Two corrections, and then the actual fix.

## There was never a db

`dmml` has never used one — no sqlite, no postgres, nothing, in any commit. The memory is
probably `written-world`, which does keep state in Cloudflare KV. Worth saying rather than
playing along with "go back to".

## And reads were not choking us — I said they were without measuring

Yesterday I wrote that "round cost is driven by world FILES, not machines — 327
accumulated commit files by round 12, each re-parsed per scan". That was an inference
from watching rounds get slow, presented as a finding. It is wrong.

```
A) vary WORLD FILES, candidates fixed:
     20 world files ->   0.07s
     60 world files ->   0.03s
    120 world files ->   0.06s
    200 world files ->   0.07s
    278 world files ->   0.08s

B) vary CANDIDATES, all 278 world files:
     10 candidates ->   0.13s
    144 candidates ->   0.14s
```

**Flat on both axes.** Reading and materializing 278 commit files costs 0.08s. A database
would have replaced the one part of this system that was already fast.

## What was actually slow

```
scan_candidates (BATCH, 144 cands, 278 worlds)   0.15s
ONE dry_fire                                     0.15s
group_into_generations(12)                       6.21s
```

**Grouping twelve candidates cost forty times the entire round's scan of a hundred and
forty-four.** And it is quadratic, so it got worse exactly as the world got interesting.

The cause is not reads and not volume — it is **process spawns**. The conflict probe asks,
for each pair, "if A fires, is B still legal and unchanged?" and it was answering with one
`fire-transition` subprocess per pair, each paying full materialization to answer a single
question.

The driver's own docstring called this two days ago:

> *"O(candidates²) extra dry-fire subprocess calls in the worst case, which is fine at the
> scale every scenario built so far actually reaches; a real, disclosed cost to revisit if
> the cannon ever produces enough candidates in one round to make that quadratic cost
> matter."*

It does now.

## The fix is the one that already worked, applied to the step it skipped

`scan-candidates` exists because the round's scan had the same disease — N materializations
for N questions — and batching it behind one snapshot gave 141×. The grouping probe was
simply never moved over.

So: a candidate entry may now carry `extra_world`, commit files applied **on top of** the
shared snapshot for that candidate alone. The base world materializes once for the whole
call; each distinct probe file is parsed once however many pairs layer it; and N² probes
become one process with N² cheap snapshot extensions.

| | before | after |
|---|---|---|
| group 12 candidates | 6.21 s | **0.13 s** |
| group 23 candidates | — | **0.28 s** |
| group 23, full comparison | 23.52 s | **0.31 s** (76×) |

**And the grouping is identical.** Run both paths over the same 23 candidates and compare
the partitions: 22 groups either way, same members. The per-probe fallback is kept, so a
checkout without `scan-candidates` still groups correctly — just slowly, which is what it
was.

## What I'd take from this

The obvious diagnosis was available, plausible, and wrong, and it survived a day because I
never profiled it. The thing that made it visible was an implausible number in the other
direction: 278 files in 0.08s could not be the problem.

A database would have been a large change that made nothing faster. The actual cost was
one function still doing what the rest of the loop had already stopped doing.

## Still open

- The probe is still **O(n²) in entries**, just cheap ones now. At a few hundred legal
  candidates in one round that will matter again, and the next move is pruning pairs that
  cannot conflict rather than making each pair cheaper.
- `render-prose` and `list-candidates` are still one subprocess per round each. Small at
  this scale, unmeasured at the next.
