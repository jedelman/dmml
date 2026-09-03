# Phase 2/3: generalizing Effect, and machines that actually govern transitions

Continues `dev-journal/2026-09-03-authoring-tools-and-narration-compulsion-
result.md`'s three-phase plan ("syntax tools, generalize Effect, and
generalize machines"). Jason's call on seeing the real finding that
`dmml-hs` had NO execution layer at all -- `Effect` had exactly one
consumer (`Governance.arbitrate`, value-matching only), and `DMML.Guard`'s
own doc comment said applying an effect "is the caller's job," with no
caller anywhere: **"oh, fantastic. this is exactly the right transition.
machines should govern all transitions. build phase 2/3 -- they're the
same."** Built as one change, per that framing.

## Phase 2: generalizing `Effect`

Was `EffectAssert Text | EffectRetract Text`, always implicitly `(self,
"state", <ident>)` -- real state-machine transitions only. Now:

```haskell
data Effect
  = EffectAssert PatternTerm PredicateRef EffectValue
  | EffectRetract PatternTerm PredicateRef
  deriving (Eq, Show)

data EffectValue
  = EffectValueTerm PatternTerm
  | EffectValueLiteral Literal
  deriving (Eq, Show)
```

A transition can now assert or retract an arbitrary fact, on an arbitrary
subject -- not just `self`. This is also, for free, how firing mints a
new node: `PatternTerm` already includes `TermParam`, and DMML is
open-world (a node exists the moment any fact mentions it, no separate
registry), so an effect whose subject is `$name` and whose predicate\/
value assert real content brings a brand-new node into existence the
instant the transition fires with a concrete binding for `$name` -- no
separate "mint" effect constructor needed, matching the project's own
smallest-generic-extension razor.

**Backward compatibility, deliberately preserved, not force-migrated.**
11 real machine example files (`examples/endurance/machines/*.dmml`) plus
`examples/shrine.dmml` use the old bare `assert <ident>`/`retract <ident>`
syntax. Rather than rewrite real, already-committed evidence, the old form
still parses as sugar for exactly its old meaning:

```
assert unlocked      -- sugar for: assert self `state` unlocked
retract stirring      -- sugar for: retract self `state`
```

alongside the new general form:

```
assert $name `title` "A freshly forged key"
assert $name `madeFrom` $material
```

Both `DMML.Surface` (text) and `DMML.Json`/`DMML.FromJson` (JSON) grew
matching general forms -- "one AST, two front-ends" preserved. JSON's
`EffectInput` dispatches on whether `ident` is present (sugar) or
`subject`/`predicate`/`value` are (general form), same discriminant-field
convention the rest of the wire format uses.

`DMML.Governance.arbitrate`'s value-matching comparison (used to resolve
disputed alternatives against a governing machine's transitions) was
narrowed, not broken: it now explicitly matches only
`EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode _))`
-- preserving its exact pre-generalization behavior (only a machine's own
`self . state` effect is a candidate for resolving a disputed pair),
since an effect on some other subject/predicate says nothing about which
alternative of THIS pair is correct. Verified: `GovernanceDemo`'s existing
state-predicate case still passes unchanged, and a new non-state-predicate
case was added and passes too (governance already generalized here in an
earlier session, ahead of `Effect` itself).

**Real verification, not just "it compiles":** all 12 real machine files
(11 endurance + shrine.dmml) still parse correctly under the new grammar,
checked directly (`check-string-cap` with a huge cap, which dual-parses
commit-then-machine -- confirms real parse success, not just "didn't
crash"). Every existing demo (`GuardDemo`, `GovernanceDemo`,
`RetroconsistencyDemo`, `RetroChainDemo`, `RetroGateDemo`) reruns and
passes with identical output to before the change -- no regressions.

## Phase 3: `DMML.Fire` -- machines actually governing transitions

New module, `src/DMML/Fire.hs`, plus a CLI (`app/FireTransition.hs`,
`fire-transition`). This is the actual execution layer that never
existed: `fireTransition` checks a named transition is declared and
legal (via the unchanged `mayFire`), then resolves every effect's
`PatternTerm`/`EffectValue` to a concrete fact using the same
`resolveTerm` guards already used (now exported from `DMML.Guard` for
this).

**Deliberately does not mutate a `WorldSnapshot` in place.** Per the "DMML
is the evidence, not any tool's or agent's say-so" principle (Paper 2
§10 -- the same principle behind `DMML.Retroconsistency` rendering a real
commit instead of poking the snapshot directly), firing renders resolved
facts as ordinary DMML Surface commit text. That text is real,
re-parseable DMML -- the actual application happens by running it through
the exact same `validate-commit`/`check-declared`/`retro-gate` pipeline
any hand-authored commit goes through, not by a silent internal mutation
this module could get wrong unobserved.

**A real, disclosed scope limit, not a bug**: `EffectRetract` currently
refuses (`FireRetractNeedsProvenance`) rather than emit an unsound commit.
DMML's real commit grammar only retracts a fact via a `consumes` block
naming the SPECIFIC prior commit (`uri#cid`, a real `StrongRef`) that
asserted it -- but a `WorldSnapshot`'s `Alternatives` only carry a
branch/agent-name provenance label, never a real commit `uri#cid`. There
is no sound way to synthesize a `consumes` entry from a snapshot alone
without fabricating provenance that doesn't exist. `EffectAssert` (the
capability actually asked for -- minting via firing) has no such gap and
is fully implemented. Fixing retract needs the caller to supply real
strong-ref provenance for whatever it wants retracted; not yet designed,
left as real follow-up rather than faked.

### Real, dogfooded proof: minting a brand-new node by firing

`examples/fire-demo/named-key.dmml` -- a machine with a `forge(name,
material)` transition whose effects assert on `$name` (a transition
parameter), not `self`:

```
machine key/forge
  states
    idle
    forged

  transition forge(name, material)
    idle -> forged
    guard self `stocked` $material
    assert $name `title` "A freshly forged key"
    assert $name `madeFrom` $material
    assert forged
```

`examples/fire-demo/world.dmml` sets up `key/forge`'s own `stocked`
fact and initial `state`. Firing:

```
$ fire-transition examples/fire-demo/named-key.dmml forge mints \
    --world examples/fire-demo/world.dmml \
    --param name=key/rusty42 --param material=iron/ingot

commit mints
  declare relation title
  declare relation madeFrom
  declare relation state
  key/rusty42 `title` "A freshly forged key"
  key/rusty42 `madeFrom` iron/ingot
  key/forge `state` forged
```

`key/rusty42` was never mentioned anywhere before this -- it exists now
purely because the transition fired with `$name` bound to it. The
produced commit was then run for real through `validate-commit` (exit 0,
re-parses as valid Surface syntax) and `check-declared` (both alone and
merged with `world.dmml`'s real context: "OK -- every used predicate is
declared"). This is the actual, verified capability Jason asked for --
not argued from the type signatures, run end to end against the real
pipeline.

## What's still open

- `EffectRetract` firing (needs a real strong-ref-provenance design, see
  above).
- No GitHub issue filed yet for the retract gap -- per `written-world/
  CLAUDE.md`'s "task/follow-up tracking → real GitHub issues, always"
  rule, this needs a real issue in `jedelman/dmml`, not just this journal
  entry, before the next session starts something else.
- `fire-transition`'s param values are read as plain node-ref text only
  -- no way yet to bind a `$param` to a literal (string/number/bool)
  value from the CLI, only to a node reference. Not a blocker for the
  demo above (both `$name` and `$material` are node refs), but a real gap
  if a future transition's guard needs to compare a param against a
  literal.
