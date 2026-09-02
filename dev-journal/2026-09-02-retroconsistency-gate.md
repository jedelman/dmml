# Gating retro commits on a whole-tree-consistent result

Jason: "continue work on retro continuity - let's gate retro commits on
a consistent tree." Follows directly from `2026-09-02-retroconsistency.md`
(`written-world`'s own entry, at the time — that whole feature has since
moved here with `android-poc/`).

## The gap this closes

`DMML.Retroconsistency`'s own doc comment claimed minting is "ALWAYS
safe by construction" at the data level. True, but incomplete — that
statement only covers the ONE guard a retro commit was generated to
satisfy. A retro-implied fact is still a real fact, visible to every
OTHER machine's guards too. Concretely: `forest/clearing`'s `deplete`
transition needs `harvestedBy` on record; a SEPARATE `protect`
transition on the SAME machine needs the opposite — nobody has
harvested it. Retro-filling `deplete`'s own gap is, by itself, entirely
correct — and it silently breaks `protect`, a transition retro-
consistency was never even asked about.

This is provably the ONLY way additive minting can cause harm: a
POSITIVE guard can never newly fail from adding facts (an `EXISTS`
pattern only gets MORE likely to hold as more facts exist), so the
entire risk surface is negated guards flipping from held to blocked.

## The gate

`DMML.Retroconsistency.gateConsistentTree machines before after`:
scans every negated guard on every transition of every machine in the
given machine map, evaluates it against both snapshots, and flags any
that held before and is blocked after. `GateOk` or `GateBroken
[BrokenGuard]` (machine, transition, predicate — enough to say exactly
what would break and where).

**Real, disclosed blind spot, not silently papered over**: a guard
referencing a transition's own `$param` can't be generically re-checked
here — its real meaning depends on a specific firing's actual argument
binding, which a whole-tree scan doesn't have and structurally can't
(it's checking "is the tree still consistent as data," not "would this
one firing still succeed"). Such guards are excluded from the scan, not
treated as passing — verified directly (Scenario 4 below): a `$param`-
guarded version of the exact same `protect` conflict correctly reports
`GateOk`, proving the exclusion is real, not just claimed.

## Real CLI: `retro-gate`

`app/RetroGate.hs` — `retro-gate <candidate.dmml> <world-file.dmml>
[...]`. Parses and classifies every world file (commits vs. machines,
same idiom `render-snapshot`/`checkpoint-rebuild` already use), builds
`before`/`after` snapshots, runs the gate, exits 0 (`GateOk`) or 1
(`GateBroken`, printing exactly which guard(s) on which machine would
break). Meant to sit in front of actually committing a retro commit —
same role `pre-merge-commit` plays for ordinary merges, just invoked
explicitly, since retro commits aren't produced at merge time.

## Verified for real, including the natural conflict scenario

`app/RetroGateDemo.hs`, four scenarios, all passing:

1. (Narrative only — shows what an incomplete gate, checked against
   too small a machine set, would miss.)
2. **The real conflict**: gating a `deplete`-retro-fill against the
   FULL `forest/clearing` machine (both transitions) correctly reports
   `GateBroken`, naming `protect`/`harvestedBy` exactly.
3. **A harmless case**: the same candidate, gated against a machine
   with no `protect` transition at all, correctly reports `GateOk` —
   proves the gate isn't just reflexively rejecting everything.
4. **The disclosed blind spot, proven real**: the same conflict,
   `$param`-guarded, correctly reports `GateOk` (excluded from the
   scan) — the limitation is real and demonstrated, not just asserted
   in a comment.

Also ran the real CLI end to end against actual files on disk (not just
the in-process demo) — same conflict, same correct REJECTED/exit 1,
and the same candidate against a machine missing `protect` correctly
OK/exit 0.

## What's still open

- **Not wired into any real commit-acceptance flow yet.** `retro-gate`
  exists as a standalone CLI; nothing currently calls it automatically
  before a retro commit lands anywhere real (`sync-spike`'s hooks,
  the endurance harness). Real, natural next step, not done here.
- **The `$param` blind spot is real, not just disclosed.** A guard tied
  to a specific firing's own argument could, in principle, still be
  checked if the caller supplied the actual binding retro-consistency
  used when it generated the candidate — not built, since retro-
  consistency itself doesn't currently thread transition-argument
  bindings through its own output (`ImpliedFact` carries no notion of
  "which firing, with which params, this was generated for").
- **Still single-transition, single-run** — this gate checks one
  candidate against one before/after pair. The recursive/fixpoint
  extension for Jason's quarry example (`2026-09-02-retroconsistency.md`'s
  own "what's real here vs. the harder quarry example") would need each
  step in that fixpoint gated too, not just the whole chain once at the
  end — real design implication of chaining this gate, not resolved
  here.
