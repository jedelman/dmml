# 2026-09-18 — Guard binding: stop throwing away the witness

Jason: *"Yep, build the guard binding. No wasted data!"*

The observation it came from: `DMML.Guard.stepHop` computes the actual objects a
guard pattern reaches, and `evalExists` then reduces them to `not (null ...)`.
**The binding was already being computed and discarded.** Binding needed no new
search, only the decision to stop dropping the answer.

## Why a new constructor and not a change to `TermVar`

The obvious move — make `TermVar` bind — would have been a silent semantic change
to content already committed. In this surface **every slash-free token in a guard
pattern parses as a `TermVar`**, which is the single most-repeated real authoring
mistake in this project and the entire reason `DMML.GuardLiterals` exists. A guard
that used to match anything would have started demanding consistency, and one
matching several facts would have started refusing.

So: `TermBind`, written `?name`. `?` occurs nowhere else in the grammar and
`pIdentRaw` requires a letter, so `?rock` was a parse error before now — **no
existing content can contain one**, and you cannot get a binder by accident. The
bareword keeps its documented non-binding behaviour exactly.

## What it does

```
transition take()
  idle -> working
  guard ?rock `in` quarry/north
  assert self `took` ?rock
  retract ?rock `in` quarry/north
```

- The witness is threaded into later guards and into every effect.
- A repeated `?name` must agree with itself — the unification a bareword lacks.
- Guards are evaluated as **one conjunctive query**: a set of candidate binding
  environments threaded through all of them.
- More than one surviving witness is **refused**, naming every candidate.
- A binder in a negated guard is an error — nothing binds from an absence.

## Two bugs the selftest caught, both real

**1. Ambiguity was judged per-guard.** The first implementation refused on guard
one's three candidates without ever consulting guard two, which exists precisely
to narrow them. Guards are a conjunction; a binder is ambiguous only if it is
*still* ambiguous after every guard has had its say. Rewritten to thread a set of
environments.

**2. The retroconsistency gate refused depletion.** A crew taking the last rock
out of a quarry it guards on was rejected by `gateConsistentTree` —
`BrokenGuard crew/digger take "in"` — because retracting the rock does break that
guard. Which is not damage. **It is depletion, the entire point.**

The fix was already precedent in that very function: it skips guards using
`$param`, because a guard whose variables are unbound has no determinate meaning
to a whole-tree scan. A `?binder` is the same case — evaluated there with empty
bindings it fans out and matches anything. Extended `usesParam` to cover binders,
with the same disclosed cost.

## Why this matters beyond convenience

Scarcity now expressible as an **external relation** instead of internal state.
Nothing records that a quarry is empty — no flag, no counter, no state on the
quarry at all. The transition stops being legal because the guard stops finding a
witness.

That connects directly to this morning's finding: a lifecycle lock is exactly what
`DMML.Recombine` **cannot** carry across a crossover, because exclusivity is a
property of the machine, not of the transitions. A guard and its effect travel
together. Breed a transition that spends a rock and the offspring spends a rock —
the constraint survives recombination because it is relational.

And the refusal is designed to be **productive**. `fire-transition` prints:

```
refused -- ?rock matches more than one thing, and choosing between them is
not this engine's to make.
Narrow it with a --param, or pick one of:
  rock/1
  rock/2
  rock/3
```

That is a Jev candidate list with the options already enumerated by the engine.
Refusing is how the choice reaches the chooser instead of being made by a fold.

## Verified

- `examples/quarry-demo/` — run through the **real `fire-transition` CLI**: one
  rock taken with a real `consumes` citation; three rocks refused with all three
  named; a second guard on the same `?rock` narrowing to the rich one; an empty
  quarry blocked.
- `guard-binding-selftest.hs` — 14 checks against genuine modules (real parser,
  real `applyIdentifiedCommits`, real `fireTransition`). Cabal executable, wired
  into CI.
- **29 apps and all 9 selftests build `-Wall`-clean and pass.** Every remaining
  warning in the tree is pre-existing in files this change didn't touch.

One process note: my earlier `ghc -isrc -iapp` invocations were missing `-Wall`
(the cabal file supplies it), which let a non-exhaustive `describeError` through
until it crashed at runtime on the new constructor. Rebuilt everything with
`-Wall` explicitly after that.

## Still open

- **`$param` cannot yet be bound FROM a guard** — the narrowing path is a caller
  passing `--param`, not the engine proposing one. That's the right division, but
  it means the driver has to turn a refusal into a question by hand; nothing wires
  `GuardAmbiguousBinding`'s candidate list into the Jev batch automatically yet.
- **`gateConsistentTree` now skips binder-bearing guards**, so real breakage
  involving one elsewhere in the tree is not caught. Same disclosed gap `$param`
  already had.
- **No substance layer** — this is the primitive the rocks-and-footpads
  conversation wanted, not the substances themselves. Nothing yet reads a
  substance off an object, and `retro-imply` can still only add, never spend.
