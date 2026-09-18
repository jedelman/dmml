# 2026-09-18 — An ambiguous guard becomes a question; and how far this runs

Jason: *"Yep, wire it in. Nice findings. How long could this run? 200 nodes? More?"*

## The wiring

`fire-transition` already refused an ambiguous `?binder` and printed every
candidate. The driver now reads that back and turns it into a real question in the
same batched Jev call as everything else.

The loop closes like this: the engine refuses and enumerates → the question rides
in the batch → the answer comes back as an ordinary `--param` → that param
**pre-binds the binder** → the transition fires on exactly that witness.

That last step needed a fix I'd gotten wrong. `fire-transition`'s own message said
*"narrow it with a `--param`"*, but `TermBind` resolved only from `ctxBindings`, so
a `--param` would have done nothing. `resolveTerm` now falls back to `ctxParams`,
which is what makes the advice true and the refusal answerable.

Also added a stable machine-readable contract on the refusal —
`ambiguous-binding: <var>` and one `candidate: <node>` per line — so a driver never
has to scrape English. Two audiences, one message.

**Bindings are per-firing.** A resolved binding is cleared the moment the candidate
fires (`Candidate.bound_params`). Left in place, the crew would keep trying to take
the rock it already took, and the guard would then refuse for a reason that looks
nothing like the real one.

**One dry-fire pass, two answers.** `scan_candidates` replaces `legal_candidates`
and returns both what's legal and what's pending-on-a-binding. Dry-firing is a
subprocess per candidate that re-parses every world file — by far the loop's
dominant cost — so scanning twice would have doubled it for nothing.

Two bugs caught in dry-run before spending live calls: the fixpoint check ignored
pending bindings (the loop stopped with a decision sitting unasked on the table),
and the structured marker was being emitted mid-line after
`fire-transition: refused -- ` so the driver's line-anchored regex never matched.

### Live

`examples/jev-driver-demo/quarry-delve/` — a crew draining a three-rock quarry.

```
round 1: bound dig's ?rock = rock/1      conf 0.87  {rock/1: .91, rock/3: .05, rock/2: .04}
round 2: fired dig
round 3: fired rest
round 4: bound dig's ?rock = rock/2      conf 0.52  {rock/2: .76, rock/3: .24}
round 5: fired dig
round 6: fired rest
round 7: fired dig          <- one rock left: not a choice, no question spent
round 8: fired rest
=== round 9: fixpoint ===
```

**Two questions for three rocks.** The last rock isn't a decision, so the loop
doesn't pay for one. The economy behaves.

## How long can this run — measured, not estimated

**The engine is linear and cheap.** One `fire-transition` against a world of N
facts, measured (best of 3):

| facts | one dry-fire | round @50 candidates | round @200 candidates |
|------:|-------------:|---------------------:|----------------------:|
|   200 |       0.033s |                 1.7s |                  6.6s |
|   800 |       0.104s |                 5.2s |                 20.8s |
|  1600 |       0.195s |                 9.8s |                 39.1s |
|  3200 |       0.388s |                19.4s |                 77.7s |

Flat ~0.12ms per fact plus ~10ms process startup. Nothing super-linear anywhere in
guard evaluation, including the new binding walk.

So: **200 nodes is nothing.** A few thousand facts with a few hundred candidates is
seconds per round. The wall is around 10⁴ facts, and it is not the engine — it is
that `fire-transition` re-parses **every** accumulated world file on **every**
call, and the driver makes one call per candidate per round. Both the candidate
count and the file count grow with the run, so round cost grows roughly
quadratically and total cost cubically.

That is a fixable shape, not a fundamental one: a persistent process, or caching a
materialized snapshot across candidates within a round, would make the per-round
cost flat in world size. Neither is built.

## But performance is not what actually stops it

A long dry run with a 120-machine cap stopped after **2 minted machines**, at a
genuine fixpoint, in 0.45 seconds. Diagnosed precisely:

```
machine room/g1                     <- chimera of room/g0 (a hall) x fork/mouth
  transition breach()
    guard room/g0 `cleared` mark/yes
    assert path/left `cleared` mark/yes    <- the fork's world effect
    ...                                    <- and NO `assert self `cleared``
```

A chimera keeps A's guards and takes B's **world effects**. A fork clears a *path*
node, not itself — so a room bred from one inherits "opens a path" without
inheriting "is itself cleared." **A room that never clears itself is a dead end**,
nothing can be built on it, and the lineage ends.

So the real limiter on run length is **fertility, not speed**. Typical lineage
depth before a sterile crossover: 2–5 generations. Worth being precise about what
kind of problem that is:

- It is not a bug in `DMML.Recombine` — chimera does exactly what it documents.
- It is not obviously wrong *architecturally* — dead ends are legitimate rooms.
- It is a real limitation on this loop as a **growth** mechanism, because sterility
  is reached by accident rather than by choice.

Two honest directions, neither taken yet:

1. **Let the chooser route around it.** In the failing run, `room_g1-goRight` *was*
   legal and would have cleared `path/right`, opening new ground — the dry run's
   first-option bias took `breach` instead. So live run length depends on whether
   the chooser picks transitions that open ground. That is desire steering fertility,
   which is the right shape, but it means the loop's reach is not a property of the
   loop alone.
2. **Make the cannon prefer fertile offspring.** It could check whether a candidate
   clears itself before committing to it. That is a fitness function in the exact
   place we keep deciding not to put one, so it needs a real decision rather than a
   quiet default.

## Still open

- Per-round snapshot caching / a persistent `fire-transition` (the only thing
  between here and ~10⁵ facts).
- Sterile-crossover fertility, above.
- Ambiguity questions currently resolve one round before the candidate fires — a
  deliberate lag, since legality depends on the binding.
