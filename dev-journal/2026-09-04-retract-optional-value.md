# Accepting general retract's trailing value

Follow-up to the real eval (`dev-journal/2026-09-04-complex-machine-
eval.md`) that found three independent free models -- `minimax-m3` and
`dots-3-note-preview` twice, unprompted -- all writing a general
retract with a trailing value symmetric to assert's, even though the
grammar at the time had no value slot there. Jason's call: "yes, that
makes sense. even if the value does nothing it may be useful in the
future."

## What changed

`DMML.Ast.Effect`'s `EffectRetract` grew a third field:
`EffectRetract PatternTerm PredicateRef (Maybe EffectValue)`. Small,
contained blast radius -- only five files actually pattern-match or
construct it (`Ast.hs`, `Surface.hs`, `Json.hs`, `FromJson.hs`,
`Fire.hs`); `Governance.hs`/`Guard.hs` never touch `EffectRetract` at
all, confirmed by grep before assuming so.

- `DMML.Surface.pEffectLine`'s `pRetractGeneral` now parses
  `optional (try pEffectValue)` after the predicate backtick -- the old
  bare sugar (`retract <ident>`) still never has one.
- `DMML.Json.EffectInput`'s `EffectRetractGeneralInput` grew a fourth,
  optional `value` field (`o .:? "value"`), same JSON-shape convention
  every other optional field in this wire format already uses.
- `DMML.FromJson` threads the optional value through
  `effectValueFromInput` via `traverse`.
- `DMML.Fire`: this is the part that makes "may be useful in the
  future" concrete rather than a discarded field. `EffectRetract`'s
  optional value maps directly onto `DMML.Ast.FactConsume`'s own
  PRE-EXISTING optional `object` field (`factConsumeObject`) -- the
  exact same `subject.predicate[=value]` shape a `consumes`/`fact`
  block already had, just never reachable from a machine's own effect
  syntax before. `ResolvedRetract` now carries `Maybe Value`, resolved
  the same way an assert's value is (`resolveTerm` for a term, direct
  for a literal), and `renderFiredCommit` renders it into the produced
  citation: `subj . pred = value` when present, `subj . pred` when not.
  `DMML.Materialize`'s `applyConsume` still deletes a (subject,
  predicate) key unconditionally regardless of what's cited -- a real,
  pre-existing, separately-tracked simplification (not something this
  change touches) -- so the value is real, parsed, and now genuinely
  round-trips into the commit, but isn't yet load-bearing for what gets
  deleted. That's the actual "future hook," not a vague gesture at one.

## Verification

Whole demo suite reruns with identical output, no regressions. All real
machine files (11 endurance + shrine.dmml + complex-demo/master.dmml +
cascade-demo's two) still parse.

**The actual test**: re-parsed the SAME saved eval candidates from the
earlier run with the fixed binary. 2 of the 3 real failures
(`trial-03.dmml`, `trial-05.dmml` -- `minimax-m3` and one of the
`dots-3-note-preview` runs) now parse AND pass `check-declared` --
real, previously-rejected, already-existing model output, not a new
synthetic test written to be easy.

**End to end, not just parse-level**: fired `trial-05.dmml`'s
`unward(target)` transition (`` retract $target `wardedBy` self ``)
against a real seed world. Produced:

```
commit unwards
  declare relation state
  npc/keeper `state` warding
  consumes
    fact local:examples/retract-value-demo/world.dmml#fnv1a64:00b9dcc6a3f807f4
      herd/aurochs . wardedBy = npc/keeper
```

The value (`npc/keeper`, resolved from `self`) genuinely made it into
the citation's object position. Passes real `validate-commit`/
`check-declared`.

**Not fixed, out of scope for what was asked**: `trial-02.dmml`
(`minimax-m3`) still fails -- it wrote a full TWO-HOP retract pattern
(`` retract self `witnessedBy` self `at` $eruption ``), not just a
trailing value. That's a different, larger ask (multi-hop retract
chains, symmetric to guard's own multi-hop patterns) than what Jason
approved here. Flagged, not built -- a real further signal about what
shape models reach for, but a separate design decision from "accept a
trailing value."
