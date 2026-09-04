# Chained retract, and gating every fire against the whole machine set

Closes jedelman/dmml#5. Follow-up to the real eval that found
`minimax-m3` writing a two-hop retract pattern (`trial-02.dmml`'s
`witnessEruption`), which Jason walked through directly and called "a
valid extension for retract" before raising the real question: "what if
a chained retract leaves it in the invalid state? I guess the
transition is invalid."

## Two real problems, both solved, neither a special case of the other

**1. Resolving the chain.** `retract self \`witnessedBy\` self \`at\`
$eruption` mirrors a guard's own multi-hop `Pattern`. `DMML.Ast.Effect`'s
`EffectRetract` grew a `[PatternHop]` field for intermediate hops
(empty list = the pre-existing single-hop shape, unchanged). Parsed
with one real disambiguation trick (`DMML.Surface.pEffectLine`): after
each `` `predicate` ``, try to read ANOTHER `term `predicate`` pair --
an intermediate hop always has a real term and is always followed by
another backtick-predicate, since only the terminal position can end
the line. If that fails, the current predicate IS the terminal one, and
whatever follows (or nothing) is its optional value via `pEffectValue`
-- same superset-of-`PatternTerm` parser `pAssertGeneral` already uses,
so only the terminal position can ever be a literal. Unambiguous,
verified directly: `subj \`p1\` self \`p2\` foo` parses as one
intermediate hop `(p1, self)` plus a terminal `(p2, foo)`, never two
value-less steps.

`DMML.Fire.resolveRetractHops` walks the chain at fire time: each hop's
term resolves via the same `resolveTerm` guards already use (never
fans out -- an intermediate hop's term must be concrete, since there's
no principled "any of several" answer for what to actually delete), and
each walked edge gets its own independent `resolveSingleRetract` check
-- the exact same discipline the single-hop case already had
(`FireRetractNoSuchFact`/`FireRetractNoProvenance`/`FireRetractAmbiguous`,
now applicable at any step of a chain, not just the one hop). A chain
resolves to N `ResolvedRetract` entries, each its own real
`consumes`/`fact` citation, rendered in hop order.

**2. Whole-tree consistency.** `DMML.Fire` only ever checked the firing
transition's OWN guards -- it had no way to know whether some other,
unrelated transition elsewhere in the machine set had a positive guard
depending on a fact this retract was about to delete. The fix already
existed in a different module, just pointed the wrong direction:
`DMML.Retroconsistency.gateConsistentTree` (built 2026-09-02, "let's
gate retro commits on a consistent tree") already checked exactly this
shape of question, but only for ADDITIVE risk (a negated guard flipping
from held to blocked when facts are added -- the only direction that
could happen back when nothing here ever retracted anything). The
reasoning generalizes for free: adding facts can only ever make a
positive guard MORE true, never less, so a positive guard can't break
from addition; removing facts can only ever make a negated guard's
underlying `EXISTS` LESS true, so a negated guard can't break from
retraction either. The two risks are perfectly disjoint by polarity --
so dropping the `guardNegated` filter and checking every non-`$param`
guard, whichever way, catches whichever direction a candidate (additive,
subtractive, or -- now possible -- both at once) actually risks. Verified
this was a safe, behavior-preserving generalization before relying on
it: reran `RetroGateDemo` (the existing additive-only test suite)
unchanged, identical PASS output -- a positive guard genuinely cannot
break from pure addition, so including it in the scan finds nothing new
for an addition-only caller.

`DMML.Fire.fireTransition` now takes the full known machine map as a
required argument (small blast radius -- only `app/FireTransition.hs`
was a real caller) and gates the WHOLE resolved effect set -- assert
and retract together, not retract alone -- by literally rendering the
commit it's about to produce, re-parsing it, applying it to `before` to
get `after`, and running `gateConsistentTree` before returning success.
Dogfoods the real render+parse path rather than a shadow diff
computation, same "DMML is the evidence" discipline as everywhere else
in this module. New `app/FireTransition.hs --machine <file>` flag
(repeatable, dual-parsed the same way `RetroGate.hs` already classifies
a mixed file list) adds extra machines into the gate's scope; the
firing machine itself is always included.

## Real verification

Whole demo suite reruns identical, no regressions. All real machine
files still parse. Every previously-verified fire-transition demo this
session (`fire-demo`, `sense-demo`, `cascade-demo`, `complex-demo`,
`retract-demo`, `retract-value-demo`) still fires successfully with the
new mandatory gate active -- confirms the gate doesn't spuriously block
anything that was actually fine.

**The actual test, `trial-02.dmml` for real**: it now parses cleanly
(all 3 of the earlier eval's real failures are now fixed). Firing its
`witnessEruption` transition against a real world:

```
$ fire-transition examples/chained-retract-demo/keeper.dmml witnessEruption witnesses \
    --world examples/chained-retract-demo/world.dmml --param eruption=volcano/ashkar

commit witnesses
  declare relation witnessedBy
  npc/keeper `witnessedBy` volcano/ashkar
  consumes
    fact ...#...
      npc/keeper . witnessedBy
    fact ...#...
      npc/keeper . at = volcano/ashkar
```

Both edges the guard's two-hop pattern walked (`witnessedBy self`,
`at volcano/ashkar`) are genuinely retracted -- re-materializing
confirms only the new, concrete `witnessedBy = volcano/ashkar` fact
survives. Passes real `validate-commit`/`check-declared`.

**The gate, proven against a real "invalid state," not a hypothetical**:
added `examples/chained-retract-demo/dependent-watcher.dmml`, a second
machine (`watchtower/relay`) whose only transition guards on
`npc/keeper \`at\` volcano/ashkar` -- exactly the fact `witnessEruption`
retracts. Firing without that machine in scope: succeeds (nothing knew
to object). Firing with `--machine dependent-watcher.dmml` added:
refuses --

```
fire-transition: refused -- firing would break the following currently-held guard(s) elsewhere in the known machine set:
  watchtower/relay's relay (predicate at)
```

This is Jason's own scenario, built and run for real, not argued
abstractly: a chained retract that's locally valid but would silently
strand another transition now refuses instead of firing.
