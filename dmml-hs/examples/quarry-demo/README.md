# quarry-demo — a substance that runs out

The smallest real use of `?binder` guard variables
(`DMML.Ast.TermBind`, `DMML.Guard.evalGuardsBinding`), and the reason
they exist.

Before binding, a guard could ask *"is there a rock?"* but could not tell
an effect **which** rock — so nothing could actually be spent. A quantity
could be looked at and never consumed.

```sh
FIRE=fire-transition   # or: cabal list-bin fire-transition

# one rock: take it. The consumes citation is real -- that rock is spent.
$FIRE digger.dmml take takes --world quarry-world.dmml

# three rocks: REFUSED, with every candidate named. Not a dead end --
# this is the engine handing a decision to whoever makes decisions.
$FIRE digger.dmml take takes --world quarry-world.dmml \
      --world rock2.dmml --world rock3.dmml

# narrow it: a second guard on the SAME ?rock picks the rich one.
$FIRE picky.dmml take takes --world quarry-world.dmml \
      --world rock2.dmml --world rock3.dmml \
      --world grade.dmml --world picky-state.dmml

# an empty quarry: blocked.
$FIRE digger.dmml take takes --world empty-quarry.dmml
```

**The point is the last one.** Nothing anywhere records that the quarry
is empty. There is no `depleted` flag, no counter, no state on the
quarry at all. The transition stops being legal because the guard stops
finding a witness — depletion as the plain absence of a fact.

That is scarcity expressed as an **external relation** rather than as
internal state, and it matters beyond tidiness: a lifecycle lock is the
one thing `DMML.Recombine` cannot carry across a crossover (see its
"Recombination cannot preserve cross-machine exclusivity"), whereas a
guard and its effect travel together. Breed a transition that spends a
rock and the offspring spends a rock too.

Every rock is its **own subject** (`rock/1 `in` quarry/north`) rather
than an alternative under one key (`quarry/north `holds` rock/1`). Both
say "the quarry has these rocks"; only the first survives the grammar,
since a commit cannot assert the same (subject, predicate) pair twice —
fifty rocks the other way would be fifty separate commits — and it makes
the retract unambiguous.
