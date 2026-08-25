# A real gap: `consumes` conflates "cite" with "spend" (2026-08-25)

Jason's question, reading Section VI's cite-and-spend finding: "don't we
have `requires` as a primitive that retains the original predicate? Seems
like we need a how-to for DMML!"

Checked directly, not assumed: `grep -rn "requires" dmml/src/*.rs` turns
up nothing resembling a distinct citation primitive — `consumes` is the
only way any commit cites a prior fact, and per `interpret.rs`'s own doc
comment it always retracts the exact `(subject, predicate)` key it cites,
unconditionally, before applying its own `produces`. There is no
`requires`-shaped alternative today. Jason's instinct names a real gap,
not a feature that already exists under a different name.

The gap matters because this Benjamin series has been running into it
mechanically: every argumentative commit in these files "cites a premise"
in the ordinary sense a philosophy paper or code review means it — I
depend on this fact being real, but I am not spending or replacing it.
`consumes` as it exists today can't express that distinction; it always
retracts. `pantheon.rs`'s Nyx never exposed this because it happened to
consume and produce the identical key (a same-key rivalry-resolution
case, not an argument chain), which makes the retraction invisible,
masked by immediate re-assertion at that same key. Section VI's chain —
consuming one subject's fact to produce a claim about a DIFFERENT
subject — is the much more common shape for an argument, and it's where
the retraction becomes visible and, for citation purposes, wrong: the
combined log loses the cited fact's own visibility even though nothing
about the argument requires that.

Two real primitives seem to want to coexist, not one replacing the
other:
- `consumes` (as it exists today): a retracting reference, right for game
  state — a lock consumed, a quest's precondition spent, a resource that
  really is gone once used.
- `requires` (proposed, NOT implemented): a referential-integrity-checked
  citation with NO retraction — right for argument/citation graphs like
  this whole Benjamin series, where dependency needs to be real and
  checkable without removing the cited fact from the combined current
  view.

This is exactly the kind of question a DMML how-to/style-guide should
settle explicitly — which primitive to reach for depending on whether
the relationship is "I am spending this" or "I am citing this" — since
getting it wrong silently produces a materialized view that's missing
facts nobody meant to retract (as Section VI's Check 2b demonstrated
concretely). Filed here as a real language-design gap, credited to
Jason's own instinct, not yet designed or implemented — that's a
separate, real piece of work for when the essay reading isn't the
priority.
