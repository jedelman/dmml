# Making retract's value load-bearing: value-qualified consumes

Jason: "let's just fix it!" -- the highest-leverage of the disclosed
gaps surfaced while reviewing today's work: `DMML.Materialize.
applyConsume` deleted an entire `(subject, predicate)` key
unconditionally, regardless of what `factConsumeObject` a `consumes`
entry actually cited. That single simplification was the real reason
behind three separate disclosed footnotes today -- retract's optional
value wasn't load-bearing, `FireRetractAmbiguous` had to refuse
outright rather than disambiguate, and a chained retract's own
intermediate hops couldn't safely target just one of several live
alternatives. One fix closes all three.

## The fix

`DMML.Materialize.applyConsume`: `factConsumeObject = Nothing` still
wipes every live alternative for the key (the original, unchanged
wildcard semantics -- this is what a value-less retract, including the
old bare `retract <ident>` sugar, still produces). `Just v` now removes
ONLY the alternative whose value equals `v`, via `Map.update` (deleting
the key entirely if that was the last alternative, never leaving a
dangling empty `Alternatives []`).

`DMML.Fire.resolveSingleRetract`: with a value to match, looks for
exactly one live alternative whose VALUE equals it (not just "exactly
one alternative overall") -- other live alternatives at the same key
are no longer even a consideration, since `applyConsume` will only ever
touch the one actually cited. Without a value, falls back to the
pre-existing wildcard discipline unchanged (exactly one alternative
overall, or refuse as ambiguous -- a value-less retract with several
live alternatives still has no principled way to pick just one).

`DMML.Fire.resolveRetractHops` (the chained-retract walk, built a few
hours earlier today) now cites the resolved intermediate target as the
value to match, rather than `Nothing` -- so a chain's own intermediate
hop correctly targets just the walked edge even when that (subject,
predicate) key has other live alternatives besides the one actually
walked.

## Real verification, not just re-running the old suite

Whole demo suite reruns identical, no regressions. Every prior
fire-transition demo this session (`fire-demo`, `sense-demo`,
`cascade-demo`, `complex-demo`, `retract-demo`, `retract-value-demo`,
`chained-retract-demo` including its subtractive-gate case) still fires
identically -- confirms the fix doesn't change behavior for anything
that was already correct.

**The actual new capability, proven directly** (`examples/value-
disambiguation-demo/`): `npc/watcher` watches BOTH `herd/aurochs` and
`herd/stagfolk` (two independently-committed live alternatives for the
same `watches` key -- would have been an outright `FireRetractAmbiguous`
refusal before today). Firing `dismiss(target=herd/aurochs)`
(`` retract self `watches` $target ``) now succeeds, citing exactly the
`herd/aurochs` alternative:

```
commit dismisses
  declare relation state
  npc/watcher `state` dismissing
  consumes
    fact ...#...
      npc/watcher . watches = herd/aurochs
```

Re-materializing world + fired commit confirms `herd/stagfolk` survives
completely untouched -- `npc/watcher . watches = herd/stagfolk`, the
only live value left. The wildcard refusal path was checked too, not
just the happy path: a separate `dismissAll()` transition with a
value-less `` retract self `watches` `` against the same two-alternative
world still correctly refuses (`FireRetractAmbiguous`) -- the safety
behavior for an unqualified retract is unchanged.

## What this doesn't change

The chained-retract-demo's cross-machine consistency gate
(`watchtower/relay`'s dependency on `npc/keeper \`at\` volcano/ashkar`)
still fires and still refuses exactly as before -- this fix is about
WHICH alternative a value-qualified retract removes, not about whether
firing is allowed at all; the two mechanisms are independent and
compose the way they should.
