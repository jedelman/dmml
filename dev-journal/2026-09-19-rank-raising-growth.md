# 2026-09-19 — Growth that prefers the feed which raises rank

Jason: *"Pull that thread - make growth prefer the feed that raises rank. That's a
prompt edit, right?"*

## Partly. The load-bearing part isn't.

Two separable pieces, and it's worth being precise about which is which.

**Which feeds get proposed at all — code, and it had to be.** The proposer used to
offer exactly one: `sorted(terminals)[0]` back to `sorted(roots)[0]`, on the
reasonable but unchecked assumption that terminal-to-root is where a loop wants
closing. Two things wrong with that. It misses every feed that would close a circuit
somewhere in the middle of the graph, and it cannot tell a feed that closes one from a
feed that merely adds another one-way path.

On the rhizome demo the difference is stark:

```
feed 'is brick/fired'   -> 'is clay/raw'        rank delta 1
feed 'is clay/raw'      -> 'is wall/section'    rank delta 0    <- plausible, useless
feed 'is wall/section'  -> 'is brick/fired'     rank delta 1
feed 'is wall/section'  -> 'is clay/raw'        rank delta 1
```

The old heuristic found one of the three real closures and would have offered the
delta-0 shortcut with exactly the same confidence.

**Whether an edge closes a circuit is an exact question about a digraph with an exact
answer.** An LLM cannot know whether adding an edge merges two strongly connected
components; it would guess, fluently. So it is computed — over every pair of
substances sharing a predicate, which is the only pair `cannon feed` can actually
connect — and the ones that raise the rank are offered first.

**What Jev is told about it — that is the prompt edit**, and it's the last inch:

> Let brick/fired become clay/raw. **This CLOSES A CIRCUIT:** matter already travels
> clay/raw → … → brick/fired and stops there; this sends it back, so the world would
> have 1 independent circuit(s) where it now has 0.

versus, for a delta-0 edge:

> This closes no circuit — nothing currently leads from {to} back to {frm}, so matter
> would still only go one way and the world would still run down. It would be a new
> path, not a loop.

The principle: **compute what is decidable, delegate what is not.** Whether an edge
closes a circuit — decidable, so compute it and say so. Whether closing *that* circuit
is worth having — not decidable, so ask.

## Live, on a real substance chain

New seed `examples/jev-driver-demo/kiln-delve/` — clay → brick → wall, rain as the only
source, wall as the end of the line.

```
r1: BUILT feed-wall/section-to-brick/fired   interest 0.51
r2: BUILT feed-brick/fired-to-clay/raw       interest 0.58
r2: BUILT feed-wall/section-to-clay/raw      interest 0.57
r8: BUILT feed-clay/raw-to-wall/section      interest 0.44
```

Before: *"no circuit, but is clay/raw is produced from nothing — sustained by a
SOURCE."* After: **CIRCULATES, 1 strongly connected component, independent circuits 4.**

## The part I got wrong, and it's the better result

I first read the run as "the delta-0 shortcut was offered every round and never taken."
Both halves were wrong, and what actually happened is more interesting.

`feed-clay/raw-to-wall/section` was **not offered at all** in rounds 1–2 — it scored
delta 0 and was out-ranked. It appears from round 3 onward, and is built in round 8.
Because by then the three back-edges existed, and against *that* graph the same edge
closes a further circuit: rank 3 → 4. The description it carried in round 8 said so.

**The same proposed edge went from pointless to worthwhile as the graph changed around
it.** That is not a ranking applied once; it is a measurement re-taken every round
against a world that keeps moving. A static preference order could not have produced
it, and neither could a prompt.

## Still open

- **Only feeds are rank-scored.** `bridge` does the same thing for the reachability
  graph and is still proposed by "two cleared nodes not already related", which is the
  heuristic this change just replaced for feeds. Same treatment available, not applied.
- `vista` raises rank in the *reference* graph, which nothing computes at all.
- The breeding policy is still a 2-cycle — and that is now the interesting limit, for
  reasons that belong in their own note.
